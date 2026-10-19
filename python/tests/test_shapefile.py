"""Shapefiles out from Python (plan/gis.md CBS-05, docs/adr/0514).

QGIS's GeoPackage (fixtures/geopackage/v1/qgis.gpkg) taken in and written out as Shapefiles in a Windows-1254 archive:
the same command the desktop runs, its answer typed. Read back here with the standard library only: the archive's
files, the .dbf's code page byte, field names and the Turkish values in Windows-1254.
"""

from __future__ import annotations

import struct
import tempfile
import unittest
import zipfile
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed, CommandWarning
from kentos.cad._runtime import UNSET
from kentos.cad.types import ShpEncoding

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/geopackage/v1/qgis.gpkg"


def dbf_fields(b: bytes) -> list[tuple[str, str, int, int]]:
    """A dBASE III table's fields: name, type, width, fraction digits."""
    header_len = struct.unpack("<H", b[8:10])[0]
    out = []
    for at in range(32, header_len - 1, 32):
        d = b[at:at + 32]
        if d[0] == 0x0D:
            break
        name = d[:11].split(b"\0")[0].decode("cp1254")
        out.append((name, chr(d[11]), d[16], d[17]))
    return out


class ShapefileTests(unittest.TestCase):
    def test_layers_go_out_as_a_windows_1254_archive(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = cad.Document.new("Ada 101", srid=5256)
            cad.file.import_.gpkg(doc, path=str(SOURCE))
            out = Path(tmp) / "teslim.zip"
            planned = cad.file.export.shp.plan(doc, path=str(out), encoding=ShpEncoding.WINDOWS_1254)
            self.assertFalse(out.exists(), "plan writes nothing")
            # A date-time is written as text, and an empty text is said: the warning names what the format lacks.
            with self.assertWarns(CommandWarning) as said:
                written = cad.file.export.shp(doc, path=str(out), encoding=ShpEncoding.WINDOWS_1254)
            self.assertIn("tarih-saat", str(said.warning))
            self.assertEqual(written.into, "zip")
            self.assertEqual([s.name for s in written.sets], [s.name for s in planned.sets])
            names = {s.name: s for s in written.sets}
            self.assertEqual(names["parsel"].shape, "POLYGON")
            self.assertEqual(names["yol"].shape, "POLYLINEZ")
            self.assertIn(names["malik"].shape, (None, UNSET), "a table alone")
            with zipfile.ZipFile(out) as z:
                dbf = z.read("parsel.dbf")
                self.assertEqual(z.read("parsel.cpg"), b"1254")
                self.assertIn("PROJCS[\"TUREF_TM36\"", z.read("parsel.prj").decode())
                self.assertNotIn("malik.shp", z.namelist())
            self.assertEqual(dbf[29], 0xCA, "Turkish Windows' language driver")
            fields = dbf_fields(dbf)
            self.assertEqual([f[0] for f in fields],
                             ["ada", "parsel_no", "alan", "kimlik", "tescil", "guncelleme", "tapulu", "not"])
            kinds = {f[0]: (f[1], f[3]) for f in fields}
            self.assertEqual(kinds["alan"], ("N", 2))
            self.assertEqual(kinds["tescil"][0], "D")
            self.assertEqual(kinds["tapulu"][0], "L")
            self.assertEqual(kinds["kimlik"][0], "C", "past 2⁵³: text, every digit kept")
            self.assertIn("Işık-Ğüşöç".encode("cp1254"), dbf)
            # Written over only when asked.
            with self.assertRaises(CommandFailed) as refused:
                cad.file.export.shp(doc, path=str(out))
            self.assertEqual(refused.exception.code, "file_exists")


if __name__ == "__main__":
    unittest.main()
