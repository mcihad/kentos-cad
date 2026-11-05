#!/usr/bin/env python3
"""cad.sheets.details v1 (“Detay paftaları”, docs/adr/1090 §2–3): the shared cases, written from the ADR without KentOS
code.

The rules (ADR 1090 §2):
- A detail's viewport on the paper is its extent times paper/model in µm, rounded to the nearest µm (half away from
  zero): w = round((east₂ − east₁) · (1e6 · paper / model)).
- The details go into the template's first fixed map frame (sys:cad-teknik: 370 × 208 mm at 30, 20 mm; read from its
  file), in their order: a cell is max(w, 40 mm) wide and h + 6.5 mm deep (1.5 mm to the title, the title 5 mm); a row
  holds cells while they fit its width with 10 mm between; a sheet holds rows while they fit its height with 10 mm
  between; a detail that does not fit an empty region is refused (detail_does_not_fit).
- One sheet per filled region, id detay-1, detay-2 … (skipping taken ones), its items detay-N-item-i; the template's map
  gives way; each detail adds its viewport detay-N-detay-k (k its place in the input from 1) and its title
  detay-N-detay-k-baslik. Numbers from `first` (D-501): the last digits count on keeping their width, numbers taken are
  skipped. The name “<number> <title>”, with “ (i/n)” when the set has n > 1 sheets; drawing information the number,
  the title (Detaylar) and the subset Detaylar.
- A table part's extent (ADR 1090 §3, ADR 1030 §7's parts): the part's rows' depth and the table's width from its top
  left, x = k · (width + gap) along its rows, grown by its frame band and half its text height on every side.
- Refusals in order: invalid_revision, revision_conflict; for each detail in order entity_not_found, not_a_table,
  table_part_missing, ambiguous_source; unknown_template, invalid_options; bad_value (no details), for each detail
  bad_value (name), invalid_extent, invalid_ratio, detail_too_small, then a repeated name bad_value; bad_drawing_info;
  detail_does_not_fit.
"""
import argparse
import copy
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.sheets.details.json"
COMMAND = "cad.sheets.details"
GAP, TITLE_GAP, TITLE_H, MIN_CELL, MIN_VP = 10_000, 1_500, 5_000, 40_000, 1_000
SUBSET = ["Detaylar"]
TABLE = {"p": (20.0, 10.0), "height": 0.25, "rows": [0.5] * 8, "columns": [2.0, 1.5], "header": True,
         "breaks": {"height": 2.0, "gap": 0.5}}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def drawing():
    t = TABLE
    return {
        "format": "kentos.document", "version": 1, "name": "Detaylar",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100, "workspace": "cad", "drawingUnit": "m"},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "detay", "name": "Detay", "type": "layer", "visible": True, "locked": False,
                    "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []},
                   {"id": "olcu", "name": "Ölçü", "type": "layer", "visible": True, "locked": False,
                    "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.18},
                    "children": []}],
        "activeLayer": "detay",
        "entities": [
            {"kind": "line", "id": 1, "layerId": "detay", "attrs": {}, "a": P(0, 0), "b": P(0.1, 0.06)},
            {"kind": "line", "id": 2, "layerId": "detay", "attrs": {}, "a": P(-5, -3), "b": P(5, 3)},
            {"kind": "line", "id": 3, "layerId": "olcu", "attrs": {}, "a": P(0, -0.2), "b": P(0.37, -0.2)},
            {"kind": "table", "id": 4, "layerId": "detay", "attrs": {}, "p": P(*t["p"]), "rotation": 0,
             "height": t["height"], "rows": t["rows"], "columns": t["columns"],
             "cells": [["Poz", "Adet"]] + [[f"P{i}", str(i)] for i in range(1, 8)], "header": True,
             "breaks": t["breaks"]},
        ],
        "styles": {"items": [], "categories": []},
    }


def template():
    t = json.loads((ROOT / "crates/shared/sheet/templates/cad-teknik.json").read_text())
    items = t["sheet"]["items"]
    at = next(i for i, it in enumerate(items) if it["kind"]["type"] == "map" and it["kind"]["view"]["type"] == "fixed")
    f = items[at]["frame"]
    pad = items[at].get("padding", 0)
    region = (f["left"] + pad, f["top"] + pad, f["width"] - 2 * pad, f["height"] - 2 * pad)
    size = t["sheet"]["page"]["size"]
    return region, len(items), at, size["width"] / 1000, size["height"] / 1000


def rnd(v):
    """Rust's f64::round: half away from zero."""
    import math
    return float(math.floor(v + 0.5)) if v >= 0 else -float(math.floor(-v + 0.5))


def parts():
    """ADR 1030 §7's parts of the fixture's table (no merges): (rows, x) each."""
    t = TABLE
    rows, n, b = t["rows"], len(t["rows"]), t["breaks"]
    width = sum(t["columns"])
    gap = max(b.get("gap", 0.0), 0.0)
    out, start = [], 0
    while start < n:
        first = not out
        head = rows[0] if (not first and t["header"]) else 0.0
        end, used = start, head
        while end < n and (end == start or used + rows[end] <= b["height"]):
            used += rows[end]
            end += 1
        if first and t["header"] and end == 1 and n > 1:
            end = 2
        own = ([0] if (not first and t["header"]) else []) + list(range(start, end))
        out.append((own, len(out) * (width + gap)))
        start = end
    return out


def part_bounds(k):
    t = TABLE
    own, x = parts()[k]
    width = sum(t["columns"])
    depth = sum(t["rows"][r] for r in own)
    grow = 0.0 + t["height"] / 2.0
    px, py = t["p"]
    xs = [px + x, px + (x + width)]
    ys = [py, py - depth]
    return [min(xs) - grow, min(ys) - grow, max(xs) + grow, max(ys) + grow]


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


def run(book, inp):
    rev = inp.get("expectedRevision")
    if rev is not None:
        if not rev.isdigit() or (len(rev) > 1 and rev[0] == "0"):
            return refusal("invalid_revision", "expectedRevision"), book
        return refusal("revision_conflict", "expectedRevision", "conflict"), book
    details = []
    for i, d in enumerate(inp["details"]):
        path = f"details[{i}]"
        if ("bounds" in d) == ("table" in d):
            return refusal("ambiguous_source", path), book
        if "table" in d:
            uid, part = d["table"]["table"], d["table"]["part"]
            if uid not in ("$uidOf:1", "$uidOf:2", "$uidOf:3", "$uidOf:4"):
                return refusal("entity_not_found", f"{path}.table"), book
            if uid != "$uidOf:4":
                return refusal("not_a_table", f"{path}.table"), book
            if part >= len(parts()):
                return refusal("table_part_missing", f"{path}.table.part"), book
            bounds = part_bounds(part)
        else:
            bounds = d["bounds"]
        details.append((d, bounds))
    source = inp.get("source")
    if source and source["type"] == "system" and source["id"] != "sys:cad-teknik":
        return refusal("unknown_template", "source.id"), book
    options = inp.get("options", {})
    if any(k in options for k in ("name", "center", "scale")):
        return refusal("invalid_options", "options"), book
    if not details:
        return refusal("bad_value", "details"), book
    sizes = []
    for i, (d, b) in enumerate(details):
        name, path = d["name"], f"details[{i}]"
        if not name.strip() or name.strip() != name or len(name) > 120:
            return refusal("bad_value", f"{path}.name"), book
        if b[2] <= b[0] or b[3] <= b[1]:
            return refusal("invalid_extent", f"{path}.bounds"), book
        r = d["ratio"]
        if not (r["paper"] > 0 and r["model"] > 0 and 1e-6 <= r["model"] / r["paper"] <= 1e8):
            return refusal("invalid_ratio", f"{path}.ratio"), book
        k = 1e6 * r["paper"] / r["model"]
        w, h = rnd((b[2] - b[0]) * k), rnd((b[3] - b[1]) * k)
        if w < MIN_VP or h < MIN_VP:
            return refusal("detail_too_small", f"{path}.ratio"), book
        sizes.append((int(w), int(h)))
        if name.casefold() in [x["name"].casefold() for x, _ in details[:i]]:
            return refusal("bad_value", f"details[{i}].name"), book
    first = inp.get("first", "D-501")
    cut = numbered(first)
    if cut is None:
        return refusal("bad_drawing_info", "first"), book
    title = inp.get("title", "Detaylar")
    (left, top, rw, rh), items, at, pw, ph = template()
    right, bottom = left + rw, top + rh
    placed, sheet, x, y, row_h, in_row, on_sheet = [], 0, left, top, 0, 0, 0
    for i, (w, h) in enumerate(sizes):
        cw, chh = max(w, MIN_CELL), h + TITLE_GAP + TITLE_H
        if cw > rw or chh > rh:
            return refusal("detail_does_not_fit", f"details[{i}]"), book
        if in_row and x + cw > right:
            y, x, row_h, in_row = y + row_h + GAP, left, 0, 0
        if on_sheet and y + chh > bottom:
            sheet, x, y, row_h, in_row, on_sheet = sheet + 1, left, top, 0, 0, 0
        placed.append((sheet, {"left": x, "top": y, "width": w, "height": h}))
        x += cw + GAP
        row_h = max(row_h, chh)
        in_row += 1
        on_sheet += 1
    count = placed[-1][0] + 1
    before, n, digits, after = cut
    used = {s["drawingInfo"]["number"].casefold() for s in book if s.get("drawingInfo")}
    taken = {s["id"] for s in book}
    new, made, out_details = [], [], []
    for s in range(count):
        while True:
            number = f"{before}{n:0{digits}d}{after}"
            n += 1
            if number.casefold() not in used:
                used.add(number.casefold())
                break
        k, sid = 1, "detay-1"
        while sid in taken:
            k += 1
            sid = f"detay-{k}"
        taken.add(sid)
        name = f"{number} {title} ({s + 1}/{count})" if count > 1 else f"{number} {title}"
        maps, own = [], 0
        for i, ((on, frame), (d, _)) in enumerate(zip(placed, details)):
            if on != s:
                continue
            own += 1
            r = d["ratio"]
            item = f"{sid}-detay-{i + 1}"
            maps.append({"item": item, "name": d["name"], "scale": r["model"] / r["paper"]})
            out_details.append({"name": d["name"], "sheet": sid, "item": item, "title": f"{item}-baslik",
                                "frame": frame, "denominator": r["model"] / r["paper"]})
        made.append({"sheet": sid, "number": number, "name": name})
        new.append({"drawingInfo": {"number": number, "title": title, "revision": "", "status": "",
                                    "subset": SUBSET},
                    "height": ph, "id": sid, "items": items - 1 + 2 * own, "maps": maps, "name": name, "width": pw})
    after_book = book + new
    # The details in the input's order (the sheets' order is the input's too).
    return ({"status": "completed",
             "output": {"inverse": "$any", "label": "$any", "revision": "$current", "sheets": after_book,
                        "made": made, "details": out_details},
             "warnings": []}, after_book)


def R(paper, model):
    return {"paper": float(paper), "model": float(model)}


FOUR = [
    {"name": "Bulon", "bounds": [0.0, 0.0, 0.1, 0.06], "ratio": R(1, 1)},
    {"name": "Levha", "bounds": [0.0, 0.0, 0.5, 0.3], "ratio": R(1, 5)},
    {"name": "Bağlantı", "bounds": [-0.5, -0.5, 1.5, 0.7], "ratio": R(1, 20)},
    {"name": "Genel", "bounds": [-5.0, -3.0, 5.0, 3.0], "ratio": R(1, 100)},
]


def cases():
    out = []

    def case(name, inputs, op="execute"):
        book, steps = [], []
        for inp in inputs:
            result, after = run(book, inp)
            step = {"op": op, "input": inp, "result": result}
            if result["status"] == "completed" and op == "execute":
                step["expect"] = {"sheets": after, "canUndo": False}
                book = after
            else:
                step["expect"] = {"sheets": book, "revision": "same"}
            steps.append(step)
        out.append({"name": name, "steps": steps})

    case("Aynı modelin 1:1, 1:5, 1:20 ve 1:100 detayları tek paftada", [{"details": FOUR}])
    case("Plan yazmaz: aynı yerleşim", [{"details": FOUR}], op="plan")
    out.append({"name": "Doğrulama: girdi geçerli", "steps": [
        {"op": "validate", "input": {"details": FOUR}, "result": {"status": "completed", "output": None,
                                                                   "warnings": []},
         "expect": {"revision": "same"}}]})
    case("Büyütme 2:1", [{"details": [{"name": "Kaynak dikişi", "bounds": [0.0, 0.0, 0.04, 0.02],
                                        "ratio": R(2, 1)}]}])
    case("Kesirli 1:37,5", [{"details": [{"name": "Ankraj", "bounds": [0.0, 0.0, 3.0, 1.5], "ratio": R(1, 37.5)}]}])
    case("İlk numara D-601 ve başlık", [{"details": FOUR[:2], "first": "D-601", "title": "Bağlantı detayları"}])
    case("İlk numara 07: genişlik korunur", [{"details": FOUR[:1], "first": "07"}])
    case("Paftaya sığmayan detay setin sonraki paftasına geçer",
         [{"details": [{"name": f"Kesit {i}", "bounds": [0.0, 0.0, 30.0, 15.0], "ratio": R(1, 100)}
                       for i in range(1, 4)]}])
    case("Satır dolunca alt satır", [{"details": [{"name": f"D{i}", "bounds": [0.0, 0.0, 1.2, 0.5], "ratio": R(1, 10)}
                                                  for i in range(1, 6)]}])
    case("Dar detayın hücresi 40 mm", [{"details": [{"name": f"Pim {i}", "bounds": [0.0, 0.0, 0.01, 0.05],
                                                     "ratio": R(1, 1)} for i in range(1, 4)]}])
    case("İkinci çağrı: sonraki numara ve kimlik", [{"details": FOUR[:1]}, {"details": FOUR[1:2]}])
    case("Katman durumu, ezme ve açıklama temsili taşınır",
         [{"details": [{"name": "Ölçülü", "bounds": [0.0, -0.3, 0.5, 0.3], "ratio": R(1, 5),
                        "annotation": {"type": "fixed", "scale": 5.0}},
                       {"name": "Ölçüsüz", "bounds": [0.0, -0.3, 0.5, 0.3], "ratio": R(1, 5),
                        "layers": {"type": "list", "layers": ["detay"]},
                        "layerOverrides": [{"layer": "olcu", "frozen": True}]}]}])
    case("Kilitsiz detay", [{"details": [dict(FOUR[1], locked=False)]}])
    case("Tablonun ilk parçası", [{"details": [{"name": "Poz listesi 1", "table": {"table": "$uidOf:4", "part": 0},
                                                "ratio": R(1, 20)}]}])
    case("Tablonun parçaları ayrı detaylar", [{"details": [
        {"name": f"Poz listesi {k + 1}", "table": {"table": "$uidOf:4", "part": k}, "ratio": R(1, 20)}
        for k in range(3)]}])
    # Refusals.
    case("Olmayan tablo parçası", [{"details": [{"name": "P", "table": {"table": "$uidOf:4", "part": 3},
                                                 "ratio": R(1, 20)}]}])
    case("Tablo olmayan nesne", [{"details": [{"name": "P", "table": {"table": "$uidOf:1", "part": 0},
                                               "ratio": R(1, 20)}]}])
    case("Çizimde olmayan tablo", [{"details": [{"name": "P", "table": {"table": "0192f5a1-1090-7000-8100-0000000000ff",
                                                                         "part": 0}, "ratio": R(1, 20)}]}])
    case("Kapsam ile tablo birlikte", [{"details": [{"name": "P", "bounds": [0.0, 0.0, 1.0, 1.0],
                                                     "table": {"table": "$uidOf:4", "part": 0}, "ratio": R(1, 20)}]}])
    case("Kaynaksız detay", [{"details": [{"name": "P", "ratio": R(1, 20)}]}])
    case("Detaysız çağrı", [{"details": []}])
    case("Boş ad", [{"details": [{"name": " ", "bounds": [0.0, 0.0, 1.0, 1.0], "ratio": R(1, 20)}]}])
    case("Ters kapsam", [{"details": [{"name": "Ters", "bounds": [1.0, 0.0, 0.0, 1.0], "ratio": R(1, 20)}]}])
    case("Sıfır paydalı oran", [{"details": [{"name": "Oran", "bounds": [0.0, 0.0, 1.0, 1.0], "ratio": R(1, 0)}]}])
    case("1 mm'den küçük pencere", [{"details": [{"name": "Nokta", "bounds": [0.0, 0.0, 0.0005, 0.01],
                                                   "ratio": R(1, 1)}]}])
    case("Aynı ad iki detayda", [{"details": [FOUR[0], dict(FOUR[1], name="bulon")]}])
    case("Boş alana bile sığmayan detay", [{"details": [{"name": "Vaziyet", "bounds": [0.0, 0.0, 50.0, 10.0],
                                                          "ratio": R(1, 100)}]}])
    case("Adı verilen seçenek", [{"details": FOUR[:1], "options": {"name": "D"}}])
    case("Olmayan sistem şablonu", [{"details": FOUR[:1], "source": {"type": "system", "id": "sys:yok"}}])
    case("Rakamla bitmeyen ilk numara", [{"details": FOUR[:1], "first": "D-"}])
    case("Geçersiz sürüm", [{"details": FOUR[:1], "expectedRevision": "01"}])
    case("Eski sürüm: hiçbir şey yazılmaz", [{"details": FOUR[:1], "expectedRevision": "9999"}])
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Detay paftaları: aynı modelin kesin oranlı detayları, yerleşim, set paftaları, tablo parçaları, retler",
            "note": "ADR 1090. Beklenenler scripts/fixtures/sheet_details_cases.py'nin ADR'den yazılmış kurallarından, "
                    "KentOS kodu olmadan; şablonun sayfası, öğeleri ve haritası kendi dosyasından okunur.",
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
