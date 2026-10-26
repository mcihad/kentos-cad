#!/usr/bin/env python3
"""Read real GUI saves; independent Decimal rectangle and jamb checks, no app calls."""
import copy
import importlib.util
import json
from decimal import Decimal as D
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]
spec = importlib.util.spec_from_file_location("kcad", ROOT / "tools/kcad/kcad.py")
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)
docs = {p.name: kcad.read(p.read_bytes())[1] for p in BASE.glob("*.kcad")}
before = docs["before.kcad"]
created = docs["before-door-created.kcad"]
assert len(before["entities"]) == 5 and len(created["entities"]) == 6
assert created["entities"][:5] == before["entities"]
assert created["uids"][:5] == before["uids"]
without = copy.deepcopy(created)
without["entities"].pop()
without["uids"].pop()
assert without == before
door = created["entities"][5]
assert door["kind"] == "opening" and door["width"] == .9
assert door["host"] == created["uids"][0]
assert before["settings"]["drawingUnit"] == "mm"
assert before["settings"]["plotScale"] == 50 and before["settings"]["srid"] == 0

def area(ring):
    pts = [(D(str(p["x"]))-500000, D(str(p["y"]))-4400000) for p in ring["pts"]]
    return abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(pts,pts[1:]+pts[:1]))) / 2

def wall_expected(base, width, offset=0, height=None):
    doc = copy.deepcopy(base)
    wall, room = doc["entities"][0], doc["entities"][4]
    wall["width"] = float(width)
    if offset:
        wall["offset"] = float(offset)
    if height is not None:
        wall["height"] = height
    inset = D(str(width))/2 + D(str(offset))
    # Closed 10x8 centreline shell, fixed partition at x=5 (200 mm).
    ring = room["parts"][0]["outer"]
    ring["pts"] = [{"x":float(500000+x),"y":float(4400000+y)} for x,y in
                   [(D('4.9'),8-inset),(inset,8-inset),(inset,inset),(D('4.9'),inset)]]
    actual = area(ring) - sum(area(h) for h in room["parts"][0]["holes"])
    assert actual == (D('4.9')-inset)*(8-2*inset)-4
    return doc

def opening_expected(base, **fields):
    doc = copy.deepcopy(base)
    doc["entities"][5].update(fields)
    return doc

wall250 = wall_expected(created,.25)
door1200 = opening_expected(created,width=1.2)
imperial = opening_expected(door1200,at=2.4384,frame=.0508,head=2.1336)
window = opening_expected(imperial,form='window')
sill = opening_expected(window,sill=.9)
expected = {
    "before.kcad": before,
    "before-wall-suffix-problem.kcad": wall_expected(before,.025),
    "before-wall-undo.kcad": before,
    "before-door-created.kcad": created,
    "before-door-80cm-rejected.kcad": created,
    "before-door-creation-undo.kcad": before,
    "before-door-120cm.kcad": opening_expected(created,width=.12),
    "before-door-120cm-undo.kcad": created,
    "after-wall-250mm.kcad": wall250,
    "after-wall-invalid.kcad": wall250,
    "after-wall-undo.kcad": created,
    "after-wall-redo.kcad": wall250,
    "after-wall-offset.kcad": wall_expected(created,.25,-.05),
    "after-wall-height.kcad": wall_expected(created,.25,height=3),
    "after-wall-all-undone.kcad": created,
    "after-door-1200mm.kcad": door1200,
    "after-door-invalid.kcad": door1200,
    "after-door-cancel.kcad": door1200,
    "after-door-undo.kcad": created,
    "after-door-redo.kcad": door1200,
    "after-door-imperial.kcad": imperial,
    "after-window-sill.kcad": sill,
    "after-window-sill-cleared.kcad": window,
    "after-window-sill-undo.kcad": sill,
    "after-window-all-undone.kcad": imperial,
    "after-final-undo.kcad": created,
}
assert set(docs) == set(expected)
for name, doc in docs.items():
    # Derived jamb points are calculated independently and tolerate only float rounding.
    reference = copy.deepcopy(expected[name])
    if len(doc["entities"]) == 6:
        opening, host = doc["entities"][5], reference["entities"][0]
        left = D(str(opening["at"]))-D(str(opening["width"]))/2
        right = left+D(str(opening["width"]))
        off = D(str(host.get("offset",0)))
        half = D(str(host["width"]))/2
        xy = [(left,off+half),(left,off-half),(right,off+half),(right,off-half)]
        assert len(opening["place"]) == 4
        for point,(x,y) in zip(opening["place"],xy):
            assert abs(D(str(point["x"]))-500000-x) < D('0.000000001'), name
            assert abs(D(str(point["y"]))-4400000-y) < D('0.000000001'), name
        reference["entities"][5]["place"] = opening["place"]
    assert doc == reference, (name,"unexpected model change")
    assert len(doc["uids"]) == len(set(doc["uids"])) == len(doc["entities"])
events = json.loads((BASE/'gui-events.json').read_text())["events"]
shots = {p.name for p in (BASE/'shots').iterdir()}
assert len(events) == 29 and {e['file'] for e in events} == shots
for event in events:
    assert (BASE/'shots'/event['file']).read_bytes().startswith(b'\xff\xd8\xff')
    assert (BASE/event['build']).is_file()
print('PASS: 26 real GUI saves, 29 raw JPEGs; exact model/UUID/settings, independent Decimal '
      'room rectangle minus column and jamb positions; metric/imperial, reject/Escape, '
      'optional clear, undo/redo. Old wrong .025 m wall/.12 m door preserved as failures.')
