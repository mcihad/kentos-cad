"""CAD-ADV-11, ADR 1125: a text in a style whose SHX font the project embeds, through the public commands.

It is drawn in the font's strokes (no substitute in the font report, none in the sheets' preflight), a letter the font
lacks is an explicit error that blocks the PDF, and a DXF export writes the style's fonts beside the drawing. The
fonts are KentOS's own test fonts (scripts/fixtures/shx_font_reference.py, fixtures/shx/v1).
"""
import copy
import json
import tempfile
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def resources(names):
    items = json.loads((ROOT / 'fixtures/kcad/v2/shx-fonts.json').read_text())['styles']['items']
    return [i for i in items if i['file'] in names]


class ShxTexts(unittest.TestCase):
    def drawing(self, words, fonts=('kentos-uni.shx', 'kentos-big.shx')):
        fx = json.loads((ROOT / 'fixtures/commands/v1/file.preflight.sheets.json').read_text())
        raw = copy.deepcopy(fx['model'])
        raw['settings']['textStyles'] = [dict(id='uni', name='Uni', font='barlow', fontFile='kentos-uni.shx',
                                              bigFontFile='kentos-big.shx')]
        raw['styles'] = dict(items=resources(fonts), categories=[])
        first = raw['entities'][0]
        raw['entities'].append(dict(kind='text', id=max(e['id'] for e in raw['entities']) + 1,
                                    layerId=first['layerId'], attrs={}, p=first.get('a', dict(x=0., y=0.)),
                                    text=words, height=1., rotation=0., textStyle='uni'))
        doc = cad.Document.from_bytes(json.dumps(raw).encode())
        sheets = cad.command('cad.sheets.edit')
        sheets.run(doc, sheets.input_type.from_json(dict(ops=[
            dict(op='addSheet', sheet=s) for s in fx['book']['sheets']]))).unwrap()
        return doc

    def issues(self, doc, code):
        query = cad.command('file.preflight.sheets')
        with tempfile.TemporaryDirectory(prefix='kentos-shx-text-') as folder:
            request = query.input_type.from_json(dict(drawing=str(Path(folder) / 'source.kcad')))
            reports = [query.run(doc, request, op=op).unwrap() for op in ['plan', 'execute']]
            self.assertEqual(reports[0].to_json(), reports[1].to_json())
        return [i for i in reports[0].issues if i.code == code]

    def fonts(self, doc):
        report = cad.command('cad.fonts.report')
        done = report.run(doc, report.input_type.from_json({}))
        return done.unwrap().to_json(), [w.code for w in done.warnings]

    def test_an_embedded_shx_style_is_drawn_from_its_font_and_travels_beside_a_dxf(self):
        doc = self.drawing('ŞU\\M+18140')
        out, warnings = self.fonts(doc)
        self.assertEqual(out['styles'][0]['embedded'], True)
        self.assertNotIn('font_substituted', warnings)
        self.assertEqual(self.issues(doc, 'text_font_substituted'), [])
        self.assertEqual(self.issues(doc, 'text_shx_unsupported'), [])
        before = doc.to_bytes()
        with tempfile.TemporaryDirectory(prefix='kentos-shx-dxf-') as folder:
            path = Path(folder) / 'plan.dxf'
            planned = cad.file.export.dxf.plan(doc, path=str(path))
            files = [r['file'] for r in planned.report['shapeResources']]
            self.assertEqual(files, ['kentos-uni.shx', 'kentos-big.shx'])
            self.assertEqual(sorted(p.name for p in Path(folder).iterdir()), [])
            cad.file.export.dxf(doc, path=str(path))
            for name in files:
                want = (ROOT / 'fixtures/shx/v1' / name).read_bytes()
                self.assertEqual((Path(folder) / name).read_bytes(), want, name)
        self.assertEqual(doc.to_bytes(), before)

    def test_a_letter_the_font_lacks_is_an_error_and_a_font_not_embedded_a_named_substitute(self):
        doc = self.drawing('SAU')
        errors = self.issues(doc, 'text_shx_unsupported')
        self.assertTrue(errors and all(i.error for i in errors) and 'U+0041' in errors[0].message, errors)
        # The Big Font not embedded: the style's face draws it, said with the missing file's name.
        doc = self.drawing('SU', fonts=('kentos-uni.shx',))
        out, warnings = self.fonts(doc)
        self.assertNotIn('embedded', out['styles'][0])
        self.assertIn('font_substituted', warnings)
        warned = self.issues(doc, 'text_font_substituted')
        self.assertTrue(warned and 'kentos-big.shx' in warned[0].message, warned)


if __name__ == '__main__':
    unittest.main()
