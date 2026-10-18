pub const VSOCK_PORT: u32 = 3924;

pub mod schema;

mod io_helpers;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;
