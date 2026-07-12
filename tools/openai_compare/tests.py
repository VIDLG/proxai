import unittest
from unittest.mock import Mock, patch

from . import checks, cli, common, rust
from . import schema as openapi_schema


class RenameAllTests(unittest.TestCase):
    def test_keeps_numeric_suffixes_attached(self):
        self.assertEqual(rust._apply_rename_all("Mp3", "snake_case"), "mp3")
        self.assertEqual(rust._apply_rename_all("Pcm16", "snake_case"), "pcm16")
        self.assertEqual(rust._apply_rename_all("Base64", "snake_case"), "base64")

    def test_applies_supported_rename_styles(self):
        self.assertEqual(
            rust._apply_rename_all("MaxOutputTokens", "snake_case"),
            "max_output_tokens",
        )
        self.assertEqual(
            rust._apply_rename_all("MaxOutputTokens", "kebab-case"),
            "max-output-tokens",
        )
        self.assertEqual(
            rust._apply_rename_all("MaxOutputTokens", "SCREAMING_SNAKE_CASE"),
            "MAX_OUTPUT_TOKENS",
        )


class EnumIndexTests(unittest.TestCase):
    def index(self, source):
        encoded = source.encode()
        enums = {}
        rust._index_enums(common.RS.parse(encoded).root_node, encoded, enums)
        return enums

    def test_indexes_rename_all_and_explicit_variant_rename(self):
        enums = self.index(
            '''
            #[serde(rename_all = "snake_case")]
            enum Retention {
                InMemory,
                #[serde(rename = "24h")]
                Hours24,
                Pcm16,
            }
            '''
        )

        self.assertEqual(
            enums["Retention"]["wire_values"],
            {"in_memory", "24h", "pcm16"},
        )
        self.assertEqual(enums["Retention"]["variant_payloads"], {})
        self.assertFalse(enums["Retention"]["open_string"])

    def test_detects_untagged_string_fallback(self):
        enums = self.index(
            '''
            enum ClosedEnum {
                Known,
                #[serde(untagged)]
                Other(String),
            }
            '''
        )

        self.assertTrue(enums["ClosedEnum"]["open_string"])
        self.assertEqual(enums["ClosedEnum"]["wire_values"], {"Known"})


class TaggedEnumDiscriminatorTests(unittest.TestCase):
    def tagged_payload_fields(self, source):
        encoded = source.encode()
        root = common.RS.parse(encoded).root_node
        structs = {}
        provenance = {}
        enums = {}
        rust._index_structs(root, encoded, structs, provenance)
        rust._index_enums(root, encoded, enums)
        rust._index_tagged_enum_payloads(root, encoded, structs, enums)
        return structs

    def test_records_variant_wire_value_on_payload_discriminator(self):
        structs = self.tagged_payload_fields(
            '''
            pub struct FunctionCall { pub name: String }
            #[serde(tag = "type", rename_all = "snake_case")]
            pub enum Item { FunctionCall(FunctionCall) }
            '''
        )
        self.assertEqual(
            structs["FunctionCall"]["type"],
            ("__serde_tag__", frozenset({"function_call"})),
        )

    def test_reports_payload_discriminator_mismatch(self):
        gaps = []
        checks._check_serde_tag_fields(
            "FunctionCall",
            {"type": ("__serde_tag__", frozenset({"function_call"}))},
            {"type": {"type": "string", "enum": ["custom_tool_call"]}},
            {},
            gaps,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("discriminator differs", gaps[0][2])


class SchemaNullabilityTests(unittest.TestCase):
    def test_honors_nullable_sibling_of_ref(self):
        schemas = {"Usage": {"type": "object"}}
        schema = {
            "$ref": "#/components/schemas/Usage",
            "nullable": True,
        }

        self.assertTrue(openapi_schema._allows_null(schema, schemas, set()))


class ClosedSchemaEnumTests(unittest.TestCase):
    def test_resolves_nullable_referenced_enum(self):
        schemas = {
            "Status": {"type": "string", "enum": ["completed", "failed"]}
        }
        schema = {
            "anyOf": [
                {"$ref": "#/components/schemas/Status"},
                {"type": "null"},
            ]
        }

        self.assertEqual(
            openapi_schema._closed_string_enum_values(schema, schemas, set()),
            {"completed", "failed"},
        )

    def test_rejects_open_string_fallback_for_closed_schema(self):
        gaps = []
        checks._check_closed_enum_fields(
            "Carrier",
            {"status": "Status"},
            {"status": {"type": "string", "enum": ["completed"]}},
            {},
            {
                "Status": {
                    "untagged": False,
                    "payloads": ["String"],
                    "wire_values": {"completed"},
                    "open_string": True,
                }
            },
            gaps,
        )

        self.assertEqual(len(gaps), 1)
        self.assertIn("untagged string fallback", gaps[0][2])


class TaggedUnionVariantTests(unittest.TestCase):
    def test_collects_discriminators_from_referenced_union_branches(self):
        schemas = {
            "Text": {
                "type": "object",
                "properties": {"type": {"type": "string", "enum": ["text"]}},
            },
            "Image": {
                "type": "object",
                "properties": {"type": {"type": "string", "enum": ["image"]}},
            },
        }
        union = {
            "oneOf": [
                {"$ref": "#/components/schemas/Text"},
                {"$ref": "#/components/schemas/Image"},
            ]
        }
        self.assertEqual(
            openapi_schema._union_discriminator_values(union, "type", schemas, set()),
            {"text", "image"},
        )

    def test_reports_missing_or_extra_tagged_union_variants(self):
        gaps = []
        checks._check_tagged_union_variants(
            {
                "Item": {
                    "tag": "type",
                    "wire_values": {"text", "other"},
                }
            },
            {
                "Item": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "enum": ["text"]}
                            },
                        },
                        {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "enum": ["image"]}
                            },
                        },
                    ]
                }
            },
            gaps,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("variants differ", gaps[0][2])


    def test_reports_wrong_payload_type_for_discriminator(self):
        gaps = []
        checks._check_tagged_union_payload_types(
            {
                "Item": {
                    "tag": "type",
                    "variant_payloads": {"text": "WrongTextPayload"},
                }
            },
            {
                "Item": {
                    "oneOf": [
                        {"$ref": "#/components/schemas/TextPayload"},
                    ]
                },
                "TextPayload": {
                    "type": "object",
                    "properties": {
                        "type": {"type": "string", "enum": ["text"]},
                        "text": {"type": "string"},
                    },
                },
            },
            gaps,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("expected official `TextPayload`", gaps[0][2])
        self.assertIn("local `WrongTextPayload`", gaps[0][2])

    def test_collects_payload_types_from_referenced_union_branches(self):
        schemas = {
            "TextPayload": {
                "type": "object",
                "properties": {
                    "type": {"type": "string", "enum": ["text"]}
                },
            },
            "ImagePayload": {
                "type": "object",
                "properties": {
                    "type": {"type": "string", "enum": ["image"]}
                },
            },
        }
        union = {
            "oneOf": [
                {"$ref": "#/components/schemas/TextPayload"},
                {"$ref": "#/components/schemas/ImagePayload"},
            ]
        }
        self.assertEqual(
            openapi_schema._union_discriminator_payloads(union, "type", schemas, set()),
            {"text": "TextPayload", "image": "ImagePayload"},
        )


class FieldShapeTests(unittest.TestCase):
    def test_classifies_builtin_rust_carriers(self):
        self.assertEqual(rust._rust_builtin_shape("Option<String>"), "string")
        self.assertEqual(rust._rust_builtin_shape("Vec<OutputItem>"), "array")
        self.assertEqual(rust._rust_builtin_shape("Option<u32>"), "integer")
        self.assertEqual(
            rust._rust_builtin_shape("RequiredNullable<Vec<String>>"), "array"
        )
        self.assertEqual(
            rust._rust_builtin_shape("std::collections::HashMap<String, Value>"),
            "object",
        )
        self.assertIsNone(rust._rust_builtin_shape("InputParam"))

    def test_collects_nullable_union_shapes(self):
        schema = {
            "anyOf": [
                {"type": "string"},
                {"type": "array", "items": {"type": "string"}},
                {"type": "null"},
            ]
        }
        self.assertEqual(
            openapi_schema._schema_shape_categories(schema, {}, set()),
            {"string", "array"},
        )

    def test_reports_provable_shape_mismatch(self):
        gaps = []
        checks._check_field_shapes(
            "Carrier",
            {"count": "String"},
            {"count": {"type": "integer"}},
            {},
            gaps,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("`string`", gaps[0][2])
        self.assertIn("['integer']", gaps[0][2])


class ArrayItemTypeTests(unittest.TestCase):
    def test_extracts_rust_array_item_identity(self):
        self.assertEqual(
            rust._rust_array_item_identity("Option<Vec<OutputItem>>"),
            "OutputItem",
        )
        self.assertEqual(rust._rust_array_item_identity("Vec<String>"), "string")
        self.assertIsNone(rust._rust_array_item_identity("Vec<serde_json::Value>"))

    def test_extracts_referenced_official_array_item_identity(self):
        schema = {
            "type": "array",
            "items": {"$ref": "#/components/schemas/OutputItem"},
        }
        self.assertEqual(
            openapi_schema._schema_array_item_identity(schema, {}, set()),
            "OutputItem",
        )

    def test_reports_provable_array_item_mismatch(self):
        gaps = []
        checks._check_array_item_types(
            "Carrier",
            {"items": "Vec<OutputItem>"},
            {
                "items": {
                    "type": "array",
                    "items": {"$ref": "#/components/schemas/InputItem"},
                }
            },
            {},
            {},
            gaps,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("`OutputItem`", gaps[0][2])
        self.assertIn("`InputItem`", gaps[0][2])

    def test_understands_single_payload_and_unit_array_enums(self):
        enums = {
            "SummaryPart": {
                "payloads": ["SummaryTextContent"],
                "wire_values": {"summary_text"},
                "open_string": False,
            },
            "ResponseModality": {
                "payloads": [],
                "wire_values": {"text", "audio"},
                "open_string": False,
            },
        }
        self.assertEqual(
            rust._rust_array_item_identity("Vec<SummaryPart>", enums),
            "SummaryTextContent",
        )
        self.assertEqual(
            rust._rust_array_item_identity("Vec<ResponseModality>", enums),
            "string",
        )

    def test_reports_closed_array_enum_value_mismatch(self):
        gaps = []
        checks._check_array_item_types(
            "Carrier",
            {"modalities": "Vec<ResponseModality>"},
            {
                "modalities": {
                    "type": "array",
                    "items": {"type": "string", "enum": ["text", "audio"]},
                }
            },
            {},
            {
                "ResponseModality": {
                    "payloads": [],
                    "wire_values": {"text"},
                    "open_string": False,
                }
            },
            gaps,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("array item enum wire values differ", gaps[0][2])

    def test_skips_ambiguous_official_array_item_union(self):
        schema = {
            "type": "array",
            "items": {
                "oneOf": [
                    {"$ref": "#/components/schemas/TextItem"},
                    {"$ref": "#/components/schemas/ImageItem"},
                ]
            },
        }
        self.assertIsNone(openapi_schema._schema_array_item_identity(schema, {}, set()))


class FieldCarrierTests(unittest.TestCase):
    def check(
        self,
        *,
        rust_type,
        required,
        nullable=False,
        skips_none=False,
        skips_optional_nullable_missing=False,
        has_obsolete_required_nullable_deserializer=False,
    ):
        gaps = []
        schema = {"type": ["string", "null"] if nullable else "string"}
        attributes = []
        if skips_none:
            attributes.extend(
                [
                    '#[serde(skip_serializing_if = "Option::is_none")]',
                    '#[serde(default)]',
                    '#[serde(deserialize_with = "crate::protocol::deserialize_present")]',
                ]
            )
        if skips_optional_nullable_missing:
            attributes.append(
                '#[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]'
            )
        if has_obsolete_required_nullable_deserializer:
            attributes.append(
                '#[serde(deserialize_with = "crate::protocol::deserialize_required_nullable")]'
            )
        checks._check_field_carriers(
            "Fixture",
            {"field": rust_type},
            {"field": attributes},
            {"field": schema},
            {"field"} if required else set(),
            {},
            gaps,
        )
        return gaps

    def test_accepts_required_non_nullable_carrier(self):
        self.assertEqual(
            self.check(rust_type="String", required=True),
            [],
        )

    def test_rejects_required_nullable_carrier_for_non_nullable_field(self):
        gaps = self.check(
            rust_type="RequiredNullable<String>",
            required=True,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("use a non-nullable carrier", gaps[0][2])

    def test_accepts_required_nullable_carrier(self):
        self.assertEqual(
            self.check(
                rust_type="crate::protocol::RequiredNullable<String>",
                required=True,
                nullable=True,
            ),
            [],
        )

    def test_rejects_obsolete_required_nullable_deserializer(self):
        gaps = self.check(
            rust_type="RequiredNullable<String>",
            required=True,
            nullable=True,
            has_obsolete_required_nullable_deserializer=True,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("obsolete", gaps[0][2])

    def test_rejects_option_for_required_nullable_field(self):
        gaps = self.check(
            rust_type="Option<String>",
            required=True,
            nullable=True,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("use `RequiredNullable<T>`", gaps[0][2])

    def test_accepts_option_for_optional_field(self):
        self.assertEqual(
            self.check(
                rust_type="Option<String>", required=False, skips_none=True
            ),
            [],
        )

    def test_rejects_plain_carrier_for_optional_field(self):
        gaps = self.check(
            rust_type="String", required=False, skips_none=True
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("use `Option<T>`", gaps[0][2])

    def test_rejects_optional_field_without_presence_attributes(self):
        gaps = self.check(rust_type="Option<String>", required=False)
        self.assertEqual(len(gaps), 3)
        self.assertTrue(any("skip_serializing_if" in gap[2] for gap in gaps))
        self.assertTrue(any("deserialize_with" in gap[2] for gap in gaps))
        self.assertTrue(any("serde(default)" in gap[2] for gap in gaps))

    def test_accepts_optional_nullable_field(self):
        self.assertEqual(
            self.check(
                rust_type="OptionalNullable<String>",
                required=False,
                nullable=True,
                skips_optional_nullable_missing=True,
            ),
            [],
        )

    def test_rejects_raw_option_nullable_carrier(self):
        gaps = self.check(
            rust_type="Option<Nullable<String>>",
            required=False,
            nullable=True,
            skips_optional_nullable_missing=True,
        )
        self.assertEqual(len(gaps), 1)
        self.assertIn("use `OptionalNullable<T>`", gaps[0][2])



class CliTests(unittest.TestCase):
    @patch.object(cli, "render_report")
    @patch.object(
        cli,
        "compare_protocol",
        return_value=Mock(has_gaps=False),
    )
    @patch.object(cli, "_load_schemas", return_value=({}, {}))
    def test_returns_normally_when_comparison_is_clean(
        self, _load_schemas, compare_protocol, render_report
    ):
        cli.main(["--structural", "chat"])

        compare_protocol.assert_called_once_with("chat", {}, {}, structural=True)
        render_report.assert_called_once_with(compare_protocol.return_value)

    @patch.object(cli, "render_report")
    @patch.object(
        cli,
        "compare_protocol",
        return_value=Mock(has_gaps=True),
    )
    @patch.object(cli, "_load_schemas", return_value=({}, {}))
    def test_exits_nonzero_when_comparison_has_gaps(
        self, _load_schemas, compare_protocol, render_report
    ):
        with self.assertRaisesRegex(SystemExit, "1"):
            cli.main(["--quiet", "chat"])

        compare_protocol.assert_called_once_with("chat", {}, {}, structural=False)
        render_report.assert_called_once_with(compare_protocol.return_value)


if __name__ == "__main__":
    unittest.main()
