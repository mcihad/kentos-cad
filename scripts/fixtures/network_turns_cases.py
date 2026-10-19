#!/usr/bin/env python3
"""Independent reference of turns, levels and time profiles in network analysis (docs/adr/0521), written from the ADR
without KentOS code.

Four scenes: a street grid with a one-way street, a curved dead end, a forbidden turn and a turn with its own delay,
asked under three U-turn policies and by length and time; a crossing over a bridge at other levels, with and without
the levels, with a connector, and a vertex crossing whose inside is the bridge's level; a highway and a side road whose
speeds change through the day, with a closure that is waited out and a turn delay. The questions: routes (one or more
legs, the best order), service areas' lines, closest facilities, cost matrices and Denetle. Written to
fixtures/network/v1/turns.json; the geometry core (crates/shared/geometry-core/tests/all/network_turns.rs) must give the
same: nodes, pieces, orders exactly; offsets, lengths and costs within 1e-9 (of their size above 1).

This is another program than the core's: the query's graph is built whole (each piece cut at the query's places), its
arcs are the search's nodes and every allowed turn an edge between them; times are walked with exact fractions; the
meeting point of a timed stretch is found exactly from the speed steps' breakpoints. Every route and closest way is also
found by trying every walk (cost-bounded): the reference checks its Dijkstra's answer is the only cheapest one, so the
cases have no ties for the core to break otherwise.

    python3 scripts/fixtures/network_turns_cases.py [--check]
"""

import argparse
import heapq
import itertools
import json
import math
import sys
from fractions import Fraction as Fr
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/turns.json"
INF = math.inf
X0, Y0 = 500000.0, 4400000.0
DAY = 1440

# ── Geometry: a path is primitives, ("seg", a, b) or ("arc", c, r, a0, sweep). ───────────────────────────────────


def arc_of(a, b, bulge):
    sweep = 4.0 * math.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    r = chord / (2.0 * math.sin(abs(sweep) / 2.0))
    mx, my = (a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0
    h = r * math.cos(abs(sweep) / 2.0)
    nx, ny = -dy / chord, dx / chord
    side = 1.0 if sweep > 0 else -1.0
    c = (mx + side * nx * h, my + side * ny * h)
    return ("arc", c, r, math.atan2(a[1] - c[1], a[0] - c[0]), sweep)


def path_of(pts, bulges):
    out = []
    for i in range(len(pts) - 1):
        bl = bulges[i] if bulges else 0.0
        out.append(("seg", pts[i], pts[i + 1]) if bl == 0.0 else arc_of(pts[i], pts[i + 1], bl))
    return out


def plen(p):
    return math.hypot(p[2][0] - p[1][0], p[2][1] - p[1][1]) if p[0] == "seg" else p[2] * abs(p[4])


def pat(p, t):
    if p[0] == "seg":
        return (p[1][0] + (p[2][0] - p[1][0]) * t, p[1][1] + (p[2][1] - p[1][1]) * t)
    _, c, r, a0, sw = p
    return (c[0] + math.cos(a0 + sw * t) * r, c[1] + math.sin(a0 + sw * t) * r)


def tangent(p, t):
    """The way a primitive goes at t (not of unit length)."""
    if p[0] == "seg":
        return (p[2][0] - p[1][0], p[2][1] - p[1][1])
    _, c, r, a0, sw = p
    a = a0 + sw * t
    return (-math.sin(a), math.cos(a)) if sw >= 0 else (math.sin(a), -math.cos(a))


def on_arc(theta, a0, sw):
    lo, span = (a0, sw) if sw >= 0 else (a0 + sw, -sw)
    return (theta - lo) % (2 * math.pi) <= span + 1e-12


def closest(p, q):
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


def pieces_between(path, a, b):
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


def level_of(text):
    """A level field's value: None (no value), an int, or "bad" (not a whole number within ±1000)."""
    if text is None or not text.strip():
        return None
    v = number(text)
    if v is None or v != int(v) or abs(v) > 1000:
        return "bad"
    return int(v)


def meets(a, b):
    return a == "any" or b == "any" or a == b


# ── Time profiles (§4): an edge's speed steps through the day, walked exactly. ───────────────────────────────────


def day_steps(periods, kmh, values):
    """[(start minute, metres a minute as a Fraction)] covering the day; [] when the edge is closed all day; and
    whether a period's value was not read."""
    mpm = lambda v: Fr(v) * 1000 / 60
    steps, at, bad = [], 0, False
    for k, p in enumerate(periods):
        if p["from"] > at:
            steps.append((at, mpm(kmh)))
        if "factor" in p:
            v = Fr(kmh) * Fr(p["factor"])
            steps.append((p["from"], v * 1000 / 60))
        else:
            t = values[k] if k < len(values) else None
            if t is None or not t.strip():
                speed = kmh
            else:
                n = number(t)
                if n is None or n < 0:
                    bad, speed = True, kmh
                else:
                    speed = n
            steps.append((p["from"], mpm(speed)))
        at = p["to"]
    if at < DAY:
        steps.append((at, mpm(kmh)))
    if not any(v > 0 for _, v in steps):
        steps = []
    return steps, bad


def walk(steps, t0, sign, limit):
    """Going on from minute t0 (sign +1 forward in time, -1 backward) until `limit` metres: the breakpoints
    [(metres, minutes taken)], exact; a zero speed is a wait (two points at one place)."""
    t0 = Fr(t0)
    limit = Fr(limit)
    pts = [(Fr(0), Fr(0))]
    if not steps:
        return None
    gone, took = Fr(0), Fr(0)
    t = t0
    while gone < limit:
        day = (t // DAY) * DAY
        m = t - day
        if sign > 0:
            k = max(i for i, s in enumerate(steps) if s[0] <= m)
            end = day + (steps[k + 1][0] if k + 1 < len(steps) else DAY)
            span = end - t
        else:
            # Going back: the step that ends at or after t (a time at a step's start is the step before's).
            if m == 0:
                day -= DAY
                m = DAY
            k = max(i for i, s in enumerate(steps) if s[0] < m)
            begin = day + steps[k][0]
            span = t - begin
        v = steps[k][1]
        if v > 0 and gone + v * span >= limit:
            took += (limit - gone) / v
            gone = limit
            pts.append((gone, took))
            break
        gone += v * span
        took += span
        t = t + sign * span
        pts.append((gone, took))
    return pts


def time_to(pts, d):
    """Minutes taken to the first reach of d metres along a walk's breakpoints."""
    d = Fr(d)
    for (d0, t0), (d1, t1) in zip(pts, pts[1:]):
        if d1 >= d and d1 > d0:
            return t0 + (t1 - t0) * (d - d0) / (d1 - d0)
        if d1 >= d and d1 == d0 == d:
            return t0
    return pts[-1][1] if d <= pts[-1][0] else None


def gone_in(pts, dt):
    """Metres gone within dt minutes along a walk's breakpoints."""
    dt = Fr(dt)
    best = Fr(0)
    for (d0, t0), (d1, t1) in zip(pts, pts[1:]):
        if t1 <= dt:
            best = d1
        elif t0 <= dt:
            best = d0 + (d1 - d0) * (dt - t0) / (t1 - t0) if t1 > t0 else d0
            break
    return best


def travel(steps, t, d, back):
    if d <= 0:
        return 0.0
    pts = walk(steps, t, -1 if back else 1, d)
    if pts is None:
        return INF
    return float(pts[-1][1])


# ── The graph (docs/adr/0209 §3, docs/adr/0521 §3) ──────────────────────────────────────────────────────────────


class Net:
    def __init__(self, scene):
        rules = scene["rules"]
        self.rules = rules
        self.tol = tol = rules["tolerance"]
        self.costs = rules.get("costs", [])
        self.turn_rule = rules.get("turns")
        lv = rules.get("levels")
        d = rules["direction"]
        self.edges = []
        self.unread_other = []
        for i, e in enumerate(scene["edges"]):
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
                for key in ("forward", "backward", "closed"):
                    if k is not None and k in [caseless(x) for x in d.get(key, [])]:
                        dr = key
                        break
            if lv is None:
                levels = (0, 0, 0)
            else:
                vals = e.get("levels", [None, None, None])
                read = [level_of(v) for v in vals]
                bad = any(r == "bad" for r in read[:2]) or ("inside" in lv and read[2] == "bad")
                a = read[0] if isinstance(read[0], int) else 0
                b = read[1] if isinstance(read[1], int) else 0
                inside = read[2] if ("inside" in lv and isinstance(read[2], int)) else (a if a == b else ("own", i))
                if bad:
                    self.unread_other.append((e["id"], pat(path[0], 0.0)))
                levels = (a, b, inside)
            self.edges.append({"id": e["id"], "path": path, "L": L, "dir": dr, "values": e.get("costs", []),
                               "levels": levels, "periods": e.get("periods", [])})
        self.junctions = scene.get("junctions", [])
        jlevels = []
        for j in self.junctions:
            if lv is None:
                jlevels.append("any")
                continue
            v = level_of(j.get("level"))
            if v == "bad":
                self.unread_other.append((j["id"], tuple(j["p"])))
                v = None
            jlevels.append("any" if v is None else v)
        self.unread = []
        # Cuts: (s, point, own, level) by edge.
        cuts = [[] for _ in self.edges]
        for i, e in enumerate(self.edges):
            cuts[i].append((0.0, pat(e["path"][0], 0.0), True, e["levels"][0]))
            if rules["connect"] == "vertices":
                s = 0.0
                for k, p in enumerate(e["path"]):
                    if k > 0:
                        cuts[i].append((s, pat(p, 0.0), True, e["levels"][2]))
                    s += plen(p)
            cuts[i].append((e["L"], pat(e["path"][-1], 1.0), True, e["levels"][1]))

        def touch(q, level, skip):
            for f, e in enumerate(self.edges):
                if not meets(level, e["levels"][2]):
                    continue
                s = 0.0
                for k, p in enumerate(e["path"]):
                    L = plen(p)
                    x, t, dist = closest(p, q)
                    if dist <= tol:
                        at = s + t * L
                        if skip and skip[0] == f and abs(at - skip[1]) <= tol:
                            s += L
                            continue
                        exact_end = (at == 0.0 and t == 0.0 and k == 0) or (at == e["L"] and t == 1.0 and k == len(e["path"]) - 1)
                        if not exact_end:
                            cuts[f].append((at, x, False, e["levels"][2]))
                    s += L

        for i, e in enumerate(self.edges):
            touch(pat(e["path"][0], 0.0), e["levels"][0], (i, 0.0))
            touch(pat(e["path"][-1], 1.0), e["levels"][1], (i, e["L"]))
        for j, jn in enumerate(self.junctions):
            touch(tuple(jn["p"]), jlevels[j], None)
        for c in cuts:
            c.sort(key=lambda x: x[0])
        points = [(x[1], x[3]) for c in cuts for x in c] + [(tuple(j["p"]), jlevels[k]) for k, j in enumerate(self.junctions)]
        parent = list(range(len(points)))

        def find(a):
            while parent[a] != a:
                a = parent[a]
            return a

        for i in range(len(points)):
            for j in range(i + 1, len(points)):
                (pi, li), (pj, lj) = points[i], points[j]
                if math.hypot(pi[0] - pj[0], pi[1] - pj[1]) <= tol and meets(li, lj):
                    ra, rb = find(i), find(j)
                    if ra != rb:
                        parent[max(ra, rb)] = min(ra, rb)
        index, node_of, self.nodes = {}, [], []
        for i in range(len(points)):
            r = find(i)
            if r not in index:
                index[r] = len(self.nodes)
                self.nodes.append(points[r][0])
            node_of.append(index[r])
        nj = len(points) - len(self.junctions)
        with_cut = {node_of[i] for i in range(nj)}
        self.attached = [node_of[nj + j] in with_cut for j in range(len(self.junctions))]
        self.closed_nodes = set()
        for j, jn in enumerate(self.junctions):
            if self.attached[j] and jn.get("closed"):
                self.closed_nodes.add(node_of[nj + j])
        # Pieces and their costs.
        self.pieces, self.short, k = [], [], 0
        for i, e in enumerate(self.edges):
            rates = [1.0]
            e["kmh"] = [None]
            for c, rule in enumerate(self.costs):
                v = e["values"][c] if c < len(e["values"]) else None
                if rule["kind"] == "speed":
                    s = number(v) if v is not None else None
                    if s is None or s <= 0:
                        self.unread.append((c + 1, e["id"]))
                        s = rule["speed"]
                    rates.append(60.0 / (s * 1000.0))
                    e["kmh"].append(s)
                else:
                    x = number(v) if v is not None else None
                    if x is None or x < 0:
                        self.unread.append((c + 1, e["id"]))
                        rates.append(None)
                    else:
                        rates.append(x / e["L"] if e["L"] > 0 else 0.0)
                    e["kmh"].append(None)
            cs = cuts[i]
            for w in range(len(cs) - 1):
                (sa, pa, _, _), (sb, _, _, _) = cs[w], cs[w + 1]
                na, nb = node_of[k + w], node_of[k + w + 1]
                L = sb - sa
                if na == nb and L <= tol:
                    continue
                cost = [None if r is None else r * L for r in rates]
                fw = [INF if c is None or e["dir"] in ("backward", "closed") else c for c in cost]
                bw = [INF if c is None or e["dir"] in ("forward", "closed") else c for c in cost]
                geom = pieces_between(e["path"], sa, sb)
                self.pieces.append({"from": na, "to": nb, "edge": i, "s0": sa, "s1": sb, "len": L, "fw": fw, "bw": bw,
                                    "geom": geom, "heads": (tangent(geom[0], 0.0), tangent(geom[-1], 1.0))})
            k += len(cs)
        # The clocks: each timed cost's day of speeds by edge.
        self.clocks = {}
        for c, rule in enumerate(self.costs):
            if rule["kind"] != "speed" or not rule.get("periods"):
                continue
            per_edge = []
            for e in self.edges:
                values = e["periods"][c] if c < len(e["periods"]) else []
                steps, bad = day_steps(rule["periods"], e["kmh"][c + 1], values)
                if bad and (c + 1, e["id"]) not in self.unread:
                    self.unread.append((c + 1, e["id"]))
                per_edge.append(steps)
            self.clocks[c + 1] = per_edge
        self.degree = [0] * len(self.nodes)
        for pc in self.pieces:
            self.degree[pc["from"]] += 1
            self.degree[pc["to"]] += 1
        # The turn lines.
        self.named, self.unplaced = {}, []
        for t in scene.get("turns", []):
            pts = [tuple(p) for p in t["pts"]]
            key = self.turn_key(pts)
            if key is None:
                self.unplaced.append((t["id"], pts[0]))
                continue
            if not t.get("field"):
                d = INF
            else:
                v = (t.get("value") or "").strip()
                n = number(v) if v else None
                if not v:
                    d = INF
                elif n is None or n < 0:
                    d = INF
                    self.unread_other.append((t["id"], pts[0]))
                else:
                    d = n / 60.0
            old = self.named.get(key)
            self.named[key] = d if old is None else max(old, d)

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

    def turn_key(self, pts):
        reach = max(1.0, self.tol)
        a, b = self.locate(pts[0], reach), self.locate(pts[-1], reach)
        if a is None or b is None:
            return None
        via = pts[1] if len(pts) >= 3 else ((pts[0][0] + pts[-1][0]) / 2, (pts[0][1] + pts[-1][1]) / 2)
        pa, pb = self.pieces[a["piece"]], self.pieces[b["piece"]]
        cands = [n for n in (pa["from"], pa["to"]) if n in (pb["from"], pb["to"])]
        if not cands:
            return None
        n = min(cands, key=lambda n: (math.hypot(self.nodes[n][0] - via[0], self.nodes[n][1] - via[1]), n))
        in_arc = a["piece"] * 2 if pa["to"] == n else a["piece"] * 2 + 1
        if a["piece"] == b["piece"]:
            out_arc = in_arc ^ 1
        else:
            out_arc = b["piece"] * 2 if pb["from"] == n else b["piece"] * 2 + 1
        return (in_arc, out_arc)

    # ── Turns (§2) ──

    def is_time(self, c):
        return c >= 1 and self.costs[c - 1]["kind"] == "speed"

    def turn(self, a_in, a_out, node, c):
        """What a turn adds to cost c; None when forbidden."""
        r = self.turn_rule
        if r is None:
            return 0.0
        named = self.named.get((a_in, a_out))
        if named is not None and named == INF:
            return None
        deg = self.degree[node]
        if a_out == a_in ^ 1:
            ok = {"allowed": True, "deadEnds": deg == 1, "never": False}[r["uTurns"]]
            if not ok:
                return None
            delay = r.get("back", 0) / 60.0
        elif deg < 3:
            delay = 0.0
        else:
            came = self.arc_way(a_in, True)
            goes = self.arc_way(a_out, False)
            ang = math.degrees(math.atan2(came[0] * goes[1] - came[1] * goes[0], came[0] * goes[0] + came[1] * goes[1]))
            if abs(ang) <= 30:
                delay = r.get("straight", 0) / 60.0
            elif ang < 0:
                delay = r.get("right", 0) / 60.0
            else:
                delay = r.get("left", 0) / 60.0
        if not self.is_time(c):
            return 0.0
        return named if named is not None else delay

    def arc_way(self, arc, arrive):
        start, end = self.pieces[arc // 2]["heads"]
        fwd = arc % 2 == 0
        if fwd:
            return end if arrive else start
        v = start if arrive else end
        return (-v[0], -v[1])

    # ── A query's graph: each piece cut at the query's places into sub-pieces. ──

    def query(self, places, barriers):
        tol = self.tol
        n = len(self.nodes)
        inside, node_of_place = {}, []
        blocked = set(self.closed_nodes)
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
        subs = []
        for k, pc in enumerate(self.pieces):
            cuts = sorted([(o, v) for (p, o), v in inside.items() if p == k])
            seq = [(0.0, pc["from"])] + cuts + [(pc["len"], pc["to"])]
            for (oa, na), (ob, nb) in zip(seq, seq[1:]):
                subs.append((na, nb, k, oa, ob))
        return {"n": n, "subs": subs, "nodes": node_of_place, "blocked": blocked, "virtual": set(inside.values())}

    def step_cost(self, k, c, real_fwd, d, so_far, at, reverse):
        pc = self.pieces[k]
        whole = pc["fw"][c] if real_fwd else pc["bw"][c]
        if whole == INF:
            return INF
        if at is not None and c in self.clocks:
            steps = self.clocks[c][pc["edge"]]
            return travel(steps, at - so_far if reverse else at + so_far, d, reverse)
        return whole * (d / pc["len"])

    def moves(self, Q, reverse):
        """Each node's moves: (next, piece, search-forward, from offset, to offset)."""
        out = [[] for _ in range(Q["n"])]
        for (na, nb, k, oa, ob) in Q["subs"]:
            out[na].append((nb, k, True, oa, ob))
            out[nb].append((na, k, False, ob, oa))
        return out

    def search(self, Q, origins, c, reverse, at=None, cutoff=None):
        """Dijkstra over the query's states: ("start", node) or (arc, node) — the arc (piece * 2 + way, the way it is
        travelled) a way came by (a reverse search: leaves by) and the node it is at. Without turns a state is a node."""
        turns = self.turn_rule is not None
        moves = self.moves(Q, reverse)
        dist, pred, heap = {}, {}, []
        counter = itertools.count()
        for o in origins:
            s = ("start", o) if turns else o
            if dist.get(s, INF) > 0:
                dist[s] = 0.0
                pred[s] = None
                heapq.heappush(heap, (0.0, next(counter), s))
        done = set()
        while heap:
            d, _, s = heapq.heappop(heap)
            if s in done or d > dist.get(s, INF):
                continue
            if cutoff is not None and d > cutoff:
                break
            done.add(s)
            node = s[1] if turns else s
            came = s[0] if turns and s[0] != "start" else None
            for (nxt, k, fwd, oa, ob) in moves[node]:
                real = fwd != reverse
                if nxt in Q["blocked"]:
                    continue
                arc = k * 2 + (0 if real else 1)
                delay = 0.0
                if came is not None:
                    if node in Q["virtual"]:
                        if came != arc:
                            continue
                    else:
                        t = self.turn(arc, came, node, c) if reverse else self.turn(came, arc, node, c)
                        if t is None:
                            continue
                        delay = t
                step = self.step_cost(k, c, real, abs(ob - oa), d + delay, at, reverse)
                if step == INF:
                    continue
                nd = d + delay + step
                ns = (arc, nxt) if turns else nxt
                if nd < dist.get(ns, INF):
                    dist[ns] = nd
                    pred[ns] = (s, (k, oa, ob))
                    heapq.heappush(heap, (nd, next(counter), ns))
        return dist, pred

    def node_best(self, dist, node):
        cands = [(v, s) for s, v in dist.items() if (s[1] if isinstance(s, tuple) else s) == node]
        return min(cands, key=lambda x: x[0]) if cands else (INF, None)

    def way(self_, pred, s, reverse):
        spans = []
        while pred.get(s) is not None:
            s, span = pred[s]
            spans.append(span)
        if reverse:
            return [(k, b, a) for (k, a, b) in spans]
        return list(reversed(spans))

    def way_cost(self, spans, c, at, arrive):
        """A way's cost with its turns (and with a time profile, set off at `at`; `arrive`: arriving at `at`)."""
        def joint(a, b):
            pc = self.pieces[a[0]]
            if a[2] == pc["len"]:
                n = pc["to"]
            elif a[2] == 0.0:
                n = pc["from"]
            else:
                return None
            if a[0] == b[0] and b[1] not in (0.0, self.pieces[b[0]]["len"]):
                return None
            arc = lambda s: s[0] * 2 + (0 if s[2] >= s[1] else 1)
            return n, arc(a), arc(b)

        def turn(k):
            j = joint(spans[k], spans[k + 1])
            if j is None:
                return 0.0
            t = self.turn(j[1], j[2], j[0], c)
            return INF if t is None else t

        timed = at is not None and c in self.clocks
        total = 0.0
        order = range(len(spans) - 1, -1, -1) if (arrive and timed) else range(len(spans))
        for k in order:
            if arrive and timed:
                if k + 1 < len(spans):
                    total += turn(k)
            elif k > 0:
                total += turn(k - 1)
            kk, a, b = spans[k]
            total += self.step_cost(kk, c, b >= a, abs(b - a), total, at, arrive and timed)
        return total

    # ── Every walk, for the uniqueness of the cheapest way ──

    def walks(self, Q, start, goal, c, reverse, at, bound):
        """Every walk of the query's graph from `start` to `goal` (sub-pieces in a row, turns allowed, cost within
        `bound`): their spans and costs; with a time, walked from `at` (a reverse search's walks end at `at`)."""
        turns = self.turn_rule is not None
        moves = self.moves(Q, False)
        found = []

        def go(node, came, spans, cost):
            if node == goal and spans:
                found.append((list(spans), cost))
            if len(spans) > 24:
                return
            for (nxt, k, fwd, oa, ob) in moves[node]:
                if nxt in Q["blocked"]:
                    continue
                arc = k * 2 + (0 if fwd else 1)
                if came is not None and turns:
                    if node in Q["virtual"]:
                        if came != arc:
                            continue
                    elif self.turn(came, arc, node, c) is None:
                        continue
                if self.step_cost(k, c, fwd, abs(ob - oa), 0.0, None, False) == INF:
                    continue
                spans.append((k, oa, ob))
                w = self.way_cost(spans, c, at, reverse)
                if w <= bound:
                    go(nxt, arc, spans, w)
                spans.pop()

        go(start, None, [], 0.0)
        return found

    def check_unique(self, Q, a, b, c, reverse, at, best):
        """The cheapest way from node a to node b (reverse: the walks from b to a) is one way only."""
        start, goal = (b, a) if reverse else (a, b)
        if start == goal:
            return
        # A walk's first spans cost no more than it does, so the walks are cut at the cheapest cost; a walk that ends at
        # a time is walked back from its end, its first spans' cost not a bound of its own: cut at half again.
        anchored = reverse and at is not None and c in self.clocks
        bound = best * (1.5 if anchored else 1 + 1e-9) + 1e-9
        ws = self.walks(Q, start, goal, c, reverse, at, bound)
        cheapest = [w for w in ws if w[1] <= best * (1 + 1e-9) + 1e-9]
        assert len(cheapest) == 1, ("the cheapest way is not one", best, cheapest)
        assert abs(cheapest[0][1] - best) <= 1e-9 * max(1.0, best), (best, cheapest)


# ── The analyses ────────────────────────────────────────────────────────────────────────────────────────────────


def leg(net, a, b, barriers, c, at):
    Q = net.query([a, b], barriers)
    dist, pred = net.search(Q, [Q["nodes"][0]], c, False, at)
    cost, s = net.node_best(dist, Q["nodes"][1])
    if cost == INF:
        return None
    net.check_unique(Q, Q["nodes"][0], Q["nodes"][1], c, False, at, cost)
    return cost, net.way(pred, s, False)


def run_route(net, stops, barriers, c, reorder, at):
    order = list(range(len(stops)))
    if reorder != "none":
        m = [[0.0 if i == j else (leg(net, stops[i], stops[j], barriers, c, at) or (INF,))[0] for j in range(len(stops))] for i in range(len(stops))]
        last = len(stops) - 1 if reorder == "keepFirstLast" else None
        middle = [i for i in range(1, len(stops)) if i != last]
        best = None
        for perm in itertools.permutations(middle):
            seq = [0, *perm] + ([last] if last is not None else [])
            total = sum(m[x][y] for x, y in zip(seq, seq[1:]))
            if total < INF and (best is None or total < best[0] or (total == best[0] and seq < best[1])):
                best = (total, seq)
        if best is None:
            return {"error": "noOrder"}
        order = best[1]
    legs, so_far = [], 0.0
    for a, b in zip(order, order[1:]):
        r = leg(net, stops[a], stops[b], barriers, c, None if at is None else at + so_far)
        if r is None:
            return {"error": "unreachable", "between": [a, b]}
        so_far += r[0]
        legs.append(r)
    totals = []
    for k in range(1 + len(net.costs)):
        t = 0.0
        for _, spans in legs:
            t += net.way_cost(spans, k, None if at is None else at + t, False)
        totals.append(t)
    return {"order": order, "cost": sum(d for d, _ in legs), "spans": [list(s) for _, sp in legs for s in sp], "totals": totals}


def stretch_times(net, k, real_fwd, entry, c, at, reverse, length):
    """A side of a stretch: its walk's breakpoints from the end set off at `entry`, or None without a time."""
    if at is None or c not in net.clocks:
        return None
    steps = net.clocks[c][net.pieces[k]["edge"]]
    return walk(steps, at - entry if reverse else at + entry, -1 if reverse else 1, length)


def bands(dg, dh, a, b, breaks, side_g, side_h, rates):
    """The stretches of [a, b] in each band: the least of the costs from either end; exact."""
    out = []
    rf, rb = rates
    g_ok, h_ok = dg < INF and rf < INF, dh < INF and rb < INF
    if not g_ok and not h_ok:
        return out
    L = b - a
    # The costs along: g(x) from the start, h(x) from the end (linear, or a walk's breakpoints).
    if side_g is None:
        gfun = lambda x: Fr(dg) + Fr(rf) * (Fr(x) - Fr(a))
        greach = lambda dt: Fr(dt) / Fr(rf) if rf > 0 else Fr(L)
    else:
        gfun = lambda x: Fr(dg) + time_to(side_g, Fr(x) - Fr(a))
        greach = lambda dt: gone_in(side_g, dt)
    if side_h is None:
        hfun = lambda x: Fr(dh) + Fr(rb) * (Fr(b) - Fr(x))
        hreach = lambda dt: Fr(dt) / Fr(rb) if rb > 0 else Fr(L)
    else:
        hfun = lambda x: Fr(dh) + time_to(side_h, Fr(b) - Fr(x))
        hreach = lambda dt: gone_in(side_h, dt)
    if g_ok and not h_ok:
        m = Fr(b)
    elif h_ok and not g_ok:
        m = Fr(a)
    elif gfun(b) <= hfun(b):
        m = Fr(b)
    elif gfun(a) > hfun(a):
        m = Fr(a)
    else:
        # The meeting point, the last place g ≤ h: between breakpoints both are straight (a wait is a jump at its
        # place), so each piece's line is read from two inner points.
        xs = {Fr(a), Fr(b)}
        if side_g is not None:
            xs |= {Fr(a) + d for d, _ in side_g if d <= L}
        if side_h is not None:
            xs |= {Fr(b) - d for d, _ in side_h if d <= L}
        xs = sorted(xs)
        m = Fr(b)
        for x0, x1 in zip(xs, xs[1:]):
            p, q = x0 + (x1 - x0) / 3, x0 + 2 * (x1 - x0) / 3
            fp, fq = gfun(p) - hfun(p), gfun(q) - hfun(q)
            slope = (fq - fp) / (q - p)
            f0, f1 = fp - slope * (p - x0), fq + slope * (x1 - q)
            if gfun(x0) - hfun(x0) > 0 or f0 > 0:
                m = x0
                break
            if f1 > 0:
                m = x0 - f0 / slope
                break
    lo = None
    for k, hi in enumerate(breaks):
        s = e = None
        if g_ok and a < m:
            x0 = Fr(a) if lo is None or lo - dg < 0 else Fr(a) + greach(Fr(lo) - Fr(dg))
            x1 = Fr(a) if hi - dg < 0 else Fr(a) + greach(Fr(hi) - Fr(dg))
            s = (min(m, max(Fr(a), x0)), min(m, max(Fr(a), x1)))
        if h_ok and m < b:
            x0 = Fr(b) if hi - dh < 0 else Fr(b) - hreach(Fr(hi) - Fr(dh))
            x1 = Fr(b) if lo is None or lo - dh < 0 else Fr(b) - hreach(Fr(lo) - Fr(dh))
            e = (min(Fr(b), max(m, x0)), min(Fr(b), max(m, x1)))
        s = s if s and s[1] > s[0] else None
        e = e if e and e[1] > e[0] else None
        if s and e and s[1] == m and e[0] == m:
            out.append((k, float(s[0]), float(e[1])))
        else:
            for x in (s, e):
                if x:
                    out.append((k, float(x[0]), float(x[1])))
        lo = hi
    return out


def entry_cost(net, Q, dist, node, k, fwd, reverse, c):
    """What being at `node` ready to go along piece k (search-forward or not) costs: its start, or the turn onto it."""
    turns = net.turn_rule is not None
    if not turns:
        return dist.get(node, INF)
    real = fwd != reverse
    arc = k * 2 + (0 if real else 1)
    best = dist.get(("start", node), INF)
    for s, v in dist.items():
        if s[0] == "start" or s[1] != node:
            continue
        if node in Q["virtual"]:
            if s[0] == arc:
                best = min(best, v)
            continue
        t = net.turn(arc, s[0], node, c) if reverse else net.turn(s[0], arc, node, c)
        if t is not None:
            best = min(best, v + t)
    return best


def run_area(net, facilities, breaks, c, toward, barriers, at):
    Q = net.query(facilities, barriers)
    dist, _ = net.search(Q, Q["nodes"][: len(facilities)], c, toward, at, cutoff=breaks[-1])
    lines = []
    for (na, nb, k, oa, ob) in Q["subs"]:
        pc = net.pieces[k]
        f, b = (pc["bw"][c], pc["fw"][c]) if toward else (pc["fw"][c], pc["bw"][c])
        dg = entry_cost(net, Q, dist, na, k, True, toward, c)
        dh = entry_cost(net, Q, dist, nb, k, False, toward, c)
        side_g = stretch_times(net, k, not toward, dg, c, at, toward, ob - oa) if dg < INF and f < INF else None
        side_h = stretch_times(net, k, toward, dh, c, at, toward, ob - oa) if dh < INF and b < INF else None
        rates = (f / pc["len"], b / pc["len"])
        timed = at is not None and c in net.clocks
        for band, x0, x1 in bands(dg, dh, oa, ob, breaks, side_g if timed else None, side_h if timed else None, rates):
            lines.append({"band": band, "piece": k, "a": x0, "b": x1})
    lines.sort(key=lambda l: (l["band"], l["piece"], l["a"]))
    return {"lines": lines}


def run_nearest(net, origins, targets, k, cutoff, c, reverse, barriers, paths, at):
    rows = []
    for o in origins:
        Q = net.query([o, *targets], barriers)
        dist, pred = net.search(Q, [Q["nodes"][0]], c, reverse, at)
        found = []
        for j in range(len(targets)):
            cost, s = net.node_best(dist, Q["nodes"][1 + j])
            if cost < INF and (cutoff is None or cost <= cutoff):
                found.append((cost, j, s))
        found.sort(key=lambda x: (x[0], x[1]))
        row = []
        for cost, j, s in found[: (k if k is not None else len(targets))]:
            item = {"target": j, "cost": cost}
            if paths:
                net.check_unique(Q, Q["nodes"][0], Q["nodes"][1 + j], c, reverse, at, cost)
                spans = net.way(pred, s, reverse)
                item["spans"] = [list(x) for x in spans]
                item["totals"] = [net.way_cost(spans, cc, at, reverse) for cc in range(1 + len(net.costs))]
            row.append(item)
        rows.append(row)
    return rows


def run_check(net):
    """Denetle's counts and the problems this ADR adds (unread levels and turns, unplaced turns), and the crossings."""
    problems = []
    for c, eid in net.unread:
        e = next(e for e in net.edges if e["id"] == eid)
        a, b = pat(e["path"][0], 0.0), pat(e["path"][-1], 1.0)
        problems.append({"kind": "unread", "at": [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2], "ids": [eid], "cost": c})
    for eid, at in net.unread_other:
        problems.append({"kind": "unread", "at": list(at), "ids": [eid], "cost": None})
    for eid, at in net.unplaced:
        problems.append({"kind": "turn", "at": list(at), "ids": [eid]})
    # Crossings: two pieces meeting farther than the tolerance from both pieces' nodes, their insides at one level.
    seen = set()
    flat = [(k, p) for k, pc in enumerate(net.pieces) for p in pc["geom"]]
    for i in range(len(flat)):
        for j in range(i + 1, len(flat)):
            (ka, pa), (kb, pb) = flat[i], flat[j]
            if ka == kb or pa[0] != "seg" or pb[0] != "seg":
                continue
            la, lb = net.edges[net.pieces[ka]["edge"]]["levels"][2], net.edges[net.pieces[kb]["edge"]]["levels"][2]
            if net.rules.get("levels") is not None and not meets(la, lb):
                continue
            (ax, ay), (bx, by) = pa[1], pa[2]
            (cx, cy), (dx, dy) = pb[1], pb[2]
            rx, ry, sx, sy = bx - ax, by - ay, dx - cx, dy - cy
            den = rx * sy - ry * sx
            if abs(den) < 1e-12:
                continue
            t = ((cx - ax) * sy - (cy - ay) * sx) / den
            u = ((cx - ax) * ry - (cy - ay) * rx) / den
            if not (-1e-9 <= t <= 1 + 1e-9 and -1e-9 <= u <= 1 + 1e-9):
                continue
            h = (ax + rx * t, ay + ry * t)
            ends = [net.nodes[net.pieces[x][y]] for x in (ka, kb) for y in ("from", "to")]
            if any(math.hypot(e[0] - h[0], e[1] - h[1]) <= net.tol for e in ends):
                continue
            key = (math.floor(h[0] / net.tol + 0.5), math.floor(h[1] / net.tol + 0.5))
            if key not in seen:
                seen.add(key)
                problems.append({"kind": "crossing", "at": list(h), "ids": [net.edges[net.pieces[ka]["edge"]]["id"], net.edges[net.pieces[kb]["edge"]]["id"]]})
    order = ["crossing", "unread", "turn"]
    problems.sort(key=lambda p: (order.index(p["kind"]), p["at"][0], p["at"][1]))
    return {"nodes": len(net.nodes), "pieces": len(net.pieces), "length": sum(pc["len"] for pc in net.pieces), "problems": problems}


# ── The scenes ────────────────────────────────────────────────────────────────────────────────────────────────────


def P(x, y):
    return [X0 + x, Y0 + y]


def edge(i, pts, bulges=None, direction=None, costs=(), closed=False, levels=None, periods=None):
    e = {"id": float(i), "pts": [P(*p) for p in pts], "direction": direction, "costs": list(costs), "closed": closed}
    if bulges:
        e["bulges"] = bulges
    if levels is not None:
        e["levels"] = levels
    if periods is not None:
        e["periods"] = periods
    return e


XS, YS = [0, 100, 230], [0, 110, 230]


def grid_edges():
    out, n = [], 1
    speeds = {(0, 0): "50", (1, 0): "40", (0, 1): "50", (1, 1): "30", (0, 2): "40", (1, 2): "50"}
    for j in range(3):
        for i in range(2):
            out.append(edge(n, [(XS[i], YS[j]), (XS[i + 1], YS[j])], costs=[speeds[(i, j)]]))
            n += 1
    vspeeds = {(0, 0): "30", (0, 1): "40", (1, 0): "50", (1, 1): "30", (2, 0): "40", (2, 1): "50"}
    for i in range(3):
        for j in range(2):
            d = {(2, 1): "FT", (2, 0): "TF"}.get((i, j))
            out.append(edge(n, [(XS[i], YS[j]), (XS[i], YS[j + 1])], direction=d, costs=[vspeeds[(i, j)]]))
            n += 1
    # A curved dead end east of the middle row.
    out.append(edge(n, [(230, 110), (330, 110)], bulges=[0.1], costs=["30"]))
    return out


def grid_rules(u_turns):
    return {
        "connect": "ends",
        "tolerance": 0.01,
        "direction": {"kind": "field", "field": "yon", "forward": ["FT"], "backward": ["TF"]},
        "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0}],
        "turns": {"uTurns": u_turns, "right": 10.0, "left": 30.0, "back": 60.0},
    }


GRID_TURNS = [
    # Eastbound along the middle row, no right turn south at (100, 110).
    {"id": 50.0, "pts": [P(90, 110), P(100, 110), P(100, 100)], "field": True, "value": ""},
    # Northbound up the middle column, 45 s to turn east at (100, 110).
    {"id": 51.0, "pts": [P(100, 95), P(100, 110), P(112, 110)], "field": True, "value": "45"},
    # On no pieces that meet.
    {"id": 52.0, "pts": [P(50, 0), P(170, 230)], "field": True, "value": "5"},
    # Its delay not a number: forbidden, said.
    {"id": 53.0, "pts": [P(230, 220), P(230, 230), P(220, 230)], "field": True, "value": "uzun"},
    # Eastbound along the middle row, no left turn north at (230, 110): the way north there is from the dead end.
    {"id": 54.0, "pts": [P(220, 110), P(230, 110), P(230, 120)], "field": True, "value": ""},
]


LEVEL_RULES = {
    "connect": "ends", "tolerance": 0.01, "direction": {"kind": "both"},
    "levels": {"start": "z_bas", "end": "z_son"},
}


def level_edges():
    return [
        edge(1, [(0, 0), (100, 0)], levels=["0", "0", None]),
        edge(2, [(100, 0), (200, 0)], levels=["0", "0", None]),
        edge(3, [(100, -100), (100, 0)], levels=["0", "1", None]),
        edge(4, [(100, 0), (100, 100)], levels=["1", "0", None]),
        edge(5, [(200, 0), (100, 100)], levels=["0", "0", None]),
        # Ends on the bridge's inside under it: not at its level (its ends differ), not cut, not connected.
        edge(6, [(170, -60), (100.003, -60)], levels=["0", "üst", None]),
        # Crosses the bridge's inside (not at its level: no crossing said) and a street at its own (said).
        edge(7, [(60, 50), (140, 50)], levels=["0", "0", None]),
        edge(8, [(120, 30), (120, 70)], levels=["0", "0", None]),
    ]


VERTEX_RULES = {
    "connect": "vertices", "tolerance": 0.01, "direction": {"kind": "both"},
    "levels": {"start": "z_bas", "end": "z_son", "inside": "z_ic"},
}


def vertex_edges(ramp):
    out = [
        edge(1, [(0, 50), (100, 50), (200, 50)], levels=["0", "0", None]),
        edge(2, [(100, -50), (100, 50), (100, 150)], levels=["0", "0", "1"]),
    ]
    if ramp:
        out.append(edge(3, [(200, 50), (100, 150)], levels=["0", "0", None]))
    return out


TIME_PERIODS = [
    {"from": 420, "to": 540, "field": "hiz_sabah"},
    {"from": 600, "to": 601, "field": "kapali"},
    {"from": 1020, "to": 1140, "factor": 0.6},
]


TIME_RULES = {
    "connect": "ends", "tolerance": 0.01, "direction": {"kind": "both"},
    "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0, "periods": TIME_PERIODS}],
    "turns": {"uTurns": "allowed", "right": 5.0, "left": 20.0, "back": 60.0},
}


def time_edges():
    # A highway (90, 25 in the morning) and a side road (50) from the west to a crossing; on east past it a street
    # closed from 10:00 to 10:01 (too short a wait for any way round to fit in it: the cheapest way is one), and a
    # short dead end north.
    return [
        edge(1, [(0, 0), (2000, 0)], costs=["90"], periods=[["25", "", ""]]),
        edge(2, [(0, 0), (1000, 800)], costs=["50"]),
        edge(3, [(1000, 800), (2000, 0)], costs=["50"]),
        edge(4, [(2000, 0), (2300, 0)], costs=["90"], periods=[["", "0", ""]]),
        edge(5, [(2000, 0), (2000, 400)], costs=["30"], periods=[["yavaş", "", ""]]),
    ]


def scene_json(name, rules, edges, junctions=(), turns=()):
    return {"name": name, "rules": rules, "edges": edges, "junctions": list(junctions), "turns": list(turns)}


def build():
    cases = []

    def at(net, x, y, reach=5.0):
        loc = net.locate((X0 + x, Y0 + y), reach)
        assert loc is not None, (x, y)
        return {**loc, "q": [X0 + x, Y0 + y], "reach": reach}

    def loc_json(loc):
        return {"x": loc["q"][0], "y": loc["q"][1], "reach": loc["reach"], "piece": loc["piece"], "offset": loc["offset"]}

    def graph_json(net):
        return {"nodes": [list(n) for n in net.nodes], "pieces": [[pc["from"], pc["to"], pc["edge"], pc["s0"], pc["s1"]] for pc in net.pieces]}

    def case(scene, places, questions):
        net = Net(scene)
        S = {k: at(net, *xy) for k, xy in places.items()}
        cases.append({"scene": scene, "places": {k: loc_json(v) for k, v in S.items()}, "graph": graph_json(net),
                      "questions": [answer(net, S, q) for q in questions]})

    grid_places = {"a": (80, 110), "b": (100, 90), "c": (30, 0), "d": (230, 160), "e": (300, 106), "f": (160, 230), "g": (0, 60), "o": (150, 110)}
    for policy in ("allowed", "deadEnds", "never"):
        scene = scene_json(f"dönüşler ({policy})", grid_rules(policy), grid_edges(), turns=GRID_TURNS)
        qs = [
            {"kind": "route", "title": "yasak sağdan sonra uzunlukla", "stops": ["a", "b"], "cost": 0, "reorder": "none"},
            {"kind": "route", "title": "yasak sağdan sonra süreyle", "stops": ["a", "b"], "cost": 1, "reorder": "none"},
            {"kind": "route", "title": "kuzeye yalnız çıkmazdan dönerek", "stops": ["o", "d"], "cost": 0, "reorder": "none"},
        ]
        if policy == "allowed":
            qs += [
                {"kind": "route", "title": "gecikmeli dönüş ve tek yön", "stops": ["c", "d"], "cost": 1, "reorder": "none"},
                {"kind": "route", "title": "dört durak, en iyi sıra", "stops": ["c", "e", "f", "g"], "cost": 1, "reorder": "keepFirst"},
                {"kind": "area", "title": "a'dan 0,15 ve 0,3 dakika", "facilities": ["a"], "breaks": [0.15, 0.3], "cost": 1, "toward": False},
                {"kind": "area", "title": "b'ye 0,2 dakika", "facilities": ["b"], "breaks": [0.2], "cost": 1, "toward": True},
                {"kind": "closest", "title": "olaylardan en yakın iki tesise", "origins": ["a", "e"], "targets": ["b", "c", "f"], "k": 2, "cost": 1, "reverse": False},
                {"kind": "closest", "title": "tesislerden olaya", "origins": ["b"], "targets": ["a", "d", "g"], "k": 3, "cost": 1, "reverse": True},
                {"kind": "matrix", "title": "matris", "origins": ["a", "d"], "targets": ["b", "g"], "cost": 0},
                {"kind": "check", "title": "Denetle"},
            ]
        case(scene, grid_places, qs)
    # Levels.
    lv_places = {"p": (50, 0), "q": (100, 60), "r": (100, -80), "s": (150, -60)}
    lv_q = [
        {"kind": "route", "title": "köprüye kavşaktan çıkılmaz", "stops": ["p", "q"], "cost": 0, "reorder": "none"},
        {"kind": "route", "title": "köprünün altından geçilir", "stops": ["r", "q"], "cost": 0, "reorder": "none"},
        {"kind": "route", "title": "köprünün içine değen uç bağlanmaz", "stops": ["s", "p"], "cost": 0, "reorder": "none"},
        {"kind": "check", "title": "Denetle"},
    ]
    case(scene_json("seviyeler", LEVEL_RULES, level_edges()), lv_places, lv_q)
    plain = {k: v for k, v in LEVEL_RULES.items() if k != "levels"}
    case(scene_json("seviyesiz", plain, level_edges()), lv_places, lv_q[:1])
    connector = [{"id": 30.0, "p": P(100, 0), "role": "junction", "closed": False, "level": None}]
    case(scene_json("seviyeler ve bağlantı", LEVEL_RULES, level_edges(), connector), lv_places, lv_q[:1])
    one = [{"id": 31.0, "p": P(100, 0), "role": "junction", "closed": False, "level": "1"}]
    case(scene_json("seviyeler ve köprü düğümü", LEVEL_RULES, level_edges(), one), lv_places, lv_q[:1])
    vx_places = {"u": (20, 50), "v": (100, 120)}
    case(scene_json("köşede köprü", VERTEX_RULES, vertex_edges(False)), vx_places,
         [{"kind": "route", "title": "köşede kesişen köprüye çıkılmaz", "stops": ["u", "v"], "cost": 0, "reorder": "none"}, {"kind": "check", "title": "Denetle"}])
    case(scene_json("köşede köprü ve rampa", VERTEX_RULES, vertex_edges(True)), vx_places,
         [{"kind": "route", "title": "rampadan", "stops": ["u", "v"], "cost": 0, "reorder": "none"}])
    # Time.
    tm_places = {"w": (10, 0), "x": (2200, 0), "y": (2000, 300), "z": (990, 790)}
    tq = [
        {"kind": "route", "title": "gece 03:00 otoyoldan", "stops": ["w", "x"], "cost": 1, "reorder": "none", "at": 180},
        {"kind": "route", "title": "sabah 08:00 yan yoldan", "stops": ["w", "x"], "cost": 1, "reorder": "none", "at": 480},
        {"kind": "route", "title": "06:59: dönem yolda başlar", "stops": ["w", "x"], "cost": 1, "reorder": "none", "at": 419},
        {"kind": "route", "title": "09:59: kapanışı bekler", "stops": ["w", "x"], "cost": 1, "reorder": "none", "at": 599},
        {"kind": "route", "title": "uzunlukla, süre saatle", "stops": ["w", "y"], "cost": 0, "reorder": "none", "at": 1030},
        {"kind": "route", "title": "iki bacak, ikincisi birincinin varışında", "stops": ["w", "z", "x"], "cost": 1, "reorder": "none", "at": 535},
        {"kind": "route", "title": "saatsiz", "stops": ["w", "x"], "cost": 1, "reorder": "none"},
        {"kind": "area", "title": "08:00'de 3 ve 6 dakika", "facilities": ["w"], "breaks": [3.0, 6.0], "cost": 1, "toward": False, "at": 480},
        {"kind": "area", "title": "10:01'e varış, 2 ve 4 dakika", "facilities": ["x"], "breaks": [2.0, 4.0], "cost": 1, "toward": True, "at": 601.05},
        {"kind": "closest", "title": "tesislerden olaya 08:10'da varış", "origins": ["x"], "targets": ["w", "z"], "k": 2, "cost": 1, "reverse": True, "at": 490},
        {"kind": "matrix", "title": "17:30 matris", "origins": ["w", "z"], "targets": ["x", "y"], "cost": 1, "at": 1050},
        {"kind": "check", "title": "Denetle"},
    ]
    case(scene_json("zaman", TIME_RULES, time_edges()), tm_places, tq)
    return {"format": "kentos.network-turns", "version": 1,
            "source": "scripts/fixtures/network_turns_cases.py: the ADR's rules in Python, exact fractions for the time profiles, every walk for the cheapest way",
            "cases": cases}


def answer(net, S, q):
    out = dict(q)
    pick = lambda names: [S[n] for n in names]
    barriers = pick(q.get("barriers", []))
    at = q.get("at")
    if q["kind"] == "route":
        out["expect"] = run_route(net, pick(q["stops"]), barriers, q["cost"], q["reorder"], at)
    elif q["kind"] == "area":
        out["expect"] = run_area(net, pick(q["facilities"]), q["breaks"], q["cost"], q["toward"], barriers, at)
    elif q["kind"] == "closest":
        out["expect"] = run_nearest(net, pick(q["origins"]), pick(q["targets"]), q["k"], q.get("cutoff"), q["cost"], q["reverse"], barriers, True, at)
    elif q["kind"] == "matrix":
        out["expect"] = run_nearest(net, pick(q["origins"]), pick(q["targets"]), None, None, q["cost"], False, barriers, False, at)
    elif q["kind"] == "check":
        out["expect"] = run_check(net)
    return out


def clean(v):
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
    data = clean(build())
    text = json.dumps(data, ensure_ascii=False, indent=1, allow_nan=False) + "\n"
    questions = sum(len(c["questions"]) for c in data["cases"])
    if args.check:
        old = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if json.loads(old or "null") != json.loads(text):
            print(f"{OUT.relative_to(ROOT)} is not what this script writes: run it without --check and read the difference.")
            return 1
        print(f"{OUT.relative_to(ROOT)} matches ({len(data['cases'])} scenes, {questions} questions)")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} written ({len(data['cases'])} scenes, {questions} questions)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
