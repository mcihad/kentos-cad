"""Precise opening placement through the public typed Python API."""
import math
from pathlib import Path
import tempfile
import unittest

from kentos import cad


class OpeningPlacement(unittest.TestCase):
    def test_physical_hinge_reference_and_two_points_survive_units_undo_and_file(self):
        for unit in ('mm', 'cm', 'm', 'in', 'ft'):
            for x, y in ((0., 0.), (500000., 4400000.)):
                with self.subTest(unit=unit, origin=(x, y)):
                    doc = cad.Document.new('Açıklık', workspace='cad', drawing_unit=unit)
                    host = cad.walls.create(doc, layer_id=doc.active_layer, walls=[
                        cad.NewWall(pts=[(x, y), (x + 10, y)], width=.2, closed=False)]).uids[0]
                    inputs = [cad.NewOpening(host=host, point=(x + 4, y + .2), distance=2,
                                             distance_reference='hinge', from_='end',
                                             hand='start', width=1, form='door')]
                    before = doc.to_bytes()
                    plan = cad.openings.create.plan(doc, layer_id=doc.active_layer, openings=inputs)
                    self.assertEqual(doc.to_bytes(), before)
                    self.assertEqual(plan.openings[0].at, 8.5)
                    made = cad.openings.create(doc, layer_id=doc.active_layer, openings=inputs,
                                               expected_revision=plan.revision)
                    entity = doc.entity(made.uids[0]).entity
                    self.assertEqual(entity.width, 1)
                    self.assertEqual(entity.host, host)
                    self.assertEqual(doc.undo(), 'Kapı')
                    self.assertEqual(len(doc), 1)
                    self.assertEqual(doc.redo(), 'Kapı')
                    span = cad.NewOpening(host=host, span=[(x + 2, y + .2), (x + 3.2, y + .2)],
                                          form='window')
                    other = cad.openings.create(doc, layer_id=doc.active_layer, openings=[span])
                    window = doc.entity(other.uids[0]).entity
                    self.assertAlmostEqual(window.width, 1.2, delta=1e-9)
                    self.assertAlmostEqual(window.at, 2.6, delta=1e-9)
                    with tempfile.TemporaryDirectory() as tmp:
                        path = Path(tmp) / 'opening.kcad'
                        doc.save(path)
                        back = cad.Document.open(path)
                        self.assertEqual(back.entity(made.uids[0]).entity, entity)
                        self.assertEqual(back.entity(other.uids[0]).entity, window)

    def test_curved_span_uses_arc_length_and_overlapping_batch_has_no_partial_write(self):
        doc = cad.Document.new(workspace='cad')
        host = cad.walls.create(doc, layer_id=doc.active_layer, walls=[
            cad.NewWall(pts=[(0, 0), (10, 0)], bulges=[1], width=.2, closed=False)]).uids[0]
        r = 5 / math.sqrt(2)
        opening = cad.NewOpening(host=host, span=[(5 - r, -r), (5, -5)])
        plan = cad.openings.create.plan(doc, layer_id=doc.active_layer, openings=[opening])
        self.assertAlmostEqual(plan.openings[0].width, 5 * math.pi / 4)
        before = doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.openings.create(doc, layer_id=doc.active_layer, openings=[opening, opening])
        self.assertEqual(caught.exception.code, 'openings_overlap')
        self.assertEqual(doc.to_bytes(), before)
