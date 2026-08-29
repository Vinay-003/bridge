use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PairingState {
    Idle,
    QrShown,
    Scanned,
    SasVerify,
    Trusted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SasCode {
    pub code: String, // 6 digits
    pub fingerprint: String,
}

pub fn pairing_qr_payload(id: &str, ecdh_pub: &str, fp: &str, port: u16) -> String {
    format!("bridge://pair?v=1&id={}&ecdh={}&fp={}&port={}", id, urlencoding::encode(ecdh_pub), fp, port)
}

// lightweight urlencoding inline to avoid dep bloat for core
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut out = String::new();
        for b in s.bytes() {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{:02X}", b));
            }
        }
        out
    }
}
