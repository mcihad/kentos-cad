"""A project kept in a data source with its data, from Python (docs/adr/9107): saved into a GeoPackage, listed,
opened again, written over; a missing project and a bad name refused. A PostgreSQL schema the same way when
``KENTOS_DATA_PG`` names a database (its password in ``PGPASSWORD``)."""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad


def drawing() -> cad.Document:
    return cad.Document.new("Ada 101", srid=5256)


class ProjectInAGeoPackage(unittest.TestCase):
    def test_saved_listed_opened_and_written_over(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            gpkg = Path(tmp) / "veri" / "sivas.gpkg"
            doc = drawing()
            self.assertEqual(cad.projects(gpkg), [], "nothing kept in a file that is not there")
            saved = doc.save_to(gpkg, "Sivas altyapı")
            self.assertTrue(gpkg.is_file(), "the GeoPackage made")
            self.assertEqual((saved.name, saved.schema), ("Sivas altyapı", None))
            self.assertGreater(saved.bytes, 0)
            kept = cad.projects(gpkg)
            self.assertEqual([p.name for p in kept], ["Sivas altyapı"])
            self.assertEqual(kept[0].bytes, saved.bytes)
            self.assertTrue(kept[0].modified and kept[0].modified.endswith("Z"))
            back = cad.Document.open_from(gpkg, "Sivas altyapı")
            self.assertEqual(len(list(back.all_layers())), len(list(doc.all_layers())))
            # A second project beside it, then the first written over: one entry each, ordered in Turkish.
            drawing().save_to(gpkg, "Çevre düzeni")
            doc.save_to(gpkg, "Sivas altyapı")
            self.assertEqual([p.name for p in cad.projects(gpkg)], ["Çevre düzeni", "Sivas altyapı"])

    def test_a_missing_project_and_a_bad_name_are_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            gpkg = Path(tmp) / "sivas.gpkg"
            drawing().save_to(gpkg, "Var")
            with self.assertRaises(cad.HostError) as missing:
                cad.Document.open_from(gpkg, "Yok")
            self.assertIn("projesi bu kaynakta yok", str(missing.exception))
            with self.assertRaises(cad.HostError) as empty:
                drawing().save_to(gpkg, "   ")
            self.assertIn("boş olamaz", str(empty.exception))


@unittest.skipUnless(os.environ.get("KENTOS_DATA_PG"), "KENTOS_DATA_PG yok: PostgreSQL proje deposu atlandı")
class ProjectInAPostgreSqlSchema(unittest.TestCase):
    def test_saved_and_opened_from_a_schema(self) -> None:
        address = os.environ["KENTOS_DATA_PG"]
        name = "Python deneme projesi"
        doc = drawing()
        doc.save_to(address, name, schema="public")
        self.assertIn(name, [p.name for p in cad.projects(address, schema="public")])
        back = cad.Document.open_from(address, name, schema="public")
        self.assertEqual(len(list(back.all_layers())), len(list(doc.all_layers())))


if __name__ == "__main__":
    unittest.main()
