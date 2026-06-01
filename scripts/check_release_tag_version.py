#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "Cargo.toml"


def load_cargo_version() -> str:
    data = tomllib.loads(CARGO_TOML.read_text(encoding="utf-8"))
    return str(data["package"]["version"])


def pushed_tags(stdin_text: str) -> list[str]:
    tags: list[str] = []
    for line in stdin_text.splitlines():
        parts = line.split()
        if len(parts) < 4:
            continue
        local_ref = parts[0]
        if local_ref.startswith("refs/tags/v"):
            tags.append(local_ref.removeprefix("refs/tags/"))
    return tags


def main() -> int:
    tags = pushed_tags(sys.stdin.read())
    if not tags:
        return 0

    version = load_cargo_version()
    expected = f"v{version}"
    mismatches = [tag for tag in tags if tag != expected]
    if not mismatches:
        return 0

    joined = ", ".join(mismatches)
    print(
        (
            "tag/Cargo.toml version mismatch: "
            f"pushing {joined}, but Cargo.toml has version {version} "
            f"(expected tag {expected})"
        ),
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
