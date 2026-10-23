"""The public Python path preserves source metadata and guards planned resource versions."""
import hashlib,json,shutil,subprocess,tempfile,unittest
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
FX=json.loads((ROOT/'fixtures/sheet/v1/source-snapshot.json').read_text())
class SheetSource(unittest.TestCase):
 def test_source_pin_in_pdf_and_manifest_matches_the_plan_after_request_bookkeeping(self):
  with tempfile.TemporaryDirectory() as folder:
   root=Path(folder);model=json.loads(json.dumps(FX['model']));logo=root/'logo.png';shutil.copy(ROOT/'fixtures'/FX['imageA'],logo)
   model['entities'].append(dict(kind='image',id=20,layerId='cizim',attrs={},file=str(logo),p=dict(x=2.4,y=-.1),width=.2,height=.2,rotation=0))
   doc=cad.Document.from_bytes(json.dumps(model).encode());cad.sheets.edit(doc,ops=[cad.AddSheetPaperOp(sheet=cad.PaperSheet.from_json(s))for s in FX['book']['sheets']])
   wrapper=cad.command('file.publish.sheets');inp=wrapper.input_type.from_json(dict(path=str(root/'a')))
   planned=wrapper.run(doc,inp,op='plan').output.to_json();self.assertFalse((root/'a').exists())
   issued=wrapper.run(doc,inp,request_id='first-source').output.to_json();self.assertEqual(issued['source'],planned['source'])
   manifest=json.loads((root/'a/yayin.json').read_text());snapshot=manifest['snapshot'];self.assertTrue(snapshot['laterEditsExcluded'])
   self.assertEqual(snapshot['source'],planned['source']);self.assertEqual(snapshot['images'][0]['sha256'],hashlib.sha256(logo.read_bytes()).hexdigest())
   for entry in issued['files']:
    info=subprocess.check_output(['pdfinfo',str(root/'a'/entry['name'])],text=True)
    keywords=next(l.split(':',1)[1].strip()for l in info.splitlines()if l.startswith('Keywords:'))
    self.assertEqual(json.loads(keywords),snapshot)
   b=wrapper.input_type.from_json(dict(path=str(root/'b'),pin=planned['source']))
   self.assertEqual(wrapper.run(doc,b,op='plan').output.to_json()['source'],planned['source'])
   shutil.copy(ROOT/'fixtures'/FX['imageB'],logo)
   refused=wrapper.run(doc,b);self.assertEqual(refused.status,'conflict');self.assertEqual(refused.error.code,'pin_mismatch');self.assertFalse((root/'b').exists())
