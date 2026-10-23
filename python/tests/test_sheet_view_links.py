"""Live named-view cameras, callout references, PDF scale and independent capture behavior."""
import json,copy,unittest,tempfile,subprocess,re,warnings
from pathlib import Path
import shutil
import kentos.cad as cad
# Poppler's pdftotext wherever PATH has it (Homebrew's on macOS, /usr/bin on Linux).
PDFTOTEXT=shutil.which('pdftotext') or '/opt/homebrew/bin/pdftotext'
DATA=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/viewport-links.json').read_text())['cases']
def save(doc,v):
    return cad.views.edit(doc,edit=cad.SaveViewsOp(name=v['name'],center=(v['center']['x'],v['center']['y']),width=v['width'],height=v['height'],twist=v['twist']))
class SheetViewLinks(unittest.TestCase):
    def test_twenty_links_follow_registry_rename_and_current_cameras_without_rewriting_capture(self):
        for c in DATA:
            with self.subTest(case=c['name']):
                doc=cad.Document.new('Canlı',workspace='cad');captured=save(doc,c['operation']['source']['view']).views[0].to_json()
                op=copy.deepcopy(c['operation']);op['source']['view']=captured
                cad.sheets.edit(doc,ops=[{'op':'addSheet','sheet':c['sheet']},op]);before=cad.sheets.set(doc,details=True).book.to_json()
                viewid=before['sheets'][0]['items'][0]['kind']['source']['view']['id']
                mark=cad.viewmarks.create(doc,layer_id=doc.active_layer,marks=[cad.NewViewMark(form='detail',points=[(0.,0.)],radius=.5,mark='42',target=cad.ViewTarget(view=viewid))]).uids[0]
                cad.views.edit(doc,edit=cad.RenameViewsOp(name=captured['name'],to=c['views'][1]['name']))
                save(doc,c['views'][1]);out=cad.sheets.set(doc,details=True)
                frame=out.sheets[0].frames[0];self.assertAlmostEqual(frame.scale,c['expected']['scale'],places=9)
                self.assertEqual(out.named_views[0].name,c['views'][1]['name'])
                self.assertEqual(out.book.to_json(),before,'live rendering cannot rewrite the source capture')
                scoped=cad.changeset.apply(doc,model_scope=cad.SheetModelScope(sheet='page',item='viewport'),steps=[cad.ChangesetStep(command='cad.line.create',input={'layerId':doc.active_layer,'a':{'x':1.,'y':2.},'b':{'x':6.,'y':2.}})])
                self.assertAlmostEqual(doc.measure(scoped.steps[0].output['uid']).length,5.);doc.undo()
                ref=next(m for m in cad.viewmarks.references(doc).marks if m.uid==mark)
                self.assertEqual(ref.words.bottom,c['views'][1]['name']);self.assertFalse(ref.broken)
                sheetmark=cad.viewmarks.create(doc,layer_id=doc.active_layer,marks=[cad.NewViewMark(form='detail',points=[(1.,1.)],radius=.5,mark='7',target=cad.ViewTarget(sheet='page',item='viewport'))]).uids[0]
                cad.sheets.edit(doc,ops=[{'op':'renameSheet','id':'page','name':'A-201 Güncel pafta'},{'op':'renameItem','id':'viewport','name':'Güncel plan penceresi'}])
                refs=cad.viewmarks.references(doc);self.assertEqual(next(m for m in refs.marks if m.uid==sheetmark).words.bottom,'A-201 Güncel pafta')
                self.assertIn('Güncel plan penceresi',next(t for t in refs.targets if t.target.to_json()=={'sheet':'page','item':'viewport'}).name)
                frame=cad.sheets.set(doc).sheets[0].frames[0];self.assertIn(sheetmark,frame.marks)
                reopened=cad.Document.from_bytes(doc.to_bytes());self.assertAlmostEqual(cad.sheets.set(reopened).sheets[0].frames[0].scale,frame.scale)
                cad.sheets.edit(doc,ops=[{'op':'setViewport','id':'viewport','source':{'type':'drawing'}}])
                self.assertAlmostEqual(cad.sheets.set(doc).sheets[0].frames[0].scale,frame.scale)
                cad.sheets.edit(doc,ops=[op]);cad.views.edit(doc,edit=cad.DeleteViewsOp(name=c['views'][1]['name']))
                ref=next(m for m in cad.viewmarks.references(doc).marks if m.uid==mark);self.assertTrue(ref.broken)
                with self.assertRaises(cad.CommandError) as bad_scope:cad.changeset.apply(doc,model_scope=cad.SheetModelScope(sheet='page',item='viewport'),steps=[cad.ChangesetStep(command='cad.line.create',input={})])
                self.assertEqual(bad_scope.exception.code,'missing_named_view')
                with tempfile.TemporaryDirectory() as folder:
                    with self.assertRaises(cad.HostError) as error:doc.export_pdf(Path(folder)/'missing.pdf')
                    self.assertIn('görünüm',str(error.exception).lower());self.assertFalse((Path(folder)/'missing.pdf').exists())
                # A deliberate detach can repair a lost link, using its retained capture.
                cad.sheets.edit(doc,ops=[{'op':'setViewport','id':'viewport','source':{'type':'drawing'}}])

    def test_viewmark_paper_text_has_the_same_physical_size_at_fifty_and_one_hundred(self):
        doc=cad.Document.new('İşaret',workspace='cad',plot_scale=50);page=copy.deepcopy(DATA[0]['sheet']);page['items']=[]
        for k,scale in enumerate([50,100]):
            name=f'Görünüm {scale}';v={'name':name,'center':{'x':0.,'y':0.},'width':scale*.12,'height':scale*.08,'twist':0.};captured=save(doc,v).views[-1].to_json()
            item=copy.deepcopy(DATA[0]['sheet']['items'][0]);item['id']=f'v{k}';item['name']=name;item['padding']=0;item['kind']['labelBand']=0;item['frame']={'left':30000+k*170000,'top':60000,'width':120000,'height':100000};item['kind']['source']={'type':'viewLink','view':captured};page['items'].append(item)
        cad.sheets.edit(doc,ops=[{'op':'addSheet','sheet':page}])
        cad.viewmarks.create(doc,layer_id=doc.active_layer,marks=[cad.NewViewMark(form='elevation',points=[(0.,0.)],mark='42',size=2.5)])
        text=cad.entities.create(doc,layer_id=doc.active_layer,objects=[cad.NewObject.from_json({'geometry':{'kind':'text','p':{'x':0.,'y':-2.},'text':'ANNOT','height':.125,'rotation':0.}})]).created[0]
        cad.annotations.scales(doc,operation='make',uids=[text],scales=[100.])
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'marks.pdf';doc.export_pdf(path)
            text=subprocess.check_output([PDFTOTEXT,'-bbox',str(path),'-'],text=True)
            words=[(float(y1)-float(y0))*25.4/72 for y0,y1 in re.findall(r'<word [^>]*yMin="([^"]+)"[^>]*yMax="([^"]+)"[^>]*>42</word>',text)]
            self.assertEqual(len(words),2,text);self.assertLess(abs(words[0]-words[1]),.02);self.assertGreater(min(words),1.)
            annotations=[(float(y1)-float(y0))*25.4/72 for y0,y1 in re.findall(r'<word [^>]*yMin="([^"]+)"[^>]*yMax="([^"]+)"[^>]*>ANNOT</word>',text)]
            self.assertEqual(len(annotations),2,text);self.assertLess(abs(annotations[0]-annotations[1]),.02);self.assertGreater(min(annotations),1.)
            dxf=Path(folder)/'links.dxf'
            with warnings.catch_warnings():
                warnings.simplefilter('ignore',cad.CommandWarning);cad.file.export.dxf(doc,path=str(dxf))
                reopened=cad.Document.new('Geri oku',workspace='cad');cad.file.import_.dxf(reopened,path=str(dxf))
            self.assertEqual([f.scale for f in cad.sheets.set(reopened).sheets[0].frames],[50.,100.])

    def test_live_camera_outside_physical_scale_budget_cannot_write_an_empty_pdf(self):
        c=DATA[0];doc=cad.Document.new('Kamera sınırı',workspace='cad')
        first=copy.deepcopy(c['operation']['source']['view']);captured=save(doc,first).views[0].to_json()
        op=copy.deepcopy(c['operation']);op['source']['view']=captured
        cad.sheets.edit(doc,ops=[{'op':'addSheet','sheet':c['sheet']},op])
        first['width']=1e-12;first['height']=1e-12;save(doc,first)
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'invalid.pdf'
            with self.assertRaises(cad.HostError) as caught:doc.export_pdf(path)
            self.assertEqual(caught.exception.code,'pdf_unwritable');self.assertIn('ölçek',str(caught.exception).lower());self.assertFalse(path.exists())
