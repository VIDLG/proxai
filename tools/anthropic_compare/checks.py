import re

from .common import SDK_FILE, norm, out
from .rust import (
    _field_by_wire_name,
    _rust_type_is_option,
    _rust_variant_wire_name,
    _serde_attr_has,
    _serde_rename,
    _serde_skip_if_none,
    _type_names_from_text,
    rust_field_shape_comments,
    rust_item_shape_bindings,
    rust_sdk_markers,
    rust_serde_items,
    rust_tagged_variant_literals,
    shape_binding_diffs,
    uncontrolled_ts_comment_fragments,
    use_annotation_diffs,
    use_reference_diffs,
)
from .sdk import _normalize_ts_type, _split_top_level_union, sdk_comment_shapes


def _ts_type_has_null(type_text):
    return any(part.strip() == "null" for part in _split_top_level_union(type_text))


def _ts_string_literals(type_text):
    return set(re.findall(r"'([^']+)'", type_text))


def sdk_literal_aliases(sdk_shapes):
    return {
        name: _ts_string_literals(shape.get("rhs", ""))
        for name, shape in sdk_shapes.items()
        if shape.get("kind") == "type" and _ts_string_literals(shape.get("rhs", ""))
    }


def rust_enum_literals(enum_item):
    return {
        _rust_variant_wire_name(name, variant, enum_item)
        for name, variant in enum_item.get("variants", {}).items()
    }


def _sdk_literal_field(sdk_field):
    value = sdk_field["type"].strip()
    if re.match(r"^'[^']+'$", value):
        return value.strip("'")
    return None


def _is_covered_discriminator(binding, sdk_field, parent_tags):
    if sdk_field["name"] != "type":
        return False
    literal = _sdk_literal_field(sdk_field)
    return bool(literal and literal in parent_tags.get(binding["item"], set()))


def serde_wire_diffs(sdk_text, only_marked=False):
    """Check conservative serde wire semantics against SDK discriminator fields."""
    sdk_shapes = sdk_comment_shapes(sdk_text)
    rust_items = rust_serde_items()
    parent_tags = rust_tagged_variant_literals()
    diffs = []
    for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked):
        sdk_shape = binding["sdk_shape"]
        item = rust_items.get(binding["item"])
        if not sdk_shape or not item or sdk_shape.get("kind") != "interface":
            continue
        type_field = None
        for field in sdk_shape.get("fields", []):
            if field["name"] == "type" and re.match(r"^'[^']+'$", field["type"]):
                type_field = field
                break
        if not type_field:
            continue
        item_attrs = " ".join(item.get("attrs", []))
        has_tag = 'serde(tag = "type")' in item_attrs
        rust_type_field_name, field = _field_by_wire_name(item, "type")
        has_renamed_field = False
        field_literal = None
        if field:
            has_renamed_field = rust_type_field_name != "type"
            field_type = field.get("type")
            if field_type and field_type in rust_items:
                variants = rust_items[field_type].get("variants", {})
                if len(variants) == 1:
                    variant_name, variant = next(iter(variants.items()))
                    field_literal = _rust_variant_wire_name(
                        variant_name, variant, rust_items[field_type]
                    )
        expected_literal = type_field["type"].strip("'")
        has_parent_tag = expected_literal in parent_tags.get(binding["item"], set())
        if not has_tag and not has_renamed_field and not has_parent_tag:
            diffs.append(
                (
                    binding["item"],
                    f"{item['file']}:{item['line']}",
                    [
                        f'SDK discriminator `{binding["sdk_name"]}.type: {type_field["type"]}` requires `serde(tag = "type")` or a `type_` field renamed to `type`'
                    ],
                )
            )
            continue
        if field_literal and field_literal != expected_literal:
            diffs.append(
                (
                    binding["item"],
                    f"{item['file']}:{field['line']}",
                    [
                        f"Rust discriminator literal `{field_literal}` differs from SDK `{expected_literal}`"
                    ],
                )
            )
    return diffs


def _rust_type_is_json_value(type_text):
    if not type_text:
        return False
    return re.search(r"(^|[<,\s:])(?:serde_json::)?Value\b", type_text) is not None


def _sdk_type_is_complex_union(type_text):
    normalized = _normalize_ts_type(type_text)
    parts = _split_top_level_union(normalized)
    return len(parts) > 1 and any(part.startswith("Array<") for part in parts)


def serde_field_diffs(sdk_text, only_marked=False):
    """Check Rust field serde names and optional/null semantics against SDK shapes."""
    sdk_shapes = sdk_comment_shapes(sdk_text)
    rust_items = rust_serde_items()
    sdk_markers = rust_sdk_markers()
    suppressed_by_item = sdk_markers.get("field_suppressed", {})
    parent_tags = rust_tagged_variant_literals()
    diffs = []
    for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked):
        sdk_shape = binding["sdk_shape"]
        item = rust_items.get(binding["item"])
        if not sdk_shape or not item or sdk_shape.get("kind") != "interface":
            continue
        item_suppressed = suppressed_by_item.get(binding["item"], set())
        for sdk_field in sdk_shape.get("fields", []):
            wire_name = sdk_field["name"]
            rust_name, rust_field = _field_by_wire_name(item, wire_name)
            if not rust_field:
                if wire_name in item_suppressed or _is_covered_discriminator(
                    binding, sdk_field, parent_tags
                ):
                    continue
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{item['line']}",
                        [
                            f"SDK field `{binding['sdk_name']}.{wire_name}` has no Rust field with matching wire name"
                        ],
                    )
                )
                continue
            if rust_name != wire_name and not _serde_rename(
                rust_field.get("attrs", [])
            ):
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{rust_field['line']}",
                        [
                            f'Rust field `{rust_name}` maps to SDK `{wire_name}` by name convention; add explicit `#[serde(rename = "{wire_name}")]`'
                        ],
                    )
                )
            ts_nullable = _ts_type_has_null(sdk_field["type"])
            ts_optional = sdk_field["optional"]
            rust_optional = _rust_type_is_option(rust_field.get("type"))
            rust_skips_none = _serde_skip_if_none(rust_field.get("attrs", []))
            if (ts_optional or ts_nullable) and not rust_optional:
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{rust_field['line']}",
                        [
                            f"SDK field `{binding['sdk_name']}.{wire_name}` is optional/nullable but Rust type is not `Option<_>`"
                        ],
                    )
                )
            if not ts_optional and not ts_nullable and rust_optional:
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{rust_field['line']}",
                        [
                            f"SDK field `{binding['sdk_name']}.{wire_name}` is required and non-null but Rust uses `Option<_>`"
                        ],
                    )
                )
            if ts_optional and not rust_skips_none:
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{rust_field['line']}",
                        [
                            f"SDK field `{binding['sdk_name']}.{wire_name}` is optional but Rust field does not skip `None` when serializing"
                        ],
                    )
                )
            if not ts_optional and ts_nullable and rust_skips_none:
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{rust_field['line']}",
                        [
                            f"SDK field `{binding['sdk_name']}.{wire_name}` is nullable but required; Rust should not skip `None`"
                        ],
                    )
                )
            rust_type = rust_field.get("type")
            if (
                wire_name not in item_suppressed
                and _sdk_type_is_complex_union(sdk_field["type"])
                and _rust_type_is_json_value(rust_type)
            ):
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{rust_field['line']}",
                        [
                            f'SDK field `{binding["sdk_name"]}.{wire_name}` is a structured union `{sdk_field["type"]}` but Rust uses `{rust_type}`; model the union explicitly or add `@sdk(field_suppress = "{wire_name}")` if intentionally loose'
                        ],
                    )
                )
    return diffs


def required_nullable_accepts_missing_diffs(sdk_text, only_marked=False):
    """Require markers for SDK required-nullable fields represented by plain Option.

    Rust `Option<T>` accepts both missing and null during deserialization, while
    SDK `field: T | null` requires the field to be present. The marker records
    intentional compatibility leniency.
    """
    sdk_shapes = sdk_comment_shapes(sdk_text)
    rust_items = rust_serde_items()
    sdk_markers = rust_sdk_markers()
    marked_by_item = sdk_markers.get("required_nullable_accepts_missing", {})
    diffs = []
    marked = []
    valid_markers = {}

    for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked):
        sdk_shape = binding["sdk_shape"]
        item = rust_items.get(binding["item"])
        if not sdk_shape or not item or sdk_shape.get("kind") != "interface":
            continue
        item_markers = marked_by_item.get(binding["item"], set())
        for sdk_field in sdk_shape.get("fields", []):
            wire_name = sdk_field["name"]
            if sdk_field["optional"] or not _ts_type_has_null(sdk_field["type"]):
                continue
            _rust_name, rust_field = _field_by_wire_name(item, wire_name)
            if not rust_field or not _rust_type_is_option(rust_field.get("type")):
                continue

            valid_markers.setdefault(binding["item"], set()).add(wire_name)
            row = (binding["item"], wire_name, item["file"], rust_field["line"])
            if wire_name in item_markers:
                marked.append(row)
            else:
                diffs.append(
                    (
                        binding["item"],
                        f"{item['file']}:{rust_field['line']}",
                        [
                            f'SDK field `{binding["sdk_name"]}.{wire_name}` is required nullable, but Rust `Option<_>` also accepts missing; add `@sdk(required_nullable_accepts_missing = "{wire_name}")` if intentional'
                        ],
                    )
                )

    for item_name, fields in marked_by_item.items():
        stale = sorted(fields - valid_markers.get(item_name, set()))
        if stale:
            item = rust_items.get(item_name)
            locn = f"{item['file']}:{item['line']}" if item else "?"
            diffs.append(
                (
                    item_name,
                    locn,
                    [
                        f"`@sdk(required_nullable_accepts_missing)` does not match a required-nullable `Option<_>` field: {', '.join(stale)}"
                    ],
                )
            )

    return diffs, marked


def field_suppress_diffs(sdk_text, only_marked=False):
    """Validate that `@sdk(field_suppress)` markers correspond to real field shape differences."""
    sdk_shapes = sdk_comment_shapes(sdk_text)
    rust_items = rust_serde_items()
    sdk_markers = rust_sdk_markers()
    suppressed_by_item = sdk_markers.get("field_suppressed", {})
    diffs = []
    bindings = {
        binding["item"]: binding
        for binding in rust_item_shape_bindings(sdk_shapes, only_marked=False)
    }
    for item_name, suppressed_fields in suppressed_by_item.items():
        binding = bindings.get(item_name)
        item = rust_items.get(item_name)
        if not binding or not item:
            diffs.append(
                (
                    item_name,
                    "?",
                    [
                        "field suppress marker is attached to an item without an SDK shape binding"
                    ],
                )
            )
            continue
        sdk_shape = binding["sdk_shape"]
        sdk_fields = (
            {field["name"] for field in sdk_shape.get("fields", [])}
            if sdk_shape
            else set()
        )
        rust_fields = {
            _field_by_wire_name(item, field_name)[0] for field_name in sdk_fields
        }
        rust_fields = {name for name in rust_fields if name}
        rust_wire_names = {
            _field_by_wire_name(item, field_name)[0] and field_name
            for field_name in sdk_fields
        }
        rust_wire_names = {name for name in rust_wire_names if name}
        extra_fields = set(item.get("fields", {})) - rust_fields
        valid = (sdk_fields - rust_wire_names) | extra_fields
        stale = sorted(field for field in suppressed_fields if field not in valid)
        if stale:
            diffs.append(
                (
                    item_name,
                    f"{item['file']}:{item['line']}",
                    [
                        f"`@sdk(field_suppress)` does not match an actual SDK/Rust field difference: {', '.join(stale)}"
                    ],
                )
            )
    return diffs


def enum_literal_diffs(sdk_text):
    """Compare Rust enum serde literals against SDK string literal unions."""
    sdk_shapes = sdk_comment_shapes(sdk_text)
    rust_items = rust_serde_items()
    sdk_literals = sdk_literal_aliases(sdk_shapes)
    diffs = []
    for sdk_name, expected in sdk_literals.items():
        rust_item = rust_items.get(sdk_name)
        if not rust_item or rust_item.get("kind") != "enum_item":
            continue
        actual = rust_enum_literals(rust_item)
        if expected != actual:
            diffs.append(
                (
                    sdk_name,
                    f"{rust_item['file']}:{rust_item['line']}",
                    [
                        f"enum literals differ: SDK `{', '.join(sorted(expected))}` vs Rust `{', '.join(sorted(actual))}`"
                    ],
                )
            )
    return diffs


def untagged_union_diffs(sdk_text, only_marked=False):
    """Compare untagged Rust enum payload coverage against SDK union aliases."""
    sdk_shapes = sdk_comment_shapes(sdk_text)
    rust_items = rust_serde_items()
    diffs = []
    for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked):
        sdk_shape = binding["sdk_shape"]
        item = rust_items.get(binding["item"])
        if not item or item.get("kind") != "enum_item":
            continue
        if not _serde_attr_has(item.get("attrs", []), "serde(untagged)"):
            continue
        if sdk_shape.get("kind") != "type":
            continue
        sdk_parts = {
            re.sub(r"^Array<(.+)>$", r"Vec<\1>", part)
            for part in _split_top_level_union(sdk_shape.get("rhs", ""))
        }
        rust_payloads = set()
        for variant in item.get("variants", {}).values():
            payloads = variant.get("payloads", [])
            if not payloads:
                continue
            if "Vec" in payloads and len(payloads) >= 2:
                inner = next(p for p in payloads if p != "Vec")
                rust_payloads.add(f"Vec<{inner}>")
            else:
                rust_payloads.add(payloads[0])
        if sdk_parts != rust_payloads:
            diffs.append(
                (
                    binding["item"],
                    f"{item['file']}:{item['line']}",
                    [
                        f"untagged union payloads differ: SDK `{', '.join(sorted(sdk_parts))}` vs Rust `{', '.join(sorted(rust_payloads))}`"
                    ],
                )
            )
    return diffs


def _shape_diffs(sdk_shape, rust_shape):
    diffs = []
    if sdk_shape["kind"] != rust_shape["kind"]:
        return [
            f"kind mismatch: SDK {sdk_shape['kind']} vs comment {rust_shape['kind']}"
        ]
    if sdk_shape["kind"] == "type":
        if _normalize_ts_type(sdk_shape["rhs"]) != _normalize_ts_type(
            rust_shape["rhs"]
        ):
            diffs.append(
                f"RHS differs: SDK `{sdk_shape['rhs']}` vs comment `{rust_shape['rhs']}`"
            )
        return diffs

    if sdk_shape.get("extends", "") != rust_shape.get("extends", ""):
        diffs.append(
            f"extends differs: SDK `{sdk_shape.get('extends', '')}` vs comment `{rust_shape.get('extends', '')}`"
        )

    sdk_fields = sdk_shape.get("fields", [])
    rust_fields = rust_shape.get("fields", [])
    sdk_names = [f["name"] for f in sdk_fields]
    rust_names = [f["name"] for f in rust_fields]
    if sdk_names != rust_names:
        diffs.append(
            f"field order differs: SDK `{', '.join(sdk_names)}` vs comment `{', '.join(rust_names)}`"
        )

    sdk_by_name = {f["name"]: f for f in sdk_fields}
    rust_by_name = {f["name"]: f for f in rust_fields}
    for name in sorted(set(sdk_by_name) & set(rust_by_name), key=sdk_names.index):
        sf = sdk_by_name[name]
        rf = rust_by_name[name]
        if sf["optional"] != rf["optional"]:
            diffs.append(f"{name}: optional differs")
        if _normalize_ts_type(sf["type"]) != _normalize_ts_type(rf["type"]):
            diffs.append(
                f"{name}: type differs: SDK `{sf['type']}` vs comment `{rf['type']}`"
            )
    return diffs


def _format_sdk_shape_doc(shape):
    lines = []
    if shape["kind"] == "type":
        parts = _split_top_level_union(shape["rhs"])
        if len(parts) <= 1:
            return [f"/// export type {shape['name']} = {shape['rhs']};"]
        lines.append(f"/// export type {shape['name']} =")
        for i, part in enumerate(parts):
            suffix = ";" if i == len(parts) - 1 else ""
            lines.append(f"///   | {part}{suffix}")
        return lines
    extends = f" extends {shape['extends']}" if shape.get("extends") else ""
    lines.append(f"/// export interface {shape['name']}{extends} {{")
    for field in shape.get("fields", []):
        opt = "?" if field.get("optional") else ""
        lines.append(f"///   {field['name']}{opt}: {field['type']};")
    lines.append("/// }")
    return lines


def emit_sdk_docs(only_marked=False):
    sdk_shapes = sdk_comment_shapes(SDK_FILE.read_text(encoding="utf-8"))
    for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked):
        out(
            f"{binding['file']}:{binding['line']} {binding['item']} -> {binding['sdk_name']}"
        )
        for line in _format_sdk_shape_doc(binding["sdk_shape"]):
            out(line)
        out()


def comment_shape_diffs(sdk_text, only_marked=False):
    sdk_shapes = sdk_comment_shapes(sdk_text)
    sdk_markers = rust_sdk_markers()
    diffs = []
    for ref in rust_field_shape_comments():
        locn = f"{ref['file']}:{ref['line']}"
        sdk_shape = sdk_shapes.get(ref["owner"])
        if not sdk_shape:
            diffs.append(
                (
                    f"{ref['owner']}.{ref['field']}",
                    locn,
                    [f"SDK export not found for `{ref['owner']}`"],
                )
            )
            continue
        if sdk_shape.get("kind") != "interface":
            diffs.append(
                (
                    f"{ref['owner']}.{ref['field']}",
                    locn,
                    [f"SDK `{ref['owner']}` is not an interface"],
                )
            )
            continue
        sdk_fields = {field["name"]: field for field in sdk_shape.get("fields", [])}
        sdk_field = sdk_fields.get(ref["field"])
        if not sdk_field:
            diffs.append(
                (
                    f"{ref['owner']}.{ref['field']}",
                    locn,
                    [f"SDK field not found for `{ref['owner']}.{ref['field']}`"],
                )
            )
            continue
        if _normalize_ts_type(sdk_field["type"]) != ref["type"]:
            diffs.append(
                (
                    f"{ref['owner']}.{ref['field']}",
                    locn,
                    [
                        f"type differs: SDK `{sdk_field['type']}` vs comment `{ref['type']}`"
                    ],
                )
            )
    for rel, line_no, text in uncontrolled_ts_comment_fragments():
        diffs.append(
            (
                "uncontrolled TS-looking comment",
                f"{rel}:{line_no}",
                [f"rewrite as `export ...` or `Type.field: `...``: {text}"],
            )
        )
    for name, locn, use_diffs in use_annotation_diffs():
        diffs.append((f"use annotation for {name}", locn, use_diffs))
    for name, locn, ref_diffs in use_reference_diffs():
        diffs.append((f"use references for {name}", locn, ref_diffs))
    if not only_marked:
        for name, locn, binding_diffs in shape_binding_diffs(sdk_shapes):
            diffs.append((f"shape binding for {name}", locn, binding_diffs))
    for name, locn, marker_diffs in sdk_markers.get("legacy", []):
        diffs.append((f"sdk marker for {name}", locn, marker_diffs))
    return diffs


def proxai_internal_diffs(sdk_text, only_marked=False):
    """Require Proxai-only Rust types to carry a structured internal marker."""
    if only_marked:
        return []
    sdk_shapes = sdk_comment_shapes(sdk_text)
    sdk_markers = rust_sdk_markers()
    rust_items = rust_serde_items()
    sk_base = {norm(name) for name in sdk_shapes}
    bound_items = {
        binding["item"]
        for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked)
    }
    aliased_items = set(sdk_markers.get("aliases", {}).values())
    marked = sdk_markers.get("proxai_internals", {})
    field_literal_wrappers = set()
    for binding in rust_item_shape_bindings(sdk_shapes, only_marked=only_marked):
        sdk_shape = binding["sdk_shape"]
        item = rust_items.get(binding["item"])
        if not item or sdk_shape.get("kind") != "interface":
            continue
        for sdk_field in sdk_shape.get("fields", []):
            if not _ts_string_literals(sdk_field["type"]):
                continue
            _, rust_field = _field_by_wire_name(item, sdk_field["name"])
            if not rust_field or not rust_field.get("type"):
                continue
            type_names = [
                name
                for name in _type_names_from_text(rust_field["type"])
                if name in rust_items
            ]
            field_literal_wrappers.update(type_names)
    diffs = []
    for name, item in rust_items.items():
        if norm(name) in sk_base:
            continue
        if (
            name in bound_items
            or name in aliased_items
            or name in field_literal_wrappers
        ):
            continue
        if _serde_attr_has(item.get("attrs", []), "serde(untagged)") or _serde_attr_has(
            item.get("attrs", []), 'serde(tag = "type")'
        ):
            continue
        if name in marked:
            continue
        diffs.append(
            (
                name,
                f"{item['file']}:{item['line']}",
                ['Proxai-only type must declare `@sdk(proxai_internal = "...")`'],
            )
        )
    return diffs
