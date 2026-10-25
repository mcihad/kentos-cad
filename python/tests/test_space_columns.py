"""Persistent physical column policy through the public SDK and real files."""
import json
import math
from pathlib import Path
import tempfile
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]

class SpaceColumns(unittest.TestCase):
    def drawing(self):
        setup = json.loads((ROOT/'fixtures/commands/v1/cad.spaces.columns.json').read_text())['setup']
        return cad.Document.from_bytes(json.dumps(setup).encode())

    def test_typed_plan_union_methods_journal_undo_and_real_file(self):
        doc = self.drawing()
        before = doc.to_bytes()
        args = dict(layer_id='mahal', level='zemin', columns='net')
        planned = cad.spaces.discover.plan(doc, **args)
        self.assertEqual(doc.to_bytes(), before)
        self.assertEqual(len(planned.proposals), 1)
        self.assertEqual(planned.proposals[0].space.columns.value, 'net')
        done = cad.spaces.discover(doc, **args, expected_revision=planned.revision)
        self.assertEqual(len(done.uids), 1)
        row = cad.spaces.quantities(doc).spaces[0]
        self.assertAlmostEqual(row.area, 76.44-6, places=7)
        self.assertAlmostEqual(row.perimeter, 35.2+10, places=7)
        self.assertEqual(row.columns.value, 'net')
        self.assertAlmostEqual(row.by_method['gross'], 83.64, places=7)
        self.assertTrue(cad.spaces.quantities(doc).spaces[0].boundary_break is cad.UNSET)
        self.assertEqual([c['command'] for c in doc.journal()['calls']], ['cad.spaces.discover'])
        repeated = cad.spaces.discover.plan(doc, **args)
        self.assertEqual(repeated.proposals[0].existing, done.uids)
        self.assertFalse(cad.spaces.discover(doc, **args).uids)
        doc.undo(); self.assertEqual(len(doc), 4)
        doc.redo()
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)/'columns.kcad'
            doc.save(path); reopened = cad.Document.open(path)
            self.assertEqual(reopened.entity(done.uids[0]).entity, doc.entity(done.uids[0]).entity)
            self.assertAlmostEqual(cad.spaces.quantities(reopened).spaces[0].area, 70.44, places=7)
            self.assertEqual(reopened.info().units.drawing.value, 'm')

    def test_legacy_absence_is_unchanged_then_policy_edit_follows_and_undoes(self):
        doc = self.drawing()
        done = cad.spaces.create(doc, layer_id='mahal', spaces=[cad.NewSpace(seed=cad.Vec2(x=5,y=6), arch=cad.ArchInfo(level='zemin'))])
        legacy = doc.entity(done.uids[0]).entity
        row = cad.spaces.quantities(doc).spaces[0]
        self.assertNotIn('columns', row.to_json())
        self.assertAlmostEqual(row.area, 76.44, places=7)
        operation = cad.SpacesOperation.from_json({'kind':'set','columns':'all','method':'gross'})
        planned = cad.spaces.edit.plan(doc, uids=done.uids, operation=operation)
        self.assertEqual(doc.entity(done.uids[0]).entity, legacy)
        self.assertEqual(planned.spaces[0].to_json()['columns'], 'all')
        cad.spaces.edit(doc, uids=done.uids, operation=operation)
        row = cad.spaces.quantities(doc).spaces[0]
        self.assertAlmostEqual(row.area, 83.64-6, places=7)
        self.assertEqual(row.columns.value, 'all')
        doc.undo(); self.assertEqual(doc.entity(done.uids[0]).entity, legacy)

    def test_analytic_round_column_far_from_origin(self):
        setup = json.loads((ROOT/'fixtures/commands/v1/cad.walls.convert.json').read_text())['setup']
        setup['entities'] = [
            dict(kind='wall',id=1,layerId='cizim',attrs={},width=.2,closed=True,pts=[dict(x=499995,y=4400000),dict(x=500005,y=4400000)],bulges=[1,1]),
            dict(kind='column',id=2,layerId='cizim',attrs={},width=1,shape='round',at=dict(x=500000,y=4400000)),
        ]
        doc = cad.Document.from_bytes(json.dumps(setup).encode())
        made = cad.spaces.discover(doc,layer_id='cizim',columns='all')
        self.assertEqual(len(made.uids),1)
        row = cad.spaces.quantities(doc).spaces[0]
        self.assertAlmostEqual(row.area,math.pi*(4.9**2-.5**2),places=6)
        self.assertTrue(any(doc.entity(made.uids[0]).entity.to_json()['parts'][0]['holes'][0]['bulges']))

    def test_drawn_policy_refusal_is_atomic(self):
        doc = self.drawing(); before = doc.to_bytes()
        spec = cad.NewSpace.from_json({'parts':[{'outer':{'pts':[{'x':1,'y':1},{'x':2,'y':1},{'x':2,'y':2},{'x':1,'y':2}]}}], 'columns':'net'})
        answer = cad.spaces.create.answer(doc,layer_id='mahal',spaces=[spec])
        self.assertFalse(answer.ok)
        self.assertEqual(answer.error.code,'invalid_space')
        self.assertEqual(answer.error.path,'spaces[0].columns')
        self.assertEqual(doc.to_bytes(),before)
