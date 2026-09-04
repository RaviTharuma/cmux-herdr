# cmux-herdr → Rust migration design

Status: proposed. Owner: RaviTharuma. Source of truth for replacing the
stdlib-Python runtime with a single Rust binary, and for porting the
`dakesan/cmux-herdr` auto-update work into that binary.

## 1. Goal and scope

Rewrite the Python runtime of `cmux-herdr` in Rust "as much as practical",
and fold in the useful features from `dakesan/cmux-herdr` (currently 5 commits
ahead: an optional Herdr binary auto-update service).

In scope (Python today → Rust):

- `bin/cmux-herdr` (~2.2k LOC argparse CLI, 61 `cmd_*` handlers).
- `bin/cmux-herdr-sidebar` launcher + `bridge/cmux_herdr_sidebar.py` TUI.
- All 16 `bridge/cmux_herdr_*.py` runtime modules (~13.7k LOC total).
- The auto-update logic ported from the fork (`cmux_herdr_update_config.py`
  + `herdr-auto-update.sh` runner semantics), redesigned in Rust.

Out of scope (stays as-is, by design):

- `sidebars/herdr.js` / `herdr.swift` — already demoted to experimental
  leftovers by commit `aa7ccf9`; not the product. Not translated.
- Platform unit files (`launchd` plist, systemd `.service`/`.timer`) — these
  are the OS-native interface. The Rust binary generates/registers them; we do
  not reimplement `launchd`/`systemd`.
- Thin install/uninstall shell wrappers that only shell out to the binary.
- Agent skill (`agent-skill/SKILL.md`), docs, license, issue templates.

Non-negotiable: the plugin must keep installing through the official
`cmux sidebar plugin install` path defined by `cmux-plugin.toml`.

## 2. Compatibility invariants (must not regress)

The rewrite is a behavioral port, not a redesign. These contracts are locked
by existing tests and by live cmux/Herdr integration:

1. **Plugin manifest.** `cmux-plugin.toml` stays `kind = "sidebar"`, `[run]`
   command `bin/cmux-herdr-sidebar`. The sidebar entry must remain an
   executable at that path (a POSIX-sh launcher that `exec`s the Rust binary
   with the `sidebar` subcommand). `[build]` changes from `chmod +x` to the
   fetch/verify bootstrap (§6); `tests/test_plugin_manager.py` and
   `tests/test_sidebar_native.py`, which currently assert `chmod`-only build
   and a `#!/usr/bin/env python3` sidebar, are rewritten in the same commit.
2. **CLI surface.** All 61 subcommands (verified: 61 `cmd_*` handlers and 61
   `sub.add_parser` registrations) keep their names, positional/optional
   arguments, exit codes, and per-command `--json` behavior. `--timeout`/hidden
   `--timeout-ms` are attached to only four commands today (`start-agent`,
   `wait-output`, `agent-prompt`, `agent-wait`); `--timeout` wins when both are
   set, and handlers inject a `timeout_ms` param — preserve exactly, do not
   add the flags elsewhere. Preserve command aliases: `start-agent --agent`
   (alias of `--kind`), `wait-output` positional `pattern` (alias of
   `--match`). `--json` output is *not* uniformly machine-only: `status`,
   `doctor`, and `mirror` print a human report first, then a JSON blob (tests
   parse from the first `{`); `api` exits 2 on result failure. `--version`
   reads `VERSION` but falls back to the hardcoded `__version__` (`0.5.0`) when
   `VERSION` is missing/unreadable — keep that fallback.
3. **Herdr socket security.** Unix socket validated before use via `lstat`:
   reject symlink, require `S_ISSOCK`, reject any group/other bit
   (`mode & 0o077`, i.e. owner bits incl. `0700` are allowed — *not* a numeric
   `<= 0600` compare), require `st_uid == getuid()`. NDJSON framing, `plugin-N`
   request ids, 512 KiB line cap (`MAX_LINE_BYTES`), protocol-17
   `events.subscribe` with the exact 24-entry `DEFAULT_SUBSCRIPTIONS` set (lock
   membership *and* count). Current Python does not correlate response `id` to
   request `id`, serialize send/read across concurrent calls, validate the
   protocol version, or guard the lstat→connect TOCTOU; the Rust port SHOULD
   add id-correlation + per-connection send/read serialization and MUST
   document any behavior it tightens beyond the baseline.
4. **API allowlist.** `ALLOWED_METHODS` (protocol 17) enforced verbatim in
   `assert_method_allowed`; `FORBIDDEN_METHODS = {server.stop}`, forbidden
   prefixes `pane.graphics.`, `plugin.`. CLI-fallback argv from `build_cli_argv`
   preserved exactly, including its quirks: `_cli_pane_target` emits `--current`
   for `pane.resize/swap/neighbor/edges/layout/process-info`; `pane.current`
   with no caller pane returns `['pane','current']` (no `--current`);
   `pane.zoom` no-pane branch emits `--current`; `tab.move` has no CLI fallback
   (socket-only); `pane.close` fallback drops `force`. `HerdrApi.call` retries
   an owned socket (close/reopen) once before CLI fallback — a mutation may run
   twice after an ambiguous transport failure; the Rust port MUST decide and
   document whether it preserves or fixes this (recommend: no retry for
   non-idempotent mutations).
5. **Status pills.** `STATUS_PREFIX = "herdr:"`; `STATUS_STYLE` locked exactly
   — working `('hammer', '#ff9500', 80)`, idle `('pause.circle', '#8e8e93', 40)`,
   done `('checkmark.circle', '#34c759', 30)`, blocked
   `('exclamationmark.triangle', '#ff3b30', 90)`, unknown
   `('questionmark.circle', '#8e8e93', 10)`, and the out-of-set fallback
   `('circle', '#8e8e93', 10)`. Icons, hex colors, and priorities are all
   compatibility-significant (priority drives pill ordering). `clear` and stale
   pruning act only on keys returned by `list_cmux_herdr_keys` (startswith
   `herdr:`). `status_key` embeds the raw `pane_id`; the Rust port SHOULD add
   id validation before it crosses the cmux arg boundary.
6. **State files.** Two distinct dir contracts, do not conflate: parent /
   association / size-authority state is **XDG-only**
   (`$XDG_STATE_HOME/cmux-herdr`, default `~/.local/state/cmux-herdr`), while
   only the handoff lease/restore readers additionally inspect an existing
   macOS Application Support directory (`state_dirs`/`application_support_dir`).
   On-disk files and their JSON schemas are locked: `parent-<fp>.json`
   (workspace_ref, cmux_surface_id, herdr_socket_path, herdr_workspace_id,
   herdr_server_pid, host_fingerprint_key, updated_at);
   `associations-<fp>.json` (version, panes{}, mirrors{}, cmux_workspace, herdr
   socket/workspace/surface/server_pid, host_fingerprint_key, updated_at, with
   the full per-pane and per-mirror record fields); writer leases
   `writer-<fp>.json` plus legacy `<owner>-live[-<fp>]` markers; restores
   `restore-<endpoint_hash>.json`; mirror `size-authority-<parent_key>`.
   Current atomic writes use `mkstemp`/`.tmp` + `os.replace` **without** fsync
   of file or directory. The Rust store MUST reproduce the schemas and the
   rename-based atomicity; adding fsync is an allowed durability *improvement*
   that MUST be called out, not presented as existing behavior. Writer lease
   TTL (`CMUX_HERDR_LEASE_TTL_MS`, default 45s, min 1s), heartbeat age check,
   and `pid_alive` (`os.kill(pid,0)`, `PermissionError`→alive) takeover logic
   preserved. Note: current `load_leases`/`read_shared_restore` do **not**
   verify the payload fingerprint/endpoint_hash matches the requested key
   (invariant 7's "fail-closed" is by filename, not payload); the port SHOULD
   add the payload check and document it.
7. **Env contract.** Honor `HERDR_SOCKET_PATH`, `HERDR_ENV`, `CMUX_TUI_SOCKET`,
   legacy `CMUX_MUX_SOCKET`, and the real election vars:
   `CMUX_HERDR_FORCE_PLUGIN` (force flag — *not* bare `FORCE_PLUGIN`),
   `CMUX_HERDR_NATIVE_LIVE`, `CMUX_HERDR_LEASE_TTL_MS`,
   `CMUX_HERDR_NATIVE_STATE_DIR`. Host fingerprint fail-closed on mismatch.
8. **No new required runtime deps for users.** Users must not need Python,
   Rust, or a package manager at run time.

## 3. Approaches considered

### A. Big-bang single Rust binary (recommended)

One `cmux-herdr` Rust binary implements the CLI, the sidebar TUI, watch/mirror
engine, socket client, state store, and auto-update. Python is deleted in one
cutover once the Rust port passes the ported test suite. `bin/cmux-herdr` and
`bin/cmux-herdr-sidebar` become thin launchers (or symlinks) resolving the
built binary.

- Pros: matches the clean-cutover rule (no shims/dual runtime); one artifact;
  removes the "stdlib-only Python" constraint cleanly; best runtime perf for
  the socket/watch hot loop.
- Cons: large single reviewable step; needs a full behavioral test port before
  merge; release/CI must produce binaries.

### B. Incremental strangler (module-by-module, Rust core called from Python)

Port leaf modules (socket, api, layout, impose) to a Rust library exposed to
Python (PyO3), shrinking Python over several releases.

- Pros: smaller PRs; continuous green.
- Cons: violates "no dual runtime / no shims"; forces PyO3 + Python + Rust
  toolchains on contributors simultaneously; the CLI/argparse and watch loop
  (the bulk) stay Python longest; churn with no user benefit until the end.
  Rejected as the primary path.

### C. Rewrite CLI+engine in Rust, keep sidebar TUI in Python

Port everything except the sidebar renderer.

- Pros: slightly less UI porting.
- Cons: keeps a Python runtime dependency for the *default `[run]` command*,
  defeating the goal; two languages forever. Rejected.

**Decision: Approach A.** It is the only one satisfying the repo's
clean-cutover rule and the user's "as much as possible in Rust".

## 4. Target architecture

Single Cargo binary crate `cmux-herdr` (workspace optional). Module map,
tracing the Python responsibilities 1:1 so review can diff behavior:

| Rust module | Replaces (Python) | Responsibility |
|---|---|---|
| `main.rs` / `cli/` | `bin/cmux-herdr` | clap CLI, 61 subcommands, dispatch, `--json`, `--timeout(-ms)`, `--version` from `VERSION` |
| `sidebar/` | `cmux_herdr_sidebar.py`, `bin/cmux-herdr-sidebar` | sidebar TUI: its own render loop, workspace tree, clip/wrap, terminal size, and event loop reading `CMUX_TUI_SOCKET`/legacy `CMUX_MUX_SOCKET`. This is the manifest `[run]` entry — it MUST keep the TUI/once/offline rendering behavior locked by `bridge/test_sidebar_tui_unit.py`, not merely shell out to `cmux-herdr` |
| `socket.rs` | `cmux_herdr_socket.py` | secure Unix socket, NDJSON, request ids, 512 KiB cap, subscriptions |
| `api.rs` | `cmux_herdr_api.py` | allowlist, forbidden methods, socket-first + CLI fallback, status extraction |
| `bridge.rs` (+ `model.rs`) | `cmux_herdr_bridge.py` | Pane/Tab/Workspace/Snapshot models, topology fetch, status-pill sync, host fingerprint, workspace resolve, diagnostics |
| `mirror.rs` | `cmux_herdr_mirror.py` | DesiredMirror/MirrorPlan, idempotent `herdr-mirror:<pane_id>` projection, scopes, prune, nesting refusal |
| `live.rs`, `engine.rs`, `impose.rs`, `layout.rs`, `io.rs`, `pump.rs`, `session.rs`, `host.rs`, `control.rs` | matching `cmux_herdr_*.py` | in-userspace window/layout model, event pump, input forwarding, layout parse/apply |
| `lifecycle.rs` | `cmux_herdr_lifecycle.py` | session discovery, socket-path/session-name validation, attach targets |
| `handoff.rs` | `cmux_herdr_handoff.py` | writer lease, XDG/App-Support state dirs, TTL/heartbeat/pid-alive |
| `state.rs` | scattered `_state_dir`/`_binding_path`/atomic writes | typed state store: schema-compatible with §2.6, rename-based atomicity (fsync added as a documented improvement) |
| `update/` | `cmux_herdr_update_config.py` + `herdr-auto-update.sh` | auto-update: managed TOML block, manifest fetch, serialized swap, rollback, uninstall |

Crate dependencies (request-only, minimal, pinned via `Cargo.lock`; if truly
offline builds are required, additionally `cargo vendor` into the tree — a
lockfile alone pins versions, it does not vendor source):
`clap` (CLI), `serde`/`serde_json` (protocol + state), `toml_edit`
(comment-preserving config ownership), `libc` or `rustix` (socket `stat`
mode/uid checks, `AF_UNIX`), `sha2` (backup checksum), `tempfile`,
`fs2`/`rustix` (advisory lock). Async: **blocking std I/O**, not tokio — the
socket is a single long-lived connection with a poll interval; a runtime adds
weight for no benefit. `crossterm` only if the current ANSI-by-hand sidebar
proves insufficient; first port keeps the existing raw-escape rendering.

## 5. Auto-update port (from dakesan/cmux-herdr)

Port the *semantics*, redesign the *mechanism* in Rust (per ForkDelta
guidance). Fold the shell runner and Python config helper into an
`cmux-herdr update-service` subcommand family so there is no Bash/Python at
runtime.

Ported semantics (kept):

- Opt-in only; disabled unless the user runs the installer subcommand.
- Managed, marker-delimited block under `[update]` in Herdr `config.toml`;
  refuses to overwrite a different active channel/manifest URL; idempotent.
- Atomic config write preserving comments/mode.
- Serialization preserves the fork's *outcomes* while changing the mechanism:
  the fork runner locks by atomically creating a `herdr-auto-update.lock`
  **directory** in XDG state, storing a PID in it, renaming a dead-owner
  directory aside, and refusing unexpected stale contents. The Rust port MAY
  use an advisory file lock (`fs2`/`rustix`) but MUST reproduce the dead-PID
  takeover and "busy → skip" contention outcomes.
- Herdr-compatibility preflight (from the fork installers, kept as observable
  safety): probe `herdr --default-config` to prove `update.manifest_url`
  support, run `herdr config check` before editing, and re-validate after the
  edit with rollback of the managed block if the post-edit check fails — so a
  syntactically valid but Herdr-incompatible config can never be installed.
- SHA-256 backup of the current binary; `herdr update --handoff`; restore the
  previous executable if replacement or post-swap version check fails.
- Reversible uninstall that removes only the managed block; uninstaller works
  without the plugin checkout present.

Hardened in the Rust design (improvements):

- `toml_edit` instead of a hand-rolled Py3.10 fallback parser — comments and
  formatting preserved deterministically.
- Bounded backup retention (keep N, prune oldest) instead of unbounded
  timestamped copies.
- Configurable manifest URL + channel (not silently pinned to `dakesan/herdr`);
  HTTPS-only enforced; record the resolved version/digest we installed so the
  post-swap check is exact, not best-effort.
- Transactional install: if unit registration (`launchctl`/`systemctl --user`)
  fails, roll back the config block and runtime payload.
- Self-contained: the binary embeds the uninstall logic; the persisted support
  payload becomes a versioned copy of the same binary, not a Bash script.
- No hardcoded `~/.local/bin/herdr`; resolve Herdr via PATH / `HERDR_BIN`.

Platform adapters: `update/launchd.rs` writes the plist + `launchctl
bootstrap`; `update/systemd.rs` writes `.service`/`.timer` + `systemctl --user
enable --now`. The two platforms do **not** share a schedule in the fork and
the port MUST NOT claim they do: the fork's systemd `.timer` uses
`OnBootSec=2min`, `OnUnitActiveSec=6h`, `RandomizedDelaySec=5min`, while the
fork's launchd plist uses `RunAtLoad=true` + `StartInterval=21600` (6h) with no
boot/randomized delay. The Rust design normalizes both to a ~6h cadence and, if
it adds a boot/randomized delay on launchd, MUST label that as a deliberate
behavior change, not fork parity. Critically, the fork's `watch` LaunchAgent
injects only `PATH` (login shell via `/bin/bash -lc`) and does **not** export
`HERDR_SOCKET_PATH`/`HERDR_ENV`; the generated Rust units MUST explicitly hand
off the socket/env contract (§2.7) or nested-context watch/update cannot
reproduce current behavior. Docs-only CHANGELOG/README churn from the fork is
not ported verbatim; new docs describe the Rust command.

## 6. Packaging & release

Default user path unchanged: `cmux sidebar plugin install`.

> **Reviewer NO-GO to clear before merge.** The current
> `.github/workflows/release.yml` runs Python unittest + tag/notes only (no
> Cargo build, no asset upload, no checksums), `.github/workflows/ci.yml` is
> Python-only, and there is no `Cargo.toml`/`Cargo.lock` yet.
> `tests/test_plugin_manager.py` asserts the `[build]` command is exactly
> `chmod +x` and the sidebar entry is a `#!/usr/bin/env python3` script. All of
> that must change atomically with the cutover; until it does, packaging is
> not implementable and the plugin cannot install a Rust binary.

- **Distribution: prebuilt, checksum-verified release binaries** (chosen).
  `release.yml` gains a Cargo build matrix producing `cmux-herdr` for
  `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, uploading each as
  `cmux-herdr-<version>-<target>` plus a `SHA256SUMS` file to a GitHub Release
  tagged from `VERSION`.
- **`[build]` contract (must be fully specified, not a bare argv).** The plugin
  manager runs `[build].command` as a subprocess in the checkout. Set it to a
  single committed, executable, dependency-free bootstrap script
  (`bin/cmux-herdr-fetch`, POSIX sh) that: (1) detects OS/arch and maps to a
  target triple; (2) resolves the version from `VERSION`; (3) downloads
  `https://github.com/RaviTharuma/cmux-herdr/releases/download/v<VERSION>/cmux-herdr-<VERSION>-<target>`
  and its `SHA256SUMS` over HTTPS (`curl`/`wget`, fail closed if neither);
  (4) verifies the SHA-256 before use; (5) installs atomically (temp file →
  `chmod +x` → rename) to a stable path that `bin/cmux-herdr` and
  `bin/cmux-herdr-sidebar` resolve to; (6) on any failure leaves an existing
  install untouched and exits non-zero with a clear message. Network/tool/hash
  behavior and the offline path are part of the contract, not left implicit.
- `bin/cmux-herdr` and `bin/cmux-herdr-sidebar` become tiny POSIX-sh launchers
  that `exec` the resolved binary (`sidebar` passes the `sidebar` subcommand),
  keeping the manifest `[run]` entry executable at its current path.
- Source build (`cargo build --release`, optionally `cargo vendor` for offline)
  is the documented fallback when no asset matches (unusual arch / air-gapped
  dev). The fetch script attempts it only when a Rust toolchain is present.
- `tests/test_plugin_manager.py` and any shebang/`chmod`-only assertions are
  updated in the same change to encode the new build/launcher contract.
- `VERSION` stays the single version source, read at runtime and by release;
  the release tag and asset names derive from it so they cannot drift.
- No telemetry/analytics (repo rule).

## 7. Test & verification strategy

The existing suite is the contract. Port it, do not weaken it — and note that
several invariants above are **not** covered by the current baseline, so the
port must *add* tests, not just translate them: `assert_socket_secure` reject
cases (symlink, non-socket, `mode & 0o077`, foreign uid), `MAX_LINE_BYTES`
oversized request/response, malformed UTF-8/JSON, response-id correlation, and
the exact 24-entry `DEFAULT_SUBSCRIPTIONS` set/order; the `--timeout-ms` alias
and precedence; the full `STATUS_STYLE` map incl. the out-of-set fallback; and
fsync/durability of the state writers. Capturing these as executable parity
tests is a precondition for the cross-check gate below.

- **Behavioral parity tests** re-expressed as Rust `#[test]` / integration
  tests: per-subsystem (api allowlist, socket validation, mirror plan
  idempotence, layout parse, lease TTL, update config idempotence/conflict/
  atomicity, service install/uninstall reversibility with a fake
  `launchctl`/`systemctl`). Mirrors `bridge/test_*` and `tests/test_*`.
- **CLI golden tests**: `--help`, `--version`, every `--json` command against
  a fake `herdr`/`cmux` on `PATH`, asserting identical output shape/exit codes
  to the Python baseline captured before deletion.
- **Cross-check gate**: before deleting Python, run both implementations
  against the same fake binaries and diff `--json` output for all read
  commands; any diff blocks cutover.
- **Smoke**: `cmux-herdr doctor`, `--version`, `--help`, `sidebar` render
  against a fake socket; auto-update install→update→rollback→uninstall on a
  temp HOME with fake `herdr`.
- CI: replace the Python matrix with `cargo test` + `cargo clippy -D warnings`
  + `cargo fmt --check` on macOS and Linux runners; keep the secret-scan step.

## 8. Sequencing

1. Scaffold Cargo crate, `VERSION` read, clap skeleton with all 61 commands
   returning `unimplemented` behind a feature gate; capture Python `--json`
   goldens.
2. Port leaf/pure modules first (layout, impose, api allowlist, socket
   validation, state store, handoff lease) with their tests.
3. Port bridge/model, status sync, mirror, live/engine/io/pump, sidebar TUI.
4. Port CLI dispatch wiring each command to the ported engine; pass goldens.
5. Port auto-update as `update-service` subcommands + platform adapters.
6. Cross-check gate green → delete `bridge/*.py`, `bin/*` Python, `scripts/*.sh`
   updater/runner; update `cmux-plugin.toml [build]`, CI, docs, CHANGELOG.
7. Release binaries; verify `cmux sidebar plugin install` end to end.

## 9. Risks

- **Hidden CLI/JSON contract drift** — mitigated by the golden + cross-check
  gate before any Python deletion.
- **Sidebar TUI fidelity** — raw ANSI escapes ported literally first; visual
  diff against Python render for identical socket input.
- **Socket `stat` portability** — `lstat` symlink/type + `mode & 0o077` +
  uid checks via `rustix`/`libc`; unit test the reject cases (symlink,
  group/other permission bit set, foreign uid) and the `0700`-allowed case.
- **Release/install trust** — checksum-verified assets; source-build fallback
  documented; `[build]` failure must not brick an existing install.
- **CONTRIBUTING/AGENTS drift** — repo currently advertises "stdlib-only
  Python, no compiler". That text and the dev workflow docs must be rewritten
  as part of cutover; contributors will need a Rust toolchain.
- **Plugin-manager contract lock** — `tests/test_plugin_manager.py` (and
  `tests/test_sidebar_native.py`) hard-assert `chmod +x` build and a
  `#!/usr/bin/env python3` sidebar. These fail the instant the manifest points
  at a Rust binary; they must be rewritten in the same commit that changes
  `[build]`/launchers, or CI blocks the cutover.
- **Untested baseline invariants** — several locked behaviors (socket reject
  cases, id correlation, full status map, state fsync) have no current tests
  (§7); porting "to green" would silently drop them. The parity tests must be
  authored from the source, not inferred from the existing suite.
- **Ambiguous-failure double mutation** — the Python `HerdrApi.call` retry can
  re-send a non-idempotent RPC; a naive port inherits the hazard. Decide the
  retry policy explicitly (§2.4).
