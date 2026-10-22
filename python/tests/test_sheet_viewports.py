"""The independent viewport cases through typed Python, plus real DXF flags and fractional scales."""
import json,tempfile,unittest,warnings
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
DATA=json.loads((ROOT/'fixtures/sheet/v1/viewports.json').read_text())
class Viewports(unittest.TestCase):
    def test_common_viewport_cases_and_undo_are_the_same_as_the_designer(self):
        for case in DATA['cases']:
            with self.subTest(case=case['name']):
                doc=cad.Document.new('Görünüm',workspace='cad',drawing_unit='mm')
                cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=case['sheet'])])
                before=cad.sheets.set(doc,details=True).book.to_json()
                if 'error' in case['expected']:
                    with self.assertRaises(cad.CommandFailed) as failed:cad.sheets.edit(doc,ops=[case['operation']])
                    self.assertEqual(failed.exception.code,case['expected']['error'])
                    self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before)
                    continue
                with doc.group('Görünüm penceresi'):
                    output=cad.sheets.edit(doc,ops=[case['operation']])
                self.assertAlmostEqual(output.sheets[0].maps[0].scale,case['expected']['denominator'],places=9)
                current=cad.sheets.set(doc,details=True)
                for a,b in zip(current.sheets[0].frames[0].bounds,case['expected']['bounds']):self.assertAlmostEqual(a,b,places=8)
                item=current.book.to_json()['sheets'][0]['items'][0]
                self.assertEqual(item['kind'].get('source',dict(type='drawing')),case['expected']['source'])
                reopened=cad.Document.from_bytes(doc.to_bytes())
                self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),current.book.to_json())
                doc.undo();self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before)
                doc.redo();self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),current.book.to_json())

    def test_dxf_keeps_fractional_scale_lock_and_content_frame_without_turning_the_viewport_off(self):
        import ezdxf
        for numerator,denominator in [(1,37.5),(2,1),(3,2)]:
            with self.subTest(ratio=(numerator,denominator)),tempfile.TemporaryDirectory() as folder:
                doc=cad.Document.new('Kesirli ölçek',workspace='cad',drawing_unit='mm')
                line=cad.line.create(doc,layer_id=doc.active_layer,a=(0,0),b=(5,0))
                entity=doc.entity(line.uid).entity.to_json()
                cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=DATA['cases'][0]['sheet']),dict(op='setViewport',id='viewport',ratio=dict(paper=numerator,model=denominator),locked=True,rotation=30000)])
                output=Path(folder)/'viewport.dxf'
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore',cad.CommandWarning)
                    cad.file.export.dxf(doc,path=str(output))
                external=ezdxf.readfile(output)
                frames=[v for v in external.layouts.get('Görünüm').query('VIEWPORT') if v.dxf.id>1]
                self.assertEqual(len(frames),1);viewport=frames[0]
                self.assertEqual(viewport.dxf.flags&16384,16384)
                self.assertEqual(viewport.dxf.flags&131072,0)
                self.assertAlmostEqual(viewport.dxf.width,380)
                self.assertAlmostEqual(viewport.dxf.height,220)
                self.assertAlmostEqual(viewport.dxf.view_height/viewport.dxf.height,denominator/numerator)
                self.assertAlmostEqual(viewport.dxf.view_twist_angle,30)
                other=cad.Document.new('Yeniden oku',workspace='cad')
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore',cad.CommandWarning)
                    cad.file.import_.dxf(other,path=str(output))
                saved=cad.sheets.set(other,details=True).book.to_json()['sheets'][0]['items']
                imported=next(i['kind'] for i in saved if i['kind']['type']=='map')
                self.assertTrue(imported['viewLocked'])
                self.assertAlmostEqual(imported['view']['ratio']['model']/imported['view']['ratio']['paper'],denominator/numerator)
                self.assertEqual(doc.entity(line.uid).entity.to_json(),entity)
                binary=Path(folder)/'viewport.dwg'
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore',cad.CommandWarning)
                    cad.file.export.dwg(doc,path=str(binary),version=cad.DwgVersion.AC1032)
                    binary_doc=cad.Document.new('DWG yeniden oku',workspace='cad')
                    cad.file.import_.dwg(binary_doc,path=str(binary))
                binary_items=cad.sheets.set(binary_doc,details=True).book.to_json()['sheets'][0]['items']
                binary_map=next(i['kind'] for i in binary_items if i['kind']['type']=='map')
                self.assertTrue(binary_map['viewLocked'])
                pair=binary_map['view']['ratio'];self.assertAlmostEqual(pair['model']/pair['paper'],denominator/numerator)

