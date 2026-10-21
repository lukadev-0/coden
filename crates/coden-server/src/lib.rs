mod config;
mod routes;

use std::{ops::Deref, sync::Arc};

use anyhow::{Context, Result};
use axum::extract::{FromRequestParts, State};
use config::Config;
use tokio::{fs, net::TcpListener};
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument};

#[derive(Clone, FromRequestParts)]
#[from_request(via(State))]
struct AppState(Arc<AppStateInner>);

struct AppStateInner {
    config: Config,
    vm_manager: coden_vm::Manager,
    vm_vmlinux: coden_vm::Resource,
    vm_rootfs: coden_vm::Resource,
}

impl Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn run(cancel: CancellationToken) -> Result<()> {
    let config = Config::from_env().context("could not load configuration")?;
    let app_state = init(config)
        .await
        .context("could not initialize app state")?;

    let router = routes::router().with_state(app_state.clone());

    let listener = TcpListener::bind(&app_state.config.http_addr)
        .await
        .with_context(|| {
            format!(
                "could not bind HTTP listener to {}",
                app_state.config.http_addr
            )
        })?;

    info!(http_addr = %listener.local_addr()?, "listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(cancel.cancelled_owned())
        .await
        .context("error in HTTP server")?;

    app_state
        .vm_manager
        .shutdown()
        .await
        .context("failed to shut down vm manager")?;

    info!("goodbye!");

    Ok(())
}

#[instrument]
async fn init(config: Config) -> Result<AppState> {
    fs::create_dir_all(&config.state_dir)
        .await
        .with_context(|| {
            format!(
                "could not create directory '{}'",
                config.state_dir.display()
            )
        })?;

    let vm_manager = coden_vm::Manager::new(&coden_vm::ManagerOptions {
        state_dir: config.state_dir.clone(),
        firecracker: config.firecracker.clone(),
        firecracker_jailer: config.firecracker_jailer.clone(),
    })
    .await
    .context("could not create VM manager")?;

    let vm_vmlinux = vm_manager
        .load_resource("vmlinux", &config.vmlinux)
        .await
        .context("could not load vmlinux into VM manager")?;
    let vm_rootfs = vm_manager
        .load_resource("rootfs", &config.rootfs)
        .await
        .context("could not load rootfs into VM manager")?;

    let inner = AppStateInner {
        config,
        vm_manager,
        vm_vmlinux,
        vm_rootfs,
    };
    Ok(AppState(Arc::new(inner)))
}
