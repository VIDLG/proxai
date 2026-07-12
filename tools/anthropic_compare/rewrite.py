"""Explicit source-derived output operations for Anthropic protocol models."""

from .common import SDK_FILE, out
from .rust import rust_item_shape_bindings
from .sdk import _split_top_level_union, sdk_comment_shapes


def _format_sdk_shape_doc(shape):
    lines = []
    if shape["kind"] == "type":
        parts = _split_top_level_union(shape["rhs"])
        if len(parts) <= 1:
            return [f"/// export type {shape['name']} = {shape['rhs']};"]
        lines.append(f"/// export type {shape['name']} =")
        for index, part in enumerate(parts):
            suffix = ";" if index == len(parts) - 1 else ""
            lines.append(f"///   | {part}{suffix}")
        return lines

    extends = f" extends {shape['extends']}" if shape.get("extends") else ""
    lines.append(f"/// export interface {shape['name']}{extends} {{")
    for field in shape.get("fields", []):
        optional = "?" if field.get("optional") else ""
        lines.append(f"///   {field['name']}{optional}: {field['type']};")
    lines.append("/// }")
    return lines


def emit_sdk_docs(only_marked=False):
    sdk_shapes = sdk_comment_shapes(SDK_FILE.read_text(encoding="utf-8"))
    for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked):
        out(
            f"{binding['file']}:{binding['line']} "
            f"{binding['item']} -> {binding['sdk_name']}"
        )
        for line in _format_sdk_shape_doc(binding["sdk_shape"]):
            out(line)
        out()
