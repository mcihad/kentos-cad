"""Rectangular architectural walls through the same semantic command as the UI.

Independent acceptance uses elementary clear-area arithmetic and the existing
space engine. Display units, project defaults, persistence and hosted openings
must remain separate from the new tool's face reference.
"""
from pathlib import Path
import tempfile
import unittest

from kentos import cad


class RectangularWalls(unittest.TestCase):
    def test_face_reference_has_the_same_clear_area_in_every_display_unit(self):
        for unit in ('mm', 'cm', 'm', 'in', 'ft'):
            for origin in ((0.0, 0.0), (500000.0, 4400000.0)):
                for reference, expected in (('inner', 12.0), ('outer', 9.36), ('center', 10.64)):
                    with self.subTest(unit=unit, origin=origin, reference=reference):
                        doc = cad.Document.new('Oda', workspace='cad', drawing_unit=unit, plot_scale=100)
                        x, y = origin
                        frame = cad.CornersRectangleFrame(a=(x, y), b=(x + 4, y + 3))
                        planned = cad.walls.rectangle.plan(
                            doc, layer_id=doc.active_layer, frame=frame, reference=reference, width=.2)
                        self.assertEqual(len(doc), 0, 'preview wrote a wall')
                        made = cad.walls.rectangle(
                            doc, layer_id=doc.active_layer, frame=frame, reference=reference,
                            width=.2, expected_revision=planned.revision)
                        self.assertEqual(len(made.uids), 1)
                        wall = doc.entity(made.uids[0]).entity
                        self.assertIsInstance(wall, cad.WallEntity)
                        self.assertTrue(wall.closed)
                        self.assertEqual(len(wall.pts), 4)
                        self.assertEqual(wall.width, .2)
                        self.assertEqual([r['command'] for r in doc.journal()['calls']],
                                         ['cad.walls.rectangle'])
                        self.assertEqual(doc.undo(), 'Duvar')
                        self.assertEqual(len(doc), 0)
                        self.assertEqual(doc.redo(), 'Duvar')
                        self.assertEqual(len(doc), 1)
                        cad.spaces.create(doc, layer_id=doc.active_layer,
                                          spaces=[cad.NewSpace(seed=(x + 2, y + 1.5), number='01')])
                        result = cad.spaces.quantities(doc)
                        self.assertEqual(len(result.spaces), 1)
                        self.assertAlmostEqual(result.spaces[0].area, expected, delta=1e-8)
                        self.assertEqual(doc.info().settings.plot_scale, 100)
                        self.assertEqual(doc.info().units.drawing.value, unit)
                        with tempfile.TemporaryDirectory() as tmp:
                            path = Path(tmp) / 'room.kcad'
                            doc.save(path)
                            back = cad.Document.open(path)
                            self.assertEqual(back.entity(made.uids[0]).entity, wall)
                            self.assertAlmostEqual(cad.spaces.quantities(back).spaces[0].area,
                                                   expected, delta=1e-8)

    def test_opening_and_space_follow_the_original_wall_without_replacement(self):
        doc = cad.Document.new('Dış ölçülü oda', workspace='cad')
        made = cad.walls.rectangle(doc, layer_id=doc.active_layer,
                                   frame=cad.SizeRectangleFrame(corner=(0, 0), length=4, width=3,
                                                                toward=(5, 4)),
                                   reference='outer', width=.2)
        uid = made.uids[0]
        door = cad.openings.create(doc, layer_id=doc.active_layer,
                                   openings=[cad.NewOpening(host=uid, at=1, width=.9, form='door')])
        cad.spaces.create(doc, layer_id=doc.active_layer,
                          spaces=[cad.NewSpace(seed=(2, 1.5), name='Oda')])
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 9.36)
        cad.walls.edit(doc, uids=[uid], operation=cad.SetWallsOperation(width=.3))
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 8.16)
        opening = doc.entity(door.uids[0]).entity
        self.assertEqual(opening.host, uid)
        self.assertEqual(opening.width, .9)
        self.assertEqual(doc.info().settings.plot_scale, 100)
        doc.undo()
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 9.36)

    def test_stale_preview_and_closed_interior_are_atomic_refusals(self):
        doc = cad.Document.new(workspace='cad')
        frame = cad.CornersRectangleFrame(a=(0, 0), b=(4, 3))
        planned = cad.walls.rectangle.plan(doc, layer_id=doc.active_layer, frame=frame)
        cad.point.create(doc, layer_id=doc.active_layer, p=(20, 20))
        with self.assertRaises(cad.RevisionConflict):
            cad.walls.rectangle(doc, layer_id=doc.active_layer, frame=frame,
                                expected_revision=planned.revision)
        self.assertEqual(len(doc), 1)
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.walls.rectangle(doc, layer_id=doc.active_layer, frame=frame,
                                reference='outer', width=1.5)
        self.assertEqual(caught.exception.code, 'invalid_wall')
        self.assertEqual(len(doc), 1)


if __name__ == '__main__':
    unittest.main()
