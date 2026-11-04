#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.hauls.plan.json: the shared cases of KentOS Yol's haul plan (plan/road.md Y23,
docs/adr/0924; ROAD-HAUL-02, ROAD-HAUL-07), their expectations built here without KentOS code. The drawing, its
corridors, volumes and mass haul diagrams are road_haul_command_cases.py's (the volume sheet's mass ordinate). The plan
is a linear programme solved by HiGHS (scipy's linprog): each stretch's net at its middle, a site at its station, the
earth flowing along the line at a cost of its distance; every cut placed, every fill met, a site taking up to its
capacity; the least haul moment. A site's use is then checked unique (its least and its most among plans of that
moment), snapped to 0 or its capacity where it lies there, the one left balancing the totals; the segments' flows
follow exactly (in fractions) from what each place gives or takes.

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
                "shortage": shortage, "surplus": surplus}
    stations = sorted({p[0] for p in places} | {s["station"] for s in sites})
    n, m = len(stations), len(sites)
    at = {s: k for k, s in enumerate(stations)}
    lengths = [stations[i + 1] - stations[i] for i in range(n - 1)]
    nv = 2 * (n - 1) + m
    cost = np.array(lengths + lengths + [0.0] * m)
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
        assert ends[1] - ends[0] < 1e-3, f"site {j}'s use is not unique: {ends}"
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
    return {**base, "feasible": True, "used": [float(e) for e in exact], "segments": segments, "moment": float(moment),
            "moved": cut + borrowed}


def answer(inp, hauls, volumes=VOLUMES, corridors=hc.CORRIDORS):
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
    sites = h.get("sites", [])
    for j, site in sorted(enumerate(sites), key=lambda t: (t[1]["station"], t[0])):
        if site["station"] < first or site["station"] > last:
            raise hc.Refused("invalid_haul", f"“{site['name']}” alanı kübajın km'leri dışında: kübaj "
                                             f"{ccmd.km_text(c['alignment'], first)} – {ccmd.km_text(c['alignment'], last)} arası.",
                             f"sites[{j}]")
    p = solve(curve, sites)
    if not p["feasible"]:
        warnings.append({"code": "haul_plan_infeasible", "path": "haul",
                         "message": f"“{h['name']}” taşıma planı her şeyi yerleştiremiyor: karşılanmayan dolgu "
                                    f"{p['shortage']:.3f} m³, yerleşmeyen yarma {p['surplus']:.3f} m³; ödünç ya da döküm "
                                    f"alanı ekleyin ya da büyütün."})
    out = {"haul": h["id"], "feasible": p["feasible"], "cut": p["cut"], "fill": p["fill"], "moved": p["moved"],
           "moment": p["moment"],
           "sites": [{"name": s["name"], "kind": s["kind"], "station": s["station"], "capacity": s["volume"], "used": u}
                     for s, u in zip(sites, p["used"])],
           "segments": p["segments"], "shortage": p["shortage"], "surplus": p["surplus"]}
    if p["feasible"] and p["moved"] > 0:
        out["average"] = p["moment"] / p["moved"]
    return out, warnings


def site(name, kind, station, volume):
    return {"name": name, "kind": kind, "station": station, "volume": volume}


def cases():
    out = []

    def q(name, inp, hauls, volumes=VOLUMES, corridors=hc.CORRIDORS, op="execute", setup_=None, result=None):
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
