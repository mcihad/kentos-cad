#!/usr/bin/env python3
"""Independent reference of why two stops of a network have no way between them (docs/adr/0570), written from the ADR
without KentOS code.

One scene of separate chains of straight edges (so that a closed edge cuts the way), each edge closed in its own way:
by the closed edges' expression, the direction field's closed value, one way, a vehicle profile's limit, a dated
closure, a closure's hours, an empty value of a field cost; one chain with a barrier, one place where the shortest
way is closed and a longer one is one way the other way. The questions: a route between two corners for a cost, a
profile, a day and a time; whether there is a way, and if not, why: the places on parts that do not meet, or the
way, every edge both ways and nothing closed, that the fewest blockers shut (the shortest of those) and, in its order, each edge the question's cost cannot travel the way it
goes (its object and reason, an edge once) and each barrier on it: on the way that the fewest of them shut, the
shortest of those (docs/adr/0575). A ramp onto a bridge over a road meets the road where both have a place, but its
level keeps them apart: a way may step from one to the other, the step shutting it once (docs/adr/0640 §3). And
service areas within their last break: what cuts each off where it stops (docs/adr/0640 §2) and how many edges lie on
parts no facility meets. Written to fixtures/network/v1/blocked.json; the geometry core
(crates/shared/geometry-core/tests/all/network_access.rs) must give the same.

The ADR's reasons, in this order: the closed edges' expression; the profile, the day, the hour (only a cost without a
clock, at the question's moment); the direction field's closed value; one way the other way; no value of the cost.
Every best way is checked to be the only one.

A service area's cut, from the ADR: one search from the facility to the last break, every move open (the edge, the
node passed, the turn). Each edge the area reaches from neither end (a stretch entered from a node within the last
break at a finite cost the edge travels), set off from an end the area reaches within it, says what stops it there:
the edge closed the way from that end (and why), else the turn from the edge the node is reached by at its least
cost. A closed junction the area arrives at within the break (by an edge) says itself. And each node reached within the
break whose level keeps a node beside it apart that the area does not reach: the two places' first edges. In the
order of the reaching nodes' costs (edges before levels, then the edges' order), each once.

    python3 scripts/fixtures/network_blocked_cases.py [--check]
"""

import argparse
import datetime
import heapq
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/blocked.json"
X0, Y0 = 600000.0, 4300000.0
STEP = 100.0

DEFINITION = {
    "id": "ag-1",
    "name": "Yollar",
    "kind": "road",
    "edges": [{"layer": "yol"}],
    "connect": "ends",
    "tolerance": 0.01,
    "direction": {"kind": "field", "field": "yon", "forward": ["FT"], "backward": ["TF"], "closed": ["N"]},
    "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0},
              {"name": "Ücret", "kind": "field", "field": "ucret", "unit": "TL"}],
    "profiles": [{"name": "Kamyon", "limits": [{"field": "agirlik", "value": 18.0}]}],
    "closures": {"from": "kapanis_bas", "to": "kapanis_son", "hours": "kapanis_saat"},
    # A closed junction and a forbidden turn shut a way at a node (docs/adr/0577); no U-turn anywhere, so that a
    # forbidden turn is not gone round by turning back at a dead end.
    "junctions": [{"layer": "dugum", "role": "junction", "closed": "kapali = 'evet'"}],
    "turns": {"uTurns": "never", "layers": [{"layer": "donus"}]},
    # An edge's level at its start and end (docs/adr/0521 §3): empty is the ground's, 0.
    "levels": {"start": "seviye_bas", "end": "seviye_son"},
}
FORWARD, BACKWARD, CLOSED = {"ft"}, {"tf"}, {"n"}


def corner(i, j):
    """A corner, moved by a few metres (a fixed sequence) so that no two ways are of one length."""
    k = i * 7 + j
    return (X0 + i * STEP + ((k * k * 3 + k * 7) % 19) * 0.53, Y0 + j * STEP + ((k * k * 5 + k * 13) % 23) * 0.47)


# Each edge: its two corners (drawn from the first), its attributes (absent: empty) and whether the expression closes it.
EDGES = [
    # Chain 0: open, closed by the expression, closed by the direction field.
    ((0, 0), (1, 0), {"ucret": "5"}, False),
    ((1, 0), (2, 0), {"ucret": "5"}, True),
    ((2, 0), (3, 0), {"ucret": "5", "yon": "N"}, False),
    # Chain 1: one way east, a weight limit of 10 t, closed from 1 to 5 November.
    ((0, 1), (1, 1), {"ucret": "5", "yon": "FT"}, False),
    ((1, 1), (2, 1), {"ucret": "5", "agirlik": "10"}, False),
    ((2, 1), (3, 1), {"ucret": "5", "kapanis_bas": "2026-11-01", "kapanis_son": "2026-11-05"}, False),
    # Chain 2: closed from 08:00 to 10:00, no fare, open.
    ((0, 2), (1, 2), {"ucret": "5", "kapanis_saat": "08:00-10:00"}, False),
    ((1, 2), (2, 2), {}, False),
    ((2, 2), (3, 2), {"ucret": "5"}, False),
    # Chain 3: two open edges, a barrier on the second in the questions.
    ((0, 3), (1, 3), {"ucret": "5"}, False),
    ((1, 3), (2, 3), {"ucret": "5"}, False),
    # A square: the short side closed by the expression, the long way round one way the other way on its last side.
    ((0, 5), (1, 5), {"ucret": "5"}, True),
    ((0, 5), (0, 6), {"ucret": "5"}, False),
    ((0, 6), (1, 6), {"ucret": "5"}, False),
    ((1, 5), (1, 6), {"ucret": "5", "yon": "FT"}, False),
    # The fewest blockers (docs/adr/0575): the straight way has two closed edges, the way round one.
    ((0, 8), (1, 8), {"ucret": "5"}, True),
    ((1, 8), (2, 8), {"ucret": "5"}, True),
    ((0, 8), (0, 10), {"ucret": "5"}, False),
    ((0, 10), (2, 10), {"ucret": "5", "yon": "N"}, False),
    ((2, 10), (2, 8), {"ucret": "5"}, False),
    # A closed junction in the middle of an open chain (docs/adr/0577).
    ((0, 12), (1, 12), {"ucret": "5"}, False),
    ((1, 12), (2, 12), {"ucret": "5"}, False),
    # A T whose turn from its stem's west arm north is forbidden.
    ((0, 14), (1, 14), {"ucret": "5"}, False),
    ((1, 14), (2, 14), {"ucret": "5"}, False),
    ((1, 14), (1, 15), {"ucret": "5"}, False),
    # A ramp from the ground up onto a bridge that goes on north, over a road that goes on east (docs/adr/0640 §3).
    ((0, 17), (1, 17), {"ucret": "5", "seviye_son": "1"}, False),
    ((1, 17), (2, 17), {"ucret": "5"}, False),
    ((1, 17), (1, 18), {"ucret": "5", "seviye_bas": "1", "seviye_son": "1"}, False),
]
# Junctions: their corner and whether they are closed.
JUNCTIONS = [((1, 12), True)]
# Forbidden turns: from the edge between the first two corners to the edge between the last two, through the middle.
TURNS = [((0, 14), (1, 14), (1, 15))]
BARRIER = ((1, 3), (2, 3), 0.4)
JUNCTION_ID = 100  # the junctions' object ids, from 100
TURN_ID = 200  # the turn lines' object ids, from 200  # the barrier's edge by its corners and the share of the way along it


def fold(s):
    return "".join({"I": "ı", "İ": "i"}.get(c, c.lower()) for c in s.strip())


def number(t):
    try:
        v = float(t.strip().replace(",", "."))
    except ValueError:
        return None
    return v if math.isfinite(v) else None


def day_of(t):
    return (datetime.date(int(t[0:4]), int(t[5:7]), int(t[8:10])) - datetime.date(1970, 1, 1)).days


def hours_of(t):
    out = []
    for part in t.split(","):
        a, b = part.split("-")
        (ah, am), (bh, bm) = (map(int, a.strip().split(":")), map(int, b.strip().split(":")))
        out.append((ah * 60 + am, bh * 60 + bm))
    return out


def closing(attrs, profile, day, time, cost):
    """Why the access closes an edge to `cost`: `profile`, `date`, `hours` or None."""
    if profile:
        for lim in DEFINITION["profiles"][0]["limits"]:
            v = attrs.get(lim["field"], "").strip()
            if v and number(v) is not None and number(v) < lim["value"]:
                return "profile"
    if day is None:
        return None
    a = attrs.get("kapanis_bas", "").strip()
    b = attrs.get("kapanis_son", "").strip()
    h = attrs.get("kapanis_saat", "").strip()
    if not (a or b or h):
        return None
    if (a and day < day_of(a)) or (b and day > day_of(b)):
        return None
    if not h:
        return "date"
    # Hours close a cost without a clock at the question's moment (the time cost waits).
    if time is not None and cost != 1 and any(s <= time < (e if e > s else e + 1440) for s, e in hours_of(h)):
        return "hours"
    return None


def way_of(attrs):
    v = fold(attrs.get("yon", ""))
    return "forward" if v in FORWARD else "backward" if v in BACKWARD else "closed" if v in CLOSED else "both"


def blocker(e, forward, profile, day, time, cost):
    """The reason edge `e` shuts a way going `forward` (the way it is drawn) for the question, or None."""
    a, b, attrs, expr = e
    if expr:
        return "expression"
    why = closing(attrs, profile, day, time, cost)
    if why:
        return why
    way = way_of(attrs)
    if way == "closed":
        return "shut"
    if (way == "forward" and not forward) or (way == "backward" and forward):
        return "oneWay"
    if cost == 2 and number(attrs.get("ucret", "")) is None:
        return "cost"
    return None


def level(attrs, key):
    v = attrs.get(key, "").strip()
    return int(v) if v else 0


def ends(e):
    """An edge's two places: its corners at its start's and end's levels."""
    return (e[0], level(e[2], "seviye_bas")), (e[1], level(e[2], "seviye_son"))


def place_of(c):
    """The one place at corner `c` (a stop's)."""
    found = {p for e in EDGES for p in ends(e) if p[0] == c}
    assert len(found) == 1, f"{c}: {found}"
    return found.pop()


def gaps_of(n):
    """The places at node n's corner at another level."""
    return sorted({p for e in EDGES for p in ends(e) if p[0] == n[0] and p[1] != n[1]})


def length(e):
    (ax, ay), (bx, by) = corner(*e[0]), corner(*e[1])
    return math.hypot(bx - ax, by - ay)


def shortest(a, b, usable):
    """The shortest way from corner a to b by length over the edges `usable(e, forward)` lets through: its steps
    (edge index, forward) and whether it is the only shortest; None when there is none."""
    adj = {}
    for k, e in enumerate(EDGES):
        p, q = ends(e)
        if usable(e, True):
            adj.setdefault(p, []).append((q, k, True))
        if usable(e, False):
            adj.setdefault(q, []).append((p, k, False))
    a, b = place_of(a), place_of(b)
    best, came, count = {a: 0.0}, {a: None}, {a: 1}
    heap, done = [(0.0, a)], set()
    while heap:
        d, n = heapq.heappop(heap)
        if n in done:
            continue
        done.add(n)
        for m, k, fw in adj.get(n, []):
            nd = d + length(EDGES[k])
            if m not in best or nd < best[m] - 1e-9:
                best[m], came[m], count[m] = nd, (n, k, fw), count[n]
                heapq.heappush(heap, (nd, m))
            elif abs(nd - best[m]) <= 1e-9:
                count[m] += count[n]
    if b not in best:
        return None, True
    steps, n = [], b
    while came[n] is not None:
        p, k, fw = came[n]
        steps.append((k, fw))
        n = p
    return list(reversed(steps)), count[b] == 1


def edge_between(p, q):
    return next(k for k, e in enumerate(EDGES) if {e[0], e[1]} == {p, q})


def node_cost(n, came, k):
    """What passing place n from edge `came` onto edge `k` adds (docs/adr/0577): a closed junction, else a U-turn
    (never allowed) or a forbidden turn; nothing at the first place."""
    n = n[0]
    if came is None:
        return 0, None
    if any(c == n and closed for c, closed in JUNCTIONS):
        return 1, ("junction", n)
    if came == k:
        return 1, ("turn", came, k)
    for a, m, b in TURNS:
        if m == n and edge_between(a, m) == came and edge_between(m, b) == k:
            return 1, ("turn", came, k)
    return 0, None


def passes(a, steps):
    """Each step of a way with the place passed before it: (place, edge arrived by, edge left by, a level's step)."""
    out, here, came_by = [], place_of(a), None
    for k, fw, jump in steps:
        out.append((here, came_by, k, jump))
        here = ends(EDGES[k])[1] if fw else ends(EDGES[k])[0]
        came_by = k
    return out


def step_cost(here, came_by, k, jump):
    """What the place passed before step k adds: a level's step 1, else the node's."""
    return 1 if jump else node_cost(here, came_by, k)[0]


def fewest(a, b, cost_of):
    """The way from corner a to b that the fewest blockers shut, the shortest of those (docs/adr/0575, 0577): every
    edge both ways, `cost_of(k, forward)` the blockers stepping on edge k adds, each node passed what `node_cost` adds.
    The search's states are (corner, edge arrived by). Its steps and whether it is the only best; None when the corners
    do not meet."""
    adj = {}
    for k, e in enumerate(EDGES):
        p, q = ends(e)
        adj.setdefault(p, []).append((q, k, True))
        adj.setdefault(q, []).append((p, k, False))
    a, b = place_of(a), place_of(b)
    start = (a, None)
    best, came, count = {start: (0, 0.0)}, {start: None}, {start: 1}
    heap, done = [(0, 0.0, 0, start)], set()
    order = {}
    while heap:
        n_cut, d, _, st = heapq.heappop(heap)
        if st in done:
            continue
        done.add(st)
        n, via = st
        # From the place itself, or (once, a blocker) from a place its level keeps apart beside it.
        for here, jump in [(n, False)] + [(g, True) for g in gaps_of(n)]:
            for m, k, fw in adj.get(here, []):
                extra = 1 if jump else node_cost(n, via, k)[0]
                nd = (n_cut + extra + cost_of(k, fw), d + length(EDGES[k]))
                to = (m, k)
                if to not in best or nd[0] < best[to][0] or (nd[0] == best[to][0] and nd[1] < best[to][1] - 1e-9):
                    best[to], came[to], count[to] = nd, (st, k, fw, jump), count[st]
                    order.setdefault(to, len(order) + 1)
                    heapq.heappush(heap, (nd[0], nd[1], order[to], to))
                elif nd[0] == best[to][0] and abs(nd[1] - best[to][1]) <= 1e-9:
                    count[to] += count[st]
    # The stop's place, or one beside it its level keeps apart (a step more).
    arrivals = [((best[st][0] + (0 if st[0] == b else 1), best[st][1]), st) for st in best
                if st[0] == b or st[0] in gaps_of(b)]
    if not arrivals:
        return None, True
    top = min(c for c, _ in arrivals)
    tops = [st for c, st in arrivals if c[0] == top[0] and abs(c[1] - top[1]) <= 1e-9]
    st = tops[0]
    assert st[0] == b, "no question here ends by a level's step"
    only = len(tops) == 1 and count[st] == 1
    steps = []
    while came[st] is not None:
        p, k, fw, jump = came[st]
        steps.append((k, fw, jump))
        st = p
    return list(reversed(steps)), only


QUESTIONS = [
    # (name, from, to, cost, profile, day, time, barrier)
    ("ifade-ve-yon-alani", (0, 0), (3, 0), 0, False, None, None, False),
    ("ters-yon-profil-tarih", (3, 1), (0, 1), 1, True, "2026-11-03", None, False),
    ("ileri-acik", (0, 1), (3, 1), 0, False, None, None, False),
    ("ileri-profil", (0, 1), (3, 1), 0, True, None, None, False),
    ("ileri-tarih", (0, 1), (3, 1), 0, False, "2026-11-05", None, False),
    ("ileri-tarih-sonrasi", (0, 1), (3, 1), 0, False, "2026-11-06", None, False),
    ("saat-uzunluk", (0, 2), (3, 2), 0, False, "2026-11-03", 540.0, False),
    ("saat-sure-bekler", (0, 2), (3, 2), 1, False, "2026-11-03", 540.0, False),
    ("saat-disi", (0, 2), (3, 2), 0, False, "2026-11-03", 600.0, False),
    ("ucret-yok", (0, 2), (3, 2), 2, False, None, None, False),
    ("engel", (0, 3), (2, 3), 0, False, None, None, True),
    ("kopuk", (0, 0), (2, 3), 0, False, None, None, False),
    ("kisa-yol-kapali", (0, 5), (1, 5), 0, False, None, None, False),
    ("kisa-yol-ters", (1, 5), (0, 5), 0, False, None, None, False),
    ("en-az-kapatan", (0, 8), (2, 8), 0, False, None, None, False),
    ("kapali-dugum", (0, 12), (2, 12), 0, False, None, None, False),
    ("yasak-donus", (0, 14), (1, 15), 0, False, None, None, False),
    ("izinli-donus", (2, 14), (1, 15), 0, False, None, None, False),
    # The ramp meets the road under it only by a level's step; the bridge goes on (docs/adr/0640 §3).
    ("seviye", (0, 17), (2, 17), 0, False, None, None, False),
    ("seviye-kopru", (0, 17), (1, 18), 0, False, None, None, False),
]


# Service areas (docs/adr/0640 §2): (name, facility, breaks, cost, profile, day, time, barrier).
AREAS = [
    ("alan-profil", (0, 1), [150.0, 400.0], 0, True, None, None, False),
    ("alan-kare", (0, 5), [150.0, 400.0], 0, False, None, None, False),
    ("alan-dugum", (0, 12), [150.0, 400.0], 0, False, None, None, False),
    ("alan-donus", (0, 14), [150.0, 400.0], 0, False, None, None, False),
    ("alan-seviye", (0, 17), [150.0, 400.0], 0, False, None, None, False),
    ("alan-mola", (0, 1), [50.0], 0, True, None, None, False),
    ("alan-saat", (0, 2), [400.0], 0, False, "2026-11-03", 540.0, False),
    ("alan-ucret", (0, 2), [20.0], 2, False, None, None, False),
    ("alan-en-az", (0, 8), [150.0, 400.0], 0, False, None, None, False),
    ("alan-engel", (0, 3), [150.0, 400.0], 0, False, None, None, True),
    ("alan-engel-mola", (0, 3), [120.0], 0, False, None, None, True),
]


def edge_cost(k, fw, cost):
    return number(EDGES[k][2].get("ucret", "")) if cost == 2 else length(EDGES[k])


def area_cut(facility, breaks, cost, profile, day, time, with_barrier):
    """What cuts a service area off within its last break, and the edges on parts no facility meets."""
    last = breaks[-1]
    (ba, bb, share) = BARRIER
    barrier_edge = next(k for k, e in enumerate(EDGES) if (e[0], e[1]) == (ba, bb))

    def why(k, fw):
        return blocker(EDGES[k], fw, profile, day, time, cost)

    adj = {}
    for k, e in enumerate(EDGES):
        p, q = ends(e)
        adj.setdefault(p, []).append((q, k, True))
        adj.setdefault(q, []).append((p, k, False))
    start = (place_of(facility), None)
    best, heap, done = {start: 0.0}, [(0.0, 0, start)], set()
    order = {start: 0}
    while heap:
        d, _, st = heapq.heappop(heap)
        if st in done or d > last:
            continue
        done.add(st)
        n, via = st
        for m, k, fw in adj.get(n, []):
            if why(k, fw) or node_cost(n, via, k)[0] or (with_barrier and k == barrier_edge):
                continue
            nd = d + edge_cost(k, fw, cost)
            to = (m, k)
            if to not in best or nd < best[to] - 1e-12:
                best[to] = nd
                order.setdefault(to, len(order))
                heapq.heappush(heap, (nd, order[to], to))
    inf = float("inf")

    def at(n):
        return min((c for st, c in best.items() if st[0] == n), default=inf)

    def enter(n, k):
        return min((c for st, c in best.items() if st[0] == n and node_cost(n, st[1], k)[0] == 0), default=inf)

    def enter_far(k, n):
        """The least cost of arriving at place n along edge k."""
        return min((c for st, c in best.items() if st == (n, k)), default=inf)

    found = []
    for k, e in enumerate(EDGES):
        p, q = ends(e)
        if any(enter(n, k) < last and not why(k, fw) for n, fw in ((p, True), (q, False))):
            continue
        for side, (n, fw) in enumerate(((p, True), (q, False))):
            cost_n = at(n)
            if not cost_n < last:
                continue
            if why(k, fw):
                b = ("edge", k + 1, why(k, fw))
            elif any(c == n[0] and closed for c, closed in JUNCTIONS):
                continue
            else:
                via = min((st for st in best if st[0] == n), key=lambda st: best[st])[1]
                if via is None:
                    continue
                b = ("turn", via + 1, k + 1)
            found.append((cost_n, 0, 2 * k + side, b))
    # The barrier, arrived at along its edge from its start (the scene's).
    if with_barrier:
        arrive = enter(ends(EDGES[barrier_edge])[0], barrier_edge) + share * length(EDGES[barrier_edge])
        if arrive < last:
            found.append((arrive, 0, 2 * barrier_edge, ("barrier", 0)))
    for c, closed in JUNCTIONS:
        n = place_of(c)
        if closed and at(n) < last and n != start[0]:
            k = min((k for k, e in enumerate(EDGES) if n in ends(e)), key=lambda k: enter_far(k, n))
            found.append((at(n), 0, 2 * k, ("junction", JUNCTION_ID + [c for c, _ in JUNCTIONS].index(c))))
    for n in sorted({p for e in EDGES for p in ends(e)}):
        if not at(n) < last:
            continue
        for m in gaps_of(n):
            if at(m) < last:
                continue
            first = lambda x: 1 + min(k for k, e in enumerate(EDGES) if x in ends(e))
            found.append((at(n), 1, 0, ("level", first(n), first(m))))
    found.sort(key=lambda f: (f[0], f[1], f[2]))
    said = []
    for f in found:
        if f[3] not in said:
            said.append(f[3])
    by = {"kind": "by", "edges": [{"id": b[1], "why": b[2]} for b in said if b[0] == "edge"],
          "barriers": [b[1] for b in said if b[0] == "barrier"]}
    junctions = [{"id": b[1], "why": "closed"} for b in said if b[0] == "junction"]
    turns = [{"from": b[1], "to": b[2]} for b in said if b[0] == "turn"]
    levels = [{"from": b[1], "to": b[2]} for b in said if b[0] == "level"]
    if junctions:
        by["junctions"] = junctions
    if turns:
        by["turns"] = turns
    if levels:
        by["levels"] = levels
    # The parts: every edge both ways and the levels' steps.
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            x = parent[x]
        return x

    for e in EDGES:
        p, q = ends(e)
        parent[find(p)] = find(q)
    for n in list({p for e in EDGES for p in ends(e)}):
        for m in gaps_of(n):
            parent[find(n)] = find(m)
    home = find(place_of(facility))
    apart = sum(1 for e in EDGES if find(ends(e)[0]) != home)
    return {"blocked": by, "apart": apart}


def build():
    edges = [{"id": i + 1, "pts": [list(corner(*e[0])), list(corner(*e[1]))], "attrs": e[2],
              **({"closed": True} if e[3] else {})} for i, e in enumerate(EDGES)]
    (ba, bb, share) = BARRIER
    (ax, ay), (bx, by) = corner(*ba), corner(*bb)
    barrier_at = [ax + (bx - ax) * share, ay + (by - ay) * share]
    barrier_edge = next(k for k, e in enumerate(EDGES) if (e[0], e[1]) == (ba, bb))
    cases = []
    for name, a, b, cost, profile, day_text, time, with_barrier in QUESTIONS:
        day = day_of(day_text) if day_text else None

        def open_(e, fw):
            k = EDGES.index(e)
            if with_barrier and k == barrier_edge:
                return False
            return blocker(e, fw, profile, day, time, cost) is None

        def cut(k, fw):
            n = 1 if blocker(EDGES[k], fw, profile, day, time, cost) else 0
            return n + (1 if with_barrier and k == barrier_edge else 0)

        # A way exists when a way no blocker shuts exists: the closed edges, barriers, closed junctions and turns.
        found, _ = shortest(a, b, open_)
        clear, _ = fewest(a, b, cut)
        if clear is not None:
            free = sum(cut(k, fw) for k, fw, _ in clear) + sum(
                step_cost(here, came, k, jump) for here, came, k, jump in passes(a, clear))
            if free > 0:
                found = None
        case = {"name": name, "from": list(corner(*a)), "to": list(corner(*b)), "cost": cost,
                "profile": "Kamyon" if profile else None, "day": day_text, "time": time,
                "barriers": [barrier_at] if with_barrier else []}
        if found is not None:
            case["reachable"] = True
            cases.append(case)
            continue
        steps, only = fewest(a, b, cut)
        assert only, f"{name}: more than one best way"
        if steps is None:
            case["blocked"] = {"kind": "apart"}
        else:
            edges_out, barriers_out, junctions_out, turns_out, levels_out = [], [], [], [], []
            here, came_by = place_of(a), None
            for k, fw, jump in steps:
                at = None if jump else node_cost(here, came_by, k)[1]
                if jump:
                    levels_out.append({"from": came_by + 1, "to": k + 1})
                elif at and at[0] == "junction":
                    jid = JUNCTION_ID + [c for c, _ in JUNCTIONS].index(at[1])
                    junctions_out.append({"id": jid, "why": "closed"})
                elif at:
                    turns_out.append({"from": at[1] + 1, "to": at[2] + 1})
                here = ends(EDGES[k])[1] if fw else ends(EDGES[k])[0]
                came_by = k
                why = blocker(EDGES[k], fw, profile, day, time, cost)
                if why and all(x["id"] != k + 1 for x in edges_out):
                    edges_out.append({"id": k + 1, "why": why})
                if with_barrier and k == barrier_edge and 0 not in barriers_out:
                    barriers_out.append(0)
            by = {"kind": "by", "edges": edges_out, "barriers": barriers_out}
            if junctions_out:
                by["junctions"] = junctions_out
            if turns_out:
                by["turns"] = turns_out
            if levels_out:
                by["levels"] = levels_out
            case["blocked"] = by if edges_out or barriers_out or junctions_out or turns_out or levels_out else {
                "kind": "other"}
        cases.append(case)
    junctions = [{"id": JUNCTION_ID + i, "p": list(corner(*c)), "attrs": {"kapali": "evet"} if closed else {}}
                 for i, (c, closed) in enumerate(JUNCTIONS)]

    def toward(p, q, share=0.5):
        (px, py), (qx, qy) = corner(*p), corner(*q)
        return [px + (qx - px) * share, py + (qy - py) * share]

    turns = [{"id": TURN_ID + i, "pts": [toward(m, a), list(corner(*m)), toward(m, b)]}
             for i, (a, m, b) in enumerate(TURNS)]
    areas = [{"name": name, "facility": list(corner(*f)), "breaks": breaks, "cost": cost,
              "profile": "Kamyon" if profile else None, "day": day_text, "time": time,
              "barriers": [barrier_at] if with_barrier else [],
              "cut": area_cut(f, breaks, cost, profile, day_of(day_text) if day_text else None, time, with_barrier)}
             for name, f, breaks, cost, profile, day_text, time, with_barrier in AREAS]
    return {"format": "kentos.network-blocked", "version": 1, "definition": DEFINITION, "edges": edges,
            "junctions": junctions, "turns": turns, "cases": cases, "areas": areas}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT} güncel değil; betiği --check'siz çalıştırın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel: {len(QUESTIONS)} soru, {len(AREAS)} hizmet alanı.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(QUESTIONS)} soru, {len(AREAS)} hizmet alanı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
