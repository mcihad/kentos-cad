"""CAD-ADV-07: independent transformed symbols through the public query and job APIs."""
from pathlib import Path
import json
import math
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing():
    raw = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
    raw["name"] = "CAD-ADV-07 — 40 döndürülmüş ve aynalanmış sembol"
    raw["settings"].update(drawingUnit="mm", cadWork="architecture")
    objects = []
    for i in range(42):
        angle = i * .17 if i < 40 else .31
        scale = 2. if i == 40 else 1.
        mirror = i % 2 == 1 or i == 40
        ox, oy = (i % 10) * 10., (i // 10) * 10.

        def point(x, y):
            y = -y if mirror else y
            return dict(x=ox+scale*(x*math.cos(angle)-y*math.sin(angle)),
                        y=oy+scale*(x*math.sin(angle)+y*math.cos(angle)))

        a0, a1 = ((angle-1.7, angle-.2) if mirror else (angle+.2, angle+1.7))
        shapes = [dict(kind="line", a=point(0, 0), b=point(2, 0)),
                  dict(kind="line", a=point(0, 0), b=point(0, 1)),
                  dict(kind="circle", c=point(.6, .3), r=(.14 if i == 41 else .13)*scale),
                  dict(kind="arc", c=point(1, .5), r=.2*scale, a0=a0, a1=a1)]
        for shape in shapes:
            objects.append(dict(shape, id=len(objects)+1, layerId="a", attrs={}))
    objects.append(dict(kind="dimension", id=len(objects)+1, layerId="a", attrs={},
                        a=dict(x=0, y=0), b=dict(x=2, y=0), height=.18, offset=-.5))
    raw["entities"] = objects
    doc = cad.Document.from_bytes(json.dumps(raw).encode())
    records = list(doc.entities())
    source = [r.uid for r in records[:4]]
    cad.dimensions.reassociate(doc, uid=records[-1].uid, points=[
        cad.PointBinding(point="a", to=cad.PlaceOn(uid=source[0], place=cad.StartSubPlace())),
        cad.PointBinding(point="b", to=cad.PlaceOn(uid=source[0], place=cad.EndSubPlace())),
    ])
    return doc, source


class BlockFind(unittest.TestCase):
    def test_forty_symbols_identity_maps_false_match_refusal_and_read_only_bound_dimension(self):
        doc, source = drawing()
        before = doc.to_bytes()
        result = cad.blocks.find(doc, source=source, mirrors=True)
        self.assertEqual(len(result.matches), 40)
        self.assertFalse(result.truncated)
        self.assertEqual(result.source, source)
        self.assertEqual(result.related, {"dimension": 1})
        self.assertEqual(len({tuple(sorted(g.members)) for g in result.matches}), 40)
        self.assertEqual(len(cad.blocks.find(doc, source=source).matches), 20)
        self.assertEqual(len(cad.blocks.find(doc, source=source, mirrors=True, scales=True).matches), 41)
        self.assertEqual(cad.blocks.find(doc, source=source, uids=[]).matches, [])
        chosen = result.matches[7].members
        self.assertEqual(len(cad.blocks.find(doc, source=source, mirrors=True, uids=chosen).matches), 1)
        self.assertAlmostEqual(cad.dimensions.query(doc).dimensions[0].measured, 2.)
        self.assertEqual(doc.to_bytes(), before)
        self.assertEqual(cad.Document.from_bytes(before).to_bytes(), before)

    def test_plan_and_execute_are_identical_without_dirtying_or_changing_undo_state(self):
        doc, source = drawing()
        before = doc.to_bytes()
        plan = cad.blocks.find.plan(doc, source=source, mirrors=True)
        result = cad.blocks.find(doc, source=source, mirrors=True)
        self.assertEqual(result.to_json(), plan.to_json())
        self.assertEqual(doc.to_bytes(), before)
        refused = cad.blocks.find.answer(doc, source=source, expected_revision="999999")
        self.assertEqual(refused.status, "conflict")
        self.assertEqual(refused.error.code, "revision_conflict")
        self.assertEqual(doc.to_bytes(), before)

    def test_background_job_uses_the_same_read_only_query_and_identity_map(self):
        doc, source = drawing()
        before = doc.to_bytes()
        expected = cad.blocks.find(doc, source=source, mirrors=True)
        answer = doc.start("cad.blocks.find", cad.BlocksFind(source=source, mirrors=True)).result()
        self.assertEqual(answer.status, "completed")
        self.assertEqual(answer.output.to_json(), expected.to_json())
        self.assertEqual(doc.to_bytes(), before)
