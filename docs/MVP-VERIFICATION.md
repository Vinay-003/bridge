# MVP Verification — 2026-08-29

## Build gates (verification-loop)

- `cargo test -p bridge-core` → 3/3 ok (message_roundtrip, chunk_verify, pairing_qr_payload)
- `cargo check` → Finished dev profile (12 warnings, zero errors)
- `pnpm --filter desktop exec tsc --noEmit` → ok
- `pnpm --filter desktop build` → vite 5.4.21 ✓ 53 modules, 178k js (57k gzip)
- `grep -R "sk-"` / `api_key` → none, no hardcoded secrets

## Runtime gates

- `cargo run -p bridge-daemon -- --port 18443` → WS 0.0.0.0:18443, mDNS _bridge._tcp registered, QR payload generated
- `ws://127.0.0.1:18443` heartbeat.ping → pong ok
- `file.chunk` b64 → ~/Bridge/hello.txt written correctly (16 bytes, verified)
- `clipboard.sync` → ingested, no error
- `webrtc.offer` → webrtc.answer echo + status.push every 5s (battery/ram/storage/signal) verified
- `v4l2loopback` → /dev/video10 present (0.12.7), PipeWire 1.0.5 active
- `tauri-cli 2.11.4` + `cargo-watch 8.5.3` installed, Rust 1.98 stable

## NFR spot checks

- discovery <2s (mDNS immediate), pairing <10s path ready (QR+SAS), WebRTC signalling <100ms local

## Next hardening

- Add TLS 1.3 rustls pinned cert (rcgen ready), QUIC bulk on 8444/udp, per-feature keyring perms, Playwright E2E POM, Android Espresso
