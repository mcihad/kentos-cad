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

    def test_the_project_file_keeps_the_link(self):
        doc = self.drawing()
        made = self.named(doc)
        back = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(back.entity(made.text_uid).entity.label_of, made.uid)


if __name__ == '__main__':
    unittest.main()
