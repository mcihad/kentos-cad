"""Physical parameter dimensions share the desktop/MCP command path; old files keep their numeric meaning."""
import json
import math
from pathlib import Path
import sys
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def add(name, kind, expression):
    return cad.ParameterEdit.from_json(dict(kind="add", parameter=dict(name=name, kind=kind, expression=expression)))


def set_value(name, expression):
    return cad.SetParameterEdit(name=name, expression=expression)


class ParameterDimensions(unittest.TestCase):
    def test_length_angle_ratio_and_power_preflight_is_atomic_in_every_project_unit(self):
        for unit in ["mm", "cm", "m", "in", "ft"]:
            with self.subTest(unit=unit):
                d = cad.Document.new(name="Units", workspace="cad", drawing_unit=unit)
                before = d.to_bytes()
                edits = [add("width", "length", "4m"), add("height", "length", "3m"),
                         add("diagonal", "length", "sqrt(width^2 + height^2)"),
                         add("ratio", "number", "width / height"), add("turn", "angle", "pi / 2"),
                         add("projection", "length", "width * sin(turn)")]
                planned = cad.parameters.edit.plan(d, edits=edits)
                self.assertEqual(d.to_bytes(), before)
                actual = cad.parameters.edit(d, edits=edits)
                self.assertEqual([v.to_json() for v in planned.values], [v.to_json() for v in actual.values])
                values = {v.name: v.value for v in actual.values}
                metres = {"mm": .001, "cm": .01, "m": 1, "in": .0254, "ft": .3048}[unit]
                self.assertAlmostEqual(values["diagonal"] * metres, 5.0, places=12)
                self.assertAlmostEqual(values["projection"] * metres, 4.0, places=12)
                self.assertAlmostEqual(values["ratio"], 4 / 3, places=12)
                good = d.to_bytes()
                for expr in ["30deg", "width + turn", "width / height", "sqrt(width)"]:
                    inputs = cad.ParametersEdit(edits=[set_value("diagonal", expr)])
                    results = [cad.parameters.edit.run(d, inputs, op=op) for op in ["validate", "plan", "execute"]]
                    self.assertTrue(all(r.error.code == "parameter_kind" for r in results))
                    errors = [(r.error.code, r.error.message, r.error.path) for r in results]
                    self.assertEqual(errors, [errors[0]] * 3)
                    self.assertEqual(d.to_bytes(), good)
                d.undo()
                self.assertEqual(d.to_bytes(), before)

    def test_dependent_dimensions_are_rechecked_when_a_named_exponent_changes(self):
        d = cad.Document.new(name="Power", workspace="cad", drawing_unit="m")
        cad.parameters.edit(d, edits=[add("width", "length", "4m"), add("power", "number", "2"),
                                      add("root", "length", "sqrt(width^power)")])
        before = d.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.parameters.edit(d, edits=[set_value("power", "1")])
        self.assertEqual(caught.exception.code, "parameter_kind")
        self.assertIn("root", str(caught.exception))
        self.assertEqual(d.to_bytes(), before)

    def test_independently_encoded_legacy_pi_is_audited_and_only_changes_on_explicit_repair(self):
        sys.path.insert(0, str(ROOT / "scripts/fixtures"))
        import kcad_v2_reference as reference
        raw = json.loads((ROOT / "fixtures/kcad/v2/constraints.json").read_text())
        for angle, half in [("deg", 180), ("grad", 200)]:
            with self.subTest(angle=angle):
                raw["settings"]["angleUnit"] = angle
                raw["constraints"] = dict(parameters=[dict(name="legacy", kind="number", expression="pi")])
                d = cad.Document.from_bytes(reference.container(reference.document(raw)))
                before = d.to_bytes()
                outcome = cad.constraints.query.run(d, cad.ConstraintsQuery())
                self.assertTrue(outcome.ok)
                self.assertEqual([w.code for w in outcome.warnings], ["parameter_units_legacy"])
                self.assertEqual(outcome.warnings[0].path, "parameters[0].expression")
                self.assertEqual(d.to_bytes(), before)
                out = cad.parameters.edit(d, edits=[cad.RenameParameterEdit(name="legacy", to="preserved")])
                self.assertAlmostEqual(out.values[0].value, half, places=12)
                before_repair = d.to_bytes()
                out = cad.parameters.edit(d, edits=[set_value("preserved", "pi / 1rad")])
                self.assertAlmostEqual(out.values[0].value, math.pi, places=12)
                self.assertEqual(cad.constraints.query.run(d, cad.ConstraintsQuery()).warnings, ())
                d.undo()
                self.assertEqual(d.to_bytes(), before_repair)


if __name__ == "__main__":
    unittest.main()
