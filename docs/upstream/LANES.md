# Agent lanes (do not collide)

Two chats are pushing Herdr ↔ cmux tmux-depth at the same time.
**Read this before touching the cmux fork.**

## Live ownership (2026-08-17)

| Lane | Who | What they own | Do not touch |
|---|---|---|---|
| **A. #10045 review** | Other chat (CodeRabbit 1/5 at `f8d64f9f`, more coming) | Existing files on `cursor/nested-topology-herdr-v1-becf`: `HerdrNestedTopologyClient*`, coordinator, controller, AppDelegate, Workspace, xcstrings, review nits | New impose/host-apply files; do not rewrite the planner |
| **B. Tmux-depth contract** | This chat (`bc-19886765-137b-4458-bb10-d6b48d0d6e7a`) | **New files only.** Plugin impose/host/io/session/control/**lifecycle**. Native twins on **separate** fork branches | Do **not** commit onto `cursor/nested-topology-herdr-v1-becf` until lane A finishes 5/5 |

## Rules

1. **Never push to `cursor/nested-topology-herdr-v1-becf` from lane B** while lane A is mid CodeRabbit series (`(N/5)` commits).
2. Lane B opens fork PRs against that branch as **draft** and leaves them unmerged so lane A can rebase/pick after 5/5.
3. Do not edit `AppDelegate.swift`, `NestedTopologyController.swift`, `Workspace*.swift`, `Localizable.xcstrings`, or `HerdrNestedTopologyClient*.swift` from lane B.
4. Plugin `main` (`RaviTharuma/cmux-herdr`) is lane B’s merge target. Lane A should not need it.
5. If you must fix a file the other lane just touched, **wait one fetch** and rebase — do not force-push shared history.

## Current artifacts

- Plugin impose planner: [cmux-herdr#21](https://github.com/RaviTharuma/cmux-herdr/pull/21) (merged)
- Native impose planner: squash `aeb11e08` already on #10045 (landed before lane A’s 1/5)
- Native host-apply twin: branch `cursor/herdr-host-apply-6e7a` (draft [RaviTharuma/cmux#12](https://github.com/RaviTharuma/cmux/pull/12), not merged into #10045)
- Plugin I/O + session host: `bridge/cmux_herdr_io.py` + `bridge/cmux_herdr_session.py` ([cmux-herdr#23](https://github.com/RaviTharuma/cmux-herdr/pull/23), merged)
- Native I/O + session twins: branch `cursor/herdr-io-session-6e7a` (draft [RaviTharuma/cmux#13](https://github.com/RaviTharuma/cmux/pull/13), not merged into #10045)
- Plugin control-depth: `bridge/cmux_herdr_control.py` ([cmux-herdr#25](https://github.com/RaviTharuma/cmux-herdr/pull/25), merged)
- Plugin attach/detach/restore/observability: `bridge/cmux_herdr_lifecycle.py` ([cmux-herdr#26](https://github.com/RaviTharuma/cmux-herdr/pull/26), merged)
- Plugin live apply machine: `bridge/cmux_herdr_live.py` (this slice — makePanel/output/drag/focus/size/attach)
- Native lifecycle twin: draft [RaviTharuma/cmux#14](https://github.com/RaviTharuma/cmux/pull/14)
