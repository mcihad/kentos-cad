"""Whole saved models against explicit geometric expectations; no KentOS runtime."""
from pathlib import Path
import copy,hashlib,importlib.util,json,struct,subprocess
B=Path(__file__).resolve().parent;R=B.parents[2]
s=importlib.util.spec_from_file_location('independent_kcad',R/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(s);s.loader.exec_module(reader)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(n):return reader.read((B/(n+'.kcad')).read_bytes())[1]
def equal(a,b,path=''):
 if isinstance(a,float) and isinstance(b,(float,int)):
  assert abs(a-b)<=1e-9,(path,a,b)
 elif isinstance(a,dict):
  assert isinstance(b,dict) and a.keys()==b.keys(),(path,a,b)
  for k in a:equal(a[k],b[k],path+'/'+k)
 elif isinstance(a,list):
  assert isinstance(b,list)and len(a)==len(b),(path,a,b)
  for i,(x,y)in enumerate(zip(a,b)):equal(x,y,path+'/'+str(i))
 else:assert a==b,(path,a,b)
before=read('before');seed=json.loads((B/'seed.json').read_text());assert before['uids'][:2]==[seed['hostUid'],seed['openingUid']]
assert sha(B/'before.kcad')==json.loads((B/'fixture-preparation.json').read_text())['sha256']
assert len(before['entities'])==3
assert before['settings']==dict(srid=0,areaUnit='m2',angleUnit='deg',plotScale=50.,workspace='cad',drawingFont='barlow',areaDecimals=2,lengthDecimals=3)
assert before['entities'][0]==dict(kind='wall',id=1,pts=[dict(x=0.,y=0.),dict(x=10.,y=0.)],attrs={},width=.2,justify='center',layerId='cizim')
assert before['entities'][1]==dict(kind='opening',id=2,at=4.,host=seed['hostUid'],attrs={},place=[dict(x=3.5,y=.1),dict(x=3.5,y=-.1),dict(x=4.5,y=.1),dict(x=4.5,y=-.1)],width=1.,layerId='cizim',form='window',head=2.2,sill=.9,frame=.06,style='kentos-pencere-120',leaves=2)
assert before['entities'][2]==dict(kind='line',id=3,a=dict(x=12.,y=3.),b=dict(x=12.5,y=3.),attrs={'qa':'unrelated-preserve'},layerId='cizim')
# The old GUI's free-point measure is evidence of the defect, never an accepted
# new association. Its clicked offset is bounded by the observed pixel position.
baseline=read('baseline-created');old=copy.deepcopy(before);old['uids'].append(baseline['uids'][-1]);dim=dict(kind='dimension',id=4,a=dict(x=3.5,y=.1),b=dict(x=4.5,y=.1),attrs={},height=.125,offset=baseline['entities'][-1]['offset'],layerId='cizim');assert .3<dim['offset']<.32;old['entities'].append(dim);equal(old,baseline,'baseline')
models={'baseline-created':old,'preview':before,'undo':before,'before-cancel':before}
if 'dimensionUid'in seed:
 assert seed['dimensionUid']not in before['uids']
 created=copy.deepcopy(before);created['uids'].append(seed['dimensionUid']);dim=copy.deepcopy(dim);dim['offset']=1.;dim['refs']={k:dict(uid=seed['openingUid'],place=dict(kind='vertex',part=0,ring=0,index=i))for k,i in [('a',0),('b',2)]};created['entities'].append(dim)
 models.update({n:created for n in ['created','redo','width-undo','final','cancelled','reopened','compact-final']})
 wide=copy.deepcopy(created);wide['entities'][1]['width']=2.;wide['entities'][1]['place']=[dict(x=3.,y=.1),dict(x=3.,y=-.1),dict(x=5.,y=.1),dict(x=5.,y=-.1)];wide['entities'][3]['a']=dict(x=3.,y=.1);wide['entities'][3]['b']=dict(x=5.,y=.1)
 models.update({n:wide for n in ['wide','width-redo','move-undo']})
 moved=copy.deepcopy(wide);moved['entities'][1]['at']=6.;moved['entities'][1]['place']=[dict(x=5.,y=.1),dict(x=5.,y=-.1),dict(x=7.,y=.1),dict(x=7.,y=-.1)];moved['entities'][3]['a']=dict(x=5.,y=.1);moved['entities'][3]['b']=dict(x=7.,y=.1)
 models.update({n:moved for n in ['moved','move-redo','thickness-undo']})
 thick=copy.deepcopy(moved);thick['entities'][0]['width']=.4
 for p in thick['entities'][1]['place']:p['y']=.2 if p['y']>0 else -.2
 thick['entities'][3]['a']['y']=.2;thick['entities'][3]['b']['y']=.2
 models.update({n:thick for n in ['thick','thickness-redo','delete-undo']})
 broken=copy.deepcopy(thick);broken['entities'].pop(0);broken['uids'].pop(0);broken['entities'][0]['broken']='hostGone'
 # KCAD column format omits ephemeral runtime slots (columns.rs:1990); reader
 # assigns 1..n in file order. Assert that exact order, never ignore ids.
 for i,e in enumerate(broken['entities'],1):e['id']=i
 for ref in broken['entities'][-1]['refs'].values():ref['broken']='no_such_place'
 models.update({n:broken for n in ['broken','delete-redo']})
records=json.loads((B/'models.json').read_text());groups={}
for rec in records:
 p=B/rec['file'];name=p.stem;assert sha(p)==rec['sha256'],name;equal(models[name],read(name),name)
 sig=json.dumps(models[name],sort_keys=True)
 if sig in groups:assert p.read_bytes()==groups[sig].read_bytes(),name
 else:groups[sig]=p
diagnostics=json.loads((B/'diagnostic-models.json').read_text())
assert {p.name for p in B.glob('*.kcad')}=={r['file']for r in records+diagnostics}|{'before.kcad'}
for rec in diagnostics:
 assert rec['accepted'] is False
 assert sha(B/rec['file'])==rec['sha256'],rec['file']
# The interrupted attempt is retained, never accepted as a controlled action.
# Check all four original entities by UUID, plus the exact count of extras.
interrupted=read('diagnostic-interrupted');expected=read('final')
assert len(interrupted['entities'])==7
for uid,e in zip(expected['uids'],expected['entities']):
 equal(e,interrupted['entities'][interrupted['uids'].index(uid)],'diagnostic/original/'+uid)
assert len(set(interrupted['uids'])-set(expected['uids']))==3
images=json.loads((B/'gui-events.json').read_text());dims=set()
assert {p.name for p in B.glob('*.jpg')}=={r['file']for r in images}
for rec in images:
 p=B/rec['file'];assert sha(p)==rec['sha256'];data=p.read_bytes();assert data[:2]==b'\xff\xd8';i=2
 while i<len(data):
  assert data[i]==255
  while data[i]==255:i+=1
  mark=data[i];i+=1;n=struct.unpack_from('>H',data,i)[0]
  if mark in (192,193,194):h,w=struct.unpack_from('>HH',data,i+3);dims.add((w,h));break
  i+=n
runs=json.loads((B/'test-runs.json').read_text())
for rec in runs:
 assert sha(B/rec['log'])==rec['sha256'],rec['name']
 assert rec['exitCode']==0,rec['name']
previous=json.loads((B/'previous-kcad.json').read_text());assert len(previous)==1828
for p,h in previous.items():assert sha(R/p)==h,p
if (B/'source-files.json').exists():
 for p,h in json.loads((B/'source-files.json').read_text()).items():assert sha(R/p)==h,p
for name in ['before-binary.json','after-binary.json','after-isolated-binary.json']:
 if (B/name).exists():
  meta=json.loads((B/name).read_text());binary=Path(meta['app'])/'Contents/MacOS/kentos-cad-bin'
  if binary.exists():assert sha(binary)==meta['sha256']
  if name.startswith('after'):
   for p,h in meta['sourceFiles'].items():assert sha(R/p)==h,p
base='ea7d1baeeb2e949062eb3aec6d900d4b697c7983'
catalog_path='apps/web/src/contracts/generated/commandCatalog.json'
old_bytes=subprocess.check_output(['git','show',base+':'+catalog_path],cwd=R)
new_bytes=(R/catalog_path).read_bytes();changes=[]
def catalog_equal(a,b,path=()):
 if isinstance(a,dict):
  assert isinstance(b,dict) and a.keys()==b.keys(),path
  for k in a:catalog_equal(a[k],b[k],path+(k,))
 elif isinstance(a,list):
  assert isinstance(b,list) and len(a)==len(b),path
  for i,(x,y)in enumerate(zip(a,b)):catalog_equal(x,y,path+(i,))
 elif a!=b:
  assert isinstance(a,str)and isinstance(b,str),path
  assert path[-1]=='description' or (len(path)==3 and path[0]=='commands' and path[-1]=='summary'),path
  changes.append(path)
catalog_equal(json.loads(old_bytes),json.loads(new_bytes))
added_ts=subprocess.check_output(['git','diff','--name-only','--diff-filter=A',base,'--','*.ts','*.tsx'],cwd=R,text=True).splitlines()
untracked_ts=subprocess.check_output(['git','ls-files','--others','--exclude-standard','--','*.ts','*.tsx'],cwd=R,text=True).splitlines()
assert not added_ts and not untracked_ts,(added_ts,untracked_ts)
catalog_report=dict(schemaStructureUnchanged=True,onlyDescriptionOrSummaryChanges=True,changedStringLeaves=len(changes),newTypeScriptFiles=0,oldCatalogSha256=hashlib.sha256(old_bytes).hexdigest(),newCatalogSha256=hashlib.sha256(new_bytes).hexdigest())
(B/'catalog-verification.json').write_text(json.dumps(catalog_report,indent=2)+'\n')
report=dict(fullNativeModelsVerified=len(records),diagnosticModelsVerified=len(diagnostics),rawImagesVerified=len(images),acceptedImages=sum(x['accepted']for x in images),diagnosticImages=sum(not x['accepted']for x in images),imageDimensions=[list(x)for x in sorted(dims)],previousKcadBytesUnchanged=len(previous),automaticDimensionChainsAccepted=False,wholeArchitectureAccepted=False)
(B/'verification.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
