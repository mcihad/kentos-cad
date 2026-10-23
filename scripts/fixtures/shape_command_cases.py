"""The shared cases of cad.rectangle.create, cad.polygon.regular (docs/adr/0290) and cad.sector.create (docs/adr/0622 §9).

    python3 scripts/fixtures/shape_command_cases.py           # writes the files
    python3 scripts/fixtures/shape_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.rectangle.create.json, cad.polygon.regular.json and cad.sector.create.json. The checks, their order, codes,
paths and messages are written here by hand from the ADR; the polygon's own (layer, revision, line weight) are the
contract's of cad.polygon.create (docs/adr/0022). The rings follow the shapes' definitions in the order of their
arithmetic, without KentOS code:

- a rectangle by two corners: u = (cos r, sin r), v = u turned left; the corners a, a + u·du, a + u·du + v·dv,
  a + v·dv with du, dv the projections of b − a; by a size, b = a + u·length·su − … as the quarter of `toward`
  gives su, sv; by an area, width = area / length; by an edge, a, b and the width to the left along the unit
  normal;
- corners rounded or cut from the last to the first: at each, the unit vectors to its neighbours, φ = acos of their
  dot product, a radius's distance r / tan(φ/2) and bulge ± tan((π − φ)/4), the sign the turn's; a chamfer's
  distance along both;
- a regular polygon by its centre: the radius from the centre to the given point (to an edge's middle: over
  cos(π/n), the start half a step back), corners at start + 2πk/n; by an edge, each corner one side length on from
  the last at the edge's direction + 2πk/n;
- a pie slice: sweep = fmod(a1 − a0, 2π), 2π added when negative; none for a radius not above a nanometre or a sweep
  within a nanoradian of nothing or of a whole turn; its ring the centre, c + r·(cos a0, sin a0) and
  c + r·(cos(a0 + sweep), sin(a0 + sweep)), the arc's bulge tan(sweep / 4) on the middle edge.

Square roots are Python's (correctly rounded, as Rust's); hypot is ECMAScript's, as the core writes it; sin, cos,
tan and acos are computed to 60 digits by their series and rounded to the nearest double, not by the platform's
libm. `--check` rebuilds the files in memory and compares them with the ones on disk.
"""
import json
import math
import sys
from decimal import Decimal, getcontext
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000

# ── Exact transcendental values ──────────────────────────────────────────────────────────────────────────────────

getcontext().prec = 60
PI = Decimal("3.14159265358979323846264338327950288419716939937510582097494459")


def _series(x, first, step):
    total, term, n = Decimal(0), first, step
    while abs(term) > Decimal(10) ** -58:
        total += term
        term *= -x * x / ((n + 1) * (n + 2))
        n += 2
    return total


def _reduce(x):
    d = Decimal(x)
    two = 2 * PI
    k = (d / two).to_integral_value()
    return d - k * two


def sin_(x):
    d = _reduce(x)
    return float(_series(d, d, 1))


def cos_(x):
    d = _reduce(x)
    return float(_series(d, Decimal(1), 0))


def tan_(x):
    d = _reduce(x)
    return float(_series(d, d, 1) / _series(d, Decimal(1), 0))


def asin_d(d):
    """asin of a Decimal |d| < 0.7 by its series."""
    total, term, n = Decimal(0), d, 0
    while abs(term) > Decimal(10) ** -58:
        total += term / (2 * n + 1)
        term *= d * d * (2 * n + 1) / (2 * n + 2)
        n += 1
    return total


def acos_(x):
    """acos of the double x, correctly rounded; only the near-right angles of these cases (|x| < 0.5)."""
    assert abs(x) < 0.5, x
    return float(PI / 2 - asin_d(Decimal(x)))


def hypot(a, b):
    """ECMAScript's Math.hypot as the core writes it (jsmath::js_hypot_n)."""
    m = max(abs(a), abs(b))
    if m == 0.0:
        return 0.0
    total, comp = 0.0, 0.0
    for x in (a, b):
        n = abs(x) / m
        summand = n * n - comp
        pre = total + summand
        comp = (pre - total) - summand
        total = pre
    return math.sqrt(total) * m


def sign(x):
    return 1.0 if x > 0 else (-1.0 if x < 0 else 0.0)


# ── Shapes ───────────────────────────────────────────────────────────────────────────────────────────────────────

def rect_corners(a, b, r):
    u = (cos_(r), sin_(r))
    v = (-u[1], u[0])
    w = (b[0] - a[0], b[1] - a[1])
    du = w[0] * u[0] + w[1] * u[1]
    dv = w[0] * v[0] + w[1] * v[1]
    if abs(du) < 1e-9 or abs(dv) < 1e-9:
        return None
    return [a, (a[0] + u[0] * du, a[1] + u[1] * du),
            (a[0] + u[0] * du + v[0] * dv, a[1] + u[1] * du + v[1] * dv),
            (a[0] + v[0] * dv, a[1] + v[1] * dv)]


def rect_size(a, length, width, r, toward):
    u = (cos_(r), sin_(r))
    w = (toward[0] - a[0], toward[1] - a[1])
    su = 1.0 if w[0] * u[0] + w[1] * u[1] >= 0 else -1.0
    sv = 1.0 if -w[0] * u[1] + w[1] * u[0] >= 0 else -1.0
    b = (a[0] + u[0] * length * su - u[1] * width * sv, a[1] + u[1] * length * su + u[0] * width * sv)
    return rect_corners(a, b, r)


def rect_edge(p1, p2, width):
    d = (p2[0] - p1[0], p2[1] - p1[1])
    ln = hypot(*d)
    if ln < 1e-9 or abs(width) < 1e-9:
        return None
    n = ((-d[1] / ln) * width, (d[0] / ln) * width)
    return [p1, p2, (p2[0] + n[0], p2[1] + n[1]), (p1[0] + n[0], p1[1] + n[1])]


def corners(ring, op, size):
    """Every corner rounded (op "fillet", size the radius) or cut ("chamfer"), last first; the bulges; or the
    reason one cannot be done."""
    pts, bulges = list(ring), [0.0] * len(ring)
    for index in reversed(range(len(ring))):
        n = len(pts)
        v, p, nx = pts[index], pts[(index - 1) % n], pts[(index + 1) % n]
        lp, ln = hypot(p[0] - v[0], p[1] - v[1]), hypot(nx[0] - v[0], nx[1] - v[1])
        u1 = ((p[0] - v[0]) / lp, (p[1] - v[1]) / lp)
        u2 = ((nx[0] - v[0]) / ln, (nx[1] - v[1]) / ln)
        dot = u1[0] * u2[0] + u1[1] * u2[1]
        phi = acos_(max(-1.0, min(1.0, dot)))
        if op == "fillet":
            d = size / tan_(phi / 2.0)
            turn = sign(-u1[0] * u2[1] + u1[1] * u2[0])
            d1, d2, bulge = d, d, turn * tan_((math.pi - phi) / 4.0)
            too_big = "Yarıçap bu kenarlar için çok büyük."
        else:
            d1, d2, bulge = size, size, 0.0
            too_big = "Pah mesafesi kenar boyundan büyük."
        if d1 > lp + 1e-9 or d2 > ln + 1e-9:
            return too_big
        t1 = (v[0] + u1[0] * d1, v[1] + u1[1] * d1)
        t2 = (v[0] + u2[0] * d2, v[1] + u2[1] * d2)
        pts = pts[:index] + [t1, t2] + pts[index + 1:]
        bulges = bulges[:index] + [bulge] + bulges[index:]
        # The core drops a point within a nanometre of the last; these cases leave none.
        for i in range(len(pts)):
            q, r = pts[i], pts[(i + 1) % len(pts)]
            assert hypot(q[0] - r[0], q[1] - r[1]) > 1e-9
    return pts, (bulges if any(b != 0 for b in bulges) else None)


def regular(center, sides, p, inscribed):
    n = float(sides)
    r0 = hypot(p[0] - center[0], p[1] - center[1])
    if r0 < 1e-9:
        return None
    a0 = math.atan2(p[1] - center[1], p[0] - center[0])
    assert a0 in (0.0, math.pi / 2, math.pi, -math.pi / 2), "axis directions only: atan2 exact"
    r = r0 if inscribed else r0 / cos_(math.pi / n)
    start = a0 if inscribed else a0 - math.pi / n
    return [(center[0] + cos_(start + (2.0 * math.pi * k) / n) * r, center[1] + sin_(start + (2.0 * math.pi * k) / n) * r)
            for k in range(sides)]


def regular_edge(p1, p2, sides):
    n = float(sides)
    s = hypot(p2[0] - p1[0], p2[1] - p1[1])
    if s < 1e-9:
        return None
    a = math.atan2(p2[1] - p1[1], p2[0] - p1[0])
    assert a in (0.0, math.pi / 2, math.pi, -math.pi / 2)
    out = [p1]
    for k in range(sides - 1):
        q = out[k]
        t = a + (2.0 * math.pi * k) / n
        out.append(p2 if k == 0 else (q[0] + cos_(t) * s, q[1] + sin_(t) * s))
    return out


def sector(c, r, a0, a1):
    sweep = math.fmod(a1 - a0, math.tau)
    if sweep < 0.0:
        sweep += math.tau
    if not r > 1e-9 or sweep < 1e-9 or math.tau - sweep < 1e-9:
        return None

    def on(a):
        return (c[0] + r * cos_(a), c[1] + r * sin_(a))

    return [c, on(a0), on(a0 + sweep)], [0.0, tan_(sweep / 4.0), 0.0]


# ── The fixture ──────────────────────────────────────────────────────────────────────────────────────────────────

def num(v):
    v = float(v)
    return int(v) if v.is_integer() else v


def P(x, y):
    return {"x": num(X + x), "y": num(Y + y)}


def W(p):
    return {"x": num(p[0]), "y": num(p[1])}


def at(x, y):
    return (float(X + x), float(Y + y))


SETUP = POLYGON["setup"]
IDS = [e["id"] for e in SETUP["entities"]]
NEXT = max(IDS) + 1
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}
POLY_ERR = {}
for case in POLYGON["cases"]:
    for step in case["steps"]:
        e = step.get("result", {}).get("error")
        if e:
            POLY_ERR.setdefault(e["code"], e)
HIDDEN = "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek."
DEGENERATE = ("Dikdörtgenin bir kenarı sıfır uzunlukta: köşeler çakışıyor ya da dönüşün doğrultusunda bir doğru üzerinde. "
              "Başka bir köşe verin.")
REGULAR_DEGENERATE = ("Çokgenin iki noktası çakışıyor: merkezden uzakta bir köşe ya da kenar ortası, ya da uzunluğu olan bir "
                      "kenar verin.")


def not_finite_point(whose, axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def not_finite_number(what):
    return f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin."


def polygon_entity(pts, bulges, layer="yapi", extra=None, id_=NEXT):
    e = {"kind": "polygon", "id": id_, "layerId": layer, **(extra or {}), "attrs": dict((extra or {}).get("attrs", {})),
         "pts": [W(p) for p in pts]}
    if bulges is not None:
        e["bulges"] = [num(b) for b in bulges]
    return e


def written(name, inp, pts, bulges=None, warnings=(), extra=None, layer="yapi"):
    entity = polygon_entity(pts, bulges, layer, extra)
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": {"status": "completed", "output": {"uid": "$uid", "id": NEXT, "revision": "$current"}, "warnings": list(warnings)},
         "expect": {"ids": IDS + [NEXT], "entities": {str(NEXT): entity}, "canUndo": True, "dirty": True}},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}]}


def planned(name, inp, pts, bulges=None):
    return {"name": name, "steps": [
        {"op": "plan", "input": inp,
         "result": {"status": "completed", "output": {"entity": polygon_entity(pts, bulges, id_=0), "revision": "$current"},
                    "warnings": []},
         "expect": UNCHANGED}]}


def refused(name, inp, code, message, path, non_finite=None):
    step = {"op": "execute", "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def polygon_refusals(base):
    e = POLY_ERR
    return [
        refused("Kalınlık 100 mm'den büyük: cad.polygon.create'in reddi", {**base, "lineWeight": 101}, "invalid_line_weight",
                e["invalid_line_weight"]["message"], "lineWeight"),
        refused("Kilitli katman", {**base, "layerId": "kilitli"}, "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        refused("Olmayan katman", {**base, "layerId": "yok"}, "layer_not_found", e["layer_not_found"]["message"], "layerId"),
        refused("Katman değil grup", {**base, "layerId": "pafta"}, "not_a_layer", e["not_a_layer"]["message"], "layerId"),
        refused("Beklenen sürüm sayı değil", {**base, "expectedRevision": "012"}, "invalid_revision",
                e["invalid_revision"]["message"], "expectedRevision"),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": base,
             "result": {"status": "completed", "output": {"uid": "$uid", "id": NEXT, "revision": "$current"}, "warnings": []}},
            {"op": "execute", "input": {**base, "expectedRevision": "$old"},
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": e["revision_conflict"]["message"],
                                                        "path": "expectedRevision", "revision": "$current"}}}]},
    ]


def rect(frame, **extra):
    return {"layerId": "yapi", "frame": frame, **extra}


def rectangle_cases():
    a, b = at(10, 10), at(30, 22.5)
    pi6 = math.pi / 6
    out = [
        written("İki köşe: kenarlar doğu ve kuzey boyunca; adım “Ekle”", rect({"kind": "corners", "a": W(a), "b": W(b)}),
                rect_corners(a, b, 0.0)),
        written("Karşı köşe güneybatıda: halka verilen ilk köşeden", rect({"kind": "corners", "a": W(b), "b": W(a)}),
                rect_corners(b, a, 0.0)),
        written("Dönüş 30°: kenarlar dönüşün doğrultusunda", rect({"kind": "corners", "a": W(a), "b": W(b)}, rotation=pi6),
                rect_corners(a, b, pi6)),
        written("Dönüş 90°", rect({"kind": "corners", "a": W(a), "b": W(b)}, rotation=math.pi / 2),
                rect_corners(a, b, math.pi / 2)),
        written("Boy ve en, kuzeydoğuya", rect({"kind": "size", "corner": W(a), "length": 20, "width": 12.5, "toward": P(15, 15)}),
                rect_size(a, 20.0, 12.5, 0.0, at(15, 15))),
        written("Boy ve en, güneybatıya", rect({"kind": "size", "corner": W(a), "length": 20, "width": 12.5, "toward": P(5, 5)}),
                rect_size(a, 20.0, 12.5, 0.0, at(5, 5))),
        written("Boy ve en, kuzeybatıya, 30° dönük",
                rect({"kind": "size", "corner": W(a), "length": 20, "width": 12.5, "toward": P(0, 20)}, rotation=pi6),
                rect_size(a, 20.0, 12.5, pi6, at(0, 20))),
        written("Alan 250 m² ve boy 20: en 12.5", rect({"kind": "area", "corner": W(a), "area": 250, "length": 20, "toward": P(15, 15)}),
                rect_size(a, 20.0, 250.0 / 20.0, 0.0, at(15, 15))),
        written("Alan 100 m², boy 8: en 12.5, güneydoğuya",
                rect({"kind": "area", "corner": W(a), "area": 100, "length": 8, "toward": P(15, 5)}),
                rect_size(a, 8.0, 100.0 / 8.0, 0.0, at(15, 5))),
        written("Kenar ve en solunda", rect({"kind": "edge", "a": P(0, 0), "b": P(20, 0), "width": 5}),
                rect_edge(at(0, 0), at(20, 0), 5.0)),
        written("Kenar ve eksi en: sağında", rect({"kind": "edge", "a": P(0, 0), "b": P(20, 0), "width": -5}),
                rect_edge(at(0, 0), at(20, 0), -5.0)),
        written("Eğik kenar (6, 8) ve en 4; dönüş okunmaz",
                rect({"kind": "edge", "a": P(0, 0), "b": P(6, 8), "width": 4}, rotation=1.0),
                rect_edge(at(0, 0), at(6, 8), 4.0)),
    ]
    ring = rect_corners(a, b, 0.0)
    pts, bulges = corners(ring, "fillet", 2.0)
    out.append(written("Köşeler 2 m yuvarlak: her köşede çeyrek yay",
                       rect({"kind": "corners", "a": W(a), "b": W(b)}, corners={"kind": "fillet", "radius": 2}), pts, bulges))
    pts, bulges = corners(ring, "chamfer", 1.5)
    out.append(written("Köşelere 1.5 m pah", rect({"kind": "corners", "a": W(a), "b": W(b)}, corners={"kind": "chamfer", "distance": 1.5}),
                       pts, bulges))
    out.append(written("Keskin köşe açıkça", rect({"kind": "corners", "a": W(a), "b": W(b)}, corners={"kind": "sharp"}), ring))
    edge = rect_edge(at(0, 0), at(20, 0), -5.0)
    pts, bulges = corners(edge, "fillet", 1.0)
    out.append(written("Sağa açılan kenarın köşeleri yuvarlak: yaylar halkanın yönünde",
                       rect({"kind": "edge", "a": P(0, 0), "b": P(20, 0), "width": -5}, corners={"kind": "fillet", "radius": 1}),
                       pts, bulges))
    out.append(written("Renk, kalınlık, öznitelik ve etiket taşınır",
                       rect({"kind": "corners", "a": W(a), "b": W(b)}, color="#E5484D", lineWeight=0.5, attrs={"Tür": "Oda"}, label="Salon"),
                       ring, extra={"color": "#E5484D", "lineWeight": 0.5, "attrs": {"Tür": "Oda"}, "label": "Salon"}))
    out.append(written("Gizli katmana uyarıyla", {**rect({"kind": "corners", "a": W(a), "b": W(b)}), "layerId": "gizli"}, ring,
                       warnings=[{"code": "layer_hidden", "message": HIDDEN, "path": "layerId"}], layer="gizli"))
    out.append(planned("Plan: alan olacağı gibi, yuvası 0", rect({"kind": "corners", "a": W(a), "b": W(b)}), ring))
    base = rect({"kind": "corners", "a": W(a), "b": W(b)})
    out += [
        refused("İlk köşe sonlu değil", base, "not_finite", not_finite_point("İlk köşenin", "x"), "frame.a.x", {"frame.a.x": "NaN"}),
        refused("Dönüş sonlu değil", base, "not_finite", not_finite_number("Dönüş"), "rotation", {"rotation": "Infinity"}),
        refused("Boy sıfır olamaz", rect({"kind": "size", "corner": W(a), "length": 0, "width": 12.5, "toward": P(15, 15)}),
                "invalid_distance", "Uzunluk sıfırdan büyük olmalı.", "frame.length"),
        refused("Alan eksi olamaz", rect({"kind": "area", "corner": W(a), "area": -5, "length": 20, "toward": P(15, 15)}),
                "invalid_distance", "Alan sıfırdan büyük olmalı.", "frame.area"),
        refused("Kenarın eni sıfır olamaz", rect({"kind": "edge", "a": P(0, 0), "b": P(20, 0), "width": 0}), "invalid_distance",
                "Genişlik sıfır olamaz: kenarın solu için artı, sağı için eksi bir genişlik verin.", "frame.width"),
        refused("Yarıçap sıfır olamaz", rect({"kind": "corners", "a": W(a), "b": W(b)}, corners={"kind": "fillet", "radius": 0}),
                "invalid_distance", "Köşe yarıçapı sıfırdan büyük olmalı; keskin köşe için corners'ı vermeyin.", "corners.radius"),
        refused("Köşeler dönüşün doğrultusunda bir doğru üzerinde", rect({"kind": "corners", "a": W(a), "b": P(30, 10)}),
                "degenerate_shape", DEGENERATE, "frame"),
        refused("Kenarın iki ucu çakışıyor", rect({"kind": "edge", "a": P(0, 0), "b": P(0, 0), "width": 5}), "degenerate_shape",
                DEGENERATE, "frame"),
        refused("Yarıçap kenarlara sığmıyor", rect({"kind": "corners", "a": W(a), "b": W(b)}, corners={"kind": "fillet", "radius": 7}),
                "invalid_corners", "Yarıçap bu kenarlar için çok büyük. Daha küçük bir yarıçap ya da pah verin.", "corners"),
        refused("Pah kenara sığmıyor", rect({"kind": "corners", "a": W(a), "b": W(b)}, corners={"kind": "chamfer", "distance": 13}),
                "invalid_corners", "Pah mesafesi kenar boyundan büyük. Daha küçük bir yarıçap ya da pah verin.", "corners"),
    ]
    out += polygon_refusals(base)
    return out


def reg(sides, frame, **extra):
    return {"layerId": "yapi", "sides": sides, "frame": frame, **extra}


def regular_cases():
    c = at(20, 20)
    out = [
        written("Altıgen, merkez ve doğudaki ilk köşe; adım “Ekle”", reg(6, {"kind": "inscribed", "center": W(c), "corner": P(25, 20)}),
                regular(c, 6, at(25, 20), True)),
        written("Kare, ilk köşe kuzeyde", reg(4, {"kind": "inscribed", "center": W(c), "corner": P(20, 25)}),
                regular(c, 4, at(20, 25), True)),
        written("Üçgen, ilk köşe batıda", reg(3, {"kind": "inscribed", "center": W(c), "corner": P(15, 20)}),
                regular(c, 3, at(15, 20), True)),
        written("On ikigen, ilk köşe güneyde", reg(12, {"kind": "inscribed", "center": W(c), "corner": P(20, 10)}),
                regular(c, 12, at(20, 10), True)),
        written("Kare çember dışında: ilk kenarın ortası doğuda", reg(4, {"kind": "circumscribed", "center": W(c), "middle": P(25, 20)}),
                regular(c, 4, at(25, 20), False)),
        written("Altıgen çember dışında, kenar ortası kuzeyde",
                reg(6, {"kind": "circumscribed", "center": W(c), "middle": P(20, 26)}), regular(c, 6, at(20, 26), False)),
        written("Sekizgen çember dışında", reg(8, {"kind": "circumscribed", "center": W(c), "middle": P(30, 20)}),
                regular(c, 8, at(30, 20), False)),
        written("Kenardan kare: kenarın solunda", reg(4, {"kind": "edge", "a": P(0, 0), "b": P(10, 0)}),
                regular_edge(at(0, 0), at(10, 0), 4)),
        written("Kenardan beşgen", reg(5, {"kind": "edge", "a": P(0, 0), "b": P(10, 0)}), regular_edge(at(0, 0), at(10, 0), 5)),
        written("Kenar batıya: çokgen güneyde", reg(6, {"kind": "edge", "a": P(10, 0), "b": P(0, 0)}),
                regular_edge(at(10, 0), at(0, 0), 6)),
        written("Kenar kuzeye: çokgen batıda", reg(3, {"kind": "edge", "a": P(0, 0), "b": P(0, 10)}),
                regular_edge(at(0, 0), at(0, 10), 3)),
        written("Renk ve öznitelik taşınır", reg(6, {"kind": "inscribed", "center": W(c), "corner": P(25, 20)}, color="#E5484D",
                                                 attrs={"Tür": "Kuyu"}),
                regular(c, 6, at(25, 20), True), extra={"color": "#E5484D", "attrs": {"Tür": "Kuyu"}}),
        written("Gizli katmana uyarıyla", {**reg(4, {"kind": "inscribed", "center": W(c), "corner": P(25, 20)}), "layerId": "gizli"},
                regular(c, 4, at(25, 20), True), warnings=[{"code": "layer_hidden", "message": HIDDEN, "path": "layerId"}], layer="gizli"),
        planned("Plan: çokgen olacağı gibi, yuvası 0", reg(6, {"kind": "inscribed", "center": W(c), "corner": P(25, 20)}),
                regular(c, 6, at(25, 20), True)),
    ]
    base = reg(6, {"kind": "inscribed", "center": W(c), "corner": P(25, 20)})
    out += [
        refused("Merkez sonlu değil", base, "not_finite", not_finite_point("Merkezin", "x"), "frame.center.x", {"frame.center.x": "NaN"}),
        refused("İlk köşe sonlu değil", base, "not_finite", not_finite_point("İlk köşenin", "y"), "frame.corner.y",
                {"frame.corner.y": "-Infinity"}),
        refused("Kenarın başı sonlu değil", reg(4, {"kind": "edge", "a": P(0, 0), "b": P(10, 0)}), "not_finite",
                not_finite_point("Kenarın başının", "x"), "frame.a.x", {"frame.a.x": "NaN"}),
        refused("İki kenar çokgen değildir", reg(2, base["frame"]), "invalid_count", "Kenar sayısı 3 ile 1024 arasında olmalı.", "sides"),
        refused("1024'ten çok kenar", reg(1025, base["frame"]), "invalid_count", "Kenar sayısı 3 ile 1024 arasında olmalı.", "sides"),
        refused("Köşe merkezde", reg(6, {"kind": "inscribed", "center": W(c), "corner": W(c)}), "degenerate_shape", REGULAR_DEGENERATE,
                "frame"),
        refused("Kenarın iki ucu çakışıyor", reg(4, {"kind": "edge", "a": P(0, 0), "b": P(0, 0)}), "degenerate_shape",
                REGULAR_DEGENERATE, "frame"),
    ]
    out += polygon_refusals(base)
    return out


SECTOR_DEGENERATE = "Dilimin süpürmesi yok: bitiş açısı başlangıçla aynı ya da tam tur. Başka bir bitiş açısı verin."


def sec(r, a0, a1, **extra):
    return {"layerId": "yapi", "c": P(20, 20), "r": r, "a0": a0, "a1": a1, **extra}


def sector_written(name, r, a0, a1, **kw):
    pts, bulges = sector(at(20, 20), float(r), float(a0), float(a1))
    extra = {k: v for k, v in kw.items() if k in ("color", "lineWeight", "attrs", "label")}
    rest = {k: v for k, v in kw.items() if k not in extra}
    return written(name, sec(r, a0, a1, **extra), pts, bulges, extra=extra or None, **rest)


def sector_cases():
    pi = math.pi
    out = [
        sector_written("Çeyrek dilim: doğudan kuzeye, yarıçap 10; adım “Ekle”", 10, 0.0, pi / 2),
        sector_written("Yarım dilim: doğudan batıya", 10, 0.0, pi),
        sector_written("Üç çeyrek: kuzeyden doğuya saat yönünün tersine dolanarak", 10, pi / 2, 0.0),
        sector_written("Eksi başlangıç açısı: güneyden doğuya", 10, -pi / 2, 0.0),
        sector_written("Tam turu aşan bitiş açısı tur kadar indirilir", 10, 0.0, 5 * pi / 2),
        sector_written("Batıdan güneye", 6, pi, 3 * pi / 2),
        sector_written("Kuzeyden batıya, yarıçap yarım metre", 0.5, pi / 2, pi),
        sector_written("Neredeyse tam tur: güneyden doğunun hemen altına", 10, -pi / 2, -pi / 2 - 0.25),
        sector_written("Renk, kalınlık, öznitelik ve etiket taşınır", 10, 0.0, pi / 2, color="#E5484D", lineWeight=0.5,
                       attrs={"Tür": "Dilim"}, label="A"),
        sector_written("Gizli katmana uyarıyla", 10, 0.0, pi / 2,
                       warnings=[{"code": "layer_hidden", "message": HIDDEN, "path": "layerId"}], layer="gizli"),
    ]
    # The hidden layer's case writes on it.
    out[-1]["steps"][0]["input"]["layerId"] = "gizli"
    pts, bulges = sector(at(20, 20), 10.0, 0.0, pi / 2)
    out.append(planned("Plan: dilim olacağı gibi, yuvası 0", sec(10, 0.0, pi / 2), pts, bulges))
    base = sec(10, 0.0, pi / 2)
    out += [
        refused("Merkez sonlu değil", base, "not_finite", not_finite_point("Merkezin", "x"), "c.x", {"c.x": "NaN"}),
        refused("Merkezin kuzeyi sonlu değil", base, "not_finite", not_finite_point("Merkezin", "y"), "c.y", {"c.y": "Infinity"}),
        refused("Yarıçap sonlu değil", base, "not_finite", not_finite_number("Yarıçap"), "r", {"r": "Infinity"}),
        refused("Başlangıç açısı sonlu değil", base, "not_finite", not_finite_number("Başlangıç açısı"), "a0", {"a0": "NaN"}),
        refused("Bitiş açısı sonlu değil", base, "not_finite", not_finite_number("Bitiş açısı"), "a1", {"a1": "-Infinity"}),
        refused("Yarıçap sıfır", sec(0, 0.0, pi / 2), "invalid_distance", "Yarıçap sıfırdan büyük olmalı.", "r"),
        refused("Yarıçap eksi", sec(-3, 0.0, pi / 2), "invalid_distance", "Yarıçap sıfırdan büyük olmalı.", "r"),
        refused("Yarıçap bir nanometreden küçük", sec(1e-10, 0.0, pi / 2), "invalid_distance", "Yarıçap sıfırdan büyük olmalı.", "r"),
        refused("Bitiş açısı başlangıçla aynı: süpürme yok", sec(10, 1.0, 1.0), "degenerate_shape", SECTOR_DEGENERATE, "a1"),
        refused("Tam tur dilim değildir", sec(10, 0.0, 2 * pi), "degenerate_shape", SECTOR_DEGENERATE, "a1"),
    ]
    for case in sector_cases_pathless():
        out.append(case)
    out += polygon_refusals(base)
    return out


def sector_cases_pathless():
    """Asserts on the reference itself: the slice is a ring of three points with one arc."""
    pts, bulges = sector(at(20, 20), 10.0, 0.0, math.pi / 2)
    assert len(pts) == 3 and bulges[0] == 0.0 and bulges[2] == 0.0 and bulges[1] > 0
    assert sector(at(20, 20), 10.0, 0.0, 0.0) is None and sector(at(20, 20), 0.0, 0.0, 1.0) is None
    return []


FILES = [
    ("cad.rectangle.create", "Dikdörtgen oluştur", rectangle_cases),
    ("cad.polygon.regular", "Düzgün çokgen oluştur", regular_cases),
    ("cad.sector.create", "Daire dilimi oluştur", sector_cases),
]


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = False
    for command, title, cases in FILES:
        doc = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
               "note": "Written by scripts/fixtures/shape_command_cases.py from the shapes' definitions (docs/adr/"
                       + ("0622 §9" if command == "cad.sector.create" else "0290") + "); do not edit by hand.",
               "setup": SETUP, "cases": cases()}
        assert_no_negative_zero(doc, command)
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        path = OUT / f"{command}.json"
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad = True
            else:
                print(f"{command} durumları güncel: {len(doc['cases'])} durum.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
