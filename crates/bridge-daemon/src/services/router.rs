use bridge_core::{BridgeMessage, MessageType};
use serde_json::json;
use std::sync::Arc;
use crate::{pairing::PairingManager, services::{file, clipboard, notify, media, telephony, control, storage, relay, mesh, plugin, ai}};
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
            }        },
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
        // Relay + Mesh Phase 6
        MessageType::RelayAnnounce => {
            let resp = relay::handle_relay_announce(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::RelayAnnounce, resp))
            }
        },
        MessageType::RelayRelay => {
            let resp = relay::handle_relay_relay(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::RelayRelay, resp))
            }
        },
        MessageType::MeshSync => {
            let resp = mesh::handle_mesh_sync(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else if resp.get("conflict").and_then(|v| v.as_bool()).unwrap_or(false) {
                Some(BridgeMessage::new(MessageType::MeshConflict, resp))
            } else {
                Some(BridgeMessage::new(MessageType::MeshSync, resp))
            }
        },
        MessageType::MeshConflict => {
            let resp = mesh::handle_mesh_conflict(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::MeshConflict, resp))
            }
        },
        // Plugin Phase 7
        MessageType::PluginList => {
            let resp = plugin::handle_plugin_list(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::PluginList, resp))
        },
        MessageType::PluginLoad => {
            let resp = plugin::handle_plugin_load(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::PluginLoad, resp))
            }
        },
        MessageType::PluginEmit => {
            let resp = plugin::handle_plugin_emit(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::PluginEmit, resp))
            }
        },
        // AI Phase 7
        MessageType::AiSummarize => {
            let resp = ai::handle_ai_summarize(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::AiResult, resp))
            }
        },
        MessageType::AiTranscribe => {
            let resp = ai::handle_ai_transcribe(msg.payload.clone()).await;
            if resp.get("code").is_some() && resp.get("error").is_some() {
                Some(BridgeMessage::new(MessageType::Error, resp))
            } else {
                Some(BridgeMessage::new(MessageType::AiResult, resp))
            }
        },
        MessageType::AiResult => {
            let resp = ai::handle_ai_result(msg.payload.clone()).await;
            Some(BridgeMessage::new(MessageType::AiResult, resp))
        },
        _ => None,
    }
}
