use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Mutex, OnceLock};
use tracing::{info, warn};
use bridge_core::{RelayState, validate_relay_announce_payload, validate_relay_relay_payload};

pub const RELAY_URL: &str = "https://relay.bridge.dev/v1/announce";
pub const STUN_SERVER: &str = "stun.l.google.com:19302";
const ANNOUNCE_RATE_LIMIT: usize = 20;
const RELAY_RATE_LIMIT: usize = 100;

static RELAY_STATE: OnceLock<Mutex<RelayState>> = OnceLock::new();
static ANNOUNCE_TIMESTAMPS: OnceLock<Mutex<Vec<i64>>> = OnceLock::new();
static RELAY_TIMESTAMPS: OnceLock<Mutex<Vec<i64>>> = OnceLock::new();
static SEEN_NONCES: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

fn now_ms() -> i64 { chrono::Utc::now().timestamp_millis() }

fn prune_and_check(limit: usize, window_ms: i64, store: &OnceLock<Mutex<Vec<i64>>>, now: i64) -> bool {
    let mut g = store.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    g.retain(|&t| now - t < window_ms);
    if g.len() >= limit { false } else { g.push(now); true }
}

pub fn check_announce_rate_limit() -> Result<(), String> {
    if prune_and_check(ANNOUNCE_RATE_LIMIT, 60_000, &ANNOUNCE_TIMESTAMPS, now_ms()) {
        Ok(())
    } else {
        Err("rate_limited: 20 relay.announce/min exceeded".into())
    }
}
pub fn check_relay_rate_limit() -> Result<(), String> {
    if prune_and_check(RELAY_RATE_LIMIT, 60_000, &RELAY_TIMESTAMPS, now_ms()) {
        Ok(())
    } else {
        Err("rate_limited: 100 relay.relay/min exceeded".into())
    }
}

pub fn is_replay_nonce(nonce: &str) -> bool {
    let now = now_ms();
    let mut g = SEEN_NONCES.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    // prune old >5min
    g.retain(|_, &mut ts| now - ts < 5 * 60 * 1000);
    if g.contains_key(nonce) { true } else { g.insert(nonce.to_string(), now); false }
}

pub fn relay_state() -> RelayState {
    RELAY_STATE.get_or_init(|| Mutex::new(RelayState::Disconnected)).lock().unwrap().clone()
}
pub fn set_relay_state(s: RelayState) {
    let lock = RELAY_STATE.get_or_init(|| Mutex::new(RelayState::Disconnected));
    *lock.lock().unwrap() = s;
}
pub fn try_transition_relay(to: RelayState) -> Result<(), String> {
    let lock = RELAY_STATE.get_or_init(|| Mutex::new(RelayState::Disconnected));
    let mut g = lock.lock().unwrap();
    if g.can_transition(&to) {
        info!(target:"audit", "relay transition {:?} -> {:?}", *g, to);
        *g = to; Ok(())
    } else {
        Err(format!("invalid relay transition {:?} -> {:?}", *g, to))
    }
}

pub fn relay_announce_url() -> &'static str { RELAY_URL }

pub fn is_opaque_blob_str(s: &str) -> bool {
    // opaque relay server sees only opaque, no plaintext check
    // must be base64 and not contain plaintext JSON markers unencrypted
    // simple: base64 and length 16..1.4M and not containing '{' or '"'
    if s.contains('{') || s.contains('"') { return false; }
    s.len() >= 16 && s.len() <= 1_400_000 && s.chars().all(|c| c.is_ascii_alphanumeric() || c=='+' || c=='/' || c=='=')
}

// ── STUN RFC5389 ───────────────────────────────────────────────────────────
pub fn encode_stun_binding_request(txid: [u8;12]) -> Vec<u8> {
    // Header 20 bytes: Type 0x0001, Length 0, Magic 0x2112A442, TxId 12
    let mut buf = Vec::with_capacity(20);
    buf.extend_from_slice(&0x0001u16.to_be_bytes());
    buf.extend_from_slice(&0u16.to_be_bytes());
    buf.extend_from_slice(&0x2112A442u32.to_be_bytes());
    buf.extend_from_slice(&txid);
    buf
}

pub fn decode_stun_mapped_address(resp: &[u8], txid: &[u8;12]) -> Option<SocketAddr> {
    if resp.len() < 20 { return None; }
    let typ = u16::from_be_bytes([resp[0], resp[1]]);
    if typ != 0x0101 { return None; } // Binding Success
    let magic = u32::from_be_bytes([resp[4], resp[5], resp[6], resp[7]]);
    if magic != 0x2112A442 { return None; }
    if &resp[8..20] != txid { return None; }
    let mut offset = 20;
    while offset + 4 <= resp.len() {
        let attr_type = u16::from_be_bytes([resp[offset], resp[offset+1]]);
        let attr_len = u16::from_be_bytes([resp[offset+2], resp[offset+3]]) as usize;
        let val_start = offset + 4;
        let val_end = val_start + attr_len;
        if val_end > resp.len() { break; }
        let val = &resp[val_start..val_end];
        // XOR-MAPPED-ADDRESS 0x0020 has xor, MAPPED-ADDRESS 0x0001 plain
        if attr_type == 0x0020 && val.len() >= 8 {
            // first byte reserved 0, second family, then xor port, xor ip
            let family = val[1];
            let xor_port = u16::from_be_bytes([val[2], val[3]]) ^ 0x2112;
            let port = xor_port;
            if family == 0x01 && val.len() >= 8 {
                // IPv4
                let ip_bytes = [val[4] ^ 0x21, val[5] ^ 0x12, val[6] ^ 0xA4, val[7] ^ 0x42];
                let ip = std::net::Ipv4Addr::from(ip_bytes);
                return Some(SocketAddr::new(ip.into(), port));
            } else if family == 0x02 && val.len() >= 20 {
                // IPv6: xor with magic+txid
                // For simplicity, not handling IPv6 fully in stub; return None
            }
        } else if attr_type == 0x0001 && val.len() >= 8 {
            let family = val[1];
            let port = u16::from_be_bytes([val[2], val[3]]);
            if family == 0x01 && val.len() >= 8 {
                let ip = std::net::Ipv4Addr::new(val[4], val[5], val[6], val[7]);
                return Some(SocketAddr::new(ip.into(), port));
            }
        }
        // pad to 4 bytes
        offset = val_end + (4 - attr_len % 4) % 4;
    }
    None
}

pub fn try_stun_hole_punch(stun_server: &str) -> Result<SocketAddr, String> {
    let server = stun_server.parse::<SocketAddr>()
        .or_else(|_| {
            // resolve via DNS? Try use ToSocketAddrs
            use std::net::ToSocketAddrs;
            let addrs: Vec<_> = stun_server.to_socket_addrs().map_err(|e| e.to_string())?.collect();
            addrs.into_iter().next().ok_or("no addr".to_string())
        })
        .map_err(|e| format!("stun parse fail {}: {}", stun_server, e))?;
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(std::time::Duration::from_secs(2))).map_err(|e| e.to_string())?;
    sock.set_write_timeout(Some(std::time::Duration::from_secs(2))).map_err(|e| e.to_string())?;
    let mut txid = [0u8;12];
    for b in txid.iter_mut() { *b = rand::random(); }
    let req = encode_stun_binding_request(txid);
    // 3 retries 500ms
    for attempt in 0..3 {
        sock.send_to(&req, server).map_err(|e| e.to_string())?;
        let mut buf = [0u8; 512];
        match sock.recv_from(&mut buf) {
            Ok((n, _src)) => {
                if let Some(mapped) = decode_stun_mapped_address(&buf[..n], &txid) {
                    info!("STUN mapped {} via {} attempt {}", mapped, stun_server, attempt);
                    return Ok(mapped);
                } else {
                    warn!("STUN decode failed attempt {}", attempt);
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                warn!("STUN timeout attempt {} {}", attempt, e);
                std::thread::sleep(std::time::Duration::from_millis(500));
                continue;
            },
            Err(e) => return Err(format!("STUN recv error: {}", e)),
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    Err("STUN hole punch failed after 3 attempts".into())
}

// QUIC relay client stub (quinn) — for TDD we validate URL and simulate handshake
pub fn try_quic_relay_connect(relay_url: &str) -> Result<String, String> {
    // Validate URL is https://relay.bridge.dev
    if !relay_url.starts_with("https://relay.bridge.dev") {
        return Err(format!("invalid relay url: {}", relay_url));
    }
    // In real, would create quinn::Endpoint::client + rustls pinned cert
    // Stub: simulate that relay is reachable if URL valid
    // We check env BRIDGE_RELAY_MOCK_FAIL to simulate failure for tests
    if std::env::var("BRIDGE_RELAY_MOCK_FAIL").is_ok() {
        return Err("mock quic relay fail".into());
    }
    info!("QUIC relay client stub connect to {}", relay_url);
    Ok(format!("quic-session-{}", &relay_url[8..12]))
}

// Handlers

pub async fn handle_relay_announce(payload: Value) -> Value {
    if let Err(e) = validate_relay_announce_payload(&payload) {
        warn!("relay.announce validation failed: {}", e);
        return json!({"error": e.to_string(), "code": "validation"});
    }
    if let Err(e) = check_announce_rate_limit() {
        return json!({"error": e, "code": "rate_limited", "retryAfterMs": 60000});
    }
    if let Some(nonce) = payload.get("nonce").and_then(|v| v.as_str()) {
        if is_replay_nonce(nonce) {
            return json!({"error": "replay nonce", "code": "replay"});
        }
    }
    let device_id = payload.get("deviceId").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let mapped = payload.get("mappedAddr").and_then(|v| v.as_str()).map(|s| s.to_string());
    info!(target:"audit", "relay.announce device={} mapped={:?} opaque_len={}", device_id, mapped, payload.get("blob").and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0));
    // Server sees only opaque, no plaintext — we assert blob is opaque
    let blob = payload.get("blob").and_then(|v| v.as_str()).unwrap_or("");
    if !is_opaque_blob_str(blob) {
        return json!({"error": "blob not opaque", "code": "validation"});
    }
    // State transition Disconnected->Announcing if needed
    let cur = relay_state();
    if cur == RelayState::Disconnected {
        let _ = try_transition_relay(RelayState::Announcing);
    }
    json!({
        "ok": true,
        "relayNonce": format!("{:08x}", rand::random::<u32>()),
        "stunHint": {"server": STUN_SERVER, "supportsPunch": true},
        "deviceId": device_id,
        "mappedAddr": mapped,
        "opaque": true,
        "note": "E2E via Noise, relay sees only opaque"
    })
}

pub async fn handle_relay_relay(payload: Value) -> Value {
    if let Err(e) = validate_relay_relay_payload(&payload) {
        warn!("relay.relay validation failed: {}", e);
        return json!({"error": e.to_string(), "code": "validation"});
    }
    if let Err(e) = check_relay_rate_limit() {
        return json!({"error": e, "code": "rate_limited", "retryAfterMs": 60000});
    }
    if let Some(nonce) = payload.get("nonce").and_then(|v| v.as_str()) {
        if is_replay_nonce(nonce) {
            return json!({"error": "replay nonce", "code": "replay"});
        }
    }
    let to = payload.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let from = payload.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let blob = payload.get("blob").and_then(|v| v.as_str()).unwrap_or("");
    if !is_opaque_blob_str(blob) {
        return json!({"error": "blob not opaque", "code": "validation"});
    }
    info!(target:"audit", "relay.relay from={} to={} opaque_len={} queued=false", from, to, blob.len());
    json!({"ok": true, "to": to, "from": from, "queued": false, "opaque": true})
}

// Reset for tests
pub fn reset_relay_state() {
    if let Some(m) = RELAY_STATE.get() { if let Ok(mut g) = m.lock() { *g = RelayState::Disconnected; } else if let Err(e) = m.lock() { *e.into_inner() = RelayState::Disconnected; } }
    if let Some(m) = ANNOUNCE_TIMESTAMPS.get() { if let Ok(mut g) = m.lock() { g.clear(); } else if let Err(e) = m.lock() { e.into_inner().clear(); } }
    if let Some(m) = RELAY_TIMESTAMPS.get() { if let Ok(mut g) = m.lock() { g.clear(); } else if let Err(e) = m.lock() { e.into_inner().clear(); } }
    if let Some(m) = SEEN_NONCES.get() { if let Ok(mut g) = m.lock() { g.clear(); } else if let Err(e) = m.lock() { e.into_inner().clear(); } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use base64::Engine as _;
    use std::sync::{OnceLock, Mutex};
    static RELAY_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    fn relay_lock() -> std::sync::MutexGuard<'static, ()> { RELAY_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner()) }

    fn b64_opaque(n: usize) -> String {
        let bytes = vec![0x42u8; n];
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn relay_url_is_correct() {
        assert_eq!(relay_announce_url(), "https://relay.bridge.dev/v1/announce");
        assert_eq!(RELAY_URL, "https://relay.bridge.dev/v1/announce");
    }
    #[test]
    fn stun_server_const() {
        assert_eq!(STUN_SERVER, "stun.l.google.com:19302");
        assert!(bridge_core::is_valid_stun_server(STUN_SERVER));
        assert!(!bridge_core::is_valid_stun_server("bad"));
        assert!(!bridge_core::is_valid_stun_server("host:99999"));
    }
    #[test]
    fn opaque_blob_check() {
        let ok = b64_opaque(64);
        assert!(is_opaque_blob_str(&ok));
        assert!(!is_opaque_blob_str("{\"plaintext\":1}"));
        assert!(!is_opaque_blob_str("short"));
    }
    #[test]
    fn stun_encode_decode_roundtrip() {
        let txid = [1u8,2,3,4,5,6,7,8,9,10,11,12];
        let req = encode_stun_binding_request(txid);
        assert_eq!(req.len(), 20);
        assert_eq!(u16::from_be_bytes([req[0], req[1]]), 0x0001);
        // fake response with XOR-MAPPED
        // Build response header + XOR-MAPPED-ADDRESS for 1.2.3.4:12345
        let mut resp = Vec::new();
        resp.extend_from_slice(&0x0101u16.to_be_bytes());
        resp.extend_from_slice(&8u16.to_be_bytes());
        resp.extend_from_slice(&0x2112A442u32.to_be_bytes());
        resp.extend_from_slice(&txid);
        // attr
        resp.extend_from_slice(&0x0020u16.to_be_bytes());
        resp.extend_from_slice(&8u16.to_be_bytes());
        resp.push(0); resp.push(0x01);
        let port = 12345u16 ^ 0x2112;
        resp.extend_from_slice(&port.to_be_bytes());
        let ip = [1u8,2,3,4];
        resp.push(ip[0] ^ 0x21); resp.push(ip[1] ^ 0x12); resp.push(ip[2] ^ 0xA4); resp.push(ip[3] ^ 0x42);
        let decoded = decode_stun_mapped_address(&resp, &txid).unwrap();
        assert_eq!(decoded.ip().to_string(), "1.2.3.4");
        assert_eq!(decoded.port(), 12345);
    }
    #[test]
    fn relay_state_transitions() {
        assert!(RelayState::Disconnected.can_transition(&RelayState::Announcing));
        assert!(RelayState::Announcing.can_transition(&RelayState::HolePunching));
        assert!(RelayState::HolePunching.can_transition(&RelayState::ConnectedDirect));
        assert!(RelayState::HolePunching.can_transition(&RelayState::RelayReady));
        assert!(RelayState::RelayReady.can_transition(&RelayState::ConnectedViaRelay));
        assert!(!RelayState::Disconnected.can_transition(&RelayState::ConnectedDirect));
        assert!(!RelayState::ConnectedDirect.can_transition(&RelayState::Announcing));
    }
    #[test]
    fn quic_relay_stub_validates_url() {
        assert!(try_quic_relay_connect("https://relay.bridge.dev/v1/announce").is_ok());
        assert!(try_quic_relay_connect("https://evil.com").is_err());
    }
    #[tokio::test]
    async fn relay_announce_valid() {
        let _g = relay_lock();
        reset_relay_state();
        let blob = b64_opaque(64);
        let payload = json!({"deviceId":"linux-abc-123","blob":blob,"ts":chrono::Utc::now().timestamp_millis(),"fp":"aabbcc112233","mappedAddr":"1.2.3.4:5678","stunServer":"stun.l.google.com:19302","nonce":"aabbccdd"});
        let resp = handle_relay_announce(payload).await;
        assert_eq!(resp["ok"], true);
        assert_eq!(resp["opaque"], true);
    }
    #[tokio::test]
    async fn relay_announce_invalid_device() {
        let _g = relay_lock();
        reset_relay_state();
        let blob = b64_opaque(64);
        let payload = json!({"deviceId":"","blob":blob});
        let resp = handle_relay_announce(payload).await;
        assert!(resp["error"].is_string());
        assert_eq!(resp["code"], "validation");
    }
    #[tokio::test]
    async fn relay_relay_valid_opaque() {
        let _g = relay_lock();
        reset_relay_state();
        let blob = b64_opaque(64);
        let payload = json!({"to":"phone-xyz","from":"linux-abc","blob":blob,"ts":chrono::Utc::now().timestamp_millis(),"nonce":"11223344"});
        let resp = handle_relay_relay(payload).await;
        assert_eq!(resp["ok"], true);
        assert_eq!(resp["queued"], false);
        assert_eq!(resp["opaque"], true);
    }
    #[tokio::test]
    async fn relay_relay_replay_detected() {
        let _g = relay_lock();
        reset_relay_state();
        let blob = b64_opaque(64);
        let payload = json!({"to":"phone-xyz","from":"linux-abc","blob":blob,"nonce":"aabbccdd","ts":chrono::Utc::now().timestamp_millis()});
        let r1 = handle_relay_relay(payload.clone()).await;
        assert_eq!(r1["ok"], true);
        let r2 = handle_relay_relay(payload).await;
        assert_eq!(r2["code"], "replay");
    }
    #[test]
    fn rate_limit_announce() {
        let _g = relay_lock();
        reset_relay_state();
        for _ in 0..20 { assert!(check_announce_rate_limit().is_ok()); }
        assert!(check_announce_rate_limit().is_err());
    }
    #[test]
    fn rate_limit_relay() {
        let _g = relay_lock();
        reset_relay_state();
        for _ in 0..100 { assert!(check_relay_rate_limit().is_ok()); }
        assert!(check_relay_rate_limit().is_err());
    }
}
