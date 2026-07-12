# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "pyyaml",
#   "tree-sitter",
#   "tree-sitter-rust",
# ]
# ///

"""Check ProxAI OpenAI wire types against the pinned official OpenAPI schema."""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tools.openai_compare.cli import main


if __name__ == "__main__":
    main()
