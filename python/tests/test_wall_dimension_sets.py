"""Live wall dimension sets through the public typed SDK, with independent geometric expectations."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from kentos import cad


def edit(kind, **values):
    return cad.DimensionSetEdit.from_json(dict(kind=kind, **values))


class WallDimensionSets(unittest.TestCase):
    def test_generated_wrapper_defaults_are_fresh_and_serializable(self):
        # Exercise the actual generated wrapper's argument construction without
        # running a drawing: enum-list defaults need their runtime codec too.
        create = cad.dimensionsets.create
        with patch.object(type(create), "execute_input", lambda self, doc, value, **kw: value):
            first = create(None, layer_id="0", wall="host", at=(1, 2))
            second = create(None, layer_id="0", wall="host", at=(3, 4))
        raw = first.to_json()
        self.assertEqual(raw["gapMm"], 7.5)
        self.assertEqual(raw["face"], "left")
        self.assertEqual(raw["openings"], "ends")
        self.assertEqual(raw["rows"], ["openings", "overall"])
        self.assertEqual(raw["at"], {"x": 1., "y": 2.})
        first.rows.pop()
        self.assertEqual(second.to_json()["rows"], ["openings", "overall"])

    def seed(self, unit="m", origin=(0., 0.)):
        x, y = origin
        doc = cad.Document.new("Duvar ölçüleri", workspace="cad", drawing_unit=unit, plot_scale=50)
        host = cad.walls.create(doc, layer_id=doc.active_layer, walls=[
            cad.NewWall(pts=[(x, y), (x + 10, y)], width=.2, closed=False)]).uids[0]
        opening = cad.openings.create(doc, layer_id=doc.active_layer, openings=[
            cad.NewOpening(host=host, at=4, width=1, form="window")]).uids[0]
        return doc, host, opening

    def geometry(self, doc, group):
        return [doc.entity(m.uid).entity.to_json() for m in group.members]

    def lengths(self, entities):
        return [round(((d["b"]["x"] - d["a"]["x"]) ** 2 +
                       (d["b"]["y"] - d["a"]["y"]) ** 2) ** .5, 8) for d in entities]

    def test_live_membership_and_source_edits_preserve_world_units_and_history(self):
        for unit in ("m", "mm", "cm"):
            for origin in ((0., 0.), (500000., 4400000.)):
                with self.subTest(unit=unit, origin=origin):
                    doc, host, opening = self.seed(unit, origin)
                    x, y = origin
                    before = doc.to_bytes()
                    args = dict(layer_id=doc.active_layer, wall=host, at=(x + 5, y - 1),
                                rows=["openings", "overall"], face="left", openings="ends", gap_mm=7.5)
                    plan = cad.dimensionsets.create.plan(doc, **args)
                    self.assertEqual(doc.to_bytes(), before)
                    self.assertEqual(self.lengths([d.to_json() for d in plan.entities]), [3.5, 1., 5.5, 10.])
                    made = cad.dimensionsets.create(doc, **args, expected_revision=plan.revision)
                    group = cad.dimensionsets.query(doc).sets[0]
                    self.assertEqual(group.id, made.set)
                    self.assertEqual(len(group.members), 4)
                    self.assertEqual(self.lengths(self.geometry(doc, group)), [3.5, 1., 5.5, 10.])
                    created = doc.to_bytes()
                    doc.undo()
                    self.assertEqual(doc.to_bytes(), before)
                    doc.redo()
                    self.assertEqual(doc.to_bytes(), created)
                    keys = [(m.row, m.a.to_json(), m.b.to_json(), m.uid) for m in group.members]
                    proposal = cad.openings.edit.plan(doc, uids=[opening],
                        operation=cad.SetOpeningsOperation(width=2))
                    self.assertEqual(doc.to_bytes(), created)
                    self.assertTrue(proposal.followers.updated)
                    cad.openings.edit(doc, uids=[opening], operation=cad.SetOpeningsOperation(width=2))
                    wide = cad.dimensionsets.query(doc).sets[0]
                    self.assertEqual([(m.row, m.a.to_json(), m.b.to_json(), m.uid) for m in wide.members], keys)
                    self.assertEqual(self.lengths(self.geometry(doc, wide)), [3., 2., 5., 10.])
                    doc.undo(); self.assertEqual(doc.to_bytes(), created)
                    doc.redo()
                    extra = cad.NewOpening(host=host, at=8, width=1, form="door")
                    before_add = doc.to_bytes()
                    add_plan = cad.openings.create.plan(doc, layer_id=doc.active_layer, openings=[extra])
                    self.assertEqual(doc.to_bytes(), before_add)
                    self.assertTrue(add_plan.followers.added)
                    self.assertTrue(add_plan.followers.removed)
                    added = cad.openings.create(doc, layer_id=doc.active_layer, openings=[extra]).uids[0]
                    new_group = cad.dimensionsets.query(doc).sets[0]
                    self.assertEqual(self.lengths(self.geometry(doc, new_group)), [3., 2., 2.5, 1., 1.5, 10.])
                    self.assertEqual(new_group.members[-1].uid, wide.members[-1].uid)
                    cad.entities.delete(doc, uids=[added])
                    self.assertEqual(self.lengths(self.geometry(doc, cad.dimensionsets.query(doc).sets[0])),
                                     [3., 2., 5., 10.])
                    self.assertEqual(doc.info().units.drawing.value, unit)

    def test_group_metadata_plan_identity_rows_suppression_and_file_reopen(self):
        doc, host, opening = self.seed()
        args = dict(layer_id=doc.active_layer, wall=host, at=(5, -1))
        proposal = cad.dimensionsets.create.answer(doc, op="plan", **args)
        self.assertTrue(proposal.ok)
        made = cad.dimensionsets.create(doc, **args, expected=cad.Expected(plan=proposal.plan_id))
        group_id = made.set
        row = edit("rowOffset", row="overall", offset=-2)
        cad.dimensionsets.edit(doc, id=group_id, operation=row)
        group = cad.dimensionsets.query(doc).sets[0]
        self.assertEqual(len(group.row_places), 1)
        self.assertAlmostEqual(group.row_places[0].at.y, -1.9)
        before = doc.to_bytes()
        plan = cad.dimensionsets.edit.answer(doc, id=group_id, operation=edit("set", gapMm=10), op="plan")
        self.assertEqual(doc.to_bytes(), before)
        changed = cad.dimensionsets.edit.answer(doc, id=group_id, operation=edit("set", gapMm=10),
                                               expected=cad.Expected(plan=plan.plan_id))
        self.assertTrue(changed.ok)
        self.assertEqual(changed.plan_id, plan.plan_id)
        cad.entities.delete(doc, uids=[group.members[0].uid])
        hidden = cad.dimensionsets.query(doc).sets[0]
        self.assertEqual(len(hidden.members), 3)
        self.assertEqual(len(hidden.suppressed), 1)
        cad.dimensionsets.edit(doc, id=group_id, operation=edit("restoreHidden"))
        self.assertEqual(len(cad.dimensionsets.query(doc).sets[0].members), 4)
        cad.dimensionsets.edit(doc, id=group_id, operation=edit("resetRows"))
        # Empty optional collections are omitted on the wire; the generated SDK
        # preserves that absence as UNSET rather than inventing an empty list.
        self.assertNotIn("rowPlaces", cad.dimensionsets.query(doc).sets[0].to_json())
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "live-wall.kcad"
            doc.save(path)
            spec = importlib.util.spec_from_file_location("independent_kcad",
                Path(__file__).resolve().parents[2] / "tools/kcad/kcad.py")
            reader = importlib.util.module_from_spec(spec); spec.loader.exec_module(reader)
            _, model = reader.read(path.read_bytes())
            self.assertEqual(model["dimensionSets"][0]["wall"], host)
            self.assertEqual(len(model["dimensionSets"][0]["members"]), 4)
            back = cad.Document.open(path)
            self.assertEqual(cad.dimensionsets.query(back).sets, cad.dimensionsets.query(doc).sets)
            cad.openings.edit(back, uids=[opening], operation=cad.SetOpeningsOperation(width=2))
            self.assertEqual(self.lengths(self.geometry(back, cad.dimensionsets.query(back).sets[0])),
                             [3., 2., 5., 10.])
        before_detach = doc.to_bytes()
        cad.dimensionsets.edit(doc, id=group_id, operation=edit("detach"))
        self.assertEqual(cad.dimensionsets.query(doc).sets, [])
        self.assertEqual(len(doc), 6)
        doc.undo(); self.assertEqual(doc.to_bytes(), before_detach)
        cad.dimensionsets.edit(doc, id=group_id, operation=edit("delete"))
        self.assertEqual(len(doc), 2)
        self.assertEqual(cad.dimensionsets.query(doc).sets, [])
        doc.undo(); self.assertEqual(doc.to_bytes(), before_detach)

    def test_source_loss_unsupported_wall_and_stale_calls_never_silently_succeed(self):
        doc, host, _ = self.seed()
        made = cad.dimensionsets.create(doc, layer_id=doc.active_layer, wall=host, at=(5, -1))
        before = doc.to_bytes()
        with self.assertRaises(cad.RevisionConflict):
            cad.dimensionsets.edit(doc, id=made.set, operation=edit("set", gapMm=12), expected_revision="0")
        self.assertEqual(doc.to_bytes(), before)
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.dimensionsets.create(doc, layer_id=doc.active_layer, wall=host, at=(5,-1), gap_mm=0)
        self.assertEqual(caught.exception.code, "invalid_dimension_set")
        self.assertEqual(doc.to_bytes(), before)
        cad.entities.delete(doc, uids=[host])
        self.assertEqual(cad.dimensionsets.query(doc).sets[0].broken.value, "hostGone")
        doc.undo(); self.assertEqual(doc.to_bytes(), before)

