use anyhow::{Context, Result};
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use bm1_proto::bm1::CsRpcMsg;

pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<CsRpcMsg> {
    let len = reader
        .read_u32()
        .await
        .context("failed to read frame length")? as usize;
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .await
        .context("failed to read frame body")?;
    CsRpcMsg::decode(&buf[..]).context("failed to decode CsRpcMsg")
}

pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, msg: &CsRpcMsg) -> Result<()> {
    let body = msg.encode_to_vec();
    writer
        .write_u32(body.len() as u32)
        .await
        .context("failed to write frame length")?;
    writer
        .write_all(&body)
        .await
        .context("failed to write frame body")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::bm1::{CsRpcCmd, PlaceholderReq};
    use bm1_proto::bm1::cs_rpc_msg::Payload;

    #[tokio::test]
    async fn test_codec_roundtrip() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let msg = CsRpcMsg {
            cmd: CsRpcCmd::Placeholder as i32,
            seq: 42,
            session_id: 1,
            payload: Some(Payload::PlaceholderReq(PlaceholderReq {
                msg: "hello".to_string(),
            })),
        };

        write_frame(&mut client, &msg).await.unwrap();
        let decoded = read_frame(&mut server).await.unwrap();

        assert_eq!(decoded.cmd, CsRpcCmd::Placeholder as i32);
        assert_eq!(decoded.seq, 42);
        assert_eq!(decoded.session_id, 1);
    }
}
