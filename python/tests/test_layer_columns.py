"""ADR 9318 through the typed layer: a data layer's table gets a column (`cad.layers.columns` add), every row reads empty
in it; a field the table does not have is named by `cad.layers.rows` (`missing`) while the rest is read, and a refresh
takes it off the layer."""
import shutil
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/data/v1/sutun.gpkg"


def project(folder: str, fields) -> tuple[cad.Document, str]:
    """A CBS project with a copy of sutun.gpkg's saha_varliklari as a data layer of `fields`; its id."""
    path = Path(folder) / "sutun.gpkg"
    shutil.copy(SOURCE, path)
    doc = cad.Document.new("Sütun", workspace="gis", srid=5256)
    data = {"driver": "gpkg", "address": str(path), "table": "saha_varliklari", "key": "fid",
            "geometry": {"column": "geom", "typeName": "POINT", "srid": 5256}}
    done = cad.layers.data(doc, operation="add", name="Saha varlıkları", data=cad.DataLayer.from_json(data),
                           fields=[cad.LayerField.from_json(f) for f in fields])
    return doc, done.layer


class LayerColumns(unittest.TestCase):
    def test_a_column_is_added_and_read_empty(self):
        with tempfile.TemporaryDirectory() as folder:
            doc, layer = project(folder, [{"name": "durum", "kind": "text"}, {"name": "no", "kind": "integer"}])
            note = cad.LayerField.from_json({"name": "saha_notu", "kind": "text"})
            done = cad.layers.columns(doc, action="add", layer=layer, fields=[note])
            self.assertEqual(done.added, ["saha_notu"])
            self.assertEqual(done.rows, 3)
            rows = cad.layers.rows(doc, layer=layer, fields=["saha_notu"])
            self.assertEqual(len(rows.rows), 3)
            self.assertTrue(all("saha_notu" not in r.attrs for r in rows.rows))

    def test_a_field_the_table_lacks_is_named_and_a_refresh_takes_it_off(self):
        with tempfile.TemporaryDirectory() as folder:
            doc, layer = project(folder, [{"name": "durum", "kind": "text"}, {"name": "kayip", "kind": "text"}])
            rows = cad.layers.rows(doc, layer=layer)
            self.assertEqual(len(rows.rows), 3, "the other columns still read")
            self.assertEqual(rows.missing, ["kayip"])
            done = cad.layers.columns(doc, action="refresh", layer=layer)
            self.assertEqual(done.removed, ["kayip"])
            self.assertEqual(done.added, ["no", "eski_not"])


if __name__ == "__main__":
    unittest.main()
