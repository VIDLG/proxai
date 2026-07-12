"""Command-line orchestration for Anthropic protocol comparison."""

import argparse

from .comparison import compare_protocol
from .report import render_report
from .rewrite import emit_sdk_docs


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=(
            "Compare ProxAI Anthropic Messages wire types against the pinned official SDK."
        )
    )
    output = parser.add_mutually_exclusive_group()
    output.add_argument("--level", "-l", type=int, choices=(1, 2, 3), default=2)
    output.add_argument("--quiet", "-q", action="store_const", const=1, dest="level")
    output.add_argument("--detail", "-d", action="store_const", const=2, dest="level")
    output.add_argument("--verbose", "-v", action="store_const", const=3, dest="level")
    parser.add_argument(
        "--emit-docs",
        action="store_true",
        help="print SDK shape documentation for bound Rust wire types",
    )
    parser.add_argument(
        "--only-marked",
        action="store_true",
        help="limit checks or generated documentation to explicitly marked bindings",
    )
    args = parser.parse_args(argv)

    if args.emit_docs:
        emit_sdk_docs(only_marked=args.only_marked)
        return

    result = compare_protocol(only_marked=args.only_marked)
    render_report(result, args.level)
    if result.has_gaps:
        raise SystemExit(1)
