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
    pub code: String,
    pub fingerprint: String,
}

pub fn pairing_qr_payload(id: &str, ecdh_pub: &str, fp: &str, port: u16) -> String {
    format!("bridge://pair?v=1&id={}&ecdh={}&fp={}&port={}", id, urlencoding::encode(ecdh_pub), fp, port)
}

pub fn pairing_qr_payload_with_host(id: &str, host: &str, ecdh_pub: &str, fp: &str, port: u16) -> String {
    format!("bridge://pair?v=1&id={}&host={}&ecdh={}&fp={}&port={}", id, host, urlencoding::encode(ecdh_pub), fp, port)
}

pub fn parse_qr_payload(qr: &str) -> Option<(String,String,String,String,u16)> {
    // returns (id, host, ecdh, fp, port)
    if !qr.starts_with("bridge://pair") { return None }
    let query = qr.split('?').nth(1)?;
    let mut id=None; let mut host=None; let mut ecdh=None; let mut fp=None; let mut port=None;
    for kv in query.split('&') {
        let mut p = kv.splitn(2, '=');
        let k = p.next()?; let v = p.next()?;
        match k {
            "id" => id=Some(v.to_string()),
            "host" => host=Some(v.to_string()),
            "ecdh" => ecdh=Some(urlencoding::decode(v)),
            "fp" => fp=Some(v.to_string()),
            "port" => port=v.parse().ok(),
            _ => {}
        }
    }
    Some((id?, host.unwrap_or_else(|| "192.168.1.36".into()), ecdh?, fp?, port.unwrap_or(8443)))
}

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
    pub fn decode(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c=='%' {
                let h1 = chars.next().unwrap_or('0');
                let h2 = chars.next().unwrap_or('0');
                let hex = format!("{}{}", h1,h2);
                if let Ok(b) = u8::from_str_radix(&hex,16) { out.push(b as char); } else { out.push('%'); out.push(h1); out.push(h2); }
            } else if c=='+' { out.push(' '); } else { out.push(c); }
        }
        out
    }
}
