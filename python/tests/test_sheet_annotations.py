"""Independent representation, freeze/style and locked camera acceptance through typed Python."""
import json,tempfile,subprocess,unittest
from pathlib import Path
import shutil
import kentos.cad as cad
# Poppler's pdftotext wherever PATH has it (Homebrew's on macOS, /usr/bin on Linux).
PDFTOTEXT=shutil.which('pdftotext') or '/opt/homebrew/bin/pdftotext'
CASES=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/viewport-annotations.json').read_text())['cases']
class SheetAnnotations(unittest.TestCase):
    def test_twenty_representations_preserve_geometry_camera_history_and_pdf_visibility(self):
        for c in CASES:
            with self.subTest(case=c['name']):
                doc=cad.Document.from_bytes(json.dumps(c['model']).encode())
                uid=next(doc.entities(kinds='text')).uid
                original=doc.entity(uid).entity.to_json()
                cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=c['sheet'])]);before=cad.sheets.set(doc,details=True).book.to_json()

                with doc.group('Açıklama ve katman'):
                    cad.sheets.edit(doc,ops=[c['operation']])
                result=cad.sheets.set(doc,details=True)
                self.assertEqual(result.sheets[0].frames[0].scale,c['expected']['physical'])
                self.assertEqual(result.book.to_json()['sheets'][0]['items'][0]['kind']['view'],before['sheets'][0]['items'][0]['kind']['view'])
                self.assertIn(50.,result.annotation_scales)
                with tempfile.TemporaryDirectory() as folder:
                    path=Path(folder)/'annotation.pdf';doc.export_pdf(path)
                    text=subprocess.check_output([PDFTOTEXT,str(path),'-'],text=True)
                    self.assertEqual('ANNO' in text,c['expected']['visible'])
                self.assertEqual(doc.entity(uid).entity.to_json(),original)
                reopened=cad.Document.from_bytes(doc.to_bytes());self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),result.book.to_json())
                doc.undo();self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before)
                cad.sheets.edit(doc,ops=[c['operation'],{'op':'setViewport','id':'viewport','annotation':{'type':'auto'},'layerOverrides':[]}])
                item=cad.sheets.set(doc,details=True).book.to_json()['sheets'][0]['items'][0]['kind'];self.assertNotIn('annotationScale',item);self.assertNotIn('layerOverrides',item)
