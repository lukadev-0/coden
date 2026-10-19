use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Context, Result};
use coden_agent_api::client::Client;
use coden_agent_api::schema::HasResponse;
use coden_agent_api::{VSOCK_PORT, schema};
use rand::distr::{Alphanumeric, SampleString};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tokio::net::unix::pipe::pipe;
use tokio::process::{Child, Command};
use tokio::sync::{SetOnce, mpsc, oneshot};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{Instrument, debug, error, info, info_span, instrument, warn};

const TMP_DIR_PREFIX: &str = "coden.";
const TMP_DIR_RAND_LEN: usize = 16;
const MACHINE_ID_LEN: usize = 16;
const RESOURCE_FILE_RAND_LEN: usize = 8;

struct DirGuard<A: AsRef<Path>>(A);
impl<A: AsRef<Path>> Drop for DirGuard<A> {
    #[instrument(skip_all)]
    fn drop(&mut self) {
        let dir = self.0.as_ref();
        debug!(dir = %dir.display(), "removing directory");
        if let Err(e) = std::fs::remove_dir_all(dir) {
            warn!(dir = %dir.display(), err = ?e, "failed to remove directory");
        }
    }
}

pub struct Manager {
    tasks: TaskTracker,
    cancel: CancellationToken,
    tmp_dir: PathBuf,
    _tmp_lock: std::fs::File,
    jailer_base_dir: PathBuf,
    firecracker: PathBuf,
    firecracker_jailer: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ManagerOptions {
    pub state_dir: PathBuf,
    pub firecracker: PathBuf,
    pub firecracker_jailer: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MachineConfig {
    pub jailer_uid: u32,
    pub jailer_gid: u32,
    pub kernel_image: Resource,
    pub rootfs: Resource,
    pub vcpu_count: usize,
    pub mem_size_mib: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct Resource(PathBuf);

impl Manager {
    pub async fn new(options: &ManagerOptions) -> Result<Self> {
        let state_dir = &options.state_dir;

        let (tmp_dir, tmp_lock) = tokio::task::spawn_blocking({
            let state_dir = state_dir.clone();
            move || {
                Self::acquire_tmp_dir(&state_dir)
                    .context("failed to acquire temporary directory for VM manager")
            }
        })
        .await??;
        let tmp_dir_guard = DirGuard(&tmp_dir);

        let firecracker_file_name = options
            .firecracker
            .file_name()
            .context("invalid firecracker path")?;
        let jailer_base_dir = tmp_dir.join(firecracker_file_name);
        tokio::fs::create_dir(&jailer_base_dir)
            .await
            .with_context(|| {
                format!("could not create directory '{}'", jailer_base_dir.display())
            })?;

        info!(?options, tmp_dir = %tmp_dir.display(), "created VM manager");

        std::mem::forget(tmp_dir_guard);
        let manager = Manager {
            tasks: TaskTracker::new(),
            cancel: CancellationToken::new(),
            tmp_dir,
            _tmp_lock: tmp_lock,
            jailer_base_dir,
            firecracker: options.firecracker.clone(),
            firecracker_jailer: options.firecracker_jailer.clone(),
        };

        manager.tasks.spawn_blocking({
            let state_dir = state_dir.clone();
            move || Self::clean_up_tmp_dirs(&state_dir)
        });

        Ok(manager)
    }

    fn acquire_tmp_dir(state_dir: &Path) -> Result<(PathBuf, std::fs::File)> {
        loop {
            let tmp_dir = state_dir.join(format!(
                "{TMP_DIR_PREFIX}{}",
                Alphanumeric.sample_string(&mut rand::rng(), TMP_DIR_RAND_LEN)
            ));
            let tmp_lock_path = tmp_dir.with_added_extension("lock");

            debug!(?tmp_dir, "attempting to acquire tmp dir");
            match std::fs::File::create_new(&tmp_lock_path) {
                Ok(tmp_lock) => {
                    if tmp_lock.try_lock().is_ok() {
                        std::fs::create_dir(&tmp_dir).with_context(|| {
                            format!("failed to make directory '{}'", tmp_dir.display())
                        })?;
                        break Ok((tmp_dir, tmp_lock));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => {
                    break Err(e).with_context(|| {
                        format!("failed to create file '{}'", tmp_lock_path.display())
                    });
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn clean_up_tmp_dirs(state_dir: &Path) {
        let dir_entries = match std::fs::read_dir(state_dir) {
            Ok(dir_entries) => dir_entries,
            Err(e) => {
                warn!(err = ?e, "failed to read state dir");
                return;
            }
        };

        for entry in dir_entries {
            if let Ok(entry) = entry
                && entry.file_type().is_ok_and(|ty| ty.is_dir())
                && entry
                    .file_name()
                    .into_string()
                    .is_ok_and(|name| name.starts_with(TMP_DIR_PREFIX))
            {
                let entry_path = entry.path();
                debug!(tmp_dir = ?entry_path, "found tmp dir");

                let entry_lock_path = entry_path.with_added_extension("lock");
                if std::fs::File::open(&entry_lock_path).is_ok_and(|f| f.try_lock().is_ok()) {
                    if let Err(e) = std::fs::remove_file(&entry_lock_path) {
                        warn!(lock_file = ?entry_lock_path, err = ?e, "failed to remove lock file");
                        continue;
                    }
                    if let Err(e) = std::fs::remove_dir_all(&entry_path) {
                        warn!(tmp_dir = ?entry_path, err = ?e, "failed to remove unused dir");
                        continue;
                    }
                    warn!(tmp_dir = ?entry_path, "removed unused dir");
                }
            }
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("shutting down VMs...");
        self.cancel.cancel();
        let did_close = self.tasks.close();
        self.tasks.wait().await;
        if did_close {
            debug!(tmp_dir = ?self.tmp_dir, "cleaning up temporary directory");
            let tmp_lock_path = self.tmp_dir.with_added_extension("lock");
            tokio::fs::remove_dir_all(&self.tmp_dir)
                .await
                .with_context(|| {
                    format!(
                        "could not remove temporary directory '{}'",
                        tmp_lock_path.display()
                    )
                })?;
            tokio::fs::remove_file(&tmp_lock_path)
                .await
                .with_context(|| {
                    format!("could not remove lock file '{}'", tmp_lock_path.display())
                })?;
        }
        Ok(())
    }

    pub async fn load_resource(&self, name: &str, src: impl AsRef<Path>) -> Result<Resource> {
        let src = src.as_ref();
        let res_file_name = format!(
            "{name}.{}",
            Alphanumeric.sample_string(&mut rand::rng(), RESOURCE_FILE_RAND_LEN)
        );
        let res_file_path = self.tmp_dir.join(res_file_name);
        tokio::fs::copy(&src, &res_file_path)
            .await
            .with_context(|| {
                format!(
                    "could not copy '{}' to '{}'",
                    src.display(),
                    res_file_path.display()
                )
            })?;
        Ok(Resource(res_file_path))
    }

    #[instrument(skip_all, fields(machine_id))]
    pub async fn spawn(&self, config: &MachineConfig) -> Result<Machine> {
        let machine_id = Alphanumeric.sample_string(&mut rand::rng(), MACHINE_ID_LEN);
        tracing::Span::current().record("machine_id", &machine_id);

        info!(?config, "spawning a new machine");

        debug!("setting up required directories and files");

        let machine_dir = self.jailer_base_dir.join(&machine_id);
        tokio::fs::create_dir(&machine_dir)
            .await
            .with_context(|| format!("could not create directory '{}'", machine_dir.display()))?;
        let machine_dir_guard = DirGuard(&machine_dir);

        let chroot_dir = machine_dir.join("root");
        tokio::fs::create_dir(&chroot_dir)
            .await
            .with_context(|| format!("could not create directory '{}'", chroot_dir.display()))?;

        tokio::fs::hard_link(&config.kernel_image.0, chroot_dir.join("kernel"))
            .await
            .with_context(|| {
                format!(
                    "could not link kernel '{}'",
                    config.kernel_image.0.display()
                )
            })?;
        tokio::fs::hard_link(&config.rootfs.0, chroot_dir.join("rootfs"))
            .await
            .with_context(|| format!("could not link rootfs '{}'", config.rootfs.0.display()))?;

        let vsock_path = chroot_dir.join(format!("vsock_{VSOCK_PORT}"));
        let vsock_listener = UnixListener::bind(&vsock_path)
            .with_context(|| format!("could not bind vsock uds '{}'", vsock_path.display()))?;
        std::os::unix::fs::chown(
            &vsock_path,
            Some(config.jailer_uid),
            Some(config.jailer_gid),
        )
        .with_context(|| {
            format!(
                "could not change owner of socket at '{}' to uid {} and gid {}",
                vsock_path.display(),
                config.jailer_uid,
                config.jailer_gid
            )
        })?;

        let fc_config = serde_json::json!({
            "boot-source": {
                "kernel_image_path": "kernel",
                "boot_args": "console=ttyS0 reboot=k panic=1",
            },
            "drives": [
                {
                    "drive_id": "rootfs",
                    "is_root_device": true,
                    "is_read_only": true,
                    "path_on_host": "rootfs",
                },
            ],
            "vsock": {
                "guest_cid": 3,
                "uds_path": "vsock",
            },
        });
        let fc_config_json =
            serde_json::to_string(&fc_config).context("failed to encode Firecracker config")?;
        tokio::fs::write(chroot_dir.join("fc-config.json"), fc_config_json)
            .await
            .context("failed to write Firecracker config file")?;

        let span = info_span!(parent: None, "machine", machine_id);
        let tasks = TaskTracker::new();
        let cancel = self.cancel.child_token();

        let (stdout_writer, output_reader) = pipe()?;
        let stdout_writer = stdout_writer.into_blocking_fd()?;
        let stderr_writer = stdout_writer.try_clone()?;

        let mut output_lines = BufReader::new(output_reader).lines();
        tasks.spawn(
            async move {
                while let Ok(Some(line)) = output_lines.next_line().await {
                    debug!("{line}");
                }
            }
            .instrument(info_span!(parent: &span, "firecracker")),
        );

        let child = Command::new(&self.firecracker_jailer)
            .arg("--id")
            .arg(&machine_id)
            .arg("--exec-file")
            .arg(&self.firecracker)
            .arg("--uid")
            .arg(config.jailer_uid.to_string())
            .arg("--gid")
            .arg(config.jailer_gid.to_string())
            .arg("--chroot-base-dir")
            .arg(&self.tmp_dir)
            .arg("--")
            .arg("--enable-pci")
            .arg("--config-file")
            .arg("fc-config.json")
            .kill_on_drop(true)
            .stdout(stdout_writer)
            .stderr(stderr_writer)
            .stdin(Stdio::null())
            .spawn()
            .context("failed to spawn Firecracker jailer")?;

        let (cmd_sender, cmd_receiver) = mpsc::channel(1);
        let ready = Arc::new(SetOnce::new());

        std::mem::forget(machine_dir_guard);
        tasks.spawn(
            Machine::run_actor(
                cancel.clone(),
                machine_dir,
                child,
                vsock_listener,
                cmd_receiver,
                ready.clone(),
            )
            .instrument(span.clone()),
        );
        tasks.close();

        let machine = Machine {
            tasks: tasks.clone(),
            cancel,
            cmd_sender,
            ready,
        };

        self.tasks.spawn({
            async move {
                tasks.wait().await;
                info!("finished");
            }
            .instrument(span)
        });

        Ok(machine)
    }
}

#[derive(Clone)]
pub struct Machine {
    tasks: TaskTracker,
    cancel: CancellationToken,
    cmd_sender: mpsc::Sender<MachineCommand>,
    ready: Arc<SetOnce<()>>,
}

struct MachineCommand {
    request: schema::Request,
    reply_to: oneshot::Sender<Result<serde_json::Value>>,
}

impl Machine {
    async fn run_actor(
        cancel: CancellationToken,
        machine_dir: PathBuf,
        mut child: Child,
        vsock_listener: UnixListener,
        mut cmd_receiver: mpsc::Receiver<MachineCommand>,
        ready: Arc<SetOnce<()>>,
    ) {
        let mut did_kill = false;
        let mut agent_client = None;
        loop {
            tokio::select! {
                res = vsock_listener.accept(), if agent_client.is_none() => {
                    let (stream, _) = match res {
                        Ok(res) => res,
                        Err(e) => {
                            error!(err = ?e, "failed to accept vsock connection");
                            continue;
                        }
                    };

                    info!("agent connected");
                    agent_client = Some(Client::new(stream));
                    _ = ready.set(());
                }
                cmd = cmd_receiver.recv(), if agent_client.is_some() => {
                    let Some(cmd) = cmd else {
                        if !cancel.is_cancelled() {
                            debug!("no longer receiving any commands");
                            cancel.cancel();
                        }
                        continue;
                    };
                    let agent_client = agent_client.as_mut().unwrap();

                    let res = agent_client.request(&cmd.request).await.context("could not perform request");
                    _ = cmd.reply_to.send(res);
                }
                _ = cancel.cancelled(), if !did_kill => {
                    did_kill = true;
                    info!("cancel requested; killing VM");
                    _ = child.start_kill();
                }
                exit_status = child.wait() => {
                    match exit_status {
                        Ok(exit_status) => info!(%exit_status, "VM exited"),
                        Err(e) => error!(err = ?e, "failed to wait on VM"),
                    }
                    break;
                }
            }
        }

        debug!("loop finished; cleaning up");
        if let Err(e) = tokio::fs::remove_dir_all(&machine_dir).await {
            error!(err = ?e, machine_dir = %machine_dir.display(), "failed to remove VM directory");
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready.get().is_some()
    }

    pub async fn ready(&self) {
        self.ready.wait().await;
    }

    pub fn request_stop(&self) {
        self.cancel.cancel();
    }

    pub async fn wait(&self) {
        self.tasks.wait().await;
    }

    pub async fn request<R>(&self, req: R) -> Result<R::Response>
    where
        R: HasResponse + Into<schema::Request> + Serialize,
        R::Response: DeserializeOwned,
    {
        let (reply_sender, reply_receiver) = oneshot::channel();
        self.cmd_sender
            .send(MachineCommand {
                request: req.into(),
                reply_to: reply_sender,
            })
            .await
            .context("machine closed")?;

        reply_receiver
            .await
            .context("no response")?
            .and_then(|res| {
                serde_json::from_value::<R::Response>(res).context("could not deserialize response")
            })
    }
}
