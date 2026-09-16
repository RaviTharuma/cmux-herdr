# Tech stack

**cmux-herdr** is a **Rust** [cmux](https://github.com/manaflow-ai/cmux) plugin that
bridges [Herdr](https://github.com/herdrdev/herdr) into cmux chrome. End users run
prebuilt, checksum-verified binaries. Contributors use **Cargo**.

## Runtime

| Layer | Choice | Notes |
|---|---|---|
| Language | Rust (edition 2021, MSRV `1.98`) | Single binary `cmux-herdr` from `src/main.rs` |
| CLI | `clap` (derive) | Subcommands: sync, watch, mirror, rail, doctor, … |
| Serialization | `serde` / `serde_json` (`preserve_order`) | Host fingerprint state + Herdr NDJSON RPC |
| Config edits | `toml_edit` | Plugin / platform registration helpers |
| OS primitives | `rustix` | fs, process, net, termios |
| Hashing | `sha2` | Fingerprints / integrity helpers |
| Tests | `tempfile` + hermetic fakes | Fake `herdr` / `cmux` scripts under `tests/` |

There is **no** Python runtime for users. Thin POSIX-sh launchers under `bin/`
resolve and exec the Rust binary (`cmux-herdr`, `cmux-herdr-sidebar`,
`cmux-herdr-fetch`).

## Product shape

```text
cmux.app (macOS) ── terminal ── herdr ── nested agents/panes
                     ▲                    ▲
                     │ subprocess CLIs    │ Unix socket (NDJSON RPC)
                     └──── cmux-herdr ────┘
```

| Concern | Implementation |
|---|---|
| Plugin manifest | `cmux-plugin.toml` (`kind=sidebar`) |
| Distribution | GitHub Releases + `SHA256SUMS`; `bin/cmux-herdr-fetch` |
| State | `$XDG_STATE_HOME/cmux-herdr/` (local only) |
| Left chrome | Workspaces / machines only — no custom Herdr left sidebar |
| Right chrome | Project into existing cmux right rail (sessions / feed / dock) |
| Inspiration | Moshi Desktop concepts only; stay cmux-native |

Design detail: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md),
[docs/RIGHT_RAIL.md](docs/RIGHT_RAIL.md),
[docs/PLUGIN_DESIGN.md](docs/PLUGIN_DESIGN.md).

## Platforms

| Target | Role |
|---|---|
| `aarch64-apple-darwin` | Primary product (Apple Silicon Mac + cmux + Herdr) |
| `x86_64-apple-darwin` | Intel Mac product builds |
| `x86_64-unknown-linux-gnu` | CI / hermetic tests (no product GUI) |
| `aarch64-unknown-linux-gnu` | CI / hermetic tests (no product GUI) |

Product flows that shell out to `cmux` / `herdr` need those CLIs on **macOS**.
Linux is for the verification gate and release artifact builds.

## Verification gate

Canonical local and CI gate:

```bash
./scripts/test.sh
```

In order:

1. Offline packaging fixtures (`bash`/`sh` syntax + packaging scripts)
2. `cargo fmt --all --check`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo test --locked`

See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md).

## What this stack is not

- Not a cloud service, SaaS, or first-party telemetry backend
- Not a patch inside `cmux.app` or Herdr
- Not a Node/Python plugin runtime for end users
- Not Moshi Desktop chrome (no Chat View, APNs, or loopback web gateway)

License and risk: [LICENSE](LICENSE), [DISCLAIMER.md](DISCLAIMER.md),
[SECURITY.md](SECURITY.md).
