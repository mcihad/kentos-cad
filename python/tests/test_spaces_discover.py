"""Room-network discovery through the public typed SDK, with independent sizes."""
import json
import math
from pathlib import Path
import tempfile
import unittest
from kentos import cad

class DiscoverSpaces(unittest.TestCase):
    def drawing(self):
        root=Path(__file__).resolve().parents[2]
        setup=json.loads((root/'fixtures/commands/v1/cad.walls.convert.json').read_text())['setup']
        setup['settings']['drawingUnit']='mm'
        setup['entities']=[
            dict(kind='wall',id=1,layerId='cizim',attrs={},width=.2,closed=True,arch=dict(level='zemin'),pts=[dict(x=x+500000,y=y+4400000) for x,y in [(0,0),(10,0),(10,8),(0,8)]]),
            dict(kind='wall',id=2,layerId='cizim',attrs={},width=.2,arch=dict(level='zemin'),pts=[dict(x=500005,y=4400000),dict(x=500005,y=4400008)])]
        return cad.Document.from_bytes(json.dumps(setup).encode())

    def test_typed_plan_atomic_create_repeat_history_and_real_file(self):
        doc=self.drawing();original={r.uid:doc.entity(r.uid).entity for r in doc.entities()}
        before=doc.to_bytes();planned=cad.spaces.discover.plan(doc,layer_id='cizim',level='zemin')
        self.assertEqual(doc.to_bytes(),before);self.assertEqual(len(planned.proposals),2)
        self.assertTrue(all(not p.existing and not p.conflicts for p in planned.proposals))
        done=cad.spaces.discover(doc,layer_id='cizim',level='zemin',expected_revision=planned.revision)
        self.assertEqual(len(done.uids),2)
        self.assertEqual([c['command'] for c in doc.journal()['calls']],['cad.spaces.discover'])
        for row in cad.spaces.quantities(doc).spaces:self.assertAlmostEqual(row.area,4.8*7.8,places=7)
        rooms={u:doc.entity(u).entity for u in done.uids}
        repeated=cad.spaces.discover.plan(doc,layer_id='cizim',level='zemin')
        self.assertTrue(all(len(p.existing)==1 and not p.conflicts for p in repeated.proposals))
        self.assertFalse(cad.spaces.discover(doc,layer_id='cizim',level='zemin').uids)
        doc.undo();self.assertEqual(len(doc),2);doc.redo()
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'rooms.kcad';doc.save(path);opened=cad.Document.open(path)
            self.assertEqual({u:opened.entity(u).entity for u in rooms},rooms)
            self.assertEqual({u:opened.entity(u).entity for u in original},original)
            self.assertEqual(opened.info().units.drawing.value,'mm')

    def test_region_selects_whole_room_and_missing_level_is_atomic(self):
        doc=self.drawing();bounds=[cad.Vec2(x=500006,y=4400000),cad.Vec2(x=500011,y=4400008)]
        plan=cad.spaces.discover.plan(doc,layer_id='cizim',level='zemin',bounds=bounds)
        self.assertEqual(len(plan.proposals),1)
        done=cad.spaces.discover(doc,layer_id='cizim',level='zemin',bounds=bounds)
        self.assertEqual(len(done.uids),1);self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area,4.8*7.8,places=7)
        before=doc.to_bytes()
        answer=cad.spaces.discover.answer(doc,op='execute',layer_id='cizim',level='missing')
        self.assertFalse(answer.ok);self.assertEqual(answer.error.code,'level_not_found');self.assertEqual(doc.to_bytes(),before)

    def test_curved_room_keeps_analytic_arcs_and_all_three_physical_areas(self):
        root=Path(__file__).resolve().parents[2]
        for unit in ('mm','m','ft'):
            setup=json.loads((root/'fixtures/commands/v1/cad.walls.convert.json').read_text())['setup']
            setup['settings']['drawingUnit']=unit
            setup['entities']=[dict(kind='wall',id=1,layerId='cizim',attrs={},width=.2,closed=True,
                pts=[dict(x=499995,y=4400000),dict(x=500005,y=4400000)],bulges=[1,1])]
            doc=cad.Document.from_bytes(json.dumps(setup).encode())
            for method,radius in [('net',4.9),('centerline',5),('gross',5.1)]:
                with self.subTest(unit=unit,method=method):
                    plan=cad.spaces.discover.plan(doc,layer_id='cizim',method=method)
                    self.assertEqual(len(plan.proposals),1)
                    self.assertTrue(any(plan.proposals[0].space.parts[0].outer.bulges))
                    made=cad.spaces.discover(doc,layer_id='cizim',method=method)
                    self.assertEqual(len(made.uids),1)
                    self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area,math.pi*radius**2,places=6)
                    doc.undo();self.assertEqual(len(doc),1)
