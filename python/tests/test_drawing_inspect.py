"""CAD-ADV-08: the typed and background query preserve source geometry and exact area."""
import json
import math
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing(entities):
    raw = json.loads((ROOT / 'fixtures/commands/v1/cad.blocks.find.json').read_text())['setup']
    raw['entities'] = [dict(e, id=i+1, layerId='a', attrs={}) for i, e in enumerate(entities)]
    return cad.Document.from_bytes(json.dumps(raw).encode())


class DrawingInspect(unittest.TestCase):
    def test_gap_threshold_and_job_are_read_only_and_equivalent(self):
        doc = drawing([
            dict(kind='line', a=dict(x=500000., y=4500000.), b=dict(x=500001., y=4500000.)),
            dict(kind='line', a=dict(x=500001.004, y=4500000.), b=dict(x=500002., y=4500000.)),
        ])
        before = [r.entity.to_json() for r in doc.entities()]
        result = cad.drawing.inspect(doc, max_gap=.005)
        gap = next(f for f in result.findings if f.code == 'boundary_gap')
        self.assertAlmostEqual(gap.distance, .004, places=8)
        job = doc.start('cad.drawing.inspect', cad.DrawingInspect(max_gap=.005)).result()
        self.assertEqual(job.status, 'completed')
        self.assertEqual(job.output.to_json(), result.to_json())
        self.assertFalse(any(f.code == 'boundary_gap' for f in cad.drawing.inspect(doc, max_gap=0).findings))
        self.assertEqual([r.entity.to_json() for r in doc.entities()], before)
        self.assertEqual(cad.drawing.inspect(doc, uids=[]).examined, 0)

    def test_analytic_area_survives_public_query_and_reopen(self):
        doc = drawing([
            dict(kind='circle', c=dict(x=0., y=0.), r=10.),
            dict(kind='circle', c=dict(x=-3., y=0.), r=1.),
            dict(kind='circle', c=dict(x=3., y=0.), r=1.),
        ])
        created = doc.call('cad.entities.hatch', dict(
            region=dict(kind='point', at=dict(x=0., y=4.), associative=True),
            pattern=dict(name='ANSI31', origin=dict(kind='point', at=dict(x=.123, y=.456))), layerId='a'))
        self.assertEqual(created.status, 'completed')
        result = cad.drawing.inspect(doc, checks=['hatches'])
        self.assertEqual(len(result.hatches), 1)
        self.assertAlmostEqual(result.hatches[0].area, 98*math.pi, places=6)
        reopened = cad.Document.from_bytes(doc.to_bytes())
        again = cad.drawing.inspect(reopened, checks=['hatches'])
        self.assertAlmostEqual(again.hatches[0].area, result.hatches[0].area, places=8)
        self.assertEqual(again.hatches[0].pattern_origin, result.hatches[0].pattern_origin)
