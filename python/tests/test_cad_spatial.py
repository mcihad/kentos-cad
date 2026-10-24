"""Independent XYZ fixtures through typed imports, edits, history and native persistence."""
import copy
import json
import math
from pathlib import Path
import tempfile
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT/'fixtures/formats/v1/cad-spatial/expected.json').read_text())['cases']


class CadSpatial(unittest.TestCase):
    def compare(self, actual, expected):
        if isinstance(expected, dict):
            for key,value in expected.items(): self.compare(actual[key],value)
        elif isinstance(expected, list):
            self.assertEqual(len(actual),len(expected))
            for a,b in zip(actual,expected): self.compare(a,b)
        elif isinstance(expected, (float,int)):
            self.assertTrue(math.isfinite(actual))
            self.assertLessEqual(abs(actual-expected),abs(expected)*32*math.ulp(1.)+1e-12)
        else:
            self.assertEqual(actual,expected)

    def entities(self, doc):
        return [record.entity.to_json() for record in doc.entities()]

    def test_all_sources_survive_move_history_and_kcad(self):
        with tempfile.TemporaryDirectory() as folder:
            for case in CASES:
                with self.subTest(name=case['name']):
                    source = ROOT/'fixtures'/case['source']
                    before = source.read_bytes()
                    doc = cad.Document.new(workspace='cad',drawing_unit='mm')
                    result = cad.file.import_.dxf(doc,path=str(source),explode_blocks=True)
                    self.assertEqual(result.objects,len(case['expected']))
                    self.compare(self.entities(doc),case['expected'])
                    uids = [r.uid for r in doc.entities()]
                    cad.entities.transform(doc,uids=uids,transform=cad.MoveTransform(dx=1.25,dy=-2.5))
                    moved = copy.deepcopy(case['expected'])
                    for entity in moved:
                        for p in ([entity['a'],entity['b']] if entity['kind']=='line' else
                                  [entity['p']] if entity['kind']=='point' else entity['pts']):
                            p['x'] += 1.25
                            p['y'] -= 2.5
                    self.compare(self.entities(doc),moved)
                    doc.undo()
                    self.compare(self.entities(doc),case['expected'])
                    doc.redo()
                    self.compare(self.entities(doc),moved)
                    native = Path(folder)/f"{case['name']}.kcad"
                    doc.save(native)
                    back = cad.Document.open(native)
                    self.compare(self.entities(back),moved)
                    self.assertEqual([r.uid for r in back.entities()],uids)
                    self.assertFalse(back.info().can_undo)
                    self.assertEqual(source.read_bytes(),before)
