"""Independent whole saved model expectations, including all unrelated sources."""
from pathlib import Path
import copy, hashlib, importlib.util, json, struct
B=Path(__file__).resolve().parent;R=B.parents[2]
s=importlib.util.spec_from_file_location('independent_kcad',R/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(s);s.loader.exec_module(reader)
def read(name):return reader.read((B/(name+'.kcad')).read_bytes())[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dimensions(p):
 d=p.read_bytes();assert d[:2]==b'\xff\xd8';i=2
 while i<len(d):
  assert d[i]==255
  while d[i]==255:i+=1
  marker=d[i];i+=1;n=struct.unpack_from('>H',d,i)[0]
  if marker in (192,193,194):
   h,w=struct.unpack_from('>HH',d,i+3);return [w,h]
  i+=n
 raise AssertionError(p)
def equal(a,b,path=''):
 if isinstance(a,float) and isinstance(b,(float,int)):
  assert abs(a-b)<=1e-9,(path,a,b)
 elif isinstance(a,dict):
  assert isinstance(b,dict) and a.keys()==b.keys(),(path,a,b)
  for k in a:equal(a[k],b[k],path+'/'+k)
 elif isinstance(a,list):
  assert isinstance(b,list) and len(a)==len(b),(path,a,b)
  for i,(x,y) in enumerate(zip(a,b)):equal(x,y,path+'/'+str(i))
 else:assert a==b,(path,a,b)
before=read('before');source=reader.read((B.parent/'2026-10-26-architecture-tag-net-area/before.kcad').read_bytes())[1]
source['entities'][3]['height']=2.75;assert before==source
content='{number}\n{name}\nNet: {netArea}\nDöşeme: {floorElevation}\nTavan: {ceilingElevation}'
models={n:before for n in ['before','preview','undo','cancelled']}
seed=json.loads((B/'seed.json').read_text()) if (B/'seed.json').exists() else {}
if 'tagUid' in seed:
 assert seed['tagUid'] not in before['uids']
 tagged=copy.deepcopy(before);tagged['uids'].append(seed['tagUid']);tagged['entities'].append(dict(kind='tag',id=5,layerId='etiket',attrs={},of=before['uids'][3],at=dict(x=9.,y=2.),anchor=dict(x=3.,y=2.),content=content,text='N-01\nÇalışma\nNet: 21.79 m²\nDöşeme: 0.000\nTavan: 2.750',size=3.5,shape='rounded',leader=True))
 models.update({n:tagged for n in ['tagged','redo','height-undo','final-cancel','reopened']})
 height=copy.deepcopy(tagged);height['entities'][3]['height']=3.125;height['entities'][4]['text']='N-01\nÇalışma\nNet: 21.79 m²\nDöşeme: 0.000\nTavan: 3.125'
 models.update({n:height for n in ['height','height-redo']})
if 'replayTagUid' in seed:
 assert seed['replayTagUid'] not in before['uids']
 replay=copy.deepcopy(tagged);replay['uids'][-1]=seed['replayTagUid']
 models.update({n:before for n in ['undo-final']})
 models.update({n:replay for n in ['created-final','redo-final','height-undo-final','cancel-final','reopened-final','bulk-undo-final','bulk-keep-final','bulk-preview-final']})
 replay_height=copy.deepcopy(replay);replay_height['entities'][3]['height']=3.125;replay_height['entities'][4]['text']='N-01\nÇalışma\nNet: 21.79 m²\nDöşeme: 0.000\nTavan: 3.125'
 models.update({n:replay_height for n in ['height-final','height-redo-final']})
 bulk=copy.deepcopy(replay);bulk['entities'][4]['content']='{name}';bulk['entities'][4]['text']='Çalışma'
 models.update({n:bulk for n in ['bulk-name-final']})
records=json.loads((B/'models.json').read_text()) if (B/'models.json').exists() else [];groups={}
for rec in records:
 p=B/rec['file'];name=p.stem;assert sha(p)==rec['sha256'],name;equal(models[name],read(name),name)
 sig=json.dumps(models[name],sort_keys=True,ensure_ascii=False)
 if sig in groups:assert p.read_bytes()==groups[sig].read_bytes(),name
 else:groups[sig]=p
diagnostics=json.loads((B/'diagnostic-models.json').read_text())+[json.loads((B/'diagnostic-external.json').read_text())]
for rec in diagnostics:
 assert rec['accepted'] is False
 assert sha(B/rec['file'])==rec['sha256']
 assert rec['file'] not in {x['file'] for x in records}
for name,h in json.loads((B/'source-files.json').read_text()).items():assert sha(R/name)==h,name
images=json.loads((B/'gui-events.json').read_text());dims=set()
for rec in images:
 p=B/rec['file'];assert sha(p)==rec['sha256'];dims.add(tuple(dimensions(p)))
for rec in json.loads((B/'test-runs.json').read_text()):assert sha(B/rec['log'])==rec['sha256'],rec['name']
previous=json.loads((B/'previous-kcad.json').read_text());assert len(previous)==1810
assert all(sha(R/p)==h for p,h in previous.items())
for name in ['before-binary.json','after-binary.json','after-isolated-binary.json']:
 p=B/name
 if p.exists():
  meta=json.loads(p.read_text());binary=Path(meta['app'])/'Contents/MacOS/kentos-cad-bin'
  if binary.exists():assert sha(binary)==meta['sha256']
  if name.startswith('after-'):
   for f,h in meta['sourceFiles'].items():assert sha(R/f)==h,f
report=dict(fullNativeModelsVerified=len(records),diagnosticModelsPreserved=len(diagnostics),rawImagesVerified=len(images),acceptedImages=sum(x["accepted"] for x in images),diagnosticImages=sum(not x["accepted"] for x in images),imageDimensions=[list(x) for x in sorted(dims)],previousKcadBytesUnchanged=len(previous),wholeArchitectureAccepted=False)
(B/'verification.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
