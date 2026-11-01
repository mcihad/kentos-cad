"""CAD-ADV-05: bounded NURBS endpoints share native commands, units and Undo."""
import json
import unittest
from pathlib import Path
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]

def drawing():
    raw=json.loads((ROOT/'fixtures/interaction/v1/objects.kcad').read_text())
    raw['settings'].update(workspace='cad',drawingUnit='mm',srid=0)
    raw['origin']=dict(x=0,y=0)
    raw['entities']=[dict(kind='nurbs',id=1,layerId='cizim',attrs={'QA':'rational'},degree=3,ctrl=[dict(x=0,y=0),dict(x=1,y=3),dict(x=3,y=3),dict(x=4,y=3),dict(x=6,y=0)],knots=[2,2,2,2,3,8,8,8,8],weights=[1,2,.7,3,1],fit=[dict(x=0,y=0),dict(x=6,y=0)],fitTolerance=1e-5,tangents=[dict(x=1,y=2),dict(x=1,y=-2)]),dict(kind='line',id=2,layerId='cizim',attrs={},a=dict(x=6,y=0),b=dict(x=10,y=0))]
    d=cad.Document.from_bytes(json.dumps(raw).encode());uids=[r.uid for r in d.entities()]
    part=lambda i,p:dict(uid=uids[i],element=dict(kind='point',place=dict(kind=p)))
    a,b,l=part(0,'start'),part(0,'end'),part(1,'start')
    edit=cad.AddConstraintEdit.from_json(dict(kind='add',constraints=[dict(rule=dict(kind='fix',a=a)),dict(rule=dict(kind='horizontal',a=a,b=b)),dict(rule=dict(kind='distance',a=a,b=b,parameter='aciklik'),value='6m'),dict(rule=dict(kind='coincident',a=b,b=l))]))
    cad.constraints.edit(d,edit=edit)
    return d,uids

class NurbsConstraints(unittest.TestCase):
    def test_endpoint_parameter_preserves_source_and_single_undo(self):
        d,uids=drawing();before=d.to_bytes();old=d.entity(uids[0]).entity.to_json()
        edits=[cad.SetParameterEdit(name='aciklik',expression='8m')]
        cad.parameters.edit.plan(d,edits=edits);self.assertEqual(d.to_bytes(),before)
        written=cad.parameters.edit(d,edits=edits)
        n=d.entity(uids[0]).entity.to_json();l=d.entity(uids[1]).entity.to_json()
        self.assertEqual(n['ctrl'][:-1],old['ctrl'][:-1])
        self.assertAlmostEqual(n['ctrl'][-1]['x'],8,places=8)
        self.assertAlmostEqual(l['a']['x'],8,places=8)
        for k in ['degree','knots','weights','attrs']:self.assertEqual(n[k],old[k])
        for k in ['fit','fitTolerance','tangents']:self.assertNotIn(k,n)
        q=cad.constraints.query(d).to_json();a=q['diagnosis'];b=written.diagnosis.to_json()
        a.pop('iterations',None);b.pop('iterations',None);self.assertEqual(a,b)
        self.assertTrue(all(row['holds'] for row in q['constraints']))
        saved=d.to_bytes();self.assertEqual(cad.Document.from_bytes(saved).to_bytes(),saved)
        d.undo();self.assertEqual(d.to_bytes(),before)

if __name__=='__main__':unittest.main()
