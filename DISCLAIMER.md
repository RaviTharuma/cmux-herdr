# Disclaimer

**cmux-herdr** is an independent, community-maintained plugin. It is **not**
an official product of [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux),
[herdrdev/herdr](https://github.com/herdrdev/herdr), or their employers.

## No warranty

This software is provided **as is**, without warranty of any kind, as stated in
the [MIT License](LICENSE). The maintainers are not liable for data loss, broken
sessions, incorrect status projection, or any other damage arising from use.

## What this plugin is (and is not)

| This plugin **is** | This plugin **is not** |
|---|---|
| A user-installed cmux plugin | A patch to `cmux.app` or Herdr |
| External pane viewers + status pills | Native Herdr UI / Ghostty PTY takeover |
| A bridge that shells out to `cmux` and `herdr` | A replacement for either CLI |
| Best-effort local tooling | A cloud service, SaaS, or hosted product |

Shipped versus planned capabilities are defined in the README
[capability matrix](README.md#capability-matrix) and [OPEN.md](OPEN.md).
Design notes under `docs/upstream/` describe **targets**, not guarantees.

## Third-party names

"cmux", "Herdr", "Ghostty", "tmux", and related marks belong to their respective
owners. Use of those names here is descriptive only and does not imply
endorsement, partnership, or affiliation.

## Security and privacy

- There is **no** cmux-herdr cloud backend, account system, or telemetry of
  its own. See [PRIVACY.md](PRIVACY.md).
- Local state under `$XDG_STATE_HOME/cmux-herdr/` stays on your machine.
- Report vulnerabilities privately per [SECURITY.md](SECURITY.md). Do not
  paste secrets, live session dumps, or employer workspace names into public
  issues.

## Upstream bugs

Bugs in cmux or Herdr themselves belong on those projects' trackers. See
[SUPPORT.md](SUPPORT.md) for where to file what.
