"""The seam measured with the second image resampled onto the first's grid (`cad.orthophoto.compare`'s
`seam.resample`, docs/adr/3660; plan/map-cad.md MAP-12), held by an independent reading.

The images are scripts/fixtures/orthophoto_compare_cases.py's synthetic GeoTIFFs: the first of 0.10 m pixels, a second
of 0.08 m pixels showing the scene displaced by a known shift s = (0.046, 0.012) m, and a third turned 9° displaced by
(−0.028, 0.039) m. Without `resample` the measurement is refused. With it:

- the systematic shift (the mean over the counted places, those whose window reaches the first's flat patch among
  them) is the known shift within 0.03 pixel (3 mm), under the 0.05 pixel each clean place holds to in the shared
  cases;
- the other way round (the 0.08 m image as the grid, the first resampled onto it) the shift turns over: −s, within the
  same bound, though every place now lies on the other image's pixels;
- the turned image's shift is its own known shift.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import OrthoSeamAsk  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.orthophoto.compare.json").read_text("utf-8"))
FINER, TURNED = (0.046, 0.012), (-0.028, 0.039)


def case(start: str) -> dict:
    return next(c for c in CASES["cases"] if c["name"].startswith(start))


def opened(start: str) -> tuple[cad.Document, dict]:
    c = case(start)
    setup = json.loads(json.dumps(c["setup"]).replace("$fixture:", str(ROOT / "fixtures") + "/"))
    return cad.Document.from_bytes(kr.container(kr.document(setup))), c["steps"][0]["input"]


def seam(inp: dict, **over) -> OrthoSeamAsk:
    s = dict(inp["seam"], **over)
    return OrthoSeamAsk(step=s["step"], window=s["window"], search=s["search"], correlation=s.get("correlation"),
                        resample=s.get("resample", False))


class Resample(unittest.TestCase):
    def compare(self, doc, inp, first, second, first_points, second_points, **over):
        return cad.orthophoto.compare.plan(doc, first=first, second=second, first_points=first_points,
                                           second_points=second_points, seam=seam(inp, **over))

    def test_the_finer_image_s_shift_and_the_other_way_round(self) -> None:
        doc, inp = opened("dikiş: ikinci görüntü 0,08 m pikselli")
        a, b = inp["first"], inp["second"]
        with self.assertRaises(CommandFailed) as refused:
            self.compare(doc, inp, a, b, inp["firstPoints"], inp["secondPoints"], resample=False)
        self.assertEqual(refused.exception.path, "seam")
        ahead = self.compare(doc, inp, a, b, inp["firstPoints"], inp["secondPoints"]).seam
        self.assertTrue(ahead.resampled)
        self.assertGreater(ahead.ok, 20)
        self.assertAlmostEqual(ahead.shift.mean_x, FINER[0], delta=0.003)
        self.assertAlmostEqual(ahead.shift.mean_y, FINER[1], delta=0.003)
        # The 0.08 m image as the grid: the first resampled onto it, the shift turned over.
        back = self.compare(doc, inp, b, a, inp["secondPoints"], inp["firstPoints"]).seam
        self.assertTrue(back.resampled)
        self.assertGreater(back.ok, 20)
        self.assertAlmostEqual(back.shift.mean_x, -FINER[0], delta=0.003)
        self.assertAlmostEqual(back.shift.mean_y, -FINER[1], delta=0.003)

    def test_the_turned_image_s_shift(self) -> None:
        doc, inp = opened("dikiş: ikinci görüntü 9° dönük")
        got = self.compare(doc, inp, inp["first"], inp["second"], inp["firstPoints"], inp["secondPoints"]).seam
        self.assertTrue(got.resampled)
        self.assertAlmostEqual(got.shift.mean_x, TURNED[0], delta=0.003)
        self.assertAlmostEqual(got.shift.mean_y, TURNED[1], delta=0.003)


if __name__ == "__main__":
    unittest.main()
