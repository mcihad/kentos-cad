"""İzlemenin dönem dizisi (`cad.asbuilt.check`'s `epochs`; docs/adr/4420; plan/map-cad.md MAP-19) through KentOS's
Python, held by this test's own reading of the drawing.

On the series cases' drawing (three epochs of the same marks, `Ad` their names): the third epoch against the second,
with the first two as the series and their days:

- every mark measured now and in another epoch is tracked, in this epoch's order; a mark measured only now (M7) is not;
- its reference is the first epoch it is in (M6: the second); each step its place now less its reference's, read here
  from the drawing's own points; a step is flagged beyond the limits (8 mm horizontal, 10 mm vertical);
- its rates: its last movement over the years between its reference's day and this one (365.25 days a year).
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.asbuilt.check.series.json").read_text("utf-8"))
SETUP = CASES["setup"]
DAYS = {"donem1": "2026-01-15", "donem2": "2026-04-15", "donem3": "2026-07-15"}


def epoch(layer: str) -> dict[str, tuple[float, float, float]]:
    """A layer's marks by name, read from the drawing by this test."""
    return {e["attrs"]["Ad"]: (e["p"]["x"], e["p"]["y"], e["z"]) for e in SETUP["entities"] if e["layerId"] == layer}


class AsbuiltSeries(unittest.TestCase):
    def test_each_mark_through_the_epochs(self) -> None:
        doc = cad.Document.from_bytes(kr.container(kr.document(SETUP)))
        check = cad.command("cad.asbuilt.check")
        answer = check.run(doc, check.input_type.from_json({
            "kind": "monitoring", "measured": "donem3", "design": "donem2", "horizontal": 0.008, "vertical": 0.01,
            "epochs": [{"layer": "donem1", "date": DAYS["donem1"]}, {"layer": "donem2", "date": DAYS["donem2"]}],
            "date": DAYS["donem3"]}), op="plan")
        self.assertIsNone(answer.error, answer.error and answer.error.message)
        series = [epoch("donem1"), epoch("donem2"), epoch("donem3")]
        days = [DAYS["donem1"], DAYS["donem2"], DAYS["donem3"]]
        want = [n for n in series[2] if sum(1 for e in series if n in e) >= 2]
        tracks = answer.output.tracks
        self.assertEqual([t.name for t in tracks], want)
        self.assertNotIn("M7", want)
        for t in tracks:
            frm = next(k for k, e in enumerate(series) if t.name in e)
            self.assertEqual(t.from_, frm, t.name)
            rx, ry, rz = series[frm][t.name]
            for k, step in enumerate(t.steps):
                if t.name not in series[k] or k < frm:
                    self.assertIsNone(step, (t.name, k))
                    continue
                x, y, z = series[k][t.name]
                self.assertAlmostEqual(step.de, x - rx, delta=1e-9)
                self.assertAlmostEqual(step.dn, y - ry, delta=1e-9)
                self.assertAlmostEqual(step.dz, z - rz, delta=1e-9)
                self.assertAlmostEqual(step.horizontal, math.hypot(x - rx, y - ry), delta=1e-9)
                beyond = math.hypot(x - rx, y - ry) > 0.008 or abs(z - rz) > 0.01
                self.assertEqual(bool(step.outside), beyond, (t.name, k))
            years = (date.fromisoformat(days[-1]) - date.fromisoformat(days[frm])).days / 365.25
            x, y, z = series[2][t.name]
            self.assertAlmostEqual(t.rate_horizontal, math.hypot(x - rx, y - ry) / years, delta=1e-9)
            self.assertAlmostEqual(t.rate_vertical, (z - rz) / years, delta=1e-9)


if __name__ == "__main__":
    unittest.main()
