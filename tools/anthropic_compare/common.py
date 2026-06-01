from pathlib import Path
import sys

import tree_sitter
import tree_sitter_rust as ts_rust
import tree_sitter_typescript as ts_ts

SDK_FILE = Path('contrib/anthropic-sdk-typescript/src/resources/messages/messages.ts')
PROTO_DIR = Path('src/protocol/anthropic/messages/wire')
SDK_PKG = Path('contrib/anthropic-sdk-typescript/package.json')


def _parser(lang_mod):
    return tree_sitter.Parser(tree_sitter.Language(lang_mod()))


TS = _parser(ts_ts.language_typescript)
RS = _parser(ts_rust.language)


def norm(n):
    return n.lower().replace('_', '').replace('-', '').rstrip(';')


def out(t=''):
    sys.stdout.buffer.write((t + '\n').encode('utf-8', errors='replace'))


def hr():
    out('=' * 66)


def h2(title):
    out(f"\n  {title}")
    out(f"  {'─' * min(64, len(title) + 2)}")


def loc(f, n):
    return f"{f}:{n}"
