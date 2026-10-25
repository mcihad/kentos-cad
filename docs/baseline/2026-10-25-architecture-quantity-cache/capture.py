"""Freeze full old query answers before an internal cache change; not an independent geometry oracle."""
import copy,hashlib,importlib.util,json,sys,time,uuid
from pathlib import Path
from kentos import cad
root=Path.cwd();base=root/'docs/baseline/2026-10-25-architecture-quantity-cache'
spec=importlib.util.spec_from_file_location('reader',root/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(spec);spec.loader.exec_module(reader)
_,template=reader.read((root/'docs/baseline/2026-10-25-architecture-space-separators/separator-created.kcad').read_bytes())
spec=importlib.util.spec_from_file_location('writer',root/'scripts/fixtures/kcad_v2_reference.py');writer=importlib.util.module_from_spec(spec);spec.loader.exec_module(writer)
mode=sys.argv[1];count=0
if mode=='before' and (base/'answers-before.json').exists():raise SystemExit('Frozen old answers must not be regenerated; use after.')
if mode=='before':
 for index,(unit,far) in enumerate((u,f)for u in ['mm','cm','m','in','ft']for f in [False,True]):
  for role in [True,False]:
   d=copy.deepcopy(template);d['version']=2;d['settings']['drawingUnit']=unit
   # Document order deliberately differs from slot id order.
   room=next(e for e in d['entities']if e['kind']=='space');wall=next(e for e in d['entities']if e['kind']=='wall')
   col=next(e for e in d['entities']if e['kind']=='column');sep=next(e for e in d['entities']if e['kind']=='polyline')
   pairs=list(zip(d['uids'],d['entities']));by_id={e['id']:u for u,e in pairs}
   variants=[{},dict(columns='net'),dict(columns='ignore'),dict(scope='unassigned'),dict(sources=[by_id[1],by_id[2]]),dict(sources=[by_id[1],by_id[2],by_id[4]]),dict(sources=[str(uuid.UUID(int=999999))]),dict(arch=dict(level='zemin')),dict(arch=dict(level='kat-1')),dict(seed=None)]
   for i,change in enumerate(variants):
    r=copy.deepcopy(room);r['id']=50+i;r.update(change)
    if r.get('seed')is None:
     r.pop('seed',None);r['columns']='ignore'
    if 'sources'in r:r['sources'].sort()
    pairs.append((str(uuid.UUID(int=8000+i)),r))
   if not role:
    sep.pop('spaceSeparator',None)
   for e in [wall,col,sep]:
    if e['id']==1:e['arch']=dict(level='zemin')
   # Unknown/absent sources and valid explicit sets coexist in one query.
   upper=copy.deepcopy(wall);upper['id']=90;upper['width']=.4;upper['arch']=dict(level='kat-1')
   pairs.append((str(uuid.UUID(int=999990)),upper))
   pairs=pairs[3:]+pairs[:3]
   if far:
    def shift(v):
     if isinstance(v,dict):
      if set(v)=={'x','y'}:v['x']+=500000;v['y']+=4400000
      else:
       for x in v.values():shift(x)
     elif isinstance(v,list):
      for x in v:shift(x)
    for _,e in pairs:shift(e)
   d['uids']=[u for u,_ in pairs];d['entities']=[e for _,e in pairs]
   name=f'{unit}-'+('far'if far else'local')+('-role'if role else'-legacy-path')
   (base/(name+'.json')).write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
records=[]
for p in sorted(base.glob('*.json')):
 if not p.stem.endswith(('-role','-legacy-path')):continue
 if mode=='before':
  canonical=json.loads(p.read_text())
  for i,e in enumerate(canonical['entities']):
   e['id']=i+1
   if e.get('columns')=='ignore':e.pop('columns')
  p.with_suffix('.kcad').write_bytes(writer.container(writer.document(canonical)))
 doc=cad.Document.open(p.with_suffix('.kcad'));before=doc.to_bytes();answers=[]
 for query in [{},{'levels':['zemin']},{'levels':['kat-1']},{'layers':['mahal']},{'usages':['ofis']}]:
  start=time.perf_counter();answer=cad.spaces.quantities.execute_input(doc,cad.SpacesQuantities.from_json(query)).to_json()
  assert doc.to_bytes()==before
  answers.append(dict(query=query,output=answer));count+=1
 records.append(dict(file=p.name,answers=answers))
if mode=='before':
 (base/'answers-before.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n')
else:
 old=json.loads((base/'answers-before.json').read_text());assert records==old,'full query output changed'
 (base/'answers-after.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n')
print('PASS',mode,count,'full query answers and read-only KCAD preservation')
