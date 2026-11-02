#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.terrains.{create,edit,query}.json: the shared cases of KentOS Yol's terrain surface
commands (plan/road.md Y2.3, docs/adr/0900 §10–§11), their expectations built here without KentOS code: the sources
are read from the drawing by the definition's rules (ADR 0900 §10), the surface is Shewchuk's Triangle's (the
`triangle` package, as scripts/fixtures/road_surface_cases.py), heights, areas and sections are exact fractions, the
digest is the canonical JSON's SHA-256 (scripts/fixtures/protocol_cases.py). The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    uv run --no-project --python 3.11 --with triangle --with numpy python3 scripts/fixtures/road_terrain_command_cases.py
    ... --check   # compares the files with the ones on disk
"""

import copy
import hashlib
import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import protocol_cases  # noqa: E402
import road_surface_cases as tri  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
ALGORITHM = "kentos.cdt/1"
TOLERANCE = 0.001

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def U(n):
    return f"0192f5a0-7c3e-7d4a-9b1e-{n:012x}"


def node(id_, name, children=None):
    return {"id": id_, "name": name, "type": "group" if children is not None else "layer", "visible": True,
            "locked": False, "expanded": True, "style": dict(STYLE), "children": children or []}


def P(x, y):
    return {"x": float(x), "y": float(y)}


# The drawing: four corner points and an inner one on `zemin`, one without a height, a line on `zemin` that is no point;
# two breaklines on `kirik`; a boundary, an area void and a closed polyline void on `sinir`; an arc breakline, an open
# polyline and duplicate points on `0`; a point in the group `arazi` (layer `ek`).
ENTITIES = [
    ("point", "zemin", {"p": P(0, 0), "z": 100.0}),
    ("point", "zemin", {"p": P(40, 0), "z": 101.5}),
    ("point", "zemin", {"p": P(40, 30), "z": 103.25}),
    ("point", "zemin", {"p": P(0, 30), "z": 102.0}),
    ("point", "zemin", {"p": P(21, 24), "z": 102.75}),
    ("point", "zemin", {"p": P(10, 26)}),
    ("line", "zemin", {"a": P(1, 29), "b": P(3, 29)}),
    ("polyline", "kirik", {"pts": [P(5, 15), P(35, 15)], "zs": [101.0, 102.5]}),
    ("line", "kirik", {"a": P(10, 5), "b": P(30, 5), "za": 100.5, "zb": 101.25}),
    ("polygon", "sinir", {"pts": [P(2, 2), P(38, 2), P(38, 28), P(2, 28)]}),
    ("polygon", "sinir", {"pts": [P(18, 8), P(24, 8), P(24, 12), P(18, 12)]}),
    ("polyline", "sinir", {"pts": [P(28, 19), P(32, 19), P(32, 22), P(28, 22)], "closed": True}),
    ("polyline", "0", {"pts": [P(6, 25), P(14, 27)], "zs": [102.0, 102.5], "bulges": [0.25, 0.0]}),
    ("polyline", "0", {"pts": [P(26, 24), P(30, 24), P(30, 27)]}),
    ("point", "0", {"p": P(40, 30), "z": 103.75}),
    ("point", "ek", {"p": P(31, 10), "z": 101.75}),
    ("polygon", "0", {"pts": [P(-5, -5), P(45, -5), P(45, 35), P(-5, 35)]}),
    ("point", "0", {"p": P(20, 15), "z": 104.0}),
]
UIDS = [U(i + 1) for i in range(len(ENTITIES))]
(Z1, Z2, Z3, Z4, Z5, ZNONE, ZLINE, BL1, BL2, BOUND, VOID1, VOID2, ARC, OPEN, DUP, EK, BIG, ZCONF) = UIDS

LAYERS = [node("0", "0"), node("zemin", "Zemin"), node("kirik", "Kırık hatlar"), node("sinir", "Sınırlar"),
          node("arazi", "Arazi", [node("ek", "Ek noktalar")])]


def entity_json(slot, kind, layer, geo):
    return {"kind": kind, "id": slot, "layerId": layer, "attrs": {}, **copy.deepcopy(geo)}


def setup(surfaces=None, work="road"):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if surfaces:
        settings["road"] = {"surfaces": surfaces}
    return {
        "format": "kentos.document", "version": 2, "name": "Yol yüzeyleri", "settings": settings,
        "origin": {"x": 0.0, "y": 0.0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "0",
        "entities": [entity_json(i + 1, k, l, g) for i, (k, l, g) in enumerate(ENTITIES)],
        "uids": list(UIDS), "styles": {"items": [], "categories": []},
    }


# ── The sources (ADR 0900 §10) ──────────────────────────────────────────────────────────────────────────────────────

def find_node(nodes, id_):
    for n in nodes:
        if n["id"] == id_:
            return n
        hit = find_node(n["children"], id_)
        if hit:
            return hit
    return None


def layer_set(named):
    out, missing = set(), []
    for id_ in named:
        n = find_node(LAYERS, id_)
        if n is None:
            missing.append(id_)
            continue
        stack = [n]
        while stack:
            m = stack.pop()
            out.add(m["id"])
            stack += m["children"]
    return out, missing


def xy(p):
    return (p["x"], p["y"])


def path_of(pts, zs, bulges):
    if bulges and any(b != 0 for b in bulges):
        return "arc_edge"
    if zs is None or len(zs) != len(pts) or any(z is None for z in zs):
        return "no_height"
    return [(p["x"], p["y"], z) for p, z in zip(pts, zs)]


def lines_of(kind, g):
    if kind == "line":
        if g.get("za") is None or g.get("zb") is None:
            return "no_height"
        return [([(g["a"]["x"], g["a"]["y"], g["za"]), (g["b"]["x"], g["b"]["y"], g["zb"])], False)]
    if kind == "polyline":
        r = path_of(g["pts"], g.get("zs"), g.get("bulges"))
        return r if isinstance(r, str) else [(r, g.get("closed", False))]
    if kind == "polygon":
        r = path_of(g["pts"], g.get("zs"), g.get("bulges"))
        return r if isinstance(r, str) else [(r, True)]
    return None


def ring_of(kind, g):
    if kind == "polygon":
        pass
    elif kind == "polyline":
        if not g.get("closed"):
            return "not_closed"
    else:
        return "wrong_kind"
    if g.get("bulges") and any(b != 0 for b in g["bulges"]):
        return "arc_edge"
    if len(g["pts"]) < 3:
        return "too_few"
    return [xy(p) for p in g["pts"]]


def gather(d):
    by_uid = {u: (k, l, g) for u, (k, l, g) in zip(UIDS, ENTITIES)}
    left, points, breaklines = [], [], []
    for role in ("points", "breaklines"):
        src = d.get(role, {})
        layers, missing = layer_set(src.get("layers", []))
        uids = src.get("uids", [])
        left += [(m, role, "missing") for m in missing]
        left += [(u, role, "missing") for u in uids if u not in by_uid]
        for uid, (kind, layer, g) in zip(UIDS, ENTITIES):
            named = uid in uids
            if not named and layer not in layers:
                continue
            if role == "points":
                if kind != "point":
                    if named:
                        left.append((uid, role, "wrong_kind"))
                    continue
                if g.get("z") is None:
                    left.append((uid, role, "no_height"))
                else:
                    points.append((g["p"]["x"], g["p"]["y"], g["z"], uid))
            else:
                r = lines_of(kind, g)
                if r is None:
                    if named:
                        left.append((uid, role, "wrong_kind"))
                elif isinstance(r, str):
                    left.append((uid, role, r))
                else:
                    breaklines += [(pts, closed, uid) for pts, closed in r]

    def ring(uid, role):
        if uid not in by_uid:
            left.append((uid, role, "missing"))
            return None
        r = ring_of(by_uid[uid][0], by_uid[uid][2])
        if isinstance(r, str):
            left.append((uid, role, r))
            return None
        return (r, uid)

    boundary = ring(d["boundary"], "boundary") if "boundary" in d else None
    voids = [r for r in (ring(v, "voids") for v in d.get("voids", [])) if r]
    return {"points": points, "breaklines": breaklines, "boundary": boundary, "voids": voids,
            "tolerance": d.get("tolerance"), "maxEdge": d.get("maxEdge")}, left


def digest(g):
    inp = {
        "points": [{"x": float(x), "y": float(y), "z": float(z), "source": s} for x, y, z, s in g["points"]],
        "breaklines": [{"points": [[float(c) for c in p] for p in pts], "closed": closed, "source": s}
                       for pts, closed, s in g["breaklines"]],
        "voids": [{"points": [[float(c) for c in p] for p in r], "source": s} for r, s in g["voids"]],
        "edge": {"kind": "maxEdge", "length": float(g["maxEdge"])} if g["maxEdge"] is not None else {"kind": "none"},
    }
    if g["boundary"]:
        r, s = g["boundary"]
        inp["boundary"] = {"points": [[float(c) for c in p] for p in r], "source": s}
    if g["tolerance"] is not None:
        inp["tolerance"] = float(g["tolerance"])
    text = protocol_cases.canonical({"algorithm": ALGORITHM, "input": inp})
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


# ── The surface (Triangle, ADR 0900 §2–§7) ──────────────────────────────────────────────────────────────────────────

class Unbuildable(Exception):
    def __init__(self, code, at=None):
        super().__init__(code)
        self.code, self.at = code, at


def cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def build(g):
    tol = g["tolerance"] if g["tolerance"] is not None else TOLERANCE
    xy_, z, at, merged = [], [], {}, 0

    def corner(x, y, h):
        nonlocal merged
        if (x, y) in at:
            i = at[(x, y)]
            if abs(z[i] - h) > tol:
                raise Unbuildable("height_ambiguous", [x, y])
            merged += 1
            return i
        at[(x, y)] = len(xy_)
        xy_.append((x, y))
        z.append(h)
        return at[(x, y)]

    for x, y, h, _ in g["points"]:
        corner(x, y, h)
    segs, is_break = [], []
    for pts, closed, _ in g["breaklines"]:
        ids = [corner(x, y, h) for x, y, h in pts]
        n = len(ids)
        for k in range(n - 1 + (1 if closed else 0)):
            segs.append((ids[k], ids[(k + 1) % n]))
    if len(xy_) < 3 or all(cross(xy_[0], xy_[1], p) == 0 for p in xy_[2:]):
        raise Unbuildable("too_few_points")
    # A corner exactly on a breakline piece splits it; its height must be the piece's there.
    for a, b in segs:
        for i, c in enumerate(xy_):
            if i in (a, b) or cross(xy_[a], xy_[b], c) != 0:
                continue
            (ax, ay), (bx, by) = xy_[a], xy_[b]
            t = ((F(c[0]) - F(ax)) * (F(bx) - F(ax)) + (F(c[1]) - F(ay)) * (F(by) - F(ay))) / (
                (F(bx) - F(ax)) ** 2 + (F(by) - F(ay)) ** 2)
            if 0 < t < 1 and abs(z[i] - float(F(z[a]) + t * (F(z[b]) - F(z[a])))) > tol:
                raise Unbuildable("height_conflict", [c[0], c[1]])
    first = tri.triangulate(xy_, segs)
    rings = ([g["boundary"][0]] if g["boundary"] else []) + [r for r, _ in g["voids"]]
    for ring in rings:
        ids = []
        for p in ring:
            if p not in at:
                h = tri.barycentric(xy_, z, first, p)
                if h is None:
                    raise Unbuildable("boundary_outside", [p[0], p[1]])
                at[p] = len(xy_)
                xy_.append(p)
                z.append(h)
            ids.append(at[p])
        for k in range(len(ids)):
            segs.append((ids[k], ids[(k + 1) % len(ids)]))
    voids = [r for r, _ in g["voids"]]
    tris = tri.triangulate(xy_, segs, [tri.inside_point(v) for v in voids] or None, clip=bool(g["boundary"]))
    trimmed = 0
    if g["maxEdge"] is not None:
        tris, trimmed = tri.trim(xy_, tris, set(map(frozenset, segs)), g["maxEdge"])
    pieces, split = tri.split_at_points(xy_, segs)
    return {"xy": xy_, "z": z, "tris": tris, "constrained": len({tuple(sorted(s)) for s in pieces}),
            "merged": merged, "split": split, "trimmed": trimmed, "voids": voids}


def figures(s):
    area = F(0)
    used = set()
    for t in s["tris"]:
        (ax, ay), (bx, by), (cx, cy) = [(F(s["xy"][i][0]), F(s["xy"][i][1])) for i in t]
        area += ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)) / 2
        used.update(t)
    hs = [s["z"][i] for i in used]
    out = {"vertices": len(s["xy"]), "triangles": len(s["tris"]), "constrained": s["constrained"], "area": float(area),
           "merged": s["merged"], "split": s["split"], "trimmed": s["trimmed"]}
    if hs:
        out["minZ"], out["maxZ"] = float(min(hs)), float(max(hs))
    return out


def inside_ring(ring, q):
    x, y = F(q[0]), F(q[1])
    c = False
    n = len(ring)
    for i in range(n):
        (ax, ay), (bx, by) = ring[i], ring[(i + 1) % n]
        ax, ay, bx, by = F(ax), F(ay), F(bx), F(by)
        if (ay > y) != (by > y) and x < ax + (y - ay) * (bx - ax) / (by - ay):
            c = not c
    return c


def height(s, q):
    h = tri.barycentric(s["xy"], s["z"], s["tris"], q)
    if h is not None:
        return {"x": float(q[0]), "y": float(q[1]), "z": h, "place": "inside"}
    place = "void" if any(inside_ring(v, q) for v in s["voids"]) else "outside"
    return {"x": float(q[0]), "y": float(q[1]), "place": place}


def section(s, line):
    edges = set()
    for t in s["tris"]:
        for k in range(3):
            edges.add(tuple(sorted((t[k], t[(k + 1) % 3]))))
    pts = [[F(c) for c in s["xy"][i]] for i in range(len(s["xy"]))]
    out, station = [], 0.0
    for k in range(len(line) - 1):
        p, q = [F(c) for c in line[k]], [F(c) for c in line[k + 1]]
        d = [q[0] - p[0], q[1] - p[1]]
        length = float(d[0] ** 2 + d[1] ** 2) ** 0.5
        if length == 0:
            continue
        at = [(F(0), None), (F(1), None)]
        for ia, ib in sorted(edges):
            a, b = pts[ia], pts[ib]
            r = [b[0] - a[0], b[1] - a[1]]
            denom = d[0] * r[1] - d[1] * r[0]
            ap = [a[0] - p[0], a[1] - p[1]]
            if denom == 0:
                if ap[0] * d[1] - ap[1] * d[0] != 0:
                    continue
                for c, zc in ((a, s["z"][ia]), (b, s["z"][ib])):
                    t = ((c[0] - p[0]) * d[0] + (c[1] - p[1]) * d[1]) / (d[0] ** 2 + d[1] ** 2)
                    if 0 <= t <= 1:
                        at.append((t, zc))
                continue
            t = (ap[0] * r[1] - ap[1] * r[0]) / denom
            u = (ap[0] * d[1] - ap[1] * d[0]) / denom
            if 0 <= t <= 1 and 0 <= u <= 1:
                at.append((t, float(F(s["z"][ia]) + u * (F(s["z"][ib]) - F(s["z"][ia])))))
        at.sort(key=lambda e: e[0])
        kept = []
        for t, zz in at:
            if kept and t == kept[-1][0]:
                if kept[-1][1] is None:
                    kept[-1] = (t, zz)
                continue
            kept.append((t, zz))
        for i, (t, zz) in enumerate(kept):
            if out and i == 0:
                continue
            x, y = p[0] + t * d[0], p[1] + t * d[1]
            if zz is None:
                zz = tri.barycentric(s["xy"], s["z"], s["tris"], (x, y))
            point = {"station": station + float(t) * length, "x": float(x), "y": float(y)}
            if zz is not None:
                point["z"] = zz
            if i > 0:
                mid = (t + kept[i - 1][0]) / 2
                if tri.barycentric(s["xy"], s["z"], s["tris"], (p[0] + mid * d[0], p[1] + mid * d[1])) is None:
                    point["gap"] = True
            out.append(point)
        station += length
    return out


# ── The answers ─────────────────────────────────────────────────────────────────────────────────────────────────────

ROLE = {"points": "nokta", "breaklines": "kırık hat", "boundary": "sınır", "voids": "boşluk"}
WHY = {"missing": "çizimde yok", "no_height": "kotu eksik", "arc_edge": "yaylı kenarı var", "wrong_kind": "türü uymuyor",
       "not_closed": "kapalı değil", "too_few": "köşesi az"}
PROBLEM = {
    "too_few_points": "kotlu yer üçten az ya da hepsi bir doğru üstünde",
    "height_ambiguous": "bir yerde iki farklı kot var",
    "height_conflict": "bir nokta bir kırık hattın üstünde ama kotu hattınkinden farklı",
    "boundary_outside": "sınırın ya da bir boşluğun köşesi noktaların dışında",
}


def rust_float(v):
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def made(d):
    """(gathered, left, digest, surface or Unbuildable)"""
    g, left = gather(d)
    try:
        s = build(g)
    except Unbuildable as e:
        s = e
    return g, left, digest(g), s


def report(d):
    _, left, dig, s = made(d)
    unbuildable = isinstance(s, Unbuildable)
    built = d.get("built")
    state = "unbuildable" if unbuildable else ("new" if built is None else "current" if built == dig else "stale")
    out = {"id": d["id"], "name": d["name"], "state": state, "digest": dig}
    if built is not None:
        out["built"] = built
    if left:
        out["left"] = [{"id": i, "role": r, "why": w} for i, r, w in left]
    if unbuildable:
        out["problem"] = {"code": s.code, "message": PROBLEM[s.code], **({"at": s.at} if s.at else {})}
    else:
        out["figures"] = figures(s)
    return out


def warnings(d):
    _, left = gather(d)
    return [{"code": "terrain_source_left", "message": f"{i} ({ROLE[r]}) kullanılamadı: {WHY[w]}.", "path": r}
            for i, r, w in left]


def unbuildable_error(d):
    _, _, _, s = made(d)
    place = f" (yer: {rust_float(s.at[0])}, {rust_float(s.at[1])})" if s.at else ""
    return failed("terrain_unbuildable", f"Kaynaklardan yüzey kurulamıyor: {PROBLEM[s.code]}{place}.")


def accepted(d):
    d = copy.deepcopy(d)
    d["built"] = made(d)[2]
    return d


def completed(output, warns=()):
    return {"status": "completed", "output": output, "warnings": list(warns)}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path is not None:
        e["path"] = path
    return {"status": "failed", "error": e}


def step(op, input_, result, expect=None, within=None):
    s = {"op": op, "input": input_, "result": result}
    if expect is not None:
        s["expect"] = expect
    if within is not None:
        s["within"] = within
    return s


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
WRITTEN = {"canUndo": False, "canRedo": False, "dirty": True, "revision": "changed"}
NOT_ROAD = failed("not_a_road_project", "Arazi yüzeyi yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
    "path": "expectedRevision", "revision": "$current"}}
W = 1e-9


def road_of(surfaces):
    return {"surfaces": surfaces} if surfaces else None


OLD = accepted({"id": "t-1", "name": "Eski zemin", "points": {"uids": [Z1, Z2, Z3, Z4]}})


def file(command, title, note, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": base, "cases": cases}


# ── road.terrains.create ────────────────────────────────────────────────────────────────────────────────────────────

def create_cases():
    cases = []

    def written(name, input_, d, note=None, op="execute"):
        """A create that writes d (the new definition, accepted) after OLD."""
        d = accepted(d)
        out = {"surfaces": [OLD, d], "surface": report(d), "revision": "$current"}
        expect = {**WRITTEN, "road": road_of([OLD, d])} if op == "execute" else {**UNCHANGED, "road": road_of([OLD])}
        result = completed(out, warnings(d)) if op != "validate" else completed(None, warnings(d))
        cases.append({"name": name, **({"note": note} if note else {}), "steps": [step(op, input_, result, expect, W)]})

    def refused(name, input_, result, setup_=None, expect=None):
        c = {"name": name, "steps": [step("execute", input_, result, expect or {**UNCHANGED, "road": road_of([OLD])})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    pts = {"layers": ["zemin"]}
    brk = {"layers": ["kirik"]}
    written("Noktalar katmanından: kimliği ilk boş t-2, kabul edilmiş", {"name": "Mevcut zemin", "points": pts},
            {"id": "t-2", "name": "Mevcut zemin", "points": pts},
            "kotsuz nokta kullanılamaz (terrain_source_left), katmandaki çizgi nokta sayılmaz")
    written("Noktalar ve kırık hatlar", {"name": "Zemin", "points": pts, "breaklines": brk},
            {"id": "t-2", "name": "Zemin", "points": pts, "breaklines": brk})
    written("Sınırla", {"name": "Zemin", "points": pts, "breaklines": brk, "boundary": BOUND},
            {"id": "t-2", "name": "Zemin", "points": pts, "breaklines": brk, "boundary": BOUND})
    written("Sınır ve iki boşluk: alan ve kapalı çoklu çizgi",
            {"name": "Zemin", "points": pts, "breaklines": brk, "boundary": BOUND, "voids": [VOID1, VOID2]},
            {"id": "t-2", "name": "Zemin", "points": pts, "breaklines": brk, "boundary": BOUND, "voids": [VOID1, VOID2]})
    written("En uzun dış kenar 35 m: dış üçgenler atılır", {"name": "Zemin", "points": pts, "maxEdge": 35.0},
            {"id": "t-2", "name": "Zemin", "points": pts, "maxEdge": 35.0})
    written("Kendi kimliği ve toleransı", {"name": "Zemin", "id": "zemin-a", "points": pts, "tolerance": 0.01},
            {"id": "zemin-a", "name": "Zemin", "points": pts, "tolerance": 0.01})
    written("Grup kaynağı: grubun katmanının noktası da", {"name": "Zemin", "points": {"layers": ["zemin", "arazi"]}},
            {"id": "t-2", "name": "Zemin", "points": {"layers": ["zemin", "arazi"]}})
    written("Nesne kimlikleriyle noktalar", {"name": "Zemin", "points": {"uids": [Z1, Z2, Z3, Z4, Z5]}},
            {"id": "t-2", "name": "Zemin", "points": {"uids": [Z1, Z2, Z3, Z4, Z5]}})
    written("Çizimde olmayan katman ve nesne: uyarıyla kurulur",
            {"name": "Zemin", "points": {"layers": ["zemin", "yok"], "uids": [U(999)]}},
            {"id": "t-2", "name": "Zemin", "points": {"layers": ["zemin", "yok"], "uids": [U(999)]}})
    written("Nokta listesinde çizgi: türü uymuyor", {"name": "Zemin", "points": {"layers": ["zemin"], "uids": [BL2]}},
            {"id": "t-2", "name": "Zemin", "points": {"layers": ["zemin"], "uids": [BL2]}})
    written("Yaylı kırık hat ve kotsuz kırık hat kullanılmaz",
            {"name": "Zemin", "points": pts, "breaklines": {"uids": [BL1, ARC, OPEN]}},
            {"id": "t-2", "name": "Zemin", "points": pts, "breaklines": {"uids": [BL1, ARC, OPEN]}})
    written("Açık çoklu çizgi boşluk olmaz, olmayan sınır yok sayılır",
            {"name": "Zemin", "points": pts, "boundary": U(998), "voids": [OPEN, VOID1]},
            {"id": "t-2", "name": "Zemin", "points": pts, "boundary": U(998), "voids": [OPEN, VOID1]})
    written("Aynı yerde aynı kot: tek köşe", {"name": "Zemin", "points": {"layers": ["zemin"], "uids": [DUP]}, "tolerance": 0.5},
            {"id": "t-2", "name": "Zemin", "points": {"layers": ["zemin"], "uids": [DUP]}, "tolerance": 0.5})
    written("Plan yazmaz", {"name": "Zemin", "points": pts}, {"id": "t-2", "name": "Zemin", "points": pts}, op="plan")
    written("Doğrulama yazmaz", {"name": "Zemin", "points": pts}, {"id": "t-2", "name": "Zemin", "points": pts},
            op="validate")
    refused("Yol projesi değil", {"name": "Zemin", "points": pts}, NOT_ROAD, setup(work="map"), {**UNCHANGED, "road": None})
    refused("Boş ad", {"name": "", "points": pts},
            failed("invalid_terrain", "Yüzey kurallarına uymuyor: yüzeyin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter.", "name"))
    refused("Aynı ad, büyük küçük harf ayrımsız", {"name": "eski ZEMIN", "points": pts},
            failed("invalid_terrain", "Yüzey kurallarına uymuyor: bu adda başka bir yüzey var.", "name"))
    refused("Aynı kimlik", {"name": "Zemin", "id": "t-1", "points": pts},
            failed("invalid_terrain", "Yüzey kurallarına uymuyor: bu kimlikte başka bir yüzey var.", "id"))
    refused("Kaynaksız", {"name": "Zemin"},
            failed("invalid_terrain", "Yüzey kurallarına uymuyor: yüzeyin noktası ya da kırık hattı olmalı.", "points"))
    refused("Tolerans 1 m'den büyük", {"name": "Zemin", "points": pts, "tolerance": 2.0},
            failed("invalid_terrain", "Yüzey kurallarına uymuyor: kot toleransı 0'dan büyük, en çok 1 m olmalı.", "tolerance"))
    refused("Sınır boşluk da", {"name": "Zemin", "points": pts, "boundary": BOUND, "voids": [BOUND]},
            failed("invalid_terrain", "Yüzey kurallarına uymuyor: sınır aynı zamanda boşluk olamaz.", "voids"))
    refused("Geçersiz beklenen sürüm", {"name": "Zemin", "points": pts, "expectedRevision": "on"},
            failed("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    refused("Eski sürüme göre hazırlanmış", {"name": "Zemin", "points": pts, "expectedRevision": "41"}, CONFLICT)
    for name, d in [
        ("İki nokta yüzey olmaz", {"name": "Zemin", "points": {"uids": [Z1, Z2]}}),
        ("Bir yerde iki kot", {"name": "Zemin", "points": {"layers": ["zemin"], "uids": [DUP]}}),
        ("Kırık hattın üstünde başka kotlu nokta", {"name": "Zemin", "points": {"layers": ["zemin"], "uids": [ZCONF]},
                                                   "breaklines": {"uids": [BL1]}}),
        ("Sınır noktaların dışında", {"name": "Zemin", "points": pts, "boundary": BIG}),
    ]:
        refused(f"Kurulamıyor: {name.lower()}", d, unbuildable_error({"id": "t-2", **d}))
    return file("road.terrains.create", "Arazi yüzeyi oluştur (plan/road.md Y2.3, ADR 0900 §11)",
                "scripts/fixtures/road_terrain_command_cases.py yazar; kaynaklar ADR 0900 §10'un kuralıyla okunur, yüzey "
                "Triangle'ındır, ölçüler kesirlerle, özet kanonik JSON'un SHA-256'sıdır; KentOS kodu yok. `expect.road` "
                "projenin yol ayarlarıdır. Yüzey proje ayarıdır: geri alma adımı yazılmaz.",
                setup([OLD]), cases)


# ── road.terrains.edit ──────────────────────────────────────────────────────────────────────────────────────────────

def edit_cases():
    base = accepted({"id": "t-2", "name": "Zemin", "points": {"layers": ["zemin"]}, "breaklines": {"layers": ["kirik"]},
                     "boundary": BOUND})
    stale = {**base, "id": "t-3", "name": "Eskimiş", "built": "0" * 64}
    weak = {"id": "t-4", "name": "Az nokta", "points": {"uids": [Z1, Z2]}}
    start = [OLD, base, stale, weak]
    cases = []

    def case(name, input_, after, surface, warns=None, op="execute", note=None):
        if warns is None:
            # A set or an accept says what its surface could not use; a remove says nothing.
            d = next((x for x in after if surface and x["id"] == surface["id"]), None)
            warns = warnings(d) if d else []
        out = {"surfaces": after, "revision": "$current"}
        if surface is not None:
            out["surface"] = surface
        # A write that leaves the settings as they were changes nothing: the drawing stays clean.
        writes = op == "execute" and after != start
        expect = {**WRITTEN, "road": road_of(after)} if writes else {**UNCHANGED, "road": road_of(start)}
        result = completed(out, warns) if op != "validate" else completed(None, warns)
        cases.append({"name": name, **({"note": note} if note else {}), "steps": [step(op, input_, result, expect, W)]})

    def refused(name, input_, result, setup_=None, road=start):
        c = {"name": name, "steps": [step("execute", input_, result, {**UNCHANGED, "road": road_of(road)})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    def put(i, d):
        out = list(start)
        out[i] = d
        return out

    renamed = {**base, "name": "Mevcut zemin"}
    del renamed["built"]
    kept = {**renamed, "built": base["built"]}
    case("Adı değişir, özet kalır: güncel", {"operation": "set", "surface": renamed}, put(1, kept), report(kept))
    widened = {k: v for k, v in base.items() if k != "built"}
    widened["voids"] = [VOID1]
    w2 = {**widened, "built": base["built"]}
    case("Boşluk eklenir: kabul edilene dek eskidi", {"operation": "set", "surface": widened}, put(1, w2), report(w2))
    me = {k: v for k, v in base.items() if k != "built"}
    me["maxEdge"] = 25.0
    me2 = {**me, "built": base["built"]}
    case("Kenar yöntemi değişir: eskidi", {"operation": "set", "surface": me}, put(1, me2), report(me2))
    lost = {k: v for k, v in base.items() if k != "built"}
    lost["points"] = {"layers": ["zemin", "yok"]}
    lost2 = {**lost, "built": base["built"]}
    case("Olmayan katman: uyarıyla yazılır", {"operation": "set", "surface": lost}, put(1, lost2), report(lost2))
    acc = accepted(stale)
    case("Eskimiş yüzey kabul edilir", {"operation": "accept", "id": "t-3"}, put(2, acc), report(acc))
    case("Güncel yüzeyi kabul etmek bir şey değiştirmez", {"operation": "accept", "id": "t-2"}, start, report(base))
    case("Kaldır", {"operation": "remove", "id": "t-3"}, [OLD, base, weak], None)
    case("Plan yazmaz", {"operation": "accept", "id": "t-3"}, put(2, acc), report(acc), op="plan")
    case("Doğrulama yazmaz", {"operation": "remove", "id": "t-1"}, [base, stale, weak], None, op="validate")
    cases.append({"name": "Son yüzey kalkınca yol ayarı da kalkar", "setup": setup([OLD]),
                  "steps": [step("execute", {"operation": "remove", "id": "t-1"},
                                 completed({"surfaces": [], "revision": "$current"}), {**WRITTEN, "road": None})]})
    refused("Yol projesi değil", {"operation": "remove", "id": "t-1"}, NOT_ROAD, setup(work="map"), None)
    refused("set'te yüzey yok", {"operation": "set"}, failed("no_terrain", "Yazılacak yüzeyin tanımını (surface) verin.", "surface"))
    refused("accept'te kimlik yok", {"operation": "accept"}, failed("no_id", "Yüzeyin kimliğini (id) verin.", "id"))
    refused("remove'da boş kimlik", {"operation": "remove", "id": ""}, failed("no_id", "Yüzeyin kimliğini (id) verin.", "id"))
    refused("set özet yazmaz", {"operation": "set", "surface": base},
            failed("invalid_terrain", "Özet (built) yazılmaz; yüzeyi kabul etmek için accept kullanın.", "surface.built"))
    refused("set'te geçersiz en uzun kenar", {"operation": "set", "surface": {**renamed, "maxEdge": -1.0}},
            failed("invalid_terrain", "Yüzey kurallarına uymuyor: en uzun dış kenar 0'dan büyük olmalı.", "surface.maxEdge"))
    refused("set'te olmayan yüzey", {"operation": "set", "surface": {**renamed, "id": "t-9", "name": "Yeni"}},
            failed("unknown_terrain", "“t-9” kimlikli yüzey projede yok. Var olan bir yüzeyin kimliğini verin.", "surface.id"))
    refused("accept'te olmayan yüzey", {"operation": "accept", "id": "t-9"},
            failed("unknown_terrain", "“t-9” kimlikli yüzey projede yok. Var olan bir yüzeyin kimliğini verin.", "id"))
    refused("Kurulamayan yüzey kabul edilmez", {"operation": "accept", "id": "t-4"}, unbuildable_error(weak))
    refused("Eski sürüme göre hazırlanmış", {"operation": "accept", "id": "t-3", "expectedRevision": "41"}, CONFLICT)
    refused("Geçersiz beklenen sürüm", {"operation": "remove", "id": "t-3", "expectedRevision": "-1"},
            failed("invalid_revision", "Beklenen sürüm “-1” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    return file("road.terrains.edit", "Arazi yüzeyini düzenle (plan/road.md Y2.3, ADR 0900 §11)",
                "scripts/fixtures/road_terrain_command_cases.py yazar; KentOS kodu yok. `expect.road` projenin yol ayarlarıdır.",
                setup(start), cases)


# ── road.terrains.query ─────────────────────────────────────────────────────────────────────────────────────────────

def query_cases():
    full = accepted({"id": "t-2", "name": "Zemin", "points": {"layers": ["zemin"]}, "breaklines": {"layers": ["kirik"]},
                     "boundary": BOUND, "voids": [VOID1, VOID2]})
    open_ = accepted({"id": "t-3", "name": "Açık zemin", "points": {"layers": ["zemin", "arazi"]}, "maxEdge": 35.0})
    stale = {**open_, "id": "t-4", "name": "Eskimiş", "built": "f" * 64}
    weak = {"id": "t-5", "name": "Az nokta", "points": {"uids": [Z1, Z2]}}
    fresh = {"id": "t-6", "name": "Yeni", "points": {"layers": ["zemin"]}}
    start = [OLD, full, open_, stale, weak, fresh]
    cases = []
    s_full = build(gather(full)[0])
    s_open = build(gather(open_)[0])

    def q(name, input_, out, note=None):
        cases.append({"name": name, **({"note": note} if note else {}),
                      "steps": [step("execute", input_, completed(out), {**UNCHANGED, "road": road_of(start)}, W)]})

    def refused(name, input_, result, setup_=None):
        c = {"name": name, "steps": [step("execute", input_, result, {**UNCHANGED})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    q("Bütün yüzeyler ve durumları", {}, {"surfaces": [report(d) for d in start]},
      "güncel, eskimiş, kurulamayan ve hiç kabul edilmemiş yüzey")
    q("Kimliğiyle bir yüzey", {"surface": "t-2"}, {"surfaces": [report(full)]})
    q("Adıyla, büyük küçük harf ayrımsız", {"surface": "açık ZEMİN".replace("İ", "i")}, {"surfaces": [report(open_)]})
    places = [[20.0, 15.0], [10.0, 10.0], [21.0, 10.0], [30.0, 20.5], [1.0, 1.0], [50.0, 50.0], [12.5, 7.25]]
    q("Kotlar: içeride, boşlukta, sınırın dışında", {"surface": "t-2", "at": places},
      {"surfaces": [report(full)], "heights": [height(s_full, p) for p in places]})
    q("Kırık hattın üstünde kot hattınkidir", {"surface": "t-2", "at": [[20.0, 5.0], [11.0, 15.0]]},
      {"surfaces": [report(full)], "heights": [height(s_full, p) for p in [[20.0, 5.0], [11.0, 15.0]]]})
    q("Köşede kot noktanın kotudur", {"surface": "t-3", "at": [[21.0, 24.0], [31.0, 10.0]]},
      {"surfaces": [report(open_)], "heights": [height(s_open, p) for p in [[21.0, 24.0], [31.0, 10.0]]]})
    for name, line, s, d in [
        ("Kesit: boşluğu geçen hat", [[3.0, 10.0], [37.0, 10.0]], s_full, full),
        ("Kesit: sınırın dışından başlayıp biten hat", [[-1.0, 20.0], [41.0, 20.0]], s_full, full),
        ("Kesit: iki parçalı hat", [[4.0, 4.0], [20.0, 20.0], [36.0, 4.0]], s_full, full),
        ("Kesit: kırık hat boyunca", [[5.0, 15.0], [35.0, 15.0]], s_full, full),
        ("Kesit: açık zeminde çapraz", [[0.0, 0.0], [40.0, 30.0]], s_open, open_),
        ("Kesit: bütünüyle dışarıda", [[50.0, 0.0], [60.0, 10.0]], s_full, full),
        ("Kesit: sıfır boylu parça atlanır", [[4.0, 25.0], [4.0, 25.0], [36.0, 25.0]], s_full, full),
    ]:
        q(name, {"surface": d["id"], "along": line}, {"surfaces": [report(d)], "section": section(s, line)})
    q("Kot ve kesit birlikte", {"surface": "t-3", "at": [[20.0, 15.0]], "along": [[0.0, 15.0], [40.0, 15.0]]},
      {"surfaces": [report(open_)], "heights": [height(s_open, [20.0, 15.0])],
       "section": section(s_open, [[0.0, 15.0], [40.0, 15.0]])})
    q("Kurulamayan yüzeyde kot yok", {"surface": "t-5", "at": [[10.0, 10.0]]}, {"surfaces": [report(weak)]})
    q("Kesit: sınırın ve boşluğun kenarı boyunca", {"surface": "t-2", "along": [[2.0, 2.0], [38.0, 2.0], [38.0, 28.0]]},
      {"surfaces": [report(full)], "section": section(s_full, [[2.0, 2.0], [38.0, 2.0], [38.0, 28.0]])})
    refused("Yol projesi değil", {}, NOT_ROAD, setup(work="map"))
    refused("Olmayan yüzey", {"surface": "yok"}, failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    refused("Yüzeysiz kot", {"at": [[1.0, 1.0]]},
            failed("no_terrain", "Kot ve kesit bir yüzeyde sorulur: yüzeyin kimliğini ya da adını (surface) verin.", "surface"))
    refused("Tek yerli hat", {"surface": "t-2", "along": [[1.0, 1.0]]},
            failed("invalid_places", "Kesit hattı en az iki yerden oluşur.", "along"))
    return file("road.terrains.query", "Arazi yüzeyini sorgula (plan/road.md Y2.3, ADR 0900 §11)",
                "scripts/fixtures/road_terrain_command_cases.py yazar; kotlar ve kesitler kesirlerle Triangle'ın üçgenleri "
                "üstünde; KentOS kodu yok. Sorgu hiçbir şey yazmaz.",
                setup(start), cases)


def main():
    files = {"road.terrains.create.json": create_cases(), "road.terrains.edit.json": edit_cases(),
             "road.terrains.query.json": query_cases()}
    check = "--check" in sys.argv[1:]
    bad = False
    for name, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text("utf-8") != text:
                print(f"güncel değil: {path.relative_to(ROOT)}")
                bad = True
        else:
            path.write_text(text, "utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(data['cases'])} durum)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
