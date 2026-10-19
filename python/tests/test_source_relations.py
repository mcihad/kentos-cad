"""The building, parcel and owner example of docs/adr/0532 from Python: three tables of a GeoPackage bound as layers, the
parcels' owners a many-to-many reference mapped into the file, a parcel and its owner written in one transaction, a form's
record carried to the file with ``commit``, the owners read back a page at a time. The file is read with sqlite3, not
with KentOS: the mapping's rows are what the layers' references say, and a refused layer stops the whole write.
"""

from __future__ import annotations

import shutil
import sqlite3
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad.errors import CommandFailed, RevisionConflict
from kentos.cad.types import LinkSourcesEditOp

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/geopackage/v1/iliskili.gpkg"
UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{:04d}"
OWNERS = [UID.format(3000 + i) for i in (1, 2, 3)]
PARCELS = [UID.format(2000 + i) for i in (1, 2, 3)]


def rows(path: Path, sql: str) -> list[tuple]:
    with sqlite3.connect(path) as c:
        return c.execute(sql).fetchall()


def world(path: Path) -> cad.Document:
    doc = cad.Document.new("Kadastro", srid=5256)
    for table in ("malik", "parsel", "bina"):
        cad.sources.edit(doc, edit=LinkSourcesEditOp(path=str(path), table=table))
    cad.layers.edit(doc, operation="fields", layer="import-parsel", fields=[
        cad.LayerField(name="ada", kind="text"),
        cad.LayerField(name="parsel_no", kind="text"),
        cad.LayerField(name="malikler", kind="text", multiple=True),
    ])
    cad.relations.edit(doc, edit=cad.CreateRelationsOp(relation=cad.ProjectRelation(
        id="parsel-malik", name="Parselin malikleri", parent="import-malik", child="import-parsel", field="malikler")))
    cad.relations.edit(doc, edit=cad.CreateRelationsOp(relation=cad.ProjectRelation(
        id="bina-parsel", name="Parselin binaları", parent="import-parsel", child="import-bina", field="parsel")))
    return doc


class SourceRelationsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "iliskili.gpkg"
        shutil.copy(SOURCE, self.path)

    def test_a_reference_is_checked_mapped_and_taken_out_again(self) -> None:
        doc = world(self.path)
        self.assertEqual(cad.sources.relate(doc, relation="parsel-malik", action="check").state, "text")
        before = self.path.read_bytes()
        planned = cad.sources.relate.plan(doc, relation="parsel-malik", action="map")
        self.assertEqual((planned.state, planned.rows, planned.mapping), ("mapped", 4, "parsel_malik"))
        self.assertEqual(self.path.read_bytes(), before, "a plan writes no file")
        mapped = cad.sources.relate(doc, relation="parsel-malik", action="map")
        self.assertEqual(mapped.rows, 4)
        self.assertEqual(rows(self.path, "SELECT base_id, related_id FROM parsel_malik ORDER BY rowid"),
                         [(1, 1), (1, 2), (2, 2), (2, 3)])
        self.assertEqual(rows(self.path, "SELECT base_table_name, related_table_name, relation_name "
                                         "FROM gpkgext_relations"), [("parsel", "malik", "kentos_references")])
        with self.assertRaises(CommandFailed) as caught:
            cad.sources.relate(doc, relation="parsel-malik", action="map")
        self.assertEqual(caught.exception.code, "relation_mapped")
        with self.assertRaises(CommandFailed) as caught:
            cad.sources.relate(doc, relation="bina-parsel", action="map")
        self.assertEqual(caught.exception.code, "relation_not_multiple")
        cad.sources.relate(doc, relation="parsel-malik", action="unmap")
        self.assertEqual(rows(self.path, "SELECT name FROM sqlite_master WHERE name LIKE '%relations%'"), [])
        self.assertEqual(cad.sources.relate(doc, relation="parsel-malik", action="check").state, "text")

    def test_a_parcel_and_its_owner_are_written_in_one_transaction(self) -> None:
        doc = world(self.path)
        cad.sources.relate(doc, relation="parsel-malik", action="map")
        owner = cad.records.create(doc, layer_id="import-malik",
                                   rows=[cad.RecordRow(attrs={"ad": "Zeynep Kaya", "tur": "Gerçek"})]).uids[0]
        cad.entities.set(doc, uids=[PARCELS[2]], operation="attributes", attrs={"malikler": owner})
        before = self.path.read_bytes()
        with self.assertRaises(CommandFailed) as caught:
            cad.sources.sync(doc, layer="import-parsel", direction="write")
        self.assertEqual(caught.exception.code, "relation_dangling", "the owner is not in the table")
        self.assertEqual(self.path.read_bytes(), before)
        written = cad.sources.sync(doc, layer="import-parsel", direction="write", with_=["import-malik"])
        self.assertEqual((written.added, [(l.layer, l.added) for l in written.layers]),
                         (1, [("import-parsel", 0), ("import-malik", 1)]))
        self.assertEqual([(r.relation, r.added, r.removed) for r in written.relations], [("parsel-malik", 1, 0)])
        self.assertEqual(rows(self.path, "SELECT count(*) FROM malik"), [(4,)])
        self.assertIn((3, 4), rows(self.path, "SELECT base_id, related_id FROM parsel_malik"))

    def test_a_refused_layer_stops_the_whole_write(self) -> None:
        doc = world(self.path)
        cad.entities.set(doc, uids=[PARCELS[0]], operation="attributes", attrs={"ada": "999"})
        cad.polygon.create(doc, layer_id="import-malik", pts=[cad.Vec2(x=0, y=0), cad.Vec2(x=1, y=0), cad.Vec2(x=1, y=1)])
        before = self.path.read_bytes()
        with self.assertRaises(CommandFailed) as caught:
            cad.sources.sync(doc, layer="import-parsel", direction="write", with_=["import-malik"])
        self.assertEqual(caught.exception.code, "geometry_mismatch")
        self.assertEqual(self.path.read_bytes(), before, "the parcel was not written either")

    def test_a_form_record_goes_to_the_file_with_its_children(self) -> None:
        doc = world(self.path)
        cad.sources.relate(doc, relation="parsel-malik", action="map")
        form = cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Malik formu", layer="import-malik", id="malik-formu"))
        self.assertEqual(form.form.id, "malik-formu")
        sent = cad.forms.submit(doc, form="malik-formu", draft=True, uid=OWNERS[0], values={"tur": "Tüzel"},
                                children=[cad.FormChild(relation="parsel-malik", uid=PARCELS[2])], commit=True)
        self.assertEqual(sent.committed.relations[0].added, 1)
        self.assertEqual(sorted(rows(self.path, "SELECT base_id, related_id FROM parsel_malik")),
                         [(1, 1), (1, 2), (2, 2), (2, 3), (3, 1)])
        self.assertEqual(rows(self.path, "SELECT tur FROM malik WHERE fid = 1"), [("Tüzel",)])

    def test_the_owners_are_read_a_page_at_a_time_with_the_sources_version(self) -> None:
        doc = world(self.path)
        cad.sources.relate(doc, relation="parsel-malik", action="map")
        first = cad.records.page(doc, layer="import-malik", limit=2)
        self.assertEqual((first.total, len(first.records), first.next), (3, 2, OWNERS[1]))
        self.assertEqual((first.source.table, first.source.edited), ("malik", False))
        second = cad.records.page(doc, layer="import-malik", limit=2, cursor=first.next,
                                  expected_source=first.source.revision)
        self.assertEqual(([r.uid for r in second.records], second.next), ([OWNERS[2]], cad.UNSET))
        owners = cad.records.page(doc, relation="parsel-malik", of=PARCELS[0], side="parents")
        self.assertEqual([r.uid for r in owners.records], [OWNERS[0], OWNERS[1]])
        parcels = cad.records.page(doc, relation="parsel-malik", of=OWNERS[1])
        self.assertEqual([r.uid for r in parcels.records], PARCELS[:2])
        cad.entities.set(doc, uids=[OWNERS[0]], operation="attributes", attrs={"tur": "Tüzel"})
        self.assertTrue(cad.records.page(doc, layer="import-malik").source.edited)
        with self.assertRaises(RevisionConflict) as caught:
            cad.records.page(doc, layer="import-malik", expected_source="0" * 64)
        self.assertEqual(caught.exception.code, "source_changed")


if __name__ == "__main__":
    unittest.main()
