"""An accepted GNSS network's ellipse labels kept clear of the ellipses and of each other (`map.survey.edit`'s network,
docs/adr/4160; plan/map-cad.md MAP-05), held by the test's own measuring (no KentOS code in the expectations).

The drawing is the shared cases' network with N6 15 m east of N1 (fixtures/commands/v1/map.survey.edit.json). The test
measures each label as the drawing does one line of text: its letters' advances in the drawing's typeface (Barlow,
weight 500, read here with fontTools) times its height wide, from 0.2 of 1.15 heights under its baseline to 1.15
heights over it, placed by its alignment (ADR 0145). Then:

- no label's box meets an ellipse (its edges kept farther than the ellipse in the ellipse's own frame) nor another label;
- N1's label at east would have met N6's ellipse, so it stands at the top right: its bottom left at the point plus
  (a + h)/√2 up and right; the others stand east;
- the answer says one label moved and none crowded.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
sys.path.insert(0, str(ROOT / "tools/kcad"))

from fontTools.ttLib import TTFont  # noqa: E402
from kentos import cad  # noqa: E402
from kentos.cad.types import MapSurveyOperation  # noqa: E402

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/map.survey.edit.json").read_text("utf-8"))
CASE = next(c for c in CASES["cases"] if c["name"].startswith("Ağı kabul et, etiketler: N1"))
FONT = TTFont(ROOT / "apps/desktop/assets/fonts/drawing/Barlow-500.ttf")
UPEM, CMAP, HMTX = FONT["head"].unitsPerEm, FONT.getBestCmap(), FONT["hmtx"]


def frame(label: dict) -> tuple[float, float, float, float]:
    h = label["height"]
    w = sum(round(HMTX[CMAP[ord(c)]][0] * 1000 / UPEM) for c in label["text"]) / 1000.0 * h
    align = label["align"]
    rise = {"bottom": -0.2, "middle": 0.5, "top": 1.0}[next(k for k in ("bottom", "middle", "top") if align.startswith(k))]
    along = {"Left": 0.0, "Center": 0.5, "Right": 1.0}[align[align.index(next(c for c in align if c.isupper())):]]
    x0, base = label["p"]["x"] - along * w, label["p"]["y"] - rise * h
    return x0, base - 0.23 * h, x0 + w, base + 1.15 * h


def meets_ellipse(f, e) -> bool:
    c, m = e["c"], e["major"]
    a = math.hypot(m["x"], m["y"])
    b = a * e["ratio"]
    if f[0] < c["x"] < f[2] and f[1] < c["y"] < f[3]:
        return True
    ux, uy = m["x"] / a, m["y"] / a
    k = [((x - c["x"]) * ux + (y - c["y"]) * uy, (-(x - c["x"]) * uy + (y - c["y"]) * ux))
         for x, y in ((f[0], f[1]), (f[2], f[1]), (f[2], f[3]), (f[0], f[3]))]
    k = [(u / a, v / b) for u, v in k]

    def near(p, q):
        dx, dy = q[0] - p[0], q[1] - p[1]
        t = min(max(-(p[0] * dx + p[1] * dy) / (dx * dx + dy * dy), 0.0), 1.0)
        return math.hypot(p[0] + t * dx, p[1] + t * dy)
    return any(near(k[i], k[(i + 1) % 4]) < 1.0 for i in range(4))


def overlap(f, g) -> bool:
    return f[0] < g[2] and g[0] < f[2] and f[1] < g[3] and g[1] < f[3]


class LabelPlaces(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASE["setup"])))
        operation = MapSurveyOperation.from_json(CASE["steps"][0]["input"]["operation"])
        self.out = cad.map.survey.edit(self.doc, operation=operation).to_json()
        _, saved = kcad.read(self.doc.to_bytes())
        on = [e for e in saved["entities"] if e["layerId"] == "gnss"]
        self.ellipses = [e for e in on if e["kind"] == "ellipse"]
        self.labels = [e for e in on if e["kind"] == "text"]

    def test_no_label_meets_an_ellipse_or_another_label(self) -> None:
        self.assertEqual(len(self.labels), 4)
        frames = [frame(lab) for lab in self.labels]
        for i, f in enumerate(frames):
            for e in self.ellipses:
                self.assertFalse(meets_ellipse(f, e), (self.labels[i]["attrs"]["İşaret"], e["attrs"]["İşaret"]))
            for j, g in enumerate(frames):
                if i != j:
                    self.assertFalse(overlap(f, g))

    def test_n1_s_label_leaves_east_for_the_top_right(self) -> None:
        by_mark = {lab["attrs"]["İşaret"]: lab for lab in self.labels}
        ellipse = {e["attrs"]["İşaret"]: e for e in self.ellipses}
        n1, e1 = by_mark["N1"], ellipse["N1"]
        a = math.hypot(e1["major"]["x"], e1["major"]["y"])
        d = (a + n1["height"]) / math.sqrt(2.0)
        self.assertEqual(n1["align"], "bottomLeft")
        self.assertAlmostEqual(n1["p"]["x"], e1["c"]["x"] + d, delta=1e-9)
        self.assertAlmostEqual(n1["p"]["y"], e1["c"]["y"] + d, delta=1e-9)
        east = dict(n1, align="middleLeft", p={"x": e1["c"]["x"] + a + n1["height"], "y": e1["c"]["y"]})
        self.assertTrue(meets_ellipse(frame(east), ellipse["N6"]), "east lay over N6's ellipse")
        for mark in ("N4", "N5", "N6"):
            self.assertEqual(by_mark[mark]["align"], "middleLeft", mark)
        self.assertEqual((self.out.get("labelsMoved"), self.out.get("labelsCrowded")), (1, None))


if __name__ == "__main__":
    unittest.main()
