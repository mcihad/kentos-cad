"""The seam measured on a turned first image, the grid itself (`cad.orthophoto.compare`'s `seam.resample`,
docs/adr/4060; plan/map-cad.md MAP-12), held by an independent reading.

The images are scripts/fixtures/orthophoto_compare_cases.py's synthetic GeoTIFFs: a north-up one of 0.10 m pixels
showing the scene as it is, and one turned 9° showing it displaced by a known shift s = (−0.028, 0.039) m. With the
turned image first:

- every place said is the centre of one of the turned image's pixels: the test's own inverse of its affine (GDAL's
  order: east = t0 + col·t1 + row·t2, north = t3 + col·t4 + row·t5) takes it to a whole pixel plus one half, and its
  window lies inside the image;
- the systematic shift is the north-up image's displacement less the turned one's, −s, within 3 mm, in east and north;
- the other way round (the north-up image first, ADR 3660's case) the shift turns over: s, within the same bound;
- without `resample` the measurement is refused.
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
from kentos.cad.types import OrthoSeamAsk, OrthoSubpixel  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.orthophoto.compare.json").read_text("utf-8"))
TURNED = (-0.028, 0.039)


def case(name: str) -> dict:
    return next(c for c in CASES["cases"] if c["name"] == name)


def opened(name: str) -> tuple[cad.Document, dict, dict]:
    c = case(name)
    setup = json.loads(json.dumps(c["setup"]).replace("$fixture:", str(ROOT / "fixtures") + "/"))
    return cad.Document.from_bytes(kr.container(kr.document(setup))), c["steps"][0]["input"], setup


def seam(inp: dict, **over) -> OrthoSeamAsk:
    s = dict(inp["seam"], **over)
    return OrthoSeamAsk(step=s["step"], window=s["window"], search=s["search"], correlation=s.get("correlation"),
                        resample=s.get("resample", False),
                        subpixel=OrthoSubpixel(s["subpixel"]) if "subpixel" in s else None)


def pixel(t: list[float], x: float, y: float) -> tuple[float, float]:
    """The test's own inverse of a GDAL affine: the (col, row) of a ground point."""
    det = t[1] * t[5] - t[2] * t[4]
    dx, dy = x - t[0], y - t[3]
    return (t[5] * dx - t[2] * dy) / det, (-t[4] * dx + t[1] * dy) / det


class TurnedGrid(unittest.TestCase):
    NAME = "dikiş: birinci görüntü 9° dönük, çift kübik piksel altı"

    def compare(self, doc, inp, first, second, **over):
        return cad.orthophoto.compare.plan(doc, first=first, second=second, first_points=inp["firstPoints"],
                                           second_points=inp["secondPoints"], seam=seam(inp, **over))

    def test_the_places_are_the_turned_image_s_pixel_centres(self) -> None:
        doc, inp, setup = opened(self.NAME)
        got = self.compare(doc, inp, inp["first"], inp["second"]).seam
        said = got.to_json()
        self.assertTrue(said.get("turned") and said.get("resampled"))
        turned = next(e for e in setup["entities"] if e["kind"] == "raster" and "dikis-e" in e["file"])
        t, (w, h) = turned["affine"], (turned["width"], turned["height"])
        half = inp["seam"]["window"] // 2
        self.assertGreater(len(got.patches), 5)
        for p in got.patches:
            c, r = pixel(t, p.at[0], p.at[1])
            self.assertAlmostEqual(c - 0.5, round(c - 0.5), delta=1e-6)
            self.assertAlmostEqual(r - 0.5, round(r - 0.5), delta=1e-6)
            ci, ri = round(c - 0.5), round(r - 0.5)
            self.assertTrue(ci - half >= 0 and ri - half >= 0 and ci + half < w and ri + half < h, (ci, ri))

    def test_the_shift_is_the_displacement_less_the_turned_one_s_and_turns_over(self) -> None:
        doc, inp, _ = opened(self.NAME)
        got = self.compare(doc, inp, inp["first"], inp["second"]).seam
        self.assertAlmostEqual(got.shift.mean_x, -TURNED[0], delta=0.003)
        self.assertAlmostEqual(got.shift.mean_y, -TURNED[1], delta=0.003)
        # The north-up image first: ADR 3660's way, the shift turned over.
        ahead = self.compare(doc, inp, inp["second"], inp["first"]).seam
        self.assertIsNone(ahead.to_json().get("turned"))
        self.assertAlmostEqual(ahead.shift.mean_x, TURNED[0], delta=0.003)
        self.assertAlmostEqual(ahead.shift.mean_y, TURNED[1], delta=0.003)

    def test_without_resample_it_is_refused(self) -> None:
        doc, inp, _ = opened(self.NAME)
        with self.assertRaises(CommandFailed) as refused:
            self.compare(doc, inp, inp["first"], inp["second"], resample=False)
        self.assertEqual(refused.exception.path, "seam")


if __name__ == "__main__":
    unittest.main()
