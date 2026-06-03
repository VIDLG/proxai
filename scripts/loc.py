"""Count core Rust source lines in proxai, excluding tests."""

import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "src"
TEST_SUFFIXES = ("_tests.rs", "_test.rs")
TEST_DIR_SEGMENTS = ("tests",)

def is_test_file(path: Path) -> bool:
    if any(seg in TEST_DIR_SEGMENTS for seg in path.parts):
        return True
    return path.name.endswith(TEST_SUFFIXES)

def count_lines(path: Path) -> int:
    return sum(1 for _ in path.open(encoding="utf-8", errors="replace"))

def main():
    total = 0
    test_total = 0
    modules: dict[str, int] = {}

    for rs in sorted(ROOT.rglob("*.rs")):
        rel = rs.relative_to(ROOT)
        lines = count_lines(rs)
        if is_test_file(rel):
            test_total += lines
            continue
        total += lines
        top = rel.parts[0] if len(rel.parts) > 1 else rel.stem
        modules[top] = modules.get(top, 0) + lines

    print(f"{'Module':<35} {'Lines':>8}")
    print("-" * 45)
    for mod, lines in sorted(modules.items(), key=lambda x: -x[1]):
        print(f"  {mod:<33} {lines:>8}")
    print("-" * 45)
    print(f"  {'Core total':<33} {total:>8}")
    print(f"  {'Tests':<33} {test_total:>8}")
    print(f"  {'Grand total':<33} {total + test_total:>8}")

if __name__ == "__main__":
    main()
