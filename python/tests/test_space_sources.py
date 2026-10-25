"""Public typed API keeps explicit room identities through edits and files."""
import json
from pathlib import Path
import tempfile
import unittest
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]
class SpaceSources(unittest.TestCase):
    def drawing(self):
        setup=json.loads((ROOT/'fixtures/commands/v1/cad.spaces.scope.json').read_text())['setup']
        doc=cad.Document.from_bytes(json.dumps(setup).encode())
        rows={r.entity.id:r.uid for r in doc.entities()}
        return doc,rows
    def test_invalid_discovery_sources_name_the_public_input_field(self):
        doc,uids=self.drawing();before=doc.to_bytes()
        for values in [[],[uids[1],uids[1]],['00000000-0000-0000-0000-000000000001']]:
            result=cad.spaces.discover.answer(doc,layer_id='mahal',sources=values)
            self.assertFalse(result.ok);self.assertEqual(result.error.path,'sources')
            self.assertEqual(doc.to_bytes(),before)
    def test_typed_subset_following_reject_undo_and_file(self):
        doc,uids=self.drawing();before=doc.to_bytes();sources=[uids[3],uids[1]]
        plan=cad.spaces.discover.plan(doc,layer_id='mahal',sources=sources,columns='all')
        self.assertEqual(doc.to_bytes(),before);self.assertEqual(len(plan.proposals),1)
        self.assertEqual(plan.proposals[0].space.sources,sorted(sources))
        done=cad.spaces.discover(doc,layer_id='mahal',sources=sources,columns='all',expected_revision=plan.revision)
        room=doc.entity(done.uids[0]).entity;row=cad.spaces.quantities(doc).spaces[0]
        self.assertAlmostEqual(row.area,72.44,places=6);self.assertEqual(row.sources,sorted(sources))
        # Deleting an excluded partition changes no source association.
        cad.entities.delete(doc,uids=[uids[2]])
        self.assertEqual(doc.entity(done.uids[0]).entity,room);doc.undo()
        cad.entities.delete(doc,uids=[uids[3]])
        stale=doc.entity(done.uids[0]).entity.to_json()
        self.assertEqual(stale['broken'],'sourceUnavailable')
        self.assertEqual(stale['parts'],room.to_json()['parts'])
        self.assertEqual(cad.spaces.quantities(doc).spaces[0].boundary_break.value,'sourceUnavailable')
        doc.undo();self.assertEqual(doc.entity(done.uids[0]).entity,room)
        for values in [[],[uids[1],uids[1]],['00000000-0000-0000-0000-000000000001']]:
            op=cad.SpacesOperation.from_json(dict(kind='set',sources=values))
            result=cad.spaces.edit.answer(doc,uids=done.uids,operation=op)
            self.assertFalse(result.ok);self.assertEqual(doc.entity(done.uids[0]).entity,room)
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'sources.kcad';doc.save(path);opened=cad.Document.open(path)
            self.assertEqual(opened.entity(done.uids[0]).entity,room)
            self.assertAlmostEqual(cad.spaces.quantities(opened).spaces[0].area,72.44,places=6)
