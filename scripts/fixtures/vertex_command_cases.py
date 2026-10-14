"""The shared cases of the product command cad.entities.vertex (docs/adr/0279).

    python3 scripts/fixtures/vertex_command_cases.py           # writes the file
    python3 scripts/fixtures/vertex_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.vertex.json. The checks, their order, codes, paths and messages are written
here by hand from the ADR. What the command does is computed independently, from the operations' definitions
(docs/adr/0047, 0160, 0279), without KentOS code:

- Köşeyi sil: the vertex goes; on a ring its two edges become one straight edge, on an open path an end vertex
  goes with its edge. A ring keeps at least three vertices, an open path two.
- Ortasına köşe ekle: a vertex at the edge's middle, or Köşe ekle at a place along a straight edge (a fraction t of
  its chord, inside it). A straight edge's middle is the chord's; an arc edge's is the
  arc's (the chord's middle moved by half the chord times the bulge, to the right of travel for a positive bulge)
  and the arc splits there into two arcs of half the sweep, bulge tan(sweep / 8) each. A line becomes a polyline of
  two edges.
- Düz kenar yap: the edge's bulge 0; a path left with no arc has no bulges. Yaya dönüştür: bulge 0.5 (sagitta a
  quarter of the chord), bowing out of a ring: right of travel (+0.5) on a counter-clockwise ring, left (−0.5) on a
  clockwise one; right of travel on an open path. An edge already as the action would make it: nothing is written.
- An area's holes and a multi-part area's other parts stay.
- Topolojik düzenleme: a shown neighbour with the same straight edge (either way) takes the inserted vertex; one with
  the same edge takes its new bulge (negated when it runs the other way); one with the removed vertex between the
  same two vertices loses it. A neighbour on a locked layer stays (counted), as does one a removal would leave with too
  few vertices (counted invalid).

The geometry is computed in exact fractions over a drawing whose every value is a double exactly; the one
transcendental value (tan(π/8) of a semicircle split in two) is computed to 60 digits by its series and rounded to the
nearest double, not by the platform's libm.
docs/adr/0288 adds, computed the same way:

- Yaya dönüştür through a point m (`through`): the arc a → m → b, its bulge the cotangent of half the angle at m,
  (|u||v| + u·v) / −(u × v) with u = a − m, v = b − m, positive bowing right of travel. Radius r (`radius`): the shorter
  arc on the chord c, bulge c / (2|r| + √(4r² − c²)), its sign r's. On a straight or an arc edge; the bulge as it is:
  nothing written. The points and radii are chosen so that every square root is of a square and every bulge a dyadic,
  as the command's double arithmetic then gives them exactly.
- Kenar genişliği (`width`): a polyline's widths, one pair a vertex (an open path's last [0, 0]), the edge's pair set;
  widths all zero are none.
- The new refusals, before the object is looked up: `invalid_arc_edge` (a point or a radius with another action, both,
  a radius of zero or not finite), `invalid_polyline` (widths with another action, Kenar genişliği without them, a
  width below zero or not finite); after it: `not_a_polyline` (Kenar genişliği on an area or a line),
  `invalid_arc_edge` (a point on the edge's line or at its end, a radius below half the chord).

`--check` rebuilds the file in memory and compares it with the one on disk.
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
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style,
            "children": []}


def num(v):
    if isinstance(v, float):
        return int(v) if v.is_integer() else v
    v = F(v)
    assert v.denominator & (v.denominator - 1) == 0, v  # a dyadic: a double exactly
    f = float(v)
    return int(f) if f.is_integer() else f


def P(x, y):
    return {"x": num(X + x), "y": num(Y + y)}


def path(kind, i, pts, layer_id="yapi", bulges=None, **extra):
    e = {"kind": kind, "id": i, "layerId": layer_id, "attrs": {}, **extra, "pts": [P(*p) for p in pts]}
    if bulges is not None:
        e["bulges"] = [num(b) for b in bulges]
    return e


A = [(0, 0), (10, 0), (10, 10), (0, 10)]
B = [(10, 0), (20, 0), (20, 10), (10, 10)]
OPEN = [(0, 20), (10, 20), (10, 30), (20, 30)]
ARCED = [(0, 40), (10, 40), (20, 40)]
CW = [(30, 0), (30, 10), (40, 0)]
TWO = [(0, 60), (10, 60)]
HOLED = [(50, 0), (60, 0), (60, 10), (50, 10)]
HOLE = [(52, 2), (54, 2), (54, 4), (52, 4)]
LOCKED_SQUARE = [(0, 10), (10, 10), (10, 20), (0, 20)]
PART0 = [(90, 0), (100, 0), (100, 10), (90, 10)]
PART1 = [(110, 0), (120, 0), (120, 10), (110, 10)]
RING = [(70, 0), (80, 0), (80, 10), (70, 10)]
ALONG = [(0, 0), (10, 0), (10, 10), (0, 10)]
QUAD = [(0, 70), (10, 70), (5, 80), (-5, 80)]
WIDE = [(0, 90), (10, 90), (20, 90)]
ONE_WIDE = [(0, 100), (10, 100), (20, 100)]
TRIANGLE = [(0, 70), (10, 70), (5, 80)]

ENTITIES = [
    path("polygon", 1, A, attrs={"Ada": "101"}, color="#E5484D"),
    path("polygon", 2, B),
    path("polyline", 3, OPEN, attrs={"Tür": "Bordür"}),
    path("polyline", 4, ARCED, bulges=[1, 0, 0]),
    {"kind": "line", "id": 5, "layerId": "yapi", "attrs": {"Tür": "Duvar"}, "a": P(0, 50), "b": P(10, 50)},
    path("polygon", 6, CW),
    path("polyline", 7, TWO),
    {"kind": "circle", "id": 8, "layerId": "yapi", "attrs": {}, "c": P(40, 40), "r": 5},
    {**path("polygon", 9, HOLED), "holes": [{"pts": [P(*p) for p in HOLE]}]},
    {"kind": "line", "id": 10, "layerId": "kilitli", "attrs": {}, "a": P(30, 50), "b": P(40, 50)},
    path("polygon", 11, LOCKED_SQUARE, layer_id="kilitli"),
    {**path("polygon", 12, PART0), "parts": [{"pts": [P(*p) for p in PART1]}]},
    {**path("polyline", 13, RING), "closed": True},
    path("polyline", 14, ALONG),
    path("polygon", 15, QUAD),
    path("polygon", 16, TRIANGLE),
    {**path("polyline", 17, WIDE), "widths": [[1, 1], [1, 1], [0, 0]]},
    {**path("polyline", 18, ONE_WIDE), "widths": [[F(1, 2), F(1, 2)], [0, 0], [0, 0]]},
]
for _e in ENTITIES:
    if "widths" in _e:
        _e["widths"] = [[num(a), num(b)] for a, b in _e["widths"]]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
LOCKED_LAYERS = {"kilitli"}

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Köşe ve kenar",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500},
    "origin": {"x": X, "y": Y},
    "layers": [layer("yapi", "Yapı"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}

UID = lambda i: f"$uidOf:{i}"  # noqa: E731
BAD = "Yapı-01"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
LOCKED = "“Kilitli” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
NOT_A_PATH = "Köşe ve kenar işlemleri çoklu çizgide ve kapalı alanda yapılır (çizgiye yalnız köşe eklenir)."
WRONG_REMOVE = "Köşeyi sil'in yeri bir köşedir (vertex)."
WRONG_EDGE = ("Köşe eklemenin yeri bir kenarın ortası (mid) ya da kenar üzerinde bir yerdir (along); kenarı düzeltme ve yaya "
              "dönüştürmenin yeri kenarın ortasıdır.")
HOLE_EDIT = "Deliğin köşe ve kenarları bu işlemle değişmez; deliği tutamacıyla sürükleyin ya da Patlat'la düzenleyin."
NO_PLACE = "Nesnede verilen yer yok: parça, köşe ya da kenar sırası nesnenin dışında."
NEIGHBOUR_LOCKED = "Kilitli katmandaki {} komşu nesne değişmedi; ortak sınır ayrıldı."
NEIGHBOUR_INVALID = "{} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe)."
LABELS = {"remove": "Köşe sil", "insert": "Köşe ekle", "straighten": "Düz kenar yap", "arc": "Yaya dönüştür",
          "width": "Kenar genişliği"}
ARC_ONLY_POINT = "Yayın geçeceği nokta yalnız Yaya dönüştür'de (arc) verilir."
ARC_ONLY_RADIUS = "Yarıçap yalnız Yaya dönüştür'de (arc) verilir."
ARC_BOTH = "Yay ya bir noktadan ya da bir yarıçapla verilir; ikisi birlikte verilmez."
ARC_RADIUS = "Yarıçap sıfırdan farklı bir sayı olmalı (eksi: kenarın soluna)."
ARC_POINT = "Yayın geçeceği noktanın koordinatları sayı olmalı."
ARC_ON_LINE = "Yayın geçeceği nokta kenarın doğrusunda ya da ucunda; doğrunun dışında bir nokta verin."
ARC_SHORT = "Yarıçap kirişin yarısından küçük; en az kirişin yarısı kadar bir yarıçap verin."
WIDTHS_ONLY = "Genişlikler yalnız Kenar genişliği'nde (width) verilir."
WIDTHS_NEEDED = "Kenar genişliği başlangıç ve bitiş genişlikleriyle (widths) verilir."
WIDTHS_BAD = "Genişlikler sıfır ya da büyük sayılar olmalı."
NOT_A_POLYLINE = "Kenar genişliği yalnız çoklu çizgide olur; alanın ve çizginin genişliği yoktur."


def bad_uid(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."


def not_found(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path_):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path_}}


def warning(code, message):
    return {"code": code, "message": message}


def vertex(index, part=0, ring=0):
    return {"kind": "vertex", "part": part, "ring": ring, "index": index}


def mid(index, part=0, ring=0):
    return {"kind": "mid", "part": part, "ring": ring, "index": index}


def along(index, t, part=0, ring=0):
    return {"kind": "along", "part": part, "ring": ring, "index": index, "t": num(t)}


# ── Geometry ──────────────────────────────────────────────────────────────────────────────────────────────────────

getcontext().prec = 60


def _series(x, first, step):
    total, term, n = Decimal(0), first, step
    while abs(term) > Decimal(10) ** -58:
        total += term
        term *= -x * x / ((n + 1) * (n + 2))
        n += 2
    return total


def tan_(x):
    """tan of the double x, correctly rounded (60 digits, then the nearest double): no platform libm."""
    d = Decimal(x)
    return float(_series(d, d, 1) / _series(d, Decimal(1), 0))


def area2(pts):
    return sum(F(a[0]) * b[1] - F(b[0]) * a[1] for a, b in zip(pts, pts[1:] + pts[:1]))


def edge_mid(a, b, bulge):
    """The edge's middle: the chord's, moved by half the chord times the bulge to the right of travel."""
    bulge = F(bulge)
    return (F(a[0] + b[0]) / 2 + (b[1] - a[1]) * bulge / 2, F(a[1] + b[1]) / 2 - (b[0] - a[0]) * bulge / 2)


class Path:
    """A path in local coordinates: its vertices, one bulge per edge (an open path's last is 0), whether it closes."""

    def __init__(self, pts, bulges, closed):
        self.pts, self.closed = [tuple(F(v) for v in p) for p in pts], closed
        self.bulges = list(bulges) if bulges else [0] * len(pts)

    def entity(self, like, per_edge=False):
        """As written: an open path's bulges one per vertex (its last 0), as the vertex operations keep them, or one per
        edge, as Topolojik düzenleme writes a neighbour's."""
        out = {k: v for k, v in like.items() if k not in ("pts", "bulges", "a", "b", "closed", "holes", "parts")}
        if like["kind"] == "line":
            out["kind"] = "polyline"
        out["pts"] = [P(*p) for p in self.pts]
        if any(b != 0 for b in self.bulges):
            bulges = self.bulges[:-1] if per_edge and not self.closed else self.bulges
            out["bulges"] = [num(b) for b in bulges]
        for k in ("holes", "parts"):
            if k in like:
                out[k] = like[k]
        if like.get("closed"):
            out["closed"] = True
        return out


def path_of(e):
    if e["kind"] == "line":
        return Path([(F(e["a"]["x"]) - X, F(e["a"]["y"]) - Y), (F(e["b"]["x"]) - X, F(e["b"]["y"]) - Y)], None, False)
    pts = [(F(p["x"]) - X, F(p["y"]) - Y) for p in e["pts"]]
    return Path(pts, e.get("bulges"), e["kind"] == "polygon" or e.get("closed", False))


def removed(p, k):
    n = len(p.pts)
    b = list(p.bulges)
    if not p.closed and k == 0:
        b.pop(0)
    elif not p.closed and k == n - 1:
        b.pop(n - 1)
        b[n - 2] = 0
    else:
        b[(k - 1) % n] = 0
        b.pop(k)
    return Path([q for i, q in enumerate(p.pts) if i != k], b, p.closed)


def inserted(p, k, at, halves):
    b = list(p.bulges)
    b[k:k + 1] = halves
    return Path(p.pts[:k + 1] + [at] + p.pts[k + 1:], b, p.closed)


def bent(p, k, bulge):
    b = list(p.bulges)
    b[k] = bulge
    return Path(p.pts, b, p.closed)


# ── Topolojik düzenleme ───────────────────────────────────────────────────────────────────────────────────────────

def follow(edited_id, change):
    """The shown neighbours put right by one change, in the drawing's order; how many are locked, how many would be
    left invalid (a ring of fewer than three vertices, an open path of fewer than two)."""
    out, locked, invalid = [], 0, 0
    for e in ENTITIES:
        if e["id"] == edited_id or e["kind"] not in ("line", "polyline", "polygon"):
            continue
        p = path_of(e)
        n = len(p.pts)
        count = n if p.closed else n - 1
        new = None
        if change[0] == "insert":
            _, a, b, at = change
            for k in reversed(range(count)):
                u, w = p.pts[k], p.pts[(k + 1) % n]
                if {u, w} == {a, b} and p.bulges[k] == 0:
                    new = inserted(new or p, k, at, [0, 0])
        elif change[0] == "bulge":
            _, a, b, was, to = change
            for k in range(count):
                u, w = p.pts[k], p.pts[(k + 1) % n]
                if (u, w) == (a, b) and p.bulges[k] == was:
                    new = bent(new or p, k, to)
                elif (u, w) == (b, a) and p.bulges[k] == -was:
                    new = bent(new or p, k, -to if to else 0)
        else:
            _, at, before, after = change
            for k in range(n):
                if p.pts[k] != at or (not p.closed and k in (0, n - 1)):
                    continue
                around = (p.pts[(k - 1) % n], p.pts[(k + 1) % n])
                if around in ((before, after), (after, before)):
                    new = "invalid" if n - 1 < (3 if p.closed else 2) else removed(p, k)
                    break
        if new is None:
            continue
        if e["layerId"] in LOCKED_LAYERS:
            locked += 1
        elif new == "invalid":
            invalid += 1
        else:
            out.append(new.entity(e, per_edge=True))
    return out, locked, invalid


# ── The command ───────────────────────────────────────────────────────────────────────────────────────────────────

def edited(i, action, index, part=0, topology=False, t=None):
    """The object after the action and the neighbours following it; None when nothing changes. `t`: Köşe ekle at
    that fraction of a straight edge's chord, not at its middle."""
    e = BY_ID[i]
    whole = path_of(e)
    p = whole if part == 0 else Path([(F(q["x"]) - X, F(q["y"]) - Y) for q in e["parts"][part - 1]["pts"]], None, True)
    n = len(p.pts)
    if action == "remove":
        new = removed(p, index)
        change = ("remove", p.pts[index], p.pts[(index - 1) % n], p.pts[(index + 1) % n])
    elif action == "insert":
        a, b, bulge = p.pts[index], p.pts[(index + 1) % n], p.bulges[index]
        if t is None:
            at = edge_mid(a, b, bulge)
        else:
            assert bulge == 0
            at = (a[0] + (b[0] - a[0]) * F(t), a[1] + (b[1] - a[1]) * F(t))
        # A semicircle (bulge 1, sweep π) splits into two quarter circles, bulge tan(π/8) each.
        assert bulge in (0, 1)
        half = tan_(math.pi / 2 / 4) if bulge == 1 else 0
        new = inserted(p, index, at, [half, half])
        change = ("insert", a, b, at)
    else:
        was = p.bulges[index]
        if action == "straighten":
            to = 0
        else:
            to = F(-1, 2) if p.closed and area2(p.pts) <= 0 else F(1, 2)
        if (action == "straighten") == (was == 0):
            return None
        new = bent(p, index, to)
        change = ("bulge", p.pts[index], p.pts[(index + 1) % n], was, to)
    if part:
        out = {**e, "parts": [{"pts": [P(*q) for q in new.pts]} if k == part - 1 else q for k, q in enumerate(e["parts"])]}
    else:
        out = new.entity(e)
    follows, locked, invalid = follow(i, change) if topology else ([], 0, 0)
    return [out] + follows, locked, invalid


# ── docs/adr/0288 ─────────────────────────────────────────────────────────────────────────────────────────────────

def isqrt_exact(q):
    """The square root of a non-negative fraction that is a square, exactly."""
    q = F(q)
    n, d = math.isqrt(q.numerator), math.isqrt(q.denominator)
    assert F(n * n, d * d) == q, f"{q} is no square"
    return F(n, d)


def bulge_via(a, m, b):
    """The arc a → m → b: cot of half the angle at m, positive bowing right of travel; None on one line."""
    u = (F(a[0]) - m[0], F(a[1]) - m[1])
    v = (F(b[0]) - m[0], F(b[1]) - m[1])
    cross = u[0] * v[1] - u[1] * v[0]
    if cross == 0:
        return None
    norms = isqrt_exact((u[0] ** 2 + u[1] ** 2) * (v[0] ** 2 + v[1] ** 2))
    return (norms + u[0] * v[0] + u[1] * v[1]) / -cross


def bulge_of_radius(a, b, r):
    """The shorter arc of radius |r| on the chord: c / (2|r| + √(4r² − c²)), its sign r's; None below half the chord."""
    c = isqrt_exact((F(b[0]) - a[0]) ** 2 + (F(b[1]) - a[1]) ** 2)
    rest = 4 * F(r) ** 2 - c * c
    if rest < 0:
        return None
    k = c / (2 * abs(F(r)) + isqrt_exact(rest))
    return -k if r < 0 else k


def bent_case(name, i, index, through=None, radius=None, topology=False, undo=False):
    """Yaya dönüştür through a point or with a radius: the edge's bulge as the arc's."""
    e = BY_ID[i]
    p = path_of(e)
    n = len(p.pts)
    a, b, was = p.pts[index], p.pts[(index + 1) % n], p.bulges[index]
    to = bulge_via(a, through, b) if through is not None else bulge_of_radius(a, b, radius)
    assert to is not None and to.denominator & (to.denominator - 1) == 0, to
    inp = {"uid": UID(i), "action": "arc", "place": mid(index)}
    if through is not None:
        inp["through"] = P(*through)
    if radius is not None:
        inp["radius"] = num(radius)
    if topology:
        inp["topology"] = {}
    if to == was:
        return {"name": name, "steps": [{"op": "execute", "input": inp,
                                          "result": completed({"changed": [], "ids": [], "revision": "$current"}),
                                          "expect": UNCHANGED}]}
    new = bent(p, index, to)
    changed = [new.entity(e)]
    if topology:
        follows, _, _ = follow(i, ("bulge", a, b, was, to))
        changed += follows
    ids = [c["id"] for c in changed]
    steps = [{"op": "execute", "input": inp,
              "result": completed({"changed": [UID(k) for k in ids], "ids": ids, "revision": "$current"}),
              "expect": {"ids": IDS, "entities": {str(c["id"]): c for c in changed}, "revision": "changed",
                         "canUndo": True, "canRedo": False, "dirty": True}}]
    if undo:
        steps.append({"op": "undo", "returns": LABELS["arc"],
                      "expect": {"ids": IDS, "entities": {str(k): BY_ID[k] for k in ids}, "canUndo": False, "canRedo": True}})
    return {"name": name, "steps": steps}


def width_case(name, i, index, pair, undo=False):
    """Kenar genişliği: the polyline's widths, one pair a vertex, the edge's set; all zero: none."""
    e = BY_ID[i]
    n = len(e["pts"])
    widths = [list(w) for w in e.get("widths", [[0, 0]] * n)]
    pair = [num(F(v)) for v in pair]
    inp = {"uid": UID(i), "action": "width", "place": mid(index), "widths": pair}
    if widths[index] == pair:
        return {"name": name, "steps": [{"op": "execute", "input": inp,
                                          "result": completed({"changed": [], "ids": [], "revision": "$current"}),
                                          "expect": UNCHANGED}]}
    widths[index] = pair
    out = {k: v for k, v in e.items() if k != "widths"}
    if any(a != 0 or b != 0 for a, b in widths):
        out["widths"] = widths
    steps = [{"op": "execute", "input": inp,
              "result": completed({"changed": [UID(i)], "ids": [i], "revision": "$current"}),
              "expect": {"ids": IDS, "entities": {str(i): out}, "revision": "changed",
                         "canUndo": True, "canRedo": False, "dirty": True}}]
    if undo:
        steps.append({"op": "undo", "returns": LABELS["width"],
                      "expect": {"ids": IDS, "entities": {str(i): e}, "canUndo": False, "canRedo": True}})
    return {"name": name, "steps": steps}


def adr_0288_cases():
    """Yaya dönüştür through a point and with a radius, Kenar genişliği, and their refusals (docs/adr/0288)."""
    arc = {"uid": UID(1), "action": "arc", "place": mid(0)}
    width = {"uid": UID(3), "action": "width", "place": mid(1), "widths": [num(F(1, 2)), 0]}
    out = [
        bent_case("Yaya dönüştür noktadan: alanın kenarı kirişin altındaki noktadan geçen yarım çember (+1); adım “Yaya dönüştür”",
                  1, 0, through=(5, -5), undo=True),
        bent_case("Yaya dönüştür noktadan: sehim kirişin dörtte biri (+0,5)", 1, 0, through=(5, F(-5, 2))),
        bent_case("Yaya dönüştür noktadan: büyük yay (+2)", 1, 0, through=(5, -10)),
        bent_case("Yaya dönüştür noktadan: alanın içine doğru (−0,5)", 1, 0, through=(5, F(5, 2))),
        bent_case("Yaya dönüştür noktadan: kirişin ortasında olmayan nokta, Thales'le yarım çember (+1)", 3, 1, through=(13, 21)),
        bent_case("Yaya dönüştür noktadan: yay kenar yeniden biçimlenir (1 → 0,5)", 4, 0, through=(5, F(75, 2))),
        bent_case("Yaya dönüştür noktadan: yay zaten o noktadan geçiyorsa hiçbir şey yazılmaz", 4, 0, through=(5, 35)),
        bent_case("Topoloji: noktadan yayın büküm komşuya da, ters yönde eksi işaretle", 1, 1, through=(F(25, 2), 5), topology=True),
        bent_case("Yaya dönüştür yarıçapla: kirişin yarısı yarıçap, yarım çember (+1)", 1, 0, radius=5),
        bent_case("Yaya dönüştür yarıçapla: 6,25 m, kısa yay (+0,5)", 1, 2, radius=F(25, 4), undo=True),
        bent_case("Yaya dönüştür yarıçapla: eksi yarıçap kenarın soluna (−0,25)", 3, 0, radius=F(-85, 8)),
        bent_case("Yaya dönüştür yarıçapla: yay kenar yeniden biçimlenir (1 → 0,5)", 4, 0, radius=F(25, 4)),
        bent_case("Yaya dönüştür yarıçapla: kapalı çoklu çizginin kenarı", 13, 0, radius=5),
        width_case("Kenar genişliği: açık yolun ikinci kenarı 0,5'ten 0,25'e; adım “Kenar genişliği”", 3, 1, (F(1, 2), F(1, 4)), undo=True),
        width_case("Kenar genişliği: kapalı çoklu çizginin kapanan kenarı", 13, 3, (1, 1)),
        width_case("Kenar genişliği: genişlikli çizginin ilk kenarı sıfıra, öbürü kalır", 17, 0, (0, 0)),
        width_case("Kenar genişliği: tek genişlikli kenar sıfır olunca genişlikler düşer", 18, 0, (0, 0)),
        width_case("Kenar genişliği: kenar zaten o genişlikteyse hiçbir şey yazılmaz", 17, 1, (1, 1)),
        width_case("Kenar genişliği: genişliksiz kenara sıfır yazılmaz", 3, 0, (0, 0)),
    ]
    out.append({"name": "Plan: noktadan yay, hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": {**arc, "through": P(5, -5)},
         "result": completed({"changed": [bent(path_of(BY_ID[1]), 0, F(1)).entity(BY_ID[1])], "revision": "$current"}),
         "expect": UNCHANGED}]})
    out += [
        refused("Nokta yalnız Yaya dönüştür'de: Düz kenar yap'a verilmez", {**arc, "action": "straighten", "through": P(5, -5)},
                "invalid_arc_edge", ARC_ONLY_POINT, "through"),
        refused("Yarıçap yalnız Yaya dönüştür'de: köşe eklemeye verilmez", {**arc, "action": "insert", "radius": 5},
                "invalid_arc_edge", ARC_ONLY_RADIUS, "radius"),
        refused("Nokta ve yarıçap birlikte verilmez", {**arc, "through": P(5, -5), "radius": 5}, "invalid_arc_edge", ARC_BOTH, "radius"),
        refused("Yarıçap sıfır olamaz", {**arc, "radius": 0}, "invalid_arc_edge", ARC_RADIUS, "radius"),
        {"name": "Yarıçap sayı olmalı (NaN)", "steps": [
            {"op": "execute", "input": {**arc, "radius": 5}, "nonFinite": {"radius": "NaN"},
             "result": failed("invalid_arc_edge", ARC_RADIUS, "radius"), "expect": UNCHANGED}]},
        {"name": "Noktanın koordinatı sayı olmalı (∞)", "steps": [
            {"op": "execute", "input": {**arc, "through": P(5, -5)}, "nonFinite": {"through.y": "Infinity"},
             "result": failed("invalid_arc_edge", ARC_POINT, "through"), "expect": UNCHANGED}]},
        refused("Seçenekler nesneden önce denetlenir: olmayan nesnede de", {**arc, "uid": MISSING, "radius": 0},
                "invalid_arc_edge", ARC_RADIUS, "radius"),
        refused("Nokta kenarın doğrusunda: yay olmaz", {**arc, "through": P(5, 0)}, "invalid_arc_edge", ARC_ON_LINE, "through"),
        refused("Nokta kenarın ucunda: yay olmaz", {**arc, "through": P(0, 0)}, "invalid_arc_edge", ARC_ON_LINE, "through"),
        refused("Yarıçap kirişin yarısından küçük", {**arc, "radius": 4}, "invalid_arc_edge", ARC_SHORT, "radius"),
        refused("Genişlikler yalnız Kenar genişliği'nde", {**arc, "widths": [1, 1]}, "invalid_polyline", WIDTHS_ONLY, "widths"),
        refused("Kenar genişliği genişliksiz verilmez", {k: v for k, v in width.items() if k != "widths"},
                "invalid_polyline", WIDTHS_NEEDED, "widths"),
        refused("Genişlik eksi olamaz", {**width, "widths": [-1, 0]}, "invalid_polyline", WIDTHS_BAD, "widths"),
        {"name": "Genişlik sayı olmalı (NaN)", "steps": [
            {"op": "execute", "input": width, "nonFinite": {"widths[1]": "NaN"},
             "result": failed("invalid_polyline", WIDTHS_BAD, "widths"), "expect": UNCHANGED}]},
        refused("Alanın kenar genişliği yoktur", {**width, "uid": UID(1)}, "not_a_polyline", NOT_A_POLYLINE, "uid"),
        refused("Çizginin kenar genişliği yoktur", {**width, "uid": UID(5), "place": mid(0)}, "not_a_polyline", NOT_A_POLYLINE, "uid"),
        refused("Kenar genişliğinin yeri kenarın ortasıdır", {**width, "place": vertex(1)}, "wrong_place", WRONG_EDGE, "place"),
        refused("Olmayan kenarın genişliği", {**width, "place": mid(3)}, "no_such_place", NO_PLACE, "place"),
    ]
    return out


def case(name, i, action, place, index=None, part=0, topology=False, extra=(), undo=False):
    t = place.get("t")
    inp = {"uid": UID(i), "action": action, "place": place}
    if topology:
        inp["topology"] = {}
    changed, locked, invalid = edited(i, action, place["index"] if index is None else index, part, topology, t)
    warnings = [warning("neighbour_locked", NEIGHBOUR_LOCKED.format(locked))] if locked else []
    warnings += [warning("neighbour_invalid", NEIGHBOUR_INVALID.format(invalid))] if invalid else []
    ids = [c["id"] for c in changed]
    steps = [{"op": "execute", "input": inp,
              "result": completed({"changed": [UID(k) for k in ids], "ids": ids, "revision": "$current"}, warnings),
              "expect": {"ids": IDS, "entities": {str(c["id"]): c for c in changed}, "revision": "changed",
                         "canUndo": True, "canRedo": False, "dirty": True}}]
    if undo:
        steps.append({"op": "undo", "returns": LABELS[action],
                      "expect": {"ids": IDS, "entities": {str(k): BY_ID[k] for k in ids}, "canUndo": False, "canRedo": True}})
    return {"name": name, "steps": [*steps, *extra]}


def refused(name, input_, code, message, path_):
    return {"name": name, "steps": [{"op": "execute", "input": input_, "result": failed(code, message, path_), "expect": UNCHANGED}]}


def cases():
    out = [
        case("Köşeyi sil: çoklu çizginin iç köşesi gider, iki kenarı bir; adım “Köşe sil”, öznitelikler kalır", 3, "remove", vertex(1), undo=True),
        case("Köşeyi sil: açık yolun ilk köşesi kenarıyla gider", 3, "remove", vertex(0)),
        case("Köşeyi sil: açık yolun son köşesi kenarıyla gider", 3, "remove", vertex(3)),
        case("Köşeyi sil: alanın köşesi gider, iki kenarı düz bir kenar olur; renk ve öznitelikler kalır", 1, "remove", vertex(2)),
        case("Köşeyi sil: kapalı çoklu çizginin ilk köşesi gider, kapanan kenar düz", 13, "remove", vertex(0)),
        case("Köşeyi sil: yayın ucu silinince yay düzleşir (öbür kenar düzdü)", 4, "remove", vertex(1)),
        case("Topoloji: silinen köşe aynı iki köşe arasındaysa komşudan da gider; ortak köşesi başka zincirde olan komşu kalır",
             1, "remove", vertex(1), topology=True),
        case("Ortasına köşe ekle: alanın düz kenarının ortasına; adım “Köşe ekle”", 1, "insert", mid(0), undo=True),
        case("Ortasına köşe ekle: çizgi iki kenarlı çoklu çizgi olur, kimliği ve öznitelikleri kalır", 5, "insert", mid(0)),
        case("Ortasına köşe ekle: yarım çember çemberinde ikiye bölünür, iki çeyrek yay", 4, "insert", mid(0)),
        case("Ortasına köşe ekle: kapalı çoklu çizginin kapanan kenarına", 13, "insert", mid(3)),
        case("Ortasına köşe ekle: delikli alanın dış kenarına; delik kalır", 9, "insert", mid(1)),
        case("Ortasına köşe ekle: çok parçalı alanın ikinci parçasına; ilk parça kalır", 12, "insert", mid(2, part=1), part=1),
        case("Topoloji: ortak düz kenarı olan komşular eklenen köşeyi alır, ters yönde giden de", 1, "insert", mid(1), topology=True),
        case("Köşe ekle kenar üzerindeki yere (along): kirişin dörtte birinde", 1, "insert", along(0, F(1, 4))),
        case("Köşe ekle çizginin üzerindeki yere: çoklu çizgi olur", 5, "insert", along(0, F(3, 4))),
        case("Köşe ekle kapalı çoklu çizginin kapanan kenarındaki yere", 13, "insert", along(3, F(5, 8))),
        case("Düz kenar yap: yay düzleşir, başka yay kalmayınca büküm listesi düşer; adım “Düz kenar yap”", 4, "straighten", mid(0), undo=True),
        case("Yaya dönüştür: saat yönünün tersindeki alanın kenarı dışarı (+0,5); adım “Yaya dönüştür”", 1, "arc", mid(0), undo=True),
        case("Yaya dönüştür: saat yönündeki alanın kenarı yine dışarı (−0,5)", 6, "arc", mid(1)),
        case("Yaya dönüştür: açık yolun kenarı yolun sağına (+0,5)", 3, "arc", mid(1)),
        case("Topoloji: ortak kenarın yayı komşuya da; ters yönde giden komşuda eksi işaretle", 1, "arc", mid(1), topology=True),
        case("Topoloji: kilitli komşu değişmez, neighbour_locked uyarısıyla sayılır; öbür komşu izler", 1, "arc", mid(2), topology=True),
        case("Topoloji: köşesi silinince geçersiz kalacak üçgen komşu değişmez, neighbour_invalid uyarısıyla sayılır", 15, "remove", vertex(1),
             topology=True),
    ]
    out.append({"name": "Zaten düz kenara Düz kenar yap: hiçbir şey yazılmaz, listeler boş", "steps": [
        {"op": "execute", "input": {"uid": UID(1), "action": "straighten", "place": mid(0)},
         "result": completed({"changed": [], "ids": [], "revision": "$current"}), "expect": UNCHANGED}]})
    out.append({"name": "Zaten yay olan kenara Yaya dönüştür: hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": {"uid": UID(4), "action": "arc", "place": mid(0)},
         "result": completed({"changed": [], "ids": [], "revision": "$current"}), "expect": UNCHANGED}]})
    changed, _, _ = edited(1, "insert", 0)
    out.append({"name": "Plan: nesne olacağı gibi; hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": {"uid": UID(1), "action": "insert", "place": mid(0)},
         "result": completed({"changed": changed, "revision": "$current"}), "expect": UNCHANGED}]})
    one = {"uid": UID(1), "action": "remove", "place": vertex(2)}
    out += [
        refused("Kimlik geçersiz", {**one, "uid": BAD}, "invalid_uid", bad_uid(BAD), "uid"),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Nesne çizimde yok", {**one, "uid": MISSING}, "entity_not_found", not_found(MISSING), "uid"),
        refused("Kilitli katmandaki çizgiye köşe eklenmez", {"uid": UID(10), "action": "insert", "place": mid(0)}, "layer_locked", LOCKED, "uid"),
        refused("Daire yol değildir", {"uid": UID(8), "action": "insert", "place": mid(0)}, "not_a_path", NOT_A_PATH, "uid"),
        refused("Çizgiden köşe silinmez", {"uid": UID(5), "action": "remove", "place": vertex(0)}, "not_a_path", NOT_A_PATH, "uid"),
        refused("Köşeyi sil'in yeri kenarın ortası olamaz", {**one, "place": mid(2)}, "wrong_place", WRONG_REMOVE, "place"),
        refused("Köşe eklemenin yeri köşe olamaz", {"uid": UID(1), "action": "insert", "place": vertex(2)}, "wrong_place", WRONG_EDGE, "place"),
        refused("Düz kenar yap'ın yeri kenarın üzerindeki bir yer olamaz", {"uid": UID(4), "action": "straighten", "place": along(0, F(1, 2))},
                "wrong_place", WRONG_EDGE, "place"),
        refused("Kenarın ucundaki yere köşe eklenmez: çekirdeğin sözü", {"uid": UID(1), "action": "insert", "place": along(0, 0)},
                "no_such_place", "Köşe bir kenarın iç kısmına eklenmeli; mevcut köşeye çok yakın.", "place"),
        refused("Çizginin ucuna köşe eklenmez", {"uid": UID(5), "action": "insert", "place": along(0, 1)},
                "no_such_place", "Köşe çizginin iç kısmına eklenmeli.", "place"),
        refused("Kenarın dışındaki yer (t 1'den büyük)", {"uid": UID(1), "action": "insert", "place": along(0, F(3, 2))},
                "no_such_place", NO_PLACE, "place"),
        refused("Deliğin köşesi silinmez", {"uid": UID(9), "action": "remove", "place": vertex(0, ring=1)}, "hole_edit", HOLE_EDIT, "place.ring"),
        refused("Olmayan köşe", {**one, "place": vertex(4)}, "no_such_place", NO_PLACE, "place"),
        refused("Açık yolun olmayan kenarı", {"uid": UID(3), "action": "arc", "place": mid(3)}, "no_such_place", NO_PLACE, "place"),
        refused("Olmayan parça", {**one, "place": vertex(0, part=1)}, "no_such_place", NO_PLACE, "place"),
        refused("Üçgenden köşe silinmez", {"uid": UID(6), "action": "remove", "place": vertex(0)}, "too_few_corners",
                "Kapalı alanda en az üç köşe kalmalı.", "place"),
        refused("İki köşeli çoklu çizgiden köşe silinmez", {"uid": UID(7), "action": "remove", "place": vertex(1)}, "too_few_points",
                "Çoklu çizgide en az iki köşe kalmalı.", "place"),
    ]
    out += adr_0288_cases()
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": {"uid": UID(3), "action": "remove", "place": vertex(1)},
         "result": completed({"changed": [UID(3)], "ids": [3], "revision": "$current"})},
        {"op": "execute", "input": {**one, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}}]})
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.entities.vertex", "commandVersion": 1,
        "title": "Köşe ve kenar",
        "note": "Written by scripts/fixtures/vertex_command_cases.py from the operations' definitions (docs/adr/0047, 0160, 0279, 0288); do not edit by hand.",
        "setup": SETUP, "cases": cases(),
    }


def main() -> int:
    doc = build()
    assert_no_negative_zero(doc, "cad.entities.vertex")
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path_ = OUT / "cad.entities.vertex.json"
    if "--check" in sys.argv[1:]:
        if not path_.exists() or path_.read_text(encoding="utf-8") != text:
            print(f"{path_.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.entities.vertex durumları güncel: {len(doc['cases'])} durum.")
        return 0
    path_.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path_.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
