#!/usr/bin/env python3
"""Independent Decimal column/room geometry; exact rest-of-model comparison."""
import copy
import importlib.util
import json
from decimal import Decimal as D
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]
spec = importlib.util.spec_from_file_location('kcad',ROOT/'tools/kcad/kcad.py')
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)
docs = {p.name:kcad.read(p.read_bytes())[1] for p in BASE.glob('*.kcad')}
before = docs['before.kcad']
assert len(before['entities']) == 6
assert before['settings']['drawingUnit'] == 'mm' and before['settings']['plotScale'] == 50

def changed(width,depth=2):
    expected=copy.deepcopy(before)
    expected['entities'][2].update(width=width,depth=depth)
    return expected

expected={
    'before.kcad':before,
    'before-column-suffix-problem.kcad':changed(.04),
    'before-column-undo.kcad':before,
    'after-column-400mm.kcad':changed(.4),
    'after-column-invalid.kcad':changed(.4),
    'after-column-cancel.kcad':changed(.4),
    'after-column-undo.kcad':before,
    'after-column-redo.kcad':changed(.4),
    'after-column-depth.kcad':changed(.4,.6096),
    'after-final-undo.kcad':before,
}
assert set(docs) == set(expected),set(docs)^set(expected)
for name,doc in docs.items():
    ref=copy.deepcopy(expected[name])
    col=ref['entities'][2]
    w,d=D(str(col['width'])),D(str(col['depth']))
    room=doc['entities'][4]
    assert len(room['parts']) == 1 and len(room['parts'][0]['holes']) == 1
    hole=room['parts'][0]['holes'][0]
    assert not hole.get('bulges') and len(hole['pts']) == 4
    coords=[(D(str(p['x']))-500003,D(str(p['y']))-4400004) for p in hole['pts']]
    for actual,want in zip(coords,[(w/2,-d/2),(-w/2,-d/2),(-w/2,d/2),(w/2,d/2)]):
        assert all(abs(a-b)<D('0.000000001') for a,b in zip(actual,want)),name
    area=abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(coords,coords[1:]+coords[:1])))/2
    assert abs(area-w*d)<D('0.000000001'),name
    # Same fixed shell/partition as the preceding accepted room fixture.
    net=D('4.8')*D('7.8')-w*d
    assert net>0
    ref['entities'][4]['parts'][0]['holes'][0]=hole
    assert doc==ref,(name,'unexpected model change')
    assert doc['uids']==before['uids']
events=json.loads((BASE/'gui-events.json').read_text())['events']
shots={p.name for p in (BASE/'shots').iterdir()}
assert shots=={e['file'] for e in events} and len(shots)==len(events)
for e in events:
    assert (BASE/'shots'/e['file']).read_bytes().startswith(b'\xff\xd8\xff')
    assert (BASE/e['build']).is_file()
print(f'PASS {len(docs)} GUI saves/{len(events)} raw screenshots: column/follower geometry, '
      'exact UUID/settings/rest-of-model, reject/Escape/undo/redo; .4x2 => 36.64 m², '
      '.4x.6096 => 37.19616 m². Old .04 m remains a failure reference.')
