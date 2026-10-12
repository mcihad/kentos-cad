"""The shared cases of the product command cad.entities.offset (docs/adr/0246).

    python3 scripts/fixtures/offset_command_cases.py           # writes the file
    python3 scripts/fixtures/offset_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.offset.json. The checks, their
order, codes, paths and messages are written here by hand from the ADR. The
copies are computed independently, from the offset's definition (ADR 0047,
0246), without KentOS code:

- a line or a path: every straight edge moved `d` along its unit normal to
  the side asked for (the side of a point is judged at the edge nearest to
  it, by the sign of the cross product: left of travel is +), consecutive
  moved edges joined where their lines cross (a right angle is well within
  the mitre limit); an open path keeps its first and last moved ends;
- an arc edge of a path keeps its centre and turning, its radius r − d to
  its left (r + d to its right), its ends moved along their radii; its bulge
  is tan(θ/4) of the new sweep θ, counter-clockwise from the start's
  direction to the end's about the centre;
- a circle or an arc: radius + d when the point is outside, − d inside;
  none left at or below zero;
- Noktadan geç: the distance is the point's from the object's nearest edge.

The straight geometry is computed in exact fractions. The drawing is chosen
so that every value is a double exactly (axis-parallel edges whose lengths
are powers of two, distances in halves and quarters): the script asserts it,
so any implementation that computes the definition without an error larger
than half an ulp must meet the file bit for bit, and the shared core does.
The one transcendental value, an arc's new bulge, is tan(π/4) of a sweep
that is exactly the double π: Python's libm and the core's (fdlibm's) give
0.9999999999999999 alike.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys
from fractions import Fraction as F

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def group(i, name, children, visible=True, locked=False):
    return {"id": i, "name": name, "type": "group", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": children}


def P(x, y):
    return {"x": x, "y": y}


HALF_PI = math.pi / 2

ENTITIES = [
    {"kind": "point", "id": 4, "layerId": "yapi", "attrs": {"Ad": "N1"}, "label": "N1", "p": P(487001, 4420001), "z": 12.5},
    {"kind": "line", "id": 10, "layerId": "yapi", "attrs": {"Tür": "Duvar"}, "color": "#E5484D", "lineWeight": 0.7, "a": P(487010, 4420010), "b": P(487026, 4420010)},
    {"kind": "arc", "id": 11, "layerId": "yapi", "attrs": {}, "c": P(487020, 4420020), "r": 4, "a0": 0, "a1": HALF_PI},
    {"kind": "line", "id": 12, "layerId": "kilitli", "attrs": {}, "a": P(487005, 4420005), "b": P(487015, 4420005)},
    {"kind": "circle", "id": 13, "layerId": "yapi", "attrs": {}, "c": P(487030, 4420010), "r": 2.5},
    {"kind": "text", "id": 14, "layerId": "yapi", "attrs": {}, "p": P(487005, 4420025), "text": "Ada 101", "height": 2, "rotation": 0},
    {"kind": "line", "id": 15, "layerId": "eski", "attrs": {}, "a": P(487005, 4420010), "b": P(487015, 4420010)},
    {"kind": "polyline", "id": 16, "layerId": "yapi", "attrs": {"Tür": "Bordür"}, "pts": [P(487000, 4420050), P(487016, 4420050), P(487016, 4420066)], "zs": [10.5, 11, 12.25]},
    {"kind": "polygon", "id": 17, "layerId": "sinir", "attrs": {"Ada": "101"}, "symbol": "parsel", "pts": [P(487040, 4420040), P(487048, 4420040), P(487048, 4420048), P(487040, 4420048)]},
    {"kind": "polyline", "id": 18, "layerId": "yapi", "attrs": {}, "pts": [P(487000, 4420080), P(487008, 4420080), P(487008, 4420088)], "bulges": [0, 1, 0]},
    {"kind": "line", "id": 20, "layerId": "gizli", "attrs": {}, "a": P(487005, 4420015), "b": P(487015, 4420015)},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = 21

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Ürün komutu",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 487000, "y": 4420000},
    "layers": [
        layer("yapi", "Yapı"),
        layer("kilitli", "Kilitli katman", locked=True),
        layer("gizli", "Gizli katman", visible=False),
        group("pafta", "Pafta", [layer("sinir", "Sınır")]),
        group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
    ],
    "activeLayer": "yapi",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# ── The offset, from its definition ───────────────────────────────────────


def exact(v):
    """A fraction that is exactly a double, as that double (the drawing keeps every value so)."""
    f = float(v)
    assert F(f) == v, f"{v} is not a double: pick other numbers"
    return int(f) if f == int(f) and isinstance(v, int) else f


def pt(p):
    return (F(p["x"]), F(p["y"]))


def J(p):
    return P(exact(p[0]), exact(p[1]))


def cross(a, b, p):
    return (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])


def seg_distance2(a, b, p):
    """Squared distance from p to the segment ab, exactly."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
    t = max(F(0), min(F(1), t))
    qx, qy = a[0] + t * dx, a[1] + t * dy
    return (p[0] - qx) ** 2 + (p[1] - qy) ** 2


def side(pts, closed, p):
    """+1 when p is left of travel at the nearest edge, −1 right; the first of equally near edges decides."""
    n = len(pts)
    edges = [(pts[i], pts[(i + 1) % n]) for i in range(n if closed else n - 1)]
    best = min(range(len(edges)), key=lambda i: (seg_distance2(*edges[i], p), i))
    a, b = edges[best]
    return 1 if cross(a, b, p) >= 0 else -1


def unit_normal(a, b):
    """The left unit normal of an axis-parallel edge (its length then exact)."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    assert dx == 0 or dy == 0, "the cases keep edges axis-parallel"
    length = abs(dx) + abs(dy)
    return (-dy / length, dx / length)


def meet(s1, s2):
    """Where the lines through two moved edges cross."""
    (a, b), (c, d) = s1, s2
    rx, ry = b[0] - a[0], b[1] - a[1]
    sx, sy = d[0] - c[0], d[1] - c[1]
    den = rx * sy - ry * sx
    if den == 0:
        return c
    t = ((c[0] - a[0]) * sy - (c[1] - a[1]) * sx) / den
    return (a[0] + t * rx, a[1] + t * ry)


def offset_path(pts, d, closed):
    n = len(pts)
    segs = []
    for i in range(n if closed else n - 1):
        a, b = pts[i], pts[(i + 1) % n]
        nx, ny = unit_normal(a, b)
        segs.append(((a[0] + nx * d, a[1] + ny * d), (b[0] + nx * d, b[1] + ny * d)))
    m = len(segs)
    if closed:
        return [meet(segs[(i - 1) % m], segs[i]) for i in range(m)]
    return [segs[0][0]] + [meet(segs[i - 1], segs[i]) for i in range(1, m)] + [segs[-1][1]]


def copy_base(src, layer=None):
    """A copy's own fields: its source's layer (or the one asked for), colour and line weight; nothing else."""
    out = {"kind": src["kind"], "id": 0, "layerId": layer or src["layerId"], "attrs": {}}
    for k in ("color", "lineWeight"):
        if k in src:
            out[k] = src[k]
    return out


def offset(i, d, at, layer=None):
    """The copy of object i at distance d on the side of the point `at` (fractions); None when none is made."""
    e = BY_ID[i]
    base = copy_base(e, layer)
    p = (F(at[0]), F(at[1]))
    d = F(d)
    if e["kind"] in ("circle", "arc"):
        c = pt(e["c"])
        outside = (p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2 > F(e["r"]) ** 2
        r = F(e["r"]) + (d if outside else -d)
        if r <= 0:
            return None
        out = {**base, "c": J(c), "r": exact(r)}
        if e["kind"] == "arc":
            out.update({"a0": e["a0"], "a1": e["a1"]})
        return out
    if e["kind"] == "line":
        pts = [pt(e["a"]), pt(e["b"])]
        s = side(pts, False, p)
        a, b = offset_path(pts, s * d, False)
        return {**base, "a": J(a), "b": J(b)}
    closed = e["kind"] == "polygon"
    pts = [pt(q) for q in e["pts"]]
    if "bulges" in e:
        return offset_bulged(e, base, pts, d, p)
    s = side(pts, closed, p)
    out = {**base, "pts": [J(q) for q in offset_path(pts, s * d, closed)]}
    if "zs" in e:
        # Ötele's copy has a vertex for each of its source's: each takes the elevation in its place (ADR 0142).
        out["zs"] = e["zs"]
    return out


def offset_bulged(e, base, pts, d, p):
    """Object 18: a straight edge east, then a half circle counter-clockwise (bulge 1) about (8, 84) from the
    origin's (487000, 4420000), tangent to it. Left of travel is the circle's inside."""
    assert e["bulges"] == [0, 1, 0]
    a, b, c_end = pts
    c = ((b[0] + c_end[0]) / 2, (b[1] + c_end[1]) / 2)  # bulge 1: the chord's middle is the centre
    r = abs(c_end[1] - b[1]) / 2
    # The side: the cases' points lie west of the centre, within 2 m of the straight edge; the half circle bulges
    # east, so its nearest point to them is an end, farther than 2 m: the straight edge decides.
    assert p[0] < c[0] - 2 and seg_distance2(a, b, p) <= 4
    s = 1 if cross(a, b, p) >= 0 else -1
    d = s * d
    nx, ny = unit_normal(a, b)
    a1, b1 = (a[0] + nx * d, a[1] + ny * d), (b[0] + nx * d, b[1] + ny * d)
    r2 = r - d  # counter-clockwise: left is the centre's side
    assert r2 > 0
    k = r2 / r
    s2 = (c[0] + (b[0] - c[0]) * k, c[1] + (b[1] - c[1]) * k)
    e2 = (c[0] + (c_end[0] - c[0]) * k, c[1] + (c_end[1] - c[1]) * k)
    assert s2 == b1, "the arc stays tangent: the straight edge and the arc meet without a join"
    start = math.atan2(float(s2[1] - c[1]), float(s2[0] - c[0]))
    end = math.atan2(float(e2[1] - c[1]), float(e2[0] - c[0]))
    sweep = (end - start) % (2 * math.pi)
    bulge = math.tan(sweep / 4)
    return {**base, "pts": [J(a1), J(b1), J(e2)], "bulges": [0, bulge, 0]}


def through(i, at):
    """Noktadan geç's distance for object i: the point's from the object's nearest edge (lines and arcs here)."""
    e = BY_ID[i]
    p = (F(at[0]), F(at[1]))
    if e["kind"] == "line":
        d2 = seg_distance2(pt(e["a"]), pt(e["b"]), p)
    elif e["kind"] == "arc":
        c = pt(e["c"])
        # The cases' points lie along the arc's end radius at a quarter turn: straight above the centre.
        assert p[0] == c[0] and e["a1"] == HALF_PI
        d2 = (p[1] - c[1] - F(e["r"])) ** 2
    else:
        raise ValueError(e["kind"])
    # The distances are whole quarters: the square root is exact.
    for q in range(0, 4000):
        if F(q, 4) ** 2 == d2:
            return F(q, 4)
    raise AssertionError("pick a point at a quarter-metre distance")


# ── Answers ──────────────────────────────────────────────────────────────

AXIS = {"x": "doğu (Y)", "y": "kuzey (X)"}


def NFC(whose, axis):
    return f"{whose} {AXIS[axis]} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def NFV(what, fix):
    return f"{what} sonlu bir sayı değil (NaN ya da sonsuz). {fix}"


CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
NOTHING = "Ötelenecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin."
DISTANCE = "Öteleme mesafesi sıfırdan büyük olmalı. Pozitif bir mesafe (metre) verin."
NOT_OFFSETTABLE = "Yalnızca çizgi, çoklu çizgi, kapalı alan, daire, yay, elips ve yardımcı çizgiler ötelenebilir."
SHRINKS = "Yarıçap sıfırın altına düşüyor; daha küçük bir mesafe girin."
ON_OBJECT = "Nokta nesnenin üzerinde; iki yana ötelemek için bir yanına tıklayın."
ZERO = "Öteleme mesafesi sıfırdan büyük olmalı."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"


def BADREV(r):
    return f"Beklenen sürüm “{r}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."


def BADUID(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."


def NOTFOUND(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def LOCKED(name):
    return f"“{name}” katmanı kilitli; üzerindeki nesne ötelenemez. Kilidi Katmanlar panelinden açın."


def U(i):
    return f"$uidOf:{i}"


def num(v):
    return exact(F(v))


def D(distance, x, y):
    return {"kind": "distance", "distance": distance, "side": P(x, y)}


def T(x, y):
    return {"kind": "through", "point": P(x, y)}


def inp(uids, by, **kw):
    out = {"uids": uids, "by": by}
    names = {"both": "bothSides", "delete": "deleteSource", "layer": "layerId", "rev": "expectedRevision"}
    for k, v in kw.items():
        out[names[k]] = v
    return out


def slotted(made, first=NEXT):
    return [(first + k, {**e, "id": first + k}) for k, e in enumerate(made)]


def done(made, distances, removed=(), warnings=()):
    return {"status": "completed", "output": {"created": [U(s) for s, _ in made], "removed": list(removed),
                                              "distances": [num(d) for d in distances], "revision": "$current"},
            "warnings": list(warnings)}


def valid(warnings=()):
    return {"status": "completed", "output": None, "warnings": list(warnings)}


def planned(sources, made, distances, removed=(), warnings=()):
    return {"status": "completed", "output": {"sources": list(sources), "entities": list(made), "removed": list(removed),
                                              "distances": [num(d) for d in distances], "revision": "$current"},
            "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def conflict():
    return {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}


def warn(code, message, path):
    return {"code": code, "message": message, "path": path}


def written(made, removed=(), extra=None):
    """What the document holds after a write: the objects less those deleted, and the copies."""
    gone = set(removed)
    out = {"ids": [i for i in IDS if i not in gone] + [s for s, _ in made], "entities": {str(s): e for s, e in made}, "revision": "changed"}
    if extra:
        out.update(extra)
    return out


untouched = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}

cases = []

# ── Lines ────────────────────────────────────────────────────────────────

made = slotted([offset(10, F(1, 2), (487018, 4420013))])
assert made[0][1]["a"] == P(487010, 4420010.5)
cases.append({
    "name": "çizgi kuzeye 0,5 m: kopya yeni bir nesnedir; katmanını, rengini ve çizgi kalınlığını alır, özniteliklerini almaz; tek adım Ötele, geri alınır ve yinelenir",
    "steps": [
        {"op": "captureUid", "id": 10, "as": "cizgi"},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013)), "result": done(made, [0.5]),
         "expect": written(made, extra={"uids": {"10": "cizgi", "21": "new"}, "canUndo": True, "canRedo": False, "dirty": True})},
        {"op": "undo", "returns": "Ötele", "note": "Ötele aracının adımı.", "expect": {"ids": IDS, "entities": {"10": BY_ID[10]}, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Ötele", "expect": {"ids": IDS + [21], "canUndo": True, "canRedo": False}},
    ],
})

made = slotted([offset(10, F(1, 4), (487030, 4420009))])
cases.append({
    "name": "taraf, noktaya en yakın kenarın yanıdır: çizginin ucunun ötesindeki güneydeki nokta, kopyayı güneye 0,25 m koyar",
    "steps": [{"op": "execute", "input": inp([U(10)], D(0.25, 487030, 4420009)), "result": done(made, [0.25]), "expect": written(made)}],
})

dist = through(10, (487015, F(4420012) + F(1, 4)))
made = slotted([offset(10, dist, (487015, F(4420012) + F(1, 4)))])
cases.append({
    "name": "Noktadan geç: kopya noktadan geçer; mesafe noktanın çizgiye uzaklığıdır (2,25 m) ve çıktıda söylenir",
    "steps": [{"op": "execute", "input": inp([U(10)], T(487015, 4420012.25)), "result": done(made, [dist]), "expect": written(made)}],
})

# ── Polylines and areas ──────────────────────────────────────────────────

made = slotted([offset(16, F(1, 2), (487008, 4420049))])
assert made[0][1]["pts"][1] == P(487016.5, 4420049.5)
cases.append({
    "name": "çoklu çizgi dışa 0,5 m: kenarlar kaydırılır, köşe kaydırılmış kenarların kesiştiği yerdir; köşeler kaynağın köşelerinin kotlarını taşır",
    "steps": [{"op": "execute", "input": inp([U(16)], D(0.5, 487008, 4420049)), "result": done(made, [0.5]), "expect": written(made)}],
})

made = slotted([offset(16, 1, (487012, 4420051))])
cases.append({
    "name": "çoklu çizgi içe 1 m: noktaya en yakın kenar (ilk kenar, 1 m) yanı söyler",
    "steps": [{"op": "execute", "input": inp([U(16)], D(1, 487012, 4420051)), "result": done(made, [1]), "expect": written(made)}],
})

made = slotted([offset(17, F(1, 2), (487050, 4420044))])
cases.append({
    "name": "kapalı alan dışa 0,5 m: her köşe kaydırılmış iki kenarın kesişimidir; kopya simgesini ve özniteliklerini almaz",
    "steps": [{"op": "execute", "input": inp([U(17)], D(0.5, 487050, 4420044)), "result": done(made, [0.5]), "expect": written(made)}],
})

made = slotted([offset(17, 1, (487044, F(4420044) + F(1, 2)))])
cases.append({
    "name": "kapalı alan içe 1 m: içerideki noktaya en yakın kenar üst kenardır",
    "steps": [{"op": "execute", "input": inp([U(17)], D(1, 487044, 4420044.5)), "result": done(made, [1]), "expect": written(made)}],
})

made = slotted([offset(18, F(1, 2), (487004, 4420082))])
assert made[0][1]["bulges"][1] == math.tan(math.pi / 4)
cases.append({
    "name": "yaylı çoklu çizgi içe 0,5 m: düz kenar kayar, yarım daire aynı merkezle 3,5 m yarıçapa küçülür, teğet kalır",
    "steps": [{"op": "execute", "input": inp([U(18)], D(0.5, 487004, 4420082)), "result": done(made, [0.5]), "expect": written(made)}],
})

made = slotted([offset(18, F(1, 2), (487004, 4420082)), offset(18, F(1, 2), (487004, F(4420079)))])
cases.append({
    "name": "yaylı çoklu çizgi iki yana 0,5 m: önce noktanın yanı (içe, 3,5 m), sonra öbür yanı (dışa, 4,5 m)",
    "steps": [{"op": "execute", "input": inp([U(18)], D(0.5, 487004, 4420082), both=True), "result": done(made, [0.5]), "expect": written(made)}],
})

# ── Circles and arcs ─────────────────────────────────────────────────────

made = slotted([offset(11, F(1, 2), (487030, 4420020))])
cases.append({
    "name": "yay dışa 0,5 m: aynı merkez ve açılar, yarıçap 4,5 m",
    "steps": [{"op": "execute", "input": inp([U(11)], D(0.5, 487030, 4420020)), "result": done(made, [0.5]), "expect": written(made)}],
})

dist = through(11, (487020, 4420026))
made = slotted([offset(11, dist, (487020, 4420026))])
cases.append({
    "name": "yay Noktadan geç: nokta yayın ucunun 2 m ötesinde; yarıçap 6 m",
    "steps": [{"op": "execute", "input": inp([U(11)], T(487020, 4420026)), "result": done(made, [dist]), "expect": written(made)}],
})

made = slotted([offset(13, F(1, 2), (487030, 4420011))])
assert made[0][1]["r"] == 2
cases.append({
    "name": "daire içe 0,5 m: nokta dairenin içinde, yarıçap 2 m",
    "steps": [{"op": "execute", "input": inp([U(13)], D(0.5, 487030, 4420011)), "result": done(made, [0.5]), "expect": written(made)}],
})

made = slotted([offset(13, 3, (487034, 4420010))])
assert offset(13, 3, (487032, 4420010)) is None
cases.append({
    "name": "daire iki yana 3 m: dışa kopya yapılır, içe kopya yarıçapı sıfırın altına düşürdüğü için offset_failed uyarısıyla atlanır",
    "steps": [{"op": "execute", "input": inp([U(13)], D(3, 487034, 4420010), both=True),
               "result": done(made, [3], warnings=[warn("offset_failed", SHRINKS, "uids[0]")]), "expect": written(made)}],
})

cases.append({
    "name": "hiçbir kopya yapılamazsa hiçbir şey yazılmaz: offset_failed, çekirdeğin sözüyle",
    "steps": [
        {"op": "execute", "input": inp([U(13)], D(3, 487030, 4420011)), "result": failed("offset_failed", SHRINKS, "uids"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], T(487020, 4420010)), "note": "Nokta çizginin üzerinde: mesafe 0.",
         "result": failed("offset_failed", ZERO, "uids"), "expect": untouched},
    ],
})

made = slotted([offset(10, 1, (487018, 4420010))])
cases.append({
    "name": "iki yana, nokta nesnenin üzerinde: noktanın yanı çizginin solu sayılır; öbür yanın kopyası uyarıyla atlanır",
    "steps": [{"op": "execute", "input": inp([U(10)], D(1, 487018, 4420010), both=True),
               "result": done(made, [1], warnings=[warn("offset_failed", ON_OBJECT, "uids[0]")]), "expect": written(made)}],
})

# ── Both sides, deleting, the target layer ──────────────────────────────

made = slotted([offset(10, 1, (487018, 4420013)), offset(10, 1, (487018, 4420009))])
cases.append({
    "name": "çizgi iki yana 1 m: önce noktanın yanı (kuzey), sonra öbür yanı; tek adım",
    "steps": [
        {"op": "execute", "input": inp([U(10)], D(1, 487018, 4420013), both=True), "result": done(made, [1]), "expect": written(made)},
        {"op": "undo", "returns": "Ötele", "expect": {"ids": IDS}},
    ],
})

made = slotted([offset(10, F(1, 2), (487018, 4420013))])
cases.append({
    "name": "Kaynağı sil: kaynak aynı adımda silinir; geri alma onu geri getirir",
    "steps": [
        {"op": "captureUid", "id": 10, "as": "cizgi"},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), delete=True), "result": done(made, [0.5], removed=["$uid:cizgi"]),
         "expect": written(made, removed=[10])},
        {"op": "undo", "returns": "Ötele", "expect": {"ids": IDS, "entities": {"10": BY_ID[10]}, "uids": {"10": "cizgi"}}},
    ],
})

made = slotted([offset(17, F(1, 2), (487050, 4420044)), offset(17, F(1, 2), (487047, 4420044))])
cases.append({
    "name": "kapalı alan iki yana ve Kaynağı sil: iki kopya ve kaynağın silinmesi tek adımdır",
    "steps": [
        {"op": "captureUid", "id": 17, "as": "alan"},
        {"op": "execute", "input": inp([U(17)], D(0.5, 487050, 4420044), both=True, delete=True), "result": done(made, [0.5], removed=["$uid:alan"]),
         "expect": written(made, removed=[17])},
    ],
})

made = slotted([offset(10, F(1, 2), (487018, 4420013), layer="sinir")])
# An undone object keeps its slot for the redo: the next copy takes the one after it.
hidden = slotted([offset(10, F(1, 2), (487018, 4420013), layer="gizli")], first=NEXT + 1)
cases.append({
    "name": "kopyaların katmanı: layerId verilince kopya oraya gider; gizli katmana uyarıyla",
    "steps": [
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), layer="sinir"), "result": done(made, [0.5]), "expect": written(made)},
        {"op": "undo", "returns": "Ötele"},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), layer="gizli"),
         "result": done(hidden, [0.5], warnings=[warn("layer_hidden", "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "layerId")]),
         "expect": {"ids": IDS + [NEXT + 1], "entities": {str(NEXT + 1): hidden[0][1]}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "kopyaların katmanı çizimde olmalı, katman olmalı ve kilitli olmamalı",
    "steps": [
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), layer="yok"),
         "result": failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), layer="pafta"),
         "result": failed("not_a_layer", "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), layer="eski"),
         "result": failed("layer_locked", "“Eski” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"), "expect": untouched},
    ],
})

made = slotted([offset(10, F(1, 2), (487020, 4420012)), offset(13, F(1, 2), (487020, 4420012))])
cases.append({
    "name": "birden çok nesne: girdinin sırasıyla, her biri noktanın kendine göre yanına; tekrarlanan kimlik bir kez",
    "steps": [{"op": "execute", "input": inp([U(10), U(13), U(10)], D(0.5, 487020, 4420012)), "result": done(made, [0.5, 0.5]), "expect": written(made)}],
})

made = slotted([offset(20, F(1, 2), (487010, 4420016))])
cases.append({
    "name": "gizli katmandaki nesne de ötelenir; kopya onun katmanına gider, uyarı yok",
    "steps": [{"op": "execute", "input": inp([U(20)], D(0.5, 487010, 4420016)), "result": done(made, [0.5]), "expect": written(made)}],
})

# ── Refusals, in the contract's order ──────────────────────────────────

cases.append({
    "name": "kilitli katmandaki nesne ötelenmez, kilitli grubunki de: hiçbir şey yazılmaz; ilk kilitli nesne söylenir",
    "steps": [
        {"op": "execute", "input": inp([U(10), U(12)], D(0.5, 487018, 4420013)), "result": failed("layer_locked", LOCKED("Kilitli katman"), "uids[1]"), "expect": untouched},
        {"op": "execute", "input": inp([U(15)], D(0.5, 487018, 4420013)), "result": failed("layer_locked", LOCKED("Eski"), "uids[0]"), "expect": untouched},
    ],
})

cases.append({
    "name": "ötelenmeyen tür: not_offsettable; nesne nesne, kilitten sonra",
    "steps": [
        {"op": "execute", "input": inp([U(10), U(14)], D(0.5, 487018, 4420013)), "result": failed("not_offsettable", NOT_OFFSETTABLE, "uids[1]"), "expect": untouched},
        {"op": "execute", "input": inp([U(4), U(12)], D(0.5, 487018, 4420013)), "result": failed("not_offsettable", NOT_OFFSETTABLE, "uids[0]"), "expect": untouched},
        {"op": "execute", "input": inp([U(12), U(4)], D(0.5, 487018, 4420013)), "result": failed("layer_locked", LOCKED("Kilitli katman"), "uids[0]"), "expect": untouched},
    ],
})

cases.append({
    "name": "nesne verilmedi, bozuk kimlik, çizimde olmayan kimlik",
    "steps": [
        {"op": "execute", "input": inp([], D(0.5, 487018, 4420013)), "result": failed("no_entities", NOTHING, "uids"), "expect": untouched},
        {"op": "validate", "input": inp([U(10), "ABC"], D(0.5, 487018, 4420013)), "result": failed("invalid_uid", BADUID("ABC"), "uids[1]"), "expect": untouched},
        {"op": "execute", "input": inp([U(10), MISSING], D(0.5, 487018, 4420013)), "result": failed("entity_not_found", NOTFOUND(MISSING), "uids[1]"), "expect": untouched},
    ],
})

cases.append({
    "name": "sayılar sonlu olmalı: önce mesafe, sonra noktanın doğusu ve kuzeyi; mesafe sıfırdan büyük olmalı",
    "steps": [
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013)), "nonFinite": {"by.distance": "NaN", "by.side.x": "Infinity"},
         "result": failed("not_finite", NFV("Öteleme mesafesi", "Mesafeyi sonlu bir sayıyla verin."), "by.distance"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013)), "nonFinite": {"by.side.x": "-Infinity"},
         "result": failed("not_finite", NFC("Taraf noktasının", "x"), "by.side.x"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], T(487018, 4420013)), "nonFinite": {"by.point.y": "NaN"},
         "result": failed("not_finite", NFC("Geçilecek noktanın", "y"), "by.point.y"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], D(0, 487018, 4420013)), "result": failed("invalid_distance", DISTANCE, "by.distance"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], D(-1, 487018, 4420013)), "result": failed("invalid_distance", DISTANCE, "by.distance"), "expect": untouched},
        {"op": "validate", "input": inp([MISSING], D(0, 487018, 4420013)), "note": "Girdinin hatası nesneden önce.",
         "result": failed("invalid_distance", DISTANCE, "by.distance"), "expect": untouched},
    ],
})

made = slotted([offset(10, F(1, 2), (487018, 4420013))])
cases.append({
    "name": "beklenen sürüm ondalık bir tamsayı yazısıdır; çizimin sürümüyse yazılır",
    "steps": [
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), rev="01"), "result": failed("invalid_revision", BADREV("01"), "expectedRevision"), "expect": untouched},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), rev="$current"), "result": done(made, [0.5]), "expect": written(made)},
    ],
})

cases.append({
    "name": "çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; çakışma nesneden ve kilitten önce",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": inp([U(13)], D(0.5, 487030, 4420011)), "result": done(slotted([offset(13, F(1, 2), (487030, 4420011))]), [0.5])},
        {"op": "execute", "input": inp([U(10)], D(0.5, 487018, 4420013), rev="$r0"), "result": conflict(), "expect": {"ids": IDS + [NEXT], "revision": "same"}},
        {"op": "execute", "input": inp([MISSING], D(0.5, 487018, 4420013), rev="$r0"), "result": conflict(), "expect": {"revision": "same"}},
        {"op": "execute", "input": inp([U(12)], D(0.5, 487018, 4420013), rev="$r0"), "result": conflict(), "expect": {"revision": "same"}},
    ],
})

copies = [offset(10, 1, (487018, 4420013)), offset(10, 1, (487018, 4420009))]
made = slotted(copies)
cases.append({
    "name": "doğrulama hiçbir şey yazmaz; plan kopyaları yuvaları 0 olarak, silinecekleri ve mesafeleri gösterir; yazma planın sürümüyle planı yazar",
    "steps": [
        {"op": "captureUid", "id": 10, "as": "cizgi"},
        {"op": "validate", "input": inp([U(10)], D(1, 487018, 4420013), both=True, delete=True), "result": valid(), "expect": untouched},
        {"op": "plan", "input": inp([U(10)], D(1, 487018, 4420013), both=True, delete=True),
         "result": planned(["$uid:cizgi"], copies, [1], removed=["$uid:cizgi"]), "expect": untouched},
        {"op": "captureRevision", "as": "plan"},
        {"op": "execute", "input": inp([U(10)], D(1, 487018, 4420013), both=True, delete=True, rev="$plan"),
         "result": done(made, [1], removed=["$uid:cizgi"]), "expect": written(made, removed=[10])},
    ],
})

cases.append({
    "name": "planın uyarıları yazmanınkilerdir; plan ve doğrulama geçmişe dokunmaz",
    "steps": [
        {"op": "execute", "input": inp([U(13)], D(0.5, 487030, 4420011)), "result": done(slotted([offset(13, F(1, 2), (487030, 4420011))]), [0.5])},
        {"op": "undo", "returns": "Ötele"},
        {"op": "plan", "input": inp([U(13)], D(3, 487034, 4420010), both=True),
         "result": planned([U(13)], [offset(13, 3, (487034, 4420010))], [3], warnings=[warn("offset_failed", SHRINKS, "uids[0]")]),
         "expect": {"canRedo": True, "revision": "same"}},
        {"op": "validate", "input": inp([U(13)], D(3, 487034, 4420010), both=True), "result": valid([warn("offset_failed", SHRINKS, "uids[0]")]),
         "expect": {"canRedo": True, "revision": "same"}},
        {"op": "redo", "returns": "Ötele"},
    ],
})

assert len(cases) >= 20, len(cases)
for c in cases:
    assert_no_negative_zero(c, c["name"])


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def write(command, title, note, cases):
    fixture = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note, "setup": SETUP, "cases": cases}
    out = ["{"]
    for key in ["format", "version", "command", "commandVersion", "title", "note"]:
        out.append(f'  "{key}": {compact(fixture[key])},')
    s = fixture["setup"]
    out.append('  "setup": {')
    for key in ["format", "version", "name", "settings", "origin"]:
        out.append(f'    "{key}": {compact(s[key])},')
    out.append('    "layers": [')
    out.append(",\n".join(f"      {compact(item)}" for item in s["layers"]))
    out.append("    ],")
    out.append(f'    "activeLayer": {compact(s["activeLayer"])},')
    out.append('    "entities": [')
    out.append(",\n".join(f"      {compact(e)}" for e in s["entities"]))
    out.append("    ],")
    out.append(f'    "styles": {compact(s["styles"])}')
    out.append("  },")
    out.append('  "cases": [')
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st)}" for st in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    text = "\n".join(out) + "\n"
    path = f"fixtures/commands/v1/{command}.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


write(
    "cad.entities.offset",
    "Nesneleri ötele: doğrulama, plan, yazma, geri alma",
    "ADR 0246, 0047, 0140. Denetim sırası: en az bir nesne; her kimliğin yazımı; mesafenin, sonra noktanın doğusu ve kuzeyinin sonlu olması; mesafenin sıfırdan büyük olması; beklenen sürümün yazımı, sonra çizimin sürümü; her kimliğin çizimde olması (tekrarlanan bir kez); nesne nesne, kilitli katmanda olmaması, sonra ötelenen bir tür olması; kopyaların katmanı (çizimde, katman, kilitsiz; gizliyse uyarı); en az bir kopya. Kopya, kenarların noktanın yanına mesafe kadar kaydırılması ve ardışık kaydırılmış kenarların doğrularının kesişimidir; yay aynı merkezle solunda r − d, sağında r + d; daire ve yay nokta dışarıdaysa büyür, içerideyse küçülür. Noktadan geç'te mesafe noktanın en yakın kenara uzaklığıdır. Kopya kaynağının katmanını (ya da layerId'yi), rengini ve çizgi kalınlığını alır; köşeler kaynağın köşelerinin kotlarını taşır. Kopyalar girdinin sırasıyla, her kaynağın önce noktanın yanındaki, iki yanada sonra öbür yanındaki kopyası; Kaynağı sil'de kopyası yapılan kaynaklar aynı adımda silinir. Adım Ötele'dir. Kurulumdaki en büyük kimlik 20; kopyalar 21'den başlar. $uidOf:N, N yuvasındaki nesnenin kalıcı kimliğidir.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
