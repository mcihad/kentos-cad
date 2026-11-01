"""CAD-ADV-04: curved holed split identity through the typed public API."""
import json
import math
from pathlib import Path
import unittest
from kentos import cad

ROOT=Path(__file__).resolve().parents[2]

def drawing():
    raw=json.loads((ROOT/'fixtures/interaction/v1/objects.kcad').read_text())
    raw['name']='CAD-ADV-04 — Yaylı delikli alan'
    raw['settings'].update(workspace='cad',cadWork='architecture',srid=0,drawingUnit='m',plotScale=50)
    raw['origin']=dict(x=0,y=0)
    def pts(values): return [dict(x=x,y=y) for x,y in values]
    raw['entities']=[dict(kind='polygon',id=1,layerId='cizim',attrs={'QA':'capsule'},
        pts=pts([(0,0),(4,0),(4,2),(0,2)]),bulges=[0,1,0,1],
        holes=[dict(pts=pts([(.5,.5),(1.5,.5),(1.5,1.5),(.5,1.5)]))])]
    doc=cad.Document.from_bytes(json.dumps(raw).encode()); source=next(doc.entities()).uid
    hatches=[]
    for x in [1,3]:
        out=cad.entities.hatch(doc,region=cad.PointHatchRegion(at=(x,1.75),associative=True),
            pattern=cad.HatchPatternSpec(name='ANSI31'),layer_id='cizim')
        hatches.append(out.created[0])
    return doc,source,hatches

class AreaCurveTies(unittest.TestCase):
    def test_split_keeps_curved_boundary_and_moves_each_hatch_to_its_seed_piece(self):
        doc,source,hatches=drawing(); before=doc.to_bytes()
        work=cad.SplitAreaWork(ids=[source],line=[(2,-1),(2,3)])
        cad.areas.edit.plan(doc,work=work); self.assertEqual(doc.to_bytes(),before)
        out=cad.areas.edit(doc,work=work); self.assertAlmostEqual(out.size,7+math.pi,places=12)
        for hatch,outer,holes in [(hatches[0],source,1),(hatches[1],out.created[0],0)]:
            e=doc.entity(hatch).entity.to_json()
            self.assertEqual(e['assoc']['outer'],outer)
            self.assertEqual(len(e.get('holes',[])),holes)
            self.assertTrue(any(any(abs(v)>.99 for v in loop.get('bulges',[])) for loop in e['boundary']))
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(),doc.to_bytes())
        self.assertEqual(doc.undo(),'Alan böl'); self.assertEqual(doc.to_bytes(),before)

if __name__=='__main__': unittest.main()
