> ## Documentation Index
> Fetch the complete documentation index at: https://mintlify.com/manaflow-ai/cmux/llms.txt
> Use this file to discover all available pages before exploring further.

# Contributing

> How to contribute to cmux development

Thank you for your interest in contributing to cmux! This guide will help you get started with development.

## Prerequisites

Before you begin, ensure you have the following installed:

* **macOS 14+**
* **Xcode 15+**
* **Zig** - Install via Homebrew: `brew install zig`

## Getting started

### 1. Clone the repository

Clone the repository with submodules:

```bash theme={null}
git clone --recursive https://github.com/manaflow-ai/cmux.git
cd cmux
```

### 2. Run the setup script

Initialize submodules and build dependencies:

```bash theme={null}
./scripts/setup.sh
```

This script will:

* Initialize git submodules (ghostty, homebrew-cmux)
* Build the GhosttyKit.xcframework from source
* Create necessary symlinks

### 3. Build and run

Launch the debug app:

```bash theme={null}
./scripts/reload.sh
```

## Development scripts

The following scripts help streamline development:

| Script                 | Description                               |
| ---------------------- | ----------------------------------------- |
| `./scripts/setup.sh`   | One-time setup (submodules + xcframework) |
| `./scripts/reload.sh`  | Build and launch Debug app                |
| `./scripts/reloadp.sh` | Build and launch Release app              |
| `./scripts/reload2.sh` | Reload both Debug and Release             |
| `./scripts/rebuild.sh` | Clean rebuild                             |

## Rebuilding GhosttyKit

If you make changes to the ghostty submodule, rebuild the xcframework:

```bash theme={null}
cd ghostty
zig build -Demit-xcframework=true -Doptimize=ReleaseFast
```

## Running tests

### Basic tests (run on VM)

```bash theme={null}
ssh cmux-vm 'cd /Users/cmux/GhosttyTabs && xcodebuild -project GhosttyTabs.xcodeproj -scheme cmux -configuration Debug -destination "platform=macOS" build && pkill -x "cmux DEV" || true && APP=$(find /Users/cmux/Library/Developer/Xcode/DerivedData -path "*/Build/Products/Debug/cmux DEV.app" -print -quit) && open "$APP" && for i in {1..20}; do [ -S /tmp/cmux.sock ] && break; sleep 0.5; done && python3 tests/test_update_timing.py && python3 tests/test_signals_auto.py && python3 tests/test_ctrl_socket.py && python3 tests/test_notifications.py'
```

### UI tests (run on VM)

```bash theme={null}
ssh cmux-vm 'cd /Users/cmux/GhosttyTabs && xcodebuild -project GhosttyTabs.xcodeproj -scheme cmux -configuration Debug -destination "platform=macOS" -only-testing:cmuxUITests test'
```

## Working with the Ghostty submodule

The `ghostty` submodule points to [manaflow-ai/ghostty](https://github.com/manaflow-ai/ghostty), a fork of the upstream Ghostty project.

### Making changes to ghostty

```bash theme={null}
cd ghostty
git checkout -b my-feature
# make your changes
git add .
git commit -m "Description of changes"
git push manaflow my-feature
```

### Keeping the fork updated

To sync with upstream Ghostty:

```bash theme={null}
cd ghostty
git fetch origin
git checkout main
git merge origin/main
git push manaflow main
```

Then update the parent repository:

```bash theme={null}
cd ..
git add ghostty
git commit -m "Update ghostty submodule"
```

<Note>
  See `docs/ghostty-fork.md` in the repository for details on fork changes and conflict notes.
</Note>

## Ways to contribute

There are many ways to get involved with cmux:

### Community engagement

* Follow [@manaflowai](https://x.com/manaflowai) or [@lawrencecchen](https://x.com/lawrencecchen) on X for updates
* Join the conversation on [Discord](https://discord.gg/xsgFEVrWCZ)
* Participate in [GitHub discussions](https://github.com/manaflow-ai/cmux/discussions)
* Share what you're building with cmux

### Code contributions

* Fix bugs and submit pull requests
* Add new features that align with cmux's philosophy
* Improve documentation
* Write tests

### Bug reports and feature requests

* Create detailed [GitHub issues](https://github.com/manaflow-ai/cmux/issues)
* Include reproduction steps for bugs
* Provide context and use cases for feature requests

## Development guidelines

When contributing code:

* Follow existing code style and conventions
* Write clear commit messages
* Test your changes thoroughly
* Update documentation as needed
* Keep changes focused and atomic

<Warning>
  Before making significant changes, consider opening a discussion or issue to align with the project's direction and avoid duplicate work.
</Warning>

## License

By contributing to this repository, you agree that your contributions are licensed under the project's GNU Affero General Public License v3.0 or later (`AGPL-3.0-or-later`).

## Community

Connect with the cmux community:

* [Discord](https://discord.gg/xsgFEVrWCZ)
* [GitHub](https://github.com/manaflow-ai/cmux)
* [X / Twitter](https://twitter.com/manaflowai)
* [YouTube](https://www.youtube.com/channel/UCAa89_j-TWkrXfk9A3CbASw)
* [LinkedIn](https://www.linkedin.com/company/manaflow-ai/)
