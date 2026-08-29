# Test Strategy (tdd-workflow)

- **Targets:** 80% coverage unit+integration+E2E
- **Unit:** bridge-core (pairing kdf, chunk sha, protocol serdes) — `cargo test`
- **Integration:** WS roundtrip, QUIC resume 50% kill, mDNS mock — `cargo test --test integration`
- **Desktop UI:** Vitest (unit) + Playwright (E2E POM) — `pnpm test`, `pnpm e2e`
- **Android:** JUnit + Compose UI test + Espresso — `./gradlew test connectedCheck`
- **Gates:** verification-loop phases: build → typecheck → lint → test → security scan (gitleaks, cargo audit) → diff review
