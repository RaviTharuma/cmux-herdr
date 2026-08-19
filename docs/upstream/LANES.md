# Agent lanes (do not collide)

Two chats are pushing Herdr ↔ cmux tmux-depth at the same time.
**Read this before touching the cmux fork.**

## Live ownership (2026-08-19)

| Lane | Who | What they own | Do not touch |
|---|---|---|---|
| **A. #10045 tip** | Shared / quiet after CR **0**/173 | Tip `cursor/nested-topology-herdr-v1-becf` @ `b02b8a954327` | Force-push without fetch; PLACEHOLDER recoveries only |
| **B. Plugin + contracts** | `cmux-herdr` integration_ops | Plugin `main`, docs freeze, Herdr-beyond-tmux CLI, fork side PRs | Do not rewrite hot tip without a quiet tip + Actions fold |

CodeRabbit on `#10045` / `#8736` is cleared (0 unresolved). Merge still needs maintainer approval (`BLOCKED` / `UNSTABLE`).

## Rules

1. Prefer side branches + one-shot Actions / MCP `push_files` when folding onto `#10045` tip.
2. Plugin and native stay separate tracks (`#8736` vs `#10045`).
3. Before editing hot `RemoteHerdr*` files, re-fetch tip SHA from [STATUS.json](./STATUS.json).
4. Plugin `main` (`RaviTharuma/cmux-herdr`) is the merge target for userspace work.
5. If you must fix a file another lane just touched, **wait one fetch** and rebase — do not force-push shared history.

## Current artifacts

- Plugin impose / I/O / control / lifecycle / live apply / handoff / SessionHost pump: on plugin `main`
- Native twins mostly on `#10045` tip; `RemoteHerdrHandoff` twin still landing from fork [#18](https://github.com/RaviTharuma/cmux/pull/18)
- Stacked fork drafts [#12](https://github.com/RaviTharuma/cmux/pull/12)–[#17](https://github.com/RaviTharuma/cmux/pull/17) are largely superseded by tip content — close after verifying
- Errors/lackings freeze: [ERRORS_AND_LACKINGS.md](./ERRORS_AND_LACKINGS.md)
- Herdr-beyond-tmux: [HERDR_BEYOND_TMUX.md](./HERDR_BEYOND_TMUX.md)
