# Contributing to cmux-herdr

Thank you for taking a look. This is a small, **stdlib-only Python CLI**.
There is no compiler, no `npm install`, and no `pip install` step.

A German overview of the project and of GitHub itself lives in
[docs/de/README.md](docs/de/README.md). Maintainer GitHub settings are in
[docs/MAINTAINING.md](docs/MAINTAINING.md). Architecture (what each file is
for) is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Before you start

1. You need **Python 3.10+**. 3.12 is what CI uses in the matrix.
2. You do **not** need macOS, `herdr`, or `cmux` to run the test suite.
   Those binaries are only required for live use.
3. Read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## How to verify a change

```bash
./scripts/test.sh
./bin/cmux-herdr --version
./bin/cmux-herdr --help
./bin/cmux-herdr doctor
```

`./scripts/test.sh` runs `python3 -m py_compile` (syntax check only — **not**
a C/Swift build) and stdlib `unittest` over `bridge/` and `tests/`.

**Do not add pytest, pip, or npm dependencies.**

Live commands (`tree`, `sync`, `mirror`) need fake or real `herdr`/`cmux` on
`PATH`. The suite already ships fakes; copy that pattern from
`tests/test_cli_behavior.py` if you add a CLI test.

## Pull requests

1. Branch from `main`.
2. Keep the diff focused (one concern per PR).
3. Run `./scripts/test.sh` and mention that you did.
4. Do not commit:
   - secrets, `.env`, private keys
   - live `cmux tree` / Herdr dumps
   - `__pycache__/`, editor backups, LaunchAgent logs
5. Use the PR template. Link an issue when one exists.

CI (GitHub Actions) runs the same `./scripts/test.sh` on Python 3.10–3.13.

## Where to file bugs

| Kind of problem | Where |
|---|---|
| This plugin (`cmux-herdr` CLI, install scripts, docs) | [Issues here](https://github.com/RaviTharuma/cmux-herdr/issues) |
| Herdr itself | [herdrdev/herdr](https://github.com/herdrdev/herdr) |
| cmux itself | [manaflow-ai/cmux](https://github.com/manaflow-ai/cmux) |
| Native nested topology inside cmux | [cmux#8737](https://github.com/manaflow-ai/cmux/issues/8737) and related PRs |

## Code layout (short)

| Path | Role |
|---|---|
| `bin/cmux-herdr` | CLI entry (argparse) |
| `bridge/*.py` | Library: snapshot, mirror, socket RPC, handoff |
| `bridge/test_*.py` | Unit tests next to the library |
| `tests/` | CLI / behavior tests with fake binaries |
| `scripts/` | install / uninstall / test / LaunchAgent |
| `sidebars/herdr.swift` | Optional cmux custom sidebar |
| `agent-skill/` | Instructions for AI agents using the dual hierarchy |

New library code belongs in `bridge/` with a matching `bridge/test_*_unit.py`.
New CLI flags belong in `bin/cmux-herdr` plus a `tests/test_cli_behavior.py`
case when they are user-visible.

## License

By contributing, you agree that your contribution is licensed under the
[MIT License](LICENSE) already used by this repository.
