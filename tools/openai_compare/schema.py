"""Official OpenAPI loading, resolution, and shape inspection."""

import yaml

from .common import SCHEMA_PATH

def _load_schemas():
    if not SCHEMA_PATH.exists():
        raise SystemExit(
            f"official OpenAI schema is missing: {SCHEMA_PATH}; initialize submodules"
        )

    document = yaml.safe_load(SCHEMA_PATH.read_text(encoding="utf-8"))
    schemas = document.get("components", {}).get("schemas", {})
    if not schemas:
        raise SystemExit(f"OpenAPI schema has no components.schemas: {SCHEMA_PATH}")
    return document, schemas

def _union_discriminator_payload_options(schema, tag, schemas, seen):
    """Map each discriminator value to every official payload schema it permits."""
    reference = schema.get("$ref")
    if reference:
        if reference in seen:
            return None
        target = _resolve_ref(reference, schemas)
        if target is None:
            return None
        if target.get("oneOf") or target.get("anyOf"):
            return _union_discriminator_payload_options(
                target, tag, schemas, seen | {reference}
            )
        properties, _ = _object_contract(target, schemas, seen | {reference})
        values = _closed_string_enum_values(
            properties.get(tag, {}), schemas, set()
        )
        if values is None or len(values) != 1:
            return None
        payload_name = reference.removeprefix("#/components/schemas/")
        return {next(iter(values)): {payload_name}}

    branches = [
        branch
        for keyword in ("oneOf", "anyOf")
        for branch in schema.get(keyword, [])
        if not _allows_null(branch, schemas, set())
    ]
    if not branches:
        return None
    payloads = {}
    for branch in branches:
        branch_payloads = _union_discriminator_payload_options(
            branch, tag, schemas, seen
        )
        # A union may contain a branch whose payload cannot be mapped through
        # this tag. Retain every mapping that is still objectively auditable.
        if branch_payloads is None:
            continue
        for wire_value, payload_names in branch_payloads.items():
            payloads.setdefault(wire_value, set()).update(payload_names)
    return payloads or None


def _union_discriminator_payloads(schema, tag, schemas, seen):
    """Return only discriminator values with one unambiguous payload schema."""
    options = _union_discriminator_payload_options(schema, tag, schemas, seen)
    if not options:
        return None
    payloads = {
        wire_value: next(iter(payload_names))
        for wire_value, payload_names in options.items()
        if len(payload_names) == 1
    }
    return payloads or None

def _union_discriminator_values(schema, tag, schemas, seen):
    if "$ref" in schema:
        reference = schema["$ref"]
        if reference in seen:
            return None
        target = _resolve_ref(reference, schemas)
        if target is None:
            return None
        return _union_discriminator_values(target, tag, schemas, seen | {reference})

    branches = [
        branch
        for keyword in ("oneOf", "anyOf")
        for branch in schema.get(keyword, [])
        if not _allows_null(branch, schemas, set())
    ]
    if not branches:
        return None

    values = set()
    for branch in branches:
        nested = _union_discriminator_values(branch, tag, schemas, seen)
        if nested is not None:
            values.update(nested)
            continue
        properties, _ = _object_contract(branch, schemas, seen)
        branch_values = _closed_string_enum_values(
            properties.get(tag, {}), schemas, set()
        )
        if not branch_values:
            return None
        values.update(branch_values)
    return values

def _schema_array_item_identity(schema, schemas, seen):
    item_schema = _schema_array_item_schema(schema, schemas, seen)
    return (
        _schema_item_identity(item_schema, schemas, seen)
        if item_schema is not None
        else None
    )

def _schema_array_item_schema(schema, schemas, seen):
    if "$ref" in schema:
        reference = schema["$ref"]
        if reference in seen:
            return None
        target = _resolve_ref(reference, schemas)
        if target is None:
            return None
        return _schema_array_item_schema(target, schemas, seen | {reference})

    if schema.get("type") == "array":
        return schema.get("items")

    branches = [
        branch
        for keyword in ("anyOf", "oneOf")
        for branch in schema.get(keyword, [])
        if not _allows_null(branch, schemas, set())
    ]
    item_schemas = [
        item_schema
        for branch in branches
        if (item_schema := _schema_array_item_schema(branch, schemas, seen))
        is not None
    ]
    return item_schemas[0] if len(branches) == 1 and len(item_schemas) == 1 else None

def _schema_item_identity(schema, schemas, seen):
    reference = schema.get("$ref")
    if reference and reference.startswith("#/components/schemas/"):
        target = _resolve_ref(reference, schemas)
        if target is not None:
            shapes = _schema_shape_categories(target, schemas, seen | {reference})
            primitive_shapes = shapes & {"string", "boolean", "integer", "number", "array"}
            if len(shapes) == 1 and len(primitive_shapes) == 1:
                return next(iter(primitive_shapes))
        return reference.removeprefix("#/components/schemas/")

    schema_type = schema.get("type")
    if schema_type in {"string", "boolean", "integer", "number", "array"}:
        return schema_type

    branches = [
        branch
        for keyword in ("anyOf", "oneOf")
        for branch in schema.get(keyword, [])
        if not _allows_null(branch, schemas, set())
    ]
    if not branches:
        return None
    identities = [
        _schema_item_identity(branch, schemas, seen) for branch in branches
    ]
    if any(identity is None for identity in identities):
        return None
    unique = set(identities)
    return next(iter(unique)) if len(unique) == 1 else None

def _schema_shape_categories(schema, schemas, seen):
    if "$ref" in schema:
        reference = schema["$ref"]
        if reference in seen:
            return set()
        target = _resolve_ref(reference, schemas)
        if target is None:
            return set()
        return _schema_shape_categories(target, schemas, seen | {reference})

    shapes = set()
    schema_type = schema.get("type")
    if isinstance(schema_type, str) and schema_type != "null":
        shapes.add(schema_type)
    elif isinstance(schema_type, list):
        shapes.update(value for value in schema_type if value != "null")

    if "properties" in schema or "additionalProperties" in schema or "allOf" in schema:
        shapes.add("object")
    for keyword in ("anyOf", "oneOf"):
        for branch in schema.get(keyword, []):
            shapes.update(_schema_shape_categories(branch, schemas, seen))
    return shapes

def _closed_string_enum_values(schema, schemas, seen):
    if "$ref" in schema:
        reference = schema["$ref"]
        if reference in seen:
            return None
        target = _resolve_ref(reference, schemas)
        if target is None:
            return None
        return _closed_string_enum_values(target, schemas, seen | {reference})

    values = schema.get("enum")
    if values and all(isinstance(value, str) for value in values):
        return set(values)

    branches = [
        part
        for keyword in ("anyOf", "oneOf")
        for part in schema.get(keyword, [])
        if not _allows_null(part, schemas, set())
    ]
    if len(branches) == 1:
        return _closed_string_enum_values(branches[0], schemas, seen)
    return None

def _all_of_component_types(schema, schemas, seen):
    if "$ref" in schema:
        reference = schema["$ref"]
        if reference in seen:
            return set()
        target = _resolve_ref(reference, schemas)
        if target is None:
            return set()
        return _all_of_component_types(target, schemas, seen | {reference})

    types = set()
    for part in schema.get("allOf", []):
        reference = part.get("$ref")
        if reference and reference.startswith("#/components/schemas/"):
            types.add(reference.removeprefix("#/components/schemas/"))
        types.update(_all_of_component_types(part, schemas, seen))
    return types

def _schema_for_type(type_name, pointer, document, schemas):
    if pointer is None:
        return schemas.get(type_name), None

    schema = _resolve_json_pointer(document, pointer)
    if schema is None:
        return None, f"explicit schema provenance does not resolve: {pointer}"
    return schema, None

def _resolve_json_pointer(document, pointer):
    if not pointer.startswith("#/"):
        return None

    value = document
    for segment in pointer.removeprefix("#/").split("/"):
        segment = segment.replace("~1", "/").replace("~0", "~")
        if isinstance(value, dict):
            value = value.get(segment)
        elif isinstance(value, list) and segment.isdigit():
            index = int(segment)
            value = value[index] if index < len(value) else None
        else:
            return None
        if value is None:
            return None
    return value

def _object_contract(schema, schemas, seen):
    """Return the merged properties and required fields for an object schema."""
    if "$ref" in schema:
        reference = schema["$ref"]
        if reference in seen:
            return {}, set()
        target = _resolve_ref(reference, schemas)
        if target is None:
            return {}, set()
        return _object_contract(target, schemas, seen | {reference})

    properties = dict(schema.get("properties", {}))
    required = set(schema.get("required", []))
    for part in schema.get("allOf", []):
        part_properties, part_required = _object_contract(part, schemas, seen)
        properties.update(part_properties)
        required.update(part_required)
    return properties, required

def _allows_null(schema, schemas, seen):
    schema_type = schema.get("type")
    if schema_type == "null" or (isinstance(schema_type, list) and "null" in schema_type):
        return True
    if schema.get("nullable") is True:
        return True

    if "$ref" in schema:
        reference = schema["$ref"]
        if reference in seen:
            return False
        target = _resolve_ref(reference, schemas)
        return target is not None and _allows_null(target, schemas, seen | {reference})

    return any(
        _allows_null(part, schemas, seen)
        for keyword in ("anyOf", "oneOf")
        for part in schema.get(keyword, [])
    )

def _resolve_ref(reference, schemas):
    prefix = "#/components/schemas/"
    if not reference.startswith(prefix):
        return None
    return schemas.get(reference.removeprefix(prefix))
