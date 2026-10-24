use std::{env, ffi::OsString, path::PathBuf, str::FromStr};

use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone)]
pub struct Config {
    pub state_dir: PathBuf,
    pub http_addr: String,
    pub vmlinux: PathBuf,
    pub rootfs: PathBuf,
    pub firecracker: PathBuf,
    pub firecracker_jailer: PathBuf,
    pub jailer_uid: u32,
    pub jailer_gid: u32,
    pub vm_vcpu_count: usize,
    pub vm_mem_size_mib: usize,
    pub max_running_vms: usize,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        fn from_env_os<T>(name: &str) -> Result<T>
        where
            T: From<OsString>,
        {
            Ok(env::var_os(name)
                .with_context(|| format!("missing '{name}' environment variable"))?
                .into())
        }

        fn parse_env_option<T>(name: &str) -> Result<Option<T>>
        where
            T: FromStr,
            T::Err: Send + Sync + std::error::Error + 'static,
        {
            match env::var_os(name) {
                Some(v) => Ok(Some(
                    v.into_string()
                        .map_err(|_| anyhow!("'{name}' is not a valid Unicode string"))?
                        .parse()
                        .with_context(|| format!("'{name}' contains an invalid value"))?,
                )),
                None => Ok(None),
            }
        }

        Ok(Self {
            state_dir: env::var_os("STATE_DIRECTORY")
                .unwrap_or("/var/lib/coden".into())
                .into(),
            http_addr: parse_env_option("HTTP_ADDR")?.unwrap_or("localhost:8080".into()),
            vmlinux: from_env_os("VMLINUX")?,
            rootfs: from_env_os("ROOTFS")?,
            firecracker: from_env_os("FIRECRACKER")?,
            firecracker_jailer: from_env_os("FIRECRACKER_JAILER")?,
            jailer_uid: parse_env_option("CODEN_JAILER_UID")?.unwrap_or(1000),
            jailer_gid: parse_env_option("CODEN_JAILER_GID")?.unwrap_or(1000),
            vm_vcpu_count: parse_env_option("CODEN_VM_VCPU_COUNT")?.unwrap_or(1),
            vm_mem_size_mib: parse_env_option("CODEN_VM_MEM_SIZE_MIB")?.unwrap_or(20),
            max_running_vms: parse_env_option("CODEN_MAX_RUNNING_VMS")?.unwrap_or(8),
        })
    }
}
