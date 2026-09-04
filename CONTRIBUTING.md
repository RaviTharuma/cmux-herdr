# Contributing to cmux-herdr

Thank you for contributing to the **cmux-herdr** plugin — the cmux plugin for
Herdr. Runtime code is a Rust Cargo binary; users receive prebuilt,
checksum-verified releases and do not need Python or Rust toolchains.

A German overview of the project and of GitHub itself lives in
[docs/de/README.md](docs/de/README.md). Maintainer GitHub settings are in
[docs/MAINTAINING.md](docs/MAINTAINING.md). Architecture (what each file is
for) is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## User install vs this clone

End users install with the official cmux plugin manager (see the README).
`./scripts/install.sh` is **contributor/dev only**: it symlinks
`bin/cmux-herdr` into `~/.local/bin` and copies the agent skill so edits in
this clone go live. It does **not** copy `sidebars/herdr.js` / `herdr.swift`
(those are experimental leftovers; uninstall removes leftover copies under
`~/.config/cmux/sidebars/`). It is not the documented user path and it does
not replace `cmux sidebar plugin install`.

```bash
./scripts/install.sh
./scripts/uninstall.sh
```

Optional LaunchAgent (also dev): `./scripts/install-watch-service.sh`.

## Before you start

1. Install Rust and Cargo (the stable toolchain, with `rustfmt` and `clippy`).
2. You do **not** need macOS, `herdr`, or `cmux` to run the test suite.
   Those binaries are only required for live use.
3. Read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## How to verify a change

```bash
./scripts/test.sh
./bin/cmux-herdr --version
./bin/cmux-herdr --help
./bin/cmux-herdr doctor
```

`./scripts/test.sh` runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
and `cargo test`.

Live commands (`tree`, `sync`, `mirror`) need fake or real `herdr`/`cmux` on
`PATH`. The suite already ships fakes; copy that pattern from the Rust
integration tests if you add a CLI test.


## Pull requests

1. Branch from `main`.
2. Keep the diff focused (one concern per PR).
3. Run `./scripts/test.sh` and mention that you did.
4. Do not commit:
   - secrets, `.env`, private keys
   - live `cmux tree` / Herdr dumps
5. Use the PR template. Link an issue when one exists.

CI runs the same `./scripts/test.sh` on macOS and Linux.

## Where to file bugs

| Kind of problem | Where |
|---|---|
| This plugin (`cmux-herdr` CLI, install scripts, docs) | [Issues here](https://github.com/RaviTharuma/cmux-herdr/issues) |
| Herdr itself | [herdrdev/herdr](https://github.com/herdrdev/herdr) |
| cmux itself | [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux) |
| Native nested topology inside cmux | [cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737) and related PRs |

## Code layout (short)

| Path | Role |
|---|---|
| `src/*.rs` | Rust runtime: CLI, sidebar, socket, state, mirror, lifecycle, update service |
| `bin/cmux-herdr` | Thin POSIX-sh launcher for the runtime binary |
| `bin/cmux-herdr-sidebar` | Thin launcher passing the `sidebar` subcommand |
| `tests/` | Cargo integration and behavior tests with fake binaries |
| `scripts/` | Contributor install / uninstall / test / LaunchAgent |
| `cmux-plugin.toml` | Official plugin-manager manifest and bootstrap configuration |
| `sidebars/` | Experimental leftover sidebars (not default-installed) |
| `agent-skill/` | Instructions for AI agents using the dual hierarchy |

New runtime code belongs in the appropriate `src/*.rs` module with a focused
Rust test. New CLI flags belong in the clap definitions plus an integration
test when they are user-visible.

## License

By contributing, you agree that your contribution is licensed under the
[MIT License](LICENSE) already used by this repository.
