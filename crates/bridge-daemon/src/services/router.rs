use bridge_core::{BridgeMessage, MessageType};
use serde_json::json;
use std::sync::Arc;
use crate::{pairing::PairingManager, services::{file, clipboard, notify, media, telephony}};

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
        // Telephony Phase 3
        MessageType::SmsList => {
            let resp = telephony::handle_sms_list(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::SmsList, resp))
        },
        MessageType::SmsSend => {
            let resp = telephony::handle_sms_send(msg.payload.clone()).await;
            // If error code, return as Error type, else SmsSend ack
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::SmsSend, resp))
            }
        },
        MessageType::SmsReceived => {
            let resp = telephony::handle_sms_received(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::SmsReceived, resp))
        },
        MessageType::CallStart => {
            let resp = telephony::handle_call_start(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::CallStart, resp))
            }
        },
        MessageType::CallAnswer => {
            let resp = telephony::handle_call_answer(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::CallAnswer, resp))
        },
        MessageType::CallHangup => {
            let resp = telephony::handle_call_hangup(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::CallHangup, resp))
        },
        MessageType::CallAudio => {
            let resp = telephony::handle_call_audio(msg.payload.clone()).await;
            if resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::CallAudio, resp))
            }
        },
        MessageType::CallLog => {
            let resp = telephony::handle_call_log(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::CallLog, resp))
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
