"""Independent clipping regions, typed automation, physical PDF and external CAD readers."""
import json,math,re,subprocess,tempfile,unittest,warnings
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
DATA=json.loads((ROOT/'fixtures/sheet/v1/viewport-clips.json').read_text())['cases']
VALID=[c for c in DATA if 'error' not in c['expected']]
def make(case):
    doc=cad.Document.new('Kırpma',workspace='cad',drawing_unit='mm')
    cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=case['sheet'])]);return doc

class SheetClips(unittest.TestCase):
    def test_independent_boundaries_keep_camera_and_group_undo(self):
        for case in DATA:
            with self.subTest(case=case['name']):
                doc=make(case);before=cad.sheets.set(doc,details=True).book.to_json()
                if 'error' in case['expected']:
                    with self.assertRaises(cad.CommandError) as error:cad.sheets.edit(doc,ops=[case['operation']])
                    self.assertEqual(error.exception.code,case['expected']['error'])
                    self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before);continue
                with doc.group('Sınır'):cad.sheets.edit(doc,ops=[case['operation']])
                after=cad.sheets.set(doc,details=True).book.to_json();item=after['sheets'][0]['items'][0]
                self.assertEqual(item['kind'].get('clip',{'type':'frame'}),case['expected']['clip'])
                self.assertEqual(item['kind'].get('framePlot',True),case['expected']['framePlot'])
                self.assertEqual(item['kind']['view'],before['sheets'][0]['items'][0]['kind']['view'])
                self.assertEqual(cad.sheets.set(cad.Document.from_bytes(doc.to_bytes()),details=True).book.to_json(),after)
                doc.undo();self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before)
                doc.redo();self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),after)

    def test_typed_boundary_and_legacy_frame_spelling(self):
        clip=cad.PolygonPaperViewportClip(points=[[0,0],[1000000,0],[0,1000000]])
        doc=make(VALID[0]);cad.sheets.edit(doc,ops=[cad.SetViewportPaperOp(id='viewport',clip=clip,frame_plot=False)])
        self.assertEqual(cad.sheets.set(doc,details=True).book.to_json()['sheets'][0]['items'][0]['kind']['clip'],clip.to_json())
        self.assertEqual(cad.PaperViewportClipName.__args__,('frame',))
        cad.sheets.edit(doc,ops=[cad.SetViewportPaperOp(id='viewport',clip=cad.PaperViewportClip.FRAME)])
        self.assertNotIn('clip',cad.sheets.set(doc,details=True).book.to_json()['sheets'][0]['items'][0]['kind'])
        self.assertEqual(cad.PaperViewportClip.from_json('frame').to_json(),{'type':'frame'})

    def test_external_dxf_and_dwg_keep_clip_handles_frame_plot_and_one_paper_item(self):
        import ezdxf
        for case,rotation in [(c,r) for c in VALID if c['name'] in ['Inset rectangle','Triangle','Concave L reverse','Diamond'] for r in [0,30000,90000]]+[(next(c for c in VALID if c['name']=='Explicit frame'),30000)]:
            with self.subTest(case=case['name'],rotation=rotation),tempfile.TemporaryDirectory() as folder:
                doc=make(case);cad.sheets.edit(doc,ops=[case['operation'],{'op':'setItemProps','id':'viewport','patch':{'rotation':rotation}}]);cad.line.create(doc,layer_id=doc.active_layer,a=(0,0),b=(1,1))
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore',cad.CommandWarning)
                    output=Path(folder)/'clip.dxf';cad.file.export.dxf(doc,path=str(output))
                external=ezdxf.readfile(output);v=next(v for v in external.layouts.get('Görünüm').query('VIEWPORT') if v.dxf.id>1)
                self.assertTrue(v.dxf.flags&65536)
                boundary=external.entitydb[v.dxf.clipping_boundary_handle];self.assertEqual(boundary.dxftype(),'LWPOLYLINE');self.assertTrue(boundary.closed)
                self.assertEqual(external.layers.get(v.dxf.layer).dxf.plot,int(case['expected']['framePlot']))
                raw=case['expected']['clip'];points=raw.get('points')
                if raw['type']=='frame':points=[[0,0],[1000000,0],[1000000,1000000],[0,1000000]]
                elif points is None:
                    l,t,r,b=raw['bounds'];points=[[l,t],[r,t],[r,b],[l,b]]
                f=case['sheet']['items'][0]['frame'];page=case['sheet']['page']['size']
                angle=math.radians(rotation/1000);sn,cs=math.sin(angle),math.cos(angle);cx=f['left']+f['width']/2;cy=f['top']+f['height']/2
                turned=[(cx+(x/1e6-.5)*f['width']*cs-(y/1e6-.5)*f['height']*sn,cy+(x/1e6-.5)*f['width']*sn+(y/1e6-.5)*f['height']*cs) for x,y in points]
                expected=[(x/1000,(page['height']-y)/1000) for x,y in turned]
                bw=abs(f['width']*cs)+abs(f['height']*sn);bh=abs(f['width']*sn)+abs(f['height']*cs)
                expected_points=[[math.floor(((x-cx)/bw+.5)*1e6+.5),math.floor(((y-cy)/bh+.5)*1e6+.5)] for x,y in turned]
                for got,want in zip(boundary.get_points('xy'),expected):
                    self.assertAlmostEqual(got[0],want[0],places=8);self.assertAlmostEqual(got[1],want[1],places=8)
                for kind in ['dxf','dwg']:
                    path=output
                    with warnings.catch_warnings():
                        warnings.simplefilter('ignore',cad.CommandWarning)
                        if kind=='dwg':
                            path=Path(folder)/'clip.dwg';cad.file.export.dwg(doc,path=str(path),version=cad.DwgVersion.AC1032)
                        reopened=cad.Document.new('Oku',workspace='cad');getattr(cad.file.import_,kind)(reopened,path=str(path))
                    items=cad.sheets.set(reopened,details=True).book.to_json()['sheets'][0]['items'];self.assertEqual(len(items),1)
                    self.assertEqual(items[0]['kind']['clip']['points'],expected_points)
                    self.assertEqual(items[0]['kind'].get('framePlot',True),case['expected']['framePlot'])

    def test_actual_pdf_pixels_keep_concave_void_and_triangle_outside_empty(self):
        for name in ['Triangle','Concave L']:
            case=next(c for c in VALID if c['name']==name);doc=make(case);cad.sheets.edit(doc,ops=[case['operation']])
            cad.layers.renderer(doc,layer=doc.active_layer,renderer={'type':'single','symbols':{'fill':{'type':'fill','layers':[{'id':'red','type':'simpleFill','color':'#ff0000'}]}}})
            cad.polygon.create(doc,layer_id=doc.active_layer,pts=[(-10,-10),(10,-10),(10,10),(-10,10)])
            with self.subTest(shape=name),tempfile.TemporaryDirectory() as folder:
                pdf=Path(folder)/'clip.pdf';prefix=Path(folder)/'clip';doc.export_pdf(pdf)
                subprocess.run(['pdftoppm','-r','127','-singlefile',str(pdf),str(prefix)],check=True,capture_output=True)
                data=prefix.with_suffix('.ppm').read_bytes();header=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',data);self.assertIsNotNone(header)
                width,height=map(int,header.groups());pixels=memoryview(data)[header.end():];f=case['sheet']['items'][0]['frame']
                for x,y,inside in case['expected']['probes']:
                    if min(x,y,1-x,1-y)<.05:continue
                    if name=='Triangle' and abs(x+y-1)<.02:continue
                    if name=='Concave L' and (abs(x-.4)<.02 or abs(y-.6)<.02):continue
                    px=int((f['left']+x*f['width'])*.005);py=int((f['top']+y*f['height'])*.005)
                    at=(py*width+px)*3;r,g,b=pixels[at:at+3]
                    self.assertEqual(r>220 and g<60 and b<60,inside,f'{name}: {x},{y}, pixel={r,g,b}')
