"""Measured partitions share the native semantic command with the desktop and agents."""
from pathlib import Path
import tempfile
import unittest
from kentos import cad


class MeasuredPartitions(unittest.TestCase):
    def room(self, unit='m', x=0, y=0):
        doc = cad.Document.new('Bölme', workspace='cad', drawing_unit=unit, plot_scale=100)
        uid = cad.walls.rectangle(doc, layer_id=doc.active_layer,
            frame=cad.CornersRectangleFrame(a=(x, y), b=(x+8, y+6)), width=.2).uids[0]
        return doc, uid

    def arguments(self, doc, uid):
        return dict(source=cad.WallEdgeRef(wall=uid, edge=0), distance=3,
            extent=cad.BetweenPartitionExtent(start=cad.WallEdgeRef(wall=uid, edge=3),
                                              end=cad.WallEdgeRef(wall=uid, edge=1)),
            wall=cad.WallsCreate(layer_id=doc.active_layer,
                                  walls=[cad.NewWall(pts=[], width=.15)]))

    def test_units_door_space_persistence_and_one_undo(self):
        for unit in ('mm', 'cm', 'm', 'in', 'ft'):
            for x, y in ((0, 0), (500000, 4400000)):
                with self.subTest(unit=unit, origin=(x, y)):
                    doc, uid = self.room(unit, x, y)
                    door = cad.openings.create(doc, layer_id=doc.active_layer,
                        openings=[cad.NewOpening(host=uid, at=2, width=.9, form='door')]).uids[0]
                    cad.spaces.create(doc, layer_id=doc.active_layer,
                        spaces=[cad.NewSpace(seed=(x+2, y+1), name='Alt oda')])
                    before = len(doc)
                    args = self.arguments(doc, uid)
                    plan = cad.walls.partition.plan(doc, **args)
                    self.assertEqual(len(doc), before)
                    self.assertAlmostEqual(plan.clear_distance, 3)
                    self.assertAlmostEqual(plan.axis_distance, 3.175)
                    part = cad.walls.partition(doc, **args).uids[0]
                    self.assertEqual(len(doc), before+1)
                    self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 24, delta=1e-7)
                    self.assertEqual(doc.entity(door).entity.host, uid)
                    self.assertEqual(doc.journal()['calls'][-1]['command'], 'cad.walls.partition')
                    self.assertEqual(doc.undo(), 'Duvar')
                    self.assertEqual(len(doc), before)
                    self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 48, delta=1e-7)
                    doc.redo()
                    self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 24, delta=1e-7)
                    with tempfile.TemporaryDirectory() as tmp:
                        path = Path(tmp)/'partition.kcad'
                        doc.save(path)
                        back = cad.Document.open(path)
                        self.assertEqual(back.entity(part).entity, doc.entity(part).entity)
                        self.assertAlmostEqual(cad.spaces.quantities(back).spaces[0].area, 24, delta=1e-7)
                    self.assertEqual(doc.info().units.drawing.value, unit)
                    self.assertEqual(doc.info().settings.plot_scale, 100)

    def test_partition_moves_with_existing_space_and_associative_dimension_system(self):
        doc, uid = self.room()
        part = cad.walls.partition(doc, **self.arguments(doc, uid)).uids[0]
        cad.spaces.create(doc, layer_id=doc.active_layer,
            spaces=[cad.NewSpace(seed=(2, 1), name='Alt oda')])
        dimension = cad.dimensions.create(doc, layer_id=doc.active_layer, at=(-1, 2),
            measure=cad.AlignedDimensionMeasure(
                a=cad.AtMeasuredPoint(at=(0, 0)),
                b=cad.PlaceMeasuredPoint(uid=part, place=cad.VertexSubPlace(part=0, ring=0, index=0)))).created[0]
        old = doc.entity(dimension).entity
        source = doc.entity(uid).entity
        cad.entities.transform(doc, uids=[part], transform=cad.MoveTransform(dx=0, dy=.5))
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 28)
        moved = doc.entity(dimension).entity
        self.assertAlmostEqual(moved.b.y-old.b.y, .5)
        self.assertEqual(doc.entity(uid).entity, source)
        doc.undo()
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, 24)
        self.assertEqual(doc.entity(dimension).entity, old)

    def test_defaults_and_explicit_directions_and_atomic_refusals(self):
        doc, uid = self.room()
        args = self.arguments(doc, uid)
        del args['extent']  # Tagged-union default is generated correctly in the Python SDK.
        self.assertAlmostEqual(cad.walls.partition.plan(doc, **args).clear_distance, 3)
        for direction, expected in [('forward', (2, 4)), ('backward', (0, 2)), ('both', (1, 3))]:
            args['extent'] = cad.LengthPartitionExtent(anchor=2, length=2, direction=direction)
            plan = cad.walls.partition.plan(doc, **args)
            self.assertEqual(tuple(p.x for p in plan.wall.walls[0].pts), expected)
        count = len(doc)
        args['distance'] = 0
        with self.assertRaises(cad.CommandFailed):
            cad.walls.partition(doc, **args)
        self.assertEqual(len(doc), count)
        args['distance'] = 3
        args['wall'].expected_revision = '0'
        with self.assertRaises(cad.RevisionConflict):
            cad.walls.partition(doc, **args)
        self.assertEqual(len(doc), count)


if __name__ == '__main__':
    unittest.main()
