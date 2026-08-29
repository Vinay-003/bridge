# Roadmap — MVP

**Phase 0 (done):** ADR, ARCH, PROTOCOL, SECURITY, SETUP, monorepo, CI
**Phase 1:** Pairing + discovery + reconnect (WS TLS + mDNS + QR SAS)
**Phase 2:** File transfer (QUIC chunks, drag&drop, progress) + Device status
**Phase 3:** Clipboard (text/image LWW)
**Phase 4:** Notifications (mirror/reply/dismiss)
**Phase 5:** Camera virtual webcam (WebRTC → v4l2loopback)
**Phase 6:** Audio mic/speaker (WebRTC Opus → PipeWire)
**Phase 7:** Screen mirror (+ record/screenshot)
**Phase 8:** Hardening, .deb/AppImage, auto-start, 80% coverage

Each phase commits: `feat(<scope>): <slice>`, TDD red→green, verification-loop gate.
