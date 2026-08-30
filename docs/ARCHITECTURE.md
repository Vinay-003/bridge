# Architecture — Bridge

## C4 Level 1 — System Context
- **Android Phone (10+)** — FG Service, Camera/Mic, Notifications, Clipboard, Files
- **Linux Desktop (Ubuntu 24.04 MVP)** — Tauri UI + Rust Daemon, v4l2loopback, PipeWire
- Both on same LAN (mDNS/BLE), TLS 1.3 + QUIC + WebRTC, no cloud.

## C4 Level 2 — Containers
```
[Android FG Service] --TLS WS 8443--> [Desktop Daemon (Tokio)]
       |                                    |
  CameraX/MediaProjection              v4l2loopback (/dev/video10)
  NotificationListener                  PipeWire virtual mic/speaker
  ClipboardManager                      arboard Wayland/X11
  Files (SAF)                           QUIC file store ~/Bridge
       |                                    |
       +--WebRTC (SDP via WS)---------------+
```

## C4 Level 3 — Daemon Modules (Tokio tasks)
- `discovery` — mdns-sd advertise/search, btleplug scan
- `pairing` — QR gen, ECDH, cert fingerprint, SAS confirm, keyring
- `transport` — WS server (control) + Quinn QUIC (bulk)
- `services/` — `file`, `clipboard`, `notify`, `status`, `media` (webrtc)
- `state` — SQLite `~/.local/share/bridge/bridge.db` + `sled` fallback

## Data Flows
1. **Pair:** Desktop QR (ecdh pub + fp) → Android scan → WS connect → SAS 6-digit confirm → trust store → `PAIRED`
2. **File:** DragDrop → QUIC stream `file/put` → 1 MB chunks SHA256 ACK → progress event → resume via `offset`
3. **Clipboard:** `arboard` watch ↔ `ClipboardManager` FG watch → LWW with `timestamp/nonce` dedup
4. **Notify:** `NotificationListener` → WS `notify/new` → `notify-rust` desktop → reply/dismiss via `notify/action`
5. **Media:** WS `webrtc/offer` ↔ SDP/ICE → WebRTC Opus/H264 → PipeWire/v4l2 bridge

## State Machines
- **Connection:** `DISCONNECTED --mdns found--> CONNECTING --WS ok--> PAIRED --keys ok--> CONNECTED --heartbeat fail--> RECONNECTING`
- **Pairing:** `IDLE → QR_SHOWN → SCANNED → SAS_VERIFY → TRUSTED`
- **File Xfer:** `QUEUED → SENDING (offset) → VERIFYING → DONE / PAUSED → RESUMED`

## Storage
- Keys: keyring/secret-service + Android Keystore
- Pairings: SQLite `pairings(id, fp, pubkey, trusted_at, perms JSON)`
- Transfers: `transfers(id, path, offset, sha, state)`

## Security boundary
LAN attacker model → mutual TLS, per-feature toggles (`clipboard:bool` etc.), revocation wipes keys + QUIC session.

### Deployment note (Render Hobby, no DB)
- **No deployment for core:** Tauri `frontendDist: ../dist` is bundled into `bridge` binary; `ws://192.168.1.36:8443` is LAN `0.0.0.0`. No `render.yaml` is needed for pairing.
- **Optional static demo:** `render.yaml` hosts `apps/desktop/dist` as static on Render free (no DB, `buildCommand` only). No Postgres; state stays in local SQLite/keyring.
- **Relay optional:** `relay` feature is additive (`--relay` flag); default `relay:false` keeps it LAN-first.
