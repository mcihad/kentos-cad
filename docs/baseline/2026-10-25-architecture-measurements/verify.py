#!/usr/bin/env python3
"""Independent KCAD decoding and Decimal geometry for real manual GUI saves."""
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
assert len(before["entities"]) == 5
assert before["settings"]["drawingUnit"] == "mm"
assert before["settings"]["plotScale"] == 50
assert before["settings"]["srid"] == 0
room_index = next(i for i, e in enumerate(before["entities"]) if e["kind"] == "space")
room = before["entities"][room_index]
assert room.get("height") is None and room["scope"] == "unassigned"

def ring_points(r):
    assert not r.get("bulges")
    return [(D(str(p["x"])) - 500000, D(str(p["y"])) - 4400000) for p in r["pts"]]

def ring_area(r):
    p = ring_points(r)
    return abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(p,p[1:]+p[:1]))) / 2

def ring_length(r):
    p = ring_points(r)
    return sum(((a[0]-b[0])**2+(a[1]-b[1])**2).sqrt() for a,b in zip(p,p[1:]+p[:1]))

area = sum(ring_area(p["outer"])-sum(ring_area(h) for h in p.get("holes",[])) for p in room["parts"])
perimeter = sum(ring_length(p["outer"])+sum(ring_length(h) for h in p.get("holes",[])) for p in room["parts"])
assert area == D("33.44") and perimeter == D("33.2")
assert area * 1_000_000 == D("33440000.00")
assert perimeter * 1000 == D("33200.000")
assert area * 3 == D("100.32")

committed = copy.deepcopy(before)
committed["entities"][room_index]["height"] = 3.0
failed = copy.deepcopy(before)
failed["entities"][room_index]["height"] = 0.003
expected = {
    "before.kcad": before,
    "before-navigation.kcad": before,
    "first-height-undo.kcad": before,
    "accepted-height-undo.kcad": before,
    "accepted-final-undo.kcad": before,
    "first-height-suffix-problem.kcad": failed,
    "accepted-height-committed.kcad": committed,
    "accepted-height-invalid.kcad": committed,
    "accepted-height-cancel.kcad": committed,
    "accepted-height-redo.kcad": committed,
    "accepted-navigation.kcad": committed,
}
assert set(docs) == set(expected)
for name, doc in docs.items():
    assert doc == expected[name], (name, "unexpected model change")
    assert len(doc["uids"]) == len(set(doc["uids"])) == 5

events = json.loads((BASE / "gui-events.json").read_text())["events"]
assert len(events) == 21 and len({e["file"] for e in events}) == 21
files = {e["file"] for e in events} | {"01-before-light-wide-18-clipped-area.jpg"}
assert files == {p.name for p in (BASE / "shots").iterdir()}
for name in files:
    assert (BASE / "shots" / name).read_bytes().startswith(b"\xff\xd8\xff"), name
for event in events:
    assert (BASE / event["build"]).is_file()
print(f"PASS {len(docs)} KCAD records: unchanged geometry/UUID/settings; 33.44 m², 33.2 m, "
      "100.32 m³ at 3 m; legacy .003 m preserved as failure; atomic reject/Escape, undo/redo; 22 raw JPEGs.")
