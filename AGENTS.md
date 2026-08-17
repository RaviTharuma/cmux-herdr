# AGENTS.md

## Cursor Cloud specific instructions

`cmux-herdr` is a **pure-Python (stdlib-only) CLI plugin** — no pip/npm dependencies, no
build step, no compiled artifacts. It needs only a Python 3.10+ interpreter (the VM ships
3.12). There is nothing to install to develop or test it.

### Lint / test / build / run

- Tests: `./scripts/test.sh` (stdlib `unittest` only — **do not add pytest**). It runs
  `py_compile` on the sources plus `unittest discover` over `bridge/` and `tests/`.
- There is no separate lint or build step; `py_compile` inside `scripts/test.sh` is the
  compile check.
- Run the CLI directly from the repo without installing: `./bin/cmux-herdr <command>`
  (e.g. `--version`, `--help`, `doctor`). `scripts/install.sh` only copies the CLI into
  `~/.local/bin` and is not required for development.
- Standard dev/verify commands are already documented in `README.md`
  ("Development and verification") and `bin/cmux-herdr --help`.

### Non-obvious runtime caveat (important for exercising core flows)

The product is a **macOS** bridge between the `herdr` and `cmux` CLIs, and neither binary
exists on the Linux cloud VM. Commands that only introspect the plugin (`--version`,
`--help`, `doctor`) run standalone, but the core flows (`tree`, `agents`, `sync`,
`mirror`, `json-dump`) shell out to `herdr` (and `cmux` for `sync`/`mirror`).

To run those flows end-to-end here, put fake `herdr`/`cmux` executables on `PATH` — the
exact pattern the suite uses lives in `tests/test_cli_behavior.py` (`FAKE_HERDR_FULL`) and
`tests/test_bridge_behavior.py`. Key details when faking:

- Each command returns JSON as `{"result": {...}}` on stdout.
- `sync`/`mirror` resolve the outer cmux workspace via `cmux identify --json`, reading
  `caller.workspace_ref` / `focused.workspace_ref`. They also require a complete host
  fingerprint: env `CMUX_SURFACE_ID` **and** `HERDR_SOCKET_PATH` (an existing file), or
  pass `--workspace <id>` explicitly. Without these, `sync` aborts with a clear message
  by design (it will not guess a host).
- The association/parent-binding cache is written under `$XDG_STATE_HOME/cmux-herdr/`
  (defaults to `~/.local/state/`); point `XDG_STATE_HOME` at a temp dir to keep runs
  isolated.
