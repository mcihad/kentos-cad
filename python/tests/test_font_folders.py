"""ADR 9313: the fonts folders, through the public commands.

`cad.fonts.report` lists the typefaces a person can pick, KentOS's own nine first, and the fonts folders' files as the
program read them when the module was loaded. A project takes a fonts folder's family by its name and the project file
keeps it; a family this program's folders do not hold is drawn in Arial and said (`font_substituted`).
"""
import copy
import json
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
OWN = ["arial", "times-new-roman", "barlow", "arimo", "overpass", "quicksand", "architects-daughter",
       "courier-prime", "plex-mono"]
ROLES = {"own", "twin", "given", "interface", "fallback", "family", "unused"}
FOLDERS = {"program", "user", "system"}


class FontFolders(unittest.TestCase):
    def drawing(self, font):
        fx = json.loads((ROOT / 'fixtures/commands/v1/file.preflight.sheets.json').read_text())
        raw = copy.deepcopy(fx['model'])
        raw['settings']['drawingFont'] = font
        first = raw['entities'][0]
        raw['entities'].append(dict(kind='text', id=max(e['id'] for e in raw['entities']) + 1,
                                    layerId=first['layerId'], attrs={}, p=first.get('a', dict(x=0., y=0.)),
                                    text='Ada 101', height=1., rotation=0.))
        return cad.Document.from_bytes(json.dumps(raw).encode())

    def report(self, doc):
        report = cad.command('cad.fonts.report')
        done = report.run(doc, report.input_type.from_json({}))
        return done.unwrap().to_json(), done.warnings

    def test_the_choices_start_with_the_own_faces_and_the_files_say_their_use(self):
        out, _ = self.report(self.drawing('arial'))
        self.assertEqual(out['choices'][:9], OWN)
        for f in out['files']:
            self.assertIn(f['role'], ROLES, f)
            self.assertIn(f['folder'], FOLDERS, f)
            if f['role'] == 'family':
                # A family the lists offer, by its name as its file gives it.
                self.assertEqual(f['face'], f['family'])
                self.assertIn(f['family'], out['choices'])
            if f['role'] == 'unused':
                self.assertTrue(f.get('note'), f)

    def test_a_folder_family_is_kept_and_a_missing_one_is_said(self):
        doc = self.drawing('Kurum Sans')
        out, warnings = self.report(doc)
        self.assertEqual(out['project'], 'Kurum Sans')
        self.assertIn('Kurum Sans', out['faces'])
        said = [w for w in warnings if w.code == 'font_substituted' and '“Kurum Sans”' in w.message]
        self.assertEqual(len(said), 1, [w.message for w in warnings])
        self.assertIn('Arial', said[0].message)
        # The project file keeps the name as it is written.
        back = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(self.report(back)[0]['project'], 'Kurum Sans')

    def test_a_name_no_family_may_have_is_refused(self):
        with self.assertRaises(Exception):
            self.drawing('A' * 65)


if __name__ == '__main__':
    unittest.main()
