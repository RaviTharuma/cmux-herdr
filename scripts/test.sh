#!/usr/bin/env bash
# Stdlib unittest only — do not invent a pytest runner.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

export PYTHONPATH="${ROOT}${PYTHONPATH:+:${PYTHONPATH}}"

echo "== py_compile =="
python3 -m py_compile bin/cmux-herdr bridge/cmux_herdr_bridge.py bridge/cmux_herdr_mirror.py bridge/cmux_herdr_layout.py bridge/cmux_herdr_socket.py bridge/cmux_herdr_engine.py bridge/cmux_herdr_impose.py bridge/cmux_herdr_host.py

echo "== unittest bridge =="
python3 -m unittest discover -s bridge -p 'test_*.py' -v

echo "== unittest tests/ =="
python3 -m unittest discover -s tests -p 'test_*.py' -v

echo "OK: all cmux-herdr tests passed (unittest only; no pytest)"
