use anyhow::{Context, Result}; // Context trait 为错误添加上下文信息，Result 是 anyhow 的结果类型
use prost::Message; // protobuf 消息的编解码 trait
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt}; // 异步 I/O trait 及其扩展方法

use bm1_proto::message::CsRpcMsg; // protobuf 生成的 RPC 消息结构体

/// 从流中读取一个完整帧：4 字节大端长度头 + protobuf 消息体
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<CsRpcMsg> {
    // 先读 4 字节的帧长度
    let len = reader
        .read_u32()
        .await
        .context("failed to read frame length")? as usize; // 转为 usize 作为缓冲区大小
    // 分配精确大小的缓冲区，读取帧体
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .await
        .context("failed to read frame body")?;
    // 将字节流反序列化为 CsRpcMsg protobuf 结构
    CsRpcMsg::decode(&buf[..]).context("failed to decode CsRpcMsg")
}

/// 向流中写入一个完整帧：4 字节大端长度头 + protobuf 消息体
pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, msg: &CsRpcMsg) -> Result<()> {
    // 将 protobuf 消息序列化为字节向量
    let body = msg.encode_to_vec();
    // 先写 4 字节长度头
    writer
        .write_u32(body.len() as u32)
        .await
        .context("failed to write frame length")?;
    // 再写完整的消息体
    writer
        .write_all(&body)
        .await
        .context("failed to write frame body")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::message::{CsRpcCmd, LoginReq};
    use bm1_proto::message::cs_rpc_msg::Payload;

    #[tokio::test]
    async fn test_codec_roundtrip() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let msg = CsRpcMsg {
            cmd: CsRpcCmd::LoginReq as i32,
            seq: 42,
            session_id: 1,
            payload: Some(Payload::LoginReq(LoginReq { player_id: 1 })),
        };

        write_frame(&mut client, &msg).await.unwrap();
        let decoded = read_frame(&mut server).await.unwrap();

        assert_eq!(decoded.cmd, CsRpcCmd::LoginReq as i32);
        assert_eq!(decoded.seq, 42);
        assert_eq!(decoded.session_id, 1);
    }
}
