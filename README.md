# Bridge — Linux ↔ Android Continuity (MVP)

LAN-first, E2E encrypted, open alternative to Phone Link / KDE Connect — pairing like iPhone↔Mac.

**MVP features**
- Secure discovery (mDNS `_bridge._tcp` + BLE) + QR ECDH-P256 SAS pairing + auto-reconnect (QUIC 0-RTT, WS)
- File sharing (1 MB chunks, SHA256, resume, drag&drop, `~/Bridge`)
- Clipboard sync (text/image LWW via `arboard`/Android ClipboardManager)
- Notifications mirror / reply / dismiss (NotificationListener → notify-rust)
- Camera → virtual webcam (`v4l2loopback` `/dev/video10`, WebRTC H.264)
- Mic + Speaker virtual devices (PipeWire `Bridge Mic`/`Bridge Speaker`, Opus, NS/AGC/EC)
- Screen mirror + record + screenshot (MediaProjection → WebRTC)
- Device status (battery/temp/RAM/storage/signal every 5s)

## Stack
- **Desktop:** Tauri v2 + Rust (Tokio, `mdns-sd`, `tokio-tungstenite`, `quinn`, `rustls`, `rcgen`) + React+TS+Vite+Tailwind+shadcn
- **Android:** Kotlin + Compose + CameraX + MediaProjection + NotificationListener + DataStore + OkHttp/WS
- **Shared:** `crates/bridge-core` (serde/ring-style crypto, chunker, protocol)

## Quick start
```bash
bash scripts/setup-linux.sh   # apt + v4l2loopback + rust
export ANDROID_HOME=~/Android/Sdk
pnpm install
cargo run -p bridge-daemon    # WS 8443 + status push
pnpm --filter desktop dev      # vite 1420 → then `cargo tauri dev` for full Tauri
# Android
cd apps/android && ./gradlew assembleDebug
```

## Docs
- `docs/ADR-001-tech-stack.md`, `docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `docs/SECURITY.md`, `docs/SETUP.md`, `docs/ROADMAP.md`

## Protocol
`bridge://pair?v=1&id&ecdh&fp&port=8443` → TLS 1.3 pinned → `BridgeMessage {v, id, type, ts, nonce, payload}` (see `docs/PROTOCOL.md`)

## Verification
`cargo test -p bridge-core` ✓ · `pnpm --filter desktop build` ✓ · `cargo check` ✓
