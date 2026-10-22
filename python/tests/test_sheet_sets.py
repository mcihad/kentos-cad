"""Independent six-page architectural books with page-specific shared master map contents."""
import json,tempfile,subprocess,unittest
from pathlib import Path
import kentos.cad as cad
CASES=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/multipage-sets.json').read_text())['cases']
class MultipageSets(unittest.TestCase):
    def test_twenty_sets_preserve_architectural_views_pages_master_content_and_source(self):
        for c in CASES:
            with self.subTest(case=c['name']):
                doc=cad.Document.from_bytes(json.dumps(c['model']).encode());source=[r.entity.to_json() for r in doc.entities()]
                ops=[{'op':'addMaster','master':m} for m in c['book']['masters']]+[{'op':'addSheet','sheet':s} for s in c['book']['sheets']]
                cad.sheets.edit(doc,ops=ops);state=cad.sheets.set(doc,details=True)
                self.assertEqual([s.id for s in state.sheets],c['expected']['pages']);self.assertEqual(len(state.book.to_json()['masters']),1)
                self.assertEqual({s.series for s in state.sheets},{'A-0','A-1','A-2','A-3','A-5'})
                reopened=cad.Document.from_bytes(doc.to_bytes());self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),state.book.to_json())
                with tempfile.TemporaryDirectory() as folder:
                    path=Path(folder)/'set.pdf';reopened.export_pdf(path);pages=subprocess.check_output(['pdftotext','-layout',str(path),'-'],text=True).split('\f')
                    self.assertNotIn('FAR',pages[0])
                    for i,words in enumerate(c['expected']['words']):
                        for word in words:self.assertIn(word,pages[i])
                self.assertEqual([r.entity.to_json() for r in doc.entities()],source);self.assertEqual(len(source),c['expected']['sourceCount'])
