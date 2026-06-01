# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "tree-sitter",
#   "tree-sitter-typescript",
#   "tree-sitter-rust",
# ]
# ///

"""
Compare proxai Anthropic Messages protocol types against the official SDK.

Uses tree-sitter AST parsing — type-level and field-level alignment.
Run from project root:
  pixi run python tools/compare_anthropic_protocol.py
  just compare-protocol

Exit code: 0 = no gaps, 1 = gaps found
"""

import sys

from anthropic_compare.checks import emit_sdk_docs
from anthropic_compare.report import run_report

# ═══════════════════════════════════════════════════════════════
#  Main
# ═══════════════════════════════════════════════════════════════

def main():
    # Parse CLI
    args = sys.argv[1:]
    only_marked = '--only-marked' in args
    if '--emit-docs' in args:
        emit_sdk_docs(only_marked=only_marked)
        return
    level = 2
    i = 0
    while i < len(args):
        if args[i] in ('--level', '-l') and i + 1 < len(args):
            level = int(args[i + 1])
            i += 1
        elif args[i] in ('--quiet', '-q'):
            level = 1
        elif args[i] in ('--detail', '-d'):
            level = 2
        elif args[i] in ('--verbose', '-v'):
            level = 3
        i += 1

    run_report(level, only_marked=only_marked)


if __name__ == '__main__':
    main()
