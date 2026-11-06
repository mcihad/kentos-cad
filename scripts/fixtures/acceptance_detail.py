#!/usr/bin/env python3
"""The CAD-ADV-20 acceptance detail (docs/adr/1004, docs/adr/1011), written without KentOS code: a plate of four lines
held by constraints with the parameters `genislik` (8 m) and `yukseklik` (4 m), an associative dimension on its bottom
edge and an associative hatch inside it (its area the plate's, w × h), the width changed by its parameter (to 10, then
to 1200 cm), undone, a write against a stale revision, a change set whose second step meets a locked layer, a script
that fails half way (Python: a group whose body raises; MCP, which runs no script: the same steps as one change set
whose last step names an object the drawing lacks), a job cancelled before it is looked at, a request sent twice, a
save–open and a 1:50 detail sheet of the plate. Python (test_acceptance_detail.py)
and the MCP server (tests/acceptance_detail.rs) replay the same file; every expected value here is worked out from the
rectangle's sides (the dimension measures the bottom edge, its length the width).

    python3 scripts/fixtures/acceptance_detail.py           # writes fixtures/acceptance/v1/detail.json
    python3 scripts/fixtures/acceptance_detail.py --check   # writes nothing; compares
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/acceptance/v1/detail.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": STYLE, "children": []}


def line(id_, a, b, layer_id="cizim"):
    return {"kind": "line", "id": id_, "layerId": layer_id, "attrs": {}, "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "Bağlantı plakası",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": 20, "workspace": "cad", "drawingUnit": "m"},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("cizim", "Çizim"), layer("tarama", "Tarama"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "cizim",
    # Drawn roughly: the constraints make it a rectangle.
    "entities": [line(1, (0, 0), (6, 0.3)), line(2, (6, 0.3), (6.2, 4)), line(3, (6.2, 4), (0, 4.1)),
                 line(4, (0, 4.1), (0, 0)), line(5, (0, -5), (6, -4.5), "kilitli")],
    "styles": {"items": [], "categories": []},
}
# The objects by their place in the drawing, bound when it opens.
NAMES = ["l1", "l2", "l3", "l4", "locked"]


def point(name, place):
    return {"uid": f"${name}", "element": {"kind": "point", "place": {"kind": place}}}


def segment(name):
    return {"uid": f"${name}", "element": {"kind": "segment", "part": 0, "ring": 0, "index": 0}}


RULES = [
    {"rule": {"kind": "coincident", "a": point("l1", "end"), "b": point("l2", "start")}},
    {"rule": {"kind": "coincident", "a": point("l2", "end"), "b": point("l3", "start")}},
    {"rule": {"kind": "coincident", "a": point("l3", "end"), "b": point("l4", "start")}},
    {"rule": {"kind": "coincident", "a": point("l4", "end"), "b": point("l1", "start")}},
    {"rule": {"kind": "horizontal", "a": segment("l1")}},
    {"rule": {"kind": "parallel", "a": segment("l1"), "b": segment("l3")}},
    {"rule": {"kind": "parallel", "a": segment("l2"), "b": segment("l4")}},
    {"rule": {"kind": "perpendicular", "a": segment("l1"), "b": segment("l2")}},
    {"rule": {"kind": "fix", "a": point("l1", "start")}},
    {"rule": {"kind": "distance", "a": point("l1", "start"), "b": point("l1", "end"), "parameter": "genislik"}, "value": "8"},
    {"rule": {"kind": "distance", "a": point("l2", "start"), "b": point("l2", "end"), "parameter": "yukseklik"}, "value": "4"},
]


def plate(w, h):
    """The four sides of a w × h plate from the fixed corner (0, 0), as their end points."""
    return {"l1": {"a": [0.0, 0.0], "b": [w, 0.0]}, "l2": {"a": [w, 0.0], "b": [w, h]},
            "l3": {"a": [w, h], "b": [0.0, h]}, "l4": {"a": [0.0, h], "b": [0.0, 0.0]}}


def at(w, h, dim=True, hatch=True):
    out = {"points": plate(w, h)}
    if dim:
        out["points"]["dim"] = {"a": [0.0, 0.0], "b": [w, 0.0]}
        out["lengths"] = {"dim": w, "l1": w, "l2": h}
    else:
        out["lengths"] = {"l1": w, "l2": h}
    if hatch:
        # The hatch fills the plate: its area is the rectangle's.
        out["areas"] = {"hatch": w * h}
    return out


# An id no object of the drawing has: the failing step of the script that stops half way.
GONE = "0192f5a2-0000-7000-8000-00000000ffff"


def width(expression):
    return {"edits": [{"kind": "set", "name": "genislik", "expression": expression}]}


STEPS = [
    {"name": "Kısıtlar: dikdörtgen, sol alt köşe sabit, genişlik 8 m, yükseklik 4 m", "command": "cad.constraints.edit",
     "input": {"edit": {"kind": "add", "constraints": RULES}}, "expect": at(8.0, 4.0, dim=False, hatch=False)},
    {"name": "İlişkili ölçü alt kenarın iki ucuna bağlanır", "command": "cad.entities.create",
     "input": {"layerId": "cizim", "objects": [{"geometry": {"kind": "dimension", "a": {"x": 0, "y": 0}, "b": {"x": 0, "y": 0},
                "offset": -0.6, "height": 0.25,
                "refs": {"a": {"uid": "$l1", "place": {"kind": "start"}}, "b": {"uid": "$l1", "place": {"kind": "end"}}}}}]},
     "as": ["dim"], "expect": at(8.0, 4.0, hatch=False)},
    {"name": "İlişkili tarama plakanın içine: alanı 8 × 4", "command": "cad.entities.hatch",
     "input": {"region": {"kind": "point", "at": {"x": 4, "y": 2}, "way": "lines"}, "pattern": {"name": "Dolu"},
               "layerId": "tarama"},
     "as": ["hatch"], "expect": at(8.0, 4.0)},
    {"name": "Genişlik parametresi 10: plaka ve ölçü izler", "command": "cad.parameters.edit", "input": width("10"),
     "expect": at(10.0, 4.0)},
    {"name": "Birimli değer 1200cm: 12 m", "command": "cad.parameters.edit", "input": width("1200cm"), "expect": at(12.0, 4.0)},
    {"name": "Geri al: 10 m", "undo": "Parametreler", "expect": at(10.0, 4.0)},
    {"name": "Eski revizyona dayanan yazım: çatışma, yazım yok", "command": "cad.parameters.edit", "input": width("20"),
     "expected": {"revision": "0"}, "refuse": "revision_conflict", "expect": at(10.0, 4.0)},
    {"name": "Değişiklik kümesinin ikinci adımı kilitli katmana çarpar: ilk adım da yazılmaz", "command": "cad.changeset.apply",
     "input": {"label": "Toplu", "steps": [
         {"command": "cad.parameters.edit", "input": width("11")},
         {"command": "cad.entities.transform", "input": {"uids": ["$locked"], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}}]},
     "refuse": "layer_locked", "expect": at(10.0, 4.0)},
    {"name": "Betik yarıda çöker: ilk adım da yazılmaz", "crash": {"label": "Yarım betik", "steps": [
         {"command": "cad.parameters.edit", "input": width("14")},
         {"command": "cad.entities.transform", "input": {"uids": [GONE], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}}]},
     "expect": at(10.0, 4.0)},
    {"name": "İş olarak başlatılan genişlik 30, bakılmadan durdurulur: hiçbir şey yazılmaz",
     "job": {"command": "cad.parameters.edit", "input": width("30")}, "cancel": True, "expect": at(10.0, 4.0)},
    {"name": "Aynı istek iki kez: yükseklik bir kez 5 olur", "command": "cad.parameters.edit",
     "input": {"edits": [{"kind": "set", "name": "yukseklik", "expression": "5"}]}, "requestId": "detay-yukseklik-5",
     "twice": True, "expect": at(10.0, 5.0)},
    {"name": "Kaydet ve aç: kaynak, bağ ve parametreler aynı", "reopen": True, "expect": at(10.0, 5.0)},
    {"name": "Açılan çizimde genişlik 9: ölçü ve tarama bağı korunmuş", "command": "cad.parameters.edit", "input": width("9"),
     "expect": at(9.0, 5.0)},
    # 10 × 6 m of ground at 1:50 is 200 × 120 mm of paper: inside the A3 template's 370 × 208 mm drawing area.
    {"name": "1:50 detay paftası: plakanın görünümü kilitli, ölçeği 1/50", "command": "cad.sheets.details",
     "input": {"details": [{"name": "Plaka", "bounds": [-0.5, -0.5, 9.5, 5.5], "ratio": {"paper": 1.0, "model": 50.0}}]},
     "output": {"sheets.0.maps.0.scale": 50.0, "details.0.denominator": 50.0, "details.0.name": "Plaka"},
     "expect": at(9.0, 5.0)},
]


def build():
    return {"format": "kentos.acceptance", "version": 1, "id": "detail", "title": "Parametreli bağlantı plakası",
            "source": "scripts/fixtures/acceptance_detail.py (KentOS kodu yok; değerler dikdörtgenin kenarlarından)",
            "setup": SETUP, "names": NAMES, "tolerance": 1e-9, "steps": STEPS}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
