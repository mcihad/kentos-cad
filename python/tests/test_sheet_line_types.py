"""CAD-ADV-11: typed agents see the same visible pattern loss without output or mutation."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class SheetLineTypes(unittest.TestCase):
    def test_shx_attachment_uses_the_public_library_command_and_travels_with_kcad(self):
        fx=json.loads((ROOT/'fixtures/commands/v1/file.preflight.sheets.json').read_text())
        resource=json.loads((ROOT/'fixtures/kcad/v2/shape-resources.json').read_text())['styles']['items'][0]
        raw=copy.deepcopy(fx['model'])
        raw['settings']['lineTypes']=[dict(name='FENCE',unit='paper',elements=[dict(length=5.),dict(length=-1.),dict(length=0.,shape=dict(name='FENCE',file='FENCE.SHX')),dict(length=-2.)])]
        raw['entities'][0]['linePattern']='FENCE'
        doc=cad.Document.from_bytes(json.dumps(raw).encode())
        library=cad.command('cad.library.edit')
        request=library.input_type.from_json(dict(operation='create',item=resource))
        before=doc.to_bytes()
        library.run(doc,request,op='plan').unwrap()
        self.assertEqual(doc.to_bytes(),before)
        library.run(doc,request).unwrap()
        saved=doc.to_bytes()
        restored=cad.Document.from_bytes(saved)
        exported=library.run(restored,library.input_type.from_json(dict(operation='export',ids=['shx:fence.shx']))).unwrap()
        package=json.loads(exported.kstil)
        self.assertEqual(package['version'],3)
        self.assertEqual(package['items'],[resource])
        refused=library.run(restored,library.input_type.from_json(dict(operation='remove',id='shx:fence.shx')))
        self.assertFalse(refused.ok)
        self.assertEqual(restored.to_bytes(),saved)

    def test_preflight_names_source_and_preserves_document_and_output_directory(self):
        fx = json.loads((ROOT / 'fixtures/commands/v1/file.preflight.sheets.json').read_text())
        for mode in ["text", "unavailable", "unused", "long", "wide", "wide_dense"]:
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
                if mode in ["wide", "wide_dense"]:
                    raw['entities'][0] = dict(kind='polyline', id=1, layerId=raw['entities'][0]['layerId'], attrs={},
                        pts=[dict(x=0., y=0.), dict(x=2., y=0.)], widths=[[.1, .3], [0., 0.]])
                    if mode == "wide_dense":
                        raw['settings']['lineTypes'][0]['elements'] = [dict(length=.00001), dict(length=-.00001)]
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
                    self.assertEqual(bool(issues), mode in ["unavailable", "wide_dense"])
                    if mode in ["unavailable", "wide_dense"]:
                        self.assertFalse(report.ready)
                        self.assertTrue(all(i.error for i in issues))
                        if mode == "unavailable":
                            self.assertTrue(all('HW' in i.message for i in issues))
                    if mode in ["long", "text", "wide"]:
                        self.assertTrue(report.ready)
                self.assertEqual(reports[0].to_json(), reports[1].to_json())
                self.assertEqual(doc.to_bytes(), before)
                self.assertEqual(list(Path(folder).iterdir()), [])
