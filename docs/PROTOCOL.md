# Protocol — Bridge v1

## Framing (all WS/QUIC JSON, CBOR optional later)
```json
{
  "v": 1,
  "id": "uuidv4",
  "type": "file.chunk | clipboard.sync | notify.new | status.push | webrtc.offer/answer/ice | pairing.sas | heartbeat.ping/pong",
  "ts": 1710000000000,
  "nonce": "hex8",
  "payload": { }
}
```
Validation: `zod` on TS, `serde`+`jsonschema` on Rust/Kotlin, unknown `type` → `error.unknown_type`.

## Control WS 8443 (TLS 1.3, pinned cert)
- `pairing.hello` — `{clientId, ecdhPub}`
- `pairing.sas` — `{sas6, confirm:bool}`
- `heartbeat.ping/pong` 15s interval, 45s timeout → RECONNECTING

## Bulk QUIC (same TLS cert, ALPN `bridge-1`, port 8444/udp)
- Stream per file: `FILE <id> <offset> <sha256> <len>\n<bytes>` + ACK per chunk
- Resume: client sends `RESUME <id> <offset>` → server seeks

## Service Schemas

### file.chunk
`{ id, name, size, offset, total, sha256, data_b64 }`

### clipboard.sync
`{ mime: "text/plain"|"image/png", data_b64, ts, source:"desktop"|"android" }` — LWW, deduplicate if ts diff < 500ms and payload equal

### notify.new / notify.action
`notify.new: { key, app, title, body, ts, hasReply:bool }`
`notify.action: { key, action:"reply"|"dismiss", text? }` → Android RemoteInput

### status.push (every 5s)
`{ battery:{pct,charging,tempC}, ram:{availMb,totalMb}, storage:{freeGb,totalGb}, signal:{dbm,bars} }`

### webrtc.offer/answer/ice
SDP/ICE forwarded via WS control, media flows over UDP WebRTC (Opus 48kHz mono 32kbps, H264 720p 30fps adaptive). Signalling encrypted within TLS.

## Error envelope
`{ code:"pairing.sas_mismatch"|"file.sha_mismatch"|"auth.untrusted", message, details? }` → HTTP-style mapping for Tauri `invoke` (api-design).
