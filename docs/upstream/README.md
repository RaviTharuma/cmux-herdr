# Upstream design notes (cmux native track)

These files are **not** required to install or run `cmux-herdr`.

They are working notes and paste-ready drafts for giving **cmux itself**
(Herdr nested topology, window mirror, ssh-tmux parity). The live discussion
lives on GitHub, not here:

- Poll: https://github.com/manaflow-ai/cmux/discussions/10106
- Design issue: https://github.com/manaflow-ai/cmux/issues/8737
- Compat dispatcher PR: https://github.com/manaflow-ai/cmux/pull/8736
- Nested topology PR: https://github.com/manaflow-ai/cmux/pull/10045

If a draft here disagrees with those links, **believe GitHub**.

| File | Contents |
|---|---|
| [TMUX_PARITY.md](TMUX_PARITY.md) | Plugin vs native vs tmux contract |
| [HERDR_BEYOND_TMUX.md](HERDR_BEYOND_TMUX.md) | Herdr verbs with no tmux analogue |
| [ERRORS_AND_LACKINGS.md](ERRORS_AND_LACKINGS.md) | Frozen inventory of gaps |
| [STATUS.json](STATUS.json) | Machine-readable upstream snapshot |
| [ISSUE.md](ISSUE.md) / [DESIGN.md](DESIGN.md) / [PR_PLAN.md](PR_PLAN.md) | Paste-ready native proposal |
| [ANNOYANCES.md](ANNOYANCES.md) | Historical engineering diary (blunt, dated) |
| [LANES.md](LANES.md) / [AGENT_LANES.md](AGENT_LANES.md) | How plugin vs native branches stay apart |

To **use** the plugin, go back to the [root README](../../README.md).
