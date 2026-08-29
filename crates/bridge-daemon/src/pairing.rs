use bridge_core::{generate_keypair, fingerprint, sas_from_secret, pairing::{pairing_qr_payload_with_host}};
use uuid::Uuid;
use qrcode::QrCode;
use qrcode::render::svg;
use if_addrs::get_if_addrs;

pub struct PairingManager {
    device_id: String,
    pubkey_b64: String,
    fp: String,
    sas: String,
    port: u16,
    host: String,
}

fn detect_host() -> String {
    if let Ok(addrs) = get_if_addrs() {
        for iface in &addrs {
            if iface.addr.is_loopback() { continue; }
            let ip = iface.ip();
            if ip.is_ipv4() {
                let s = ip.to_string();
                if s.starts_with("192.168.") || s.starts_with("10.") { return s; }
            }
        }
        for iface in &addrs {
            if !iface.addr.is_loopback() && iface.ip().is_ipv4() {
                return iface.ip().to_string();
            }
        }
    }
    "192.168.1.36".into()
}

impl PairingManager {
    pub fn new(port: u16) -> Self {
        let device_id = Uuid::new_v4().to_string();
        let kp = generate_keypair();
        let fp = fingerprint(&kp.public_b64);
        let sas = sas_from_secret(kp.public_b64.as_bytes());
        let host = detect_host();
        Self { device_id, pubkey_b64: kp.public_b64, fp, sas, port, host }
    }
    pub fn device_id(&self) -> &str { &self.device_id }
    pub fn fingerprint(&self) -> &str { &self.fp }
    pub fn sas_preview(&self) -> &str { &self.sas }
    pub fn host(&self) -> &str { &self.host }
    pub fn qr_payload(&self) -> String {
        pairing_qr_payload_with_host(&self.device_id, &self.host, &self.pubkey_b64, &self.fp, self.port)
    }
    pub fn qr_svg(&self) -> String {
        let code = QrCode::new(self.qr_payload()).unwrap();
        code.render::<svg::Color>().min_dimensions(200,200).build()
    }
}
