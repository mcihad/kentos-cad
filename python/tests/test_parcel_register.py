"""Parsel kayıtları (`cad.parcels.record`, `cad.parcels.records`; docs/adr/4220, plan/map-cad.md MAP-08), held by the
test's own computation (no KentOS code in the expectations).

The drawing is the shared cases' (scripts/fixtures/parcel_register_cases.py): Ada 101's 101/7 (A1, 20 × 10 m, its
“Tapu alanı (m²)” 200.00) and 101/8 (A2, east of it), which share the edge from (20, 0) to (20, 10); A10 is 101/7
measured again, 5 cm wider. MAP-08's acceptance, as a technical model (no legal claim):

- the same parcel's two measurements and documents are both kept: 101/7 by Kadastro 1985 (200.00, 200 m²), then
  closed and made again from its re-measured area by 2026/77 yenileme (200,50, 200.5 m²);
- a corner moved in the drawing changes the geometric area, not the document's stated area nor the recorded shape;
  recorded by a new document, it changes the neighbours too: the moved boundary is no longer the one 101/8 uses;
- the saved file carries the register at schema 216, read back by the independent reader;
- one undo takes a document back.
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


def shoelace(places: list[tuple[float, float]]) -> float:
    """A straight ring's area, from its first vertex."""
    ox, oy = places[0]
    rel = [(x - ox, y - oy) for x, y in places]
    return abs(sum(rel[k][0] * rel[(k + 1) % len(rel)][1] - rel[(k + 1) % len(rel)][0] * rel[k][1]
                   for k in range(len(rel)))) / 2


def doc_of(name: str, **more: str) -> cad.ParcelDocumentInput:
    return cad.ParcelDocumentInput.from_json({"name": name, **more})


def made(name: str, stated: str | None = None) -> cad.ParcelToRecord:
    item = {"uid": pc.U(name)}
    if stated is not None:
        item["statedArea"] = stated
    return cad.ParcelToRecord.from_json(item)


class ParcelRegister(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(pc.setup())))

    def listed(self, history: bool = False) -> dict:
        return cad.parcels.records(self.doc, history=history).to_json()

    def test_two_measurements_and_documents_of_one_parcel_are_kept(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985", kind="Tapu planı"), create=[made("A1")])
        out = cad.parcels.record(self.doc, document=doc_of("2026/77 yenileme"), create=[made("A10")], close=[1])
        self.assertEqual((out.document, list(out.created), list(out.closed)), (2, [2], [1]))
        rows = {p["number"]: p for p in self.listed(history=True)["parcels"]}
        old, new = rows[1], rows[2]
        self.assertEqual((old["ada"], old["parsel"], new["ada"], new["parsel"]), ("101", "7", "101", "7"))
        self.assertEqual((old["state"], old["closedBy"], old["statedArea"]), ("closed", 2, "200.00"))
        self.assertEqual((new["state"], new["createdBy"], new["statedArea"]), ("current", 2, "200,50"))
        self.assertAlmostEqual(old["recordedArea"], shoelace([(0, 0), (20, 0), (20, 10), (0, 10)]), delta=1e-9)
        self.assertAlmostEqual(new["recordedArea"], shoelace([(0, 0), (20.05, 0), (20.05, 10), (0, 10)]), delta=1e-9)
        # The re-measured parcel follows the closed one through its document.
        self.assertEqual((new["before"], old["after"]), ([1], [2]))

    def test_a_moved_corner_changes_the_area_and_once_recorded_the_neighbours_not_the_document(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"), create=[made("A1"), made("A2")])
        before = {p["number"]: p for p in self.listed()["parcels"]}
        self.assertEqual((before[1]["neighbours"], before[2]["neighbours"]), ([2], [1]))
        # 101/7's north-east corner (20, 10) goes 0.4 m east and 0.2 m north in the drawing, 101/8 stays.
        window = cad.StretchWindow.from_json({"min": pc.P(19.9, 9.9), "max": pc.P(20.1, 10.1)})
        cad.entities.stretch(self.doc, window=window, dx=0.4, dy=0.2, uids=[pc.U("A1")])
        drawn = self.listed()
        p1 = {p["number"]: p for p in drawn["parcels"]}[1]
        self.assertEqual(p1["state"], "changed")
        self.assertAlmostEqual(p1["area"], shoelace([(0, 0), (20, 0), (20.4, 10.2), (0, 10)]), delta=1e-6)
        self.assertEqual((p1["statedArea"], p1["recordedArea"]), ("200.00", 200.0))
        self.assertEqual(p1["neighbours"], [2], "the register's neighbours stay until a document records the move")
        self.assertEqual(drawn["moved"], 2)
        # A new document records the move: the boundary 101/8 uses is no longer 101/7's.
        cad.parcels.record(self.doc, document=doc_of("2026/90 yenileme"), create=[made("A1", "203,10")], close=[1])
        after = {p["number"]: p for p in self.listed(history=True)["parcels"]}
        self.assertEqual((after[3]["neighbours"], after[2]["neighbours"]), ([], []))
        self.assertAlmostEqual(after[3]["recordedArea"], p1["area"], delta=1e-6)
        self.assertEqual((after[1]["statedArea"], after[1]["recordedArea"]), ("200.00", 200.0))

    def test_the_file_is_schema_216_and_keeps_the_register(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"), create=[made("A1"), made("A2")])
        data = self.doc.to_bytes()
        _, payload = kcad.read_header(data)
        self.assertEqual(kcad.decode_payload(payload)["version"], 216)
        _, saved = kcad.read(data)
        register = saved["settings"]["parcelRegister"]
        self.assertEqual([d["name"] for d in register["documents"]], ["Kadastro 1985"])
        # Seven boundaries: A1's four and A2's three more, the edge between them once, run west by A2.
        self.assertEqual(len(register["boundaries"]), 7)
        shared = [b for b in register["boundaries"]
                  if (b["from"]["x"] - X0, b["from"]["y"] - Y0, b["to"]["x"] - X0, b["to"]["y"] - Y0) == (20, 0, 20, 10)]
        self.assertEqual(len(shared), 1)
        n = shared[0]["number"]
        rings = [p["rings"][0]["edges"] for p in register["parcels"]]
        self.assertIn(n, rings[0])
        self.assertIn(-n, rings[1])

    def test_one_undo_takes_the_document_back(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"), create=[made("A1")])
        cad.parcels.record(self.doc, document=doc_of("2026/9 terkin"), close=[1])
        self.doc.undo()
        listed = self.listed(history=True)
        self.assertEqual([d["name"] for d in listed["documents"]], ["Kadastro 1985"])
        self.assertEqual(listed["parcels"][0]["state"], "current")


if __name__ == "__main__":
    unittest.main()
