#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.hauls.network.json: the shared cases of KentOS Yol's haul road network (plan/road.md
Y49, docs/adr/0955; ROAD-HAUL-06), their expectations built here without KentOS code. The drawing, its alignments,
corridors, volumes and diagrams are road_haul_plan_cases.py's; the roads are this script's lines and polylines on the
layer "tasima", placed by the main alignment's own reference (road_alignment_cases.py in mpmath): a road's end set at a
station and offset of it. Distances, feet and crossings are GEOS's (shapely); an arc's length is R·|sweep| exactly,
its geometry chords of 0.0005 rad; the shortest way along the joined roads is this script's Dijkstra.

The rules, as Trimble Business Center joins haul roads: two roads join only where an end of one lies within the
connection tolerance of the other, at its end (merged) or along it (a T, the other cut there); bodies that only
cross do not join and are told. A road end within the reach of the alignment's centre line joins it at its foot's
station. A site joins its nearest road within the tolerance; its access is the shortest way to the alignment, the
smaller station where two are as short.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with scipy \\
        --with shapely python3 scripts/fixtures/road_haul_network_cases.py [--check]
"""

import copy
import heapq
import json
import math
import sys
from pathlib import Path

from shapely.geometry import LineString, Point

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_command_cases as acmd  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402
import road_haul_command_cases as hc  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.hauls.network.json"
completed, failed, step, case = hc.completed, hc.failed, hc.step, hc.case
UNCHANGED = hc.UNCHANGED
W = 1e-3
LAYER = "tasima"
AXIS = pcmd.ALIGNED["g-1"]


def spot(s, o):
    """The point `o` metres right of the main alignment at station `s`, to the millimetre."""
    p = acmd.place(AXIS, s)
    t = p["direction"]
    return [round(p["x"] + o * math.sin(t), 3), round(p["y"] - o * math.cos(t), 3)]


def ahead(p, s, along, side=0.0):
    """`p` moved `along` the alignment's direction at `s` and `side` to its right."""
    t = acmd.place(AXIS, s)["direction"]
    return [round(p[0] + along * math.cos(t) + side * math.sin(t), 3), round(p[1] + along * math.sin(t) - side * math.cos(t), 3)]


# The roads: (kind, points, bulges). R1 an access at station 100 to a waste site 80 m right; R2 and R3 a way to a
# borrow site 60 m left of station 450 (R3 bending on an arc), R4 a T off R3's body to station 520; R5 crosses R1's body
# and joins nothing; R6 ends 0.8 m short of R2; R7 crosses the alignment's body at station 300.
R1 = [spot(100.0, 6.0), spot(100.0, 80.0)]
R2 = [spot(450.0, -7.0), spot(450.0, -60.0)]
R3_END = ahead(spot(450.0, -60.0), 450.0, 70.0, -40.0)
R3 = [spot(450.0, -60.0), R3_END]
# A quarter turn: the bulge tan(π/8) bends R3 left of its chord.
R3_BULGE = [math.tan(math.pi / 8)]
R4 = [None, spot(520.0, -8.0)]
R5 = [ahead(spot(100.0, 41.0), 100.0, -30.0), ahead(spot(100.0, 41.0), 100.0, 30.0)]
R6 = [ahead(spot(450.0, -30.0), 450.0, -60.0), ahead(spot(450.0, -30.0), 450.0, -0.8)]
R7 = [spot(300.0, -30.0), spot(300.0, 30.0)]


def arc_points(a, b, bulge, step=0.0005):
    """An arc from `a` to `b` of `bulge` as chords of `step` rad, and its exact length."""
    chord = math.dist(a, b)
    sweep = 4 * math.atan(bulge)
    r = chord / (2 * abs(math.sin(sweep / 2)))
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    ux, uy = (b[0] - a[0]) / chord, (b[1] - a[1]) / chord
    h = r * math.cos(sweep / 2) * (1 if sweep > 0 else -1)
    c = (mx - uy * h, my + ux * h)
    start = math.atan2(a[1] - c[1], a[0] - c[0])
    n = max(1, math.ceil(abs(sweep) / step))
    pts = [list(a)] + [[c[0] + r * math.cos(start + sweep * k / n), c[1] + r * math.sin(start + sweep * k / n)]
                       for k in range(1, n)] + [list(b)]
    return pts, r * abs(sweep)


def r4_start():
    """R4 starts on R3's body, a third along its arc (the arc's own point, to the millimetre)."""
    pts, _ = arc_points(R3[0], R3[1], R3_BULGE[0])
    line = LineString(pts)
    q = line.interpolate(line.length / 3)
    return [round(q.x, 3), round(q.y, 3)]


R4[0] = r4_start()
ROADS = [("line", R1, None), ("line", R2, None), ("polyline", R3, R3_BULGE), ("line", R4, None),
         ("line", R5, None), ("polyline", R6, None), ("line", R7, None)]

# The sites off the road: the waste site at R1's end, the borrow site at R3's end, one on R6 (cut off), one far.
SITE_W = {"name": "Dere döküm", "kind": "waste", "station": 100.0, "volume": 30000.0, "at": {"x": R1[1][0], "y": R1[1][1]}}
SITE_B = {"name": "Tepe ödünç", "kind": "borrow", "station": 450.0, "volume": 5000.0, "at": {"x": R3_END[0], "y": R3_END[1]}}
R6_MID = [round((R6[0][0] + R6[1][0]) / 2, 3), round((R6[0][1] + R6[1][1]) / 2, 3)]
SITE_C = {"name": "Kopuk döküm", "kind": "waste", "station": 450.0, "volume": 2000.0, "at": {"x": R6_MID[0], "y": R6_MID[1]}}
FAR = ahead(spot(300.0, 120.0), 300.0, 0.0)
SITE_F = {"name": "Uzak döküm", "kind": "waste", "station": 300.0, "volume": 1000.0, "at": {"x": FAR[0], "y": FAR[1]}}
NETWORK = {"layers": [LAYER], "connect": 0.5, "reach": 10.0}


def entities(first_id):
    out = []
    for k, (kind, pts, bulges) in enumerate(ROADS):
        e = {"kind": kind, "id": first_id + k, "layerId": LAYER, "attrs": {}}
        if kind == "line":
            e.update({"a": {"x": pts[0][0], "y": pts[0][1]}, "b": {"x": pts[1][0], "y": pts[1][1]}})
        else:
            e["pts"] = [{"x": p[0], "y": p[1]} for p in pts]
            if bulges:
                e["bulges"] = bulges
        out.append(e)
    return out


def setup(hauls, volumes, corridors=hc.CORRIDORS, roads=True, work="road"):
    base = hc.setup(hauls, volumes, corridors, work=work)
    base["layers"] = base["layers"] + [grc.node(LAYER, "Taşıma yolları")]
    if roads:
        first = max(e["id"] for e in base["entities"]) + 1
        ents = entities(first)
        base["entities"] = base["entities"] + ents
        base["uids"] = base["uids"] + [f"0192f5a0-7c3e-7d4a-9b1e-{920000000000 + k:012d}" for k in range(len(ents))]
    return base


# ── The network ───────────────────────────────────────────────────────────────────────────────────────────────────────

class Net:
    """The roads as this reference reads them: points, geometry, exact lengths."""

    def __init__(self, roads=ROADS):
        self.pts, self.lengths = [], []
        for kind, pts, bulges in roads:
            if bulges and abs(bulges[0]) > 1e-12:
                p, length = arc_points(pts[0], pts[1], bulges[0])
            else:
                p, length = [list(x) for x in pts], sum(math.dist(pts[i], pts[i + 1]) for i in range(len(pts) - 1))
            self.pts.append(p)
            self.lengths.append(length)
        self.lines = [LineString(p) for p in self.pts]

    def end(self, r, e):
        return self.pts[r][-1] if e else self.pts[r][0]

    def along(self, r, q):
        """Length along road r of its point nearest q, scaled to its exact length."""
        line = self.lines[r]
        return line.project(Point(q)) * self.lengths[r] / line.length

    def at(self, r, s):
        line = self.lines[r]
        q = line.interpolate(s * line.length / self.lengths[r])
        return [q.x, q.y]


def axis_foot(p):
    got = acmd.locate(AXIS, p)
    n = got.get("nearest")
    return None if n is None else (n["at"]["station"], n["offset"])


def axis_line():
    n = 40000
    return LineString([[acmd.place(AXIS, float(AXIS.length) * k / n)[c] for c in ("x", "y")] for k in range(n + 1)])


AXIS_LINE = axis_line()


def read(sites, connect, reach, roads=ROADS):
    if not roads:
        return Net([]), [], 0, [], [{"joined": False, "nearest": None} for _ in sites]
    net = Net(roads)
    n = len(net.pts)
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def join(a, b):
        a, b = find(a), find(b)
        if a != b:
            parent[max(a, b, key=str)] = min(a, b, key=str)

    cuts = {r: [(0.0, ("end", r, False)), (net.lengths[r], ("end", r, True))] for r in range(n)}
    touched = set()
    joins = 0
    counter = 0
    for r in range(n):
        for e in (False, True):
            p = net.end(r, e)
            me = ("end", r, e)
            if e and math.dist(p, net.end(r, False)) <= connect and net.lengths[r] > connect:
                join(me, ("end", r, False))
                touched.update({me, ("end", r, False)})
            for o in range(n):
                if o == r:
                    continue
                near = [x for x in (False, True) if math.dist(p, net.end(o, x)) <= connect]
                if near:
                    x = min(near, key=lambda x: math.dist(p, net.end(o, x)))
                    join(me, ("end", o, x))
                    touched.add(me)
                    if r < o:
                        joins += 1
                    continue
                if net.lines[o].distance(Point(p)) <= connect:
                    counter += 1
                    k = ("cut", counter)
                    cuts[o].append((net.along(o, p), k))
                    join(me, k)
                    touched.add(me)
                    joins += 1
    junctions = []
    junction_of = {}
    for r in range(n):
        for e in (False, True):
            f = axis_foot(net.end(r, e))
            if f is not None and abs(f[1]) <= reach:
                junction_of[("end", r, e)] = len(junctions)
                junctions.append({"road": r, "end": e, "station": f[0], "km": acmd.km_text(1000.0 + f[0]), "offset": f[1]})
    site_nodes = []
    for p in sites:
        best = min(range(n), key=lambda r: net.lines[r].distance(Point(p)))
        d = net.lines[best].distance(Point(p))
        if d <= connect:
            counter += 1
            k = ("cut", counter)
            cuts[best].append((net.along(best, p), k))
            site_nodes.append((k, best, None))
        else:
            site_nodes.append((None, None, d))
    graph = {}
    for r in range(n):
        c = sorted(cuts[r], key=lambda x: (x[0], str(x[1])))
        for (s0, a), (s1, b) in zip(c, c[1:]):
            a, b = find(a), find(b)
            graph.setdefault(a, []).append((b, s1 - s0, r))
            graph.setdefault(b, []).append((a, s1 - s0, r))
    target = {}
    for k, j in junction_of.items():
        root = find(k)
        if root not in target or junctions[j]["station"] < junctions[target[root]]["station"]:
            target[root] = j
    out_sites = []
    for k, road, nearest in site_nodes:
        if k is None:
            out_sites.append({"joined": False, "nearest": nearest})
            continue
        start = find(k)
        dist, via = {start: 0.0}, {}
        heap = [(0.0, 0, start)]
        tick = 1
        while heap:
            d, _, u = heapq.heappop(heap)
            if d > dist[u]:
                continue
            for v, length, r in graph.get(u, []):
                if d + length < dist.get(v, math.inf):
                    dist[v] = d + length
                    via[v] = (u, r)
                    heapq.heappush(heap, (dist[v], tick, v))
                    tick += 1
        reached = [(dist[u], junctions[j]["station"], u, j) for u, j in target.items() if u in dist]
        if not reached:
            out_sites.append({"joined": False, "cut": road})
            continue
        best = min(reached, key=lambda x: (round(x[0], 6), x[1]))
        on, v = [], best[2]
        while v in via:
            v, r = via[v]
            if not on or on[-1] != r:
                on.append(r)
        if not on:
            on = [road]
        on.reverse()
        jn = junctions[best[3]]
        out_sites.append({"joined": True, "length": best[0], "station": jn["station"], "km": jn["km"],
                          "offset": jn["offset"], "roads": on})
    # Crossings along the bodies with no join near.
    problems = []
    for a in range(n):
        for b in range(a + 1, n):
            seen = []
            pa, pb = net.pts[a], net.pts[b]
            for i in range(len(pa) - 1):
                sa = LineString([pa[i], pa[i + 1]])
                for j in range(len(pb) - 1):
                    x = sa.intersection(LineString([pb[j], pb[j + 1]]))
                    if x.is_empty or x.geom_type != "Point":
                        continue
                    x = [x.x, x.y]
                    if any(math.dist(y, x) <= connect for y in seen):
                        continue
                    seen.append(x)
                    shared = {find(k) for _, k in cuts[a]} & {find(k) for _, k in cuts[b]}
                    near = any(find(k) in shared and math.dist(net.at(a, s), x) <= connect for s, k in cuts[a])
                    if not near:
                        problems.append(("crossing", [a, b], x, None))
    for r in range(n):
        x = net.lines[r].intersection(AXIS_LINE)
        pts = [] if x.is_empty else ([x] if x.geom_type == "Point" else list(x.geoms))
        seen = []
        for q in sorted(([p.x, p.y] for p in pts), key=lambda q: net.along(r, q)):
            if any(math.dist(net.end(r, e), q) <= reach for e in (False, True)) or any(math.dist(y, q) <= connect for y in seen):
                continue
            seen.append(q)
            problems.append(("axis", [r], q, axis_foot(q)[0]))
    for r in range(n):
        for e in (False, True):
            k = ("end", r, e)
            p = net.end(r, e)
            if k in touched or k in junction_of or any(math.dist(s, p) <= connect for s in sites):
                continue
            if any(find(("end", o, x)) == find(k) for o in range(n) for x in (False, True) if (o, x) != (r, e)):
                continue
            to_road = min((net.lines[o].distance(Point(p)) for o in range(n) if o != r), default=math.inf)
            f = axis_foot(p)
            gap = min(to_road, abs(f[1]) if f else math.inf)
            problems.append(("loose", [r, e], p, None if math.isinf(gap) else gap))
    return net, junctions, joins, problems, out_sites


def why(c, connect):
    if c["joined"]:
        return None
    if "cut" in c:
        return "bağlandığı yollar aksa ulaşmıyor"
    if c.get("nearest") is None:
        return "taşıma ağının katmanlarında yol yok"
    return f"en yakın taşıma yolu {c['nearest']:.2f} m uzakta; bağlantı toleransı {connect:.2f} m"


def unconnected_warning(name, j, words):
    return {"code": "haul_site_unconnected", "path": f"sites[{j}]",
            "message": f"“{name}” alanı taşıma ağıyla aksa bağlanmıyor ({words}); hesaba katılmadı."}


def placed(h):
    """The sites as a plan places them (docs/adr/0955): (station, access or None, unconnected) and the warnings."""
    sites = h.get("sites", [])
    off = [s for s in sites if "at" in s]
    if not off:
        return [(s["station"], None, False) for s in sites], []
    n = h["network"]
    _, _, _, _, conns = read([[s["at"]["x"], s["at"]["y"]] for s in off], n["connect"], n.get("reach", n["connect"]))
    out, warnings, k = [], [], 0
    for j, s in enumerate(sites):
        if "at" not in s:
            out.append((s["station"], None, False))
            continue
        c = conns[k]
        k += 1
        if c["joined"]:
            out.append((c["station"], c["length"], False))
        else:
            out.append((s["station"], None, True))
            warnings.append(unconnected_warning(s["name"], j, why(c, n["connect"])))
    return out, warnings


def answer(h, roads=ROADS):
    n = h["network"]
    connect, reach = n["connect"], n.get("reach", n["connect"])
    off = [(j, s) for j, s in enumerate(h.get("sites", [])) if "at" in s]
    net, junctions, joins, problems, conns = read([[s["at"]["x"], s["at"]["y"]] for _, s in off], connect, reach, roads)
    roads = []
    first = max(e["id"] for e in hc.setup()["entities"]) + 1
    for k in range(len(net.pts)):
        roads.append({"entity": first + k, "layer": LAYER, "length": net.lengths[k]})
    out_problems = []
    for kind, ids, at, extra in problems:
        if kind == "crossing":
            a, b = ids
            msg = (f"{a + 1}. ve {b + 1}. yol ({at[0]:.2f}, {at[1]:.2f}) noktasında kesişiyor ama birleşmiyor: biri "
                   "ötekinin üstünde bitmeli.")
            out_problems.append({"kind": "crossing", "roads": ids, "x": at[0], "y": at[1], "message": msg})
        elif kind == "axis":
            r = ids[0]
            msg = f"{r + 1}. yol aksı {acmd.km_text(1000.0 + extra)} km'sinde gövdesiyle kesiyor; aksa yalnız ucu bağlanır."
            out_problems.append({"kind": "crossesAlignment", "roads": [r], "x": at[0], "y": at[1], "message": msg})
        else:
            r, e = ids
            tail = "" if extra is None else f" (en yakın yol ya da aks {extra:.2f} m uzakta; bağlantı toleransı {connect:.2f} m)"
            row = {"kind": "loose", "roads": [r], "x": at[0], "y": at[1]}
            if extra is not None:
                row["gap"] = extra
            row["message"] = f"{r + 1}. yolun {'sonu' if e else 'başı'} hiçbir yere bağlanmıyor{tail}."
            out_problems.append(row)
    warnings = []
    if out_problems:
        warnings.append({"code": "haul_network_gap", "path": "network",
                         "message": f"“{h['name']}” taşıma ağında birleşmeyen {len(out_problems)} yer var: "
                                    + " ".join(p["message"] for p in out_problems)})
    sites = []
    for (j, s), c in zip(off, conns):
        row = {"name": s["name"], "kind": s["kind"], "x": s["at"]["x"], "y": s["at"]["y"], "joined": c["joined"]}
        if c["joined"]:
            row.update({"length": c["length"], "station": c["station"], "km": c["km"], "offset": c["offset"],
                        "roads": c["roads"]})
        else:
            row["why"] = why(c, connect)
            warnings.append(unconnected_warning(s["name"], j, row["why"]))
        sites.append(row)
    out = {"haul": h["id"], "connect": connect, "reach": reach, "roads": roads,
           "junctions": [{"road": j["road"], "end": j["end"], "station": j["station"], "km": j["km"], "offset": j["offset"]}
                         for j in junctions],
           "joins": joins, "problems": out_problems, "sites": sites}
    return out, warnings


H1N = {**hc.H1, "sites": [SITE_W, SITE_B, SITE_C, SITE_F], "network": NETWORK}


def cases():
    import road_haul_plan_cases as pc
    volumes = pc.VOLUMES
    out = []

    def q(name, h, inp=None, hauls=None, roads=True, result=None, op="execute"):
        hauls = hauls or [h, pc.H2, pc.H3]
        if result is None:
            o, w = answer(h, roads=ROADS if roads else [])
            result = completed(None if op == "validate" else o, w)
        base = setup(hauls, volumes, roads=roads)
        out.append(case(name, [{"op": op, "input": inp or {"haul": h["id"]}, "result": result,
                                "expect": {**UNCHANGED, "road": base["settings"]["road"]}, "within": W}], base))

    q("Ağ okunur: bağlanan, kopuk ve uzak alanlar", H1N)
    q("Bağlantı toleransı 1 m: 0,8 m açık uç birleşir", {**H1N, "network": {**NETWORK, "connect": 1.0}})
    q("Aksa erişim 5 m: aksa 6 ve 7 m'de biten yollar bağlanmaz", {**H1N, "network": {**NETWORK, "reach": 5.0}})
    q("Erişim verilmezse bağlantı toleransı kadar", {**H1N, "network": {"layers": [LAYER], "connect": 0.5}})
    q("Adla okunur", H1N, {"haul": "Ana Brükner"})
    q("Aksa erişim 50 m: aksa 30 m'de biten uçlar bağlanır", {**H1N, "network": {**NETWORK, "reach": 50.0}})
    q("Bağlantı toleransı 2 m", {**H1N, "network": {**NETWORK, "connect": 2.0}})
    q("Yalnız dere döküm alanı", {**H1N, "sites": [SITE_W]})
    q("Konumlu alanı olmayan ağ", {**H1N, "sites": [{k: v for k, v in SITE_W.items() if k != "at"}]})
    on_body = {**SITE_W, "name": "Gövdedeki döküm", "at": dict(zip("xy", spot(100.0, 40.0)))}
    q("Alan yolun gövdesinde: erişim kalan boy", {**H1N, "sites": [on_body]})
    beside = {**SITE_W, "name": "Yandaki döküm", "at": dict(zip("xy", ahead(spot(100.0, 40.0), 100.0, 0.3)))}
    q("0,3 m yandaki alan en yakın yola bağlanır", {**H1N, "sites": [beside]})
    on_t = {**SITE_B, "name": "T yolundaki ödünç", "at": dict(zip("xy", [round(v, 3) for v in Net().at(3, 30.0)]))}
    q("T yolundaki alan kısa yoldan km 520'ye", {**H1N, "sites": [on_t]})
    near_r2 = {**SITE_B, "name": "R2 üstündeki ödünç", "at": dict(zip("xy", spot(450.0, -50.0)))}
    q("İki kavşaktan kısası seçilir", {**H1N, "sites": [near_r2]})
    q("Yolu olmayan katman", H1N, roads=False)
    q("İki katman", {**H1N, "network": {**NETWORK, "layers": [LAYER, "bos"]}})
    q("Plan cevabı verir", H1N, op="plan")
    q("Doğrulama uyarıları verir", H1N, op="validate")
    nonet = {**hc.H1}
    q("Ağı olmayan Brükner", nonet, result=failed(
        "invalid_haul", "“Ana Brükner” Brüknerinin taşıma ağı yok; önce ağın katmanlarını verin (road.hauls.edit network).", "haul"))
    q("Ağın katmanı çizimde yok", {**H1N, "network": {**NETWORK, "layers": ["yok"]}}, result=failed(
        "invalid_haul", "Taşıma ağının “yok” kimlikli katmanı çizimde yok.", "network.layers[0]"))
    q("Olmayan Brükner", H1N, {"haul": "yok"}, result=failed("unknown_haul", "“yok” kimlikli ya da adlı Brükner projede yok.", "haul"))
    base = setup([H1N, pc.H2, pc.H3], volumes, work="map")
    out.append(case("Yol projesi değil", [{"op": "execute", "input": {"haul": "h-1"}, "result": hc.NOT_ROAD,
                                            "expect": {**UNCHANGED}, "within": W}], base))
    return out


def main():
    found = cases()
    import road_haul_plan_cases as pc
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.hauls.network", "commandVersion": 1,
           "title": "Taşıma ağı: yolların birleşmesi ve alanların erişimi",
           "note": "scripts/fixtures/road_haul_network_cases.py yazar; çizim, aks, koridor, kübaj ve Brüknerler "
                   "road_haul_plan_cases.py'nin; yollar aksın bağımsız referansıyla (mpmath) km ve kaymaya kurulur, "
                   "uzaklıklar ve kesişmeler GEOS (shapely), en kısa yol Dijkstra; KentOS kodu yok.",
           "setup": setup([H1N, pc.H2, pc.H3], pc.VOLUMES), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit("güncel değil: road.hauls.network.json")
    else:
        OUT.write_text(text)
    print(f"road.hauls.network.json: {len(found)} durum")


if __name__ == "__main__":
    main()
