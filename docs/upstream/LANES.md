# Plugin vs native tracks

This plugin repository (`RaviTharuma/cmux-herdr`) is the **userspace** track.
Native Herdr support lands on [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux)
(and the maintainer fork [RaviTharuma/cmux](https://github.com/RaviTharuma/cmux)).

Keep the tracks separate:

| Track | Lives in | Do not |
|---|---|---|
| Plugin CLI / bridge | this repo, `main` | Put Swift/AppKit here |
| Hidden `__herdr-compat` | cmux PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) | Mix plugin CLI shims into that PR |
| Nested topology / window mirror | cmux PR [#10045](https://github.com/manaflow-ai/cmux/pull/10045) | Force-push that branch from plugin work |

Shared contracts belong in tests and docs, not by merging the two PRs
together. Machine-readable snapshot: [STATUS.json](./STATUS.json).

If you must touch a file another branch just changed: fetch, rebase, and
open a follow-up PR. Do not force-push shared history.

Current native-side artifacts (impose, I/O, control, lifecycle, live apply)
have Python twins on plugin `main`. Native twins belong on the cmux fork,
not in this tree.
