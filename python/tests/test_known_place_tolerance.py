"""The user's tolerance of a known point's place against its mark's accepted point (`map.survey.query`'s
`tolerance`, docs/adr/4520; plan/map-cad.md MAP-03), held by the test's own rule.

The drawing is the shared case's (fixtures/commands/v1/map.survey.query.json) where N1 is both a known point and an
accepted one, its published coordinates put 6 mm east, 4 mm south and 12 mm up of its accepted point. By the test's
rule (Trimble Business Center's point comparison: a horizontal and a vertical tolerance):

- N1's horizontal distance is √(6² + 4²) mm and its height difference 12 mm (within 0.1 mm: the coordinates went
  through the chain);
- it is beyond a limit its horizontal distance passes, or its height difference passes; BN1 and BN2, without accepted
  points, are not judged;
- a tolerance without the places, empty, or not above 0 is refused at its own path.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/map.survey.query.json").read_text("utf-8"))
SETUP = next(c for c in CASES["cases"] if c["name"].startswith("Kabul farkı sınırı: yatay 5 mm"))["setup"]
EAST, NORTH, UP = 0.006, -0.004, 0.012


def opened() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(SETUP)))


def places(doc: cad.Document, tolerance: dict) -> dict:
    query = cad.command("map.survey.query")
    answer = query.run(doc, query.input_type.from_json({"places": True, "tolerance": tolerance}), op="plan")
    assert answer.error is None, answer.error and answer.error.message
    return {p["mark"]: p for p in answer.output.to_json()["knownPlaces"]}


class KnownPlaceTolerance(unittest.TestCase):
    def test_n1_against_the_limits(self) -> None:
        doc = opened()
        horizontal = math.hypot(EAST, NORTH)
        for tolerance in [{"horizontal": 0.005}, {"horizontal": 0.01}, {"vertical": 0.01}, {"vertical": 0.02},
                          {"horizontal": 0.01, "vertical": 0.01}]:
            got = places(doc, tolerance)
            n1 = got["N1"]
            self.assertAlmostEqual(n1["horizontal"], horizontal, delta=1e-4)
            self.assertAlmostEqual(n1["dz"], UP, delta=1e-4)
            want = (("horizontal" in tolerance and horizontal > tolerance["horizontal"])
                    or ("vertical" in tolerance and abs(UP) > tolerance["vertical"]))
            self.assertEqual(n1["beyond"], want, tolerance)
            for mark in ("BN1", "BN2"):
                self.assertNotIn("beyond", got[mark], (mark, tolerance))

    def test_refusals(self) -> None:
        doc = opened()
        query = cad.command("map.survey.query")
        for input_, path in [({"tolerance": {"horizontal": 0.01}}, "tolerance"),
                             ({"places": True, "tolerance": {}}, "tolerance"),
                             ({"places": True, "tolerance": {"horizontal": 0.0}}, "tolerance.horizontal"),
                             ({"places": True, "tolerance": {"vertical": -1.0}}, "tolerance.vertical")]:
            answer = query.run(doc, query.input_type.from_json(input_), op="plan")
            self.assertEqual((answer.error.code, answer.error.path), ("invalid_input", path), input_)
        with self.assertRaises(CommandFailed):
            cad.map.survey.query(doc, places=True, tolerance=cad.SurveyPlaceTolerance(horizontal=-0.5))


if __name__ == "__main__":
    unittest.main()
