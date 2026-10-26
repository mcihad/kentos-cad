#!/usr/bin/env python3
"""Standalone KCAD acceptance checks; no native geometry or SDK import."""
import importlib.util
import math
from pathlib import Path
base=Path(__file__).resolve().parent
root=base.parents[2]
spec=importlib.util.spec_from_file_location('kcad',root/'tools/kcad/kcad.py')
kcad=importlib.util.module_from_spec(spec);spec.loader.exec_module(kcad)
def read(name):return kcad.read((base/name).read_bytes())[1]
def semantic(doc):
 return {u:{k:v for k,v in e.items() if k!='id'} for u,e in zip(doc['uids'],doc['entities'])}
def same(a,b):
 assert a['settings']==b['settings']
 assert semantic(a)==semantic(b)
before=read('before.kcad');after=read('upper-floor.kcad')
assert len(before['entities'])==3 and len(after['entities'])==6
assert before['settings']==after['settings']
# KCAD omits the default drawingUnit=m; this reader preserves omitted keys.
assert after['settings'].get('drawingUnit','m')=='m' and after['settings']['plotScale']==100
old=semantic(before);new=semantic(after)
for uid,e in old.items():assert new[uid]==e
created=[e for uid,e in new.items() if uid not in old]
assert len(created)==3
paths=[[(p['x'],p['y']) for p in w['pts']] for w in created]
assert paths==[[(4,3),(0,3),(0,0),(4,0)],[(4,0),(4,3)],[(4,0),(8,0),(8,3),(4,3)]]
assert all(w['kind']=='wall' and w['width']==.2 and w['justify']=='center' and not w.get('closed',False) for w in created)
assert all(w['arch']=={'level':'kat-1'} for w in created)
assert [w['attrs']['oda'] for w in created]==['A','A','B']
assert all(w['color']=='#3B94D9' for w in created)
length=sum(math.dist(a,b) for p in paths for a,b in zip(p,p[1:]))
assert length==25
assert next(e for e in before['entities'] if e['kind']=='wall')['arch']['level']=='zemin'
same(before,read('upper-undo.kcad'))
for name in ['upper-redo.kcad','upper-repeat-refused.kcad','ground-refused.kcad','reopened.kcad']:
 same(after,read(name))
print('PASS: 3→6, 25 m upper wall network, explicit floors, original source/wall/settings, one undo/redo, refusals and reopen')
