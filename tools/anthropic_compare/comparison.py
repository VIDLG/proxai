"""Anthropic SDK/Rust comparison orchestration and result model."""

import json
from dataclasses import dataclass

from .checks import (
    comment_shape_diffs,
    enum_literal_diffs,
    explicit_provenance_diffs,
    field_carrier_diffs,
    field_suppress_diffs,
    named_field_type_diffs,
    proxai_internal_diffs,
    serde_field_diffs,
    serde_wire_diffs,
    untagged_union_diffs,
)
from .common import SDK_FILE, SDK_PKG, norm
from .rust import px_enum_variants, px_types, rust_sdk_markers
from .sdk import sdk_tool_union, sdk_types


def _norm_field(name):
    value = name.lower().replace("_", "").replace("-", "").replace("#", "").rstrip(";")
    if value.startswith("r"):
        value = value.lstrip("r")
    return value


def _structural_diffs(matched, sdk_index, rust_index, sdk_markers):
    structural_diffs = []
    matched_count = 0
    field_suppressed = sdk_markers.get("field_suppressed", {})
    for normalized_name in sorted(matched):
        sdk_name, sdk_info, _tag = sdk_index[normalized_name]
        rust_name, rust_info = rust_index[normalized_name]
        if sdk_info["kind"] != "interface" or rust_info["kind"] != "struct":
            matched_count += 1
            continue

        sdk_fields = sdk_info.get("fields", [])
        rust_fields = rust_info.get("fields", [])
        sdk_normalized = {_norm_field(field): field for field in sdk_fields}
        rust_normalized = {_norm_field(field): field for field in rust_fields}
        sdk_field_names = set(sdk_normalized)
        rust_field_names = set(rust_normalized)
        exclusions = {
            _norm_field(field)
            for field in sdk_info.get("deprecated_fields", set())
            | rust_info.get("deprecated_fields", set())
        }
        missing_fields = [
            sdk_normalized[field]
            for field in sorted(sdk_field_names - rust_field_names - exclusions)
        ]
        extra_fields = [
            rust_normalized[field]
            for field in sorted(rust_field_names - sdk_field_names)
        ]

        common_sdk = []
        common_rust = []
        for field in sorted(sdk_field_names & rust_field_names):
            common_sdk.extend(item for item in sdk_fields if _norm_field(item) == field)
            common_rust.extend(
                item for item in rust_fields if _norm_field(item) == field
            )
        sdk_order = [_norm_field(field) for field in common_sdk]
        rust_order = [_norm_field(field) for field in common_rust]
        order_mismatch = (
            (common_sdk, common_rust)
            if common_sdk and common_rust and sdk_order != rust_order
            else None
        )

        suppressed = {
            _norm_field(field)
            for field in field_suppressed.get(rust_name, set())
        }
        missing_fields = [
            field
            for field in missing_fields
            if _norm_field(field) != "type" and _norm_field(field) not in suppressed
        ]
        extra_fields = [
            field for field in extra_fields if _norm_field(field) not in suppressed
        ]

        if missing_fields or extra_fields or order_mismatch:
            structural_diffs.append(
                (sdk_name, rust_name, missing_fields, extra_fields, order_mismatch)
            )
        else:
            matched_count += 1
    return structural_diffs, matched_count


def _missing_tool_union_variants(sdk_variants, rust_variants, variant_map):
    sdk_normalized = {norm(variant): variant for variant in sdk_variants}
    rust_normalized = {norm(variant): variant for variant in rust_variants}
    missing = []
    for key in sorted(set(sdk_normalized) - set(rust_normalized)):
        variant = sdk_normalized[key]
        if variant in variant_map and norm(variant_map[variant]) in rust_normalized:
            continue
        missing.append(variant)
    return missing


@dataclass(frozen=True)
class AnthropicComparison:
    sdk_version: str
    sdk_types: dict
    rust_types: dict
    matched: set
    sdk_index: dict
    sdk_only_count: int
    missing_types: list
    namespaced_types: list
    external_types: list
    api_classes: list
    aliases: list
    skipped_types: list
    structural_diffs: list
    has_missing_fields: bool
    sdk_tool_union: list
    rust_tool_union: list
    missing_tool_variants: list
    comment_diffs: list
    provenance_diffs: list
    serde_diffs: list
    serde_field_diffs: list
    field_carrier_diffs: list
    named_field_type_diffs: list
    required_nullable_fields: list
    field_suppress_diffs: list
    enum_diffs: list
    union_diffs: list
    proxai_only_diffs: list
    has_gaps: bool


def compare_protocol(*, only_marked=False):
    sdk_version = ""
    if SDK_PKG.exists():
        try:
            sdk_version = json.loads(SDK_PKG.read_text()).get("version", "")
        except Exception:
            pass

    text = SDK_FILE.read_text(encoding="utf-8")
    sdk_raw = sdk_types(text)
    rust_raw = px_types()
    comment_diffs = comment_shape_diffs(text, only_marked=only_marked)
    provenance_diffs = explicit_provenance_diffs(text, only_marked=only_marked)
    serde_diffs = serde_wire_diffs(text, only_marked=only_marked)
    serde_field_semantic_diffs = serde_field_diffs(text, only_marked=only_marked)
    field_carrier_semantic_diffs, required_nullable_modeled = field_carrier_diffs(
        text, only_marked=only_marked
    )
    named_field_type_semantic_diffs = named_field_type_diffs(
        text, only_marked=only_marked
    )
    field_suppress_marker_diffs = field_suppress_diffs(
        text, only_marked=only_marked
    )
    enum_semantic_diffs = enum_literal_diffs(text)
    union_semantic_diffs = untagged_union_diffs(text, only_marked=only_marked)
    proxai_only_diffs = proxai_internal_diffs(text, only_marked=only_marked)
    sdk_markers = rust_sdk_markers()
    naming_map = sdk_markers["aliases"]
    external_type_names = sdk_markers["externals"]
    skip_type_names = sdk_markers["internals"]
    union_variant_map = sdk_markers["union_variants"]

    sdk_index = {}
    for tag, info in sdk_raw.items():
        base = tag.split(".")[-1]
        sdk_index[norm(base)] = (base, info, tag)

    rust_index = {
        norm(name): (name, info)
        for name, info in rust_raw.items()
    }
    sdk_names = set(sdk_index)
    rust_names = set(rust_index)
    matched = sdk_names & rust_names

    reverse_alias = {}
    for tag, info in sdk_raw.items():
        alias_of = info.get("alias_of")
        if alias_of:
            reverse_alias.setdefault(norm(alias_of), []).append(
                info.get("alias_name", "")
            )

    covered = set()
    for normalized_name in sorted(sdk_names - rust_names):
        base, info, tag = sdk_index[normalized_name]
        if (
            "." in tag
            or info["kind"] == "class"
            or base in external_type_names
            or base in skip_type_names
        ):
            continue
        alias_names = reverse_alias.get(normalized_name, [])
        for alias_name in alias_names:
            if alias_name and norm(alias_name) in rust_names:
                covered.add(normalized_name)
                break
        else:
            alias_of = info.get("alias_of")
            if alias_of and norm(alias_of) in rust_names:
                covered.add(normalized_name)
            elif base.startswith("Raw") and len(base) > 3 and norm(base[3:]) in rust_names:
                covered.add(normalized_name)
            elif base in naming_map and norm(naming_map[base]) in rust_names:
                covered.add(normalized_name)

    missing_types = []
    namespaced_types = []
    external_types = []
    api_classes = []
    aliases = []
    skipped_types = []

    for normalized_name in sorted(sdk_names - rust_names):
        base, info, tag = sdk_index[normalized_name]
        if info["kind"] == "class":
            api_classes.append((base, info, tag))
        elif base in external_type_names:
            external_types.append((base, info, tag))
        elif base in skip_type_names:
            skipped_types.append((base, info, tag))
        elif "." in tag:
            parent = tag.rsplit(".", 1)[0]
            parent_base = parent.split(".")[-1]
            parent_name = norm(parent_base)
            if parent_name in rust_names or parent_name in covered:
                namespaced_types.append((base, info, tag, parent_base))
            else:
                missing_types.append((base, info, tag))
        elif normalized_name in covered:
            matched_name = ""
            for alias_name in reverse_alias.get(normalized_name, []):
                if alias_name and norm(alias_name) in rust_names:
                    matched_name = alias_name
                    break
            if not matched_name and base.startswith("Raw") and len(base) > 3:
                matched_name = base[3:]
            if not matched_name and base in naming_map:
                matched_name = naming_map[base]
            if not matched_name:
                alias_of = info.get("alias_of", "")
                if alias_of and norm(alias_of) in rust_names:
                    matched_name = alias_of
            aliases.append((base, info, tag, matched_name))
        else:
            missing_types.append((base, info, tag))

    structural_diffs, _ = _structural_diffs(
        matched, sdk_index, rust_index, sdk_markers
    )
    sdk_tool_variants = sdk_tool_union(text)
    rust_tool_variants = px_enum_variants("tools/mod.rs", "ToolUnion")
    missing_tool_variants = _missing_tool_union_variants(
        sdk_tool_variants, rust_tool_variants, union_variant_map
    )
    has_missing_fields = any(
        missing for _, _, missing, _, _ in structural_diffs
    )
    has_gaps = bool(
        missing_types
        or missing_tool_variants
        or has_missing_fields
        or comment_diffs
        or provenance_diffs
        or serde_diffs
        or serde_field_semantic_diffs
        or field_carrier_semantic_diffs
        or named_field_type_semantic_diffs
        or field_suppress_marker_diffs
        or enum_semantic_diffs
        or union_semantic_diffs
        or proxai_only_diffs
    )

    return AnthropicComparison(
        sdk_version=sdk_version,
        sdk_types=sdk_raw,
        rust_types=rust_raw,
        matched=matched,
        sdk_index=sdk_index,
        sdk_only_count=len(sdk_names - rust_names),
        missing_types=missing_types,
        namespaced_types=namespaced_types,
        external_types=external_types,
        api_classes=api_classes,
        aliases=aliases,
        skipped_types=skipped_types,
        structural_diffs=structural_diffs,
        has_missing_fields=has_missing_fields,
        sdk_tool_union=sdk_tool_variants,
        rust_tool_union=rust_tool_variants,
        missing_tool_variants=missing_tool_variants,
        comment_diffs=comment_diffs,
        provenance_diffs=provenance_diffs,
        serde_diffs=serde_diffs,
        serde_field_diffs=serde_field_semantic_diffs,
        field_carrier_diffs=field_carrier_semantic_diffs,
        named_field_type_diffs=named_field_type_semantic_diffs,
        required_nullable_fields=required_nullable_modeled,
        field_suppress_diffs=field_suppress_marker_diffs,
        enum_diffs=enum_semantic_diffs,
        union_diffs=union_semantic_diffs,
        proxai_only_diffs=proxai_only_diffs,
        has_gaps=has_gaps,
    )
