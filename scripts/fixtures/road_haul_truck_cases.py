#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.hauls.trucks.json: the shared cases of KentOS Yol's haul fleet (plan/road.md Y32.2,
docs/adr/0936 §2; ROAD-HAUL-10), their expectations built here without KentOS code. The drawing, its corridors,
volumes and diagrams are road_haul_command_cases.py's; the plan is road_haul_plan_cases.py's (HiGHS); the road's
heights the profile reference's (road_vertical_cases.py, every metre, its PVIs and curve ends) or the terrain's
(covered as the terrain profile says, its heights the triangles' own every metre: along a curve the height is
not linear in station between two edges' crossings); each run road_haul_fleet_cases.py's, integrated in time (RK4,
at 1 ms, as in Y32.1); the
loader its Markov chain solved numerically. The hauls are the plan's flows split first in, first out, by definition.
The results are compared within 2·10⁻³ of their size (`relative`): seconds, litres and ratios side by side.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with scipy \\
        python3 scripts/fixtures/road_haul_truck_cases.py [--check]
"""

import json
import math
import sys
from collections import deque
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_haul_command_cases as hc  # noqa: E402
import road_haul_fleet_cases as fleet  # noqa: E402
import road_haul_plan_cases as hp  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 2e-3
fleet.DT = 1e-3
completed, case = hc.completed, hc.case
UNCHANGED = hc.UNCHANGED

TRUCK = {"empty": 15.0, "gross": 40.0, "power": 300.0, "efficiency": 0.8, "top": 50.0, "downhill": 30.0,
         "traction": 0.6, "drive": 0.7, "brake": 2.0}
BASE = {"track": "profile", "truck": TRUCK, "rolling": 3.0, "load": 15.0, "loading": 2.5, "dumping": 1.0,
        "spotting": 0.5}
SITES2 = [hp.site("Ödünç", "borrow", 50.0, 2000.0), hp.site("Döküm 1", "waste", 400.0, 20000.0)]
HAULS = [hc.H1, {**hc.H2, "sites": SITES2}, hp.H3]


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.result = {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def bad(path, words):
    return Refused("invalid_haul_trucks", words, path)


def judged(q):
    t = q["truck"]

    def positive(v):
        return v is not None and math.isfinite(v) and v > 0

    checks = [
        (positive(t["empty"]), "truck.empty", "Kamyonun boş kütlesi 0'dan büyük olmalı (t)."),
        (t["gross"] > t["empty"], "truck.gross", "Kamyonun yüklü kütlesi boş kütlesinden büyük olmalı (t)."),
        (positive(t["power"]), "truck.power", "Motor gücü 0'dan büyük olmalı (kW)."),
        (positive(t["efficiency"]) and t["efficiency"] <= 1, "truck.efficiency", "Aktarma verimi 0 ile 1 arası olmalı."),
        (positive(t["top"]), "truck.top", "En yüksek hız 0'dan büyük olmalı (km/sa)."),
        (t.get("downhill") is None or positive(t["downhill"]), "truck.downhill", "İniş hızı 0'dan büyük olmalı (km/sa)."),
        (positive(t["traction"]) and t["traction"] <= 2, "truck.traction", "Çekiş katsayısı 0 ile 2 arası olmalı."),
        (positive(t["drive"]) and t["drive"] <= 1, "truck.drive", "Tahrik dingilindeki ağırlık payı 0 ile 1 arası olmalı."),
        (positive(t["brake"]), "truck.brake", "Fren yavaşlaması 0'dan büyük olmalı (m/s²)."),
        (0 <= q["rolling"] < 50, "rolling", "Yuvarlanma direnci 0 ile %50 arası olmalı."),
        (positive(q["load"]), "load", "Kamyonun taşıdığı hacim 0'dan büyük olmalı (m³)."),
        (positive(q["loading"]), "loading", "Yükleme süresi 0'dan büyük olmalı (dk)."),
        (q["dumping"] >= 0, "dumping", "Boşaltma süresi negatif olamaz (dk)."),
        (q.get("spotting") is None or q["spotting"] >= 0, "spotting", "Manevra süresi negatif olamaz (dk)."),
        (q.get("trucks") is None or 1 <= q["trucks"] <= 500, "trucks", "Kamyon sayısı 1 ile 500 arası olmalı."),
        (q.get("hour") is None or 0 < q["hour"] <= 60, "hour", "Saatte çalışılan dakika 0 ile 60 arası olmalı."),
        (q.get("fuel") is None or positive(q["fuel"]), "fuel", "Özgül yakıt tüketimi 0'dan büyük olmalı (L/kWh)."),
        (q.get("idle") is None or q["idle"] >= 0, "idle", "Rölanti yakıtı negatif olamaz (L/sa)."),
        (q.get("co2") is None or positive(q["co2"]), "co2", "Yakıtın CO₂ salımı 0'dan büyük olmalı (kg/L)."),
    ]
    for ok, path, words in checks:
        if not ok:
            raise bad(path, words)
    if q.get("idle") is not None and q.get("fuel") is None:
        raise bad("idle", "Rölanti yakıtı özgül yakıt tüketimiyle (fuel) birlikte verilir.")
    if q.get("co2") is not None and q.get("fuel") is None:
        raise bad("co2", "CO₂ yakıttan hesaplanır: özgül yakıt tüketimini (fuel) de verin.")


def split(segments):
    """The flows split first in, first out: forward left to right, then backward right to left."""
    out = []
    n = len(segments)
    if n == 0:
        return out
    for forward in (True, False):
        flow = [max(s["flow"], 0.0) if forward else max(-s["flow"], 0.0) for s in segments]
        places = [s["from"] for s in segments] + [segments[-1]["to"]]
        queue = deque()
        order = range(n + 1) if forward else range(n, -1, -1)
        for i in order:
            if forward:
                inflow, outflow = (flow[i - 1] if i > 0 else 0.0), (flow[i] if i < n else 0.0)
            else:
                inflow, outflow = (flow[i] if i < n else 0.0), (flow[i - 1] if i > 0 else 0.0)
            if outflow > inflow + 1e-9:
                queue.append([places[i], outflow - inflow])
            elif inflow > outflow + 1e-9:
                need = inflow - outflow
                while need > 1e-9 and queue:
                    take = min(queue[0][1], need)
                    out.append((queue[0][0], places[i], take))
                    queue[0][1] -= take
                    need -= take
                    if queue[0][1] <= 1e-9:
                        queue.popleft()
    return out


def profile_heights(prof):
    curves, ev, _, _ = pcmd.solve(prof)
    breaks = [p["station"] for p in prof["pvis"]]
    for b, e, *_ in curves.values():
        breaks += [float(b), float(e)]

    def path(lo, hi):
        stations = {lo, hi} | {float(s) for s in breaks if lo < s < hi}
        s = math.floor(lo) + 1.0
        while s < hi:
            stations.add(s)
            s += 1.0
        pts = []
        for s in sorted(stations):
            r = ev(s)
            if r is None:
                return None, s
            pts.append((s, float(r[0])))
        return pts, None

    return path


def ground_heights(alignment):
    """The terrain between two stations: covered as the profile's points say (a gap or a point off the surface is
    the first station not covered), its heights the triangles' own every metre and at the ends (pcmd.black_at, exact:
    along a curve the height is not linear in station between two edges' crossings)."""
    points = pcmd.ground(alignment)
    b = pcmd.ALIGNED[alignment]

    def covered(s):
        for k in range(1, len(points)):
            p0, p1 = points[k - 1], points[k]
            if p1["station"] >= s - 1e-12:
                if abs(p1["station"] - s) < 1e-9:
                    return "z" in p1
                return not p1["gap"] and "z" in p0 and "z" in p1
        return False

    def path(lo, hi):
        if not covered(lo):
            return None, lo
        for p in points:
            if lo < p["station"] < hi and (p["gap"] or "z" not in p):
                return None, p["station"]
        if not covered(hi):
            return None, hi
        stations = {lo, hi} | {p["station"] for p in points if lo < p["station"] < hi}
        s = math.floor(lo) + 1.0
        while s < hi:
            stations.add(s)
            s += 1.0
        return [(s, pcmd.black_at(b, s)) for s in sorted(stations)], None

    return path


RUNS = {}


def run_on(path, mass, truck, rolling):
    key = (tuple(map(tuple, path)), mass, json.dumps(truck, sort_keys=True), rolling)
    if key not in RUNS:
        RUNS[key] = fleet.run(path, mass, truck, rolling)
    return RUNS[key]


def answer(q, hauls=HAULS, volumes=hp.VOLUMES, corridors=hc.CORRIDORS):
    name = q["haul"]
    h = next((x for x in hauls if x["id"] == name), None) or next(
        (x for x in hauls if x["name"].lower() == name.lower()), None)
    if h is None:
        raise Refused("unknown_haul", f"“{name}” kimlikli ya da adlı Brükner projede yok.", "haul")
    judged(q)
    plan_in = {"haul": q["haul"]}
    if "objective" in q:
        plan_in["objective"] = q["objective"]
    try:
        plan, warnings = hp.answer(plan_in, hauls, volumes, corridors)
    except hc.Refused as r:
        raise Refused(r.result["error"]["code"], r.result["error"]["message"], r.result["error"]["path"]) from None
    v = next(x for x in volumes if x["id"] == h["volume"])
    c = next(x for x in corridors if x["id"] == v["corridor"])
    prof = scmd.BY_PROFILE[c["profile"]]
    heights = profile_heights(prof) if q["track"] == "profile" else ground_heights(c["alignment"])

    def km(s):
        return ccmd.km_text(c["alignment"], s)

    t = q["truck"]
    truck = {"empty": t["empty"] * 1000, "gross": t["gross"] * 1000, "power": t["power"] * 1000,
             "efficiency": t["efficiency"], "top": t["top"] / 3.6,
             "downhill": None if t.get("downhill") is None else t["downhill"] / 3.6,
             "traction": t["traction"], "drive": t["drive"], "brake": t["brake"]}
    rolling = q["rolling"] / 100
    fixed = 60 * (q["loading"] + q["dumping"] + q.get("spotting", 0.0))
    hs = split(plan["segments"])
    if q.get("trace") is not None and q["trace"] >= len(hs):
        raise bad("trace", f"Planda {len(hs)} taşıma var; {q['trace']} sıralı taşıma yok.")
    rows, runs = [], []
    for a, b, vol in hs:
        lo, hi = min(a, b), max(a, b)
        pts, miss = heights(lo, hi)
        if pts is None:
            if q["track"] == "profile":
                raise bad("track", f"“{prof['name']}” boykesiti km {km(miss)} noktasını kapsamıyor; taşıma {km(a)} – {km(b)} arası.")
            raise bad("track", f"Arazi km {km(miss)} noktasında yok (yüzeyin dışı ya da boşluğu); taşıma {km(a)} – {km(b)} arası.")
        path = [[s - lo, z] for s, z in pts]
        if a > b:
            end = path[-1][0]
            path = [[end - x, z] for x, z in reversed(path)]
        back = [[path[-1][0] - x, z] for x, z in reversed(path)]
        loaded = run_on(path, truck["gross"], truck, rolling)
        if loaded is None:
            raise Refused("haul_truck_stalls", stall_words(True, km(a), km(b), truck), "truck.traction")
        empty = run_on(back, truck["empty"], truck, rolling)
        if empty is None:
            raise Refused("haul_truck_stalls", stall_words(False, km(b), km(a), truck), "truck.traction")
        cycle = loaded["time"] + empty["time"] + fixed
        trips = max(math.ceil(vol / q["load"] - 1e-9), 1)
        row = {"from": a, "to": b, "fromText": km(a), "toText": km(b), "volume": vol, "distance": abs(b - a),
               "rise": path[-1][1] - path[0][1], "loaded": loaded["time"], "empty": empty["time"], "cycle": cycle,
               "trips": float(trips), "balance": cycle / (60 * q["loading"])}
        if q.get("fuel") is not None:
            engine = (loaded["work"] + empty["work"]) / truck["efficiency"] / 3.6e6
            row["fuel"] = q["fuel"] * engine + q.get("idle", 0.0) * (loaded["idle"] + empty["idle"] + fixed) / 3600
        rows.append(row)
        runs.append((loaded, empty))
    trips = sum(r["trips"] for r in rows)
    out = {"haul": plan["haul"], "track": q["track"], "hauls": rows, "trips": float(trips)}
    service = 60 * q["loading"]
    if trips > 0:
        out["cycle"] = sum(r["trips"] * r["cycle"] for r in rows) / trips
        out["balance"] = out["cycle"] / service
    out["energy"] = sum(r["trips"] * (l["work"] + e["work"]) / truck["efficiency"] / 3.6e6 for r, (l, e) in zip(rows, runs))
    hour = q.get("hour", 60.0)
    if q.get("trucks") is not None and trips > 0:
        n = q["trucks"]
        steady_s = queued_s = busy_s = waits = 0.0
        for r in rows:
            ld = fleet.loader(n, r["cycle"] - service, service)
            steady_s += r["trips"] / ld["steady"]
            queued_s += r["trips"] / ld["queued"]
            busy_s += r["trips"] / ld["queued"] * ld["busy"]
            waits += r["trips"] * ld["wait"]

        def hours(s):
            return s / 3600 * 60 / hour

        volume = sum(r["volume"] for r in rows)
        out["fleet"] = {"trucks": n, "matchFactor": n * service / out["cycle"], "steady": volume / hours(steady_s),
                        "queued": volume / hours(queued_s), "busy": busy_s / queued_s, "wait": waits / trips,
                        "hoursSteady": hours(steady_s), "hoursQueued": hours(queued_s)}
    if q.get("fuel") is not None:
        fuel = sum(r["trips"] * r["fuel"] for r in rows)
        if "fleet" in out:
            fuel += q.get("idle", 0.0) * out["fleet"]["wait"] * trips / 3600
        out["fuel"] = fuel
        if q.get("co2") is not None:
            out["co2"] = q["co2"] * fuel
    if q.get("trace") is not None:
        out["trace"] = {"haul": q["trace"], "loaded": "$any", "empty": "$any"}
    return out, warnings


def stall_words(loaded, a, b, truck):
    return (f"Kamyon {'yüklü' if loaded else 'boş dönerken'} {a} → {b} taşımasında bir eğimi çıkamıyor: tahrik "
            f"tekerleklerinin tutunması (çekiş × pay = {truck['traction'] * truck['drive']:.3f}) yuvarlanma ile eğimin "
            f"direncinden az. Çekiş katsayısını ya da tahrik payını gözden geçirin.")


def cases():
    out = []

    def q(name, inp, hauls=HAULS, op="execute", setup_=None, result=None, volumes=hp.VOLUMES, corridors=hc.CORRIDORS):
        if result is None:
            try:
                o, w = answer(inp, hauls, volumes, corridors)
                result = completed(None if op == "validate" else o, w)
            except Refused as r:
                result = r.result
        if setup_ is None:
            setup_ = hc.setup(hauls, volumes, corridors)
            road = hc.road_of(hauls, volumes, corridors)
        else:
            road = setup_["settings"].get("road")
        step = {"op": op, "input": inp, "result": result, "expect": {**UNCHANGED, "road": road}, "within": W,
                "relative": True}
        out.append(case(name, [step], setup_))
        print(name, file=sys.stderr, flush=True)

    side = {"haul": "h-2", **BASE}
    q("Yan Brükner proje kotunda, filosuz", side)
    q("Arazi yan aksın başını kapsamaz", {**side, "track": "ground"})
    q("Dört kamyon, saatte 50 dakika", {**side, "trucks": 4, "hour": 50.0})
    q("Sekiz kamyon: yükleyici darboğaz", {**side, "trucks": 8})
    q("Tek kamyon", {**side, "trucks": 1})
    q("Yakıt, rölanti ve karbon", {**side, "trucks": 4, "fuel": 0.25, "idle": 5.0, "co2": 2.68})
    q("Yakıt, filosuz ve rölantisiz", {**side, "fuel": 0.24})
    q("Az güçlü kamyon", {**side, "truck": {**TRUCK, "power": 120.0}})
    q("İniş hızı sınırı yok", {**side, "truck": {k: v for k, v in TRUCK.items() if k != "downhill"}})
    q("Yumuşak yol, %8 yuvarlanma", {**side, "rolling": 8.0, "trucks": 5})
    q("Manevrasız, büyük kasa", {**{k: v for k, v in side.items() if k != "spotting"}, "load": 22.0})
    q("Hızı izlenen taşıma", {**side, "trace": 1})
    q("Adıyla, ekonomik plan", {**side, "haul": "Yan Brükner", "objective": "cost"},
      [hc.H1, {**hc.H2, "cost": {"currency": "TL", "overhaul": 800.0, "waste": 40.0, "borrow": 60.0}, "sites": SITES2}, hp.H3])
    q("Ana Brükner, sonda döküm alanı", {"haul": "h-1", **BASE, "trucks": 6, "fuel": 0.25, "idle": 4.0, "co2": 2.68},
      [{**hc.H1, "sites": [hp.site("Döküm", "waste", 600.0, 30000.0)]}, hc.H2, hp.H3])
    # The main corridor from km 25: the terrain covers the main alignment from 13,6 m on.
    trimmed = {**ccmd.C1, "regions": [{"from": 25.0, "to": 300.0, "template": "tk-1"},
                                      {"from": 300.0, "to": 600.0, "template": "tk-2"}]}
    q("Ana Brükner arazide", {"haul": "h-1", **BASE, "track": "ground", "trucks": 6},
      [{**hc.H1, "sites": [hp.site("Döküm", "waste", 600.0, 30000.0)]}, hc.H2, hp.H3], corridors=[trimmed, ccmd.C2])
    c_gap = {**ccmd.C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"},
                                    {"from": 250.0, "to": 400.0, "template": "tk-2"}]}
    q("Kesitsiz aralıklı kübaj", {"haul": "h-1", **BASE},
      [{**hc.H1, "sites": [hp.site("Döküm", "waste", 400.0, 30000.0)]}, hc.H2, hp.H3], corridors=[c_gap, ccmd.C2])
    q("Plan kurulamaz: taşıma yok", {**side, "trucks": 4}, [hc.H1, hc.H2, hp.H3])
    q("Plan cevabı verir", {**side, "trucks": 3}, op="plan")
    q("Doğrulama uyarıları verir", side, op="validate")
    # Refusals.
    q("Kaygan: çıkamaz", {**side, "rolling": 12.0, "truck": {**TRUCK, "traction": 0.15}})
    q("Yüklü boştan hafif", {**side, "truck": {**TRUCK, "gross": 15.0}})
    q("Verim 1'den büyük", {**side, "truck": {**TRUCK, "efficiency": 1.2}})
    q("Rölanti yakıtsız", {**side, "idle": 5.0})
    q("Karbon yakıtsız", {**side, "co2": 2.68})
    q("Saat 60 dakikadan uzun", {**side, "hour": 75.0})
    q("Olmayan taşıma izlenir", {**side, "trace": 99})
    q("Yol projesi değil", side, setup_=hc.setup(work="map"), result=hc.NOT_ROAD)
    q("Olmayan Brükner", {**side, "haul": "yok"})
    q("Km'lerin dışında alan", {**side, "haul": "h-1"},
      [{**hc.H1, "sites": [hp.site("Uzak", "waste", 800.0, 30000.0)]}, hc.H2, hp.H3])
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.hauls.trucks", "commandVersion": 1,
           "title": "Taşıma filosu: kamyonun çevrimi, sefer, yükleyici, yakıt",
           "note": "scripts/fixtures/road_haul_truck_cases.py yazar; plan road_haul_plan_cases.py'nin (HiGHS), kotlar "
                   "boykesit ve arazi referanslarının, her gidiş zamanda RK4 (1 ms), yükleyici Markov zinciri; sonuçlar "
                   "boylarının 2·10⁻³'ü içinde (relative); KentOS kodu yok.",
           "setup": hc.setup(HAULS, hp.VOLUMES), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.hauls.trucks.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.hauls.trucks.json")
    else:
        path.write_text(text)
    print(f"road.hauls.trucks.json: {len(found)} durum")


if __name__ == "__main__":
    main()
