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
                self.device_at.setdefault(tuple(j["p"]), j["device"])
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

    def terminal(self, at, value):
        """The index of the terminal an end names at its node's device, None when it names none or is not at a device."""
        d = self.device_at.get(at)
        if d is None or value is None:
            return None
        key = folded(value)
        if not key:
            return None
        for i, t in enumerate(self.devices[d]["terminals"]):
            if folded(t["id"]) == key or folded(t["name"]) == key:
                return i
        return None

    def linked(self, node, a, b):
        d = self.device_at.get(node)
        if d is None or a is None or b is None or a == b:
            return True
        ids = [t["id"] for t in self.devices[d]["terminals"]]
        return any(ids[a] in path and ids[b] in path for path in self.devices[d]["paths"])

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


def junction(id_, p, role="junction", device=None, closed=False):
    j = {"id": id_, "p": list(p), "role": role}
    if device is not None:
        j["device"] = device
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
    return out


def build():
    cases = []
    for scene, traces, subnets in scenes():
        sc = Scene(scene)
        cases.append({
            "scene": scene,
            "unterminated": sc.unterminated,
            "traces": [trace_case(sc, q) for q in traces],
            "subnets": [subnet_case(sc, q) for q in subnets],
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
