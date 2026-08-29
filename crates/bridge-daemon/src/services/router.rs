use bridge_core::{BridgeMessage, MessageType};
use serde_json::json;
use std::sync::Arc;
use crate::{pairing::PairingManager, services::{file, clipboard, notify, media, control, storage}};

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
        // Control Phase 4
        MessageType::InputEvent => {
            let resp = control::handle_input_event(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else if resp.get("throttled").and_then(|v| v.as_bool()).unwrap_or(false) {
                // Throttled is not error but special ack: send as InputAck with throttled flag
                Some(BridgeMessage::new(MessageType::InputAck, resp))
            } else {
                Some(BridgeMessage::new(MessageType::InputEvent, resp))
            }
        },
        MessageType::InputAck => {
            let resp = control::handle_input_ack(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::InputAck, resp))
        },
        MessageType::DisplayInfo => {
            let resp = control::handle_display_info(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::DisplayInfo, resp))
            }
        },
        MessageType::DisplayFrame => {
            let resp = control::handle_display_frame(msg.payload.clone()).await;
            if resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::DisplayFrame, resp))
            }
        },
        MessageType::ControlStart => {
            let resp = control::handle_control_start(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::ControlStart, resp))
            }
        },
        MessageType::ControlStop => {
            let resp = control::handle_control_stop(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::ControlStop, resp))
        },
        // Storage Phase 5
        MessageType::StorageLs => {
            let resp = storage::handle_storage_ls(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::StorageLs, resp))
            }
        },
        MessageType::StorageStat => {
            let resp = storage::handle_storage_stat(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::StorageStat, resp))
            }
        },
        MessageType::StorageMkdir => {
            let resp = storage::handle_storage_mkdir(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::StorageMkdir, resp))
            }
        },
        MessageType::StorageRm => {
            let resp = storage::handle_storage_rm(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::StorageRm, resp))
            }
        },
        MessageType::StorageSync => {
            let resp = storage::handle_storage_sync(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else if resp.get("conflict").and_then(|v| v.as_bool()).unwrap_or(false) {
                Some(BridgeMessage::new(MessageType::StorageConflict, resp))
            } else {
                Some(BridgeMessage::new(MessageType::StorageSync, resp))
            }
        },
        MessageType::StorageConflict => {
            let resp = storage::handle_storage_conflict(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::StorageConflict, resp))
            }
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
