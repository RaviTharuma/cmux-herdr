# Launching cmux on Show HN

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)February 21, 2026

We posted cmux on [Show HN](https://news.ycombinator.com/item?id=47079718) on Feb 19:

> I run a lot of Claude Code and Codex sessions in parallel. I was using Ghostty with a bunch of split panes, and relying on native macOS notifications to know when an agent needed me. But Claude Code's notification body is always just "Claude is waiting for your input" with no context, and with enough tabs open, I couldn't even read the titles anymore.
>
> I tried a few coding orchestrators but most of them were Electron/Tauri apps and the performance bugged me. I also just prefer the terminal since GUI orchestrators lock you into their workflow. So I built cmux as a native macOS app in Swift/AppKit. It uses libghostty for terminal rendering and reads your existing Ghostty config for themes, fonts, colors, and more.
>
> The main additions are the sidebar and notification system. The sidebar has vertical tabs that show git branch, working directory, listening ports, and the latest notification text for each workspace. The notification system picks up terminal sequences (OSC 9/99/777) and has a CLI (cmux notify) you can wire into agent hooks for Claude Code, OpenCode, etc. When an agent is waiting, its pane gets a blue ring and the tab lights up in the sidebar, so I can tell which one needs me across splits and tabs. Cmd+Shift+U jumps to the most recent unread.
>
> The in-app browser has a scriptable API. Agents can snapshot the accessibility tree, get element refs, click, fill forms, evaluate JS, and read console logs. You can split a browser pane next to your terminal and have Claude Code interact with your dev server directly.
>
> Everything is scriptable through the CLI and socket API: create workspaces/tabs, split panes, send keystrokes, open URLs in the browser.

At peak it hit #2 on Hacker News. Mitchell Hashimoto shared it:

My favorite comment from the [HN thread](https://news.ycombinator.com/item?id=47079718):

> Hey, this looks seriously awesome. Love the ideas here, specifically: the programmability (I haven't tried it yet, but had been considering learning tmux partly for this), layered UI, browser w/ api. Looking forward to giving this a spin. Also want to add that I really appreciate Mitchell Hashimoto creating libghostty; it feels like an exciting time to be a terminal user.
>
> Some feedback (since you were asking for it elsewhere in the thread!):
>
> -   It's not obvious/easy to open browser dev tools (cmd-alt-i didn't work), and when I did find it (right click page → inspect element) none of the controls were visible but I could see stuff happening when I moved my mouse over the panel
> -   Would be cool to borrow more of ghostty's behavior:
>     -   hotkey overrides
>     -   command palette (cmd-shift-p)
>     -   cmd-z to "zoom in" to a pane
>
> — [johnthedebs](https://news.ycombinator.com/item?id=47083596)

Surprisingly, cmux went viral in Japan:

Translation: "This looks good. A Ghostty-based terminal app designed so you don't get lost running multiple CLIs like Claude Code in parallel. The waiting-for-input panel gets a blue frame, and it has its own notification system."

And semi-viral in China:

Another exciting thing was seeing people build on top of the cmux CLI. sasha built a pi-cmux extension that shows model info, token usage, and agent state in the sidebar:

Everything in cmux is scriptable through the CLI: creating workspaces, sending keystrokes, controlling the browser, reading notifications. Part of the cmux philosophy is being programmable and composable, so people can customize the way they work with coding agents. The state of the art for coding agents is changing fast, and you don't want to be locked into an inflexible GUI orchestrator that can't keep up.

If you're running multiple coding agents, [give cmux a try](https://github.com/manaflow-ai/cmux).

![cmux GitHub star history showing growth from near 0 to 900+ stars after the Show HN launch](https://cmux.com/_next/image?url=%2F_next%2Fstatic%2Fimmutable%2Fmedia%2Fstar-history.2ty4kfbm1ai_2.png&w=3840&q=75)

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[The Zen of cmux](https://cmux.com/blog/zen-of-cmux) [Introducing cmux](https://cmux.com/blog/introducing-cmux)

Canonical: https://cmux.com/blog/show-hn-launch
