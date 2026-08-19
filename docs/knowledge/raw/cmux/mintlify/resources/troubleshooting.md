> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Troubleshooting

> Common issues and solutions for cmux

This guide covers common issues you might encounter while using cmux and how to resolve them.

## Installation issues

### App won't open on first launch

On first launch, macOS may ask you to confirm opening an app from an identified developer.

**Solution:** Click **Open** when prompted. This is a standard macOS security measure for apps downloaded outside the App Store.

### Homebrew installation fails

If `brew install --cask cmux` fails, the tap might not be properly added.

**Solution:** Ensure you've added the tap first:

```bash theme={null}
brew tap manaflow-ai/cmux
brew install --cask cmux
```

## Configuration issues

### Themes and colors not loading

cmux reads your Ghostty configuration from `~/.config/ghostty/config`.

**Solution:** Verify your Ghostty config file exists and contains valid configuration. Create the directory if needed:

```bash theme={null}
mkdir -p ~/.config/ghostty
```

Reload the configuration with `Cmd+Shift+,` after making changes.

### Custom fonts not appearing

**Solution:** Ensure the font is installed system-wide and properly configured in your `~/.config/ghostty/config`:

```
font-family = "Your Font Name"
```

Reload configuration with `Cmd+Shift+,`.

## Notification issues

### Notifications not appearing

If terminal notifications aren't showing blue rings or lighting up tabs, the notification system might not be receiving signals.

**Solution:** Verify you're sending notifications using the correct method:

```bash theme={null}
cmux notify "Your message here"
```

Or use OSC sequences (OSC 9/99/777) directly from your agent hooks.

### Blue notification rings stuck

**Solution:** Jump to the notification with `Cmd+Shift+U` to clear it, or click directly on the pane with the blue ring.

## Browser issues

### Browser won't load localhost URLs

If the in-app browser fails to load localhost or 127.0.0.1 URLs:

**Solution:** Check the HTTP host allowlist in Settings. You may need to add localhost to the allowlist or click "Proceed" when prompted.

### Browser developer tools not working

**Solution:** Toggle Developer Tools with `Opt+Cmd+I` (Safari default). If this doesn't work, check your keyboard shortcuts in Settings.

## Performance issues

### Terminal feels laggy

**Solution:**

* Check if you have many split panes open. Consider closing unused splits.
* Verify GPU acceleration is working by checking Activity Monitor for GPU usage.
* Reload the configuration with `Cmd+Shift+,`.

### High CPU usage

**Solution:**

* Check for notification loops or excessive logging.
* Restart the app to clear any stuck processes.
* Check Activity Monitor to identify which process is consuming CPU.

## Session restore issues

### Active processes not resuming after restart

<Warning>
  cmux currently restores app layout and metadata only (window/workspace/pane layout, working directories, terminal scrollback, browser URLs). It does **not** restore live process state inside terminal apps yet.
</Warning>

Active Claude Code, tmux, or vim sessions won't automatically resume after restart.

**Solution:** Use tmux or similar session managers to preserve your terminal sessions across cmux restarts.

## Update issues

### Auto-update not working

**Solution:**

* Check for updates manually via the cmux menu.
* If using Homebrew, update with: `brew upgrade --cask cmux`
* For DMG installations, cmux uses Sparkle for auto-updates. Check Settings for update preferences.

### Nightly builds not appearing

**Solution:** Enable nightly updates in Settings under "Receive Nightly Builds". Note that nightly builds are a separate app that runs alongside the stable version.

## CLI and scripting issues

### Socket commands not working

**Solution:** Verify the socket exists:

```bash theme={null}
ls -la /tmp/cmux.sock
```

If it doesn't exist, cmux might not be running or the socket path might be different for debug builds (`/tmp/cmux-debug.sock`).

### Commands target wrong workspace

By default, CLI commands use the `CMUX_WORKSPACE_ID` environment variable.

**Solution:** Use explicit targeting flags:

```bash theme={null}
cmux send --workspace <id> "your command"
```

## Getting help

If you're still experiencing issues:

* Check the [GitHub issues](https://github.com/manaflow-ai/cmux/issues) for similar problems
* Join the [Discord](https://discord.gg/xsgFEVrWCZ) community for help
* Create a new issue with details about your environment and the problem

<Note>
  When reporting issues, include your macOS version, cmux version (`cmux --version`), and relevant configuration from `~/.config/ghostty/config`.
</Note>
