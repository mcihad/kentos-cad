#!/usr/bin/env python3
"""Read real GUI saves independently; no SDK or product geometry calculations."""
import importlib.util
import json
from decimal import Decimal as D
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]
spec = importlib.util.spec_from_file_location('kcad', ROOT / 'tools/kcad/kcad.py')
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)

def read(name):
    return kcad.read((BASE / name).read_bytes())[1]

def objects(doc):
    return {u: {k: v for k, v in e.items() if k != 'id'}
            for u, e in zip(doc['uids'], doc['entities'])}

def same(a, b):
    assert a['settings'] == b['settings']
    assert objects(a) == objects(b)

def area(ring):
    assert not ring.get('bulges'), 'These independent GUI fixtures have straight edges.'
    pts = [(D(str(p['x'])) - 500000, D(str(p['y'])) - 4400000) for p in ring['pts']]
    return abs(sum(a[0]*b[1] - b[0]*a[1] for a, b in zip(pts, pts[1:] + pts[:1]))) / 2

def rooms(base, doc, count, expected, level):
    assert doc['settings'] == base['settings']
    old, new = objects(base), objects(doc)
    for uid, entity in old.items():
        assert new[uid] == entity, (uid, 'source changed')
    extras = {u: e for u, e in new.items() if u not in old}
    assert len(extras) == count
    for room in extras.values():
        assert room['kind'] == 'space' and not room.get('broken')
        assert room.get('arch', {}).get('level') == level
        assert room.get('method', 'net') == 'net'
        measured = sum(area(p['outer']) - sum(area(h) for h in p.get('holes', []))
                       for p in room['parts'])
        assert abs(measured - D(expected)) < D('0.00000001'), measured
    return extras

before = read('before.kcad')
assert len(before['entities']) == 4
ground = read('ground-two.kcad')
rooms(before, ground, 2, '37.44', 'zemin')
same(before, read('ground-undo.kcad'))
same(ground, read('ground-redo.kcad'))
same(ground, read('ground-repeat.kcad'))
same(before, read('repeat-undo.kcad'))
chosen = rooms(before, read('region-one.kcad'), 1, '37.44', 'zemin')
assert next(iter(chosen.values()))['seed'] == {'x': 500007.5, 'y': 4400004.0}
conflict = read('conflict-before.kcad')
assert len(conflict['entities']) == 5
same(conflict, read('conflict-after.kcad'))
grid = read('grid-before.kcad')
assert len(grid['entities']) == 39
created = read('grid-created.kcad')
rooms(grid, created, 400, '14.44', None)
same(grid, read('grid-undo.kcad'))
same(created, read('grid-redo.kcad'))
final = read('final-created.kcad')
rooms(before, final, 2, '37.44', 'zemin')
same(before, read('final-undo.kcad'))
same(final, read('final-redo.kcad'))
same(final, read('final-narrow-cancel.kcad'))
same(final, read('final-reopened.kcad'))
rooms(final, read('delivery-upper.kcad'), 1, '115.64', 'kat-1')
same(final, read('delivery-upper-undo.kcad'))
same(final, read('delivery-catalog-cancel.kcad'))
for event in json.loads((BASE / 'gui-events.json').read_text())['events']:
    if 'file' in event:
        assert (BASE / 'shots' / event['file']).is_file(), event
print('PASS: 2×37.44 m²; same UUID repeat/no empty undo; region keeps whole right room; conflict writes nothing; 400×14.44 m²; single undo/redo; all original source/settings content preserved.')
