"""Structural contract checks between Rust wire models and OpenAPI schemas."""

import re

from tools.protocol_compare import (
    classify_rust_carrier,
    expected_carrier,
    validate_field_contract,
)

from .schema import (
    _all_of_component_types,
    _allows_null,
    _closed_string_enum_values,
    _schema_array_item_identity,
    _schema_array_item_schema,
    _schema_shape_categories,
    _union_discriminator_payloads,
    _union_discriminator_values,
)
from .rust import (
    _is_serde_tag_type,
    _rust_array_item_base_type,
    _rust_array_item_identity,
    _rust_base_type,
    _rust_builtin_shape,
    _serde_tag_values,
)


def _check_field_carriers(
    type_name, local, field_attributes, properties, required, schemas, gaps
):
    for field_name, property_schema in properties.items():
        rust_type = local.get(field_name)
        if rust_type is None or _is_serde_tag_type(rust_type):
            continue

        expected = expected_carrier(
            required=field_name in required,
            nullable=_allows_null(property_schema, schemas, set()),
        )
        actual = classify_rust_carrier(rust_type)
        attributes = field_attributes.get(field_name, [])
        for message in validate_field_contract(
            expected=expected,
            actual=actual,
            attributes=attributes,
            source="official field",
            rust_type=rust_type,
        ):
            gaps.append((type_name, field_name, message))


def _check_tagged_union_variants(enums, schemas, gaps):
    for enum_name, enum in enums.items():
        tag = enum.get("tag")
        schema = schemas.get(enum_name)
        if tag is None or not isinstance(schema, dict):
            continue
        official_values = _union_discriminator_values(schema, tag, schemas, set())
        if official_values is None:
            continue
        local_values = enum["wire_values"]
        if local_values != official_values:
            gaps.append(
                (
                    enum_name,
                    "<variants>",
                    f"tagged union `{tag}` variants differ: expected "
                    f"{sorted(official_values)}, found {sorted(local_values)}",
                )
            )

def _check_tagged_union_payload_types(enums, schemas, gaps):
    for enum_name, enum in enums.items():
        tag = enum.get("tag")
        schema = schemas.get(enum_name)
        if tag is None or not isinstance(schema, dict):
            continue
        official_payloads = _union_discriminator_payloads(
            schema, tag, schemas, set()
        )
        if not official_payloads:
            continue
        for wire_value, local_payload in enum["variant_payloads"].items():
            official_payload = official_payloads.get(wire_value)
            if official_payload is None or local_payload == official_payload:
                continue
            gaps.append(
                (
                    enum_name,
                    wire_value,
                    "tagged union payload type differs: expected official "
                    f"`{official_payload}`, found local `{local_payload}`",
                )
            )

def _check_serde_tag_fields(type_name, local, properties, schemas, gaps):
    for field_name, rust_type in local.items():
        local_values = _serde_tag_values(rust_type)
        if local_values is None or field_name not in properties:
            continue
        official_values = _closed_string_enum_values(
            properties[field_name], schemas, set()
        )
        if official_values is None:
            gaps.append(
                (
                    type_name,
                    field_name,
                    f"serde-generated discriminators {sorted(local_values)} have no auditable official const",
                )
            )
        elif official_values != local_values:
            gaps.append(
                (
                    type_name,
                    field_name,
                    "serde-generated discriminator differs from official payload const: "
                    f"expected {sorted(official_values)}, found {sorted(local_values)}",
                )
            )

def _check_closed_enum_fields(type_name, local, properties, schemas, enums, gaps):
    for field_name, rust_type in local.items():
        if _is_serde_tag_type(rust_type) or field_name not in properties:
            continue
        official_values = _closed_string_enum_values(
            properties[field_name], schemas, set()
        )
        if official_values is None:
            continue

        rust_base = _rust_base_type(rust_type)
        if rust_base == "String":
            gaps.append(
                (
                    type_name,
                    field_name,
                    "official schema is a closed string enum, but local `String` accepts "
                    f"arbitrary values; expected {sorted(official_values)}",
                )
            )
            continue

        enum = enums.get(rust_base)
        if enum is None:
            gaps.append(
                (
                    type_name,
                    field_name,
                    f"official schema is a closed string enum, but local type `{rust_type}` "
                    "has no auditable serde enum contract",
                )
            )
            continue
        if enum["open_string"]:
            gaps.append(
                (
                    type_name,
                    field_name,
                    f"local enum `{rust_base}` has an untagged string fallback for closed "
                    f"official values {sorted(official_values)}",
                )
            )
            continue
        local_values = enum["wire_values"]
        if local_values != official_values:
            gaps.append(
                (
                    type_name,
                    field_name,
                    f"local enum `{rust_base}` wire values differ: expected "
                    f"{sorted(official_values)}, found {sorted(local_values)}",
                )
            )

def _check_field_shapes(type_name, local, properties, schemas, gaps):
    for field_name, rust_type in local.items():
        if _is_serde_tag_type(rust_type) or field_name not in properties:
            continue
        local_shape = _rust_builtin_shape(rust_type)
        if local_shape is None:
            continue
        official_shapes = _schema_shape_categories(properties[field_name], schemas, set())
        if not official_shapes or local_shape in official_shapes:
            continue
        gaps.append(
            (
                type_name,
                field_name,
                f"local built-in carrier shape `{local_shape}` differs from official schema "
                f"shape(s) {sorted(official_shapes)} for Rust type `{rust_type}`",
            )
        )

def _check_array_item_types(type_name, local, properties, schemas, enums, gaps):
    for field_name, rust_type in local.items():
        if _is_serde_tag_type(rust_type) or field_name not in properties:
            continue
        local_item = _rust_array_item_identity(rust_type, enums)
        if local_item is None:
            continue
        official_item = _schema_array_item_identity(
            properties[field_name], schemas, set()
        )
        if official_item is not None and local_item != official_item:
            gaps.append(
                (
                    type_name,
                    field_name,
                    f"local array item type `{local_item}` differs from official schema "
                    f"item type `{official_item}` for Rust type `{rust_type}`",
                )
            )

        local_enum = enums.get(_rust_array_item_base_type(rust_type))
        official_item_schema = _schema_array_item_schema(
            properties[field_name], schemas, set()
        )
        if local_enum is None or official_item_schema is None:
            continue
        official_values = _closed_string_enum_values(
            official_item_schema, schemas, set()
        )
        if official_values is None:
            continue
        if local_enum["open_string"]:
            gaps.append(
                (
                    type_name,
                    field_name,
                    "local array item enum has an untagged string fallback for "
                    f"closed official values {sorted(official_values)}",
                )
            )
        elif local_enum["wire_values"] != official_values:
            gaps.append(
                (
                    type_name,
                    field_name,
                    "local array item enum wire values differ: expected "
                    f"{sorted(official_values)}, found "
                    f"{sorted(local_enum['wire_values'])}",
                )
            )

def _check_flatten_contract(type_name, flattened_types, schema, schemas, gaps):
    """Allow flatten only when the official schema composes that exact `$ref` via allOf."""
    if not flattened_types:
        return

    all_of_types = _all_of_component_types(schema, schemas, set())
    for flattened_type in flattened_types:
        if flattened_type not in all_of_types:
            gaps.append(
                (
                    type_name,
                    "<serde flatten>",
                    f"`#[serde(flatten)] {flattened_type}` has no matching official allOf `$ref`",
                )
            )

def _check_field_order(type_name, local, properties, gaps):
    """Require local fields to retain the official schema property order.

    JSON objects are unordered on the wire, but keeping declarations ordered makes
    schema review and generated JSON inspection deterministic. Serde discriminator
    fields are omitted because no physical Rust field carries them.
    """
    local_order = [
        name
        for name, rust_type in local.items()
        if name in properties and not _is_serde_tag_type(rust_type)
    ]
    schema_order = [name for name in properties if name in local_order]
    if local_order != schema_order:
        gaps.append(
            (
                type_name,
                "<field order>",
                "local fields must follow official schema property order: "
                f"expected {schema_order}, found {local_order}",
            )
        )
