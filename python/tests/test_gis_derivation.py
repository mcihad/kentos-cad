"""The drawing as a traceable GIS dataset from Python (plan/gis.md CBS-16, docs/adr/0530).

The CBS-16 drawing (fixtures/gis-derivation/v1/drawing.json: an arced parcel, a block with attributes, curves) written as a
GeoPackage of chords with ``cad.file.export.gis``, its rows read here with the standard library's ``sqlite3`` (the ids,
kinds and deviations, the derivation record in the metadata), checked against the drawing with ``cad.gis.derivation``,
and the policy tried: a changed source makes its row stale, the derivative's table is never bound back, and it says what it
is when taken into another drawing. The same commands the desktop and MCP run.
"""

from __future__ import annotations

import json
import sqlite3
import tempfile
import unittest
import warnings
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed
from kentos.cad.types import GisAnnotations, LinkSourcesEditOp

# The commands warn in words (what is left out, what is old); the tests read the same answers.
warnings.simplefilter("ignore", cad.CommandWarning)

ROOT = Path(__file__).resolve().parents[2]
DRAWING = ROOT / "fixtures/gis-derivation/v1/drawing.json"


def rows(path: Path, table: str, *columns: str) -> list[tuple]:
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        return c.execute(f'SELECT {", ".join(columns)} FROM "{table}" ORDER BY fid').fetchall()
    finally:
        c.close()


class GisDerivationTests(unittest.TestCase):
    def test_a_drawing_goes_out_as_a_derived_dataset_and_is_checked(self) -> None:
        doc = cad.Document.open(DRAWING)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "ada.gpkg"
            planned = cad.file.export.gis.plan(doc, path=str(out), tolerance=0.001)
            self.assertFalse(out.exists(), "plan writes nothing")
            written = cad.file.export.gis(doc, path=str(out), tolerance=0.001)
            self.assertEqual(written.algorithm, "kentos.chords/1")
            self.assertEqual(written.tolerance, 0.001)
            self.assertEqual(written.chorded, planned.chorded)
            self.assertLessEqual(written.max_deviation, 0.001)
            self.assertEqual(
                [t.table for t in written.tables],
                ["Parsel", "blok_Bina", "Bina", "Yol", "kentos_annotations", "Çizim"],
            )
            self.assertEqual(sorted(l.kind for l in written.left), ["ray", "xline"])
            # Each row of a layer's table has its source's id, kind and deviation.
            parcels = rows(out, "Parsel", "kentos_uid", "kentos_kind", "kentos_deviation", "ada")
            self.assertEqual([r[1] for r in parcels], ["polygon", "polygon", "record", "circle"])
            self.assertTrue(all(0.0 <= r[2] <= 0.001 for r in parcels))
            self.assertGreater(parcels[0][2], 0.0, "the arced parcel is chords")
            self.assertEqual(parcels[1][2], 0.0, "the straight one is its own points")
            ids = {r[0] for r in parcels}
            self.assertEqual(len(ids), 4)
            ext = sqlite3.connect(out).execute("SELECT uid FROM kentos_uids WHERE table_name = 'Parsel'").fetchall()
            self.assertEqual({r[0] for r in ext}, ids, "the file's lasting ids and the column agree")
            # The block's attributes are the table's columns.
            blocks = rows(out, "blok_Bina", "NO", "KAT", "kentos_layer")
            self.assertEqual(blocks, [("B-1", "3", "Bina"), ("B-2", "1", "Bina")])
            # The table says it is a derivative.
            c = sqlite3.connect(out)
            records = [json.loads(r[0]) for r in c.execute("SELECT metadata FROM gpkg_metadata WHERE md_standard_uri = 'https://kentos.app/derivation/1'")]
            c.close()
            self.assertEqual(len(records), 6)
            self.assertTrue(all(r["tolerance"] == 0.001 and r["algorithm"] == "kentos.chords/1" for r in records))
            self.assertTrue(all(r["source"]["name"] == "Ada 101 türevi" for r in records))
            # The dataset matches the drawing.
            checked = cad.gis.derivation(doc, path=str(out))
            self.assertEqual((checked.current, checked.stale, checked.edited), (17, 0, 0))
            self.assertTrue(checked.same_project)
            self.assertEqual(checked.entries, [])
            # A changed source: its row is stale, and only it.
            parcel = next(r for r in doc.entities(layer="parsel") if r.entity.kind == "polygon")
            cad.entities.set(doc, uids=[parcel.uid], operation=cad.PropertiesOperation.ATTRIBUTES, attrs={"parsel_no": "70"})
            stale = cad.gis.derivation(doc, path=str(out))
            self.assertEqual((stale.current, stale.stale), (16, 1))
            self.assertEqual([(e.table, e.status.value, e.uid) for e in stale.entries], [("Parsel", "stale", parcel.uid)])
            # A finer tolerance for the same drawing writes more, within its bound.
            fine = Path(tmp) / "ince.gpkg"
            finer = cad.file.export.gis(doc, path=str(fine), tolerance=0.000001)
            self.assertLessEqual(finer.max_deviation, 0.000001)
            self.assertGreater(finer.bytes, written.bytes)
            # Taken out: a file there is kept unless asked.
            with self.assertRaises(CommandFailed) as refused:
                cad.file.export.gis(doc, path=str(out))
            self.assertEqual(refused.exception.code, "file_exists")
            with self.assertRaises(CommandFailed) as bad:
                cad.file.export.gis(doc, path=str(out), overwrite=True, tolerance=0.0)
            self.assertEqual(bad.exception.code, "invalid_tolerance")

    def test_texts_and_blocks_can_be_left_out_and_a_derivative_is_not_bound_back(self) -> None:
        doc = cad.Document.open(DRAWING)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "atla.gpkg"
            skipped = cad.file.export.gis(doc, path=str(out), annotations=GisAnnotations.SKIP)
            self.assertTrue(all(t.role.value == "layer" for t in skipped.tables))
            self.assertEqual(sorted(l.kind for l in skipped.left), ["insert", "ray", "text", "xline"])
            full = Path(tmp) / "ada.gpkg"
            cad.file.export.gis(doc, path=str(full))
            # The derivative's table is not bound as a layer, nor into one.
            with self.assertRaises(CommandFailed) as refused:
                cad.sources.edit(doc, edit=LinkSourcesEditOp(path=str(full), table="Parsel"))
            self.assertEqual(refused.exception.code, "derived_dataset")
            # Taken into a new drawing it says what it is: the road's arc is a polyline of chords.
            other = cad.Document.new("Ada 101", srid=5256)
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter("always")
                cad.file.import_.gpkg(other, path=str(full), tables=["Yol"])
            self.assertTrue(any("derived_import" in str(w.message) for w in caught), [str(w.message) for w in caught])
            road = next(r for r in other.entities() if r.entity.attrs.get("ad") == "D-100")
            self.assertEqual(road.entity.kind, "polyline")
            self.assertGreater(len(road.entity.pts), 20)


if __name__ == "__main__":
    unittest.main()
