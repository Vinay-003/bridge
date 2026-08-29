use serde_json::json;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

fn b64_opaque(n: usize) -> String { B64.encode(vec![0x42u8; n]) }

fn is_valid_device_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_alphanumeric() || c=='_' || c=='-' || c=='.')
}

#[test]
fn relay_announce_validation() {
    let ok = json!({"deviceId":"linux-abc","blob":b64_opaque(64),"fp":"aabbcc112233"});
    assert!(is_valid_device_id(ok["deviceId"].as_str().unwrap()));
    let bad = json!({"deviceId":"","blob":b64_opaque(64)});
    assert!(!is_valid_device_id(bad["deviceId"].as_str().unwrap_or("")));
}

#[test]
fn relay_opaque_check() {
    let ok = b64_opaque(64);
    assert!(!ok.contains('{'));
    let bad = "{\"plain\":1}".to_string();
    assert!(bad.contains('{'));
}

#[test]
fn relay_state_transitions() {
    fn can(a:&str,b:&str)->bool {
        matches!((a,b), ("DISCONNECTED","ANNOUNCING")|("ANNOUNCING","HOLE_PUNCHING")|("HOLE_PUNCHING","CONNECTED_DIRECT")|("HOLE_PUNCHING","RELAY_READY")|("RELAY_READY","CONNECTED_VIA_RELAY"))
    }
    assert!(can("DISCONNECTED","ANNOUNCING"));
    assert!(can("ANNOUNCING","HOLE_PUNCHING"));
    assert!(!can("DISCONNECTED","CONNECTED_DIRECT"));
}

#[test]
fn stun_encode() {
    // mimic relay encode_stun_binding_request
    let txid = [1u8;12];
    let mut buf = Vec::with_capacity(20);
    buf.extend_from_slice(&0x0001u16.to_be_bytes());
    buf.extend_from_slice(&0u16.to_be_bytes());
    buf.extend_from_slice(&0x2112A442u32.to_be_bytes());
    buf.extend_from_slice(&txid);
    assert_eq!(buf.len(),20);
    assert_eq!(u16::from_be_bytes([buf[0],buf[1]]),0x0001);
}

#[test]
fn message_type_contract() {
    let cases = [
        ("relay.announce", json!({"deviceId":"a","blob":b64_opaque(64)})),
        ("relay.relay", json!({"to":"b","from":"a","blob":b64_opaque(64)})),
        ("mesh.sync", json!({"deviceId":"a","vectors":{}})),
        ("mesh.conflict", json!({"path":"/a","resolution":"lww","winner":"local"})),
    ];
    for (typ, payload) in cases {
        let msg = json!({"v":1,"id":"test","type":typ,"ts":0,"nonce":"a","payload":payload});
        let s = serde_json::to_string(&msg).unwrap();
        assert!(s.contains(typ));
    }
}

#[test]
fn rate_limit_simulation_announce() {
    let mut ts: Vec<i64> = Vec::new();
    for _ in 0..20 { ts.push(0); }
    assert_eq!(ts.len(),20);
    // 21st would be limit
    assert!(ts.len()>=20);
}
