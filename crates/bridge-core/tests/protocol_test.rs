use bridge_core::{BridgeMessage, MessageType, chunk_file};
use serde_json::json;
#[test]
fn message_roundtrip() {
    let m = BridgeMessage::new(MessageType::Ping, json!({"ok":true}));
    let j = m.to_json();
    let back = BridgeMessage::from_json(&j).unwrap();
    assert_eq!(back.typ, MessageType::Ping);
}
#[test]
fn chunk_verify() {
    let data = vec![0u8; 3_000_000];
    let chunks = chunk_file("id1", "big.bin", &data);
    assert_eq!(chunks.len(), 3);
    for c in chunks { assert!(bridge_core::file::verify_chunk(&c)); }
}
#[test]
fn pairing_qr_payload_contains_fields() {
    let p = bridge_core::pairing::pairing_qr_payload("abc", "pub==", "fp123", 8443);
    assert!(p.starts_with("bridge://pair"));
    assert!(p.contains("fp=fp123"));
}
#[test]
fn pairing_with_host_contains_host() {
    let p = bridge_core::pairing::pairing_qr_payload_with_host("id1", "192.168.1.36", "pub", "fp", 8443);
    assert!(p.contains("host=192.168.1.36"));
    assert!(p.contains("id=id1"));
}
#[test]
fn parse_qr_roundtrip() {
    let orig = bridge_core::pairing::pairing_qr_payload_with_host("abc", "10.0.0.5", "my+pub==", "fp12", 8443);
    let parsed = bridge_core::pairing::parse_qr_payload(&orig).unwrap();
    assert_eq!(parsed.0, "abc");
    assert_eq!(parsed.1, "10.0.0.5");
    assert_eq!(parsed.3, "fp12");
}
#[test]
fn notify_message_type_serde() {
    let m = BridgeMessage::new(MessageType::NotifyNew, json!({"key":"k","app":"WhatsApp","title":"hi"}));
    let j = m.to_json();
    assert!(j.contains("notify.new"));
    let back = BridgeMessage::from_json(&j).unwrap();
    assert_eq!(back.typ, MessageType::NotifyNew);
}
#[test]
fn clipboard_message_type_serde() {
    let m = BridgeMessage::new(MessageType::ClipboardSync, json!({"mime":"text/plain","data_b64":"aGVsbG8="}));
    let j = m.to_json();
    assert!(j.contains("clipboard.sync"));
}
