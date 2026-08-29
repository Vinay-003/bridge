# Security & Threat Model

## Adversary: LAN passive/active, stolen device, rogue app
- **LAN MITM:** TLS 1.3 pinned self-signed, ECDH + SAS confirm → no CA needed, no cloud
- **Stolen device:** Keystore hardware-backed, revocation per device, local-only by default
- **Rogue desktop app:** per-feature permissions stored per pairing, toggle requires user confirm

## Key lifecycle
- Generate P-256 keypair per device install
- ECDH shared secret → HKDF → TLS session keys, never leaves memory
- Persist trust: `keyring` (SecretService) / `EncryptedSharedPreferences` + `Keystore`
- Rotate: `Settings → Paired devices → Rotate keys` → new QR
- Revoke: delete pairing row + close WS/QUIC sessions

## Checklist (security-review skill)
- [ ] No hardcoded secrets — use `KEYRING_BACKEND`/`ENCRYPTED_PREFS`, env for dev
- [ ] Input validation via Zod/serde on every WS/QUIC message
- [ ] .env, *.pem, *.key in .gitignore, git history scanned (`gitleaks`)
- [ ] Per-feature permission toggles + audit log `~/.local/share/bridge/audit.log`
- [ ] Logging excludes payload b64, only ids/codes
- [ ] Rate limit: 10 pairing tries/min/IP, 100 chunks/sec

## Future
- WireGuard/relay optional, with additional Noise handshake
