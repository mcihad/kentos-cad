#!/usr/bin/env python3
"""Independent KCAD and Decimal checks; no KentOS SDK or geometry implementation."""
import importlib.util
import json
from decimal import Decimal as D
from pathlib import Path

BASE=Path(__file__).resolve().parent
ROOT=BASE.parents[2]
spec=importlib.util.spec_from_file_location('kcad',ROOT/'tools/kcad/kcad.py')
kcad=importlib.util.module_from_spec(spec);spec.loader.exec_module(kcad)
def read(name):return kcad.read((BASE/name).read_bytes())[1]
def objects(doc):
    return {u:{k:v for k,v in e.items() if k!='id'} for u,e in zip(doc['uids'],doc['entities'])}
def same(a,b):
    assert a['settings']==b['settings'];assert objects(a)==objects(b)
def area(ring):
    pts=[(D(str(p['x']))-500000,D(str(p['y']))-4400000) for p in ring['pts']]
    assert not ring.get('bulges')
    return abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(pts,pts[1:]+pts[:1])))/2
def check_room(doc,level,expected):
    old=objects(before);new=objects(doc)
    assert doc['settings']==before['settings']
    for uid,e in old.items():assert new[uid]==e
    extra=[(u,e) for u,e in new.items() if u not in old]
    assert len(extra)==1
    uid,room=extra[0];assert room['kind']=='space'
    assert room['arch']['level']==level and not room.get('broken')
    assert room.get('method','net')=='net'
    assert sum(area(p['outer'])-sum(area(h) for h in p.get('holes',[])) for p in room['parts'])==D(expected)
    return uid,room

before=read('before.kcad');assert len(before['entities'])==3
upper=read('upper-after.kcad');uid,room=check_room(upper,'kat-1','76.44')
same(before,read('upper-undo.kcad'));same(upper,read('upper-redo.kcad'))
lost=read('upper-lost.kcad');lost_objects=objects(lost);upper_objects=objects(upper)
assert lost['settings']==upper['settings']
removed=set(upper_objects)-set(lost_objects);assert len(removed)==1
removed_wall=upper_objects[next(iter(removed))]
assert removed_wall['kind']=='wall' and removed_wall['arch']['level']=='kat-1'
for key,e in lost_objects.items():
    if key!=uid:assert e==upper_objects[key]
assert lost_objects[uid]['broken']=='lostEnclosure'
assert {k:v for k,v in lost_objects[uid].items() if k!='broken'}==room
same(upper,read('upper-lost-undo.kcad'))
ground=read('ground-relevel.kcad');new_uid,ground_room=check_room(ground,'zemin','22.04');assert new_uid==uid
assert {k:v for k,v in room.items() if k not in ('arch','parts')}=={k:v for k,v in ground_room.items() if k not in ('arch','parts')}
assert {k:v for k,v in room['arch'].items() if k!='level'}=={k:v for k,v in ground_room['arch'].items() if k!='level'}
same(upper,read('relevel-undo.kcad'));same(ground,read('relevel-redo.kcad'))
same(ground,read('reopened.kcad'))
same(ground,read('narrow-cancel.kcad'))
for event in json.loads((BASE/'gui-events.json').read_text())['events']:
    if 'file' in event:assert (BASE/'shots'/event['file']).is_file(),event
print('PASS: same-elevation semantic levels, 76.44→22.04 m², same room UUID; exact source/settings preservation, one create/relevel undo/redo, lost enclosure/undo, reopen and narrow preview cancellation')
