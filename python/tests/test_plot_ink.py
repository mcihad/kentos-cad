"""The same profile fields produce measured, searchable PDF output through the typed command surface."""
import json,tempfile,unittest,subprocess
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
CASES=json.loads((ROOT/'fixtures/sheet/v1/plot-ink.json').read_text())['cases']
class PlotInk(unittest.TestCase):
 def test_twenty_ink_profiles_export_without_rewriting_the_model(self):
  for c in CASES:
   with self.subTest(case=c['name']):
    doc=cad.Document.from_bytes(json.dumps(c['model']).encode());source=[r.entity.to_json() for r in doc.entities()]
    cad.sheets.edit(doc,ops=[dict(op='savePlotProfiles',profiles=[c['profile']]),dict(op='savePageSetups',setups=[c['setup']]),dict(op='addSheet',sheet=c['sheet']),dict(op='applyPageSetup',sheet='page',setup=c['setup']['id'])])
    book=cad.sheets.set(doc,details=True).book.to_json();self.assertEqual(book['sheets'][0]['plot']['settings']['profileSnapshot'],c['profile'])
    reopened=cad.Document.from_bytes(doc.to_bytes())
    with tempfile.TemporaryDirectory() as folder:
     path=Path(folder)/'ink.pdf';reopened.export_pdf(path);self.assertIn('PRINT',subprocess.check_output(['pdftotext',str(path),'-'],text=True))
    self.assertEqual([r.entity.to_json() for r in doc.entities()],source);self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),book)
