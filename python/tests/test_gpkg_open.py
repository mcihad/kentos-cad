"""A GeoPackage opened as a project from Python (docs/adr/9116): a file of no project opens as a new project of its
tables, written into the file, and ``save()`` writes there again; opened again it gives that project; a file of
several projects names them; ``open_from`` opens one by name. Its tile tables open as rasters linked to the file,
found again where the file is carried, and ``save()`` gives a changed look to its table's default style
(docs/adr/9117). The files are fixtures/data/v1/gpkg-open's, written by scripts/fixtures/gpkg_open_cases.py with
sqlite3."""

from __future__ import annotations

import shutil
import sqlite3
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



def fill(color: str) -> dict:
    return {"fill": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": color},
                                                {"id": "o", "type": "simpleLine", "color": "#232323", "width": 0.5}]}}


class GeoPackageRastersAndLooks(unittest.TestCase):
    def test_a_tile_table_opens_as_a_raster_found_where_the_file_is_carried(self) -> None:
        with tempfile.TemporaryDirectory() as tmp, tempfile.TemporaryDirectory() as elsewhere:
            gpkg = copy("ortofotolu.gpkg", tmp)
            doc = cad.Document.open(gpkg)
            self.assertEqual([layer.name for layer in doc.all_layers()], ["Taslak", "parseller", "ortofoto"])
            raster = next(r.entity for r in doc.entities(kinds="raster"))
            self.assertEqual(Path(raster.file).resolve(), gpkg.resolve())
            self.assertEqual((raster.tiles.table, raster.tiles.zoom), ("ortofoto", 1))
            moved = Path(elsewhere) / gpkg.name
            shutil.copyfile(gpkg, moved)
            there = cad.Document.open(moved)
            raster = next(r.entity for r in there.entities(kinds="raster"))
            self.assertEqual(Path(raster.file).resolve(), moved.resolve(), "read from where the file is now")

    def test_save_gives_a_changed_look_to_its_tables_default_style_once(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            gpkg = copy("katmanlar.gpkg", tmp)
            doc = cad.Document.open(gpkg)
            binalar = doc.layer("binalar")
            doc.run("cad.layers.renderer", {"layer": binalar.id, "renderer": {
                "type": "categorized", "expr": "[kat]", "other": fill("#abd9e9"), "categories": [
                    {"value": "1", "label": "Bir katlı", "symbols": fill("#fdae61")},
                    {"value": "2", "label": "İki katlı", "symbols": fill("#d7191c")}]}})
            self.assertEqual(doc.save().looks, ("binalar",))
            self.assertEqual(doc.save().looks, (), "unchanged: nothing more")
            with sqlite3.connect(gpkg) as db:
                rows = db.execute("SELECT f_table_name, description FROM layer_styles WHERE useAsDefault").fetchall()
            self.assertIn(("binalar", "KentOS"), rows)

    def test_a_category_value_given_as_a_number_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = cad.Document.open(copy("katmanlar.gpkg", tmp))
            out = doc.run("cad.layers.renderer", {"layer": doc.layer("binalar").id, "renderer": {
                "type": "categorized", "expr": "[kat]", "categories": [
                    {"value": 1, "label": "Bir katlı", "symbols": fill("#fdae61")}]}})
            self.assertEqual((out["status"], out["error"]["code"]), ("failed", "invalid_renderer"))
            self.assertIn('1 yerine "1" yazın', out["error"]["message"])


if __name__ == "__main__":
    unittest.main()
