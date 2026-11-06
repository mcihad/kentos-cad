"""CAD-ADV-11, ADR 1120: SHX unifont and Big Font text in a line type through the public commands.

The fonts are KentOS's own test fonts (scripts/fixtures/shx_font_reference.py, fixtures/shx/v1).
"""
import copy
import json
import tempfile
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
STYLE = '0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1120'


def resource(name):
    return json.loads((ROOT / 'fixtures/kcad/v2/shx-fonts.json').read_text())['styles']['items'][
        ['kentos-uni.shx', 'kentos-big.shx'].index(name)]


class LineTypeShxFonts(unittest.TestCase):
    def drawing(self, text):
        fx = json.loads((ROOT / 'fixtures/commands/v1/file.preflight.sheets.json').read_text())
        raw = copy.deepcopy(fx['model'])
        raw['settings']['lineTypes'] = [dict(name='SU', unit='paper', elements=[
            dict(length=6.), dict(length=-1.),
            dict(length=0., text=dict(text=text, style='Uni', scale=2.)),
            dict(length=-6.),
        ])]
        raw['entities'][0]['linePattern'] = 'SU'
        doc = cad.Document.from_bytes(json.dumps(raw).encode())
        sheets = cad.command('cad.sheets.edit')
        sheets.run(doc, sheets.input_type.from_json(dict(ops=[
            dict(op='addSheet', sheet=s) for s in fx['book']['sheets']]))).unwrap()
        return doc

    def losses(self, doc):
        query = cad.command('file.preflight.sheets')
        with tempfile.TemporaryDirectory(prefix='kentos-shx-font-') as folder:
            request = query.input_type.from_json(dict(drawing=str(Path(folder) / 'source.kcad')))
            reports = [query.run(doc, request, op=op).unwrap() for op in ['plan', 'execute']]
            self.assertEqual(reports[0].to_json(), reports[1].to_json())
        return [i for i in reports[0].issues if i.code == 'linetype_unsupported']

    def test_a_unifont_and_big_font_style_draws_and_travels_with_kcad(self):
        doc = self.drawing('ŞU\\M+18140')
        edit = cad.command('cad.styles.edit')
        create = edit.input_type.from_json(dict(kind='text', operation='create', text=dict(
            id=STYLE, name='Uni', font='barlow', fontFile='kentos-uni.shx', bigFontFile='kentos-big.shx')))
        edit.run(doc, create).unwrap()
        # Neither font is in the project yet: an explicit loss naming the file, never a gap.
        issues = self.losses(doc)
        self.assertTrue(issues and all(i.error for i in issues))
        self.assertTrue(any('kentos-uni.shx' in i.message for i in issues))
        library = cad.command('cad.library.edit')
        for name in ['kentos-uni.shx', 'kentos-big.shx']:
            library.run(doc, library.input_type.from_json(dict(operation='create', item=resource(name)))).unwrap()
        self.assertEqual(self.losses(doc), [])
        # A font a text style draws with is in use: removing it is refused, the drawing unchanged.
        saved = doc.to_bytes()
        refused = library.run(doc, library.input_type.from_json(dict(operation='remove', id='shx:kentos-big.shx')))
        self.assertFalse(refused.ok)
        self.assertEqual(doc.to_bytes(), saved)
        reopened = cad.Document.from_bytes(saved)
        self.assertEqual(self.losses(reopened), [])

    def test_a_missing_glyph_or_big_font_is_a_loss(self):
        for text, words in [('SQU', 'Q'), ('S\\M+18150', '8150')]:
            with self.subTest(text=text):
                doc = self.drawing(text)
                edit = cad.command('cad.styles.edit')
                edit.run(doc, edit.input_type.from_json(dict(kind='text', operation='create', text=dict(
                    id=STYLE, name='Uni', font='barlow', fontFile='kentos-uni.shx',
                    bigFontFile='kentos-big.shx')))).unwrap()
                library = cad.command('cad.library.edit')
                for name in ['kentos-uni.shx', 'kentos-big.shx']:
                    library.run(doc, library.input_type.from_json(dict(operation='create', item=resource(name)))).unwrap()
                issues = self.losses(doc)
                self.assertTrue(issues and any(words in i.message for i in issues), issues)

    def test_a_big_font_that_is_not_an_shx_file_is_refused(self):
        doc = self.drawing('SU')
        edit = cad.command('cad.styles.edit')
        before = doc.to_bytes()
        bad = edit.run(doc, edit.input_type.from_json(dict(kind='text', operation='create', text=dict(
            id=STYLE, name='Uni', font='barlow', fontFile='kentos-uni.shx', bigFontFile='kentos-big.ttf'))))
        self.assertFalse(bad.ok)
        self.assertEqual(doc.to_bytes(), before)


if __name__ == '__main__':
    unittest.main()
