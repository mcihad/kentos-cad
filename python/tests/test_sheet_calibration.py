"""Physical reference dimensions are rational mm values, independent of the drawing-unit preference."""
import json,math,tempfile,unittest
from pathlib import Path
import kentos.cad as cad
from acceptance_files import pdf_pages
DATA=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/calibration.json').read_text())
class SheetCalibration(unittest.TestCase):
 def test_twenty_physical_marks_export_and_leave_source_unchanged(self):
  with tempfile.TemporaryDirectory() as folder:
   for n,c in enumerate(DATA['cases']):
    with self.subTest(c['name']):
     doc=cad.Document.from_bytes(json.dumps(c['model']).encode());before=[r.entity.to_json() for r in doc.entities()]
     with doc.group('Kalibrasyon'):cad.sheets.edit(doc,ops=[cad.AddSheetPaperOp(sheet=cad.PaperSheet.from_json(c['sheet']))])
     path=Path(folder)/f'{n}.pdf';doc.export_pdf(path)
     lengths=[math.hypot(s[2]-s[0],s[3]-s[1]) for s in pdf_pages(path.read_bytes())[0]['segments']]
     self.assertGreaterEqual(sum(abs(x-100)<c['expected']['toleranceMM'] for x in lengths),2)
     self.assertEqual([r.entity.to_json() for r in doc.entities()],before)
     self.assertIsNotNone(doc.undo());self.assertEqual(doc.sheets(),[])
 def test_calibration_template_is_typed_reopenable_and_one_undo(self):
  doc=cad.Document.from_bytes(json.dumps(DATA['cases'][6]['model']).encode())
  with doc.group('Kalibrasyon'):cad.sheets.create(doc,id='cal',source=cad.SystemSheetTemplateSource(id='sys:baski-kalibrasyonu'),options={'center':{'x':0,'y':0},'scale':200})
  book=cad.sheets.set(doc,details=True).book.to_json();viewport=next(i for i in book['sheets'][0]['items'] if i['kind']['type']=='map')
  self.assertTrue(viewport['kind']['viewLocked']);self.assertEqual(viewport['bindings'],[{'property':'map.scale','expression':'50'}])
  self.assertEqual(cad.sheets.set(cad.Document.from_bytes(doc.to_bytes()),details=True).book.to_json(),book)
  self.assertIsNotNone(doc.undo());self.assertEqual(doc.sheets(),[])
  self.assertIsNotNone(doc.redo());self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),book)
