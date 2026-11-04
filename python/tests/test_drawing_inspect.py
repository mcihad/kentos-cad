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

    def test_typed_origin_edit_preserves_every_other_hatch_property_and_undo(self):
        doc = drawing([dict(kind='circle', c=dict(x=0., y=0.), r=5.)])
        made = doc.call('cad.entities.hatch', dict(
            region=dict(kind='point', at=dict(x=0., y=0.), associative=True),
            pattern=dict(name='ANSI31', angle=31., scale=2.), layerId='a'))
        self.assertEqual(made.status, 'completed')
        row = next(r for r in doc.entities() if r.entity.to_json()['kind'] == 'hatch')
        before = row.entity.to_json()
        cad.hatches.edit(doc, edit=cad.OriginHatchEdit(
            uids=[row.uid], origin=cad.PointHatchOrigin(at=cad.Vec2(x=.123, y=.456))))
        current = next(r.entity.to_json() for r in doc.entities() if r.uid == row.uid)
        expected = json.loads(json.dumps(before))
        expected['pattern']['origin'] = dict(x=.123, y=.456)
        self.assertEqual(current, expected)
        self.assertEqual(doc.undo(), 'Tarama başlangıcı')
        self.assertEqual(next(r.entity.to_json() for r in doc.entities() if r.uid == row.uid), before)

    def test_annotations_expose_true_measure_apart_from_override_and_match_query(self):
        doc = drawing([dict(kind='dimension', a=dict(x=0., y=0.), b=dict(x=2.5, y=0.),
                            offset=.3, height=.125, text='2400')])
        before = doc.to_bytes()
        result = cad.drawing.inspect(doc, checks=['annotations'])
        values = cad.dimensions.query(doc)
        self.assertEqual([d.to_json() for d in result.dimensions], [d.to_json() for d in values.dimensions])
        self.assertEqual(result.dimensions[0].measured, 2.5)
        self.assertEqual(result.dimensions[0].shown, '2400')
        self.assertTrue(result.dimensions[0].overridden)
        self.assertEqual({f.code for f in result.findings}, {'dimension_free', 'dimension_override'})
        self.assertEqual(doc.to_bytes(), before)

    def test_reference_snap_keeps_coincident_sources_without_changing_default_cycle(self):
        doc = drawing([
            dict(kind='line', a=dict(x=0., y=0.), b=dict(x=2., y=0.)),
            dict(kind='line', a=dict(x=2., y=0.), b=dict(x=2., y=2.)),
        ])
        before = doc.to_bytes()
        at = cad.Vec2(x=2., y=0.)
        ordinary = cad.snap.find(doc, at=at, aperture=.01, kinds=['endpoint'], all=True)
        sources = cad.snap.find(doc, at=at, aperture=.01, kinds=['endpoint'], all=True, sources=True)
        self.assertEqual(len(ordinary.snaps), 1)
        self.assertEqual(len(sources.snaps), 2)
        self.assertNotEqual(sources.snaps[0].uid, sources.snaps[1].uid)
        self.assertEqual(sources.snaps[0].point, sources.snaps[1].point)
        self.assertEqual(doc.to_bytes(), before)

    def test_unicode_profile_metadata_and_font_diagnostics_survive_python_save_reopen(self):
        words='Ög\u0306ç ⌀32 — العربية עברית'
        doc=drawing([dict(kind='text',p=dict(x=5000000.,y=4400000.),height=.18,rotation=37.,
                          text=words,font='arimo',layout=dict(engine='unicode-v1'))])
        before=doc.to_bytes()
        result=cad.drawing.inspect(doc,checks=['annotations'])
        fallback=next(f for f in result.findings if f.code=='text_font_fallback')
        self.assertEqual(fallback.severity,'info')
        self.assertIn('noto-sans-arabic',fallback.message)
        self.assertEqual(doc.to_bytes(),before)
        reopened=cad.Document.from_bytes(before)
        entity=next(iter(reopened.entities())).entity.to_json()
        self.assertEqual(entity['text'],words)
        self.assertEqual(entity['layout']['engine'],'unicode-v1')
        self.assertEqual(reopened.to_bytes(),before)
