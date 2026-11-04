"""CAD-ADV-11: typed agents see the same visible pattern loss without output or mutation."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class SheetLineTypes(unittest.TestCase):
    def test_preflight_names_source_and_preserves_document_and_output_directory(self):
        fx = json.loads((ROOT / 'fixtures/commands/v1/file.preflight.sheets.json').read_text())
        for mode in ["text", "unavailable", "unused", "long"]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(prefix='kentos-linetype-') as folder:
                raw = copy.deepcopy(fx['model'])
                raw['settings']['lineTypes'] = [dict(name='HW', unit='paper', elements=[
                    dict(length=2.), dict(length=-1.),
                    dict(length=0., text=dict(text='HW', style='STANDARD', scale=.1)),
                    dict(length=-1.),
                ])]
                if mode == "unavailable":
                    raw['settings']['lineTypes'][0]['elements'][2]['text']['style'] = 'MISSING_FONT_STYLE'
                if mode == "long":
                    raw['settings']['lineTypes'][0]['elements'] = [
                        dict(length=(i // 2 + 1) * (1 if i % 2 == 0 else -1))
                        for i in range(12)
                    ]
                if mode != "unused":
                    raw['entities'][0]['linePattern'] = 'HW'
                doc = cad.Document.from_bytes(json.dumps(raw).encode())
                wrapper = cad.command('cad.sheets.edit')
                wrapper.run(doc, wrapper.input_type.from_json(dict(ops=[
                    dict(op='addSheet', sheet=s) for s in fx['book']['sheets']
                ]))).unwrap()
                before = doc.to_bytes()
                query = cad.command('file.preflight.sheets')
                request = query.input_type.from_json(dict(drawing=str(Path(folder) / 'source.kcad')))
                reports = [query.run(doc, request, op=op).unwrap() for op in ['plan', 'execute']]
                for report in reports:
                    issues = [i for i in report.issues if i.code == 'linetype_unsupported']
                    self.assertEqual(bool(issues), mode == "unavailable")
                    if mode == "unavailable":
                        self.assertFalse(report.ready)
                        self.assertTrue(all(i.error and 'HW' in i.message for i in issues))
                    if mode in ["long", "text"]:
                        self.assertTrue(report.ready)
                self.assertEqual(reports[0].to_json(), reports[1].to_json())
                self.assertEqual(doc.to_bytes(), before)
                self.assertEqual(list(Path(folder).iterdir()), [])
