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

    def test_the_fields_rules_go_into_the_table_and_come_back(self):
        # ADR 9324: the list written into the GeoPackage; a value the rows hold outside it said; another layer on the
        # same table reads the list back on refresh.
        with tempfile.TemporaryDirectory() as folder:
            listed = {"name": "durum", "kind": "text",
                      "values": [{"code": "A", "label": "Aktif"}, {"code": "C", "label": "Kapalı"}]}
            doc, layer = project(folder, [listed, {"name": "no", "kind": "integer"}])
            done = cad.layers.columns(doc, action="rules", layer=layer)
            self.assertEqual(done.ruled, ["durum"])
            self.assertEqual([(o.field, [(v.value, v.rows) for v in o.values]) for o in done.outside],
                             [("durum", [("B", 1)])])
            path = Path(folder) / "sutun.gpkg"
            data = {"driver": "gpkg", "address": str(path), "table": "saha_varliklari", "key": "fid",
                    "geometry": {"column": "geom", "typeName": "POINT", "srid": 5256}}
            other = cad.layers.data(doc, operation="add", name="Aynı tablo", data=cad.DataLayer.from_json(data),
                                    fields=[cad.LayerField.from_json({"name": "durum", "kind": "text"})]).layer
            back = cad.layers.columns(doc, action="refresh", layer=other)
            durum = next(f for f in back.fields if f.name == "durum")
            self.assertEqual([(c.code, c.label) for c in durum.values], [("A", "Aktif"), ("C", "Kapalı")])


if __name__ == "__main__":
    unittest.main()
