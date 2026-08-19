# [#](#title)Getting Started

# [#](https://cmux-docs-release.vercel.app/docs/getting-started#title)Getting Started

cmux is a lightweight, native macOS terminal built on Ghostty for managing multiple AI coding agents. It features vertical tabs, a notification panel, and a socket-based control API.

## [#](https://cmux-docs-release.vercel.app/docs/getting-started#install)Install

### [#](https://cmux-docs-release.vercel.app/docs/getting-started#dmg-recommended)DMG (recommended)

[Download for Mac](https://cmux-docs-release.vercel.app/download/confirmation?dl=1)

Open the .dmg and drag cmux to your Applications folder. cmux auto-updates via Sparkle, so you only need to download once.

### [#](https://cmux-docs-release.vercel.app/docs/getting-started#homebrew)Homebrew

```
brew tap manaflow-ai/cmux
brew install --cask cmux
```

To update later:

```
brew upgrade --cask cmux
```

On first launch, macOS may ask you to confirm opening an app from an identified developer. Click **Open** to proceed.

## [#](https://cmux-docs-release.vercel.app/docs/getting-started#verify-title)Verify installation

Open cmux and you should see:

-   A terminal window with a vertical tab sidebar on the left
-   One initial workspace already open
-   The Ghostty-powered terminal ready for input

## [#](https://cmux-docs-release.vercel.app/docs/getting-started#cli-setup)CLI setup

cmux includes a command-line tool for automation. Inside cmux terminals it works automatically. To use the CLI from outside cmux, create a symlink:

```
sudo ln -sf "/Applications/cmux.app/Contents/Resources/bin/cmux" /usr/local/bin/cmux
```

Then you can run commands like:

```
cmux list-workspaces
cmux notify --title "Build Complete" --body "Your build finished"
```

## [#](https://cmux-docs-release.vercel.app/docs/getting-started#auto-updates)Auto-updates

cmux checks for updates automatically via Sparkle. When an update is available you'll see an update pill in the titlebar. You can also check manually via cmux > Check for Updates in the menu bar.

## [#](https://cmux-docs-release.vercel.app/docs/getting-started#session-restore)Session restore (current behavior)

cmux restores layout and metadata after relaunch. Supported agent sessions can also resume when hooks have captured a native resume token.

cmux does not checkpoint arbitrary live process state. Unsupported terminal apps such as tmux, vim, and ordinary shells reopen as normal terminals.

See the [session restore guide](https://cmux-docs-release.vercel.app/docs/session-restore) for agent hook setup and the supported resume matrix.

## [#](https://cmux-docs-release.vercel.app/docs/getting-started#requirements)Requirements

-   macOS 14.0 or later
-   Apple Silicon or Intel Mac

[cmux TUI](https://cmux-docs-release.vercel.app/docs/tui)

Canonical: https://cmux-docs-release.vercel.app/docs/getting-started
