"""Wide values through a bound layer from Python (plan/gis.md CBS-06, docs/adr/0512).

fixtures/geopackage/v1/genis.gpkg (OGR wrote it; scripts/fixtures/wide_values_cases.py) holds an integer past 2⁵³ and
the smallest 64-bit one, 0 and false, a double with no short decimal, an empty text beside NULL, a time with its zone.
Python reads each as the same text the desktop shows, NULL as the attribute's absence, the integer column as a big integer
and the zoned time as a time stamp (docs/adr/0629 §4): nothing is lost. One text changed and written back leaves every
other cell of the file as it was, value and SQLite type.
"""

from __future__ import annotations

import shutil
import sqlite3
import tempfile
import unittest
import warnings
from pathlib import Path

import kentos.cad as cad
from kentos.cad.types import LinkSourcesEditOp
from kentos.cad import CommandWarning

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/geopackage/v1/genis.gpkg"
COLUMNS = "fid, buyuk, typeof(buyuk), sayi, typeof(sayi), evet, typeof(evet), ondalik, typeof(ondalik), " \
    "metin, typeof(metin), an, typeof(an)"


def cells(path: Path) -> list[tuple]:
    with sqlite3.connect(path) as c:
        return c.execute(f"SELECT {COLUMNS} FROM deger ORDER BY fid").fetchall()


class WideValuesTests(unittest.TestCase):
    def test_a_round_trip_keeps_every_cell(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "genis.gpkg"
            shutil.copy(SOURCE, path)
            doc = cad.Document.new("Geniş", srid=5256)
            # Nothing is lost: the integer past 2⁵³ is a big integer, the zoned time a time stamp (docs/adr/0629 §4).
            with warnings.catch_warnings():
                warnings.simplefilter("error", CommandWarning)
                linked = cad.sources.edit(doc, edit=LinkSourcesEditOp(path=str(path), table="deger"))
            self.assertEqual(linked.objects, 3)
            layer = linked.link.layer
            kinds = {f.name: f.kind for f in doc.layer(layer).fields}
            self.assertEqual(kinds["buyuk"], "bigint")
            self.assertEqual(kinds["an"], "instant")
            self.assertEqual(kinds["sayi"], "integer")
            rows = {r.entity.attrs.get("sayi"): r for r in doc.entities(layer=layer)}
            first, second, third = rows["0"], rows["7"], rows[None]
            self.assertEqual(first.entity.attrs["buyuk"], "9007199254740993")
            self.assertEqual(first.entity.attrs["metin"], "")
            self.assertEqual(first.entity.attrs["evet"], "false")
            self.assertEqual(first.entity.attrs["ondalik"], "0.1")
            self.assertEqual(first.entity.attrs["an"], "2026-03-29T00:30:00+03:00")
            self.assertEqual(second.entity.attrs["an"], "2026-10-25T03:30:00Z")
            self.assertEqual(second.entity.attrs["buyuk"], "-9223372036854775807")
            self.assertEqual(second.entity.attrs["ondalik"], "123456789.12345679")
            self.assertNotIn("metin", second.entity.attrs, "NULL is the attribute's absence")
            self.assertEqual(third.entity.attrs, {"metin": "ş\"'"})
            # Right after linking nothing differs, the zone the drawing does not keep included.
            self.assertEqual(cad.sources.sync.plan(doc, layer=layer, direction="write").modified, 0)
            before = cells(path)
            cad.entities.set(doc, uids=[third.uid], operation="attributes", attrs={"metin": "değişti"})
            self.assertEqual(cad.sources.sync(doc, layer=layer, direction="write").modified, 1)
            after = cells(path)
            want = [list(r) for r in before]
            want[2][9] = "değişti"
            self.assertEqual([list(r) for r in after], want)


if __name__ == "__main__":
    unittest.main()
