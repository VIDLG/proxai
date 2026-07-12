"""Presence/nullability contracts shared by protocol comparison tools."""

from enum import Enum
import re


class CarrierKind(str, Enum):
    REQUIRED = "required"
    OPTIONAL = "optional"
    REQUIRED_NULLABLE = "required-nullable"
    OPTIONAL_NULLABLE = "optional-nullable"

    @property
    def rust_description(self) -> str:
        return {
            CarrierKind.REQUIRED: "a non-nullable carrier",
            CarrierKind.OPTIONAL: "`Option<T>`",
            CarrierKind.REQUIRED_NULLABLE: "`RequiredNullable<T>`",
            CarrierKind.OPTIONAL_NULLABLE: "`OptionalNullable<T>`",
        }[self]

    @property
    def is_optional(self) -> bool:
        return self in {CarrierKind.OPTIONAL, CarrierKind.OPTIONAL_NULLABLE}


def expected_carrier(*, required: bool, nullable: bool) -> CarrierKind:
    if required:
        return (
            CarrierKind.REQUIRED_NULLABLE
            if nullable
            else CarrierKind.REQUIRED
        )
    return CarrierKind.OPTIONAL_NULLABLE if nullable else CarrierKind.OPTIONAL


def classify_rust_carrier(type_text: str) -> CarrierKind:
    if _is_named_outer_generic(type_text, "OptionalNullable"):
        return CarrierKind.OPTIONAL_NULLABLE
    if _is_named_outer_generic(type_text, "Option"):
        return CarrierKind.OPTIONAL
    if _is_named_outer_generic(type_text, "RequiredNullable"):
        return CarrierKind.REQUIRED_NULLABLE
    return CarrierKind.REQUIRED


def validate_field_contract(
    *,
    expected: CarrierKind,
    actual: CarrierKind,
    attributes: list[str],
    source: str,
    rust_type: str,
) -> list[str]:
    messages = []
    if actual != expected:
        messages.append(
            f"{source} is {expected.value}; use {expected.rust_description} "
            f"instead of `{rust_type}`"
        )

    skips_option_none = _skip_option_none(attributes)
    skips_optional_nullable_missing = _skip_optional_nullable_missing(attributes)
    if expected == CarrierKind.OPTIONAL and not skips_option_none:
        messages.append(
            f"{source} is optional; add "
            '`#[serde(skip_serializing_if = "Option::is_none")]`'
        )
    elif expected == CarrierKind.OPTIONAL_NULLABLE and not skips_optional_nullable_missing:
        messages.append(
            f"{source} is optional-nullable; add "
            '`#[serde(skip_serializing_if = "OptionalNullable::is_missing")]`'
        )
    elif expected not in {CarrierKind.OPTIONAL, CarrierKind.OPTIONAL_NULLABLE} and (
        skips_option_none or skips_optional_nullable_missing
    ):
        messages.append(
            f"{source} is {expected.value}; it must not be omitted when serializing"
        )

    if expected == CarrierKind.OPTIONAL and not _deserializes_present(attributes):
        messages.append(
            f"{source} is optional; add "
            '`deserialize_with = "deserialize_present"` '
            "to reject present JSON null values"
        )
    if expected == CarrierKind.OPTIONAL_NULLABLE and _deserializes_present(attributes):
        messages.append(
            f"{source} uses obsolete `deserialize_present`; remove the field-level "
            "deserializer because `OptionalNullable<T>` now enforces "
            "missing/null/value semantics itself"
        )
    if _deserializes_required_nullable(attributes):
        messages.append(
            f"{source} uses obsolete `deserialize_required_nullable`; "
            "remove the field-level deserializer because `RequiredNullable<T>` "
            "now enforces missing/null semantics itself"
        )
    if expected.is_optional and not _defaults_missing(attributes):
        messages.append(f"{source} is optional; add `#[serde(default)]`")

    return messages


def _is_named_outer_generic(type_text: str, name: str) -> bool:
    compact = re.sub(r"\s+", "", type_text or "")
    return re.match(
        rf"^(?:(?:r#)?[A-Za-z_][A-Za-z0-9_]*::)*{name}<",
        compact,
    ) is not None


def _deserializes_present(attributes: list[str]) -> bool:
    return any("deserialize_present" in attribute for attribute in attributes)


def _deserializes_required_nullable(attributes: list[str]) -> bool:
    return any(
        "deserialize_required_nullable" in attribute for attribute in attributes
    )


def _defaults_missing(attributes: list[str]) -> bool:
    return any(
        re.search(r"\bserde\s*\([^]]*\bdefault\b", attribute)
        for attribute in attributes
    )


def _skip_option_none(attributes: list[str]) -> bool:
    return any(
        re.search(
            r'skip_serializing_if\s*=\s*"(?:[A-Za-z_][A-Za-z0-9_]*::)*Option::is_none"',
            attribute,
        )
        for attribute in attributes
    )


def _skip_optional_nullable_missing(attributes: list[str]) -> bool:
    return any(
        re.search(
            r'skip_serializing_if\s*=\s*"(?:[A-Za-z_][A-Za-z0-9_]*::)*OptionalNullable::is_missing"',
            attribute,
        )
        for attribute in attributes
    )
