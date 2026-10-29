#!/usr/bin/env python3
"""The shared cases of the Doğrusal referans tools (docs/adr/0224 §5), İşlemler's on both platforms.

    python3 scripts/fixtures/lrs_processing_cases.py           # writes the drawing, the tables and the cases
    python3 scripts/fixtures/lrs_processing_cases.py --check   # writes nothing; compares

Writes fixtures/processing/v1/lrs.kcad (a drawing of routes, pieces, points, lines and an area), the event tables
lrs-noktalar.csv, lrs-cizgiler.csv and lrs-hizlar.csv, and lrs.json, the runs' expectations. The routes' rules are
scripts/fixtures/lrs_cases.py's (50 digits, no KentOS code); the tools' rules are written here from the ADR on their
own:

- A route is a line or a polyline of the routes' input: its route id the attribute's value, trimmed; an empty one no
  route; of several objects of one id the first (“n rota kimliği birden çok kez var; ilki kullanıldı.”). A line one
  part (its ends' ma, mb), a polyline its own part and its parts, their arcs and measures.
- A number: docs/adr/0180's (a sign, digits, one point or comma). Written numbers (attributes, table cells): the
  project's length decimals by the display rule (scripts/fixtures/numeric_display.py).
- Rota oluştur: the input's lines, polylines (each part a piece) and arcs (start to end, counter-clockwise) by the
  id field, its measures by the method (lrs_cases.create); a new layer Rotalar (#E5484D) of polylines, each with
  the id field's value; an object left out once in the table Nesne, Neden (“F” boş, “F” sayı değil: <cell>).
- Rotayı kalibre et: each route's points (the point's route field its id, its measure a number; other points are
  counted: “n noktanın ölçüsü sayı değil; kullanılmadı.”); a polyline takes vertices where the points fall, a line
  keeps its two (split false); written in place, an arc's bulge and a vertex's elevation carried (none here); the
  table Rota, Kullanılan nokta, Durum (Kalibre edildi, İkiden az nokta).
- Olayları rotada göster: the table's rows (a file's or the objects' attributes) on their routes: a point event at
  its measure, a line event from its from to its to (its parts, offset), the offset column's cell (empty 0) turned
  for Sol; a new layer Olaylar (#8E4EC6), points or polylines with the row's filled cells; a row that gives nothing
  in the table Satır, Neden.
- Rota boyunca konumla: each object's places along the routes within the radius (lrs_cases.locate, locate_line,
  locate_area: a point the nearest route only unless all; a line when every vertex is within the radius; an area
  every route's pieces inside), the distance turned for Sol; the table Nesne, Rota, Ölçü, Başlangıç, Bitiş, Uzaklık;
  with Özniteliklere yaz the first place written as Rota, Ölçü or Başlangıç and Bitiş, Uzaklık.
- Olayları çakıştır: two line event tables cut where any event begins or ends (lrs_cases.overlay), Kesişim or
  Birleşim; the table Rota, Başlangıç, Bitiş, then the first table's other columns, then the second's (one the first
  has too with “ (2)”); a row without a route or a number left out and counted.
- M değerlerini çıkar: each route's first, last, least and most measure written as <prefix>bas, <prefix>son,
  <prefix>min, <prefix>max; a route without measures counted.
"""
import json
import math
import sys
from pathlib import Path

import mpmath as mp

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import lrs_cases as L  # noqa: E402
from layer_ops_cases import new_id  # noqa: E402
from numeric_display import shown  # noqa: E402

ROOT = HERE.parents[1]
PROC = ROOT / "fixtures/processing/v1"
TOL = 1e-7
DECIMALS = 3
SPACES = " \t\n\r\x0b\x0c                 　﻿"


def js_trim(t):
    return t.strip(SPACES)


def read_number(t):
    """docs/adr/0180's number: a sign, digits and one decimal point or comma."""
    t = js_trim(t)
    body = t[1:] if t[:1] in "+-" else t
    if not body or any(c not in "0123456789.," for c in body):
        return None
    if sum(c.isdigit() for c in body) == 0 or sum(c in ".," for c in body) > 1:
        return None
    return float(t.lstrip("+").replace(",", "."))


def text(v):
    return shown(float(v), DECIMALS)


def style(color):
    return {"color": color, "lineType": "continuous", "lineWeight": 0.25}


def P(x, y):
    return {"x": x, "y": y}


# ── The drawing ─────────────────────────────────────────────────────────

LAYERS = [
    {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": style(color), "children": []}
    for id_, name, color in [
        ("yol", "Yol parçaları", "#0D74CE"),
        ("rota", "Ölçülü rotalar", "#E5484D"),
        ("kalib", "Kalibre edilecek", "#12A594"),
        ("km", "Km taşları", "#F76B15"),
        ("kaza", "Kazalar", "#AB4ABA"),
        ("hat", "Hatlar", "#0090FF"),
        ("parsel", "Parseller", "#30A46C"),
        ("olay", "Olay nesneleri", "#8E4EC6"),
    ]
]

QUARTER = math.pi / 2
ENTITIES = [
    # Rota oluştur's pieces: A in two chains (a gap of 50 m between them, the last piece drawn backwards), B with an
    # arc, C an arc object, one without its id.
    {"id": 1, "kind": "line", "layerId": "yol", "attrs": {"Yol": "A", "Uzunluk": "100", "Bas": "0", "Son": "100"}, "a": P(487000, 4420000), "b": P(487100, 4420000)},
    {"id": 2, "kind": "polyline", "layerId": "yol", "attrs": {"Yol": "A", "Uzunluk": "50", "Bas": "100", "Son": "150"}, "pts": [P(487100, 4420000), P(487100, 4420050)]},
    {"id": 3, "kind": "line", "layerId": "yol", "attrs": {"Yol": "A ", "Uzunluk": "100,5", "Bas": "200", "Son": "300"}, "a": P(487250, 4420050), "b": P(487150, 4420050)},
    {"id": 4, "kind": "polyline", "layerId": "yol", "attrs": {"Yol": "B", "Uzunluk": "200", "Bas": "1000", "Son": "1157,08"}, "pts": [P(487000, 4420200), P(487100, 4420200)], "bulges": [1.0]},
    {"id": 5, "kind": "line", "layerId": "yol", "attrs": {"Yol": "", "Uzunluk": "10"}, "a": P(487300, 4420300), "b": P(487310, 4420300)},
    {"id": 6, "kind": "arc", "layerId": "yol", "attrs": {"Yol": "C", "Uzunluk": "x", "Bas": "5"}, "c": P(487400, 4420400), "r": 10, "a0": 0, "a1": QUARTER},
    # Routes with measures.
    {"id": 7, "kind": "polyline", "layerId": "rota", "attrs": {"Rota": "D-100", "Ad": "Ana yol"}, "pts": [P(488000, 4421000), P(488100, 4421000), P(488100, 4421050)], "ms": [0, 100, 150]},
    {"id": 8, "kind": "polyline", "layerId": "rota", "attrs": {"Rota": "D-200"}, "pts": [P(488000, 4421200), P(488100, 4421200)], "bulges": [1.0], "ms": [0, 100]},
    {"id": 9, "kind": "line", "layerId": "rota", "attrs": {"Rota": "D-300"}, "a": P(488300, 4421000), "b": P(488300, 4421100), "ma": 1000, "mb": 900},
    {"id": 10, "kind": "polyline", "layerId": "rota", "attrs": {"Rota": "D-400"}, "pts": [P(488500, 4421000), P(488600, 4421000)]},
    {"id": 11, "kind": "polyline", "layerId": "rota", "attrs": {"Rota": "D-500"}, "pts": [P(488000, 4421400), P(488100, 4421400)], "ms": [0, 100],
     "parts": [{"pts": [P(488000, 4421450), P(488100, 4421450)], "ms": [100, 200]}]},
    {"id": 12, "kind": "polyline", "layerId": "rota", "attrs": {"Rota": "D-100"}, "pts": [P(489900, 4429900), P(489950, 4429900)], "ms": [7, 8]},
    # Routes to calibrate and their stones.
    {"id": 20, "kind": "polyline", "layerId": "kalib", "attrs": {"Rota": "K-1"}, "pts": [P(489000, 4422000), P(489100, 4422000), P(489100, 4422100)]},
    {"id": 21, "kind": "line", "layerId": "kalib", "attrs": {"Rota": "K-2"}, "a": P(489300, 4422000), "b": P(489400, 4422000)},
    {"id": 22, "kind": "line", "layerId": "kalib", "attrs": {"Rota": "K-3"}, "a": P(489300, 4422200), "b": P(489400, 4422200)},
    {"id": 30, "kind": "point", "layerId": "km", "attrs": {"Rota": "K-1", "Km": "1000"}, "p": P(489020, 4422001)},
    {"id": 31, "kind": "point", "layerId": "km", "attrs": {"Rota": "K-1", "Km": "1130"}, "p": P(489101, 4422050)},
    {"id": 32, "kind": "point", "layerId": "km", "attrs": {"Rota": "K-1", "Km": "x"}, "p": P(489050, 4422030)},
    {"id": 33, "kind": "point", "layerId": "km", "attrs": {"Rota": "K-2", "Km": "500"}, "p": P(489350, 4421999)},
    {"id": 34, "kind": "point", "layerId": "km", "attrs": {"Rota": "K-1", "Km": "1200"}, "p": P(489500, 4423000)},
    {"id": 35, "kind": "point", "layerId": "km", "attrs": {"Rota": "K-3", "Km": "10"}, "p": P(489320, 4422201)},
    {"id": 36, "kind": "point", "layerId": "km", "attrs": {"Rota": "K-3", "Km": "50"}, "p": P(489360, 4422199)},
    # Objects to locate.
    {"id": 40, "kind": "point", "layerId": "kaza", "attrs": {"No": "1"}, "p": P(488040, 4421010)},
    {"id": 41, "kind": "point", "layerId": "kaza", "attrs": {"No": "2"}, "p": P(488120, 4421020)},
    {"id": 42, "kind": "point", "layerId": "kaza", "attrs": {"No": "3"}, "p": P(488300, 4421050)},
    {"id": 43, "kind": "point", "layerId": "kaza", "attrs": {"No": "4"}, "p": P(489000, 4425000)},
    {"id": 44, "kind": "point", "layerId": "kaza", "attrs": {"No": "5"}, "p": P(488050, 4421430)},
    {"id": 50, "kind": "polyline", "layerId": "hat", "attrs": {"Tur": "kaplama"}, "pts": [P(488010, 4421001), P(488090, 4420999)]},
    {"id": 51, "kind": "line", "layerId": "hat", "attrs": {"Tur": "uzak"}, "a": P(488010, 4421030), "b": P(488090, 4421030)},
    {"id": 60, "kind": "polygon", "layerId": "parsel", "attrs": {"Ada": "1"}, "pts": [P(488040, 4420990), P(488060, 4420990), P(488060, 4421010), P(488040, 4421010)]},
    # Event objects written by Olayları rotada göster before their routes changed: some still in place, some drifted.
    {"id": 70, "kind": "point", "layerId": "olay", "attrs": {"Rota": "D-100", "Km": "40", "Ofset": "5"}, "p": P(488040, 4420995)},
    {"id": 71, "kind": "point", "layerId": "olay", "attrs": {"Rota": "D-100", "Km": "125"}, "p": P(488100, 4421020)},
    {"id": 72, "kind": "point", "layerId": "olay", "attrs": {"Rota": "D-999", "Km": "10"}, "p": P(488500, 4421500)},
    {"id": 73, "kind": "point", "layerId": "olay", "attrs": {"Rota": "D-100", "Km": "500"}, "p": P(488100, 4421400)},
    {"id": 74, "kind": "point", "layerId": "olay", "attrs": {"Rota": "D-100", "Km": "x"}, "p": P(488010, 4421000)},
    {"id": 75, "kind": "point", "layerId": "olay", "attrs": {"Rota": "D-200", "Km": "50"}, "p": P(488050, 4421150.0004)},
    {"id": 76, "kind": "polyline", "layerId": "olay", "attrs": {"Rota": "D-100", "Bas": "20", "Son": "130"},
     "pts": [P(488020, 4421000), P(488100, 4421000), P(488100, 4421030)], "ms": [20, 100, 130]},
    {"id": 77, "kind": "polyline", "layerId": "olay", "attrs": {"Rota": "D-100", "Bas": "20", "Son": "120"},
     "pts": [P(488020, 4421000), P(488090, 4421000), P(488090, 4421020)], "ms": [20, 90, 110]},
    {"id": 78, "kind": "line", "layerId": "olay", "attrs": {"Rota": "D-300", "Bas": "990", "Son": "920"}, "a": P(488301, 4421010), "b": P(488301, 4421080), "ma": 990, "mb": 920},
    {"id": 79, "kind": "line", "layerId": "olay", "attrs": {"Rota": "D-100", "Bas": "50", "Son": "120"}, "a": P(488050, 4421000), "b": P(488100, 4421020)},
    {"id": 80, "kind": "polyline", "layerId": "olay", "attrs": {"Rota": "D-200", "Bas": "0", "Son": "100"},
     "pts": [P(488000, 4421200), P(488100, 4421200)], "ms": [0, 100]},
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: Doğrusal referans",
    "settings": {"srid": 5256, "lengthDecimals": DECIMALS, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 488000, "y": 4421000},
    "layers": LAYERS,
    "activeLayer": "yol",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}
BY_ID = {e["id"]: e for e in ENTITIES}

# ── The tables ──────────────────────────────────────────────────────────

FILES = {
    "lrs-noktalar.csv": (
        "Rota;Km;Ofset;Tur\n"
        "D-100;40;5;kaza\n"
        "D-100;125;-3;levha\n"
        "D-200;50;;köprü\n"
        "D-300;950;2;kavşak\n"
        "D-999;10;;yok\n"
        "D-100;500;;dışarı\n"
        "D-400;10;;ölçüsüz\n"
        "D-100;abc;;bozuk\n"
    ),
    "lrs-cizgiler.csv": (
        "Rota,Bas,Son,Kaplama\n"
        "D-100,20,130,asfalt\n"
        "D-100,140,120,beton\n"
        "D-300,990,920,parke\n"
        "D-500,50,150,stabilize\n"
        "D-100,60,60,aynı\n"
    ),
    "lrs-hizlar.csv": (
        "Rota,Bas,Son,Hiz,Kaplama\n"
        "D-100,0,50,50,eski\n"
        "D-100,50,150,70,eski\n"
        "D-300,900,950,30,\n"
    ),
}


def table_of(name):
    lines = FILES[name].strip("\n").split("\n")
    sep = ";" if ";" in lines[0] else ","
    header = [js_trim(c) for c in lines[0].split(sep)]
    rows = [line.split(sep) for line in lines[1:]]
    return header, rows, [str(i + 2) for i in range(len(rows))]


def objects_table(objs):
    header = sorted({k for e in objs for k in e["attrs"]})
    rows = [[e["attrs"].get(h, "") for h in header] for e in objs]
    return header, rows, [f"#{e['id']}" for e in objs]


# ── Routes of the drawing ───────────────────────────────────────────────


def route_parts(e):
    """A line's or a polyline's parts as lrs_cases reads them: pts as pairs, bulges, ms (None for none)."""
    if e["kind"] == "line":
        return [{"pts": [[e["a"]["x"], e["a"]["y"]], [e["b"]["x"], e["b"]["y"]]], "ms": [e.get("ma"), e.get("mb")]}]
    out = []
    for q in [e] + e.get("parts", []):
        n = len(q["pts"])
        part = {"pts": [[p["x"], p["y"]] for p in q["pts"]], "ms": q.get("ms", [None] * n)}
        if "bulges" in q:
            part["bulges"] = q["bulges"]
        out.append(part)
    return out


def routes_of(layer, field):
    """The routes of a layer by id (the first object of an id), in object order, and how many ids came again."""
    by_id, again = {}, 0
    for e in ENTITIES:
        if e["layerId"] != layer or e["kind"] not in ("line", "polyline"):
            continue
        rid = js_trim(e["attrs"].get(field, ""))
        if not rid:
            continue
        if rid in by_id:
            again += 1
            continue
        by_id[rid] = e
    return by_id, again


def again_log(again):
    return [{"level": "warn", "text": f"{again} rota kimliği birden çok kez var; ilki kullanıldı."}] if again else []


def num(v):
    return None if v is None else float(v)


def part_out(q):
    """A route part written: pts, bulges only when one bends, ms."""
    out = {"pts": [P(float(p[0]), float(p[1])) for p in q["pts"]]}
    bulges = [float(b) for b in q.get("bulges", [])]
    if any(b != 0 for b in bulges):
        out["bulges"] = bulges
    out["ms"] = [num(m) for m in q["ms"]]
    return out


def polyline_of(parts, layer, attrs):
    first, *rest = [part_out(q) for q in parts]
    obj = {"kind": "polyline", "layerId": layer, "attrs": attrs, **first}
    if rest:
        obj["parts"] = rest
    return obj


def new_layer(name, color):
    lid = new_id(name, set())
    return lid, {"id": lid, "name": name, "style": {**style(color), "fill": f"{color}26"}}


def cell_reason(field, cell):
    return f"“{field}” boş" if not js_trim(cell) else f"“{field}” sayı değil: {js_trim(cell)}"


# ── The tools' runs ─────────────────────────────────────────────────────


def create_run(values):
    idf, method = values["idField"], values["method"]
    objs = [e for e in ENTITIES if e["layerId"] == "yol" and e["kind"] in ("line", "polyline", "arc")]
    pieces, owner = [], []
    for e in objs:
        rid = js_trim(e["attrs"].get(idf, ""))
        value = read_number(e["attrs"].get(values.get("lengthField", ""), "")) if method == "oneField" else None
        frm = read_number(e["attrs"].get(values.get("fromField", ""), "")) if method == "twoFields" else None
        to = read_number(e["attrs"].get(values.get("toField", ""), "")) if method == "twoFields" else None
        if e["kind"] == "line":
            paths = [([[e["a"]["x"], e["a"]["y"]], [e["b"]["x"], e["b"]["y"]]], [0])]
        elif e["kind"] == "arc":
            sweep = e["a1"] - e["a0"]
            if sweep <= 0:
                sweep += 2 * math.pi
            c, r = e["c"], e["r"]
            a = [c["x"] + r * math.cos(e["a0"]), c["y"] + r * math.sin(e["a0"])]
            b = [c["x"] + r * math.cos(e["a1"]), c["y"] + r * math.sin(e["a1"])]
            paths = [([a, b], [math.tan(sweep / 4)])]
        else:
            paths = [([[p["x"], p["y"]] for p in q["pts"]], q.get("bulges", [0] * (len(q["pts"]) - 1))) for q in [e] + e.get("parts", [])]
        for pts, bulges in paths:
            pieces.append({"id": rid, "pts": pts, "bulges": bulges, "value": value, "from": frm, "to": to})
            owner.append(e)
    rules = {"method": method, "priority": values.get("priority", "upperLeft"), "factor": values.get("factor", 1), "start": values.get("start", 0),
             "ignoreGaps": values.get("ignoreGaps", True)}
    got = L.create(pieces, rules)
    lid, layer = new_layer("Rotalar", "#E5484D")
    added = [polyline_of([{"pts": q[0], "bulges": q[1], "ms": q[2]} for q in r["parts"]], lid, {idf: r["id"]}) for r in got["routes"]]
    report, seen = [], set()
    for f in got["faults"]:
        e = owner[f["piece"]]
        if e["id"] in seen:
            continue
        seen.add(e["id"])
        a = e["attrs"]
        if f["why"] == "noId":
            why = f"“{idf}” boş"
        elif f["why"] == "noValue":
            why = cell_reason(values["lengthField"], a.get(values["lengthField"], ""))
        else:
            bad = values["fromField"] if read_number(a.get(values["fromField"], "")) is None else values["toField"]
            why = cell_reason(bad, a.get(bad, ""))
        report.append([f"#{e['id']}", why])
    expect = {"status": "ok", "summary": f"{len(objs)} nesneden {len(added)} rota yazıldı.", "undo": "Rota oluştur" if added else None}
    if added:
        expect["layers"] = [layer]
        expect["added"] = added
    outputs = {"count": len(added)}
    if report:
        expect["log"] = [{"level": "warn", "text": f"{len(report)} nesne atlandı; nedenleri tabloda."}]
        outputs["table"] = {"columns": ["Nesne", "Neden"], "rows": report}
    expect["outputs"] = outputs
    return expect


# lrs_cases.create returns parts as (pts, bulges, ms) tuples of mpf values.
def _patch_create():
    real = L.create

    def wrapped(pieces, rules):
        got = real(pieces, rules)
        for r in got["routes"]:
            r["parts"] = [(q["pts"], q["bulges"], q["ms"]) if isinstance(q, dict) else q for q in r["parts"]]
        return got

    L.create = wrapped


_patch_create()


def calibrate_run(values):
    routes, again = routes_of("kalib", values["routeField"])
    stones = [e for e in ENTITIES if e["layerId"] == "km" and e["kind"] == "point"]
    bad = sum(1 for e in stones if js_trim(e["attrs"].get(values["pointRoute"], "")) and read_number(e["attrs"].get(values["measure"], "")) is None)
    rules = {"radius": values["radius"], "before": values["before"], "after": values["after"], "ignoreGaps": values["ignoreGaps"]}
    updated, rows, done, used_all = [], [], 0, 0
    for rid, e in routes.items():
        marks = []
        for s in stones:
            if js_trim(s["attrs"].get(values["pointRoute"], "")) != rid:
                continue
            m = read_number(s["attrs"].get(values["measure"], ""))
            if m is None:
                continue
            marks.append(((s["p"]["x"], s["p"]["y"]), m))
        got = L.calibrate(L.route_of(route_parts(e)), marks, {**rules, "split": e["kind"] != "line"})
        used = sum(1 for u in got["used"] if u)
        if "fault" in got:
            rows.append([rid, str(used), "İkiden az nokta"])
            continue
        done += 1
        used_all += used
        rows.append([rid, str(used), "Kalibre edildi"])
        parts = got["parts"]
        # The run's update as the cases observe it: the attributes, then the changed geometry whole.
        if e["kind"] == "line":
            ms = parts[0][2]
            # A line's measure left unknown is no field at all (docs/adr/0224 §1).
            geometry = {"a": e["a"], "b": e["b"], **{k: num(m) for k, m in (("ma", ms[0]), ("mb", ms[1])) if m is not None}}
        else:
            first, *rest = [part_out({"pts": q[0], "bulges": q[1], "ms": q[2]}) for q in parts]
            geometry = dict(first)
            if rest:
                geometry["parts"] = rest
        updated.append({"id": e["id"], "attrs": e["attrs"], "geometry": geometry})
    expect = {"status": "ok", "summary": f"{len(routes)} rotadan {done} rota kalibre edildi; {used_all} nokta kullanıldı.",
              "undo": "Rotayı kalibre et" if updated else None}
    log = again_log(again)
    if bad:
        log.append({"level": "warn", "text": f"{bad} noktanın ölçüsü sayı değil; kullanılmadı."})
    if log:
        expect["log"] = log
    if updated:
        expect["updated"] = updated
    expect["outputs"] = {"count": done, "table": {"columns": ["Rota", "Kullanılan nokta", "Durum"], "rows": rows}}
    return expect


def _patch_calibrate():
    real = L.calibrate

    def wrapped(route, points, rules):
        got = real(route, points, rules)
        if "parts" in got:
            got["parts"] = [(q["pts"], q["bulges"], q["ms"]) for q in got["parts"]]
        return got

    L.calibrate = wrapped


_patch_calibrate()


FAULTS = {"noMeasures": "Rotanın ölçüsü yok", "same": "Başlangıç ve bitiş ölçüsü aynı", "offset": "Öteleme yapılamadı"}


def fault_text(f):
    if f["fault"] == "outside":
        return f"Ölçü rotanın dışında ({text(f['least'])} – {text(f['most'])})"
    return FAULTS[f["fault"]]


def events_run(values, file=None, objects=None):
    header, rows, labels = table_of(file) if file else objects_table(objects)
    kind = values["eventKind"]
    cols = [values["eventRoute"]] + ([values["measure"]] if kind == "point" else [values["from"], values["to"]]) + ([values["offset"]] if values.get("offset") else [])
    for c in cols:
        if c not in header:
            names = [h for h in header if h]
            return {"status": "error", "message": f"Kaynakta “{c}” alanı yok; alanları: {', '.join(names)}."}
    routes, again = routes_of("rota", values["routeField"])
    turn = -1 if values.get("side") == "left" else 1
    lid, layer = new_layer("Olaylar", "#8E4EC6")
    added, report = [], []
    for i, r in enumerate(rows):
        cell = lambda c: r[header.index(c)] if header.index(c) < len(r) else ""  # noqa: E731
        rid = js_trim(cell(values["eventRoute"]))
        nums, why = [], None
        for c in cols[1:3] if kind == "line" else cols[1:2]:
            v = read_number(cell(c))
            if v is None:
                why = cell_reason(c, cell(c))
                break
            nums.append(v)
        off = 0.0
        if why is None and values.get("offset"):
            raw = cell(values["offset"])
            if js_trim(raw):
                off = read_number(raw)
                if off is None:
                    why = cell_reason(values["offset"], raw)
        if why is None and rid not in routes:
            why = f"“{rid}” rotası yok"
        if why is not None:
            report.append([labels[i], why])
            continue
        route = L.route_of(route_parts(routes[rid]))
        attrs = {h: r[k] for k, h in enumerate(header) if k < len(r) and js_trim(r[k])}
        if kind == "point":
            got = L.point_at(route, nums[0], off * turn)
            if "fault" in got:
                report.append([labels[i], fault_text(got)])
                continue
            added.append({"kind": "point", "layerId": lid, "attrs": attrs, "p": P(float(got["p"][0]), float(got["p"][1]))})
        else:
            got = L.stretch(route, nums[0], nums[1], off * turn)
            if "fault" in got:
                report.append([labels[i], fault_text(got)])
                continue
            added.append(polyline_of([{"pts": q["pts"], "bulges": q["bulges"], "ms": q["ms"]} for q in got["parts"]], lid, attrs))
    expect = {"status": "ok", "summary": f"{len(rows)} satırdan {len(added)} olay yazıldı.", "undo": "Olayları rotada göster" if added else None}
    log = again_log(again)
    if report:
        log.append({"level": "warn", "text": f"{len(report)} satır atlandı; nedenleri tabloda."})
    if log:
        expect["log"] = log
    if added:
        expect["layers"] = [layer]
        expect["added"] = added
    outputs = {"count": len(added)}
    if report:
        outputs["table"] = {"columns": ["Satır", "Neden"], "rows": report}
    expect["outputs"] = outputs
    return expect


def locate_run(values, ids):
    routes, again = routes_of("rota", values["routeField"])
    order = list(routes.items())
    turn = -1 if values.get("side") == "left" else 1
    radius = mp.mpf(values["radius"])
    rows, updated, located = [], [], 0
    for i in ids:
        e = BY_ID[i]
        found = []  # (route id, m, from, to, distance)
        if e["kind"] == "point":
            near = []
            for k, (rid, re) in enumerate(order):
                q = L.locate(L.route_of(route_parts(re)), L.P(e["p"]["x"], e["p"]["y"]))
                if q is not None and abs(q["distance"]) <= radius:
                    near.append((abs(q["distance"]), k, rid, q))
            near.sort(key=lambda t: (t[0], t[1]))
            if not values["all"]:
                near = near[:1]
            found = [(rid, q["m"], None, None, q["distance"] * turn) for _, _, rid, q in near]
        elif e["kind"] in ("line", "polyline"):
            pts = [[e["a"]["x"], e["a"]["y"]], [e["b"]["x"], e["b"]["y"]]] if e["kind"] == "line" else [[p["x"], p["y"]] for p in e["pts"]]
            near = []
            for k, (rid, re) in enumerate(order):
                q = L.locate_line(L.route_of(route_parts(re)), pts, values["radius"])
                if "none" not in q:
                    near.append((q["distance"], k, rid, q))
            if not values["all"]:
                near.sort(key=lambda t: (t[0], t[1]))
                near = near[:1]
            found = [(rid, None, q["from"], q["to"], q["distance"]) for _, _, rid, q in near]
        else:
            rings = [[[p["x"], p["y"]] for p in e["pts"]]]
            for rid, re in order:
                for q in L.locate_area(L.route_of(route_parts(re)), rings):
                    found.append((rid, None, q["from"], q["to"], None))
        if found:
            located += 1
        for rid, m, f, t, d in found:
            rows.append([f"#{i}", rid, "" if m is None else text(m), "" if f is None else text(f), "" if t is None else text(t), "" if d is None else text(d)])
        if values.get("write") and found:
            rid, m, f, t, d = found[0]
            attrs = dict(e["attrs"])
            attrs["Rota"] = rid
            if m is not None:
                attrs["Ölçü"] = text(m)
            if f is not None:
                attrs["Başlangıç"] = text(f)
                attrs["Bitiş"] = text(t)
            if d is not None:
                attrs["Uzaklık"] = text(d)
            updated.append({"id": i, "attrs": attrs})
    expect = {"status": "ok", "summary": f"{len(ids)} nesneden {located} nesne konumlandı; {len(rows)} satır.", "undo": "Rota boyunca konumla" if updated else None}
    log = again_log(again)
    if log:
        expect["log"] = log
    if updated:
        expect["updated"] = updated
    expect["outputs"] = {"count": located, "table": {"columns": ["Nesne", "Rota", "Ölçü", "Başlangıç", "Bitiş", "Uzaklık"], "rows": rows}}
    return expect


def overlay_run(values):
    sides = []
    for prefix in ("a", "b"):
        if values.get(f"{prefix}SourceKind") == "layer":
            lid = values[f"{prefix}Source"]["layerId"]
            header, rows, _ = objects_table([e for e in ENTITIES if e["layerId"] == lid])
        else:
            header, rows, _ = table_of(values[f"{prefix}File"])
        keys = [values[f"{prefix}Route"], values[f"{prefix}From"], values[f"{prefix}To"]]
        events, kept, dropped = [], [], 0
        for r in rows:
            cell = lambda c: r[header.index(c)] if header.index(c) < len(r) else ""  # noqa: E731
            rid, f, t = js_trim(cell(keys[0])), read_number(cell(keys[1])), read_number(cell(keys[2]))
            if not rid or f is None or t is None:
                dropped += 1
                continue
            events.append({"route": rid, "from": f, "to": t})
            kept.append(r)
        others = [h for h in header if h not in keys]
        sides.append((events, kept, header, others, dropped))
    (ea, ka, ha, oa, da), (eb, kb, hb, ob, db) = sides
    got = L.overlay(ea, eb, values["how"] == "union")
    columns = ["Rota", "Başlangıç", "Bitiş"] + oa + [h + " (2)" if h in oa else h for h in ob]
    rows = []
    for g in got:
        row = [g["route"], text(g["from"]), text(g["to"])]
        for idx, kept, header, others in ((g["a"], ka, ha, oa), (g["b"], kb, hb, ob)):
            for h in others:
                k = header.index(h)
                row.append("" if idx is None or k >= len(kept[idx]) else kept[idx][k])
        rows.append(row)
    expect = {"status": "ok", "summary": f"{len(ea)} ve {len(eb)} olaydan {len(rows)} parça.", "undo": None}
    if da + db:
        expect["log"] = [{"level": "warn", "text": f"{da + db} satırın rotası ya da ölçüsü yok; alınmadı."}]
    expect["outputs"] = {"count": len(rows), "table": {"columns": columns, "rows": rows}}
    return expect


def extract_run(values):
    prefix = values["prefix"]
    objs = [e for e in ENTITIES if e["layerId"] == "rota" and e["kind"] in ("line", "polyline")]
    updated, without = [], 0
    for e in objs:
        s = L.summary(L.route_of(route_parts(e)))
        if s["count"] == 0:
            without += 1
            continue
        attrs = dict(e["attrs"])
        for key, v in (("bas", s["first"]), ("son", s["last"]), ("min", s["least"]), ("max", s["most"])):
            attrs[prefix + key] = text(v)
        updated.append({"id": e["id"], "attrs": attrs})
    expect = {"status": "ok", "summary": f"{len(objs)} rotadan {len(updated)} rotanın ölçüleri yazıldı.", "undo": "M değerlerini çıkar" if updated else None}
    if without:
        expect["log"] = [{"level": "info", "text": f"{without} nesnenin ölçüsü yok; yazılmadı."}]
    if updated:
        expect["updated"] = updated
    expect["outputs"] = {"count": len(updated)}
    return expect


def probes(parts):
    """A path's vertices, then each arc edge's midpoint."""
    out = []
    for q in parts:
        pts, bulges, ms = q
        out.extend(pts)
        for e in L.edges(q):
            if e.arc():
                out.append(e.at(mp.mpf("0.5"))[0])
    return out


def farthest(a, b):
    route = [(pts, bulges, [None] * len(pts)) for pts, bulges, _ in b]
    return max(abs(L.locate(route, p)["distance"]) for p in probes(a))


def line_drift(now, placed):
    return max(farthest(now, placed), farthest(placed, now))


def drift_run(values):
    routes, again = routes_of("rota", values["routeField"])
    line = values["eventKind"] == "line"
    fields = [values["eventRoute"]] + ([values["from"], values["to"]] if line else [values["measure"]])
    turn = -1 if values.get("side") == "left" else 1
    tol = mp.mpf(values["tolerance"])
    objs = [e for e in ENTITIES if e["layerId"] == "olay" and e["kind"] in ("point", "line", "polyline")]
    rows, updated, other, unplaced, drifted, total = [], [], 0, 0, 0, 0
    for e in objs:
        if (e["kind"] == "point") == line:
            other += 1
            continue
        total += 1
        cell = lambda f: e["attrs"].get(f, "")  # noqa: E731
        rid = js_trim(cell(fields[0]))
        nums, why = [], None
        for f in fields[1:]:
            v = read_number(cell(f))
            if v is None:
                why = cell_reason(f, cell(f))
                break
            nums.append(v)
        off = 0.0
        if why is None and values.get("offset") and js_trim(cell(values["offset"])):
            off = read_number(cell(values["offset"]))
            if off is None:
                why = cell_reason(values["offset"], cell(values["offset"]))
        if why is None and rid not in routes:
            why = f"“{rid}” rotası yok"
        if why is None:
            route = L.route_of(route_parts(routes[rid]))
            got = L.stretch(route, nums[0], nums[1], off * turn) if line else L.point_at(route, nums[0], off * turn)
            if "fault" in got:
                why = fault_text(got)
        if why is not None:
            unplaced += 1
            rows.append([f"#{e['id']}", rid, "", why])
            continue
        if line:
            placed = [(q["pts"], q["bulges"], q["ms"]) for q in got["parts"]]
            now = L.route_of(route_parts(e))
            d = line_drift(now, placed)
        else:
            d = L.dist(L.P(e["p"]["x"], e["p"]["y"]), got["p"])
        if d <= tol:
            continue
        drifted += 1
        said = "Kaydı"
        if values["fix"]:
            if e["kind"] == "point":
                geometry = {"p": P(float(got["p"][0]), float(got["p"][1]))}
            elif e["kind"] == "polyline":
                first, *rest = [part_out({"pts": q[0], "bulges": q[1], "ms": q[2]}) for q in placed]
                geometry = dict(first)
                if rest:
                    geometry["parts"] = rest
            elif len(placed) == 1 and len(placed[0][0]) == 2 and all(b == 0 for b in placed[0][1]):
                (a, b), ms = placed[0][0], placed[0][2]
                geometry = {"a": P(float(a[0]), float(a[1])), "b": P(float(b[0]), float(b[1])), "ma": num(ms[0]), "mb": num(ms[1])}
            else:
                geometry = None
            if geometry is None:
                said = "Kaydı; çizgi nesnesi kırıklı yola taşınamaz"
            else:
                said = "Yerine taşındı"
                updated.append({"id": e["id"], "attrs": e["attrs"], "geometry": geometry})
        rows.append([f"#{e['id']}", rid, text(d), said])
    summary = f"{total} olaydan {drifted} olay kaymış"
    if unplaced:
        summary += f", {unplaced} olay rotasında gösterilemiyor"
    if values["fix"] and updated:
        summary += f"; {len(updated)} olay yerine taşındı"
    expect = {"status": "ok", "summary": summary + ".", "undo": "Olay kaymasını denetle" if updated else None}
    log = again_log(again)
    if other:
        log.append({"level": "info", "text": f"{other} nesne olay türünde değil; denetlenmedi."})
    if drifted and not values["fix"]:
        log.append({"level": "warn", "text": f"{drifted} olay rotanın verdiği yerden kaymış; Yerine taşı ile düzeltilir."})
    if log:
        expect["log"] = log
    if updated:
        expect["updated"] = updated
    outputs = {"count": drifted}
    if rows:
        outputs["table"] = {"columns": ["Nesne", "Rota", "Kayma", "Durum"], "rows": rows}
    expect["outputs"] = outputs
    return expect


# ── The cases ────────────────────────────────────────────────────────────


def case(cid, title, tool, values, expect, selection=None):
    c = {"id": cid, "title": title, "document": "lrs.kcad", "run": {"tool": tool}, "values": values, "expect": expect}
    if selection is not None:
        c["selection"] = selection
    return c


def cases():
    yol = {"scope": "layer", "layerId": "yol"}
    rota = {"scope": "layer", "layerId": "rota"}
    create = {"input": yol, "idField": "Yol", "method": "length", "priority": "lowerLeft", "factor": 1, "start": 0, "ignoreGaps": True}
    gaps = {**create, "ignoreGaps": False, "start": 1000, "factor": 0.001}
    one = {**create, "method": "oneField", "lengthField": "Uzunluk"}
    two = {**create, "method": "twoFields", "fromField": "Bas", "toField": "Son"}
    calibrate = {"routes": {"scope": "layer", "layerId": "kalib"}, "routeField": "Rota", "points": {"scope": "layer", "layerId": "km"}, "pointRoute": "Rota",
                 "measure": "Km", "radius": 2, "before": True, "after": True, "ignoreGaps": True}
    points = {"routes": rota, "routeField": "Rota", "sourceKind": "file", "file": "lrs-noktalar.csv", "eventRoute": "Rota", "eventKind": "point",
              "measure": "Km", "offset": "Ofset", "side": "right"}
    lines = {"routes": rota, "routeField": "Rota", "sourceKind": "file", "file": "lrs-cizgiler.csv", "eventRoute": "Rota", "eventKind": "line",
             "from": "Bas", "to": "Son", "side": "left"}
    locate = {"input": {"scope": "layer", "layerId": "kaza"}, "routes": rota, "routeField": "Rota", "radius": 25, "all": False, "side": "right", "write": False}
    shapes = {**locate, "input": {"scope": "ids", "ids": [50, 51, 60]}, "radius": 2, "all": True}
    overlay = {"aSourceKind": "file", "aFile": "lrs-cizgiler.csv", "aRoute": "Rota", "aFrom": "Bas", "aTo": "Son",
               "bSourceKind": "file", "bFile": "lrs-hizlar.csv", "bRoute": "Rota", "bFrom": "Bas", "bTo": "Son", "how": "intersect"}
    olay = {"scope": "layer", "layerId": "olay"}
    drift = {"events": olay, "routes": rota, "routeField": "Rota", "eventRoute": "Rota", "eventKind": "point", "measure": "Km",
             "offset": "Ofset", "side": "right", "tolerance": 0.001, "fix": False}
    lines_drift = {**drift, "eventKind": "line", "from": "Bas", "to": "Son", "offset": "", "fix": True}
    return [
        case("create-length", "Rota oluştur, Uzunluk: parçalar sol alttan uç uca, ters çizilen çevrilir, boşluk sayılmaz; yaylı ve yay nesnesi; kimliksiz parça atlanır",
             "linearRef.createRoutes", create, create_run(create)),
        case("create-gaps", "Rota oluştur, Uzunluk: boşluk sayılır, çarpan 0,001 ve başlangıç 1000 (km)",
             "linearRef.createRoutes", gaps, create_run(gaps)),
        case("create-onefield", "Rota oluştur, Tek alan: her parça kendi uzunluk alanıyla; sayı olmayan alan atlanır",
             "linearRef.createRoutes", one, create_run(one)),
        case("create-twofields", "Rota oluştur, İki alan: her parça başlangıç ve bitiş ölçüsüyle, başlangıca göre sırada",
             "linearRef.createRoutes", two, create_run(two)),
        case("calibrate", "Rotayı kalibre et: çoklu çizgiye noktaların yerinde köşe, çizgi iki köşesini korur; tek noktalı rota ve sayı olmayan ölçü söylenir",
             "linearRef.calibrateRoutes", calibrate, calibrate_run(calibrate)),
        case("events-point", "Olayları rotada göster, nokta: ölçüde, sağa ötelemeyle; olmayan rota, dışarıdaki ölçü, ölçüsüz rota ve sayı olmayan ölçü atlanır",
             "linearRef.routeEvents", points, events_run(points, file="lrs-noktalar.csv")),
        case("events-line", "Olayları rotada göster, çizgi: başlangıçtan bitişe (ters yönlü olay ters çizilir), iki parçalı rotada iki parça; aynı ölçü atlanır",
             "linearRef.routeEvents", lines, events_run(lines, file="lrs-cizgiler.csv")),
        case("events-missing", "Olayları rotada göster: kaynakta olmayan sütun çalıştırmayı reddeder",
             "linearRef.routeEvents", {**points, "measure": "Kilometre"}, events_run({**points, "measure": "Kilometre"}, file="lrs-noktalar.csv")),
        case("locate-points", "Rota boyunca konumla, noktalar: yarıçaptaki en yakın rota, ölçü ve sağa artı uzaklık; uzaktaki nokta konumlanmaz",
             "linearRef.locateAlong", locate, locate_run(locate, [40, 41, 42, 43, 44])),
        case("locate-write", "Rota boyunca konumla: Özniteliklere yaz, sola artı uzaklık",
             "linearRef.locateAlong", {**locate, "write": True, "side": "left"}, locate_run({**locate, "write": True, "side": "left"}, [40, 41, 42, 43, 44])),
        case("locate-shapes", "Rota boyunca konumla, çizgi ve alan: çizginin uçlarının ölçüleri, alanın içindeki parça; yarıçapın dışındaki çizgi konumlanmaz",
             "linearRef.locateAlong", shapes, locate_run(shapes, [50, 51, 60])),
        case("overlay-intersect", "Olayları çakıştır, Kesişim: iki tablonun ortak parçaları, sütunlarıyla",
             "linearRef.overlayEvents", overlay, overlay_run(overlay)),
        case("overlay-union", "Olayları çakıştır, Birleşim: bir tablonun kapladığı parçalar da",
             "linearRef.overlayEvents", {**overlay, "how": "union"}, overlay_run({**overlay, "how": "union"})),
        case("extract", "M değerlerini çıkar: ilk, son, en küçük ve en büyük ölçü; ölçüsüz rota yazılmaz",
             "linearRef.extractMeasures", {"input": rota, "prefix": "M_"}, extract_run({"input": rota, "prefix": "M_"})),
        case("drift-points", "Olay kaymasını denetle, nokta: yerindeki ve toleransın içindeki olay söylenmez, kayan söylenir; olmayan rota, dışarıdaki ölçü ve sayı olmayan ölçü gösterilemez; çizgiler sayılır",
             "linearRef.eventDrift", drift, drift_run(drift)),
        case("drift-tolerance", "Olay kaymasını denetle: toleranstan az kayan olay yerinde sayılır",
             "linearRef.eventDrift", {**drift, "tolerance": 6}, drift_run({**drift, "tolerance": 6})),
        case("drift-lines-fix", "Olay kaymasını denetle, çizgi, Yerine taşı: köşesi kayan çoklu çizgi ve kayan çizgi taşınır, kırıklı yola düşen çizgi nesnesi taşınamaz; yaylı rotadaki olay yerinde",
             "linearRef.eventDrift", lines_drift, drift_run(lines_drift)),
    ] + more_cases(create, calibrate, points, overlay, drift, olay)


def more_cases(create, calibrate, points, overlay, drift, olay):
    """The options the first cases leave untried (docs/adr/0741 §2): the other three start corners, calibration without
    extrapolating before or after or with the gaps counted, events and overlays read from a layer's objects, and the
    drift's left side."""
    out = []
    for priority, name in (("upperLeft", "sol üst"), ("upperRight", "sağ üst"), ("lowerRight", "sağ alt")):
        v = {**create, "priority": priority}
        out.append(case(f"create-{priority}", f"Rota oluştur, Uzunluk, başlangıç {name} köşeden", "linearRef.createRoutes", v, create_run(v)))
    for key, name in (("before", "ilk noktadan öncesi uzatılmaz"), ("after", "son noktadan sonrası uzatılmaz"),
                      ("ignoreGaps", "boşluklar sayılır")):
        v = {**calibrate, key: False}
        out.append(case(f"calibrate-no-{key}", f"Rotayı kalibre et: {name}", "linearRef.calibrateRoutes", v, calibrate_run(v)))
    from_layer = {**{k: v for k, v in points.items() if k != "file"}, "sourceKind": "layer", "source": olay}
    olay_objects = [e for e in ENTITIES if e["layerId"] == "olay"]
    out.append(case("events-layer", "Olayları rotada göster, nokta, Olay nesneleri katmanından: nesnelerin öznitelikleri satırdır",
                    "linearRef.routeEvents", from_layer, events_run(from_layer, objects=olay_objects)))
    a_layer = {**{k: v for k, v in overlay.items() if k != "aFile"}, "aSourceKind": "layer", "aSource": olay}
    out.append(case("overlay-a-layer", "Olayları çakıştır, Kesişim: ilk olaylar Olay nesneleri katmanından; ölçüsüz nesneler alınmaz",
                    "linearRef.overlayEvents", a_layer, overlay_run(a_layer)))
    b_layer = {**{k: v for k, v in overlay.items() if k != "bFile"}, "bSourceKind": "layer", "bSource": olay, "how": "union"}
    out.append(case("overlay-b-layer", "Olayları çakıştır, Birleşim: ikinci olaylar Olay nesneleri katmanından",
                    "linearRef.overlayEvents", b_layer, overlay_run(b_layer)))
    left = {**drift, "side": "left"}
    out.append(case("drift-left", "Olay kaymasını denetle, nokta, öteleme solda: sağdaki olaylar kaymış sayılır", "linearRef.eventDrift",
                    left, drift_run(left)))
    return out


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def outputs():
    doc = {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL, "toleranceFields": ["bulges", "ms", "ma", "mb", "parts", "geometry"],
           "documents": {}, "files": {name: name for name in FILES}, "cases": cases()}
    return [(PROC / "lrs.json", text_of(doc).encode("utf-8")), (PROC / "lrs.kcad", text_of(DOCUMENT).encode("utf-8"))] + [
        (PROC / name, data.encode("utf-8")) for name, data in FILES.items()
    ]


def main():
    outs = outputs()
    if "--check" in sys.argv[1:]:
        bad = [p for p, data in outs if not p.exists() or p.read_bytes() != data]
        for p in bad:
            print(f"{p.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print("Doğrusal referans araçlarının durumları tutarlı: fixtures/processing/v1/lrs*")
        return 0
    for p, data in outs:
        p.write_bytes(data)
    print(f"written: {', '.join(str(p.relative_to(ROOT)) for p, _ in outs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
