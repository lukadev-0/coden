use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

pub trait HasResponse {
    type Response;
}

macro_rules! impl_request {
    ($variant:ident: $req:ident -> $res:ident) => {
        impl HasResponse for $req {
            type Response = $res;
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Request {
    Exec(ExecRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecRequest {
    pub command: Vec<String>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecResponse {
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub time: Duration,
    pub output: Vec<u8>,
}

impl_request!(Exec: ExecRequest -> ExecResponse);
