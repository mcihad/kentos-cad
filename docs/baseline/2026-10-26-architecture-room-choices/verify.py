"""Compare COMPLETE native saves with independent expectations and raw evidence hashes."""
import copy
import hashlib
import importlib.util
import json
import struct
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


seed = json.loads((B / "seed.json").read_text())
before = read("before.kcad")
assert sha(B / "before.kcad") == seed["beforeSha256"]
assert (B / "before.kcad").read_bytes() == (B.parent / "2026-10-26-architecture-space-identity/numbered.kcad").read_bytes()
assert len(before["entities"]) == len(before["uids"]) == 10
assert before["settings"]["drawingUnit"] == "m"
assert before["settings"]["plotScale"] == 50.
for i, name in enumerate(["Ana giriş", "Çalışma", "Mutfak", "Yatak odası", "Depo"]):
    source, tag = before["entities"][i], before["entities"][i + 5]
    assert source["name"] == name and source["usage"] == "Konut"
    assert source.get("number") == ("N-001" if i == 0 else None if i < 3 else "Z-001")
    assert source["parts"] == [{"outer": {"pts": [
        dict(x=i * 5., y=0.), dict(x=i * 5. + 4., y=0.),
        dict(x=i * 5. + 4., y=3.), dict(x=i * 5., y=3.),
    ]}}]
    assert tag["of"] == before["uids"][i]
    assert tag["anchor"] == tag["at"] == dict(x=i * 5. + 2., y=1.5)
    assert tag["text"] == f'{source.get("number", "")}\n{name}\n12.00 m²'
assert before["entities"][4]["layerId"] == "gizli"
assert next(l for l in before["layers"] if l["id"] == "gizli")["visible"] is False

prefs = copy.deepcopy(before)
prefs["settings"]["arch"].update(
    spaceNames=["Mutfak", "Çalışma", "mutfak"],
    spaceUsages=["Konut", "Ofis"],
    spaceNumberPrefixes={"zemin": "G-", "kat-1": "01-"},
)
named = copy.deepcopy(prefs)
named["entities"][0]["name"] = "Mutfak"
named["entities"][5]["text"] = "N-001\nMutfak\n12.00 m²"
used = copy.deepcopy(named)
used["entities"][0]["usage"] = "Ofis"
created = copy.deepcopy(prefs)
created["uids"].append(seed["newRoomUid"])
created["entities"].append(dict(
    kind="space", id=11, attrs={}, layerId="mahal",
    parts=[{"outer": {"pts": [dict(x=0., y=5.), dict(x=4., y=5.),
                               dict(x=4., y=8.), dict(x=0., y=8.)]}}],
    arch={"level": "zemin"}, name="Özel oda", usage="Ofis", number="G-001",
))
numbered = copy.deepcopy(created)
numbered["entities"][0]["number"] = "G-001"
numbered["entities"][5]["text"] = "G-001\nAna giriş\n12.00 m²"
numbered["entities"][10]["number"] = "G-002"

expected = {}
for exemplar, model, names in [
    ("preferences.kcad", prefs, ["preferences", "cancelled-settings", "choices-undo", "choices-reset", "preview", "created-undo"]),
    ("name-picked.kcad", named, ["name-picked", "usage-undo"]),
    ("usage-picked.kcad", used, ["usage-picked", "choices-redo"]),
    ("created.kcad", created, ["created", "created-redo", "number-refused", "number-preview", "number-undo"]),
    ("numbered.kcad", numbered, ["numbered", "number-redo", "reopened", "light18-cancel", "partial-cancel", "light18-wide", "final-save", "validation-cancel"]),
]:
    for name in names:
        expected[name + ".kcad"] = model, exemplar
records = json.loads((B / "models.json").read_text())
assert set(expected) == {r["file"] for r in records}
assert len(records) == len(expected) == 23
for record in records:
    name = record["file"]
    model, exemplar = expected[name]
    assert sha(B / name) == record["sha256"], name
    assert read(name) == model, name
    assert (B / name).read_bytes() == (B / exemplar).read_bytes(), name

events = json.loads((B / "gui-events.json").read_text())
assert len(events) == len({e["file"] for e in events}) == 42
dimensions = {}
for event in events:
    assert sha(B / event["file"]) == event["sha256"]
    build = json.loads((B / event["build"]).read_text())
    assert sha(Path(build["app"]) / build["executable"]) == build["sha256"]
    dimensions[event["file"]] = jpeg_size(B / event["file"])
assert dimensions["02-after-project-settings.jpg"] == [2880, 1720]
assert dimensions["28-dark13-compact-prefixes.jpg"] == [2200, 1364]
assert dimensions["30-light18-compact-prefixes.jpg"] == [2200, 1364]
assert dimensions["35-light18-wide-prefixes.jpg"] == [2880, 1720]
assert "Tanısal" in events[16]["note"]  # Initial active-tool Undo attempt is not a successful document Undo.
final_sources = json.loads((B / "after-validation-binary.json").read_text())["sourceFiles"]
for file, digest in final_sources.items():
    assert sha(R / file) == digest, file
# The last actual GUI repetition fixed only list-local error presentation in arch.rs.
# Earlier raw captures keep their original immutable binaries/source manifests.
for name in ["after-binary.json", "after-final-binary.json"]:
    earlier = json.loads((B / name).read_text())["sourceFiles"]
    assert {file for file, digest in earlier.items() if digest != final_sources[file]} == {
        "apps/desktop/src/project/arch.rs"
    }, name
for record in json.loads((B / "test-runs.json").read_text()):
    assert sha(B / record["log"]) == record["sha256"]
for name, theme, size in [("dark13-profile.json", "dark", 13), ("light18-profile.json", "light", 18)]:
    profile = json.loads((B / name).read_text())["user"]
    assert profile["appearance.theme"] == theme
    assert profile["appearance.textSize"] == size
audit = json.loads((B / "contract-audit.json").read_text())
assert sha(R / "apps/web/src/contracts/generated/commandCatalog.json") == audit["catalogSha256"]
assert sha(B / "room-choices-gui.dxf") == seed["dxfSha256"]
(B / "image-dimensions.json").write_text(json.dumps(dimensions, indent=2) + "\n")
print("PASS: 23 complete native saves, 42 raw images, binary/source/profile/log hashes; only explicit preferences, source fields, tag text and the one new measured room change. Final inline validation preserves the complete model.")
