"""A GeoPackage opened as a project from Python (docs/adr/9116): a file of no project opens as a new project of its
tables, written into the file, and ``save()`` writes there again; opened again it gives that project; a file of
several projects names them; ``open_from`` opens one by name. The files are fixtures/data/v1/gpkg-open's, written by
scripts/fixtures/gpkg_open_cases.py with sqlite3."""

from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

CASES = Path(__file__).resolve().parents[2] / "fixtures" / "data" / "v1" / "gpkg-open"


def copy(name: str, folder: str) -> Path:
    to = Path(folder) / name
    shutil.copyfile(CASES / name, to)
    return to


class GeoPackageOpened(unittest.TestCase):
    def test_a_file_of_no_project_opens_as_one_written_into_it(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            gpkg = copy("katmanlar.gpkg", tmp)
            doc = cad.Document.open(gpkg)
            info = doc.info()
            self.assertEqual(info.settings.srid, 5256)
            self.assertEqual((info.project["name"], info.project["schema"]), ("katmanlar", None))
            self.assertEqual(Path(info.project["address"]).resolve(), gpkg.resolve())
            opened = info.opened_geopackage
            self.assertTrue(opened and opened["made"] and opened["written"], opened)
            self.assertEqual(opened["notes"], [])
            self.assertEqual(
                [layer.name for layer in doc.all_layers()], ["Taslak", "kuyular", "yollar", "binalar", "bos", "malikler"]
            )
            self.assertEqual([p.name for p in cad.projects(gpkg)], ["katmanlar"])
            # save() without a path goes into the file again.
            saved = doc.save()
            self.assertIsInstance(saved, cad.StoredSaved)
            self.assertEqual((saved.name, saved.schema), ("katmanlar", None))
            # Opened again: the project it keeps now, nothing made.
            again = cad.Document.open(gpkg)
            self.assertEqual(again.info().opened_geopackage["made"], False)
            self.assertEqual(len(list(again.all_layers())), 6)
            self.assertEqual([p.name for p in cad.projects(gpkg)], ["katmanlar"])

    def test_a_file_without_a_system_says_so(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = cad.Document.open(copy("yabanci-sistem.gpkg", tmp))
            self.assertEqual(doc.info().settings.srid, 0)
            notes = doc.info().opened_geopackage["notes"]
            self.assertEqual([n["code"] for n in notes], ["system_not_listed"])
            self.assertIn("EPSG:2154", notes[0]["text"])

    def test_a_file_of_several_projects_names_them_and_one_opens_by_name(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            gpkg = copy("iki-projeli.gpkg", tmp)
            with self.assertRaises(cad.HostError) as several:
                cad.Document.open(gpkg)
            self.assertEqual(several.exception.code, "several_projects")
            self.assertIn("“Çamlık”, “Zeytinlik”", str(several.exception))
            doc = cad.Document.open_from(gpkg, "Zeytinlik")
            self.assertEqual(doc.info().project["name"], "Zeytinlik")

    def test_a_file_that_is_not_a_geopackage_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(cad.HostError) as refused:
                cad.Document.open(copy("gpkg-degil.gpkg", tmp))
            self.assertIn("GeoPackage değil", str(refused.exception))


if __name__ == "__main__":
    unittest.main()
