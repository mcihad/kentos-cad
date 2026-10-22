"""Analytic paper geometry through the real Python command host, including saved source metadata."""
import copy,json,math,unittest
from pathlib import Path
import kentos.cad as cad
from kentos.cad.errors import CommandFailed
ROOT=Path(__file__).resolve().parents[2]
class PaperCad(unittest.TestCase):
    def test_analytic_geometry_survives_commands_undo_and_reopen(self):
        data=json.loads((ROOT/'fixtures/sheet/v1/paper-cad.json').read_text())
        original=json.loads((ROOT/'fixtures/commands/v1/cad.sheets.edit.json').read_text())['cases'][0]['steps'][0]['input']['ops'][0]['sheet']
        for case in data['cases']:
            with self.subTest(case=case['name']):
                e=case['entity'];kind=e['kind'];point=lambda p:[p['x']*1e6,p['y']*1e6]
                if kind in ['circle','arc']:
                    geometry=dict(type=kind,center=point(e['c']),radius=e['r']*1e6)
                    if kind=='arc':geometry.update(start=e['a0'],sweep=e['a1']-e['a0'])
                elif kind=='ellipse':geometry=dict(type=kind,center=point(e['c']),major=point(e['major']),ratio=e['ratio'],start=e['t0'],sweep=e['t1']-e['t0'])
                elif kind=='nurbs':geometry=dict(type=kind,degree=e['degree'],control=list(map(point,e['ctrl'])),knots=e['knots'],weights=e['weights'],fit=list(map(point,e['fit'])),tangents=[[p['x'],p['y']] for p in e['tangents']],fitTolerance=e['fitTolerance']*1e6)
                else:geometry=dict(type='polyline',paths=[dict(points=list(map(point,e['pts'])),bulges=e['bulges'],closed=e['closed'],widths=[[a*1e6,b*1e6] for a,b in e['widths']])])
                sheet=copy.deepcopy(original);item=copy.deepcopy(sheet['items'][0]);sheet['items']=[item]
                item['id']='curve';item['name']='Eğri';item['frame']=dict(left=10000,top=10000,width=250000,height=200000)
                item['kind']=dict(type='shape',shape=dict(type='cad',reference=dict(width=250000,height=200000),geometry=geometry),stroke=dict(color='#1747aa',width=250))
                item['cadOrigin']=dict(layout='Layout1',record=0,entityKind=kind,file='analytic.dxf',sha256='a'*64,layer='0')
                doc=cad.Document.new('Analitik',workspace='cad')
                with doc.group('Analitik pafta'):
                    cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=sheet)])
                got=cad.sheets.set(doc,details=True).book.to_json()['sheets'][0]['items'][0]
                self.assertEqual(got['kind']['shape']['geometry']['type'],geometry['type'])
                self.assertEqual(got['cadOrigin'],item['cadOrigin'])
                saved=doc.to_bytes();reopened=cad.Document.from_bytes(saved)
                self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),cad.sheets.set(doc,details=True).book.to_json())
                doc.undo();self.assertEqual(doc.sheets(),[]);doc.redo();self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),cad.sheets.set(reopened,details=True).book.to_json())
