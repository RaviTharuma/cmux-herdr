# Development lanes (plugin vs native)

Updated when the upstream snapshot in [STATUS.json](./STATUS.json) is
refreshed. This is **coordination for people working on both tracks**, not
user documentation. Plugin users can ignore it.

## Lanes

| Lane | Owns | Leave alone unless you own it |
|---|---|---|
| Native nested topology | cmux PR [#10045](https://github.com/manaflow-ai/cmux/pull/10045) | Force-push; drive-by refactors of mirror hosts mid-review |
| Native compat dispatcher | cmux PR [#8736](https://github.com/manaflow-ai/cmux/pull/8736) | Mixing this plugin's CLI into that PR |
| This plugin | `RaviTharuma/cmux-herdr` `main` | Rewriting cmux Swift from here |
| Docs in `docs/upstream/` | Design notes for the native track | Overwriting a rewrite in flight |

## Rules

1. Native product code lands on the current `#10045` tip after `git fetch`.
   No force-push except recovering a known-broken tip.
2. Plugin and native stay separate (`#8736` vs `#10045`).
3. Before editing hot `RemoteHerdr*` files on the cmux fork, re-read
   [STATUS.json](./STATUS.json).
4. Do not commit live session dumps, tokens, or `.env` files.

## Snapshot

See [`STATUS.json`](./STATUS.json) for tip SHAs and review counts.
Errors/lackings freeze: [`ERRORS_AND_LACKINGS.md`](./ERRORS_AND_LACKINGS.md).
