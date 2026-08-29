use bridge_core::{BridgeMessage, MessageType, RelayState, is_valid_device_id, is_valid_stun_server, validate_relay_announce_payload, validate_relay_relay_payload};
use serde_json::json;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

fn b64_opaque(n: usize) -> String { B64.encode(vec![0x42u8; n]) }

#[test]
fn relay_announce_serde() {
    let m = BridgeMessage::new(MessageType::RelayAnnounce, json!({"deviceId":"linux-abc","blob":b64_opaque(64)}));
    let j = m.to_json();
    assert!(j.contains("relay.announce"));
    assert_eq!(BridgeMessage::from_json(&j).unwrap().typ, MessageType::RelayAnnounce);
}
#[test]
fn relay_relay_serde() {
    let m = BridgeMessage::new(MessageType::RelayRelay, json!({"to":"phone","from":"desktop","blob":b64_opaque(64)}));
    assert!(m.to_json().contains("relay.relay"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::RelayRelay);
}
#[test]
fn mesh_sync_serde() {
    let m = BridgeMessage::new(MessageType::MeshSync, json!({"deviceId":"phone","vectors":{"phone":1}}));
    assert!(m.to_json().contains("mesh.sync"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::MeshSync);
}
#[test]
fn mesh_conflict_serde() {
    let m = BridgeMessage::new(MessageType::MeshConflict, json!({"path":"/a","resolution":"lww","winner":"local"}));
    assert!(m.to_json().contains("mesh.conflict"));
    assert_eq!(BridgeMessage::from_json(&m.to_json()).unwrap().typ, MessageType::MeshConflict);
}
#[test]
fn relay_state_valid() {
    assert!(RelayState::Disconnected.can_transition(&RelayState::Announcing));
    assert!(RelayState::Announcing.can_transition(&RelayState::HolePunching));
    assert!(RelayState::Announcing.can_transition(&RelayState::RelayReady));
    assert!(RelayState::HolePunching.can_transition(&RelayState::ConnectedDirect));
    assert!(RelayState::HolePunching.can_transition(&RelayState::RelayReady));
    assert!(RelayState::RelayReady.can_transition(&RelayState::ConnectedViaRelay));
}
#[test]
fn relay_state_invalid() {
    assert!(!RelayState::Disconnected.can_transition(&RelayState::ConnectedDirect));
    assert!(!RelayState::ConnectedDirect.can_transition(&RelayState::Announcing));
    assert!(!RelayState::Failed.can_transition(&RelayState::Announcing));
}
#[test]
fn is_valid_device_id_ok() {
    assert!(is_valid_device_id("linux-abc-123"));
    assert!(is_valid_device_id("phone_xyz"));
    assert!(!is_valid_device_id(""));
    assert!(!is_valid_device_id("bad/id"));
    assert!(!is_valid_device_id(&"a".repeat(65)));
}
#[test]
fn is_valid_stun_server_ok() {
    assert!(is_valid_stun_server("stun.l.google.com:19302"));
    assert!(!is_valid_stun_server("bad"));
    assert!(!is_valid_stun_server("host:99999"));
}
#[test]
fn validate_relay_announce_ok() {
    let blob = b64_opaque(64);
    let p = json!({"deviceId":"linux-abc-123","blob":blob,"ts":chrono::Utc::now().timestamp_millis(),"fp":"aabbcc112233","mappedAddr":"1.2.3.4:5678","stunServer":"stun.l.google.com:19302","nonce":"aabbccdd"});
    assert!(validate_relay_announce_payload(&p).is_ok());
}
#[test]
fn validate_relay_announce_bad_device() {
    let blob = b64_opaque(64);
    let p = json!({"deviceId":"","blob":blob});
    assert!(validate_relay_announce_payload(&p).is_err());
}
#[test]
fn validate_relay_announce_bad_blob_not_opaque() {
    let p = json!({"deviceId":"linux-abc","blob":"{\"plain\":1}"});
    assert!(validate_relay_announce_payload(&p).is_err());
}
#[test]
fn validate_relay_relay_ok() {
    let blob = b64_opaque(64);
    let p = json!({"to":"phone-xyz","from":"linux-abc","blob":blob,"ts":chrono::Utc::now().timestamp_millis(),"nonce":"11223344"});
    assert!(validate_relay_relay_payload(&p).is_ok());
}
#[test]
fn validate_relay_relay_invalid() {
    let blob = b64_opaque(64);
    let p = json!({"to":"","from":"linux-abc","blob":blob});
    assert!(validate_relay_relay_payload(&p).is_err());
}
#[test]
fn stun_encode_length() {
    // just ensure helper not panic; relay encode via bridge-daemon but core has constants
    assert_eq!(bridge_core::STUN_SERVER, "stun.l.google.com:19302");
    assert_eq!(bridge_core::RELAY_ANNOUNCE_URL, "https://relay.bridge.dev/v1/announce");
}
