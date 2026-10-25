"""Explicit source scope through public typed commands and real persistence."""
import json
from pathlib import Path
import tempfile
import unittest
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]
class SpaceScope(unittest.TestCase):
    def drawing(self):
        setup=json.loads((ROOT/'fixtures/commands/v1/cad.spaces.scope.json').read_text())['setup']
        return cad.Document.from_bytes(json.dumps(setup).encode())
    def test_plan_write_edit_single_undo_and_file_keep_scope(self):
        doc=self.drawing();before=doc.to_bytes()
        args=dict(layer_id='mahal',scope='unassigned',columns='all')
        plan=cad.spaces.discover.plan(doc,**args)
        self.assertEqual(doc.to_bytes(),before)
        self.assertEqual(len(plan.proposals),1)
        self.assertEqual(plan.proposals[0].space.scope.value,'unassigned')
        done=cad.spaces.discover(doc,**args,expected_revision=plan.revision)
        row=cad.spaces.quantities(doc).spaces[0]
        self.assertEqual(row.scope.value,'unassigned')
        self.assertAlmostEqual(row.area,72.44,places=6)
        entity=doc.entity(done.uids[0]).entity
        self.assertEqual([r['command']for r in doc.journal()['calls']],['cad.spaces.discover'])
        repeated=cad.spaces.discover.plan(doc,**args)
        self.assertEqual(repeated.proposals[0].existing,done.uids)
        unsafe=cad.spaces.edit.answer(doc,uids=done.uids,operation=cad.SpacesOperation.from_json(dict(kind='set',scope='project')))
        self.assertFalse(unsafe.ok);self.assertEqual(unsafe.error.path,'operation.scope')
        self.assertEqual(doc.entity(done.uids[0]).entity,entity)
        rebound=cad.SpacesOperation.from_json(dict(kind='rebound',seed=dict(x=2.,y=6.)))
        cad.spaces.edit(doc,uids=done.uids,operation=rebound)
        entity=doc.entity(done.uids[0]).entity
        op=cad.SpacesOperation.from_json(dict(kind='set',scope='project'))
        cad.spaces.edit(doc,uids=done.uids,operation=op)
        changed=cad.spaces.quantities(doc).spaces[0]
        self.assertNotIn('scope',changed.to_json())
        self.assertAlmostEqual(changed.area,33.44,places=6)
        doc.undo();self.assertEqual(doc.entity(done.uids[0]).entity,entity)
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'scope.kcad';doc.save(path)
            opened=cad.Document.open(path)
            self.assertEqual(opened.entity(done.uids[0]).entity,entity)
            self.assertAlmostEqual(cad.spaces.quantities(opened).spaces[0].area,72.44,places=6)
    def test_conflicting_explicit_target_refuses_without_writing(self):
        doc=self.drawing();before=doc.to_bytes()
        result=cad.spaces.discover.answer(doc,layer_id='mahal',scope='unassigned',level='zemin')
        self.assertFalse(result.ok);self.assertEqual(result.error.code,'invalid_space')
        self.assertEqual(result.error.path,'scope');self.assertEqual(doc.to_bytes(),before)
    def test_absence_and_explicit_project_remain_identical(self):
        a=self.drawing();b=self.drawing()
        old=cad.spaces.discover.plan(a,layer_id='mahal')
        explicit=cad.spaces.discover.plan(b,layer_id='mahal',scope='project')
        self.assertEqual([p.to_json()for p in old.proposals],[p.to_json()for p in explicit.proposals])
        self.assertEqual(len(old.proposals),2)
