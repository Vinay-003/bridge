use serde_json::Value;
use tracing::info;

pub async fn handle_action(payload: Value) {
    info!("notify action: {}", payload);
    // forward to Android via broadcast; mock
}

pub fn push_test() -> Value {
    serde_json::json!({
        "key": "test-1",
        "app": "Bridge",
        "title": "Hello from Android",
        "body": "This is a mirrored notification",
        "ts": 0,
        "hasReply": true
    })
}
