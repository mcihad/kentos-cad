#!/usr/bin/env python3
"""Independent reader + Decimal source/area verification of real GUI saves.

Screenshots 05/06/08 expose the first build's defects. The later fixed build
has separate navigation, bulk, dependency and save/open acceptance records.
"""
import importlib.util
import json
from decimal import Decimal as D
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]
spec = importlib.util.spec_from_file_location('kcad', ROOT/'tools/kcad/kcad.py')
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)
PI = D('3.14159265358979323846264338327950288419716939937510')

def read(name):
    return kcad.read((BASE/name).read_bytes())[1]

def objects(doc):
    return {u:{k:v for k,v in e.items() if k!='id'} for u,e in zip(doc['uids'],doc['entities'])}

def ring_area(r):
    pts=[(D(str(p['x']))-500000,D(str(p['y']))-4400000)for p in r['pts']]
    cross=sum(a[0]*b[1]-b[0]*a[1]for a,b in zip(pts,pts[1:]+pts[:1]))/2
    if r.get('bulges'):
        assert len(pts)==2 and len(r['bulges'])==2
        for i,b in enumerate(r['bulges']):
            assert abs(abs(b)-1)<1e-15, 'This reference covers only semicircular arcs.'
            a,c=pts[i],pts[(i+1)%2]
            chord2=(a[0]-c[0])**2+(a[1]-c[1])**2
            cross += (1 if b>0 else -1)*PI*chord2/8
    return abs(cross)

before=read('before.kcad');made=read('first-ground-created.kcad');undo=read('first-ground-undo.kcad')
assert len(before['entities'])==6
for key in before:
    if key not in ('entities','uids','nextId'):
        assert made[key]==before[key]==undo[key],(key,'project metadata changed')
assert made['settings']==before['settings']==undo['settings']
old,new=objects(before),objects(made)
for uid,e in old.items(): assert new[uid]==e,(uid,'source changed')
assert objects(undo)==old
assert objects(read('first-ground-redo.kcad'))==new
assert objects(read('first-inspector-undo.kcad'))==new
ignored=read('first-inspector-ignore.kcad')
assert ignored['settings']==before['settings']
assert set(objects(ignored))==set(new)
for uid,e in old.items(): assert objects(ignored)[uid]==e
legacy_room=next(e for u,e in objects(ignored).items() if u not in old)
assert 'columns' not in legacy_room and not legacy_room['parts'][0].get('holes')
assert abs(ring_area(legacy_room['parts'][0]['outer'])-D('76.44'))<D('.000001')
extras={u:e for u,e in new.items() if u not in old}
assert len(extras)==1
room=next(iter(extras.values()))
assert room['kind']=='space' and room['columns']=='net'
assert room['arch']['level']=='zemin' and not room.get('broken')
assert len(room['parts'])==1 and len(room['parts'][0]['holes'])==2
measured=sum(ring_area(p['outer'])-sum(ring_area(h)for h in p.get('holes',[]))for p in room['parts'])
expected=D('9.8')*D('7.8')-D('.6')*D('.8')-PI/4
assert abs(measured-expected)<D('.000001'),measured
assert before['settings']['drawingUnit']=='mm'
assert before['settings']['plotScale']==100
assert read('fixed-navigation-cancel.kcad')==before, 'Navigation/cancel changed saved drawing'
bulk=read('fixed-bulk-created.kcad')
assert objects(read('fixed-bulk-undo.kcad'))==old
assert objects(read('fixed-bulk-redo.kcad'))==objects(bulk)
for uid,e in old.items(): assert objects(bulk)[uid]==e
bulk_rooms=[e for e in bulk['entities'] if e['kind']=='space']
assert len(bulk_rooms)==1 and bulk_rooms[0]['columns']=='net'
assert bulk_rooms[0]['arch']['level']=='zemin'
assert abs(sum(ring_area(p['outer'])-sum(ring_area(h) for h in p.get('holes',[])) for p in bulk_rooms[0]['parts'])-expected)<D('.000001')
related=read('fixed-related-before.kcad');moved=read('fixed-grid-moved.kcad')
assert objects(read('fixed-grid-undo.kcad'))==objects(related)
assert read('fixed-grid-redo.kcad')==moved
assert read('fixed-reopened.kcad')==moved, 'GUI reopen/save changed the saved model'
assert len(related['entities'])==len(moved['entities'])==9
assert set(objects(related))==set(objects(moved)), 'A dependent UUID changed'
for doc in (bulk,read('fixed-bulk-undo.kcad'),related,moved):
    for key in before:
        if key not in ('entities','uids','nextId'):
            assert doc[key]==before[key],(key,'project metadata changed')
for uid,e in objects(related).items():
    if e['kind'] not in ('space','tag','table') and not (e['kind']=='column' and e.get('tag')=='S1') and not (e['kind']=='grid' and e.get('name')=='A'):
        assert objects(moved)[uid]==e,(uid,'unrelated source changed')
for doc,area,perimeter in ((related,expected,'41141.593'),(moved,expected+D('.32'),'38741.593')):
    s=next(e for e in doc['entities'] if e['kind']=='space')
    assert s['columns']=='net' and not s.get('broken')
    got=sum(ring_area(p['outer'])-sum(ring_area(h) for h in p.get('holes',[])) for p in s['parts'])
    assert abs(got-area)<D('.000001'),(got,area)
    tag=next(e for e in doc['entities'] if e['kind']=='tag')
    table=next(e for e in doc['entities'] if e['kind']=='table')
    text=f'{area:.2f}'
    assert tag['text'].endswith(text+' m²')
    assert table['cells'][1][4]==table['cells'][2][4]==text
    assert table['cells'][1][5]==table['cells'][2][5]==perimeter
    room_uid=doc['uids'][doc['entities'].index(s)]
    assert tag['of']==room_uid and table['source']['members']==[[room_uid]]
col=next(e for e in moved['entities'] if e['kind']=='column' and e['tag']=='S1')
axis=next(e for e in moved['entities'] if e['kind']=='grid' and e['name']=='A')
assert col['at']=={'x':500000.,'y':4400002.}
assert all(p['x']==500000. for p in axis['pts'])
gross_net=read('fixed-gross-net.kcad');gross_all=read('fixed-gross-all.kcad')
assert read('fixed-covered-undo.kcad')==gross_all
gross_expected=D('10.2')*D('8.2')
for doc,policy,target in ((gross_net,'net',gross_expected),(gross_all,'all',gross_expected-D('.4')*D('.8')-PI/4)):
    assert doc['settings']==before['settings']
    assert set(objects(doc))==set(objects(moved))
    for uid,e in objects(moved).items():
        if e['kind'] not in ('space','tag','table'): assert objects(doc)[uid]==e
    s=next(e for e in doc['entities'] if e['kind']=='space')
    assert s['method']=='gross' and s['columns']==policy and not s.get('broken')
    area=sum(ring_area(p['outer'])-sum(ring_area(h) for h in p.get('holes',[])) for p in s['parts'])
    assert abs(area-target)<D('.000001'),(area,target)
    assert next(e for e in doc['entities'] if e['kind']=='tag')['text'].endswith(f'{target:.2f} m²')
    table=next(e for e in doc['entities'] if e['kind']=='table')
    assert table['cells'][1][4]==table['cells'][2][4]==f'{target:.2f}'
covered=read('fixed-fully-covered.kcad')
assert covered['settings']==gross_all['settings'] and set(objects(covered))==set(objects(gross_all))
for uid,e in objects(gross_all).items():
    later=objects(covered)[uid]
    if e['kind']=='column' and e.get('tag')=='S2':
        assert later==dict(e,width=20.)
    elif e['kind']=='space':
        assert later==dict(e,broken='noFreeArea'), 'Last usable boundary was lost'
    else: assert later==e, (uid,'full coverage changed another object')
assert read('accepted-covered-reopened.kcad')==covered, 'Final GUI build/settings/navigation changed the saved model'
for event in json.loads((BASE/'gui-events.json').read_text())['events']:
    assert (BASE/'shots'/event['file']).read_bytes().startswith(b'\xff\xd8\xff'),event
print('PASS real GUI saves: analytic area',measured,'m²; navigation/cancel unchanged; bulk one undo/redo; axis → column → room → tag/table +0.32 m²; redo/reopen exact; Gross Net/All policy; full coverage retains last boundary and undo recovers; UUIDs/metadata preserved.')
