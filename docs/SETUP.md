# Setup — Bridge

## Host tested: Ubuntu 24.04, Node 24, Rust stable 1.98, Android SDK 34

### Deps (one-shot, host terminal)
```bash
bash scripts/setup-linux.sh # wraps:
# apt: libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev pkg-config protobuf-compiler avahi-daemon avahi-utils libavahi-compat-libdnssd-dev v4l2loopback-dkms v4l2loopback-utils pipewire wireplumber
# modprobe v4l2loopback devices=1 video_nr=10 card_label="Bridge Cam" exclusive_caps=1
# rustup default stable; cargo install tauri-cli --locked; cargo install cargo-watch --locked
```

### Env
```bash
export ANDROID_HOME=~/Android/Sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools
```

### Run (no sudo)
```bash
just dev        # runs bridge-daemon + desktop vite + tauri dev (or vite-only fallback)
just check      # cargo check + pnpm typecheck
just test       # cargo test + pnpm test
just android    # ./gradlew assembleDebug in apps/android
```

### Virtual devices
- Cam: `ls /dev/video10` → appears as “Bridge Cam” in Meet/Zoom
- Mic/Speaker: PipeWire `Bridge Mic` / `Bridge Speaker` (created by daemon, fallback to loopback if pipewire unavailable)

### OpenCode Zen (AI cloud)
Bridge AI (`ai.summarize` for notifications, `ai.transcribe` for calls) prefers local `whisper.cpp`/`llama.cpp` if `BRIDGE_LOCAL_AI=1` or binary exists. Else it uses **OpenCode Zen** cloud if you provide a key.

1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   # edit .env, set:
   # OPENCODE_ZEN_API_KEY=sk_zen_...
   # OPENCODE_ZEN_BASE_URL=https://zen.opencode.ai/v1  # default
   # OPENCODE_ZEN_MODEL=zen-3  # default
   ```
2. The daemon reads `OPENCODE_ZEN_API_KEY` at runtime (`zen_api_key()` checks `OPENCODE_ZEN_API_KEY` → `OPENCODE_ZEN_KEY` → `ZEN_API_KEY` → `BRIDGE_OPENAI_KEY`). If set, `ai.summarize`/`transcribe` will call `POST $BASE/chat/completions` with `Authorization: Bearer $KEY`. No key → mock fallback (good for E2E `BRIDGE_ALLOW_CLOUD_MOCK=1`).

3. Test:
   ```bash
   OPENCODE_ZEN_API_KEY=sk_test cargo test -p bridge-daemon ai -- --nocapture
   # or via WS:
   # {"type":"ai.summarize","payload":{"notifications":[{"app":"WhatsApp","body":"hi"}],"maxLen":200,"cloudConsent":true}}
   ```

No DB needed — Bridge is LAN-first: `~/.local/share/bridge/bridge.db` (SQLite) + `keyring` + Android `DataStore`/`Keystore`. No Postgres for Render.

### Deployment (Render Hobby — optional, no DB)
Core Bridge needs **no deployment**: desktop is Tauri local (`pnpm --filter desktop build` → `apps/desktop/dist` bundled into native app). `vite dev` at `localhost:1420` is dev only.

If you want a public landing/demo page on **Render free (512 MB, no DB)**, this repo includes `render.yaml`:
```yaml
services:
  - type: web
    name: bridge-static
    env: static
    plan: free
    buildCommand: pnpm install && pnpm --filter desktop build
    staticPublishPath: apps/desktop/dist
```
Push to GitHub → Render auto-deploys `dist` as static (no DB, no env). Works on hobby.

### LAN/Bluetooth only (no relay)
By default `relay=false` (`crates/bridge-daemon/src/main.rs --relay`). Set `relay` only if you need global `https://relay.bridge.dev/v1/announce` + `STUN stun.l.google.com:19302`. LAN `mDNS _bridge._tcp + BLE` is default and needs no deployment.

