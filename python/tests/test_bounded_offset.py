"""User-toleranced parallels through the public typed Python API."""
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
    raw['entities'] = [dict(kind='nurbs', id=1, layerId='cizim', attrs={'QA': 'source'},
        degree=2, ctrl=[dict(x=1, y=0), dict(x=1, y=1), dict(x=0, y=1)],
        knots=[0, 0, 0, 1, 1, 1], weights=[1, math.sqrt(0.5), 1], closed=False)]
    return cad.Document.from_bytes(json.dumps(raw).encode())


def chord_distance(p, a, b):
    dx, dy = b['x']-a['x'], b['y']-a['y']
    t = max(0, min(1, ((p[0]-a['x'])*dx+(p[1]-a['y'])*dy)/(dx*dx+dy*dy)))
    return math.hypot(p[0]-a['x']-t*dx, p[1]-a['y']-t*dy)


class BoundedOffset(unittest.TestCase):
    def test_consent_plan_bound_source_trace_and_undo(self):
        doc = drawing()
        source = next(doc.entities()).uid
        before = doc.to_bytes()
        by = cad.DistanceOffsetBy(distance=0.3, side=(2, 2))
        with self.assertRaises(cad.CommandFailed):
            cad.entities.offset(doc, uids=[source], by=by)
        self.assertEqual(doc.to_bytes(), before)
        approximation = cad.OffsetApproximation(tolerance=0.0001)
        planned = cad.entities.offset.plan(doc, uids=[source], by=by, approximation=approximation)
        self.assertEqual(doc.to_bytes(), before)
        answer = cad.entities.offset.answer(doc, op='plan', uids=[source], by=by,
                                            approximation=approximation)
        made = cad.entities.offset(doc, uids=[source], by=by, approximation=approximation,
                                   expected=cad.Expected(plan=answer.plan_id))
        self.assertEqual(made.approximations, planned.approximations)
        report = made.approximations[0]
        self.assertEqual(report.source, source)
        self.assertEqual(report.copy_index, 0)
        self.assertLessEqual(report.error_bound, approximation.tolerance)
        self.assertEqual(report.crossings, 0)
        result = next(r.entity.to_json() for r in doc.entities() if r.uid == made.created[0])
        pts = result['pts']
        self.assertEqual(report.vertices, len(pts))
        # This input is an exact rational quarter circle. Its true parallel
        # has independent radius 1.3, whatever the implementation samples.
        for i in range(1001):
            t = math.pi/2*i/1000
            p = (1.3*math.cos(t), 1.3*math.sin(t))
            self.assertLessEqual(min(chord_distance(p, a, b) for a, b in zip(pts, pts[1:])),
                                 report.error_bound)
        saved = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(), saved)
        self.assertEqual(doc.undo(), 'Ötele')
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), saved)

    def test_tighter_tolerance_and_failed_collapse_keep_the_source(self):
        doc = drawing()
        source = next(doc.entities()).uid
        before = doc.to_bytes()
        by = cad.DistanceOffsetBy(distance=0.3, side=(2, 2))
        a = cad.entities.offset.plan(doc, uids=[source], by=by,
                                    approximation=cad.OffsetApproximation(tolerance=0.001))
        b = cad.entities.offset.plan(doc, uids=[source], by=by,
                                    approximation=cad.OffsetApproximation(tolerance=0.00001))
        self.assertGreater(b.approximations[0].vertices, a.approximations[0].vertices)
        with self.assertRaises(cad.CommandFailed):
            cad.entities.offset(doc, uids=[source], by=cad.DistanceOffsetBy(distance=1.2, side=(0, 0)),
                approximation=cad.OffsetApproximation(tolerance=0.0001), delete_source=True)
        self.assertEqual(doc.to_bytes(), before)
