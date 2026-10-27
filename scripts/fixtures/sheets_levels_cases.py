#!/usr/bin/env python3
"""cad.sheets.levels v1 (“Kat planı paftaları”, docs/adr/0718): the shared cases, written from the ADR without KentOS
code.

The drawing: three levels (Zemin kat at 0, 1. kat at 3 m, 2. kat at 6 m, none of its own on 2. kat); a closed 10 × 8 m
wall 0.2 m thick on Zemin kat, a straight 12 m wall on 1. kat, a line on no level far away (100, 100).

The rules (ADR 0718 §1):
- One sheet per level asked (all of the project's in its order without `levels`; each once), from the template
  (a custom one here, an A3 sheet with one 390 × 230 mm map; the system ones read from their files), each named
  “<number> <level name> planı”, its id `kat-plani-<level>` (then `-2`, `-3` … while taken), its items
  `<id>-item-<n>`, its drawing information the number, “<level name> planı” and Mimari › Kat planları.
- Numbers from `first` (A-101 without one): its last digits count on, keeping their width; a number a sheet has is
  skipped.
- Every map shows the chosen levels' objects' box (a level's objects: its walls), the whole drawing's when they have
  none; without `options.scale` and `options.center`, at the smallest standard scale not below
  ceil(max(w/frame w, h/frame h) × 1.1) (5 % each side), a view-locked map keeping its scale; its level the sheet's,
  `below` with it.
- A level some map shows already is left out unless `again`; none left: levels_have_sheets.
- Refusals in order: invalid_revision, revision_conflict, level_not_found, unknown_template, invalid_options,
  levels_have_sheets, not_fixed_map, bad_drawing_info.
"""
import argparse
import copy
import json
import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.sheets.levels.json"
COMMAND = "cad.sheets.levels"
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0},
          {"id": "kat-2", "name": "2. kat", "elevation": 6.0, "height": 3.0}]
STANDARD = [100, 200, 250, 500, 1000, 2000, 2500, 5000, 10000, 25000, 50000, 100000]
SUBSET = ["Mimari", "Kat planları"]


def P(x, y):
    return {"x": float(x), "y": float(y)}


def drawing(levels=True):
    settings = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 100, "workspace": "cad", "drawingUnit": "m"}
    if levels:
        settings["arch"] = {"levels": copy.deepcopy(LEVELS)}
    return {
        "format": "kentos.document", "version": 1, "name": "Kat planları",
        "settings": settings,
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "duvar", "name": "Duvar", "type": "layer", "visible": True, "locked": False,
                    "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []}],
        "activeLayer": "duvar",
        "entities": [
            {"kind": "wall", "id": 1, "layerId": "duvar", "attrs": {},
             "pts": [P(0, 0), P(10, 0), P(10, 8), P(0, 8)], "width": 0.2, "justify": "center", "closed": True,
             "arch": {"level": "zemin"}},
            {"kind": "wall", "id": 2, "layerId": "duvar", "attrs": {}, "pts": [P(0, 0), P(12, 0)], "width": 0.2,
             "justify": "center", "arch": {"level": "kat-1"}},
            {"kind": "line", "id": 3, "layerId": "duvar", "attrs": {}, "a": P(100, 100), "b": P(101, 100)},
        ],
        "styles": {"items": [], "categories": []},
    }


# The objects' boxes (a wall's faces, a line's ends).
BOXES = {"zemin": (-0.1, -0.1, 10.1, 8.1), "kat-1": (0.0, -0.1, 12.0, 0.1)}
EVERYTHING = (-0.1, -0.1, 101.0, 100.0)


def union(boxes):
    boxes = list(boxes)
    if not boxes:
        return None
    return (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))


def map_item(view_locked=False, with_map=True):
    kind = {"clipToAtlasFeature": False, "grids": [], "labelBand": 0, "layers": {"type": "all"},
            "overviewFrame": {"cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
            "type": "map", "view": {"rotation": 0, "scale": 50, "type": "fixed"}}
    if view_locked:
        kind["viewLocked"] = True
    if not with_map:
        kind = {"type": "border", "style": "single", "stroke": {"width": 500}}
    return {"bindings": [], "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
            "frame": {"height": 230000, "left": 20000, "top": 10000, "width": 390000}, "hidden": False,
            "id": "plan-harita", "kind": kind, "locked": False, "name": "Harita", "opacity": 100, "padding": 0,
            "printable": True, "rotation": 0}


def custom(view_locked=False, with_map=True):
    return {"type": "custom", "template": {
        "schema": "kentos.sheet.template/1",
        "meta": {"id": "user:kat-plani", "revision": 1, "name": "Kat planı", "workspaces": ["cad"]},
        "sheet": {"export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "svg"}, "guides": [],
                  "id": "plan", "items": [map_item(view_locked, with_map)], "name": "Kat planı",
                  "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000},
                           "orientation": "landscape", "paper": "a3", "size": {"height": 297000, "width": 420000}},
                  "snapGrid": {"enabled": False, "spacing": 5000, "visible": False}, "variables": [],
                  "variants": []},
        "variables": [], "assets": []}}


def system(name):
    """A system template's sheet as its file has it: its page, its items, its map's place among them."""
    t = json.loads((ROOT / f"crates/shared/sheet/templates/{name}.json").read_text())
    items = t["sheet"]["items"]
    at = next(i for i, it in enumerate(items) if it["kind"]["type"] == "map")
    return t, at


def template_facts(source):
    """(page width mm, height mm, items, map index, map name, frame w µm, h µm, the map's own scale, locked view,
    has a map)."""
    if source is None:
        source = {"type": "system", "id": "sys:cad-mimari"}
    if source["type"] == "system":
        t, at = system(source["id"].split(":", 1)[1])
        sheet = t["sheet"]
    else:
        sheet = source["template"]["sheet"]
        at = next((i for i, it in enumerate(sheet["items"]) if it["kind"]["type"] == "map"), None)
    size = sheet["page"]["size"]
    if at is None:
        return (size["width"] / 1000, size["height"] / 1000, len(sheet["items"]), None, None, 0, 0, None, False, False)
    m = sheet["items"][at]
    pad = m.get("padding", 0) + m["kind"].get("labelBand", 0)
    return (size["width"] / 1000, size["height"] / 1000, len(sheet["items"]), at, m["name"],
            m["frame"]["width"] - 2 * pad, m["frame"]["height"] - 2 * pad, m["kind"]["view"]["scale"],
            bool(m["kind"].get("viewLocked")), True)


def fitted(box, fw, fh):
    w, h = box[2] - box[0], box[3] - box[1]
    need = max(w * 1e6 / fw, h * 1e6 / fh) * 1.1
    s = max(math.ceil(need), 1)
    return next((x for x in STANDARD if x >= s), s)


def refusal(code, path, status="failed"):
    err = {"code": code, "message": "$any"}
    if path is not None:
        err["path"] = path
    out = {"status": status, "error": err}
    if status == "conflict":
        out["error"]["revision"] = "$current"
    return out


def numbered(first):
    m = re.match(r"^(.*?)(\d+)(\D*)$", first)
    if not m:
        return None
    return m.group(1), int(m.group(2)), len(m.group(2)), m.group(3)


def run(doc, book, inp):
    """(result, book after): `book` the sheets so far, as the answer lists them."""
    rev = inp.get("expectedRevision")
    if rev is not None:
        if not rev.isdigit() or (len(rev) > 1 and rev[0] == "0"):
            return refusal("invalid_revision", "expectedRevision"), book
        return refusal("revision_conflict", "expectedRevision", "conflict"), book
    levels = (doc["settings"].get("arch") or {}).get("levels") or []
    if not levels:
        return refusal("level_not_found", "levels"), book
    names = {l["id"]: l["name"] for l in levels}
    if "levels" in inp:
        if not inp["levels"]:
            return refusal("level_not_found", "levels"), book
        chosen = []
        for i, lid in enumerate(inp["levels"]):
            if lid not in names:
                return refusal("level_not_found", f"levels[{i}]"), book
            if lid not in chosen:
                chosen.append(lid)
    else:
        chosen = [l["id"] for l in levels]
    source = inp.get("source")
    if source and source["type"] == "system" and source["id"] not in ("sys:cad-mimari", "sys:cad-teknik"):
        return refusal("unknown_template", "source.id"), book
    options = inp.get("options", {})
    if set(options) - {"paper", "name", "center", "scale", "values"}:
        return refusal("invalid_options", "options"), book
    if "name" in options:
        return refusal("invalid_options", "options.name"), book
    shown = {m["level"] for s in book for m in s["maps"] if "level" in m}
    todo = [l for l in chosen if inp.get("again") or l not in shown]
    if not todo:
        return refusal("levels_have_sheets", "levels"), book
    width, height, items, at, map_name, fw, fh, own_scale, locked, has_map = template_facts(source)
    if not has_map:
        return refusal("not_fixed_map", "source"), book
    first = inp.get("first", "A-101")
    cut = numbered(first)
    if cut is None:
        return refusal("bad_drawing_info", "first"), book
    before, n, digits, after = cut
    used = {s["drawingInfo"]["number"] for s in book if s.get("drawingInfo")}
    taken = {s["id"] for s in book}
    box = union(BOXES[l] for l in chosen if l in BOXES) or EVERYTHING
    if "scale" in options:
        scale = options["scale"]
    elif "center" in options or locked:
        scale = own_scale
    else:
        scale = fitted(box, fw, fh)
    new = []
    for lid in todo:
        while True:
            number = f"{before}{n:0{digits}d}{after}"
            n += 1
            if number not in used:
                used.add(number)
                break
        sid, k = f"kat-plani-{lid}", 2
        while sid in taken:
            sid, k = f"kat-plani-{lid}-{k}", k + 1
        taken.add(sid)
        title = f"{names[lid]} planı"
        m = {"item": f"{sid}-item-{at}", "name": map_name, "scale": scale, "level": lid}
        if inp.get("below"):
            m["below"] = True
        new.append({"drawingInfo": {"number": number, "title": title, "revision": "", "status": "",
                                    "subset": SUBSET},
                    "height": height, "id": sid, "items": items, "maps": [m], "name": f"{number} {title}",
                    "width": width})
    after_book = book + new
    return ({"status": "completed",
             "output": {"inverse": "$any", "label": "$any", "revision": "$current", "sheets": after_book},
             "warnings": []}, after_book)


def cases():
    doc = drawing()
    out = []

    def case(name, inputs, op="execute", setup=None):
        src = setup or doc
        book, steps = [], []
        for inp in inputs:
            result, after = run(src, book, inp)
            step = {"op": op, "input": inp, "result": result}
            if result["status"] == "completed" and op == "execute":
                step["expect"] = {"sheets": after, "canUndo": False}
                book = after
            else:
                step["expect"] = {"sheets": book, "revision": "same"}
            steps.append(step)
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    t = custom()
    case("Bütün katlar: her kat için pafta, haritalar katların kutusuna sığdığı standart ölçekte", [{"source": t}])
    case("options.scale 50: her harita 1:50", [{"source": t, "options": {"scale": 50}}])
    case("Plan yazmaz: aynı paftalar", [{"source": t}], op="plan")
    out.append({"name": "Doğrulama: girdi geçerli", "steps": [
        {"op": "validate", "input": {"source": t}, "result": {"status": "completed", "output": None, "warnings": []},
         "expect": {"revision": "same"}}]})
    case("Seçilen katlar verilen sırayla numaralanır", [{"source": t, "levels": ["kat-1", "zemin"]}])
    case("Aynı kat iki kez: bir kez", [{"source": t, "levels": ["zemin", "zemin"]}])
    case("Alt kat soluk: her harita", [{"source": t, "below": True}])
    case("İlk numara A-201", [{"source": t, "first": "A-201"}])
    case("İlk numara P-07: genişlik korunur", [{"source": t, "first": "P-07"}])
    case("İlk numara 9: 9, 10, 11", [{"source": t, "first": "9"}])
    case("Paftası olan numara atlanır", [{"source": t, "levels": ["kat-2"], "first": "A-102"},
                                          {"source": t, "levels": ["zemin", "kat-1"]}])
    case("Haritası olan kat atlanır", [{"source": t, "levels": ["zemin"]}, {"source": t}])
    case("Yeniden: aynı kat ikinci pafta, kimliği -2", [{"source": t, "levels": ["zemin"]},
                                                       {"source": t, "levels": ["zemin"], "again": True}])
    case("Bütün katların paftası var: ret", [{"source": t}, {"source": t}])
    case("Yalnız nesnesiz kat: bütün çizime sığar", [{"source": t, "levels": ["kat-2"]}])
    case("Nesnesiz ve dolu kat: dolu katın kutusu", [{"source": t, "levels": ["zemin", "kat-2"]}])
    case("Yalnız 1. kat: o katın kutusu", [{"source": t, "levels": ["kat-1"]}])
    case("Merkez verilince ölçek şablonunki", [{"source": t, "options": {"center": P(5, 4)}}])
    case("Görünümü kilitli harita: ölçek şablonunki, kat verilir", [{"source": custom(view_locked=True)}])
    case("Sistem şablonu sys:cad-teknik", [{"source": {"type": "system", "id": "sys:cad-teknik"}}])
    case("Şablon verilmeyince sys:cad-mimari", [{}])
    # Refusals.
    case("Projede olmayan kat", [{"source": t, "levels": ["zemin", "çatı"]}])
    case("Boş kat listesi", [{"source": t, "levels": []}])
    case("Katsız proje", [{"source": t}], setup=drawing(levels=False))
    case("Olmayan sistem şablonu", [{"source": {"type": "system", "id": "sys:yok"}}])
    case("Okunmayan seçenek", [{"source": t, "options": {"bogus": 1}}])
    case("Adı verilen seçenek", [{"source": t, "options": {"name": "Plan"}}])
    case("Haritasız şablon", [{"source": custom(with_map=False)}])
    case("Rakamla bitmeyen ilk numara", [{"source": t, "first": "A-"}])
    case("Geçersiz sürüm", [{"source": t, "expectedRevision": "01"}])
    case("Eski sürüm: hiçbir şey yazılmaz", [{"source": t, "expectedRevision": "9999"}])
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Kat planı paftaları: her kat için pafta, haritası o katı çizer; numaralar, ortak zemin ve "
                     "ölçek, haritası olan kat, retler",
            "note": "ADR 0718. Beklenenler scripts/fixtures/sheets_levels_cases.py'nin ADR'den yazılmış kurallarından, "
                    "KentOS kodu olmadan; sistem şablonlarının sayfası, öğeleri ve haritası kendi dosyalarından okunur.",
            "setup": drawing(), "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    doc = document()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND}: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
