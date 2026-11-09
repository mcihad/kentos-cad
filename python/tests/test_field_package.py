"""`cad.field.package` (Saha paketi, docs/adr/1920; plan/map-cad.md MAP-19) read back by an independent reader.

The drawing is scripts/fixtures/asbuilt_cases.py's. KentOS writes the package; Python's own `zipfile`, `csv` and `json`
read it, no KentOS code. What the files hold is held against the drawing as the generator writes it:

- the files are the three, in order, and nothing else; a surface given, its `yuzey.xml` fourth (docs/adr/3220);
- noktalar.csv: `Ad;Y;X;Z;Kimlik`, a row per design point in the drawing's order; Y the east, X the north; every
  number reads back exactly as the drawing's (nothing rounded); a point without a height leaves Z empty;
- cizgiler.csv: `Kimlik;Sıra;Y;X;Bulge`, each line's vertices in order;
- saha-paketi.json: the format and version, the design's digest (asbuilt_cases.py's rule), the system, the limits given
  and no other, the notes, the surface's id and name, the files;
- yuzey.xml (docs/adr/3220), read with Python's `xml.etree`: a LandXML 1.2 TIN in TUREF / TM36 (EPSG 5256) whose
  points are the drawing's 121 surface points exactly (northing, easting, height), whose 200 faces each name three of
  them and cover the grid: each face's plan area is half a 10 m square, together 100 m × 100 m, every face's corners
  on G; its SHA-256 and the counts are the manifest's and the answer's;
- the answer's size and SHA-256 are the file's;
- the epoch and the heights' system: none (null) where the project says none, the project's where it says them
  (TUREF at 2005.0; TUDKA-99's orthometric heights by TG-20, EPSG:5775's name as EPSG's registry has it).

A package over an existing one without `overwrite` is refused and leaves the file as it was.
"""

from __future__ import annotations

import csv
import hashlib
import io
import json
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402

import asbuilt_cases as asb  # noqa: E402
import kcad_v2_reference as kr  # noqa: E402

NOTES = ["Kontrol noktaları KN1 ve KN2'den başlanır.", "Kotlar TUDKA-99'a göredir."]
FILES = ["saha-paketi.json", "noktalar.csv", "cizgiler.csv"]
SURFACE = "yuzey.xml"
LX = "{http://www.landxml.org/schema/LandXML-1.2}"


def opened() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(asb.setup())))


def rows(data: bytes) -> list[list[str]]:
    return list(csv.reader(io.StringIO(data.decode("utf-8")), delimiter=";"))


class FieldPackageReadBack(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.path = Path(self.dir.name) / "saha.zip"

    def tearDown(self) -> None:
        self.dir.cleanup()

    def write(self, doc: cad.Document, **over):
        asked = dict(design="tasarim", lines="bordur", surface="Tasarım", horizontal=0.02, quality=0.05, notes=NOTES,
                     path=str(self.path), stamp="2026-11-07T10:00:00")
        asked.update(over)
        return cad.field.package(doc, **asked)

    def test_the_files_hold_the_design_as_drawn(self) -> None:
        r = self.write(opened())
        data = self.path.read_bytes()
        self.assertEqual(r.bytes, len(data))
        self.assertEqual(r.sha256, hashlib.sha256(data).hexdigest())
        with zipfile.ZipFile(self.path) as z:
            self.assertEqual(z.namelist(), FILES + [SURFACE])
            self.assertIsNone(z.testzip())
            manifest = json.loads(z.read(FILES[0]))
            points, lines = rows(z.read(FILES[1])), rows(z.read(FILES[2]))
        design, line_ids, _ = asb.ids()

        # The points: name, east, north, height, id; the numbers exactly the drawing's.
        self.assertEqual(points[0], ["Ad", "Y", "X", "Z", "Kimlik"])
        self.assertEqual(len(points) - 1, len(asb.DESIGN))
        for row, (name, x, y, z), uid in zip(points[1:], asb.DESIGN, design):
            self.assertEqual(row[0], name)
            self.assertEqual(float(row[1]), asb.X0 + x, name)
            self.assertEqual(float(row[2]), asb.Y0 + y, name)
            if z is None:
                self.assertEqual(row[3], "", name)
            else:
                self.assertEqual(float(row[3]), z, name)
            self.assertEqual(row[4], uid)

        # The lines: each vertex in order, no bulge.
        self.assertEqual(lines[0], ["Kimlik", "Sıra", "Y", "X", "Bulge"])
        want = [(uid, k + 1, asb.X0 + x, asb.Y0 + y) for uid, (_, pts) in zip(line_ids, asb.LINES)
                for k, (x, y) in enumerate(pts)]
        got = [(row[0], int(row[1]), float(row[2]), float(row[3])) for row in lines[1:]]
        self.assertEqual(got, want)
        self.assertTrue(all(float(row[4]) == 0.0 for row in lines[1:]))

        # The manifest: the digest by the check's rule, the limits given and no other.
        self.assertEqual((manifest["format"], manifest["version"]), ("kentos.field-package", 1))
        self.assertEqual(manifest["designDigest"], asb.digest())
        self.assertEqual(manifest["designDigest"], r.digest)
        self.assertEqual(manifest["system"], {"epsg": "EPSG:5256", "name": "TUREF / TM36"})
        self.assertEqual(manifest["acceptance"], {"horizontal": 0.02, "quality": 0.05})
        self.assertEqual(manifest["notes"], NOTES)
        self.assertEqual((manifest["surface"]["id"], manifest["surface"]["name"]), ("t-1", "Tasarım"))
        self.assertEqual(manifest["files"], FILES + [SURFACE])
        self.assertEqual((manifest["points"], manifest["lines"]), (len(asb.DESIGN), len(asb.LINES)))
        self.assertEqual(manifest["stamp"], "2026-11-07T10:00:00")
        self.assertEqual(manifest["revision"], r.revision)
        self.assertIsNone(manifest["epoch"])
        self.assertIsNone(manifest["heights"])

    def test_the_surface_is_carried_as_a_landxml_tin(self) -> None:
        r = self.write(opened())
        with zipfile.ZipFile(self.path) as z:
            manifest = json.loads(z.read(FILES[0]))
            data = z.read(SURFACE)
        surface = manifest["surface"]
        self.assertEqual(surface["file"], SURFACE)
        self.assertEqual(surface["sha256"], hashlib.sha256(data).hexdigest())
        self.assertEqual((r.surface.file, r.surface.sha256), (SURFACE, surface["sha256"]))
        root = ET.fromstring(data)
        self.assertEqual(root.tag, LX + "LandXML")
        self.assertEqual(root.find(LX + "CoordinateSystem").get("epsgCode"), "5256")
        tin = root.find(f"{LX}Surfaces/{LX}Surface")
        self.assertEqual(tin.get("name"), "Tasarım")
        self.assertEqual(tin.find(LX + "Definition").get("surfType"), "TIN")
        points = {}
        for p in tin.iter(LX + "P"):
            north, east, z = (float(v) for v in p.text.split())
            points[p.get("id")] = (east, north, z)
        # The drawing's surface points, exactly: the 11 × 11 grid on G.
        want = sorted((asb.X0 + 10.0 * i, asb.Y0 + 10.0 * j, asb.G(10.0 * i, 10.0 * j)) for i in range(11) for j in range(11))
        self.assertEqual(sorted(points.values()), want)
        faces = [tuple(points[k] for k in f.text.split()) for f in tin.iter(LX + "F")]
        self.assertEqual((len(points), len(faces)), (121, 200))
        self.assertEqual((surface["vertices"], surface["triangles"]), (121, 200))
        self.assertEqual((r.surface.vertices, r.surface.triangles), (121, 200))
        total = 0.0
        for a, b, c in faces:
            area = abs((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0
            self.assertAlmostEqual(area, 50.0, places=6)
            total += area
        self.assertAlmostEqual(total, 100.0 * 100.0, places=4)

    def test_no_surface_no_surface_file(self) -> None:
        r = self.write(opened(), surface=None)
        with zipfile.ZipFile(self.path) as z:
            self.assertEqual(z.namelist(), FILES)
            self.assertIsNone(json.loads(z.read(FILES[0]))["surface"])
        self.assertIs(r.surface, cad.UNSET)

    def test_the_epoch_and_heights_go_in_when_the_project_says_them(self) -> None:
        setup = asb.setup()
        setup["settings"]["coordinateEpoch"] = 2005.0
        setup["settings"]["heights"] = {"kind": "orthometric", "verticalSrid": 5775, "geoid": {"name": "TG-20"}}
        r = self.write(cad.Document.from_bytes(kr.container(kr.document(setup))))
        with zipfile.ZipFile(self.path) as z:
            manifest = json.loads(z.read(FILES[0]))
        self.assertEqual(manifest["epoch"], 2005.0)
        self.assertEqual(manifest["heights"], "Ortometrik yükseklik (Antalya height; geoit TG-20)")
        self.assertEqual((r.epoch, r.heights), (manifest["epoch"], manifest["heights"]))

    def test_an_existing_package_is_kept_without_overwrite(self) -> None:
        doc = opened()
        self.write(doc)
        before = self.path.read_bytes()
        with self.assertRaises(CommandFailed) as refused:
            self.write(doc, notes=["başka"])
        self.assertEqual(refused.exception.code, "file_exists")
        self.assertEqual(self.path.read_bytes(), before)
        again = self.write(doc, notes=["başka"], overwrite=True)
        with zipfile.ZipFile(self.path) as z:
            self.assertEqual(json.loads(z.read(FILES[0]))["notes"], ["başka"])
        self.assertEqual(again.sha256, hashlib.sha256(self.path.read_bytes()).hexdigest())
        self.assertFalse(Path(str(self.path) + ".part").exists())
        self.assertFalse(self.path.with_suffix(".zip.part").exists())


if __name__ == "__main__":
    unittest.main()
