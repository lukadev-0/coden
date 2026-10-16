use anyhow::{Context, Result};
use bytes::{Buf, Bytes, BytesMut};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn read_line<R>(reader: &mut R, buf: &mut BytesMut) -> Result<Option<Bytes>>
where
    R: AsyncRead + Unpin,
{
    let mut start = 0;
    loop {
        if let Some(pos) = buf[start..].iter().position(|&b| b == b'\n') {
            let line = buf.split_to(pos + 1).freeze();
            break Ok(Some(line));
        }

        start += buf.remaining();

        if reader.read_buf(buf).await? == 0 {
            if buf.is_empty() {
                break Ok(None);
            } else {
                break Ok(Some(buf.split().freeze()));
            }
        }
    }
}

pub async fn write_json_line<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let mut data = serde_json::to_vec(value).context("could not serialize message")?;
    data.push(b'\n');
    writer
        .write_all(&data)
        .await
        .context("could not write line")?;
    Ok(())
}

pub async fn read_json_line<R, T>(reader: &mut R, buf: &mut BytesMut) -> Result<Option<T>>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let Some(line) = read_line(reader, buf)
        .await
        .context("could not read line")?
    else {
        return Ok(None);
    };

    let value = serde_json::from_slice(&line).context("could not deserialize message")?;
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn test_read_line() {
        let mut data = &b"abc\ndef\nghi\n"[..];
        let mut buf = BytesMut::new();

        assert_eq!(
            read_line(&mut data, &mut buf).await.unwrap(),
            Some("abc\n".into()),
        );
        assert_eq!(
            read_line(&mut data, &mut buf).await.unwrap(),
            Some("def\n".into()),
        );
        assert_eq!(
            read_line(&mut data, &mut buf).await.unwrap(),
            Some("ghi\n".into()),
        );
        assert_eq!(read_line(&mut data, &mut buf).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_write_json_line() {
        let mut data = Vec::new();

        write_json_line(&mut data, &json!({ "a": 6, "b": 7 }))
            .await
            .unwrap();

        assert_eq!(data, b"{\"a\":6,\"b\":7}\n");
    }

    #[tokio::test]
    async fn test_read_json_line() {
        let mut data = &b"{\"a\":6,\"b\":7}\n{\"a\":2,\"b\":1}"[..];
        let mut buf = BytesMut::new();

        assert_eq!(
            read_json_line(&mut data, &mut buf).await.unwrap(),
            Some(json!({ "a": 6, "b": 7 })),
        );
        assert_eq!(
            read_json_line(&mut data, &mut buf).await.unwrap(),
            Some(json!({ "a": 2, "b": 1 })),
        );
        assert_eq!(
            read_json_line(&mut data, &mut buf).await.unwrap(),
            None::<serde_json::Value>,
        );
    }
}
