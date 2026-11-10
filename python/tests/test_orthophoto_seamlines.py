"""Mozaik birleşme çizgisi (`cad.orthophoto.seamlines`, docs/adr/4700; plan/map-cad.md MAP-12): the seamline
(birleşme çizgisi) and the mosaic elements (mozaik elemanı), held by this test's own readings.

The drawings are scripts/fixtures/orthophoto_seamlines_cases.py's: the synthetic GeoTIFFs of
orthophoto_compare_cases.py (20 × 12 m at 10 cm, overlapping 10 m), their footprints the rasters' extents or areas.

- Voronoi: every seam point is as far from the first image's own part as from the second's, read here as the
  distances to the stretches of each footprint's boundary inside the other (the staggered strips' two L-shapes).
- The two polygons do not overlap and cover both footprints: their areas (shoelace here) add up to the union's.
- Written, they are the seamline polygons Ortofoto karşılaştırması measures along (docs/adr/4460): the shared
  boundary it finds is the seam itself, and nothing else.
- Radiometry with a building to avoid: no seam edge passes inside it (sampled every millimetre), and the bottleneck is
  no larger than the largest difference down the pixel column the Voronoi seam runs in (a path itself), read here
  from the images' own pixel values.
- Footprints crossing like a plus are refused at `footprints`.
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
from kentos.cad.types import OrthoSeamAsk  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import orthophoto_compare_cases as oc  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.orthophoto.seamlines.json").read_text("utf-8"))
X0, Y0 = oc.X0, oc.Y0


def setup_of(start: str) -> dict:
    case = next(c for c in CASES["cases"] if c["name"].startswith(start))
    return json.loads(json.dumps(case["setup"]).replace("$fixture:", str(ROOT / "fixtures") + "/"))


def opened(setup: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def uid(n: int) -> str:
    return f"0192f5c0-4700-7000-8000-{n:012x}"


def seg_dist(p, a, b) -> float:
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)))
    return math.hypot(p[0] - (a[0] + t * dx), p[1] - (a[1] + t * dy))


def shoelace(ring) -> float:
    return sum(ring[k][0] * ring[(k + 1) % len(ring)][1] - ring[(k + 1) % len(ring)][0] * ring[k][1]
               for k in range(len(ring))) / 2


class Seamlines(unittest.TestCase):
    def test_the_voronoi_seam_is_equally_far_from_both_images_own_parts(self) -> None:
        doc = opened(setup_of("kaydırılmış şeritler"))
        got = cad.orthophoto.seamlines.plan(doc, first=uid(1), second=uid(2), footprints=[uid(3), uid(4)])
        # The first image is [500, 520] × [108, 120], the second [510, 530] × [105, 117]: the first's own part ends
        # at the second's west and north edges inside it, the second's at the first's south and east edges.
        at = lambda x, y: (X0 + x, Y0 + y)  # noqa: E731
        first_end = [(at(510, 108), at(510, 117)), (at(510, 117), at(520, 117))]
        second_end = [(at(510, 108), at(520, 108)), (at(520, 108), at(520, 117))]
        for k in range(len(got.seam) - 1):
            a, b = got.seam[k], got.seam[k + 1]
            for i in range(101):
                p = (a[0] + (b[0] - a[0]) * i / 100, a[1] + (b[1] - a[1]) * i / 100)
                d1 = min(seg_dist(p, *e) for e in first_end)
                d2 = min(seg_dist(p, *e) for e in second_end)
                self.assertAlmostEqual(d1, d2, delta=1e-9)
        union = 240.0 + 240.0 - 90.0
        self.assertAlmostEqual(shoelace(got.polygons[0].ring) + shoelace(got.polygons[1].ring), union, delta=1e-6)
        self.assertAlmostEqual(got.overlap_area, 90.0, delta=1e-9)

    def test_written_polygons_are_what_the_comparison_measures_along(self) -> None:
        doc = opened(setup_of("iki şerit yan yana"))
        made = cad.orthophoto.seamlines(doc, first=uid(1), second=uid(2))
        ids = [p.uid for p in made.polygons]
        self.assertTrue(all(ids))
        compared = cad.orthophoto.compare.plan(
            doc, first=uid(1), second=uid(2), first_points="0", second_points="0",
            seam=OrthoSeamAsk(step=1.0, window=21, search=5, correlation=0.8, polygons=ids))
        self.assertEqual([[tuple(v) for v in part] for part in compared.seam.shared.parts],
                         [[tuple(v) for v in made.seam]])
        self.assertAlmostEqual(compared.seam.shared.length, made.length, delta=1e-9)

    def test_radiometry_keeps_out_of_the_building(self) -> None:
        doc = opened(setup_of("radyometrik fark, binalardan geçmeden"))
        got = cad.orthophoto.seamlines.plan(doc, first=uid(1), second=uid(2), footprints=[uid(3), uid(4)],
                                            method="radiometry", avoid="bina")
        x0, x1, y0, y1 = X0 + 514.4, X0 + 515.6, Y0 + 114.4, Y0 + 115.6
        for k in range(len(got.seam) - 1):
            a, b = got.seam[k], got.seam[k + 1]
            n = max(1, int(math.dist(a, b) / 0.001))
            for i in range(n + 1):
                x, y = a[0] + (b[0] - a[0]) * i / n, a[1] + (b[1] - a[1]) * i / n
                self.assertFalse(x0 < x < x1 and y0 < y < y1, f"inside the building at {x}, {y}")

    def test_the_bottleneck_is_no_larger_than_the_voronoi_columns_worst(self) -> None:
        doc = opened(setup_of("radyometrik fark: en büyük farkı en küçük yol"))
        got = cad.orthophoto.seamlines.plan(doc, first=uid(1), second=uid(2), footprints=[uid(3), uid(4)],
                                            method="radiometry")
        # The Voronoi seam runs down x = 514.985, in the first's pixel column 149 (the second's 49): that column from
        # the start's cell to the end's is a path itself, so no bottleneck is larger than its worst difference.
        a = oc.image(*oc.IMAGES["dikis-a.tif"])
        b = oc.image(*oc.IMAGES["dikis-b.tif"])
        worst = max(abs(a[j][149] - b[j][49]) for j in range(120))
        self.assertLessEqual(got.difference.bottleneck, worst)
        self.assertLess(got.difference.mean, got.difference.searched_mean)

    def test_footprints_crossing_like_a_plus_are_refused(self) -> None:
        doc = opened(setup_of("artı gibi kesişen"))
        with self.assertRaises(CommandFailed) as refused:
            cad.orthophoto.seamlines.plan(doc, first=uid(1), second=uid(2), footprints=[uid(3), uid(4)])
        self.assertEqual((refused.exception.code, refused.exception.path), ("invalid_input", "footprints"))


if __name__ == "__main__":
    unittest.main()
