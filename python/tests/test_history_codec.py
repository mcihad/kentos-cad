"""The history audit permits declared defaults without hiding changed Python inputs."""
from __future__ import annotations
import importlib.util
from pathlib import Path
import unittest

_spec = importlib.util.spec_from_file_location(
    "kentos_history_audit", Path(__file__).resolve().parents[2] / "scripts/python/history_calls.py"
)
assert _spec is not None and _spec.loader is not None
_audit = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_audit)


class HistoryCodec(unittest.TestCase):
    def test_only_declared_nonempty_defaults_may_be_omitted(self) -> None:
        schema = {"properties": {"mode": {"default": "new"}, "layers": {"default": True}}}
        self.assertEqual(_audit.but_defaults({"mode": "new", "layers": True}, {}, schema), {})
        self.assertEqual(_audit.but_defaults({"mode": "append", "unexpected": True}, {}, schema),
                         {"mode": "append", "unexpected": True})
        self.assertEqual(_audit.but_defaults({"mode": "new"}, {"mode": "new"}, schema), {"mode": "new"})
        self.assertEqual(_audit.but_defaults({"mode": "append"}, {"mode": "new"}, schema), {"mode": "append"})
        self.assertEqual(_audit.but_defaults({"layers": 1}, {}, schema), {"layers": 1})

    def test_nullable_and_free_json_schema_are_supported(self) -> None:
        schema = {"$defs": {"Grid": {"type": "object", "properties": {"origin": {"default": "center"}}}},
                  "properties": {"grid": {"anyOf": [{"$ref": "#/$defs/Grid"}, {"type": "null"}]},
                                 "metadata": {"type": "object", "properties": {"anything": True}}}}
        self.assertEqual(_audit.but_defaults({"grid": {"origin": "center"}}, {"grid": {}}, schema), {"grid": {}})
        value = {"metadata": {"anything": "kept"}}
        self.assertEqual(_audit.but_defaults(value, value, schema), value)

    def test_nested_tagged_defaults_follow_the_selected_variant(self) -> None:
        schema = {"$defs": {"Frame": {"enum": ["hide", "show"]}}, "properties": {"operation": {"oneOf": [
            {"properties": {"kind": {"const": "clip"}, "frame": {"$ref": "#/$defs/Frame", "default": "hide"}}},
            {"properties": {"kind": {"const": "other"}, "frame": {"default": "show"}}},
        ]}}}
        want = {"operation": {"kind": "clip"}}
        self.assertEqual(_audit.but_defaults({"operation": {"kind": "clip", "frame": "hide"}}, want, schema), want)
        given = {"operation": {"kind": "clip", "frame": "show"}}
        self.assertEqual(_audit.but_defaults(given, want, schema), given)


if __name__ == "__main__":
    unittest.main()
