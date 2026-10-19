"""CSV tables out and in from Python (plan/gis.md CBS-05, docs/adr/0522).

QGIS's GeoPackage (fixtures/geopackage/v1/qgis.gpkg) taken in, written out as one CSV with its geometry as WKT and its
`.csvt` and `.prj` beside it, read back here with the standard library's `csv`, then taken into a new drawing: the
same commands the desktop runs, their answers typed. A CSV of points comes in through its X and Y columns.
"""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed
from kentos.cad.types import CsvGeometry, CsvSeparator, XyCsvGeometryIn

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/geopackage/v1/qgis.gpkg"
POINTS = ROOT / "fixtures/csv/v1/nokta.csv"


class CsvTests(unittest.TestCase):
    def test_layers_go_out_as_one_csv_and_come_back(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = cad.Document.new("Ada 101", srid=5256)
            cad.file.import_.gpkg(doc, path=str(SOURCE))
            out = Path(tmp) / "teslim.csv"
            planned = cad.file.export.csv.plan(doc, path=str(out), separator=CsvSeparator.SEMICOLON)
            self.assertFalse(out.exists(), "plan writes nothing")
            written = cad.file.export.csv(doc, path=str(out), separator=CsvSeparator.SEMICOLON)
            self.assertEqual(written.rows, planned.rows)
            self.assertEqual(written.columns[:2], ["WKT", "katman"])
            self.assertEqual(sorted(f.name for f in written.files), ["teslim.csv", "teslim.csvt", "teslim.prj"])
            with out.open(newline="", encoding="utf-8") as f:
                rows = list(csv.reader(f, delimiter=";"))
            self.assertEqual(rows[0], written.columns)
            self.assertEqual(len(rows) - 1, written.rows)
            self.assertTrue(any(r[0].startswith("MULTIPOLYGON") or r[0].startswith("POLYGON") for r in rows[1:]))
            self.assertTrue(any("Işık-Ğüşöç" in c for r in rows for c in r))
            types = out.with_suffix(".csvt").read_text().strip().split(",")
            self.assertEqual(types[0], '"WKT"')
            # Written over only when asked.
            with self.assertRaises(CommandFailed) as refused:
                cad.file.export.csv(doc, path=str(out))
            self.assertEqual(refused.exception.code, "file_exists")
            # The CSV back into a new drawing: as many objects and records as rows.
            back = cad.Document.new("Geri", srid=5256)
            took = cad.file.import_.csv(back, path=str(out))
            self.assertEqual(took.objects, written.rows)

    def test_points_come_in_through_their_x_and_y_columns(self) -> None:
        doc = cad.Document.new("Noktalar", srid=5256)
        took = cad.file.import_.csv(doc, path=str(POINTS), geometry=XyCsvGeometryIn(x="X", y="Y", z="Z"))
        self.assertEqual(took.objects, 1)
        self.assertEqual(CsvGeometry.WKT.value, "wkt")


if __name__ == "__main__":
    unittest.main()
