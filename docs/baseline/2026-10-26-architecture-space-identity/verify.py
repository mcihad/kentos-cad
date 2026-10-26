"""Compare complete native saves using the independent KCAD reader, not the host."""
import copy
import hashlib
import importlib.util
import json
import struct
import subprocess
from pathlib import Path

B = Path(__file__).resolve().parent
R = B.parents[2]
spec = importlib.util.spec_from_file_location("reader", R / "tools/kcad/kcad.py")
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)


def read(name):
    return reader.read((B / name).read_bytes())[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def jpeg_size(path):
    """Read raw JPEG dimensions without resizing or depending on a renderer."""
    data = path.read_bytes()
    assert data[:2] == b"\xff\xd8", path
    i = 2
    while i < len(data):
        assert data[i] == 255
        while data[i] == 255:
            i += 1
        marker = data[i]
        i += 1
        length = struct.unpack_from(">H", data, i)[0]
        if marker in (192, 193, 194):
            height, width = struct.unpack_from(">HH", data, i + 3)
            return [width, height]
        i += length
    raise AssertionError(path)


before = read("before.kcad")
seed = json.loads((B / "seed.json").read_text())
assert len(before["entities"]) == 10
assert before["uids"][:5] == seed["rooms"]
assert before["uids"][:3] == seed["expectedMissing"]
assert before["uids"][3:5] == seed["expectedDuplicate"]
for i, name in enumerate(["Ana giriş", "Çalışma", "Mutfak", "Yatak odası", "Depo"]):
    source, tag = before["entities"][i], before["entities"][i + 5]
    assert source["name"] == name
    assert source.get("number") == (None if i < 3 else "Z-001")
    points = source["parts"][0]["outer"]["pts"]
    assert points == [dict(x=i * 5., y=0.), dict(x=i * 5. + 4., y=0.),
                      dict(x=i * 5. + 4., y=3.), dict(x=i * 5., y=3.)]
    area = abs(sum(a["x"] * b["y"] - b["x"] * a["y"]
                   for a, b in zip(points, points[1:] + points[:1]))) / 2
    assert area == 12.
    assert tag["of"] == seed["rooms"][i]
    assert tag["anchor"] == tag["at"] == dict(x=i * 5. + 2., y=1.5)
    assert tag["text"] == f'{source.get("number", "")}\n{name}\n12.00 m²'
assert before["entities"][4]["layerId"] == "gizli"
assert next(l for l in before["layers"] if l["id"] == "gizli")["visible"] is False
assert "arch" not in before["entities"][2]
assert before["settings"]["drawingUnit"] == "m"
assert before["settings"]["plotScale"] == 50.

numbered = copy.deepcopy(before)
numbered["entities"][0]["number"] = "N-001"
numbered["entities"][5]["text"] = "N-001\nAna giriş\n12.00 m²"
expected = {name: before for name in ["before-report.kcad", "reviewed.kcad", "shown.kcad", "undo.kcad", "cleared.kcad"]}
expected.update({name: numbered for name in ["numbered.kcad", "redo.kcad", "clear-undo.kcad", "cancelled.kcad", "compact.kcad", "dark.kcad", "reopened.kcad"]})
records = json.loads((B / "models.json").read_text())
assert len(records) == len(expected)
assert set(expected) == {r["file"] for r in records}
for record in records:
    name = record["file"]
    assert sha(B / name) == record["sha256"], name
    assert read(name) == expected[name], name
    # These actions must preserve the complete file, including old identities,
    # all geometry, settings, layers, tag placement and unrelated sources.
    same_bytes = "before.kcad" if expected[name] is before else "numbered.kcad"
    assert (B / name).read_bytes() == (B / same_bytes).read_bytes(), name

events = json.loads((B / "gui-events.json").read_text())
assert len({e["file"] for e in events}) == len(events)
dimensions = {}
for event in events:
    assert sha(B / event["file"]) == event["sha256"]
    build = json.loads((B / event["build"]).read_text())
    assert sha(Path(build["app"]) / build["executable"]) == build["sha256"]
    dimensions[event["file"]] = jpeg_size(B / event["file"])
assert dimensions["02-after-missing-found.jpg"] == [2880, 1720]
assert dimensions["14-light18-compact.jpg"] == [2200, 1364]
assert dimensions["16-dark13-wide.jpg"] == [2880, 1720]
assert dimensions["17-dark13-compact.jpg"] == [2200, 1364]
for file, digest in json.loads((B / "after-binary.json").read_text())["sourceFiles"].items():
    assert sha(R / file) == digest, file
audit = json.loads((B / "contract-audit.json").read_text())
catalog = "apps/web/src/contracts/generated/commandCatalog.json"
assert sha(R / catalog) == audit["catalogSha256"]
old = json.loads(subprocess.check_output(["git", "show", f'{audit["parentCommit"]}:{catalog}'], cwd=R))
new = json.loads((R / catalog).read_text())
assert [c["id"] for c in old["commands"]] == [c["id"] for c in new["commands"]]
changed = [(a, b) for a, b in zip(old["commands"], new["commands"]) if a != b]
assert len(changed) == 1 and changed[0][0]["id"] == "cad.tags.check"


def without_descriptions(v):
    if isinstance(v, dict):
        return {k: without_descriptions(s) for k, s in v.items() if k != "description"}
    if isinstance(v, list):
        return [without_descriptions(s) for s in v]
    return v


a, b = changed[0][0], copy.deepcopy(changed[0][1])
assert b["input"]["properties"].pop("missingSpaceNumbers")["type"] == ["boolean", "null"]
b["summary"] = a["summary"]
assert without_descriptions(a) == without_descriptions(b)
assert [c for c in new["codes"] if c not in old["codes"]] == audit["addedCodes"]
assert [c["code"] for c in audit["addedCodes"]] == ["tag_missing_space_number"]
assert all(c in new["codes"] for c in old["codes"])
assert {k: v for k, v in old.items() if k not in ("commands", "codes")} == {k: v for k, v in new.items() if k not in ("commands", "codes")}
for record in json.loads((B / "test-runs.json").read_text()):
    assert sha(B / record["log"]) == record["sha256"]
for name, theme, size in [("dark13-profile.json", "dark", 13), ("light18-profile.json", "light", 18)]:
    profile = json.loads((B / name).read_text())["user"]
    assert profile["appearance.theme"] == theme
    assert profile["appearance.textSize"] == size
(B / "image-dimensions.json").write_text(json.dumps(dimensions, indent=2) + "\n")
print(f"PASS: {len(records)} complete native saves are byte-identical to independent expectations; only the explicit source number and linked tag text change. {len(events)} raw images, binary/source/log hashes and actual appearance profiles verified.")
