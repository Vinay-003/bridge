use bridge_core::{BridgeMessage, MessageType};
use serde_json::json;
use std::sync::Arc;
use crate::{pairing::PairingManager, services::{file, clipboard, notify, media}};

pub async fn route(msg: BridgeMessage, pairing: Arc<PairingManager>) -> Option<BridgeMessage> {
    match msg.typ {
        MessageType::Ping => Some(BridgeMessage::new(MessageType::Pong, json!({"ok": true}))),
        MessageType::FileChunk => {
            let ack = file::handle_chunk(msg.payload).await;
            Some(BridgeMessage::new(MessageType::FileAck, ack))
        },
        MessageType::ClipboardSync => {
            clipboard::handle(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::ClipboardSync, msg.payload))
        },
        MessageType::NotifyNew => {
            Some(BridgeMessage::new(MessageType::NotifyNew, msg.payload))
        },
        MessageType::NotifyAction => {
            notify::handle_action(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::NotifyAction, msg.payload))
        },
        MessageType::StatusPush => {
            // Phone's status: broadcast to desktops
            Some(BridgeMessage::new(MessageType::StatusPush, msg.payload))
        },
        MessageType::WebrtcOffer => {
            let resp = media::handle_offer(msg.payload).await;
            Some(BridgeMessage::new(MessageType::WebrtcAnswer, resp))
        },
        MessageType::WebrtcAnswer | MessageType::WebrtcIce => {
            Some(BridgeMessage::new(MessageType::WebrtcAnswer, msg.payload))
        },
        MessageType::PairingHello => {
            Some(BridgeMessage::new(MessageType::PairingTrusted, json!({
                "qr": pairing.qr_payload(),
                "host": pairing.host(),
                "port": 8443,
                "fp": pairing.fingerprint(),
                "sas": pairing.sas_preview(),
                "device_id": pairing.device_id()
            })))
        },
        MessageType::PairingSas => {
            Some(BridgeMessage::new(MessageType::PairingTrusted, json!({"trusted": true, "host": pairing.host(), "fp": pairing.fingerprint()})))
        },
        _ => None,
    }
}
