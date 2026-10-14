"""The shared cases of the product commands cad.entities.trim, .extend, .fillet and .chamfer (docs/adr/0250).

    python3 scripts/fixtures/edge_command_cases.py           # writes the files
    python3 scripts/fixtures/edge_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.{trim,extend,fillet,chamfer}.json. The checks, their order, codes, paths and
messages are written here by hand from the ADR. What the commands do is computed independently, from the operations'
definitions (docs/adr/0047, 0250), without KentOS code:

- Buda: the target is walked from its start; the boundaries cross it at distances s along it; the pick's distance is
  where it projects; the part between the nearest crossing before it and the nearest after it (or the target's ends)
  goes. Nothing goes when neither is a crossing. A later pick of the same object cuts the piece it lies on; a piece
  no boundary cuts between its ends goes whole, its ends being earlier cuts or the object's (docs/adr/0294).
- Uzat: the end nearer the pick goes on along its edge to the nearest boundary it meets; each end at most once.
- Boundaries: every shown object but the target (a hidden layer's are not), those meeting a rectangle, or those given.
- Köşe yuvarla: the tangent points lie r / tan(φ/2) from the corner along each side, the centre r / sin(φ/2) along
  the bisector; the arc is the minor one between the tangent points, its angles in [0, 2π). In a path the corner is
  replaced by the tangent points and the arc between them is a bulge tan((π − φ)/4), plus when the path turns left.
- Pah: the cut points lie d1 and d2 from the corner along its sides; in a path they replace it.
- Two lines keep the side of their picks; the first takes the arc or the cut, which is a new object from it.

The straight geometry is computed in exact fractions over a drawing whose every value is a double exactly. The
rounded corners' few transcendental values (tan and sin of the half angle, the bulge's tan((π − φ)/4)) are computed
to 60 digits by their series and rounded to the nearest double, not by the platform's libm (macOS's tan(π/8) is one
ulp off; the core's fdlibm is correctly rounded); acos 0 and atan2 of axis vectors are exact. At these coordinates
the tangent points round to whole metres.

`--check` rebuilds the files in memory and compares them with the ones on disk.
"""
import json
import math
import sys
from decimal import Decimal, getcontext
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
X, Y = 487000, 4420000


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def P(x, y):
    return {"x": num(x), "y": num(y)}


def num(v):
    if isinstance(v, F):
        assert v.denominator & (v.denominator - 1) == 0, v  # a dyadic: a double exactly
        v = float(v)
    return int(v) if float(v).is_integer() else v


def line(i, a, b, layer_id="yapi", **extra):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, **extra, "a": P(*a), "b": P(*b)}


ENTITIES = [
    line(10, (X + 10, Y + 10), (X + 26, Y + 10), attrs={"Tür": "Duvar"}, color="#E5484D", lineWeight=0.7),
    line(11, (X + 14, Y + 6), (X + 14, Y + 14)),
    line(12, (X + 22, Y + 6), (X + 22, Y + 14)),
    line(13, (X + 25, Y + 6), (X + 25, Y + 14)),
    line(14, (X + 30, Y + 20), (X + 34, Y + 20), attrs={"Tür": "Bordür"}),
    line(15, (X + 40, Y + 16), (X + 40, Y + 24)),
    line(16, (X + 18, Y + 6), (X + 18, Y + 14), layer_id="gizli"),
    line(17, (X + 10, Y + 30), (X + 20, Y + 30), layer_id="kilitli"),
    {"kind": "polygon", "id": 18, "layerId": "yapi", "attrs": {}, "pts": [P(X + 60, Y), P(X + 70, Y), P(X + 70, Y + 10), P(X + 60, Y + 10)],
     "holes": [{"pts": [P(X + 62, Y + 2), P(X + 64, Y + 2), P(X + 64, Y + 4), P(X + 62, Y + 4)]}]},
    {"kind": "polyline", "id": 20, "layerId": "yapi", "attrs": {"Tür": "Bordür"}, "pts": [P(X + 50, Y + 50), P(X + 66, Y + 50), P(X + 66, Y + 66)]},
    line(21, (X + 50, Y + 80), (X + 60, Y + 80), color="#3E63DD"),
    line(22, (X + 66, Y + 86), (X + 66, Y + 96)),
    {"kind": "polygon", "id": 23, "layerId": "yapi", "attrs": {"Ada": "101"},
     "pts": [P(X + 80, Y + 50), P(X + 90, Y + 50), P(X + 90, Y + 60), P(X + 80, Y + 60)]},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = 24
SHOWN = [e for e in ENTITIES if e["layerId"] != "gizli"]

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Kenar komutları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500},
    "origin": {"x": X, "y": Y},
    "layers": [layer("yapi", "Yapı"), layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False)],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}

UID = lambda i: f"$uidOf:{i}"  # noqa: E731
BAD = "Yapı-01"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def bad_uid(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."


def not_found(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


LOCKED = "“Kilitli” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def warning(code, message, path):
    return {"code": code, "message": message, "path": path}


def refused(name, input_, code, message, path, non_finite=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = failed(code, message, path)
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


# ── Buda and Uzat on axis-parallel lines, exactly ─────────────────────────────────────────────────────────────────

def seg(e):
    return (F(e["a"]["x"]), F(e["a"]["y"])), (F(e["b"]["x"]), F(e["b"]["y"]))


def crossing(target, other):
    """Where `other` crosses `target` (both axis-parallel lines), as the distance along `target`; None when not."""
    (ax, ay), (bx, by) = target
    (cx, cy), (dx, dy) = other
    if ay == by and cx == dx:  # horizontal target, vertical boundary
        lo, hi = min(cy, dy), max(cy, dy)
        if lo <= ay <= hi and min(ax, bx) <= cx <= max(ax, bx):
            return abs(cx - ax)
    if ax == bx and cy == dy:  # vertical target, horizontal boundary
        lo, hi = min(cx, dx), max(cx, dx)
        if lo <= ax <= hi and min(ay, by) <= cy <= max(ay, by):
            return abs(cy - ay)
    return None


def at_s(target, s):
    (ax, ay), (bx, by) = target
    length = abs(bx - ax) + abs(by - ay)
    return (ax + (bx - ax) * s / length, ay + (by - ay) * s / length)


def boundaries(kind, target_id, uids=None, rect=None):
    if kind == "objects":
        return [BY_ID[i] for i in uids if i != target_id]
    shown = [e for e in SHOWN if e["id"] != target_id and e["kind"] == "line"]
    if kind == "area":
        (x0, y0), (x1, y1) = rect
        def meets(e):
            (a, b) = seg(e)
            return not (max(a[0], b[0]) < x0 or min(a[0], b[0]) > x1 or max(a[1], b[1]) < y0 or min(a[1], b[1]) > y1)
        shown = [e for e in shown if meets(e)]
    return shown


NO_EDGE = "Tıklanan kısmı kesen bir kenar yok."


def trim_piece(piece, pick, bounds):
    """The pieces left of `piece` (a line) trimmed at `pick`, or the core's words."""
    (a, b) = piece
    length = abs(b[0] - a[0]) + abs(b[1] - a[1])
    cuts = sorted(c for c in (crossing(piece, seg(e)) for e in bounds) if c is not None)
    sp = abs(F(pick[0]) - a[0]) + abs(F(pick[1]) - a[1])
    lo = max([F(0)] + [c for c in cuts if c < sp])
    hi = min([length] + [c for c in cuts if c > sp])
    if lo == 0 and hi == length:
        return NO_EDGE
    pieces = []
    if lo > 0:
        pieces.append((a, at_s(piece, lo)))
    if hi < length:
        pieces.append((at_s(piece, hi), b))
    return pieces


def on(piece, p):
    (a, b) = piece
    p = (F(p[0]), F(p[1]))
    if a[1] == b[1]:
        return p[1] == a[1] and min(a[0], b[0]) <= p[0] <= max(a[0], b[0])
    return p[0] == a[0] and min(a[1], b[1]) <= p[1] <= max(a[1], b[1])


def trimmed(target_id, picks, bounds):
    """The pieces of a line after its picks in order, and each pick's failure."""
    pieces = [seg(BY_ID[target_id])]
    fails = []
    cut = False
    for n, pick in enumerate(picks):
        k = 0 if not cut else next((i for i, p in enumerate(pieces) if on(p, pick)), None)
        if k is None:
            continue
        r = trim_piece(pieces[k], pick, bounds)
        if isinstance(r, str):
            if cut:  # a piece an earlier cut left: its ends are cuts, it goes whole
                del pieces[k]
                continue
            fails.append((n, r))
            continue
        pieces[k:k + 1] = r
        cut = True
    return pieces, fails


NO_REACH = "Bu doğrultuda ulaşılacak bir sınır yok."


def extended(target_id, pick, bounds):
    """The line grown at its end nearer `pick` to the nearest boundary, or the core's words."""
    (a, b) = seg(BY_ID[target_id])
    da = abs(F(pick[0]) - a[0]) + abs(F(pick[1]) - a[1])
    db = abs(F(pick[0]) - b[0]) + abs(F(pick[1]) - b[1])
    at_end = db <= da
    tip, prev = (b, a) if at_end else (a, b)
    d = ((tip[0] - prev[0]) / (abs(tip[0] - prev[0]) + abs(tip[1] - prev[1])), (tip[1] - prev[1]) / (abs(tip[0] - prev[0]) + abs(tip[1] - prev[1])))
    best = None
    for e in bounds:
        (c, f) = seg(e)
        # Axis-parallel only: the ray along x meets a vertical boundary, along y a horizontal one.
        if d[1] == 0 and c[0] == f[0] and min(c[1], f[1]) <= tip[1] <= max(c[1], f[1]):
            t = (c[0] - tip[0]) / d[0]
        elif d[0] == 0 and c[1] == f[1] and min(c[0], f[0]) <= tip[0] <= max(c[0], f[0]):
            t = (c[1] - tip[1]) / d[1]
        else:
            continue
        if t > 0 and (best is None or t < best):
            best = t
    if best is None:
        return NO_REACH
    new = (tip[0] + d[0] * best, tip[1] + d[1] * best)
    return (a, new) if at_end else (new, b)


def line_entity(base, piece, keep, eid):
    e = {"kind": "line", "id": eid, "layerId": base["layerId"], "attrs": dict(base["attrs"]) if keep else {}}
    for k in ("color", "lineWeight"):
        if k in base:
            e[k] = base[k]
    e["a"], e["b"] = P(*piece[0]), P(*piece[1])
    return e


def shared_refusals(kind, one):
    """Buda's and Uzat's refusals before their own, in the contract's order."""
    nothing = "Budanacak" if kind == "trim" else "Uzatılacak"
    return [
        refused("Seçim yok", {"picks": []}, "no_entities", f"{nothing} nesne yok. En az bir nesneyi ve tıklandığı yeri verin.", "picks"),
        refused("Sınır listesi boş", {**one, "boundaries": {"kind": "objects", "uids": []}}, "no_entities",
                "Sınır nesnesi yok. Sınırları verin ya da bütün görünen nesneler için “all”u kullanın.", "boundaries.uids"),
        refused("Seçimin kimliği geçersiz", {"picks": [{"uid": BAD, "at": P(X, Y)}]}, "invalid_uid", bad_uid(BAD), "picks[0].uid"),
        refused("Sınırın kimliği geçersiz: seçimlerden sonra", {**one, "boundaries": {"kind": "objects", "uids": [BAD]}}, "invalid_uid", bad_uid(BAD), "boundaries.uids[0]"),
        refused("Tıklanan nokta sonlu değil", one, "not_finite",
                "Tıklanan noktanın doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", "picks[0].at.x",
                non_finite={"picks[0].at.x": "NaN"}),
        refused("Dikdörtgenin köşesi sonlu değil", {**one, "boundaries": {"kind": "area", "min": P(X, Y), "max": P(X + 50, Y + 50)}}, "not_finite",
                "Dikdörtgenin köşesinin kuzey (X) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", "boundaries.max.y",
                non_finite={"boundaries.max.y": "Infinity"}),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Nesne çizimde yok", {"picks": [{"uid": MISSING, "at": P(X, Y)}]}, "entity_not_found", not_found(MISSING), "picks[0].uid"),
        refused("Sınır çizimde yok", {**one, "boundaries": {"kind": "objects", "uids": [MISSING]}}, "entity_not_found", not_found(MISSING), "boundaries.uids[0]"),
        refused("Kilitli katmandaki nesne", {"picks": [{"uid": UID(17), "at": P(X + 15, Y + 30)}]}, "layer_locked", LOCKED, "picks[0]"),
    ]


def trim_cases():
    out = []
    H = BY_ID[10]

    def trim_case(name, picks, kind="all", uids=None, rect=None, extra_steps=()):
        bounds = boundaries(kind, 10, uids, rect)
        own = [n for n, (i, _) in enumerate(picks) if i == 10]
        pieces, fails = trimmed(10, [picks[n][1] for n in own], bounds)
        fails = [(own[n], why) for n, why in fails]
        for n, (i, p) in enumerate(picks):
            if i != 10:  # another object's pick: these cases' cut nothing
                why = trim_piece(seg(BY_ID[i]), p, boundaries(kind, i, uids, rect))
                assert isinstance(why, str), (i, p)
                fails.append((n, why))
        fails.sort()
        keep = len(pieces) == 1
        first = line_entity(H, pieces[0], keep, 10)
        created = [line_entity(H, p, keep, NEXT + i) for i, p in enumerate(pieces[1:])]
        inp = {"picks": [{"uid": UID(i), "at": P(*p)} for i, p in picks]}
        if kind == "objects":
            inp["boundaries"] = {"kind": "objects", "uids": [UID(u) for u in uids]}
        elif kind == "area":
            inp["boundaries"] = {"kind": "area", "min": P(*rect[0]), "max": P(*rect[1])}
        warnings = [warning("trim_failed", why, f"picks[{n}]") for n, why in fails]
        ents = {"10": first, **{str(e["id"]): e for e in created}}
        out.append({"name": name, "steps": [
            {"op": "execute", "input": inp,
             "result": completed({"changed": [UID(10)], "created": [UID(e["id"]) for e in created], "removed": [], "revision": "$current"}, warnings),
             "expect": {"ids": IDS + [e["id"] for e in created], "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}},
            *extra_steps,
        ]})

    trim_case("Bütün görünen sınırlarla: iki kesen arası gider, iki parça kalır; ilk parça yerinde, öbürü yeni nesne, öznitelikler kalmaz; adım “Buda”",
              [(10, (X + 18, Y + 10))],
              extra_steps=[{"op": "undo", "returns": "Buda", "expect": {"ids": IDS, "entities": {"10": H}, "canUndo": False, "canRedo": True}}])
    trim_case("Ucun yanında: uçla ilk kesen arası gider, tek parça kalır ve öznitelikleri korunur", [(10, (X + 12, Y + 10))])
    trim_case("Gizli katmanın nesnesi sınır değildir; verilirse sınırdır", [(10, (X + 19, Y + 10))], kind="objects", uids=[16])
    trim_case("Yalnız verilen sınırlar: soldaki kesen yokken baştan kesene kadar gider", [(10, (X + 18, Y + 10))], kind="objects", uids=[12])
    trim_case("Dikdörtgene değen görünen nesneler sınırdır (aracın görünümü)", [(10, (X + 18, Y + 10))], kind="area",
              rect=((X + 12, Y + 4), (X + 16, Y + 16)))
    trim_case("Aynı nesnenin ikinci seçimi kalan parçayı budar (Çit)", [(10, (X + 18, Y + 10)), (10, (X + 24, Y + 10))])
    trim_case("Yapamayan seçim trim_failed uyarısıyla atlanır, öbürü yazılır", [(14, (X + 32, Y + 20)), (10, (X + 18, Y + 10))])
    trim_case("Kalan parçayı kesen sınır yoksa parça bütünüyle gider: ucu önceki kesimdir (AutoCAD'in Çit'i)",
              [(10, (X + 18, Y + 10)), (10, (X + 12, Y + 10))])
    # The plan: what execute would write, nothing written.
    bounds = boundaries("all", 10)
    pieces, _ = trimmed(10, [(X + 18, Y + 10)], bounds)
    out.append({"name": "Plan: parçalar yazılacakları gibi, yeni olan id 0; hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": {"picks": [{"uid": UID(10), "at": P(X + 18, Y + 10)}]},
         "result": completed({"changed": [line_entity(H, pieces[0], False, 10)], "created": [line_entity(H, pieces[1], False, 0)], "removed": [], "revision": "$current"}),
         "expect": UNCHANGED}]})
    one = {"picks": [{"uid": UID(10), "at": P(X + 18, Y + 10)}]}
    out += shared_refusals("trim", one)
    out += [
        refused("Adalı alan budanmaz", {"picks": [{"uid": UID(18), "at": P(X + 65, Y)}]}, "not_trimmable",
                "Adalı alanda budama yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın.", "picks[0]"),
        refused("Hiçbir seçim bir şey yapamıyor: ilkinin nedeniyle", {"picks": [{"uid": UID(14), "at": P(X + 32, Y + 20)}]}, "trim_failed", NO_EDGE, "picks"),
    ]
    first_cut = trimmed(10, [(X + 12, Y + 10)], boundaries("all", 10))[0][0]
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": {"picks": [{"uid": UID(10), "at": P(X + 12, Y + 10)}]},
         "result": completed({"changed": [UID(10)], "created": [], "removed": [], "revision": "$current"})},
        {"op": "execute", "input": {**one, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"entities": {"10": line_entity(H, first_cut, True, 10)}}},
    ]})
    return out


def extend_cases():
    out = []
    T = BY_ID[14]
    bounds = boundaries("all", 14)
    grown = extended(14, (X + 33, Y + 20), bounds)
    after = {**T, "a": P(*grown[0]), "b": P(*grown[1])}
    out.append({"name": "Yakın uç ilk sınıra uzar; nesne verisini korur; adım “Uzat”", "steps": [
        {"op": "execute", "input": {"picks": [{"uid": UID(14), "at": P(X + 33, Y + 20)}]},
         "result": completed({"changed": [UID(14)], "created": [], "removed": [], "revision": "$current"}),
         "expect": {"ids": IDS, "entities": {"14": after}, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}},
        {"op": "undo", "returns": "Uzat", "expect": {"entities": {"14": T}, "canUndo": False, "canRedo": True}},
    ]})
    failure = extended(14, (X + 31, Y + 20), bounds)
    assert failure == NO_REACH
    out.append({"name": "İki uç: ulaşamayan uç extend_failed uyarısıyla atlanır", "steps": [
        {"op": "execute", "input": {"picks": [{"uid": UID(14), "at": P(X + 33, Y + 20)}, {"uid": UID(14), "at": P(X + 31, Y + 20)}]},
         "result": completed({"changed": [UID(14)], "created": [], "removed": [], "revision": "$current"}, [warning("extend_failed", NO_REACH, "picks[1]")]),
         "expect": {"entities": {"14": after}}},
    ]})
    out.append({"name": "Aynı uç bir kez uzar: uzamış nesnede aynı uca yakın ikinci seçim yok sayılır", "steps": [
        {"op": "execute", "input": {"picks": [{"uid": UID(14), "at": P(X + 33, Y + 20)}, {"uid": UID(14), "at": P(X + 39, Y + 20)}]},
         "result": completed({"changed": [UID(14)], "created": [], "removed": [], "revision": "$current"}),
         "expect": {"entities": {"14": after}}},
    ]})
    for name, b in [("Dikdörtgene değen sınırla", {"kind": "area", "min": P(X + 38, Y + 15), "max": P(X + 42, Y + 25)}),
                    ("Verilen sınırla", {"kind": "objects", "uids": [UID(15)]})]:
        out.append({"name": name, "steps": [{"op": "execute", "input": {"picks": [{"uid": UID(14), "at": P(X + 33, Y + 20)}], "boundaries": b},
                    "result": completed({"changed": [UID(14)], "created": [], "removed": [], "revision": "$current"}),
                    "expect": {"entities": {"14": after}}}]})
    out.append({"name": "Plan: uzamış nesne; hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": {"picks": [{"uid": UID(14), "at": P(X + 33, Y + 20)}]},
         "result": completed({"changed": [after], "created": [], "removed": [], "revision": "$current"}), "expect": UNCHANGED}]})
    one = {"picks": [{"uid": UID(14), "at": P(X + 33, Y + 20)}]}
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": one, "result": completed({"changed": [UID(14)], "created": [], "removed": [], "revision": "$current"})},
        {"op": "execute", "input": {**one, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"entities": {"14": after}}},
    ]})
    out += shared_refusals("extend", one)
    out += [
        refused("Verilen sınıra uç ulaşmıyor", {"picks": [{"uid": UID(14), "at": P(X + 33, Y + 20)}], "boundaries": {"kind": "objects", "uids": [UID(11)]}},
                "extend_failed", NO_REACH, "picks"),
        refused("Sol uç verilen sınıra ulaşmıyor: sınır öbür yanda", {"picks": [{"uid": UID(14), "at": P(X + 31, Y + 20)}], "boundaries": {"kind": "objects", "uids": [UID(15)]}},
                "extend_failed", NO_REACH, "picks"),
        refused("Kapalı alan uzatılmaz", {"picks": [{"uid": UID(23), "at": P(X + 85, Y + 50)}]}, "extend_failed",
                "Yalnız açık çizgiler, çoklu çizgiler ve yaylar uzatılabilir.", "picks"),
    ]
    return out


# ── Köşe yuvarla and Pah ───────────────────────────────────────────────────────────────────────────────────────────

getcontext().prec = 60


def _series(x, first, step):
    total, term, n = Decimal(0), first, step
    while abs(term) > Decimal(10) ** -58:
        total += term
        term *= -x * x / ((n + 1) * (n + 2))
        n += 2
    return total


def sin_(x):
    """sin of the double x, correctly rounded (60 digits, then the nearest double): no platform libm."""
    d = Decimal(x)
    return float(_series(d, d, 1))


def tan_(x):
    """tan of the double x, correctly rounded."""
    d = Decimal(x)
    return float(_series(d, d, 1) / _series(d, Decimal(1), 0))


def unit(v):
    h = math.hypot(v[0], v[1])
    return (v[0] / h, v[1] / h)


def norm_angle(a):
    r = math.fmod(a, 2 * math.pi)
    return r + 2 * math.pi if r < 0 else r


def kept(a, b, pick, x):
    d = unit((b[0] - a[0], b[1] - a[1]))
    s = (pick[0] - x[0]) * d[0] + (pick[1] - x[1]) * d[1]
    u = d if s >= 0 else (-d[0], -d[1])
    far = a if (a[0] - x[0]) * u[0] + (a[1] - x[1]) * u[1] >= (b[0] - x[0]) * u[0] + (b[1] - x[1]) * u[1] else b
    return u, far


def lines_corner(first, p1, second, p2):
    a1, b1 = [(float(v[0]), float(v[1])) for v in seg(BY_ID[first])]
    a2, b2 = [(float(v[0]), float(v[1])) for v in seg(BY_ID[second])]
    # Axis-parallel: the vertical one's x and the horizontal one's y.
    x = (a2[0], a1[1]) if a1[1] == b1[1] else (a1[0], a2[1])
    u1, far1 = kept(a1, b1, p1, x)
    u2, far2 = kept(a2, b2, p2, x)
    return x, u1, far1, u2, far2


def fillet_lines(first, p1, second, p2, r):
    x, u1, far1, u2, far2 = lines_corner(first, p1, second, p2)
    phi = math.acos(max(-1.0, min(1.0, u1[0] * u2[0] + u1[1] * u2[1])))
    half = phi / 2
    dt = r / tan_(half)
    t1 = (x[0] + u1[0] * dt, x[1] + u1[1] * dt)
    t2 = (x[0] + u2[0] * dt, x[1] + u2[1] * dt)
    bis = unit((u1[0] + u2[0], u1[1] + u2[1]))
    dc = r / sin_(half)
    c = (x[0] + bis[0] * dc, x[1] + bis[1] * dc)
    a0 = norm_angle(math.atan2(t1[1] - c[1], t1[0] - c[0]))
    a1 = norm_angle(math.atan2(t2[1] - c[1], t2[0] - c[0]))
    if norm_angle(a1 - a0) > math.pi:
        a0, a1 = a1, a0
    return (far1, t1), (far2, t2), {"c": c, "r": r, "a0": a0, "a1": a1}


def along(first, kept):
    """A side keeps its direction (docs/adr/0250 §3): the kept end first is turned round when it is the line's end."""
    a, b = [(float(v[0]), float(v[1])) for v in seg(BY_ID[first])]
    return (kept[1], kept[0]) if kept[0] == b and kept[0] != a else kept


def cut_lines(first, p1, second, p2, d1, d2):
    x, u1, far1, u2, far2 = lines_corner(first, p1, second, p2)
    c1 = (x[0] + u1[0] * d1, x[1] + u1[1] * d1)
    c2 = (x[0] + u2[0] * d2, x[1] + u2[1] * d2)
    return (far1, c1), (far2, c2), (c1, c2)


def corner_of_path(pts, bulges, closed, i, op):
    """The path with corner `i` replaced: `op` is ("r", radius) or ("c", d1, d2)."""
    n = len(pts)
    v, p, nx = pts[i], pts[(i - 1) % n], pts[(i + 1) % n]
    lp, ln = math.hypot(p[0] - v[0], p[1] - v[1]), math.hypot(nx[0] - v[0], nx[1] - v[1])
    u1 = ((p[0] - v[0]) / lp, (p[1] - v[1]) / lp)
    u2 = ((nx[0] - v[0]) / ln, (nx[1] - v[1]) / ln)
    phi = math.acos(max(-1.0, min(1.0, u1[0] * u2[0] + u1[1] * u2[1])))
    if op[0] == "r":
        d1 = d2 = op[1] / tan_(phi / 2)
        turn = math.copysign(1.0, -u1[0] * u2[1] + u1[1] * u2[0])
        b = turn * tan_((math.pi - phi) / 4)
    else:
        d1, d2, b = op[1], op[2], 0.0
    t1 = (v[0] + u1[0] * d1, v[1] + u1[1] * d1)
    t2 = (v[0] + u2[0] * d2, v[1] + u2[1] * d2)
    src = [bulges[k] if bulges else 0.0 for k in range(n)]
    out_p = pts[:i] + [t1, t2] + pts[i + 1:]
    out_b = src[:i] + [b] + src[i:]
    if not closed:
        out_b[-1] = 0.0
    return out_p, (out_b if any(x != 0 for x in out_b) else None)


def path_entity(base, pts, bulges):
    e = dict(base)
    e["pts"] = [P(*p) for p in pts]
    if bulges:
        e["bulges"] = [num(b) for b in bulges]
    else:
        e.pop("bulges", None)
    return e


def fpts(e):
    return [(float(p["x"]), float(p["y"])) for p in e["pts"]]


def arc_entity(base, arc, eid):
    e = {"kind": "arc", "id": eid, "layerId": base["layerId"], "attrs": {}}
    for k in ("color", "lineWeight"):
        if k in base:
            e[k] = base[k]
    e.update({"c": P(*arc["c"]), "r": num(arc["r"]), "a0": num(arc["a0"]), "a1": num(arc["a1"])})
    return e


def lines_case(name, kind, size, trim=True, plan=False):
    A, B = BY_ID[21], BY_ID[22]
    p1, p2 = (X + 55.0, Y + 80.0), (X + 66.0, Y + 90.0)
    corner = {"kind": "lines", "first": {"uid": UID(21), "at": P(*p1)}, "second": {"uid": UID(22), "at": P(*p2)}}
    if kind == "fillet":
        l1, l2, arc = fillet_lines(21, p1, 22, p2, size)
        piece = arc_entity(A, arc, NEXT) if size > 0 else None
        inp = {"corner": corner, "radius": num(size)}
    else:
        l1, l2, cut = cut_lines(21, p1, 22, p2, *size)
        piece = line_entity(A, cut, False, NEXT)
        inp = {"corner": corner, "distance": num(size[0]), "second": num(size[1])}
    if not trim:
        inp["trim"] = False
    changed, ents = [], {}
    l1, l2 = along(21, l1), along(22, l2)
    if trim:
        ents = {"21": {**A, "a": P(*l1[0]), "b": P(*l1[1])}, "22": {**B, "a": P(*l2[0]), "b": P(*l2[1])}}
        changed = [UID(21), UID(22)]
    created = [piece] if piece else []
    if piece:
        ents[str(NEXT)] = piece
    if plan:
        return {"name": name, "steps": [{"op": "plan", "input": inp,
                "result": completed({"changed": [ents["21"], ents["22"]], "created": [{**piece, "id": 0}], "done": 1, "skipped": 0, "revision": "$current"}),
                "expect": UNCHANGED}]}
    label = "Köşe yuvarla" if kind == "fillet" else "Pah"
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"changed": changed, "created": [UID(NEXT)] if created else [], "done": 1, "skipped": 0, "revision": "$current"}),
         "expect": {"ids": IDS + ([NEXT] if created else []), "entities": ents, "revision": "changed", "canUndo": True}},
        {"op": "undo", "returns": label, "expect": {"ids": IDS, "entities": {"21": A, "22": B}}},
    ]}


def vertex_case(name, kind, op):
    Pl = BY_ID[20]
    pts, bulges = corner_of_path(fpts(Pl), None, False, 1, op)
    inp = {"corner": {"kind": "vertex", "uid": UID(20), "index": 1}}
    if kind == "fillet":
        inp["radius"] = num(op[1])
    else:
        inp["distance"], inp["second"] = num(op[1]), num(op[2])
    return {"name": name, "steps": [{"op": "execute", "input": inp,
            "result": completed({"changed": [UID(20)], "created": [], "done": 1, "skipped": 0, "revision": "$current"}),
            "expect": {"ids": IDS, "entities": {"20": path_entity(Pl, pts, bulges)}, "revision": "changed", "canUndo": True}}]}


def all_case(name, kind, op):
    S = BY_ID[23]
    pts, bulges = fpts(S), None
    for i in reversed(range(len(pts))):
        pts, bulges = corner_of_path(pts, bulges, True, i, op)
    inp = {"corner": {"kind": "all", "uids": [UID(23)]}}
    if kind == "fillet":
        inp["radius"] = num(op[1])
    else:
        inp["distance"] = num(op[1])
    return {"name": name, "steps": [{"op": "execute", "input": inp,
            "result": completed({"changed": [UID(23)], "created": [], "done": 4, "skipped": 0, "revision": "$current"}),
            "expect": {"ids": IDS, "entities": {"23": path_entity(S, pts, bulges)}, "revision": "changed", "canUndo": True}}]}


VERTEX_NOT = "Bu köşe işlenemez: köşe yolun dışında, açık yolun ucunda ya da bir yayın yanında; yalnızca düz kenarlar arasındaki köşe işlenir."
LINES_CORNER = {"kind": "lines", "first": {"uid": UID(21), "at": P(X + 55, Y + 80)}, "second": {"uid": UID(22), "at": P(X + 66, Y + 90)}}


def corner_refusals(kind):
    size = {"radius": 1} if kind == "fillet" else {"distance": 1}
    field = "radius" if kind == "fillet" else "distance"
    what = "Yarıçap" if kind == "fillet" else "Pah mesafesi"
    failed_code = f"{kind}_failed"
    out = [
        refused("Tüm köşeler: nesne yok", {"corner": {"kind": "all", "uids": []}, **size}, "no_entities",
                "Köşeleri işlenecek nesne yok. En az bir çoklu çizgi ya da alan verin.", "corner.uids"),
        refused("Kimlik geçersiz", {"corner": {"kind": "vertex", "uid": BAD, "index": 1}, **size}, "invalid_uid", bad_uid(BAD), "corner.uid"),
        refused(f"{what} sonlu değil", {"corner": LINES_CORNER, **size}, "not_finite",
                f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı yazın.", field, non_finite={field: "Infinity"}),
        refused(f"{what} eksi olamaz", {"corner": LINES_CORNER, field: -1}, "invalid_distance",
                f"{what} eksi olamaz; sıfır ya da sıfırdan büyük bir değer yazın.", field),
        refused("Kırpmadan sıfır: eklenecek bir şey yok", {"corner": LINES_CORNER, field: 0, "trim": False}, "invalid_distance",
                "Kırpmadan çalışırken sıfırdan büyük bir boyut verin; yoksa eklenecek bir şey yok.", field),
        refused("Tüm köşelerde sıfır", {"corner": {"kind": "all", "uids": [UID(23)]}, field: 0}, "invalid_distance",
                "Tüm köşelerde her değer sıfırdan büyük olmalı; sıfır hiçbir köşeyi değiştirmez.", field),
        refused("Nesne çizimde yok", {"corner": {"kind": "vertex", "uid": MISSING, "index": 1}, **size}, "entity_not_found", not_found(MISSING), "corner.uid"),
        refused("Kilitli katmandaki çizgi", {"corner": {"kind": "lines", "first": {"uid": UID(17), "at": P(X + 15, Y + 30)},
                                                            "second": {"uid": UID(22), "at": P(X + 66, Y + 90)}}, **size},
                "layer_locked", LOCKED, "corner.first.uid"),
        refused("Açık yolun ucu köşe değildir", {"corner": {"kind": "vertex", "uid": UID(20), "index": 0}, **size}, "not_a_corner", VERTEX_NOT, "corner.index"),
        refused("Aynı çizgi iki kez", {"corner": {"kind": "lines", "first": {"uid": UID(21), "at": P(X + 55, Y + 80)},
                                                   "second": {"uid": UID(21), "at": P(X + 58, Y + 80)}}, **size},
                "not_a_corner", "İkinci çizgi ilkinden farklı olmalı.", "corner"),
        refused("Paralel çizgiler", {"corner": {"kind": "lines", "first": {"uid": UID(21), "at": P(X + 55, Y + 80)},
                                                 "second": {"uid": UID(14), "at": P(X + 33, Y + 20)}}, **size},
                "not_a_corner", "Çizgiler paralel ya da seçilen tarafta çizgi yok.", "corner"),
        refused("Tüm köşeler: tek başına çizgi köşe değildir", {"corner": {"kind": "all", "uids": [UID(21)]}, **size}, "not_a_corner",
                "Tüm köşeler çoklu çizgide, alanda ve uçları birbirine değen çizgilerde işlenir.", "corner.uids[0]"),
    ]
    out.append(refused("Beklenen sürüm sayı değil", {"corner": LINES_CORNER, **size, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append(refused("Hiçbir köşeye sığmıyor", {"corner": {"kind": "all", "uids": [UID(23)]}, field: 6}, failed_code,
                       "Hiçbir köşeye sığmadı; daha küçük bir değer verin ya da kenarları uzatın.", "corner.uids"))
    too_big = "Yarıçap bu kenarlar için çok büyük." if kind == "fillet" else "Pah mesafesi kenar boyundan büyük."
    out.append(refused("Köşeye sığmıyor", {"corner": {"kind": "vertex", "uid": UID(20), "index": 1}, field: 20}, failed_code, too_big, "corner"))
    return out


def mixed_case(kind):
    """Tüm köşeler on a polyline that takes it and a square that does not: the square warned, its corners skipped."""
    Pl = BY_ID[20]
    op = ("r", 6.0) if kind == "fillet" else ("c", 6.0, 6.0)
    pts, bulges = corner_of_path(fpts(Pl), None, False, 1, op)
    field = "radius" if kind == "fillet" else "distance"
    return {"name": "Tüm köşeler: sığan nesne yazılır, hiçbir köşesi sığmayan uyarıyla atlanır, köşeleri sayılır", "steps": [
        {"op": "execute", "input": {"corner": {"kind": "all", "uids": [UID(20), UID(23)]}, field: 6},
         "result": completed({"changed": [UID(20)], "created": [], "done": 1, "skipped": 4, "revision": "$current"},
                             [warning(f"{kind}_failed", "Hiçbir köşeye sığmadı; daha küçük bir değer verin ya da kenarları uzatın.", "corner.uids[1]")]),
         "expect": {"entities": {"20": path_entity(Pl, pts, bulges), "23": BY_ID[23]}}}]}


def corner_conflict(kind):
    field = "radius" if kind == "fillet" else "distance"
    inp = {"corner": {"kind": "vertex", "uid": UID(20), "index": 1}, field: 2}
    return {"name": "Beklenen sürüm geçmişte kaldıysa conflict", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": {"corner": {"kind": "all", "uids": [UID(23)]}, field: 1},
         "result": completed({"changed": [UID(23)], "created": [], "done": 4, "skipped": 0, "revision": "$current"})},
        {"op": "execute", "input": {**inp, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"entities": {"20": BY_ID[20]}}}]}


def fillet_cases():
    return [
        vertex_case("Çoklu çizginin köşesi 2 m yarıçapla: köşenin yerine teğet noktaları, aralarında sola dönen yay; adım “Köşe yuvarla”", "fillet", ("r", 2.0)),
        lines_case("İki çizgi 4 m yarıçapla: kenarlar yayın teğet noktalarına kırpılır, yay ilk çizgiden yeni nesne", "fillet", 4.0),
        lines_case("Sıfır yarıçap iki çizgiyi buluştukları noktada birleştirir; yay yok", "fillet", 0.0),
        lines_case("Plan: kenarlar ve yay yazılacakları gibi; hiçbir şey yazılmaz", "fillet", 4.0, plan=True),
        all_case("Alanın bütün köşeleri 1 m yarıçapla: dört köşe, her biri sola dönen yay", "fillet", ("r", 1.0)),
        mixed_case("fillet"),
        corner_conflict("fillet"),
        *corner_refusals("fillet"),
    ]


def chamfer_cases():
    return [
        vertex_case("Çoklu çizginin köşesine 2 ve 3 m pah: köşenin yerine iki nokta; adım “Pah”", "chamfer", ("c", 2.0, 3.0)),
        lines_case("İki çizgiye 2 ve 3 m pah: kenarlar kırpılır, pah ilk çizgiden yeni çizgi", "chamfer", (2.0, 3.0)),
        lines_case("Kırp kapalı: kenarlar kalır, yalnız pah çizgisi eklenir", "chamfer", (2.0, 3.0), trim=False),
        all_case("Alanın bütün köşelerine 1 m pah; ikinci mesafe verilmezse birincisi", "chamfer", ("c", 1.0, 1.0)),
        lines_case("Plan: kenarlar ve pah yazılacakları gibi; hiçbir şey yazılmaz", "chamfer", (2.0, 3.0), plan=True),
        mixed_case("chamfer"),
        corner_conflict("chamfer"),
        *corner_refusals("chamfer"),
    ]


FILES = [
    ("cad.entities.trim", "Nesneleri buda", trim_cases),
    ("cad.entities.extend", "Nesneleri uzat", extend_cases),
    ("cad.entities.fillet", "Köşe yuvarla", fillet_cases),
    ("cad.entities.chamfer", "Pah kır", chamfer_cases),
]


def build(command, title, cases):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
        "note": "Written by scripts/fixtures/edge_command_cases.py from the operations' definitions (docs/adr/0047, 0250); do not edit by hand.",
        "setup": SETUP, "cases": cases(),
    }


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = False
    for command, title, cases in FILES:
        doc = build(command, title, cases)
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
