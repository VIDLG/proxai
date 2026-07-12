"""Shared paths and parser state for OpenAI protocol comparison."""

from pathlib import Path

import tree_sitter
import tree_sitter_rust as ts_rust

SCHEMA_PATH = Path("contrib/openai-openapi/openapi.yaml")
PROTOCOL_DIRS = {
    "responses": [Path("src/protocol/openai/responses/wire")],
    "chat": [
        Path("src/protocol/openai/chat_completions/wire"),
        Path("src/protocol/openai/chat_completions/request/wire"),
    ],
}

RS = tree_sitter.Parser(tree_sitter.Language(ts_rust.language()))
