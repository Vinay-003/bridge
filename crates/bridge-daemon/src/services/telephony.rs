use serde_json::{json, Value};
use bridge_core::{validate_sms_send_payload, validate_call_start_payload, is_valid_phone_number};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tracing::{info, warn};

// Simple in-memory rate limiting: 20 sms/min, 3 calls/min per pseudo-peer (daemon single)
// For TDD we expose check_rate_limit

static SMS_TIMESTAMPS: OnceLock<Mutex<Vec<i64>>> = OnceLock::new();
static CALL_TIMESTAMPS: OnceLock<Mutex<Vec<i64>>> = OnceLock::new();

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn prune_and_check(limit: usize, window_ms: i64, store: &OnceLock<Mutex<Vec<i64>>>) -> bool {
    let mut guard = store.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    let now = now_ms();
    guard.retain(|&t| now - t < window_ms);
    if guard.len() >= limit {
        false
    } else {
        guard.push(now);
        true
    }
}

pub fn check_sms_rate_limit() -> Result<(), String> {
    if prune_and_check(20, 60_000, &SMS_TIMESTAMPS) {
        Ok(())
    } else {
        Err("rate_limited: 20 sms/min exceeded".into())
    }
}

pub fn check_call_rate_limit() -> Result<(), String> {
    if prune_and_check(3, 60_000, &CALL_TIMESTAMPS) {
        Ok(())
    } else {
        Err("rate_limited: 3 calls/min exceeded".into())
    }
}

pub fn redact_number(n: &str) -> String {
    let digits: String = n.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() <= 4 {
        return "****".into();
    }
    let last4 = &digits[digits.len()-4..];
    // preserve country prefix if starts with +
    if n.trim().starts_with('+') {
        format!("+** ****{}", last4)
    } else {
        format!("** ****{}", last4)
    }
}

pub fn audit_log(typ: &str, number: Option<&str>, result: &str) {
    let redacted = number.map(redact_number).unwrap_or_else(|| "-".into());
    // In real daemon this writes to ~/.local/share/bridge/audit.log
    // Here we just trace, not writing file in tests
    info!(target: "audit", "telephony {} number={} result={}", typ, redacted, result);
}

// Handlers

pub async fn handle_sms_list(payload: Value) -> Value {
    let limit = payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(50).min(200);
    let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let subscription_id = payload.get("subscriptionId").and_then(|v| v.as_i64());
    // Daemon doesn't have SMS; it relays to phone. Return echo for direct response (tests)
    // Phone will actually query ContentProvider and respond with sms.list
    json!({
        "limit": limit,
        "offset": offset,
        "subscriptionId": subscription_id,
        "messages": [],
        "subscriptions": [],
        "relay": true,
        "note": "phone should respond with sms.list via broadcast"
    })
}

pub async fn handle_sms_send(payload: Value) -> Value {
    // Validate
    if let Err(e) = validate_sms_send_payload(&payload) {
        warn!("sms.send validation failed: {}", e);
        audit_log("sms.send", payload.get("address").and_then(|v| v.as_str()), "validation_failed");
        return json!({"error": e.to_string(), "code": "invalid_number"});
    }
    if let Err(e) = check_sms_rate_limit() {
        warn!("sms rate limited");
        return json!({"error": e, "code": "rate_limited"});
    }
    let address = payload.get("address").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let body_len = payload.get("body").and_then(|v| v.as_str()).map(|s| s.chars().count()).unwrap_or(0);
    audit_log("sms.send", Some(&address), "accepted");
    // Relay to phone via WS broadcast; daemon ack
    json!({
        "id": payload.get("id").and_then(|v| v.as_str()).unwrap_or("sms-1"),
        "address": address,
        "body_len": body_len,
        "subscriptionId": payload.get("subscriptionId").and_then(|v| v.as_i64()),
        "status": "relayed",
        "note": "phone SmsManager.sendTextMessage will handle"
    })
}

pub async fn handle_call_start(payload: Value) -> Value {
    if let Err(e) = validate_call_start_payload(&payload) {
        warn!("call.start validation failed: {}", e);
        audit_log("call.start", payload.get("number").and_then(|v| v.as_str()), "validation_failed");
        return json!({"error": e.to_string(), "code": "invalid_number"});
    }
    if let Err(e) = check_call_rate_limit() {
        return json!({"error": e, "code": "rate_limited"});
    }
    let number = payload.get("number").and_then(|v| v.as_str()).unwrap_or("").to_string();
    audit_log("call.start", Some(&number), "accepted");
    // Must be confirmed by phone tap; daemon marks pendingConfirm
    json!({
        "callId": format!("call-{}", uuid::Uuid::new_v4()),
        "number": number,
        "subscriptionId": payload.get("subscriptionId").and_then(|v| v.as_i64()),
        "state": "RINGING",
        "requires_tap": true,
        "note": "phone must show Allow once dialog then TelecomManager.placeCall"
    })
}

pub async fn handle_call_answer(payload: Value) -> Value {
    let call_id = payload.get("callId").and_then(|v| v.as_str()).unwrap_or("unknown");
    audit_log("call.answer", None, "relayed");
    json!({"callId": call_id, "state":"OFFHOOK"})
}

pub async fn handle_call_hangup(payload: Value) -> Value {
    let call_id = payload.get("callId").and_then(|v| v.as_str()).unwrap_or("unknown");
    audit_log("call.hangup", None, "relayed");
    json!({"callId": call_id, "state":"HUNGUP"})
}

pub async fn handle_call_audio(payload: Value) -> Value {
    // Just relay; validate callId present
    let call_id = payload.get("callId").and_then(|v| v.as_str()).unwrap_or("");
    if call_id.is_empty() {
        return json!({"error": "missing callId", "code":"validation"});
    }
    json!({"callId": call_id, "relayed": true, "payload": payload})
}

pub async fn handle_call_log(payload: Value) -> Value {
    let limit = payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(50).min(200);
    json!({
        "limit": limit,
        "calls": [],
        "relay": true,
        "note": "phone should respond with CallLog.Calls query"
    })
}

pub async fn handle_sms_received(payload: Value) -> Value {
    // Inbound from phone when SMS arrives (READ_SMS)
    // Validate and broadcast to desktops
    json!({"received": true, "payload": payload})
}

// Test helpers to reset rate limit in tests
pub fn reset_rate_limits() {
    if let Some(m) = SMS_TIMESTAMPS.get() { m.lock().unwrap().clear(); }
    if let Some(m) = CALL_TIMESTAMPS.get() { m.lock().unwrap().clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn sms_list_returns_limit() {
        let v = handle_sms_list(json!({"limit":10})).await;
        assert_eq!(v["limit"], 10);
        assert!(v["messages"].is_array());
    }

    #[tokio::test]
    async fn sms_send_valid_relays() {
        reset_rate_limits();
        let v = handle_sms_send(json!({"address":"+33612345678","body":"Hello"})).await;
        assert_eq!(v["status"], "relayed");
    }

    #[tokio::test]
    async fn sms_send_invalid_number() {
        reset_rate_limits();
        let v = handle_sms_send(json!({"address":"bad","body":"hi"})).await;
        assert!(v["error"].is_string());
        assert_eq!(v["code"], "invalid_number");
    }

    #[tokio::test]
    async fn sms_send_rate_limit() {
        reset_rate_limits();
        for _ in 0..20 {
            let _ = handle_sms_send(json!({"address":"+33612345678","body":"hi"})).await;
        }
        let v = handle_sms_send(json!({"address":"+33612345678","body":"hi"})).await;
        assert_eq!(v["code"], "rate_limited");
        reset_rate_limits();
    }

    #[tokio::test]
    async fn call_start_valid() {
        reset_rate_limits();
        let v = handle_call_start(json!({"number":"+33612345678"})).await;
        assert_eq!(v["state"], "RINGING");
        assert_eq!(v["requires_tap"], true);
    }

    #[tokio::test]
    async fn call_start_invalid() {
        reset_rate_limits();
        let v = handle_call_start(json!({"number":"bad"})).await;
        assert!(v["error"].is_string());
    }

    #[tokio::test]
    async fn call_start_rate_limit() {
        reset_rate_limits();
        for _ in 0..3 {
            let _ = handle_call_start(json!({"number":"+33612345678"})).await;
        }
        let v = handle_call_start(json!({"number":"+33612345678"})).await;
        assert_eq!(v["code"], "rate_limited");
        reset_rate_limits();
    }

    #[tokio::test]
    async fn call_answer_ok() {
        let v = handle_call_answer(json!({"callId":"abc-123"})).await;
        assert_eq!(v["state"], "OFFHOOK");
    }

    #[tokio::test]
    async fn call_hangup_ok() {
        let v = handle_call_hangup(json!({"callId":"abc-123"})).await;
        assert_eq!(v["state"], "HUNGUP");
    }

    #[tokio::test]
    async fn call_audio_missing_callid() {
        let v = handle_call_audio(json!({"sdp":"v=0"})).await;
        assert!(v["error"].is_string());
    }

    #[tokio::test]
    async fn call_audio_ok() {
        let v = handle_call_audio(json!({"callId":"uuid","sdp":"v=0"})).await;
        assert_eq!(v["relayed"], true);
    }

    #[test]
    fn redact_works() {
        assert_eq!(redact_number("+33612345678"), "+** ****5678");
        assert_eq!(redact_number("0612345678"), "** ****5678");
        assert_eq!(redact_number("123"), "****");
    }
}
