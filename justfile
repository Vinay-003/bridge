set shell := ["bash", "-cu"]
default:
  @just --list

dev:
  cargo run -p bridge-daemon &
  pnpm --filter desktop dev

check:
  cargo check --workspace
  pnpm -r exec tsc --noEmit || true
  cargo test --workspace --no-run

test:
  cargo test --workspace
  pnpm -r test || true

android:
  cd apps/android && ./gradlew assembleDebug

lint:
  cargo fmt --check || true
  cargo clippy -- -D warnings || true
