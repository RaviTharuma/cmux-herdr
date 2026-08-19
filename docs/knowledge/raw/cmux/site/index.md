# cmux

[![cmux icon](https://cmux.com/logo.png)](https://cmux.com/)


The terminal built for multitasking, organization, and programmability.

Free and open source native macOS terminal built on Ghostty. Vertical tabs, notification rings when agents need attention, split panes, and a [CLI](https://cmux.com/docs/api) for programmability.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

## Features

-   **Vertical tabs**: sidebar shows git branch, working directory, ports, and notification text
-   **Notification rings**: panes light up when agents need attention
-   **In-app browser**: split a browser alongside your terminal with a scriptable API
-   **Split panes**: horizontal and vertical splits within each tab
-   **Programmable**: CLI and socket API for automation and scripting
-   **GPU-accelerated**: powered by libghostty for smooth rendering
-   **Lightweight**: native Swift + AppKit, no Electron
-   **Open source**: free and GPL-licensed
-   **Keyboard shortcuts**: [extensive shortcuts](https://cmux.com/docs/keyboard-shortcuts) for workspaces, splits, browser, and more
-   **[iOS companion](https://github.com/manaflow-ai/cmux#founders-edition)**: your terminals sync to iPhone and iPad in realtime

![cmux terminal app screenshot](https://cmux.com/_next/image?url=%2F_next%2Fstatic%2Fimmutable%2Fmedia%2Flanding-image.1y26igml71ekg.png&w=3840&q=85)

[![cmux iOS app mirroring a live agent terminal](https://cmux.com/_next/image?url=%2F_next%2Fstatic%2Fimmutable%2Fmedia%2Flanding-iphone.30uuk4f1z7m5c.png&w=3840&q=75)](https://cmux.com/ios)

## FAQ

How does cmux relate to Ghostty?

cmux is not a fork of Ghostty. It uses [libghostty](https://github.com/ghostty-org/ghostty) as a library for terminal rendering, the same way apps use WebKit for web views. Ghostty is a standalone terminal; cmux is a different app built on top of its rendering engine.

What platforms does it support?

Open the Download menu to see available cmux builds.  for any platform that is not released yet.

Is there an iOS app?

Yes, in beta. Pair your iPhone with your Mac from the Mobile Connect window and attach to your terminals from your phone, with optional forwarding of terminal notifications. It ships on TestFlight as cmux BETA. Early access is included with [cmux Founders Edition](https://github.com/manaflow-ai/cmux#founders-edition).

What coding agents does cmux work with?

All of them. cmux is a terminal, so any agent that runs in a terminal works out of the box: Claude Code, Codex, OpenCode, Gemini CLI, Kiro, Aider, Goose, Amp, Cline, Cursor Agent, and anything else you can launch from the command line.

Can cmux orchestrate multiple agents and subagents?

Yes. When an agent spawns subagents or teammates, cmux turns them into native panes and splits instead of hidden background processes. It supports [Claude Code teams](https://cmux.com/docs/agent-integrations/claude-code-teams) and [oh-my-opencode](https://cmux.com/docs/agent-integrations/oh-my-opencode) multi-model orchestration, so every agent in a run is visible and controllable.

Can I use cmux with remote machines?

Yes. Open workspaces over SSH and attach to remote tmux sessions, so agents can run on a remote host while you drive them from cmux. See [SSH and remote](https://cmux.com/docs/ssh).

How do notifications work?

When a process needs attention, cmux shows notification rings around panes, unread badges in the sidebar, a notification popover, and a macOS desktop notification. These fire automatically via standard terminal escape sequences (OSC 9/99/777), or you can trigger them with the [cmux CLI](https://cmux.com/docs/notifications#cli-usage) and [agent hooks](https://cmux.com/docs/notifications#integration-examples). Any agent that supports hooks or OSC works, including Claude Code, Codex, OpenCode, and pi.

Is cmux programmable?

Yes. Every action is available through the cmux CLI and a Unix socket: create workspaces, open split panes, send input, read screen contents, take screenshots, and drive the in-app browser. See the [CLI reference](https://cmux.com/docs/api) and [browser automation](https://cmux.com/docs/browser-automation) docs.

What can the built-in browser do?

cmux can split a real browser pane next to your terminal, and it is fully programmable: navigate, snapshot the DOM, click, type, evaluate JavaScript, and read console and network activity over the same socket API. Agents use it to verify their own web changes without leaving cmux. See [browser automation](https://cmux.com/docs/browser-automation).

Does cmux have skills?

Yes. Skills are reusable workflows you can give any agent running in cmux, for things like CLI control, workspace automation, settings, and browser surfaces. Browse the open collection at [cmux-skills](https://github.com/manaflow-ai/cmux-skills), or read the [skills docs](https://cmux.com/docs/skills).

Can I customize keyboard shortcuts?

Terminal keybindings are read from your Ghostty config file (`~/.config/ghostty/config`). cmux-specific shortcuts (workspaces, splits, browser, notifications) can be customized in Settings. See the [default shortcuts](https://cmux.com/docs/keyboard-shortcuts) for a full list.

Can I customize cmux?

Yes. Terminal rendering uses your Ghostty config, so themes, fonts, colors, and cursor carry over directly. cmux's own settings in `~/.config/cmux/cmux.json` control the sidebar, tab bar, split panes, and behavior, and every [keyboard shortcut](https://cmux.com/docs/keyboard-shortcuts) is editable. See [configuration](https://cmux.com/docs/configuration).

Are my sessions saved?

Yes. cmux restores your windows, workspaces, panes, working directories, and scrollback when you relaunch, and the state survives a full computer restart, not just quitting the app. Agent sessions like Claude Code, Codex, and OpenCode come back too. See [session restore](https://cmux.com/docs/session-restore).

How does it compare to tmux?

tmux is a terminal multiplexer that runs inside any terminal. cmux is a native macOS app with a GUI: vertical tabs, split panes, an embedded browser, and a socket API, all built in, no config files or prefix keys needed. That said, lots of people happily run cmux with SSH and tmux together, and cmux can attach to your remote tmux sessions natively ([beta](https://cmux.com/docs/remote-tmux)).

Is cmux free?

Yes, cmux is free to use. The source code is available on [GitHub](https://github.com/manaflow-ai/cmux).

How can I support cmux?

cmux is free and open source, and always will be. If you want to back development and get early access to what's next, including cmux AI, the iOS app, and Cloud VMs, check out [cmux Founders Edition](https://github.com/manaflow-ai/cmux#founders-edition).

I have a feature request or found a bug

We want to hear it. Open an [issue](https://github.com/manaflow-ai/cmux/issues) or [pull request](https://github.com/manaflow-ai/cmux/pulls) on GitHub, or [email us](mailto:founders@manaflow.com?subject=%5Bcmux%20feature%20request%20landing%5D&body=Hi%20cmux%20team%2C%20).

## Community

-   ["Another day another libghostty-based project, this time a macOS terminal with vertical tabs, better organization/notifications, embedded/scriptable browser specifically targeted towards people who use a ton of terminal-based agentic workflows."](https://x.com/mitchellh/status/2024913161238053296) [—![Mitchell Hashimoto](https://cmux.com/avatars/mitchellh.jpg)Mitchell Hashimoto, Creator of Ghostty and founder of HashiCorp](https://x.com/mitchellh/status/2024913161238053296)
-   ["I'm late to the party, but cmux is great. Current split: Codex Mac app for knowledge work, learning, reading; cmux + Codex CLI for coding."](https://x.com/steipete/status/2058093406874689770) [—![Peter Steinberger](https://cmux.com/avatars/steipete.jpg)Peter Steinberger, OpenClaw creator. Founder of PSPDFKit.](https://x.com/steipete/status/2058093406874689770)
-   ["This is exactly the product I've been looking for. After two hours this am I've in love."](https://x.com/schrockn/status/2025182278637207857) [—![Nick Schrock](https://cmux.com/avatars/schrockn.jpg)Nick Schrock, Creator of Dagster. GraphQL co-creator.](https://x.com/schrockn/status/2025182278637207857)
-   ["I've been using this all weekend and it's amazing."](https://x.com/egrefen/status/2026806171563184199) [—![Edward Grefenstette](https://cmux.com/avatars/egrefen.jpg)Edward Grefenstette, Director of Research at Google DeepMind](https://x.com/egrefen/status/2026806171563184199)
-   ["\> learn cmux > trust me"](https://x.com/DavidOndrej1/status/2059360111336865901) [—![David Ondrej](https://cmux.com/avatars/davidondrej1.jpg)David Ondrej](https://x.com/DavidOndrej1/status/2059360111336865901)
-   ["this has been my favorite tool for past two weeks"](https://x.com/max4c_/status/2027266664270889204) [—![Max Forsey](https://cmux.com/avatars/max4c_.jpg)Max Forsey](https://x.com/max4c_/status/2027266664270889204)
-   ["아직 늦지 않았어요. 저도 Ghostty 많이 쓰는데 이어서 cmux도 사랑입니다. 세로 탭, 알림 링, 내장 브라우저, 분할 패널, GPU 가속 등등.. 정말 이점이 많아요!" — It's not too late. I use Ghostty a lot, and cmux is love too. Vertical tabs, notification rings, built-in browser, split panes, GPU acceleration... there are so many real benefits!](https://x.com/lucas_flatwhite/status/2058215633259831694) [—![lucas](https://cmux.com/avatars/lucas_flatwhite.jpg)lucas](https://x.com/lucas_flatwhite/status/2058215633259831694)
-   ["cmux しばらく使ってみたが好きだな めちゃくちゃ良いというよりは、あーこれだわこれ、という感触 k1Low/moとの相性も良い" — I've used cmux for a while and I like it. It feels less like 'this is amazing' and more like 'yes, this is it.' It also pairs well with k1Low/mo.](https://x.com/yamadashy/status/2057255883751788567) [—![yamadashy / やまだし](https://cmux.com/avatars/yamadashy.jpg)yamadashy / やまだし](https://x.com/yamadashy/status/2057255883751788567)
-   ["我也主力用 cmux，还推荐给其他同事，原因就是通知系统，分工作区，快捷键好用，多工作并行时能提高效率，尽管 cmux 比较丑，但它的功能让我不得不用它。" — I also use cmux as my main terminal and recommend it to coworkers. The notifications, workspaces, and shortcuts improve efficiency when running multiple jobs in parallel. Even though cmux is a bit ugly, its functionality makes it indispensable.](https://x.com/minixalpha/status/2037496984890986576) [—![minixalpha](https://cmux.com/avatars/minixalpha.jpg)minixalpha](https://x.com/minixalpha/status/2037496984890986576)
-   ["Tuve algún tema con el navegador pero cmux es insustituible en mi día a día." — I had an issue with the browser, but cmux is indispensable in my day to day.](https://x.com/juan_barbat/status/2055270317270921668) [—![Juan Barbat](https://cmux.com/avatars/juan_barbat.jpg)Juan Barbat](https://x.com/juan_barbat/status/2055270317270921668)
-   ["اقتراحي هو استعملوا Cmux وخلاص... فك لي ازمة بكل شيء تقريبًا من ناحية التيرمنل" — My suggestion is just use cmux. It solved almost every terminal problem for me.](https://x.com/yousefrol/status/2054034664940068890) [—![Yousef Rol](https://cmux.com/avatars/yousefrol.jpg)Yousef Rol](https://x.com/yousefrol/status/2054034664940068890)
-   ["Hab mir gerade cmux installiert, hab bisher ghostty genutzt. Aber cmux ist nochmal besser für KI Agenten und Coding geeignet." — I just installed cmux. I had been using Ghostty, but cmux is even better suited for AI agents and coding.](https://x.com/TobiasGloeckler/status/2032322168122720660) [—![Tobias Glöckler](https://cmux.com/avatars/tobiasgloeckler.jpg)Tobias Glöckler](https://x.com/TobiasGloeckler/status/2032322168122720660)
-   ["po nao sei como vivi tanto tempo sem cmux" — Man, I don't know how I lived so long without cmux.](https://x.com/wescld/status/2059611549677863347) [—![Wesley](https://cmux.com/avatars/wescld.jpg)Wesley](https://x.com/wescld/status/2059611549677863347)
-   ["요즘 최애 터미널 cmux. 개인적으로 멀티 터미널 돌리기 너무 좋은거 같아" — cmux is my favorite terminal lately. Personally, I think it's really good for running multiple terminals.](https://x.com/blitz_zidan/status/2049857904162025795) [—![ub:)ub 🎗️](https://cmux.com/avatars/blitz_zidan.jpg)ub:)ub 🎗️](https://x.com/blitz_zidan/status/2049857904162025795)
-   ["cmux 良さそうすぎてついにバイバイ VSCode するときなのかもしれない" — cmux looks so good it might finally be time to say goodbye to VSCode](https://x.com/asaza_0928/status/2026057269075698015) [—![あさざ](https://cmux.com/avatars/asaza_0928.jpg)あさざ](https://x.com/asaza_0928/status/2026057269075698015)
-   ["eğer birden fazla terminal ile çalışmanız gerekiyorsa kesinlikle cmux'u denemelisiniz. terminal sizden bir cevap beklediğinde otomatik bildirim geliyor." — If you need to work with multiple terminals, you should definitely try cmux. When a terminal waits for your input, it sends an automatic notification.](https://x.com/ssarisen/status/2046289729281294567) [—![Şerafettin Sarışen](https://cmux.com/avatars/ssarisen.jpg)Şerafettin Sarışen](https://x.com/ssarisen/status/2046289729281294567)
-   ["最近用的最多的终端工具就是这个 cmux，开源免费。基本上代替 iTerm2 了。完美解决了多终端窗口排列问题。" — cmux is the terminal tool I use most lately. It's open source and free. It has basically replaced iTerm2 for me and perfectly solves the multi-terminal window layout problem.](https://x.com/jinchenma_ai/status/2057038510323016067) [—![金尘马](https://cmux.com/avatars/jinchenma_ai.jpg)金尘马](https://x.com/jinchenma_ai/status/2057038510323016067)
-   ["Я уже какое-то время назад на него переехал с warp и как будто пересел на ракету. Он написан нативно для Mac OS на Swift и его супер активно развивают." — I moved to it from Warp a while ago and it felt like switching to a rocket. It's native macOS Swift and being developed super actively.](https://x.com/zvasil/status/2058873355172810894) [—![Закиев Василь](https://cmux.com/avatars/zvasil.jpg)Закиев Василь](https://x.com/zvasil/status/2058873355172810894)
-   ["推荐一个最近喜欢用的工具: cmux，不用频繁切换终端窗口了" — A tool I've liked using recently: cmux. I no longer have to switch terminal windows constantly.](https://x.com/immazzystar/status/2044695370492707124) [—![Mazzystar](https://cmux.com/avatars/immazzystar.jpg)Mazzystar](https://x.com/immazzystar/status/2044695370492707124)
-   ["Hey, this looks seriously awesome. Love the ideas here, specifically: the programmability, layered UI, browser w/ api. Looking forward to giving this a spin. Also want to add that I really appreciate Mitchell Hashimoto creating libghostty; it feels like an exciting time to be a terminal user."](https://news.ycombinator.com/item?id=47083596) [—johnthedebs](https://news.ycombinator.com/item?id=47083596)
-   ["Vertical tabs in my terminal 🤤 I never thought of that before. I use and love Firefox vertical tabs."](https://x.com/joeriddles10/status/2024914132416561465) [—![Joe Riddle](https://cmux.com/avatars/joeriddles10.jpg)Joe Riddle](https://x.com/joeriddles10/status/2024914132416561465)
-   ["Gave this a run and it was pretty intuitive. Good work!"](https://news.ycombinator.com/item?id=47082577) [—dchu17](https://news.ycombinator.com/item?id=47082577)
-   ["I like it, ran it in the past day on three parallel projects each with several worktrees. Having this paired with lazygit and yazi / nvim made me a bit more productive than usual without having to chase multiple ghostty / iTerm instances. Also feels more natural than tmux."](https://www.reddit.com/r/ClaudeCode/comments/1r9g45u/comment/o6sxbr3/) [—afruth](https://www.reddit.com/r/ClaudeCode/comments/1r9g45u/comment/o6sxbr3/)
-   ["cmux良さそうなので入れてみたけれど、良い" — Tried cmux since it looked good — it's good](https://x.com/northprint/status/2025740286677434581) [—![Norihiro Narayama](https://cmux.com/avatars/northprint.jpg)Norihiro Narayama](https://x.com/northprint/status/2025740286677434581)
-   ["cmux is pretty good."](https://x.com/indykish/status/2025318347970412673) [—![Kishore Neelamegam](https://cmux.com/avatars/indykish.jpg)Kishore Neelamegam](https://x.com/indykish/status/2025318347970412673)
-   ["cmux.dev に乗り換えた" — Switched to cmux.dev](https://x.com/kataring/status/2026189035056832718) [—![かたりん](https://cmux.com/avatars/kataring.jpg)かたりん](https://x.com/kataring/status/2026189035056832718)
-   ["This has been such a useful find. I can't recommend it enough."](https://x.com/scottw/status/2026806893067551084) [—![Scott Watermasysk](https://cmux.com/avatars/scottw.jpg)Scott Watermasysk](https://x.com/scottw/status/2026806893067551084)
-   ["grabbed this over the weekend and loved it. been waiting for something like this."](https://x.com/johnblythe/status/2026812731844637010) [—![John Blythe](https://cmux.com/avatars/johnblythe.jpg)John Blythe](https://x.com/johnblythe/status/2026812731844637010)
-   ["This is exactly what I've wanted. Amazing job thank you!"](https://x.com/BChris91/status/2026821091637838273) [—![Christopher](https://cmux.com/avatars/bchris91.jpg)Christopher](https://x.com/BChris91/status/2026821091637838273)
-   ["Been using this for a week and it's fantastic. Vert tab for each WIP task. Inside, claudes on one side and browser with PR and resources on the other, switch between tasks and stay organized. Mix that with skills to have Claude watch CI recursively, etc. feeling enlightened tbh"](https://x.com/connorelsea/status/2026867085750440390) [—![Connor](https://cmux.com/avatars/connorelsea.jpg)Connor](https://x.com/connorelsea/status/2026867085750440390)
-   ["年初にWarpからGhosttyに乗り換えたけど、今はcmuxに乗り換えた💻 垂直タブが便利で、Claude Codeのタスクの終了が通知されるのがありがたい。Ghosttyベースだから爆速動作はそのまま。ghosttyでやったブランチ表示や補完もそのまま使える" — I switched from Warp to Ghostty at the start of the year, but now I've switched to cmux. The vertical tabs are convenient, and I appreciate getting notified when Claude Code tasks finish. It's Ghostty-based so the blazing fast performance carries over. Branch display and completions I set up in Ghostty still work too.](https://x.com/tonkotsuboy_com/status/2028458464801108212) [—![鹿野 壮 Takeshi Kano](https://cmux.com/avatars/tonkotsuboy_com.jpg)鹿野 壮 Takeshi Kano](https://x.com/tonkotsuboy_com/status/2028458464801108212)

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

More cmux builds are on the way.

[Read the Docs](https://cmux.com/docs/getting-started) [View Changelog](https://cmux.com/docs/changelog)

Canonical: https://cmux.com/
