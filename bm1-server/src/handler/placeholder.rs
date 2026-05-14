use bm1_proto::message::{CsRpcCmd, CsRpcMsg, PlaceholderResp}; // 命令枚举、消息体、Placeholder 响应结构
use bm1_proto::message::cs_rpc_msg::Payload; // oneof payload 枚举

use crate::router::{Context, MessageHandler}; // 路由上下文和处理器 trait

/// Placeholder 命令处理器：将客户端发来的消息原样 echo 回去
pub struct PlaceholderHandler;

impl MessageHandler for PlaceholderHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        // 从 oneof payload 中提取 PlaceholderReq 的 msg 字段
        let req_msg = match &msg.payload {
            Some(Payload::PlaceholderReq(req)) => &req.msg, // 取出请求中的消息文本
            _ => return None, // payload 类型不匹配，不回复
        };

        // 构造 echo 响应
        Some(CsRpcMsg {
            cmd: CsRpcCmd::Placeholder as i32, // Placeholder 命令号
            seq: msg.seq,                      // 保留原序列号（请求-响应配对）
            session_id: ctx.session_id,        // 填入当前会话 ID
            payload: Some(Payload::PlaceholderResp(PlaceholderResp {
                msg: format!("echo: {}", req_msg), // 消息前加 "echo: " 前缀返回
            })),
        })
    }
}
