"""The independent drawing-set reference through generated typed operations, with real persistence and undo."""
import copy,json,tempfile,unittest
from pathlib import Path
import kentos.cad as cad
DATA=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/drawing-set.json').read_text())
class DrawingSet(unittest.TestCase):
 def test_independent_cases_metadata_roundtrip_and_inverse(self):
  for c in DATA['cases']:
   with self.subTest(c['name']):
    if not c['ops']:continue
    doc=cad.Document.from_bytes(json.dumps(DATA['model']).encode())
    cad.sheets.edit(doc,ops=[cad.AddMasterPaperOp(master=cad.PaperMaster.from_json(m)) for m in c['book'].get('masters',[])]+[cad.AddSheetPaperOp(sheet=cad.PaperSheet.from_json(s)) for s in c['book']['sheets']])
    before=cad.sheets.set(doc,details=True).book.to_json();model=[r.entity.to_json() for r in doc.entities()]
    wrapper=cad.command('cad.sheets.edit');answer=wrapper.run(doc,wrapper.input_type.from_json(dict(ops=c['ops'])))
    if c['code']:
     self.assertEqual(answer.error.code,c['code']);self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before);continue
    self.assertEqual(answer.status,'completed')
    book=cad.sheets.set(doc,details=True).book.to_json()
    self.assertEqual({s['id']:s.get('drawingInfo') for s in book['sheets']},c['metadata'])
    self.assertEqual(cad.sheets.set(cad.Document.from_bytes(doc.to_bytes()),details=True).book.to_json(),book)
    back=wrapper.run(doc,wrapper.input_type.from_json(dict(ops=answer.output.inverse)))
    self.assertEqual(back.status,'completed');self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before)
    self.assertEqual([r.entity.to_json() for r in doc.entities()],model)
 def test_metadata_batch_is_one_unified_undo(self):
  doc=cad.Document.from_bytes(json.dumps(DATA['model']).encode())
  cad.sheets.edit(doc,ops=[cad.AddSheetPaperOp(sheet=cad.PaperSheet.from_json(s)) for s in DATA['book']['sheets']])
  before=cad.sheets.set(doc,details=True).book.to_json()
  with doc.group('Pafta seti'):
   cad.command('cad.sheets.edit').run(doc,cad.SheetsEdit.from_json(dict(ops=[dict(op='setDrawingSet',entries=[dict(id='ground',info=dict(number='A-01',title='Zemin kat',revision='B',status='Yayın',subset=['Mimari']))])]))).unwrap()
  after=cad.sheets.set(doc,details=True).book.to_json();self.assertNotEqual(after,before)
  self.assertIsNotNone(doc.undo());self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before)
  self.assertIsNotNone(doc.redo());self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),after)
