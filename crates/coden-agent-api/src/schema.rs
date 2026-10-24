use std::{path::PathBuf, time::Duration};

use bytes::Bytes;
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
        impl From<$req> for Request {
            fn from(value: $req) -> Request {
                Request::$variant(value)
            }
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request", content = "args")]
pub enum Request {
    Exec(ExecRequest),
    WriteFile(WriteFileRequest),
}

impl HasResponse for Request {
    type Response = serde_json::Value;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    pub program: String,
    pub args: Vec<String>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResponse {
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub time: Duration,
    pub output: String,
    pub status: ExecResponseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecResponseStatus {
    Code(i32),
    Signal(i32),
    #[serde(untagged)]
    Other,
}

impl_request!(Exec: ExecRequest -> ExecResponse);

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: PathBuf,
    #[serde_as(as = "serde_with::base64::Base64")]
    pub contents: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileResponse {}

impl_request!(WriteFile: WriteFileRequest -> WriteFileResponse);
