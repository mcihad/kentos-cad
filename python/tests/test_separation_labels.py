"""The numbered pieces outside on the drawing (`cad.readjustment.sheets`, docs/adr/3480; plan/map-cad.md MAP-11) through
the SDK, held by scripts/fixtures/readjustment_sheets_cases.py's own reading (no KentOS code in the expectations).

- With the three parcels' outside pieces numbered, each sheet answers the reading's labels (the piece's name, its
  number, where it stands).
- The drawing then has each piece as an area with its parcel, name and number, and a point labelled with the number
  inside it, on the parcel's layer of numbered pieces; the points' layer keeps only the numbered corners.
- Without numbers no piece is written.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import readjustment_scope_cases as rs  # noqa: E402
import readjustment_sheets_cases as sc  # noqa: E402

P = rs.P_
THREE = [P(3), P(6), P(10)]
NUMBERS = {rs.uid(P(3) + 1): ["101/30"], rs.uid(P(6) + 1): ["101/31"], rs.uid(P(10) + 1): ["101/24", "101/25"]}


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(rs.setup(rs.MAIN))))


class SeparationLabels(unittest.TestCase):
    def test_the_numbered_pieces_are_written_with_their_numbers_inside(self) -> None:
        doc = drawing()
        scope = cad.ReadjustmentScope.from_json(sc.scope_input(THREE))
        out = cad.readjustment.sheets(doc, scope=scope, numbers=NUMBERS)
        rows, _ = sc.sheets(rs.MAIN, THREE, {}, numbers=NUMBERS)
        for sheet, want in zip(out.sheets, rows):
            got = [(l.part, l.number, l.at[0], l.at[1]) for l in sheet.labels]
            self.assertEqual([(g[0], g[1]) for g in got], [(w["part"], w["number"]) for w in want["labels"]])
            for g, w in zip(got, want["labels"]):
                self.assertAlmostEqual(g[2], w["at"][0], places=9)
                self.assertAlmostEqual(g[3], w["at"][1], places=9)
        records = [doc.entity(u).entity.to_json() for u in out.uids]
        areas = [r for r in records if r["kind"] == "polygon"]
        self.assertEqual(sorted(r["attrs"]["ParselNo"] for r in areas), ["101/24", "101/25", "101/30", "101/31"])
        labelled = [r for r in records if r["kind"] == "point" and r.get("label", "").startswith("101/")]
        self.assertEqual(len(labelled), 4)
        corners = [r for r in records if r["kind"] == "point" and "Nokta" in r["attrs"]]
        self.assertTrue(all(r["layerId"] != a["layerId"] for r in corners for a in areas), "pieces on their own layer")

    def test_without_numbers_no_piece_is_written(self) -> None:
        doc = drawing()
        out = cad.readjustment.sheets(doc, scope=cad.ReadjustmentScope.from_json(sc.scope_input(THREE)))
        records = [doc.entity(u).entity.to_json() for u in out.uids]
        self.assertFalse(any(r["kind"] == "polygon" for r in records))
        self.assertTrue(all(sheet.labels is cad.UNSET or not sheet.labels for sheet in out.sheets))


if __name__ == "__main__":
    unittest.main()
