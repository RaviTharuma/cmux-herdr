# How I used 367 billion tokens in 30 days

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)July 29, 2026

I used 367 billion tokens in the last 30 days. My peak day was 37 billion tokens on GPT 5.6 Sol.

[Watch the video on YouTube](https://www.youtube.com/watch?v=YOst-qdMW0o).

## The entire workflow

I run one task per cmux workspace with vanilla Codex or Claude Code. Each agent notifies me when it needs a review.

1.  Press [Cmd+Shift+U](https://cmux.com/blog/cmd-shift-u). cmux jumps to the newest unread notification and focuses the exact workspace and pane.
2.  Review the dev build, artifact, or bug fix. Reply to the agent.
3.  Press Cmd+Shift+U again and repeat.
4.  When no notifications are waiting, create a workspace and start another task.

That loop is the 80/20. cmux keeps active work visible and turns completed agent work into a review queue.

## No custom harness

I use vanilla Claude Code and Codex. Their built-in subagents handle the occasional delegated task.

I do not use a custom agent harness or external orchestration. I do not use `/loop`, and I rarely use `/goal`.

## The economics

At API prices, this usage would have cost about $85,000. We paid $7,400 for 37 ChatGPT Pro 20x accounts and used the quota resets Tibo issued. [CodexBar](https://codexbar.app/) produced the local token and API-cost estimates.

The 367-billion figure still understates the accounts' total usage. [CodeRouter](https://cmux.com/dashboard/coderouter) lets my cofounder share the same account pool, while CodexBar records only the local token usage on my machine.

The workflow keeps my attention on decisions. I review completed work and unblock agents; when the queue clears, I start another task.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[Superrepos and Why Claude Code Is the Best Worktree Manager](https://cmux.com/blog/claude-code-best-worktree-manager)

Canonical: https://cmux.com/blog/367-billion-tokens
