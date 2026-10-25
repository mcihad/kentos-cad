#!/usr/bin/env python3
"""Read actual GUI saves with the independent KCAD specification reader."""
import importlib.util
from pathlib import Path
root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('kcad', root/'tools/kcad/kcad.py')
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)
base = Path(__file__).parent
def read(p): return kcad.read(p.read_bytes())[1]
prior = read(base.parent/'2026-10-25-architecture-opening-arrays'/'final-one-copy.kcad')
undo = read(base/'dark18-undo-twelve.kcad')
redo = read(base/'dark18-k10-thirteen.kcad')
edit = read(base/'dark18-k10-edited.kcad')
reopened = read(base/'dark18-k10-reopened.kcad')
assert len(prior['entities']) == len(undo['entities']) == 12
for key in ['settings','entities','uids']: assert prior[key] == undo[key], key
assert len(redo['entities']) == len(edit['entities']) == len(reopened['entities']) == 13
for d in [redo, edit, reopened]:
    assert d['settings'] == prior['settings']
    assert d['uids'][:12] == prior['uids']
    assert d['uids'] == redo['uids']
    for i in range(12):
        if i != 4: assert d['entities'][i] == prior['entities'][i]
    o = d['entities'][-1]
    assert o['at'] == 16 # source centre 1 + width 1 + clear gap 14
    assert o['number'] == 'K10'
assert redo['entities'][-1]['width'] == reopened['entities'][-1]['width'] == 1
assert edit['entities'][-1]['width'] == 1.1
assert redo['entities'][4]['cells'][-1] == reopened['entities'][4]['cells'][-1] == ['K10','Kapı','1.000']
assert edit['entities'][4]['cells'][-1] == ['K10','Kapı','1.100']
for key in ['settings','entities','uids']: assert redo[key] == reopened[key], key
print('Dark18 GUI: single undo/redo, K10 centre/width, live schedule, prior objects/UUID/settings and edit undo/reopen preserved')

fit = read(base/'final-labels-fit-fourteen.kcad')
back = read(base/'final-labels-undo-thirteen.kcad')
assert len(fit['entities']) == 14
assert fit['settings'] == redo['settings']
assert fit['uids'][:13] == redo['uids']
for i in range(13):
    if i != 4: assert fit['entities'][i] == redo['entities'][i]
o = fit['entities'][-1]
assert o['at'] == 18 and o['width'] == 1 and 'number' not in o
for key in ['settings','entities','uids']: assert back[key] == redo[key], key
print('Final compact-label GUI: fit midpoint 18 m, one apply, complete undo preserved the thirteen prior objects')
