"""Explicit virtual room boundaries through the public typed Python API."""
import json
from pathlib import Path
import tempfile
import unittest
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]
class SpaceSeparators(unittest.TestCase):
    def drawing(self):
        setup=json.loads((ROOT/'fixtures/commands/v1/cad.spaces.separators.json').read_text())['setup']
        doc=cad.Document.from_bytes(json.dumps(setup).encode())
        room=cad.spaces.create(doc,layer_id='mahal',spaces=[cad.NewSpace.from_json(dict(seed=dict(x=2.,y=6.),columns='all'))])
        return doc,room.uids[0]
    def test_typed_plan_one_step_role_edit_following_journal_and_reopen(self):
        doc,room=self.drawing();before=doc.to_bytes()
        op=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=[dict(pts=[dict(x=5.,y=0.),dict(x=5.,y=8.)])]))
        planned=cad.spaces.separators.plan(doc,operation=op)
        self.assertEqual(doc.to_bytes(),before)
        self.assertEqual(planned.entities[0].to_json()['spaceSeparator'],{})
        made=cad.spaces.separators(doc,operation=op,expected_revision=planned.revision)
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area,34.22,places=6)
        role=doc.entity(made.uids[0]).entity
        self.assertIn('spaceSeparator',role.to_json())
        self.assertEqual(doc.journal()['calls'][-1]['command'],'cad.spaces.separators')
        doc.undo();self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area,72.44,places=6)
        doc.redo();self.assertEqual(doc.entity(made.uids[0]).entity,role)
        cad.spaces.separators(doc,operation=cad.SpaceSeparatorOperation.from_json(dict(kind='clear',uids=made.uids)))
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area,72.44,places=6)
        self.assertNotIn('spaceSeparator',doc.entity(made.uids[0]).entity.to_json())
        doc.undo()
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'separator.kcad';doc.save(path);opened=cad.Document.open(path)
            self.assertEqual(opened.entity(made.uids[0]).entity,role)
            self.assertEqual(opened.entity(room).entity,doc.entity(room).entity)
            self.assertAlmostEqual(cad.spaces.quantities(opened).spaces[0].area,34.22,places=6)
    def test_invalid_create_refuses_atomically_with_the_public_nested_path(self):
        doc,_=self.drawing();before=doc.to_bytes()
        op=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=[dict(pts=[dict(x=0.,y=0.)])]))
        result=cad.spaces.separators.answer(doc,operation=op)
        self.assertFalse(result.ok);self.assertEqual(result.error.code,'too_few_points')
        self.assertEqual(result.error.path,'operation.paths[0].pts')
        self.assertEqual(doc.to_bytes(),before)
    def test_separator_tag_schedule_and_source_move_follow_in_one_undo(self):
        doc,room=self.drawing()
        tag=cad.tags.create(doc,layer_id='mahal',tags=[cad.NewTag.from_json(dict(of=room,content='{area}'))]).uids[0]
        table=cad.schedules.create(doc,layer_id='mahal',at=cad.Vec2(x=12,y=0),of='spaces',columns=[cad.ScheduleColumn.from_json(dict(field='area'))]).uid
        before={uid:doc.entity(uid).entity for uid in (room,tag,table)}
        op=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=[dict(pts=[dict(x=5,y=0),dict(x=5,y=8)])]))
        made=cad.spaces.separators(doc,operation=op)
        def measured(expected):
            self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area,expected,places=6)
            self.assertIn(f'{expected:.2f}',doc.entity(tag).entity.to_json()['text'])
            self.assertIn(f'{expected:.2f}',json.dumps(doc.entity(table).entity.to_json()['cells']))
        measured(34.22)
        divided={uid:doc.entity(uid).entity for uid in (room,tag,table)}
        cad.entities.transform.execute_input(doc,cad.EntitiesTransform.from_json(dict(uids=made.uids,transform=dict(kind='move',dx=1,dy=0))))
        measured(42.02)
        doc.undo();self.assertEqual({uid:doc.entity(uid).entity for uid in (room,tag,table)},divided)
        doc.undo();self.assertEqual({uid:doc.entity(uid).entity for uid in (room,tag,table)},before)
        measured(72.44)
        doc.redo();measured(34.22)
    def test_block_conversion_is_an_explicit_atomic_refusal(self):
        doc,_=self.drawing()
        made=cad.spaces.separators(doc,operation=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=[dict(pts=[dict(x=5,y=0),dict(x=5,y=8)])])))
        before=doc.to_bytes()
        answer=cad.blocks.define.answer_input(doc,cad.BlocksDefine.from_json(dict(name='Sınır',uids=made.uids,base=dict(x=0,y=0),replace=True,layerId='cizim')))
        self.assertFalse(answer.ok);self.assertEqual(answer.error.code,'block_refused');self.assertEqual(doc.to_bytes(),before)
        clip=cad.entities.copy(doc,uids=made.uids)
        answer=cad.entities.paste.answer_input(doc,cad.EntitiesPaste.from_json(dict(clip=clip.to_json(),asBlock='Sınır')))
        self.assertFalse(answer.ok);self.assertEqual(answer.error.code,'invalid_clip');self.assertEqual(doc.to_bytes(),before)
        pasted=cad.entities.paste(doc,clip=clip,at=cad.Vec2(x=20,y=0))
        self.assertIn('spaceSeparator',doc.entity(pasted.created[0]).entity.to_json())
        doc.undo();self.assertEqual(doc.to_bytes(),before)
    def test_existing_curve_and_vertex_tools_validate_the_same_role_constraints(self):
        doc,_=self.drawing()
        made=cad.spaces.separators(doc,operation=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=[dict(pts=[dict(x=0,y=0),dict(x=4,y=0),dict(x=4,y=4)])])))
        before=doc.to_bytes()
        for command,args,code,field in [
            (cad.curves.edit,cad.CurvesEdit.from_json(dict(uid=made.uids[0],edit=dict(kind='toSpline',degree=2,through=False))),'edit_not_applicable','edit'),
            (cad.entities.vertex,cad.EntitiesVertex.from_json(dict(uid=made.uids[0],action='width',place=dict(kind='mid',part=0,ring=0,index=0),widths=[1,1])),'invalid_polyline','uid'),
        ]:
            for op in ('validate','plan','execute'):
                result=command.run(doc,args,op=op)
                self.assertFalse(result.ok)
                self.assertEqual((result.error.code,result.error.path),(code,field))
                self.assertEqual(doc.to_bytes(),before)

    def test_quantity_report_marks_a_last_boundary_without_other_room_options(self):
        for virtual in (False,True):
            with self.subTest(virtual=virtual):
                setup=json.loads((ROOT/'fixtures/commands/v1/cad.spaces.separators.json').read_text())['setup']
                source=dict(kind='polyline' if virtual else 'wall',id=1,layerId='cizim',attrs={},closed=True,pts=[dict(x=0,y=0),dict(x=4,y=0),dict(x=4,y=3),dict(x=0,y=3)])
                source.update(dict(spaceSeparator={}) if virtual else dict(width=.2))
                setup['entities']=[source]
                doc=cad.Document.from_bytes(json.dumps(setup).encode())
                source_uid=next(doc.entities()).uid
                cad.spaces.create(doc,layer_id='mahal',spaces=[cad.NewSpace.from_json(dict(seed=dict(x=1,y=1)))])
                expected=12 if virtual else 10.64
                before=doc.to_bytes()
                valid=cad.spaces.quantities(doc)
                self.assertNotIn('boundaryBreak',valid.spaces[0].to_json())
                cad.entities.delete(doc,uids=[source_uid])
                lost=doc.to_bytes()
                plan=cad.spaces.quantities.plan(doc)
                measured=cad.spaces.quantities(doc)
                self.assertEqual(measured,plan)
                self.assertEqual(measured.spaces[0].boundary_break,'lostEnclosure')
                self.assertIn('son geçerli sınır',measured.spaces[0].rule)
                self.assertAlmostEqual(measured.spaces[0].area,expected)
                self.assertEqual(measured.spaces[0].by_method,{})
                self.assertEqual(doc.to_bytes(),lost)
                doc.undo();self.assertEqual(doc.to_bytes(),before)
                self.assertEqual(cad.spaces.quantities(doc),valid)
