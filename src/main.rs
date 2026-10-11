use std::env;
use std::path::PathBuf;

use anyhow::Result;
use coden::vm::{MachineConfig, ManagerOptions};

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
    let manager = coden::vm::Manager::new(&manager_options).await?;

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

    let machine = manager.spawn(&machine_config).await?;
    machine.wait().await;

    manager.shutdown().await?;

    Ok(())
}
