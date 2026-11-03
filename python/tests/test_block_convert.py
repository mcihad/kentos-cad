"""Chosen repeat conversion through the typed SDK, guarded plans, job API and independent 40-symbol scenes."""
import unittest
import math
from kentos import cad
from test_block_find import drawing, path_drawing


class BlockConvert(unittest.TestCase):
    def test_forty_analytic_symbols_keep_each_uuid_piece_mapping_and_bound_two_metre_dimension(self):
        doc, source = drawing()
        found = cad.blocks.find(doc, source=source, mirrors=True)
        groups = [m.members for m in found.matches]
        records = list(doc.entities())
        before = doc.to_bytes()
        args = dict(name="Teknik sembol", source=source, groups=groups, mirrors=True,
                    expected_revision=doc.info().revision)
        plan = cad.blocks.convert.plan(doc, **args)
        self.assertEqual(doc.to_bytes(), before)
        self.assertEqual(len(plan.placements), 40)
        self.assertEqual(len(plan.mapping), 160)
        self.assertEqual(len(plan.removed), 120)
        self.assertEqual(len(plan.dimensions), 2)
        self.assertEqual(plan.block.units, "m")
        for group in groups:
            for index, uid in enumerate(group):
                mapped = next(m for m in plan.mapping if m.from_ == uid)
                self.assertEqual(mapped.insert, group[0])
                self.assertEqual(mapped.path, [index])
        guarded = cad.blocks.convert.answer(doc, **args, op="plan")
        self.assertEqual(guarded.status, "completed")
        self.assertEqual(doc.to_bytes(), before)
        done = cad.blocks.convert.answer(doc, **args, expected=cad.Expected(plan=guarded.plan_id))
        self.assertEqual(done.status, "completed")
        self.assertEqual(done.output.mapping, plan.mapping)
        self.assertEqual(len(doc.blocks()), 1)
        self.assertEqual(len(list(doc.entities())), len(records)-120)
        for group in groups:
            self.assertEqual(doc.entity(group[0]).entity.kind, "insert")
        dimension = doc.entity(records[-1].uid).entity
        self.assertEqual(dimension.refs.a.uid, groups[0][0])
        self.assertEqual(dimension.refs.a.path, [0])
        self.assertEqual(dimension.refs.b.path, [0])
        self.assertAlmostEqual(cad.dimensions.query(doc).dimensions[0].measured, 2.)
        self.assertNotIn("broken",dimension.refs.a.to_json())
        self.assertNotIn("broken",dimension.refs.b.to_json())
        after = doc.to_bytes()
        reopened = cad.Document.from_bytes(after)
        self.assertEqual(reopened.to_bytes(), after)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), after)
        cad.entities.transform(doc, uids=[groups[0][0]], transform=cad.MoveTransform(dx=5,dy=3))
        dimension = doc.entity(records[-1].uid).entity
        self.assertEqual((dimension.a.x,dimension.a.y), (5.,3.))
        self.assertEqual((dimension.b.x,dimension.b.y), (7.,3.))
        doc.undo()
        self.assertEqual(doc.to_bytes(), after)

    def test_curved_tapered_paths_convert_in_a_background_job_and_keep_closing_refs(self):
        doc, source = path_drawing()
        dimension = list(doc.entities())[-1].uid
        cad.dimensions.reassociate(doc,uid=dimension,points=[cad.PointBinding(point="b",
            to=cad.PlaceOn(uid=source[0],place=cad.MidSubPlace(part=0,ring=0,index=2)))])
        groups = [m.members for m in cad.blocks.find(doc,source=source,mirrors=True).matches]
        before = doc.to_bytes()
        result = doc.start("cad.blocks.convert", cad.BlocksConvert(name="Yaylı tekrar",source=source,
            groups=groups,mirrors=True,expected_revision=doc.info().revision)).result()
        self.assertEqual(result.status, "completed")
        self.assertEqual(len(result.output.inserts), 40)
        self.assertEqual(len(doc.blocks()), 1)
        self.assertTrue(doc.blocks()[0].entities[0].closed)
        self.assertEqual(doc.blocks()[0].entities[0].widths[-1], [.3,.1])
        d = doc.entity(dimension).entity
        self.assertEqual(d.refs.b.path, [0])
        self.assertEqual(d.refs.b.place.index, 2)
        self.assertNotIn("broken",d.refs.b.to_json())
        self.assertAlmostEqual(d.b.x, .8)
        self.assertAlmostEqual(d.b.y, .425)
        after = doc.to_bytes()
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), after)

    def test_mirrored_arc_start_and_fraction_keep_the_world_canonical_place(self):
        doc, source = drawing()
        records=list(doc.entities());dimension=records[-1].uid;target_arc=records[7].uid
        cad.dimensions.reassociate(doc,uid=dimension,points=[
            cad.PointBinding(point="a",to=cad.PlaceOn(uid=target_arc,place=cad.StartSubPlace())),
            cad.PointBinding(point="b",to=cad.PlaceOn(uid=target_arc,place=cad.AlongSubPlace(part=0,ring=0,index=0,t=.25))),
        ])
        group=cad.blocks.find(doc,source=source,mirrors=True).matches[1].members
        before=doc.to_bytes()
        cad.blocks.convert(doc,name="Aynalı yay",source=source,groups=[group],mirrors=True)
        d=doc.entity(dimension).entity
        c=(10+math.cos(.17)+.5*math.sin(.17),math.sin(.17)-.5*math.cos(.17))
        for p,t in [(d.a,0.),(d.b,.25)]:
            angle=.17-1.7+1.5*t
            self.assertAlmostEqual(p.x,c[0]+.2*math.cos(angle),places=11)
            self.assertAlmostEqual(p.y,c[1]+.2*math.sin(angle),places=11)
        self.assertEqual(d.refs.a.place.kind,"start")
        self.assertEqual(d.refs.b.place.t,.25)
        self.assertEqual(d.refs.a.path,[3])
        after=doc.to_bytes();doc.undo();self.assertEqual(doc.to_bytes(),before)
        doc.redo();self.assertEqual(doc.to_bytes(),after)

    def test_explicit_scaled_candidate_keeps_four_metre_measurement_in_mm_project(self):
        doc,source=drawing();records=list(doc.entities());dimension=records[-1].uid
        found=cad.blocks.find(doc,source=source,mirrors=True,scales=True)
        group=found.matches[40].members
        cad.dimensions.reassociate(doc,uid=dimension,points=[
            cad.PointBinding(point="a",to=cad.PlaceOn(uid=group[0],place=cad.StartSubPlace())),
            cad.PointBinding(point="b",to=cad.PlaceOn(uid=group[0],place=cad.EndSubPlace())),
        ])
        before=doc.to_bytes()
        cad.blocks.convert(doc,name="İki kat boyut",source=source,groups=[group],mirrors=True,scales=True)
        i=doc.entity(group[0]).entity
        self.assertAlmostEqual(i.scale,2.)
        self.assertEqual(doc.blocks()[0].units,"m")
        self.assertEqual(doc.info().units.drawing,"mm")
        self.assertAlmostEqual(cad.dimensions.query(doc).dimensions[0].measured,4.)
        self.assertEqual(doc.entity(source[0]).entity.kind,"line")
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(),doc.to_bytes())
        doc.undo();self.assertEqual(doc.to_bytes(),before)

    def test_only_chosen_candidates_change_and_overlap_data_and_stale_revision_refuse(self):
        doc, source = drawing()
        groups = [m.members for m in cad.blocks.find(doc,source=source,mirrors=True).matches]
        records = list(doc.entities())
        args = dict(name="Seçilen",source=source,groups=[groups[5]],mirrors=True)
        before = doc.to_bytes()
        done = cad.blocks.convert(doc,**args)
        self.assertEqual(done.inserts,[groups[5][0]])
        self.assertEqual(doc.entity(source[0]).entity.kind,"line")
        self.assertEqual(doc.entity(records[-1].uid).entity.refs.a.uid,source[0])
        doc.undo()
        self.assertEqual(doc.to_bytes(),before)
        for more in [dict(groups=[groups[0],groups[0]]),dict(expected_revision="999")]:
            with self.assertRaises((cad.CommandFailed,cad.RevisionConflict)):
                cad.blocks.convert(doc,**dict(args,**more))
            self.assertEqual(doc.to_bytes(),before)
        # Recognition's geometry-only policy cannot grant conversion permission to discard attributes.
        cad.entities.set(doc,uids=source+groups[5],operation="attributes",attrs={"TYPE":"Door"})
        before=doc.to_bytes()
        self.assertEqual(len(cad.blocks.find(doc,source=source,mirrors=True,geometry_only=True).matches),40)
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.blocks.convert(doc,**args)
        self.assertEqual(refused.exception.code,"recognition_conversion_refused")
        self.assertEqual(doc.to_bytes(),before)



if __name__ == "__main__":
    unittest.main()
