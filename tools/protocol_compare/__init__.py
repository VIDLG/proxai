"""Shared protocol comparison primitives."""

from .field_contract import (
    CarrierKind,
    classify_rust_carrier,
    expected_carrier,
    validate_field_contract,
)

__all__ = [
    "CarrierKind",
    "classify_rust_carrier",
    "expected_carrier",
    "validate_field_contract",
]
