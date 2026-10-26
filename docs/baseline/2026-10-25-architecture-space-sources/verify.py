#!/usr/bin/env python3
"""Independent Decimal areas and exact persistent source/model preservation."""
import copy, hashlib, importlib.util, json
from decimal import Decimal as D
from pathlib import Path
BASE=Path(__file__).resolve().parent;ROOT=BASE.parents[2]
spec=importlib.util.spec_from_file_location('kcad',ROOT/'tools/kcad/kcad.py')
kcad=importlib.util.module_from_spec(spec);spec.loader.exec_module(kcad)
docs={p.stem:kcad.read(p.read_bytes())[1] for p in BASE.glob('*.kcad')}
before=docs['before'];single=docs['single-selected']
def rows(doc):
    result={}
    for uid,entity in zip(doc['uids'],doc['entities']):
        e=copy.deepcopy(entity);e.pop('id');result[uid]=e
    return result
original=rows(before);first=rows(single);room_id=next(uid for uid,e in first.items() if e['kind']=='space')
source_ids={e['id']:uid for uid,e in zip(before['uids'],before['entities'])}
assert before['settings']['drawingUnit']=='m' and before['settings']['plotScale']==100
# expected area, persistent source members (None means all), deleted source slot, break
states={
 'before':None,'before-bulk':None,'bulk-undo':None,
 'single-selected':('72.44',[1,3],None,None),
 'edit-wall-only':('76.44',[1],None,None),
 'edit-redo':('76.44',[1],None,None),
 'edit-empty-rejected':('76.44',[1],None,None),
 'edit-undo':('72.44',[1,3],None,None),
 'edit-restored-two':('72.44',[1,3],None,None),
 'empty-rejected':('72.44',[1,3],None,None),
 'final-empty-rejected':('72.44',[1,3],None,None),
 'partition-added':('33.44',[1,2,3],None,None),
 'all-sources-cleared':('33.44',None,None,None),
 'clear-undo':('33.44',[1,2,3],None,None),
 'partition-undo':('72.44',[1,3],None,None),
 'excluded-column-deleted':('72.44',[1,3],4,None),
 'excluded-delete-undo':('72.44',[1,3],None,None),
 'selected-column-deleted':('72.44',[1,3],3,'sourceUnavailable'),
 'selected-delete-undo':('72.44',[1,3],None,None),
 'bulk-selected':('72.44',[1,3],None,None),
 'bulk-redo':('72.44',[1,3],None,None),
 'matrix-cancelled':('72.44',[1,3],None,None),
 'verified-wall-only':('76.44',[1],None,None),
 'verified-undo':('72.44',[1,3],None,None),
 'verified-redo':('76.44',[1],None,None),
 'verified-reopened':('76.44',[1],None,None),
 'verified-context-confirm':('72.44',[1,3],None,None),
 'verified-context-undo':('76.44',[1],None,None),
 'latest-unchanged':('76.44',[1],None,None),
 'verified-dark18-cancelled':('76.44',[1],None,None),
}
assert set(docs)==set(states),(set(docs)^set(states))
bulk_names={name for name in states if name.startswith(('bulk-selected','bulk-redo','matrix-','verified-','latest-'))}
bulk_uid,bulk_room=next((uid,e) for uid,e in rows(docs['bulk-selected']).items() if e['kind']=='space')
def ring_area(ring):
    assert not ring.get('bulges')
    pts=[(D(str(p['x'])),D(str(p['y']))) for p in ring['pts']]
    return abs(sum(a[0]*b[1]-a[1]*b[0] for a,b in zip(pts,pts[1:]+pts[:1])))/2
for name,doc in docs.items():
    assert {k:v for k,v in doc.items() if k not in ('entities','uids')}=={k:v for k,v in before.items() if k not in ('entities','uids')},(name,'settings or root changed')
    actual=rows(doc);state=states[name]
    deleted=state[2] if state else None
    for slot,uid in source_ids.items():
        if slot==deleted:assert uid not in actual,(name,'source was not deleted')
        else:assert actual[uid]==original[uid],(name,'unrelated source changed',slot)
    rooms={u:e for u,e in actual.items() if e['kind']=='space'}
    if state is None:
        assert not rooms and actual==original,name
        continue
    expected_area,members,_,broken=state
    assert len(rooms)==1 and len(actual)==len(original)+1-int(deleted is not None),name
    uid,room=next(iter(rooms.items()))
    assert uid==(bulk_uid if name in bulk_names else room_id),(name,'room UUID changed')
    assert room.get('sources')==(sorted(source_ids[n] for n in members) if members is not None else None),name
    assert room.get('broken')==broken,name
    area=sum(ring_area(p['outer'])-sum(ring_area(h) for h in p.get('holes',[])) for p in room['parts'])
    assert abs(area-D(expected_area))<D('0.000000001'),(name,area,expected_area)
    assert room.get('columns')=='net' and room.get('method','net')=='net' and room.get('seed'),name
    prior=bulk_room if name in bulk_names else first[room_id]
    assert {k:v for k,v in room.items() if k not in ('sources','parts','broken')}=={k:v for k,v in prior.items() if k not in ('sources','parts','broken')},(name,'room metadata changed')
for a,b in [('single-selected','edit-undo'),('single-selected','edit-restored-two'),('single-selected','empty-rejected'),('single-selected','final-empty-rejected'),('single-selected','partition-undo'),('single-selected','selected-delete-undo'),('single-selected','excluded-delete-undo'),('partition-added','clear-undo'),('bulk-selected','bulk-redo'),('before','bulk-undo'),('bulk-selected','matrix-cancelled'),('bulk-selected','verified-undo'),('verified-wall-only','verified-redo'),('verified-wall-only','verified-reopened'),('bulk-selected','verified-context-confirm'),('verified-wall-only','verified-context-undo'),('verified-wall-only','latest-unchanged')]:
    assert docs[a]==docs[b],(a,b,'undo/reject changed persisted document')
assert docs['verified-wall-only']==docs['verified-dark18-cancelled'],'dark18 cancellation changed the document'
events=json.loads((BASE/'gui-events.json').read_text())['events']
assert {p.name for p in (BASE/'shots').glob('*.jpg')}=={e['file'] for e in events}
for e in events:
    assert (BASE/'shots'/e['file']).read_bytes().startswith(b'\xff\xd8\xff')
    assert (BASE/e['build']).is_file()
images=json.loads((BASE/'shots.json').read_text())['images']
assert {i['file'] for i in images}=={e['file'] for e in events}
for image in images:
    assert hashlib.sha256((BASE/'shots'/image['file']).read_bytes()).hexdigest()==image['sha256'],image['file']
for run in json.loads((BASE/'test-runs.json').read_text())['runs']:
    assert run['exitCode']==0,run['name']
    assert hashlib.sha256((BASE/run['log']).read_bytes()).hexdigest()==run['logSha256'],run['log']
print(f'PASS {len(docs)} actual GUI saves, {len(events)} raw screenshots: Decimal area, UUID membership, source loss, undo/redo, exact settings/source preservation. Candidate defects remain recorded separately.')
