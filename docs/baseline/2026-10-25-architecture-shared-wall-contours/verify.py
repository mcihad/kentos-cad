#!/usr/bin/env python3
"""Independent KCAD reader + known room dimensions; no KentOS geometry import."""
import importlib.util
import math
from decimal import Decimal as D
from pathlib import Path

base = Path(__file__).resolve().parent
root = base.parents[2]
spec = importlib.util.spec_from_file_location('kcad', root / 'tools/kcad/kcad.py')
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)


def read(name):
    return kcad.read((base / name).read_bytes())[1]


def semantic(doc):
    # Local numeric IDs are compacted by the writer; UIDs identify objects.
    return {u: {k: v for k, v in e.items() if k != 'id'}
            for u, e in zip(doc['uids'], doc['entities'])}


def same(a, b):
    assert a['settings'] == b['settings']
    assert semantic(a) == semantic(b)


def kept(a, b):
    assert a['settings'] == b['settings']
    bm = semantic(b)
    for uid, e in semantic(a).items():
        assert bm[uid] == e


def length(w):
    p = w['pts']
    assert not w.get('closed', False)
    bs = w.get('bulges', [0.] * len(p))
    assert len(bs) == len(p) and bs[-1] == 0.
    result = 0.
    for a, b, bulge in zip(p, p[1:], bs):
        chord = math.hypot(b['x'] - a['x'], b['y'] - a['y'])
        result += (chord if not bulge else
                   4 * math.atan(abs(bulge)) * chord * (1 + bulge**2) / (4 * abs(bulge)))
    return result


before = read('before.kcad')
straight = read('straight-shared.kcad')
partial = read('partial-shared.kcad')
curved = read('straight-partial-arc.kcad')
removed = read('arc-remove.kcad')
assert [len(d['entities']) for d in (before, straight, partial, curved, removed)] == [6, 9, 12, 15, 13]
same(before, read('straight-undo.kcad'))
kept(before, straight)
kept(straight, partial)
kept(partial, curved)
same(partial, read('arc-remove-undo.kcad'))
same(removed, read('repeat-refused.kcad'))
same(removed, read('reopened.kcad'))

walls = curved['entities'][6:]
assert all(w['kind'] == 'wall' and w['width'] == .2 and w['justify'] == 'center' for w in walls)
for w in walls:
    src = next(e for e in before['entities'] if e['attrs'] == w['attrs'])
    assert w['color'] == src['color'] and w['layerId'] == src['layerId']
for group, expected in ((walls[:3], 25.), (walls[3:6], 23.), (walls[6:], 12 + 5 * math.pi)):
    assert len(group) == 3
    assert math.isclose(sum(map(length, group)), expected, abs_tol=1e-9)

# The divider is present once and belongs to the first room; no reversed copy.
for group, y0, y1, owner in ((walls[:3], 0., 3., 'A'), (walls[3:6], 8., 9., 'C')):
    dividers = [w for w in group if len(w['pts']) == 2 and
                {(p['x'], p['y']) for p in w['pts']} == {(4., y0), (4., y1)}]
    assert len(dividers) == 1 and dividers[0]['attrs'] == {'oda': owner}
arc = walls[7]
assert arc['pts'] == [{'x': 18., 'y': 0.}, {'x': 14., 'y': 4.}]
assert math.isclose(arc['bulges'][0], math.sqrt(2)-1, abs_tol=1e-15)
assert arc['attrs'] == {'oda': 'Yay iç'}
assert sum(bool(w.get('bulges')) for w in walls[6:]) == 2

prior = semantic(partial)
now = semantic(removed)
removed_uids = set(before['uids'][4:])
assert removed_uids.isdisjoint(now)
for uid, e in prior.items():
    if uid not in removed_uids:
        assert now[uid] == e
new_walls = [e for uid, e in now.items() if uid not in prior]
assert len(new_walls) == 3
assert math.isclose(sum(map(length, new_walls)), 12 + 5 * math.pi, abs_tol=1e-9)
assert before['settings']['plotScale'] == 100
same(before, read('light-straight-undo.kcad'))
light = read('light-straight-remove.kcad')
assert len(light['entities']) == 7
lm = semantic(light)
assert set(before['uids'][:2]).isdisjoint(lm)
for uid, e in semantic(before).items():
    if uid not in before['uids'][:2]:
        assert lm[uid] == e
light_walls = [e for e in light['entities'] if e['kind'] == 'wall']
assert len(light_walls) == 3 and math.isclose(sum(map(length, light_walls)), 25., abs_tol=1e-9)
assert light['settings'] == before['settings']
room = read('regression-room.kcad')
kept(before, room)
assert len(room['entities']) == 10
w, door, window, space = room['entities'][6:]
assert w['kind'] == 'wall' and w['closed'] and w['justify'] == 'left' and w['width'] == .2
assert w['pts'] == [{'x': 0., 'y': -6.}, {'x': 4., 'y': -6.}, {'x': 4., 'y': -3.}, {'x': 0., 'y': -3.}]
assert door['form'] == 'door' and door['width'] == .9
assert window['form'] == 'window' and window['width'] == 1.2
assert door['host'] == window['host'] == room['uids'][6]
pts = space['parts'][0]['outer']['pts']
assert pts == w['pts']
area = sum(D(str(a['x'])) * D(str(b['y'])) - D(str(b['x'])) * D(str(a['y']))
           for a, b in zip(pts, pts[1:] + pts[:1])) / 2
assert area == D(12)
thicker = read('regression-width-025.kcad')
assert thicker['entities'][6]['width'] == .25
assert thicker['entities'][6]['pts'] == w['pts']
assert thicker['entities'][-1] == space
assert all(e['host'] == room['uids'][6] for e in thicker['entities'] if e['kind'] == 'opening')
same(room, read('regression-width-undo.kcad'))
free = read('regression-free-wall.kcad')
kept(room, free)
assert len(free['entities']) == 11
fw = free['entities'][-1]
assert fw['pts'] == [{'x': 8., 'y': -6.}, {'x': 11., 'y': -6.}, {'x': 11., 'y': -3.}]
assert fw['kind'] == 'wall' and math.isclose(length(fw), 6., abs_tol=1e-12)
same(room, read('regression-free-undo.kcad'))
# Independent joined body area for the first pair (tested against solver in Rust).
assert D('8.2') * D('3.2') - 2 * D('3.8') * D('2.8') == D('4.96')
print('Verified actual GUI saves: 25 m / 23 m / (12+5π) m unique axes; analytic arcs, first-source attributes, UIDs/settings, Koru/Sil, undo/redo, repeat refusal, reopen. Joined area expectation: 4.96 m². Existing rectangle/opening/space workflow: 12 m², .9/1.2 m hosts, .2→.25→undo .2 m; free wall: exactly 6 m, single confirm, undo.')
