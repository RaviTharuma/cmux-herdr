# AGENTS.md


## Testing (HARD)
Sources: https://x.com/anshnanda/status/2101627891721371971 · https://x.com/nimsbh_ai/status/2102083469362790401 · https://x.com/imrobertjames/status/2100787901701456057

- NEVER write unit tests after you write code.
- Highly prefer E2E tests as the sole testing mechanism. Use them to verify complex features work. At the end of E2E tests, produce a verifiable and repeatable artifact.
- If you must test a system in isolation, FIRST write down all the ways it could fail, THEN write the code.
- When writing E2E tests, do not pick the simplest possible scenario to prove it works — pick a medium-to-hard scenario (models love to cheat).
- Tautological tests considered harmful.
- Change-detector tests considered harmful.
- Do not create regression tests for bug fixes without a genuine gap in behavior testing.

## Cursor Cloud specific instructions

`cmux-herdr` is a **cmux plugin for Herdr** implemented in Rust. Runtime source
lives under `src/*.rs`, with integration tests under `tests/*.rs`. Contributors
need the Rust toolchain and Cargo; end users run prebuilt plugin binaries and
do not need Python or Rust installed.

### Lint / test / build / run

- The complete Rust verification gate is `./scripts/test.sh`. It runs, in
  order, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo test --locked`.
- Use Cargo for local development and builds. Keep runtime changes in `src/*.rs`
  and integration coverage in `tests/*.rs`.
- The release workflow produces prebuilt binaries for users. Do not require
  users to install Python or Rust to run those binaries.

### Non-obvious runtime caveat (important for exercising core flows)

The product is a **macOS** bridge between the `herdr` and `cmux` CLIs, and neither
binary exists on the Linux cloud VM. Commands that only introspect the plugin run
standalone, but core flows shell out to `herdr` (and to `cmux` where needed).

Integration fakes are implemented in the Rust tests under `tests/*.rs`; use those
fixtures when exercising core flows in environments without the host CLIs. Key
details when faking:

- Each command returns JSON as `{"result": {...}}` on stdout.
- `sync`/`mirror` resolve the outer cmux workspace via `cmux identify --json`,
  reading `caller.workspace_ref` / `focused.workspace_ref`. They also require a
  complete host fingerprint: env `CMUX_SURFACE_ID` and `HERDR_SOCKET_PATH` (an
  existing file), or pass `--workspace <id>` explicitly. Without these, `sync`
  aborts with a clear message by design (it will not guess a host).
- The association/parent-binding cache is written under `$XDG_STATE_HOME/cmux-herdr/`
  (defaults to `~/.local/state/`); point `XDG_STATE_HOME` at a temp dir to keep runs
  isolated.
