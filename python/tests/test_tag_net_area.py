"""Net room tags through the public SDK; rectangle/Decimal and circle formulas are independent."""
import copy
from decimal import Decimal, ROUND_HALF_UP
import json
import math
from pathlib import Path
import tempfile
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
CONTENT = '{number} {name}\nAlan {area}\nNet {netArea} / {netArea}'

class TagNetArea(unittest.TestCase):
    def drawing(self, method='gross', columns='net', explicit=True, unassigned=False):
        setup = json.loads((ROOT/'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
        wall = dict(kind='wall',id=1,layerId='duvar',attrs={},width=.2,closed=True,
                    pts=[dict(x=0,y=0),dict(x=6,y=0),dict(x=6,y=4),dict(x=0,y=4)],arch=dict(level='zemin'))
        column = dict(kind='column',id=2,layerId='kolon',attrs={},at=dict(x=3,y=2),width=.5,arch=dict(level='zemin'))
        other_wall=copy.deepcopy(wall);other_wall.update(id=3,width=1.2,arch=dict(level='kat-1'))
        other_column=copy.deepcopy(column);other_column.update(id=4,width=1.5,arch=dict(level='kat-1'))
        if unassigned:
            wall['arch']={};column['arch']={}
        setup['entities']=[wall,column,other_wall,other_column]
        doc=cad.Document.from_bytes(json.dumps(setup).encode())
        ids={r.entity.id:r.uid for r in doc.entities()}
        fields=dict(seed=dict(x=1,y=2),number='N-01',name='Çalışma',method=method,columns=columns,arch=dict(level='zemin'))
        if unassigned:fields.update(arch={},scope='unassigned')
        if explicit:fields['sources']=[ids[1],ids[2]]
        source=cad.spaces.create(doc,layer_id='mahal',spaces=[cad.NewSpace.from_json(fields)]).uids[0]
        before=doc.to_bytes()
        args=dict(layer_id='etiket',tags=[cad.NewTag.from_json(dict(of=source,at=dict(x=10,y=5),leader=True,content=CONTENT))])
        plan=cad.tags.create.plan(doc,**args);self.assertEqual(doc.to_bytes(),before)
        tag=cad.tags.create(doc,**args,expected_revision=plan.revision).uids[0]
        planned=plan.tags[0].to_json();stored=doc.entity(tag).entity.to_json();planned['id']=stored['id']
        self.assertEqual(stored,planned)
        return doc,ids,source,tag

    def rows(self,doc):return {r.uid:r.entity.to_json() for r in doc.entities()}
    def text(self,doc,tag):return doc.entity(tag).entity.to_json()['text']
    def area(self,number,unit='m2',decimals=2):
        ratio=dict(m2=Decimal(1),donum=Decimal(1000),ha=Decimal(10000))[unit]
        word=dict(m2='m²',donum='dönüm',ha='ha')[unit]
        return f'{(Decimal(str(number))/ratio).quantize(Decimal(1).scaleb(-decimals),rounding=ROUND_HALF_UP)} {word}'

    def test_twenty_seven_methods_column_policies_area_units_and_saved_files(self):
        for method,selected in [('net','22.04'),('centerline','24'),('gross','26.04')]:
            for columns in ['ignore','net','all']:
                for unit in ['m2','donum','ha']:
                    with self.subTest(method=method,columns=columns,unit=unit):
                        doc,_,source,tag=self.drawing(method,columns)
                        net=Decimal('22.04')-(Decimal('.25') if columns!='ignore' else Decimal(0))
                        selected=Decimal(selected)
                        chosen=selected-(Decimal('.25') if columns=='all' or (columns=='net' and method=='net') else Decimal(0))
                        original=self.rows(doc)[source]
                        cad.settings.set(doc,area_unit=unit,area_decimals=4,drawing_unit='mm')
                        expected=f'N-01 Çalışma\nAlan {self.area(chosen,unit,4)}\nNet {self.area(net,unit,4)} / {self.area(net,unit,4)}'
                        self.assertEqual(self.text(doc,tag),expected)
                        self.assertEqual(self.rows(doc)[source],original)
                        with tempfile.TemporaryDirectory() as folder:
                            path=Path(folder)/'net.kcad';doc.save(path)
                            self.assertEqual(self.rows(cad.Document.open(path)),self.rows(doc))

    def test_net_only_column_change_leaves_gross_room_identical_and_single_history(self):
        for explicit in [False,True]:
            with self.subTest(explicit=explicit):
                doc,ids,source,tag=self.drawing(explicit=explicit)
                before=self.rows(doc)
                self.assertIn('Net 21.79 m²',self.text(doc,tag))
                cad.entities.transform(doc,uids=[ids[2]],transform=cad.MoveTransform(dx=10,dy=0))
                after=self.rows(doc)
                self.assertEqual(after[source],before[source])
                self.assertIn('Net 22.04 m²',self.text(doc,tag))
                for key in ['at','anchor','of','content','leader']:
                    self.assertEqual(after[tag][key],before[tag][key])
                self.assertEqual(after[ids[1]],before[ids[1]])
                doc.undo();self.assertEqual(self.rows(doc),before)
                doc.redo();self.assertEqual(self.rows(doc),after)
                with self.assertRaisesRegex(RuntimeError,'cancel'):
                    with doc.group('Cancel net source'):
                        cad.entities.transform(doc,uids=[ids[2]],transform=cad.MoveTransform(dx=-10,dy=0))
                        raise RuntimeError('cancel')
                self.assertEqual(self.rows(doc),after)

    def test_history_after_area_settings_change_uses_current_format(self):
        doc,ids,_,tag=self.drawing()
        cad.entities.transform(doc,uids=[ids[2]],transform=cad.MoveTransform(dx=10,dy=0))
        cad.settings.set(doc,area_unit='donum',area_decimals=4)
        self.assertIn('Net 0.0220 dönüm',self.text(doc,tag))
        doc.undo();self.assertIn('Net 0.0218 dönüm',self.text(doc,tag))
        doc.redo();self.assertIn('Net 0.0220 dönüm',self.text(doc,tag))

    def test_missing_explicit_source_is_unknown_then_undo_restores(self):
        doc,ids,source,tag=self.drawing();before=self.rows(doc)
        cad.entities.delete(doc,uids=[ids[2]])
        self.assertEqual(self.rows(doc)[source]['broken'],'sourceUnavailable')
        self.assertEqual(self.text(doc,tag),'N-01 Çalışma\nAlan 26.04 m²\nNet  /')
        doc.undo();self.assertEqual(self.rows(doc),before)

    def test_curved_sources_round_column_and_far_origin_are_analytic(self):
        setup=json.loads((ROOT/'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
        setup['entities']=[dict(kind='wall',id=1,layerId='duvar',attrs={},width=.2,closed=True,
            pts=[dict(x=499995,y=4400000),dict(x=500005,y=4400000)],bulges=[1,1]),
            dict(kind='column',id=2,layerId='kolon',attrs={},at=dict(x=500000,y=4400000),width=1,shape='round')]
        doc=cad.Document.from_bytes(json.dumps(setup).encode())
        source=cad.spaces.create(doc,layer_id='mahal',spaces=[cad.NewSpace.from_json(dict(seed=dict(x=500002,y=4400000),columns='net',method='gross'))]).uids[0]
        tag=cad.tags.create(doc,layer_id='etiket',tags=[cad.NewTag.from_json(dict(of=source,content='{area}\n{netArea}'))]).uids[0]
        self.assertEqual(self.text(doc,tag),f'{math.pi*5.1**2:.2f} m²\n{math.pi*(4.9**2-.5**2):.2f} m²')

    def test_virtual_separator_uses_same_net_boundary_and_follows(self):
        doc,ids,source,tag=self.drawing(explicit=False)
        made=cad.spaces.separators(doc,operation=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=[dict(pts=[dict(x=2,y=0),dict(x=2,y=4)],arch=dict(level='zemin'))])))
        # Source seed x=1: clear width 1.9 and height 3.8; column is on the other side.
        self.assertIn('Net 7.22 m²',self.text(doc,tag))
        before=self.rows(doc)
        cad.entities.transform(doc,uids=made.uids,transform=cad.MoveTransform(dx=.5,dy=0))
        self.assertIn('Net 9.12 m²',self.text(doc,tag))
        doc.undo();self.assertEqual(self.rows(doc),before)

    def test_unavailable_net_area_and_refused_writes_are_explicit(self):
        doc,ids,source,tag=self.drawing();before=self.rows(doc)
        # Cover every inner face while gross geometry remains a valid recorded room.
        cad.entities.transform(doc,uids=[ids[2]],transform=cad.ScaleTransform(center=cad.Vec2(x=3,y=2),factor=20))
        self.assertEqual(self.rows(doc)[source],before[source])
        self.assertEqual(self.text(doc,tag),'N-01 Çalışma\nAlan 26.04 m²\nNet  /')
        doc.undo();self.assertEqual(self.rows(doc),before)
        args=dict(layer_id='kilitli',tags=[cad.NewTag.from_json(dict(of=source,content='{netArea}'))])
        unchanged=doc.to_bytes()
        for operation in [cad.tags.create.validate,cad.tags.create.plan,cad.tags.create]:
            with self.assertRaises(cad.CommandFailed) as raised:operation(doc,**args)
            self.assertEqual(raised.exception.code,'layer_locked');self.assertEqual(doc.to_bytes(),unchanged)

    def test_unassigned_scope_never_borrows_another_levels_sources(self):
        doc,ids,source,tag=self.drawing(explicit=False,unassigned=True)
        self.assertEqual(self.rows(doc)[source]['scope'],'unassigned')
        self.assertIn('Net 21.79 m²',self.text(doc,tag))
        before=self.rows(doc)
        cad.entities.transform(doc,uids=[ids[4]],transform=cad.MoveTransform(dx=10,dy=0))
        self.assertEqual(self.rows(doc)[source],before[source])
        self.assertEqual(self.rows(doc)[tag],before[tag])

    def test_net_area_and_elevations_share_one_template_and_keep_manual_placement(self):
        doc,_,source,tag=self.drawing()
        cad.spaces.edit(doc,uids=[source],operation=cad.SetSpacesOperation(height=2.75))
        before=self.rows(doc)
        content='{number} {name}\nAlan {area}\nNet {netArea}\n{floorElevation}/{ceilingElevation}'
        cad.tags.edit(doc,uids=[tag],change=cad.TagChange.from_json(dict(content=content)))
        self.assertEqual(self.text(doc,tag),'N-01 Çalışma\nAlan 26.04 m²\nNet 21.79 m²\n0.000/2.750')
        for key in ['at','anchor','of','size','shape','leader']:
            self.assertEqual(self.rows(doc)[tag].get(key),before[tag].get(key))
        doc.undo();self.assertEqual(self.rows(doc),before)
