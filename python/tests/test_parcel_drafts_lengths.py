"""Belge taslağı, onayı, kenar ölçüleri ve yaylı sınırın bölünmesi (`cad.parcels.record`, `cad.parcels.approve`,
`cad.parcels.withdraw`, `cad.parcels.records`; docs/adr/5020, plan/map-cad.md MAP-08), held by the test's own arithmetic
and the independent file reader (no KentOS code in the expectations).

The drawing is the shared cases' (scripts/fixtures/parcel_register_cases.py): 101/7 (A1, (0, 0)–(20, 10)), 101/8 east
of it, 101/9 north of it with an arc for its north edge (bulge 0.2 from (20, 20) to (0, 20)); 101/19 (A18) north of
101/9 shares that arc; 101/20 and 101/21 (A19, A20) divide 101/19 at the arc's middle (10, 22).

- A stated length is the document's: the south edge stated 20,08 m is 20 m drawn; BÖHHBÜY m. 48(1)'s limit is
  0.05 + 0.001·20.08 m, and the difference is past it.
- A draft changes nothing current; its approval writes it as the next document with the approval as given; a withdrawn
  draft frees its name.
- The arc's middle is on 101/9's circle (the test's own centre from the sagitta); 101/9's ring takes it as a corner.
"""

from __future__ import annotations

import math
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


def made(name: str, lengths: list[dict] | None = None) -> cad.ParcelToRecord:
    item = {"uid": pc.U(name)}
    if lengths is not None:
        item["lengths"] = lengths
    return cad.ParcelToRecord.from_json(item)


class ParcelDraftsAndLengths(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(pc.setup(ents=pc.MORE_ENTS))))

    def register(self) -> dict:
        _, saved = kcad.read(self.doc.to_bytes())
        return saved["settings"].get("parcelRegister")

    def test_a_stated_length_is_checked_against_the_frontage_limit(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"),
                           create=[made("A1", [{"edge": 0, "length": "20,08"}, {"edge": 1, "length": "10,01"}])])
        reg = self.register()
        self.assertEqual([(x["document"], x["length"]) for x in reg["lengths"]], [(1, "20,08"), (1, "10,01")])
        rows = cad.parcels.records(self.doc).to_json()
        south = next(b for b in rows["boundaries"] if b.get("lengths") and b["lengths"][0]["length"] == "20,08")
        drawn = math.hypot(south["to"][0] - south["from"][0], south["to"][1] - south["from"][1])
        stated = 20.08
        self.assertAlmostEqual(south["lengths"][0]["difference"], drawn - stated, places=9)
        self.assertAlmostEqual(south["lengths"][0]["limit"], 0.05 + 0.001 * stated, places=12)
        self.assertFalse(south["lengths"][0]["within"])
        self.assertEqual(rows["overLimit"], 1)

    def test_a_draft_changes_nothing_until_its_approval(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"), create=[made("A1"), made("A2"), made("A3")])
        before = self.register()
        out = cad.parcels.record(self.doc, document=doc_of("2026/145 ifraz"), create=[made("A11"), made("A12")],
                                 close=[1], draft=True).to_json()
        self.assertEqual((out["document"], out["draft"]), (1, True))
        reg = self.register()
        self.assertEqual([d["name"] for d in reg["drafts"]], ["2026/145 ifraz"])
        self.assertEqual((reg["documents"], reg["parcels"], reg["boundaries"]),
                         (before["documents"], before["parcels"], before["boundaries"]))
        approval = cad.ParcelApprovalInput.from_json({"reference": "2026/145-K", "by": "Kadastro Müdürlüğü"})
        approved = cad.parcels.approve(self.doc, draft=1, approval=approval).to_json()
        self.assertEqual(approved["document"], 2)
        reg = self.register()
        self.assertNotIn("drafts", reg)
        self.assertEqual(reg["documents"][1]["approval"], {"reference": "2026/145-K", "by": "Kadastro Müdürlüğü"})
        closed = next(p for p in reg["parcels"] if p["number"] == 1)
        self.assertEqual(closed.get("closedBy"), 2)

    def test_a_withdrawn_draft_frees_its_name(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"), create=[made("A1")])
        cad.parcels.record(self.doc, document=doc_of("2026/9 terkin"), close=[1], draft=True)
        self.assertEqual(cad.parcels.withdraw(self.doc, draft=1).to_json()["name"], "2026/9 terkin")
        self.assertNotIn("drafts", self.register())
        out = cad.parcels.record(self.doc, document=doc_of("2026/9 terkin"), close=[1]).to_json()
        self.assertEqual(out["closed"], [1])

    def test_an_arc_boundary_splits_at_its_middle(self) -> None:
        cad.parcels.record(self.doc, document=doc_of("Kadastro 1985"),
                           create=[made("A1"), made("A2"), made("A3"), made("A18")])
        out = cad.parcels.record(self.doc, document=doc_of("2026/150 ifraz"), create=[made("A19"), made("A20")],
                                 close=[4]).to_json()
        self.assertEqual((out["boundariesSplit"], out["neighbours"]), (1, [3]))
        # The test's own circle: chord 20 m, sagitta 0.2 · 20 / 2 = 2 m, r = (c²/4 + s²) / (2s) = 26 m, centre 24 m below
        # the chord's middle (10, 20).
        c, s = 20.0, 2.0
        r = (c * c / 4 + s * s) / (2 * s)
        centre = (10.0, 20.0 + s - r)
        self.assertAlmostEqual(math.hypot(10.0 - centre[0], 22.0 - centre[1]), r, places=9)
        reg = self.register()
        by = {b["number"]: b for b in reg["boundaries"]}
        p = next(p for p in reg["parcels"] if p["number"] == 3)
        corners = []
        for e in p["rings"][0]["edges"]:
            b = by[abs(e)]
            start = b["from"] if e > 0 else b["to"]
            corners.append((round(start["x"] - X0, 9), round(start["y"] - Y0, 9)))
        self.assertIn((10.0, 22.0), corners)
        self.assertEqual(len(corners), 5)


if __name__ == "__main__":
    unittest.main()
