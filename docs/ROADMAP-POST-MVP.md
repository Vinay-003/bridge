# Post-MVP Roadmap — Deep Implementation Beyond MVP

## Current MVP status (v0.2.0)
- Secure pairing (QR ECDH + host), mDNS, WS 8443 + status.push, arboard clipboard bidir, file chunk SHA256 → ~/Bridge, notify.new via WS, WebRTC signalling stub (v4l2loopback + PipeWire), phone status push 5s.
- Tests: bridge-core 7, bridge-daemon 4, desktop vitest 3, android 3, e2e simulate_e2e.py 6 suites ALL PASSED.
- APK at http://192.168.1.36:8000/app-debug.apk (35M), daemon on 8443, vite 1420.

## Philosophy — Go Deep, Not Surface
Per karpathy-guidelines + security-review: no stub handlers. Every future feature needs:
1. Threat model + permission matrix
2. State machine + sequence diagram
3. Protocol spec (MessageType + zod/serde)
4. TDD red→green + E2E simulation
5. Verification loop (build→typecheck→lint→test→security→diff)

## Phase 3: Telephony — Calls & SMS via Desktop (2 weeks)
- Android: `TelecomManager` + `InCallService` + `SmsManager` + `RoleManager` (DEFAULT_DIALER/SMS), `READ_CALL_LOG`, `READ_PHONE_STATE`, `SEND_SMS`, `READ_SMS`. Desktop dialer UI, incall audio via `webrtc-audio` Opus → PipeWire Bridge Mic/Speaker. SMS list via `ContentProvider`.
- Protocol: `sms.list`, `sms.send`, `call.start`, `call.answer`, `call.hangup`, `call.audio` (WebRTC)
- Security: per-call explicit user tap on phone (no silent dial), SMS preview requires unlock.
- Deep: handle `ConnectionService` for VoIP-like bridging, dual-SIM selection, RCS fallback.

## Phase 4: Remote Control — Input Injection (2 weeks)
- Android `AccessibilityService` (gestures: tap/swipe/pinch, `dispatchGesture`, `performGlobalAction`), `MediaProjection` for screen (already stubbed). Desktop captures input via `rdev`/`enigo` (Tauri plugin), sends `input.event` WS (x,y,action,pressure). Throttle 60fps, coalesce.
- Security: explicit toggle on phone “Allow input control”, auto-off when screen locked, no background injection.
- Deep: handle display metrics scaling, multi-display, drag, clipboard via input path.

## Phase 5: Storage Deep — Folder Sync, MTP, Trash, SAF (2 weeks)
- Android SAF `DocumentFile`, `MediaStore`, `/sdcard` via `MANAGE_EXTERNAL_STORAGE` (scoped). Desktop `notify` + `inotify` via Rust `notify` crate, `~/Bridge` watch, `~/Bridge/.bridge-sync` manifest. Folder sync bidirectional with conflict LWW.
- Protocol: `storage.ls`, `storage.stat`, `storage.mkdir`, `storage.rm`, `storage.sync` (chunked like files but for dirs)
- Deep: handle trash (`~/.local/share/Trash`), conflict rename, 4GB+ resume, foreground sync notification.

## Phase 6: Global Relay + Multi-device Mesh (2 weeks)
- Beyond LAN: QUIC relay server (Rust `quinn` + `axum` signalling) with capability: NAT hole punching via STUN (public `stun.l.google.com:19302`), fallback relay. Global discovery via `libp2p` DHT or custom `https://relay.bridge.dev/v1/announce` with E2E (no server sees plaintext).
- Multi-device: mesh where one Linux pairs N phones, phone pairs M desktops, consistent pairing DB with `CRDT` (last-write-wins for clipboard, vector clocks for files).
- Security: relay is E2E via Noise; server only sees opaque relay.

## Phase 7: Plugin Platform + AI (2 weeks)
- Plugin: `bridge-extension` manifest (`bridge.json`), `wasmtime` sandbox or `deno` JS. APIs: `bridge.on('notify.new')`, `bridge.clipboard`, `bridge.storage`. Example plugins: OCR, translate, auto-reply.
- AI: on-device `whisper.cpp` for call transcription, `llama.cpp` for notification summarization, desktop GPU fallback. Trigger via `ai.summarize` WS.
- Deep: handle plugin permissions (capabilities), version pinning, hot reload.

## Cross-cutting
- **Tests:** each phase adds 80% coverage, `simulate_e2e.py` grows to cover new MessageTypes.
- **Security:** `security-review` checklist per phase, gitleaks, cargo audit.
- **Docs:** per-phase `docs/PHASE-X.md` with sequence diagrams (mermaid).

## Execution — Parallel Agents
- Agent A: Telephony (calls/sms)
- Agent B: Remote Control (a11y + input)
- Agent C: Storage Deep
- Agent D: Relay/Mesh + Plugins/AI (can be combined)
Each agent: plan → implement → test → commit → push, no surface stubs.

## Next step
Branch `feat/post-mvp` and let 4 agents work parallel; main merges after verification-loop.
