"""Sections along a drawn axis and their drawing (`road.terrains.compare`'s `axis`, `road.terrains.cutfill`'s `sections`
and `materials`, docs/adr/4760; plan/map-cad.md MAP-15), held by this test's own readings of the shared cases' drawings.

- On the axis's quarter circle round (8005, 6) every section lies along the radius: its two ends and the centre on one
  line, the left end 1.5 m nearer the centre and the right one 1.5 m farther than the axis's place.
- The volume is the test's own average end area over the sections' stations.
- Drawn with a start of 1000 m, each section's kilometre is the test's own `k+mmm.cc` of 1000 + its station, the
  table's running sums are the test's own sums of the volumes before, and its Brückner value is the running cut less
  the running fill, each times its factor (1 when none; docs/adr/5140).
- A material's words are its name and its own cut and fill to two decimals.
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

import kcad_v2_reference as kr  # noqa: E402

SECTIONS = json.loads((ROOT / "fixtures/commands/v1/road.terrains.compare.sections.json").read_text("utf-8"))
CUTFILL = json.loads((ROOT / "fixtures/commands/v1/road.terrains.cutfill.json").read_text("utf-8"))


def opened(cases) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(cases["setup"])))


def case(cases, start: str) -> dict:
    return next(c for c in cases["cases"] if c["name"].startswith(start))["steps"][0]


class AlongAnAxis(unittest.TestCase):
    def test_on_the_arc_the_sections_follow_the_radius(self) -> None:
        s = case(SECTIONS, "Yaylı eksen: kurpta")
        out = cad.road.terrains.compare.plan_input(opened(SECTIONS), cad.TerrainsCompare.from_json(s["input"]))
        rows = out.sections
        centre = (8005.0, 6.0)
        on_arc = [r for r in rows if 4.0 < r.station < 4.0 + 2.0 * math.pi]
        self.assertGreater(len(on_arc), 2)
        for r in on_arc:
            (lx, ly), (rx, ry) = r.ends
            ax, ay = r.at
            # The centre, the ends and the place on one line.
            cross = (lx - centre[0]) * (ry - centre[1]) - (ly - centre[1]) * (rx - centre[0])
            self.assertLess(abs(cross), 1e-9, r)
            here = math.dist((ax, ay), centre)
            self.assertAlmostEqual(math.dist((lx, ly), centre), here - 1.5, delta=1e-9)
            self.assertAlmostEqual(math.dist((rx, ry), centre), here + 1.5, delta=1e-9)
        mine_cut = sum((b.station - a.station) * (a.cut_area + b.cut_area) / 2 for a, b in zip(rows, rows[1:]))
        mine_fill = sum((b.station - a.station) * (a.fill_area + b.fill_area) / 2 for a, b in zip(rows, rows[1:]))
        self.assertAlmostEqual(out.cut, mine_cut, delta=1e-9)
        self.assertAlmostEqual(out.fill, mine_fill, delta=1e-9)

    def test_drawn_the_kilometres_and_the_running_sums_are_the_test_s(self) -> None:
        s = case(CUTFILL, "Kesitler eksen boyunca")
        out = cad.road.terrains.cutfill.plan_input(opened(CUTFILL), cad.TerrainsCutFill.from_json(s["input"]))
        start = s["input"]["sections"]["start"]

        def km(v: float) -> str:
            cm = math.floor(v * 100.0 + 0.5)
            return f"{cm // 100000}+{(cm % 100000) // 100:03d}.{cm % 100:02d}"
        self.assertEqual([x.km for x in out.sections], [km(start + x.station) for x in out.sections])
        cut = fill = 0.0
        factors = s["input"]["sections"]
        k_cut, k_fill = factors.get("cutFactor", 1.0), factors.get("fillFactor", 1.0)
        for k, row in enumerate(out.section_table[2:], start=1):
            cut += out.sections[k].cut_volume
            fill += out.sections[k].fill_volume
            self.assertEqual(row[6:], [f"{cut:.2f}", f"{fill:.2f}", f"{cut * k_cut - fill * k_fill:.2f}"])
        self.assertAlmostEqual(cut, out.section_cut, delta=1e-9)

    def test_a_material_s_words_are_its_name_and_volumes(self) -> None:
        s = case(CUTFILL, "Malzeme bölgeleri")
        out = cad.road.terrains.cutfill.plan_input(opened(CUTFILL), cad.TerrainsCutFill.from_json(s["input"]))
        for m, words in zip(out.materials, out.material_labels):
            self.assertEqual(words.text, f"{m.name}: kazı {m.cut:.2f} m³, dolgu {m.fill:.2f} m³")
        self.assertEqual(out.materials[1].name, "Toprak", "the name trimmed")


if __name__ == "__main__":
    unittest.main()
