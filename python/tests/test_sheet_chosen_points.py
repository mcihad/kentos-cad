"""The surveyor's chosen points' heights on the sheet (`road.terrains.sheetcontours` with `points`, docs/adr/3520;
plan/map-cad.md MAP-14) through the SDK, held by the shared case file scripts/fixtures/road_sheet_contour_cases.py
writes (its own reading of the surface; no KentOS code in the expectations).

- The plan answers the case's spots: a point with its own height keeps it (the surface's beside it), one without takes
  the surface's, a place given twice once; the note counts the one off the surface without a height.
- Drawn, each chosen point gets only its height's text beside it (the point is the surveyor's); the slopes' corners
  keep their points.
- A package without a spot height refuses the chosen points.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/road.terrains.sheetcontours.json").read_text())
CMD = cad.road.terrains.sheetcontours


def drawing() -> cad.Document:
    # The case's drawing is the file's v2 form: written by the independent KCAD v2 writer, then opened as a file.
    return cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))


def step(prefix: str) -> dict:
    return next(c for c in CASES["cases"] if c["name"].startswith(prefix))["steps"][0]


class SheetChosenPoints(unittest.TestCase):
    def test_the_plan_is_the_reading_s(self) -> None:
        s = step("Seçilen noktalar planı yazmaz")
        doc = drawing()
        before = doc.to_bytes()
        out = CMD.plan_input(doc, cad.TerrainsSheetContours.from_json(s["input"]))
        self.assertEqual(doc.to_bytes(), before, "the plan writes nothing")
        want = s["result"]["output"]["spots"]
        self.assertEqual([(g.kind, g.text) for g in out.spots], [(w["kind"], w["text"]) for w in want])
        for g, w in zip(out.spots, want):
            for key in ("x", "y", "z"):
                self.assertAlmostEqual(getattr(g, key), w[key], places=9)
            if "surface" in w:
                self.assertAlmostEqual(g.surface, w["surface"], places=9)
            else:
                self.assertIn(g.surface, (None, cad.UNSET))
        note = next(n for n in out.notes if n.startswith("Ölçmecinin seçtiği"))
        self.assertEqual(note, next(n for n in s["result"]["output"]["notes"] if n.startswith("Ölçmecinin seçtiği")))
        self.assertIn("kendi kotu olan 2 nokta", note)
        self.assertIn("1 nokta yazılmadı", note)

    def test_drawn_only_the_height_is_written_at_a_chosen_point(self) -> None:
        s = step("Şevler ve seçilen noktalar birlikte")
        doc = drawing()
        out = CMD.execute_input(doc, cad.TerrainsSheetContours.from_json(s["input"]))
        records = [doc.entity(u).entity.to_json() for u in out.uids]
        points = {(r["p"]["x"], r["p"]["y"]) for r in records if r["kind"] == "point"}
        texts = [r for r in records if r["kind"] == "text"]
        chosen = [g for g in out.spots if g.kind == "chosen"]
        slopes = [g for g in out.spots if g.kind != "chosen"]
        self.assertEqual(len(chosen), 4)
        self.assertEqual(len(slopes), 9)
        self.assertEqual(points, {(g.x, g.y) for g in slopes}, "a point only at the slopes' corners")
        for g in out.spots:
            beside = [t for t in texts if t["text"] == g.text and t["p"]["y"] == g.y and t["p"]["x"] > g.x]
            self.assertEqual(len(beside), 1, g)

    def test_a_package_without_a_spot_height_refuses(self) -> None:
        s = step("Seçilen noktalar, kot noktası yazı yüksekliği vermeyen paket")
        doc = drawing()
        before = doc.to_bytes()
        answer = CMD.run(doc, cad.TerrainsSheetContours.from_json(s["input"]))
        self.assertEqual(answer.error.code, s["result"]["error"]["code"])
        self.assertEqual(str(answer.error.path), s["result"]["error"]["path"])
        self.assertEqual(doc.to_bytes(), before)


if __name__ == "__main__":
    unittest.main()
