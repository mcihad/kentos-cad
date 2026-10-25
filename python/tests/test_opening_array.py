"""Measured arrays use the same semantic command as the desktop and MCP."""
import math
from pathlib import Path
import tempfile
import unittest
from kentos import cad


class OpeningArray(unittest.TestCase):
    def setup_drawing(self, unit='m', x=0., y=0., arc=False):
        doc = cad.Document.new('Dağıtım kabulü', workspace='cad', drawing_unit=unit, plot_scale=100)
        host = cad.walls.create(doc, layer_id=doc.active_layer, walls=[
            cad.NewWall(pts=[(x, y), (x+20, y)], width=.2, closed=False,
                        **({'bulges': [1.]} if arc else {}))]).uids[0]
        source = cad.openings.create(doc, layer_id=doc.active_layer, openings=[
            cad.NewOpening(host=host, at=1, width=1, form='window', number='P1')]).uids[0]
        return doc, host, source

    def test_units_origins_analytic_arc_source_identity_undo_and_persistence(self):
        for unit in ('mm', 'cm', 'm', 'in', 'ft'):
            for x, y in ((0., 0.), (500000., 4400000.)):
                for arc in (False, True):
                    with self.subTest(unit=unit, origin=(x, y), arc=arc):
                        doc, host, source = self.setup_drawing(unit, x, y, arc)
                        original = doc.entity(source).entity
                        before = doc.to_bytes()
                        args = dict(uids=[source], operation=cad.ArrayOpeningsOperation(
                            layout=cad.SpacingOpeningArrayLayout(count=3, distance=1, direction='end'),
                            numbering=cad.OpeningArrayNumbering(prefix='P', start=2, step=2)))
                        plan = cad.openings.edit.plan(doc, **args)
                        self.assertEqual(doc.to_bytes(), before)
                        self.assertEqual([o.at for o in plan.added], [3., 5., 7.])
                        self.assertEqual([o.number for o in plan.added], ['P2', 'P4', 'P6'])
                        made = cad.openings.edit(doc, **args, expected_revision=plan.revision)
                        self.assertEqual(len(made.added), 3)
                        entities = made.added
                        uids = [r.uid for r in doc.entities() if r.entity.id in {o.id for o in entities}]
                        self.assertEqual(doc.entity(source).entity, original)
                        self.assertTrue(all(o.host == host for o in entities))
                        if arc:
                            # Semicircle radius 10, centre (x+10,y), analytic angle pi+s/10.
                            for o, at in zip(entities, (3., 5., 7.)):
                                angle = math.pi + (at-.5)/10
                                self.assertAlmostEqual(o.place[0].x, x+10+9.9*math.cos(angle), delta=1e-8)
                                self.assertAlmostEqual(o.place[0].y, y+9.9*math.sin(angle), delta=1e-8)
                        self.assertEqual(doc.journal()['calls'][-1]['command'], 'cad.openings.edit')
                        self.assertEqual(doc.undo(), 'Açıklığı düzenle')
                        self.assertEqual(len(doc), 2)
                        doc.redo()
                        self.assertEqual([doc.entity(u).entity for u in uids], entities)
                        with tempfile.TemporaryDirectory() as tmp:
                            path = Path(tmp)/'array.kcad'
                            doc.save(path)
                            back = cad.Document.open(path)
                            self.assertEqual([back.entity(u).entity for u in uids], entities)
                            self.assertEqual(back.entity(source).entity, original)
                            self.assertEqual(back.info().settings.plot_scale, 100)
                            self.assertEqual(back.info().units.drawing.value, unit)

    def test_fitted_copy_and_number_conflict_are_atomic(self):
        doc, host, source = self.setup_drawing()
        op = cad.ArrayOpeningsOperation(layout=cad.FitOpeningArrayLayout(count=1, start_margin=2, end_margin=1))
        plan = cad.openings.edit.plan(doc, uids=[source], operation=op)
        self.assertEqual(plan.added[0].at, 10.5)
        self.assertFalse(plan.added[0].number)
        op.numbering = cad.OpeningArrayNumbering(prefix='P', start=1)
        before = doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.openings.edit(doc, uids=[source], operation=op)
        self.assertEqual(caught.exception.code, 'invalid_opening')
        self.assertEqual(doc.to_bytes(), before)
        op.numbering = cad.OpeningArrayNumbering(prefix='P', start=2)
        made = cad.openings.edit(doc, uids=[source], operation=op)
        self.assertEqual(made.added[0].number, 'P2')

    def test_schedule_tracks_array_undo_redo_and_idempotent_request(self):
        doc, host, source = self.setup_drawing()
        table_uid = cad.schedules.create(doc, layer_id=doc.active_layer, of='openings', at=(0, -5),
            columns=[cad.ScheduleColumn(field='number')], group=False, total=False).uid
        before = doc.entity(table_uid).entity
        args = dict(uids=[source], operation=cad.ArrayOpeningsOperation(
            layout=cad.SpacingOpeningArrayLayout(count=3, distance=1, direction='end'),
            numbering=cad.OpeningArrayNumbering(prefix='P', start=2)))
        first = cad.openings.edit.answer(doc, **args, request_id='architecture:array:1')
        self.assertTrue(first.ok)
        self.assertFalse(first.replayed)
        after = doc.entity(table_uid).entity
        self.assertNotEqual(after, before)
        raw = after.to_json()
        words = str(raw)
        for mark in ('P1', 'P2', 'P3', 'P4'):
            self.assertIn(mark, words)
        length = len(doc)
        again = cad.openings.edit.answer(doc, **args, request_id='architecture:array:1')
        self.assertTrue(again.ok)
        self.assertTrue(again.replayed)
        self.assertEqual(first.output, again.output)
        self.assertEqual(len(doc), length)
        self.assertEqual(doc.entity(table_uid).entity, after)
        doc.undo()
        self.assertEqual(doc.entity(table_uid).entity, before)
        doc.redo()
        self.assertEqual(doc.entity(table_uid).entity, after)

    def test_decimal_jamb_fit_does_not_confuse_roundoff_with_real_shortfall(self):
        from decimal import Decimal
        doc, host, source = self.setup_drawing()
        cad.openings.edit(doc, uids=[source], operation=cad.SetOpeningsOperation(width=.1, frame=0))
        self.assertEqual(Decimal('20') - Decimal('17.8') - Decimal('2'), Decimal('.1') * 2)
        op = cad.ArrayOpeningsOperation(layout=cad.FitOpeningArrayLayout(count=2, start_margin=2, end_margin=17.8))
        before = doc.to_bytes()
        plan = cad.openings.edit.plan(doc, uids=[source], operation=op)
        self.assertEqual(doc.to_bytes(), before)
        for opening, at in zip(plan.added, (2.05, 2.15)):
            self.assertAlmostEqual(opening.at, at, delta=1e-12)
        cad.openings.edit(doc, uids=[source], operation=op)
        self.assertEqual(len(doc), 4)
        doc.undo()
        op.layout.end_margin = 17.800001
        before = doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.openings.edit(doc, uids=[source], operation=op)
        self.assertEqual(caught.exception.code, 'invalid_opening')
        self.assertEqual(doc.to_bytes(), before)
