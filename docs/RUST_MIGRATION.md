# cmux-herdr → Rust migration

Status: implemented in the 0.7.0 clean cutover. Owner: RaviTharuma. This
document records the historical Python baseline and the resulting single Rust
binary, including the port of the optional Herdr auto-update service.

## 1. Goal and scope

The former Python runtime has been replaced by one Rust Cargo binary. The
release package includes the CLI, sidebar, socket client, state/handoff,
watch/mirror engine, and opt-in `update-service`. Users install a prebuilt,
checksum-verified binary through `cmux sidebar plugin install`; no Python or
Rust toolchain is required at runtime.

Historical baseline (for parity rationale): the deleted `bridge/*.py` modules
and Python launchers implemented these responsibilities. Current runtime code
lives under `src/*.rs`; `bin/*` are thin POSIX-sh launchers.

The migration also ported useful auto-update semantics from
`dakesan/cmux-herdr`; the service remains opt-in.

Out of scope (stays as-is, by design):

- `sidebars/herdr.js` / `herdr.swift` — already demoted to experimental
  leftovers by commit `aa7ccf9`; not the product. Not translated.
- Platform unit files (`launchd` plist, systemd `.service`/`.timer`) — these
  are the OS-native interface. The Rust binary generates/registers them; we do
  not reimplement `launchd`/`systemd`.
- Thin install/uninstall shell wrappers that only shell out to the binary.
- Agent skill (`agent-skill/SKILL.md`), docs, license, issue templates.

Non-negotiable: the plugin keeps installing through the official
`cmux sidebar plugin install` path defined by `cmux-plugin.toml`.
## 2. Compatibility invariants (must not regress)

The rewrite is a behavioral port, not a redesign. These contracts are locked
by existing tests and by live cmux/Herdr integration:

1. **Plugin manifest.** `cmux-plugin.toml` stays `kind = "sidebar"`, `[run]`
   command `bin/cmux-herdr-sidebar`. The sidebar entry is an executable POSIX-sh
   launcher that `exec`s the Rust binary with the `sidebar` subcommand. `[build]`
   invokes the fetch/verify bootstrap (§6), and plugin-manager tests cover this
   contract rather than a chmod-only build or Python shebang.
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
   membership *and* count). The historical Python baseline did not correlate
   response `id` to request `id`, serialize send/read across concurrent calls,
   validate the protocol version, or guard the lstat→connect TOCTOU. The Rust
   implementation adds correlation and per-connection serialization.
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

The selected big-bang approach produced one `cmux-herdr` Rust binary covering
CLI, sidebar TUI, watch/mirror engine, socket client, state store, and
auto-update. The former Python runtime was deleted after parity checks;
`bin/cmux-herdr` and `bin/cmux-herdr-sidebar` now resolve the binary.

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
| `main.rs` / `cli/` | historical `bin/cmux-herdr` | clap CLI, 61 subcommands, dispatch, `--json`, `--timeout(-ms)`, `--version` from `VERSION` |
| `sidebar/` | historical sidebar launcher/module | sidebar TUI render loop and event input; manifest `[run]` entry |
| `socket.rs` | historical socket module | secure Unix socket, NDJSON, request ids, 512 KiB cap, subscriptions |
| `api.rs` | historical API module | allowlist, forbidden methods, socket-first + CLI fallback, status extraction |
| `bridge.rs` (+ `model.rs`) | historical bridge module | Pane/Tab/Workspace/Snapshot models, topology fetch, status-pill sync, host fingerprint, workspace resolve, diagnostics |
| `mirror.rs` | historical `cmux_herdr_mirror.py` | DesiredMirror/MirrorPlan, idempotent `herdr-mirror:<pane_id>` projection, scopes, prune, nesting refusal |
| `live.rs`, `engine.rs`, `impose.rs`, `layout.rs`, `io.rs`, `pump.rs`, `session.rs`, `host.rs`, `control.rs` | historical `cmux_herdr_*.py` modules | in-userspace window/layout model, event pump, input forwarding, layout parse/apply |
| `lifecycle.rs` | `cmux_herdr_lifecycle.py` | session discovery, socket-path/session-name validation, attach targets |
| `handoff.rs` | `cmux_herdr_handoff.py` | writer lease, XDG/App-Support state dirs, TTL/heartbeat/pid-alive |
| `state.rs` | scattered `_state_dir`/`_binding_path`/atomic writes | typed state store: schema-compatible with §2.6, rename-based atomicity (fsync added as a documented improvement) |
| `update/` | historical `cmux_herdr_update_config.py` and `herdr-auto-update.sh` | opt-in auto-update: managed TOML block, manifest fetch, serialized swap, rollback, uninstall |

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

The fork's update semantics were ported into the Rust `cmux-herdr
update-service` subcommand family. There is no Bash/Python runtime dependency.

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

The former packaging gap is resolved: the manifest now invokes the committed
`bin/cmux-herdr-fetch` bootstrap, and release automation builds and publishes
four checksum-verified Rust targets. The launchers are thin POSIX-sh scripts;
`[build]` is no longer a chmod-only operation.

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
- `tests/plugin_contract.rs` encodes the build, launcher, and sidebar-subcommand
  contract that replaced the former Python plugin-manager tests.
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
  `launchctl`/`systemctl`). These cover the historical `bridge/test_*` and
  `tests/test_*` contracts.
- **CLI golden tests**: `--help`, `--version`, every `--json` command against
  a fake `herdr`/`cmux` on `PATH`, asserting identical output shape/exit codes
  to the Python baseline captured before deletion.
 - **Cross-check gate (completed):** the Rust implementation was compared with
   the historical Python baseline against the same fake binaries before the
   cutover.
- **Smoke**: `cmux-herdr doctor`, `--version`, `--help`, `sidebar` render
  against a fake socket; auto-update install→update→rollback→uninstall on a
  temp HOME with fake `herdr`.
 - CI runs `cargo test` + `cargo clippy -D warnings` + `cargo fmt --check` on
   macOS and Linux runners; the secret-scan step remains.

## 8. Sequencing

1. Scaffolded and ported the Cargo crate, preserving CLI and protocol contracts.
2. Ported leaf modules, engine, sidebar, and update service with parity tests.
3. Completed the cross-check gate, removed the Python runtime, and updated
   packaging, CI, and docs.
4. Configured the four-target release workflow and verified bootstrap and
   launcher contracts in hermetic Rust tests.

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
- **Contributor workflow (resolved)** — docs now require Rust/Cargo for source
  builds; users receive prebuilt binaries and need neither Python nor Rust.
- **Plugin-manager contract (resolved)** — bootstrap and thin launcher tests
  cover the checksum-verified release asset path.
- **Baseline invariants** — parity tests cover socket reject cases, id
  correlation, status mapping, and state durability.
- **Ambiguous-failure mutation** — the Rust API documents its retry policy for
  non-idempotent RPCs.
