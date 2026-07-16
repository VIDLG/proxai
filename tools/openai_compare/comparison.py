"""OpenAI OpenAPI/Rust comparison orchestration and result model."""

from dataclasses import dataclass

from .checks import (
    _check_array_item_types,
    _check_closed_enum_fields,
    _check_field_carriers,
    _check_field_order,
    _check_field_shapes,
    _check_flatten_contract,
    _check_named_field_types,
    _check_serde_tag_fields,
    _check_tagged_union_payload_types,
    _check_tagged_union_variants,
)
from .common import PROTOCOL_DIRS
from .rust import _is_serde_tag_type, _local_structs
from .schema import _object_contract, _schema_for_type


@dataclass(frozen=True)
class OpenAIComparison:
    protocol: str
    checked: int
    gaps: list
    structural: bool

    @property
    def has_gaps(self):
        return bool(self.gaps)


def compare_protocol(protocol, document, schemas, *, structural):
    (
        local_types,
        local_provenance,
        local_flattens,
        local_enums,
        local_field_attributes,
    ) = _local_structs(PROTOCOL_DIRS[protocol])
    gaps = []
    checked = 0

    for type_name, local in sorted(local_types.items()):
        pointer = local_provenance.get(type_name)
        if structural and pointer is None:
            gaps.append(
                (
                    type_name,
                    "<schema provenance>",
                    "public wire struct is missing an `OpenAPI schema` doc comment",
                )
            )
        schema, provenance_error = _schema_for_type(
            type_name, pointer, document, schemas
        )
        if provenance_error:
            gaps.append((type_name, "<schema provenance>", provenance_error))
            continue
        if not isinstance(schema, dict):
            if structural:
                gaps.append(
                    (
                        type_name,
                        "<schema provenance>",
                        "public wire struct has no matching official schema component or explicit alias",
                    )
                )
            continue

        properties, required = _object_contract(schema, schemas, set())
        if not properties and not required:
            continue
        checked += 1

        for field_name in sorted(required):
            property_schema = properties.get(field_name)
            # Upstream occasionally marks an undeclared property as required.
            if property_schema is None:
                continue

            rust_type = local.get(field_name)
            if _is_serde_tag_type(rust_type):
                continue
            if rust_type is None:
                gaps.append((type_name, field_name, "missing local field"))

        _check_field_carriers(
            type_name,
            local,
            local_field_attributes.get(type_name, {}),
            properties,
            required,
            schemas,
            gaps,
        )

        if structural:
            _check_flatten_contract(
                type_name,
                local_flattens.get(type_name, []),
                schema,
                schemas,
                gaps,
            )
            for field_name in sorted(properties):
                rust_type = local.get(field_name)
                if _is_serde_tag_type(rust_type):
                    continue
                if rust_type is None:
                    gaps.append(
                        (
                            type_name,
                            field_name,
                            "official schema property is not modeled locally",
                        )
                    )

            for field_name, rust_type in sorted(local.items()):
                if not _is_serde_tag_type(rust_type) and field_name not in properties:
                    gaps.append(
                        (
                            type_name,
                            field_name,
                            "local wire field has no official schema property",
                        )
                    )

            _check_serde_tag_fields(type_name, local, properties, schemas, gaps)
            _check_field_order(type_name, local, properties, gaps)
            _check_closed_enum_fields(
                type_name, local, properties, schemas, local_enums, gaps
            )
            _check_field_shapes(type_name, local, properties, schemas, gaps)
            _check_named_field_types(
                type_name, local, properties, schemas, gaps, local_provenance
            )
            _check_array_item_types(
                type_name, local, properties, schemas, local_enums, gaps
            )

    if structural:
        _check_tagged_union_variants(local_enums, schemas, gaps)
        _check_tagged_union_payload_types(local_enums, schemas, gaps)

    return OpenAIComparison(
        protocol=protocol,
        checked=checked,
        gaps=gaps,
        structural=structural,
    )
