"""Independent real-GUI CSV/XLSX/ODS and KCAD preservation check; run from repo root."""
from pathlib import Path
import csv,hashlib,importlib.util,json,xml.etree.ElementTree as E,zipfile
from decimal import Decimal as D
ROOT=Path.cwd();B=Path(__file__).resolve().parent

def csv_file(name):
 with (B/name).open(encoding='utf-8-sig',newline='')as f:
  r=csv.DictReader(f,delimiter=';');return r.fieldnames,list(r)
before,wrong=csv_file('before-walls.csv');assert len(wrong)==400 and 'Alan'in before and 'Uzunluk'not in before
h,walls=csv_file('walls.csv');assert len(walls)==200 and len(h)==9 and 'Uzunluk'in h and 'Alan'not in h
assert all(abs(D(r['Uzunluk'])-36)<D('1e-9')and abs(D(r['Kalınlık'])-D('.2'))<D('1e-9')for r in walls)
assert sum(D(r['Uzunluk'])for r in walls)==7200
h,spaces=csv_file('spaces.csv');assert len(spaces)==400 and len(h)==12
assert abs(sum(D(r['Alan'])for r in spaces)-14488)<D('1e-4')
assert (B/'spaces.csv').read_bytes()==(ROOT/'docs/baseline/2026-10-25-architecture-quantity-cache/large-gutter.csv').read_bytes(),'old raw quantity cells changed'
h,empty=csv_file('openings-empty.csv');assert not empty and h==['Numara','Biçim','Tür','Genişlik','Yükseklik','Alan']
N={'x':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with zipfile.ZipFile(B/'quantities.xlsx')as z:
 strings=[''.join(si.itertext())for si in E.fromstring(z.read('xl/sharedStrings.xml')).findall('x:si',N)]
 doc=E.fromstring(z.read('xl/workbook.xml'));assert [s.attrib['name']for s in doc.findall('x:sheets/x:sheet',N)]==['Mahaller','Duvarlar','Açıklıklar']
 for i,n in enumerate([401,201,1],1):
  rows=E.fromstring(z.read(f'xl/worksheets/sheet{i}.xml')).findall('x:sheetData/x:row',N);assert len(rows)==n
  if i in (1,2):
   col='F'if i==1 else'C';values=[D(strings[int(c.find('x:v',N).text)]if c.attrib.get('t')=='s'else c.find('x:v',N).text)for r in rows[1:]for c in r.findall('x:c',N)if c.attrib['r'].startswith(col)];assert len(values)==n-1
   assert abs(sum(values)-(14488 if i==1 else 7200))<D('1e-4')
T='{urn:oasis:names:tc:opendocument:xmlns:table:1.0}';O='{urn:oasis:names:tc:opendocument:xmlns:office:1.0}'
with zipfile.ZipFile(B/'quantities.ods')as z:
 tables=E.fromstring(z.read('content.xml')).findall('.//'+T+'table');assert [t.attrib[T+'name']for t in tables]==['Mahaller','Duvarlar','Açıklıklar']
 for i,(table,n)in enumerate(zip(tables,[401,201,1])):
  rows=table.findall(T+'table-row');assert sum(int(r.attrib.get(T+'number-rows-repeated',1))for r in rows)==n
  if i<2:
   cells=[[c for c in r.findall(T+'table-cell')for _ in range(int(c.attrib.get(T+'number-columns-repeated',1)))][5 if i==0 else 2]for r in rows[1:]]
   values=[D(c.attrib.get(O+'value')or ''.join(c.itertext()))for c in cells]
   assert abs(sum(values)-(14488 if i==0 else 7200))<D('1e-4')
spec=importlib.util.spec_from_file_location('reader',ROOT/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(spec);spec.loader.exec_module(reader)
assert reader.read((B/'model-after.kcad').read_bytes())[1]==reader.read((ROOT/'docs/baseline/2026-10-25-architecture-quantity-cache/workload.kcad').read_bytes())[1]
assert not (ROOT/'.run/architecture-quantity-export/cancelled-export.csv').exists(),'Cancel wrote a file'
events=json.loads((B/'gui-events.json').read_text());assert {e['file']for e in events}=={p.name for p in B.glob('*.jpg')}
for e in events:assert hashlib.sha256((B/e['file']).read_bytes()).hexdigest()==e['sha256']
x=json.loads((B/'binary.json').read_text())
for f,h in x['sourceFiles'].items():assert hashlib.sha256((ROOT/f).read_bytes()).hexdigest()==h,f
assert hashlib.sha256((Path(x['app'])/x['executable']).read_bytes()).hexdigest()==x['sha256']
print('PASS actual wall CSV200/7200m, room CSV400/14488m² unchanged, empty header, XLSX/ODS three sheets and totals, exact model,',len(events),'raw GUI screenshots')
