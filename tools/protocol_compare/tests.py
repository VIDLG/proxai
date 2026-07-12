import unittest

from .field_contract import (
    CarrierKind,
    classify_rust_carrier,
    expected_carrier,
    validate_field_contract,
)


class CarrierClassificationTests(unittest.TestCase):
    def test_classifies_all_four_named_carriers(self):
        self.assertEqual(classify_rust_carrier("String"), CarrierKind.REQUIRED)
        self.assertEqual(
            classify_rust_carrier("std::option::Option<String>"),
            CarrierKind.OPTIONAL,
        )
        self.assertEqual(
            classify_rust_carrier("crate::protocol::RequiredNullable<String>"),
            CarrierKind.REQUIRED_NULLABLE,
        )
        self.assertEqual(
            classify_rust_carrier("crate::protocol::OptionalNullable<String>"),
            CarrierKind.OPTIONAL_NULLABLE,
        )

    def test_does_not_misclassify_nested_or_raw_nullable_carriers(self):
        self.assertEqual(
            classify_rust_carrier("Vec<Option<String>>"),
            CarrierKind.REQUIRED,
        )
        self.assertEqual(
            classify_rust_carrier("Option<Nullable<String>>"),
            CarrierKind.OPTIONAL,
        )


class FieldContractTests(unittest.TestCase):
    OPTIONAL_ATTRIBUTES = [
        '#[serde(skip_serializing_if = "Option::is_none")]',
        "#[serde(default)]",
        '#[serde(deserialize_with = "deserialize_present")]',
    ]
    OPTIONAL_NULLABLE_ATTRIBUTES = [
        '#[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]',
    ]

    def test_expected_carrier_covers_presence_nullability_matrix(self):
        self.assertEqual(
            expected_carrier(required=True, nullable=False),
            CarrierKind.REQUIRED,
        )
        self.assertEqual(
            expected_carrier(required=False, nullable=False),
            CarrierKind.OPTIONAL,
        )
        self.assertEqual(
            expected_carrier(required=True, nullable=True),
            CarrierKind.REQUIRED_NULLABLE,
        )
        self.assertEqual(
            expected_carrier(required=False, nullable=True),
            CarrierKind.OPTIONAL_NULLABLE,
        )

    def test_accepts_complete_optional_nullable_contract(self):
        self.assertEqual(
            validate_field_contract(
                expected=CarrierKind.OPTIONAL_NULLABLE,
                actual=CarrierKind.OPTIONAL_NULLABLE,
                attributes=self.OPTIONAL_NULLABLE_ATTRIBUTES,
                source="official field",
                rust_type="OptionalNullable<String>",
            ),
            [],
        )

    def test_rejects_obsolete_optional_nullable_deserializer(self):
        messages = validate_field_contract(
            expected=CarrierKind.OPTIONAL_NULLABLE,
            actual=CarrierKind.OPTIONAL_NULLABLE,
            attributes=[
                '#[serde(default, deserialize_with = "deserialize_present", '
                'skip_serializing_if = "OptionalNullable::is_missing")]'
            ],
            source="official field",
            rust_type="OptionalNullable<String>",
        )
        self.assertEqual(len(messages), 1)
        self.assertIn("obsolete", messages[0])

    def test_reports_each_missing_optional_presence_attribute(self):
        messages = validate_field_contract(
            expected=CarrierKind.OPTIONAL,
            actual=CarrierKind.OPTIONAL,
            attributes=[],
            source="official field",
            rust_type="Option<String>",
        )
        self.assertEqual(len(messages), 3)
        self.assertTrue(any("skip_serializing_if" in message for message in messages))
        self.assertTrue(any("deserialize_present" in message for message in messages))
        self.assertTrue(any("serde(default)" in message for message in messages))


    def test_required_nullable_needs_no_field_deserializer(self):
        self.assertEqual(
            validate_field_contract(
                expected=CarrierKind.REQUIRED_NULLABLE,
                actual=CarrierKind.REQUIRED_NULLABLE,
                attributes=[],
                source="official field",
                rust_type="RequiredNullable<String>",
            ),
            [],
        )

    def test_rejects_obsolete_required_nullable_deserializer(self):
        messages = validate_field_contract(
            expected=CarrierKind.REQUIRED_NULLABLE,
            actual=CarrierKind.REQUIRED_NULLABLE,
            attributes=[
                '#[serde(deserialize_with = "crate::protocol::deserialize_required_nullable")]'
            ],
            source="official field",
            rust_type="RequiredNullable<String>",
        )
        self.assertEqual(len(messages), 1)
        self.assertIn("obsolete", messages[0])


if __name__ == "__main__":
    unittest.main()
