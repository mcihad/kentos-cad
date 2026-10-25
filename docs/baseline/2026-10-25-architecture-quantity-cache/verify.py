"""Independent preservation/CSV check. Run from repository root after actual GUI saves."""
import copy,csv,hashlib,importlib.util,json
from decimal import Decimal as D
from pathlib import Path
ROOT=Path.cwd();BASE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('reader',ROOT/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(spec);spec.loader.exec_module(reader)
for f,h in json.loads((BASE/'frozen-before-sha256.json').read_text()).items():assert hashlib.sha256((BASE/f).read_bytes()).hexdigest()==h,f
assert json.loads((BASE/'answers-before.json').read_text())==json.loads((BASE/'answers-after.json').read_text()),'full query outputs'
def load(path):return reader.read(path.read_bytes())[1]
small=load(ROOT/'docs/baseline/2026-10-25-architecture-space-separators/separator-created.kcad')
assert load(BASE/'gui-small-restored.kcad')==small,'small real Undo changed the model'
assert load(BASE/'gui-large-after.kcad')==load(BASE/'workload.kcad'),'large real report changed the model'
assert load(BASE/'gui-gutter-after.kcad')==load(BASE/'workload.kcad'),'final UI build changed the model'
assert (BASE/'large-gutter.csv').read_bytes()==(BASE/'large-gui.csv').read_bytes(),'final UI CSV changed'
def rows(d):return dict(zip(d['uids'],d['entities']))
a=rows(small);moved=load(BASE/'gui-small-moved.kcad');b=rows(moved)
assert a.keys()==b.keys()
for uid,e in a.items():
 if e['kind']=='polyline':
  expected=copy.deepcopy(e)
  for p in expected['pts']:p['x']+=1
  assert b[uid]==expected,'separator move'
 elif e['kind']=='space':
  assert {k:v for k,v in e.items()if k!='parts'}=={k:v for k,v in b[uid].items()if k!='parts'},'room metadata'
  def area(r):
   assert not r.get('bulges');p=[(D(str(v['x'])),D(str(v['y'])))for v in r['pts']]
   return abs(sum(x[0]*y[1]-x[1]*y[0]for x,y in zip(p,p[1:]+p[:1])))/2
  measured=sum(area(p['outer'])-sum(area(h)for h in p.get('holes',[]))for p in b[uid]['parts'])
  assert abs(measured-D('42.02'))<D('1e-9'),measured
 else:assert b[uid]==e,'untouched wall/column'
assert {k:v for k,v in small.items()if k!='entities'}=={k:v for k,v in moved.items()if k!='entities'},'project root/settings'
with (BASE/'large-gui.csv').open(encoding='utf-8-sig',newline='')as f:csv_rows=list(csv.DictReader(f,delimiter=';'))
assert len(csv_rows)==400 and all(len(r)==12 and 'Durum'not in r for r in csv_rows),'CSV shape'
counts=[0,0]
for r in csv_rows:
 matched=False
 for i,values in enumerate([('34.22','33.4','34.22','36','37.82'),('38.22','25.4','38.22','40','41.82')]):
  if all(abs(D(r[k])-D(v))<D('1e-6')for k,v in zip(['Alan','Çevre','Net','Eksen','Brüt'],values)):
   counts[i]+=1;matched=True;break
 assert matched,r
assert counts==[200,200],counts
assert abs(sum(D(r['Alan'])for r in csv_rows)-D('14488'))<D('1e-4'),'independent total'
events=json.loads((BASE/'gui-events.json').read_text());assert {r['file']for r in events}=={p.name for p in BASE.glob('*.jpg')}
for r in events:assert hashlib.sha256((BASE/r['file']).read_bytes()).hexdigest()==r['sha256'],r['file']
print('PASS 100 old/new query outputs; frozen input hashes; actual move 42.02 / exact undo; full unchanged large model; 400 real GUI CSV rows / 14488 m²;',len(events),'raw screenshots')
