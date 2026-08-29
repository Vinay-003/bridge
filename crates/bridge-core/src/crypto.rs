use p256::{ecdh::EphemeralSecret, PublicKey, EncodedPoint};
use p256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use rand::rngs::OsRng;
use hkdf::Hkdf;
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose::STANDARD as b64, Engine as _};

pub struct KeyPair {
    pub secret: EphemeralSecret,
    pub public_b64: String,
}

pub fn generate_keypair() -> KeyPair {
    let secret = EphemeralSecret::random(&mut OsRng);
    let pubkey = secret.public_key();
    let encoded = pubkey.to_encoded_point(false);
    let public_b64 = b64.encode(encoded.as_bytes());
    KeyPair { secret, public_b64 }
}

pub fn decode_pubkey(b64_str: &str) -> Option<PublicKey> {
    let bytes = b64.decode(b64_str).ok()?;
    let point = EncodedPoint::from_bytes(&bytes).ok()?;
    PublicKey::from_encoded_point(&point).into()
}

pub fn derive_shared(secret: EphemeralSecret, peer_b64: &str) -> Option<Vec<u8>> {
    let peer = decode_pubkey(peer_b64)?;
    let shared = secret.diffie_hellman(&peer);
    let hk = Hkdf::<Sha256>::new(None, shared.raw_secret_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"bridge-v1", &mut okm).ok()?;
    Some(okm.to_vec())
}

pub fn fingerprint(pub_b64: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pub_b64.as_bytes());
    let out = hasher.finalize();
    hex::encode(&out[..6])
}

pub fn sas_from_secret(secret_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret_bytes);
    hasher.update(b"sas");
    let out = hasher.finalize();
    let num = ((out[0] as u32) << 16) | ((out[1] as u32) << 8) | out[2] as u32;
    format!("{:06}", num % 1_000_000)
}
