# cmux-herdr v0.8.0 E2E release report

Date: 2026-10-07
Release commit: `8fdbbad95dd35ee6741cd7bea3f1a318d53da2f4`
Tag: `v0.8.0` (annotated)
Release: https://github.com/RaviTharuma/cmux-herdr/releases/tag/v0.8.0
Release workflow: https://github.com/RaviTharuma/cmux-herdr/actions/runs/37664778277

## Gate 1 — Full Ubuntu test gate: PASS

- `./scripts/test.sh` exited 0 on the final v0.8.0 release tree.
- The gate ran offline packaging fixtures, `cargo fmt --all --check`, strict Clippy, and the locked Rust test suite.
- Release-tag verification reran the same gate successfully in GitHub Actions.
- Release PR CI passed on Ubuntu and macOS: https://github.com/RaviTharuma/cmux-herdr/actions/runs/37664516198

## Gate 2 — Real binary smoke: PASS with host-limited paths

Built with `cargo build --release --locked`; `./target/release/cmux-herdr --version` printed `cmux-herdr 0.8.0`.
All final smoke commands removed `HERDR_ENV` from the command environment. No command stopped the Herdr server.

| Command | Status | Evidence |
| --- | --- | --- |
| `doctor` | PASS | Exercised the live binary and Herdr socket. It correctly reported Linux cmux compatibility as unsupported and accepted the missing `CMUX_SURFACE_ID` for non-nested inspection. Exit 1 reflects the unavailable macOS cmux host operation. |
| `status` | PASS | Exit 0. Reported live mixed counts: `{"idle":11,"done":4,"working":1}`. |
| `tree` | PASS | Exit 0 against the live Herdr socket. |
| `sync` | BLOCKED | Correct fail-closed exit 1: no complete host fingerprint (`CMUX_SURFACE_ID` + `HERDR_SOCKET_PATH`) on Linux, and no explicit macOS workspace was available. It did not borrow the focused workspace. |
| `watch` | PASS | Started, attached the mirror, reached the expected unsupported Linux cmux apply boundary, and detached on timeout with `server_stopped=false`. Exit 124 is the five-second smoke timeout. |

The first smoke found a real `status` panic while inserting the first value into a `serde_json::Map`. The release branch changed this to `Map::insert`, added a medium-hard CLI integration flow with repeated `working` plus `done` agents, and passed that focused test and the full gate. The final release binary then returned exit 0 with live mixed status counts.

## Gate 3 — A04 dependency upgrades: PASS

### sha2 0.11.0

- Fresh branch from current `main`; no old Dependabot branch was force-pushed.
- Updated digest formatting for the sha2 0.11 output type, including bootstrap checksum fixtures.
- PR: https://github.com/RaviTharuma/cmux-herdr/pull/87
- CI: https://github.com/RaviTharuma/cmux-herdr/actions/runs/37662100785
- Result: merged; Ubuntu and macOS checks passed.

### rustix 1.1.4

- Fresh branch from current `main`; no old Dependabot branch was force-pushed.
- Updated rustix signal constants to `Signal::KILL` and `Signal::TERM`.
- PR: https://github.com/RaviTharuma/cmux-herdr/pull/88
- CI: https://github.com/RaviTharuma/cmux-herdr/actions/runs/37662651702
- Result: merged; Ubuntu and macOS checks passed.

## Gate 4 — Release documentation and version cut: PASS

- Release PR: https://github.com/RaviTharuma/cmux-herdr/pull/89
- `VERSION`, `Cargo.toml`, `Cargo.lock`, `cmux-plugin.toml`, README version text, and `RELEASE.md` moved to 0.8.0.
- `CHANGELOG.md` moved Unreleased changes into `## [0.8.0] - 2026-10-07` and records the dependency upgrades, corrected install command, and smoke-discovered status fix.
- User install documentation now consistently uses `cmux-tui sidebar plugin install|use|update|remove`. The unavailable `cmux sidebar plugin` path remains only in the changelog explanation of the corrected bug.
- Native Sidebar issue #75 remains blocked upstream and did not block this release.
- The optional Moshi gap was not required for the release and was not used as a gate.

## Gate 5 — Annotated tag and publication: PASS

- Annotated tag `v0.8.0` points to merged release commit `8fdbbad95dd35ee6741cd7bea3f1a318d53da2f4`.
- Release workflow completed successfully: https://github.com/RaviTharuma/cmux-herdr/actions/runs/37664778277
- Immutable GitHub release published: https://github.com/RaviTharuma/cmux-herdr/releases/tag/v0.8.0
- Published assets:
  - `cmux-herdr-0.8.0-aarch64-apple-darwin`
  - `cmux-herdr-0.8.0-x86_64-apple-darwin`
  - `cmux-herdr-0.8.0-aarch64-unknown-linux-gnu`
  - `cmux-herdr-0.8.0-x86_64-unknown-linux-gnu`
  - `SHA256SUMS`

## Final result: PASS

v0.8.0 is merged, tagged, published, and backed by green Ubuntu/macOS CI plus the available live Linux/Herdr smoke. The only blocked subpath is a successful `sync` apply against a real macOS cmux workspace, which is unavailable on this Linux host; the fail-closed behavior was exercised and correct.
