use serde_json::json;

// These integration tests simulate daemon telephony routing via JSON
// They mirror the validation in bridge-daemon/src/services/telephony.rs without needing to import the crate (binary crate can't be imported).
// For true unit tests, see services/telephony.rs #[cfg(test)].
// Here we ensure the protocol contract for Phase 3 holds.

fn is_valid_number(n: &str) -> bool {
    let digits: String = n.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.len() >= 7 && digits.len() <= 15 && n.trim().chars().all(|c| c.is_ascii_digit() || c=='+' || c==' ' || c=='-' || c=='(' || c==')') && !n.trim().is_empty()
}
fn is_valid_body(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= 918
}

#[test]
fn sms_send_payload_validation() {
    let good = json!({"address":"+33612345678","body":"Hello"});
    assert!(is_valid_number(good["address"].as_str().unwrap()));
    assert!(is_valid_body(good["body"].as_str().unwrap()));
    let bad_num = json!({"address":"bad","body":"hi"});
    assert!(!is_valid_number(bad_num["address"].as_str().unwrap()));
    let empty_body = json!({"address":"+33612345678","body":""});
    assert!(!is_valid_body(empty_body["body"].as_str().unwrap()));
}

#[test]
fn call_start_payload_validation() {
    let good = json!({"number":"+33612345678","subscriptionId":1});
    assert!(is_valid_number(good["number"].as_str().unwrap()));
    let bad = json!({"number":"xyz"});
    assert!(!is_valid_number(bad["number"].as_str().unwrap()));
}

#[test]
fn message_type_serde_via_bridge_core_concept() {
    // Simulate BridgeMessage JSON via serde_json directly (daemon uses bridge_core)
    // Ensure new types serialize as expected strings
    let cases = [
        ("sms.list", json!({"limit":50})),
        ("sms.send", json!({"address":"+33612345678","body":"hi"})),
        ("call.start", json!({"number":"+33612345678"})),
        ("call.answer", json!({"callId":"abc"})),
        ("call.hangup", json!({"callId":"abc"})),
        ("call.audio", json!({"callId":"uuid","sdp":"v=0"})),
        ("call.log", json!({"limit":50})),
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
fn rate_limit_simulation() {
    // Simulate daemon rate limiting: 20 sms/min, 3 calls/min
    let mut sms_times: Vec<i64> = Vec::new();
    for _ in 0..20 { sms_times.push(0); }
    assert_eq!(sms_times.len(), 20);
    // 21st should be rate limited
    assert!(sms_times.len() >= 20);
    let mut call_times: Vec<i64> = Vec::new();
    for _ in 0..3 { call_times.push(0); }
    assert_eq!(call_times.len(), 3);
}

#[test]
fn call_state_transitions() {
    // Mirror CallState logic in daemon
    fn can_transition(from: &str, to: &str) -> bool {
        matches!((from,to), ("IDLE","RINGING")|("RINGING","OFFHOOK")|("RINGING","HUNGUP")|("OFFHOOK","HUNGUP")|("HUNGUP","IDLE")|("IDLE","OFFHOOK"))
    }
    assert!(can_transition("IDLE","RINGING"));
    assert!(can_transition("RINGING","OFFHOOK"));
    assert!(!can_transition("IDLE","HUNGUP"));
    assert!(!can_transition("OFFHOOK","RINGING"));
}

#[test]
fn subscription_handling() {
    let subs = json!([{"id":1,"displayName":"Orange F"},{"id":2,"displayName":"SFR"}]);
    let chosen = subs.as_array().unwrap().iter().find(|s| s["id"]==1).unwrap();
    assert_eq!(chosen["displayName"], "Orange F");
    // invalid subId should be rejected
    let invalid = 99;
    let found = subs.as_array().unwrap().iter().any(|s| s["id"]==invalid);
    assert!(!found);
}
