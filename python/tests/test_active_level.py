"""Etkin kat through the public Python host (docs/adr/0720): every level is explicit in a command's input, a level an
object names stays (its objects' ids in the refusal), and the level and view a drawing opens with are its settings,
written into its file and read back (Katsız as None)."""
import json
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


class ActiveLevel(unittest.TestCase):
    def drawing(self):
        # ADR 0717's drawing, its file's v2 form written by the independent KCAD writer.
        setup = json.loads((ROOT / 'fixtures/commands/v1/cad.levels.assign.json').read_text())['setup']
        sys.path.insert(0, str(ROOT / 'scripts/fixtures'))
        import kcad_v2_reference as kr
        return cad.Document.from_bytes(kr.container(kr.document(setup)))

    def wall(self, doc, level):
        return cad.walls.create(doc, layer_id='duvar', walls=[cad.NewWall(
            pts=[cad.Vec2(x=0, y=40), cad.Vec2(x=5, y=40)], width=0.2, arch=cad.ArchInfo(level=level))])

    def test_levels_are_explicit_and_one_an_object_names_stays(self):
        doc = self.drawing()
        uid = self.wall(doc, 'kat-1').uids[0]
        self.assertEqual(doc.entity(uid).entity.arch.level, 'kat-1')
        answer = cad.walls.create.answer(doc, op='execute', layer_id='duvar', walls=[cad.NewWall(
            pts=[cad.Vec2(x=0, y=50), cad.Vec2(x=5, y=50)], width=0.2, arch=cad.ArchInfo(level='kat-9'))])
        self.assertFalse(answer.ok)
        self.assertEqual(answer.error.code, 'level_not_found')
        arch = doc.info().settings.arch
        before = doc.to_bytes()
        with self.assertRaises(CommandFailed) as refused:
            cad.settings.set(doc, arch=replace(arch, levels=arch.levels[:1]))
        self.assertEqual(refused.exception.code, 'level_in_use')
        self.assertTrue(refused.exception.message.startswith('“1. kat” katını 2 duvar anıyor; kaldırılmadı.'), refused.exception)
        self.assertIn(uid, refused.exception.uids)
        self.assertEqual(doc.to_bytes(), before)

    def test_the_level_and_view_a_drawing_opens_with_go_into_its_file(self):
        doc = self.drawing()
        arch = doc.info().settings.arch
        for active, view in [('kat-1', 'below'), (None, 'active')]:
            cad.settings.set(doc, arch=replace(arch, active_level=active, level_view=view))
            with tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / 'katlar.kcad'
                doc.save(path)
                written = kcad.read(path.read_bytes())[1]['settings']['arch']
                self.assertEqual((written['activeLevel'], written['levelView']), (active, view))
                back = cad.Document.open(path).info().settings.arch
                self.assertEqual((back.active_level, str(back.level_view)), (active, view))
        # Bütün katlar is no setting; nothing said, nothing written (the file stays in its old schema).
        cad.settings.set(doc, arch=replace(arch, level_view='all'))
        self.assertNotIn('levelView', kcad.read(doc.to_bytes())[1]['settings']['arch'])
        with self.assertRaises(CommandFailed) as refused:
            cad.settings.set(doc, arch=replace(arch, active_level='yok'))
        self.assertEqual(refused.exception.code, 'invalid_arch')


if __name__ == '__main__':
    unittest.main()
