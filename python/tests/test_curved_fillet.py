"""CAD-ADV-04: analytic tangent joins through the public typed Python API."""
import json
import math
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]

def drawing():
    raw = json.loads((ROOT / 'fixtures/interaction/v1/objects.kcad').read_text())
    raw['settings'].update(workspace='cad', srid=0, drawingUnit='m')
    raw['origin'] = dict(x=0, y=0)
    raw['entities'] = [
        dict(kind='line', id=1, layerId='cizim', attrs={'QA': 'line'},
             a=dict(x=-5, y=0), b=dict(x=10, y=0)),
        dict(kind='arc', id=2, layerId='cizim', attrs={'QA': 'arc'},
             c=dict(x=4, y=3), r=2, a0=math.pi, a1=0),
    ]
    return cad.Document.from_bytes(json.dumps(raw).encode())

class CurvedFillet(unittest.TestCase):
    def test_typed_plan_matches_the_ui_endpoint_pick_and_undo(self):
        doc = drawing()
        uids = [r.uid for r in doc.entities()]
        corner = cad.LinesCornerPick(first=cad.TrimPick(uid=uids[0], at=(0, 0)),
                                     second=cad.TrimPick(uid=uids[1], at=(2.049, 3.007)))
        before = doc.to_bytes()
        planned = cad.entities.fillet.plan(doc, corner=corner, radius=1)
        self.assertEqual(doc.to_bytes(), before)
        made = cad.entities.fillet(doc, corner=corner, radius=1)
        self.assertEqual(made.changed, uids)
        self.assertEqual(len(made.created), 1)
        records = {r.uid: r.entity.to_json() for r in doc.entities()}
        join = records[made.created[0]]
        self.assertEqual(join['kind'], 'arc')
        self.assertAlmostEqual(join['c']['x'], 4 - math.sqrt(5), places=10)
        self.assertAlmostEqual(join['c']['y'], 1, places=10)
        self.assertEqual(records[uids[1]]['a0'], math.pi)
        self.assertEqual(records[uids[1]]['attrs'], {'QA': 'arc'})
        self.assertEqual(planned.created[0].to_json()['r'], join['r'])
        reopened = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(reopened.to_bytes(), doc.to_bytes())
        self.assertEqual(doc.undo(), 'Köşe yuvarla')
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), reopened.to_bytes())

    def test_no_trim_and_failed_radius_preserve_the_sources(self):
        doc = drawing()
        before = {r.uid: r.entity.to_json() for r in doc.entities()}
        a, b = before
        corner = cad.LinesCornerPick(first=cad.TrimPick(uid=a, at=(0, 0)),
                                     second=cad.TrimPick(uid=b, at=(2, 3)))
        with self.assertRaises(cad.CommandFailed):
            cad.entities.fillet(doc, corner=corner, radius=-1)
        self.assertEqual({r.uid: r.entity.to_json() for r in doc.entities()}, before)
        cad.entities.fillet(doc, corner=corner, radius=1, trim=False)
        after = {r.uid: r.entity.to_json() for r in doc.entities()}
        for uid, entity in before.items():
            self.assertEqual(after[uid], entity)

    def test_associative_hatch_uses_the_connector_and_undo_restores_its_original_ring(self):
        raw = json.loads((ROOT / 'fixtures/interaction/v1/objects.kcad').read_text())
        raw['settings'].update(workspace='cad', srid=0, drawingUnit='m')
        raw['origin'] = dict(x=0, y=0)
        raw['entities'] = [
            dict(kind='line', id=1, layerId='cizim', attrs={}, a=dict(x=-5, y=3), b=dict(x=2, y=3)),
            dict(kind='arc', id=2, layerId='cizim', attrs={}, c=dict(x=4, y=3), r=2,
                 a0=math.pi, a1=1.5*math.pi),
            dict(kind='line', id=3, layerId='cizim', attrs={}, a=dict(x=4, y=1), b=dict(x=-5, y=1)),
            dict(kind='line', id=4, layerId='cizim', attrs={}, a=dict(x=-5, y=1), b=dict(x=-5, y=3)),
        ]
        doc = cad.Document.from_bytes(json.dumps(raw).encode())
        sources = [r.uid for r in doc.entities()]
        made = doc.run('cad.entities.hatch', dict(region=dict(kind='point', at=dict(x=0, y=2),
                       way='lines', associative=True), pattern=dict(name='Dolu'), layerId='cizim'))
        self.assertEqual(made['status'], 'completed', made)
        before = doc.to_bytes()
        corner = cad.LinesCornerPick(first=cad.TrimPick(uid=sources[0], at=(0, 3)),
                                     second=cad.TrimPick(uid=sources[1], at=(2.1, 2.4)))
        plan = cad.entities.fillet.plan(doc, corner=corner, radius=0.5)
        self.assertEqual(doc.to_bytes(), before)
        self.assertTrue(any(e.to_json()['kind'] == 'hatch' for e in plan.changed))
        proposal = cad.entities.fillet.answer(doc, op="plan", corner=corner, radius=0.5)
        fillet = cad.entities.fillet(doc, corner=corner, radius=0.5,
                                    expected=cad.Expected(plan=proposal.plan_id))
        hatch = next(r.entity.to_json() for r in doc.entities() if r.entity.to_json()['kind'] == 'hatch')
        self.assertEqual(len(hatch['assoc']['bounds']), 5)
        self.assertIn(fillet.created[0], hatch['assoc']['bounds'])
        after = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), after)
