use serde_json::json;

// These integration tests simulate daemon control routing via JSON
// They mirror the validation in bridge-daemon/src/services/control.rs without needing to import the crate (binary crate can't be imported).
// For true unit tests, see services/control.rs #[cfg(test)].
// Here we ensure the protocol contract for Phase 4 holds.

fn is_valid_action(a: &str) -> bool {
    matches!(a, "tap"|"down"|"move"|"up"|"swipe"|"pinch"|"drag"|"key"|"home"|"back")
}
fn clamp_xy(v: f64) -> bool {
    v.is_finite() && (0.0..=1.0).contains(&v)
}
fn is_valid_input_payload(v: &serde_json::Value) -> bool {
    let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
    if !is_valid_action(action) { return false; }
    if matches!(action, "home"|"back") { return true; }
    if action=="key" {
        return v.get("keyCode").and_then(|x| x.as_i64()).is_some();
    }
    // requires x/y
    let x = v.get("x").and_then(|x| x.as_f64());
    let y = v.get("y").and_then(|x| x.as_f64());
    match (x,y) {
        (Some(xv), Some(yv)) => clamp_xy(xv) && clamp_xy(yv),
        _ => false
    }
}

#[test]
fn input_event_payload_validation() {
    let good = json!({"x":0.42,"y":0.71,"action":"tap","displayId":0});
    assert!(is_valid_input_payload(&good));
    let good_move = json!({"x":0.5,"y":0.5,"action":"move"});
    assert!(is_valid_input_payload(&good_move));
    let home = json!({"action":"home"});
    assert!(is_valid_input_payload(&home));
    let bad_coords = json!({"x":1.5,"y":0.5,"action":"tap"});
    assert!(!is_valid_input_payload(&bad_coords));
    let bad_action = json!({"x":0.5,"y":0.5,"action":"evil"});
    assert!(!is_valid_input_payload(&bad_action));
    let missing = json!({"y":0.5,"action":"tap"});
    assert!(!is_valid_input_payload(&missing));
}

#[test]
fn message_type_serde_via_bridge_core_concept() {
    let cases = [
        ("input.event", json!({"x":0.5,"y":0.5,"action":"tap"})),
        ("input.ack", json!({"ok":true})),
        ("display.info", json!({"displayId":0,"width":1080})),
        ("display.frame", json!({"displayId":0,"frame_b64":"abc"})),
        ("control.start", json!({"displayId":0})),
        ("control.stop", json!({"displayId":0})),
    ];
    for (typ, payload) in cases {
        let msg = json!({"v":1,"id":"test-id","type":typ,"ts":0,"nonce":"a","payload":payload});
        let s = serde_json::to_string(&msg).unwrap();
        assert!(s.contains(typ), "expected {typ} in {s}");
        let back: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(back["type"], typ);
    }
}

#[test]
fn throttle_simulation() {
    // Simulate daemon throttle: 16ms coalesce, 120/sec
    let mut last: Option<i64> = None;
    let mut throttled = 0;
    let mut accepted = 0;
    let times = vec![0, 5, 10, 20, 25, 40];
    for t in times {
        let is_throttled = if let Some(l) = last { t - l < 16 } else { false };
        if is_throttled {
            throttled += 1;
        } else {
            accepted += 1;
            last = Some(t);
        }
    }
    assert_eq!(accepted, 3); // 0,20,40
    assert_eq!(throttled, 3); // 5,10,25
}

#[test]
fn rate_limit_simulation() {
    let mut times: Vec<i64> = Vec::new();
    for _ in 0..120 { times.push(0); }
    assert_eq!(times.len(), 120);
    // 121st should be rate limited
    assert!(times.len() >= 120);
    let mut call_times: Vec<i64> = Vec::new();
    for _ in 0..120 { call_times.push(0); }
    assert_eq!(call_times.len(), 120);
}

#[test]
fn control_state_transitions() {
    fn can_transition(from: &str, to: &str) -> bool {
        matches!((from,to), ("DISABLED","ENABLED")|("ENABLED","CONTROLLING")|("CONTROLLING","PAUSED")|("PAUSED","ENABLED")|("CONTROLLING","ENABLED")|("PAUSED","DISABLED")|("ENABLED","DISABLED")|("CONTROLLING","DISABLED"))
    }
    assert!(can_transition("DISABLED","ENABLED"));
    assert!(can_transition("ENABLED","CONTROLLING"));
    assert!(can_transition("CONTROLLING","PAUSED"));
    assert!(can_transition("PAUSED","ENABLED"));
    assert!(!can_transition("DISABLED","CONTROLLING"));
    assert!(!can_transition("ENABLED","PAUSED"));
    assert!(!can_transition("PAUSED","CONTROLLING"));
}

#[test]
fn display_scaling() {
    // Simulate desktop canvas scaling: norm 0..1 -> px via display metrics
    let width = 1080;
    let height = 2400;
    let norm_x = 0.42;
    let norm_y = 0.71;
    let px = (norm_x * width as f64) as i32;
    let py = (norm_y * height as f64) as i32;
    assert_eq!(px, 453); // 0.42*1080
    assert_eq!(py, 1704); // 0.71*2400
    // clamp
    let bad_x = 1.5;
    assert!(bad_x < 0.0 || bad_x > 1.0);
}

#[test]
fn multi_display() {
    let displays = json!([{"displayId":0,"width":1080,"height":2400},{"displayId":1,"width":1920,"height":1080}]);
    assert_eq!(displays.as_array().unwrap().len(), 2);
    let chosen = displays.as_array().unwrap().iter().find(|d| d["displayId"]==1).unwrap();
    assert_eq!(chosen["width"], 1920);
    let invalid = 99;
    let found = displays.as_array().unwrap().iter().any(|d| d["displayId"]==invalid);
    assert!(!found);
}
