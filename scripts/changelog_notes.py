#!/usr/bin/env python3
"""Print the CHANGELOG.md section for a git tag such as v0.3.4."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHANGELOG = ROOT / "CHANGELOG.md"


def tag_to_version(tag: str) -> str:
    """Return the semver inside a v-prefixed tag, or raise ValueError."""
    text = tag.strip()
    if text.startswith("v"):
        text = text[1:]
    if not re.fullmatch(r"\d+\.\d+\.\d+", text):
        raise ValueError(f"not a vX.Y.Z tag: {tag!r}")
    return text


def changelog_section(changelog: str, version: str) -> str:
    """Return the body of ``## [version]`` including the heading."""
    heading = f"## [{version}]"
    start = changelog.find(heading)
    if start < 0:
        raise ValueError(f"CHANGELOG.md has no {heading} section")
    rest = changelog[start:]
    nxt = re.search(r"\n## \[", rest[len(heading) :])
    if nxt:
        rest = rest[: len(heading) + nxt.start()]
    return rest.strip() + "\n"


def release_notes(tag: str, changelog_text: str | None = None) -> str:
    """Build GitHub Release markdown for ``tag``."""
    version = tag_to_version(tag)
    if changelog_text is None:
        changelog_text = CHANGELOG.read_text(encoding="utf-8")
    section = changelog_section(changelog_text, version)
    return (
        f"## cmux-herdr {tag}\n\n"
        f"{section}\n"
        f"---\n\n"
        f"Install:\n\n"
        f"```bash\n"
        f"cmux sidebar plugin install "
        f"https://github.com/RaviTharuma/cmux-herdr.git\n"
        f"cmux sidebar plugin use cmux-herdr\n"
        f"```\n\n"
        f"Python 3.10+, stdlib only. Plugin-manager `[build]` is chmod +x, not Cargo.\n"
    )


def main(argv: list[str] | None = None) -> int:
    """CLI entry: print notes for one tag to stdout."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag", help="git tag, e.g. v0.3.4")
    args = parser.parse_args(argv)
    try:
        sys.stdout.write(release_notes(args.tag))
    except ValueError as exc:
        sys.stderr.write(f"{exc}\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
