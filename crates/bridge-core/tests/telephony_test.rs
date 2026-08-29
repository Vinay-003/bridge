use bridge_core::{BridgeMessage, MessageType, CallState, is_valid_phone_number, is_valid_sms_body, validate_sms_send_payload, validate_call_start_payload};
use serde_json::json;

#[test]
fn sms_list_serde() {
    let m = BridgeMessage::new(MessageType::SmsList, json!({"limit":50}));
    let j = m.to_json();
    assert!(j.contains("sms.list"), "expected sms.list in {j}");
    let back = BridgeMessage::from_json(&j).unwrap();
    assert_eq!(back.typ, MessageType::SmsList);
}

#[test]
fn sms_send_serde() {
    let m = BridgeMessage::new(MessageType::SmsSend, json!({"address":"+33612345678","body":"Hello"}));
    let j = m.to_json();
    assert!(j.contains("sms.send"));
    let back = BridgeMessage::from_json(&j).unwrap();
    assert_eq!(back.typ, MessageType::SmsSend);
}

#[test]
fn sms_received_serde() {
    let m = BridgeMessage::new(MessageType::SmsReceived, json!({"address":"+33600000000","body":"hi"}));
    let j = m.to_json();
    assert!(j.contains("sms.received"));
    let back = BridgeMessage::from_json(&j).unwrap();
    assert_eq!(back.typ, MessageType::SmsReceived);
}

#[test]
fn call_start_serde() {
    let m = BridgeMessage::new(MessageType::CallStart, json!({"number":"+33612345678"}));
    let j = m.to_json();
    assert!(j.contains("call.start"));
    assert_eq!(BridgeMessage::from_json(&j).unwrap().typ, MessageType::CallStart);
}

#[test]
fn call_answer_serde() {
    let m = BridgeMessage::new(MessageType::CallAnswer, json!({"callId":"abc"}));
    assert!(m.to_json().contains("call.answer"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::CallAnswer);
}

#[test]
fn call_hangup_serde() {
    let m = BridgeMessage::new(MessageType::CallHangup, json!({"callId":"abc"}));
    assert!(m.to_json().contains("call.hangup"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::CallHangup);
}

#[test]
fn call_audio_serde() {
    let m = BridgeMessage::new(MessageType::CallAudio, json!({"callId":"uuid","sdp":"v=0..."}));
    assert!(m.to_json().contains("call.audio"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::CallAudio);
}

#[test]
fn call_log_serde() {
    let m = BridgeMessage::new(MessageType::CallLog, json!({"calls":[]}));
    assert!(m.to_json().contains("call.log"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::CallLog);
}

#[test]
fn phone_number_validation() {
    assert!(is_valid_phone_number("+33612345678"));
    assert!(is_valid_phone_number("0612345678"));
    assert!(is_valid_phone_number("+1 650 555 1234"));
    assert!(!is_valid_phone_number("123"));
    assert!(!is_valid_phone_number(""));
    assert!(!is_valid_phone_number("abc"));
    assert!(!is_valid_phone_number("+33-6-12-34-56-7890123456")); // too long
}

#[test]
fn sms_body_validation() {
    assert!(is_valid_sms_body("Hello"));
    assert!(!is_valid_sms_body(""));
    let long = "a".repeat(919);
    assert!(!is_valid_sms_body(&long));
    assert!(is_valid_sms_body(&"a".repeat(918)));
}

#[test]
fn validate_sms_send_ok() {
    let payload = json!({"address":"+33612345678","body":"Hello via Bridge"});
    assert!(validate_sms_send_payload(&payload).is_ok());
}

#[test]
fn validate_sms_send_bad_number() {
    let payload = json!({"address":"bad","body":"hi"});
    assert!(validate_sms_send_payload(&payload).is_err());
}

#[test]
fn validate_sms_send_empty_body() {
    let payload = json!({"address":"+33612345678","body":""});
    assert!(validate_sms_send_payload(&payload).is_err());
}

#[test]
fn validate_call_start_ok() {
    let payload = json!({"number":"+33612345678","subscriptionId":1});
    assert!(validate_call_start_payload(&payload).is_ok());
}

#[test]
fn validate_call_start_bad_number() {
    let payload = json!({"number":"xyz"});
    assert!(validate_call_start_payload(&payload).is_err());
}

#[test]
fn call_state_machine_valid_transitions() {
    assert!(CallState::Idle.can_transition(&CallState::Ringing));
    assert!(CallState::Ringing.can_transition(&CallState::Offhook));
    assert!(CallState::Ringing.can_transition(&CallState::Hungup));
    assert!(CallState::Offhook.can_transition(&CallState::Hungup));
    assert!(CallState::Hungup.can_transition(&CallState::Idle));
}

#[test]
fn call_state_machine_invalid_transitions() {
    assert!(!CallState::Idle.can_transition(&CallState::Hungup));
    assert!(!CallState::Offhook.can_transition(&CallState::Ringing));
    assert!(!CallState::Hungup.can_transition(&CallState::Ringing));
    assert!(!CallState::Idle.can_transition(&CallState::Idle));
    assert!(!CallState::Offhook.can_transition(&CallState::Offhook));
}

#[test]
fn unknown_type_fails() {
    let json_str = r#"{"v":1,"id":"x","type":"unknown.foo","ts":0,"nonce":"a","payload":{}}"#;
    let res = BridgeMessage::from_json(json_str);
    assert!(res.is_err(), "unknown type should fail deserialization");
}
