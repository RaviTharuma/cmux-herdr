# Optional shims

The plugin works **without** shell shims. This directory documents optional convenience wrappers you may add to PATH if desired.

## Not installed by default

`scripts/install.sh` does **not** inject shims into PATH. Prefer the real CLIs:

- `herdr` — inner mux
- `cmux` — outer app
- `cmux-herdr` — this plugin

## Optional ideas (manual)

| Shim name | Behavior | Risk |
|-----------|----------|------|
| `h` | alias to `herdr` | low |
| `ch` | alias to `cmux-herdr` | low |
| `tmux` | **do not** point at herdr globally | high — breaks scripts expecting real tmux |

## Nested PATH notes

cmux may inject agent wrapper shims (`CMUX_CLAUDE_WRAPPER_SHIM`, etc.). Those are **cmux-owned** and unrelated to this plugin. Do not delete them from this repo's install/uninstall.

## If you want local wrappers

```bash
mkdir -p ~/.local/bin
printf '%s\n' '#!/usr/bin/env bash' 'exec cmux-herdr "$@"' > ~/.local/bin/ch
chmod +x ~/.local/bin/ch
```

Keep wrappers thin; all logic stays in `bin/cmux-herdr` and `bridge/`.
