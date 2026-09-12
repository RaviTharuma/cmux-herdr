# Privacy

**cmux-herdr** is a local CLI plugin. It does not operate a cloud backend, user
accounts, analytics pipeline, or crash-reporting service of its own.

## Data this plugin may touch

| Data | Where it lives | Shared? |
|---|---|---|
| Association / parent-binding cache | `$XDG_STATE_HOME/cmux-herdr/` (default `~/.local/state/cmux-herdr/`) | No — stays on your disk |
| Writer lease / restore hints | Same state directory | No |
| Host fingerprint inputs | Env (`CMUX_SURFACE_ID`, `HERDR_SOCKET_PATH`, …) | Used locally to select cache files |
| Pane content via `attach-pane` / `read-*` | Passed through Herdr / cmux CLIs | Not uploaded by this plugin |
| Release binary download | GitHub Releases over HTTPS (`bin/cmux-herdr-fetch`) | Standard GitHub traffic |

The plugin shells out to `herdr` and `cmux`. Those tools have their own
behavior; this document only covers **cmux-herdr**.

## What we ask you not to share

When filing issues or PRs, redact personal paths, hostnames, employer or
client workspace names, tokens, and live session dumps. See
[SUPPORT.md](SUPPORT.md) and [SECURITY.md](SECURITY.md).

## Third parties

Installing from GitHub Releases and browsing this repository involves GitHub
(Microsoft) as the host. Review GitHub's own privacy policy for that traffic.
