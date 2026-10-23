"""Actual comparison sheets: independent Poppler pixels, a linked PNG and an attached PDF."""
import copy,json,math,os,struct,subprocess,tempfile,unittest,zlib
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
FIXTURE=json.loads((ROOT/'fixtures/sheet/v1/plot-comparison.json').read_text())
def png():
 def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
 return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',32,32,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+bytes([20,160,200])*32)*32))+chunk(b'IEND',b'')
def source_pdf():
 # A vector-only source with a known red square. Real PDF attach, not prebuilt KentOS paths.
 stream=b"0.8 0.2 0.2 rg 100 100 200 200 re f 1 1 1 rg 150 150 100 100 re f 0 0 0 RG 40 40 520 320 re S"
 objects=[b"<< /Type /Catalog /Pages 2 0 R >>",b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 400] /Contents 4 0 R >>",b"<< /Length "+str(len(stream)).encode()+b" >>\nstream\n"+stream+b"\nendstream"]
 data=b"%PDF-1.4\n";offsets=[0]
 for n,obj in enumerate(objects,1):offsets.append(len(data));data+=str(n).encode()+b" 0 obj\n"+obj+b"\nendobj\n"
 xref=len(data);data+=b"xref\n0 5\n0000000000 65535 f \n"+b"".join(f"{o:010} 00000 n \n".encode() for o in offsets[1:])+b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n"+str(xref).encode()+b"\n%%EOF\n"
 return data
def ppm(data):
 # Read binary PPM including optional header comments; keep payload whitespace intact.
 pos=0;tokens=[]
 while len(tokens)<4:
  while data[pos] in b' \t\r\n':pos+=1
  if data[pos]==35:pos=data.index(b'\n',pos)+1;continue
  end=pos
  while data[end] not in b' \t\r\n':end+=1
  tokens.append(data[pos:end]);pos=end
 assert tokens[0]==b'P6' and tokens[3]==b'255'
 return int(tokens[1]),int(tokens[2]),data[pos+1:]
def run_comparison(folder,with_raster=True,underlay_look=None):
 folder=Path(folder);folder.mkdir(parents=True,exist_ok=True);image=folder/'underlay.png';image.write_bytes(png());underlay=folder/'source.pdf';underlay.write_bytes(source_pdf())
 model=copy.deepcopy(FIXTURE['model']);model['entities'][-1]['file']=str(image)
 if not with_raster:model['entities'].pop()
 doc=cad.Document.from_bytes(json.dumps(model).encode())
 cad.xrefs.edit(doc,operation=cad.AttachXrefsOperation(path=str(underlay),name='PDF kontrolü',layer_id='cizim',at=cad.Vec2(x=14.5,y=0),scale=10))
 if underlay_look:cad.xrefs.edit(doc,operation=cad.UnderlayXrefsOperation(name='PDF kontrolü',**underlay_look))
 book=FIXTURE['book'];cad.sheets.edit(doc,ops=[dict(op='savePlotProfiles',profiles=book['plotProfiles'])]+[dict(op='addSheet',sheet=s) for s in book['sheets']])
 source=doc.to_bytes();pdf=folder/'comparison.pdf';doc.export_pdf(pdf)
 assert doc.to_bytes()==source
 subprocess.run(['pdftoppm','-r','127',str(pdf),str(folder/'comparison')],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 return pdf,sorted(folder.glob('comparison-*.ppm'))
class PlotComparison(unittest.TestCase):
 def test_the_pdf_underlay_plot_switch_really_removes_its_ink(self):
  with tempfile.TemporaryDirectory() as folder:
   _,pages=run_comparison(folder,False,dict(plot=False))
   for path in pages:
    w,_,pixels=ppm(path.read_bytes())
    for y in range(920,1035):
     for x in range(1705,1900):
      at=(y*w+x)*3;self.assertEqual(pixels[at:at+3],bytes([255]*3))

 def test_colour_mono_gray_keep_underlays_masks_and_order(self):
  for with_raster in [True,False]:
   with tempfile.TemporaryDirectory() as folder:self.check_pages(folder,with_raster)
 def check_pages(self,folder,with_raster):
   pdf,pages=run_comparison(folder,with_raster);self.assertEqual(len(pages),3)
   for n,path in enumerate(pages):
    w,h,pixels=ppm(path.read_bytes());self.assertEqual(w,2100);self.assertLessEqual(abs(h-1485),1) # PDF point quantization can ceil the last raster row.
    for check in FIXTURE['checks']:
     if not with_raster and check['name']=='raster keeps pixels':continue
     with self.subTest(page=n,check=check['name']):
      expected=check['rgb']
      if check['name']!='raster keeps pixels' and expected!=[255]*3:
       if n==1:expected=[0]*3
       elif n==2:expected=[math.floor(sum(c*k for c,k in zip(expected,[.2126,.7152,.0722]))+.5)]*3
      x,y=[int(v*5) for v in check['at']];got=list(pixels[(y*w+x)*3:(y*w+x)*3+3]);self.assertTrue(all(abs(a-b)<=3 for a,b in zip(got,expected)),(got,expected))
    # Whole interiors prove hatch order and wipeout coverage, rather than a point between hatch lines.
    for rect,source_rgb in [([212,145,228,175],[51,153,102]),([142,168,158,172],[255,255,255])]:
     want=source_rgb
     if want!=[255]*3:
      if n==1:want=[0]*3
      elif n==2:want=[math.floor(sum(c*k for c,k in zip(want,[.2126,.7152,.0722]))+.5)]*3
     for y in range(rect[1]*5,rect[3]*5):
      for x in range(rect[0]*5,rect[2]*5):
       at=(y*w+x)*3;self.assertTrue(all(abs(a-b)<=3 for a,b in zip(pixels[at:at+3],want)),(n,x,y,list(pixels[at:at+3]),want))
    # The independently imported source PDF is visible in its own panel, not an empty raster.
    ink=sum(1 for y in range(910,1045) for x in range(1705,1870) if min(pixels[(y*w+x)*3:(y*w+x)*3+3])<220)
    self.assertGreater(ink,20)
   words=subprocess.check_output(['pdftotext',str(pdf),'-'],text=True);self.assertIn('Raster',words)
   if not with_raster:self.assertIn('MASK',words)
if __name__=='__main__':
 if len(os.sys.argv)>1 and os.sys.argv[1]=='--artifact':run_comparison(ROOT/'.run/shots/sheet21')
 else:unittest.main()
