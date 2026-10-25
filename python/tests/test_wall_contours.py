"""The typed contour workflow preserves sources, analytic rings and SI units."""
from pathlib import Path
import tempfile
import unittest
from kentos import cad

class WallContours(unittest.TestCase):
    def test_winding_units_source_identity_one_history_and_persistence(self):
        for unit in ('m','mm','cm','in','ft'):
            for reverse in (False,True):
                with self.subTest(unit=unit,reverse=reverse):
                    doc=cad.Document.new('Kontur',workspace='cad',drawing_unit=unit,plot_scale=100)
                    x,y=500000.,4400000.
                    pts=[(x,y),(x+6,y),(x+6,y+2),(x+2,y+2),(x+2,y+5),(x,y+5)]
                    if reverse:pts[1:]=reversed(pts[1:])
                    source=cad.polygon.create(doc,layer_id=doc.active_layer,pts=pts,attrs={'oda':'A'},color='#123456').uid
                    original=doc.entity(source).entity
                    before=doc.to_bytes()
                    plan=cad.walls.convert.plan(doc,uids=[source],mode=cad.WallsFromMode.CONTOURS,justify='left',keep_sources=True,width=.2)
                    self.assertEqual(doc.to_bytes(),before)
                    self.assertEqual(len(plan.walls),1)
                    wall=plan.walls[0]
                    self.assertEqual(wall.width,.2)
                    self.assertEqual(wall.pts[0],cad.Vec2(x=x,y=y))
                    self.assertEqual(wall.pts[1],cad.Vec2(x=x+6,y=y))
                    self.assertEqual(wall.attrs,original.attrs)
                    self.assertEqual(wall.color,original.color)
                    args=dict(uids=[source],mode='contours',justify='left',keep_sources=True,width=.2,expected_revision=plan.revision)
                    first=cad.walls.convert.answer(doc,**args,request_id='contour-once')
                    self.assertTrue(first.ok)
                    again=cad.walls.convert.answer(doc,**args,request_id='contour-once')
                    self.assertTrue(again.replayed)
                    self.assertEqual(first.output,again.output)
                    self.assertEqual(len(doc),2)
                    made=doc.entity(first.output.uids[0]).entity
                    self.assertEqual(doc.entity(source).entity,original)
                    self.assertEqual(doc.journal()['calls'][-1]['command'],'cad.walls.convert')
                    doc.undo();self.assertEqual(len(doc),1)
                    doc.redo();self.assertEqual(doc.entity(first.output.uids[0]).entity,made)
                    with tempfile.TemporaryDirectory() as tmp:
                        path=Path(tmp)/'contour.kcad';doc.save(path);back=cad.Document.open(path)
                        self.assertEqual(back.entity(source).entity,original)
                        self.assertEqual(back.entity(first.output.uids[0]).entity,made)
                        self.assertEqual(back.info().settings.plot_scale,100)
    def test_holes_source_deletion_and_repeat_are_atomic(self):
        doc=cad.Document.new('Avlu',workspace='cad',drawing_unit='m',plot_scale=100)
        source=cad.polygon.create(doc,layer_id=doc.active_layer,pts=[(0,0),(10,0),(10,8),(0,8)],holes=[cad.RingGeometry(pts=[(4,3),(6,3),(6,5),(4,5)])]).uid
        original=doc.entity(source).entity
        made=cad.walls.convert(doc,uids=[source],mode='contours',justify='left',keep_sources=False)
        self.assertEqual(len(made.uids),2);self.assertEqual(len(doc),2)
        doc.undo();self.assertEqual(doc.entity(source).entity,original)
        made=cad.walls.convert(doc,uids=[source],mode='contours',justify='left',keep_sources=True)
        before=doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as error:cad.walls.convert(doc,uids=[source],mode='contours',justify='left',keep_sources=True)
        self.assertEqual(error.exception.code,'invalid_wall');self.assertEqual(doc.to_bytes(),before)

    def test_shared_axes_are_hostable_and_keep_atomic_history_and_persistence(self):
        doc=cad.Document.new('Ortak duvar',workspace='cad',drawing_unit='mm',plot_scale=100)
        sources=[cad.polygon.create(doc,layer_id=doc.active_layer,pts=[(x,0),(x+4,0),(x+4,3),(x,3)],attrs={'oda':name}).uid for x,name in [(0,'A'),(4,'B')]]
        before=[doc.entity(u).entity for u in sources]
        args=dict(uids=sources,mode='contours',justify='center',keep_sources=True,width=.2)
        plan=cad.walls.convert.plan(doc,**args)
        answer=cad.walls.convert.answer(doc,op='plan',**args)
        self.assertTrue(answer.ok)
        self.assertEqual([w.code for w in answer.warnings],['wall_shared_boundary'])
        self.assertEqual(answer.output,plan)
        self.assertEqual(len(doc),2);self.assertEqual(len(plan.walls),3)
        with self.assertWarns(cad.CommandWarning):made=cad.walls.convert(doc,**args,expected_revision=plan.revision)
        self.assertEqual(len(doc),5)
        doc.undo();self.assertEqual(len(doc),2);doc.redo();self.assertEqual(len(doc),5)
        self.assertEqual([doc.entity(u).entity for u in sources],before)
        divider=next(u for u in made.uids if len(doc.entity(u).entity.pts)==2)
        door=cad.openings.create(doc,layer_id=doc.active_layer,openings=[cad.NewOpening(host=divider,at=1.5,width=.9,form='door')]).uids[0]
        self.assertEqual(doc.entity(door).entity.host,divider)
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'shared.kcad';doc.save(path);back=cad.Document.open(path)
            self.assertEqual(back.entity(door).entity.host,divider)
            self.assertEqual([back.entity(u).entity for u in sources],before)
            unchanged=back.to_bytes()
            with self.assertRaises(cad.CommandFailed):cad.walls.convert(back,**args)
            self.assertEqual(back.to_bytes(),unchanged)

    def test_explicit_levels_are_typed_atomic_and_persist_without_attribute_guessing(self):
        doc=cad.Document.new('Katlı kontur',workspace='cad',drawing_unit='m',plot_scale=100)
        cad.settings.set(doc,arch=cad.ArchSettings(levels=[
            cad.ArchLevel(id='zemin',name='Zemin kat',elevation=0,height=3),
            cad.ArchLevel(id='kat-1',name='1. kat',elevation=0,height=3)]))
        source=cad.polygon.create(doc,layer_id=doc.active_layer,pts=[(0,0),(4,0),(4,3),(0,3)],attrs={'level':'kat-1'}).uid
        original=doc.entity(source).entity
        made=[]
        for level in (None,'zemin','kat-1'):
            args=dict(uids=[source],mode='contours',justify='center',keep_sources=True)
            if level is not None:args['arch']=cad.ArchInfo(level=level)
            before=doc.to_bytes()
            plan=cad.walls.convert.plan(doc,**args)
            self.assertEqual(doc.to_bytes(),before)
            done=cad.walls.convert(doc,**args,expected_revision=plan.revision)
            uid=done.uids[0];made.append(uid)
            wall=doc.entity(uid).entity
            self.assertEqual(wall.to_json().get('arch',{}).get('level'),level)
            count=len(doc)
            doc.undo();self.assertEqual(len(doc),count-1)
            doc.redo();self.assertEqual(doc.entity(uid).entity,wall)
            unchanged=doc.to_bytes()
            with self.assertRaises(cad.CommandFailed) as error:cad.walls.convert(doc,**args)
            self.assertEqual(error.exception.code,'invalid_wall')
            self.assertEqual(doc.to_bytes(),unchanged)
        before=doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as error:
            cad.walls.convert(doc,uids=[source],mode='contours',keep_sources=False,arch=cad.ArchInfo(level='missing-floor'))
        self.assertEqual(error.exception.code,'level_not_found')
        self.assertEqual(doc.to_bytes(),before)
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'levels.kcad';doc.save(path);back=cad.Document.open(path)
            self.assertEqual(back.entity(source).entity,original)
            self.assertEqual([back.entity(u).entity for u in made],[doc.entity(u).entity for u in made])
            self.assertEqual(back.info().settings.arch,doc.info().settings.arch)
