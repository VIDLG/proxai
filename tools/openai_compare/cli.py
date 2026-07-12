"""Command-line orchestration for OpenAI protocol comparison."""

import argparse
import sys

from .comparison import compare_protocol
from .common import PROTOCOL_DIRS
from .report import render_report
from .rewrite import _annotate_direct_components, _reorder_fields
from .schema import _load_schemas


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Check ProxAI OpenAI wire types against the official OpenAPI schema."
    )
    parser.add_argument(
        "protocols",
        nargs="*",
        choices=sorted(PROTOCOL_DIRS),
        default=sorted(PROTOCOL_DIRS),
        help="protocol groups to check (default: responses chat)",
    )
    parser.add_argument("--quiet", "-q", action="store_true")
    parser.add_argument(
        "--structural",
        action="store_true",
        help=(
            "also check provenance, field coverage/order, flatten composition, "
            "closed enums, built-in scalar/container shapes, array item types, "
            "and tagged-union drift"
        ),
    )
    parser.add_argument(
        "--annotate-direct-components",
        action="store_true",
        help=(
            "add missing provenance comments for public wire structs whose Rust name exactly "
            "matches an official component; dry-run unless --write is also supplied"
        ),
    )
    parser.add_argument(
        "--reorder-fields",
        action="store_true",
        help=(
            "reorder public wire struct fields to match their declared OpenAPI schema; "
            "dry-run unless --write is also supplied"
        ),
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="apply changes requested by an explicit rewrite mode",
    )
    args = parser.parse_args(argv)

    rewrite_modes = args.annotate_direct_components + args.reorder_fields
    if rewrite_modes > 1:
        parser.error("select only one rewrite mode at a time")
    if args.write and not rewrite_modes:
        parser.error("--write requires an explicit rewrite mode")

    document, schemas = _load_schemas()
    if args.annotate_direct_components:
        changed = _annotate_direct_components(args.protocols, schemas, write=args.write)
        action = "annotated" if args.write else "would annotate"
        print(f"{action} {changed} public wire struct(s) with direct component provenance")
        if not args.write:
            print("dry run; pass --write to apply these comments")
        return
    if args.reorder_fields:
        changed = _reorder_fields(args.protocols, document, schemas, write=args.write)
        action = "reordered" if args.write else "would reorder"
        print(f"{action} fields in {changed} public wire struct(s)")
        if not args.write:
            print("dry run; pass --write to apply these field-order changes")
        return

    has_gaps = False
    for protocol in args.protocols:
        result = compare_protocol(
            protocol, document, schemas, structural=args.structural
        )
        has_gaps |= result.has_gaps
        if not args.quiet or result.has_gaps:
            render_report(result)

    if has_gaps:
        sys.exit(1)
