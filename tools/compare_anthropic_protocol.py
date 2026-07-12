# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "tree-sitter",
#   "tree-sitter-typescript",
#   "tree-sitter-rust",
# ]
# ///

"""Compare ProxAI Anthropic Messages types against the pinned official SDK."""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tools.anthropic_compare.cli import main


if __name__ == "__main__":
    main()
