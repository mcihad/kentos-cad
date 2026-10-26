"""Independent full saved model expectations, including unrelated and hidden sources."""
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
before=read('before');assert (B/'before.kcad').read_bytes()==(B.parent/'2026-10-26-architecture-tag-refresh/before.kcad').read_bytes()
visible=copy.deepcopy(before)
next(l for l in visible['layers'] if l['id']=='gizli')['visible']=True
assert read('before-visible')==visible
base=copy.deepcopy(visible);base['activeLayer']='etiket'
seed=json.loads((B/'seed.json').read_text()) if (B/'seed.json').exists() else {}
assert len(set(seed.values()))==len(seed)
assert not set(seed.values()).intersection(before['uids'])
def added(model,key,content,text,size=None,shape='rectangle'):
 out=copy.deepcopy(model);out['uids'].append(seed[key]);t=dict(kind='tag',id=12,layerId='etiket',attrs={},of=before['uids'][10],at=dict(x=2.,y=6.5),anchor=dict(x=2.,y=6.5),content=content,text=text,shape=shape)
 if size is not None:t['size']=size
 out['entities'].append(t);return out
models={'before':before,'before-visible':visible,'target':base,'preview-all':base,'preview-ground':base,'preview-unassigned':base,'preview-upper':base,'rejected-upper':base,'cancelled':base,'keep-undo':base,'refresh-preview':base,'refresh-undo':base}
if 'keepUid' in seed:
 keep=added(base,'keepUid','{number}','G-002');models.update({'keep':keep,'keep-redo':keep})
if 'refreshUid' in seed:
 ground=copy.deepcopy(base)
 for i in [0,1,3]:ground['entities'][i+5].update(content='{name}',text=before['entities'][i]['name'],size=3.5,shape='rounded')
 ground=added(ground,'refreshUid','{name}','Özel oda',3.5,'rounded')
 models.update({name:ground for name in ['refresh','refresh-redo','unassigned-preview','unassigned-undo','empty-upper','empty-rejected','final-cancel']})
 unassigned=copy.deepcopy(ground);unassigned['entities'][7].update(content='{name}',text='Mutfak')
 upper=copy.deepcopy(ground);upper['entities'][9].update(content='{name}',text='Depo')
 models.update({'upper-preview':ground,'upper':upper,'upper-undo':ground,'upper-redo':upper})
 models.update({name:unassigned for name in ['unassigned','unassigned-redo','reopened','final-layout','final-cancel']})
 final_upper=copy.deepcopy(unassigned);final_upper['entities'][9].update(content='{name}',text='Depo')
 models.update({'final-upper':final_upper,'final-upper-redo':final_upper,'final-upper-undo':unassigned})
records=json.loads((B/'models.json').read_text()) if (B/'models.json').exists() else [];groups={}
for rec in records:
 p=B/rec['file'];name=p.stem;assert sha(p)==rec['sha256'],name;assert read(name)==models[name],name
 signature=json.dumps(models[name],sort_keys=True,ensure_ascii=False)
 if signature in groups:assert p.read_bytes()==groups[signature].read_bytes(),name
 else:groups[signature]=p
images=json.loads((B/'gui-events.json').read_text());dims=set()
for rec in images:
 p=B/rec['file'];assert sha(p)==rec['sha256'];dims.add(tuple(dimensions(p)))
for rec in json.loads((B/'test-runs.json').read_text()):assert sha(B/rec['log'])==rec['sha256'],rec['name']
previous_files=json.loads((B/'previous-kcad.json').read_text());assert len(previous_files)==1781
assert all(sha(R/p)==h for p,h in previous_files.items())
for name in ['before-binary.json','after-binary.json','after-label-binary.json']:
 p=B/name
 if p.exists():
  meta=json.loads(p.read_text());binary=Path(meta['app'])/'Contents/MacOS/kentos-cad-bin'
  if binary.exists():assert sha(binary)==meta['sha256']
  for f,h in meta.get('sourceFiles',{}).items():
   if name=='after-binary.json' and (B/'after-label-binary.json').exists() and f=='crates/native/interaction/src/tag_tools.rs':
    code=(R/f).read_text();line='        source.label = "Kaynak katı";\n';assert code.count(line)==1
    old=code.replace(line,line+'        source.value = self.source.label();\n')
    assert hashlib.sha256(old.encode()).hexdigest()==h
   else:assert sha(R/f)==h,f
report=dict(fullNativeModelsVerified=len(records),rawImagesVerified=len(images),imageDimensions=[list(x) for x in sorted(dims)],previousKcadBytesUnchanged=len(previous_files),wholeArchitectureAccepted=False)
(B/'verification.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
