"""A Shapefile, a GeoJSON file and a KML taken into a drawing from Python (plan/gis.md CBS-05, docs/adr/0563).

The same commands the desktop's windows run, their answers typed: the file's system is a question the input answers
(`srid`, `reproject`) and the answer says how the coordinates were moved and how sure that is; each import is one undo
step named after the format and the file.
"""

from __future__ import annotations

import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
FILES = ROOT / "fixtures/gis-import/v1"


def path(name: str) -> str:
    return str(FILES / name)


class GisImportTests(unittest.TestCase):
    def test_a_shapefile_archive_goes_in_one_layer_at_a_time_or_all_at_once(self) -> None:
        doc = cad.Document.new("Ada 101", srid=5256)
        planned = cad.file.import_.shp.plan(doc, path=path("katmanlar.zip"))
        self.assertEqual(len(list(doc.entities())), 0, "plan writes nothing")
        took = cad.file.import_.shp(doc, path=path("katmanlar.zip"))
        self.assertEqual(took.objects, planned.objects)
        self.assertEqual(took.source_layers, ["parsel", "yol", "nokta"])
        self.assertEqual(took.srid, 5256)
        self.assertIs(took.moved, cad.UNSET)
        self.assertEqual(took.declared.source.value, "prj")
        self.assertEqual(doc.undo(), "Shapefile: katmanlar.zip")
        # One layer of the archive, by its path, onto a layer of the drawing.
        layer = doc.layers()[0].id
        one = cad.file.import_.shp(doc, path=path("katmanlar.zip"), entry="veri/nokta", layer=layer)
        self.assertEqual(one.source_layers, ["nokta"])
        self.assertEqual(one.layers, [])
        with self.assertRaises(CommandFailed) as refused:
            cad.file.import_.shp(doc, path=path("katmanlar.zip"), entry="yok")
        self.assertEqual(refused.exception.code, "entry_not_found")

    def test_a_file_in_wgs84_is_a_question_and_the_answer_says_how_it_was_moved(self) -> None:
        doc = cad.Document.new("Ada 101", srid=5256)
        with self.assertRaises(CommandFailed) as refused:
            cad.file.import_.geojson(doc, path=path("parsel-rfc7946.geojson"))
        self.assertEqual(refused.exception.code, "crs_mismatch")
        self.assertEqual(refused.exception.path, "reproject")
        took = cad.file.import_.geojson(doc, path=path("parsel-rfc7946.geojson"), reproject=True)
        self.assertEqual(took.declared.srid, 4326)
        moved = took.moved
        self.assertIsNotNone(moved)
        assert moved is not None
        self.assertEqual((moved.from_, moved.to, moved.moved, moved.left), (4326, 5256, 2, 0))
        self.assertEqual(moved.text, "±1 m, EPSG:5261")
        self.assertFalse(moved.unofficial)
        # The same file taken to be in the project's system, whatever it says: said, and not moved.
        other = cad.Document.new("Ada 102", srid=5256)
        said = cad.file.import_.geojson(other, path=path("parsel-rfc7946.geojson"), srid=5256)
        self.assertIs(said.moved, cad.UNSET)

    def test_a_kml_goes_in_with_its_folders_as_layers(self) -> None:
        doc = cad.Document.new("Ada 101", srid=5256)
        took = cad.file.import_.kml(doc, path=str(ROOT / "fixtures/formats/v1/kml/parsel.kml"), reproject=True)
        self.assertEqual(sorted(took.source_layers), ["Kadastro", "Saha"])
        self.assertEqual(took.declared.source.value, "kml")
        self.assertEqual(took.moved.from_, 4326)
        self.assertEqual(doc.undo(), "KML: parsel.kml")

    def test_a_project_without_a_system_takes_the_coordinates_as_they_are(self) -> None:
        doc = cad.Document.new("Yerel", workspace="cad", coords="local")
        took = cad.file.import_.geojson(doc, path=path("nokta-rfc7946.geojson"))
        self.assertIs(took.moved, cad.UNSET)
        self.assertEqual(took.srid, 4326)
        with self.assertRaises(CommandFailed) as refused:
            cad.file.import_.geojson(doc, path=path("nokta-rfc7946.geojson"), reproject=True)
        self.assertEqual(refused.exception.code, "crs_mismatch")


if __name__ == "__main__":
    unittest.main()
