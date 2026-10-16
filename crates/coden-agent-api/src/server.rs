use anyhow::Result;
use bytes::BytesMut;
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::io_helpers;

pub struct Server<T> {
    transport: T,
    buf: BytesMut,
}

impl<T> Server<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            buf: BytesMut::new(),
        }
    }
}

impl<T> Server<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn read_request<Req>(&mut self) -> Result<Option<Req>>
    where
        Req: DeserializeOwned,
    {
        io_helpers::read_json_line(&mut self.transport, &mut self.buf).await
    }

    pub async fn send_response<Res>(&mut self, response: &Res) -> Result<()>
    where
        Res: Serialize,
    {
        io_helpers::write_json_line(&mut self.transport, &response).await
    }
}
