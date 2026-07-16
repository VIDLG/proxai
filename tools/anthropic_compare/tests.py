import unittest
from unittest.mock import Mock, patch

from . import checks, cli


class FieldCarrierTests(unittest.TestCase):
    SKIP_NONE = ['#[serde(skip_serializing_if = "Option::is_none")]']

    OPTIONAL_ATTRS = [
        '#[serde(skip_serializing_if = "Option::is_none")]',
        '#[serde(default)]',
        '#[serde(deserialize_with = "deserialize_present")]',
    ]
    OPTIONAL_NULLABLE_ATTRS = [
        '#[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]',
    ]


    def run_check(self, *, sdk_type, optional, rust_type, attrs=None):
        sdk_shapes = {
            "Message": {
                "kind": "interface",
                "fields": [
                    {
                        "name": "field",
                        "optional": optional,
                        "type": sdk_type,
                    }
                ],
            }
        }
        rust_items = {
            "Message": {
                "kind": "struct_item",
                "file": "message.rs",
                "line": 1,
                "fields": {
                    "field": {
                        "type": rust_type,
                        "attrs": attrs or [],
                        "line": 2,
                    }
                },
            }
        }
        bindings = [
            {
                "item": "Message",
                "sdk_name": "Message",
                "sdk_shape": sdk_shapes["Message"],
            }
        ]
        with (
            patch.object(checks, "sdk_comment_shapes", return_value=sdk_shapes),
            patch.object(checks, "rust_serde_items", return_value=rust_items),
            patch.object(
                checks,
                "rust_item_shape_bindings",
                return_value=bindings,
            ),
        ):
            return checks.field_carrier_diffs("sdk")

    def test_accepts_required_non_nullable_carrier(self):
        diffs, modeled = self.run_check(
            sdk_type="String",
            optional=False,
            rust_type="String",
        )
        self.assertEqual(diffs, [])
        self.assertEqual(modeled, [])

    def test_rejects_nullable_carrier_for_required_non_nullable_field(self):
        diffs, _ = self.run_check(
            sdk_type="String",
            optional=False,
            rust_type="Option<String>",
        )
        self.assertEqual(len(diffs), 1)
        self.assertIn("required; use a non-nullable carrier", diffs[0][2][0])

    def test_accepts_optional_carrier_that_preserves_presence(self):
        diffs, modeled = self.run_check(
            sdk_type="String",
            optional=True,
            rust_type="Option<String>",
            attrs=self.OPTIONAL_ATTRS,
        )
        self.assertEqual(diffs, [])
        self.assertEqual(modeled, [])

    def test_accepts_optional_nullable_field(self):
        diffs, modeled = self.run_check(
            sdk_type="String | null",
            optional=True,
            rust_type="OptionalNullable<String>",
            attrs=self.OPTIONAL_NULLABLE_ATTRS,
        )
        self.assertEqual(diffs, [])
        self.assertEqual(modeled, [])

    def test_rejects_raw_option_nullable_carrier(self):
        diffs, modeled = self.run_check(
            sdk_type="String | null",
            optional=True,
            rust_type="Option<Nullable<String>>",
            attrs=self.OPTIONAL_NULLABLE_ATTRS,
        )
        self.assertEqual(modeled, [])
        self.assertEqual(len(diffs), 1)
        self.assertIn("use `OptionalNullable<T>`", diffs[0][2][0])

    def test_rejects_optional_carrier_that_serializes_none(self):
        diffs, _ = self.run_check(
            sdk_type="String",
            optional=True,
            rust_type="Option<String>",
        )
        self.assertEqual(len(diffs), 3)
        messages = [message for _, _, items in diffs for message in items]
        self.assertTrue(any("skip_serializing_if" in message for message in messages))
        self.assertTrue(any("deserialize_present" in message for message in messages))
        self.assertTrue(any("serde(default)" in message for message in messages))

    def test_accepts_explicit_required_nullable_carrier(self):
        diffs, modeled = self.run_check(
            sdk_type="StopReason | null",
            optional=False,
            rust_type="RequiredNullable<StopReason>",
        )
        self.assertEqual(diffs, [])
        self.assertEqual(
            modeled,
            [("Message", "field", "message.rs", 2)],
        )

    def test_accepts_qualified_required_nullable_carrier(self):
        diffs, modeled = self.run_check(
            sdk_type="StopReason | null",
            optional=False,
            rust_type="crate::protocol::RequiredNullable<StopReason>",
        )
        self.assertEqual(diffs, [])
        self.assertEqual(len(modeled), 1)

    def test_rejects_obsolete_required_nullable_deserializer(self):
        diffs, modeled = self.run_check(
            sdk_type="StopReason | null",
            optional=False,
            rust_type="RequiredNullable<StopReason>",
            attrs=[
                '#[serde(deserialize_with = "crate::protocol::deserialize_required_nullable")]'
            ],
        )
        self.assertEqual(len(modeled), 1)
        self.assertEqual(len(diffs), 1)
        self.assertIn("obsolete", diffs[0][2][0])

    def test_rejects_plain_option_for_required_nullable_field(self):
        diffs, modeled = self.run_check(
            sdk_type="StopReason | null",
            optional=False,
            rust_type="Option<StopReason>",
        )
        self.assertEqual(modeled, [])
        self.assertEqual(len(diffs), 1)
        self.assertIn("use `RequiredNullable<T>`", diffs[0][2][0])

    def test_rejects_omitting_required_nullable_none(self):
        diffs, modeled = self.run_check(
            sdk_type="StopReason | null",
            optional=False,
            rust_type="RequiredNullable<StopReason>",
            attrs=self.SKIP_NONE,
        )
        self.assertEqual(len(modeled), 1)
        self.assertEqual(len(diffs), 1)
        self.assertIn("must not be omitted", diffs[0][2][0])


class NamedFieldReferenceTests(unittest.TestCase):
    def run_check(self, *, sdk_type, rust_type, rust_items, markers=None):
        sdk_shapes = {
            "Envelope": {
                "kind": "interface",
                "fields": [
                    {"name": "payload", "optional": False, "type": sdk_type}
                ],
            },
            **{name: {"kind": "type", "rhs": name} for name in rust_items},
        }
        rust_items = {
            "Envelope": {
                "kind": "struct_item",
                "file": "envelope.rs",
                "line": 1,
                "attrs": [],
                "fields": {"payload": {"type": rust_type, "line": 2, "attrs": []}},
                "variants": {},
            },
            **rust_items,
        }
        bindings = [
            {
                "item": "Envelope",
                "sdk_name": "Envelope",
                "sdk_shape": sdk_shapes["Envelope"],
            }
        ]
        with (
            patch.object(checks, "sdk_comment_shapes", return_value=sdk_shapes),
            patch.object(checks, "rust_serde_items", return_value=rust_items),
            patch.object(
                checks, "rust_item_shape_bindings", return_value=bindings
            ),
            patch.object(
                checks,
                "rust_sdk_markers",
                return_value=markers
                or {"aliases": {}, "field_suppressed": {}},
            ),
        ):
            return checks.named_field_type_diffs("sdk")

    def test_rejects_wrong_named_input_output_reference(self):
        diffs = self.run_check(
            sdk_type="InputItem | null",
            rust_type="OptionalNullable<OutputItem>",
            rust_items={
                "InputItem": {"kind": "struct_item", "attrs": [], "variants": {}},
                "OutputItem": {"kind": "struct_item", "attrs": [], "variants": {}},
            },
        )

        self.assertEqual(len(diffs), 1)
        self.assertIn("references ['InputItem']", diffs[0][2][0])
        self.assertIn("uses ['OutputItem']", diffs[0][2][0])

    def test_accepts_explicit_sdk_alias_for_named_reference(self):
        diffs = self.run_check(
            sdk_type="InputItem",
            rust_type="InputItemParam",
            rust_items={
                "InputItemParam": {
                    "kind": "struct_item",
                    "attrs": [],
                    "variants": {},
                }
            },
            markers={
                "aliases": {"InputItem": "InputItemParam"},
                "field_suppressed": {},
            },
        )

        self.assertEqual(diffs, [])

    def test_requires_complete_payload_coverage_for_named_sdk_unions(self):
        rust_items = {
            "InputA": {"kind": "struct_item", "attrs": [], "variants": {}},
            "InputB": {"kind": "struct_item", "attrs": [], "variants": {}},
            "Payload": {
                "kind": "enum_item",
                "attrs": ['#[serde(tag = "type")]'],
                "variants": {
                    "A": {"payloads": ["InputA"]},
                    "B": {"payloads": ["InputB"]},
                },
            },
        }
        diffs = self.run_check(
            sdk_type="InputA | InputB",
            rust_type="Payload",
            rust_items=rust_items,
        )
        self.assertEqual(diffs, [])

        rust_items["Payload"]["variants"]["B"]["payloads"] = ["InputA"]
        diffs = self.run_check(
            sdk_type="InputA | InputB",
            rust_type="Payload",
            rust_items=rust_items,
        )
        self.assertEqual(len(diffs), 1)
        self.assertIn("named union", diffs[0][2][0])

    def test_rejects_extra_payload_in_named_sdk_union(self):
        rust_items = {
            "InputA": {"kind": "struct_item", "attrs": [], "variants": {}},
            "InputB": {"kind": "struct_item", "attrs": [], "variants": {}},
            "Extra": {"kind": "struct_item", "attrs": [], "variants": {}},
            "Payload": {
                "kind": "enum_item",
                "attrs": [],
                "variants": {
                    "A": {"payloads": ["InputA"]},
                    "B": {"payloads": ["InputB"]},
                    "Extra": {"payloads": ["Extra"]},
                },
            },
        }
        diffs = self.run_check(
            sdk_type="InputA | InputB",
            rust_type="Payload",
            rust_items=rust_items,
        )

        self.assertEqual(len(diffs), 1)
        self.assertIn("('Extra', 0)", diffs[0][2][0])

    def test_rejects_array_depth_mismatch_for_named_reference(self):
        diffs = self.run_check(
            sdk_type="InputItem",
            rust_type="Vec<InputItem>",
            rust_items={
                "InputItem": {"kind": "struct_item", "attrs": [], "variants": {}}
            },
        )

        self.assertEqual(len(diffs), 1)
        self.assertIn("references ['InputItem']", diffs[0][2][0])

    def test_accepts_multiple_local_bindings_for_one_sdk_shape(self):
        sdk_shapes = {
            "Envelope": {
                "kind": "interface",
                "fields": [
                    {"name": "payload", "optional": False, "type": "TextBlockParam"}
                ],
            },
            "TextBlockParam": {"kind": "interface", "fields": []},
        }
        rust_items = {
            "Envelope": {
                "kind": "struct_item",
                "file": "envelope.rs",
                "line": 1,
                "attrs": [],
                "fields": {"payload": {"type": "TypedTextBlockParam", "line": 2, "attrs": []}},
                "variants": {},
            },
            "TypedTextBlockParam": {
                "kind": "struct_item",
                "file": "text.rs",
                "line": 1,
                "attrs": [],
                "fields": {},
                "variants": {},
            },
        }
        bindings = [
            {"item": "Envelope", "sdk_name": "Envelope", "sdk_shape": sdk_shapes["Envelope"]},
            {
                "item": "TypedTextBlockParam",
                "sdk_name": "TextBlockParam",
                "sdk_shape": sdk_shapes["TextBlockParam"],
            },
        ]
        with (
            patch.object(checks, "sdk_comment_shapes", return_value=sdk_shapes),
            patch.object(checks, "rust_serde_items", return_value=rust_items),
            patch.object(checks, "rust_item_shape_bindings", return_value=bindings),
            patch.object(
                checks,
                "rust_sdk_markers",
                return_value={"aliases": {}, "field_suppressed": {}},
            ),
        ):
            diffs = checks.named_field_type_diffs("sdk")

        self.assertEqual(diffs, [])

    def test_accepts_tagged_union_and_normalized_payload_names(self):
        sdk_shapes = {
            "Envelope": {
                "kind": "interface",
                "fields": [
                    {
                        "name": "payload",
                        "optional": False,
                        "type": "Base64ImageSource | URLImageSource",
                    }
                ],
            },
            "Base64ImageSource": {"kind": "interface", "fields": []},
            "URLImageSource": {"kind": "interface", "fields": []},
        }
        rust_items = {
            "Envelope": {
                "kind": "struct_item",
                "file": "envelope.rs",
                "line": 1,
                "attrs": [],
                "fields": {"payload": {"type": "ImageSource", "line": 2, "attrs": []}},
                "variants": {},
            },
            "Base64ImageSource": {"kind": "struct_item", "attrs": [], "variants": {}},
            "UrlImageSource": {"kind": "struct_item", "attrs": [], "variants": {}},
            "ImageSource": {
                "kind": "enum_item",
                "attrs": ['#[serde(tag = "type")]'],
                "variants": {
                    "Base64": {"payloads": ["Base64ImageSource"]},
                    "Url": {"payloads": ["UrlImageSource"]},
                },
            },
        }
        bindings = [
            {"item": "Envelope", "sdk_name": "Envelope", "sdk_shape": sdk_shapes["Envelope"]}
        ]
        with (
            patch.object(checks, "sdk_comment_shapes", return_value=sdk_shapes),
            patch.object(checks, "rust_serde_items", return_value=rust_items),
            patch.object(checks, "rust_item_shape_bindings", return_value=bindings),
            patch.object(
                checks,
                "rust_sdk_markers",
                return_value={"aliases": {}, "field_suppressed": {}},
            ),
        ):
            diffs = checks.named_field_type_diffs("sdk")

        self.assertEqual(diffs, [])

    def test_skips_primitive_sdk_aliases(self):
        sdk_shapes = {
            "Envelope": {
                "kind": "interface",
                "fields": [{"name": "payload", "optional": False, "type": "Model"}],
            },
            "Model": {"kind": "type", "rhs": "string"},
        }
        rust_items = {
            "Envelope": {
                "kind": "struct_item",
                "file": "envelope.rs",
                "line": 1,
                "attrs": [],
                "fields": {"payload": {"type": "String", "line": 2, "attrs": []}},
                "variants": {},
            }
        }
        bindings = [
            {"item": "Envelope", "sdk_name": "Envelope", "sdk_shape": sdk_shapes["Envelope"]}
        ]
        with (
            patch.object(checks, "sdk_comment_shapes", return_value=sdk_shapes),
            patch.object(checks, "rust_serde_items", return_value=rust_items),
            patch.object(checks, "rust_item_shape_bindings", return_value=bindings),
            patch.object(
                checks,
                "rust_sdk_markers",
                return_value={"aliases": {}, "field_suppressed": {}},
            ),
        ):
            diffs = checks.named_field_type_diffs("sdk")

        self.assertEqual(diffs, [])


class ExplicitProvenanceTests(unittest.TestCase):
    def run_check(self, *, docs, markers):
        sdk_shapes = {"Message": {"kind": "interface", "fields": []}}
        rust_items = {
            "Message": {
                "file": "message.rs",
                "line": 1,
                "kind": "struct_item",
                "attrs": [],
                "fields": {},
                "variants": {},
            }
        }
        with (
            patch.object(checks, "sdk_comment_shapes", return_value=sdk_shapes),
            patch.object(checks, "rust_serde_items", return_value=rust_items),
            patch.object(checks, "rust_doc_items", return_value=docs),
            patch.object(checks, "rust_sdk_markers", return_value=markers),
        ):
            return checks.explicit_provenance_diffs("sdk")

    def test_rejects_implicit_same_name_sdk_binding(self):
        diffs = self.run_check(
            docs=[],
            markers={"aliases": {}, "proxai_internals": {}},
        )

        self.assertEqual(len(diffs), 1)
        self.assertIn('@sdk(shape = "Message")', diffs[0][2][0])

    def test_accepts_explicit_shape_binding(self):
        diffs = self.run_check(
            docs=[
                {
                    "name": "Message",
                    "doc": [{"line": 1, "text": '@sdk(shape = "Message")'}],
                }
            ],
            markers={"aliases": {}, "proxai_internals": {}},
        )

        self.assertEqual(diffs, [])

    def test_accepts_explicit_alias_binding(self):
        diffs = self.run_check(
            docs=[],
            markers={"aliases": {"RawMessage": "Message"}, "proxai_internals": {}},
        )

        self.assertEqual(diffs, [])

    def test_accepts_explicit_internal_classification(self):
        diffs = self.run_check(
            docs=[],
            markers={"aliases": {}, "proxai_internals": {"Message": "union_wrapper"}},
        )

        self.assertEqual(diffs, [])


class CliTests(unittest.TestCase):
    @patch.object(cli, "render_report")
    @patch.object(
        cli,
        "compare_protocol",
        return_value=Mock(has_gaps=False),
    )
    def test_returns_normally_when_comparison_is_clean(
        self, compare_protocol, render_report
    ):
        cli.main(["--verbose", "--only-marked"])

        compare_protocol.assert_called_once_with(only_marked=True)
        render_report.assert_called_once_with(compare_protocol.return_value, 3)

    @patch.object(cli, "render_report")
    @patch.object(
        cli,
        "compare_protocol",
        return_value=Mock(has_gaps=True),
    )
    def test_exits_nonzero_when_comparison_has_gaps(
        self, compare_protocol, render_report
    ):
        with self.assertRaisesRegex(SystemExit, "1"):
            cli.main(["--quiet"])

        compare_protocol.assert_called_once_with(only_marked=False)
        render_report.assert_called_once_with(compare_protocol.return_value, 1)

    @patch.object(cli, "emit_sdk_docs")
    @patch.object(cli, "compare_protocol")
    def test_emit_docs_bypasses_comparison(self, compare_protocol, emit_sdk_docs):
        cli.main(["--emit-docs", "--only-marked"])

        emit_sdk_docs.assert_called_once_with(only_marked=True)
        compare_protocol.assert_not_called()


if __name__ == "__main__":
    unittest.main()
