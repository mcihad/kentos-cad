"""Viewport navigation and explicitly scoped model commands use the same model, metres and undo."""
import json,unittest
from pathlib import Path
import kentos.cad as cad
DATA=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/viewport-navigation.json').read_text())['cases']
class SheetModelScope(unittest.TestCase):
    def test_navigation_and_explicit_model_scope_preserve_one_source_and_undo(self):
        for case in DATA:
            with self.subTest(case=case['name']):
                doc=cad.Document.new('Kapsam',workspace='cad');cad.sheets.edit(doc,ops=[{'op':'addSheet','sheet':case['sheet']}])
                before=cad.sheets.set(doc,details=True).book.to_json()
                nav=cad.NavigateViewportPaperOp.from_json(case['operation'])
                cad.sheets.edit(doc,ops=[nav]);after=cad.sheets.set(doc,details=True).book.to_json()
                view=after['sheets'][0]['items'][0]['kind']['view']
                for j,k in enumerate(['x','y']):self.assertAlmostEqual(view['center'][k],case['expected']['center'][j],places=8)
                self.assertAlmostEqual(view['ratio']['model']/view['ratio']['paper'] if 'ratio' in view else view['scale'],case['expected']['denominator'])
                scope=cad.SheetModelScope(sheet='page',item='viewport')
                out=cad.changeset.apply(doc,model_scope=scope,steps=[cad.ChangesetStep(command='cad.line.create',input={'layerId':doc.active_layer,'a':{'x':1,'y':2},'b':{'x':6,'y':2}})])
                uid=out.steps[0].output['uid'];self.assertAlmostEqual(doc.measure(uid).length,5.)
                self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),after)
                doc.undo();self.assertEqual(doc.info().objects,0)
                doc.redo();self.assertAlmostEqual(doc.measure(uid).length,5.)
                # Scope cannot silently become another page/model or mutate paper in the group.
                for bad_scope,command,input_,error in [
                    (cad.SheetModelScope(sheet='wrong',item='viewport'),'cad.line.create',{'layerId':doc.active_layer,'a':{'x':0,'y':0},'b':{'x':1,'y':0}},'unknown_sheet'),
                    (cad.SheetModelScope(sheet='page',item='wrong'),'cad.line.create',{},'scope_mismatch'),
                    (scope,'cad.sheets.edit',{'ops':[{'op':'removeSheet','id':'page'}]},'model_scope_command'),
                ]:
                    with self.assertRaises(cad.CommandError) as caught:cad.changeset.apply(doc,model_scope=bad_scope,steps=[cad.ChangesetStep(command=command,input=input_)])
                    self.assertEqual(caught.exception.code,error)
                    self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),after)
                self.assertEqual(cad.sheets.set(cad.Document.from_bytes(doc.to_bytes()),details=True).book.to_json(),after)
