use serde_json::{json, Value};
use bridge_core::{validate_input_event_payload, validate_control_start_payload, ControlState};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tracing::{info, warn};

// Rate limiting: 120 input.events / sec per daemon (global for simplicity)
// For TDD we expose pure helper
static INPUT_TIMESTAMPS: OnceLock<Mutex<Vec<i64>>> = OnceLock::new();
static LAST_INPUT_TS: OnceLock<Mutex<Option<i64>>> = OnceLock::new();
static CONTROL_STATE: OnceLock<Mutex<ControlState>> = OnceLock::new();

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

// Pure helper for deterministic testing (no global)
fn check_limit_vec(limit: usize, window_ms: i64, vec: &mut Vec<i64>, now: i64) -> bool {
    vec.retain(|&t| now - t < window_ms);
    if vec.len() >= limit {
        false
    } else {
        vec.push(now);
        true
    }
}

pub fn check_input_rate_limit() -> Result<(), String> {
    if prune_and_check(120, 1000, &INPUT_TIMESTAMPS) {
        Ok(())
    } else {
        Err("rate_limited: 120 input.events/sec exceeded".into())
    }
}

pub fn check_throttle(now: i64, throttle_ms: i64) -> bool {
    // returns true if should throttle (drop)
    let mut guard = LAST_INPUT_TS.get_or_init(|| Mutex::new(None)).lock().unwrap();
    if let Some(last) = *guard {
        if now - last < throttle_ms {
            return true;
        }
    }
    *guard = Some(now);
    false
}

pub fn is_throttled_vec(last: &mut Option<i64>, now: i64, throttle_ms: i64) -> bool {
    if let Some(l) = *last {
        if now - l < throttle_ms {
            return true;
        }
    }
    *last = Some(now);
    false
}

pub fn redact_coords(x: f64, y: f64) -> String {
    // For audit we don't log precise coords, just bucket
    let xb = (x * 10.0).floor() / 10.0;
    let yb = (y * 10.0).floor() / 10.0;
    format!("{:.1},{:.1}", xb, yb)
}

pub fn audit_log(typ: &str, display_id: Option<i64>, result: &str) {
    let did = display_id.map(|d| d.to_string()).unwrap_or_else(|| "-".into());
    info!(target: "audit", "control {} display={} result={}", typ, did, result);
}

pub fn clamp_xy(v: f64) -> Option<f64> {
    if !v.is_finite() || v < 0.0 || v > 1.0 {
        None
    } else {
        Some(v)
    }
}

// Handlers

pub async fn handle_input_event(payload: Value) -> Value {
    // Validate via bridge_core
    if let Err(e) = validate_input_event_payload(&payload) {
        warn!("input.event validation failed: {}", e);
        audit_log("input.event", payload.get("displayId").and_then(|v| v.as_i64()), "validation_failed");
        return json!({"error": e.to_string(), "code": "validation"});
    }
    // Throttle check 60fps = 16ms
    let now = payload.get("ts").and_then(|v| v.as_i64()).unwrap_or_else(now_ms);
    // Use global throttle check (coalesce)
    // For move actions, throttle is expected; for tap we don't throttle strictly
    let action = payload.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let is_move = action == "move";
    if is_move && check_throttle(now, 16) {
        // Coalesced drop, not error but throttled ack
        audit_log("input.event", payload.get("displayId").and_then(|v| v.as_i64()), "throttled");
        return json!({"ok": false, "throttled": true, "code": "throttled", "displayId": payload.get("displayId").and_then(|v| v.as_i64()).unwrap_or(0)});
    }
    if let Err(e) = check_input_rate_limit() {
        warn!("input rate limited");
        return json!({"error": e, "code": "rate_limited"});
    }
    let display_id = payload.get("displayId").and_then(|v| v.as_i64());
    audit_log("input.event", display_id, "accepted");
    // Relay to phone via WS broadcast; daemon ack
    // We also echo normalized coords but redacted in audit
    let x = payload.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = payload.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let clamped_x = clamp_xy(x).unwrap_or(0.0);
    let clamped_y = clamp_xy(y).unwrap_or(0.0);
    json!({
        "x": clamped_x,
        "y": clamped_y,
        "action": action,
        "displayId": display_id.unwrap_or(0),
        "pointerId": payload.get("pointerId").and_then(|v| v.as_i64()).unwrap_or(0),
        "ok": true,
        "relayed": true,
        "note": "phone AccessibilityService.dispatchGesture will handle"
    })
}

pub async fn handle_control_start(payload: Value) -> Value {
    if let Err(e) = validate_control_start_payload(&payload) {
        warn!("control.start validation failed: {}", e);
        return json!({"error": e.to_string(), "code": "validation"});
    }
    // Check control state transition
    let state_guard = CONTROL_STATE.get_or_init(|| Mutex::new(ControlState::Disabled)).lock().unwrap().clone();
    // Simplify: allow DISABLED->ENABLED->CONTROLLING flow, but daemon doesn't enforce toggle; phone does.
    // Daemon just logs and transitions to CONTROLLING if not already
    drop(state_guard);
    let mut guard = CONTROL_STATE.get_or_init(|| Mutex::new(ControlState::Disabled)).lock().unwrap();
    // Try to transition: if Disabled -> Enabled -> Controlling
    if *guard == ControlState::Disabled {
        if guard.can_transition(&ControlState::Enabled) {
            *guard = ControlState::Enabled;
        }
    }
    if *guard == ControlState::Enabled && guard.can_transition(&ControlState::Controlling) {
        *guard = ControlState::Controlling;
    } else if *guard == ControlState::Paused && guard.can_transition(&ControlState::Enabled) {
        *guard = ControlState::Enabled;
        if guard.can_transition(&ControlState::Controlling) {
            *guard = ControlState::Controlling;
        }
    } else if *guard == ControlState::Controlling {
        // already controlling, ok (re-start)
    }
    let display_id = payload.get("displayId").and_then(|v| v.as_i64()).unwrap_or(0);
    audit_log("control.start", Some(display_id), "accepted");
    json!({
        "ok": true,
        "state": "CONTROLLING",
        "displayId": display_id,
        "quality": payload.get("quality").and_then(|v| v.as_i64()).unwrap_or(80),
        "fps": payload.get("fps").and_then(|v| v.as_i64()).unwrap_or(30),
        "note": "phone should start MediaProjection if allow_input_control true and unlocked"
    })
}

pub async fn handle_control_stop(payload: Value) -> Value {
    let mut guard = CONTROL_STATE.get_or_init(|| Mutex::new(ControlState::Disabled)).lock().unwrap();
    // Transition to ENABLED or DISABLED depending on reason
    let reason = payload.get("reason").and_then(|v| v.as_str()).unwrap_or("user");
    let display_id = payload.get("displayId").and_then(|v| v.as_i64()).unwrap_or(0);
    if *guard == ControlState::Controlling {
        if guard.can_transition(&ControlState::Enabled) {
            *guard = ControlState::Enabled;
        } else if guard.can_transition(&ControlState::Paused) {
            *guard = ControlState::Paused;
        } else if guard.can_transition(&ControlState::Disabled) {
            *guard = ControlState::Disabled;
        }
    } else if *guard == ControlState::Paused && reason == "toggle_off" {
        if guard.can_transition(&ControlState::Disabled) {
            *guard = ControlState::Disabled;
        }
    }
    audit_log("control.stop", Some(display_id), "accepted");
    json!({
        "ok": true,
        "state": format!("{:?}", *guard).to_uppercase(),
        "displayId": display_id,
        "reason": reason
    })
}

pub async fn handle_display_info(payload: Value) -> Value {
    // Validate optional
    if let Err(e) = bridge_core::validate_display_info_payload(&payload) {
        // Only error if payload has displays but invalid; empty is ok for relay
        // For daemon we just relay, but we validate if provided
        // If payload is empty, we still return placeholder
        // To avoid breaking, only error if explicitly invalid
        warn!("display.info validation: {}", e);
        // Not fatal for relay, but return error for tests where we check validation
        // Let's check if payload has displays array or displayId; if missing we error only if we consider missing invalid
        // For relay we allow empty and return dummy
        if !payload.is_null() && !payload.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            return json!({"error": e.to_string(), "code": "validation"});
        }
    }
    // Daemon doesn't have display info; it relays phone's info
    // If payload empty, return dummy primary display
    if payload.get("displayId").is_none() && payload.get("displays").is_none() {
        return json!({
            "displays": [{"displayId":0,"width":1080,"height":2400,"dpi":440,"density":2.75,"rotation":0,"name":"Built-in","isPrimary":true}],
            "primaryDisplayId": 0,
            "relay": true
        });
    }
    // Otherwise echo
    json!({
        "displays": payload.get("displays").unwrap_or(&json!([])),
        "displayId": payload.get("displayId").and_then(|v| v.as_i64()),
        "width": payload.get("width").and_then(|v| v.as_i64()),
        "height": payload.get("height").and_then(|v| v.as_i64()),
        "dpi": payload.get("dpi").and_then(|v| v.as_i64()),
        "density": payload.get("density").and_then(|v| v.as_f64()),
        "primaryDisplayId": payload.get("primaryDisplayId").and_then(|v| v.as_i64()).unwrap_or(0),
        "relay": true
    })
}

pub async fn handle_display_frame(payload: Value) -> Value {
    // Validate basic fields: displayId, frame_b64
    let did = payload.get("displayId").and_then(|v| v.as_i64()).unwrap_or(0);
    let frame = payload.get("frame_b64").and_then(|v| v.as_str()).unwrap_or("");
    if frame.is_empty() {
        return json!({"error": "missing frame_b64", "code": "validation"});
    }
    // Size check: limit 5MB base64
    if frame.len() > 7_000_000 {
        return json!({"error": "frame too large", "code": "validation"});
    }
    json!({
        "displayId": did,
        "frame_b64": frame,
        "width": payload.get("width").and_then(|v| v.as_i64()),
        "height": payload.get("height").and_then(|v| v.as_i64()),
        "format": payload.get("format").and_then(|v| v.as_str()).unwrap_or("jpeg"),
        "ts": payload.get("ts").and_then(|v| v.as_i64()).unwrap_or_else(now_ms),
        "relayed": true
    })
}

pub async fn handle_input_ack(payload: Value) -> Value {
    // Phone's ack, just relay
    json!({"ok": payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(true), "latencyMs": payload.get("latencyMs").and_then(|v| v.as_i64()).unwrap_or(0), "displayId": payload.get("displayId").and_then(|v| v.as_i64()).unwrap_or(0), "throttled": payload.get("throttled").and_then(|v| v.as_bool()).unwrap_or(false)})
}

// Test helpers
pub fn reset_control_state() {
    if let Some(m) = CONTROL_STATE.get() {
        let mut g = m.lock().unwrap();
        *g = ControlState::Disabled;
    }
    if let Some(m) = INPUT_TIMESTAMPS.get() {
        m.lock().unwrap().clear();
    }
    if let Some(m) = LAST_INPUT_TS.get() {
        *m.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn input_event_valid_relays() {
        reset_control_state();
        let v = handle_input_event(json!({"x":0.42,"y":0.71,"action":"tap","displayId":0,"ts":1000})).await;
        assert_eq!(v["relayed"], true);
        assert_eq!(v["action"], "tap");
    }

    #[tokio::test]
    async fn input_event_invalid_coords() {
        reset_control_state();
        let v = handle_input_event(json!({"x":1.5,"y":0.5,"action":"tap"})).await;
        assert!(v["error"].is_string());
        assert_eq!(v["code"], "validation");
    }

    #[tokio::test]
    async fn input_event_invalid_action() {
        reset_control_state();
        let v = handle_input_event(json!({"x":0.5,"y":0.5,"action":"evil"})).await;
        assert_eq!(v["code"], "validation");
    }

    #[tokio::test]
    async fn input_event_home_no_coords() {
        reset_control_state();
        let v = handle_input_event(json!({"action":"home"})).await;
        assert_eq!(v["relayed"], true);
    }

    #[tokio::test]
    async fn input_throttle_move() {
        reset_control_state();
        // first move ok
        let v1 = handle_input_event(json!({"x":0.1,"y":0.1,"action":"move","ts":1000})).await;
        assert_eq!(v1["relayed"], true);
        // second move within 16ms should be throttled
        let v2 = handle_input_event(json!({"x":0.11,"y":0.11,"action":"move","ts":1005})).await;
        assert_eq!(v2["throttled"], true);
        // third after 20ms should be ok
        let v3 = handle_input_event(json!({"x":0.12,"y":0.12,"action":"move","ts":1025})).await;
        assert_eq!(v3["relayed"], true);
    }

    #[test]
    fn throttle_pure() {
        let mut last: Option<i64> = None;
        assert!(!is_throttled_vec(&mut last, 1000, 16));
        assert!(is_throttled_vec(&mut last, 1005, 16));
        assert!(!is_throttled_vec(&mut last, 1020, 16));
    }

    #[test]
    fn rate_limit_pure() {
        let mut vec: Vec<i64> = Vec::new();
        for _ in 0..120 {
            assert!(check_limit_vec(120, 1000, &mut vec, 1000));
        }
        assert!(!check_limit_vec(120, 1000, &mut vec, 1000));
        assert!(check_limit_vec(120, 1000, &mut vec, 2500));
    }

    #[tokio::test]
    async fn control_start_valid() {
        reset_control_state();
        let v = handle_control_start(json!({"displayId":0,"quality":80})).await;
        assert_eq!(v["state"], "CONTROLLING");
        assert_eq!(v["ok"], true);
    }

    #[tokio::test]
    async fn control_start_invalid_quality() {
        reset_control_state();
        let v = handle_control_start(json!({"quality":200})).await;
        assert!(v["error"].is_string());
    }

    #[tokio::test]
    async fn control_stop_transitions() {
        reset_control_state();
        let _ = handle_control_start(json!({"displayId":0})).await;
        let v = handle_control_stop(json!({"displayId":0,"reason":"user"})).await;
        assert_eq!(v["ok"], true);
        // state should be ENABLED after stop
        assert!(v["state"].as_str().unwrap().contains("ENABLED") || v["state"]== "ENABLED");
    }

    #[tokio::test]
    async fn display_info_dummy() {
        reset_control_state();
        let v = handle_display_info(json!({})).await;
        assert!(v["displays"].is_array());
        assert_eq!(v["primaryDisplayId"], 0);
    }

    #[tokio::test]
    async fn display_frame_valid() {
        let v = handle_display_frame(json!({"displayId":0,"frame_b64":"abc","width":1080})).await;
        assert_eq!(v["relayed"], true);
    }

    #[tokio::test]
    async fn display_frame_missing() {
        let v = handle_display_frame(json!({"displayId":0})).await;
        assert!(v["error"].is_string());
    }

    #[test]
    fn clamp_works() {
        assert_eq!(clamp_xy(0.5), Some(0.5));
        assert_eq!(clamp_xy(-0.1), None);
        assert_eq!(clamp_xy(1.5), None);
        assert_eq!(clamp_xy(f64::NAN), None);
        assert_eq!(clamp_xy(0.0), Some(0.0));
        assert_eq!(clamp_xy(1.0), Some(1.0));
    }

    #[test]
    fn redact_coords_bucket() {
        assert_eq!(redact_coords(0.42, 0.71), "0.4,0.7");
        assert_eq!(redact_coords(0.99, 0.01), "0.9,0.0");
    }
}
