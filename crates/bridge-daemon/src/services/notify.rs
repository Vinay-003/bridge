use serde_json::{Value, json};
use tracing::info;

pub async fn handle_action(payload: Value) {
    info!("notify action from desktop: {}", payload);
    // This will be broadcast back to phone via router; phone's BridgeService.handle will execute dismiss/reply
}

pub fn push_test() -> Value {
    json!({
        "key": "test-1",
        "app": "Bridge",
        "title": "Hello from Android",
        "body": "This is a mirrored notification",
        "ts": 0,
        "hasReply": true
    })
}

// Unit test helper
pub fn build_notify_payload(key: &str, app: &str, title: &str, body: &str, has_reply: bool) -> Value {
    json!({
        "key": key,
        "app": app,
        "title": title,
        "body": body,
        "ts": chrono::Utc::now().timestamp_millis(),
        "hasReply": has_reply
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notify_payload_has_required_fields() {
        let p = build_notify_payload("k1", "WhatsApp", "Mom", "Call me", true);
        assert_eq!(p["key"], "k1");
        assert_eq!(p["app"], "WhatsApp");
        assert!(p["hasReply"].as_bool().unwrap());
    }
}
