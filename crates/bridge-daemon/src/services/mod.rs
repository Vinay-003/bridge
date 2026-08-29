pub mod clipboard;
pub mod file;
pub mod notify;
pub mod status;
pub mod router;

use bridge_core::{BridgeMessage, MessageType};
use serde_json::json;
use tracing::info;

pub async fn heartbeat_ping() -> BridgeMessage {
    BridgeMessage::new(MessageType::Pong, json!({"ok":true}))
}
