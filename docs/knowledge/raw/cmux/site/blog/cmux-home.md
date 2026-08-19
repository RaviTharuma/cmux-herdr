# cmux home

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)June 23, 2026

People keep asking whether we'll add git worktrees to cmux. We're not going to, because we don't want to impose worktrees on everyone. cmux is [a primitive, not a solution](https://cmux.com/blog/zen-of-cmux).

Everyone works differently. Some projects live across several git repos. Some people keep many checkouts of the same repo (my cofounder has clones named cmux0 through cmux40, for reasons I still don't understand). Some people default to SSH and remote development.

cmux is just terminals and browsers, with a CLI to drive them. You can write a few bash scripts to manage worktrees, juggle multiple checkouts, or open remote sessions, and lay out your workspace however fits your head.

cmux should bend to your workflow instead of forcing one on you. We want cmux to feel like home.

[cmux-home](https://github.com/manaflow-ai/cmux-home) is one example of what that looks like. It's a small Rust TUI that starts Claude and Codex workspaces and watches their live status, built entirely on cmux primitives. The launcher uses command templates, so you can swap in your own scripts for creating worktrees, picking a checkout, or SSHing into a VM. Fork it and make it yours.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[Introducing cmux Fork](https://cmux.com/blog/cmux-fork) [cmux history](https://cmux.com/blog/cmux-history)

Canonical: https://cmux.com/blog/cmux-home
