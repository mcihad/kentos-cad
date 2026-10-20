#!/usr/bin/env python3
"""Independent reference of terminals and subnetworks in utility network tracing (docs/adr/0557), written from the ADR
without KentOS code.

Four scenes of straight pipes and cables whose ends meet exactly: a transformer (two terminals, one path) between a
source and two feeders; a switch with three terminals (a and b on a path, c on none) with the edges' directions; two
sources with a shared junction between them, an unreachable pipe and a closed valve; and a device whose edge names no
terminal. The questions: traces (Bağlı, Akış aşağı, Akış yukarı and Yalıtım from the middle of an edge or from a node)
and the subnetworks of groups of controllers (what they reach: edges, junctions, length; Akış aşağı or Bağlı). Written to
fixtures/network/v1/terminals.json; the geometry core (crates/shared/geometry-core/tests/all/network_terminals.rs) must
give the same, edges and junctions by their order in the scene.

This is another program than the core's: nodes are the exact coordinates the ends meet at, an edge's ends are looked up
there, and the walk is a plain search over (node, terminal) pairs written from the ADR's rules: a way that reached a
device by a terminal goes on along the edges attached to a terminal on a path with it (a terminal always connects to
itself); an end that names no terminal, or one the device has not, passes to every terminal, both ways; a start on a
device itself starts at no terminal. A trace stops at closed junctions (reached, not passed), Yalıtım also at open
valves; a subnetwork stops at closed junctions and at a node of a controller of another group.

ADR 0562 adds what the searches do at a device: a shortest route, the cost matrix, the closest facilities and the
service areas cross a device only along its terminal paths (the same rule, in the search: Dijkstra over (node,
terminal) states, written here from the rule alone), and a device's terminal configuration may come from an attribute
of the device (`by`, `configs`: the scenes whose devices carry them say so). A reversed question (reaching the origin,
Tesise) is the forward one over the flipped edges. The questions: routes through stops (cost and the edges in the order
they are travelled; none when a stop is not reached), a matrix of costs both ways with the closest facilities of each
origin, and service areas (the length reached on each edge within a break). Locations are an edge and the fraction of
its length, or a node.

    python3 scripts/fixtures/network_terminals_cases.py [--check]
"""

import argparse
import copy
import json
import math
import sys
from collections import deque
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/terminals.json"
sys.dont_write_bytecode = True


def folded(text):
    """Trimmed, case folded with Turkish I and İ (the rules' comparison)."""
    return "".join({"I": "ı", "İ": "i"}.get(c, c.lower()) for c in text.strip())


class Scene:
    def __init__(self, scene):
        self.scene = scene
        rules = scene["rules"]
        self.digitized = rules["direction"]["kind"] == "digitized"
        self.devices = (rules.get("terminals") or {}).get("devices", [])
        self.nodes = {}
        for e in scene["edges"]:
            for p in (e["pts"][0], e["pts"][-1]):
                self.nodes.setdefault(tuple(p), len(self.nodes))
        for j in scene["junctions"]:
            self.nodes.setdefault(tuple(j["p"]), len(self.nodes))
        self.edges = []
        self.unterminated = []
        # The device of a node: that of its first junction that has one.
        self.device_at = {}
        for j in scene["junctions"]:
            if j.get("device") is not None:
                self.device_at.setdefault(tuple(j["p"]), self.configured(j))
        for e in scene["edges"]:
            a, b = tuple(e["pts"][0]), tuple(e["pts"][-1])
            term = [self.terminal(a, e["terminals"][0]), self.terminal(b, e["terminals"][1])]
            for end, at in ((0, a), (1, b)):
                if at in self.device_at and term[end] is None:
                    self.unterminated.append([e["id"], list(at)])
            self.edges.append({"a": a, "b": b, "t": term, "closed": e.get("closed", False), "id": e["id"],
                               "length": math.dist(a, b)})
        self.unterminated = sorted(self.unterminated, key=lambda u: (u[0], u[1][0], u[1][1]))
        self.junction_nodes = [tuple(j["p"]) for j in scene["junctions"]]
        # A closed junction (a closed valve) is reached and not passed.
        self.closed_nodes = {tuple(j["p"]) for j in scene["junctions"] if j.get("closed")}

    def configured(self, j):
        """The terminals and paths of a junction's device: its configuration's when its attribute value (`config`)
        names one of the device's `configs` (caseless), else the device's own (docs/adr/0562 §2)."""
        d = self.devices[j["device"]]
        value = j.get("config")
        if value is not None and folded(value):
            for c in d.get("configs", []):
                if folded(c["value"]) == folded(value):
                    return {"terminals": c["terminals"], "paths": c["paths"]}
        return {"terminals": d["terminals"], "paths": d["paths"]}

    def terminal(self, at, value):
        """The index of the terminal an end names at its node's device, None when it names none or is not at a device."""
        d = self.device_at.get(at)
        if d is None or value is None:
            return None
        key = folded(value)
        if not key:
            return None
        for i, t in enumerate(d["terminals"]):
            if folded(t["id"]) == key or folded(t["name"]) == key:
                return i
        return None

    def linked(self, node, a, b):
        d = self.device_at.get(node)
        if d is None or a is None or b is None or a == b:
            return True
        ids = [t["id"] for t in d["terminals"]]
        return any(ids[a] in path and ids[b] in path for path in d["paths"])

    def direction_allows(self, e, forward, kind):
        if e["closed"]:
            return False
        if kind in ("connected", "isolation"):
            return True
        if not self.digitized:
            return True
        return forward if kind == "downstream" else not forward


def walk(sc, starts, kind, stops=lambda n: False, junction_closed=None, valve_nodes=frozenset(), own_nodes=frozenset()):
    """The search over (node, terminal): starts are ("node", node) or ("edge", edge index). Returns the reached
    edges, the touched nodes and the valves' nodes the walk stopped at."""
    closed_nodes = junction_closed
    reached = set()
    touched = set()
    valves = set()
    seen = set()
    queue = deque()

    def arrive(node, term):
        touched.add(node)
        if node in closed_nodes or stops(node):
            return
        if kind == "isolation" and node in valve_nodes and node not in own_nodes:
            valves.add(node)
            return
        if (node, term) not in seen:
            seen.add((node, term))
            queue.append((node, term))

    for s in starts:
        if s[0] == "node":
            touched.add(s[1])
            seen.add((s[1], None))
            queue.append((s[1], None))
        else:
            i = s[1]
            e = sc.edges[i]
            for forward, node, term in ((True, e["b"], e["t"][1]), (False, e["a"], e["t"][0])):
                if sc.direction_allows(e, forward, kind):
                    reached.add(i)
                    arrive(node, term)
    while queue:
        node, term = queue.popleft()
        for i, e in enumerate(sc.edges):
            for forward, here, there, t_out, t_in in ((True, e["a"], e["b"], e["t"][0], e["t"][1]), (False, e["b"], e["a"], e["t"][1], e["t"][0])):
                if here != node or not sc.direction_allows(e, forward, kind):
                    continue
                if not sc.linked(node, term, t_out):
                    continue
                reached.add(i)
                arrive(there, t_in)
    return reached, touched, valves


def trace_case(sc, q):
    kind = q["kind"]
    closed_nodes = {tuple(j["p"]) for j in sc.scene["junctions"] if j.get("closed")}
    valve_nodes = {tuple(j["p"]) for j in sc.scene["junctions"] if j["role"] == "valve" and not j.get("closed")}
    if "edge" in q:
        starts = [("edge", q["edge"])]
        own = frozenset()
    else:
        node = tuple(q["node"])
        starts = [("node", node)]
        own = frozenset([node])
    reached, touched, valves = walk(sc, starts, kind, junction_closed=closed_nodes, valve_nodes=valve_nodes, own_nodes=own)
    junctions = [k for k, p in enumerate(sc.junction_nodes) if p in touched]
    stopped = [k for k, j in enumerate(sc.scene["junctions"])
               if j["role"] == "valve" and not j.get("closed") and tuple(j["p"]) in valves]
    out = {"query": q, "edges": [sc.edges[i]["id"] for i in sorted(reached)], "junctions": junctions}
    if kind == "isolation":
        out["valves"] = stopped
    return out


def subnet_case(sc, q):
    groups = q["groups"]
    kind = "downstream" if q["reach"] == "downstream" else "connected"
    closed_nodes = {tuple(j["p"]) for j in sc.scene["junctions"] if j.get("closed")}
    owners = {}
    for g, members in enumerate(groups):
        for k in members:
            owners.setdefault(sc.junction_nodes[k], set()).add(g)
    out = []
    for g, members in enumerate(groups):
        starts = [("node", sc.junction_nodes[k]) for k in members]
        own = frozenset(sc.junction_nodes[k] for k in members)
        reached, touched, _ = walk(
            sc, starts, kind, stops=lambda n, g=g: any(o != g for o in owners.get(n, ())),
            junction_closed=closed_nodes)
        # A start at a node no edge ends at reaches nothing and touches only itself.
        out.append({
            "edges": sorted(reached),
            "junctions": [k for k, p in enumerate(sc.junction_nodes) if p in touched or p in own],
            "length": sum(sc.edges[i]["length"] for i in reached),
        })
    return {"query": q, "reached": out}


# ─── Routing across devices (docs/adr/0562 §1) ─────────────────────────────────────────────────────────────

INF = float("inf")


def allows(sc, e, forward):
    """Whether a way may travel the edge `forward` (a to b): its direction (digitized: forward only) and not closed."""
    if e["closed"]:
        return False
    if not sc.digitized:
        return True
    return forward


def place(sc, loc):
    """A location as ("edge", index, offset metres) or ("node", coordinates)."""
    if "edge" in loc:
        e = sc.edges[loc["edge"]]
        return ("edge", loc["edge"], e["length"] * loc.get("at", 0.5))
    return ("node", tuple(loc["node"]))


def departures(sc, node, term):
    """The ways on from a node reached by `term`: (edge index, forward, the node it leads to, the terminal it arrives by)
    for each edge end at the node the device lets a way on to."""
    for i, e in enumerate(sc.edges):
        for forward, here, there, t_out, t_in in ((True, e["a"], e["b"], e["t"][0], e["t"][1]),
                                                  (False, e["b"], e["a"], e["t"][1], e["t"][0])):
            if here == node and allows(sc, e, forward) and sc.linked(node, term, t_out):
                yield i, forward, there, t_in


def search_from(sc, origins):
    """The cost to every (node, terminal) state from the origins (places), with the way to it. A start inside an
    edge leaves by both its ends the edge allows; a start at a node starts at no terminal."""
    dist, pred = {}, {}
    heap = []
    import heapq

    def push(state, cost, before):
        if cost < dist.get(state, INF):
            dist[state] = cost
            pred[state] = before
            heapq.heappush(heap, (cost, state[0], -1 if state[1] is None else state[1], state))

    for o in origins:
        if o[0] == "node":
            push((o[1], None), 0.0, None)
        else:
            _, i, x = o
            e = sc.edges[i]
            if allows(sc, e, True):
                push((e["b"], e["t"][1]), e["length"] - x, ("edge", i))
            if allows(sc, e, False):
                push((e["a"], e["t"][0]), x, ("edge", i))
    while heap:
        cost, _, _, state = heapq.heappop(heap)
        if cost > dist[state]:
            continue
        node, term = state
        if node in sc.closed_nodes:
            continue
        for i, forward, there, t_in in departures(sc, node, term):
            push((there, t_in), cost + sc.edges[i]["length"], (state, i))
    return dist, pred


def cost_to(sc, origins, dist, target):
    """The cost to a place and the way: (cost, the state it was reached from or a direct hop, the edges in order)."""
    best = (INF, None)
    if target[0] == "node":
        for state, c in dist.items():
            if state[0] == target[1] and c < best[0]:
                best = (c, ("state", state))
        for o in origins:
            if o[0] == "node" and o[1] == target[1]:
                best = (0.0, ("here",))
        return best
    _, j, y = target
    e = sc.edges[j]
    for (node, term), c in dist.items():
        if node in sc.closed_nodes:
            continue
        if node == e["a"] and allows(sc, e, True) and sc.linked(node, term, e["t"][0]) and c + y < best[0]:
            best = (c + y, ("state", (node, term)))
        if node == e["b"] and allows(sc, e, False) and sc.linked(node, term, e["t"][1]) \
                and c + (e["length"] - y) < best[0]:
            best = (c + (e["length"] - y), ("state", (node, term)))
    for o in origins:
        if o[0] == "edge" and o[1] == j:
            x = o[2]
            if y >= x and allows(sc, e, True) and y - x < best[0]:
                best = (y - x, ("direct",))
            if y < x and allows(sc, e, False) and x - y < best[0]:
                best = (x - y, ("direct",))
    return best


def edges_of_way(sc, pred, origin, target, how):
    """The edge ids in the order a way to `target` travels them: the origin's edge, those between, the target's."""
    out = []
    if origin[0] == "edge":
        out.append(origin[1])
    if how[0] == "state":
        way = []
        state = how[1]
        while pred.get(state) is not None:
            before = pred[state]
            if before[0] == "edge":
                way.append(before[1])
                break
            state, i = before
            way.append(i)
        out.extend(reversed(way))
    if target[0] == "edge":
        out.append(target[1])
    ids = []
    for i in out:
        k = sc.edges[i]["id"]
        if not ids or ids[-1] != k:
            ids.append(k)
    return ids


def route_case(sc, q):
    stops = [place(sc, p) for p in q["stops"]]
    total, ids = 0.0, []
    for a, b in zip(stops, stops[1:]):
        if a == b:
            continue
        dist, pred = search_from(sc, [a])
        cost, how = cost_to(sc, [a], dist, b)
        if cost == INF:
            return {"query": q, "cost": None, "edges": None}
        total += cost
        for k in edges_of_way(sc, pred, a, b, how):
            if not ids or ids[-1] != k:
                ids.append(k)
    return {"query": q, "cost": total, "edges": ids}


def flipped(sc):
    """The scene with every edge turned round: a search over it is the reverse search of the scene."""
    out = copy.copy(sc)
    out.edges = []
    for e in sc.edges:
        out.edges.append({"a": e["b"], "b": e["a"], "t": [e["t"][1], e["t"][0]], "closed": e["closed"],
                          "id": e["id"], "length": e["length"], "flip": True})
    out.digitized = sc.digitized
    out.flipped = True
    return out


def costs_matrix(sc, origins, targets, reverse):
    places = [place(sc, p) for p in origins]
    spots = [place(sc, p) for p in targets]
    work = sc
    if reverse:
        work = flipped(sc)
        mirror = lambda p: p if p[0] == "node" else ("edge", p[1], sc.edges[p[1]]["length"] - p[2])
        places = [mirror(p) for p in places]
        spots = [mirror(p) for p in spots]
    rows = []
    for o in places:
        dist, _ = search_from(work, [o])
        rows.append([cost_to(work, [o], dist, t)[0] for t in spots])
    return rows


def matrix_case(sc, q):
    fwd = costs_matrix(sc, q["origins"], q["targets"], False)
    rev = costs_matrix(sc, q["origins"], q["targets"], True)
    # A matrix read the other way is what the targets cost each other's way: the reverse of origin i from target j.
    shown = lambda rows: [[None if c == INF else c for c in row] for row in rows]
    closest = []
    for k in q.get("closest", []):
        rows = []
        for row in fwd:
            found = sorted(((c, j) for j, c in enumerate(row) if c != INF
                            and (k.get("cutoff") is None or c <= k["cutoff"])))
            rows.append([[j, c] for c, j in found[:k["k"]]])
        closest.append({"k": k["k"], "cutoff": k.get("cutoff"), "rows": rows})
    return {"query": q, "from": shown(fwd), "to": shown(rev), "closest": closest}


def reached_lengths(sc, origins, d):
    """The length reached within cost `d` on each edge (by id), from the origins together."""
    dist, _ = search_from(sc, origins)
    out = {}
    for i, e in enumerate(sc.edges):
        L = e["length"]
        spans = []
        g = min((c for (node, term), c in dist.items()
                 if node == e["a"] and node not in sc.closed_nodes and allows(sc, e, True)
                 and sc.linked(node, term, e["t"][0])), default=INF)
        h = min((c for (node, term), c in dist.items()
                 if node == e["b"] and node not in sc.closed_nodes and allows(sc, e, False)
                 and sc.linked(node, term, e["t"][1])), default=INF)
        if g <= d:
            spans.append((0.0, min(L, d - g)))
        if h <= d:
            spans.append((max(0.0, L - (d - h)), L))
        for o in origins:
            if o[0] == "edge" and o[1] == i:
                x = o[2]
                if allows(sc, e, True):
                    spans.append((x, min(L, x + d)))
                if allows(sc, e, False):
                    spans.append((max(0.0, x - d), x))
        spans.sort()
        total, top = 0.0, -1.0
        lo = None
        for a, b in spans:
            if b <= a:
                continue
            if lo is None or a > top:
                if lo is not None:
                    total += top - lo
                lo, top = a, b
            else:
                top = max(top, b)
        if lo is not None:
            total += top - lo
        if total > 1e-9:
            out[e["id"]] = out.get(e["id"], 0.0) + total
    return out


def area_case(sc, q):
    places = [place(sc, p) for p in q["facilities"]]
    work = sc
    if q.get("toward"):
        work = flipped(sc)
        places = [p if p[0] == "node" else ("edge", p[1], sc.edges[p[1]]["length"] - p[2]) for p in places]
    lengths = reached_lengths(work, places, q["break"])
    return {"query": q, "lengths": [[k, lengths[k]] for k in sorted(lengths)]}


def rules(direction, devices=None):
    r = {"connect": "ends", "tolerance": 0.01, "direction": {"kind": direction}}
    if devices is not None:
        r["terminals"] = {"start": "uc_bas", "end": "uc_son", "devices": devices}
    return r


def edge(id_, a, b, start=None, end=None, closed=False):
    e = {"id": id_, "pts": [list(a), list(b)], "terminals": [start, end]}
    if closed:
        e["closed"] = True
    return e


def junction(id_, p, role="junction", device=None, closed=False, config=None):
    j = {"id": id_, "p": list(p), "role": role}
    if device is not None:
        j["device"] = device
    if config is not None:
        j["config"] = config
    if closed:
        j["closed"] = True
    return j


def scene_transformer():
    devices = [{"layer": "trafo", "terminals": [{"id": "yg", "name": "Yüksek gerilim"}, {"id": "ag", "name": "Alçak gerilim"}],
                "paths": [["yg", "ag"]]}]
    return {
        "name": "Trafo: yüksek ve alçak taraf",
        "rules": rules("both", devices),
        "edges": [
            edge(1, (0, 0), (100, 0), end="yg"),
            edge(2, (100, 0), (200, 0), start="Alçak gerilim"),
            edge(3, (200, 0), (300, 0)),
            edge(4, (200, 0), (200, 80)),
            edge(5, (100, 0), (100, 60), start="YG"),
            edge(6, (500, 0), (600, 0)),
        ],
        "junctions": [junction(10, (0, 0), "source"), junction(11, (100, 0), device=0), junction(12, (200, 0), "valve")],
    }


def scene_switch():
    devices = [{"layer": "ayirici", "terminals": [{"id": "a", "name": "A"}, {"id": "b", "name": "B"}, {"id": "c", "name": "C"}],
                "paths": [["a", "b"]]}]
    return {
        "name": "Üç terminalli ayırıcı: a ile b bir yolda, c hiçbirinde",
        "rules": rules("digitized", devices),
        "edges": [
            edge(1, (0, 0), (100, 0), end="a"),
            edge(2, (100, 0), (200, 0), start="b"),
            edge(3, (100, 0), (100, 80), start="c"),
            edge(4, (200, 0), (300, 0)),
            edge(5, (100, -80), (100, 0), end="C"),
        ],
        "junctions": [junction(20, (0, 0), "source"), junction(21, (100, 0), device=0), junction(22, (200, 0), "valve")],
    }


def scene_subnets():
    return {
        "name": "İki kaynak: ortak düğüm, kaynaksız boru ve kapalı vana",
        "rules": rules("digitized"),
        "edges": [
            edge(1, (0, 0), (100, 0)),
            edge(2, (100, 0), (200, 0)),
            edge(3, (200, 0), (300, 0)),
            edge(4, (400, 0), (300, 0)),
            edge(5, (500, 0), (400, 0)),
            edge(6, (700, 0), (800, 0)),
            edge(7, (100, 0), (100, 90)),
        ],
        "junctions": [
            junction(30, (0, 0), "source"),
            junction(31, (500, 0), "source"),
            junction(32, (300, 0)),
            junction(33, (200, 0), "valve", closed=True),
            junction(34, (100, 90), "valve"),
        ],
    }


def scene_unnamed():
    devices = [{"layer": "trafo", "terminals": [{"id": "yg", "name": "YG"}, {"id": "ag", "name": "AG"}], "paths": [["yg", "ag"]]}]
    return {
        "name": "Terminal adı olmayan uç ve cihazda olmayan terminal",
        "rules": rules("both", devices),
        "edges": [
            edge(1, (0, 0), (100, 0), end="yg"),
            edge(2, (100, 0), (200, 0), start="ag"),
            edge(3, (100, 0), (100, 80)),
            edge(4, (100, 0), (100, -80), start="orta gerilim"),
            edge(5, (100, 80), (200, 80), start="yg"),
        ],
        "junctions": [junction(40, (0, 0), "source"), junction(41, (100, 0), device=0)],
    }


def scene_ring():
    devices = [{"layer": "ayirici", "terminals": [{"id": "p", "name": "P"}, {"id": "q", "name": "Q"}, {"id": "r", "name": "R"}],
                "paths": [["p", "q"]]}]
    return {
        "name": "Halka: kısa yol cihazdan geçemez",
        "rules": rules("both", devices),
        "edges": [
            edge(1, (0, 0), (100, 0), end="p"),
            edge(2, (100, 0), (220, 0), start="q"),
            edge(3, (100, 0), (100, 100), start="r"),
            edge(4, (100, 100), (220, 100)),
            edge(5, (220, 0), (220, 100)),
            edge(6, (0, 0), (0, 100)),
            edge(7, (0, 100), (100, 100)),
        ],
        "junctions": [junction(50, (0, 0), "source"), junction(51, (100, 0), device=0)],
    }


def scene_configs():
    devices = [{
        "layer": "ayirici", "terminals": [{"id": "p", "name": "P"}, {"id": "q", "name": "Q"}], "paths": [["p", "q"]],
        "by": "tip",
        "configs": [
            {"value": "trafo", "terminals": [{"id": "yg", "name": "Yüksek gerilim"}, {"id": "ag", "name": "Alçak gerilim"}],
             "paths": [["yg", "ag"]]},
            {"value": "üçlü", "terminals": [{"id": "a", "name": "A"}, {"id": "b", "name": "B"}, {"id": "c", "name": "C"}],
             "paths": [["a", "b"], ["b", "c"]]},
        ],
    }]
    return {
        "name": "Yapılandırmalar: aynı katmanın cihazları noktalarındaki alana göre",
        "rules": rules("both", devices),
        "edges": [
            edge(1, (0, 0), (100, 0), end="yg"),
            edge(2, (100, 0), (200, 0), start="ag"),
            edge(3, (100, 0), (100, 80), start="p"),
            edge(4, (200, 0), (300, 0), end="a"),
            edge(5, (300, 0), (400, 0), start="b"),
            edge(6, (300, 0), (300, 80), start="c"),
            edge(7, (300, 0), (300, -80), start="q"),
            edge(8, (400, 0), (500, 0), end="p"),
            edge(9, (500, 0), (600, 0), start="q", end="p"),
            edge(10, (500, 0), (500, 80), start="ag"),
            edge(11, (600, 0), (700, 0), start="q"),
        ],
        "junctions": [
            junction(60, (0, 0), "source"),
            junction(61, (100, 0), device=0, config="trafo"),
            junction(62, (300, 0), device=0, config="ÜÇLÜ"),
            junction(63, (500, 0), device=0),
            junction(64, (600, 0), device=0, config="bilinmeyen"),
        ],
    }


def scenes():
    out = []
    t = scene_transformer()
    out.append((t, [
        {"kind": "connected", "edge": 0},
        {"kind": "connected", "edge": 1},
        {"kind": "connected", "edge": 4},
        {"kind": "connected", "edge": 5},
        {"kind": "connected", "node": [100, 0]},
        {"kind": "downstream", "edge": 0},
        {"kind": "upstream", "edge": 2},
    ], [
        {"groups": [[0]], "reach": "connected"},
        {"groups": [[0]], "reach": "downstream"},
        {"groups": [[1]], "reach": "connected"},
    ]))
    s = scene_switch()
    out.append((s, [
        {"kind": "connected", "edge": 0},
        {"kind": "connected", "edge": 1},
        {"kind": "connected", "edge": 2},
        {"kind": "connected", "edge": 4},
        {"kind": "connected", "node": [100, 0]},
        {"kind": "downstream", "edge": 0},
        {"kind": "downstream", "edge": 4},
        {"kind": "upstream", "edge": 1},
        {"kind": "isolation", "edge": 1},
        {"kind": "isolation", "edge": 3},
    ], [
        {"groups": [[0]], "reach": "downstream"},
        {"groups": [[0]], "reach": "connected"},
        {"groups": [[1]], "reach": "downstream"},
    ]))
    n = scene_subnets()
    shut_edge = copy.deepcopy(n)
    shut_edge["name"] = "İki kaynak: kapalı kenar"
    shut_edge["edges"][1]["closed"] = True
    shut_edge["junctions"][3]["closed"] = False
    out.append((n, [
        {"kind": "connected", "edge": 0},
        {"kind": "downstream", "edge": 3},
        {"kind": "isolation", "edge": 6},
    ], [
        {"groups": [[0], [1]], "reach": "downstream"},
        {"groups": [[0], [1]], "reach": "connected"},
        {"groups": [[0, 1]], "reach": "connected"},
        {"groups": [[1], [0]], "reach": "downstream"},
        {"groups": [[2]], "reach": "connected"},
    ]))
    out.append((shut_edge, [
        {"kind": "connected", "edge": 0},
    ], [
        {"groups": [[0], [1]], "reach": "downstream"},
        {"groups": [[0], [1]], "reach": "connected"},
    ]))
    u = scene_unnamed()
    out.append((u, [
        {"kind": "connected", "edge": 0},
        {"kind": "connected", "edge": 4},
        {"kind": "downstream", "edge": 0},
    ], [
        {"groups": [[0]], "reach": "connected"},
        {"groups": [[1]], "reach": "downstream"},
    ]))
    out.append((scene_ring(), [
        {"kind": "connected", "edge": 0},
        {"kind": "connected", "edge": 2},
        {"kind": "isolation", "edge": 5},
    ], [
        {"groups": [[0]], "reach": "connected"},
    ]))
    out.append((scene_configs(), [
        {"kind": "connected", "edge": 0},
        {"kind": "connected", "edge": 1},
        {"kind": "connected", "edge": 2},
        {"kind": "connected", "edge": 3},
        {"kind": "connected", "edge": 4},
        {"kind": "connected", "edge": 5},
        {"kind": "connected", "edge": 6},
        {"kind": "connected", "edge": 8},
        {"kind": "connected", "edge": 10},
        {"kind": "connected", "node": [300, 0]},
    ], [
        {"groups": [[0]], "reach": "connected"},
        {"groups": [[0]], "reach": "downstream"},
        {"groups": [[1]], "reach": "connected"},
    ]))
    return out


def E(i, at=0.5):
    return {"edge": i, "at": at}


def N(x, y):
    return {"node": [x, y]}


# The routing questions of each scene (docs/adr/0562 §1), by the scene's name.
ROUTING = {
    "Trafo: yüksek ve alçak taraf": {
        "routes": [
            {"stops": [E(0), E(1)]},
            {"stops": [E(0), E(4)]},
            {"stops": [E(1), E(4)]},
            {"stops": [E(0), E(5)]},
            {"stops": [E(0), E(1), E(2)]},
            {"stops": [E(2, 0.25), E(3, 0.75)]},
        ],
        "matrices": [
            {"origins": [E(0), E(3)], "targets": [E(2), E(4), E(5)], "closest": [{"k": 2}, {"k": 1, "cutoff": 120}]},
        ],
        "areas": [
            {"facilities": [E(0)], "break": 120},
            {"facilities": [E(0)], "break": 120, "toward": True},
            {"facilities": [E(4), E(3)], "break": 90},
        ],
    },
    "Üç terminalli ayırıcı: a ile b bir yolda, c hiçbirinde": {
        "routes": [
            {"stops": [E(0), E(1)]},
            {"stops": [E(0), E(2)]},
            {"stops": [E(4), E(2)]},
            {"stops": [E(4), E(1)]},
            {"stops": [E(0), E(3)]},
            {"stops": [E(1), E(0)]},
        ],
        "matrices": [
            {"origins": [E(0), E(4)], "targets": [E(1), E(2), E(3)], "closest": [{"k": 2}, {"k": 1}]},
        ],
        "areas": [
            {"facilities": [E(0)], "break": 200},
            {"facilities": [E(4)], "break": 200},
            {"facilities": [E(3)], "break": 200, "toward": True},
            {"facilities": [E(1)], "break": 150, "toward": True},
        ],
    },
    "İki kaynak: ortak düğüm, kaynaksız boru ve kapalı vana": {
        "routes": [
            {"stops": [E(0), E(3)]},
            {"stops": [E(3), E(0)]},
            {"stops": [E(0), E(6)]},
        ],
        "matrices": [
            {"origins": [E(0)], "targets": [E(3), E(4), E(6)], "closest": [{"k": 3}]},
        ],
        "areas": [
            {"facilities": [E(0)], "break": 250},
            {"facilities": [E(3)], "break": 250, "toward": True},
        ],
    },
    "Terminal adı olmayan uç ve cihazda olmayan terminal": {
        "routes": [
            {"stops": [E(0), E(2)]},
            {"stops": [E(2), E(3)]},
            {"stops": [E(1), E(3)]},
            {"stops": [E(0), E(4)]},
        ],
        "matrices": [
            {"origins": [E(0), E(3)], "targets": [E(1), E(2), E(4)], "closest": [{"k": 2}]},
        ],
        "areas": [
            {"facilities": [E(0)], "break": 130},
        ],
    },
    "Yapılandırmalar: aynı katmanın cihazları noktalarındaki alana göre": {
        "routes": [
            {"stops": [E(0), E(1)]},
            {"stops": [E(0), E(2)]},
            {"stops": [E(1), E(3)]},
            {"stops": [E(3), E(4)]},
            {"stops": [E(3), E(5)]},
            {"stops": [E(4), E(5)]},
            {"stops": [E(3), E(6)]},
            {"stops": [E(0), E(7)]},
            {"stops": [E(0), E(10)]},
            {"stops": [E(7), E(9)]},
            {"stops": [E(7), E(8)]},
        ],
        "matrices": [
            {"origins": [E(0), E(3), E(4)], "targets": [E(1), E(2), E(5), E(9), E(10)],
             "closest": [{"k": 2}, {"k": 1, "cutoff": 400}]},
        ],
        "areas": [
            {"facilities": [E(0)], "break": 400},
            {"facilities": [E(0)], "break": 400, "toward": True},
            {"facilities": [E(5)], "break": 300},
            {"facilities": [E(4)], "break": 300, "toward": True},
            {"facilities": [E(7), E(3)], "break": 250},
        ],
    },
    "Halka: kısa yol cihazdan geçemez": {
        "routes": [
            {"stops": [E(0), E(3)]},
            {"stops": [E(5), E(1)]},
            {"stops": [E(0), E(2)]},
            {"stops": [N(100, 0), E(2)]},
            {"stops": [E(0), E(1)]},
            {"stops": [E(0), E(2), E(4)]},
            {"stops": [E(2), E(0)]},
            {"stops": [E(6), E(1)]},
        ],
        "matrices": [
            {"origins": [E(0), E(5)], "targets": [E(2), E(3), E(4)], "closest": [{"k": 2, "cutoff": 400}, {"k": 1}]},
            {"origins": [N(100, 0), E(2)], "targets": [E(0), E(1), E(2, 0.9)], "closest": [{"k": 1}]},
        ],
        "areas": [
            {"facilities": [E(0)], "break": 150},
            {"facilities": [E(0)], "break": 150, "toward": True},
            {"facilities": [E(0), E(4)], "break": 120},
            {"facilities": [E(2)], "break": 250},
            {"facilities": [N(100, 0)], "break": 100},
        ],
    },
}


def build():
    cases = []
    for scene, traces, subnets in scenes():
        sc = Scene(scene)
        ask = ROUTING.get(scene["name"], {})
        cases.append({
            "scene": scene,
            "unterminated": sc.unterminated,
            "traces": [trace_case(sc, q) for q in traces],
            "subnets": [subnet_case(sc, q) for q in subnets],
            "routes": [route_case(sc, q) for q in ask.get("routes", [])],
            "matrices": [matrix_case(sc, q) for q in ask.get("matrices", [])],
            "areas": [area_case(sc, q) for q in ask.get("areas", [])],
        })
    return {
        "format": "kentos.network-terminals",
        "version": 1,
        "note": "scripts/fixtures/network_terminals_cases.py yazar; ADR 0557'nin kurallarından, KentOS kodu olmadan. Kenarlar ve düğümler sahnedeki sırasıyla.",
        "cases": cases,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"Terminal ve alt ağ durumları güncel: {len(build()['cases'])} sahne.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
