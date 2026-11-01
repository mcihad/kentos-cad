"""CAD-ADV-05: ellipse constraints share the desktop's command/diagnosis path."""
import json
import unittest
from pathlib import Path
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]

def drawing():
    raw=json.loads((ROOT/'fixtures/interaction/v1/objects.kcad').read_text())
    raw['name']='CAD-ADV-05 — Elips kısıtları'
    raw['settings'].update(workspace='cad',cadWork='architecture',srid=0,drawingUnit='m',angleUnit='deg',plotScale=50)
    raw['origin']=dict(x=0,y=0)
    raw['entities']=[dict(kind='ellipse',id=1,layerId='cizim',attrs={'QA':'analytic'},c=dict(x=0,y=0),major=dict(x=4,y=0),ratio=.5,t0=0,t1=0),dict(kind='line',id=2,layerId='cizim',attrs={},a=dict(x=4,y=0),b=dict(x=7,y=0))]
    d=cad.Document.from_bytes(json.dumps(raw).encode()); uids=[r.uid for r in d.entities()]
    def part(i,p):return dict(uid=uids[i],element=dict(kind='point',place=dict(kind=p)))
    c,s,l=part(0,'center'),part(0,'start'),part(1,'start')
    cs=[dict(rule=dict(kind='fix',a=c)),dict(rule=dict(kind='horizontal',a=c,b=s)),dict(rule=dict(kind='distance',a=c,b=s,parameter='eksen'),value='4'),dict(rule=dict(kind='coincident',a=s,b=l))]
    cad.constraints.edit(d,edit=cad.AddConstraintEdit.from_json(dict(kind='add',constraints=cs)))
    return d,uids

class EllipseConstraints(unittest.TestCase):
    def test_axis_drives_true_ellipse_and_attached_line_then_undoes(self):
        d,uids=drawing();before=d.to_bytes()
        cad.parameters.edit.plan(d,edits=[cad.SetParameterEdit(name='eksen',expression='6')])
        self.assertEqual(d.to_bytes(),before)
        out=cad.parameters.edit(d,edits=[cad.SetParameterEdit(name='eksen',expression='6')])
        e=d.entity(uids[0]).entity.to_json();l=d.entity(uids[1]).entity.to_json()
        self.assertEqual(e['kind'],'ellipse');self.assertAlmostEqual(e['major']['x'],6,places=9)
        self.assertAlmostEqual(e['ratio'],1/3,places=9);self.assertAlmostEqual(l['a']['x'],6,places=9)
        self.assertEqual(e['t0'],e['t1']);q=cad.constraints.query(d)
        a,b=q.diagnosis.to_json(),out.diagnosis.to_json()
        a.pop('iterations',None);b.pop('iterations',None)
        self.assertEqual(a,b)
        self.assertFalse(q.diagnosis.broken);saved=d.to_bytes()
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(),saved)
        d.undo();self.assertEqual(d.to_bytes(),before)


    def test_quarter_turn_is_atomic_plannable_and_keeps_axes(self):
        d,uids=drawing()
        q=cad.constraints.query(d).to_json()
        horizontal=next(row['constraint']['id'] for row in q['constraints'] if row['constraint']['rule']['kind']=='horizontal')
        cad.constraints.edit(d,edit=cad.RemoveConstraintEdit(ids=[horizontal]))
        before=d.to_bytes()
        part=lambda p:dict(uid=uids[0],element=dict(kind='point',place=dict(kind=p)))
        edit=cad.AddConstraintEdit.from_json(dict(kind='add',constraints=[dict(rule=dict(kind='vertical',a=part('center'),b=part('start')))]))
        cad.constraints.edit.plan(d,edit=edit)
        self.assertEqual(d.to_bytes(),before)
        cad.constraints.edit(d,edit=edit)
        e=d.entity(uids[0]).entity.to_json();l=d.entity(uids[1]).entity.to_json()
        self.assertAlmostEqual(e['major']['x'],0,places=8)
        self.assertAlmostEqual(abs(e['major']['y']),4,places=8)
        self.assertAlmostEqual(e['ratio'],.5,places=8)
        self.assertAlmostEqual(l['a']['x'],e['major']['x'],places=8)
        self.assertAlmostEqual(l['a']['y'],e['major']['y'],places=8)
        self.assertEqual(e['t0'],e['t1'])
        self.assertFalse(cad.constraints.query(d).diagnosis.conflicting)
        d.undo();self.assertEqual(d.to_bytes(),before)

if __name__=='__main__':unittest.main()
