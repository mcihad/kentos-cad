"""The shared cases of the product command cad.dimensions.create (docs/adr/0255).

    python3 scripts/fixtures/dimension_create_cases.py           # writes the file
    python3 scripts/fixtures/dimension_create_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.dimensions.create.json. The checks, their order, codes, paths and messages are
written here by hand from the ADR. What the command makes is worked out independently, from Ölçülendirme's
definitions (docs/adr/0061, 0147 §7, 0183, 0205, 0249), without KentOS code:

- Hizalı, Semt, Eğim: the dimension from a to b, its line the signed distance of `at` from the line a→b (left
  positive): (dx·(y − ay) − dy·(x − ax)) / |ab|.
- Doğrusal: the direction given, else ΔX (90) when `at` lies further beyond the points across than along, else
  ΔY (0); the distance −sin t·(x − ax) + cos t·(y − ay), t the direction in radians.
- Koordinat: the point, its leader ending at `at`; its Y (0) when `at` is further up or down than across, else X.
- Açı between two straight edges: their lines' crossing the vertex; of the four arm directions the two bounding
  the sector `at` is in (the one just clockwise of it first); each arm as far as its edge was clicked (at least a
  millimetre); the arc's radius the distance of `at` from the vertex. Köşeden: from the first arm point
  counter-clockwise to the second when `at` lies in that sweep, else the other way. Yaydan: the arc's own ends.
- Yarıçap and Çap: the point on the circle toward `at`, its line `at`'s distance outside the circle.
- Yay uzunluğu: the arc's ends counter-clockwise (Kısmi: the points put on its circle), the distance of `at` from
  the arc outward.
- Kırıklı yarıçap: from the centre shown to the circle toward `on`, its jog where `at` is along the radius, as far
  as the jog fits.
- A point given as a place is that place's point now and is bound (`refs`): on Hizalı, Doğrusal, Koordinat (a only),
  Semt, Eğim and Açı by its vertex (docs/adr/0249 §3, 0320 §1; the vertex's arms bound as they lie after the sector
  is chosen). Eğim's elevations are given or the place's vertex's.
- What the object kinds follow (docs/adr/0320 §1): Açı between two edges binds `a` and `b` to the edges their arms
  lie along (the arm nearer parallel to the first edge on it), each at the fraction of its edge nearest where it was
  clicked, with the rule `edges`; Yaydan and Yay uzunluğu their two ends on the arc, at the fractions of its sweep
  of the ends, with the rule `curve`; Yarıçap, Çap and Kırıklı yarıçap `b` on the curve, at the fraction of the
  point of it nearest `b` (past an arc's end, that end), with the rule `curve`.
- Zincir ölçü and Baz ölçü (docs/adr/0320 §3): the base's direction (aligned: its own; linear: its angle), its line
  through `a + n·offset` (n the direction's left normal); Zincir from the base's `b` to the point, its line on the
  base's; Baz from the base's `a`, one level (three times the base's height) further out on the side the base lies.
  Their first point follows what the base's follows; their look and height the base's.
- Merkez işareti, Eksen çizgisi and Kot (docs/adr/0322): a centre mark is the curve's centre and the point of it east
  of the centre, its extension (`offset`) the value height unless typed, its extension lines dash-dot; it binds `b` on
  the curve with the rule `curve`. An axis runs from the middle of the two edges' starts to the middle of their ends
  (the second edge turned to run the first's way when their directions are against each other), the same extension,
  each edge bound where it was clicked with the rule `pair`. A level is its point and `at`, its elevation given or its
  place's.
- Height: the style's at the project's scale (mm / 1000 × N), Standart's the project's Ölçü height (2.5 mm).

The drawing's coordinates are doubles exactly and the distances whole (3-4-5 triangles), so the arithmetic is
exact but where a direction is divided out; that is done here in doubles in the definition's order. The few
transcendental values (sin and cos of π/2 and π/4) are computed to 60 digits by their series and rounded to the
nearest double, not by the platform's libm; atan2 decides sectors only, but for π/4 (atan2 of a diagonal, which
every libm gives correctly rounded).

`--check` rebuilds the file in memory and compares it with the one on disk.
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
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
X, Y = 487000, 4420000
PLOT_SCALE = 500
MIMARI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001"

getcontext().prec = 60


def _series(x, first, step):
    total, term, n = Decimal(0), first, step
    while abs(term) > Decimal(10) ** -58:
        total += term
        term *= -x * x / ((n + 1) * (n + 2))
        n += 2
    return total


def sin_(x):
    """sin of the double x, correctly rounded (60 digits, then the nearest double)."""
    d = Decimal(x)
    return float(_series(d, d, 1))


def cos_(x):
    """cos of the double x, correctly rounded."""
    return float(_series(Decimal(x), Decimal(1), 0))


def layer(i, name, visible=True, locked=False, kind="layer", children=()):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True, "style": style,
            "children": list(children)}


def plain(v):
    return int(v) if float(v).is_integer() else v


def P(x, y):
    return {"x": plain(x), "y": plain(y)}


ENTITIES = [
    {"kind": "point", "id": 10, "layerId": "yapi", "attrs": {}, "p": P(X + 2, Y + 20), "z": 50},
    {"kind": "line", "id": 11, "layerId": "yapi", "attrs": {}, "a": P(X, Y), "b": P(X + 12, Y), "za": 100, "zb": 103},
    {"kind": "polygon", "id": 12, "layerId": "yapi", "attrs": {},
     "pts": [P(X + 20, Y), P(X + 28, Y), P(X + 28, Y + 6), P(X + 20, Y + 6)]},
    {"kind": "arc", "id": 13, "layerId": "yapi", "attrs": {}, "c": P(X + 30, Y + 30), "r": 5, "a0": 0, "a1": math.pi / 2},
    {"kind": "circle", "id": 14, "layerId": "yapi", "attrs": {}, "c": P(X + 50, Y + 30), "r": 5},
    {"kind": "line", "id": 15, "layerId": "yapi", "attrs": {}, "a": P(X, Y + 40), "b": P(X + 10, Y + 40)},
    {"kind": "line", "id": 16, "layerId": "yapi", "attrs": {}, "a": P(X + 5, Y + 36), "b": P(X + 5, Y + 48)},
    {"kind": "line", "id": 17, "layerId": "yapi", "attrs": {}, "a": P(X, Y + 50), "b": P(X + 10, Y + 50)},
]
IDS = [e["id"] for e in ENTITIES]
NEXT = 18
LINE_11 = ((X, Y), (X + 12, Y))
RECT_12 = [(X + 20, Y), (X + 28, Y), (X + 28, Y + 6), (X + 20, Y + 6)]
ARC_C, CIRCLE_C, R = (X + 30, Y + 30), (X + 50, Y + 30), 5

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Ölçü komutu",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": PLOT_SCALE,
                 "dimensionStyles": [{"id": MIMARI, "name": "Mimari", "height": 3.5}]},
    "origin": {"x": X, "y": Y},
    "layers": [
        layer("yapi", "Yapı"), layer("olcu", "Ölçü"), layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False),
        layer("grup", "Grup", kind="group", children=[layer("alt", "Alt")]),
    ],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}

UID = lambda i: f"$uidOf:{i}"  # noqa: E731
BAD = "Yapı-01"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STANDARD_HEIGHT = 2.5 / 1000.0 * PLOT_SCALE
MIMARI_HEIGHT = 3.5 / 1000.0 * PLOT_SCALE


def bad_uid(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."


def not_found(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def not_finite_point(whose, axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def warning(code, message, path=None):
    w = {"code": code, "message": message}
    if path is not None:
        w["path"] = path
    return w


def refused(name, input_, code, message, path, non_finite=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = failed(code, message, path)
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


# ── The measures' input ───────────────────────────────────────────────────────────────────────────────────────────

def at_(x, y):
    return {"kind": "at", "at": P(x, y)}


def place(eid, kind, **where):
    return {"kind": "place", "uid": UID(eid), "place": {"kind": kind, **where}}


def vertex(eid, index):
    return place(eid, "vertex", part=0, ring=0, index=index)


def edge(eid, near):
    return {"uid": UID(eid), "near": P(*near)}


def point_of(p):
    """A measured point's coordinates now (the setup's)."""
    if p["kind"] == "at":
        return (p["at"]["x"], p["at"]["y"])
    eid, where = int(p["uid"].split(":")[1]), p["place"]
    if eid == 10:
        return (X + 2, Y + 20)
    if eid == 11 and where["kind"] == "along":
        # Along the line: a + t·(b − a) (docs/adr/0263 §1).
        (ax, ay), (bx, by), t = LINE_11[0], LINE_11[1], where["t"]
        return (ax + (bx - ax) * t, ay + (by - ay) * t)
    if eid == 11:
        return LINE_11[0] if where["kind"] == "start" else LINE_11[1]
    if eid == 12:
        return RECT_12[where["index"]]
    raise ValueError(p)


def ref_of(p):
    return None if p["kind"] == "at" else {"uid": p["uid"], "place": p["place"]}


def along(eid, t):
    """An `along` place on an object's one edge or curve (docs/adr/0263 §1)."""
    return {"uid": UID(eid), "place": {"kind": "along", "part": 0, "ring": 0, "index": 0, "t": plain(t)}}


def t_on_segment(a, b, p):
    """The fraction of the segment a→b at the point of it nearest p, within 0 and 1."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    return min(1.0, max(0.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)))


def t_on_arc(c, a0, sw, p):
    """The fraction of a counter-clockwise sweep from a0 at the point of the arc nearest p: its angle's; past the
    arc, the nearer end's; a full circle's angle over a turn."""
    d = norm_angle(math.atan2(p[1] - c[1], p[0] - c[0]) - a0)
    if sw >= 2 * math.pi - 1e-12:
        return d / (2 * math.pi)
    if d <= sw:
        return min(1.0, max(0.0, d / sw))
    return 1.0 if d - sw < 2 * math.pi - d else 0.0



# ── The dimensions (see the module) ───────────────────────────────────────────────────────────────────────────────

def hypot(dx, dy):
    return math.hypot(dx, dy)


def signed_offset(a, b, p):
    dx, dy = b[0] - a[0], b[1] - a[1]
    length = hypot(dx, dy) or 1.0
    return (dx * (p[1] - a[1]) - dy * (p[0] - a[0])) / length


def linear_angle_for(a, b, p):
    out_x = max(min(a[0], b[0]) - p[0], p[0] - max(a[0], b[0]), 0.0)
    out_y = max(min(a[1], b[1]) - p[1], p[1] - max(a[1], b[1]), 0.0)
    return 90.0 if out_x > out_y else 0.0


def linear_offset(a, angle, p):
    t = angle * math.pi / 180.0
    return -sin_(t) * (p[0] - a[0]) + cos_(t) * (p[1] - a[1])


def norm_angle(a):
    r = math.fmod(a, 2 * math.pi)
    return r + 2 * math.pi if r < 0 else r


def sector_arms(c, u1, u2, p):
    dirs = [(u, math.atan2(u[1], u[0])) for u in (u1, u2, (-u1[0], -u1[1]), (-u2[0], -u2[1]))]
    tp = math.atan2(p[1] - c[1], p[0] - c[0])
    best, best_sweep = (u1, u2), math.inf
    for si, s in enumerate(dirs):
        back = norm_angle(tp - s[1])
        for ei, e in enumerate(dirs):
            if ei == si:
                continue
            sweep = norm_angle(e[1] - s[1])
            if sweep > 1e-9 and back <= sweep and sweep < best_sweep:
                best_sweep, best = sweep, (s[0], e[0])
    return best


def edge_arms(e1, e2, p):
    """Two straight edges (a, b, clicked) as an angle's arms; their lines cross at a dyadic parameter here."""
    (a, b, n1), (c2, d, n2) = e1, e2
    r, s = (b[0] - a[0], b[1] - a[1]), (d[0] - c2[0], d[1] - c2[1])
    den = r[0] * s[1] - r[1] * s[0]
    t = ((c2[0] - a[0]) * s[1] - (c2[1] - a[1]) * s[0]) / den
    c = (a[0] + t * r[0], a[1] + t * r[1])

    def unit(e):
        length = hypot(e[1][0] - e[0][0], e[1][1] - e[0][1])
        return ((e[1][0] - e[0][0]) / length, (e[1][1] - e[0][1]) / length)

    u1, u2 = unit(e1), unit(e2)
    s0, e0 = sector_arms(c, u1, u2, p)

    def reach(u):
        clicked = n1 if abs(u[0] * u1[1] - u[1] * u1[0]) < 1e-9 else n2
        return max(hypot(clicked[0] - c[0], clicked[1] - c[1]), 1e-3)

    return c, (c[0] + s0[0] * reach(s0), c[1] + s0[1] * reach(s0)), (c[0] + e0[0] * reach(e0), c[1] + e0[1] * reach(e0))


def vertex_arms(c, p1, p2, loc):
    t = norm_angle(math.atan2(loc[1] - c[1], loc[0] - c[0]) - math.atan2(p1[1] - c[1], p1[0] - c[0]))
    sweep = norm_angle(math.atan2(p2[1] - c[1], p2[0] - c[0]) - math.atan2(p1[1] - c[1], p1[0] - c[0]))
    return (c, p1, p2) if t <= sweep else (c, p2, p1)


def arc_at(t):
    """The arc 13 (centre ARC_C, radius 5, from 0 a quarter turn counter-clockwise) at t from its start."""
    return (ARC_C[0] + cos_(t) * R, ARC_C[1] + sin_(t) * R)


def arc_ends(between=None):
    span = math.pi / 2
    if between is None:
        return arc_at(0.0), arc_at(span)

    def along(p):
        t = norm_angle(math.atan2(p[1] - ARC_C[1], p[0] - ARC_C[0]) - 0.0)
        return t if t <= span else (span if t - span < 2 * math.pi - t else 0.0)

    t1, t2 = along(between[0]), along(between[1])
    return arc_at(min(t1, t2)), arc_at(max(t1, t2))


ARC_SWEEP = norm_angle(math.pi / 2 - 0.0)


def radial(c, r, loc):
    length = hypot(loc[0] - c[0], loc[1] - c[1])
    u = ((loc[0] - c[0]) / length, (loc[1] - c[1]) / length)
    return (c[0] + u[0] * r, c[1] + u[1] * r), max(0.0, length - r)


def jog(a, b, shown, p):
    r = hypot(b[0] - a[0], b[1] - a[1])
    u = ((b[0] - a[0]) / r, (b[1] - a[1]) / r)
    n = (-u[1], u[0])
    s = (shown[0] - b[0]) * n[0] + (shown[1] - b[1]) * n[1]
    t = (b[0] - shown[0]) * u[0] + (b[1] - shown[1]) * u[1]
    along = (p[0] - shown[0]) * u[0] + (p[1] - shown[1]) * u[1]
    return max(0.0, min(along, t - abs(s)))


def dimension(eid, a, b, offset, layer_id="olcu", kind=None, angle=None, c=None, za=None, zb=None, refs=None,
              height=STANDARD_HEIGHT, look=None, mask=False, **extra):
    e = {"kind": "dimension", "id": eid, "layerId": layer_id, "attrs": {}, **extra, "a": P(*a), "b": P(*b),
         "offset": plain(offset), "height": plain(height)}
    if kind is not None:
        e["style"] = kind
    if angle is not None:
        e["angle"] = plain(angle)
    if c is not None:
        e["c"] = P(*c)
    if mask:
        e["mask"] = True
    if za is not None:
        e["za"] = plain(za)
    if zb is not None:
        e["zb"] = plain(zb)
    if refs:
        e["refs"] = refs
    e.update(look or {})
    return e


def refs(**points):
    out = {k: v for k, v in points.items() if v is not None}
    return out or None


def case(name, inp, made, warnings=(), plan=False, undo=False, within=None):
    """`made(eid)`: the dimension written; `within`, how near its transcendental numbers must be."""
    if plan:
        step = {"op": "plan", "input": inp, "result": completed({"entities": [made(0)], "revision": "$current"}, warnings),
                "expect": UNCHANGED}
        if within is not None:
            step["within"] = within
        return {"name": name, "steps": [step]}
    steps = [{"op": "execute", "input": inp,
              "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"}, warnings),
              "expect": {"ids": IDS + [NEXT], "entities": {str(NEXT): made(NEXT)}, "revision": "changed", "canUndo": True,
                         "canRedo": False, "dirty": True}}]
    if within is not None:
        steps[0]["within"] = within
    if undo:
        steps.append({"op": "undo", "returns": "Ekle", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}})
    return {"name": name, "steps": steps}


def inp(measure, at, layer_id="olcu", **extra):
    return {"measure": measure, "at": P(*at), "layerId": layer_id, **extra}


def written_cases():
    out = []
    # Hizalı by the line's two ends: bound.
    m = {"kind": "aligned", "a": place(11, "start"), "b": place(11, "end")}
    a, b, at = LINE_11[0], LINE_11[1], (X + 6, Y + 3)
    out.append(case("Hizalı: çizginin iki ucu yerleriyle, onlara bağlı; adım “Ekle”", inp(m, at),
                    lambda i: dimension(i, a, b, signed_offset(a, b, at), refs=refs(a=ref_of(m["a"]), b=ref_of(m["b"]))), undo=True))
    # Hizalı from a place along the line (docs/adr/0263 §1) to the area's corner: both bound.
    m = {"kind": "aligned", "a": place(11, "along", part=0, ring=0, index=0, t=0.25), "b": vertex(12, 0)}
    a, b, at = point_of(m["a"]), point_of(m["b"]), (X + 10, Y + 3)
    out.append(case("Hizalı: çizginin çeyreğinden (along) alanın köşesine, ikisi de bağlı", inp(m, at),
                    lambda i: dimension(i, a, b, signed_offset(a, b, at), refs=refs(a=ref_of(m["a"]), b=ref_of(m["b"])))))
    # Hizalı by coordinates, the distance typed.
    m = {"kind": "aligned", "a": at_(X + 20, Y + 10), "b": at_(X + 26, Y + 18)}
    out.append(case("Hizalı: koordinatlarla, serbest; uzaklık yazılmış", inp(m, (X + 40, Y + 40), offset=-2),
                    lambda i: dimension(i, (X + 20, Y + 10), (X + 26, Y + 18), -2)))
    # Doğrusal: ΔY as it lies; the area's vertices bound.
    m = {"kind": "linear", "a": vertex(12, 0), "b": vertex(12, 2)}
    a, b, at = RECT_12[0], RECT_12[2], (X + 24, Y + 10)
    angle = linear_angle_for(a, b, at)
    out.append(case("Doğrusal: imleç noktaların üstünde, ΔY; alanın köşelerine bağlı", inp(m, at),
                    lambda i: dimension(i, a, b, linear_offset(a, angle, at), kind="linear", angle=angle,
                                        refs=refs(a=ref_of(m["a"]), b=ref_of(m["b"])))))
    # Doğrusal: the direction given.
    m = {"kind": "linear", "a": at_(X + 20, Y), "b": at_(X + 28, Y + 6), "angle": 90}
    a, b, at = (X + 20, Y), (X + 28, Y + 6), (X + 32, Y + 3)
    out.append(case("Doğrusal: doğrultu verilmiş (90, ΔX)", inp(m, at),
                    lambda i: dimension(i, a, b, linear_offset(a, 90.0, at), kind="linear", angle=90)))
    # Koordinat of a point object: its Y as the leader lies.
    m = {"kind": "ordinate", "a": place(10, "node", part=0)}
    a, at = (X + 2, Y + 20), (X + 3, Y + 26)
    axis = 0.0 if abs(at[1] - a[1]) >= abs(at[0] - a[0]) else 90.0
    out.append(case("Koordinat: noktanın yeri, eksen imlece göre (Y); noktaya bağlı", inp(m, at),
                    lambda i: dimension(i, a, at, 0, kind="ordinate", angle=axis, refs=refs(a=ref_of(m["a"])))))
    # Semt.
    m = {"kind": "azimuth", "a": at_(X, Y + 60), "b": at_(X + 3, Y + 64)}
    a, b, at = (X, Y + 60), (X + 3, Y + 64), (X + 1, Y + 63)
    out.append(case("Semt: iki nokta", inp(m, at), lambda i: dimension(i, a, b, signed_offset(a, b, at), kind="azimuth")))
    # Eğim: the elevations from the line's ends.
    m = {"kind": "slope", "a": place(11, "start"), "b": place(11, "end")}
    a, b, at = LINE_11[0], LINE_11[1], (X + 6, Y - 2)
    out.append(case("Eğim: kotlar yerlerin köşelerinden", inp(m, at),
                    lambda i: dimension(i, a, b, signed_offset(a, b, at), kind="slope", za=100, zb=103,
                                        refs=refs(a=ref_of(m["a"]), b=ref_of(m["b"])))))
    m = {"kind": "slope", "a": at_(X, Y + 70), "b": at_(X + 8, Y + 70), "za": 10, "zb": 12.5}
    a, b, at = (X, Y + 70), (X + 8, Y + 70), (X + 4, Y + 71)
    out.append(case("Eğim: kotlar verilmiş", inp(m, at),
                    lambda i: dimension(i, a, b, signed_offset(a, b, at), kind="slope", za=10, zb=12.5)))
    # Açı between two edges.
    m = {"kind": "angle", "first": edge(15, (X + 9, Y + 40)), "second": edge(16, (X + 5, Y + 47))}
    at = (X + 8, Y + 44)
    c, a, b = edge_arms(((X, Y + 40), (X + 10, Y + 40), (X + 9, Y + 40)), ((X + 5, Y + 36), (X + 5, Y + 48), (X + 5, Y + 47)), at)
    first_t = t_on_segment((X, Y + 40), (X + 10, Y + 40), (X + 9, Y + 40))
    second_t = t_on_segment((X + 5, Y + 36), (X + 5, Y + 48), (X + 5, Y + 47))

    def across(p, e):
        (ax, ay), (bx, by) = e
        ux, uy, vx, vy = bx - ax, by - ay, p[0] - c[0], p[1] - c[1]
        return abs(ux * vy - uy * vx) / max(hypot(ux, uy) * hypot(vx, vy), 2.2250738585072014e-308)

    on_first = across(a, ((X, Y + 40), (X + 10, Y + 40))) <= across(b, ((X, Y + 40), (X + 10, Y + 40)))
    ra, rb = (along(15, first_t), along(16, second_t)) if on_first else (along(16, second_t), along(15, first_t))
    out.append(case("Açı: iki düz kenar, kollar tıklanan yere kadar, yay imlecin bölgesinde; kenarlara bağlı (edges)", inp(m, at),
                    lambda i: dimension(i, a, b, hypot(at[0] - c[0], at[1] - c[1]), kind="angular", c=c,
                                        refs={"a": ra, "b": rb, "rule": "edges"})))
    # Köşeden, both ways.
    for name, at in (("Açı: köşeden, birinci koldan saat yönünün tersine", (X + 43, Y + 44)),
                     ("Açı: köşeden, imleç öbür yanda: kollar yer değiştirir", (X + 37, Y + 36))):
        m = {"kind": "angleVertex", "c": at_(X + 40, Y + 40), "a": at_(X + 44, Y + 40), "b": at_(X + 40, Y + 43)}
        vc, va, vb = vertex_arms((X + 40, Y + 40), (X + 44, Y + 40), (X + 40, Y + 43), at)
        out.append(case(name, inp(m, at), lambda i, va=va, vb=vb, vc=vc, at=at: dimension(
            i, va, vb, hypot(at[0] - vc[0], at[1] - vc[1]), kind="angular", c=vc)))
    # Köşeden by the area's corners: the three points bound as the arms lie.
    for name, at in (("Açı: köşeden, alanın köşeleri yerleriyle; üçü de bağlı", (X + 22, Y + 2)),
                     ("Açı: köşeden yerlerle, imleç öbür yanda: kollar ve bağları yer değiştirir", (X + 18, Y - 2))):
        m = {"kind": "angleVertex", "c": vertex(12, 0), "a": vertex(12, 1), "b": vertex(12, 3)}
        vc, va, vb = vertex_arms(RECT_12[0], RECT_12[1], RECT_12[3], at)
        ra, rb = (ref_of(m["a"]), ref_of(m["b"])) if va == RECT_12[1] else (ref_of(m["b"]), ref_of(m["a"]))
        out.append(case(name, inp(m, at), lambda i, va=va, vb=vb, vc=vc, at=at, ra=ra, rb=rb, rc=ref_of(m["c"]): dimension(
            i, va, vb, hypot(at[0] - vc[0], at[1] - vc[1]), kind="angular", c=vc, refs={"a": ra, "b": rb, "c": rc})))
    # Yaydan.
    m = {"kind": "angleArc", "of": edge(13, (X + 34, Y + 33))}
    at = (ARC_C[0] + 6, ARC_C[1] + 8)
    a, b = arc_ends()
    arc_refs = {"a": along(13, t_on_arc(ARC_C, 0.0, ARC_SWEEP, a)), "b": along(13, t_on_arc(ARC_C, 0.0, ARC_SWEEP, b)), "rule": "curve"}
    out.append(case("Açı: yayın kendi açısı (Yaydan); yayın iki ucuna bağlı (curve)", inp(m, at),
                    lambda i: dimension(i, a, b, hypot(at[0] - ARC_C[0], at[1] - ARC_C[1]), kind="angular", c=ARC_C, refs=arc_refs)))
    # Yarıçap and Çap.
    at = (CIRCLE_C[0] + 6, CIRCLE_C[1] + 8)
    pb, off = radial(CIRCLE_C, R, at)
    circle_ref = {"b": along(14, t_on_arc(CIRCLE_C, 0.0, 2 * math.pi, pb)), "rule": "curve"}
    out.append(case("Yarıçap: daire, imlecin yönünde; dairenin üstündeki noktaya bağlı (curve)",
                    inp({"kind": "radius", "of": edge(14, (X + 55, Y + 30))}, at),
                    lambda i: dimension(i, CIRCLE_C, pb, off, kind="radius", refs=circle_ref), within=1e-12))
    at2 = (ARC_C[0] - 6, ARC_C[1] + 8)
    pb2, off2 = radial(ARC_C, R, at2)
    out.append(case("Çap: yayın dairesi; nokta yayın ötesinde, bağı yayın yakın ucunda (curve)",
                    inp({"kind": "diameter", "of": edge(13, (X + 34, Y + 33))}, at2),
                    lambda i: dimension(i, ARC_C, pb2, off2, kind="diameter",
                                        refs={"b": along(13, t_on_arc(ARC_C, 0.0, ARC_SWEEP, pb2)), "rule": "curve"})))
    # Yay uzunluğu.
    at = (ARC_C[0] + 6, ARC_C[1] + 8)
    a, b = arc_ends()
    out.append(case("Yay uzunluğu: bütün yay, imlecin uzaklığında; yayın iki ucuna bağlı (curve)",
                    inp({"kind": "arcLength", "of": edge(13, (X + 34, Y + 33))}, at),
                    lambda i: dimension(i, a, b, hypot(at[0] - ARC_C[0], at[1] - ARC_C[1]) - hypot(a[0] - ARC_C[0], a[1] - ARC_C[1]),
                                        kind="arcLength", c=ARC_C, refs=arc_refs)))
    between = [(ARC_C[0] + 8, ARC_C[1]), (ARC_C[0] + 2, ARC_C[1] + 2)]
    ka, kb = arc_ends(between)
    part_refs = {"a": along(13, t_on_arc(ARC_C, 0.0, ARC_SWEEP, ka)), "b": along(13, t_on_arc(ARC_C, 0.0, ARC_SWEEP, kb)), "rule": "curve"}
    out.append(case("Yay uzunluğu: Kısmi, iki nokta arası, uzaklık yazılmış; iki ucu yayın üstünde bağlı",
                    inp({"kind": "arcLength", "of": edge(13, (X + 34, Y + 33)), "between": [P(*between[0]), P(*between[1])]}, at, offset=2),
                    lambda i: dimension(i, ka, kb, 2, kind="arcLength", c=ARC_C, refs=part_refs), within=1e-12))
    # Kırıklı yarıçap.
    on, shown, at = (CIRCLE_C[0] + 10, CIRCLE_C[1]), (CIRCLE_C[0] - 15, CIRCLE_C[1] + 2), (CIRCLE_C[0] - 5, CIRCLE_C[1] + 3)
    length = hypot(on[0] - CIRCLE_C[0], on[1] - CIRCLE_C[1])
    jb = (CIRCLE_C[0] + (on[0] - CIRCLE_C[0]) / length * R, CIRCLE_C[1] + (on[1] - CIRCLE_C[1]) / length * R)
    out.append(case("Kırıklı yarıçap: gösterilen merkezden, kırığı imleçte; dairenin üstündeki noktaya bağlı (curve)",
                    inp({"kind": "jogged", "of": edge(14, (X + 55, Y + 30)), "shownCentre": P(*shown), "on": P(*on)}, at),
                    lambda i: dimension(i, CIRCLE_C, jb, jog(CIRCLE_C, jb, shown, at), kind="jogged", c=shown,
                                        refs={"b": along(14, t_on_arc(CIRCLE_C, 0.0, 2 * math.pi, jb)), "rule": "curve"})))
    # A style, a mask, the look, a hidden layer, a plan.
    m = {"kind": "aligned", "a": at_(X + 20, Y + 10), "b": at_(X + 26, Y + 18)}
    a, b, at = (X + 20, Y + 10), (X + 26, Y + 18), (X + 20, Y + 20)
    off = signed_offset(a, b, at)
    out.append(case("Ölçü stiliyle (adı büyük küçük harf gözetmez): görünüşü ve yüksekliği stilin", inp(m, at, dimensionStyle="mimari"),
                    lambda i: dimension(i, a, b, off, height=MIMARI_HEIGHT, look={"dimStyle": MIMARI})))
    out.append(case("Standart adıyla: Standart'ın görünüşü ve projenin Ölçü yüksekliği", inp(m, at, dimensionStyle="Standart"),
                    lambda i: dimension(i, a, b, off)))
    out.append(case("Zemin, renk ve kalınlık", inp(m, at, mask=True, color="#E5484D", lineWeight=0.35),
                    lambda i: dimension(i, a, b, off, mask=True, color="#E5484D", lineWeight=0.35)))
    out.append(case("Yazı yüksekliği verilmiş", inp(m, at, height=2), lambda i: dimension(i, a, b, off, height=2)))
    out.append(case("Gizli katmana: layer_hidden uyarısıyla yazılır", inp(m, at, "gizli"),
                    lambda i: dimension(i, a, b, off, layer_id="gizli"),
                    warnings=[warning("layer_hidden", "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "layerId")]))
    m = {"kind": "aligned", "a": place(11, "start"), "b": place(11, "end")}
    a, b, at = LINE_11[0], LINE_11[1], (X + 6, Y + 3)
    out.append(case("Plan: ölçü yazılacağı gibi, id 0; hiçbir şey yazılmaz", inp(m, at),
                    lambda i: dimension(i, a, b, signed_offset(a, b, at), refs=refs(a=ref_of(m["a"]), b=ref_of(m["b"]))), plan=True))
    return out


# ── Zincir ölçü and Baz ölçü (docs/adr/0320 §3) ───────────────────────────────────────────────────────────────────

LEVEL_TEXT_HEIGHTS = 3.0
NO_BASE = "Temel ölçü hizalı ya da doğrusal bir ölçü değil: zincir ve baz ölçü yalnız hizalı ya da doğrusal ölçüden sürer."
# The base: Hizalı along the line 11, bound to its ends, its line 3 m above (left of a→b).
BASE_INPUT = inp({"kind": "aligned", "a": place(11, "start"), "b": place(11, "end")}, (X + 6, Y + 3))
BASE = {"a": LINE_11[0], "b": LINE_11[1], "offset": 3.0, "kind": None, "angle": None,
        "refs": {"a": {"uid": UID(11), "place": {"kind": "start"}}, "b": {"uid": UID(11), "place": {"kind": "end"}}}}


def measuring(base):
    """The base's direction in degrees from east and a point of its line."""
    a, b = base["a"], base["b"]
    deg = base["angle"] if base["kind"] == "linear" else math.atan2(b[1] - a[1], b[0] - a[0]) * 180.0 / math.pi
    t = deg * math.pi / 180.0
    n = (-sin_(t), cos_(t))
    return deg, (a[0] + n[0] * base["offset"], a[1] + n[1] * base["offset"]), n


def linear_through(base, a, p, extra):
    deg, line_at, n = measuring(base)
    offset = (line_at[0] - a[0]) * n[0] + (line_at[1] - a[1]) * n[1]
    side = -1.0 if offset < 0.0 else 1.0
    return {"a": a, "b": p, "offset": offset + side * extra, "angle": deg}


def chained(name, measure, to, made_point, which):
    """A base written first, then a Zincir or Baz to `to` from it."""
    to_p = point_of(to)
    g = (linear_through(BASE, BASE["b"], to_p, 0.0) if which == "continue"
         else linear_through(BASE, BASE["a"], to_p, 1.0 * LEVEL_TEXT_HEIGHTS * STANDARD_HEIGHT))
    first = BASE["refs"]["b" if which == "continue" else "a"]
    made = dimension(NEXT + 1, g["a"], g["b"], g["offset"], kind="linear", angle=g["angle"],
                     refs=refs(a=first, b=ref_of(to)))
    base_made = dimension(NEXT, BASE["a"], BASE["b"], BASE["offset"], refs=BASE["refs"])
    return {"name": name, "steps": [
        {"op": "execute", "input": BASE_INPUT, "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"}),
         "expect": {"entities": {str(NEXT): base_made}}},
        {"op": "execute", "input": inp({"kind": which, "base": UID(NEXT), "to": to}, made_point),
         "result": completed({"created": [UID(NEXT + 1)], "ids": [NEXT + 1], "revision": "$current"}),
         "expect": {"ids": IDS + [NEXT, NEXT + 1], "entities": {str(NEXT + 1): made}, "canUndo": True}},
        {"op": "undo", "returns": "Zincir ölçü" if which == "continue" else "Baz ölçü", "expect": {"ids": IDS + [NEXT]}}]}


def chain_cases():
    out = [
        chained("Zincir ölçü: temelin ikinci noktasından alanın köşesine, temelin doğrusunda; ilk noktası temelinkini izler",
                "continue", vertex(12, 0), (X + 99, Y + 99), "continue"),
        chained("Zincir ölçü: koordinatla verilen nokta serbest; ilk noktası yine temelin ucuna bağlı",
                "continue", at_(X + 16, Y + 2), (X, Y), "continue"),
        chained("Baz ölçü: temelin ilk noktasından, bir kat (değer yüksekliğinin üç katı) dışarıda; ilk noktası temelinkini izler",
                "baseline", vertex(12, 1), (X, Y), "baseline"),
    ]
    return out


def chain_refusals():
    radius = inp({"kind": "radius", "of": edge(14, (X + 55, Y + 30))}, (X + 56, Y + 38))
    cont = lambda base, **extra: inp({"kind": "continue", "base": base, "to": vertex(12, 0)}, (X, Y), **extra)  # noqa: E731
    out = [
        refused("Zincir: temelin kimliği geçersiz", cont(BAD), "invalid_uid", bad_uid(BAD), "measure.base"),
        refused("Zincir: temel çizimde yok", cont(MISSING), "entity_not_found", not_found(MISSING), "measure.base"),
        refused("Zincir: temel ölçü değil (çizgi)", cont(UID(11)), "not_straight", NO_BASE, "measure.base"),
        refused("Zincir ve baz uzaklık almaz", cont(UID(11), offset=2), "invalid_offset",
                "Zincir ve baz ölçü uzaklık almaz; yerini temel ölçü verir. offset'i kaldırın.", "offset"),
    ]
    out.append({"name": "Baz: temel yarıçap ölçüsü; zincir ve baz yalnız hizalı ya da doğrusal ölçüden", "steps": [
        {"op": "execute", "input": radius, "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"})},
        {"op": "execute", "input": inp({"kind": "baseline", "base": UID(NEXT), "to": vertex(12, 0)}, (X, Y)),
         "result": {"status": "failed", "error": {"code": "not_straight", "message": NO_BASE, "path": "measure.base"}},
         "expect": {"ids": IDS + [NEXT]}}]})
    return out


NO_OFFSET = "Yarıçap, çap, koordinat ve kot ölçüsü uzaklık almaz; yerini at verir. offset'i kaldırın."
NO_STRAIGHT = "Nesnenin düz kenarı yok: açı iki düz kenar arasında ölçülür; yaydan ölçmek için angleArc, köşe noktasından angleVertex verin."
NO_ARC = "Nesnenin uçları olan yayı yok: yaydan ya da çoklu çizginin ya da alanın yaylı kenarından ölçülür; tam daire için Yarıçap ya da Çap'ı kullanın."
NO_CIRCLE = "Nesne daire değil: daireden, yaydan ya da çoklu çizginin yay parçasından ölçülür."
NO_DIMENSION = "Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır."
NO_ARC_DIMENSION = "Bu yerde ölçü oluşmuyor; ölçü yayı merkeze ulaşıyor ya da iki nokta aynı yerde."


def refused_cases():
    aligned = {"kind": "aligned", "a": at_(X + 20, Y + 10), "b": at_(X + 26, Y + 18)}
    one = inp(aligned, (X + 20, Y + 20))
    radius = inp({"kind": "radius", "of": edge(14, (X + 55, Y + 30))}, (X + 56, Y + 38))
    out = [
        refused("Yeri sonlu değil", one, "not_finite", not_finite_point("Ölçünün yerinin", "x"), "at.x", non_finite={"at.x": "NaN"}),
        refused("Yazılan uzaklık sonlu değil", {**one, "offset": 1}, "not_finite",
                "Uzaklık sonlu bir sayı değil (NaN ya da sonsuz). Uzaklığı metre olarak sonlu bir sayıyla verin.", "offset",
                non_finite={"offset": "Infinity"}),
        refused("Ölçülen nokta sonlu değil", one, "not_finite", not_finite_point("Ölçülen noktanın", "y"), "measure.b.at.y",
                non_finite={"measure.b.at.y": "NaN"}),
        refused("Doğrultu sonlu değil", inp({"kind": "linear", "a": at_(X, Y), "b": at_(X + 8, Y + 6), "angle": 0}, (X + 4, Y + 9)),
                "not_finite", "Ölçünün doğrultusu sonlu bir sayı değil (NaN ya da sonsuz). Açıyı derece olarak sonlu bir sayıyla verin.",
                "measure.angle", non_finite={"measure.angle": "NaN"}),
        refused("Kenarın tıklandığı yer sonlu değil", radius, "not_finite", not_finite_point("Tıklanan noktanın", "x"),
                "measure.of.near.x", non_finite={"measure.of.near.x": "-Infinity"}),
        refused("Yazı yüksekliği sıfır", {**one, "height": 0}, "invalid_height",
                "Ölçü yazısının yüksekliği sıfırdan büyük, sonlu bir sayı olmalı (metre).", "height"),
        refused("Yerin nesne kimliği geçersiz", inp({**aligned, "a": {"kind": "place", "uid": BAD, "place": {"kind": "start"}}}, (X, Y)),
                "invalid_uid", bad_uid(BAD), "measure.a.uid"),
        refused("Kenarın nesne kimliği geçersiz", inp({"kind": "radius", "of": {"uid": BAD, "near": P(X, Y)}}, (X, Y)),
                "invalid_uid", bad_uid(BAD), "measure.of.uid"),
        refused("Yarıçap uzaklık almaz", {**radius, "offset": 2}, "invalid_offset",
                NO_OFFSET, "offset"),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Kenarın nesnesi çizimde yok", inp({"kind": "radius", "of": {"uid": MISSING, "near": P(X, Y)}}, (X, Y)),
                "entity_not_found", not_found(MISSING), "measure.of.uid"),
        refused("Yerin nesnesi çizimde yok", inp({**aligned, "b": {"kind": "place", "uid": MISSING, "place": {"kind": "end"}}}, (X, Y)),
                "entity_not_found", not_found(MISSING), "measure.b.uid"),
        refused("Çizginin ikinci kenarı yok (along)", inp({**aligned, "b": place(11, "along", part=0, ring=0, index=1, t=0.5)}, (X, Y)),
                "no_such_place",
                "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor.", "measure.b.place"),
        refused("Blok yolu yerleştirme olmayan nesnede (docs/adr/0263 §3)",
                inp({**aligned, "b": {**place(11, "end"), "path": [0]}}, (X, Y)), "piece_gone",
                "Blok yolunda böyle bir parça yok: yolun bir sırası tanımın dışında, yol bir yerleştirmede bitiyor ya da yerleştirme olmayan bir nesneden geçiyor.",
                "measure.b.path"),
        refused("Alanın böyle bir köşesi yok", inp({**aligned, "b": vertex(12, 9)}, (X, Y)), "no_such_place",
                "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor.", "measure.b.place"),
        refused("Çizginin yarıçapı ölçülmez", inp({"kind": "radius", "of": edge(11, (X + 6, Y))}, (X + 6, Y + 5)),
                "not_measurable", NO_CIRCLE, "measure.of"),
        refused("Tam dairenin yay uzunluğu ölçülmez", inp({"kind": "arcLength", "of": edge(14, (X + 55, Y + 30))}, (X + 60, Y + 30)),
                "not_measurable", NO_ARC, "measure.of"),
        refused("Dairenin düz kenarı yok: iki kenar arası açı ölçülmez",
                inp({"kind": "angle", "first": edge(14, (X + 55, Y + 30)), "second": edge(16, (X + 5, Y + 47))}, (X + 8, Y + 44)),
                "not_measurable", NO_STRAIGHT, "measure.first"),
        refused("Eğim: kot yok", inp({"kind": "slope", "a": at_(X, Y + 70), "b": at_(X + 8, Y + 70), "zb": 2}, (X + 4, Y + 71)),
                "missing_elevation",
                "Eğim için noktanın kotu yok: kotu metre olarak verin ya da noktayı kotlu bir köşeye bağlayın.", "measure.za"),
        refused("Bilinmeyen ölçü stili", {**one, "dimensionStyle": "Yok"}, "unknown_style",
                "“Yok” adlı ölçü stili yok. Projenin ölçü stillerinden birinin adını verin ya da alanı kaldırın (Standart).", "dimensionStyle"),
        refused("Noktalar aynı yerde: ölçü oluşmaz", inp({"kind": "aligned", "a": at_(X + 5, Y + 5), "b": at_(X + 5, Y + 5)}, (X + 8, Y + 8)),
                "no_dimension", NO_DIMENSION, "at"),
        refused("Paralel kenarlar arasında açı yok",
                inp({"kind": "angle", "first": edge(15, (X + 9, Y + 40)), "second": edge(17, (X + 9, Y + 50))}, (X + 8, Y + 44)),
                "no_dimension", "Kenarlar paralel; aralarında açı yok.", "measure.second"),
        refused("Kısmi'nin iki noktası aynı yerde",
                inp({"kind": "arcLength", "of": edge(13, (X + 34, Y + 33)),
                     "between": [P(ARC_C[0] + 2, ARC_C[1] + 2), P(ARC_C[0] + 4, ARC_C[1] + 4)]}, (ARC_C[0] + 6, ARC_C[1] + 8)),
                "no_dimension", NO_ARC_DIMENSION, "at"),
        refused("Katman yok", {**one, "layerId": "yok"}, "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Grup katman değildir", {**one, "layerId": "grup"}, "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        refused("Kilitli katmana yazılmaz", {**one, "layerId": "kilitli"}, "layer_locked",
                "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        refused("Kalınlık aralıkta değil", {**one, "lineWeight": 200}, "invalid_line_weight",
                "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"),
    ]
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": radius, "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"})},
        {"op": "execute", "input": {**one, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}}]})
    return out


# ── Merkez işareti, Eksen çizgisi and Kot (docs/adr/0322) ───────────────────────────────────────────────────────────

MARK_LOOK = {"extLineType": "dashdot"}
NO_CIRCLE_MARK = "Nesne daire değil: merkez işareti daireye, yaya ya da çoklu çizginin yay parçasına konur."
NO_AXIS_EDGE = "Nesnenin düz kenarı yok: eksen çizgisi iki düz kenar (çizgi ya da çoklu çizginin, alanın düz kenarı) arasında çizilir."
NO_AXIS = "Eksen çizgisi oluşmuyor: iki kenar aynı ya da eksenin uzunluğu sıfır. İki ayrı düz kenar gösterin."
NEGATIVE_EXTENSION = "Merkez işaretinin ve eksen çizgisinin uzantısı eksi olamaz; 0 ya da daha büyük bir uzaklık verin (metre)."
NO_LEVEL = "Kot için noktanın kotu yok: kotu metre olarak verin ya da noktayı kotlu bir köşeye bağlayın."


def on_edge(eid, index, t):
    """An `along` place on a path's edge `index`."""
    return {"uid": UID(eid), "place": {"kind": "along", "part": 0, "ring": 0, "index": index, "t": plain(t)}}


def axis(first, second):
    """The axis between two edges: the middles of their starts and of their ends, the second turned to run the
    first's way when its direction is against the first's."""
    (p0, p1), (q0, q1) = first, second
    if (p1[0] - p0[0]) * (q1[0] - q0[0]) + (p1[1] - p0[1]) * (q1[1] - q0[1]) < 0:
        q0, q1 = q1, q0
    mid = lambda a, b: ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)  # noqa: E731
    return mid(p0, q0), mid(p1, q1)


def mark_cases():
    out = []
    # Merkez işareti: the centre, its point on the curve east of it (the cross upright), the value height's extension.
    east = (CIRCLE_C[0] + R, CIRCLE_C[1])
    out.append(case("Merkez işareti: dairenin merkezinde; eğrinin üstündeki noktası doğuda, uzantı değer yüksekliği; eğriye bağlı (curve)",
                    inp({"kind": "centerMark", "of": edge(14, (X + 55, Y + 30))}, (X + 70, Y + 70)),
                    lambda i: dimension(i, CIRCLE_C, east, STANDARD_HEIGHT, kind="centerMark", look=MARK_LOOK,
                                        refs={"b": along(14, t_on_arc(CIRCLE_C, 0.0, 2 * math.pi, east)), "rule": "curve"})))
    arc_east = (ARC_C[0] + R, ARC_C[1])
    out.append(case("Merkez işareti: yayın merkezinde, uzantı yazılmış; yayın başına bağlı", 
                    inp({"kind": "centerMark", "of": edge(13, (X + 34, Y + 33))}, (X, Y), offset=2),
                    lambda i: dimension(i, ARC_C, arc_east, 2, kind="centerMark", look=MARK_LOOK,
                                        refs={"b": along(13, t_on_arc(ARC_C, 0.0, ARC_SWEEP, arc_east)), "rule": "curve"})))
    out.append(case("Merkez işareti: ölçü stiliyle; uzantı stilin değer yüksekliği, uzatma çizgileri yine kesik noktalı",
                    inp({"kind": "centerMark", "of": edge(14, (X + 50, Y + 25))}, (X, Y), dimensionStyle="Mimari"),
                    lambda i: dimension(i, CIRCLE_C, east, MIMARI_HEIGHT, kind="centerMark", height=MIMARI_HEIGHT,
                                        look={"dimStyle": MIMARI, **MARK_LOOK},
                                        refs={"b": along(14, t_on_arc(CIRCLE_C, 0.0, 2 * math.pi, east)), "rule": "curve"})))
    out.append(case("Merkez işareti: plan; hiçbir şey yazılmaz", inp({"kind": "centerMark", "of": edge(14, (X + 55, Y + 30))}, (X, Y)),
                    lambda i: dimension(i, CIRCLE_C, east, STANDARD_HEIGHT, kind="centerMark", look=MARK_LOOK,
                                        refs={"b": along(14, 0), "rule": "curve"}), plan=True))
    # Eksen çizgisi between two lines running the same way, and between the opposite edges of a rectangle (the second
    # running the other way); each edge bound where it was clicked (pair).
    a, b = axis(((X, Y + 40), (X + 10, Y + 40)), ((X, Y + 50), (X + 10, Y + 50)))
    out.append(case("Eksen çizgisi: iki paralel çizginin ortasında; kenarlarına bağlı (pair)",
                    inp({"kind": "centerLine", "first": edge(15, (X + 9, Y + 40)), "second": edge(17, (X + 3, Y + 50))}, (X, Y)),
                    lambda i: dimension(i, a, b, STANDARD_HEIGHT, kind="centerLine", look=MARK_LOOK,
                                        refs={"a": along(15, 0.9), "b": along(17, 0.3), "rule": "pair"})))
    a2, b2 = axis(((X + 20, Y), (X + 28, Y)), ((X + 28, Y + 6), (X + 20, Y + 6)))
    out.append(case("Eksen çizgisi: alanın karşı kenarları, ikincisi ters yönde; uzantı yazılmış", 
                    inp({"kind": "centerLine", "first": edge(12, (X + 24, Y)), "second": edge(12, (X + 24, Y + 6))}, (X, Y), offset=0),
                    lambda i: dimension(i, a2, b2, 0, kind="centerLine", look=MARK_LOOK,
                                        refs={"a": on_edge(12, 0, 0.5), "b": on_edge(12, 2, 0.5), "rule": "pair"})))
    a3, b3 = axis(((X, Y + 40), (X + 10, Y + 40)), ((X + 5, Y + 36), (X + 5, Y + 48)))
    out.append(case("Eksen çizgisi: dik kenarlar arasında da uçların ortalarından", 
                    inp({"kind": "centerLine", "first": edge(15, (X + 5, Y + 40)), "second": edge(16, (X + 5, Y + 42))}, (X, Y)),
                    lambda i: dimension(i, a3, b3, STANDARD_HEIGHT, kind="centerLine", look=MARK_LOOK,
                                        refs={"a": along(15, 0.5), "b": along(16, 0.5), "rule": "pair"}), undo=True))
    # Kot: the elevation of a point object, of a line's end, given; a given one wins over the place's.
    out.append(case("Kot: nokta nesnesinin kotu; değer imleçte, noktaya bağlı", 
                    inp({"kind": "elevation", "a": place(10, "node", part=0)}, (X + 4, Y + 22)),
                    lambda i: dimension(i, (X + 2, Y + 20), (X + 4, Y + 22), 0, kind="elevation", za=50,
                                        refs={"a": {"uid": UID(10), "place": {"kind": "node", "part": 0}}})))
    out.append(case("Kot: çizginin başının kotu", inp({"kind": "elevation", "a": place(11, "start")}, (X - 2, Y + 3)),
                    lambda i: dimension(i, LINE_11[0], (X - 2, Y + 3), 0, kind="elevation", za=100,
                                        refs={"a": ref_of(place(11, "start"))})))
    out.append(case("Kot: yazılan kot yerin kotunun yerine", inp({"kind": "elevation", "a": place(11, "end"), "z": 7.5}, (X + 14, Y + 3)),
                    lambda i: dimension(i, LINE_11[1], (X + 14, Y + 3), 0, kind="elevation", za=7.5,
                                        refs={"a": ref_of(place(11, "end"))})))
    out.append(case("Kot: koordinatla ve yazılan kotla; bağsız", inp({"kind": "elevation", "a": at_(X + 40, Y + 60), "z": -1.25}, (X + 43, Y + 62)),
                    lambda i: dimension(i, (X + 40, Y + 60), (X + 43, Y + 62), 0, kind="elevation", za=-1.25)))
    return out


def mark_refusals():
    mark = lambda of, **extra: inp({"kind": "centerMark", "of": of}, (X, Y), **extra)  # noqa: E731
    line = lambda first, second, **extra: inp({"kind": "centerLine", "first": first, "second": second}, (X, Y), **extra)  # noqa: E731
    level = lambda a, **extra: inp({"kind": "elevation", "a": a, **extra}, (X + 1, Y + 1))  # noqa: E731
    return [
        refused("Merkez işareti: çizginin merkezi yok", mark(edge(11, (X + 6, Y))), "not_measurable", NO_CIRCLE_MARK, "measure.of"),
        refused("Merkez işareti: yaysız alanın merkezi yok", mark(edge(12, (X + 24, Y))), "not_measurable", NO_CIRCLE_MARK, "measure.of"),
        refused("Merkez işaretinin uzantısı eksi olamaz", mark(edge(14, (X + 55, Y + 30)), offset=-1), "invalid_offset",
                NEGATIVE_EXTENSION, "offset"),
        refused("Eksen çizgisi: dairenin düz kenarı yok",
                line(edge(14, (X + 55, Y + 30)), edge(15, (X + 5, Y + 40))), "not_measurable", NO_AXIS_EDGE, "measure.first"),
        refused("Eksen çizgisi: aynı kenar iki kez", line(edge(15, (X + 2, Y + 40)), edge(15, (X + 8, Y + 40))), "no_dimension", NO_AXIS, "at"),
        refused("Eksen çizgisinin uzantısı eksi olamaz", line(edge(15, (X + 2, Y + 40)), edge(17, (X + 8, Y + 50)), offset=-0.5),
                "invalid_offset", NEGATIVE_EXTENSION, "offset"),
        refused("Kot: koordinatın kotu yok", level(at_(X + 40, Y + 60)), "missing_elevation", NO_LEVEL, "measure.z"),
        refused("Kot: alanın köşesinin kotu yok", level(vertex(12, 1)), "missing_elevation", NO_LEVEL, "measure.z"),
        refused("Kot sonlu değil", level(at_(X + 40, Y + 60), z=1), "not_finite",
                "Kot sonlu bir sayı değil (NaN ya da sonsuz). Kotu metre olarak sonlu bir sayıyla verin.", "measure.z",
                non_finite={"measure.z": "NaN"}),
        refused("Kot uzaklık almaz", level(at_(X + 40, Y + 60), z=1) | {"offset": 1}, "invalid_offset", NO_OFFSET, "offset"),
    ]


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.dimensions.create", "commandVersion": 1, "title": "Ölçü ekle",
        "note": "Written by scripts/fixtures/dimension_create_cases.py from Ölçülendirme's definitions (docs/adr/0061, 0147, 0183, 0205, 0249, 0255, 0320, 0322); do not edit by hand.",
        "setup": SETUP, "cases": written_cases() + chain_cases() + mark_cases() + refused_cases() + chain_refusals() + mark_refusals(),
    }


def main() -> int:
    doc = build()
    assert_no_negative_zero(doc, "cad.dimensions.create")
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = OUT / "cad.dimensions.create.json"
    if "--check" in sys.argv[1:]:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.dimensions.create durumları güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
