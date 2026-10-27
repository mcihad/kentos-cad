"""A Kot bound to a level through the public Python host (docs/adr/0721 §2): its value is the level's elevation plus
its offset, it follows the level when the project's settings raise it, it keeps the level from being taken away (its
id in the refusal), and the file it goes into says it (schema 121, read by the independent KCAD reader)."""
from dataclasses import replace
from pathlib import Path
import sys
import tempfile
import unittest

from kentos import cad
from kentos.cad.errors import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/kcad'))
import kcad  # Independent saved-file reader.

LEVELLED = (b'{"format":"kentos.document","version":1,"name":"Katlar","settings":{"srid":0,"lengthDecimals":3,'
            b'"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.0,"workspace":"cad","arch":{"levels":['
            b'{"id":"zemin","name":"Zemin kat","elevation":0.0},{"id":"k1","name":"1. kat","elevation":3.25}]}},'
            b'"origin":{"x":0.0,"y":0.0},"layers":[{"id":"olcu","name":"\xc3\x96l\xc3\xa7\xc3\xbc","type":"layer",'
            b'"visible":true,"locked":false,"expanded":true,"style":{"color":"fg","lineType":"continuous",'
            b'"lineWeight":0.25},"children":[]}],"activeLayer":"olcu","entities":[],"styles":{"items":[],"categories":[]}}')


class DimensionLevels(unittest.TestCase):
    def kot(self, doc, level, at=(4.0, 3.0)):
        made = cad.dimensions.create(doc, layer_id='olcu', at=cad.Vec2(x=at[0] + 1, y=at[1] + 1),
                                     measure=cad.ElevationDimensionMeasure(a=cad.AtMeasuredPoint(at=cad.Vec2(x=at[0], y=at[1])),
                                                                           level=level))
        return made.created[0]

    def test_a_kot_reads_its_level_follows_it_and_keeps_it(self):
        doc = cad.Document.from_bytes(LEVELLED)
        uid = self.kot(doc, cad.DimensionLevel(id='k1', offset=-0.02))
        d = doc.entity(uid).entity
        self.assertEqual((d.za, d.level.id, d.level.offset), (3.25 + -0.02, 'k1', -0.02))
        # Raised in the project's settings: the Kot follows, no undo step of its own.
        arch = doc.info().settings.arch
        raised = [arch.levels[0], replace(arch.levels[1], elevation=6.5)]
        cad.settings.set(doc, arch=replace(arch, levels=raised))
        self.assertEqual(doc.entity(uid).entity.za, 6.5 + -0.02)
        # Taken away while the Kot names it: kept, the Kot's id said.
        before = doc.to_bytes()
        with self.assertRaises(CommandFailed) as refused:
            cad.settings.set(doc, arch=replace(arch, levels=raised[:1]))
        self.assertEqual(refused.exception.code, 'level_in_use')
        self.assertTrue(refused.exception.message.startswith('“1. kat” katını 1 kot işareti anıyor; kaldırılmadı.'))
        self.assertEqual(list(refused.exception.uids), [uid])
        self.assertEqual(doc.to_bytes(), before)
        # Set free through cad.dimensions.edit: the value it had stays.
        cad.dimensions.edit(doc, edit=cad.LevelDimensionEdit(uids=[uid], level=None))
        d = doc.entity(uid).entity
        self.assertEqual((d.za, d.level), (6.5 + -0.02, cad.UNSET))

    def test_the_file_says_the_level_in_schema_121(self):
        doc = cad.Document.from_bytes(LEVELLED)
        self.kot(doc, cad.DimensionLevel(id='zemin'))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'kotlar.kcad'
            doc.save(path)
            data = path.read_bytes()
            self.assertEqual(kcad.decode_payload(kcad.read_header(data)[1])['version'], 121)
            content = kcad.read(data)[1]
            kot = content['entities'][0]
            self.assertEqual((kot['style'], kot['level'], kot['za']), ('elevation', {'id': 'zemin'}, 0.0))
            back = cad.Document.open(path)
            self.assertEqual(next(iter(back.entities())).entity.level.id, 'zemin')
        # Not both a z and a level; a level the project lacks is refused.
        with self.assertRaises(CommandFailed) as both:
            cad.dimensions.create(doc, layer_id='olcu', at=cad.Vec2(x=1, y=1), measure=cad.ElevationDimensionMeasure(
                a=cad.AtMeasuredPoint(at=cad.Vec2(x=0, y=0)), z=3.0, level=cad.DimensionLevel(id='k1')))
        self.assertEqual((both.exception.code, both.exception.path), ('invalid_dimension', 'measure.level'))
        with self.assertRaises(CommandFailed) as lacking:
            self.kot(doc, cad.DimensionLevel(id='k9'))
        self.assertEqual(lacking.exception.code, 'level_not_found')


if __name__ == '__main__':
    unittest.main()
