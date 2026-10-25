#!/usr/bin/env python3
"""Independent saved-model and Decimal-area acceptance; no KentOS native code."""
import copy,csv,hashlib,importlib.util,json
from decimal import Decimal as D
from pathlib import Path
BASE=Path(__file__).resolve().parent;ROOT=BASE.parents[2]
spec=importlib.util.spec_from_file_location('kcad',ROOT/'tools/kcad/kcad.py');kcad=importlib.util.module_from_spec(spec);spec.loader.exec_module(kcad)
docs={p.stem:kcad.read(p.read_bytes())[1]for p in BASE.glob('*.kcad')}
def rows(doc):return {uid:dict(e)for uid,e in zip(doc['uids'],doc['entities'])}
before=docs['before'];original=rows(before);room_id=next(u for u,e in original.items()if e['kind']=='space')
source_ids={e['id']:u for u,e in original.items()};assert before['settings']['drawingUnit']=='m'and before['settings']['plotScale']==100
created=rows(docs['separator-created']);separator_id=next(u for u,e in created.items()if e['id']==4)
path=[dict(x=5.,y=0.),dict(x=5.,y=8.)]
states={
 'before':('72.44',None,False),'separator-undo':('72.44',None,False),
 'separator-created':('34.22',{},False),'separator-redo':('34.22',{},False),
 'role-cleared':('72.44','cleared',False),'role-undo':('34.22',{},False),
 'level-set':('34.22',dict(level='zemin'),False),'level-undo':('34.22',{},False),
 'phase-set':('34.22',dict(phase='existing'),False),'phase-undo':('34.22',{},False),
 'context-confirmed':('27.36',{},True),'context-undo':('34.22',{},False),
 'reopened':('34.22',{},False),'missing-wall-rejected':('34.22',{},False),'explicit-sources':('34.22',{},False),'explicit-sources-undo':('34.22',{},False),
 'final-matrix-unchanged':('34.22',{},False),
 'final-context-confirmed':('27.36',{},True),'final-context-undo':('34.22',{},False),
 'final-unchanged':('34.22',{},False),'final-role-cleared':('72.44','cleared',False),'final-role-undo':('34.22',{},False),
 'escape-cancelled':('34.22',{},False),'after-zoom':('34.22',{},False),'small-zoom-return':('34.22',{},False),
 'verified-created':('27.36',{},True),'verified-undo':('34.22',{},False),'verified-reopened':('34.22',{},False),
 'verified-warning-ignore':('38.22',{},False),'verified-warning-lost':('38.22',{},False),'verified-warning-repaired':('38.22',{},False),'verified-warning-restored':('34.22',{},False),
 'warning-before-ignore':('38.22',{},False),'warning-before-lost':('38.22',{},False),'warning-before-restored':('34.22',{},False),
}
assert set(docs)==set(states),(set(docs)^set(states))
def area(r):
 assert not r.get('bulges')
 pts=[(D(str(p['x'])),D(str(p['y'])))for p in r['pts']]
 return abs(sum(a[0]*b[1]-a[1]*b[0]for a,b in zip(pts,pts[1:]+pts[:1])))/2
for name,doc in docs.items():
 expected,role,extra=states[name];actual=rows(doc)
 warning_ignore=name in ('warning-before-ignore','warning-before-lost','verified-warning-ignore','verified-warning-lost','verified-warning-repaired')
 lost=name in ('warning-before-lost','verified-warning-lost')
 assert {k:v for k,v in doc.items()if k not in ('entities','uids')}=={k:v for k,v in before.items()if k not in ('entities','uids')},(name,'root/settings')
 for u,e in original.items():
  if lost and e['id']==1:
   assert u not in actual,(name,'only the deliberately removed outer wall is absent')
   continue
  if u!=room_id:
   expected_source=dict(e)
   if lost:expected_source['id']-=1 # KCAD canonical slots compact after the first entity is removed; UUIDs stay identical.
   assert actual[u]==expected_source,(name,'untouched source',e['id'])
 room=actual[room_id]
 expected_room=copy.deepcopy(original[room_id])
 if lost:expected_room['id']-=1
 if warning_ignore:expected_room.pop('columns',None)
 if lost:expected_room['broken']='lostEnclosure'
 assert {k:v for k,v in room.items()if k not in ('parts','sources')}=={k:v for k,v in expected_room.items()if k not in ('parts','sources')},(name,'room metadata')
 expected_sources=sorted([source_ids[1],source_ids[2],separator_id]) if name=='explicit-sources' else None
 assert room.get('sources')==expected_sources,(name,'source UUIDs')
 measured=sum(area(p['outer'])-sum(area(h)for h in p.get('holes',[]))for p in room['parts'])
 assert abs(measured-D(expected))<D('1e-9'),(name,measured,expected)
 assert len(actual)==3+int(role is not None)+int(extra)-int(lost),(name,'object count')
 if role is None:assert separator_id not in actual,name
 else:
  e=actual[separator_id];assert e['pts']==path and e.get('lineType')=='dashed'and e.get('widths')is None,(name,'separator geometry/look')
  assert e.get('spaceSeparator')==(None if role=='cleared'else role),(name,'role')
 if extra:
  second=next(e for e in actual.values()if e['id']==5)
  assert second['pts']==[dict(x=0.,y=6.5),dict(x=5.,y=6.5)]and second['spaceSeparator']=={},name
for a,b in [('before','separator-undo'),('separator-created','separator-redo'),('separator-created','role-undo'),('separator-created','level-undo'),('separator-created','phase-undo'),('separator-created','context-undo'),('separator-created','escape-cancelled'),('separator-created','after-zoom'),('separator-created','small-zoom-return'),('separator-created','reopened'),('separator-created','missing-wall-rejected'),('separator-created','explicit-sources-undo'),('separator-created','final-unchanged'),('separator-created','final-role-undo'),('separator-created','final-context-undo'),('separator-created','final-matrix-unchanged'),('role-cleared','final-role-cleared')]:
 assert docs[a]==docs[b],(a,b,'undo/redo exact model')
assert docs['warning-before-restored']==docs['separator-created'],'warning regression real double undo exact model'
for a,b in [('verified-undo','separator-created'),('verified-reopened','separator-created'),('verified-warning-restored','separator-created'),('verified-warning-ignore','warning-before-ignore'),('verified-warning-repaired','warning-before-ignore'),('verified-warning-lost','warning-before-lost')]:
 assert docs[a]==docs[b],(a,b,'final real GUI model preservation')

events=json.loads((BASE/'gui-events.json').read_text())
assert {e['file']for e in events}=={p.name for p in BASE.glob('*.jpg')},'image/event inventory mismatch'
for event in events:assert hashlib.sha256((BASE/event['file']).read_bytes()).hexdigest()==event['sha256'],event['file']
print('PASS',len(docs),'actual GUI saves;',len(events),'raw screenshots; Decimal areas, roles, UUIDs, exact source/settings and undo/redo preservation')

# These are the actual native Save dialog exports, not generated example tables.
checks=[]
for file,area_expected,perimeter_expected,columns in [('warning-verified.csv','38.22','25.4',13),('valid-verified.csv','34.22','33.4',12)]:
 raw=(BASE/file).read_bytes()
 with (BASE/file).open(encoding='utf-8-sig',newline='')as f: records=list(csv.DictReader(f,delimiter=';'))
 assert len(records)==1 and len(records[0])==columns,(file,'shape')
 r=records[0];assert r['Numara']=='M01'and r['Ad']=='Salon',file
 assert abs(D(r['Alan'])-D(area_expected))<D('1e-9')and abs(D(r['Çevre'])-D(perimeter_expected))<D('1e-9'),(file,r)
 if columns==13: assert r['Durum'].startswith('Son geçerli sınır')and 'Mahali duvarlar artık kapatmıyor' in r['Durum'],r
 else:
  assert 'Durum'not in r and all(abs(D(r[k])-D(v))<D('1e-9') for k,v in [('Net','34.22'),('Eksen','36'),('Brüt','37.82')]),r
 checks.append(dict(file=file,sha256=hashlib.sha256(raw).hexdigest(),rows=1,columns=columns,values=r))
print('PASS actual CSV exports:',json.dumps(checks,ensure_ascii=False))
