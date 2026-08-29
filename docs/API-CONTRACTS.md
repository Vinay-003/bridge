# API Contracts — Tauri invoke + WS

## Tauri invoke (desktop UI → Rust)
- `pairing_start() -> { qrDataUrl, sas6, fp }`
- `pairing_confirm(sasMatch:bool) -> { trustedId }`
- `list_devices() -> Device[]`
- `file_send(paths: string[]) -> TransferId[]`
- `clipboard_set(text:string) -> ok`
- `notify_list() -> Notification[]` / `notify_action(key, action, text?)`

## WS (see PROTOCOL.md)
Errors map to `{ code, message, details }` with HTTP-style codes for UI toasts.
