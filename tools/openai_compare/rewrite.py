"""Explicit source rewrite operations for provenance and field ordering."""

import re

from .common import PROTOCOL_DIRS, RS
from .schema import _object_contract, _schema_for_type
from .rust import _is_public, _openapi_schema_pointer, _type_name, _wire_field_name


def _reorder_fields(protocols, document, schemas, *, write):
    changes = {}
    for protocol in protocols:
        for directory in PROTOCOL_DIRS[protocol]:
            for path in sorted(directory.rglob("*.rs")):
                source = path.read_bytes()
                replacements = _field_order_replacements(
                    RS.parse(source).root_node, source, document, schemas
                )
                if replacements:
                    changes[path] = replacements

    if write:
        for path, replacements in changes.items():
            source = path.read_bytes()
            for start, end, replacement in sorted(replacements, reverse=True):
                source = source[:start] + replacement + source[end:]
            path.write_bytes(source)

    return sum(len(replacements) for replacements in changes.values())

def _field_order_replacements(node, source, document, schemas):
    replacements = []
    if node.type == "struct_item" and _is_public(node):
        type_name = _type_name(node)
        pointer = _openapi_schema_pointer(node, source)
        schema, error = _schema_for_type(type_name, pointer, document, schemas)
        fields_node = next(
            (child for child in node.children if child.type == "field_declaration_list"),
            None,
        )
        if error is None and isinstance(schema, dict) and fields_node is not None:
            properties, _ = _object_contract(schema, schemas, set())
            replacement = _reordered_field_body(fields_node, source, properties)
            if replacement is not None:
                replacements.append(
                    (fields_node.start_byte + 1, fields_node.end_byte - 1, replacement)
                )
    for child in node.children:
        replacements.extend(
            _field_order_replacements(child, source, document, schemas)
        )
    return replacements

def _reordered_field_body(fields_node, source, properties):
    declarations = [
        child for child in fields_node.children if child.type == "field_declaration"
    ]
    if len(declarations) < 2:
        return None

    body_start = fields_node.start_byte + 1
    body_end = fields_node.end_byte - 1
    cursor = body_start
    blocks = {}
    original_order = []
    for declaration in declarations:
        field_name = next(
            (
                part.text.decode()
                for part in declaration.children
                if part.type == "field_identifier"
            ),
            None,
        )
        if field_name is None:
            return None
        attributes = [
            child.text.decode()
            for child in fields_node.children
            if child.type == "attribute_item"
            and cursor <= child.start_byte < declaration.start_byte
        ]
        if any(
            re.search(r"\bserde\s*\([^]]*\bflatten\b", attribute)
            for attribute in attributes
        ):
            return None
        wire_name = _wire_field_name(field_name, attributes)
        comma = source.find(b",", declaration.end_byte, body_end)
        if comma < 0:
            return None
        newline = source.find(b"\n", comma, body_end)
        block_end = newline + 1 if newline >= 0 else comma + 1
        blocks[wire_name] = source[cursor:block_end]
        original_order.append(wire_name)
        cursor = block_end

    if len(blocks) != len(declarations):
        return None
    schema_order = [name for name in properties if name in blocks]
    local_only = [name for name in original_order if name not in properties]
    desired_order = schema_order + local_only
    if desired_order == original_order:
        return None

    return b"".join(blocks[name] for name in desired_order) + source[cursor:body_end]

def _annotate_direct_components(protocols, schemas, *, write):
    """Annotate only exact-name component matches; inline schemas stay manual."""
    candidates = []
    for protocol in protocols:
        for directory in PROTOCOL_DIRS[protocol]:
            for path in sorted(directory.rglob("*.rs")):
                source = path.read_bytes()
                tree = RS.parse(source)
                candidates.extend(
                    _direct_component_annotation_candidates(
                        path, tree.root_node, source, schemas
                    )
                )

    if write:
        by_path = {}
        for path, offset, comment in candidates:
            by_path.setdefault(path, []).append((offset, comment))
        for path, annotations in by_path.items():
            source = path.read_bytes()
            for offset, comment in sorted(annotations, reverse=True):
                source = source[:offset] + comment.encode() + source[offset:]
            path.write_bytes(source)

    return len(candidates)

def _direct_component_annotation_candidates(path, node, source, schemas):
    candidates = []
    if node.type == "struct_item" and _is_public(node):
        name = _type_name(node)
        if (
            name in schemas
            and _openapi_schema_pointer(node, source) is None
        ):
            candidates.append(
                (
                    path,
                    _declaration_start(node),
                    f"/// OpenAPI schema: `#/components/schemas/{name}`\n",
                )
            )
    for child in node.children:
        candidates.extend(
            _direct_component_annotation_candidates(path, child, source, schemas)
        )
    return candidates

def _declaration_start(node):
    """Put documentation before serde/derive attributes, like hand-written docs."""
    start = node.start_byte
    sibling = node.prev_named_sibling
    while sibling is not None and sibling.type == "attribute_item":
        start = sibling.start_byte
        sibling = sibling.prev_named_sibling
    return start
