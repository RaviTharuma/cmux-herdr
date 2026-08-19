# Superrepos and Why Claude Code Is the Best Worktree Manager

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)July 23, 2026

Different workloads want different forms of parallelism. Worktrees suit ordinary Git changes. Multiple checkouts help when submodules or build tools dislike worktrees. E2B, Freestyle, Daytona, and Modal provide remote sandboxes, while GitHub Actions can be pressed into the same role. Docker, UTM, Lima, and OrbStack cover local isolation. Some teams use Mac minis or Linux boxes. ML experiments often belong on Ray, Modal, or Slurm. Sometimes the fastest answer is to yolo everything on main.

A useful manager should choose among these execution models from the task context. This follows the [Zen of cmux](https://cmux.com/blog/zen-of-cmux): keep the primitive composable, then let project rules decide how to use it.

## A superrepo is project headquarters

I want to call this control repo a superrepo. In the context of coding agents, a superrepo sits above one or more source repositories and holds the data, skills, origins, worktrees, and operating rules that turn a prompt into an isolated task environment.

```
~/my-superrepo/
├── AGENTS.md
├── skills/
├── data/
├── origins/
│   └── [repo]/
└── worktrees/
    └── [task]/
        └── [repo]/
```

The `AGENTS.md` file is the control plane. It names the repositories an agent may need, defines where each task worktree belongs, and points to per-repo setup instructions. One task can create matching worktrees for a terminal, browser integration, and router without cloning all three for every task.

AGENTS.md

```
You are working in cmux-hq. Tasks may involve:
- manaflow-ai/cmux
- manaflow-ai/cmux-browser
- manaflow-ai/coderouter

When the user asks for a new task:
1. Decide which repositories the task needs.
2. Create matching worktrees in worktrees/[task]/[repo].
3. Read each worktree's AGENTS.md.
4. Start its setup scripts immediately, then continue the task.
```

## Let the agent choose the topology

cmux uses this pattern internally. The public source lives in [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux), while our private control repo is `manaflow-ai/cmuxterm-hq`. From the hq root, we can ask Codex or Claude Code to fix a CodeRouter problem that may touch cmux. The agent creates only the worktrees it needs, reads each repository's AGENTS.md, starts its setup scripts, and begins inspecting code while those scripts run.

```
cd ~/fun/cmux-hq
codex --yolo "fix CodeRouter xyz issues, might relate to cmux in xyz way"
```

A `./scripts/new-worktree.sh` command implements one topology. An agent can choose a second checkout for incompatible submodules, a remote sandbox for untrusted setup, a VM for another operating system, or a GPU scheduler for experiments, then reuse scripts for the deterministic pieces.

## Why this changes the clock time

Worktrees are only half the parallelism story for many projects. In ML, one coding agent can write several experiments in one worktree while a scheduler fans the runs out across GPUs. Creating more worktrees would add coordination without adding compute.

Agent orchestration can also shorten startup. The agent can read and edit code as soon as the checkout exists while dependency installs, builds, and local services continue in the background. A wrapper that finishes every setup step before starting the agent puts the slowest prerequisite on the critical path.

## The tradeoffs

Using Claude Code or Codex as the manager spends more tokens and consumes part of the context window on repository topology. Starting at the superrepo root is also less familiar than starting inside a single checkout, so the instructions need to be explicit. cmux supplies [custom commands](https://cmux.com/docs/custom-commands#new-workspace-button), a programmable [CLI and socket API](https://cmux.com/docs/api), and reusable [skills](https://cmux.com/docs/skills) for those instructions to call.

For us, the flexibility is worth that cost. [cmux home](https://cmux.com/blog/cmux-home) gives the workflow a surface, while the superrepo tells the agent how to assemble it for each task. The best worktree manager can decide when a worktree is the wrong unit of parallelism.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[How I used 367 billion tokens in 30 days](https://cmux.com/blog/367-billion-tokens) [Introducing cmux Fork](https://cmux.com/blog/cmux-fork)

Canonical: https://cmux.com/blog/claude-code-best-worktree-manager
