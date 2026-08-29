use bridge_core::{BridgeMessage, MessageType};
use serde_json::json;
use crate::services::{file, clipboard, notify, status};

pub async fn route(msg: BridgeMessage) -> Option<BridgeMessage> {
    match msg.typ {
        MessageType::Ping => Some(BridgeMessage::new(MessageType::Pong, json!({"ok": true}))),
        MessageType::FileChunk => {
            // store chunk
            let ack = file::handle_chunk(msg.payload).await;
            Some(BridgeMessage::new(MessageType::FileAck, ack))
        },
        MessageType::ClipboardSync => {
            clipboard::handle(msg.payload).await;
            None
        },
        MessageType::NotifyNew => None, // desktop doesn't originate notify.new, just relays
        MessageType::NotifyAction => {
            notify::handle_action(msg.payload).await;
            None
        },
        MessageType::StatusPush => None,
        MessageType::WebrtcOffer | MessageType::WebrtcAnswer | MessageType::WebrtcIce => {
            // echo as answer for local signalling demo
            Some(BridgeMessage::new(MessageType::WebrtcAnswer, msg.payload))
        },
        MessageType::PairingHello => {
            Some(BridgeMessage::new(MessageType::PairingSas, json!({"sas":"123456","fp":"abc123"})))
        },
        _ => None,
    }
}
