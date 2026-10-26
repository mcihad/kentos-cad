"""Semantic tag placement keeps its source, derived text and saved identity."""
import copy
from decimal import Decimal
import json
from pathlib import Path
import tempfile
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class TagPlacement(unittest.TestCase):
    def drawing(self):
        setup = json.loads((ROOT/'fixtures/commands/v1/cad.spaces.number.json').read_text())['setup']
        setup['entities'] = [dict(kind='wall', id=1, layerId='duvar', attrs={},
            closed=True, width=.2, justify='center', arch=dict(level='zemin'),
            pts=[dict(x=0,y=0),dict(x=6,y=0),dict(x=6,y=4),dict(x=0,y=4)])]
        doc = cad.Document.from_bytes(json.dumps(setup).encode())
        wall = next(doc.entities()).uid
        room = cad.spaces.create(doc, layer_id='duvar', spaces=[cad.NewSpace.from_json(
            dict(seed=dict(x=3,y=2),number='A-001',name='Salon',arch=dict(level='zemin')))]).uids[0]
        tag = cad.tags.create(doc, layer_id='duvar', tags=[cad.NewTag.from_json(
            dict(of=room,at=dict(x=-2,y=2),leader=True,content='{number}\n{name}\n{area}'))]).uids[0]
        table = cad.schedules.create(doc, layer_id='duvar', at=cad.Vec2(x=15,y=10), of='spaces').uid
        return doc, wall, room, tag, table

    def rows(self, doc):
        return {r.uid:r.entity.to_json() for r in doc.entities()}

    def test_twenty_moves_plan_write_undo_and_saved_identity(self):
        for dx,dy in [(0,4),(0,-4),(-10,0),(6,0),(8,5),(-15,-10),(2,-2),
                (3,-1),(32,18),(-28,-22),(.5,.5),(-.5,-.5),(.125,2.25),
                (2.25,-4.125),(14,-6),(27,-2),(2,18),(1,-3),(-38,8),(42,-12)]:
            with self.subTest(dx=dx,dy=dy):
                doc, _, _, tag, _ = self.drawing()
                before = self.rows(doc)
                args = dict(uids=[tag], transform=cad.MoveTransform(dx=dx,dy=dy))
                plan = cad.entities.transform.plan(doc, **args)
                self.assertEqual(self.rows(doc),before)
                expected = copy.deepcopy(before)
                expected[tag]['at'] = dict(x=-2+dx,y=2+dy)
                self.assertEqual(plan.entities[0].to_json(),expected[tag])
                cad.entities.transform(doc, **args, expected_revision=plan.revision)
                self.assertEqual(self.rows(doc),expected)
                self.assertEqual(doc.journal()['calls'][-1]['command'],'cad.entities.transform')
                doc.undo(); self.assertEqual(self.rows(doc),before)
                doc.redo(); self.assertEqual(self.rows(doc),expected)
                with tempfile.TemporaryDirectory() as folder:
                    path = Path(folder)/'tag.kcad'
                    doc.save(path)
                    self.assertEqual(self.rows(cad.Document.open(path)),expected)

    def test_moved_tag_follows_twenty_wall_widths_in_one_undo(self):
        doc, wall, room, tag, table = self.drawing()
        cad.entities.transform(doc,uids=[tag],transform=cad.MoveTransform(dx=-1,dy=3))
        before = self.rows(doc)
        for i in range(1,21):
            width = Decimal(i)/10
            area = (Decimal(6)-width)*(Decimal(4)-width)
            with self.subTest(width=str(width)):
                cad.walls.edit(doc,uids=[wall],operation=cad.WallsOperation.from_json(
                    dict(kind='set',width=float(width))))
                rows = self.rows(doc)
                # Recomputed centroids can differ by a float64 ulp; the
                # physical tolerance is 1e-10 m, not the display precision.
                for axis,value in [('x',-3),('y',5)]:
                    self.assertAlmostEqual(rows[tag]['at'][axis],value,places=10)
                    self.assertAlmostEqual(rows[tag]['anchor'][axis],before[tag]['anchor'][axis],places=10)
                self.assertEqual(rows[tag]['of'],room)
                self.assertEqual(rows[tag]['content'],before[tag]['content'])
                self.assertEqual(rows[tag]['text'],f'A-001\nSalon\n{area:.2f} m²')
                self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area,float(area),places=8)
                self.assertIn(f'{area:.2f}',json.dumps(rows[table]['cells']))
                if width == Decimal('.2'):
                    self.assertEqual(rows,before)  # An unchanged width adds no edit to undo.
                else:
                    doc.undo(); self.assertEqual(self.rows(doc),before)

    def test_deleted_source_preserves_moved_tag_and_restores_in_one_undo(self):
        doc, _, room, tag, table = self.drawing()
        cad.entities.transform(doc,uids=[tag],transform=cad.MoveTransform(dx=-1,dy=3))
        before = self.rows(doc)
        cad.entities.delete(doc,uids=[room])
        after = self.rows(doc)
        self.assertNotIn(room,after)
        expected = copy.deepcopy(before[tag]); expected['broken']='targetGone'
        self.assertEqual(after[tag],expected)
        self.assertEqual(len(after[table]['cells']),1)
        doc.undo(); self.assertEqual(self.rows(doc),before)
        doc.redo(); self.assertEqual(self.rows(doc),after)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder)/'broken.kcad'; doc.save(path)
            reopened = self.rows(cad.Document.open(path))
            # Numeric slots compact on reopen; persistent UUIDs and source links do not.
            for rows in (reopened,after):
                for entity in rows.values():
                    entity.pop('id')
            self.assertEqual(reopened,after)
