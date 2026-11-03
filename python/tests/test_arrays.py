"""Associative arrays from Python (docs/adr/0300): ``kentos.cad.entities.array`` with
``associative=True`` makes the objects a block and the places its inserts, one array the
document keeps; ``Document.arrays()`` reads it, the block is ``Document.blocks()``'s, the
items are ordinary inserts, a file keeps it (schema 53) and one undo takes it all back.

The shared cases (fixtures/commands/v1/cad.entities.array.assoc.json) run here too through
test_command_cases.py; this test is the reading side.
"""

from __future__ import annotations

import unittest
import json
import math
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
EDITS = ROOT / "fixtures/interaction/v1/edits.kcad"


def drawing() -> cad.Document:
    return cad.Document.from_bytes(EDITS.read_bytes())


def uid_of(doc: cad.Document, slot: int) -> str:
    return next(r.uid for r in doc.entities(page_size=1000) if r.entity.id == slot)


def closed_path_drawing():
    setup = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
    setup["settings"].update(drawingUnit="mm", cadWork="architecture")
    setup["entities"] = [
        dict(kind="circle", id=1, layerId="a", attrs={}, c=dict(x=0., y=0.), r=.12),
        dict(kind="polyline", id=2, layerId="a", attrs={}, closed=True,
             pts=[dict(x=x, y=y) for x, y in [(0., 0.), (4., 0.), (4., 4.), (0., 4.)]]),
    ]
    doc = cad.Document.from_bytes(json.dumps(setup).encode())
    records = list(doc.entities())
    return doc, records[0].uid, records[1].uid


class ArrayTests(unittest.TestCase):
    def test_linear_find_is_read_only_and_reviewed_outlier_is_explicitly_kept_before_adoption(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.adopt.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        roots = {r.entity.id: r.uid for r in doc.entities()}
        before = doc.to_bytes()
        found = cad.arrays.find(doc, source=roots[4])
        self.assertEqual(doc.to_bytes(), before)
        self.assertFalse(found.truncated)
        self.assertEqual(len(found.matches), 1)
        candidate = found.matches[0]
        self.assertEqual((candidate.present, candidate.missing), (5, 3))
        self.assertTrue(all(not i.kept for i in candidate.adoption.items))
        self.assertIn(roots[2], found.eligible)
        self.assertFalse(any(i.uid == roots[2] for i in candidate.adoption.items))
        self.assertEqual(found.excluded[roots[9]], "not_insert")
        # A user explicitly reviews and assigns the moved item to place 1.
        candidate.adoption.items[1] = cad.ArrayItem(uid=roots[2], kept=True)
        args = dict(source=candidate.adoption.source, layout=candidate.adoption.layout,
                    items=candidate.adoption.items, tolerance=candidate.adoption.tolerance,
                    expected_revision=candidate.adoption.expected_revision)
        review = cad.arrays.adopt.plan(doc, **args)
        self.assertEqual((review.missing, review.kept), (2, 1))
        made = cad.arrays.adopt(doc, **args)
        self.assertEqual(len(made.retained), 6)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)

    def test_linear_find_native_job_has_typed_candidates_and_writes_no_history(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.find.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        root = next(r.uid for r in doc.entities() if r.entity.id == 1)
        before = doc.to_bytes()
        job = doc.start("cad.arrays.find", cad.ArraysFind(source=root))
        answer = job.result()
        self.assertEqual(answer.status, "completed")
        self.assertIsInstance(answer.output, cad.ArraysFound)
        self.assertIsInstance(answer.output.matches[0].adoption, cad.ArraysAdopt)
        self.assertEqual((answer.output.matches[0].present, answer.output.matches[0].missing), (6, 2))
        self.assertEqual(doc.to_bytes(), before)
        self.assertIsNone(doc.undo())

    def test_linear_find_rejects_stale_review_and_empty_scope_finds_nothing(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.find.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        root = next(r.uid for r in doc.entities() if r.entity.id == 1)
        self.assertEqual(cad.arrays.find(doc, source=root, uids=[]).matches, [])
        proposal = cad.arrays.find(doc, source=root).matches[0].adoption
        cad.entities.transform(doc, uids=[root], transform=cad.MoveTransform(dx=1., dy=0.))
        before = doc.to_bytes()
        with self.assertRaises(cad.RevisionConflict):
            cad.arrays.find(doc, source=root, expected_revision=proposal.expected_revision)
        with self.assertRaises(cad.RevisionConflict):
            cad.arrays.adopt(doc, source=proposal.source, layout=proposal.layout,
                             items=proposal.items, tolerance=proposal.tolerance,
                             expected_revision=proposal.expected_revision)
        self.assertEqual(doc.to_bytes(), before)

    def test_reviewed_existing_blocks_adopt_without_replacing_roots_and_keep_bound_dimensions(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.adopt.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        records = list(doc.entities())
        roots = {r.entity.id: r.uid for r in records}
        measure = cad.AlignedDimensionMeasure(
            a=cad.PlaceMeasuredPoint(uid=roots[4], path=[0], place=cad.StartSubPlace()),
            b=cad.PlaceMeasuredPoint(uid=roots[5], path=[0], place=cad.StartSubPlace()))
        dimension = cad.dimensions.create(doc, measure=measure, at=(7., 1.), layer_id="a", height=.18).created[0]
        items = [cad.ArrayItem(uid=roots[k+1], kept=k == 1) if k not in [2, 6] else cad.ArrayItem()
                 for k in range(8)]
        layout = cad.LineArrayLayout(count=8, dx=2., dy=0., fit=False)
        before = doc.to_bytes()
        objects = [(r.uid, r.entity.to_json()) for r in doc.entities()]
        plan = cad.arrays.adopt.plan(doc, source=roots[1], items=items, layout=layout)
        self.assertEqual((plan.missing, plan.kept, plan.deviation), (2, 1, 0.))
        self.assertEqual(doc.to_bytes(), before)
        made = cad.arrays.adopt(doc, source=roots[1], items=items, layout=layout,
                                expected_revision=plan.revision)
        self.assertEqual(made.retained, [roots[k+1] for k in range(8) if k not in [2, 6]])
        self.assertEqual([(r.uid, r.entity.to_json()) for r in doc.entities()], objects)
        adopted = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(adopted).to_bytes(), adopted)
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), adopted)
        cad.arrays.edit(doc, array=made.array,
            edit=cad.LayoutArrayEdit(layout=cad.LineArrayLayout(count=10, dx=3., dy=0., fit=False)))
        current = doc.arrays()[0]["items"]
        for k in [0, 1, 3, 4, 5, 7]:
            self.assertEqual(current[k]["uid"], roots[k+1])
        self.assertNotIn("uid", current[2]); self.assertNotIn("uid", current[6])
        kept = doc.entity(roots[2]).entity.p
        self.assertEqual((kept.x, kept.y), (2.5, .25))
        self.assertAlmostEqual(cad.dimensions.query(doc).dimensions[0].measured, 3.)
        self.assertEqual(doc.entity(dimension).entity.refs.a.uid, roots[4])
        self.assertEqual(doc.entity(dimension).entity.refs.a.path, [0])
        expanded = doc.to_bytes()
        cad.arrays.edit(doc, array=made.array, edit=cad.RemoveArrayEdit(items=[4]))
        removed = doc.to_bytes()
        cad.arrays.edit(doc, array=made.array, edit=cad.ResetArrayEdit())
        self.assertTrue(all(i.get("uid") and not i.get("kept", False) for i in doc.arrays()[0]["items"]))
        self.assertEqual(doc.entity(roots[2]).entity.p.x, 3.)
        doc.undo(); self.assertEqual(doc.to_bytes(), removed)
        doc.undo(); self.assertEqual(doc.to_bytes(), expanded)
        doc.undo(); self.assertEqual(doc.to_bytes(), adopted)

    def test_adoption_request_is_cached_once_without_a_second_array_or_undo_step(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.adopt.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        roots = {r.entity.id: r.uid for r in doc.entities()}
        objects = [(r.uid, r.entity.to_json()) for r in doc.entities()]
        args = dict(source=roots[1], layout=cad.LineArrayLayout(count=8, dx=2., dy=0., fit=False),
            items=[cad.ArrayItem(uid=roots[k+1], kept=k == 1) if k not in [2, 6] else cad.ArrayItem() for k in range(8)],
            expected_revision=doc.revision, request_id="array-adopt-once")
        made = cad.arrays.adopt(doc, **args)
        after = doc.to_bytes()
        replay = cad.arrays.adopt(doc, **args)
        self.assertEqual(replay.array, made.array)
        self.assertEqual(doc.to_bytes(), after)
        self.assertEqual(len(doc.arrays()), 1)
        self.assertEqual([(r.uid, r.entity.to_json()) for r in doc.entities()], objects)
        self.assertEqual(doc.undo(), "Mevcut blokları diziye bağla")
        self.assertEqual(doc.arrays(), [])
        self.assertEqual([(r.uid, r.entity.to_json()) for r in doc.entities()], objects)
        self.assertIsNone(doc.undo())

    def test_missing_and_kept_places_cannot_hide_coordinate_overflow_in_a_review(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.adopt.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        roots = {r.entity.id: r.uid for r in doc.entities()}
        before = doc.to_bytes()
        for last in [cad.ArrayItem(), cad.ArrayItem(uid=roots[5], kept=True)]:
            result = cad.arrays.adopt.answer(doc, source=roots[1],
                layout=cad.LineArrayLayout(count=3, dx=1e308, dy=0., fit=False),
                items=[cad.ArrayItem(uid=roots[1]), cad.ArrayItem(uid=roots[4]), last],
                tolerance=1e308, op="plan")
            self.assertEqual(result.status, "failed")
            self.assertEqual(result.error.code, "array_adoption_refused")
            self.assertEqual(result.error.path, "layout")
            self.assertEqual(doc.to_bytes(), before)

    def test_a_guarded_adoption_plan_reads_existing_roots_and_kept_items_before_metadata_only_write(self):
        from kentos.cad.protocol import Expected
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.adopt.json").read_text())
        for kept_only in [False, True]:
            doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
            roots = {r.entity.id: r.uid for r in doc.entities()}
            args = dict(source=roots[1], layout=cad.LineArrayLayout(count=8, dx=2., dy=0., fit=False),
                items=[cad.ArrayItem(uid=roots[k+1], kept=k == 1) if k not in [2, 6] else cad.ArrayItem() for k in range(8)])
            plan = cad.arrays.adopt.answer(doc, **args, op="plan")
            self.assertEqual(plan.status, "completed")
            self.assertEqual(len(plan.summary.sources.objects), 6)
            self.assertEqual(len(plan.summary.sources.blocks), 1)
            self.assertEqual(plan.summary.undo_label, "Mevcut blokları diziye bağla")
            changed = [roots[2]] if kept_only else [u for k, u in roots.items() if k != 9]
            cad.entities.transform(doc, uids=changed, transform=cad.MoveTransform(dx=1., dy=1.))
            current = cad.arrays.adopt.answer(doc, **args, op="plan")
            self.assertEqual(current.status, "completed")
            self.assertNotEqual(current.plan_id, plan.plan_id)
            before = doc.to_bytes()
            result = cad.arrays.adopt.answer(doc, **args, expected=Expected(plan=plan.plan_id))
            self.assertEqual(result.status, "conflict")
            self.assertEqual(result.error.code, "plan_changed")
            self.assertEqual(doc.to_bytes(), before)

    def test_adoption_job_refuses_old_geometry_or_cancels_and_a_valid_job_has_typed_output(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.adopt.json").read_text())
        for action in ["move", "cancel", "success"]:
            doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
            roots = {r.entity.id: r.uid for r in doc.entities()}
            input_ = cad.ArraysAdopt(source=roots[1], layout=cad.LineArrayLayout(count=8, dx=2., dy=0., fit=False),
                items=[cad.ArrayItem(uid=roots[k+1], kept=k == 1) if k not in [2, 6] else cad.ArrayItem() for k in range(8)])
            before = doc.to_bytes()
            job = doc.start("cad.arrays.adopt", input_)
            if action == "move":
                cad.entities.transform(doc, uids=[roots[1]], transform=cad.MoveTransform(dx=0., dy=1.))
                before = doc.to_bytes()
                with self.assertRaises(cad.RevisionConflict) as caught:
                    job.result()
                self.assertEqual(caught.exception.code, "document_changed")
                self.assertEqual(doc.to_bytes(), before)
            elif action == "cancel":
                job.cancel()
                with self.assertRaises(cad.CommandCancelled):
                    job.result()
                self.assertEqual(doc.to_bytes(), before)
            else:
                result = job.result()
                self.assertEqual(result.status, "completed")
                self.assertEqual(result.output.array, doc.arrays()[0]["id"])
                self.assertEqual(len(result.output.retained), 6)
                doc.undo(); self.assertEqual(doc.to_bytes(), before)

    def test_closed_path_preview_committed_items_and_closure_edits_agree_in_millimetre_projects(self):
        doc, source, path = closed_path_drawing()
        layout = cad.PathArrayLayout(path=path, count=8, align=False)
        before = doc.to_bytes()
        expected = [(0., 0.), (2., 0.), (4., 0.), (4., 2.), (4., 4.), (2., 4.), (0., 4.), (0., 2.)]
        plan = cad.entities.array.plan(doc, uids=[source], layout=layout, associative=True)
        self.assertEqual([(e.c.x, e.c.y) for e in plan.entities], expected[1:])
        self.assertEqual(doc.to_bytes(), before)
        made = cad.entities.array(doc, uids=[source], layout=layout, associative=True)
        self.assertEqual([(doc.entity(u).entity.p.x, doc.entity(u).entity.p.y) for u in made.created], expected)
        after = doc.to_bytes()
        cad.curves.edit(doc, uid=path, edit=cad.OpenCurveEdit())
        opened = doc.to_bytes()
        self.assertEqual((doc.entity(made.created[-1]).entity.p.x, doc.entity(made.created[-1]).entity.p.y), (0., 4.))
        cad.curves.edit(doc, uid=path, edit=cad.CloseCurveEdit())
        self.assertEqual(doc.to_bytes(), after)
        self.assertEqual([i["uid"] for i in doc.arrays()[0]["items"]], made.created)
        doc.undo(); self.assertEqual(doc.to_bytes(), opened)
        doc.undo(); self.assertEqual(doc.to_bytes(), after)
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), after)
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)

    def test_closed_path_spacing_keeps_two_removed_items_and_a_manual_override_until_explicit_reset(self):
        doc, source, path = closed_path_drawing()
        made = cad.entities.array(doc, uids=[source],
            layout=cad.PathArrayLayout(path=path, count=8, spacing=2., align=False), associative=True)
        cad.entities.transform(doc, uids=[made.created[1]], transform=cad.MoveTransform(dx=.5, dy=.25))
        cad.arrays.edit(doc, array=made.array, edit=cad.RemoveArrayEdit(items=[2, 6]))
        before = doc.to_bytes()
        cad.arrays.edit(doc, array=made.array,
            edit=cad.LayoutArrayEdit(layout=cad.PathArrayLayout(path=path, count=12, align=False)))
        items = doc.arrays()[0]["items"]
        self.assertEqual(len(items), 12)
        self.assertNotIn("uid", items[2]); self.assertNotIn("uid", items[6])
        for k in [0, 1, 3, 4, 5, 7]: self.assertEqual(items[k]["uid"], made.created[k])
        held = doc.entity(made.created[1]).entity.p
        self.assertEqual((held.x, held.y), (2.5, .25))
        self.assertTrue(items[1]["kept"])
        expanded = doc.to_bytes()
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), expanded)
        cad.arrays.edit(doc, array=made.array, edit=cad.ResetArrayEdit())
        reset = doc.arrays()[0]["items"]
        self.assertTrue(all(i.get("uid") and not i.get("kept", False) for i in reset))
        for k in [0, 1, 3, 4, 5, 7]: self.assertEqual(reset[k]["uid"], made.created[k])
        self.assertAlmostEqual(doc.entity(reset[-1]["uid"]).entity.p.x, 0.)
        self.assertAlmostEqual(doc.entity(reset[-1]["uid"]).entity.p.y, 4./3.)
        doc.undo(); self.assertEqual(doc.to_bytes(), expanded)

    def test_closing_arc_spacing_uses_the_arc_length_and_tangent_in_both_windings(self):
        for winding in [1., -1.]:
            doc, source, path = closed_path_drawing()
            raw = doc.entity(path).entity.to_json()
            raw.update(pts=[dict(x=0., y=0.), dict(x=4., y=0.)], bulges=[0., winding])
            setup = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
            setup["entities"] = [doc.entity(source).entity.to_json(), raw]
            doc = cad.Document.from_bytes(json.dumps(setup).encode())
            source, path = [r.uid for r in doc.entities()]
            made = cad.entities.array(doc, uids=[source],
                layout=cad.PathArrayLayout(path=path, count=3, spacing=4., align=True), associative=True)
            last = doc.entity(made.created[-1]).entity
            self.assertAlmostEqual(last.p.x, 2.+2.*math.cos(2.), places=12)
            self.assertAlmostEqual(last.p.y, winding*2.*math.sin(2.), places=12)
            angle = winding*(2.+math.pi/2.)
            self.assertAlmostEqual(math.cos(last.rotation), math.cos(angle), places=12)
            self.assertAlmostEqual(math.sin(last.rotation), math.sin(angle), places=12)

    def test_an_associative_grid_is_a_block_its_inserts_and_the_array(self) -> None:
        doc = drawing()
        line = uid_of(doc, 2)
        made = cad.entities.array(
            doc, uids=[line], layout=cad.GridArrayLayout(rows=2, cols=2, dx=5.0, dy=4.0), associative=True
        )
        self.assertIsNotNone(made.array)
        self.assertEqual(len(made.created), 4)
        arrays = doc.arrays()
        self.assertEqual(len(arrays), 1)
        array = arrays[0]
        self.assertEqual(array["id"], made.array)
        self.assertEqual([i["uid"] for i in array["items"]], made.created)
        self.assertEqual(array["layout"], {"kind": "grid", "rows": 2, "cols": 2, "dx": 5.0, "dy": 4.0})
        block = next(b for b in doc.blocks() if b.id == array["block"])
        self.assertEqual(block.name, "Dizi 1")
        kinds = {r.entity.to_json()["kind"] for r in doc.entities(page_size=1000) if r.uid in made.created}
        self.assertEqual(kinds, {"insert"})
        # A file keeps the array; one undo takes the block, the items and the array back.
        again = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(again.arrays(), arrays)
        self.assertEqual(doc.undo(), "Dizi")
        self.assertEqual(doc.arrays(), [])
        self.assertEqual(doc.blocks(), [])


if __name__ == "__main__":
    unittest.main()
