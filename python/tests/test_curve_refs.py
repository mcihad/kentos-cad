"""CAD-ADV-04: weighted curve reference identity through the public SDK."""
import json
import math
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing():
    raw = json.loads((ROOT / 'fixtures/interaction/v1/objects.kcad').read_text())
    raw['name'] = 'CAD-ADV-04 — Bağlı NURBS ölçüleri'
    raw['settings'].update(workspace='cad', cadWork='architecture', srid=0,
                           drawingUnit='m', plotScale=10)
    raw['origin'] = dict(x=0, y=0)
    raw['entities'] = [
        dict(kind='nurbs', id=1, layerId='cizim', attrs={'QA': 'weighted'}, degree=2,
             ctrl=[dict(x=1,y=0),dict(x=1,y=1),dict(x=0,y=1)],
             knots=[2,2,2,6,6,6],weights=[1,math.sqrt(.5),1]),
        dict(kind='dimension',id=2,layerId='cizim',attrs={},a=dict(x=1,y=0),
             b=dict(x=0,y=1),height=.05,offset=.2),
        dict(kind='dimension',id=3,layerId='cizim',attrs={},a=dict(x=1,y=0),
             b=dict(x=.7,y=.7),height=.05,offset=.4),
    ]
    doc = cad.Document.from_bytes(json.dumps(raw).encode())
    source, end_dim, middle_dim = [r.uid for r in doc.entities()]
    for uid, place in [(end_dim,cad.EndSubPlace()),
                       (middle_dim,cad.AlongSubPlace(part=0,ring=0,index=0,t=.5))]:
        cad.dimensions.reassociate(doc,uid=uid,points=[
            cad.PointBinding(point='a',to=cad.PlaceOn(uid=source,place=cad.StartSubPlace())),
            cad.PointBinding(point='b',to=cad.PlaceOn(uid=source,place=place)),
        ])
    return doc, source, end_dim, middle_dim


class CurveRefs(unittest.TestCase):
    def test_control_edit_and_knot_keep_live_refs_with_atomic_undo(self):
        doc, source, end_dim, middle_dim = drawing()
        before = doc.to_bytes()
        cad.curves.edit(doc,uid=source,edit=cad.MoveControlCurveEdit(index=0,at=(2,0)))
        end = doc.entity(end_dim).entity.to_json()
        middle = doc.entity(middle_dim).entity.to_json()
        self.assertEqual(end['a'],dict(x=2,y=0))
        self.assertEqual(middle['a'],dict(x=2,y=0))
        self.assertAlmostEqual(middle['b']['x'],1,places=13)
        self.assertAlmostEqual(middle['b']['y'],math.sqrt(.5),places=13)
        self.assertNotIn('broken',middle['refs']['b'])
        self.assertEqual(doc.undo(),'Eğriyi düzenle')
        self.assertEqual(doc.to_bytes(),before)
        cad.curves.edit(doc,uid=source,edit=cad.InsertKnotCurveEdit(at=3))
        self.assertNotIn('broken',doc.entity(middle_dim).entity.to_json()['refs']['b'])
        doc.undo()
        self.assertEqual(doc.to_bytes(),before)

    def test_exact_split_moves_the_end_reference_to_the_new_piece_and_roundtrips(self):
        doc, source, end_dim, middle_dim = drawing()
        before = doc.to_bytes()
        cad.entities.break_(doc,uid=source,first=(math.sqrt(.5),math.sqrt(.5)))
        end = doc.entity(end_dim).entity.to_json()
        self.assertNotEqual(end['refs']['b']['uid'],source)
        self.assertNotIn('broken',end['refs']['b'])
        self.assertNotIn('broken',doc.entity(middle_dim).entity.to_json()['refs']['b'])
        saved = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(),saved)
        self.assertEqual(doc.undo(),'Kır')
        self.assertEqual(doc.to_bytes(),before)


if __name__ == '__main__':
    unittest.main()
