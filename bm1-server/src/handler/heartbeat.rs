use bm1_proto::message::{CsRpcCmd, CsRpcMsg, HeartbeatResp}; // 命令枚举、消息体、心跳响应结构
use bm1_proto::message::cs_rpc_msg::Payload; // oneof payload 枚举

use crate::router::{Context, MessageHandler}; // 路由上下文和处理器 trait

/// Heartbeat 命令处理器：将客户端发来的时间戳原样回传
pub struct HeartbeatHandler;

impl MessageHandler for HeartbeatHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        // 从 oneof payload 中提取 HeartbeatReq 的时间戳
        let timestamp = match &msg.payload {
            Some(Payload::HeartbeatReq(req)) => req.timestamp, // 取出请求中的时间戳
            _ => 0, // payload 类型不匹配，默认返回 0
        };

        // 构造心跳响应
        Some(CsRpcMsg {
            cmd: CsRpcCmd::Heartbeat as i32, // Heartbeat 命令号
            seq: msg.seq,                    // 保留原序列号
            session_id: ctx.session_id,      // 填入当前会话 ID
            payload: Some(Payload::HeartbeatResp(HeartbeatResp {
                timestamp, // 原样回传时间戳
            })),
        })
    }
}
