# Upstream review-thread process (mandatory)

Applies to every fix/commit that addresses feedback on upstream PRs
(especially https://github.com/manaflow-ai/cmux/pull/10045).

## After every fix or commit

1. **Reply** on each relevant CodeRabbit / Greptile / human review thread with
   what changed (or an explicit won’t-fix + reason).
2. **Resolve** the conversation only after that reply is posted.
3. Never silent-resolve. Never leave fixed findings unanswered.

## Ready payloads for #10045

- Reply one-liners: [PR10045_CODERABBIT_REPLIES.md](./PR10045_CODERABBIT_REPLIES.md)
- Comment-ID map: [patches/pr10045-thread-replies.json](./patches/pr10045-thread-replies.json)
- Poster script: [patches/post-pr10045-replies.sh](./patches/post-pr10045-replies.sh)

## Auth note

Posting/resolving on `manaflow-ai/cmux` requires a GitHub token for
`RaviTharuma` with write access to that repository (classic `public_repo` or
fine-grained write on `manaflow-ai/cmux`). Repo-scoped PATs that only cover
owned forks return `403 Resource not accessible by personal access token`.
