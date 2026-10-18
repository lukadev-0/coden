use std::fmt::Debug;

use anyhow::{Context, Result};
use bytes::BytesMut;
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::instrument;

use crate::{io_helpers, schema::HasResponse};

pub struct Client<T> {
    transport: T,
    buf: BytesMut,
}

impl<T> Client<T> {
    pub fn new(transport: T) -> Self {
        Client {
            transport,
            buf: BytesMut::new(),
        }
    }
}

impl<T> Client<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    #[instrument(skip(self))]
    pub async fn request<Req>(&mut self, request: &Req) -> Result<Req::Response>
    where
        Req: HasResponse + Debug + Serialize,
        Req::Response: DeserializeOwned + Debug,
    {
        io_helpers::write_json_line(&mut self.transport, &request)
            .await
            .context("could not send request")?;

        tracing::debug!("request sent");

        let response = io_helpers::read_json_line(&mut self.transport, &mut self.buf)
            .await
            .context("could not read response")?
            .context("connection closed")?;

        tracing::debug!(?response, "received response");
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::Client;
    use crate::schema;

    use serde::{Deserialize, Serialize};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct TestRequest {
        foo: i32,
        msg: String,
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct TestResponse {
        ok: bool,
    }

    impl schema::HasResponse for TestRequest {
        type Response = TestResponse;
    }

    #[tokio::test]
    async fn test_request() {
        let (client, server) = duplex(1024);
        let mut client = Client::new(client);
        let mut server = BufReader::new(server);

        let client_task = async move {
            let response = client
                .request(&TestRequest {
                    foo: 5,
                    msg: "hello".into(),
                })
                .await
                .unwrap();

            assert_eq!(response, TestResponse { ok: true });
        };

        let server_task = async move {
            let mut line = String::new();
            server.read_line(&mut line).await.unwrap();
            assert_eq!(line, concat!(r#"{"foo":5,"msg":"hello"}"#, "\n"));

            server.write_all(br#"{"ok":true}"#).await.unwrap();
        };

        tokio::join!(client_task, server_task);
    }
}
