#!/usr/bin/env python3
"""Independent reference of the network analyses (docs/adr/0209 §3–§9), written from the ADR without KentOS code.

Three scenes (a road network with one-way streets, an arc, a closed street, a bridge, a near miss and a part on its
own; a water network with valves, a closed valve and a source; two lines crossing at a vertex) and their questions:
the graph, places, routes (as given, the best order with the first or the first and the last fixed, barriers, a stop
without a way), service areas (their lines exactly, their areas with shapely), closest facilities, cost matrices,
traces (connected, downstream, upstream, isolation with what is no longer fed) and Denetle. Written to
fixtures/network/v1/cases.json; the geometry core (crates/shared/geometry-core/tests/all/network.rs) must give the
same: nodes, pieces, orders, edges and valves exactly; offsets, lengths and costs within 1e-9 m; areas within 1e-5 of
themselves.

This is another program than the core's: the query graph is built whole (each piece cut at the query's places into
sub-pieces) and searched by a plain heap Dijkstra; best orders by trying every order; the areas by shapely's buffers.

    python3 scripts/fixtures/network_cases.py [--check]
"""

import argparse
import heapq
import itertools
import json
import math
import sys
from pathlib import Path

from shapely.geometry import LineString
from shapely.ops import unary_union

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/cases.json"
INF = math.inf
X0, Y0 = 500000.0, 4400000.0

# ── Geometry: a path is a list of primitives, ("seg", a, b) or ("arc", c, r, a0, sweep). ─────────────────────────


def arc_of(a, b, bulge):
    """The arc a bulge makes from a to b: its centre, radius, start angle and signed sweep (4·atan(bulge))."""
    sweep = 4.0 * math.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    r = chord / (2.0 * math.sin(abs(sweep) / 2.0))
    # The centre lies off the chord's middle, to the left for a counter-clockwise arc.
    mx, my = (a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0
    h = r * math.cos(abs(sweep) / 2.0)
    nx, ny = -dy / chord, dx / chord
    # cos(|sweep| / 2) is below zero past a half circle: the centre crosses the chord by itself.
    side = 1.0 if sweep > 0 else -1.0
    c = (mx + side * nx * h, my + side * ny * h)
    a0 = math.atan2(a[1] - c[1], a[0] - c[0])
    return ("arc", c, r, a0, sweep)


def path_of(pts, bulges):
    out = []
    for i in range(len(pts) - 1):
        a, b = pts[i], pts[i + 1]
        bl = bulges[i] if bulges else 0.0
        out.append(("seg", a, b) if bl == 0.0 else arc_of(a, b, bl))
    return out


def plen(p):
    return math.hypot(p[2][0] - p[1][0], p[2][1] - p[1][1]) if p[0] == "seg" else p[2] * abs(p[4])


def pat(p, t):
    if p[0] == "seg":
        return (p[1][0] + (p[2][0] - p[1][0]) * t, p[1][1] + (p[2][1] - p[1][1]) * t)
    _, c, r, a0, sw = p
    a = a0 + sw * t
    return (c[0] + math.cos(a) * r, c[1] + math.sin(a) * r)


def on_arc(theta, a0, sw):
    """Whether angle theta is on the arc from a0 through sw (either way)."""
    lo, span = (a0, sw) if sw >= 0 else (a0 + sw, -sw)
    d = (theta - lo) % (2 * math.pi)
    return d <= span + 1e-12


def closest(p, q):
    """The point of primitive p nearest q: (point, t, distance)."""
    if p[0] == "seg":
        (ax, ay), (bx, by) = p[1], p[2]
        dx, dy = bx - ax, by - ay
        L2 = dx * dx + dy * dy
        t = 0.0 if L2 == 0 else max(0.0, min(1.0, ((q[0] - ax) * dx + (q[1] - ay) * dy) / L2))
        x = pat(p, t)
        return x, t, math.hypot(q[0] - x[0], q[1] - x[1])
    _, c, r, a0, sw = p
    th = math.atan2(q[1] - c[1], q[0] - c[0])
    if on_arc(th, a0, sw):
        x = (c[0] + math.cos(th) * r, c[1] + math.sin(th) * r)
        # The angle's fraction along the arc.
        d = (th - a0) % (2 * math.pi) if sw >= 0 else (a0 - th) % (2 * math.pi)
        return x, d / abs(sw), abs(math.hypot(q[0] - c[0], q[1] - c[1]) - r)
    s, f = pat(p, 0.0), pat(p, 1.0)
    ds, df = math.hypot(q[0] - s[0], q[1] - s[1]), math.hypot(q[0] - f[0], q[1] - f[1])
    return (s, 0.0, ds) if ds <= df else (f, 1.0, df)


def sub(p, t0, t1):
    if p[0] == "seg":
        return ("seg", pat(p, t0), pat(p, t1))
    _, c, r, a0, sw = p
    return ("arc", c, r, a0 + sw * t0, sw * (t1 - t0))


def rev(p):
    if p[0] == "seg":
        return ("seg", p[2], p[1])
    _, c, r, a0, sw = p
    return ("arc", c, r, a0 + sw, -sw)


def pieces_between(path, a, b):
    """The primitives of a path from a to b metres along it (a ≤ b)."""
    out, s = [], 0.0
    for p in path:
        L = plen(p)
        k0, k1 = s, s + L
        s = k1
        if L <= 0 or k1 <= a or k0 >= b:
            continue
        t0 = (a - k0) / L if a > k0 else 0.0
        t1 = (b - k0) / L if b < k1 else 1.0
        out.append(p if (t0, t1) == (0.0, 1.0) else sub(p, t0, t1))
    return out


def caseless(s):
    out = []
    for ch in s.strip():
        if ch == "I":
            out.append("ı")
        elif ch == "İ":
            out.append("i")
        else:
            low = ch.lower()
            out.append(low if len(low) == 1 else ch)
    return "".join(out)


def number(text):
    t = (text or "").strip().replace(",", ".")
    try:
        v = float(t)
    except ValueError:
        return None
    return v if math.isfinite(v) and t.count(".") <= 1 else None


# ── The graph (§3) ───────────────────────────────────────────────────────────────────────────────────────────────


class Net:
    def __init__(self, scene):
        rules = scene["rules"]
        self.tol = tol = rules["tolerance"]
        self.costs = rules.get("costs", [])
        d = rules["direction"]
        self.edges = []
        for e in scene["edges"]:
            pts = [tuple(p) for p in e["pts"]]
            path = path_of(pts, e.get("bulges"))
            L = sum(plen(p) for p in path)
            if e.get("closed"):
                dr = "closed"
            elif d["kind"] == "both":
                dr = "both"
            elif d["kind"] == "digitized":
                dr = "forward"
            else:
                v = e.get("direction")
                k = caseless(v) if v is not None else None
                dr = "both"
                for name, key in (("forward", "forward"), ("backward", "backward"), ("closed", "closed")):
                    if k is not None and k in [caseless(x) for x in d.get(key, [])]:
                        dr = name
                        break
            self.edges.append({"id": e["id"], "path": path, "L": L, "dir": dr, "values": e.get("costs", [])})
        self.junctions = scene.get("junctions", [])
        self.unread = []
        # Cuts: (s, point, own) by edge: each edge's own ends (and vertices), then T touches and junction projections.
        cuts = [[] for _ in self.edges]
        for i, e in enumerate(self.edges):
            pa = pat(e["path"][0], 0.0)
            cuts[i].append((0.0, pa, True))
            if rules["connect"] == "vertices":
                s = 0.0
                for k, p in enumerate(e["path"]):
                    if k > 0:
                        cuts[i].append((s, pat(p, 0.0), True))
                    s += plen(p)
            cuts[i].append((e["L"], pat(e["path"][-1], 1.0), True))

        def touch(q, skip):
            hit = False
            for f, e in enumerate(self.edges):
                s = 0.0
                for p in e["path"]:
                    L = plen(p)
                    x, t, dist = closest(p, q)
                    if dist <= tol:
                        at = s + t * L
                        if not (skip and skip[0] == f and abs(at - skip[1]) <= tol):
                            cuts[f].append((at, x, False))
                            hit = True
                    s += L
            return hit

        for i, e in enumerate(self.edges):
            touch(pat(e["path"][0], 0.0), (i, 0.0))
            touch(pat(e["path"][-1], 1.0), (i, e["L"]))
        self.attached = [touch(tuple(j["p"]), None) for j in self.junctions]
        for c in cuts:
            c.sort(key=lambda x: x[0])
        # Nodes: cuts in order, then junction points; joined within the tolerance; a node where its first member is.
        points = [x[1] for c in cuts for x in c] + [tuple(j["p"]) for j in self.junctions]
        parent = list(range(len(points)))

        def find(a):
            while parent[a] != a:
                a = parent[a]
            return a

        for i in range(len(points)):
            for j in range(i + 1, len(points)):
                if math.hypot(points[i][0] - points[j][0], points[i][1] - points[j][1]) <= tol:
                    ra, rb = find(i), find(j)
                    if ra != rb:
                        parent[max(ra, rb)] = min(ra, rb)
        index, node_of, self.nodes = {}, [], []
        for i in range(len(points)):
            r = find(i)
            if r not in index:
                index[r] = len(self.nodes)
                self.nodes.append(points[r])
            node_of.append(index[r])
        nj = len(points) - len(self.junctions)
        self.kind = [{"source": False, "valve": False, "closed": False} for _ in self.nodes]
        self.jnode = []
        for j, jn in enumerate(self.junctions):
            n = node_of[nj + j] if self.attached[j] else None
            self.jnode.append(n)
            if n is not None:
                if jn["role"] == "source":
                    self.kind[n]["source"] = True
                if jn["role"] == "valve":
                    self.kind[n]["valve"] = True
                if jn.get("closed"):
                    self.kind[n]["closed"] = True
        # Pieces and their costs: [length, then the definition's].
        self.pieces, self.short, k = [], [], 0
        self.ends = []
        for i, e in enumerate(self.edges):
            rates = [1.0]
            for c, rule in enumerate(self.costs):
                v = e["values"][c] if c < len(e["values"]) else None
                if rule["kind"] == "speed":
                    s = number(v) if v is not None else None
                    if s is None or s <= 0:
                        self.unread.append((c + 1, e["id"]))
                        s = rule["speed"]
                    rates.append(60.0 / (s * 1000.0))
                else:
                    x = number(v) if v is not None else None
                    if x is None or x < 0:
                        self.unread.append((c + 1, e["id"]))
                        rates.append(None)
                    else:
                        rates.append(x / e["L"] if e["L"] > 0 else 0.0)
            cs = cuts[i]
            self.ends.append((node_of[k], node_of[k + len(cs) - 1]))
            # The data's own short stretches (§9): the edge's own places closer than the tolerance along it; a sliver
            # another end or a junction cuts beside a place (an arc's end computed a hair short) is dropped unsaid.
            own = [(s0, p0) for s0, p0, mine in cs if mine]
            for (sa, pa), (sb, _) in zip(own, own[1:]):
                if 0 < sb - sa <= tol:
                    self.short.append((e["id"], pa))
            if e["L"] <= 0:
                self.short.append((e["id"], cs[0][1]))
            for w in range(len(cs) - 1):
                (sa, pa, _), (sb, _, _) = cs[w], cs[w + 1]
                na, nb = node_of[k + w], node_of[k + w + 1]
                L = sb - sa
                if na == nb and L <= tol:
                    continue
                cost = [None if r is None else r * L for r in rates]
                fw = [INF if c is None or e["dir"] in ("backward", "closed") else c for c in cost]
                bw = [INF if c is None or e["dir"] in ("forward", "closed") else c for c in cost]
                self.pieces.append({"from": na, "to": nb, "edge": i, "s0": sa, "s1": sb, "len": L, "fw": fw, "bw": bw,
                                    "geom": pieces_between(e["path"], sa, sb)})
            k += len(cs)

    def locate(self, q, reach):
        best = None
        for k, pc in enumerate(self.pieces):
            s = 0.0
            for p in pc["geom"]:
                L = plen(p)
                x, t, d = closest(p, q)
                if d <= reach:
                    cand = (d, k, s + t * L, x)
                    if best is None or cand[:3] < best[:3]:
                        best = cand
                s += L
        if best is None:
            return None
        return {"piece": best[1], "offset": best[2], "p": best[3]}

    def point_on(self, k, off):
        s = 0.0
        g = self.pieces[k]["geom"]
        for i, p in enumerate(g):
            L = plen(p)
            if off <= s + L or i == len(g) - 1:
                return pat(p, min(1.0, max(0.0, (off - s) / L)) if L > 0 else 0.0)
            s += L

    # ── A query's graph (§4): each piece cut at the query's places into sub-pieces. ───────────────────────────────

    def query(self, places, barriers):
        """The query's nodes: graph nodes then one per place inside a piece; its sub-pieces; each place's node."""
        tol = self.tol
        n = len(self.nodes)
        inside = {}
        node_of_place = []
        blocked = set(i for i, k in enumerate(self.kind) if k["closed"])
        for loc, barrier in [(x, False) for x in places] + [(x, True) for x in barriers]:
            pc = self.pieces[loc["piece"]]
            o = loc["offset"]
            if o <= tol and o <= pc["len"] - o:
                node = pc["from"]
            elif pc["len"] - o <= tol:
                node = pc["to"]
            else:
                key = (loc["piece"], o)
                if key not in inside:
                    inside[key] = n
                    n += 1
                node = inside[key]
            if barrier:
                blocked.add(node)
            node_of_place.append(node)
        subs = []  # (from, to, piece, a, b)
        for k, pc in enumerate(self.pieces):
            cuts = sorted([(o, v) for (p, o), v in inside.items() if p == k])
            seq = [(0.0, pc["from"])] + cuts + [(pc["len"], pc["to"])]
            for (oa, na), (ob, nb) in zip(seq, seq[1:]):
                subs.append((na, nb, k, oa, ob))
        return n, subs, node_of_place, blocked

    def dijkstra(self, n, subs, origins, blocked, c, reverse, cutoff=None):
        adj = [[] for _ in range(n)]
        for (na, nb, k, oa, ob) in subs:
            pc = self.pieces[k]
            share = (ob - oa) / pc["len"]
            f, b = pc["fw"][c] * share if pc["fw"][c] < INF else INF, pc["bw"][c] * share if pc["bw"][c] < INF else INF
            if reverse:
                f, b = b, f
            if f < INF:
                adj[na].append((nb, f, (k, oa, ob)))
            if b < INF:
                adj[nb].append((na, b, (k, ob, oa)))
        # A place inside a piece tries its way forward first, then back (the ADR's order of equal costs).
        for v in range(len(self.nodes), n):
            adj[v].sort(key=lambda x: x[2][2] < x[2][1])
        dist = [INF] * n
        pred = [None] * n
        heap = []
        for o in origins:
            if dist[o] > 0:
                dist[o] = 0.0
                heapq.heappush(heap, (0.0, o))
        done = [False] * n
        while heap:
            d, u = heapq.heappop(heap)
            if done[u] or d > dist[u]:
                continue
            if cutoff is not None and d > cutoff:
                break
            done[u] = True
            for v, w, span in adj[u]:
                if v in blocked:
                    continue
                nd = d + w
                if nd < dist[v]:
                    dist[v] = nd
                    pred[v] = (u, span)
                    heapq.heappush(heap, (nd, v))
        return dist, pred

    def way(self, pred, node, reverse):
        spans = []
        while pred[node] is not None:
            u, (k, a, b) = pred[node]
            spans.append((k, a, b))
            node = u
        if reverse:
            return [(k, b, a) for (k, a, b) in spans]
        return list(reversed(spans))

    def spans_cost(self, spans, c):
        total = 0.0
        for k, a, b in spans:
            pc = self.pieces[k]
            w = pc["fw"][c] if b >= a else pc["bw"][c]
            if w == INF:
                return INF
            total += w * (abs(b - a) / pc["len"])
        return total


# ── The analyses ──────────────────────────────────────────────────────────────────────────────────────────────────


def run_route(net, stops, barriers, c, reorder):
    if len(stops) < 2:
        return {"error": "tooFew"}

    def leg(i, j):
        n, subs, nodes, blocked = net.query([stops[i], stops[j]], barriers)
        dist, pred = net.dijkstra(n, subs, [nodes[0]], blocked, c, False)
        if dist[nodes[1]] == INF:
            return None
        return dist[nodes[1]], net.way(pred, nodes[1], False)

    order = list(range(len(stops)))
    if reorder != "none":
        m = [[0.0 if i == j else (leg(i, j) or (INF,))[0] for j in range(len(stops))] for i in range(len(stops))]
        last = len(stops) - 1 if reorder == "keepFirstLast" else None
        middle = [i for i in range(1, len(stops)) if i != last]
        best = None
        for perm in itertools.permutations(middle):
            seq = [0, *perm] + ([last] if last is not None else [])
            total = 0.0
            for a, b in zip(seq, seq[1:]):
                total += m[a][b]
            if total < INF and (best is None or total < best[0] or (total == best[0] and seq < best[1])):
                best = (total, seq)
        if best is None:
            return {"error": "noOrder"}
        order = best[1]
    legs = []
    for a, b in zip(order, order[1:]):
        r = leg(a, b)
        if r is None:
            return {"error": "unreachable", "between": [a, b]}
        legs.append(r)
    spans = [s for _, sp in legs for s in sp]
    return {
        "order": order,
        "cost": sum(d for d, _ in legs),
        "spans": [list(s) for s in spans],
        "totals": [net.spans_cost(spans, k) for k in range(1 + len(net.costs))],
    }


def band_stretches(dg, dh, rf, rb, a, b, breaks):
    """The stretches of [a, b] in each band by the ADR's rule: the least of the costs from either end."""
    out = []
    g_ok, h_ok = dg < INF and rf < INF, dh < INF and rb < INF
    if not g_ok and not h_ok:
        return out
    if g_ok and not h_ok:
        m = b
    elif h_ok and not g_ok:
        m = a
    elif rf + rb > 0:
        m = min(b, max(a, (dh - dg + rf * a + rb * b) / (rf + rb)))
    else:
        m = a if dh < dg else b
    lo = -INF
    for k, hi in enumerate(breaks):
        s = e = None
        if g_ok and a < m:
            if rf > 0:
                x0, x1 = a + (lo - dg) / rf, a + (hi - dg) / rf
            elif lo < dg <= hi:
                x0, x1 = a, m
            else:
                x0 = x1 = m
            s = (min(m, max(a, x0)), min(m, max(a, x1)))
        if h_ok and m < b:
            if rb > 0:
                x0, x1 = b - (hi - dh) / rb, b - (lo - dh) / rb
            elif lo < dh <= hi:
                x0, x1 = m, b
            else:
                x0 = x1 = m
            e = (min(b, max(m, x0)), min(b, max(m, x1)))
        s = s if s and s[1] > s[0] else None
        e = e if e and e[1] > e[0] else None
        if s and e and s[1] == m and e[0] == m:
            # Both sides of the meeting point in one band: one stretch.
            out.append((k, s[0], e[1]))
        else:
            for x in (s, e):
                if x:
                    out.append((k, x[0], x[1]))
        lo = hi
    return out


def run_area(net, facilities, breaks, c, toward, separate, barriers, trim, shapes=False):
    groups = [[i] for i in range(len(facilities))] if separate else [list(range(len(facilities)))]
    lines = []
    for gi, members in enumerate(groups):
        n, subs, nodes, blocked = net.query([facilities[i] for i in members], barriers)
        # The facilities are the origins; the barriers' nodes follow them in the list.
        dist, _ = net.dijkstra(n, subs, nodes[: len(members)], blocked, c, toward, cutoff=breaks[-1])
        for (na, nb, k, oa, ob) in subs:
            pc = net.pieces[k]
            f, b = (pc["bw"][c], pc["fw"][c]) if toward else (pc["fw"][c], pc["bw"][c])
            rf, rb = f / pc["len"], b / pc["len"]
            for band, x0, x1 in band_stretches(dist[na], dist[nb], rf, rb, oa, ob, breaks):
                lines.append({"facility": members[0] if separate else None, "band": band, "piece": k, "a": x0, "b": x1})
    areas = []
    for f in sorted({l["facility"] for l in lines}, key=lambda x: -1 if x is None else x):
        below = None
        for band in range(len(breaks)):
            geoms = []
            for l in lines:
                if l["facility"] == f and l["band"] <= band:
                    pts = []
                    for p in pieces_between(net.pieces[l["piece"]]["geom"], l["a"], l["b"]):
                        steps = 1 if p[0] == "seg" else 512
                        for s in range(steps + 1):
                            q = pat(p, s / steps)
                            if not pts or pts[-1] != q:
                                pts.append(q)
                    if len(pts) >= 2:
                        geoms.append(LineString([(x - X0, y - Y0) for x, y in pts]).buffer(trim, quad_segs=512))
            if not geoms:
                continue
            disc = unary_union(geoms)
            ring = disc.difference(below) if below is not None else disc
            area = {"facility": f, "band": band, "disc": disc.area, "ring": ring.area}
            if shapes:
                # Each one's parts and holes (the processing cases, scripts/fixtures/network_processing_cases.py).
                for key, g in (("disc", disc), ("ring", ring)):
                    polys = list(g.geoms) if g.geom_type == "MultiPolygon" else [g]
                    area[key + "Parts"] = len(polys)
                    area[key + "Holes"] = sum(len(q.interiors) for q in polys)
            areas.append(area)
            below = disc
    lines.sort(key=lambda l: (-1 if l["facility"] is None else l["facility"], l["band"], l["piece"], l["a"]))
    return {"lines": lines, "areas": areas}


def run_nearest(net, origins, targets, k, cutoff, c, reverse, barriers, paths):
    rows = []
    for o in origins:
        n, subs, nodes, blocked = net.query([o, *targets], barriers)
        dist, pred = net.dijkstra(n, subs, [nodes[0]], blocked, c, reverse)
        found = [(dist[nodes[1 + j]], j) for j in range(len(targets)) if dist[nodes[1 + j]] < INF and (cutoff is None or dist[nodes[1 + j]] <= cutoff)]
        found.sort()
        row = []
        for cost, j in found[: (k if k is not None else len(targets))]:
            item = {"target": j, "cost": cost}
            if paths:
                item["spans"] = [list(s) for s in net.way(pred, nodes[1 + j], reverse)]
            row.append(item)
        rows.append(row)
    return rows


def run_trace(net, starts, barriers, kind):
    n, subs, nodes, blocked = net.query(starts, barriers)
    adj = [[] for _ in range(n)]
    for (na, nb, k, oa, ob) in subs:
        dr = net.edges[net.pieces[k]["edge"]]["dir"]
        fwd_ok = {"connected": dr != "closed", "isolation": dr != "closed", "downstream": dr in ("both", "forward"), "upstream": dr in ("both", "backward")}[kind]
        bwd_ok = {"connected": dr != "closed", "isolation": dr != "closed", "downstream": dr in ("both", "backward"), "upstream": dr in ("both", "forward")}[kind]
        if fwd_ok:
            adj[na].append((nb, (k, oa, ob)))
        if bwd_ok:
            adj[nb].append((na, (k, ob, oa)))
    start = list(dict.fromkeys(nodes[: len(starts)]))
    seen = set(start)
    queue = list(start)
    spans, keys, valves = [], set(), set()
    while queue:
        u = queue.pop(0)
        for v, (k, a, b) in adj[u]:
            key = (k, min(a, b), max(a, b))
            if key not in keys:
                keys.add(key)
                spans.append((k, a, b))
            if v in blocked or v in seen:
                continue
            seen.add(v)
            if kind == "isolation" and v < len(net.nodes) and net.kind[v]["valve"] and v not in start:
                valves.add(v)
                continue
            queue.append(v)
    out = {
        "length": sum(abs(b - a) for _, a, b in spans),
        "edges": sorted({net.pieces[k]["edge"] for k, _, _ in spans}),
        "spans": sorted([[k, min(a, b), max(a, b)] for k, a, b in spans]),
    }
    if kind == "isolation":
        out["valves"] = sorted(j for j, jn in enumerate(net.junctions) if jn["role"] == "valve" and not jn.get("closed") and net.jnode[j] in valves)
        sources = [i for i, k in enumerate(net.kind) if k["source"]]
        if sources:
            _, bsubs, _, bblocked = net.query([], barriers)

            def fed(stop):
                adj2 = [[] for _ in range(len(net.nodes) + 1000)]
                for (na, nb, k, oa, ob) in bsubs:
                    if net.edges[net.pieces[k]["edge"]]["dir"] != "closed":
                        adj2[na].append((nb, k))
                        adj2[nb].append((na, k))
                got, seen2, q = set(), set(sources), list(sources)
                while q:
                    u = q.pop(0)
                    for v, k in adj2[u]:
                        got.add(k)
                        if v in bblocked or v in stop or v in seen2:
                            continue
                        seen2.add(v)
                        q.append(v)
                return got

            cut = {k for k, _, _ in spans}
            unfed = sorted((fed(set()) - fed(valves)) - cut)
            out["unfed"] = unfed
            out["unfedLength"] = sum(net.pieces[k]["len"] for k in unfed)
            out["unfedEdges"] = sorted({net.pieces[k]["edge"] for k in unfed})
    return out


def seg_hits(p, q):
    """Where two primitives meet (points), segments and arcs."""
    if p[0] == "seg" and q[0] == "seg":
        (ax, ay), (bx, by) = p[1], p[2]
        (cx, cy), (dx, dy) = q[1], q[2]
        rx, ry, sx, sy = bx - ax, by - ay, dx - cx, dy - cy
        den = rx * sy - ry * sx
        if abs(den) < 1e-12:
            return []
        t = ((cx - ax) * sy - (cy - ay) * sx) / den
        u = ((cx - ax) * ry - (cy - ay) * rx) / den
        return [(ax + rx * t, ay + ry * t)] if -1e-9 <= t <= 1 + 1e-9 and -1e-9 <= u <= 1 + 1e-9 else []
    if p[0] == "arc" and q[0] == "seg":
        p, q = q, p
    if p[0] == "seg" and q[0] == "arc":
        (ax, ay), (bx, by) = p[1], p[2]
        _, c, r, a0, sw = q
        dx, dy = bx - ax, by - ay
        fx, fy = ax - c[0], ay - c[1]
        A, B, C = dx * dx + dy * dy, 2 * (fx * dx + fy * dy), fx * fx + fy * fy - r * r
        disc = B * B - 4 * A * C
        if disc < 0:
            return []
        out = []
        for t in ((-B - math.sqrt(disc)) / (2 * A), (-B + math.sqrt(disc)) / (2 * A)):
            if -1e-9 <= t <= 1 + 1e-9:
                x = (ax + dx * t, ay + dy * t)
                if on_arc(math.atan2(x[1] - c[1], x[0] - c[0]), a0, sw):
                    out.append(x)
        return out
    return []


def run_check(net):
    tol = net.tol
    problems = []
    parent = list(range(len(net.nodes)))

    def find(a):
        while parent[a] != a:
            a = parent[a]
        return a

    for pc in net.pieces:
        a, b = find(pc["from"]), find(pc["to"])
        if a != b:
            parent[max(a, b)] = min(a, b)
    parts = {}
    for k, pc in enumerate(net.pieces):
        r = find(pc["from"])
        e = parts.setdefault(r, [0, 0.0, k])
        e[0] += 1
        e[1] += pc["len"]
    plist = sorted(parts.values(), key=lambda e: (-e[1], e[2]))
    for n_, L, first in plist[1:]:
        pc = net.pieces[first]
        problems.append({"kind": "detached", "at": list(net.point_on(first, pc["len"] / 2)), "ids": [net.edges[pc["edge"]]["id"]], "value": L})
    degree = [0] * len(net.nodes)
    for pc in net.pieces:
        degree[pc["from"]] += 1
        degree[pc["to"]] += 1
    pairs = set()
    for n, d in enumerate(degree):
        if d != 1:
            continue
        own = next(k for k, pc in enumerate(net.pieces) if n in (pc["from"], pc["to"]))
        q = net.nodes[n]
        best = None
        for k, pc in enumerate(net.pieces):
            if k == own or n in (pc["from"], pc["to"]):
                continue
            for p in pc["geom"]:
                x, _, dist = closest(p, q)
                if tol < dist <= 1.0 and (best is None or (dist, k) < (best[0], best[1])):
                    best = (dist, k, x)
        if best:
            key = (min(own, best[1]), max(own, best[1]))
            if key not in pairs:
                pairs.add(key)
                problems.append({"kind": "nearMiss", "at": [(q[0] + best[2][0]) / 2, (q[1] + best[2][1]) / 2],
                                 "ids": [net.edges[net.pieces[own]["edge"]]["id"], net.edges[net.pieces[best[1]]["edge"]]["id"]], "value": best[0]})
    seen = set()
    flat = [(k, p) for k, pc in enumerate(net.pieces) for p in pc["geom"]]
    for i in range(len(flat)):
        for j in range(i + 1, len(flat)):
            (ka, pa), (kb, pb) = flat[i], flat[j]
            if ka == kb:
                continue
            ends = [net.nodes[net.pieces[ka]["from"]], net.nodes[net.pieces[ka]["to"]], net.nodes[net.pieces[kb]["from"]], net.nodes[net.pieces[kb]["to"]]]
            for h in seg_hits(pa, pb):
                if any(math.hypot(e[0] - h[0], e[1] - h[1]) <= tol for e in ends):
                    continue
                key = (math.floor(h[0] / tol + 0.5), math.floor(h[1] / tol + 0.5))
                if key not in seen:
                    seen.add(key)
                    problems.append({"kind": "crossing", "at": list(h), "ids": [net.edges[net.pieces[ka]["edge"]]["id"], net.edges[net.pieces[kb]["edge"]]["id"]], "value": None})
    for j, jn in enumerate(net.junctions):
        if not net.attached[j]:
            problems.append({"kind": "offNetwork", "at": list(jn["p"]), "ids": [jn["id"]], "value": None})
    for eid, at in net.short:
        problems.append({"kind": "short", "at": list(at), "ids": [eid], "value": None})
    for c, eid in net.unread:
        e = next(e for e in net.edges if e["id"] == eid)
        a, b = pat(e["path"][0], 0.0), pat(e["path"][-1], 1.0)
        problems.append({"kind": "unread", "at": [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2], "ids": [eid], "value": None, "cost": c})
    order = ["detached", "nearMiss", "crossing", "offNetwork", "short", "unread"]
    problems.sort(key=lambda p: (order.index(p["kind"]), p["at"][0], p["at"][1]))
    return {
        "nodes": len(net.nodes),
        "pieces": len(net.pieces),
        "length": sum(pc["len"] for pc in net.pieces),
        "parts": [[e[0], e[1]] for e in plist],
        "deadEnds": sum(1 for d in degree if d == 1),
        "problems": problems,
    }


# ── The scenes ────────────────────────────────────────────────────────────────────────────────────────────────────


def P(x, y):
    return [X0 + x, Y0 + y]


def edge(i, pts, bulges=None, direction=None, costs=(), closed=False):
    e = {"id": float(i), "pts": [P(*p) for p in pts], "direction": direction, "costs": list(costs), "closed": closed}
    if bulges:
        e["bulges"] = bulges
    return e


ROADS = {
    "name": "yollar",
    "rules": {
        "connect": "ends",
        "tolerance": 0.01,
        "direction": {"kind": "field", "field": "yon", "forward": ["FT", "1"], "backward": ["TF"], "closed": ["N"]},
        "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0}, {"name": "Ücret", "kind": "field", "field": "ucret", "unit": "TL"}],
    },
    "edges": [
        edge(1, [(0, 0), (210, 0), (400, 0)], costs=["50", "4"]),
        edge(2, [(0, 150), (400, 150)], direction="FT", costs=["30", "3"]),
        edge(3, [(210, 0), (210, 150)], bulges=[0.35], costs=["40,5", "2"]),
        edge(4, [(0, 0), (0, 150)], costs=["hızlı", "1"]),
        edge(5, [(400, 0), (400, 150)], direction="tf", costs=["50"]),
        edge(6, [(300, 260), (300, 150.004)], costs=["30", "1"]),
        edge(7, [(100, 0), (100, 150)], costs=["50", "1"], closed=True),
        edge(8, [(250, -50), (250, 200)], costs=["60", "1"]),
        edge(9, [(600, 0), (650, 0)], costs=["50", "1"]),
        edge(10, [(452, 80), (400.5, 80)], direction="N", costs=["20", "1"]),
    ],
    "junctions": [],
}

WATER = {
    "name": "su",
    "rules": {"connect": "vertices", "tolerance": 0.005, "direction": {"kind": "digitized"}},
    "edges": [
        edge(1, [(0, 0), (100, 0), (200, 0), (300, 0)]),
        edge(2, [(150, 0), (150, 120)]),
        edge(3, [(200, 0), (200, -90)]),
        edge(4, [(300, 0), (300, 100), (380, 100)]),
        edge(5, [(150, 120), (300, 100)]),
    ],
    "junctions": [
        {"id": 20.0, "p": P(0, 0), "role": "source", "closed": False},
        {"id": 21.0, "p": P(60, 0.003), "role": "valve", "closed": False},
        {"id": 22.0, "p": P(200, 0), "role": "valve", "closed": False},
        {"id": 23.0, "p": P(150, 70), "role": "valve", "closed": False},
        {"id": 24.0, "p": P(300, 50), "role": "valve", "closed": True},
        {"id": 25.0, "p": P(250, 0), "role": "junction", "closed": False},
        {"id": 26.0, "p": P(500, 500), "role": "valve", "closed": False},
    ],
}

# Ends on an arc's ends: the arc's computed end lies a hair off the vertex, a T touch there cuts a sliver that is
# dropped unsaid; the 5 mm line at the street's end is the data's own short stretch, said.
ARC_ENDS = {
    "name": "yay uçları",
    "rules": {"connect": "ends", "tolerance": 0.01, "direction": {"kind": "both"}},
    "edges": [
        edge(1, [(0, 0), (120, 0)], bulges=[0.2]),
        edge(2, [(120, 0), (240, 0)]),
        edge(3, [(120, -100), (120, 0)]),
        edge(4, [(0, -100), (0, 0)]),
        edge(5, [(240, 0), (240.005, 0)]),
    ],
    "junctions": [],
}

CROSS = {
    "name": "köşeler",
    "edges": [edge(1, [(0, 0), (50, 0), (100, 0)]), edge(2, [(50, -50), (50, 0), (50, 50)])],
    "junctions": [],
}


def scene_json(s, rules=None):
    return {"name": s["name"], "rules": rules or s["rules"], "edges": s["edges"], "junctions": s.get("junctions", [])}


def build():
    cases = []

    def at(net, x, y, reach=5.0):
        loc = net.locate((X0 + x, Y0 + y), reach)
        assert loc is not None, (x, y)
        return {**loc, "q": [X0 + x, Y0 + y], "reach": reach}

    def loc_json(loc):
        # Where the place was asked, how far it was looked for, and where it is on the network.
        return {"x": loc["q"][0], "y": loc["q"][1], "reach": loc["reach"], "piece": loc["piece"], "offset": loc["offset"]}

    def graph_json(net):
        return {
            "nodes": [list(n) for n in net.nodes],
            "pieces": [[pc["from"], pc["to"], pc["edge"], pc["s0"], pc["s1"]] for pc in net.pieces],
        }

    # The road network.
    roads = Net(ROADS)
    S = {k: at(roads, *xy) for k, xy in {
        "a": (50, 1), "b": (350, 151), "c": (210 + 0, 2), "d": (395, 1), "e": (20, 150), "f": (300, 255),
        "g": (620, 1), "h": (150, 149), "i": (300, 0.5), "j": (4, 75),
    }.items()}
    places = {k: loc_json(v) for k, v in S.items()}
    questions = [
        {"kind": "route", "title": "a'dan b'ye uzunlukla", "stops": ["a", "b"], "cost": 0, "reorder": "none"},
        {"kind": "route", "title": "a'dan b'ye süreyle (yay ve hız)", "stops": ["a", "b"], "cost": 1, "reorder": "none"},
        {"kind": "route", "title": "b'den a'ya (tek yönler)", "stops": ["b", "a"], "cost": 0, "reorder": "none"},
        {"kind": "route", "title": "dört durak, ilki sabit, en iyi sıra", "stops": ["a", "f", "d", "e"], "cost": 0, "reorder": "keepFirst"},
        {"kind": "route", "title": "beş durak, ilki ve sonu sabit", "stops": ["a", "f", "e", "i", "d"], "cost": 1, "reorder": "keepFirstLast"},
        {"kind": "route", "title": "engelle", "stops": ["a", "b"], "cost": 0, "reorder": "none", "barriers": ["c"]},
        {"kind": "route", "title": "ücretle: ücretsiz sokak geçilmez", "stops": ["d", "i"], "cost": 2, "reorder": "none"},
        {"kind": "route", "title": "ayrık parçaya yol yok", "stops": ["a", "g"], "cost": 0, "reorder": "none"},
        {"kind": "area", "title": "c'den 150 ve 300 m", "facilities": ["c"], "breaks": [150.0, 300.0], "cost": 0, "toward": False, "separate": False, "trim": 20.0},
        {"kind": "area", "title": "c'ye 0,2 ve 0,4 dakika", "facilities": ["c"], "breaks": [0.2, 0.4], "cost": 1, "toward": True, "separate": False, "trim": 15.0},
        {"kind": "area", "title": "iki tesis birleşik", "facilities": ["a", "f"], "breaks": [120.0, 260.0], "cost": 0, "toward": False, "separate": False, "trim": 25.0},
        {"kind": "area", "title": "iki tesis ayrı, engelle", "facilities": ["a", "f"], "breaks": [200.0], "cost": 0, "toward": False, "separate": True, "trim": 25.0, "barriers": ["h"]},
        {"kind": "closest", "title": "olaylardan en yakın iki tesise", "origins": ["e", "i", "j"], "targets": ["a", "f", "d"], "k": 2, "cost": 1, "reverse": False},
        {"kind": "closest", "title": "tesislerden olaylara, 300 m içinde", "origins": ["e", "i", "j"], "targets": ["a", "f", "d"], "k": 3, "cost": 0, "reverse": True, "cutoff": 300.0},
        {"kind": "matrix", "title": "iki başlangıçtan üç varışa", "origins": ["a", "d"], "targets": ["b", "e", "g"], "cost": 0},
        {"kind": "check", "title": "Denetle"},
    ]
    cases.append({"scene": scene_json(ROADS), "places": places, "graph": graph_json(roads), "questions": [answer(roads, S, q) for q in questions]})
    # The water network.
    water = Net(WATER)
    W = {k: at(water, *xy) for k, xy in {"k": (120, 0.5), "l": (150, 30), "m": (380, 100.5), "n": (203, -60), "o": (1, 0)}.items()}
    questions = [
        {"kind": "trace", "title": "kırık boru: yalıtım", "starts": ["k"], "trace": "isolation"},
        {"kind": "trace", "title": "dal: yalıtım", "starts": ["l"], "trace": "isolation"},
        {"kind": "trace", "title": "depodan akış aşağı", "starts": ["o"], "trace": "downstream"},
        {"kind": "trace", "title": "uçtan akış yukarı", "starts": ["m"], "trace": "upstream"},
        {"kind": "trace", "title": "bağlı, engelle", "starts": ["n"], "trace": "connected", "barriers": ["k"]},
        {"kind": "check", "title": "Denetle"},
    ]
    cases.append({"scene": scene_json(WATER), "places": {k: loc_json(v) for k, v in W.items()}, "graph": graph_json(water), "questions": [answer(water, W, q) for q in questions]})
    # Streets ending on an arc's ends: connected there, no sliver said; the 5 mm line said.
    arcs = Net(ARC_ENDS)
    A = {"r": at(arcs, 120, -50), "s": at(arcs, 180, 0), "t": at(arcs, 0, -50)}
    questions = [
        {"kind": "route", "title": "yayın ucundan geçen yol", "stops": ["r", "s"], "cost": 0, "reorder": "none"},
        {"kind": "route", "title": "yayın üstünden", "stops": ["t", "r"], "cost": 0, "reorder": "none"},
        {"kind": "check", "title": "Denetle"},
    ]
    cases.append({"scene": scene_json(ARC_ENDS), "places": {k: loc_json(v) for k, v in A.items()}, "graph": graph_json(arcs), "questions": [answer(arcs, A, q) for q in questions]})
    # Two lines crossing at a vertex: not connected at their ends only, connected at their vertices.
    for connect in ("ends", "vertices"):
        rules = {"connect": connect, "tolerance": 0.01, "direction": {"kind": "both"}}
        net = Net({**CROSS, "rules": rules})
        C = {"p": at(net, 0, 1), "q": at(net, 50, 49)}
        questions = [{"kind": "route", "title": "köşeden geçen yol", "stops": ["p", "q"], "cost": 0, "reorder": "none"}, {"kind": "check", "title": "Denetle"}]
        cases.append({"scene": scene_json(CROSS, rules), "places": {k: loc_json(v) for k, v in C.items()}, "graph": graph_json(net), "questions": [answer(net, C, q) for q in questions]})
    return {
        "format": "kentos.network-cases",
        "version": 1,
        "source": "scripts/fixtures/network_cases.py: the ADR's rules in Python, shapely " + __import__("shapely").__version__ + " for the areas",
        "cases": cases,
    }


def answer(net, S, q):
    out = dict(q)
    pick = lambda names: [S[n] for n in names]
    barriers = pick(q.get("barriers", []))
    if q["kind"] == "route":
        out["expect"] = run_route(net, pick(q["stops"]), barriers, q["cost"], q["reorder"])
    elif q["kind"] == "area":
        out["expect"] = run_area(net, pick(q["facilities"]), q["breaks"], q["cost"], q["toward"], q["separate"], barriers, q["trim"])
    elif q["kind"] == "closest":
        out["expect"] = run_nearest(net, pick(q["origins"]), pick(q["targets"]), q["k"], q.get("cutoff"), q["cost"], q["reverse"], barriers, True)
    elif q["kind"] == "matrix":
        out["expect"] = run_nearest(net, pick(q["origins"]), pick(q["targets"]), None, None, q["cost"], False, barriers, False)
    elif q["kind"] == "trace":
        out["expect"] = run_trace(net, pick(q["starts"]), barriers, q["trace"])
    elif q["kind"] == "check":
        out["expect"] = run_check(net)
    return out


def clean(v):
    """Infinite costs (no way) as null; tuples as lists."""
    if isinstance(v, float) and not math.isfinite(v):
        return None
    if isinstance(v, (list, tuple)):
        return [clean(x) for x in v]
    if isinstance(v, dict):
        return {k: clean(x) for k, x in v.items()}
    return v


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(clean(build()), ensure_ascii=False, indent=1, allow_nan=False) + "\n"
    if args.check:
        old = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if json.loads(old or "null") != json.loads(text):
            print(f"{OUT.relative_to(ROOT)} is not what this script writes: run it without --check and read the difference.")
            return 1
        print(f"{OUT.relative_to(ROOT)} matches")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
