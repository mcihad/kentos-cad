"""`cad.readjustment.sheets` (Ayırma çapı paftaları, docs/adr/2680; plan/map-cad.md MAP-11) held to what must be true of
any answer, with Python's own `decimal`, no KentOS code. The drawing is scripts/fixtures/readjustment_scope_cases.py's:

- a sheet's table of areas keeps registered = the part inside + what the part outside keeps, exactly (Article 31(2):
  the area error stays outside), and its difference is the written computed area less the record, exactly;
- the parts' areas on a side add up to that side's row within half a unit of the last place for each part rounded;
- every number a part's boundary names is one of the sheet's points (1 … points), each ring closed on its first;
- asked again, the sheets there already are said and nothing is written.
"""

from __future__ import annotations

import sys
import unittest
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import readjustment_scope_cases as rs  # noqa: E402

PARTIAL = [rs.uid(4), rs.uid(7), rs.uid(11)]  # 101/3, 101/6, 101/10


def opened() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(rs.setup(rs.MAIN))))


def scope(**kw) -> cad.ReadjustmentScope:
    return cad.ReadjustmentScope(boundary=rs.uid(1), scale=kw.get("scale", 2), rounding="half_even", uids=PARTIAL)


class SheetsHold(unittest.TestCase):
    def test_the_tables_keep_the_record(self) -> None:
        made = cad.readjustment.sheets(opened(), scope=scope())
        self.assertEqual([s.name for s in made.sheets], ["Ayırma çapı 101/3", "Ayırma çapı 101/6", "Ayırma çapı 101/10"])
        for s in made.sheets:
            rows = dict(s.areas)
            if rows["Girmeyen kısma kalan (tescilli − giren)"] != "—":
                self.assertEqual(Decimal(rows["Giren kısım"]) + Decimal(rows["Girmeyen kısma kalan (tescilli − giren)"]),
                                 Decimal(rows["Tescilli yüzölçümü"]), s.parcel)
            if rows["Yüzölçümü farkı"] != "—":
                self.assertEqual(Decimal(rows["Hesaplanan yüzölçümü"]) - Decimal(rows["Tescilli yüzölçümü"]),
                                 Decimal(rows["Yüzölçümü farkı"]), s.parcel)
            for side, row in (("Giren", "Giren kısım"), ("Girmeyen", "Girmeyen kısım")):
                parts = [Decimal(p[2]) for p in s.parts if p[0].startswith(side)]
                self.assertLessEqual(abs(sum(parts, Decimal(0)) - Decimal(rows[row])), Decimal("0.005") * len(parts),
                                     f"{s.parcel} {side}")
            for name, boundary, _ in s.parts:
                for ring in boundary.replace(")", "").split(" (delik "):
                    numbers = [int(n) for n in ring.split("-")]
                    self.assertEqual(numbers[0], numbers[-1], f"{s.parcel} {name} closed")
                    self.assertTrue(all(1 <= n <= s.points for n in numbers), f"{s.parcel} {name}: {numbers}")

    def test_asked_again_nothing_is_written(self) -> None:
        doc = opened()
        cad.readjustment.sheets(doc, scope=scope())
        before = doc.to_bytes()
        with self.assertRaises(CommandFailed) as refused:
            cad.readjustment.sheets(doc, scope=scope())
        self.assertEqual(refused.exception.code, "separation_sheet_exists")
        self.assertEqual(doc.to_bytes(), before)


if __name__ == "__main__":
    unittest.main()
