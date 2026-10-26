"""Independent complete saved-model expectations and hashes for real native GUI runs."""
from pathlib import Path
import copy,hashlib,importlib.util,json,struct
B=Path(__file__).resolve().parent;R=B.parents[2]
spec=importlib.util.spec_from_file_location('independent_kcad',R/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(spec);spec.loader.exec_module(reader)
def read(name):return reader.read((B/(name+'.kcad')).read_bytes())[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def size(path):
 data=path.read_bytes();assert data[:2]==b'\xff\xd8';i=2
 while i<len(data):
  assert data[i]==255
  while data[i]==255:i+=1
  marker=data[i];i+=1;length=struct.unpack_from('>H',data,i)[0]
  if marker in (192,193,194):
   height,width=struct.unpack_from('>HH',data,i+3);return [width,height]
  i+=length
 raise AssertionError(path)
before=read('before');assert (B/'before.kcad').read_bytes()==(B.parent/'2026-10-26-architecture-room-choices/final-save.kcad').read_bytes()
assert len(before['entities'])==len(before['uids'])==11
assert [e['kind'] for e in before['entities']]==['space']*5+['tag']*5+['space']
assert before['settings']['plotScale']==50 and before['settings']['drawingUnit']=='m'
for i,e in enumerate(before['entities'][:5]):
 assert e['parts']==[{'outer':{'pts':[dict(x=i*5.,y=0.),dict(x=i*5.+4.,y=0.),dict(x=i*5.+4.,y=3.),dict(x=i*5.,y=3.)]}}]
 assert before['entities'][i+5]['of']==before['uids'][i]
 assert before['entities'][i+5]['at']==dict(x=i*5.+2.,y=1.5)
moved=copy.deepcopy(before);moved['activeLayer']='etiket';moved['entities'][5]['at']=dict(x=-1.,y=2.5)
seed=json.loads((B/'seed.json').read_text())
assert seed['keepUid'] != seed['refreshUid']
assert all(uid not in before['uids'] for uid in seed.values())
def plus(model,key,id_,size_,shape):
 out=copy.deepcopy(model);out['uids'].append(seed[key]);out['entities'].append(dict(kind='tag',id=id_,layerId='etiket',attrs={},of=before['uids'][10],at=dict(x=2.,y=6.5),anchor=dict(x=2.,y=6.5),content='{number}\n{name}',text='G-002\nÖzel oda',size=size_,shape=shape));return out
keep=plus(moved,'keepUid',12,3.,'rectangle')
refreshed=copy.deepcopy(moved)
for i in range(4):
 source=before['entities'][i];refreshed['entities'][i+5].update(content='{number}\n{name}',text=source.get('number','')+'\n'+source['name'],size=3.5,shape='rounded')
refreshed=plus(refreshed,'refreshUid',12,3.5,'rounded')
models={'before':before,'preview':before,'moved':moved,'keep':keep,'keep-undo':moved,'keep-redo':keep,'before-refresh':moved,'refresh-preview':moved,'refreshed':refreshed,'refresh-undo':moved,'refresh-redo':refreshed,
'cancelled':refreshed,'reopened':refreshed,'no-op':refreshed,'final-layout':refreshed,'invalid-draft':refreshed,'compact-cancel':refreshed}
target_moved=copy.deepcopy(moved);target_moved['activeLayer']='cizim'
def target_added(key):
 out=copy.deepcopy(target_moved);out['uids'].append(seed[key]);out['entities'].append(dict(kind='tag',id=12,layerId='cizim',attrs={},of=before['uids'][10],at=dict(x=2.,y=6.5),anchor=dict(x=2.,y=6.5),text='G-002\nÖzel oda\n12.00 m²',shape='rectangle'));return out
models.update({'target-layout-confirmed':target_added('targetMouseUid'),'target-layout-undo':target_added('targetMouseUid'),'target-old-keyboard-write':target_added('targetKeyboardUid')})
models.update({'target-reviewed':target_moved,'target-cancelled':target_moved,'target-undo':target_moved})
if 'targetFinalUid' in seed:
 models.update({name:target_added('targetFinalUid') for name in ['target-confirmed','target-redo','target-reopened']})
records=json.loads((B/'models.json').read_text());groups={}
for record in records:
 p=B/record['file'];name=p.stem;assert sha(p)==record['sha256'],name;assert name in models,name
 assert read(name)==models[name],name
 # Every equivalent state must also have exactly identical serialized bytes.
 fingerprint=json.dumps(models[name],sort_keys=True,ensure_ascii=False)
 if fingerprint in groups:assert p.read_bytes()==groups[fingerprint].read_bytes(),name
 else:groups[fingerprint]=p
images=json.loads((B/'gui-events.json').read_text());dimensions=set()
for record in images:
 p=B/record['file'];assert sha(p)==record['sha256'],p;dimensions.add(tuple(size(p)))
for record in json.loads((B/'test-runs.json').read_text()):assert sha(B/record['log'])==record['sha256'],record['name']
for file in B.glob('*binary.json'):
 record=json.loads(file.read_text());binary=Path(record['app'])/'Contents/MacOS/kentos-cad-bin'
 if binary.exists():assert sha(binary)==record['sha256'],file
# All current production sources must match the final frozen QA build.
final=B/'after-target-binary.json'
if not final.exists():final=B/'after-layout-binary.json'
if final.exists():
 for file,hash_ in json.loads(final.read_text())['sourceFiles'].items():assert sha(R/file)==hash_,file
report=dict(fullNativeModelsVerified=len(records),diagnosticDiskCopiesVerified=1,totalDiskCopiesChecked=len(records)+1,rawImagesVerified=len(images),imageDimensions=[list(d) for d in sorted(dimensions)],sourceGeometryUidsSettingsAndHiddenTag='unchanged',manualTagPosition=dict(x=-1.,y=2.5),keepNewTags=1,refreshNewTags=1,refreshExistingTags=4)
(B/'verification.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
