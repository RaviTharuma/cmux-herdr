> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Installation

> Install cmux on macOS via DMG or Homebrew

cmux is available for macOS 13 (Ventura) and later. Choose your preferred installation method below.

## DMG (recommended)

The DMG installer includes auto-update via Sparkle, so you only need to download once.

<Steps>
  <Step title="Download cmux">
    Download the latest release:

    [Download cmux for macOS](https://github.com/manaflow-ai/cmux/releases/latest/download/cmux-macos.dmg)
  </Step>

  <Step title="Install the app">
    Open the downloaded `.dmg` file and drag cmux to your Applications folder.

    <img src="https://mintlify.s3.us-west-1.amazonaws.com/manaflow-ai-cmux/images/install-dmg.png" alt="Drag cmux to Applications" />
  </Step>

  <Step title="First launch">
    On first launch, macOS may ask you to confirm opening an app from an identified developer. Click **Open** to proceed.

    <Note>
      cmux is notarized by Apple and safe to run. This dialog is standard for apps downloaded from the web.
    </Note>
  </Step>

  <Step title="Automatic updates">
    cmux will automatically check for updates and notify you in the sidebar when a new version is available. Click the update pill to install.
  </Step>
</Steps>

## Homebrew

Install via Homebrew Cask:

```bash theme={null}
brew tap manaflow-ai/cmux
brew install --cask cmux
```

### Updating with Homebrew

To update cmux later:

```bash theme={null}
brew upgrade --cask cmux
```

<Note>
  Apps installed via Homebrew still receive in-app update notifications, but updates are managed through Homebrew.
</Note>

## Nightly builds

For early access to the latest features, you can download nightly builds:

[Download cmux NIGHTLY](https://github.com/manaflow-ai/cmux/releases/download/nightly/cmux-nightly-macos.dmg)

cmux NIGHTLY is a separate app with its own bundle ID, so it runs alongside the stable version. It's built automatically from the latest `main` commit and auto-updates via its own Sparkle feed.

<Warning>
  Nightly builds may contain bugs or incomplete features. Use them for testing and experimentation, not production work.
</Warning>

### Enabling nightly updates in stable

You can also opt into nightly builds from the stable app:

<Steps>
  <Step title="Open Settings">
    Press `Cmd+,` or select **cmux > Settings** from the menu bar.
  </Step>

  <Step title="Enable nightly channel">
    Check the **Receive Nightly Builds** option in the Updates section.
  </Step>

  <Step title="Restart cmux">
    Restart the app to check for nightly updates.
  </Step>
</Steps>

## System requirements

* macOS 13.0 (Ventura) or later
* Apple Silicon or Intel Mac
* \~100 MB disk space

## Ghostty configuration

cmux is compatible with Ghostty and reads your existing configuration:

```bash theme={null}
~/.config/ghostty/config
```

If you have a Ghostty config file, cmux will automatically use your themes, fonts, colors, and keybindings. If you don't have one, cmux uses sensible defaults.

<Note>
  cmux uses libghostty for terminal rendering, so all Ghostty configuration options are supported.
</Note>

## Verification

After installation, verify that cmux is working:

<CodeGroup>
  ```bash Terminal theme={null}
  cmux --version
  ```

  ```bash Expected output theme={null}
  cmux 0.61.0 (build 361)
  ```
</CodeGroup>

If the command is not found, the CLI may not be in your PATH. You can add it manually:

```bash theme={null}
ln -s /Applications/cmux.app/Contents/MacOS/cmux /usr/local/bin/cmux
```

## Next steps

<CardGroup cols={2}>
  <Card title="Quickstart" icon="rocket" href="/quickstart">
    Get started with workspaces and notifications
  </Card>

  <Card title="CLI Reference" icon="terminal" href="/automation/cli-reference">
    Explore CLI commands
  </Card>
</CardGroup>
