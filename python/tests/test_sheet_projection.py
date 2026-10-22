"""Physical PDF dimensions with Poppler as an independent reader, plus external-unit DXFs."""
import copy,json,math,re,shutil,subprocess,tempfile,unittest,xml.etree.ElementTree as ET
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
DATA=json.loads((ROOT/'fixtures/sheet/v1/viewport-projection.json').read_text())
BASE=json.loads((ROOT/'fixtures/sheet/v1/viewports.json').read_text())['cases'][0]['sheet']
NUM=r'[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?'
IDENTITY=(1.,0.,0.,1.,0.,0.)
def multiply(a,b):
    A,B,C,D,E,F=a;a,b,c,d,e,f=b
    return(A*a+C*b,B*a+D*b,A*c+C*d,B*c+D*d,A*e+C*f+E,B*e+D*f+F)
def matrix(text):
    result=IDENTITY
    for name,args in re.findall(r'(\w+)\(([^)]*)\)',text or ''):
        values=list(map(float,re.findall(NUM,args)))
        if name=='matrix':part=tuple(values)
        elif name=='translate':part=(1,0,0,1,values[0],values[1] if len(values)>1 else 0)
        elif name=='scale':part=(values[0],0,0,values[-1],0,0)
        else:raise AssertionError(f'Unmeasured SVG transform: {name}')
        result=multiply(result,part)
    return result

def red_lines_mm(svg,page_mm=(420,297)):
    root=ET.parse(svg).getroot();box=list(map(float,root.attrib['viewBox'].split()));found=[]
    def walk(element,parent=IDENTITY,style=None):
        current=multiply(parent,matrix(element.get('transform')));styles=dict(style or {})
        styles.update({k:v for k,v in re.findall(r'([\w-]+)\s*:\s*([^;]+)',element.get('style',''))})
        styles.update({k:v for k,v in element.attrib.items() if k in ['fill','stroke']})
        stroke=re.sub(r'\s+','',styles.get('stroke','')).lower()
        if element.tag.endswith('}path') and stroke in ['#ff0000','rgb(100%,0%,0%)','rgb(255,0,0)']:
            tokens=re.findall(r'[MLmlZz]|'+NUM,element.attrib['d']);points=[];index=0;command=None
            while index<len(tokens):
                if tokens[index] in ['M','L','m','l','Z','z']:command=tokens[index];index+=1
                if command in ['Z','z']:break
                if command not in ['M','L']:raise AssertionError('The acceptance line must remain a straight absolute path')
                x,y=map(float,tokens[index:index+2]);index+=2;a,b,c,d,e,f=current
                points.append(((a*x+c*y+e-box[0])*page_mm[0]/box[2],(b*x+d*y+f-box[1])*page_mm[1]/box[3]))
            found.append(points)
        for child in element:walk(child,current,styles)
    walk(root);return found

def sheet_of(case):
    sheet=copy.deepcopy(BASE);item=sheet['items'][0];item['frame']=case['frame'];item['padding']=0;item['kind']['labelBand']=0
    item['kind']['view'].update(center=dict(x=case['center'][0],y=case['center'][1]),ratio=dict(paper=1,model=case['denominator']),rotation=case['rotation'])
    return sheet

class SheetProjection(unittest.TestCase):
    def test_independent_pdf_reader_preserves_binary_streams_ending_in_newlines(self):
        import zlib
        from acceptance_files import _objects
        for content in [b'\t', b'\x0c']:
            packed=zlib.compress(content)
            self.assertIn(packed[-1], [10,13])
            pdf=b'1 0 obj\n<< /Length '+str(len(packed)).encode()+b' /Filter /FlateDecode >>\nstream\n'+packed+b'\nendstream\nendobj'
            self.assertEqual(_objects(pdf)[1][1], content)

    def test_twenty_real_pdfs_keep_physical_length_position_units_and_model(self):
        self.assertIsNotNone(shutil.which('pdftocairo'),'Poppler is required for physical PDF acceptance')
        # Every unit/denominator pair; alternate 0 and 30 degrees instead of repeating equal files.
        cases=[c for n,c in enumerate(DATA['cases']) if n%2==((n//8)%2)]
        self.assertEqual(len(cases),20)
        with tempfile.TemporaryDirectory() as folder:
            for index,case in enumerate(cases):
                with self.subTest(case=case['name']):
                    doc=cad.Document.new('Fiziksel ölçek',workspace='cad',drawing_unit=case['unit'])
                    line=cad.line.create(doc,layer_id=doc.active_layer,a=case['world'][0],b=case['world'][1],color='#ff0000',line_weight=.25)
                    before=doc.entity(line.uid).entity.to_json()
                    with doc.group('Kâğıt'):
                        cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=sheet_of(case))])
                    pdf=Path(folder)/f'{index}.pdf';svg=pdf.with_suffix('.svg');doc.export_pdf(pdf)
                    # Cairo's default print fit can shrink fractional-point pages: disable it explicitly.
                    subprocess.run(['pdftocairo','-svg','-origpagesizes','-noshrink','-nocenter',str(pdf),str(svg)],check=True,capture_output=True)
                    lines=red_lines_mm(svg);self.assertEqual(len(lines),1);self.assertEqual(len(lines[0]),2)
                    a,b=lines[0];self.assertAlmostEqual(math.dist(a,b),case['expected']['lengthMm'],delta=.005)
                    for got,want in zip([a,b],case['expected']['paper'][:2]):
                        for j in range(2):self.assertAlmostEqual(got[j],want[j]/1000,delta=.005)
                    self.assertEqual(doc.entity(line.uid).entity.to_json(),before)
                    saved=cad.Document.from_bytes(doc.to_bytes());self.assertEqual(saved.entity(line.uid).entity.to_json(),before)
                    doc.undo();self.assertEqual(doc.sheets(),[]);self.assertEqual(doc.entity(line.uid).entity.to_json(),before)

    def test_external_dxf_file_units_are_normalized_once_before_the_viewport_scale(self):
        import ezdxf
        codes={'m':6,'mm':4,'cm':5,'dm':14,'in':1,'ft':2}
        for unit,code in codes.items():
            factor={'m':1.,'mm':1000.,'cm':100.,'dm':10.,'in':1000/25.4,'ft':1/.3048}[unit]
            with self.subTest(unit=unit),tempfile.TemporaryDirectory() as folder:
                source=ezdxf.new('R2010');source.units=code;source.modelspace().add_line((0,0),(5*factor,0))
                layout=source.layouts.new('Units');layout.page_setup(size=(420,297),margins=(0,0,0,0),units='mm')
                layout.add_viewport(center=(210,150),size=(200,100),view_center_point=(0,0),view_height=3.75*factor,dxfattribs={'flags':32768})
                path=Path(folder)/'units.dxf';source.saveas(path)
                doc=cad.Document.new('Oku',workspace='cad',drawing_unit='ft' if unit!='ft' else 'mm')
                cad.file.import_.dxf(doc,path=str(path))
                model=[r.entity.to_json() for r in doc.entities() if r.entity.to_json()['kind']=='line'];self.assertEqual(len(model),1)
                self.assertAlmostEqual(math.dist([model[0]['a']['x'],model[0]['a']['y']],[model[0]['b']['x'],model[0]['b']['y']]),5,places=9)
                page=next(s for s in cad.sheets.set(doc).sheets if s.name=='Units')
                self.assertAlmostEqual(page.frames[0].scale,37.5,places=9)
                bounds=page.frames[0].bounds;self.assertAlmostEqual(bounds[2]-bounds[0],7.5,places=9);self.assertAlmostEqual(bounds[3]-bounds[1],3.75,places=9)

    def test_a_long_model_line_is_visibly_clipped_to_the_two_hundred_mm_frame(self):
        case=DATA['cases'][0];doc=cad.Document.new('Kırp',workspace='cad',drawing_unit='mm')
        cad.line.create(doc,layer_id=doc.active_layer,a=(-10,0),b=(10,0),color='#ff0000',line_weight=.25)
        cad.sheets.edit(doc,ops=[dict(op='addSheet',sheet=sheet_of(case))])
        with tempfile.TemporaryDirectory() as folder:
            pdf=Path(folder)/'clip.pdf';prefix=Path(folder)/'clip';doc.export_pdf(pdf)
            subprocess.run(['pdftoppm','-r','127','-singlefile','-aa','no','-aaVector','no',str(pdf),str(prefix)],check=True,capture_output=True)
            data=prefix.with_suffix('.ppm').read_bytes();header=re.match(rb'P6\s+(\d+)\s+(\d+)\s+255\s',data);self.assertIsNotNone(header)
            width,height=map(int,header.groups());pixels=memoryview(data)[header.end():];xs=[]
            for y in range(height):
                row=pixels[y*width*3:(y+1)*width*3]
                for x in range(width):
                    if row[x*3]>180 and row[x*3+1]<90 and row[x*3+2]<90:xs.append(x)
            self.assertTrue(xs);self.assertAlmostEqual(min(xs),550,delta=2);self.assertAlmostEqual(max(xs),1549,delta=2)
