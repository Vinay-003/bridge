# ADR-001: Technology Stack — Bridge Linux ↔ Android Continuity

**Status:** Accepted — 2026-08-29  
**Deciders:** Bridge core

## Context
Bridge must pair Android 10+ with Linux (MVP, later Windows) LAN-first, idle <10% CPU / <250 MB RAM, pairing <10s, discovery <2s, audio <50ms, webcam <100ms, E2E encrypted, modular services, installable via .deb/AppImage. Host is Ubuntu 24.04, Rust 1.98, Node 24, Android SDK 34.

## Decision

| Layer | Choice | Rationale |
|-------|--------|-----------|
| **Desktop UI + Daemon** | **Tauri v2 (Rust + WebKitGTK) + React 18 + Vite + TypeScript + Tailwind + shadcn/ui** | 15-30 MB binary, <50 MB RAM idle vs Electron 300+ MB; Rust Tokio for transport; system tray/single-instance/autostart built-in; matches NFR 6 |
| **Shared core** | `crates/bridge-core` Rust (serde/ring/rustls/tokio) + parity schemas for Android | Single source for crypto/protocol, no TS/Kotlin drift |
| **Transport control** | TLS 1.3 WebSocket (`tokio-tungstenite` + `rustls`, port 8443) | Browser-compatible, easy signalling for WebRTC |
| **Transport bulk** | QUIC (`quinn`) multiplexed streams 1 MB chunks + SHA256 | 0-RTT reconnect, no head-of-line blocking, resume |
| **Discovery** | mDNS (`mdns-sd` desktop + `NsdManager`/`JmDNS` Android) primary, BLE (`btleplug` + Android advertise) secondary, Wi-Fi Direct future | mDNS <2s on LAN, BLE solves isolated AP |
| **Pairing** | QR `bridge://pair?v=1&id&ecdh&fp&port` + ECDH P-256, SAS 6-digit confirm, Noise XX style, keys in `keyring` (libsecret) / Android `Keystore+EncryptedSharedPreferences` | Local-only, MITM resistant, per Security 10 |
| **Media** | WebRTC (`webrtc-rs` / Pion) for camera/mic/speaker/screen; Opus 48kHz, H.264 HW fallback VP8/AV1, bitrate adaptive via RTCP | Meets latency targets, echo/NS/AGC via `webrtc-audio-processing` |
| **Virtual devices** | `v4l2loopback` `/dev/video10` (webcam), PipeWire virtual source/sink for mic/speaker, `xdg-desktop-portal ScreenCast` fallback | Native Meet/Zoom integration w/o kernel signing on Linux |
| **Android** | Kotlin + Jetpack Compose + CameraX + MediaProjection + NotificationListenerService + DataStore + WorkManager | Only way to implement FG service, MediaProjection, notification reply without root |
| **Build** | pnpm workspaces, Cargo workspace, Gradle 8 Kotlin DSL, `just` runner, GitHub Actions | |

## Alternatives rejected
- **Electron:** violates idle RAM, 150 MB+ installer, higher CPU
- **Flutter/RN Android:** cannot reliably implement NotificationListener/FG service OEM whitelisting, adds 60 MB overhead
- **Raw RTP/GStreamer pipeline without WebRTC:** heavier pipeline, no built-in congestion control, more custom code
- **WebSocket-only bulk:** no multiplex resume

## Consequences
- Requires `libwebkit2gtk-4.1-dev` etc. for Tauri build — handled in `scripts/setup-linux.sh`
- Requires `v4l2loopback-dkms` + `modprobe` step — documented, graceful degrade if missing
- QUIC needs UDP hole punch friendly firewall rule (`8443/udp`)

## Skills mapping
`security-review` → pairing/crypto, `api-design`+`backend-patterns` → WS/QUIC contracts, `frontend-patterns`+`taste-skill`+`impeccable` → UI, `tdd-workflow`+`e2e-testing`+`verification-loop` → 80% coverage
