"""Named setup persistence, application and command inverse through the typed SDK."""
import json,tempfile,unittest,subprocess
from pathlib import Path
import kentos.cad as cad
CASES=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/page-setups.json').read_text())['cases']
class PageSetups(unittest.TestCase):
 def test_twenty_named_setups_through_python(self):
  for c in CASES:
   with self.subTest(case=c['name']):
    doc=cad.Document.from_bytes(json.dumps(c['model']).encode());source=[r.entity.to_json() for r in doc.entities()]
    cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=c['sheet'])]);before=cad.sheets.set(doc,details=True).book.to_json()
    done=cad.sheets.edit(doc,ops=[dict(op='savePageSetups',setups=[c['setup']]),dict(op='applyPageSetup',sheet='page',setup='office')])
    kept=cad.sheets.set(doc,details=True).book.to_json();self.assertEqual(kept['sheets'][0]['plot']['settings']['area'],c['setup']['settings']['area'])
    reopened=cad.Document.from_bytes(doc.to_bytes());self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),kept)
    with tempfile.TemporaryDirectory() as folder:
     path=Path(folder)/'print.pdf';reopened.export_pdf(path);self.assertIn('PRINT',subprocess.check_output(['pdftotext',str(path),'-'],text=True))
    cad.sheets.edit(doc,ops=done.inverse);self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before);self.assertEqual([r.entity.to_json() for r in doc.entities()],source)
