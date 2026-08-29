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
