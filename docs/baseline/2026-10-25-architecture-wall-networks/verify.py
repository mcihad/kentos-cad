#!/usr/bin/env python3
"""Read GUI-saved KCAD independently of native geometry and the Python SDK."""
import importlib.util
import json
import math
from collections import Counter
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]
spec = importlib.util.spec_from_file_location('kcad', ROOT/'tools/kcad/kcad.py')
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)

def read(name):
    return kcad.read((BASE/name).read_bytes())[1]

def objects(doc):
    return {u: {k:v for k,v in e.items() if k != 'id'}
            for u,e in zip(doc['uids'], doc['entities'])}

def same(a, b):
    assert a['settings'] == b['settings']
    assert objects(a) == objects(b)

def wall_length(w):
    pts=w['pts'];total=0.
    for i in range(len(pts) if w.get('closed',False) else len(pts)-1):
        a,b=pts[i],pts[(i+1)%len(pts)]
        chord=math.hypot(b['x']-a['x'], b['y']-a['y'])
        bulge=w.get('bulges',[0.]*len(pts))[i]
        total+=chord if bulge==0 else chord*(1+bulge*bulge)/abs(bulge)*math.atan(abs(bulge))
    return total

grid=read('grid-before.kcad');after=read('grid-after-delete.kcad')
assert len(grid['entities']) == 1600 and len(after['entities']) == 3276
assert grid['settings'] == after['settings']
assert all(e['kind']=='polygon' for e in grid['entities'])
assert not (objects(grid).keys() & objects(after).keys())
coverage=Counter()
for w in after['entities']:
    assert w['kind']=='wall' and w['width']==.2 and w['justify']=='center'
    assert not w.get('closed',False) and not w.get('bulges')
    assert w['color']=='#3B94D9' and w['layerId']=='cizim'
    assert not w.get('arch')
    for a,b in zip(w['pts'],w['pts'][1:]):
        x1,y1=(a['x']-500000)/4,(a['y']-4400000)/3
        x2,y2=(b['x']-500000)/4,(b['y']-4400000)/3
        assert all(v==int(v) and 0<=v<=40 for v in [x1,y1,x2,y2])
        x1,y1,x2,y2=map(int,[x1,y1,x2,y2])
        assert (x1==x2) != (y1==y2)
        if y1==y2:
            for x in range(min(x1,x2),max(x1,x2)):coverage['h',y1,x]+=1
        else:
            for y in range(min(y1,y2),max(y1,y2)):coverage['v',x1,y]+=1
expected={('h',r,c) for r in range(41) for c in range(40)}
expected|={('v',c,r) for c in range(41) for r in range(40)}
assert set(coverage)==expected and set(coverage.values())=={1}
assert sum(map(wall_length,after['entities']))==11480
same(grid,read('grid-undo.kcad'))
for name in ['grid-redo.kcad','grid-reopened.kcad']:same(after,read(name))

final_after=read('grid-final-source-after.kcad')
assert final_after['settings']==after['settings']
normalize=lambda items:Counter(json.dumps(e,sort_keys=True) for e in items)
assert normalize(objects(final_after).values())==normalize(objects(after).values())
same(grid,read('grid-final-source-undo.kcad'))
same(final_after,read('grid-final-source-redo.kcad'))

mixed=read('mixed-before.kcad');kept=read('mixed-after-keep.kcad')
old,new=objects(mixed),objects(kept)
assert len(old)==7 and len(new)==16
assert mixed['settings']==kept['settings']
for u,e in old.items():assert new[u]==e
walls=[e for u,e in new.items() if u not in old]
assert len(walls)==9 and all(w['kind']=='wall' for w in walls)
assert sum(bool(w.get('closed',False)) for w in walls)==6
assert sum(bool(w.get('bulges')) for w in walls)==2
assert math.isclose(sum(map(wall_length,walls)),115+8*math.pi,abs_tol=1e-8)
assert all(w['width']==.2 and w['justify']=='center' and w['color']=='#3B94D9'
           and w['layerId']=='cizim' and not w.get('arch') for w in walls)
assert Counter(w['attrs']['oda'] for w in walls)=={'A':2,'C':3,'D':1,'E':1,'F':1,'G':1}
same(mixed,read('mixed-undo.kcad'))
same(mixed,read('mixed-refused.kcad'))
same(kept,read('mixed-redo.kcad'))
deleted=read('mixed-after-delete.kcad')
assert len(deleted['entities'])==9 and mixed['settings']==deleted['settings']
assert not (objects(mixed).keys() & objects(deleted).keys())
normalize=lambda items:Counter(json.dumps(e,sort_keys=True) for e in items)
assert normalize(walls)==normalize(objects(deleted).values())
same(deleted,read('mixed-reopened.kcad'))
for doc in [grid,after,mixed,kept,deleted]:
    assert doc['settings'].get('drawingUnit','m')=='m'
    assert doc['settings']['plotScale']==100
for event in json.loads((BASE/'gui-events.json').read_text())['events']:
    if 'file' in event:assert (BASE/'shots'/event['file']).is_file(),event
print('PASS: grid 1600→3276, all 3280 elementary spans once, 11480 m; mixed 7→16/9, 115+8π m, exact analytic curves, ownership, source UUID/settings, one undo/redo, atomic reference refusal and reopen')
