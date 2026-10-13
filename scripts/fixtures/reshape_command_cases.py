"""The shared cases of the product commands cad.entities.stretch, .break, .join and .divide (docs/adr/0254).

    python3 scripts/fixtures/reshape_command_cases.py           # writes the files
    python3 scripts/fixtures/reshape_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.{stretch,break,join,divide}.json. The checks, their order, codes, paths and
messages are written here by hand from the ADR. What the commands do is computed independently, from the operations'
definitions (docs/adr/0047, 0057, 0160, 0161, 0254), without KentOS code:

- Esnet: a vertex inside the window (its edges included) moves by (dx, dy), the others stay. Without ids the
  objects are the shown ones with a vertex in the window, in the drawing's order; with ids, those listed (one
  without a vertex in the window is left out with a warning). Topolojik düzenleme: a shown object with a corner
  on a moved vertex moves that corner the same way, in the drawing's order; one on a locked layer stays
  (counted). A neighbour is invalid only by its count of corners (fewer than 2 on an open path, 3 on a ring),
  which a move never changes: a line whose ends meet after the move follows too.
- Kır: the object is walked from its start; each point's place is where it projects. On an open object the part
  between the two places goes, what is left either side stays; at one place it splits there. On a closed ring
  running counter-clockwise the part from the first place forwards to the second goes; what stays runs from the
  second place round to the first, an open polyline. A piece of two ends with a straight edge is a line. The
  first piece takes the object's place; its data stays only when it is the only piece and the object was not a
  closed area.
- Birleştir: chains grow from each object in the input's order, at the tail by an object whose start (or, turned,
  end) meets it, else at the head by one whose end (or start) meets it; a joined object's meeting point is the
  chain's own. A chain of two or more becomes one polyline (a closed area when its ends meet) in its first
  object's place, with its data; the others go. Zincir: the seed's chain is walked over the shown lines, arcs and
  open polylines, each end on to the one other end meeting it, up to a free end, a junction, a locked object or
  the start again; the seed keeps its place.
- Böl: parts n on an open object put a point at each k/n of its length (k = 1 … n−1), on a closed one at k = 0 …
  n−1; a step puts one every step from the start while short of the end; from the end, measured back from it.

The geometry is computed in exact fractions over a drawing whose every value is a double exactly.
`--check` rebuilds the files in memory and compares them with the ones on disk.
"""
import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
X, Y = 487000, 4420000


def layer(i, name, visible=True, locked=False, kind="layer", children=()):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True, "style": style,
            "children": list(children)}


def num(v):
    v = F(v)
    assert v.denominator & (v.denominator - 1) == 0, v  # a dyadic: a double exactly
    f = float(v)
    return int(f) if f.is_integer() else f


def P(x, y):
    return {"x": num(x), "y": num(y)}


def line(i, a, b, layer_id="yapi", **extra):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, **extra, "a": P(*a), "b": P(*b)}


def poly(kind, i, pts, layer_id="yapi", **extra):
    return {"kind": kind, "id": i, "layerId": layer_id, "attrs": {}, **extra, "pts": [P(*p) for p in pts]}


ENTITIES = [
    line(10, (X, Y), (X + 10, Y), attrs={"Tür": "Duvar"}, color="#E5484D"),
    poly("polyline", 11, [(X, Y + 20), (X + 10, Y + 20), (X + 10, Y + 30)], attrs={"Tür": "Bordür"}),
    poly("polygon", 12, [(X + 20, Y), (X + 30, Y), (X + 30, Y + 10), (X + 20, Y + 10)], attrs={"Ada": "101"}),
    poly("polygon", 13, [(X + 30, Y), (X + 40, Y), (X + 40, Y + 10), (X + 30, Y + 10)]),
    line(14, (X, Y + 40), (X + 10, Y + 40), layer_id="kilitli"),
    line(15, (X + 50, Y + 40), (X + 60, Y + 40), layer_id="gizli"),
    {"kind": "point", "id": 16, "layerId": "yapi", "attrs": {}, "p": P(X + 50, Y)},
    line(17, (X, Y + 60), (X + 10, Y + 60), attrs={"Tür": "Yol"}),
    line(18, (X + 10, Y + 60), (X + 20, Y + 60)),
    line(19, (X + 20, Y + 60), (X + 20, Y + 70)),
    {"kind": "circle", "id": 20, "layerId": "yapi", "attrs": {}, "c": P(X + 60, Y + 60), "r": 5},
    poly("polygon", 21, [(X + 20, Y + 10), (X + 30, Y + 10), (X + 30, Y + 20), (X + 20, Y + 20)], layer_id="kilitli"),
    line(22, (X + 30, Y + 10), (X + 32, Y + 10)),
    line(23, (X + 10, Y + 40), (X + 20, Y + 40)),
    line(24, (X + 20, Y + 40), (X + 30, Y + 40)),
    {**poly("polygon", 25, [(X + 70, Y), (X + 80, Y), (X + 80, Y + 10), (X + 70, Y + 10)]),
     "holes": [{"pts": [P(X + 72, Y + 2), P(X + 74, Y + 2), P(X + 74, Y + 4), P(X + 72, Y + 4)]}]},
    line(26, (X, Y + 90), (X + 10, Y + 90)),
    line(27, (X + F(21, 2), Y + 90), (X + 20, Y + 90)),
    line(28, (X, Y + 100), (X + 10, Y + 100)),
    line(29, (X + 10, Y + 100), (X, Y + 110)),
    line(30, (X, Y + 110), (X, Y + 100)),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = 31
LOCKED_LAYERS = {"kilitli"}
HIDDEN_LAYERS = {"gizli"}

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Biçim komutları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500},
    "origin": {"x": X, "y": Y},
    "layers": [
        layer("yapi", "Yapı"), layer("nokta", "Nokta"), layer("kilitli", "Kilitli", locked=True),
        layer("gizli", "Gizli", visible=False), layer("grup", "Grup", kind="group", children=[layer("alt", "Alt")]),
    ],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}

UID = lambda i: f"$uidOf:{i}"  # noqa: E731
BAD = "Yapı-01"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
LOCKED = "“Kilitli” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."


def bad_uid(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."


def bad_uid_long(u):
    return bad_uid(u)[:-1] + ". Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."


def not_found(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def not_finite_point(whose, axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path, uids=None):
    error = {"code": code, "message": message, "path": path}
    if uids:
        error["uids"] = uids
    return {"status": "failed", "error": error}


def warning(code, message, path=None):
    w = {"code": code, "message": message}
    if path is not None:
        w["path"] = path
    return w


def refused(name, input_, code, message, path, non_finite=None, uids=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = failed(code, message, path, uids)
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def conflict_case(input_, earlier, before=()):
    """An expected revision gone by: conflict, nothing written."""
    return {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        *before,
        {"op": "captureRevision", "as": "old"},
        earlier,
        {"op": "execute", "input": {**input_, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}}]}


def pts_of(e):
    """An object's vertices as fractions, in order (a line's two ends)."""
    if e["kind"] == "line":
        return [(F(e["a"]["x"]), F(e["a"]["y"])), (F(e["b"]["x"]), F(e["b"]["y"]))]
    if e["kind"] == "point":
        return [(F(e["p"]["x"]), F(e["p"]["y"]))]
    if e["kind"] == "circle":
        return [(F(e["c"]["x"]), F(e["c"]["y"]))]
    return [(F(p["x"]), F(p["y"])) for p in e["pts"]]


def with_pts(e, pts):
    """`e` with its vertices replaced (a line's two ends, a path's points)."""
    out = {k: v for k, v in e.items()}
    if e["kind"] == "line":
        out["a"], out["b"] = P(*pts[0]), P(*pts[1])
    else:
        out["pts"] = [P(*p) for p in pts]
    return out


# ── Esnet ─────────────────────────────────────────────────────────────────────────────────────────────────────────

def inside(p, w):
    (x0, y0), (x1, y1) = w
    return min(x0, x1) <= p[0] <= max(x0, x1) and min(y0, y1) <= p[1] <= max(y0, y1)


def window_input(w):
    return {"min": P(*w[0]), "max": P(*w[1])}


def hole_pts(e):
    return [[(F(q["x"]), F(q["y"])) for q in h["pts"]] for h in e.get("holes", [])]


def stretched(e, w, dx, dy):
    """`e` with its vertices in `w` moved, its holes' too; None when none is."""
    pts, holes = pts_of(e), hole_pts(e)
    if not any(inside(p, w) for p in pts + [q for h in holes for q in h]):
        return None

    def mv(p):
        return (p[0] + dx, p[1] + dy) if inside(p, w) else p

    out = with_pts(e, [mv(p) for p in pts])
    if holes:
        out["holes"] = [{"pts": [P(*mv(q)) for q in h]} for h in holes]
    if e["kind"] == "point":
        out = {**e, "p": P(*mv(pts[0]))}
    return out


def shown(e):
    return e["layerId"] not in HIDDEN_LAYERS


def followers(moved, edited_ids, points=False):
    """Topolojik düzenleme: the shown objects with a corner at a moved vertex move it; the locked counted, the
    collapsing ones counted invalid. `moved`: [(at, to)]. Their new shapes in the drawing's order."""
    out, locked, invalid = [], 0, 0
    for e in ENTITIES:
        if e["id"] in edited_ids or not shown(e) or e["kind"] not in ("line", "polyline", "polygon") + (("point",) if points else ()):
            continue
        pts = pts_of(e)
        if not any(p == at for p in pts for at, _ in moved):
            continue
        if e["layerId"] in LOCKED_LAYERS:
            locked += 1
            continue
        # A move keeps every corner: the neighbour stays valid by its count, even when two corners meet.
        new = [next((to for at, to in moved if at == p), p) for p in pts]
        out.append(with_pts(e, new))
    return out, locked, invalid


NEIGHBOUR_LOCKED = "Kilitli katmandaki {} komşu nesne değişmedi; ortak sınır ayrıldı."
NEIGHBOUR_INVALID = "{} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe)."
NOTHING_TO_STRETCH = "Pencerede köşesi olan düzenlenebilir nesne yok. Pencereyi taşınacak köşelerin üstüne verin."


def stretch_case(name, w, dx, dy, uids=None, topology=False, extra=()):
    inp = {"window": window_input(w), "dx": num(dx), "dy": num(dy)}
    warnings = []
    if uids is None:
        cands = [e for e in ENTITIES if shown(e) and stretched(e, w, 0, 0) is not None]
        locked = [e for e in cands if e["layerId"] in LOCKED_LAYERS]
        targets = [e for e in cands if e["layerId"] not in LOCKED_LAYERS]
        if locked:
            warnings.append(warning("locked_skipped", f"{len(locked)} nesne kilitli katmanda olduğu için atlandı."))
    else:
        inp["uids"] = [UID(i) for i in uids]
        targets = []
        for k, i in enumerate(uids):
            e = BY_ID[i]
            if stretched(e, w, 0, 0) is None:
                warnings.append(warning("outside_window", "Nesnenin pencerede köşesi yok; esnemedi.", f"uids[{k}]"))
            elif e["layerId"] in LOCKED_LAYERS:
                warnings.append(warning("locked_skipped", "Nesne kilitli katmanda olduğu için atlandı.", f"uids[{k}]"))
            else:
                targets.append(e)
    if topology:
        inp["topology"] = {}
    new = [stretched(e, w, dx, dy) for e in targets]
    follow = []
    if topology:
        moved = [(p, (p[0] + dx, p[1] + dy)) for e in targets for p in pts_of(e) if inside(p, w)]
        follow, nl, ni = followers(moved, {e["id"] for e in targets})
        if nl:
            warnings.append(warning("neighbour_locked", NEIGHBOUR_LOCKED.format(nl)))
        if ni:
            warnings.append(warning("neighbour_invalid", NEIGHBOUR_INVALID.format(ni)))
    ents = {str(e["id"]): e for e in new + follow}
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"stretched": [UID(e["id"]) for e in new], "followed": [UID(e["id"]) for e in follow], "revision": "$current"}, warnings),
         "expect": {"ids": IDS, "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}},
        *extra,
    ]}


def stretch_cases():
    end10 = ((X + 9, Y - 1), (X + 11, Y + 1))
    corner = ((X + 29, Y - 1), (X + 31, Y + 1))
    top = ((X + 29, Y + 9), (X + 31, Y + 11))
    out = [
        stretch_case("Pencere çizginin ucunu alır: uç 2 m doğuya taşınır, öbürü ve öznitelikler kalır; adım “Esnet”", end10, 2, 0,
                     extra=[{"op": "undo", "returns": "Esnet", "expect": {"ids": IDS, "entities": {"10": BY_ID[10]}, "canUndo": False, "canRedo": True}}]),
        stretch_case("Pencere iki alanın ortak köşesini alır: ikisi de esner, belgenin sırasıyla", corner, 1, -1),
        stretch_case("Seçim yalnız onları esnetir; pencerede köşesi olmayan outside_window uyarısıyla atlanır", corner, 1, -1, uids=[12, 10]),
        stretch_case("Topoloji: ortak köşeli komşular izler (iki ucu buluşan çizgi de, köşe sayısı değişmez); kilitli komşu uyarıyla değişmez",
                     top, 2, 0, uids=[12], topology=True),
        stretch_case("Pencerede kilitli nesne locked_skipped uyarısıyla atlanır, öbürü esner", ((X + 9, Y + 39), (X + 11, Y + 41)), 0, 1),
        stretch_case("Pencerenin köşeleri ters verilse de aynı pencere", ((X + 11, Y + 1), (X + 9, Y - 1)), 2, 0),
        stretch_case("Nokta penceredeyse taşınır", ((X + 49, Y - 1), (X + 51, Y + 1)), -1, 2),
        stretch_case("Alanın deliğinin köşesi de esner", ((X + 73, Y + 3), (X + 75, Y + 5)), 1, 1),
        stretch_case("Eksi yer değiştirme: çizginin başı batıya ve kuzeye", ((X - 1, Y - 1), (X + 1, Y + 1)), -2, 3),
    ]
    out.append({"name": "Sıfır yer değiştirme: hiçbir şey yazılmaz, listeler boş", "steps": [
        {"op": "execute", "input": {"window": window_input(end10), "dx": 0, "dy": 0},
         "result": completed({"stretched": [], "followed": [], "revision": "$current"}), "expect": UNCHANGED}]})
    out.append({"name": "Plan: nesneler olacakları gibi; hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": {"window": window_input(end10), "dx": 2, "dy": 0},
         "result": completed({"stretched": [stretched(BY_ID[10], end10, 2, 0)], "followed": [], "revision": "$current"}), "expect": UNCHANGED}]})
    one = {"window": window_input(end10), "dx": 2, "dy": 0}
    out += [
        refused("Pencerenin köşesi sonlu değil", one, "not_finite", not_finite_point("Pencerenin köşesinin", "x"), "window.min.x",
                non_finite={"window.min.x": "NaN"}),
        refused("Yer değiştirme sonlu değil", one, "not_finite",
                "Doğuya (Y) yer değiştirme sonlu bir sayı değil (NaN ya da sonsuz). Yer değiştirmeyi sonlu bir sayıyla verin.", "dx",
                non_finite={"dx": "Infinity"}),
        refused("Kimlik listesi boş", {**one, "uids": []}, "no_entities", "Esnetilecek nesne yok. En az bir nesnenin kalıcı kimliğini verin.", "uids"),
        refused("Kimlik geçersiz", {**one, "uids": [BAD]}, "invalid_uid", bad_uid_long(BAD), "uids[0]"),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Nesne çizimde yok", {**one, "uids": [MISSING]}, "entity_not_found", not_found(MISSING), "uids[0]"),
        refused("Penceredeki tek nesne kilitli: ret, atlanan nesne hatada", {"window": window_input(((X - 1, Y + 39), (X + 1, Y + 41))), "dx": 1, "dy": 0},
                "nothing_to_stretch", NOTHING_TO_STRETCH, "window", uids=[UID(14)]),
        refused("Gizli katmanın nesnesi pencereye girmez", {"window": window_input(((X + 49, Y + 39), (X + 51, Y + 41))), "dx": 1, "dy": 0},
                "nothing_to_stretch", NOTHING_TO_STRETCH, "window"),
    ]
    out.append(conflict_case(one, {"op": "execute", "input": {"window": window_input(corner), "dx": 1, "dy": 0},
                                   "result": completed({"stretched": [UID(12), UID(13)], "followed": [], "revision": "$current"})}))
    return out


# ── Kır ───────────────────────────────────────────────────────────────────────────────────────────────────────────

def path_of(e):
    """The object as a walk: its vertices and whether it closes (lines and straight paths only)."""
    pts = pts_of(e)
    return pts, e["kind"] == "polygon"


def edges(pts, closed):
    ring = pts + [pts[0]] if closed else pts
    return list(zip(ring, ring[1:]))


def length(a, b):
    # Axis-parallel edges only in these cases: the length is exact.
    assert a[0] == b[0] or a[1] == b[1]
    return abs(b[0] - a[0]) + abs(b[1] - a[1])


def place(pts, closed, p):
    """Where `p` projects on the walk: the distance along it (axis-parallel edges)."""
    best, s0 = None, F(0)
    for a, b in edges(pts, closed):
        ln = length(a, b)
        if a[1] == b[1]:
            t = min(max((p[0] - a[0]) / (b[0] - a[0]), 0), 1)
        else:
            t = min(max((p[1] - a[1]) / (b[1] - a[1]), 0), 1)
        q = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
        d = abs(q[0] - p[0]) + abs(q[1] - p[1])
        if best is None or d < best[0]:
            best = (d, s0 + ln * t)
        s0 += ln
    return best[1]


def walk(pts, closed, s0, s1):
    """The vertices from s0 to s1 along the walk (s1 may pass round a closed one)."""
    es = edges(pts, closed)
    total = sum(length(a, b) for a, b in es)

    def at(s):
        s = s % total if closed else s
        g = F(0)
        for a, b in es:
            ln = length(a, b)
            if s <= g + ln:
                t = (s - g) / ln
                return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
            g += ln
        return es[-1][1]

    out = [at(s0)]
    g = F(0)
    for rnd in range(2 if closed else 1):
        for a, b in es:
            ln = length(a, b)
            lo, hi = max(s0, g), min(s1, g + ln)
            if hi > lo:
                out.append(at(hi) if hi < g + ln else b)
            g += ln
    return out


def piece(e, pts, pid, keep):
    """A piece as written: a line for two ends, else a polyline; the object's look, its data when kept."""
    base = {"layerId": e["layerId"], "attrs": dict(e["attrs"]) if keep else {}}
    for k in ("color", "lineWeight"):
        if k in e:
            base[k] = e[k]
    if len(pts) == 2:
        return {"kind": "line", "id": pid, **base, "a": P(*pts[0]), "b": P(*pts[1])}
    return {"kind": "polyline", "id": pid, **base, "pts": [P(*p) for p in pts]}


def broken(e, p1, p2):
    pts, closed = path_of(e)
    total = sum(length(a, b) for a, b in edges(pts, closed))
    s1, s2 = place(pts, closed, p1), place(pts, closed, p2)
    if closed and s1 == s2:
        # Opened at one place: one open polyline round the whole ring.
        return [walk(pts, True, s1, s1 + total)]
    if not closed:
        if s1 == s2:
            return [walk(pts, False, F(0), s1), walk(pts, False, s1, total)]
        lo, hi = min(s1, s2), max(s1, s2)
        out = []
        if lo > 0:
            out.append(walk(pts, False, F(0), lo))
        if hi < total:
            out.append(walk(pts, False, hi, total))
        return out
    # Counter-clockwise rings in these cases: the part s1 → s2 goes, s2 → s1 (+L) stays.
    keep_end = s1 if s1 > s2 else s1 + total
    return [walk(pts, True, s2, keep_end)]


def break_case(name, eid, p1, p2=None, plan=False, extra=()):
    e = BY_ID[eid]
    pieces = broken(e, p1, p1 if p2 is None else p2)
    keep = len(pieces) == 1 and e["kind"] != "polygon"
    first = piece(e, pieces[0], eid, keep)
    created = [piece(e, q, (0 if plan else NEXT + k), keep) for k, q in enumerate(pieces[1:])]
    inp = {"uid": UID(eid), "first": P(*p1)}
    if p2 is not None:
        inp["second"] = P(*p2)
    if plan:
        return {"name": name, "steps": [{"op": "plan", "input": inp,
                "result": completed({"changed": [first], "created": created, "removed": [], "revision": "$current"}), "expect": UNCHANGED}]}
    ents = {str(eid): first, **{str(c["id"]): c for c in created}}
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"changed": [UID(eid)], "created": [UID(c["id"]) for c in created], "removed": [], "revision": "$current"}),
         "expect": {"ids": IDS + [c["id"] for c in created], "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}},
        *extra,
    ]}


def break_cases():
    out = [
        break_case("Çizginin iki noktası arası gider: iki parça, ilki yerinde, öbürü yeni; öznitelik kalmaz, renk kalır; adım “Kır”",
                   10, (X + 4, Y), (X + 6, Y),
                   extra=[{"op": "undo", "returns": "Kır", "expect": {"ids": IDS, "entities": {"10": BY_ID[10]}, "canUndo": False, "canRedo": True}}]),
        break_case("İkinci nokta yoksa çizgi ilk noktada ikiye bölünür", 10, (X + 5, Y)),
        break_case("Nokta nesnenin dışındaysa izdüşümü sayılır: uca kadar gider, tek parça öznitelikleriyle kalır", 10, (X + 8, Y), (X + 12, Y)),
        break_case("Çoklu çizginin iki kenarı arası: iki uçlu parçalar çizgi olur", 11, (X + 5, Y + 20), (X + 10, Y + 25)),
        break_case("Kapalı alan: ilk noktadan ikinciye saat tersi giden kısım silinir, kalan açık çoklu çizgi; alanın verisi kalmaz",
                   12, (X + 25, Y), (X + 30, Y + 5)),
        break_case("Kapalı alan tek noktada: halka o noktadan açılır, tek açık çoklu çizgi", 12, (X + 25, Y)),
        break_case("Noktalar ters sırayla verilse de aynı kısım gider", 10, (X + 6, Y), (X + 4, Y)),
        break_case("Plan: parçalar yazılacakları gibi, yeni olan id 0; hiçbir şey yazılmaz", 10, (X + 4, Y), (X + 6, Y), plan=True),
    ]
    one = {"uid": UID(10), "first": P(X + 4, Y), "second": P(X + 6, Y)}
    out += [
        refused("Kimlik geçersiz", {**one, "uid": BAD}, "invalid_uid", bad_uid(BAD), "uid"),
        refused("İlk nokta sonlu değil", one, "not_finite", not_finite_point("İlk kırma noktasının", "x"), "first.x", non_finite={"first.x": "NaN"}),
        refused("İkinci nokta sonlu değil", one, "not_finite", not_finite_point("İkinci kırma noktasının", "y"), "second.y",
                non_finite={"second.y": "-Infinity"}),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Nesne çizimde yok", {**one, "uid": MISSING}, "entity_not_found", not_found(MISSING), "uid"),
        refused("Nokta kırılmaz", {"uid": UID(16), "first": P(X + 50, Y)}, "not_breakable",
                "Bu türdeki nesne kırılmaz: çizgi, çoklu çizgi, alan, yay, daire, elips, yardımcı çizgi ya da ışın olmalı.", "uid"),
        refused("Adalı alan kırılmaz", {"uid": UID(25), "first": P(X + 75, Y), "second": P(X + 80, Y + 5)}, "not_breakable",
                "Adalı alanda kırma yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın.", "uid"),
        refused("Kilitli katmandaki çizgi", {"uid": UID(14), "first": P(X + 4, Y + 40)}, "layer_locked", LOCKED, "uid"),
        refused("Uçtan kırılmaz: çekirdeğin sözü", {"uid": UID(10), "first": P(X, Y)}, "break_failed",
                "Nesne ucundan kırılamaz; iç kısmında bir nokta gösterin.", "first"),
        refused("İki nokta bütün nesneyi kapsıyor", {"uid": UID(10), "first": P(X - 1, Y), "second": P(X + 11, Y)}, "break_failed",
                "İki nokta nesnenin tamamını kapsıyor; silmek için Sil kullanın.", "first"),
        refused("Daire tek noktadan kırılmaz", {"uid": UID(20), "first": P(X + 65, Y + 60)}, "break_failed",
                "Daire tek noktadan kırılamaz; ikinci bir nokta gösterin.", "first"),
    ]
    out.append(conflict_case(one, {"op": "execute", "input": {"uid": UID(11), "first": P(X + 5, Y + 20)},
                                   "result": completed({"changed": [UID(11)], "created": [UID(NEXT)], "removed": [], "revision": "$current"})}))
    return out


# ── Birleştir ─────────────────────────────────────────────────────────────────────────────────────────────────────

def ends(e):
    pts = pts_of(e)
    return pts


def join_chains(ids, tol):
    """The chains grown from each object in order (see the module): [(sources, points)]."""
    pool = [i for i in ids if BY_ID[i]["kind"] in ("line", "polyline")]
    used, groups = set(), []

    def near(a, b):
        return abs(a[0] - b[0]) ** 2 + abs(a[1] - b[1]) ** 2 <= tol * tol

    for seed in pool:
        if seed in used:
            continue
        used.add(seed)
        chain, sources = list(ends(BY_ID[seed])), [seed]
        grew = True
        while grew:
            grew = False
            for i in pool:
                if i in used:
                    continue
                head, tail = chain[0], chain[-1]
                if near(head, tail) and len(chain) > 2:
                    break
                c = ends(BY_ID[i])
                if near(tail, c[0]):
                    chain, at_tail = chain + c[1:], True
                elif near(tail, c[-1]):
                    chain, at_tail = chain + c[::-1][1:], True
                elif near(head, c[-1]):
                    chain, at_tail = c + chain[1:], False
                elif near(head, c[0]):
                    chain, at_tail = c[::-1] + chain[1:], False
                else:
                    continue
                if at_tail:
                    sources.append(i)
                else:
                    sources.insert(0, i)
                used.add(i)
                grew = True
        if len(sources) >= 2:
            groups.append((sources, chain))
    return groups


def chain_walk(seed):
    """Zincir over the shown lines, arcs and open polylines (see the module): the members, whether it stopped at a
    locked one."""
    cands = [e for e in ENTITIES if shown(e) and e["kind"] in ("line", "polyline")]
    endpoints = {e["id"]: (pts_of(e)[0], pts_of(e)[-1]) for e in cands}
    sides, stopped = [[], []], False
    for side in (1, 0):
        current, end = seed, side
        at = endpoints[seed][side]
        while True:
            others = [(j, k) for j, ee in endpoints.items() for k in (0, 1) if not (j == current and k == end) and ee[k] == at]
            if len(others) != 1:
                break
            j, k = others[0]
            if j == seed or j in sides[0] + sides[1]:
                break
            if BY_ID[j]["layerId"] in LOCKED_LAYERS:
                stopped = True
                break
            sides[side].append(j)
            current, end = j, 1 - k
            at = endpoints[j][end]
    return list(reversed(sides[0])) + [seed] + sides[1], stopped


def joined_entity(first, pts):
    e = BY_ID[first]
    closed = len(pts) > 2 and pts[0] == pts[-1]
    out = {"kind": "polygon" if closed else "polyline", "id": first, "layerId": e["layerId"], "attrs": dict(e["attrs"])}
    for k in ("color", "lineWeight"):
        if k in e:
            out[k] = e[k]
    out["pts"] = [P(*p) for p in (pts[:-1] if closed else pts)]
    return out


def join_case(name, uids, tol=F(1, 1000), chain=False, plan=False, input_tol=None, extra=()):
    warnings = []
    if chain:
        members, stopped = chain_walk(uids[0])
        groups = join_chains(members, tol)
        keep = uids[0]
        if stopped:
            warnings.append(warning("chain_stopped", "Zincir kilitli katmandaki bir nesnede durdu."))
    else:
        targets = []
        for k, i in enumerate(uids):
            if BY_ID[i]["layerId"] in LOCKED_LAYERS:
                warnings.append(warning("locked_skipped", "Nesne kilitli katmanda olduğu için birleştirilmedi.", f"uids[{k}]"))
            else:
                targets.append(i)
        groups = join_chains(targets, tol)
        keep = None
    changed, removed, ents = [], [], {}
    for sources, pts in groups:
        first = keep if keep in sources else sources[0]
        changed.append(first)
        removed += [s for s in sources if s != first]
        ents[str(first)] = joined_entity(first, pts)
    inp = {"uids": [UID(i) for i in uids]}
    if input_tol is not None:
        inp["tolerance"] = num(input_tol)
    if chain:
        inp["chain"] = {"kind": "all"}
    if plan:
        return {"name": name, "steps": [{"op": "plan", "input": inp,
                "result": completed({"changed": list(ents.values()), "created": [], "removed": [UID(i) for i in removed], "revision": "$current"}),
                "expect": UNCHANGED}]}
    # The objects that go are named by their ids taken before.
    captures = [{"op": "captureUid", "id": i, "as": f"u{i}"} for i in removed]
    return {"name": name, "steps": [
        *captures,
        {"op": "execute", "input": inp,
         "result": completed({"changed": [UID(i) for i in changed], "created": [], "removed": [f"$uid:u{i}" for i in removed], "revision": "$current"}, warnings),
         "expect": {"ids": [i for i in IDS if i not in removed], "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}},
        *extra,
    ]}


NOTHING_TO_JOIN = "Uçları birleşen çizgi, yay ya da açık çoklu çizgi bulunamadı. Toleransı artırmayı deneyin."


def join_cases():
    out = [
        join_case("Uç uca üç çizgi tek çoklu çizgi olur: ilkinin yerinde, kimliği ve verisiyle; öbürleri gider; adım “Birleştir”", [17, 18, 19],
                  extra=[{"op": "undo", "returns": "Birleştir", "expect": {"ids": IDS, "entities": {"17": BY_ID[17], "18": BY_ID[18]}, "canUndo": False, "canRedo": True}}]),
        join_case("Kapanan zincir kapalı alan olur", [28, 29, 30]),
        join_case("Toleransla: 0,5 m aralıklı uçlar 1 m toleransla buluşur; birleşme noktası zincirin kendi ucu", [26, 27], tol=F(1), input_tol=1),
        join_case("Kilitli katmandaki nesne locked_skipped uyarısıyla dışarıda kalır, öbürleri birleşir", [17, 18, 14]),
        join_case("Zincir: tohum ortadaki çizgi; bağlı çizgiler onunla birleşir, tohum yerini ve kimliğini verir", [18], chain=True),
        join_case("Zincir kilitli nesnede durur: chain_stopped uyarısı", [23], chain=True),
        join_case("Plan: birleşen nesne yazılacağı gibi; hiçbir şey yazılmaz", [17, 18, 19], plan=True),
    ]
    out += [
        refused("Kimlik yok", {"uids": []}, "no_entities", "Birleştirilecek nesne yok. En az bir nesnenin kalıcı kimliğini verin.", "uids"),
        refused("Zincir tek tohumla başlar", {"uids": [UID(17), UID(18)], "chain": {"kind": "all"}}, "no_entities",
                "Zincir tek bir nesneden (tohum) başlar; bir kimlik verin.", "uids"),
        refused("Kimlik geçersiz", {"uids": [BAD]}, "invalid_uid", bad_uid(BAD), "uids[0]"),
        refused("Zincirin nesne listesi boş", {"uids": [UID(18)], "chain": {"kind": "objects", "uids": []}}, "no_entities",
                "Zincirin nesneleri yok. Nesneleri verin ya da bütün görünen nesneler için “all”u kullanın.", "chain.uids"),
        refused("Tolerans sıfır olamaz", {"uids": [UID(17), UID(18)], "tolerance": 0}, "invalid_distance",
                "Tolerans sıfırdan büyük sonlu bir uzunluk (metre) olmalı.", "tolerance"),
        refused("Tolerans sonlu değil", {"uids": [UID(17), UID(18)], "tolerance": 1}, "invalid_distance",
                "Tolerans sıfırdan büyük sonlu bir uzunluk (metre) olmalı.", "tolerance", non_finite={"tolerance": "Infinity"}),
        refused("Zincirin dikdörtgeni sonlu değil", {"uids": [UID(18)], "chain": {"kind": "area", "min": P(X, Y), "max": P(X + 30, Y + 80)}},
                "not_finite", not_finite_point("Dikdörtgenin köşesinin", "x"), "chain.min.x", non_finite={"chain.min.x": "NaN"}),
        refused("Beklenen sürüm sayı değil", {"uids": [UID(17), UID(18)], "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Nesne çizimde yok", {"uids": [MISSING]}, "entity_not_found", not_found(MISSING), "uids[0]"),
        refused("Zincirin nesnesi çizimde yok", {"uids": [UID(18)], "chain": {"kind": "objects", "uids": [MISSING]}}, "entity_not_found",
                not_found(MISSING), "chain.uids[0]"),
        refused("Zincir noktadan başlamaz", {"uids": [UID(16)], "chain": {"kind": "all"}}, "not_joinable",
                "Zincir bir çizgiden, yaydan ya da açık çoklu çizgiden başlar.", "uids[0]"),
        refused("Kilitli tohum", {"uids": [UID(14)], "chain": {"kind": "all"}}, "layer_locked", LOCKED, "uids[0]"),
        refused("Uçları buluşmayan nesneler", {"uids": [UID(17), UID(19)]}, "nothing_to_join", NOTHING_TO_JOIN, "uids"),
        refused("Toleransı küçük kalan aralık", {"uids": [UID(26), UID(27)]}, "nothing_to_join", NOTHING_TO_JOIN, "uids"),
        refused("Zincirin tek nesnesi", {"uids": [UID(10)], "chain": {"kind": "all"}}, "nothing_to_join",
                "Bu nesneye ucu ucuna bağlanan nesne yok; zincir kurulamadı.", "uids[0]"),
    ]
    out.append(conflict_case({"uids": [UID(17), UID(18)]}, {"op": "execute", "input": {"uids": [UID(28), UID(29), UID(30)]},
                             "result": completed({"changed": [UID(28)], "created": [], "removed": ["$uid:u29", "$uid:u30"], "revision": "$current"})},
                             before=[{"op": "captureUid", "id": 29, "as": "u29"}, {"op": "captureUid", "id": 30, "as": "u30"}]))
    return out


# ── Böl ───────────────────────────────────────────────────────────────────────────────────────────────────────────

def division(e, by, from_end=False):
    pts, closed = path_of(e)
    total = sum(length(a, b) for a, b in edges(pts, closed))
    if by[0] == "parts":
        n = by[1]
        ks = range(0 if closed else 1, n)
        ss = [total * k / n for k in ks]
    else:
        step = F(by[1])
        ss, s = [], step
        eps = F(1, 10 ** 9) * max(1, total)
        while s < total - eps:
            ss.append(s)
            s += step
    if from_end:
        ss = [total - s for s in ss]
    return [walk(pts, closed, s, s)[0] for s in ss]


def point_entity(pid, p, layer_id="nokta", **extra):
    return {"kind": "point", "id": pid, "layerId": layer_id, "attrs": {}, **extra, "p": P(*p)}


def divide_case(name, eid, by, from_end=False, layer_id="nokta", look=None, warnings=(), plan=False):
    points = division(BY_ID[eid], by, from_end)
    extra = {k: v for k, v in (look or {}).items() if k in ("color", "label", "symbol", "lineWeight")}
    division_input = {"kind": "parts", "count": by[1]} if by[0] == "parts" else {"kind": "step", "length": num(by[1])}
    inp = {"uid": UID(eid), "division": division_input, "layerId": layer_id}
    if from_end:
        inp["fromEnd"] = True
    if look:
        inp.update(look)
    if plan:
        ents = [point_entity(0, p, layer_id, **extra) for p in points]
        return {"name": name, "steps": [{"op": "plan", "input": inp, "result": completed({"entities": ents, "revision": "$current"}, warnings),
                                         "expect": UNCHANGED}]}
    ids = list(range(NEXT, NEXT + len(points)))
    ents = {str(i): point_entity(i, p, layer_id, **extra) for i, p in zip(ids, points)}
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"created": [UID(i) for i in ids], "ids": ids, "revision": "$current"}, warnings),
         "expect": {"ids": IDS + ids, "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}},
        {"op": "undo", "returns": "Böl", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
    ]}


def divide_cases():
    out = [
        divide_case("Çizgi dört eşit parçaya: aralarına üç nokta; adım “Böl”", 10, ("parts", 4)),
        divide_case("Çoklu çizgi her 3 metrede: sonun kısa kalanına nokta konmaz", 11, ("step", 3)),
        divide_case("Sondan ölçülerek her 3 metrede", 10, ("step", 3), from_end=True),
        divide_case("Kapalı alan dört parçaya: başlangıcı dahil dört nokta", 12, ("parts", 4)),
        divide_case("Noktalar verilen renk ve etiketle", 10, ("parts", 2), look={"color": "#3E63DD", "label": "B"}),
        divide_case("Gizli katmana: layer_hidden uyarısıyla yazılır", 10, ("parts", 2), layer_id="gizli",
                    warnings=[warning("layer_hidden", "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "layerId")]),
        divide_case("Plan: noktalar yazılacakları gibi, id 0; hiçbir şey yazılmaz", 10, ("parts", 4), plan=True),
    ]
    one = {"uid": UID(10), "division": {"kind": "parts", "count": 4}, "layerId": "nokta"}
    out += [
        refused("Kimlik geçersiz", {**one, "uid": BAD}, "invalid_uid", bad_uid(BAD), "uid"),
        refused("Bir parça bölme değildir", {**one, "division": {"kind": "parts", "count": 1}}, "invalid_count",
                "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.", "division.count"),
        refused("10 000'den çok parça", {**one, "division": {"kind": "parts", "count": 10001}}, "invalid_count",
                "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.", "division.count"),
        refused("Aralık sıfır olamaz", {**one, "division": {"kind": "step", "length": 0}}, "invalid_distance",
                "Aralık sıfırdan büyük sonlu bir uzunluk (metre) olmalı.", "division.length"),
        refused("Aralık sonlu değil", {**one, "division": {"kind": "step", "length": 1}}, "invalid_distance",
                "Aralık sıfırdan büyük sonlu bir uzunluk (metre) olmalı.", "division.length", non_finite={"division.length": "NaN"}),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Nesne çizimde yok", {**one, "uid": MISSING}, "entity_not_found", not_found(MISSING), "uid"),
        refused("Nokta bölünmez", {**one, "uid": UID(16)}, "not_divisible",
                "Bu türdeki nesne bölünmez: çizgi, çoklu çizgi, alan, yay, daire, elips ya da eğri olmalı.", "uid"),
        refused("Aralık nesneden uzun", {**one, "division": {"kind": "step", "length": 20}}, "no_points",
                "Aralık nesnenin boyundan uzun; nokta konmadı. Daha kısa bir aralık verin.", "division.length"),
        refused("10 000'den çok nokta", {**one, "division": {"kind": "step", "length": num(F(1, 1024))}}, "too_many_points",
                "10 000'den çok nokta olurdu; daha büyük bir aralık verin.", "division"),
        refused("Katman yok", {**one, "layerId": "yok"}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Grup katman değildir", {**one, "layerId": "grup"}, "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        refused("Kilitli katmana yazılmaz", {**one, "layerId": "kilitli"}, "layer_locked",
                "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        refused("Kalınlık aralıkta değil: yol görünüşün kendi alanı", {**one, "lineWeight": 200}, "invalid_line_weight",
                "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"),
    ]
    out.append(conflict_case(one, {"op": "execute", "input": {"uid": UID(11), "division": {"kind": "parts", "count": 2}, "layerId": "nokta"},
                                   "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"})}))
    return out


FILES = [
    ("cad.entities.stretch", "Nesneleri esnet", stretch_cases),
    ("cad.entities.break", "Nesneyi kır", break_cases),
    ("cad.entities.join", "Nesneleri birleştir", join_cases),
    ("cad.entities.divide", "Nesneyi böl", divide_cases),
]


def build(command, title, cases):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
        "note": "Written by scripts/fixtures/reshape_command_cases.py from the operations' definitions (docs/adr/0047, 0057, 0160, 0161, 0254); do not edit by hand.",
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
