#!/usr/bin/env python3
"""The shared cases of what a `project.processing` server job decides without a database (docs/adr/0559, 0561 §1): the
request checked when it is made, the memory a project's file is estimated to need, the budget a request may ask for,
and the delta, what a tool (or an import) changed, found by comparing the drawing as it was opened with the drawing
after the run: objects by their persistent ids, block definitions by theirs.

Written from the ADR's rules and plain Python, not from KentOS code: the delta of a case is worked out here from a
compact description of the two drawings (objects by persistent id with a layer, a place and attributes; the layer
tree's ids; the project's name, plot scale, active layer and its block definitions as (id, name)) and the Rust test
(crates/server/application/tests/processing_job_cases.rs) builds the two documents the description names and compares
what its function answers.

    python3 scripts/fixtures/processing_job_cases.py [--check]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/server-jobs/v1/processing.json"

MAX_CHANGES = 100_000
SELECTION_MAX = 100_000
PARAMETERS_MAX = 256
MEMORY_FLOOR_MB = 64
U64 = (1 << 64) - 1
MEMORY_BASE = 192 * 1024 * 1024
MEMORY_FACTOR = 24

# ── The estimate and the budget ─────────────────────────────────────────


def estimate(size: int) -> int:
    return min(MEMORY_BASE + min(size * MEMORY_FACTOR, U64), U64)


def budget(request: dict, limits: dict) -> dict:
    cpu = limits["cpuSeconds"]
    if request.get("cpuSeconds") is not None:
        cpu = min(request["cpuSeconds"], cpu)
    mb = limits["memoryMb"]
    if request.get("memoryMb") is not None:
        mb = min(request["memoryMb"], mb)
    return {"cpuSeconds": cpu, "memoryBytes": mb * 1024 * 1024}


# ── The request ─────────────────────────────────────────────────────────


def is_uuid(text: str) -> bool:
    """An id as KentOS writes it: lowercase, with hyphens (an upper-case one is not an id)."""
    parts = text.split("-")
    if [len(p) for p in parts] != [8, 4, 4, 4, 12]:
        return False
    return all(c in "0123456789abcdef" for p in parts for c in p)


def file_value(value, at: str):
    if isinstance(value, dict):
        if isinstance(value.get("path"), str):
            return f"{at}.path"
        for k, v in value.items():
            found = file_value(v, f"{at}.{k}")
            if found:
                return found
    elif isinstance(value, list):
        for i, v in enumerate(value):
            found = file_value(v, f"{at}[{i}]")
            if found:
                return found
    return None


def fault(request: dict):
    tool = request.get("tool", "").strip()
    if not tool or len(tool) > 200 or any(ord(c) < 32 or ord(c) == 127 for c in tool):
        return "input.tool"
    parameters = request.get("parameters", {})
    if len(parameters) > PARAMETERS_MAX:
        return "input.parameters"
    for name, value in parameters.items():
        found = file_value(value, f"input.parameters.{name}")
        if found:
            return found
    selection = request.get("selection", [])
    if len(selection) > SELECTION_MAX:
        return "input.selection"
    for i, s in enumerate(selection):
        if not is_uuid(s):
            return f"input.selection[{i}]"
    view = request.get("view")
    if view is not None and (view[0] > view[2] or view[1] > view[3]):
        return "input.view"
    expected = request.get("expectedRevision")
    if expected is not None and not (expected.isdigit() and int(expected) <= (1 << 63) - 1):
        return "input.expectedRevision"
    if request.get("cpuSeconds") == 0:
        return "input.cpuSeconds"
    if request.get("memoryMb") is not None and request["memoryMb"] < MEMORY_FLOOR_MB:
        return "input.memoryMb"
    return None


# ── The delta ───────────────────────────────────────────────────────────


def uid(i: int) -> str:
    return f"00000000-0000-4000-8000-{i:012x}"


def expand(drawing: dict) -> dict:
    """The compact drawing with its `generate` (a count of points) written out."""
    entities = list(drawing.get("entities", []))
    gen = drawing.get("generate")
    if gen:
        for i in range(gen["count"]):
            entities.append({"uid": uid(gen.get("from", 1) + i), "slot": i + 1, "layer": gen["layer"], "x": float(i), "y": 0.0, "attrs": {}})
    return {**drawing, "entities": entities}


def content(e: dict):
    return (e["layer"], e["x"], e["y"], json.dumps(e.get("attrs", {}), sort_keys=True))


def delta(before: dict, after: dict) -> dict:
    before, after = expand(before), expand(after)
    was_blocks = {b[0]: b for b in before.get("blocks", [])}
    now_blocks = {b[0]: b for b in after.get("blocks", [])}
    blocks = {
        "created": [i for i in now_blocks if i not in was_blocks],
        "updated": [i for i in now_blocks if i in was_blocks and now_blocks[i] != was_blocks[i]],
        "deleted": [i for i in was_blocks if i not in now_blocks],
    }
    was = {e["uid"]: e for e in before["entities"]}
    kept = {e["uid"] for e in after["entities"]}
    created = [e["uid"] for e in after["entities"] if e["uid"] not in was]
    updated = [e["uid"] for e in after["entities"] if e["uid"] in was and content(e) != content(was[e["uid"]])]
    deleted = [e["uid"] for e in before["entities"] if e["uid"] not in kept]
    if len(created) + len(updated) + len(deleted) + sum(len(v) for v in blocks.values()) > MAX_CHANGES:
        return {"refused": "too_many_changes"}
    patch = []
    if before["name"] != after["name"]:
        patch.append("name")
    if before.get("plotScale") != after.get("plotScale"):
        patch.append("settings")
    if before["layers"] != after["layers"]:
        patch.append("layers")
    if before["activeLayer"] != after["activeLayer"]:
        patch.append("activeLayer")
    out = {
        "patch": sorted(patch),
        "added": len(created),
        "changed": len(updated),
        "removed": len(deleted),
        "blocks": blocks,
    }
    # The ids are listed for a small delta; a big one (the generated ones) is told by its counts alone.
    for key, ids in (("created", created), ("updated", updated), ("deleted", deleted)):
        if len(ids) <= 50:
            out[key] = ids
    return out


def drawing(layers=("cizim",), active="cizim", name="Proje", scale=1000.0, blocks=(), entities=(), generate=None):
    out = {
        "name": name,
        "plotScale": scale,
        "layers": list(layers),
        "activeLayer": active,
        "blocks": list(blocks),
        "entities": list(entities),
    }
    if generate:
        out["generate"] = generate
    return out


def pt(i: int, layer="cizim", x=0.0, y=0.0, attrs=None, slot=None) -> dict:
    return {"uid": uid(i), "slot": slot if slot is not None else i, "layer": layer, "x": x, "y": y, "attrs": attrs or {}}


def delta_cases() -> list[dict]:
    cases: list[tuple[str, dict, dict]] = []

    def add(name, before, after):
        cases.append((name, before, after))

    base = [pt(1, x=1.0), pt(2, x=2.0), pt(3, x=3.0, attrs={"a": "1"})]
    add("nothing changed", drawing(entities=base), drawing(entities=base))
    add("one object created", drawing(entities=base), drawing(entities=[*base, pt(4, x=4.0)]))
    add("three objects created", drawing(entities=base), drawing(entities=[*base, pt(4), pt(5), pt(6)]))
    add("one object moved", drawing(entities=base), drawing(entities=[pt(1, x=1.5), base[1], base[2]]))
    add("one attribute changed", drawing(entities=base), drawing(entities=[base[0], base[1], pt(3, x=3.0, attrs={"a": "2"})]))
    add("an attribute added", drawing(entities=base), drawing(entities=[base[0], pt(2, x=2.0, attrs={"yeni": "x"}), base[2]]))
    add("an object moved to another layer", drawing(layers=("cizim", "yol"), entities=base), drawing(layers=("cizim", "yol"), entities=[pt(1, layer="yol", x=1.0), base[1], base[2]]))
    add("one object removed", drawing(entities=base), drawing(entities=[base[0], base[2]]))
    add("every object removed", drawing(entities=base), drawing())
    add("created, changed and removed together", drawing(entities=base), drawing(entities=[pt(1, x=9.0), base[2], pt(7)]))
    add("slot numbers are not content", drawing(entities=base), drawing(entities=[pt(1, x=1.0, slot=11), pt(2, x=2.0, slot=12), pt(3, x=3.0, attrs={"a": "1"}, slot=13)]))
    add("the same objects in another order", drawing(entities=base), drawing(entities=[base[2], base[0], base[1]]))
    add("objects created in an empty project", drawing(), drawing(entities=[pt(1), pt(2)]))
    add("a layer added", drawing(), drawing(layers=("cizim", "sonuc")))
    add("a layer added with its objects", drawing(entities=base), drawing(layers=("cizim", "sonuc"), entities=[*base, pt(8, layer="sonuc")]))
    add("a layer removed", drawing(layers=("cizim", "yol")), drawing(layers=("cizim",)))
    add("layers reordered", drawing(layers=("cizim", "yol")), drawing(layers=("yol", "cizim")))
    add("the active layer changed", drawing(layers=("cizim", "yol")), drawing(layers=("cizim", "yol"), active="yol"))
    add("the project renamed", drawing(), drawing(name="Yeni ad"))
    add("the plot scale changed", drawing(), drawing(scale=500.0))
    add("metadata and objects together", drawing(entities=base), drawing(layers=("cizim", "sonuc"), name="Başka", scale=250.0, entities=[*base, pt(9, layer="sonuc")]))
    door, window, gate = (1, "Kapı"), (2, "Pencere"), (3, "Kapı")
    add("a block added is carried", drawing(), drawing(blocks=(door,)))
    add("a block removed is carried", drawing(blocks=(door,)), drawing())
    add("a block renamed is replaced", drawing(blocks=(door,)), drawing(blocks=((1, "Pencere"),)))
    add("another block under the same name is a new one and the old is gone", drawing(blocks=(door,)), drawing(blocks=(gate,)))
    add("blocks beside objects", drawing(entities=base, blocks=(door,)), drawing(entities=[*base, pt(4)], blocks=(door, window)))
    add("the same blocks and a created object", drawing(entities=base, blocks=(door,)), drawing(entities=[*base, pt(4)], blocks=(door,)))
    add("two blocks, one replaced, one removed, one made", drawing(blocks=(door, window)), drawing(blocks=((1, "Kapı 2"), (4, "Yeni"))))
    add("a block change beside metadata", drawing(), drawing(name="Başka", blocks=(door,)))
    add("the most a job takes: a hundred thousand objects", drawing(), drawing(generate={"count": MAX_CHANGES, "layer": "cizim"}))
    add("one past the most", drawing(), drawing(generate={"count": MAX_CHANGES + 1, "layer": "cizim"}))
    add("an object and a block past the most together", drawing(), drawing(blocks=(door,), generate={"count": MAX_CHANGES, "layer": "cizim"}))
    add(
        "a removal and a creation count together",
        drawing(generate={"count": 50_001, "layer": "cizim"}),
        drawing(generate={"count": 50_001, "layer": "cizim", "from": 50_002}),
    )
    add("more than a request takes, within a job's", drawing(), drawing(generate={"count": 6000, "layer": "cizim"}))
    add("many unchanged objects are not changes", drawing(generate={"count": 6000, "layer": "cizim"}), drawing(generate={"count": 6000, "layer": "cizim"}))
    add("a change of place the size of a float step", drawing(entities=[pt(1, x=0.1)]), drawing(entities=[pt(1, x=0.1 + 1e-16)]))
    return [{"name": n, "before": b, "after": a, "expect": delta(b, a)} for n, b, a in cases]


# ── The inputs, memory and budgets ──────────────────────────────────────


def input_cases() -> list[dict]:
    good = {"tool": "geometry.buffer"}
    u = uid(5)
    inputs = [
        ("a tool alone", good),
        ("values, selection and view", {"tool": "geometry.buffer", "parameters": {"mesafe": 5, "ad": "x"}, "selection": [u], "view": [0.0, 0.0, 10.0, 10.0]}),
        ("a table given as rows", {"tool": "stats.summary", "parameters": {"tablo": {"name": "t", "rows": [["a", 1]]}}}),
        ("an empty tool", {"tool": ""}),
        ("a blank tool", {"tool": "   "}),
        ("a tool with a control character", {"tool": "geo\u0007metry"}),
        ("a tool of 200 characters", {"tool": "t" * 200}),
        ("a tool of 201 characters", {"tool": "t" * 201}),
        ("a file value", {"tool": "x", "parameters": {"dosya": {"path": "/etc/passwd"}}}),
        ("a file value deep in a list", {"tool": "x", "parameters": {"a": [1, {"b": {"path": "x"}}]}}),
        ("a path that is not a text is a value", {"tool": "x", "parameters": {"a": {"path": 3}}}),
        ("a selection that is not an id", {"tool": "x", "selection": [u, "yok"]}),
        ("an upper-case id is not an id", {"tool": "x", "selection": [uid(255).upper()]}),
        ("a box with its corners the wrong way", {"tool": "x", "view": [5.0, 0.0, 1.0, 1.0]}),
        ("a box of one point", {"tool": "x", "view": [1.0, 1.0, 1.0, 1.0]}),
        ("a revision that is not a number", {"tool": "x", "expectedRevision": "bir"}),
        ("revision zero", {"tool": "x", "expectedRevision": "0"}),
        ("a revision past the database's", {"tool": "x", "expectedRevision": "9223372036854775808"}),
        ("no CPU", {"tool": "x", "cpuSeconds": 0}),
        ("one CPU second", {"tool": "x", "cpuSeconds": 1}),
        ("memory under the floor", {"tool": "x", "memoryMb": 63}),
        ("memory at the floor", {"tool": "x", "memoryMb": 64}),
        ("the most values", {"tool": "x", "parameters": {f"p{i}": i for i in range(PARAMETERS_MAX)}}),
        ("one value too many", {"tool": "x", "parameters": {f"p{i}": i for i in range(PARAMETERS_MAX + 1)}}),
    ]
    return [{"name": n, "input": i, "fault": fault(i)} for n, i in inputs]


def memory_cases() -> list[dict]:
    sizes = [0, 1, 1024, 1_000_000, 50 * 1024 * 1024, 1 << 30, 1 << 40, U64 // 24, U64 // 24 + 1, U64]
    return [{"bytes": s, "estimate": estimate(s)} for s in sizes]


def budget_cases() -> list[dict]:
    limits = {"cpuSeconds": 1800, "memoryMb": 4096}
    requests = [
        {},
        {"cpuSeconds": 60},
        {"cpuSeconds": 99999},
        {"memoryMb": 512},
        {"memoryMb": 100000},
        {"cpuSeconds": 60, "memoryMb": 512},
        {"cpuSeconds": 1800, "memoryMb": 4096},
    ]
    out = [{"request": r, "limits": limits, "budget": budget(r, limits)} for r in requests]
    small = {"cpuSeconds": 10, "memoryMb": 128}
    out.append({"request": {"cpuSeconds": 60}, "limits": small, "budget": budget({"cpuSeconds": 60}, small)})
    out.append({"request": {"memoryMb": 4096}, "limits": small, "budget": budget({"memoryMb": 4096}, small)})
    return out


def build() -> dict:
    return {
        "doc": "docs/adr/0559-processing-server-jobs.md",
        "note": "written by scripts/fixtures/processing_job_cases.py; read by crates/server/application/tests/processing_job_cases.rs",
        "limits": {"maxChanges": MAX_CHANGES, "selectionMax": SELECTION_MAX, "parametersMax": PARAMETERS_MAX, "memoryFloorMb": MEMORY_FLOOR_MB},
        "inputs": input_cases(),
        "memory": memory_cases(),
        "budgets": budget_cases(),
        "deltas": delta_cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print("fixtures/server-jobs/v1/processing.json eski; scripts/fixtures/processing_job_cases.py'yi --check'siz çalıştırın")
            return 1
        data = json.loads(text)
        print(f"İşlem işi durumları güncel: {len(data['inputs'])} girdi, {len(data['memory'])} bellek, {len(data['budgets'])} bütçe, {len(data['deltas'])} değişiklik")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
