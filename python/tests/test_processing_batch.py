"""Toplu çalıştırma ve çalışmanın kaydı from Python (plan/gis.md CBS-24, docs/adr/0529).

The three-step parcel model and a batch of one tool over a table of rows, through ``cad.processing``: the same
runner the desktop's İşlemler window runs, one undo step, all or nothing, and the record every run hands back
(``kentos.run-record`` 1) with its counts and steps.
"""

from __future__ import annotations

import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad.errors import CommandFailed
from kentos.cad.types import BatchRow

ROOT = Path(__file__).resolve().parents[2]
PARCELS = ROOT / "fixtures/processing/v1/parcels.kcad"


def parcels(doc: cad.Document, n: int) -> list[str]:
    """The persistent ids of the first `n` parcels."""
    return [r.uid for r in doc.entities(layer="parsel")][:n]


class ProcessingBatchTests(unittest.TestCase):
    def test_the_parcel_model_runs_as_one_undo_step_with_its_record(self) -> None:
        doc = cad.Document.open(PARCELS)
        ids = parcels(doc, 2)
        ran = cad.processing.run(doc, tool="model:builtin.parcelSheet", selection=ids)
        self.assertEqual(ran.step, "Parsel ölçü yazıları")
        r = ran.record
        self.assertEqual((r.kind, r.format, r.version), ("model", "kentos.run-record", 1))
        self.assertEqual([s.tool for s in r.steps],
                         ["points.numberVertices", "annotation.edgeLengths", "attributes.calculate"])
        # Each step read the two chosen parcels; the whole made or changed 6 points, 7 texts and 2 parcels.
        self.assertEqual([s.records_in for s in r.steps], [2, 2, 2])
        self.assertEqual((r.total.records_in, r.total.records_out), (6, 15))
        self.assertEqual(r.selection, ids)
        self.assertIsNotNone(r.definition)
        self.assertEqual(doc.undo(), "Parsel ölçü yazıları")

    def test_a_batch_is_one_undo_step_and_each_row_has_its_record(self) -> None:
        doc = cad.Document.open(PARCELS)
        ids = parcels(doc, 3)
        ran = cad.processing.batch(
            doc,
            tool="attributes.calculate",
            selection=ids,
            parameters={"value": "$genişlik || ' x ' || $yükseklik"},
            rows=[BatchRow(label="B1", parameters={"field": "B1"}), BatchRow(parameters={"field": "B2"})],
        )
        self.assertEqual([r.label for r in ran.rows], ["B1", "Satır 2"])
        self.assertEqual(ran.step, "Toplu: Öznitelik hesapla")
        self.assertEqual(ran.record.kind, "batch")
        self.assertEqual([s.id for s in ran.record.steps], ["rows[0]", "rows[1]"])
        self.assertTrue(all(row.ran.record.kind == "tool" for row in ran.rows))
        self.assertEqual(doc.undo(), "Toplu: Öznitelik hesapla")
        self.assertIsNone(doc.undo(), "one step took both rows back")

    def test_a_row_that_stops_takes_the_batch_back(self) -> None:
        doc = cad.Document.open(ROOT / "fixtures/processing/v1/queries.kcad")
        with self.assertRaises(CommandFailed) as caught:
            cad.processing.batch(
                doc,
                tool="attributes.calculate",
                parameters={"input": {"scope": "layer", "layerId": "parsel"}, "field": "Ağaç sayısı"},
                rows=[BatchRow(parameters={"value": "5"}), BatchRow(parameters={"value": "'çok'"})],
            )
        self.assertEqual(caught.exception.code, "batch_row_failed")
        self.assertEqual(caught.exception.path, "rows[1]")
        self.assertIsNone(doc.undo(), "nothing was written")

    def test_the_models_are_listed_when_asked(self) -> None:
        doc = cad.Document.open(PARCELS)
        listed = cad.processing.tools(doc, models=True)
        self.assertIn("model:builtin.parcelSheet", [t.id for t in listed.tools])
        self.assertNotIn("model:builtin.parcelSheet", [t.id for t in cad.processing.tools(doc).tools])


if __name__ == "__main__":
    unittest.main()
