"""İmar planını denetle from a script (`cad.processing.run` with `zoning.check`, docs/adr/3840; plan/map-cad.md MAP-09),
held by `cad.zoning.check`'s shared case (its independent reference's findings and regions), no KentOS code in the
expectations.

On the drawing with four decisions and three parcels in a wide road (fixtures/processing/v1/zoning-yollar.kcad):

- without Bulguları hata katmanına yaz the findings' objects are chosen and nothing is written;
- with it, each region of the case's seven findings is an area on the new error layer, its fields the finding's
  (kind in words, the objects' lasting ids, the region's place among the finding's), its area the region's by this
  test's own shoelace; one undo step takes them away.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

import kentos.cad as cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

FOLDER = ROOT / "fixtures/processing/v1"
CASE = "yol genişliği çizilenden geniş: yola giren kararlar ve parseller"


def drawing() -> cad.Document:
    setup = json.loads((FOLDER / "zoning-yollar.kcad").read_text("utf-8"))
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def shoelace(pts: list[dict]) -> float:
    o = pts[0]
    rel = [(p["x"] - o["x"], p["y"] - o["y"]) for p in pts]
    return abs(sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(rel, rel[1:] + rel[:1]))) / 2.0


def findings() -> list[dict]:
    cases = json.loads((ROOT / "fixtures/commands/v1/cad.zoning.check.json").read_text("utf-8"))["cases"]
    case = next(c for c in cases if c["name"].startswith(CASE))
    return case["steps"][0]["result"]["output"]["findings"]


class ZoningProcessing(unittest.TestCase):
    def test_without_the_box_the_objects_are_chosen(self) -> None:
        doc = drawing()
        ran = cad.processing.run(doc, tool="zoning.check", parameters={})
        want = {u for f in findings() for u in f["uids"]}
        self.assertEqual(set(ran.selected or []), want)
        self.assertFalse(ran.added)
        self.assertIn("7 bulgu verdi; bulgulu nesneler seçildi.", ran.summary)

    def test_the_regions_go_to_the_error_layer_in_one_step(self) -> None:
        doc = drawing()
        before = len(list(doc.entities()))
        ran = cad.processing.run(doc, tool="zoning.check",
                                 parameters={"write": True, "layer": {"newName": "İmar bulguları"}})
        fs = findings()
        regions = [(f, k, r) for f in fs for k, r in enumerate(f["regions"])]
        self.assertEqual(len(ran.added), len(regions))
        self.assertEqual(ran.step, "İmar planını denetle")
        for uid, (f, k, r) in zip(ran.added, regions):
            e = doc.entity(uid).entity.to_json()
            self.assertEqual(e["kind"], "polygon")
            self.assertEqual(e["attrs"]["Nesneler"], ", ".join(f["uids"]))
            self.assertEqual(e["attrs"]["Bölge"], f"{k + 1}/{len(f['regions'])}")
            self.assertEqual(e["attrs"]["İleti"], f["message"])
            self.assertAlmostEqual(shoelace(e["pts"]), shoelace(r["outer"]["pts"]), places=6)
        doc.undo()
        self.assertEqual(len(list(doc.entities())), before, "one step")


if __name__ == "__main__":
    unittest.main()
