#!/usr/bin/env python3
"""The shared cases of `map.features.reconcile` (plan/map-cad.md MAP-04, docs/adr/1760), written from the ADR without
KentOS code.

    uv run --with shapely==2.0.6 python3 scripts/fixtures/map_features_reconcile_cases.py           # writes the cases
    uv run --with shapely==2.0.6 python3 scripts/fixtures/map_features_reconcile_cases.py --check   # compares

The drawing is a map project with the KentOS profile `harita-varliklari`'s layers for poles, manholes, trees, doors and
kerbs, an accepted pole already on its layer, and the candidates of a LiDAR and an orthophoto extraction on their own
layers (agreeing poles, a duplicate, a pole/tree conflict, a single manhole, a pole by the accepted one, a code no class
has, doors without their required number, kerbs as polylines, a candidate reconciled before, a polyline coded as a
pole). The rules (ADR 1760), written here by hand:

- a candidate: a point or an open polyline (two vertices or more) on a source's layer; its class: a decision's
  correction, else its field's text (trimmed) as a class id, else the class whose codes hold the capitals its first
  word begins with (upper-cased); one keeping `Eşleştirme` (or the earlier `Uzlaştırma`) is `reviewed`; a rejected one `rejected`; no class
  `unknown`; a point class on a polyline or a line class on a point (or an area class) `wrongKind`;
- the confidence: the field given, a finite number; none is below any number;
- groups: candidates of one kind within the radius (points: the plane's distance; polylines: the larger of the two
  discrete Hausdorff ways, each vertex to the other's segments), joined transitively, in the order of their first;
- a source's representative: its member with the highest confidence (a tie to the first); its other members
  `duplicate`; a line group's geometry: the representative with the highest confidence (a tie to the earlier source);
- the place: a point group's representatives' mean (z when all have one), the spread their largest distance from it;
  a line group's chosen geometry's first vertex, the spread the largest Hausdorff between representatives;
- `existing` when an object of the group's one class's layer (not a candidate) lies within the radius of a
  representative; else `conflict` (classes differ), `agreed` (two sources or more), `single`;
- the fields: the representatives' (confidence, then source order), but the class field, the confidence field,
  `Eşleştirme`, `Uzlaştırma` and the provenance's; the decisions' fields over them; missing: a required field not given;
- written: agreed, or single with an accept, and nothing missing; the asset on the class's layer with the fields, the
  class field = the class's first code, Kaynaklar, Adaylar, Yayılım (3 decimals); every member of a written or existing
  group marked `Eşleştirme` = the asset, a rejected one `Reddedildi: reason`. One undo step “Detay eşleştirme”;
  nothing to write or mark writes nothing.
- ADR 4140: the decisions are review notes, not the asset's attributes: each decision that says something (accept, a
  class, fields, a reason) and whose candidate is marked becomes, in the input's order, a resolved note numbered after
  the drawing's, its author the reviewer (trimmed, none when blank), its time `reviewedAt` (trimmed), bound to the
  candidate and the asset (none for a rejection) with their stamps (`$any`), its text “Detay eşleştirme, <source>
  adayı: <what>.” (kabul edildi; sınıfı “<title>” olarak düzeltildi; alanları verildi (k = v, …), or “n alanı verildi”
  past 200 characters; reddedildi (reason), or “reddedildi (nedeni kararda)” past 200), its decision the one
  given (fields and reason trimmed) with the source's name and the asset; the output's `notes` their numbers. A source's
  name, the reviewer and the time must be short labels (120, 120, 64 characters, no edge space, no control character);
  a reason and a field's value a note's text (trimmed, at most 4000, no control character but a line break or a tab;
  a blank value is left out of the note as of the asset), a field's name a short label (64).

ADR 2140 adds areas, by shapely (GEOS): a closed polygon (three vertices or more, area above 0) is a candidate of an area
class; two are one group when their overlap ratio, intersection area over union area, exceeds `overlap` (0.5 when not
given; it must lie strictly between 0 and 1); an area group's footprint is its most trusted representative's (as a
line group's), its place that footprint's centroid, its spread the largest Hausdorff between the representatives'
closed outer rings, `overlap` the smallest ratio between two representatives (none with one); an area of the class's
layer overlapping a representative above the ratio makes it `existing`; the written asset is the footprint with
`Örtüşme` (3 decimals) beside Yayılım.

ADR 3340 measures a polyline's arcs as arcs: an edge of bulge b has the included angle 4·atan b, the radius
c(1 + b²)/(4|b|) and its centre on the chord's left normal at c(1 − b²)/(4b) from its middle; a polyline's points for
the discrete Hausdorff are its vertices and each arc's points at its sweep's seven inner eighths; a point's distance to
an arc is to its circle when its direction from the centre lies within the sweep, else to the nearer end. The written
line keeps its bulges.

ADR 2540 adds the class layer's field rules: when the class's layer has fields (the profile applied: a field with a
domain carries the domain's list), each value of the asset (its candidates' and the decisions') that a field names
passes ADR 0199's rule, written here for the kinds the cases use — a list's label (Turkish case folded) gives its code,
else the trimmed text must be one of its codes; a text no longer than its length; an integer of digits (leading zeros
dropped); a decimal with a point or a comma, written with a point — and the canonical text is written. A value that does
not fit is the group's `invalid` (field and the rule's sentence) and keeps the group from being written.
"""

import copy
import json
import math
import re
import sys
import unicodedata
from pathlib import Path

from shapely.geometry import Polygon

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "map.features.reconcile.json"
PROFILE = json.loads((ROOT / "catalog/v1/profiles/harita-varliklari.json").read_text("utf-8"))
CLASSES = PROFILE["classes"]
X0, Y0 = 500000.0, 4400000.0
G = "Harita varlıkları"
CLASS_LAYERS = {"direk": "hv-drk", "rogar": "hv-rog", "agac": "hv-agc", "kapi": "hv-kap", "bordur": "hv-brd",
                "bina-izi": "hv-bna", "yesil-alan": "hv-ysl"}
PROVENANCE = ("Kaynaklar", "Adaylar", "Yayılım", "Örtüşme", "İnceleme", "Gözden geçiren")
SOURCES = [{"layer": "aday-lidar", "name": "LiDAR"}, {"layer": "aday-orto", "name": "Ortofoto"}]
WITHIN = 1e-9


def layer(id_, name, kind="layer", locked=False, children=()):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": list(children)}


def uid(n):
    return f"0192f5c0-1760-7000-8000-{n:012x}"


def pt(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def point(layer_id, attrs, x, y, z=None):
    e = {"kind": "point", "layerId": layer_id, "attrs": attrs, "p": pt(x, y)}
    if z is not None:
        e["z"] = z
    return e


def line(layer_id, attrs, pts, bulges=None):
    e = {"kind": "polyline", "layerId": layer_id, "attrs": attrs, "pts": [pt(x, y) for x, y in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def area(layer_id, attrs, x0, y0, x1, y1):
    """A rectangle's footprint, counter-clockwise."""
    return {"kind": "polygon", "layerId": layer_id, "attrs": attrs,
            "pts": [pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)]}


L, O = "aday-lidar", "aday-orto"
OBJECTS = [
    point("hv-drk", {"Kod": "DRK", "Ad": "D-90", "Tur": "Aydınlatma"}, 90.0, 5.0),           # 1 an accepted pole
    point(L, {"Kod": "DRK", "Güven": "0.92", "Tur": "Aydınlatma"}, 10.0, 5.0, 851.20),        # 2 agreed pole …
    point(O, {"Kod": "DRK", "Güven": "0.81", "Ad": "D-10"}, 10.18, 5.10, 851.26),             # 3 … with the orthophoto's
    point(L, {"Kod": "DRK 2", "Güven": "0.88"}, 30.0, 5.0, 852.00),                           # 4 agreed pole …
    point(L, {"Kod": "DRK", "Güven": "0.40"}, 30.12, 4.95, 852.10),                           # 5 … its duplicate
    point(O, {"Kod": "direk", "Güven": "0.77", "Tur": "Elektrik"}, 29.90, 5.20),              # 6 … the orthophoto's
    point(L, {"Kod": "DRK", "Güven": "0.65"}, 50.0, 5.0),                                     # 7 a pole …
    point(O, {"Kod": "AGC", "Güven": "0.70", "Cins": "Çınar"}, 50.20, 5.10),                  # 8 … or a tree
    point(O, {"Kod": "ROG", "Güven": "0.55", "Sebeke": "Atıksu"}, 70.0, 8.0),                 # 9 a single manhole
    point(L, {"Kod": "DRK", "Güven": "0.90"}, 90.05, 5.02),                                   # 10 by the accepted pole
    point(L, {"Kod": "XYZ", "Güven": "0.50"}, 110.0, 5.0),                                    # 11 a code no class has
    point(L, {"Kod": "KAP", "Güven": "0.90"}, 130.0, 12.0),                                   # 12 a door …
    point(O, {"Kod": "KAP", "Güven": "0.80"}, 130.10, 12.05),                                 # 13 … no door number
    line(L, {"Kod": "BRD", "Güven": "0.86"}, [(0.0, 0.0), (20.0, 0.0), (40.0, 0.10)]),       # 14 a kerb …
    line(O, {"Kod": "BRD", "Güven": "0.93", "Ad": "Bordür 1"}, [(0.10, 0.15), (20.05, 0.10), (39.90, 0.20)]),  # 15
    point(L, {"Kod": "DRK", "Güven": "0.95", "Uzlaştırma": uid(0x99)}, 150.0, 5.0),          # 16 matched before (old name)
    line(L, {"Kod": "DRK", "Güven": "0.60"}, [(170.0, 0.0), (172.0, 0.0)]),                  # 17 a pole drawn as a line
]

def arc_points(a, b, bulge, offset):
    """ADR 3340: the points of the arc from `a` to `b` of `bulge` at its sweep's ends and eighths, `offset` metres off its
    circle (outwards), as a second extraction draws the same kerb by its vertices."""
    c, r, a0, sweep = arc_of(a, b, bulge)
    return [(c[0] + (r + offset) * math.cos(a0 + sweep * e / 8.0), c[1] + (r + offset) * math.sin(a0 + sweep * e / 8.0))
            for e in range(9)]


# ADR 3340: kerbs with arcs. An arc 3 m off its chord and a straight line along that chord (the chord would make them
# one group); an arc 1 m off its chord and the same kerb drawn by its vertices 2 cm off the arc.
def arcs():
    return [
        line(L, {"Kod": "BRD", "Güven": "0.90"}, [(0.0, 0.0), (20.0, 0.0)], [0.3, 0.0]),               # 1 an arc …
        line(O, {"Kod": "BRD", "Güven": "0.80"}, [(0.0, 0.05), (20.0, 0.05)]),                        # 2 … its chord
        line(L, {"Kod": "BRD", "Güven": "0.95"}, [(40.0, 0.0), (60.0, 0.0)], [0.1, 0.0]),              # 3 an arc …
        line(O, {"Kod": "BRD", "Güven": "0.85"}, arc_points((40.0, 0.0), (60.0, 0.0), 0.1, 0.02)),     # 4 … by its vertices
    ]

# ADR 2140: building footprints and green areas from the two extractions.
AREAS = [
    area("hv-bna", {"Kod": "BNA", "Ad": "B-1"}, 200.0, 0.0, 212.0, 10.0),                    # 1 an accepted building
    area(L, {"Kod": "BNA", "Güven": "0.90", "KatSayisi": "3"}, 100.0, 0.0, 112.0, 10.0),     # 2 a building …
    area(O, {"Kod": "BNA", "Güven": "0.80", "Ad": "A-1"}, 100.5, 0.4, 112.4, 10.3),          # 3 … the orthophoto's
    area(L, {"Kod": "BNA", "Güven": "0.50"}, 100.2, 0.1, 111.8, 9.9),                        # 4 … a duplicate
    area(L, {"Kod": "BNA", "Güven": "0.85"}, 130.0, 0.0, 140.0, 10.0),                       # 5 a building …
    area(O, {"Kod": "BNA", "Güven": "0.75"}, 136.0, 0.0, 146.0, 10.0),                       # 6 … overlapping 0.25
    area(L, {"Kod": "BNA", "Güven": "0.95"}, 200.2, 0.1, 212.1, 10.2),                       # 7 on the accepted one
    area(L, {"Kod": "YSL", "Güven": "0.70", "Ad": "Park"}, 160.0, 0.0, 180.0, 15.0),         # 8 a green area …
    area(O, {"Kod": "BNA", "Güven": "0.60"}, 161.0, 0.5, 179.0, 14.5),                       # 9 … or a building
    area(O, {"Kod": "DRK", "Güven": "0.66"}, 220.0, 0.0, 221.0, 1.0),                        # 10 a pole drawn as an area
    point(L, {"Kod": "BNA", "Güven": "0.55"}, 230.0, 5.0),                                   # 11 a building as a point
]


def class_fields(cid):
    """A class's fields as applying the profile puts them on its layer: a domain's list, its id kept (ADR 1360)."""
    domains = {dm["id"]: dm for dm in PROFILE["domains"]}
    out = []
    for f in next(k for k in CLASSES if k["id"] == cid).get("fields", []):
        f = copy.deepcopy(f)
        if "domain" in f:
            f["values"] = copy.deepcopy(domains[f["domain"]]["values"])
        out.append(f)
    return out


def setup(objects=OBJECTS, missing_layers=(), locked=(), fields=False):
    kids = [layer(CLASS_LAYERS[c], next(k for k in CLASSES if k["id"] == c)["layer"][-1], locked=CLASS_LAYERS[c] in locked)
            for c in CLASS_LAYERS if c not in missing_layers]
    if fields:
        for k, c in zip(kids, [c for c in CLASS_LAYERS if c not in missing_layers]):
            k["fields"] = class_fields(c)
    layers = [layer("0", "0"), layer("hv", G, "group", children=kids), layer(L, "Adaylar (LiDAR)", locked=L in locked),
              layer(O, "Adaylar (ortofoto)", locked=O in locked), layer("cizim", "Çizim")]
    gone = {CLASS_LAYERS[c] for c in missing_layers}
    ents = []
    for i, e in enumerate(objects, 1):
        x = copy.deepcopy(e)
        x["id"] = i
        if x["layerId"] in gone:
            # An object on a class's layer the drawing does not have stays in the drawing, on Çizim.
            x["layerId"] = "cizim"
        ents.append(x)
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"}
    if fields:
        settings["domains"] = copy.deepcopy(PROFILE["domains"])
    return {"format": "kentos.document", "version": 2, "name": "Detay eşleştirme",
            "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "cizim", "entities": ents,
            "uids": [uid(i) for i in range(1, len(ents) + 1)], "styles": {"items": [], "categories": []}}


# ── The rules ────────────────────────────────────────────────────────────────────────────────────────────────────


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(code)
        self.code, self.message, self.path = code, message, path


def class_named(text):
    for i, c in enumerate(CLASSES):
        if c["id"] == text:
            return i
    words = text.split()
    if not words:
        return None
    first = words[0].upper()
    letters = ""
    for ch in first:
        if "A" <= ch <= "Z":
            letters += ch
        else:
            break
    if not letters:
        return None
    return next((i for i, c in enumerate(CLASSES) if letters in c["codes"]), None)


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def to_segment(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    if l2 == 0.0:
        return dist(p, a)
    t = min(max(((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2, 0.0), 1.0)
    return dist(p, (a[0] + t * dx, a[1] + t * dy))


def arc_of(a, b, bulge):
    """ADR 3340: an edge's arc (centre, radius, start angle, signed sweep); none when straight."""
    if abs(bulge) <= 1e-12:
        return None
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    if chord < 1e-12:
        return None
    k = (1.0 - bulge * bulge) / (4.0 * bulge)
    c = ((a[0] + b[0]) / 2.0 - dy * k, (a[1] + b[1]) / 2.0 + dx * k)
    r = chord * (1.0 + bulge * bulge) / (4.0 * abs(bulge))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), 4.0 * math.atan(bulge)


def edge_bulge(bulges, k):
    return bulges[k] if bulges and k < len(bulges) else 0.0


def samples(pts, bulges=None):
    """The vertices and each arc's points at its sweep's seven inner eighths, in order."""
    out = []
    for k, p in enumerate(pts):
        out.append(p)
        if k + 1 < len(pts):
            arc = arc_of(p, pts[k + 1], edge_bulge(bulges, k))
            if arc is not None:
                c, r, a0, sweep = arc
                for e in range(1, 8):
                    angle = a0 + sweep * e / 8.0
                    out.append((c[0] + r * math.cos(angle), c[1] + r * math.sin(angle)))
    return out


def within(arc, angle):
    _, _, a0, sweep = arc
    turn = 2.0 * math.pi
    return (angle - a0) % turn <= sweep if sweep >= 0 else (a0 - angle) % turn <= -sweep


def to_curve(p, pts, bulges=None):
    if len(pts) == 1:
        return dist(p, pts[0])
    best = math.inf
    for k in range(len(pts) - 1):
        arc = arc_of(pts[k], pts[k + 1], edge_bulge(bulges, k))
        if arc is None:
            d = to_segment(p, pts[k], pts[k + 1])
        else:
            c, r, _, _ = arc
            if within(arc, math.atan2(p[1] - c[1], p[0] - c[0])):
                d = abs(dist(p, c) - r)
            else:
                d = min(dist(p, pts[k]), dist(p, pts[k + 1]))
        best = min(best, d)
    return best


def one_way(a, b, ba=None, bb=None):
    return max((to_curve(p, b, bb) for p in samples(a, ba)), default=0.0)


def hausdorff(a, b, ba=None, bb=None):
    return max(one_way(a, b, ba, bb), one_way(b, a, bb, ba))


def iou(a, b):
    """Two polygons' overlap ratio: their intersection's area over their union's (shapely, GEOS)."""
    shared = a.intersection(b).area
    union = a.area + b.area - shared
    return shared / union if union > 0 else 0.0


def better(a, b):
    ca, cb = a["confidence"], b["confidence"]
    if ca is not None and cb is not None:
        return ca > cb
    return ca is not None and cb is None


def xy(p):
    return (p["x"], p["y"])


def layers_of(d):
    out = {}

    def walk(nodes):
        for n in nodes:
            out[n["id"]] = n
            walk(n.get("children", []))
    walk(d["layers"])
    return out


def path_of(d, lid):
    def walk(nodes, trail):
        for n in nodes:
            t = trail + [n["name"]]
            if n["id"] == lid:
                return t
            got = walk(n.get("children", []), t)
            if got:
                return got
        return None
    return " / ".join(walk(d["layers"], []) or [lid])


def fold(text):
    """Turkish case folding, a character at a time (I → ı, İ → i)."""
    return "".join({"I": "ı", "İ": "i"}.get(c, c.lower() if len(c.lower()) == 1 else c) for c in text)


def check(field, text):
    """ADR 0199's rule for a value of `field`, for the kinds the cases use: (canonical, None) or (None, why)."""
    label = field.get("alias") or field["name"]
    s = text.strip()
    values = field.get("values") or []
    for v in values:
        if fold(v["label"].strip()) == fold(s):
            return v["code"], None
    kind = field["kind"]
    if kind == "integer":
        if not re.fullmatch(r"[+-]?[0-9]+", s):
            return None, f"“{label}” alanı tam sayı ister; “{text}” verildi. Rakamlarla, ondalıksız yazın."
        minus, digits = s.startswith("-"), s.lstrip("+-").lstrip("0") or "0"
        value = f"-{digits}" if minus and digits != "0" else digits
        assert "min" not in field or int(value) >= int(field["min"]), "a range the cases do not reach"
        assert "max" not in field or int(value) <= int(field["max"]), "a range the cases do not reach"
    elif kind == "decimal":
        m = re.fullmatch(r"([+-]?)([0-9]*)(?:[.,]([0-9]*))?", s)
        if not m or not (m.group(2) or m.group(3)):
            return None, (f"“{label}” alanı ondalık sayı ister; “{text}” verildi. Rakamlarla, ondalığı nokta ya da "
                          "virgülle yazın.")
        whole = m.group(2).lstrip("0") or "0"
        fraction = m.group(3) or ""
        assert len(fraction) <= field.get("scale", 99), "a scale the cases do not reach"
        value = whole + (f".{fraction}" if fraction else "")
        assert not (m.group(1) == "-") and ("min" not in field or float(value) >= float(field["min"])), "a range"
    else:
        length = field.get("length")
        if length and len(text) > length:
            return None, f"“{label}” alanı en çok {length} karakter alır; {len(text)} karakter verildi."
        value = s if values else text
    if values and value not in [v["code"] for v in values]:
        return None, f"“{label}” alanı listedeki değerlerden birini ister; “{text}” listede yok."
    return value, None


def layer_fields(d, layer_id):
    for n in d["layers"]:
        for k in [n, *n.get("children", [])]:
            if k["id"] == layer_id:
                return k.get("fields", [])
    return []


def class_layer(d, ci):
    want = CLASSES[ci]["layer"]
    for n in d["layers"]:
        if n["name"] == want[0] and n["type"] == "group":
            for k in n.get("children", []):
                if k["name"] == want[1] and k["type"] == "layer":
                    return k["id"]
    return None


def analyse(d, inp):
    """ADR 1760's answer for the drawing and the input, and what execute writes; or Refused."""
    if inp.get("name") != "harita-varliklari":
        raise Refused("unknown_profile", f"“{inp.get('name')}” adlı veri profili yok; KentOS'un profilleri: tucbs-temel, "
                                         "harita-varliklari, imar-plani.", "name")
    sources = inp.get("sources", [])
    if not 1 <= len(sources) <= 8:
        raise Refused("invalid_input", "Aday kaynağı 1 ile 8 arasında olmalı; adayların katmanlarını verin.", "sources")
    lays = layers_of(d)
    for i, s in enumerate(sources):
        if not s["name"].strip():
            raise Refused("invalid_input", "Kaynağın adı boş olamaz (LiDAR, Ortofoto …).", f"sources[{i}].name")
        if not label_ok(s["name"].strip(), 120):
            raise Refused("invalid_input", "Kaynağın adı en çok 120 karakter olabilir ve denetim karakteri taşıyamaz.",
                          f"sources[{i}].name")
        if any(o["layer"] == s["layer"] for o in sources[:i]):
            raise Refused("invalid_input", f"“{s['layer']}” katmanı iki kaynakta verilmiş.", f"sources[{i}].layer")
        if s["layer"] not in lays:
            raise Refused("layer_not_found", f"“{s['layer']}” kimlikli katman çizimde yok.", f"sources[{i}].layer")
    field = inp.get("field", "Kod")
    if not field.strip():
        raise Refused("invalid_input", "Sınıf alanının adı boş olamaz.", "field")
    radius = inp["radius"]
    if not (math.isfinite(radius) and radius > 0):
        raise Refused("invalid_input", "Eşleme yarıçapı 0'dan büyük olmalı (m).", "radius")
    least = inp.get("overlap", 0.5)
    if not (math.isfinite(least) and 0.0 < least < 1.0):
        raise Refused("invalid_input", "Örtüşme oranı 0 ile 1 arasında olmalı (kesişimin alanı bölü birleşiminki; "
                                       "verilmezse 0,5).", "overlap")
    # ADR 4140: the review notes' author and time.
    if (inp.get("reviewer") or "").strip() and not label_ok(inp["reviewer"].strip(), 120):
        raise Refused("invalid_input", "Gözden geçirenin adı en çok 120 karakter olabilir ve denetim karakteri "
                                       "taşıyamaz.", "reviewer")
    if (inp.get("reviewedAt") or "").strip() and not label_ok(inp["reviewedAt"].strip(), 64):
        raise Refused("invalid_input", "Gözden geçirme zamanı en çok 64 karakter olabilir "
                                       "(“2026-11-09T16:00:00+03:00”).", "reviewedAt")
    src = {s["layer"]: i for i, s in enumerate(sources)}
    by_uid = {u: e for u, e in zip(d["uids"], d["entities"])}
    decided = {}
    for i, dc in enumerate(inp.get("decisions", [])):
        at = f"decisions[{i}]"
        u = dc["candidate"]
        if len(u) != 36 or u.lower() != u or u.count("-") != 4:
            raise Refused("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil.", f"{at}.candidate")
        if u not in by_uid:
            raise Refused("entity_not_found", f"“{u}” kimlikli nesne çizimde yok.", f"{at}.candidate")
        if by_uid[u]["layerId"] not in src:
            raise Refused("invalid_input", f"“{u}” bir aday değil: kaynakların katmanlarında değil.", f"{at}.candidate")
        if "class" in dc and not any(c["id"] == dc["class"] for c in CLASSES):
            raise Refused("invalid_input", f"“{dc['class']}” “harita-varliklari” profilinin bir sınıfı değil.",
                          f"{at}.class")
        if dc.get("accept") and "reject" in dc:
            raise Refused("invalid_input", "Bir aday hem kabul hem ret edilemez.", f"{at}.reject")
        if "reject" in dc and not dc["reject"].strip():
            raise Refused("invalid_input", "Ret nedeni boş olamaz.", f"{at}.reject")
        if "reject" in dc and not text_ok(dc["reject"].strip()):
            raise Refused("invalid_input", "Ret nedeni en çok 4000 karakter olabilir ve satır sonu ile sekme dışında "
                                           "denetim karakteri taşıyamaz.", f"{at}.reject")
        for k, v in sorted(dc.get("fields", {}).items()):
            # A blank value is left out, as from the asset.
            if not label_ok(k, 64) or (v.strip() and not text_ok(v.strip())):
                raise Refused("invalid_input", f"“{k}” alanının adı ya da değeri uzun ya da denetim karakterli: "
                                               "inceleme notuna yazılamaz.", f"{at}.fields")
        decided[u] = dc
    conf = inp.get("confidence")
    cands = []
    for u, e in zip(d["uids"], d["entities"]):
        if e["layerId"] not in src:
            continue
        poly = None
        bulges = None
        if e["kind"] == "point":
            kind, pts = "point", [xy(e["p"])]
        elif e["kind"] == "polyline" and not e.get("closed") and len(e["pts"]) >= 2:
            kind, pts = "line", [xy(p) for p in e["pts"]]
            bulges = list(e.get("bulges", []))[:len(pts) - 1]
        elif e["kind"] == "polygon" and len(e["pts"]) >= 3:
            ring = [xy(p) for p in e["pts"]]
            poly = Polygon(ring)
            if not poly.area > 0:
                continue
            kind, pts = "area", ring + [ring[0]]
        else:
            continue
        is_line = kind == "line"
        attrs = e.get("attrs", {})
        read = attrs.get(field, "").strip() or None
        dc = decided.get(u)
        ci = None
        if dc and "class" in dc:
            ci = next(i for i, c in enumerate(CLASSES) if c["id"] == dc["class"])
        elif read is not None:
            ci = class_named(read)
        confidence = None
        if conf and conf in attrs:
            try:
                v = float(attrs[conf].strip())
                confidence = v if math.isfinite(v) else None
            except ValueError:
                confidence = None
        if attrs.get("Eşleştirme", "").strip() or attrs.get("Uzlaştırma", "").strip():
            status = "reviewed"
        elif dc and "reject" in dc:
            status = "rejected"
        elif ci is None:
            status = "unknown"
        else:
            g = CLASSES[ci]["geometry"]
            status = "member" if g == kind else "wrongKind"
        cands.append({"uid": u, "source": src[e["layerId"]], "class": ci, "read": read, "confidence": confidence,
                      "line": is_line, "kind": kind, "poly": poly, "status": status, "attrs": attrs, "pts": pts,
                      "bulges": bulges, "z": e.get("z"), "entity": e})
    eligible = [i for i, c in enumerate(cands) if c["status"] == "member"]
    # Components by breadth-first search from each candidate not reached, in order.
    seen, comps = set(), []
    for i in eligible:
        if i in seen:
            continue
        comp, queue = [], [i]
        seen.add(i)
        while queue:
            k = queue.pop(0)
            comp.append(k)
            for j in eligible:
                if j in seen or cands[j]["kind"] != cands[k]["kind"]:
                    continue
                a, b = cands[k], cands[j]
                if a["kind"] == "area":
                    joined = iou(a["poly"], b["poly"]) > least
                else:
                    gap = hausdorff(a["pts"], b["pts"], a["bulges"], b["bulges"]) if a["line"] else \
                        dist(a["pts"][0], b["pts"][0])
                    joined = gap <= radius
                if joined:
                    seen.add(j)
                    queue.append(j)
        comps.append(sorted(comp))
    groups = []
    cand_uids = {c["uid"] for c in cands}
    for members in comps:
        reps = []
        for m in members:
            k = next((x for x, r in enumerate(reps) if cands[r]["source"] == cands[m]["source"]), None)
            if k is None:
                reps.append(m)
            elif better(cands[m], cands[reps[k]]):
                reps[k] = m
        classes = sorted({cands[m]["class"] for m in members})
        is_line = cands[members[0]]["line"]
        kind = cands[members[0]]["kind"]
        srcs = sorted({cands[r]["source"] for r in reps})
        smallest = None
        if kind == "area":
            for a in range(len(reps)):
                for b in range(a + 1, len(reps)):
                    o = iou(cands[reps[a]]["poly"], cands[reps[b]]["poly"])
                    smallest = o if smallest is None else min(smallest, o)
        if kind != "point":
            chosen = reps[0]
            for r in reps[1:]:
                a, b = cands[r], cands[chosen]
                if better(a, b) or (a["confidence"] == b["confidence"] and a["source"] < b["source"]):
                    chosen = r
            spread = 0.0
            for a in range(len(reps)):
                for b in range(a + 1, len(reps)):
                    spread = max(spread, hausdorff(cands[reps[a]]["pts"], cands[reps[b]]["pts"],
                                                   cands[reps[a]]["bulges"], cands[reps[b]]["bulges"]))
            if kind == "area":
                c = cands[chosen]["poly"].centroid
                x, y = c.x, c.y
            else:
                x, y = cands[chosen]["pts"][0]
            z = None
            reps = [chosen] + [r for r in reps if r != chosen]
        else:
            n = float(len(reps))
            sx = sy = 0.0
            for r in reps:
                sx += cands[r]["pts"][0][0]
                sy += cands[r]["pts"][0][1]
            x, y = sx / n, sy / n
            z = None
            if all(cands[r]["z"] is not None for r in reps):
                sz = 0.0
                for r in reps:
                    sz += cands[r]["z"]
                z = sz / n
            spread = max(dist(cands[r]["pts"][0], (x, y)) for r in reps)
        existing = None
        if len(classes) == 1:
            lid = class_layer(d, classes[0])
            if lid is not None:
                for u, e in zip(d["uids"], d["entities"]):
                    if e["layerId"] != lid or u in cand_uids:
                        continue
                    if e["kind"] == "point" and kind == "point":
                        near = any(dist(cands[r]["pts"][0], xy(e["p"])) <= radius for r in reps)
                    elif e["kind"] == "polyline" and is_line and len(e["pts"]) >= 2:
                        near = any(hausdorff(cands[r]["pts"], [xy(p) for p in e["pts"]], cands[r]["bulges"],
                                             list(e.get("bulges", []))) <= radius for r in reps)
                    elif e["kind"] == "polygon" and kind == "area" and len(e["pts"]) >= 3:
                        there = Polygon([xy(p) for p in e["pts"]])
                        near = there.area > 0 and any(iou(cands[r]["poly"], there) > least for r in reps)
                    else:
                        near = False
                    if near:
                        existing = u
                        break
        status = "existing" if existing else ("conflict" if len(classes) > 1 else ("agreed" if len(srcs) >= 2 else "single"))
        trust = sorted(reps, key=lambda r: (-(cands[r]["confidence"]) if cands[r]["confidence"] is not None else math.inf,
                                            cands[r]["source"]))
        attrs = {}
        for r in trust:
            for k, v in cands[r]["attrs"].items():
                if k == field or k == conf or k in ("Eşleştirme", "Uzlaştırma") or k in PROVENANCE or not v.strip():
                    continue
                attrs.setdefault(k, v)
        accepted, review = False, []
        for m in members:
            dc = decided.get(cands[m]["uid"])
            if not dc:
                continue
            for k, v in dc.get("fields", {}).items():
                if v.strip():
                    attrs[k] = v
            if dc.get("accept"):
                accepted = True
                review.append("kabul")
            if "class" in dc:
                review.append(f"sınıf {dc['class']}")
            if dc.get("fields"):
                review.append("alan " + ", ".join(sorted(dc["fields"])))
        klass = classes[0] if len(classes) == 1 else None
        missing = []
        if klass is not None:
            for f in CLASSES[klass].get("fields", []):
                if f.get("required") and not attrs.get(f["name"], "").strip():
                    missing.append(f["name"])
        # ADR 2540: the values through the class layer's field rules; the canonical text written.
        invalid = []
        if klass is not None and class_layer(d, klass) is not None:
            for f in layer_fields(d, class_layer(d, klass)):
                v = attrs.get(f["name"])
                if v is None or not v.strip():
                    continue
                canonical, why = check(f, v)
                if why is None:
                    attrs[f["name"]] = canonical
                else:
                    invalid.append({"field": f["name"], "message": why})
        writes = (status == "agreed" or (status == "single" and accepted)) and not missing and not invalid
        if writes and class_layer(d, klass) is None:
            c = CLASSES[klass]
            raise Refused("class_layer_missing", f"“{c['title']}” sınıfının katmanı ({' › '.join(c['layer'])}) çizimde "
                                                 "yok; önce “harita-varliklari” profilini uygulayın.", "name")
        row = {"classes": [CLASSES[c]["id"] for c in classes], "status": status,
               "sources": [sources[s]["name"] for s in srcs], "candidates": [cands[m]["uid"] for m in members],
               "x": x, "y": y, "spread": spread, "writes": writes}
        if smallest is not None:
            row["overlap"] = smallest
        if klass is not None:
            row["class"] = CLASSES[klass]["id"]
        if z is not None:
            row["z"] = z
        if existing:
            row["asset"] = existing
        if missing:
            row["missing"] = missing
        if invalid:
            row["invalid"] = invalid
        groups.append({"members": members, "reps": reps, "row": row, "class": klass, "attrs": attrs, "review": review})
    for g in groups:
        for m in g["members"]:
            if m not in g["reps"]:
                cands[m]["status"] = "duplicate"
    return cands, groups


def label_ok(t, most):
    """A review note's short label (ADR 4140, ADR 1000 §2): not empty, no space at its edges, at most `most` characters,
    no control character."""
    return t != "" and t.strip() == t and len(t) <= most and not any(unicodedata.category(c) == "Cc" for c in t)


def text_ok(t):
    """A review note's text: not empty, no edge space, at most 4000 characters, no control character but a line break
    or a tab."""
    return (t != "" and t.strip() == t and len(t) <= 4000
            and not any(unicodedata.category(c) == "Cc" and c not in "\n\t" for c in t))


def given_fields(dc):
    """A decision's fields that say something, trimmed (a blank value is left out, as from the asset)."""
    return {k: v.strip() for k, v in sorted(dc.get("fields", {}).items()) if v.strip()}


def decision_words(dc, source):
    """ADR 4140: what a decision did, in its note's words."""
    parts = []
    if dc.get("accept"):
        parts.append("kabul edildi")
    if "class" in dc:
        title = next((c["title"] for c in CLASSES if c["id"] == dc["class"]), dc["class"])
        parts.append(f"sınıfı “{title}” olarak düzeltildi")
    fields = given_fields(dc)
    if fields:
        given = ", ".join(f"{k} = {v}" for k, v in fields.items())
        parts.append(f"alanları verildi ({given})" if len(given) <= 200 else f"{len(fields)} alanı verildi")
    if "reject" in dc:
        r = dc["reject"].strip()
        parts.append(f"reddedildi ({r})" if len(r) <= 200 else "reddedildi (nedeni kararda)")
    return f"Detay eşleştirme, {source} adayı: {'; '.join(parts)}."


def answer(cands, groups, sources, written=(), marked=0, notes=()):
    group_of = {m: gi for gi, g in enumerate(groups) for m in g["members"]}
    rows = []
    for i, c in enumerate(cands):
        r = {"uid": c["uid"], "source": sources[c["source"]]["name"]}
        if c["class"] is not None:
            r["class"] = CLASSES[c["class"]]["id"]
        if c["read"] is not None:
            r["read"] = c["read"]
        if c["confidence"] is not None:
            r["confidence"] = c["confidence"]
        if i in group_of:
            r["group"] = group_of[i]
        r["status"] = c["status"]
        rows.append(r)
    out = {"groups": [g["row"] for g in groups], "candidates": rows}
    if written:
        out["written"] = list(written)
    out["marked"] = marked
    if notes:
        out["notes"] = list(notes)
    out["revision"] = "$current"
    return out


def executed(d, inp):
    """Execute: the answer, the objects after (by slot: their fields) and the slots there after."""
    cands, groups = analyse(d, inp)
    sources = inp["sources"]
    lays = layers_of(d)
    field = inp.get("field", "Kod")
    written_classes = {g["class"] for g in groups if g["row"]["writes"]}
    for ci in sorted(written_classes):
        lid = class_layer(d, ci)
        if lays[lid]["locked"]:
            raise Refused("layer_locked", f"“{path_of(d, lid)}” katmanı kilitli: eşleştirilen varlık oraya yazılamaz. "
                                          "Kilidi açın.", "name")
    marks_any = any(g["row"]["writes"] or g["row"]["status"] == "existing" for g in groups) or any(
        c["status"] == "rejected" for c in cands)
    if marks_any:
        for s in sources:
            used = any(sources[c["source"]]["name"] == s["name"] and c["status"] in ("member", "duplicate", "rejected")
                       for c in cands)
            if used and lays[s["layer"]]["locked"]:
                raise Refused("layer_locked", f"“{path_of(d, s['layer'])}” katmanı kilitli: adaylar işaretlenemez. "
                                              "Kilidi açın.", "sources")
    if not marks_any:
        return answer(cands, groups, sources), {}, len(d["entities"]), None
    slot = len(d["entities"]) + 1
    fields, written, marks = {}, [], []
    reviewer = inp.get("reviewer")
    for g in groups:
        if g["row"]["writes"]:
            klass = CLASSES[g["class"]]
            attrs = dict(g["attrs"])
            attrs[field] = klass["codes"][0]
            attrs["Kaynaklar"] = ", ".join(g["row"]["sources"])
            attrs["Adaylar"] = ", ".join(g["row"]["candidates"])
            attrs["Yayılım"] = f"{g['row']['spread']:.3f}"
            if "overlap" in g["row"]:
                attrs["Örtüşme"] = f"{g['row']['overlap']:.3f}"
            first = cands[g["reps"][0]]
            f = {"layerId": class_layer(d, g["class"]), "attrs": attrs}
            if first["kind"] != "point":
                f["pts"] = copy.deepcopy(first["entity"]["pts"])
                if first["entity"].get("bulges"):
                    f["bulges"] = list(first["entity"]["bulges"])
            else:
                f["p"] = {"x": g["row"]["x"], "y": g["row"]["y"]}
                if "z" in g["row"]:
                    f["z"] = g["row"]["z"]
            fields[str(slot)] = f
            asset = f"$uidOf:{slot}"
            g["row"]["asset"] = asset
            written.append(asset)
            marks += [(m, asset) for m in g["members"]]
            slot += 1
        elif g["row"]["status"] == "existing":
            marks += [(m, g["row"]["asset"]) for m in g["members"]]
    for dc in inp.get("decisions", []):
        if "reject" in dc:
            c = next((i for i, c in enumerate(cands) if c["uid"] == dc["candidate"] and c["status"] == "rejected"), None)
            if c is not None:
                marks.append((c, f"Reddedildi: {dc['reject'].strip()}"))
    for m, text in marks:
        c = cands[m]
        k = d["uids"].index(c["uid"]) + 1
        attrs = dict(c["attrs"])
        attrs["Eşleştirme"] = text
        fields[str(k)] = {"attrs": attrs}
    # ADR 4140: each decision that says something and whose candidate is marked, a resolved review note.
    marked_by = dict(marks)
    book = list(d["settings"].get("reviewNotes", []))
    numbers = []
    for dc in inp.get("decisions", []):
        says = dc.get("accept") or "class" in dc or given_fields(dc) or "reject" in dc
        c = next((i for i, c in enumerate(cands) if c["uid"] == dc["candidate"]), None)
        if not says or c is None or c not in marked_by:
            continue
        if "reject" in dc and cands[c]["status"] != "rejected":
            continue
        source = sources[cands[c]["source"]]["name"].strip()
        asset = None if "reject" in dc else marked_by[c]
        number = max((n["number"] for n in book), default=0) + 1
        note = {"id": "$any", "number": number, "text": decision_words(dc, source), "status": "resolved"}
        if (reviewer or "").strip():
            note["author"] = reviewer.strip()
        if (inp.get("reviewedAt") or "").strip():
            note["createdAt"] = inp["reviewedAt"].strip()
        note["anchors"] = [{"uid": cands[c]["uid"], "stamp": "$any"}] + (
            [{"uid": asset, "stamp": "$any"}] if asset else [])
        decision = {"command": "map.features.reconcile", "candidate": cands[c]["uid"], "source": source}
        if dc.get("accept"):
            decision["accept"] = True
        if "class" in dc:
            decision["class"] = dc["class"]
        if given_fields(dc):
            decision["fields"] = given_fields(dc)
        if "reject" in dc:
            decision["reject"] = dc["reject"].strip()
        if asset:
            decision["asset"] = asset
        note["decision"] = decision
        book.append(note)
        numbers.append(number)
    return answer(cands, groups, sources, written, len(marks), numbers), fields, slot - 1, (book if numbers else None)


# ── Cases ────────────────────────────────────────────────────────────────────────────────────────────────────────

SAME = {"revision": "same", "canUndo": False}


def failed(e):
    return {"status": "failed", "error": {"code": e.code, "message": e.message, "path": e.path}}


def step(d, inp, op="execute", undo=True):
    try:
        if op == "execute":
            out, fields, last, notes = executed(d, inp)
        else:
            cands, groups = analyse(d, inp)
            out, fields, last, notes = answer(cands, groups, inp["sources"]), {}, len(d["entities"]), None
    except Refused as e:
        return [{"op": op, "input": inp, "result": failed(e), "expect": SAME}]
    result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
    if op != "execute" or not fields:
        return [{"op": op, "input": inp, "result": result, "expect": SAME, "within": WITHIN}]
    expect = {"revision": "changed", "canUndo": True, "ids": list(range(1, last + 1)), "fields": fields}
    if notes is not None:
        expect["settings"] = {"reviewNotes": notes}
    steps = [{"op": op, "input": inp, "result": result, "expect": expect, "within": WITHIN}]
    if undo:
        steps.append({"op": "undo", "returns": "Detay eşleştirme",
                      "expect": {"canRedo": True, "ids": list(range(1, len(d["entities"]) + 1))}})
    return steps


def case(name, d, steps, note=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["setup"] = d
    c["steps"] = steps
    return c


BASE = {"name": "harita-varliklari", "sources": SOURCES, "confidence": "Güven", "radius": 0.5}


def cases():
    d = setup()
    out = [
        case("Plan: iki çıkarımın adayları; uyuşan, yinelenen, çelişen, tek, var olan, bilinmeyen, eksik alanlı, çizgi",
             d, step(d, BASE, op="plan")),
        case("Eşleştir: uyuşan direkler ve bordür yazılır, var olan direğin adayı işaretlenir; tek adımda geri alınır",
             d, step(d, BASE),
             "Kapı uyuşuyor ama zorunlu KapiNo yok: yazılmaz. Tek rögar ve çelişen direk/ağaç karar bekler."),
    ]
    decided = dict(BASE, reviewer="Ayşe Demir", decisions=[
        {"candidate": uid(9), "accept": True},
        {"candidate": uid(8), "class": "direk"},
        {"candidate": uid(12), "fields": {"KapiNo": "14"}},
        {"candidate": uid(11), "reject": "Gölge; nesne yok"}])
    out.append(case("Kararlarla: tek rögar kabul, ağaç adayı direğe düzeltildi, kapının numarası verildi, bilinmeyen "
                    "aday reddedildi", d, step(d, decided)))
    # ADR 4140: the decisions as review notes, with the time; after the drawing's own notes.
    noted = setup()
    noted["settings"]["reviewNotes"] = [{"id": "0192f5b0-4140-7000-8000-000000000001", "number": 3,
                                         "text": "Paftadaki ölçeği kontrol edin.", "status": "open"}]
    timed = dict(decided, reviewer=" Ayşe Demir ", reviewedAt="2026-11-09T16:00:00+03:00")
    out.append(case("Kararlar inceleme notu olarak: dördü 4–7 numaralı çözülmüş notlar, adaya ve varlığa damgalarıyla "
                    "bağlı; ret notunun varlığı yok", noted, step(noted, timed),
                    "Notlar çizimin 3 numaralı notundan sonra sayılır; gözden geçirenin adı kırpılır. Varlık artık "
                    "İnceleme ve Gözden geçiren özniteliklerini taşımaz."))
    blank = dict(BASE, decisions=[{"candidate": uid(12), "fields": {"KapiNo": "  "}}])
    out.append(case("Boş alan değeri karar sayılmaz: not yazılmaz, kapı yine yazılmaz", d, step(d, blank)))
    out.append(case("Güven alanı yokken temsilci ilk adaydır", d,
                    step(d, {k: v for k, v in BASE.items() if k != "confidence"}, op="plan")))
    out.append(case("Dar yarıçap (0,1 m): uyuşan direkler ayrı tekler olur", d, step(d, dict(BASE, radius=0.1), op="plan")))
    out.append(case("Tek kaynak: yalnız LiDAR", d, step(d, dict(BASE, sources=SOURCES[:1]), op="plan")))
    # After a run: the candidates are reviewed, nothing is written again.
    after = setup()
    out_, fields, _, _ = executed(d, BASE)
    for k, f in fields.items():
        i = int(k) - 1
        if i < len(after["entities"]):
            after["entities"][i]["attrs"] = f["attrs"]
    marked = [a for a in after["entities"] if "Eşleştirme" in a["attrs"]]
    for e in marked:
        e["attrs"]["Eşleştirme"] = uid(0x500)
    out.append(case("Yeniden çalıştırma: eşleştirilmiş adaylar okunmaz, insan kararı ezilmez", after,
                    step(after, BASE)))
    out.append(case("Doğrulama", d, step(d, BASE, op="validate")))
    # ADR 3340: the arcs measured as arcs.
    arc_drawing = setup(objects=arcs())
    near = dict(BASE, radius=0.5)
    out.append(case("Yaylı bordür kirişindeki düz çizgiyle eşleşmez; köşeleriyle çizilmiş eşiyle eşleşir (yay, sekizde "
                    "birleriyle ölçülür)", arc_drawing, step(arc_drawing, near, op="plan"),
                    "Kirişle ölçülseydi ilk yay ile düz çizgi 5 cm'de bir grup olurdu; yayın ortası 3 m ötede."))
    out.append(case("Eşleştir: yaylı bordürle köşeleriyle çizilmiş eşi yazılır, yazılan çizgi yayını taşır", arc_drawing,
                    step(arc_drawing, near)))
    # Refusals, in their order.
    out.append(case("Sürüm yazısı geçersiz", d, [{"op": "execute", "input": dict(BASE, expectedRevision="abc"),
                                                  "result": {"status": "failed", "error": {
                                                      "code": "invalid_revision",
                                                      "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” "
                                                                 "gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
                                                                 "komutun planından alın.", "path": "expectedRevision"}},
                                                  "expect": SAME}]))
    out.append(case("Beklenen sürüm eski: çatışma, hiçbir şey yazılmaz", d, [
        {"op": "execute", "input": dict(BASE, expectedRevision="7"),
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline "
                        "göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
         "expect": SAME}]))
    out.append(case("Bilinmeyen profil", d, step(d, dict(BASE, name="yok"))))
    out.append(case("Kaynak yok", d, step(d, dict(BASE, sources=[]))))
    out.append(case("Bir katman iki kaynakta", d, step(d, dict(BASE, sources=[SOURCES[0], dict(SOURCES[1], layer=L)]))))
    out.append(case("Kaynağın adı boş", d, step(d, dict(BASE, sources=[SOURCES[0], dict(SOURCES[1], name=" ")]))))
    out.append(case("Kaynağın katmanı yok", d, step(d, dict(BASE, sources=[SOURCES[0], dict(SOURCES[1], layer="yok")]))))
    out.append(case("Sınıf alanının adı boş", d, step(d, dict(BASE, field=""))))
    out.append(case("Yarıçap sıfır", d, step(d, dict(BASE, radius=0.0))))
    out.append(case("Karar: kimlik UUID değil", d, step(d, dict(BASE, decisions=[{"candidate": "aday-9"}]))))
    out.append(case("Karar: çizimde olmayan nesne", d, step(d, dict(BASE, decisions=[{"candidate": uid(0x999)}]))))
    out.append(case("Karar: aday olmayan nesne", d, step(d, dict(BASE, decisions=[{"candidate": uid(1), "accept": True}]))))
    out.append(case("Karar: profilde olmayan sınıf", d, step(d, dict(BASE, decisions=[{"candidate": uid(8), "class": "kopru"}]))))
    out.append(case("Karar: hem kabul hem ret", d, step(d, dict(BASE, decisions=[{"candidate": uid(9), "accept": True,
                                                                                   "reject": "?"}]))))
    # ADR 4140: what a review note cannot hold.
    out.append(case("Kaynağın adı 120 karakterden uzun", d,
                    step(d, dict(BASE, sources=[SOURCES[0], dict(SOURCES[1], name="O" * 121)]))))
    out.append(case("Gözden geçirenin adında denetim karakteri", d, step(d, dict(BASE, reviewer="Ayşe\u0007"))))
    out.append(case("Gözden geçirme zamanı 64 karakterden uzun", d, step(d, dict(BASE, reviewedAt="2" * 65))))
    out.append(case("Ret nedeni 4000 karakterden uzun", d,
                    step(d, dict(BASE, decisions=[{"candidate": uid(11), "reject": "g" * 4001}]))))
    out.append(case("Alanın değerinde denetim karakteri", d,
                    step(d, dict(BASE, decisions=[{"candidate": uid(12), "fields": {"KapiNo": "1\u00074"}}]))))
    # ADR 2140: areas by their overlap ratio.
    da = setup(objects=AREAS)
    out.append(case("Alanlar (plan): bina izi iki kaynakta örtüşür (yineleneniyle), 0,25 örtüşen iki iz ayrı tekler, var "
                    "olan binanın üstündeki aday, yeşil alan/bina çelişkisi, nokta çizilmiş bina, alan çizilmiş direk",
                    da, step(da, BASE, op="plan")))
    out.append(case("Alanlar (eşleştir): uyuşan bina izinin en güvenli izi Örtüşme ile yazılır, var olanın adayı "
                    "işaretlenir; tek adımda geri alınır", da, step(da, BASE)))
    out.append(case("Örtüşme 0,2: 0,25 örtüşen iki iz bir gruptur", da, step(da, dict(BASE, overlap=0.2), op="plan")))
    # ADR 2540: the class layers' field rules (the profile applied: each class's fields on its layer, a domain's list).
    df = setup(fields=True)
    accepted = dict(BASE, decisions=[{"candidate": uid(9), "accept": True}])
    out.append(case("Alan kuralları: direğin türü (Aydınlatma) listenin kodu olarak yazılır; kabul edilen rögarın şebekesi "
                    "“Atıksu” listede yok, rögar yazılmaz; kapının numarası eksik", df, step(df, accepted)))
    given = dict(BASE, reviewer="Ayşe Demir", decisions=[
        {"candidate": uid(9), "accept": True, "fields": {"Sebeke": "Kanalizasyon"}},
        {"candidate": uid(12), "fields": {"KapiNo": "14"}}])
    out.append(case("Pencereden alan: rögarın şebekesi listeden verilir (Kanalizasyon → kanalizasyon), kapının numarası "
                    "verilir; ikisi de yazılır", df, step(df, given)))
    long_no = dict(BASE, decisions=[{"candidate": uid(12), "fields": {"KapiNo": "1234567890123456789012345"}}])
    out.append(case("Karardaki kapı numarası 20 karakterden uzun: kapı yazılmaz, neden grupta", df,
                    step(df, long_no, op="plan")))
    daf = setup(objects=AREAS, fields=True)
    out.append(case("Karardaki kat sayısı tam sayı değil (üç): bina izi yazılmaz, neden grupta", daf,
                    step(daf, dict(BASE, decisions=[{"candidate": uid(2), "fields": {"KatSayisi": "üç"}}]), op="plan")))
    out.append(case("Karardaki kat sayısı 04: tam sayının yazılışıyla 4 yazılır", daf,
                    step(daf, dict(BASE, decisions=[{"candidate": uid(2), "fields": {"KatSayisi": "04"}}]))))
    out.append(case("Örtüşme 0", da, step(da, dict(BASE, overlap=0.0))))
    out.append(case("Örtüşme 1", da, step(da, dict(BASE, overlap=1.0))))
    nolayer = setup(missing_layers=("direk",))
    out.append(case("Direğin katmanı yok: önce profili uygulayın", nolayer, step(nolayer, BASE)))
    locked = setup(locked=("hv-drk",))
    out.append(case("Direğin katmanı kilitli: yazılmaz", locked, step(locked, BASE)))
    locked_src = setup(locked=(O,))
    out.append(case("Ortofoto adaylarının katmanı kilitli: işaretlenemez", locked_src, step(locked_src, BASE)))
    return out


def assert_cases():
    cands, groups = analyse(setup(), BASE)
    by = {tuple(g["row"]["candidates"]): g["row"]["status"] for g in groups}
    assert by[(uid(2), uid(3))] == "agreed", by
    assert by[(uid(4), uid(5), uid(6))] == "agreed", by
    assert by[(uid(7), uid(8))] == "conflict", by
    assert by[(uid(9),)] == "single", by
    assert by[(uid(10),)] == "existing", by
    assert by[(uid(12), uid(13))] == "agreed", by
    assert by[(uid(14), uid(15))] == "agreed", by
    st = {c["uid"]: c["status"] for c in cands}
    assert st[uid(5)] == "duplicate" and st[uid(11)] == "unknown" and st[uid(16)] == "reviewed" and st[uid(17)] == "wrongKind", st
    narrow = {tuple(g["row"]["candidates"]): g["row"]["status"] for g in analyse(setup(), dict(BASE, radius=0.1))[1]}
    assert narrow.get((uid(2),)) == "single" and narrow.get((uid(3),)) == "single", narrow
    # ADR 2140's areas.
    cands, groups = analyse(setup(objects=AREAS), BASE)
    by = {tuple(g["row"]["candidates"]): g["row"] for g in groups}
    assert by[(uid(2), uid(3), uid(4))]["status"] == "agreed" and by[(uid(2), uid(3), uid(4))]["overlap"] > 0.85, by
    assert by[(uid(5),)]["status"] == "single" and by[(uid(6),)]["status"] == "single", by
    assert by[(uid(7),)]["status"] == "existing" and by[(uid(8), uid(9))]["status"] == "conflict", by
    st = {c["uid"]: c["status"] for c in cands}
    assert st[uid(4)] == "duplicate" and st[uid(10)] == "wrongKind" and st[uid(11)] == "wrongKind", st
    wide = {tuple(g["row"]["candidates"]): g["row"]["status"] for g in analyse(setup(objects=AREAS), dict(BASE, overlap=0.2))[1]}
    assert wide[(uid(5), uid(6))] == "agreed", wide


def main():
    assert_cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "map.features.reconcile", "commandVersion": 1,
           "title": "Detay eşleştirme: iki kaynak, yinelenen, çelişen, tek, var olan, eksik alan, karar",
           "note": "ADR 1760 (MAP-04) ve ADR 2140 (alanlar). Bağımsız başvuru scripts/fixtures/map_features_reconcile_cases.py: "
                   "kurallar ADR'den, uzaklıklar Python'un math.hypot'uyla, Hausdorff köşe-doğru parçası, alanların örtüşme "
                   "oranı ve ağırlık merkezi shapely (GEOS) ile; KentOS kodu yok. Sınıflar KentOS'un harita-varliklari "
                   "profilinden (veri).",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("map.features.reconcile ortak durumları güncel." if ok else f"{OUT.relative_to(ROOT)} eski.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
