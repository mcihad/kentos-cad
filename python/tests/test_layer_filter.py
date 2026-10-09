"""kentos.cad.layers.filter (docs/adr/0211 §5): a layer's filter written and taken away through the command, as one
undo step, on the traces' scene (fixtures/interaction/v1/feature-table.kcad); the filter read back from the KCAD
sample (fixtures/kcad/v2/filters.kcad); its refusals. A filter is the layer's view: Python sees every object."""

from __future__ import annotations

import unittest
from pathlib import Path

from kentos import cad
from kentos.cad.types import LayerFilter

ROOT = Path(__file__).resolve().parents[2]
SCENE = ROOT / "fixtures/interaction/v1/feature-table.kcad"
SAMPLE = ROOT / "fixtures/kcad/v2/filters.kcad"


class LayerFilters(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.open(SCENE)

    def test_a_filter_is_written_counted_and_undone(self) -> None:
        done = cad.layers.filter(self.doc, layer="parsel", filter=LayerFilter(expression="Ada = '101'"))
        self.assertEqual((done.changed, done.passed, done.total), (True, 2, 5))
        self.assertEqual(self.doc.layer("parsel").filter.expression, "Ada = '101'")
        # The same filter again writes nothing.
        again = cad.layers.filter(self.doc, layer="parsel", filter=LayerFilter(expression="Ada = '101'"))
        self.assertFalse(again.changed)
        # Geometry values: every parcel is 150 m².
        plan = cad.layers.filter.plan(self.doc, layer="parsel", filter=LayerFilter(expression="$alan > 100"))
        self.assertEqual((plan.passed, plan.total), (5, 5))
        # A filter is the layer's view: Python still sees every object.
        self.assertEqual(len(list(self.doc.entities())), 7)
        self.assertEqual(self.doc.undo(), "Katman süzgeci")
        self.assertFalse(isinstance(self.doc.layer("parsel").filter, LayerFilter))

    def test_a_list_of_objects_and_taking_it_away(self) -> None:
        uids = [r.uid for r in self.doc.entities() if r.entity.layer_id == "parsel"][:2]
        done = cad.layers.filter(self.doc, layer="parsel", filter=LayerFilter(objects=uids))
        self.assertEqual((done.passed, done.total), (2, 5))
        gone = cad.layers.filter(self.doc, layer="parsel")
        self.assertTrue(gone.changed)
        self.assertEqual(gone.passed, gone.total)

    def test_refusals(self) -> None:
        cases = [
            (dict(layer="parsel", filter=LayerFilter(expression="Ada =")), "invalid_expression"),
            (dict(layer="parsel", filter=LayerFilter(expression="Ada = '101' ")), "invalid_filter"),
            (dict(layer="parsel", filter=LayerFilter(expression="2 >= $sıra")), "invalid_expression"),
            (dict(layer="parsel", filter=LayerFilter()), "invalid_filter"),
            (dict(layer="kadastro", filter=LayerFilter(expression="Ada = '101'")), "not_a_layer"),
            (dict(layer="yok", filter=LayerFilter(expression="Ada = '101'")), "layer_not_found"),
        ]
        for kwargs, code in cases:
            with self.subTest(code=code, kwargs=kwargs):
                with self.assertRaises(cad.CommandError) as e:
                    cad.layers.filter(self.doc, **kwargs)
                self.assertEqual(e.exception.code, code)
        self.assertFalse(self.doc.info().can_undo)

    def test_the_kcad_sample_keeps_its_filters(self) -> None:
        doc = cad.Document.open(SAMPLE)
        parcels = doc.layer("parsel").filter
        self.assertIsInstance(parcels, LayerFilter)
        self.assertTrue(parcels.expression)
        self.assertTrue(parcels.objects)
        self.assertTrue(doc.layer("bina").filter.expression)
        self.assertTrue(doc.layer("yol").filter.objects)


if __name__ == "__main__":
    unittest.main()
