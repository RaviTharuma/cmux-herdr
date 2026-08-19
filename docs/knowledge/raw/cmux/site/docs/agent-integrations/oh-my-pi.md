# [#](#title)oh-my-pi

# [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi#title)oh-my-pi

oh-my-pi is a fork of Pi rewritten as a coding-first agent. Its binary is omp, and cmux integrates with it through the omp hooks extension.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi#setup-usage)Setup and usage

```
bun install -g @oh-my-pi/pi-coding-agent
# or
brew install can1357/tap/omp

cmux hooks setup omp
# or
cmux hooks omp install
```

Install omp, then install the cmux hooks extension. The extension is written to ~/.omp/agent/extensions/cmux-omp-session.ts, and cmux upgrades it in place when the bundled version changes.

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi#what-you-get)What you get

The hooks extension lets omp report lifecycle and session information back to cmux:

-   Workspace busy and idle status from omp turn boundaries
-   Turn-end notifications in cmux
-   Session tracking so closed panes can be resumed from the session index with omp --session SESSION\_ID or forked with omp --fork SESSION\_ID
-   Workspace auto-naming, with omp available as the summarizer model
-   Task Manager process attribution for omp processes

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi#directories)Directories

cmux reads omp data from the agent directory. PI\_CODING\_AGENT\_DIR or PI\_CONFIG\_DIR can override that directory.

| Path | Purpose |
| --- | --- |
| `~/.omp/agent/extensions/cmux-omp-session.ts` | cmux hooks extension installed and upgraded by cmux |
| `~/.omp/agent/sessions` | omp session files that cmux reads for restore and fork workflows |

## [#](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi#env-vars)Environment variables

| Variable | Purpose |
| --- | --- |
| `CMUX_OMP_HOOKS_DISABLED=1` | Disables the omp hooks extension installed by cmux |
| `CMUX_OMP_CMUX_BIN` | Overrides the cmux binary that the hooks extension invokes |

[oh-my-codex](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-codex) [oh-my-claudecode](https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-claudecode)

Canonical: https://cmux-docs-release.vercel.app/docs/agent-integrations/oh-my-pi
