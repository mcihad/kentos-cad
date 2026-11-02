"""CAD-ADV-05 circular path edges share commands, persistence and atomicity with the desktop."""
import json
import math
import unittest
from pathlib import Path
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]

def drawing(origin=(0.0,0.0)):
    raw=json.loads((ROOT/'fixtures/interaction/v1/constraint-arc-edge.kcad').read_text())
    for e in raw['entities']:
        for p in e['pts'] if e['kind']=='polyline' else [e['a'],e['b']]:
            p['x']+=origin[0];p['y']+=origin[1]
    d=cad.Document.from_bytes(json.dumps(raw).encode())
    uid=next(d.entities()).uid
    def vertex(i):return dict(uid=uid,element=dict(kind='point',place=dict(kind='vertex',part=0,ring=0,index=i)))
    arc=cad.ArcEdgeCadElement(part=0,ring=0,index=0)
    assert cad.CadElement.from_json(arc.to_json())==arc
    rules=[dict(rule=dict(kind='fix',a=vertex(0))),dict(rule=dict(kind='fix',a=vertex(1))),dict(rule=dict(kind='radius',a=dict(uid=uid,element=arc.to_json()),parameter='r1'),value='2.5m')]
    cad.constraints.edit(d,edit=cad.AddConstraintEdit.from_json(dict(kind='add',constraints=rules)))
    return d,uid

class ArcEdgeConstraints(unittest.TestCase):
    def test_radius_plan_execute_file_and_undo_preserve_the_original_path(self):
        d,uid=drawing();before=d.to_bytes();old=d.entity(uid).entity.to_json()
        edits=[cad.SetParameterEdit(name='r1',expression='3m')]
        plan=cad.parameters.edit.plan(d,edits=edits);self.assertEqual(d.to_bytes(),before)
        cad.parameters.edit(d,edits=edits);e=d.entity(uid).entity.to_json()
        self.assertEqual(e,plan.changed[0].to_json())
        for k in ['kind','id','layerId','attrs','pts','widths']:self.assertEqual(e[k],old[k])
        self.assertAlmostEqual(e['bulges'][0],(3-math.sqrt(5))/2,places=9)
        q=cad.constraints.query(d).to_json();self.assertEqual(q['diagnosis']['dof'],0);self.assertTrue(all(r['holds'] for r in q['constraints']))
        saved=d.to_bytes();reopened=cad.Document.from_bytes(saved);self.assertEqual(reopened.to_bytes(),saved)
        self.assertTrue(all(r['holds'] for r in cad.constraints.query(reopened).to_json()['constraints']))
        d.undo();self.assertEqual(d.to_bytes(),before)
    def test_impossible_radius_is_atomic(self):
        d,_=drawing();before=d.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:cad.parameters.edit(d,edits=[cad.SetParameterEdit(name='r1',expression='1.5m')])
        self.assertEqual(caught.exception.code,'constraint_conflict')
        self.assertIn('sabit uçları 4 m uzaklıkta; yarıçap en az 2 m olmalı',str(caught.exception))
        self.assertEqual(d.to_bytes(),before)

    def test_large_coordinate_saved_tangent_and_length_share_a_readonly_roundtrip_certificate(self):
        d,uid=drawing((487000.0,4420000.0));line=list(d.entities())[1].uid
        v=dict(uid=uid,element=dict(kind='point',place=dict(kind='vertex',part=0,ring=0,index=0)))
        p=lambda place:dict(uid=line,element=dict(kind='point',place=dict(kind=place)))
        def add(cs):cad.constraints.edit(d,edit=cad.AddConstraintEdit.from_json(dict(kind='add',constraints=cs)))
        add([dict(rule=dict(kind='coincident',a=v,b=p('end')))])
        add([dict(rule=dict(kind='distance',a=p('start'),b=p('end'),parameter='boy'),value='2.5m'),
             dict(rule=dict(kind='tangent',a=dict(uid=line,element=dict(kind='segment',part=0,ring=0,index=0)),
                            b=dict(uid=uid,element=dict(kind='arcEdge',part=0,ring=0,index=0))))])
        before=d.to_bytes();cad.parameters.edit(d,edits=[cad.SetParameterEdit(name='r1',expression='3m')]);saved=d.to_bytes()
        q=cad.constraints.query(d).to_json();self.assertTrue(all(r['holds'] for r in q['constraints']),q)
        self.assertEqual(d.to_bytes(),saved)
        again=cad.Document.from_bytes(saved);self.assertTrue(all(r['holds'] for r in cad.constraints.query(again).to_json()['constraints']))
        d.undo();self.assertEqual(d.to_bytes(),before)

if __name__=='__main__':unittest.main()
