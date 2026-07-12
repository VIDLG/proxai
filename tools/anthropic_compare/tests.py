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
