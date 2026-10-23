"""Requested font faces, Unicode and analytic paper geometry also export through the typed Python surface."""
import json,subprocess,tempfile,unittest
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
DATA=json.loads((ROOT/'fixtures/sheet/v1/plot-document.json').read_text())
class PlotDocument(unittest.TestCase):
 def test_twenty_two_font_and_curve_pages_survive_kcad_and_export(self):
  doc=cad.Document.from_bytes(json.dumps(DATA['model']).encode());source=[r.entity.to_json() for r in doc.entities()]
  cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=c['sheet']) for c in DATA['cases']])
  book=cad.sheets.set(doc,details=True).book.to_json();reopened=cad.Document.from_bytes(doc.to_bytes());self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),book)
  with tempfile.TemporaryDirectory() as folder:
   path=Path(folder)/'quality.pdf';reopened.export_pdf(path);pages=subprocess.check_output(['pdftotext',str(path),'-'],text=True).split('\f')
   for n,c in enumerate(DATA['cases']):self.assertIn(c['expected']['text'],pages[n]);self.assertIn('MODEL 5000 mm',pages[n])
   rows=subprocess.check_output(['pdffonts',str(path)],text=True).splitlines()[2:];self.assertEqual(len(rows),22)
   for row in rows:self.assertEqual(row.split()[-5:-2],['yes']*3)
  self.assertEqual([r.entity.to_json() for r in doc.entities()],source)
