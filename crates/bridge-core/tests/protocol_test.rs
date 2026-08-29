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
