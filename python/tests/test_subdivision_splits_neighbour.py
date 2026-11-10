"""İfrazda komşunun sınırının bölünmesi (`cad.parcels.record`, docs/adr/4620; plan/map-cad.md MAP-08), held by the
test's own walk of the register (no KentOS code in the expectations).

The drawing is the shared cases' (scripts/fixtures/parcel_register_cases.py): 101/7 (A1, (0, 0)–(20, 10)), 101/8 (A2,
east of it) and 101/9 (A3, north of 101/7, sharing the edge (0, 10)–(20, 10)), recorded by Kadastro 1985; 101/7 is
divided at x = 10 into 101/13 (A11) and 101/14 (A12). By the test's rule (ArcGIS: a bounding line shared with a current
adjacent parcel is split so that it has coincident current lines, the original kept as history):

- 101/9's ring walks its corners (0, 10), (10, 10), (20, 10), (20, 20), (0, 20): the new corner (10, 10) is in it;
- the old boundary (0, 10)–(20, 10) is closed by the subdivision's document; its pieces are 101/13's and 101/14's north
  boundaries, so 101/9's neighbours are 101/13 and 101/14;
- 101/9's area, drawn without the corner, reads changed until the corner is drawn; with it, current;
- one undo puts 101/9's ring back.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402
import parcel_register_cases as pc  # noqa: E402  (the drawing only)

X0, Y0 = 500000.0, 4400000.0


def doc_of(name: str) -> cad.ParcelDocumentInput:
    return cad.ParcelDocumentInput.from_json({"name": name})


def made(name: str) -> cad.ParcelToRecord:
    return cad.ParcelToRecord.from_json({"uid": pc.U(name)})


class SubdivisionSplitsNeighbour(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(pc.setup())))
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"), create=[made("A1"), made("A2"), made("A3")])

    def register(self) -> dict:
        _, saved = kcad.read(self.doc.to_bytes())
        return saved["settings"]["parcelRegister"]

    def walk(self, number: int) -> list[tuple[float, float]]:
        """A parcel's first ring's corners as the ring runs its boundaries (each boundary's start that way round)."""
        reg = self.register()
        by = {b["number"]: b for b in reg["boundaries"]}
        p = next(p for p in reg["parcels"] if p["number"] == number)
        out = []
        for e in p["rings"][0]["edges"]:
            b = by[abs(e)]
            start = b["from"] if e > 0 else b["to"]
            out.append((round(start["x"] - X0, 9), round(start["y"] - Y0, 9)))
        return out

    def test_the_neighbour_takes_the_new_corner(self) -> None:
        old = self.walk(3)
        self.assertEqual(len(old), 4)
        out = cad.parcels.record(self.doc, document=doc_of("2026/145 ifraz"), create=[made("A11"), made("A12")],
                                 close=[1]).to_json()
        self.assertEqual((out["boundariesSplit"], out["neighbours"]), (1, [3]))
        walked = self.walk(3)
        self.assertEqual(walked[:3], [(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)])
        self.assertEqual(len(walked), 5)
        reg = self.register()
        north = next(b for b in reg["boundaries"]
                     if (b["from"]["x"] - X0, b["from"]["y"] - Y0, b["to"]["x"] - X0, b["to"]["y"] - Y0) == (0, 10, 20, 10))
        self.assertEqual(north.get("closedBy"), 2)
        rows = {p["number"]: p for p in cad.parcels.records(self.doc).to_json()["parcels"]}
        self.assertEqual(rows[3]["neighbours"], [4, 5])
        self.assertEqual(rows[3]["state"], "changed", "101/9's area is drawn without (10, 10)")

    def test_with_the_corner_drawn_the_neighbour_is_current(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("2026/145 ifraz"), create=[made("A11"), made("A12")], close=[1])
        # 101/9's south edge (0, 10)–(20, 10) given a vertex at its middle: (10, 10).
        cad.entities.vertex(self.doc, uid=pc.U("A3"), action="insert",
                            place=cad.SubPlace.from_json({"kind": "mid", "part": 0, "ring": 0, "index": 0}))
        rows = {p["number"]: p for p in cad.parcels.records(self.doc).to_json()["parcels"]}
        self.assertEqual(rows[3]["state"], "current")

    def test_one_undo_puts_the_ring_back(self) -> None:
        before = self.walk(3)
        cad.parcels.record(self.doc, document=doc_of("2026/145 ifraz"), create=[made("A11"), made("A12")], close=[1])
        self.doc.undo()
        self.assertEqual(self.walk(3), before)


if __name__ == "__main__":
    unittest.main()
