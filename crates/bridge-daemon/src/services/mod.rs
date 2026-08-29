pub mod clipboard;
pub mod file;
pub mod notify;
pub mod status;
pub mod router;
pub mod media;
pub mod control;
pub mod storage;
pub mod relay;
pub mod mesh;
pub mod plugin;
pub mod ai;

use bridge_core::{BridgeMessage, MessageType};
use serde_json::json;

pub async fn heartbeat_ping() -> BridgeMessage {
    BridgeMessage::new(MessageType::Pong, json!({"ok":true}))
}
