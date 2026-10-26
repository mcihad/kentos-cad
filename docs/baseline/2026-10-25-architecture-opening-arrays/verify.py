#!/usr/bin/env python3
"""Independent saved-GUI acceptance. Reads only the KCAD specification reader."""
import importlib.util
import math
from pathlib import Path
root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("kcad", root / "tools/kcad/kcad.py")
kcad = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kcad)
base = Path(__file__).parent
def read(name):
    return kcad.read((base/name).read_bytes())[1]
a, b, c = map(read, ["interactive-before.kcad", "straight-three.kcad", "straight-and-arc.kcad"])
assert a["settings"] == b["settings"] == c["settings"]
assert a["uids"] == b["uids"][:5] == c["uids"][:5]
assert b["uids"] == c["uids"][:8]
for k in (0,1,2,3):
    assert a["entities"][k] == b["entities"][k] == c["entities"][k]
assert b["entities"][5:8] == c["entities"][5:8]
straight = b["entities"][5:8]
assert [o["at"] for o in straight] == [3,5,7]
assert [o["number"] for o in straight] == ["K2","K4","K6"]
assert [row[0] for row in b["entities"][4]["cells"]][1:] == ["K1","P1","K2","K4","K6"]
centres = [2.5, (1+10*math.pi)/2, 10*math.pi-1.5]
for e, at in zip(c["entities"][8:], centres):
    assert e["width"] == 1
    assert "number" not in e
    assert abs(e["at"]-at) < 1e-12
    for i,(s,r) in enumerate([(at-.5,9.9),(at-.5,10.1),(at+.5,9.9),(at+.5,10.1)]):
        angle = math.pi+s/10
        assert abs(e["place"][i]["x"]-(10+r*math.cos(angle))) < 1e-12
        assert abs(e["place"][i]["y"]-(20+r*math.sin(angle))) < 1e-12
print("Independent source/UUID/settings/straight/analytic arc/schedule checks passed")

last = read("final-one-copy.kcad")
assert len(last["entities"]) == 12
assert last["settings"] == c["settings"]
assert last["uids"][:11] == c["uids"]
for i in range(11):
    if i != 4: assert last["entities"][i] == c["entities"][i]
assert last["entities"][-1]["at"] == 12
assert last["entities"][-1]["width"] == 1
print("Final GUI copy/edit/undo/navigation preserved prior geometry and settings")
