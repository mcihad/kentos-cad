#!/usr/bin/env python3
"""The shared cases of `cad.parcels.report` and `cad.parcels.table` (Parsel alan çizelgesi; docs/adr/9306), without
KentOS code.

    scripts/fixtures/py scripts/fixtures/parcels_report_cases.py           # writes the two files below
    scripts/fixtures/py scripts/fixtures/parcels_report_cases.py --check   # writes nothing; compares

    fixtures/commands/v1/cad.parcels.report.json, fixtures/commands/v1/cad.parcels.table.json

The rules, by hand, as ADR 9306 §3–§5 states them:
- Scope: the areas `uids` names, in its order; else every area of the parcel layer `parsel`, in the drawing's order.
- Rings: each part's outer ring, then its holes. A ring is listed clockwise as the map shows it (x east, y north): one
  whose signed area (the shoelace, counter-clockwise positive, plus each arc's segment signed as its bulge) is positive
  is walked backwards, an arc's bulge turned. The first corner is the northernmost; of two as far north, the western;
  of two at one place, the earlier one walked.
- Names: with `names: points` (the default) a corner takes the name of the first named point object of the drawing
  (in its order) within 1 µm: its label trimmed, a multi-point object's k-th point “label (k)”. Every other corner,
  and with `names: numbers` every corner, takes `prefix` and a whole number from `start` (default 1) up by one; a
  number whose name a point of the drawing or the report has is passed. A place the report named before (1 µm, the
  first) keeps its name.
- An edge: the chord to the ring's next corner; an arc's radius r = c / (2 sin(θ/2)) and length r·θ, θ = 4 atan|b|.
- Areas: the ring's shoelace in rationals of the floats as stored, each arc's segment r²/2 (θ − sin θ) at 50 digits
  signed as its bulge; a ring's area the absolute value; a parcel's its outer rings' less its holes'.
- The record (`field`, default `Tapu alanı (m²)`): trimmed; a plain decimal is read, the difference (computed −
  recorded) and the 2022 regulation's Article 8(2) tolerance (archive-approved geometry, slices; a fraction between
  two slices by the lower one) beside it; the status within, outside, noRecord, invalidRecord.
- Heading: “{Ada} ada {Parsel} parsel” (each part when written), else the area's label, else “Alan {n}” by its place
  in the report; “ ({Mahalle})” after it when written.
- The words: the title “Parsel alan çizelgesi (EPSG:5254)”; the heads Nokta, Y (m), X (m), Kenar (m) with lengths,
  Yarıçap (m) when lengths are listed and an edge is an arc; per parcel its heading, “Delik n”, “Parça n” or “Parça n,
  delik m” above the rings after the first, each corner's name, x, y, chord and radius; Hesap alanı (m²), and with a
  record Tapu alanı (m²) as written, and when it reads Fark (m²) (“+” before a positive one) and Yanılma sınırı (m²)
  (“±”); with more than one parcel Toplam hesap alanı (m²). Numbers by the display rule (numeric_display.py): lengths
  with the project's length decimals, areas with its area decimals. Words from the drawing are made one line.
- The table: those words; the title, the headings and the ring names merged across the row, an area's words across
  all columns but the last; the first row the heading row, the heads centred and bold, the parcels' headings bold;
  the first column left, the others right; its layer the active one, its height the project's table height at its
  plot scale unless given.
"""

import copy
import json
import sys
import unicodedata
from fractions import Fraction
from pathlib import Path

import crmath as math  # correctly rounded: the same digits on every system (docs/adr/9303)
import mpmath as mp

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from numeric_display import shown  # noqa: E402  (the display rule's own reference)

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT_REPORT = ROOT / "fixtures" / "commands" / "v1" / "cad.parcels.report.json"
OUT_TABLE = ROOT / "fixtures" / "commands" / "v1" / "cad.parcels.table.json"
X0, Y0 = 487000.0, 4420000.0
FIELD = "Tapu alanı (m²)"
RULE = "tkgm-2022-m8-arsiv"
SRID = 5254
LENGTH_DECIMALS, AREA_DECIMALS = 3, 2
PLOT_SCALE = 1000.0
TOUCH = 1e-6
TITLE = "Parsel alan çizelgesi"
MAX_PREFIX, MAX_FIELD = 12, 120


def uid(n):
    return f"0192f5c0-9306-7000-8000-{n:012x}"


def layer(id_, name, visible=True, locked=False, children=None, kind="layer"):
    return {"id": id_, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def rect_ccw(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def rect_cw(x0, y0, x1, y1):
    return [P(x0, y0), P(x0, y1), P(x1, y1), P(x1, y0)]


MERKEZ = {"İl": "Ankara", "İlçe": "Çankaya", "Mahalle": "Merkez"}


def area_entity(n, pts, layer_="parsel", attrs=None, bulges=None, holes=None, parts=None, label=None):
    e = {"kind": "polygon", "id": n, "layerId": layer_, "attrs": dict(attrs or {}), "pts": pts}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = holes
    if parts:
        e["parts"] = parts
    if label is not None:
        e["label"] = label
    return e


def point(n, x, y, label=None, parts=None):
    e = {"kind": "point", "id": n, "layerId": "nokta", "attrs": {}, "p": P(x, y)}
    if label is not None:
        e["label"] = label
    if parts:
        e["parts"] = [{"p": P(px, py)} for px, py in parts]
    return e


ENTS = [
    # 1, 2, 3: three parcels side by side; 1 stored counter-clockwise, 2 clockwise, 3 with an arc on its north edge.
    area_entity(1, rect_ccw(0, 0, 20, 15), attrs={**MERKEZ, "Ada": "101", "Parsel": "1", FIELD: "301,2"}),
    area_entity(2, rect_cw(20, 0, 40, 15), attrs={**MERKEZ, "Ada": "101", "Parsel": "2", FIELD: "290"}),
    area_entity(3, rect_ccw(40, 0, 60, 15), bulges=[0.0, 0.0, 0.25, 0.0],
                attrs={**MERKEZ, "Ada": "101", "Parsel": "3"}),
    # 4: a parcel round an enclave; its record does not read (a thousands separator).
    area_entity(4, rect_cw(0, 30, 40, 60), holes=[{"pts": rect_ccw(10, 40, 20, 50)}],
                attrs={"İl": "Ankara", "İlçe": "Çankaya", "Mahalle": "Yenimahalle", "Ada": "102", "Parsel": "5",
                       FIELD: "1.250,50"}),
    # 5: a line on the parcel layer: no parcel.
    {"kind": "line", "id": 5, "layerId": "parsel", "attrs": {}, "a": P(0, -5), "b": P(50, -5)},
    # 6: two parts, the second with a hole; off the parcel layer, given by id.
    area_entity(6, rect_cw(100, 0, 110, 10), layer_="cizim", attrs={"Ada": "103", "Parsel": "1", FIELD: "200"},
                parts=[{"pts": rect_ccw(120, 0, 130, 10), "holes": [{"pts": rect_cw(122, 2, 124, 4)}]}]),
    # 7: a label and a mahalle, no Ada or Parsel; 8: nothing to name it by.
    area_entity(7, rect_cw(0, 80, 10, 85), layer_="cizim", attrs={"Mahalle": "Merkez"}, label="Yol fazlası"),
    area_entity(8, rect_cw(20, 80, 25, 84), layer_="cizim"),
    # The points: on 1's two northern corners, an unnamed one on 1's south-east corner, “1” and “2” away from every
    # corner (numbers to pass), a multi-point object whose second point stands on 2's and 3's north-east corner, and a
    # later point within 1 µm of 1's north-west corner (the first one names it).
    point(9, 0, 15, "P101"),
    point(10, 20, 15, "P102"),
    point(11, 20, 0),
    point(12, 200, 200, "1"),
    point(13, 210, 200, "2"),
    point(14, 300, 300, "K", parts=[(40, 15)]),
    point(15, 0.0000005, 15, "P199"),
    # 16: a parcel whose two northern corners are as far north, stored from its south-east corner, a corner of it
    # within 1 µm of 2's south-east corner (one place, one name).
    area_entity(16, [P(40.0000004, 0), P(40, -10), P(20, -10), P(20, 0)], layer_="cizim",
                attrs={"Ada": "101", "Parsel": "4", FIELD: " 200.10 "}),
]
IDS = [e["id"] for e in ENTS]
assert IDS == list(range(1, len(ENTS) + 1))
NEW_ID = len(ENTS) + 1


def setup():
    return {"format": "kentos.document", "version": 2, "name": "Parsel alan çizelgesi",
            "settings": {"srid": SRID, "lengthDecimals": LENGTH_DECIMALS, "areaDecimals": AREA_DECIMALS,
                         "areaUnit": "m2", "angleUnit": "grad", "plotScale": PLOT_SCALE, "workspace": "cad",
                         "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("parsel", "Parsel sınırı"), layer("nokta", "Noktalar"),
                       layer("cizim", "Çizim"), layer("tablo", "Tablolar"), layer("tablo2", "Çizelgeler"),
                       layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False),
                       layer("grup", "Grup", kind="group", children=[layer("ic", "İç")])],
            "activeLayer": "tablo", "entities": copy.deepcopy(ENTS), "uids": [uid(i) for i in IDS],
            "styles": {"items": [], "categories": []}}


# ── The geometry ─────────────────────────────────────────────────────────────────────────────────────────────────


def xy(p):
    return (p["x"], p["y"])


def bulge(bulges, i):
    return bulges[i] if bulges and i < len(bulges) else 0.0


def arc_of(a, b, bl):
    """An arc edge's radius, length and segment (signed as its bulge); none for a straight edge."""
    if abs(bl) <= 1e-12:
        return None
    chord = math.hypot(b[0] - a[0], b[1] - a[1])
    if chord < 1e-12:
        return None
    t = 4 * mp.atan(abs(mp.mpf(bl)))
    r = mp.mpf(chord) / (2 * mp.sin(t / 2))
    seg = r * r / 2 * (t - mp.sin(t))
    return r, r * t, seg if bl > 0 else -seg


def signed_area(pts, bulges):
    """Counter-clockwise positive: the shoelace in rationals, each arc's segment at 50 digits."""
    q = [(Fraction(x), Fraction(y)) for x, y in pts]
    n = len(q)
    s = sum(q[i][0] * q[(i + 1) % n][1] - q[(i + 1) % n][0] * q[i][1] for i in range(n)) / 2
    total = mp.mpf(s.numerator) / s.denominator
    for i in range(n):
        arc = arc_of(pts[i], pts[(i + 1) % n], bulge(bulges, i))
        if arc:
            total += arc[2]
    return total


def listed(pts, bulges):
    """The ring clockwise from its northernmost corner: (place, the bulge of the edge to the next)."""
    n = len(pts)
    if signed_area(pts, bulges) > 0:
        walked = [(pts[(n - k) % n], -bulge(bulges, (n - k - 1) % n)) for k in range(n)]
    else:
        walked = [(pts[k], bulge(bulges, k)) for k in range(n)]
    start = 0
    for k in range(n):
        p, b = walked[k][0], walked[start][0]
        if p[1] > b[1] or (p[1] == b[1] and p[0] < b[0]):
            start = k
    return [walked[(start + k) % n] for k in range(n)]


def rings(e):
    """(part, hole, places, bulges) of each ring in order."""
    out = [(0, None, [xy(p) for p in e["pts"]], e.get("bulges"))]
    for h, ring in enumerate(e.get("holes", [])):
        out.append((0, h, [xy(p) for p in ring["pts"]], ring.get("bulges")))
    for k, part in enumerate(e.get("parts", []), start=1):
        out.append((k, None, [xy(p) for p in part["pts"]], part.get("bulges")))
        for h, ring in enumerate(part.get("holes", [])):
            out.append((k, h, [xy(p) for p in ring["pts"]], ring.get("bulges")))
    return out


# ── The record, by the 2022 regulation ───────────────────────────────────────────────────────────────────────────

SLICES = [(0, 0, 0.0, 0.05), (11, 10, 0.5, 0.02), (101, 100, 2.3, 0.01), (501, 500, 6.3, 0.005),
          (1001, 1000, 8.8, 0.004), (5001, 5000, 24.8, 0.003), (25001, 25000, 84.8, 0.0015)]


def tolerance(a):
    _, start, base, rate = [s for s in SLICES if a >= s[0]][-1]
    return base + (a - start) * rate


def read(text):
    whole, sep, frac = text.replace(",", ".").partition(".")
    if not whole.isdigit() or not whole.isascii() or (sep and not (frac.isdigit() and frac.isascii())):
        return None
    if text.count(".") + text.count(",") > 1:
        return None
    return float(text.replace(",", "."))


def one_line(words):
    out = "".join(" " if unicodedata.category(c) == "Cc" else c for c in words)
    return out[:999] + "…" if len(out) > 1000 else out


# ── The report ───────────────────────────────────────────────────────────────────────────────────────────────────


def point_names(ents):
    out = []
    for e in ents:
        if e["kind"] != "point":
            continue
        label = (e.get("label") or "").strip()
        if not label:
            continue
        label = one_line(label)
        out.append((xy(e["p"]), label))
        for k, part in enumerate(e.get("parts", []), start=2):
            out.append((xy(part["p"]), f"{label} ({k})"))
    return out


def near(places, p):
    for i, (q, *_rest) in enumerate(places):
        if math.hypot(q[0] - p[0], q[1] - p[1]) <= TOUCH:
            return i
    return None


class Namer:
    def __init__(self, ents, names, prefix, start):
        self.names, self.prefix, self.next = names, prefix, start
        self.points = point_names(ents)
        self.taken = {name for _, name in self.points}
        self.given = []
        self.named = self.numbered = 0

    def name(self, p):
        i = near(self.given, p)
        if i is not None:
            return self.given[i][1], self.given[i][2]
        j = near(self.points, p) if self.names == "points" else None
        if j is not None:
            got = (self.points[j][1], "point")
            self.named += 1
        else:
            while f"{self.prefix}{self.next}" in self.taken:
                self.next += 1
            got = (f"{self.prefix}{self.next}", "number")
            self.next += 1
            self.numbered += 1
        self.taken.add(got[0])
        self.given.append((p, *got))
        return got


def heading(attrs, label, place):
    ada, parsel, mahalle = (attrs.get(k, "").strip() for k in ("Ada", "Parsel", "Mahalle"))
    words = ([f"{ada} ada"] if ada else []) + ([f"{parsel} parsel"] if parsel else [])
    out = " ".join(words) if words else (label or f"Alan {place}")
    if mahalle:
        out += f" ({mahalle})"
    return one_line(out)


def ring_words(part, hole):
    if part == 0:
        return None if hole is None else f"Delik {hole + 1}"
    return f"Parça {part + 1}" if hole is None else f"Parça {part + 1}, delik {hole + 1}"


def fixed_len(v):
    return shown(v, LENGTH_DECIMALS)


def fixed_area(v):
    return shown(v, AREA_DECIMALS)


def report(uids=None, names="points", prefix="", start=1, lengths=True, field=None, ents=None):
    """The report's output, and the table's merges and bold rows."""
    ents = ENTS if ents is None else ents
    by_id = {e["id"]: e for e in ents}
    chosen = [by_id[i] for i in uids] if uids is not None else [
        e for e in ents if e["layerId"] == "parsel" and e["kind"] == "polygon"]
    field = (field or "").strip() or FIELD
    namer = Namer(ents, names, prefix, start)
    parcels = []
    for place, e in enumerate(chosen, start=1):
        attrs = e["attrs"]
        out_rings, area = [], mp.mpf(0)
        for part, hole, pts, bulges in rings(e):
            corners = []
            ring = listed(pts, bulges)
            for k, (p, b) in enumerate(ring):
                q = ring[(k + 1) % len(ring)][0]
                name, how = namer.name(p)
                c = {"name": name, "named": how, "at": {"x": p[0], "y": p[1]},
                     "length": math.hypot(q[0] - p[0], q[1] - p[1])}
                arc = arc_of(p, q, b)
                if arc:
                    c["radius"] = float(arc[0])
                    c["arc"] = float(arc[1])
                corners.append(c)
            ring_area = abs(signed_area(pts, bulges))
            area += -ring_area if hole is not None else ring_area
            r = {"part": part, "corners": corners, "area": float(ring_area)}
            if hole is not None:
                r["hole"] = hole
            out_rings.append(r)
        area = float(area)
        text = attrs.get(field, "").strip() or None
        rec = read(text) if text else None
        diff = area - rec if rec is not None else None
        tol = tolerance(area)
        status = "noRecord" if text is None else "invalidRecord" if rec is None else \
            "within" if abs(diff) <= tol else "outside"
        g = lambda k: attrs.get(k, "").strip()  # noqa: E731
        p = {"uid": uid(e["id"]), "il": g("İl"), "ilce": g("İlçe"), "mahalle": g("Mahalle"), "ada": g("Ada"),
             "parsel": g("Parsel"), "heading": heading(attrs, (e.get("label") or "").strip(), place),
             "rings": out_rings, "area": area}
        if text is not None:
            p["recordedText"] = one_line(text)
        if rec is not None:
            p["recorded"] = rec
            p["difference"] = diff
        p["tolerance"] = tol
        p["status"] = status
        parcels.append(p)
    radii = lengths and any("radius" in c for p in parcels for r in p["rings"] for c in r["corners"])
    m = 3 + int(lengths) + int(radii)
    cells, merges, bold = [], [], []

    def row(words):
        return words + [""] * (m - len(words))

    def across(cols):
        return {"row": len(cells), "col": 0, "rows": 1, "cols": cols}

    def area_row(words, value):
        merges.append(across(m - 1))
        r = row([words])
        r[m - 1] = value
        cells.append(r)

    merges.append(across(m))
    cells.append(row([f"{TITLE} (EPSG:{SRID})"]))
    bold.append(len(cells))
    cells.append(["Nokta", "Y (m)", "X (m)"] + (["Kenar (m)"] if lengths else []) + (["Yarıçap (m)"] if radii else []))
    for p in parcels:
        bold.append(len(cells))
        merges.append(across(m))
        cells.append(row([p["heading"]]))
        for r in p["rings"]:
            words = ring_words(r["part"], r.get("hole"))
            if words:
                merges.append(across(m))
                cells.append(row([words]))
            for c in r["corners"]:
                line = [c["name"], fixed_len(c["at"]["x"]), fixed_len(c["at"]["y"])]
                if lengths:
                    line.append(fixed_len(c["length"]))
                if radii:
                    line.append(fixed_len(c["radius"]) if "radius" in c else "")
                cells.append(line)
        area_row("Hesap alanı (m²)", fixed_area(p["area"]))
        if "recordedText" in p:
            area_row("Tapu alanı (m²)", p["recordedText"])
        if "difference" in p:
            d = p["difference"]
            text = fixed_area(d)
            area_row("Fark (m²)", "+" + text if d > 0 and any(ch in "123456789" for ch in text) else text)
            area_row("Yanılma sınırı (m²)", "±" + fixed_area(p["tolerance"]))
    total = float(sum((mp.mpf(p["area"]) for p in parcels), mp.mpf(0)))
    if len(parcels) > 1:
        area_row("Toplam hesap alanı (m²)", fixed_area(total))
    out = {"srid": SRID, "rule": RULE, "field": field, "parcels": parcels, "area": total,
           "places": len(namer.given), "named": namer.named, "numbered": namer.numbered, "cells": cells}
    return out, merges, bold, m


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────

SAME = {"revision": "same", "canUndo": False}
UNCHANGED = {"canUndo": False, "dirty": False, "revision": "same", "ids": IDS}


def report_input(uids=None, names=None, prefix=None, start=None, lengths=None, field=None):
    out = {}
    if uids is not None:
        out["uids"] = [uid(i) for i in uids]
    for k, v in (("names", names), ("prefix", prefix), ("start", start), ("lengths", lengths), ("field", field)):
        if v is not None:
            out[k] = v
    return out


def kwargs(uids=None, names=None, prefix=None, start=None, lengths=None, field=None):
    return {"uids": uids, "names": names or "points", "prefix": prefix or "", "start": 1 if start is None else start,
            "lengths": True if lengths is None else lengths, "field": field}


def report_ok(name, op="plan", **given):
    out, *_ = report(**kwargs(**given))
    return {"name": name, "steps": [{"op": op, "input": report_input(**given), "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}


def refused(name, input_, code, message, path_, expect=None):
    return {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": expect or SAME}]}


def report_cases():
    whole, *_ = report()
    assert [p["status"] for p in whole["parcels"]] == ["within", "outside", "noRecord", "invalidRecord"], whole
    assert whole["named"] == 3 and whole["numbered"] == 13, whole
    out = [
        report_ok("plan: parsel katmanının bütün parselleri; köşeler noktalardan, olmayana yeni numara; ortak köşe tek ad"),
        report_ok("uygula: salt okunur, çizim değişmez", op="execute"),
        report_ok("doğrulama", op="validate"),
        report_ok("verilen sırayla: önce 2, sonra 1", uids=[2, 1]),
        report_ok("hepsine yeni numara: K öneki, 101'den", names="numbers", prefix="K", start=101),
        report_ok("hepsine yeni numara, önekisiz: noktaların adları (1, 2) atlanır", names="numbers"),
        report_ok("kenarsız: üç sütun, yay yarıçapı da yok", lengths=False),
        report_ok("yaylı kenar: kiriş, yarıçap ve yay boyu; alan yay parçasıyla", uids=[3]),
        report_ok("delikli parsel: delik saat yönünde, alanı düşülür; tapu alanı okunmaz", uids=[4]),
        report_ok("çok parçalı alan: ikinci parça ve deliği; kayıt sınır dışında", uids=[6]),
        report_ok("başlık: adı ve mahallesi; adı olmayan alan sırasıyla", uids=[7, 8]),
        report_ok("saat yönünün tersine saklanan halka saat yönünde, kuzeybatıdan", uids=[1]),
        report_ok("aynı kuzeyde iki köşe: batıdaki başlar; 1 µm yakındaki köşe aynı yer", uids=[2, 16]),
        report_ok("kayıt sınır içinde: fark ve yanılma sınırı", uids=[16]),
        report_ok("çok noktalı nesnenin ikinci noktası köşeyi adlandırır", uids=[3, 2]),
        report_ok("başka öznitelikten kayıtlı alan", uids=[1, 2], field="Parsel"),
        report_ok("boş öznitelik adı: Tapu alanı (m²)", uids=[1], field="   "),
        report_ok("ilk numara 0", uids=[8], names="numbers", start=0),
        report_ok("boş liste: parsel yok, yalnız başlıklar", uids=[]),
        refused("çizgi bir parsel değil", {"uids": [uid(1), uid(5)]}, "not_a_parcel",
                f"“{uid(5)}” bir alan değil; parsel alan çizelgesi yalnız alanları listeler.", "uids[1]"),
        refused("kimlik UUID değil", {"uids": ["x"]}, "invalid_uid", "“x” geçerli bir nesne kimliği değil.", "uids[0]"),
        refused("çizimde olmayan nesne", {"uids": [uid(99)]}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uids[0]"),
        refused("önek 12 harften uzun", {"prefix": "ÇOKUZUNBİRÖNEK"}, "invalid_input",
                "Numara öneki “ÇOKUZUNBİRÖNEK” 12 harften uzun; daha kısa bir önek yazın.", "prefix"),
        refused("önekte satır sonu", {"prefix": "P\n"}, "invalid_input",
                "Numara önekinde satır sonu ya da denetim karakteri var; önek tek satırlık bir yazı olmalı.", "prefix"),
        refused("öznitelik adı 120 harften uzun", {"field": "a" * 121}, "invalid_input",
                "Kayıtlı alanın öznitelik adı 120 harften uzun; tapu alanını tutan özniteliğin adını yazın.", "field"),
    ]
    return out


# ── The table ────────────────────────────────────────────────────────────────────────────────────────────────────


def table_fields(at, height, layer_id, **given):
    out, merges, bold, m = report(**kwargs(**given))
    aligns = ["left"] + ["right"] * (m - 1)
    styles = []
    for r in bold:
        s = {"row": r, "col": 0, "rows": 1, "cols": m}
        if r == 1:
            s["align"] = "center"
        s["bold"] = True
        styles.append(s)
    fields = {"kind": "table", "layerId": layer_id, "p": at, "rotation": 0.0, "height": height,
              "cells": out["cells"], "merges": merges, "aligns": aligns, "header": True, "styles": styles}
    return out, fields


def table_input(at, height=None, layer_id=None, expected=None, **given):
    out = report_input(**given)
    out["at"] = at
    if layer_id is not None:
        out["layerId"] = layer_id
    if height is not None:
        out["height"] = height
    if expected is not None:
        out["expectedRevision"] = expected
    return out


AT = {"x": X0 + 80.0, "y": Y0 + 60.0}
TABLE_HEIGHT = 2.5 / 1000.0 * PLOT_SCALE


def table_plan(name, at=AT, height=None, layer_id=None, warnings=(), **given):
    out, _ = table_fields(at, height or TABLE_HEIGHT, layer_id or "tablo", **given)
    return {"name": name, "steps": [{"op": "plan", "input": table_input(at, height, layer_id, **given), "result": {
        "status": "completed", "output": {"table": "$any", "parcels": len(out["parcels"]), "rows": len(out["cells"]),
                                          "numbered": out["numbered"], "revision": "$any"},
        "warnings": list(warnings)}, "expect": {"canUndo": False, "dirty": False, "revision": "same", "ids": IDS}}]}


def table_written(name, at=AT, height=None, layer_id=None, warnings=(), undo=False, **given):
    out, fields = table_fields(at, height or TABLE_HEIGHT, layer_id or "tablo", **given)
    steps = [{"op": "execute", "input": table_input(at, height, layer_id, **given), "result": {
        "status": "completed", "output": {"uid": "$any", "parcels": len(out["parcels"]), "rows": len(out["cells"]),
                                          "numbered": out["numbered"], "revision": "$any"},
        "warnings": list(warnings)},
        "expect": {"ids": IDS + [NEW_ID], "canUndo": True, "dirty": True, "revision": "changed",
                   "fields": {str(NEW_ID): fields}}}]
    if undo:
        steps.append({"op": "undo", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "dirty": True}})
    return {"name": name, "steps": steps}


HIDDEN = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}


def table_cases():
    return [
        table_plan("plan: parsel katmanının çizelgesi, hiçbir şey yazılmaz"),
        table_written("uygula: tablo etkin katmana tek adımda; geri alınınca gider", undo=True),
        {"name": "doğrulama", "steps": [{"op": "validate", "input": table_input(AT), "result": {
            "status": "completed", "output": None, "warnings": []}, "expect": UNCHANGED}]},
        table_written("yazı yüksekliği ve katman verilmiş", height=2.0, layer_id="tablo2"),
        table_written("kenarsız, seçili iki parsel", uids=[2, 1], lengths=False),
        table_written("yaylı parsel: yarıçap sütunu", uids=[3]),
        table_written("delikli ve çok parçalı parseller", uids=[4, 6]),
        table_written("yeni numaralar K öneki ile", names="numbers", prefix="K", start=1),
        table_written("kaydı sınır içinde tek parsel: toplam satırı yok", uids=[16]),
        table_written("gizli katmana yazılır, uyarılır", uids=[1], layer_id="gizli", warnings=[HIDDEN]),
        table_plan("plan: başlıksız alanlar", uids=[7, 8]),
        refused("kilitli katman", table_input(AT, layer_id="kilitli"), "layer_locked",
                "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "layerId", UNCHANGED),
        refused("olmayan katman", table_input(AT, layer_id="yok"), "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId", UNCHANGED),
        refused("katman grubu", table_input(AT, layer_id="grup"), "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId",
                UNCHANGED),
        refused("yazı yüksekliği sıfır", table_input(AT, height=0.0), "invalid_table",
                "Tablonun yazı yüksekliği 0; sıfırdan büyük ve sonlu olmalı.", "height", UNCHANGED),
        {"name": "köşesi sonlu olmalı", "steps": [{"op": "execute", "input": table_input(AT),
                                                   "nonFinite": {"at.x": "Infinity"}, "result": {
            "status": "failed", "error": {"code": "not_finite", "message": "$any", "path": "at.x"}},
            "expect": UNCHANGED}]},
        {"name": "çizim değişmiş", "steps": [{"op": "execute", "input": table_input(AT, expected="999"), "result": {
            "status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision",
                                            "revision": "$current"}}, "expect": UNCHANGED}]},
        refused("sürüm yazısı değil", table_input(AT, expected="x"), "invalid_revision", "$any", "expectedRevision",
                UNCHANGED),
        refused("yazılacak parsel yok", table_input(AT, uids=[]), "no_objects",
                "Çizelgeye yazılacak parsel yok: parsel katmanında alan yok ya da hiçbir alan verilmedi. Parselleri "
                "seçin ya da parsel katmanına alan çizin.", "uids", UNCHANGED),
        refused("çizgi bir parsel değil", table_input(AT, uids=[5]), "not_a_parcel",
                f"“{uid(5)}” bir alan değil; parsel alan çizelgesi yalnız alanları listeler.", "uids[0]", UNCHANGED),
        refused("kimlik UUID değil", {"uids": ["x"], "at": AT}, "invalid_uid", "“x” geçerli bir nesne kimliği değil.",
                "uids[0]", UNCHANGED),
        refused("çizimde olmayan nesne", {"uids": [uid(99)], "at": AT}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uids[0]", UNCHANGED),
        refused("önek 12 harften uzun", {"prefix": "ÇOKUZUNBİRÖNEK", "at": AT}, "invalid_input",
                "Numara öneki “ÇOKUZUNBİRÖNEK” 12 harften uzun; daha kısa bir önek yazın.", "prefix", UNCHANGED),
    ]


def write(path, command, title, note, cases):
    doc = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
           "note": note, "setup": setup(), "cases": cases}
    return path, json.dumps(doc, ensure_ascii=False, indent=1) + "\n"


def main():
    note = ("ADR 9306. Bağımsız başvuru scripts/fixtures/parcels_report_cases.py: halkalar saat yönünde ve en "
            "kuzeydeki köşeden, adlar noktalardan ya da yeni numara, alan rasyonel shoelace ve 50 basamaklı yay "
            "parçası, yanılma sınırı 2022 Yönetmeliği Madde 8(2) tablosundan, sözler gösterim kuralıyla; KentOS kodu yok.")
    files = [
        write(OUT_REPORT, "cad.parcels.report", "Parsel alan çizelgesi", note, report_cases()),
        write(OUT_TABLE, "cad.parcels.table", "Parsel alan çizelgesi tablosu", note, table_cases()),
    ]
    if "--check" in sys.argv[1:]:
        stale = [p for p, text in files if not p.exists() or p.read_text("utf-8") != text]
        for p in stale:
            print(f"{p.relative_to(ROOT)} güncel değil.")
        if not stale:
            print("Parsel alan çizelgesi ortak durumları güncel.")
        return 1 if stale else 0
    for p, text in files:
        p.write_text(text, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)} ({len(json.loads(text)['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
