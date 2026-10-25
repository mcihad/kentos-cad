#!/usr/bin/env python3
"""Independent KCAD reader and translated Decimal polygon arithmetic for GUI saves.

No KentOS native extension or geometry implementation is used. The rectangular
source drawing is SDK-prepared; all saved operations listed in README are real GUI.
"""
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


def objects(doc):
    return {uid: {k: v for k, v in e.items() if k != "id"}
            for uid, e in zip(doc["uids"], doc["entities"])}


def room(doc):
    rooms = [e for e in doc["entities"] if e["kind"] == "space"]
    assert len(rooms) == 1
    return rooms[0]


def ring_area(r):
    assert not r.get("bulges"), "Only straight rings belong to this reference"
    pts = [(D(str(p["x"])) - 500000, D(str(p["y"])) - 4400000)
           for p in r["pts"]]
    return abs(sum(a[0] * b[1] - b[0] * a[1]
                   for a, b in zip(pts, pts[1:] + pts[:1]))) / 2


def area(s):
    return sum(ring_area(p["outer"]) - sum(ring_area(h) for h in p.get("holes", []))
               for p in s["parts"])


def equal(names, reference):
    for name in names:
        assert docs[name] == docs[reference], (name, reference, "model differs")


before = docs["before.kcad"]
old = objects(before)
assert len(old) == 4 and before["settings"]["drawingUnit"] == "mm"
assert before["settings"]["plotScale"] == 50
assert before["activeLayer"] == "mahal" and before["origin"] == {"x": 0., "y": 0.}
levels = before["settings"]["arch"]["levels"]
assert [v["id"] for v in levels] == ["zemin", "kat-1"]
assert levels[0]["elevation"] == levels[1]["elevation"] == 0
for name, doc in docs.items():
    for key in before:
        if key not in ("entities", "uids", "nextId"):
            assert doc[key] == before[key], (name, key, "project metadata changed")
    assert len(doc["uids"]) == len(set(doc["uids"])) == len(doc["entities"])

equal(["before-cancel.kcad", "first-single-undo.kcad", "first-bulk-preview.kcad",
       "first-bulk-undo.kcad", "first-bulk-noop-undo.kcad"], "before.kcad")
equal(["first-single-redo.kcad", "first-single-scope-undo.kcad"], "first-single-created.kcad")
equal(["first-bulk-redo.kcad", "first-bulk-noop.kcad", "first-bulk-noop-redo.kcad",
       "first-bulk-unsafe-scope-refused.kcad", "first-wall-level-undo.kcad"], "first-bulk-created.kcad")
equal(["first-reseed-preview.kcad", "first-reseed-undo.kcad", "first-wall-level-redo.kcad"],
      "first-wall-unassigned-lost.kcad")
equal(["first-reseed-cancel.kcad", "first-reseed-redo.kcad", "accepted-reopened.kcad",
       "accepted-navigation-cancel.kcad", "accepted-right-undo.kcad", "accepted-reseed-cancel.kcad"],
      "first-reseed-confirmed.kcad")
equal(["accepted-right-redo.kcad"], "accepted-right-confirmed.kcad")

single = docs["first-single-created.kcad"]
project = docs["first-single-project.kcad"]
bulk = docs["first-bulk-created.kcad"]
lost = docs["first-wall-unassigned-lost.kcad"]
healed = docs["first-reseed-confirmed.kcad"]
right = docs["accepted-right-confirmed.kcad"]
single_uid = next(u for u in objects(single) if u not in old)
bulk_uid = next(u for u in objects(bulk) if u not in old)
assert single_uid != bulk_uid, "Undo then new creation is a new identity"
assert set(objects(project)) == set(objects(single))
for name in ("first-wall-unassigned-lost.kcad", "first-reseed-confirmed.kcad",
             "accepted-right-confirmed.kcad"):
    assert set(objects(docs[name])) == set(objects(bulk)), name

for doc, target, scope, holes in ((single, "72.44", "unassigned", 1),
                                 (bulk, "72.44", "unassigned", 1),
                                 (project, "33.44", None, 1),
                                 (lost, "72.44", "unassigned", 1),
                                 (healed, "33.44", "unassigned", 1),
                                 (right, "37.44", "unassigned", 0)):
    s = room(doc)
    assert abs(area(s) - D(target)) < D(".000001"), (area(s), target)
    assert s.get("scope") == scope and s["columns"] == "net"
    assert s.get("method", "net") == "net" and not s.get("arch", {}).get("level")
    assert len(s["parts"]) == 1 and len(s["parts"][0].get("holes", [])) == holes
    if holes:
        assert ring_area(s["parts"][0]["holes"][0]) == 4, "Wrong column deducted"

wall_uid = before["uids"][1]
wall_without_level = copy.deepcopy(old[wall_uid])
wall_without_level.pop("arch")
for doc in (single, project, bulk):
    for uid, e in old.items():
        assert objects(doc)[uid] == e, (uid, "source changed")
for doc in (lost, healed, right):
    for uid, e in old.items():
        assert objects(doc)[uid] == (wall_without_level if uid == wall_uid else e), uid
assert objects(lost)[bulk_uid] == dict(objects(bulk)[bulk_uid], broken="lostEnclosure")
for doc in (healed, right):
    s = objects(doc)[bulk_uid]
    original = objects(bulk)[bulk_uid]
    assert {k: v for k, v in s.items() if k not in ("parts", "seed")} == {
        k: v for k, v in original.items() if k not in ("parts", "seed")}
    assert not s.get("broken")
assert room(bulk)["seed"] == {"x": 500005., "y": 4400004.}
assert 500000.1 < room(healed)["seed"]["x"] < 500004.9
assert 500005.1 < room(right)["seed"]["x"] < 500009.9

events = json.loads((BASE / "gui-events.json").read_text())["events"]
assert len(events) == 56 and len({e["file"] for e in events}) == 56
for e in events:
    assert (BASE / "shots" / e["file"]).read_bytes().startswith(b"\xff\xd8\xff"), e
old_reader = json.loads((BASE / "old-reader-refusal.json").read_text())
assert old_reader["status"] == "refused" and "104" in old_reader["message"]
print(f"PASS {len(docs)} real GUI saves; 33.44/72.44/37.44 m²; scoped column hole; atomic refusal; "
      "source level/follower, single undo/redo, preview/cancel/noop/reopen, UUID/metadata; 56 raw JPEGs.")
