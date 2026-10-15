"""Uzat-kısalt by its meaning: shared cases of cad.entities.lengthen (docs/adr/0298).

    python3 scripts/fixtures/lengthen_command_cases.py           # writes the file
    python3 scripts/fixtures/lengthen_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.lengthen.json. The ADR's rules, computed here without KentOS code, angles and
an elliptical arc's lengths with mpmath at 40 digits:

- The new length: Fark adds `length` (negative shortens) or, for an arc, its included angle grows by `angle` (the new
  length r times the new angle); Yüzde is the length times percent / 100; Toplam is `length`, or r times `angle` for an
  arc; Dinamik puts the moving end where the point is: past a straight end edge its projection on the edge's line, on
  an arc the angle the point is seen at from the centre, from the fixed end round the arc's way; before the end the
  nearest point of the path.
- Lengthening continues the end edge: a line along itself, an arc (and a polyline's arc edge, its bulge tan(θ/4) the
  new θ) on its circle, an elliptical arc on its ellipse (the parameter whose length along it is the new one). A
  polyline's edges keep their widths; the moving end's elevation and measure run on along the end edge by length,
  from its two ends'.
- Shortening cuts the path at the new length from the fixed end: the vertices kept keep theirs, the new end takes the
  cut edge's elevation, measure and width by length.
- Refusals in order: invalid_uid, not_finite, invalid_change (Fark or Toplam with neither or both of length and
  angle), entity_not_found, layer_locked, not_lengthenable (a closed polyline, an area, a circle, a whole ellipse, a
  point), multi_part, not_lengthenable again for an object with no length (a line whose ends meet), angle_needs_arc,
  invalid_length (the new length not above 0), arc_closes (a whole turn or more).

Plans compare within 1e-9 (`within`); writes compare what is exact.
"""
import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.dont_write_bytecode = True  # no __pycache__ in the tree

mp.mp.dps = 40
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.lengthen.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
WITHIN = 1e-9
TAU = 2 * math.pi
U = lambda i: f"$uidOf:{i}"  # noqa: E731
B90 = math.tan(math.pi / 8)  # a quarter turn's bulge


def node(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def num(v):
    v = float(v)
    return int(v) if v.is_integer() else v


def p(x, y):
    return {"x": num(x), "y": num(y)}


def base(i, layer="cizim"):
    return {"id": i, "layerId": layer, "attrs": {}}


ENTITIES = [
    {"kind": "line", **base(1), "a": p(0, 0), "b": p(10, 0), "za": 1, "zb": 3, "ma": 0, "mb": 10},
    {"kind": "arc", **base(2), "c": p(30, 0), "r": 5, "a0": 0, "a1": math.pi / 2},
    {"kind": "polyline", **base(3), "pts": [p(0, 20), p(10, 20), p(10, 30)], "zs": [1, 2, 3], "ms": [0, 10, 20],
     "widths": [[0.2, 0.4], [0.4, 0.6], [0, 0]]},
    {"kind": "polyline", **base(4), "pts": [p(20, 20), p(30, 20), p(40, 20)], "bulges": [0, B90, 0]},
    {"kind": "ellipse", **base(5), "c": p(60, 0), "major": p(6, 0), "ratio": 0.5, "t0": 0, "t1": math.pi / 2},
    {"kind": "polyline", **base(6), "pts": [p(0, 50), p(10, 50), p(10, 60)], "closed": True},
    {"kind": "circle", **base(7), "c": p(30, 50), "r": 3},
    {"kind": "polygon", **base(8), "pts": [p(40, 50), p(50, 50), p(50, 60)]},
    {"kind": "line", **base(9, "kilitli"), "a": p(0, 70), "b": p(10, 70)},
    {"kind": "polyline", **base(10), "pts": [p(20, 70), p(30, 70)], "parts": [{"pts": [p(20, 75), p(30, 75)]}]},
    {"kind": "line", **base(11), "a": p(0, 40), "b": p(3, 44)},
    {"kind": "ellipse", **base(12), "c": p(80, 0), "major": p(4, 0), "ratio": 0.5, "t0": 0, "t1": TAU},
    {"kind": "point", **base(13), "p": p(90, 0)},
    {"kind": "line", **base(14), "a": p(5, 80), "b": p(5, 80)},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Uzat-kısalt",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(40, 30),
        "layers": [node("cizim", "Çizim"), node("kilitli", "Kilitli", locked=True)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


# ── Geometry ───────────────────────────────────────────────────────────────────────────────────────────────────────

def xy(q):
    return (q["x"], q["y"])


def norm(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0 else r


def bulge_arc(a, b, bulge):
    """The arc of a bulged edge: centre, radius, start angle, signed sweep."""
    theta = 4 * math.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    r = chord / (2 * abs(math.sin(theta / 2)))
    h = (chord / 2) / math.tan(theta / 2)
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    c = (mx - dy / chord * h, my + dx / chord * h)
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), theta


def ellipse_len(e, t0, t1):
    """The length along an ellipse between two parameters (t1 above t0)."""
    a = mp.mpf(math.hypot(e["major"]["x"], e["major"]["y"]))
    b = a * mp.mpf(e["ratio"])
    return mp.quad(lambda t: mp.sqrt((a * mp.sin(t)) ** 2 + (b * mp.cos(t)) ** 2), [t0, t1])


def ellipse_param_for(e, length, forward):
    """The parameter where the length along the arc from its start (forward) or back from its end is `length`."""
    t0, t1 = mp.mpf(e["t0"]), mp.mpf(e["t1"])
    if forward:
        return mp.findroot(lambda t: ellipse_len(e, t0, t) - length, (t0 + t1) / 2)
    return mp.findroot(lambda t: ellipse_len(e, t, t1) - length, (t0 + t1) / 2)


def length_of(e):
    k = e["kind"]
    if k == "line":
        return math.hypot(e["b"]["x"] - e["a"]["x"], e["b"]["y"] - e["a"]["y"])
    if k == "arc":
        return e["r"] * norm(e["a1"] - e["a0"])
    if k == "ellipse":
        return float(ellipse_len(e, e["t0"], e["t1"]))
    pts = [xy(q) for q in e["pts"]]
    bulges = e.get("bulges") or [0] * len(pts)
    total = 0.0
    for i in range(len(pts) - 1):
        if bulges[i]:
            _, r, _, th = bulge_arc(pts[i], pts[i + 1], bulges[i])
            total += r * abs(th)
        else:
            total += math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1])
    return total


# ── What each command writes ───────────────────────────────────────────────────────────────────────────────────────

def lengthened(e, at_end, new):
    """`e` given the length `new` at its end or start, with the ADR's carried values."""
    e = copy.deepcopy(e)
    k = e["kind"]
    old = length_of(e)
    if k == "line":
        a, b = xy(e["a"]), xy(e["b"])
        fixed, moving = (a, b) if at_end else (b, a)
        f = new / old
        end = (fixed[0] + (moving[0] - fixed[0]) * f, fixed[1] + (moving[1] - fixed[1]) * f)
        e["b" if at_end else "a"] = p(*end)
        for zf, zm in (("za", "zb"), ("ma", "mb")) if at_end else (("zb", "za"), ("mb", "ma")):
            if zf in e and zm in e:
                e[zm] = num(e[zf] + (e[zm] - e[zf]) * f)
        return e
    if k == "arc":
        sweep = new / e["r"]
        if at_end:
            e["a1"] = norm(e["a0"] + sweep)
        else:
            e["a0"] = norm(e["a1"] - sweep)
        return e
    if k == "ellipse":
        if at_end:
            e["t1"] = float(ellipse_param_for(e, new, True))
        else:
            e["t0"] = float(ellipse_param_for(e, new, False))
        return e
    # A polyline.
    pts = [xy(q) for q in e["pts"]]
    n = len(pts)
    bulges = e.get("bulges") or [0] * n
    if new > old:
        seg = n - 2 if at_end else 0
        fixed, moving = (pts[n - 2], pts[n - 1]) if at_end else (pts[1], pts[0])
        delta = new - old
        if bulges[seg]:
            c, r, a0, th = bulge_arc(pts[seg], pts[seg + 1], bulges[seg])
            sw = th + math.copysign(delta / r, th)
            start = a0 if at_end else a0 + th - sw
            q = (c[0] + r * math.cos(start + sw), c[1] + r * math.sin(start + sw)) if at_end else \
                (c[0] + r * math.cos(start), c[1] + r * math.sin(start))
            bulges[seg] = math.tan(sw / 4)
            old_edge = r * abs(th)
            new_edge = r * abs(sw)
        else:
            old_edge = math.hypot(moving[0] - fixed[0], moving[1] - fixed[1])
            new_edge = old_edge + delta
            g = new_edge / old_edge
            q = (fixed[0] + (moving[0] - fixed[0]) * g, fixed[1] + (moving[1] - fixed[1]) * g)
        pts[n - 1 if at_end else 0] = q
        e["pts"] = [p(*v) for v in pts]
        if any(bulges):
            e["bulges"] = bulges
        mi, fi = (n - 1, n - 2) if at_end else (0, 1)
        for key in ("zs", "ms"):
            if key in e:
                vals = e[key]
                vals[mi] = num(vals[fi] + (vals[mi] - vals[fi]) * (new_edge / old_edge))
        return e
    # Shortening a straight polyline: cut the path `s` from its start (the new length at the end, the rest at the
    # start), keep the head or the tail; the cut edge's elevation, measure and width at the cut by length.
    s = new if at_end else old - new
    run = 0.0
    for i in range(n - 1):
        edge = math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1])
        if run + edge > s:
            t = (s - run) / edge
            break
        run += edge
    else:
        raise AssertionError("the cut is on the path")
    cut = (pts[i][0] + (pts[i + 1][0] - pts[i][0]) * t, pts[i][1] + (pts[i + 1][1] - pts[i][1]) * t)
    out = {k2: v for k2, v in e.items() if k2 not in ("pts", "zs", "ms", "widths", "bulges")}
    if at_end:
        out["pts"] = [p(*v) for v in pts[:i + 1]] + [p(*cut)]
    else:
        out["pts"] = [p(*cut)] + [p(*v) for v in pts[i + 1:]]
    for key in ("zs", "ms"):
        if key in e:
            vals = e[key]
            at_cut = num(vals[i] + (vals[i + 1] - vals[i]) * t)
            out[key] = [num(v) for v in vals[:i + 1]] + [at_cut] if at_end else [at_cut] + [num(v) for v in vals[i + 1:]]
    if "widths" in e:
        w = e["widths"]
        w0, w1 = w[i]
        at_cut = num(w0 + (w1 - w0) * t)
        if at_end:
            out["widths"] = [list(x) for x in w[:i]] + [[num(w0), at_cut], list(w[-1])]
        else:
            out["widths"] = [[at_cut, num(w1)]] + [list(x) for x in w[i + 1:]]
    return out


def toward(e, at_end, q):
    """The new length Dinamik gives: the ADR's rule for a line and an arc."""
    if e["kind"] == "line":
        a, b = xy(e["a"]), xy(e["b"])
        fixed, moving = (a, b) if at_end else (b, a)
        L = math.hypot(moving[0] - fixed[0], moving[1] - fixed[1])
        t = ((q[0] - fixed[0]) * (moving[0] - fixed[0]) + (q[1] - fixed[1]) * (moving[1] - fixed[1])) / L
        return t if t > L else max(0.0, min(L, t))
    if e["kind"] == "arc":
        c = xy(e["c"])
        seen = math.atan2(q[1] - c[1], q[0] - c[0])
        return e["r"] * norm(seen - e["a0"] if at_end else e["a1"] - seen)
    raise AssertionError(e["kind"])


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def completed(output, warnings=()):
    return {"status": "completed", "warnings": list(warnings), "output": output}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def inp(uid, end, change):
    return {"uid": U(uid), "end": end, "change": change}


def case(name, uid, end, change, after, exact=False, undo=False):
    """`after`: the object as the command writes it. Exact cases compare the write too."""
    i = inp(uid, end, change)
    plan = {"op": "plan", "input": i, "within": WITHIN,
            "result": completed({"changed": [after], "created": [], "removed": [], "revision": "$current"}),
            "expect": {"revision": "same"}}
    steps = [plan, {"op": "execute", "input": i,
                    "result": completed({"changed": [U(uid)], "created": [], "removed": [], "revision": "$current"}),
                    "expect": {**({"entities": {str(uid): after}} if exact else {}), "revision": "changed", "canUndo": True}}]
    if undo:
        steps.append({"op": "undo", "returns": "Uzat-kısalt", "expect": {"entities": {str(uid): BY_ID[uid]}}})
    return {"name": name, "steps": steps}


def refused(name, input_, code, message, path, non_finite=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = failed(code, message, path)
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


NOT_LENGTHENABLE = ("Uzat-kısalt çizgi, yay, elips yayı ve açık çoklu çizgide çalışır. Kapalı çoklu çizgiyi önce açın; daire "
                    "ve bütün elips için Kır kullanın.")
INVALID_LENGTH = "Yeni uzunluk sıfırdan büyük olmalı. Daha küçük bir kısaltma ya da sıfırdan büyük bir uzunluk verin."
CLOSES = "Yay bu uzunlukta kendini kapatır; en çok bir tam daireden kısa olabilir. Daha kısa bir uzunluk ya da açı verin."
ANGLE = "Açıyla yalnız yay uzatılır ya da kısaltılır. Çizgiye, elips yayına ve çoklu çizgiye uzunluk verin."
CHANGE = "Fark ve Toplam'da uzunluk (length) ya da açıdan (angle) yalnız birini verin."
MULTI = "Bu işlem çok parçalı nesnede çalışmaz; önce Parçalara ayır ile parçalarına ayırın."


def cases():
    L1, L3 = length_of(BY_ID[1]), length_of(BY_ID[3])
    arc, ell = BY_ID[2], BY_ID[5]
    out = [
        case("Fark: çizginin ucu 2 m uzar; ucun kotu ve ölçüsü (M) uç kenar boyunca sürer; adım “Uzat-kısalt”",
             1, "end", {"kind": "delta", "length": 2}, lengthened(BY_ID[1], True, L1 + 2), exact=True, undo=True),
        case("Fark eksi: çizginin başı 3 m kısalır; kalan uçların kotu yerinden", 1, "start", {"kind": "delta", "length": -3},
             {**BY_ID[1], "a": p(3, 0), "za": 1.6, "ma": 3}),
        case("Fark: çizginin başı 5 m uzar; başın kotu geriye sürer", 1, "start", {"kind": "delta", "length": 5},
             lengthened(BY_ID[1], False, L1 + 5), exact=True),
        case("Fark: yay ucundan kendi çemberinde uzar", 2, "end", {"kind": "delta", "length": 2.5},
             lengthened(arc, True, length_of(arc) + 2.5)),
        case("Fark açıyla: yayın açısı 45° büyür, başı geriye", 2, "start", {"kind": "delta", "angle": math.pi / 4},
             lengthened(arc, False, arc["r"] * (math.pi / 2 + math.pi / 4))),
        case("Toplam açıyla: yay yarım daire olur", 2, "end", {"kind": "total", "angle": math.pi},
             lengthened(arc, True, arc["r"] * math.pi)),
        refused("Toplam açı bir tam tur: yay kendini kapatır", inp(2, "end", {"kind": "total", "angle": TAU}), "arc_closes", CLOSES,
                "change"),
        refused("Açı yalnız yayda: çizgiye açı angle_needs_arc", inp(1, "end", {"kind": "delta", "angle": 0.5}), "angle_needs_arc",
                ANGLE, "change.angle"),
        case("Yüzde: çizgi %150", 1, "end", {"kind": "percent", "percent": 150}, lengthened(BY_ID[1], True, L1 * 1.5)),
        case("Yüzde: çoklu çizgi %40 başından kısalır; kalan köşeler kotunu ve ölçüsünü korur, yeni baş kesilen kenarınkini alır",
             3, "start", {"kind": "percent", "percent": 40}, lengthened(BY_ID[3], False, L3 * 0.4)),
        refused("Yüzde sıfır: yeni uzunluk sıfırdan büyük değil", inp(1, "end", {"kind": "percent", "percent": 0}), "invalid_length",
                INVALID_LENGTH, "change"),
        case("Toplam: çoklu çizgi 25 m'ye uzar; kenarlar genişliklerini korur, ucun kotu ve ölçüsü sürer", 3, "end",
             {"kind": "total", "length": 25}, lengthened(BY_ID[3], True, 25)),
        case("Toplam: çoklu çizgi 15 m'ye kısalır; yeni uç kesilen kenarın kotunu, ölçüsünü ve genişliğini uzunlukla alır", 3, "end",
             {"kind": "total", "length": 15}, lengthened(BY_ID[3], True, 15)),
        case("Fark: çoklu çizginin yaylı uç kenarı kendi çemberinde uzar, bükümü yeni açısının", 4, "end",
             {"kind": "delta", "length": 3}, lengthened(BY_ID[4], True, length_of(BY_ID[4]) + 3)),
        case("Fark: elips yayı ucundan kendi elipsinde uzar", 5, "end", {"kind": "delta", "length": 1},
             lengthened(ell, True, length_of(ell) + 1)),
        case("Toplam: elips yayı başından kısalır", 5, "start", {"kind": "total", "length": 3},
             lengthened(ell, False, 3.0)),
        refused("Toplam sıfır: invalid_length", inp(1, "end", {"kind": "total", "length": 0}), "invalid_length", INVALID_LENGTH,
                "change"),
        refused("Fark çizgiden uzun eksi: invalid_length", inp(1, "end", {"kind": "delta", "length": -12}), "invalid_length",
                INVALID_LENGTH, "change"),
        case("Dinamik: imleç ucun ötesinde, çizgi doğrusunda uzar", 1, "end", {"kind": "dynamic", "to": p(14, 3)},
             lengthened(BY_ID[1], True, toward(BY_ID[1], True, (14, 3)))),
        case("Dinamik: imleç çizginin üstünde, en yakın yerine kısalır", 1, "end", {"kind": "dynamic", "to": p(6, 1)},
             lengthened(BY_ID[1], True, toward(BY_ID[1], True, (6, 1)))),
        case("Dinamik: yayın ucu imlecin merkezden göründüğü açıya", 2, "end",
             {"kind": "dynamic", "to": p(30 + 5 * math.cos(2.1), 5 * math.sin(2.1))},
             lengthened(arc, True, toward(arc, True, (30 + 5 * math.cos(2.1), 5 * math.sin(2.1))))),
        case("Çizgi eğik: Toplam 10 m (3-4-5)", 11, "end", {"kind": "total", "length": 10}, {**BY_ID[11], "b": p(6, 48)},
             exact=True),
        refused("Kapalı çoklu çizgi uzamaz", inp(6, "end", {"kind": "delta", "length": 1}), "not_lengthenable", NOT_LENGTHENABLE, "uid"),
        refused("Daire uzamaz", inp(7, "end", {"kind": "delta", "length": 1}), "not_lengthenable", NOT_LENGTHENABLE, "uid"),
        refused("Alan uzamaz", inp(8, "end", {"kind": "delta", "length": 1}), "not_lengthenable", NOT_LENGTHENABLE, "uid"),
        refused("Bütün elips uzamaz", inp(12, "end", {"kind": "delta", "length": 1}), "not_lengthenable", NOT_LENGTHENABLE, "uid"),
        refused("Nokta uzamaz", inp(13, "end", {"kind": "delta", "length": 1}), "not_lengthenable", NOT_LENGTHENABLE, "uid"),
        refused("Uçları aynı yerde olan çizginin uzunluğu yok", inp(14, "end", {"kind": "delta", "length": 1}), "not_lengthenable",
                "Nesnenin uzunluğu yok. Uzat-kısalt uzunluğu olan nesnenin ucunu taşır.", "uid"),
        refused("Çok parçalı çoklu çizgi: multi_part", inp(10, "end", {"kind": "delta", "length": 1}), "multi_part", MULTI, "uid"),
        refused("Kilitli katmandaki çizgi: layer_locked", inp(9, "end", {"kind": "delta", "length": 1}), "layer_locked",
                "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uid"),
        refused("Fark'ta uzunluk da açı da: invalid_change", inp(2, "end", {"kind": "delta", "length": 1, "angle": 0.1}),
                "invalid_change", CHANGE, "change"),
        refused("Toplam'da ikisi de yok: invalid_change", inp(2, "end", {"kind": "total"}), "invalid_change", CHANGE, "change"),
        refused("Fark sonlu değil", inp(1, "end", {"kind": "delta", "length": 1}), "not_finite",
                "Uzunluk farkı sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "change.length",
                non_finite={"change.length": "NaN"}),
        refused("Dinamik'in noktası sonlu değil", inp(1, "end", {"kind": "dynamic", "to": p(1, 1)}), "not_finite",
                "Hedef noktanın doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                "change.to.x", non_finite={"change.to.x": "Infinity"}),
        refused("Beklenen sürüm sayı değil", {**inp(1, "end", {"kind": "delta", "length": 1}), "expectedRevision": "on iki"},
                "invalid_revision", "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": inp(11, "end", {"kind": "delta", "length": 1}), "result": "$any"},
            {"op": "execute", "input": {**inp(1, "end", {"kind": "delta", "length": 1}), "expectedRevision": "$old"},
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
                            "yeniden hazırlayın."}}}]},
        refused("Kimlik UUID değil", {"uid": "on iki", "end": "end", "change": {"kind": "delta", "length": 1}}, "invalid_uid",
                "“on iki” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "uid"),
        refused("Çizimde olmayan nesne", {"uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "end": "end",
                                          "change": {"kind": "delta", "length": 1}}, "entity_not_found",
                "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. "
                "Var olan bir nesnenin kimliğini verin.", "uid"),
    ]
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.lengthen", "commandVersion": 1,
            "title": "Uzat-kısalt: Fark, Yüzde, Toplam ve Dinamik; yay açıyla",
            "note": "ADR 0298. Bağımsız başvuru: scripts/fixtures/lengthen_command_cases.py (açılar ve elips yayının uzunlukları "
                    "mpmath'le 40 basamakta).",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
