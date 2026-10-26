"""Precise opening placement through the public typed Python API."""
import math
from pathlib import Path
import tempfile
import unittest

from kentos import cad


class OpeningPlacement(unittest.TestCase):
    def test_dimensions_follow_jambs_and_preserve_units_history_and_saved_sources(self):
        for unit in ('m', 'mm'):
            for form in ('door', 'window', 'opening'):
                for x, y in ((0., 0.), (500000., 4400000.)):
                    with self.subTest(unit=unit, form=form, origin=(x, y)):
                        doc = cad.Document.new('Bağlı açıklık ölçüsü', workspace='cad', drawing_unit=unit)
                        host = cad.walls.create(doc, layer_id=doc.active_layer, walls=[
                            cad.NewWall(pts=[(x,y),(x+10,y)], width=.2, closed=False)]).uids[0]
                        opening = cad.openings.create(doc, layer_id=doc.active_layer,
                            openings=[cad.NewOpening(host=host, at=4, width=1, form=form)]).uids[0]
                        model = lambda: {r.uid:r.entity.to_json() for r in doc.entities()}
                        sources = model()
                        for index, dx, dy in ((0,3.5,.1),(1,3.5,-.1),(2,4.5,.1),(3,4.5,-.1)):
                            for all_candidates in (False, True):
                                hit = cad.snap.find(doc,at=(x+dx,y+dy),aperture=.0001,
                                    kinds=['endpoint'],all=all_candidates).snaps[0]
                                self.assertEqual(hit.uid,opening)
                                self.assertEqual(hit.point.to_json(),dict(x=x+dx,y=y+dy))
                                self.assertEqual(hit.place.to_json(),dict(kind='vertex',part=0,ring=0,index=index))
                        self.assertEqual(model(),sources)
                        args = dict(layer_id=doc.active_layer, at=(x+4,y-1),
                            measure=cad.AlignedDimensionMeasure(
                                a=cad.PlaceMeasuredPoint(uid=opening, place=cad.VertexSubPlace(part=0,ring=0,index=0)),
                                b=cad.PlaceMeasuredPoint(uid=opening, place=cad.VertexSubPlace(part=0,ring=0,index=2))))
                        plan = cad.dimensions.create.plan(doc, **args)
                        self.assertEqual(model(), sources)
                        dim = cad.dimensions.create(doc, **args, expected_revision=plan.revision).created[0]
                        created = model()
                        self.assertEqual({k:v for k,v in created.items() if k != dim}, sources)
                        self.assertEqual(created[dim]['a'], dict(x=x+3.5,y=y+.1))
                        self.assertEqual(created[dim]['b'], dict(x=x+4.5,y=y+.1))
                        self.assertEqual(created[dim]['refs']['a']['uid'], opening)
                        self.assertEqual(created[dim]['refs']['b']['uid'], opening)
                        doc.undo(); self.assertEqual(model(), sources)
                        doc.redo(); self.assertEqual(model(), created)
                        cad.openings.edit(doc,uids=[opening],operation=cad.SetOpeningsOperation(width=2))
                        wide = model()
                        self.assertEqual(wide[dim]['a'], dict(x=x+3,y=y+.1))
                        self.assertEqual(wide[dim]['b'], dict(x=x+5,y=y+.1))
                        self.assertEqual(wide[host], sources[host])
                        doc.undo(); self.assertEqual(model(), created)
                        doc.redo(); self.assertEqual(model(), wide)
                        with tempfile.TemporaryDirectory() as tmp:
                            path=Path(tmp)/'opening-dimension.kcad'; doc.save(path)
                            back=cad.Document.open(path)
                            self.assertEqual({r.uid:r.entity.to_json() for r in back.entities()}, wide)
                        self.assertEqual(doc.info().units.drawing.value, unit)

    def test_lost_host_breaks_dimension_and_refuses_new_binding_atomically(self):
        doc=cad.Document.new(workspace='cad')
        host=cad.walls.create(doc,layer_id=doc.active_layer,walls=[
            cad.NewWall(pts=[(0,0),(10,0)],width=.2,closed=False)]).uids[0]
        opening=cad.openings.create(doc,layer_id=doc.active_layer,
            openings=[cad.NewOpening(host=host,at=4,width=2)]).uids[0]
        args=dict(layer_id=doc.active_layer,at=(4,-1),measure=cad.AlignedDimensionMeasure(
            a=cad.PlaceMeasuredPoint(uid=opening,place=cad.StartSubPlace()),
            b=cad.PlaceMeasuredPoint(uid=opening,place=cad.EndSubPlace())))
        dim=cad.dimensions.create(doc,**args).created[0]
        before=doc.to_bytes()
        cad.entities.delete(doc,uids=[host])
        entity=doc.entity(dim).entity.to_json()
        self.assertEqual(entity['a'],dict(x=3.,y=0.))
        self.assertEqual(entity['b'],dict(x=5.,y=0.))
        for key in ('a','b'):self.assertEqual(entity['refs'][key]['broken'],'no_such_place')
        broken=doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:cad.dimensions.create(doc,**args)
        self.assertEqual(caught.exception.code,'no_such_place')
        self.assertEqual(doc.to_bytes(),broken)
        doc.undo();self.assertEqual(doc.to_bytes(),before)
        doc.redo();self.assertEqual(doc.to_bytes(),broken)

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
