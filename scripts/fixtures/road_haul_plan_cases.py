#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.hauls.plan.json: the shared cases of KentOS Yol's haul plan (plan/road.md Y23,
docs/adr/0924; ROAD-HAUL-02, ROAD-HAUL-07), their expectations built here without KentOS code. The drawing, its
corridors, volumes and mass haul diagrams are road_haul_command_cases.py's (the volume sheet's mass ordinate). The plan
is a linear programme solved by HiGHS (scipy's linprog): each stretch's net at its middle, a site at its station, the
earth flowing along the line at a cost of its distance; every cut placed, every fill met, a site taking up to its
capacity; the least haul moment. A site's use is then checked unique (its least and its most among plans of that
moment), snapped to 0 or its capacity where it lies there, the one left balancing the totals; the segments' flows
follow exactly (in fractions) from what each place gives or takes.

The economic plan (docs/adr/0925) is a transportation programme: each cut to each fill, each cut to each waste site and
each borrow site to each fill an arc, its cost the overhaul's price on the distance beyond the free haul and the site's
price; HiGHS makes the cost least, then the moment among plans of that cost. The sites' uses are checked unique; the
arcs HiGHS uses are then solved again exactly in fractions (the cut and fill rows, the sites at a bound) and the cost,
the moment and the moment beyond the free haul read from them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with scipy \\
        python3 scripts/fixtures/road_haul_plan_cases.py [--check]
"""

import json
import sys
from fractions import Fraction
from pathlib import Path

import numpy as np
from scipy.optimize import linprog

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_haul_command_cases as hc  # noqa: E402
import road_haul_sorted_cases as sref  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-3
completed, failed, case = hc.completed, hc.failed, hc.case
UNCHANGED = hc.UNCHANGED
H1, H2 = hc.H1, hc.H2
V3 = {"id": "v-3", "name": "Dolgu kübajı", "corridor": "c-1", "revision": 1, "method": "endArea",
      "factors": [{"from": 0.0, "to": 600.0, "cut": 0.1, "fill": 5.0}]}
H3 = {"id": "h-3", "name": "Dolgu Brükneri", "volume": "v-3", "revision": 1}
VOLUMES = hc.VOLUMES + [V3]


def solve(points, sites):
    places = []
    for a, b in zip(points, points[1:]):
        net = b[1] - a[1]
        if net != 0.0 and b[0] > a[0]:
            places.append((0.5 * (a[0] + b[0]), net))
    cut = sum(p[1] for p in places if p[1] > 0.0)
    fill = sum(-p[1] for p in places if p[1] < 0.0)
    gives = sum(s["volume"] for s in sites if s["kind"] == "borrow")
    takes = sum(s["volume"] for s in sites if s["kind"] == "waste")
    tiny = 1e-9 * max(cut + fill + gives + takes, 1.0)
    shortage = max(fill - cut - gives, 0.0)
    surplus = max(cut - fill - takes, 0.0)
    base = {"cut": cut, "fill": fill, "shortage": 0.0, "surplus": 0.0}
    if shortage > tiny or surplus > tiny:
        return {**base, "feasible": False, "used": [0.0] * len(sites), "segments": [], "moment": 0.0, "moved": 0.0,
                "shortage": shortage, "surplus": surplus, "access": 0.0}
    stations = sorted({p[0] for p in places} | {s["station"] for s in sites})
    n, m = len(stations), len(sites)
    at = {s: k for k, s in enumerate(stations)}
    lengths = [stations[i + 1] - stations[i] for i in range(n - 1)]
    nv = 2 * (n - 1) + m
    # Docs/adr/0955: a cubic metre a site gives or takes goes its access road's length more.
    cost = np.array(lengths + lengths + [s.get("access", 0.0) for s in sites])
    rows = np.zeros((n, nv))
    rhs = np.zeros(n)
    for station, v in places:
        rhs[at[station]] -= v
    for i in range(n - 1):
        rows[i, i] -= 1.0
        rows[i + 1, i] += 1.0
        rows[i, n - 1 + i] += 1.0
        rows[i + 1, n - 1 + i] -= 1.0
    for j, s in enumerate(sites):
        rows[at[s["station"]], 2 * (n - 1) + j] = 1.0 if s["kind"] == "borrow" else -1.0
    bounds = [(0, None)] * (2 * (n - 1)) + [(0, s["volume"]) for s in sites]
    best = linprog(cost, A_eq=rows, b_eq=rhs, bounds=bounds, method="highs")
    assert best.status == 0, best.message
    used = []
    for j in range(m):
        ends = []
        for sign in (1.0, -1.0):
            c = np.zeros(nv)
            c[2 * (n - 1) + j] = sign
            r = linprog(c, A_ub=[cost], b_ub=[best.fun * (1 + 1e-9) + 1e-6], A_eq=rows, b_eq=rhs, bounds=bounds,
                        method="highs")
            assert r.status == 0, r.message
            ends.append(sign * r.fun)
        # HiGHS leaves ~1e-9 of a moment of 10⁷ m³·m: a use may move ~1e-6 of itself within it, not more.
        assert ends[1] - ends[0] < 1e-3 + 1e-6 * abs(ends[0]), f"site {j}'s use is not unique: {ends}"
        used.append(ends[0])
    scale = max(cut + fill + gives + takes, 1.0)
    exact = []
    for u, s in zip(used, sites):
        if abs(u) < 1e-9 * scale:
            exact.append(Fraction(0))
        elif abs(u - s["volume"]) < 1e-9 * scale:
            exact.append(Fraction(s["volume"]))
        else:
            exact.append(None)
    # A segment without flow parts the line; each part balances on its own, its one partly used site (or, where all
    # snapped to a bound, its largest use: a tight capacity's rounding) what the part leaves.
    flows = best.x[: n - 1] - best.x[n - 1: 2 * (n - 1)]
    give = {k: Fraction(0) for k in range(n)}
    for station, v in places:
        give[at[station]] += Fraction(v)
    parts, begin = [], 0
    for i in range(n - 1):
        if abs(flows[i]) < 1e-9 * scale:
            parts.append(range(begin, i + 1))
            begin = i + 1
    parts.append(range(begin, n))
    for part in parts:
        mine = [j for j, s in enumerate(sites) if at[s["station"]] in part]
        free = [j for j in mine if exact[j] is None]
        assert len(free) <= 1, f"more than one site partly used in a part: {free}"
        if not free:
            if not mine:
                continue
            free = [max(mine, key=lambda k: (exact[k], -k))]
            exact[free[0]] = None
        j = free[0]
        rest = sum(give[k] for k in part) + sum((exact[k] if sites[k]["kind"] == "borrow" else -exact[k])
                                                for k in mine if k != j)
        exact[j] = -rest if sites[j]["kind"] == "borrow" else rest
        assert -Fraction(1, 1000) <= exact[j] <= Fraction(sites[j]["volume"]) + Fraction(1, 1000), (j, float(exact[j]))
    for j, s in enumerate(sites):
        give[at[s["station"]]] += exact[j] if s["kind"] == "borrow" else -exact[j]
    assert abs(sum(give.values())) < 1e-6
    flow = Fraction(0)
    segments, moment = [], Fraction(0)
    for i in range(n - 1):
        flow += give[i]
        segments.append({"from": stations[i], "to": stations[i + 1], "flow": float(flow)})
        moment += abs(flow) * Fraction(lengths[i])
    borrowed = sum(float(e) for e, s in zip(exact, sites) if s["kind"] == "borrow")
    access = sum(e * Fraction(s.get("access", 0.0)) for e, s in zip(exact, sites))
    return {**base, "feasible": True, "used": [float(e) for e in exact], "segments": segments,
            "moment": float(moment + access), "moved": cut + borrowed, "access": float(access)}


def gauss(rows, rhs, n):
    """The one solution of rows · x = rhs in fractions (rows a list of {column: value}); fails where there is none or
    more than one."""
    a = [[Fraction(r.get(k, 0)) for k in range(n)] + [Fraction(b)] for r, b in zip(rows, rhs)]
    pivots, r = [], 0
    for c in range(n):
        p = next((i for i in range(r, len(a)) if a[i][c] != 0), None)
        if p is None:
            continue
        a[r], a[p] = a[p], a[r]
        piv = a[r][c]
        a[r] = [v / piv for v in a[r]]
        for i in range(len(a)):
            if i != r and a[i][c] != 0:
                f = a[i][c]
                a[i] = [v - f * w for v, w in zip(a[i], a[r])]
        pivots.append(c)
        r += 1
    assert len(pivots) == n, f"the arcs used are not fixed by the rows: {len(pivots)} of {n}"
    for i in range(r, len(a)):
        assert abs(a[i][n]) < Fraction(1, 10**6), f"the rows disagree by {float(a[i][n])}"
    return [a[i][n] for i in range(n)]


def solve_priced(points, sites, prices):
    places = []
    for a, b in zip(points, points[1:]):
        net = b[1] - a[1]
        if net != 0.0 and b[0] > a[0]:
            places.append((0.5 * (a[0] + b[0]), net))
    cut = sum(q[1] for q in places if q[1] > 0.0)
    fill = sum(-q[1] for q in places if q[1] < 0.0)
    gives = sum(x["volume"] for x in sites if x["kind"] == "borrow")
    takes = sum(x["volume"] for x in sites if x["kind"] == "waste")
    tiny = 1e-9 * max(cut + fill + gives + takes, 1.0)
    shortage = max(fill - cut - gives, 0.0)
    surplus = max(cut - fill - takes, 0.0)
    base = {"cut": cut, "fill": fill, "shortage": 0.0, "surplus": 0.0}
    if shortage > tiny or surplus > tiny:
        return {**base, "feasible": False, "used": [0.0] * len(sites), "segments": [], "moment": 0.0, "moved": 0.0,
                "shortage": shortage, "surplus": surplus}
    F, o = prices["free"], prices["overhaul"]
    src = [(q[0], q[1], None) for q in places if q[1] > 0] + [(x["station"], x["volume"], j) for j, x in enumerate(sites) if x["kind"] == "borrow"]
    snk = [(q[0], -q[1], None) for q in places if q[1] < 0] + [(x["station"], x["volume"], j) for j, x in enumerate(sites) if x["kind"] == "waste"]
    arcs = [(i, j) for i, a in enumerate(src) for j, b in enumerate(snk) if not (a[2] is not None and b[2] is not None)]
    # Along the road and the access roads at either end (docs/adr/0955).
    reach = lambda j: 0.0 if j is None else sites[j].get("access", 0.0)  # noqa: E731
    dist = [abs(snk[j][0] - src[i][0]) + reach(src[i][2]) + reach(snk[j][2]) for i, j in arcs]
    cost = np.array([o * max(0.0, d - F) + (prices["borrow"] if src[i][2] is not None else 0.0)
                     + (prices["waste"] if snk[j][2] is not None else 0.0) for (i, j), d in zip(arcs, dist)])
    na = len(arcs)
    eq, eqb, ub, ubb = [], [], [], []
    for i, a in enumerate(src):
        row = np.zeros(na)
        for k, (ii, _) in enumerate(arcs):
            if ii == i:
                row[k] = 1.0
        (ub if a[2] is not None else eq).append(row)
        (ubb if a[2] is not None else eqb).append(a[1])
    for j, b in enumerate(snk):
        row = np.zeros(na)
        for k, (_, jj) in enumerate(arcs):
            if jj == j:
                row[k] = 1.0
        (ub if b[2] is not None else eq).append(row)
        (ubb if b[2] is not None else eqb).append(b[1])
    kw = {"A_eq": np.array(eq), "b_eq": np.array(eqb), "bounds": [(0, None)] * na, "method": "highs"}
    if ub:
        kw["A_ub"], kw["b_ub"] = np.array(ub), np.array(ubb)
    first = linprog(cost, **kw)
    assert first.status == 0, first.message
    # The plans of the least cost: the arcs whose reduced cost is 0 (HiGHS's marginals of the lower bounds); among
    # them the least moment, a vertex of that face.
    reduced = first.lower.marginals
    keep = [(0, None) if reduced[k] <= 1e-9 * max(1.0, abs(cost[k])) else (0, 0) for k in range(na)]
    kw2 = {**kw, "bounds": keep}
    d = np.array(dist)
    second = linprog(d, **kw2)
    assert second.status == 0, second.message
    assert abs(float(cost @ second.x) - first.fun) < 1e-6 * max(1.0, abs(first.fun))
    kw3 = dict(kw2)
    kw3["A_ub"] = np.vstack([kw2["A_ub"], d]) if ub else np.array([d])
    kw3["b_ub"] = np.concatenate([kw2["b_ub"], [second.fun * (1 + 1e-9) + 1e-6]]) if ub else np.array([second.fun * (1 + 1e-9) + 1e-6])
    side = {}
    for j, x in enumerate(sites):
        c = np.zeros(na)
        for k, (i, jj) in enumerate(arcs):
            if (x["kind"] == "borrow" and src[i][2] == j) or (x["kind"] == "waste" and snk[jj][2] == j):
                c[k] = 1.0
        lo = linprog(c, **kw3)
        hi = linprog(-c, **kw3)
        assert lo.status == 0 and hi.status == 0
        assert -hi.fun - lo.fun < 1e-3, f"site {j}'s use is not unique: {lo.fun} – {-hi.fun}"
        side[j] = (c, lo.fun)
    scale = max(cut + fill + gives + takes, 1.0)
    support = [k for k in range(na) if second.x[k] > 1e-9 * scale]
    col = {k: n for n, k in enumerate(support)}
    rows, rhs = [], []
    for i, a in enumerate(src):
        if a[2] is None:
            rows.append({col[k]: 1 for k in support if arcs[k][0] == i})
            rhs.append(Fraction(a[1]))
    for j, b in enumerate(snk):
        if b[2] is None:
            rows.append({col[k]: 1 for k in support if arcs[k][1] == j})
            rhs.append(Fraction(b[1]))
    for j, x in enumerate(sites):
        c, u = side[j]
        mine = {col[k]: 1 for k in support if c[k] == 1.0}
        if abs(u) < 1e-9 * scale:
            assert not mine
        elif abs(u - x["volume"]) < 1e-9 * scale:
            rows.append(mine)
            rhs.append(Fraction(x["volume"]))
    flows = gauss(rows, rhs, len(support))
    assert all(f >= -Fraction(1, 10**6) for f in flows)
    used = [Fraction(0)] * len(sites)
    moment = over = Fraction(0)
    total = Fraction(0)
    for n, k in enumerate(support):
        i, j = arcs[k]
        f = flows[n]
        dd = Fraction(dist[k])
        moment += f * dd
        beyond = max(Fraction(0), dd - Fraction(F))
        over += f * beyond
        if src[i][2] is not None:
            used[src[i][2]] += f
        if snk[j][2] is not None:
            used[snk[j][2]] += f
    stations = sorted({q[0] for q in places} | {x["station"] for x in sites})
    at = {x: k for k, x in enumerate(stations)}
    give = [Fraction(0)] * len(stations)
    for x, v in places:
        give[at[x]] += Fraction(v)
    for x, u in zip(sites, used):
        give[at[x["station"]]] += u if x["kind"] == "borrow" else -u
    flow, segments = Fraction(0), []
    for k in range(len(stations) - 1):
        flow += give[k]
        segments.append({"from": stations[k], "to": stations[k + 1], "flow": float(flow)})
    wasted = sum((u for x, u in zip(sites, used) if x["kind"] == "waste"), Fraction(0))
    borrowed = sum((u for x, u in zip(sites, used) if x["kind"] == "borrow"), Fraction(0))
    overhaul = Fraction(o) * over
    waste = Fraction(prices["waste"]) * wasted
    borrow = Fraction(prices["borrow"]) * borrowed
    total = overhaul + waste + borrow
    assert abs(float(total) - first.fun) < 1e-6 * max(1.0, abs(first.fun)), (float(total), first.fun)
    access = sum(u * Fraction(x.get("access", 0.0)) for x, u in zip(sites, used))
    return {**base, "feasible": True, "used": [float(u) for u in used], "segments": segments, "moment": float(moment),
            "moved": cut + float(borrowed), "over": float(over), "overhaul": float(overhaul), "waste": float(waste),
            "borrow": float(borrow), "total": float(total), "access": float(access)}


# Docs/adr/0954: within this share (of the larger, at least 1) two of a plan's figures are the same.
SAME = 1e-6


def differ(a, b):
    return abs(a - b) > SAME * max(abs(a), abs(b), 1.0)


def find(name, hauls):
    h = next((x for x in hauls if x["id"] == name), None) or next(
        (x for x in hauls if x["name"].lower() == name.lower()), None)
    if h is None:
        raise hc.Refused("unknown_haul", f"“{name}” kimlikli ya da adlı Brükner projede yok.", "haul")
    return h


def volume_approval(h, volumes):
    """The diagram's volume's revision and its approval's number (an unnumbered one counting as 1; 0: none)."""
    v = next((x for x in volumes if x["id"] == h["volume"]), None)
    if v is None:
        return 0, 0
    return v.get("revision", 0), (max(v["approved"].get("number", 0), 1) if v.get("approved") else 0)


def approval_of(h, plan, at, volumes, number=1, note=None):
    """A haul plan approved as planned now (docs/adr/0954): what it was made from and its figures."""
    vrev, vnum = volume_approval(h, volumes)
    out = {"number": number, "at": at}
    if note is not None:
        out["note"] = note
    out.update({"revision": h.get("revision", 0), "volumeRevision": vrev})
    if vnum:
        out["volumeApproval"] = vnum
    if plan["objective"] == "cost":
        out["cost"] = True
    out.update({"moment": plan["moment"], "moved": plan["moved"]})
    if "cost" in plan:
        out["total"] = plan["cost"]["total"]
    if plan["sites"]:
        out["sites"] = [{"name": x["name"], "capacity": x["capacity"], "used": x["used"]} for x in plan["sites"]]
    return out


def against(h, a, today, volumes):
    """The approval `a` against today's plan made with its objective (none: it does not compute): why it went stale,
    the moment's change and the sites that changed."""
    reasons, sites = [], []
    if h.get("revision", 0) != a["revision"]:
        reasons.append(f"Brüknerin tanımı değişti (sürüm {a['revision']} → {h.get('revision', 0)})")
    vrev, vnum = volume_approval(h, volumes)
    if vrev != a["volumeRevision"]:
        reasons.append(f"kübajın tanımı değişti (sürüm {a['volumeRevision']} → {vrev})")
    if vnum != a.get("volumeApproval", 0):
        words = lambda n: "onaysız" if n == 0 else f"{n}. onay"  # noqa: E731
        reasons.append(f"kübajın onayı değişti ({words(a.get('volumeApproval', 0))} → {words(vnum)})")
    defs = h.get("sites", [])
    approved = a.get("sites", [])

    def used_of(name):
        if today is None:
            return None
        return next((x["used"] for x in today["sites"] if x["name"] == name), None)

    for x in approved:
        now = next((d for d in defs if d["name"] == x["name"]), None)
        capacity = now["volume"] if now else None
        used = used_of(x["name"])
        changed = capacity is None or differ(capacity, x["capacity"])
        if changed:
            reasons.append(f"{x['name']} alanının kapasitesi değişti ({x['capacity']:.1f} → {capacity:.1f} m³)"
                           if capacity is not None else f"{x['name']} alanı kalktı")
        if changed or used is None or differ(used, x["used"]):
            row = {"name": x["name"], "approvedCapacity": x["capacity"]}
            if capacity is not None:
                row["capacity"] = capacity
            row["approvedUsed"] = x["used"]
            if used is not None:
                row["used"] = used
            sites.append(row)
    for d in defs:
        if not any(x["name"] == d["name"] for x in approved):
            reasons.append(f"{d['name']} alanı eklendi ({d['volume']:.1f} m³)")
            row = {"name": d["name"], "capacity": d["volume"]}
            used = used_of(d["name"])
            if used is not None:
                row["used"] = used
            sites.append(row)
    if today is None:
        reasons.append("plan bugün kurulamıyor")
    else:
        parts = []
        if differ(today["moment"], a["moment"]):
            parts.append(f"moment {today['moment'] - a['moment']:+.1f} m³·m")
        if differ(today["moved"], a["moved"]):
            parts.append(f"taşınan {today['moved'] - a['moved']:+.1f} m³")
        c = today.get("cost")
        if c is not None and "total" in a:
            if differ(c["total"], a["total"]):
                parts.append(f"tutar {c['total'] - a['total']:+.0f} {c['currency']}")
        elif c is None and "total" in a:
            parts.append("tutar bugün hesaplanamıyor")
        elif c is not None:
            parts.append("tutar onayda yoktu")
        if parts:
            reasons.append(f"planın kendisi değişti ({', '.join(parts)})")
    out = {"approved": a, "stale": bool(reasons), "reasons": reasons}
    if today is not None:
        out["momentChange"] = today["moment"] - a["moment"]
    if sites:
        out["sites"] = sites
    return out


def answer(inp, hauls, volumes=VOLUMES, corridors=hc.CORRIDORS):
    """The plan now, and against its approval where the diagram has one (docs/adr/0954): the plan made again with the
    approved objective, its figures and sources against the approved ones."""
    out, warnings = raw(inp, hauls, volumes, corridors)
    h = find(inp["haul"], hauls)
    a = h.get("approved")
    if a is not None:
        objective = "cost" if a.get("cost") else "moment"
        if objective == out["objective"]:
            today = out
        else:
            try:
                today = raw({"haul": h["id"], "objective": objective}, hauls, volumes, corridors)[0]
            except hc.Refused:
                today = None
        report = against(h, a, today, volumes)
        if report["stale"]:
            warnings.append({"code": "haul_plan_stale", "path": "haul",
                             "message": f"“{h['name']}” Brüknerinin {a['number']}. onaylı taşıma planı ({a['at']}) eskidi: "
                                        f"{'; '.join(report['reasons'])}."})
        out = {**out, "approval": report}
    return out, warnings


def raw(inp, hauls, volumes=VOLUMES, corridors=hc.CORRIDORS):
    """The plan now, without its approval."""
    name = inp["haul"]
    h = next((x for x in hauls if x["id"] == name), None) or next(
        (x for x in hauls if x["name"].lower() == name.lower()), None)
    if h is None:
        raise hc.Refused("unknown_haul", f"“{name}” kimlikli ya da adlı Brükner projede yok.", "haul")
    v = next(x for x in volumes if x["id"] == h["volume"])
    try:
        rep, vw = vcmd.compute(v, corridors)
    except vcmd.Refused as r:
        raise hc.Refused("invalid_haul", f"“{v['name']}” kübajı hesaplanamıyor: {r.result['error']['message']}",
                         "volume") from None
    warnings = [w for w in vw if w["code"] in ("corridor_stale", "volume_gap")]
    curve = hc.ordinate(rep)
    c = next(x for x in corridors if x["id"] == v["corridor"])
    first, last = curve[0][0], curve[-1][0]
    # Docs/adr/0955: a site off the road at the station its access meets the alignment, its access the extra way; one
    # the network does not reach left out (no capacity), with a warning.
    import road_haul_network_cases as net  # noqa: PLC0415
    defs = h.get("sites", [])
    where, unplaced = net.placed(h)
    warnings += unplaced
    sites = []
    for x, (st, access, off) in zip(defs, where):
        y = {**x, "station": st}
        if access is not None:
            y["access"] = access
        if off:
            y["volume"] = 0.0
            y["unconnected"] = True
        sites.append(y)
    for j, site in sorted(enumerate(sites), key=lambda t: (t[1]["station"], t[0])):
        if site.get("unconnected"):
            continue
        if site["station"] < first or site["station"] > last:
            raise hc.Refused("invalid_haul", f"“{site['name']}” alanı kübajın km'leri dışında: kübaj "
                                             f"{ccmd.km_text(c['alignment'], first)} – {ccmd.km_text(c['alignment'], last)} arası.",
                             f"sites[{j}]")
    objective = inp.get("objective", "moment")
    if h.get("unsuitable") or h.get("barriers"):
        if objective == "cost":
            raise hc.Refused("invalid_haul", f"“{h['name']}” Brüknerinde uygunsuz malzeme ya da engel var; ekonomik plan "
                                             "onlarla henüz çözülmez, momentle planlayın.", "objective")
        return sorted_answer(h, rep, sites, warnings, defs)
    priced = None
    if objective == "cost":
        c = h.get("cost")
        if c is None:
            raise hc.Refused("invalid_haul", f"“{h['name']}” Brüknerinin fiyatları yok; ekonomik plan fazla taşıma birim fiyatı ister.",
                             "objective")
        if not c.get("overhaul"):
            raise hc.Refused("invalid_haul", f"“{h['name']}” Brüknerinin fazla taşıma birim fiyatı yok ya da 0; ekonomik plan onu ister.",
                             "objective")
        for kind, key, words in (("waste", "waste", "döküm alanı var ama döküm birim fiyatı yok"),
                                 ("borrow", "borrow", "ödünç alanı var ama ödünç birim fiyatı yok")):
            if key not in c and any(x["kind"] == kind for x in defs):
                raise hc.Refused("invalid_haul", f"“{h['name']}” Brüknerinin {words}.", "objective")
        prices = {"free": h.get("free", 0.0), "overhaul": c["overhaul"] / 1000, "waste": c.get("waste", 0.0),
                  "borrow": c.get("borrow", 0.0)}
        p = solve_priced(curve, sites, prices)
        priced = (c, prices)
    else:
        p = solve(curve, sites)
    if not p["feasible"]:
        warnings.append({"code": "haul_plan_infeasible", "path": "haul",
                         "message": f"“{h['name']}” taşıma planı her şeyi yerleştiremiyor: karşılanmayan dolgu "
                                    f"{p['shortage']:.3f} m³, yerleşmeyen yarma {p['surplus']:.3f} m³; ödünç ya da döküm "
                                    f"alanı ekleyin ya da büyütün."})
    out = {"haul": h["id"], "feasible": p["feasible"], "cut": p["cut"], "fill": p["fill"], "moved": p["moved"],
           "moment": p["moment"],
           "sites": [site_row(d, s, u) for d, s, u in zip(defs, sites, p["used"])],
           "segments": p["segments"], "shortage": p["shortage"], "surplus": p["surplus"]}
    if any("access" in s for s in sites):
        out["accessMoment"] = p.get("access", 0.0)
    if p["feasible"] and p["moved"] > 0:
        out["average"] = p["moment"] / p["moved"]
    out["objective"] = objective
    if priced is not None:
        c, prices = priced
        if "free" in h:
            out["free"] = h["free"]
        if p["feasible"]:
            cost = {"currency": c["currency"], "overMoment": p["over"], "overhaul": p["overhaul"], "waste": p["waste"],
                    "borrow": p["borrow"], "total": p["total"]}
            if "waste" in c or "borrow" in c:
                cost["economic"] = prices["free"] + (prices["waste"] + prices["borrow"]) / prices["overhaul"]
            out["cost"] = cost
    return out, warnings


def site_row(d, s, used):
    """A site in the plan's answer: its own capacity, the station it is placed at, its access or that it is cut off."""
    row = {"name": d["name"], "kind": d["kind"], "station": s["station"], "capacity": d["volume"], "used": used}
    if "access" in s:
        row["access"] = s["access"]
    if s.get("unconnected"):
        row["unconnected"] = True
    return row


def sorted_answer(h, rep, sites, warnings, defs=None):
    """A diagram with unsuitable cut or barriers (docs/adr/0944): road_haul_sorted_cases.py's programme on each
    stretch's factored cut and fill, the unsuitable share at its middle."""
    def share(s):
        return next((u["share"] / 100 for u in h.get("unsuitable", []) if u["from"] <= s < u["to"]), 0.0)
    stretches = [{"from": r["from"], "to": r["to"], "cut": r["cut"] * r.get("cutFactor", 1.0),
                  "fill": r["fill"] * r.get("fillFactor", 1.0), "unsuitable": share((r["from"] + r["to"]) / 2)}
                 for r in rep["rows"] if r.get("cut") is not None and r.get("fill") is not None]
    defs = defs if defs is not None else sites
    plan_sites = [{"station": x["station"], "capacity": x["volume"], "borrow": x["kind"] == "borrow",
                   "access": x.get("access", 0.0)} for x in sites]
    barriers = [{"station": b["station"], "detour": b.get("detour")} for b in h.get("barriers", [])]
    d = sref.solve(stretches, plan_sites, barriers, detail=True)
    tiny = 1e-6 * max(sum(r["cut"] + r["fill"] for r in stretches) + sum(x["volume"] for x in sites), 1.0)
    feasible = bool(d["shortage"] <= tiny and d["surplus"] <= tiny and d["stranded"] <= tiny)
    cut = sum(max(r["cut"] - r["fill"], 0.0) for r in stretches)
    fill = sum(max(r["fill"] - r["cut"], 0.0) for r in stretches)
    leaves = sum(max(r["cut"] * (1 - r["unsuitable"]) - r["fill"], 0.0) + r["cut"] * r["unsuitable"] for r in stretches)
    borrowed = sum(u for x, u in zip(sites, d["used"]) if x["kind"] == "borrow")
    moved = max(leaves - d["surplus"] - d["stranded"], 0.0) + borrowed
    if not feasible:
        warnings.append({"code": "haul_plan_infeasible", "path": "haul",
                         "message": f"“{h['name']}” taşıma planı her şeyi yerleştiremiyor: karşılanmayan dolgu "
                                    f"{d['shortage']:.3f} m³, yerleşmeyen uygun yarma {d['surplus']:.3f} m³, dökülemeyen "
                                    f"uygunsuz yarma {d['stranded']:.3f} m³; ödünç ya da döküm alanı ekleyin, büyütün ya da "
                                    "kapalı engeli açın."})
    out = {"haul": h["id"], "feasible": feasible, "cut": cut, "fill": fill, "moved": moved, "moment": d["moment"]}
    if feasible and moved > 0:
        out["average"] = d["moment"] / moved
    out["sites"] = []
    for dd, x, u, ub in zip(defs, sites, d["used"], d["usedUnsuitable"]):
        row = site_row(dd, x, u)
        if x["kind"] == "waste":
            row["usedUnsuitable"] = ub
        out["sites"].append(row)
    out["segments"] = [{"from": a, "to": b, "flow": g + bb, "unsuitableFlow": bb} for a, b, g, bb in d["segments"]]
    out.update({"shortage": d["shortage"], "surplus": d["surplus"], "objective": "moment",
                "unsuitable": sum(r["cut"] * r["unsuitable"] for r in stretches), "stranded": d["stranded"],
                "unsuitableMoment": d["unsuitableMoment"]})
    if h.get("barriers"):
        out["barriers"] = [{"name": b["name"], "station": b["station"], **({"detour": b["detour"]} if "detour" in b else {}),
                            "crossed": c} for b, c in zip(h["barriers"], d["crossed"])]
    if any("access" in x for x in sites):
        out["accessMoment"] = d.get("access", 0.0)
    return out, warnings


def site(name, kind, station, volume):
    return {"name": name, "kind": kind, "station": station, "volume": volume}


def cases():
    out = []

    def q(name, inp, hauls, volumes=VOLUMES, corridors=hc.CORRIDORS, op="execute", setup_=None, result=None,
          relative=False):
        if result is None:
            try:
                o, w = answer(inp, hauls, volumes, corridors)
                result = completed(None if op == "validate" else o, w)
            except hc.Refused as r:
                result = r.result
        if setup_ is None:
            setup_ = hc.setup(hauls, volumes, corridors)
            road = hc.road_of(hauls, volumes, corridors)
        else:
            road = setup_["settings"].get("road")
        step = {"op": op, "input": inp, "result": result, "expect": {**UNCHANGED, "road": road}, "within": W}
        if relative:
            # Moments of 10⁷ m³·m summed in other orders agree to ~1e-9 of their size, not to 1e-3 m³·m; and where the
            # plan cannot place everything, more than one placement is free on a stretch and HiGHS's tolerance stays in
            # them (~1e-5 m³). A different plan moves thousands of m³ over metres: 1e-5 of the size still tells it.
            step.update({"within": 1e-5, "relative": True})
        out.append(case(name, [step], setup_))

    big = 30000.0
    q("Alanı olmayan, yarması fazla Brükner: plan kurulamaz", {"haul": "h-1"}, [H1, H2, H3])
    q("Döküm alanı yetmez", {"haul": "h-2"}, [H1, H2, H3])
    q("Döküm alanı biraz küçük", {"haul": "h-1"},
      [{**H1, "sites": [site("Döküm", "waste", 600.0, 28116.2)]}, H2, H3])
    q("Sonda döküm alanı", {"haul": "h-1"}, [{**H1, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3])
    q("Ortada döküm alanı", {"haul": "h-1"}, [{**H1, "sites": [site("Döküm", "waste", 300.0, big)]}, H2, H3])
    q("Başta döküm alanı", {"haul": "h-1"}, [{**H1, "sites": [site("Döküm", "waste", 0.0, big)]}, H2, H3])
    q("İki döküm alanı: yakın olan dolar", {"haul": "ANA brükner"},
      [{**H1, "sites": [site("D1", "waste", 40.0, 10000.0), site("D2", "waste", 590.0, 25000.0)]}, H2, H3])
    q("İki döküm alanı, ikisi de kısmen değil", {"haul": "h-1"},
      [{**H1, "sites": [site("D1", "waste", 100.0, 8000.0), site("D2", "waste", 560.0, 20116.20365964641)]}, H2, H3])
    q("Yan Brükner: büyük döküm alanı", {"haul": "h-2"},
      [H1, {**H2, "sites": [site("Döküm 1", "waste", 400.0, 20000.0)]}, H3])
    q("Yan Brükner: ödünç alanı dolguları besler", {"haul": "Yan Brükner"},
      [H1, {**H2, "sites": [site("Ödünç", "borrow", 50.0, 2000.0), site("Döküm 1", "waste", 400.0, 20000.0)]}, H3])
    q("Yan Brükner: büyük ödünç alanı", {"haul": "h-2"},
      [H1, {**H2, "sites": [site("Ödünç", "borrow", 50.0, 9000.0), site("Döküm 1", "waste", 380.0, 25000.0)]}, H3])
    q("Yan Brükner: uzak ödünç kullanılmaz", {"haul": "h-2"},
      [H1, {**H2, "sites": [site("Ödünç", "borrow", 400.0, 3000.0), site("Döküm 1", "waste", 380.0, 15000.0)]}, H3])
    q("Dolgu fazlası: ödünçsüz plan kurulamaz", {"haul": "h-3"}, [H1, H2, H3])
    q("Dolgu fazlası: sonda ödünç alanı", {"haul": "h-3"},
      [H1, H2, {**H3, "sites": [site("Ödünç", "borrow", 600.0, 20000.0)]}])
    q("Dolgu fazlası: ortada ödünç alanı", {"haul": "h-3"},
      [H1, H2, {**H3, "sites": [site("Ödünç", "borrow", 390.0, 20000.0)]}])
    q("Dolgu fazlası: iki ödünç alanı", {"haul": "Dolgu Brükneri"},
      [H1, H2, {**H3, "sites": [site("Ö1", "borrow", 160.0, 3000.0), site("Ö2", "borrow", 420.0, 18000.0)]}])
    q("Dolgu fazlası: ödünç yetmez", {"haul": "h-3"},
      [H1, H2, {**H3, "sites": [site("Ödünç", "borrow", 400.0, 10000.0)]}])
    q("Dolgu fazlası: ödünç ve döküm birlikte", {"haul": "h-3"},
      [H1, H2, {**H3, "sites": [site("Ödünç", "borrow", 380.0, 16000.0), site("Döküm", "waste", 30.0, 1000.0)]}])
    c_gap = {**ccmd.C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"},
                                    {"from": 250.0, "to": 400.0, "template": "tk-2"}]}
    q("Kesitsiz aralıklı kübaj", {"haul": "h-1"}, [{**H1, "sites": [site("Döküm", "waste", 400.0, big)]}, H2, H3],
      VOLUMES, [c_gap, ccmd.C2])
    q("Prizmoidal kübaj", {"haul": "h-1"}, [{**H1, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3],
      [{**vcmd.V1, "method": "prismoidal"}, vcmd.V2, V3])
    hauls = [{**H1, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3]
    q("Plan cevabı verir", {"haul": "h-1"}, hauls, op="plan")
    q("Doğrulama uyarıları verir", {"haul": "h-2"}, hauls, op="validate")
    # The economic plan (docs/adr/0925).
    P = {"currency": "TL", "overhaul": 800.0, "waste": 40.0, "borrow": 60.0}
    def cost_case(name, haul, hauls, inp=None):
        q(name, {"haul": haul, "objective": "cost", **(inp or {})}, hauls)
    cost_case("Ekonomik: sonda döküm alanı, serbest taşıma 150 m", "h-1",
              [{**H1, "cost": P, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3])
    cost_case("Ekonomik: iki döküm alanı", "h-1",
              [{**H1, "cost": P, "sites": [site("D1", "waste", 40.0, 10000.0), site("D2", "waste", 590.0, 25000.0)]}, H2, H3])
    cost_case("Ekonomik: pahalı fazla taşıma dökümü öne çeker", "h-1",
              [{**H1, "cost": {**P, "overhaul": 5000.0}, "sites": [site("D1", "waste", 40.0, 10000.0),
                                                                  site("D2", "waste", 590.0, 25000.0)]}, H2, H3])
    cost_case("Ekonomik: yan Brükner, ödünç ve döküm", "h-2",
              [H1, {**H2, "cost": P, "sites": [site("Ödünç", "borrow", 50.0, 2000.0), site("Döküm 1", "waste", 400.0, 20000.0)]}, H3])
    cost_case("Ekonomik: serbest taşımalı yan Brükner", "Yan Brükner",
              [H1, {**H2, "free": 100.0, "cost": P, "sites": [site("Ödünç", "borrow", 50.0, 2000.0),
                                                                site("Döküm 1", "waste", 400.0, 20000.0)]}, H3])
    cost_case("Ekonomik: dolgu fazlası, iki ödünç", "h-3",
              [H1, H2, {**H3, "free": 100.0, "cost": P, "sites": [site("Ö1", "borrow", 160.0, 3000.0),
                                                                  site("Ö2", "borrow", 420.0, 18000.0)]}])
    cost_case("Ekonomik: dolgu fazlası, ödünç ve döküm", "h-3",
              [H1, H2, {**H3, "cost": P, "sites": [site("Ödünç", "borrow", 380.0, 16000.0), site("Döküm", "waste", 30.0, 1000.0)]}])
    cost_case("Ekonomik: yalnız ödünç fiyatı", "h-3",
              [H1, H2, {**H3, "free": 50.0, "cost": {"currency": "EUR", "overhaul": 2.0, "borrow": 4.5},
                        "sites": [site("Ödünç", "borrow", 600.0, 20000.0)]}])
    cost_case("Ekonomik: plan kurulamaz", "h-1", [{**H1, "cost": P}, H2, H3])
    q("Amaç açıkça moment", {"haul": "h-1", "objective": "moment"},
      [{**H1, "cost": P, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3])
    cost_case("Ekonomik: fiyat yok", "h-1", [{**H1, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3])
    cost_case("Ekonomik: fazla taşıma fiyatı yok", "h-1",
              [{**H1, "cost": {"currency": "TL", "waste": 40.0}, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3])
    cost_case("Ekonomik: döküm fiyatı yok", "h-1",
              [{**H1, "cost": {"currency": "TL", "overhaul": 800.0}, "sites": [site("Döküm", "waste", 600.0, big)]}, H2, H3])
    cost_case("Ekonomik: ödünç fiyatı yok", "h-3",
              [H1, H2, {**H3, "cost": {"currency": "TL", "overhaul": 800.0}, "sites": [site("Ödünç", "borrow", 400.0, 20000.0)]}])
    # Docs/adr/0944: the unsuitable cut goes to waste only; barriers detour or close the haul road.
    sorting = {"sites": [site("Döküm", "waste", 30.0, big), site("Ödünç", "borrow", 560.0, big)]}
    q("Uygunsuz yarma yalnız dökümde", {"haul": "h-1"},
      [{**H1, **sorting, "unsuitable": [{"from": 0.0, "to": 300.0, "share": 40.0}]}, H2, H3], relative=True)
    q("Bütün yarma uygunsuz: dolgular ödünçten", {"haul": "h-1"},
      [{**H1, **sorting, "unsuitable": [{"from": 0.0, "to": 600.0, "share": 100.0}]}, H2, H3], relative=True)
    q("Uygunsuz için döküm yetmez", {"haul": "h-1"},
      [{**H1, "sites": [site("Döküm", "waste", 30.0, 500.0), site("Ödünç", "borrow", 560.0, big)],
        "unsuitable": [{"from": 0.0, "to": 600.0, "share": 60.0}]}, H2, H3], relative=True)
    q("Dere geçişinde 400 m dolaşma", {"haul": "h-1"},
      [{**H1, **sorting, "barriers": [{"name": "Dere", "station": 250.0, "detour": 400.0}]}, H2, H3], relative=True)
    q("Kapalı kavşak taşımayı ikiye böler", {"haul": "h-1"},
      [{**H1, **sorting, "barriers": [{"name": "Kavşak", "station": 300.0}]}, H2, H3], relative=True)
    q("Uygunsuz yarma ve engel birlikte", {"haul": "Ana Brükner"},
      [{**H1, **sorting, "unsuitable": [{"from": 100.0, "to": 250.0, "share": 30.0}],
        "barriers": [{"name": "Dere", "station": 420.0, "detour": 150.0}]}, H2, H3], relative=True)
    q("Engel sona yakın, kimse geçmiyor", {"haul": "h-1"},
      [{**H1, **sorting, "barriers": [{"name": "Uzak dere", "station": 590.0, "detour": 50.0}]}, H2, H3], relative=True)
    q("Uygunsuzla ekonomik plan çözülmez", {"haul": "h-1", "objective": "cost"},
      [{**H1, **sorting, "cost": P, "unsuitable": [{"from": 0.0, "to": 300.0, "share": 40.0}]}, H2, H3])
    # Docs/adr/0954: a plan read against its approval.
    def signed(h, hauls_with, volumes=VOLUMES, objective="moment", at="2026-11-04", number=1, note=None):
        """`h` approved as `hauls_with` (its own state then) planned with `volumes`."""
        then = find(h["id"], hauls_with)
        plan = raw({"haul": h["id"], "objective": objective}, hauls_with, volumes)[0]
        return {**h, "approved": approval_of(then, plan, at, volumes, number, note)}

    two = [site("D1", "waste", 40.0, 10000.0), site("D2", "waste", 590.0, 25000.0)]
    ha = {**H1, "sites": two}
    q("Onaylı plan değişmedi", {"haul": "h-1"}, [signed(ha, [ha, H2, H3]), H2, H3])
    hb = {**H1, "revision": 2, "sites": [site("D1", "waste", 40.0, 14000.0), site("D2", "waste", 590.0, 25000.0)]}
    q("Onaydan sonra alanın kapasitesi büyüdü", {"haul": "h-1"},
      [{**hb, "approved": signed(ha, [ha, H2, H3])["approved"]}, H2, H3])
    hc_ = {**H1, "revision": 3, "sites": [site("D1", "waste", 40.0, 10000.0), site("D3", "waste", 560.0, 25000.0)]}
    q("Onaydan sonra alan kalktı, yenisi eklendi", {"haul": "Ana Brükner"},
      [{**hc_, "approved": signed(ha, [ha, H2, H3], note="İlk plan")["approved"]}, H2, H3])
    v1b = {**vcmd.V1, "revision": 2, "factors": [{"from": 0.0, "to": 600.0, "cut": 1.3, "fill": 0.9}]}
    q("Onaydan sonra kübaj değişti", {"haul": "h-1"},
      [signed(ha, [ha, H2, H3]), H2, H3], [v1b, vcmd.V2, V3])
    q("Onaydan sonra kübaj onaylandı", {"haul": "h-1"},
      [signed(ha, [ha, H2, H3]), H2, H3], [vcmd.approved(vcmd.V1), vcmd.V2, V3])
    q("Onaylı kübajla onaylanan plan değişmedi", {"haul": "h-1"},
      [signed(ha, [ha, H2, H3], [vcmd.approved(vcmd.V1), vcmd.V2, V3]), H2, H3], [vcmd.approved(vcmd.V1), vcmd.V2, V3])
    hp = {**H1, "cost": P, "sites": two}
    q("Ekonomik onay moment planıyla okunur", {"haul": "h-1"},
      [signed(hp, [hp, H2, H3], objective="cost", number=3), H2, H3])
    q("Ekonomik onay, fiyat değişti", {"haul": "h-1", "objective": "cost"},
      [{**hp, "revision": 2, "cost": {**P, "overhaul": 5000.0},
        "approved": signed(hp, [hp, H2, H3], objective="cost")["approved"]}, H2, H3])
    hn = {**H1, "revision": 2, "sites": two}
    q("Ekonomik onay, fiyatlar kalktı: plan bugün kurulamıyor", {"haul": "h-1"},
      [{**hn, "approved": signed(hp, [hp, H2, H3], objective="cost")["approved"]}, H2, H3])
    # Docs/adr/0955: sites off the road, reached by the haul road network.
    import road_haul_network_cases as net  # noqa: PLC0415

    def qn(name, inp, hauls, volumes=VOLUMES, relative=False):
        o, w = answer(inp, hauls, volumes)
        q(name, inp, hauls, volumes, setup_=net.setup(hauls, volumes), result=completed(o, w), relative=relative)

    hn = {**H1, "network": net.NETWORK}
    qn("Ağdan döküm alanı: erişim yolu momente katılır", {"haul": "h-1"}, [{**hn, "sites": [net.SITE_W]}, H2, H3])
    on_road = {k: v for k, v in net.SITE_W.items() if k != "at"}
    q("Aynı alan yol üstünde: erişimsiz", {"haul": "h-1"}, [{**H1, "sites": [on_road]}, H2, H3])
    qn("Kopuk alan hesaba katılmaz", {"haul": "h-1"},
       [{**hn, "sites": [net.SITE_C, site("Son döküm", "waste", 600.0, big)]}, H2, H3])
    qn("Uzak alan hesaba katılmaz, plan kurulamaz", {"haul": "h-1"}, [{**hn, "sites": [net.SITE_F]}, H2, H3])
    qn("Ağdan döküm ve yol üstü döküm birlikte", {"haul": "Ana Brükner"},
       [{**hn, "sites": [net.SITE_W, site("D2", "waste", 590.0, 25000.0)]}, H2, H3], relative=True)
    qn("Ekonomik: erişim fazla taşımaya katılır", {"haul": "h-1", "objective": "cost"},
       [{**hn, "cost": P, "sites": [net.SITE_W, site("D2", "waste", 590.0, 25000.0)]}, H2, H3])
    h3n = {**H3, "network": net.NETWORK}
    qn("Ağdan ödünç alanı km 520'den dolguları besler", {"haul": "h-3"},
       [H1, H2, {**h3n, "sites": [net.SITE_B, site("Ödünç", "borrow", 380.0, 12000.0)]}], relative=True)
    qn("Uygunsuz yarma ağdaki dökümde", {"haul": "h-1"},
       [{**hn, "sites": [net.SITE_W, site("Ödünç", "borrow", 560.0, big)],
         "unsuitable": [{"from": 0.0, "to": 300.0, "share": 40.0}]}, H2, H3], relative=True)
    # Refusals.
    q("Yol projesi değil", {"haul": "h-1"}, [H1, H2, H3], setup_=hc.setup(work="map"), result=hc.NOT_ROAD)
    q("Olmayan Brükner", {"haul": "yok"}, [H1, H2, H3])
    q("Km'lerin dışında alan", {"haul": "h-1"},
      [{**H1, "sites": [site("Uzak", "waste", 800.0, big), site("Yakın", "waste", 100.0, 10.0)]}, H2, H3])
    broken = {**ccmd.C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"},
                                     {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    q("Hesaplanamayan kübaj", {"haul": "h-1"}, [H1, H2, H3], VOLUMES, [broken, ccmd.C2])
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.hauls.plan", "commandVersion": 1,
           "title": "Taşıma planı: en küçük taşıma momentiyle dağıtım",
           "note": "scripts/fixtures/road_haul_plan_cases.py yazar; çizim, koridorlar, kübajlar ve Brüknerler "
                   "road_haul_command_cases.py'nin, plan HiGHS (scipy linprog) doğrusal programıdır; alanların kullanımı "
                   "teklik denetiminden sonra tam, akışlar kesirlerle; KentOS kodu yok.",
           "setup": hc.setup([H1, H2, H3], VOLUMES), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.hauls.plan.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.hauls.plan.json")
    else:
        path.write_text(text)
    print(f"road.hauls.plan.json: {len(found)} durum")


if __name__ == "__main__":
    main()
