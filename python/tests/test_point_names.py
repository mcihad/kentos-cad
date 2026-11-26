"""ADR 9314: a point's name as a text that follows it, through the public commands.

`cad.point.create` with `name_text` writes the name up and right of the point as a text linked to it without a scale.
Moving the point moves the text as far; renaming the point (as the point list and the inspector do) rewrites the text;
the project file keeps the link (document schema 257).
"""
import json
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class PointNames(unittest.TestCase):
    def drawing(self):
        fx = json.loads((ROOT / 'fixtures/commands/v1/cad.point.create.json').read_text())
        return cad.Document.from_bytes(json.dumps(fx['setup']).encode())

    def named(self, doc):
        made = cad.point.create(doc, layer_id='yapi', p=cad.Vec2(x=487010.0, y=4420010.0), label='P1', name_text=True)
        self.assertIsNotNone(made.text_uid)
        return made

    def test_the_name_is_a_text_up_and_right_of_its_point(self):
        doc = self.drawing()
        made = self.named(doc)
        text = doc.entity(made.text_uid).entity
        self.assertEqual(text.text, 'P1')
        self.assertEqual(text.label_of, made.uid)
        self.assertIs(text.label_scale, cad.UNSET, 'a point\'s name has no scale')
        self.assertGreater(text.p.x, 487010.0)
        self.assertGreater(text.p.y, 4420010.0)

    def test_the_text_moves_with_its_point_and_takes_its_new_name(self):
        doc = self.drawing()
        made = self.named(doc)
        before = doc.entity(made.text_uid).entity.p
        cad.entities.transform(doc, uids=[made.uid], transform=cad.MoveTransform(dx=10.0, dy=5.0))
        after = doc.entity(made.text_uid).entity.p
        self.assertAlmostEqual(after.x - before.x, 10.0, places=9)
        self.assertAlmostEqual(after.y - before.y, 5.0, places=9)
        cad.entities.set(doc, uids=[made.uid], operation='label', label='P8')
        self.assertEqual(doc.entity(made.text_uid).entity.text, 'P8')

    def test_a_point_named_later_gets_its_name_text_in_the_same_step(self):
        """ADR 9316: `cad.entities.set` with `name_text` writes the text of a point given its first name."""
        doc = self.drawing()
        made = cad.point.create(doc, layer_id='yapi', p=cad.Vec2(x=487020.0, y=4420020.0))
        self.assertIs(made.text_uid, cad.UNSET, 'no name, no text')
        plan = cad.entities.set.plan(doc, uids=[made.uid], operation='label', label='P9', name_text=True)
        self.assertEqual([t.text for t in plan.texts], ['P9'])
        named = cad.entities.set(doc, uids=[made.uid], operation='label', label='P9', name_text=True)
        self.assertEqual(len(named.texts), 1)
        text = doc.entity(named.texts[0]).entity
        self.assertEqual((text.text, text.label_of), ('P9', made.uid))
        # A second name: the same text, no new one.
        again = cad.entities.set(doc, uids=[made.uid], operation='label', label='P10', name_text=True)
        self.assertIs(again.texts, cad.UNSET, 'no second text')
        self.assertEqual(doc.entity(named.texts[0]).entity.text, 'P10')
        self.assertEqual(doc.undo(), 'Ad değiştir')

    def test_the_project_file_keeps_the_link(self):
        doc = self.drawing()
        made = self.named(doc)
        back = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(back.entity(made.text_uid).entity.label_of, made.uid)


if __name__ == '__main__':
    unittest.main()
