"""Office packages keep drawing settings unless an explicit preview says otherwise."""
from pathlib import Path
import tempfile
import unittest

from kentos import cad


class TemplateReview(unittest.TestCase):
    def test_keep_preserves_every_display_unit_geometry_and_complete_undo_redo(self):
        for unit in ('mm', 'cm', 'm', 'in', 'ft'):
            with self.subTest(unit=unit):
                doc = cad.Document.new(workspace='cad', drawing_unit=unit, plot_scale=100)
                made = cad.walls.rectangle(doc, layer_id=doc.active_layer, width=.2,
                    frame=cad.CornersRectangleFrame(a=(500000, 4400000), b=(500004, 4400003)))
                wall = doc.entity(made.uids[0]).entity
                before = doc.info()
                plan = cad.templates.apply.plan(doc, name='mimari', settings_mode='keep')
                self.assertEqual(plan.unit.value, unit)
                self.assertEqual(plan.review.plot_scale, 100)
                self.assertEqual(doc.info(), before, 'preview changed the drawing')
                out = cad.templates.apply(doc, name='mimari', settings_mode='keep',
                                          expected_revision=plan.revision)
                self.assertEqual(out.review.mode.value, 'keep')
                self.assertEqual(doc.entity(made.uids[0]).entity, wall)
                self.assertEqual(len(out.sheets), 3)
                after = doc.info()
                self.assertEqual(after.units.drawing.value, unit)
                self.assertEqual(after.settings.plot_scale, 100)
                with tempfile.TemporaryDirectory() as folder:
                    path = Path(folder) / 'office.kcad'
                    doc.save(path)
                    back = cad.Document.open(path)
                    self.assertEqual(back.entity(made.uids[0]).entity, wall)
                    self.assertEqual(back.info().settings, after.settings)
                self.assertEqual(doc.undo(), 'Mimari şablonu')
                self.assertEqual(doc.info().settings, before.settings)
                self.assertEqual(doc.entity(made.uids[0]).entity, wall)
                self.assertEqual(doc.redo(), 'Mimari şablonu')
                self.assertEqual(doc.info().settings, after.settings)
                revision = doc.revision
                twice = cad.templates.apply(doc, name='mimari', settings_mode='keep')
                self.assertEqual(twice.layers + twice.styles + twice.blocks + twice.wall_styles
                                 + twice.levels + twice.sheets, [])
                self.assertEqual(doc.revision, revision)

    def test_explicit_package_settings_do_not_rescale_and_legacy_is_unchanged(self):
        doc = cad.Document.new(workspace='cad')
        uid = cad.point.create(doc, layer_id=doc.active_layer, p=(12.5, 30)).uid
        entity = doc.entity(uid).entity
        plan = cad.templates.apply.plan(doc, name='mimari', settings_mode='template', unit='ft')
        self.assertEqual(plan.unit.value, 'ft')
        self.assertEqual(plan.review.plot_scale, 50)
        cad.templates.apply(doc, name='mimari', settings_mode='template', unit='ft',
                            expected_revision=plan.revision)
        self.assertEqual(doc.entity(uid).entity, entity)
        self.assertEqual(doc.info().units.drawing.value, 'ft')
        doc.undo()
        self.assertEqual(doc.info().settings.plot_scale, 100)
        old = cad.Document.new(workspace='cad')
        result = cad.templates.apply(old, name='mimari')
        self.assertNotIn('review', result.to_json())
        self.assertEqual(old.info().units.drawing.value, 'cm')
        self.assertEqual(old.info().settings.plot_scale, 50)

    def test_conflicting_policy_and_stale_review_write_nothing(self):
        doc = cad.Document.new(workspace='cad')
        before = doc.info()
        for operation in (cad.templates.apply, cad.templates.apply.plan, cad.templates.apply.validate):
            with self.assertRaises(cad.CommandFailed) as caught:
                operation(doc, name='mimari', settings_mode='keep', unit='mm')
            self.assertEqual(caught.exception.code, 'template_settings_conflict')
            self.assertEqual(doc.info(), before)
        plan = cad.templates.apply.plan(doc, name='mimari', settings_mode='keep')
        cad.point.create(doc, layer_id=doc.active_layer, p=(0, 0))
        before = doc.info()
        with self.assertRaises(cad.RevisionConflict):
            cad.templates.apply(doc, name='mimari', settings_mode='keep', expected_revision=plan.revision)
        self.assertEqual(doc.info(), before)

    def test_projected_cad_keeps_the_coordinate_system_unit(self):
        doc = cad.Document.new(workspace='cad', coords='real', srid=5256, plot_scale=100)
        before = doc.info()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.templates.apply(doc, name='mimari', settings_mode='template')
        self.assertEqual(caught.exception.code, 'unit_needs_local')
        self.assertEqual(doc.info(), before)
        plan = cad.templates.apply.plan(doc, name='mimari', settings_mode='template', unit='m')
        self.assertEqual(plan.unit.value, 'm')
        self.assertEqual(plan.review.previous_unit.value, 'm')
        cad.templates.apply(doc, name='mimari', settings_mode='template', unit='m',
                            expected_revision=plan.revision)
        self.assertEqual(doc.info().settings.srid, 5256)
        self.assertEqual(doc.info().units.drawing.value, 'm')
        self.assertEqual(doc.info().settings.plot_scale, 50)
        doc.undo()
        self.assertEqual(doc.info().settings, before.settings)
