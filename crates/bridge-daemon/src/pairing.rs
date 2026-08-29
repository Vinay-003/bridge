use bridge_core::{generate_keypair, fingerprint, sas_from_secret, pairing::pairing_qr_payload};
use uuid::Uuid;
use qrcode::QrCode;
use qrcode::render::svg;

pub struct PairingManager {
    device_id: String,
    pubkey_b64: String,
    fp: String,
    // keep secret only in mem for derivation; for demo we derive SAS from pubkey hash
    sas: String,
    port: u16,
}

impl PairingManager {
    pub fn new(port: u16) -> Self {
        let device_id = Uuid::new_v4().to_string();
        let kp = generate_keypair();
        let fp = fingerprint(&kp.public_b64);
        // for preview SAS we hash pubkey; real SAS from shared secret after ECDH
        let sas = sas_from_secret(kp.public_b64.as_bytes()); // placeholder
        Self { device_id, pubkey_b64: kp.public_b64, fp, sas, port }
    }
    pub fn device_id(&self) -> &str { &self.device_id }
    pub fn pubkey(&self) -> &str { &self.pubkey_b64 }
    pub fn fingerprint(&self) -> &str { &self.fp }
    pub fn sas_preview(&self) -> &str { &self.sas }
    pub fn qr_payload(&self) -> String {
        pairing_qr_payload(&self.device_id, &self.pubkey_b64, &self.fp, self.port)
    }
    pub fn qr_svg(&self) -> String {
        let code = QrCode::new(self.qr_payload()).unwrap();
        code.render::<svg::Color>().min_dimensions(200,200).build()
    }
    pub fn trust_key(&self, _peer_fp: &str) -> bool {
        // TODO keyring store
        true
    }
}
