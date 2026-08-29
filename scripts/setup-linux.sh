#!/usr/bin/env bash
set -euo pipefail
echo "Bridge setup (requires sudo passwd 123 externally for apt/dkms)"
echo '123' | sudo -S apt update
echo '123' | sudo -S apt install -y libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev pkg-config build-essential curl wget file xdg-utils protobuf-compiler avahi-daemon avahi-utils libavahi-compat-libdnssd-dev v4l2loopback-dkms v4l2loopback-utils pipewire pipewire-pulse wireplumber gstreamer1.0-tools gstreamer1.0-pipewire linux-headers-$(uname -r) || true
echo '123' | sudo -S dkms autoinstall || true
echo '123' | sudo -S modprobe v4l2loopback devices=1 video_nr=10 card_label="Bridge Cam" exclusive_caps=1 || true
rustup default stable
cargo install tauri-cli --locked || true
cargo install cargo-watch --locked || true
corepack enable || true
echo "Done. Export ANDROID_HOME=~/Android/Sdk"
