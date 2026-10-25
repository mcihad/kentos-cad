"""Explicit room levels use measured overlapping rooms, through the existing typed SDK."""
import json
from dataclasses import replace
from pathlib import Path
import tempfile
import unittest
from kentos import cad

class SpaceLevels(unittest.TestCase):
    def drawing(self):
        root=Path(__file__).resolve().parents[2]
        setup=json.loads((root/'fixtures/commands/v1/cad.walls.convert.json').read_text())['setup']
        setup['settings']['drawingUnit']='m';setup['settings']['plotScale']=100
        for level in setup['settings']['arch']['levels']:level['elevation']=0
        setup['entities']=[]
        for level,w,h in [('zemin',6,4),('kat-1',10,8),(None,12,10)]:
            wall=dict(kind='wall',id=len(setup['entities'])+1,layerId='cizim',attrs={},width=.2,closed=True,
                pts=[dict(x=500000+x,y=4400000+y) for x,y in [(0,0),(w,0),(w,h),(0,h)]])
            if level is not None:wall['arch']=dict(level=level)
            setup['entities'].append(wall)
        return cad.Document.from_bytes(json.dumps(setup).encode())

    def test_explicit_level_plan_update_history_quantities_and_persistence(self):
        doc=self.drawing();original={r.uid:doc.entity(r.uid).entity for r in doc.entities()}
        for level,area in [('zemin',5.8*3.8),('kat-1',9.8*7.8)]:
            args=dict(layer_id='cizim',spaces=[cad.NewSpace(seed=cad.Vec2(x=500002,y=4400002),
                arch=cad.ArchInfo(level=level),number='Z-01',name='Salon')])
            before=doc.to_bytes();plan=cad.spaces.create.plan(doc,**args)
            self.assertEqual(doc.to_bytes(),before)
            self.assertEqual(plan.spaces[0].arch.level,level)
            done=cad.spaces.create(doc,**args,expected_revision=plan.revision)
            uid=done.uids[0];saved=doc.entity(uid).entity
            self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[-1].area,area,places=7)
            doc.undo();self.assertEqual(len(doc),3)
            doc.redo();self.assertEqual(doc.entity(uid).entity,saved)
            if level=='zemin':
                edit=dict(uids=[uid],operation=cad.SetSpacesOperation(level='kat-1'))
                planned=cad.spaces.edit.plan(doc,**edit)
                self.assertEqual(planned.spaces[0].arch.level,'kat-1')
                self.assertEqual(doc.entity(uid).entity,saved)
                cad.spaces.edit(doc,**edit,expected_revision=planned.revision)
                self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[-1].area,9.8*7.8,places=7)
                doc.undo();self.assertEqual(doc.entity(uid).entity,saved)
            with tempfile.TemporaryDirectory() as tmp:
                path=Path(tmp)/'rooms.kcad';doc.save(path);back=cad.Document.open(path)
                # KCAD compacts transient numeric IDs after an undone creation;
                # the UUID lookup and all persistent content must survive intact.
                reopened=back.entity(uid).entity
                self.assertEqual(replace(reopened,id=saved.id),saved)
                self.assertEqual({u:back.entity(u).entity for u in original},original)
                self.assertEqual(back.info().settings.plot_scale,100)
            doc.undo();self.assertEqual(len(doc),3)

    def test_other_level_and_missing_target_are_atomic_refusals(self):
        doc=self.drawing()
        for level,code in [('zemin','space_not_enclosed'),('missing','level_not_found')]:
            before=doc.to_bytes()
            for op in ('plan','execute'):
                answer=cad.spaces.create.answer(doc,op=op,layer_id='cizim',spaces=[
                    cad.NewSpace(seed=cad.Vec2(x=500008,y=4400006),arch=cad.ArchInfo(level=level))])
                self.assertFalse(answer.ok);self.assertEqual(answer.error.code,code)
                self.assertEqual(doc.to_bytes(),before)
