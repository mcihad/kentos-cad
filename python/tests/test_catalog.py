"""The generated layer against the catalog itself (TODOS.md PY-11, PY-16):
every command has its wrapper under its own name, every type is here, and
every example of the catalog reads into its type and writes back the same."""

from __future__ import annotations

import json
import unittest
from typing import Any

import kentos.cad as cad
from kentos import _native
from kentos.cad import types
from kentos.cad._runtime import LocalCommand, ServerCommand

CATALOG = json.loads(_native.catalog())
COMMANDS = CATALOG["commands"]


def contains(expected: Any, actual: Any, where: str = "") -> None:
    """Every value of `expected` is in `actual` (a writer may add a field's default)."""
    if isinstance(expected, dict):
        assert isinstance(actual, dict), f"{where}: {actual!r} is not an object"
        for k, v in expected.items():
            assert k in actual, f"{where}.{k} missing"
            contains(v, actual[k], f"{where}.{k}")
    elif isinstance(expected, list):
        assert isinstance(actual, list) and len(actual) == len(expected), f"{where}: {actual!r}"
        for i, (a, b) in enumerate(zip(expected, actual)):
            contains(a, b, f"{where}[{i}]")
    else:
        assert expected == actual, f"{where}: {expected!r} != {actual!r}"


class Coverage(unittest.TestCase):
    def test_every_command_has_its_wrapper_under_its_name(self) -> None:
        self.assertEqual(sorted(cad.COMMANDS), sorted(c["id"] for c in COMMANDS))
        for c in COMMANDS:
            w = cad.command(c["id"])
            parts = c["id"].split(".")
            parts = parts[1:] if parts[0] == "cad" else parts
            obj: Any = cad
            for p in parts[:-1]:
                obj = getattr(obj, p)
            name = parts[-1] + "_" if parts[-1] == "import" else parts[-1]
            self.assertIs(getattr(obj, name), w, c["id"])
            self.assertEqual((w.id, w.version, w.title), (c["id"], c["version"], c["title"]))
            self.assertEqual(w.input_type.__name__, c["input"]["title"])
            self.assertEqual(w.output_type.__name__, c["output"]["title"])
            local = "desktop" in c["hosts"]
            self.assertIsInstance(w, LocalCommand if local else ServerCommand, c["id"])
            if local:
                self.assertEqual(w.plan_type.__name__, c["plan"]["title"])

    def test_the_native_host_runs_exactly_the_local_wrappers(self) -> None:
        local = sorted((w.id, w.version) for w in cad.COMMANDS.values() if isinstance(w, LocalCommand))
        self.assertEqual(local, sorted(_native.local_commands()))

    def test_every_type_of_the_catalog_is_here(self) -> None:
        names = set()
        for c in COMMANDS:
            for side in ("input", "output", "plan"):
                if side in c:
                    names.add(c[side]["title"])
                    # A named text that is no choice (a block's id) is a plain `str`, as the generator writes it.
                    defs = c[side].get("$defs", {}).items()
                    names.update(n for n, d in defs if not (d.get("type") == "string" and "enum" not in d))
        missing = sorted(n for n in names if not hasattr(types, n))
        self.assertEqual(missing, [])

    def test_the_package_exports_no_name_twice(self) -> None:
        self.assertEqual(len(cad.__all__), len(set(cad.__all__)))
        for n in cad.__all__:
            self.assertTrue(hasattr(cad, n), n)


class Examples(unittest.TestCase):
    def test_every_example_reads_into_its_type_and_writes_back(self) -> None:
        count = 0
        for c in COMMANDS:
            w = cad.command(c["id"])
            for ex in c["examples"]:
                with self.subTest(command=c["id"], example=ex["title"]):
                    value = w.input_type.from_json(ex["input"])
                    written = value.to_json()
                    contains(ex["input"], written, c["id"])
                    self.assertEqual(w.input_type.from_json(written), value)
                    count += 1
                    if ex.get("output") is not None:
                        out = w.output_type.from_json(ex["output"])
                        contains(ex["output"], out.to_json(), c["id"] + " output")
                        count += 1
        self.assertGreater(count, 30)

    def test_a_union_reads_each_variant_as_its_class(self) -> None:
        move = types.Transform.from_json({"kind": "move", "dx": 1, "dy": 2})
        self.assertIsInstance(move, types.MoveTransform)
        self.assertEqual(move.kind, "move")
        self.assertEqual(move.to_json(), {"kind": "move", "dx": 1.0, "dy": 2.0})
        with self.assertRaises(cad.DecodeError):
            types.Transform.from_json({"kind": "bük", "dx": 1})

    def test_an_enum_takes_its_member_or_its_name(self) -> None:
        a = types.EntitiesSetProperties(uids=["x"], operation="layer", layer_id="yol").to_json()
        b = types.EntitiesSetProperties(uids=["x"], operation=types.PropertiesOperation.LAYER, layer_id="yol").to_json()
        self.assertEqual(a, b)
        self.assertEqual(a, {"uids": ["x"], "operation": "layer", "layerId": "yol"})

    def test_left_out_is_not_sent_and_none_is_null(self) -> None:
        kept = types.EntitiesSetProperties(uids=["x"], operation="color").to_json()
        self.assertNotIn("color", kept)
        cleared = types.EntitiesSetProperties(uids=["x"], operation="color", color=None).to_json()
        self.assertIsNone(cleared["color"])
        self.assertIs(types.PolygonCreate.from_json({"layerId": "a", "pts": []}).color, cad.UNSET)
        self.assertIsNone(types.PolygonCreate.from_json({"layerId": "a", "pts": [], "color": None}).color)

    def test_a_point_is_a_pair_too(self) -> None:
        self.assertEqual(types.Vec2.of((1, 2)), types.Vec2(1.0, 2.0))
        x, y = types.Vec2(3, 4)
        self.assertEqual((x, y), (3, 4))
        typed = types.LineCreate(layer_id="a", a=types.Vec2(0, 0), b=types.Vec2(1, 1)).to_json()
        self.assertEqual(typed["b"], {"x": 1.0, "y": 1.0})
        # A pair is taken at run time too, though the field's type is Vec2.
        loose = types.LineCreate(layer_id="a", a=(0, 0), b=(1, 1))  # type: ignore[arg-type]
        self.assertEqual(loose.to_json(), typed)


if __name__ == "__main__":
    unittest.main()
