"""A changed object's shift set against a limit, and the profiles suggested for an item's kind (`map.work.query`'s
`limit` and `suggest`, docs/adr/5280; plan/map-cad.md MAP-01), held by an independent reading: the file is read by
tools/kcad/kcad.py, the reader written apart from KentOS, and what the answers should say is worked out here.

- The schema 211 sample's approved design is approved again (its approval keeps its objects' values) and its line moved
  0.3 m east and 0.4 m north: each end 0.5 m (the reader's ends before and after). Against the error limit regulation's
  m. 9(1) table, written here from its text: sayısal 1/1000 is 0.09 m (both ends beyond), takeometric near a traverse
  0.5 m (the full value, not a rounded one: 0.5 is not beyond 0.5 m, unless the ends moved a little more).
- A limit the table does not give is refused at its path.
- The design takes no profile in the sample: the zoning profile's classes say they suit designs (its own file).
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import MapWorkOperation, ShiftLimit  # noqa: E402

import kcad  # noqa: E402  (the independent reader)

SAMPLE = ROOT / "fixtures/kcad/v2/map-work-approved-parts.kcad"
LINE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13"
# Yanılma Sınırı Yönetmeliği m. 9(1), 1/1000: sayısal 0,09 m; takeometrik, poligona uzaklık 100 m'ye dek 0,5 m.
DIGITAL_1000 = 0.09
NEAR_1000 = 0.5


def ends(e: dict):
    return (e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])


class Limit(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        cad.map.work.edit(self.doc, operation=MapWorkOperation.from_json({"op": "set", "item": "is-3",
                                                                          "status": "approved"}))
        _, before = kcad.read(self.doc.to_bytes())
        cad.entities.transform(self.doc, uids=[LINE], transform=cad.MoveTransform(dx=0.3, dy=0.4))
        _, after = kcad.read(self.doc.to_bytes())
        a0, b0 = ends(dict(zip(before["uids"], before["entities"]))[LINE])
        a1, b1 = ends(dict(zip(after["uids"], after["entities"]))[LINE])
        self.moves = [math.dist(a0, a1), math.dist(b0, b1)]

    def ask(self, **limit) -> tuple[dict, dict]:
        tree = cad.map.work.query(self.doc, limit=ShiftLimit.from_json(limit)).to_json()
        design = tree["items"][2]
        return tree, design

    def test_sayisal_1000_puts_both_ends_beyond(self) -> None:
        tree, design = self.ask(basis="method", method="sayisal", scale=1000)
        self.assertEqual(tree["limit"]["metres"], DIGITAL_1000)
        self.assertIn("m. 9(1)", tree["limit"]["source"])
        shift = next(o for o in design["changes"]["objects"] if o["uid"] == LINE)["shift"]
        self.assertTrue(shift["beyond"])
        self.assertEqual(shift["over"], sum(m > DIGITAL_1000 for m in self.moves))
        self.assertEqual(design["changes"]["beyond"], 1)
        self.assertTrue(any("sınırın dışına" in w for w in design["why"]))

    def test_the_near_traverse_value_is_set_against_the_full_move(self) -> None:
        _, design = self.ask(basis="method", method="takeometrik-yakin", scale=1000)
        shift = next(o for o in design["changes"]["objects"] if o["uid"] == LINE)["shift"]
        self.assertEqual(shift["beyond"], max(self.moves) > NEAR_1000)
        self.assertEqual(shift["over"], sum(m > NEAR_1000 for m in self.moves))

    def test_the_drawing_limit_is_two_tenths_of_a_millimetre_at_the_scale(self) -> None:
        tree, _ = self.ask(basis="drawing", scale=5000)
        self.assertAlmostEqual(tree["limit"]["metres"], 0.2 * 5000 / 1000, delta=1e-12)

    def test_a_cell_the_table_leaves_empty_is_refused(self) -> None:
        with self.assertRaises(CommandFailed) as refused:
            self.ask(basis="method", method="fotoplan", scale=500)
        self.assertEqual((refused.exception.code, refused.exception.path), ("invalid_input", "limit.scale"))


class Suggest(unittest.TestCase):
    def test_a_design_is_offered_the_zoning_profile_s_classes(self) -> None:
        doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        _, saved = kcad.read(doc.to_bytes())
        items = saved["settings"]["mapWork"]["items"]
        zoning = json.loads((ROOT / "catalog/v1/profiles/imar-plani.json").read_text("utf-8"))
        rows = cad.map.work.query(doc, suggest=True).to_json()["items"]
        for item, row in zip(items, rows):
            want = [c["id"] for c in zoning["classes"] if item["kind"] in c.get("work", [])]
            got = next((s["classes"] for s in row.get("suggested", []) if s["name"] == "imar-plani"), [])
            self.assertEqual(got, [] if "profile" in item else want, item["kind"])
        self.assertTrue(any(i["kind"] == "design" for i in items))


if __name__ == "__main__":
    unittest.main()
