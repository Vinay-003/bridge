use bridge_core::{BridgeMessage, MessageType, ControlState, is_valid_input_action, validate_input_event_payload};
use serde_json::json;

#[test]
fn input_event_serde() {
    let m = BridgeMessage::new(MessageType::InputEvent, json!({"x":0.5,"y":0.5,"action":"tap","displayId":0}));
    let j = m.to_json();
    assert!(j.contains("input.event"), "expected input.event in {j}");
    let back = BridgeMessage::from_json(&j).unwrap();
    assert_eq!(back.typ, MessageType::InputEvent);
}

#[test]
fn input_ack_serde() {
    let m = BridgeMessage::new(MessageType::InputAck, json!({"ok":true,"latencyMs":12}));
    assert!(m.to_json().contains("input.ack"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::InputAck);
}

#[test]
fn display_info_serde() {
    let m = BridgeMessage::new(MessageType::DisplayInfo, json!({"displayId":0,"width":1080}));
    assert!(m.to_json().contains("display.info"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::DisplayInfo);
}

#[test]
fn display_frame_serde() {
    let m = BridgeMessage::new(MessageType::DisplayFrame, json!({"displayId":0,"frame_b64":"abc"}));
    assert!(m.to_json().contains("display.frame"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::DisplayFrame);
}

#[test]
fn control_start_serde() {
    let m = BridgeMessage::new(MessageType::ControlStart, json!({"displayId":0}));
    assert!(m.to_json().contains("control.start"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::ControlStart);
}

#[test]
fn control_stop_serde() {
    let m = BridgeMessage::new(MessageType::ControlStop, json!({"displayId":0}));
    assert!(m.to_json().contains("control.stop"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::ControlStop);
}

#[test]
fn valid_actions() {
    assert!(is_valid_input_action("tap"));
    assert!(is_valid_input_action("down"));
    assert!(is_valid_input_action("move"));
    assert!(is_valid_input_action("up"));
    assert!(is_valid_input_action("swipe"));
    assert!(is_valid_input_action("pinch"));
    assert!(is_valid_input_action("drag"));
    assert!(is_valid_input_action("key"));
    assert!(is_valid_input_action("home"));
    assert!(is_valid_input_action("back"));
    assert!(!is_valid_input_action("evil"));
    assert!(!is_valid_input_action(""));
}

#[test]
fn validate_input_event_ok() {
    let payload = json!({"x":0.42,"y":0.71,"action":"move","displayId":0});
    assert!(validate_input_event_payload(&payload).is_ok());
}

#[test]
fn validate_input_event_tap_ok() {
    let payload = json!({"x":0.5,"y":0.5,"action":"tap","displayId":0,"pressure":0.5});
    assert!(validate_input_event_payload(&payload).is_ok());
}

#[test]
fn validate_input_event_invalid_coords() {
    let payload = json!({"x":1.5,"y":0.5,"action":"tap"});
    assert!(validate_input_event_payload(&payload).is_err());
    let payload2 = json!({"x":-0.1,"y":0.5,"action":"tap"});
    assert!(validate_input_event_payload(&payload2).is_err());
    let payload3 = json!({"x":0.5,"y":f64::NAN,"action":"tap"});
    // NaN will be serialized as null in json, but we test via raw Value with string?
    // Instead test missing x
    let payload4 = json!({"y":0.5,"action":"tap"});
    assert!(validate_input_event_payload(&payload4).is_err());
}

#[test]
fn validate_input_event_invalid_action() {
    let payload = json!({"x":0.5,"y":0.5,"action":"evil"});
    assert!(validate_input_event_payload(&payload).is_err());
}

#[test]
fn validate_input_event_home_no_coords() {
    // home/back don't require x/y
    let payload = json!({"action":"home"});
    assert!(validate_input_event_payload(&payload).is_ok());
    let payload2 = json!({"action":"back"});
    assert!(validate_input_event_payload(&payload2).is_ok());
}

#[test]
fn validate_input_event_key_requires_keycode() {
    let payload = json!({"action":"key","keyCode":4});
    assert!(validate_input_event_payload(&payload).is_ok());
    let payload2 = json!({"action":"key"});
    assert!(validate_input_event_payload(&payload2).is_err());
}

#[test]
fn validate_input_event_pinch_scale() {
    let payload = json!({"x":0.5,"y":0.5,"action":"pinch","scale":1.2});
    assert!(validate_input_event_payload(&payload).is_ok());
    let bad = json!({"x":0.5,"y":0.5,"action":"pinch","scale":10.0});
    assert!(validate_input_event_payload(&bad).is_err());
}

#[test]
fn control_state_machine_valid() {
    assert!(ControlState::Disabled.can_transition(&ControlState::Enabled));
    assert!(ControlState::Enabled.can_transition(&ControlState::Controlling));
    assert!(ControlState::Controlling.can_transition(&ControlState::Paused));
    assert!(ControlState::Paused.can_transition(&ControlState::Enabled));
    assert!(ControlState::Controlling.can_transition(&ControlState::Enabled));
    assert!(ControlState::Paused.can_transition(&ControlState::Disabled));
    assert!(ControlState::Enabled.can_transition(&ControlState::Disabled));
}

#[test]
fn control_state_machine_invalid() {
    assert!(!ControlState::Disabled.can_transition(&ControlState::Controlling));
    assert!(!ControlState::Disabled.can_transition(&ControlState::Paused));
    // CONTROLLING -> DISABLED is allowed (toggle OFF mid-session) per spec, so not tested as invalid
    assert!(!ControlState::Enabled.can_transition(&ControlState::Paused)); // ENABLED -> PAUSED must via CONTROLLING
    assert!(!ControlState::Paused.can_transition(&ControlState::Controlling));
    assert!(!ControlState::Disabled.can_transition(&ControlState::Disabled));
    assert!(!ControlState::Enabled.can_transition(&ControlState::Enabled));
}
