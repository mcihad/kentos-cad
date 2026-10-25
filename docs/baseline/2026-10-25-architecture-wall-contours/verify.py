#!/usr/bin/env python3
"""Independent KCAD reader and Decimal expectations for actual GUI saves."""
import importlib.util
from decimal import Decimal as D
from pathlib import Path
root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('kcad', root/'tools/kcad/kcad.py')
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)
base = Path(__file__).parent
def read(name): return kcad.read((base/name).read_bytes())[1]
def semantic(doc):
    # File-local IDs are compacted on write; persistent UIDs are authoritative.
    return {u:{k:v for k,v in e.items() if k != 'id'} for u,e in zip(doc['uids'],doc['entities'])}
def same(a,b):
    assert a['settings'] == b['settings']
    assert semantic(a) == semantic(b)
def kept(a,b):
    assert a['settings'] == b['settings']
    bmap=semantic(b)
    for uid,e in semantic(a).items(): assert bmap[uid] == e

def inner_area(w):
    pts=[(D(str(p['x'])),D(str(p['y']))) for p in w['pts']]
    width=D(str(w['width']))
    offset={'left':D(0),'right':width,'center':width/2}[w['justify']]
    normals=[]
    for a,b in zip(pts,pts[1:]+pts[:1]):
        x,y=b[0]-a[0],b[1]-a[1]
        assert (x==0) != (y==0), 'This independent fixture expects orthogonal edges'
        length=abs(x)+abs(y)
        normals.append((-y/length,x/length))
    face=[(p[0]+offset*(normals[i-1][0]+normals[i][0]),p[1]+offset*(normals[i-1][1]+normals[i][1])) for i,p in enumerate(pts)]
    return sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(face,face[1:]+face[:1]))/2

before=read('before.kcad'); l=read('l-inner-keep.kcad'); avlu=read('courtyard-outer-keep.kcad')
assert len(before['entities'])==4 and len(l['entities'])==5 and len(avlu['entities'])==7
same(before,read('l-undo.kcad'));kept(before,l);kept(l,avlu)
same(avlu,read('repeat-cancel.kcad'));same(avlu,read('reverse-undo.kcad'))
lwall=l['entities'][-1]
assert lwall['width']==.25 and lwall['justify']=='left' and lwall['closed']
assert lwall['pts']==before['entities'][0]['pts']
assert inner_area(lwall)==D(18)
avlu_walls=avlu['entities'][-2:]
assert all(w['width']==.25 and w['justify']=='right' and w['closed'] for w in avlu_walls)
assert sum(inner_area(w) for w in avlu_walls)==D(65)
for doc in (l,avlu):
    for w in doc['entities'][4:]:
        source=next(e for e in before['entities'] if e['attrs']==w['attrs'])
        assert source['color']==w['color'] and source['layerId']==w['layerId']
replaced=read('reverse-center-remove.kcad'); reopened=read('reopened.kcad')
assert len(replaced['entities'])==7
prior=semantic(avlu); current=semantic(replaced)
removed_uid=before['uids'][2]
assert removed_uid not in current
for uid,e in prior.items():
    if uid!=removed_uid: assert current[uid]==e
new_ids=set(current)-set(prior)
assert len(new_ids)==1
w=current[next(iter(new_ids))]
assert w['kind']=='wall' and w['justify']=='center' and w['width']==.25
assert w['pts']==[{'x':0.,'y':10.},{'x':6.,'y':10.},{'x':6.,'y':14.},{'x':0.,'y':14.}]
assert w['attrs']==before['entities'][2]['attrs'] and w['color']==before['entities'][2]['color']
assert inner_area(w)==D('21.5625')
same(replaced,reopened)
assert reopened['settings']==before['settings'] and reopened['settings']['plotScale']==100
print('GUI saves verified: L 18 m², courtyard 65 m², reverse centre 21.5625 m²; source attributes/UIDs/settings; undo, repeat refusal, replacement and reopen.')
