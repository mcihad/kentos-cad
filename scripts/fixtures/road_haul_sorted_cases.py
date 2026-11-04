#!/usr/bin/env python3
"""A haul plan that sorts its material, shared cases (docs/adr/0944, plan/road.md Y38.1; ROAD-HAUL-04, 05), from an
independent reference: a linear programme solved by HiGHS (scipy), not KentOS's successive shortest paths. Variables:
each segment's flow both ways on the suitable and the unsuitable line (none across a closed barrier, the detour added
to the length across an open one), each stretch's suitable and unsuitable cut placed and fill met, each borrow site's
gift and each waste site's take from either line (both within its capacity). Phase one places the most (cut placed
plus fill met, the unsuitable cut counting twice: where the waste sites are short it goes first, having nowhere else
to go); phase two, keeping that, moves it at the least moment. No KentOS code.

Before writing, the cases built for it are checked against closed forms (a 30 % unsuitable cut: 70 m³ fill the fill
100 m away, 30 come from the borrow site 150 m away and 30 go to the waste site 50 m back: 13 000 m³·m), and every case
with no unsuitable share and no barrier against the unsorted plan's own reference
(scripts/fixtures/road_haul_plan_cases.py's linear programme on the net mass).

    uv run --no-project --python 3.11 --with numpy --with scipy python3 scripts/fixtures/road_haul_sorted_cases.py
    ... --check   # fails when fixtures/road/v1/haul_sorted.json is not what this writes
"""

import json
import random
import sys
from pathlib import Path

import numpy as np
from scipy.optimize import linprog

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "haul_sorted.json"


def solve(stretches, sites, barriers, detail=False):
    kept = [r for r in stretches if r["to"] > r["from"] and (r["cut"] > 0 or r["fill"] > 0)]
    mids = [(r["from"] + r["to"]) / 2 for r in kept]
    st = sorted(set(mids + [s["station"] for s in sites]))
    n = len(st)
    idx = {v: i for i, v in enumerate(st)}
    # Segments: length with detours, or closed.
    segs = []
    for i in range(n - 1):
        on = [b for b in barriers if st[i] <= b["station"] < st[i + 1]]
        if any(b.get("detour") is None for b in on):
            continue
        segs.append((i, st[i + 1] - st[i] + sum(b["detour"] for b in on)))
    var = []  # (kind, data)

    def new(kind, data=None):
        var.append((kind, data))
        return len(var) - 1
    flows = []
    for (i, length) in segs:
        for layer in (0, 1):
            flows.append((new("f", (layer, i, i + 1)), length, layer))
            flows.append((new("f", (layer, i + 1, i)), length, layer))
    good, bad, met = [], [], []
    for r, m in zip(kept, mids):
        u = min(max(r["unsuitable"], 0.0), 1.0)
        good.append((new("good", idx[m]), r["cut"] * (1 - u)))
        bad.append((new("bad", idx[m]), r["cut"] * u))
        met.append((new("met", idx[m]), r["fill"]))
    borrow, waste = [], []
    for j, s in enumerate(sites):
        if s["borrow"]:
            borrow.append((j, new("borrow", idx[s["station"]]), s["capacity"]))
        else:
            waste.append((j, new("wg", idx[s["station"]]), new("wb", idx[s["station"]]), s["capacity"]))
    nv = len(var)
    a_eq, b_eq = [], []
    for layer in (0, 1):
        for k in range(n):
            row = np.zeros(nv)
            for (v, _, ly) in flows:
                _, (l2, a, b) = var[v]
                if ly == layer and b == k:
                    row[v] += 1
                if ly == layer and a == k:
                    row[v] -= 1
            for (v, _) in (good if layer == 0 else bad):
                if var[v][1] == k:
                    row[v] += 1
            if layer == 0:
                for (v, _) in met:
                    if var[v][1] == k:
                        row[v] -= 1
                for (_, v, _) in borrow:
                    if var[v][1] == k:
                        row[v] += 1
                for (_, vg, _, _) in waste:
                    if var[vg][1] == k:
                        row[vg] -= 1
            else:
                for (_, _, vb, _) in waste:
                    if var[vb][1] == k:
                        row[vb] -= 1
            a_eq.append(row)
            b_eq.append(0.0)
    bounds = [(0, None)] * nv
    for (v, cap) in good + bad + met:
        bounds[v] = (0, cap)
    for (_, v, cap) in borrow:
        bounds[v] = (0, cap)
    a_ub, b_ub = [], []
    for (_, vg, vb, cap) in waste:
        row = np.zeros(nv)
        row[vg] = row[vb] = 1
        a_ub.append(row)
        b_ub.append(cap)
    # The unsuitable cut placed first where the waste sites are short (it has nowhere else to go): it counts twice.
    placed = np.zeros(nv)
    for (v, _) in good + met:
        placed[v] = 1
    for (v, _) in bad:
        placed[v] = 2
    kw = dict(A_eq=np.array(a_eq) if a_eq else None, b_eq=b_eq or None, A_ub=np.array(a_ub) if a_ub else None,
              b_ub=b_ub or None, bounds=bounds, method="highs")
    one = linprog(-placed, **kw)
    assert one.status == 0, one.message
    best = -one.fun
    moment = np.zeros(nv)
    for (v, length, _) in flows:
        moment[v] = length
    a2 = (np.vstack([kw["A_ub"], -placed]) if kw["A_ub"] is not None else np.array([-placed]))
    b2 = (b_ub or []) + [-best + 1e-9 * max(best, 1.0)]
    two = linprog(moment, A_eq=kw["A_eq"], b_eq=kw["b_eq"], A_ub=a2, b_ub=b2, bounds=bounds, method="highs")
    assert two.status == 0, two.message
    x = two.x
    if detail:
        return plan_detail(x, var, flows, good, bad, met, borrow, waste, st, segs, sites, barriers, kept, two, moment, kw,
                           a2, b2, bounds)
    shortage = sum(cap - x[v] for v, cap in met)
    surplus = sum(cap - x[v] for v, cap in good)
    stranded = sum(cap - x[v] for v, cap in bad)
    cut = sum(r["cut"] for r in kept)
    tiny = 1e-6 * max(cut + sum(r["fill"] for r in kept) + sum(s["capacity"] for s in sites), 1.0)
    return {"feasible": bool(shortage <= tiny and surplus <= tiny and stranded <= tiny),
            "cut": cut, "fill": sum(r["fill"] for r in kept),
            "unsuitable": sum(r["cut"] * min(max(r["unsuitable"], 0.0), 1.0) for r in kept),
            "moment": float(two.fun), "shortage": max(shortage, 0.0), "surplus": max(surplus, 0.0),
            "stranded": max(stranded, 0.0)}


def plan_detail(x, var, flows, good, bad, met, borrow, waste, st, segs, sites, barriers, kept, two, moment, kw, a2, b2,
                bounds):
    """The plan in full, as the command reports it: every placement checked unique (its least and its most among plans
    of the least moment), the segments' flows following from them on the two lines."""
    nv = len(var)
    a3 = np.vstack([a2, moment])
    b3 = list(b2) + [two.fun + 1e-11 * max(abs(two.fun), 1.0)]
    placed = [v for v, _ in good + bad + met] + [v for _, v, _ in borrow] + [v for _, vg, vb, _ in waste for v in (vg, vb)]
    for v in placed:
        c = np.zeros(nv)
        c[v] = 1
        lo = linprog(c, A_eq=kw["A_eq"], b_eq=kw["b_eq"], A_ub=a3, b_ub=b3, bounds=bounds, method="highs")
        hi = linprog(-c, A_eq=kw["A_eq"], b_eq=kw["b_eq"], A_ub=a3, b_ub=b3, bounds=bounds, method="highs")
        assert lo.status == 0 and hi.status == 0
        # HiGHS's own tolerance on volumes of thousands of m³ is ~1e-4 m³: unique within 1e-5 of the value.
        assert -hi.fun - lo.fun < 1e-4 + 1e-5 * abs(lo.fun), ("a placement is not unique", var[v], lo.fun, -hi.fun)
    n = len(st)
    # Polish the solver's answer (its tolerance leaves ~1e-5 m³ where a bound belongs): a placement near its bound is
    # the bound; on each stretch of road between closed barriers the one placement left off its bounds follows exactly
    # from that stretch's balance (what is placed there is what is taken there).
    x = list(x)
    scale = max(1.0, sum(abs(b[1]) if b[1] is not None else 0.0 for b in bounds if b[1] is not None))
    for v in placed:
        lo_b, hi_b = bounds[v]
        if abs(x[v] - lo_b) < 1e-6 * scale:
            x[v] = float(lo_b)
        elif hi_b is not None and abs(x[v] - hi_b) < 1e-6 * scale:
            x[v] = float(hi_b)
    open_ = {i: length for i, length in segs}
    comp = [0] * n
    for i in range(1, n):
        comp[i] = comp[i - 1] + (0 if (i - 1) in open_ else 1)
    sign = {}
    for (v, _) in good + bad:
        sign[v] = 1.0
    for (_, v, _) in borrow:
        sign[v] = 1.0
    for (v, _) in met:
        sign[v] = -1.0
    for (_, vg, vb, _) in waste:
        sign[vg] = sign[vb] = -1.0
    for c in set(comp):
        mine = [v for v in placed if comp[var[v][1]] == c]
        free = [v for v in mine if not (x[v] == bounds[v][0] or (bounds[v][1] is not None and x[v] == bounds[v][1]))]
        if len(free) == 1:
            v = free[0]
            rest = sum(sign[u] * x[u] for u in mine if u != v)
            x[v] = -rest / sign[v]
    # Each node's balance on each line, from the placements; the segment's flow is what lies to its left.
    bal = [[0.0] * n, [0.0] * n]
    for (v, _) in good:
        bal[0][var[v][1]] += x[v]
    for (v, _) in met:
        bal[0][var[v][1]] -= x[v]
    for (v, _) in bad:
        bal[1][var[v][1]] += x[v]
    for (_, v, _) in borrow:
        bal[0][var[v][1]] += x[v]
    for (_, vg, vb, _) in waste:
        bal[0][var[vg][1]] -= x[vg]
        bal[1][var[vb][1]] -= x[vb]
    segments, moment_total, bad_moment = [], 0.0, 0.0
    crossed = [0.0] * len(barriers)
    run = [0.0, 0.0]
    for i in range(n - 1):
        run = [run[0] + bal[0][i], run[1] + bal[1][i]]
        g, b = (run[0], run[1]) if i in open_ else (0.0, 0.0)
        if i in open_:
            moment_total += (abs(g) + abs(b)) * open_[i]
            bad_moment += abs(b) * open_[i]
            for m, bar in enumerate(barriers):
                if st[i] <= bar["station"] < st[i + 1]:
                    crossed[m] += abs(g) + abs(b)
        segments.append((st[i], st[i + 1], g, b))
    used = [0.0] * len(sites)
    used_bad = [None] * len(sites)
    for (j, v, _) in borrow:
        used[j] = x[v]
    for (j, vg, vb, _) in waste:
        used[j] = x[vg] + x[vb]
        used_bad[j] = x[vb]
    shortage = sum(cap - x[v] for v, cap in met)
    surplus = sum(cap - x[v] for v, cap in good)
    stranded = sum(cap - x[v] for v, cap in bad)
    return {"x": x, "segments": segments, "moment": moment_total, "unsuitableMoment": bad_moment, "crossed": crossed,
            "used": used, "usedUnsuitable": used_bad, "shortage": max(shortage, 0.0), "surplus": max(surplus, 0.0),
            "stranded": max(stranded, 0.0)}


def unsorted(stretches, sites):
    """The plain plan's own reference on the net mass (no unsuitable share, no barrier): every stretch's net at its
    middle, one line."""
    flat = [dict(r, cut=max(r["cut"] - r["fill"], 0.0), fill=max(r["fill"] - r["cut"], 0.0), unsuitable=0.0)
            for r in stretches]
    return solve(flat, sites, [])


def rs(a, b, cut, fill, u=0.0):
    return {"from": float(a), "to": float(b), "cut": float(cut), "fill": float(fill), "unsuitable": float(u)}


def site(station, capacity, borrow):
    return {"station": float(station), "capacity": float(capacity), "borrow": borrow}


def case(name, stretches, sites, barriers=(), closed=None):
    got = solve(stretches, sites, list(barriers))
    if closed is not None:
        for k, v in closed.items():
            assert abs(got[k] - v) < 1e-6 * max(abs(v), 1.0), (name, k, got[k], v)
    if not barriers and all(r["unsuitable"] == 0 for r in stretches):
        ref = unsorted(stretches, sites)
        assert abs(ref["moment"] - got["moment"]) < 1e-6 * max(ref["moment"], 1.0), (name, ref, got)
    return {"name": name, "stretches": stretches, "sites": sites, "barriers": list(barriers), "expected": got}


def random_road(seed, n=24, u_share=0.0):
    rng = random.Random(seed)
    out = []
    for i in range(n):
        a = i * 40.0
        cut = round(rng.random() * 900 * (rng.random() < 0.55), 1)
        fill = round(rng.random() * 900 * (rng.random() < 0.55), 1)
        u = round(rng.random() * u_share, 3) if cut > 0 else 0.0
        out.append(rs(a, a + 40, cut, fill, u))
    return out


def cases():
    out = []
    out.append(case("Bir yarma bir dolgu", [rs(0, 100, 100, 0), rs(100, 200, 0, 100)], [], closed={"moment": 10000}))
    out.append(case("Yarmanın %30'u uygunsuz", [rs(0, 100, 100, 0, 0.3), rs(100, 200, 0, 100)],
                    [site(300, 100, True), site(0, 100, False)],
                    closed={"moment": 70 * 100 + 30 * 150 + 30 * 50, "feasible": True}))
    out.append(case("Uygunsuz için döküm yok", [rs(0, 100, 100, 0, 0.3), rs(100, 200, 0, 100)], [site(300, 100, True)],
                    closed={"stranded": 30, "feasible": False}))
    out.append(case("Aynı aralıkta yarma ve dolgu, yarısı uygunsuz", [rs(0, 100, 100, 80, 0.5)],
                    [site(150, 100, True), site(-20, 100, False)],
                    closed={"moment": 30 * 100 + 50 * 70, "feasible": True}))
    out.append(case("Döküm kapasitesini iki tür paylaşır", [rs(0, 100, 120, 0, 0.5), rs(100, 200, 0, 0)],
                    [site(300, 100, False)], closed={"surplus": 20, "stranded": 0, "feasible": False}))
    out.append(case("Kapalı engel: dere geçilmiyor", [rs(0, 100, 100, 0), rs(100, 200, 0, 100)],
                    [site(20, 100, False), site(180, 100, True)], [{"station": 100.0, "detour": None}],
                    closed={"moment": 100 * 30 + 100 * 30, "feasible": True}))
    out.append(case("Engelde 700 m dolaşma: döküm ve ödünç daha ucuz", [rs(0, 100, 100, 0), rs(100, 200, 0, 100)],
                    [site(-300, 100, False), site(500, 100, True)], [{"station": 100.0, "detour": 700.0}],
                    closed={"moment": 100 * 350 + 100 * 350}))
    out.append(case("Engelde 20 m dolaşma: dengelemek daha ucuz", [rs(0, 100, 100, 0), rs(100, 200, 0, 100)],
                    [site(-300, 100, False), site(500, 100, True)], [{"station": 100.0, "detour": 20.0}],
                    closed={"moment": 100 * 120}))
    out.append(case("Yakın döküm ve ödünç dengelemekten ucuz", [rs(0, 100, 100, 0), rs(100, 200, 0, 100)],
                    [site(20, 100, False), site(180, 100, True)], closed={"moment": 100 * 30 + 100 * 30}))
    out.append(case("Kapalı engel, başka yol yok", [rs(0, 100, 100, 0), rs(100, 200, 0, 100)], [],
                    [{"station": 100.0, "detour": None}], closed={"shortage": 100, "surplus": 100, "feasible": False}))
    out.append(case("Uygunsuzsuz, engelsiz rastgele yol", random_road(1), [site(-50, 4000, False), site(1000, 4000, True)]))
    out.append(case("Rastgele yol, %40'a kadar uygunsuz", random_road(2, u_share=0.4),
                    [site(-50, 6000, False), site(500, 3000, False), site(1000, 6000, True)]))
    out.append(case("Rastgele yol, iki engel", random_road(3, u_share=0.2),
                    [site(-50, 8000, False), site(1000, 8000, True), site(450, 3000, True)],
                    [{"station": 300.0, "detour": 250.0}, {"station": 700.0, "detour": None}]))
    out.append(case("Rastgele yol, dökümü yetmeyen", random_road(4, u_share=0.6), [site(1000, 1500, False), site(0, 9000, True)]))
    out.append(case("Bütün yarma uygunsuz", random_road(5, u_share=1.0) and [dict(r, unsuitable=1.0 if r["cut"] else 0.0) for r in random_road(5)],
                    [site(-50, 20000, False), site(1000, 20000, True)]))
    out.append(case("Engel bir yerin tam istasyonunda", [rs(0, 100, 100, 0), rs(100, 200, 0, 100), rs(200, 300, 50, 0)],
                    [site(350, 100, False)], [{"station": 150.0, "detour": 40.0}]))
    return out


def main():
    data = {"cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what scripts/fixtures/road_haul_sorted_cases.py writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        e = c["expected"]
        print(f"  {c['name'][:44]:44} {'olur' if e['feasible'] else 'olmaz'} moment {e['moment']:.1f} "
              f"eksik {e['shortage']:.1f} artan {e['surplus']:.1f} kalan uygunsuz {e['stranded']:.1f}")


if __name__ == "__main__":
    main()
