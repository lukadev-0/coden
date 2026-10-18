use std::env;
use std::path::PathBuf;

use anyhow::Result;
use coden_agent_api::schema;
use coden_server::vm::{MachineConfig, Manager, ManagerOptions};
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let vmlinux = env::var_os("VMLINUX").unwrap();
    let rootfs = env::var_os("ROOTFS").unwrap();
    let firecracker = env::var_os("FIRECRACKER").unwrap();
    let firecracker_jailer = env::var_os("FIRECRACKER_JAILER").unwrap();

    let manager_options = ManagerOptions {
        state_dir: PathBuf::from("/tmp"),
        firecracker: PathBuf::from(firecracker),
        firecracker_jailer: PathBuf::from(firecracker_jailer),
    };
    let manager = Manager::new(&manager_options).await?;

    let vmlinux = manager.load_resource("vmlinux", vmlinux).await?;
    let rootfs = manager.load_resource("rootfs", rootfs).await?;

    let machine_config = MachineConfig {
        jailer_uid: 1000,
        jailer_gid: 1000,
        kernel_image: vmlinux,
        rootfs,
        vcpu_count: 1,
        mem_size_mib: 20,
    };

    eprint!("starting VM... ");
    let machine = manager.spawn(&machine_config).await?;
    machine.ready().await;
    eprintln!("connected!");

    let (line_sender, mut line_receiver) = mpsc::channel(1);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        loop {
            let mut input = String::new();
            if stdin.read_line(&mut input).unwrap() == 0 {
                break;
            }
            if line_sender.blocking_send(input).is_err() {
                break;
            }
        }
    });

    loop {
        eprint!("> ");
        tokio::select! {
            line = line_receiver.recv() => {
                let Some(line) = line else { break; };
                let res = machine
                    .request(schema::ExecRequest {
                        program: "sh".into(),
                        args: vec!["-c".into(), line],
                    })
                .await?;

                println!("---");
                println!("took {}ms", res.time.as_millis());
                match res.status {
                    schema::ExecResponseStatus::Code(code) => println!("exited with code {code}"),
                    schema::ExecResponseStatus::Signal(signal) => println!("signal {signal}"),
                    schema::ExecResponseStatus::Other => {},
                };
                println!("---");
                print!("{}", res.output);
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutting down");
                manager.shutdown().await?;
                info!("shut down");
                break;
            }
        }
    }

    Ok(())
}
