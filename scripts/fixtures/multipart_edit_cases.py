"""Kır, Buda and Uzat on a multi-part polyline: shared cases (docs/adr/0307).

    python3 scripts/fixtures/multipart_edit_cases.py           # writes the files
    python3 scripts/fixtures/multipart_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.{break,trim,extend}.parts.json. The ADR's rules, computed here without KentOS
code over a drawing of straight, axis-parallel edges (every crossing and length exact):

- a pick acts on the part it is near; Kır's two points on one part; the other parts stay as they are;
- the pieces stay parts of the one object, in place of the part they came from: its id, attributes, look, elevations
  and measures are kept; Buda taking every part away removes the object;
- a part's piece an earlier pick left that no boundary cuts goes whole (Buda's rule for pieces); in quick mode a part
  no boundary cuts goes alone;
- Uzat grows the end of the part nearest the pick to the first boundary ahead, each end of each part once;
- elevations and measures are carried by ADR 0142's rules: on a source vertex its value, on a source edge linearly
  along it, at a grown end the end edge's grade carried on;
- an area of several parts, an area with holes and a locked layer are refused as before.
"""
import copy
import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def num(v):
    v = F(v)
    return int(v) if v.denominator == 1 else float(v)


def P(x, y):
    return {"x": num(x), "y": num(y)}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


# ── The drawing ────────────────────────────────────────────────────────────────────────────────────────────────────

# The multi-part line: each part its points, elevations and measures (None: the part has none).
HAT = [
    {"pts": [(0, 0), (10, 0)], "zs": [10, 20], "ms": [0, 10]},
    {"pts": [(0, 5), (10, 5), (10, 9)], "zs": [0, 10, 14], "ms": [0, 10, 14]},
    {"pts": [(0, 12), (4, 12)], "zs": None, "ms": None},
]
# The boundaries, on their own layer.
BOUNDS = [((6, -2), (6, 14)), ((-2, 7), (12, 7)), ((14, -2), (14, 4)), ((8, 16), (12, 16)), ((-3, -2), (-3, 6))]
HAT_ID, HOLED, AREAS, LOCKED, FAR = 1, 7, 8, 9, 10


def line_json(i, a, b, lay="sinir"):
    return {"kind": "line", "id": i, "layerId": lay, "attrs": {}, "a": P(*a), "b": P(*b)}


def polyline_json(i, parts, lay="cizim", base=None):
    """A polyline of parts: the first's fields its own, the others its parts."""
    e = {"kind": "polyline", "id": i, "layerId": lay, "attrs": {}}
    if base:
        e.update(base)

    def fields(part):
        out = {"pts": [P(*p) for p in part["pts"]]}
        if part.get("zs") is not None:
            out["zs"] = [num(z) if z is not None else None for z in part["zs"]]
        if part.get("ms") is not None:
            out["ms"] = [num(m) if m is not None else None for m in part["ms"]]
        return out

    e.update(fields(parts[0]))
    if len(parts) > 1:
        e["parts"] = [fields(p) for p in parts[1:]]
    return e


HAT_BASE = {"attrs": {"Ad": "Hat 1"}, "color": "#30A46C"}


def setup():
    rect = lambda x0, y0, x1, y1: [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]  # noqa: E731
    entities = [polyline_json(HAT_ID, HAT, base=HAT_BASE)]
    entities += [line_json(2 + k, a, b) for k, (a, b) in enumerate(BOUNDS)]
    entities.append({"kind": "polygon", "id": HOLED, "layerId": "cizim", "attrs": {}, "pts": rect(30, 0, 40, 10),
                     "holes": [{"pts": rect(33, 3, 37, 7)}]})
    entities.append({"kind": "polygon", "id": AREAS, "layerId": "cizim", "attrs": {}, "pts": rect(50, 0, 54, 4),
                     "parts": [{"pts": rect(56, 0, 60, 4)}]})
    entities.append(polyline_json(LOCKED, [{"pts": [(30, 20), (40, 20)]}, {"pts": [(30, 22), (40, 22)]}], lay="kilitli"))
    entities.append(polyline_json(FAR, [{"pts": [(70, 0), (74, 0)]}, {"pts": [(70, 2), (74, 2)]}]))
    return {
        "format": "kentos.document", "version": 1, "name": "Çok parçalı çizgi",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100, "workspace": "cad"},
        "origin": P(0, 0),
        "layers": [layer("cizim", "Çizim"), layer("sinir", "Sınır"), layer("kilitli", "Kilitli", locked=True)],
        "activeLayer": "cizim",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── Paths, crossings and carried values (exact) ────────────────────────────────────────────────────────────────────

def pt(p):
    return (F(p[0]), F(p[1]))


def seg_len(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    assert dx == 0 or dy == 0, "axis-parallel edges only"
    return abs(dx) + abs(dy)


def length(pts):
    return sum(seg_len(pt(a), pt(b)) for a, b in zip(pts, pts[1:]))


def at_s(pts, s):
    run = F(0)
    for a, b in zip(pts, pts[1:]):
        a, b = pt(a), pt(b)
        l = seg_len(a, b)
        if s <= run + l:
            t = (s - run) / l
            return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
        run += l
    return pt(pts[-1])


def s_of(pts, p):
    """The arc length of a point on the path."""
    p = pt(p)
    run = F(0)
    for a, b in zip(pts, pts[1:]):
        a, b = pt(a), pt(b)
        lo_x, hi_x = sorted((a[0], b[0]))
        lo_y, hi_y = sorted((a[1], b[1]))
        if lo_x <= p[0] <= hi_x and lo_y <= p[1] <= hi_y:
            return run + seg_len(a, p)
        run += seg_len(a, b)
    raise ValueError(p)


def sub_path(pts, s0, s1):
    out = [at_s(pts, s0)]
    run = F(0)
    for a, b in zip(pts, pts[1:]):
        run += seg_len(pt(a), pt(b))
        if s0 < run < s1:
            out.append(pt(b))
    out.append(at_s(pts, s1))
    return out


def crossing(a, b, c, d):
    """Where segment ab meets cd (axis-parallel, crossing at right angles), or None."""
    a, b, c, d = map(pt, (a, b, c, d))
    if a[0] == b[0] and c[1] == d[1]:
        x, y = a[0], c[1]
        ok = min(c[0], d[0]) <= x <= max(c[0], d[0]) and min(a[1], b[1]) <= y <= max(a[1], b[1])
        return (x, y) if ok else None
    if a[1] == b[1] and c[0] == d[0]:
        return crossing(c, d, a, b)
    return None


def cuts(pts):
    out = set()
    for a, b in zip(pts, pts[1:]):
        for c, d in BOUNDS:
            q = crossing(a, b, c, d)
            if q is not None:
                out.add(s_of(pts, q))
    return sorted(out)


def carried(source, p):
    """A value (elevation or measure) at p on the source part: on a vertex its own, on an edge linearly along it."""
    pts, vals = source
    if vals is None:
        return None
    p = pt(p)
    for q, v in zip(pts, vals):
        if pt(q) == p:
            return v
    for (a, va), (b, vb) in zip(zip(pts, vals), zip(pts[1:], vals[1:])):
        a, b = pt(a), pt(b)
        if min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1]):
            if va is None or vb is None:
                return None
            return F(va) + (F(vb) - F(va)) * seg_len(a, p) / seg_len(a, b)
    raise ValueError(p)


def piece(part, new_pts):
    """A piece of `part` with its points' values carried."""
    return {"pts": new_pts,
            "zs": None if part["zs"] is None else [carried((part["pts"], part["zs"]), p) for p in new_pts],
            "ms": None if part["ms"] is None else [carried((part["pts"], part["ms"]), p) for p in new_pts]}


def near_part(parts, p):
    """The part nearest p (axis-parallel distance to its edges)."""
    p = pt(p)

    def dist(part):
        best = None
        for a, b in zip(part["pts"], part["pts"][1:]):
            a, b = pt(a), pt(b)
            x = min(max(p[0], min(a[0], b[0])), max(a[0], b[0]))
            y = min(max(p[1], min(a[1], b[1])), max(a[1], b[1]))
            d2 = (p[0] - x) ** 2 + (p[1] - y) ** 2
            best = d2 if best is None else min(best, d2)
        return best

    return min(range(len(parts)), key=lambda k: (dist(parts[k]), k))


def nearest_s(pts, p):
    """The arc length of the point of the path nearest p."""
    p = pt(p)
    best, s_best, run = None, F(0), F(0)
    for a, b in zip(pts, pts[1:]):
        a, b = pt(a), pt(b)
        x = min(max(p[0], min(a[0], b[0])), max(a[0], b[0]))
        y = min(max(p[1], min(a[1], b[1])), max(a[1], b[1]))
        d2 = (p[0] - x) ** 2 + (p[1] - y) ** 2
        if best is None or d2 < best:
            best, s_best = d2, run + seg_len(a, (x, y))
        run += seg_len(a, b)
    return s_best


# ── The operations, as the ADR says them ───────────────────────────────────────────────────────────────────────────

NO_CUT = "Tıklanan kısmı kesen bir kenar yok."
NO_BOUND = "Bu doğrultuda ulaşılacak bir sınır yok."


def trim_part(part, pick):
    """Buda on one part (a piece as the object had it): the pieces left, or None when nothing cuts it."""
    pts = part["pts"]
    l = length(pts)
    s = nearest_s(pts, pick)
    cs = cuts(pts)
    lo = max([F(0)] + [c for c in cs if c < s])
    hi = min([l] + [c for c in cs if c > s])
    if lo == 0 and hi == l:
        return None
    out = []
    if lo > 0:
        out.append(piece(part, sub_path(pts, F(0), lo)))
    if hi < l:
        out.append(piece(part, sub_path(pts, hi, l)))
    return out


def grow(part, pick):
    """Uzat on one part: the end nearest the pick to the first boundary ahead; the part grown or a refusal."""
    pts = [pt(p) for p in part["pts"]]
    at_end = nearest_s(part["pts"], pick) > length(part["pts"]) / 2
    end, before = (pts[-1], pts[-2]) if at_end else (pts[0], pts[1])
    d = (end[0] - before[0], end[1] - before[1])
    best = None
    for c, dd in BOUNDS:
        far = (end[0] + d[0] * 1000, end[1] + d[1] * 1000)
        q = crossing(end, far, c, dd)
        if q is not None and q != end:
            t = seg_len(end, q)
            if best is None or t < best[0]:
                best = (t, q)
    if best is None:
        return None, at_end
    t, q = best
    new = pts[:-1] + [q] if at_end else [q] + pts[1:]

    def carry(vals):
        if vals is None:
            return None
        out = list(vals)
        i, j = (-1, -2) if at_end else (0, 1)
        grade = (F(vals[i]) - F(vals[j])) / seg_len(pts[i], pts[j])
        out[i] = F(vals[i]) + grade * t
        return out

    return {"pts": new, "zs": carry(part["zs"]), "ms": carry(part["ms"])}, at_end


def break_part(part, first, second):
    pts = part["pts"]
    l = length(pts)
    s1, s2 = nearest_s(pts, first), nearest_s(pts, second)
    if s1 == s2:
        if s1 in (0, l):
            return "Nesne ucundan kırılamaz; iç kısmında bir nokta gösterin."
        return [piece(part, sub_path(pts, F(0), s1)), piece(part, sub_path(pts, s1, l))]
    lo, hi = sorted((s1, s2))
    out = []
    if lo > 0:
        out.append(piece(part, sub_path(pts, F(0), lo)))
    if hi < l:
        out.append(piece(part, sub_path(pts, hi, l)))
    return out


# ── Steps ──────────────────────────────────────────────────────────────────────────────────────────────────────────

U = lambda i: f"$uidOf:{i}"  # noqa: E731


def hat(parts):
    return polyline_json(HAT_ID, parts, base=HAT_BASE)


def done(command_input, parts, within=False, warnings=(), op="execute"):
    """The one object written (or, planned, shown) with `parts`; the plan gives the object itself."""
    changed = [U(HAT_ID)] if op == "execute" else [hat(parts)]
    step = {"op": op, "input": command_input,
            "result": {"status": "completed",
                       "output": {"changed": changed, "created": [], "removed": [], "revision": "$current"},
                       "warnings": list(warnings)},
            "expect": {"entities": {str(HAT_ID): hat(parts if op == "execute" else HAT)},
                       "revision": "changed" if op == "execute" else "same"}}
    if op == "execute":
        step["expect"]["canUndo"] = True
    if within:
        step["within"] = 1e-9
    return step


def refused(command_input, code, message, path, status="failed"):
    return {"op": "execute", "input": command_input,
            "result": {"status": status, "error": {"code": code, "message": message, "path": path}},
            "expect": {"revision": "same"}}


def validate(command_input, warnings=()):
    return {"op": "validate", "input": command_input,
            "result": {"status": "completed", "output": None, "warnings": list(warnings)},
            "expect": {"revision": "same"}}


def conflict(command_input):
    """A revision the drawing has moved past: nothing is written."""
    return [{"op": "captureRevision", "as": "old"},
            {"op": "execute", "command": "cad.entities.transform",
             "input": {"uids": [U(2)], "transform": {"kind": "move", "dx": 0, "dy": 0.5}}, "result": "$any"},
            {"op": "execute", "input": {**command_input, "expectedRevision": "$old"},
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki "
                            "hâline göre yeniden hazırlayın."}},
             "expect": {"revision": "same"}}]


def redo(label, parts):
    return {"op": "redo", "returns": label, "expect": {"entities": {str(HAT_ID): hat(parts)}}}


def undo(label):
    return {"op": "undo", "returns": label, "expect": {"entities": {str(HAT_ID): hat(HAT)}}}


LOCKED_MSG = "“Kilitli” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
HOLES_TRIM = ("Adalı alanda budama yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl "
              "kullanın.")
HOLES_BREAK = ("Adalı alanda kırma yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl "
               "kullanın.")
MULTI = "Bu işlem çok parçalı nesnede çalışmaz; önce Parçalara ayır ile parçalarına ayırın."


def picks(*pairs):
    return [{"uid": U(i), "at": P(*p)} for i, p in pairs]


# Buda ---------------------------------------------------------------------------------------------------------------

class Trim:
    """Buda's walk over one multi-part line: the pieces and whether each is as the object had it."""

    def __init__(self, parts):
        self.pieces = copy.deepcopy(parts)
        self.fresh = [True] * len(parts)

    def pick(self, at, erase=False):
        k = near_part(self.pieces, at)
        got = trim_part(self.pieces[k], at)
        if got is None:
            if not self.fresh[k] or erase:
                got = []
            else:
                return NO_CUT
        self.pieces[k:k + 1] = got
        self.fresh[k:k + 1] = [False] * len(got)
        return None


def trimmed(*ats, erase=False):
    t = Trim(HAT)
    whys = [t.pick(a, erase) for a in ats]
    return t.pieces, whys


def trim_cases():
    out = []
    p, _ = trimmed((8, 0))
    out.append({"name": "Buda tıklanan parçayı keser: kesenle uç arası gider, öbür parçalar yerinde; nesne kimliğini, "
                        "özniteliklerini, rengini, kotunu ve ölçüsünü korur; adım “Buda”",
                "steps": [done({"picks": picks((HAT_ID, (8, 0)))}, p, within=True), undo("Buda")]})
    p, _ = trimmed((2, 0))
    out.append({"name": "Parçanın öbür ucu: uçla kesen arası gider, kot ve ölçü kesimde kenar boyunca",
                "steps": [done({"picks": picks((HAT_ID, (2, 0)))}, p, within=True)]})
    p, _ = trimmed((8, 5))
    out.append({"name": "İki kesen arası gider: parça iki parça olur, ikisi de aynı nesnenin parçası",
                "steps": [done({"picks": picks((HAT_ID, (8, 5)))}, p, within=True)]})
    p, _ = trimmed((10, 8))
    out.append({"name": "Köşeli parçanın ucundaki kenar: kesenle uç arası gider, köşe kalır",
                "steps": [done({"picks": picks((HAT_ID, (10, 8)))}, p, within=True)]})
    p, whys = trimmed((8, 0), (2, 12))
    out.append({"name": "Kesilmeyen parçaya tıklama uyarıdır, öbürü kesilir",
                "steps": [done({"picks": picks((HAT_ID, (8, 0)), (HAT_ID, (2, 12)))}, p, within=True,
                               warnings=[{"code": "trim_failed", "message": whys[1], "path": "picks[1]"}])]})
    p, _ = trimmed((2, 12), erase=True)
    out.append({"name": "Hızlı kipte kesilmeyen parça yalnız kendisi gider, nesne kalır",
                "steps": [done({"picks": picks((HAT_ID, (2, 12))), "eraseUncut": True}, p)]})
    p, _ = trimmed((8, 5), (3, 5))
    out.append({"name": "Önceki kesimin bıraktığı, kesen olmayan parça bütünüyle gider",
                "steps": [done({"picks": picks((HAT_ID, (8, 5)), (HAT_ID, (3, 5)))}, p, within=True)]})
    p, _ = trimmed((8, 0), (8, 5))
    out.append({"name": "Çit iki parçayı keser: her kesişim bir tıklama",
                "steps": [done({"picks": [], "fence": [P(8, -1), P(8, 6)]}, p, within=True)]})
    p, _ = trimmed((8, 0))
    out.append({"name": "Kesişen pencere parçanın penceredeki kısmını keser",
                "steps": [done({"picks": [], "crossing": {"min": P(7, -1), "max": P(9, 1)}}, p, within=True)]})
    out.append({"name": "Hızlı kipte bütün parçalar giderse nesne silinir",
                "steps": [{"op": "captureUid", "id": FAR, "as": "far"},
                          {"op": "execute", "input": {"picks": picks((FAR, (72, 0)), (FAR, (72, 2))), "eraseUncut": True},
                           "result": {"status": "completed",
                                      "output": {"changed": [], "created": [], "removed": ["$uid:far"],
                                                 "revision": "$current"}, "warnings": []},
                           "expect": {"revision": "changed", "canUndo": True}}]})
    out.append({"name": "Kilitli katmandaki çok parçalı çizgi budanmaz",
                "steps": [refused({"picks": picks((LOCKED, (35, 20)))}, "layer_locked", LOCKED_MSG, "picks[0]")]})
    out.append({"name": "Delikli alan budanmaz: delikler kaybolurdu",
                "steps": [refused({"picks": picks((HOLED, (35, 0)))}, "not_trimmable", HOLES_TRIM, "picks[0]")]})
    out.append({"name": "Çok parçalı alan budanmaz: parçaları alan olarak kalamazdı",
                "steps": [refused({"picks": picks((AREAS, (52, 0)))}, "trim_failed", MULTI, "picks")]})
    p, _ = trimmed((8, 5))
    out.append({"name": "Plan yazmadan gösterir",
                "steps": [done({"picks": picks((HAT_ID, (8, 5)))}, p, within=True, op="plan")]})
    p, _ = trimmed((3, 5))
    out.append({"name": "Köşeli parçanın başı: uçla ilk kesen arası gider, kalan köşesiyle",
                "steps": [done({"picks": picks((HAT_ID, (3, 5)))}, p, within=True)]})
    p, _ = trimmed((10, 8), (8, 5))
    out.append({"name": "Kesilmiş parçanın kalanı yeniden kesilir",
                "steps": [done({"picks": picks((HAT_ID, (10, 8)), (HAT_ID, (8, 5)))}, p, within=True)]})
    _, whys = trimmed((8, 0), (2, 12))
    out.append({"name": "Doğrulama kesilmeyen parçanın uyarısını verir",
                "steps": [validate({"picks": picks((HAT_ID, (8, 0)), (HAT_ID, (2, 12)))},
                                   [{"code": "trim_failed", "message": whys[1], "path": "picks[1]"}])]})
    out.append({"name": "Çizim değiştiyse conflict, hiçbir şey yazılmaz",
                "steps": conflict({"picks": picks((HAT_ID, (8, 0)))})})
    out.append({"name": "Çit yalnız kesilmeyen parçayı geçerse ret",
                "steps": [refused({"picks": [], "fence": [P(2, 11), P(2, 13)]}, "trim_failed", NO_CUT, "fence")]})
    p, _ = trimmed((2, 12), erase=True)
    out.append({"name": "Hızlı kipte kesişen pencere kesilmeyen parçayı alır",
                "steps": [done({"picks": [], "crossing": {"min": P(1, 11), "max": P(3, 13)}, "eraseUncut": True}, p)]})
    return out


# Uzat ---------------------------------------------------------------------------------------------------------------

def grown(*ats):
    parts = copy.deepcopy(HAT)
    used = set()
    whys = []
    for at in ats:
        k = near_part(parts, at)
        g, at_end = grow(parts[k], at)
        if (k, at_end) in used:
            whys.append(None)
            continue
        used.add((k, at_end))
        if g is None:
            whys.append(NO_BOUND)
            continue
        parts[k] = g
        whys.append(None)
    return parts, whys


def extend_cases():
    out = []
    p, _ = grown((9, 0))
    out.append({"name": "Uzat tıklanan parçanın yakın ucunu ilk sınıra uzatır; kot ve ölçü uç kenarın eğimiyle sürer; "
                        "adım “Uzat”",
                "steps": [done({"picks": picks((HAT_ID, (9, 0)))}, p, within=True), undo("Uzat")]})
    p, _ = grown((3, 12))
    out.append({"name": "Kotsuz parçanın ucu: kot ve ölçü yok kalır",
                "steps": [done({"picks": picks((HAT_ID, (3, 12)))}, p)]})
    p, _ = grown((10, 8.5))
    out.append({"name": "Köşeli parçanın son kenarı kendi doğrultusunda uzar",
                "steps": [done({"picks": picks((HAT_ID, (10, 8.5)))}, p, within=True)]})
    p, _ = grown((9, 0), (1, 0))
    out.append({"name": "Aynı parçanın iki ucu: ikisi de uzar",
                "steps": [done({"picks": picks((HAT_ID, (9, 0)), (HAT_ID, (1, 0)))}, p, within=True)]})
    p, _ = grown((9, 0), (9.5, 0))
    out.append({"name": "Aynı uç bir kez uzar",
                "steps": [done({"picks": picks((HAT_ID, (9, 0)), (HAT_ID, (9.5, 0)))}, p, within=True)]})
    p, _ = grown((1, 5))
    out.append({"name": "Parçanın başı geriye doğru uzar",
                "steps": [done({"picks": picks((HAT_ID, (1, 5)))}, p, within=True)]})
    p, _ = grown((4, 12))
    out.append({"name": "Kesişen pencerenin içindeki parça ucu uzar",
                "steps": [done({"picks": [], "crossing": {"min": P(3.5, 11.5), "max": P(4.5, 12.5)}}, p)]})
    out.append({"name": "Önünde sınır olmayan uç uzamaz",
                "steps": [refused({"picks": picks((HAT_ID, (1, 12)))}, "extend_failed", NO_BOUND, "picks")]})
    out.append({"name": "Kilitli katmandaki çok parçalı çizgi uzatılmaz",
                "steps": [refused({"picks": picks((LOCKED, (39, 20)))}, "layer_locked", LOCKED_MSG, "picks[0]")]})
    p, _ = grown((10, 8.5))
    out.append({"name": "Plan yazmadan gösterir",
                "steps": [done({"picks": picks((HAT_ID, (10, 8.5)))}, p, within=True, op="plan")]})
    p, _ = grown((9, 0), (10, 8.5), (3, 12))
    out.append({"name": "Üç parçanın uçları birlikte uzar, tek adımda",
                "steps": [done({"picks": picks((HAT_ID, (9, 0)), (HAT_ID, (10, 8.5)), (HAT_ID, (3, 12)))}, p,
                               within=True)]})
    p, _ = grown((9, 0))
    out.append({"name": "Çit parçanın ucunun yanından geçer: o uç uzar",
                "steps": [done({"picks": [], "fence": [P(9, -1), P(9, 1)]}, p, within=True)]})
    out.append({"name": "Doğrulama yazmaz",
                "steps": [validate({"picks": picks((HAT_ID, (9, 0)))})]})
    out.append({"name": "Çizim değiştiyse conflict, hiçbir şey yazılmaz",
                "steps": conflict({"picks": picks((HAT_ID, (9, 0)))})})
    out.append({"name": "Çok parçalı alan uzatılmaz",
                "steps": [refused({"picks": picks((AREAS, (52, 0)))}, "extend_failed",
                                  "Yalnız açık çizgiler, çoklu çizgiler ve yaylar uzatılabilir.", "picks")]})
    p, _ = grown((1, 0))
    out.append({"name": "Parçanın başı: kot ve ölçü geriye doğru eğimle",
                "steps": [done({"picks": picks((HAT_ID, (1, 0)))}, p, within=True)]})
    p, _ = grown((1, 5), (10, 8.5))
    out.append({"name": "Köşeli parçanın iki ucu",
                "steps": [done({"picks": picks((HAT_ID, (1, 5)), (HAT_ID, (10, 8.5)))}, p, within=True)]})
    p, _ = grown((3, 12), (3.5, 12))
    out.append({"name": "Kotsuz parçanın aynı ucu bir kez",
                "steps": [done({"picks": picks((HAT_ID, (3, 12)), (HAT_ID, (3.5, 12)))}, p)]})
    p, _ = grown((9, 0))
    out.append({"name": "Geri alınan uzatma yinelenir",
                "steps": [done({"picks": picks((HAT_ID, (9, 0)))}, p, within=True), undo("Uzat"), redo("Uzat", p)]})
    p, _ = grown((10, 8.5))
    out.append({"name": "Kesişen pencere köşeli parçanın ucunu uzatır",
                "steps": [done({"picks": [], "crossing": {"min": P(9.5, 8.5), "max": P(10.5, 9.5)}}, p, within=True)]})
    return out


# Kır ----------------------------------------------------------------------------------------------------------------

def broken(first, second=None):
    parts = copy.deepcopy(HAT)
    k = near_part(parts, first)
    got = break_part(parts[k], first, second if second is not None else first)
    if isinstance(got, str):
        return got
    parts[k:k + 1] = got
    return parts


def break_input(first, second=None, uid=HAT_ID):
    out = {"uid": U(uid), "first": P(*first)}
    if second is not None:
        out["second"] = P(*second)
    return out


def break_cases():
    out = []
    out.append({"name": "Kır tek noktada parçayı ikiye böler: iki parça aynı nesnede; kimlik, öznitelikler, kot ve ölçü "
                        "kalır; adım “Kır”",
                "steps": [done(break_input((4, 0)), broken((4, 0)), within=True), undo("Kır")]})
    out.append({"name": "İki nokta arası gider: köşeli parçada iki parça kalır",
                "steps": [done(break_input((2, 5), (10, 8)), broken((2, 5), (10, 8)), within=True)]})
    out.append({"name": "Kotsuz parça ikiye bölünür",
                "steps": [done(break_input((2, 12)), broken((2, 12)))]})
    out.append({"name": "İki nokta arası parçanın ucuna dek: tek parça kalır",
                "steps": [done(break_input((4, 0), (10, 0)), broken((4, 0), (10, 0)), within=True)]})
    out.append({"name": "İki nokta başka parçalarda: ret",
                "steps": [refused(break_input((2, 5), (8, 0)), "break_failed",
                                  "İki kırma noktası çok parçalı çizginin aynı parçasında olmalı; ikinci noktayı ilkinin "
                                  "parçasında verin.", "second")]})
    out.append({"name": "Parçanın ucundan kırılmaz",
                "steps": [refused(break_input((4, 12)), "break_failed", broken((4, 12)), "first")]})
    out.append({"name": "Çok parçalı alan kırılmaz: parçaları alan olarak kalamazdı",
                "steps": [refused(break_input((52, 0), uid=AREAS), "break_failed", MULTI, "first")]})
    out.append({"name": "Delikli alan kırılmaz: delikler kaybolurdu",
                "steps": [refused(break_input((35, 0), uid=HOLED), "not_breakable", HOLES_BREAK, "uid")]})
    out.append({"name": "Kilitli katmandaki çok parçalı çizgi kırılmaz",
                "steps": [refused(break_input((35, 20), uid=LOCKED), "layer_locked", LOCKED_MSG, "uid")]})
    out.append({"name": "Plan yazmadan gösterir",
                "steps": [done(break_input((2, 5), (10, 8)), broken((2, 5), (10, 8)), within=True, op="plan")]})
    out.append({"name": "Köşeli parçanın yatay kenarından tek noktada",
                "steps": [done(break_input((6, 5)), broken((6, 5)), within=True)]})
    out.append({"name": "Köşeli parçanın düşey kenarından tek noktada",
                "steps": [done(break_input((10, 7)), broken((10, 7)), within=True)]})
    out.append({"name": "Aynı kenarda iki nokta arası gider",
                "steps": [done(break_input((1, 0), (3, 0)), broken((1, 0), (3, 0)), within=True)]})
    out.append({"name": "Noktaların sırası sonucu değiştirmez",
                "steps": [done(break_input((3, 0), (1, 0)), broken((3, 0), (1, 0)), within=True)]})
    out.append({"name": "Köşeyi içine alan iki nokta: köşe gider",
                "steps": [done(break_input((8, 5), (10, 6)), broken((8, 5), (10, 6)), within=True)]})
    out.append({"name": "Parçanın başından bir noktaya: tek parça kalır",
                "steps": [done(break_input((0, 12), (2, 12)), broken((0, 12), (2, 12)))]})
    out.append({"name": "Doğrulama yazmaz",
                "steps": [validate(break_input((4, 0)))]})
    out.append({"name": "Çizim değiştiyse conflict, hiçbir şey yazılmaz",
                "steps": conflict(break_input((4, 0)))})
    p = broken((4, 0))
    out.append({"name": "Geri alınan kırma yinelenir",
                "steps": [done(break_input((4, 0)), p, within=True), undo("Kır"), redo("Kır", p)]})
    out.append({"name": "Düşey kenarda iki nokta, sonu dahil: tek parça kalır",
                "steps": [done(break_input((10, 6), (10, 9)), broken((10, 6), (10, 9)), within=True)]})
    return out


FILES = {
    "cad.entities.trim": ("Buda: çok parçalı çizgi", trim_cases),
    "cad.entities.extend": ("Uzat: çok parçalı çizgi", extend_cases),
    "cad.entities.break": ("Kır: çok parçalı çizgi", break_cases),
}


def build(command):
    title, make = FILES[command]
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "ADR 0307. Bağımsız başvuru: scripts/fixtures/multipart_edit_cases.py (eksenlere paralel kenarlar, "
                    "kesirlerle).",
            "setup": setup(), "cases": make()}


def main():
    check = "--check" in sys.argv[1:]
    bad = 0
    for command in FILES:
        path = OUT / f"{command}.parts.json"
        data = build(command)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad = 1
            else:
                print(f"{path.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        else:
            path.write_text(text)
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return bad


if __name__ == "__main__":
    sys.exit(main())
