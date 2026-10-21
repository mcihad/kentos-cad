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
shortest of those (docs/adr/0575). Written to fixtures/network/v1/blocked.json; the
geometry core (crates/shared/geometry-core/tests/all/network_access.rs) must give the same.

The ADR's reasons, in this order: the closed edges' expression; the profile, the day, the hour (only a cost without a
clock, at the question's moment); the direction field's closed value; one way the other way; no value of the cost.
Every best way is checked to be the only one.

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
]
BARRIER = ((1, 3), (2, 3), 0.4)  # the barrier's edge by its corners and the share of the way along it


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


def length(e):
    (ax, ay), (bx, by) = corner(*e[0]), corner(*e[1])
    return math.hypot(bx - ax, by - ay)


def shortest(a, b, usable):
    """The shortest way from corner a to b by length over the edges `usable(e, forward)` lets through: its steps
    (edge index, forward) and whether it is the only shortest; None when there is none."""
    adj = {}
    for k, e in enumerate(EDGES):
        if usable(e, True):
            adj.setdefault(e[0], []).append((e[1], k, True))
        if usable(e, False):
            adj.setdefault(e[1], []).append((e[0], k, False))
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


def fewest(a, b, cost_of):
    """The way from corner a to b that the fewest blockers shut, the shortest of those (docs/adr/0575): every edge
    both ways, `cost_of(k, forward)` the blockers stepping on edge k adds; its steps and whether it is the only best;
    None when the corners do not meet."""
    adj = {}
    for k, e in enumerate(EDGES):
        adj.setdefault(e[0], []).append((e[1], k, True))
        adj.setdefault(e[1], []).append((e[0], k, False))
    best, came, count = {a: (0, 0.0)}, {a: None}, {a: 1}
    heap, done = [(0, 0.0, a)], set()
    while heap:
        n_cut, d, n = heapq.heappop(heap)
        if n in done:
            continue
        done.add(n)
        for m, k, fw in adj.get(n, []):
            nd = (n_cut + cost_of(k, fw), d + length(EDGES[k]))
            if m not in best or nd[0] < best[m][0] or (nd[0] == best[m][0] and nd[1] < best[m][1] - 1e-9):
                best[m], came[m], count[m] = nd, (n, k, fw), count[n]
                heapq.heappush(heap, (nd[0], nd[1], m))
            elif nd[0] == best[m][0] and abs(nd[1] - best[m][1]) <= 1e-9:
                count[m] += count[n]
    if b not in best:
        return None, True
    steps, n = [], b
    while came[n] is not None:
        p, k, fw = came[n]
        steps.append((k, fw))
        n = p
    return list(reversed(steps)), count[b] == 1


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
]


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

        found, _ = shortest(a, b, open_)
        case = {"name": name, "from": list(corner(*a)), "to": list(corner(*b)), "cost": cost,
                "profile": "Kamyon" if profile else None, "day": day_text, "time": time,
                "barriers": [barrier_at] if with_barrier else []}
        if found is not None:
            case["reachable"] = True
            cases.append(case)
            continue
        def cut(k, fw):
            n = 1 if blocker(EDGES[k], fw, profile, day, time, cost) else 0
            return n + (1 if with_barrier and k == barrier_edge else 0)

        steps, only = fewest(a, b, cut)
        assert only, f"{name}: more than one best way"
        if steps is None:
            case["blocked"] = {"kind": "apart"}
        else:
            edges_out, barriers_out = [], []
            for k, fw in steps:
                why = blocker(EDGES[k], fw, profile, day, time, cost)
                if why and all(x["id"] != k + 1 for x in edges_out):
                    edges_out.append({"id": k + 1, "why": why})
                if with_barrier and k == barrier_edge and 0 not in barriers_out:
                    barriers_out.append(0)
            case["blocked"] = ({"kind": "by", "edges": edges_out, "barriers": barriers_out}
                               if edges_out or barriers_out else {"kind": "other"})
        cases.append(case)
    return {"format": "kentos.network-blocked", "version": 1, "definition": DEFINITION, "edges": edges, "cases": cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT} güncel değil; betiği --check'siz çalıştırın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel: {len(QUESTIONS)} soru.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(QUESTIONS)} soru.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
