#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.superelevations.widen.json: the shared cases of `road.superelevations.widen`
(plan/road.md Y28.2, docs/adr/0932 §3), their expectations built without KentOS code:

- the drawing, schedules and templates are road_superelevation_command_cases.py's; the curves (PI, TS, ST, radius)
  the reference alignment's (road_alignment_cases.py, mpmath);
- each curve's widening road_widening_cases.py's (the steady turn built axle by axle at 40 digits);
- the drive the bicycle model's differential equations integrated by fourth-order Runge–Kutta every 2.5 cm along the
  alignment's own curvature (the front axle's place integrated with them), the rearmost axle's distance read against
  that path every 10 cm. KentOS drives the tractrix exactly over 25 cm chords: the two agree within 5 mm.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_widening_command_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import math
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_superelevation_command_cases as sup  # noqa: E402
import road_widening_cases as wref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.superelevations.widen.json"
completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED = pcmd.UNCHANGED
D1, D2 = scmd.D1, scmd.D2
NOT_ROAD = failed("not_a_road_project", "Dever yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
NO_LANE = {"id": "tk-9", "name": "Şeritsiz", "revision": 1,
           "left": {"components": [scmd.comp("shoulder", 1.0, -4.0)], "cut": 1.0, "fill": 1.5}}
DRIVE_WITHIN = 5e-3

# Vehicles in road.swept.query's shape; sizes in the range of the Green Book's metric vehicles, not its tables.
WB19 = {"wheelbase": 6.6, "front": 1.2, "rear": 0.0, "width": 2.59,
        "trailer": {"kingpin": -0.8, "wheelbase": 12.5, "front": 0.9, "rear": 1.0, "width": 2.59}}
SU9 = {"wheelbase": 6.1, "front": 1.2, "rear": 1.8, "width": 2.44}
BUS = {"name": "Otobüs", "wheelbase": 7.6, "front": 2.1, "rear": 2.4, "width": 2.59}
LONG = {"name": "Uydurma uzun taşıt", "wheelbase": 100.0, "front": 1.0, "rear": 1.0, "width": 2.5,
        "trailer": {"kingpin": 0.0, "wheelbase": 120.0, "front": 1.0, "rear": 1.0, "width": 2.5}}


def lanes_of(t):
    sides = [t["left"], t.get("right") or t["left"]]
    lanes = [c for sd in sides for c in sd["components"] if c["kind"] == "lane"]
    return len(lanes), sum(c["width"] for c in lanes)


def theta_at(els, s):
    e = els[0]
    for x in els:
        if float(x.station) <= s:
            e = x
    t = s - float(e.station)
    return float(e.theta0) + float(e.k0) * t + float(e.rate()) * t * t / 2


def drive(b, vehicle, frm, to):
    """The bicycle model along the alignment from far enough before the TS (twice the vehicle's length and ten metres)
    to the ST; the largest distance of the rearmost axle from the front's path (with its lead-in line)."""
    l1 = vehicle["wheelbase"]
    tr = vehicle.get("trailer")
    k, l2 = (tr["kingpin"], tr["wheelbase"]) if tr else (0.0, 0.0)
    long_ = l1 + (abs(k) + l2 if tr else 0.0)
    start = max(0.0, frm - 2 * long_ - 10)
    end = min(to, float(b.length))
    _, p0 = pcmd.aref.place_at(b.els, pcmd.aref.mp.mpf(start))
    th0 = theta_at(b.els, start)
    h = 0.025

    def deriv(s, y):
        fx, fy, psi, chi = y
        th = theta_at(b.els, s)
        dpsi = math.sin(th - psi) / l1
        if not tr:
            return np.array([math.cos(th), math.sin(th), dpsi, 0.0])
        # The kingpin, k behind the rear axle on the tractor's axis: its velocity, then the trailer's turn.
        vk = np.array([math.cos(th), math.sin(th)]) - (l1 + k) * dpsi * np.array([-math.sin(psi), math.cos(psi)])
        dchi = (vk @ np.array([-math.sin(chi), math.cos(chi)])) / l2
        return np.array([math.cos(th), math.sin(th), dpsi, dchi])

    y = np.array([float(p0[0]), float(p0[1]), th0, th0])
    s = start
    path = [(-long_, float(p0[0]) - long_ * math.cos(th0), float(p0[1]) - long_ * math.sin(th0)), (0.0, y[0], y[1])]
    tracked = []
    n = math.ceil((end - start) / h)
    for i in range(n):
        hh = min(h, end - s)
        k1 = deriv(s, y)
        k2 = deriv(s + hh / 2, y + hh / 2 * k1)
        k3 = deriv(s + hh / 2, y + hh / 2 * k2)
        k4 = deriv(s + hh, y + hh * k3)
        y = y + hh / 6 * (k1 + 2 * k2 + 2 * k3 + k4)
        s += hh
        path.append((s - start, y[0], y[1]))
        if (i + 1) % 4 == 0 or i == n - 1:
            fx, fy, psi, chi = y
            rx, ry = fx - l1 * math.cos(psi), fy - l1 * math.sin(psi)
            if tr:
                kx, ky = rx - k * math.cos(psi), ry - k * math.sin(psi)
                px, py = kx - l2 * math.cos(chi), ky - l2 * math.sin(chi)
            else:
                px, py = rx, ry
            tracked.append((s - start, px, py))
    pts = np.array([[p[1], p[2]] for p in path])
    along = np.array([p[0] for p in path])
    best = 0.0
    for d, px, py in tracked:
        lo = max(int(np.searchsorted(along, d - 2 * long_ - 1)) - 1, 0)
        hi = min(int(np.searchsorted(along, d + 1)) + 1, len(pts))
        a, bb = pts[lo:hi - 1], pts[lo + 1:hi]
        seg = bb - a
        w = np.array([px, py]) - a
        t = np.clip((w * seg).sum(1) / np.maximum((seg * seg).sum(1), 1e-300), 0, 1)
        dist = np.hypot(*(w - seg * t[:, None]).T).min()
        best = max(best, float(dist))
    return best


def answer(d, t, vehicle, clearance, track=None, speed=None, method="standard", step_=None, least=None, drive_=False):
    b = pcmd.ALIGNED[d["alignment"]]
    n, wn = lanes_of(t)
    road = {"lanes": n, "tangent": wn, "clearance": clearance, "method": method}
    if speed is not None:
        road["speed"] = speed
    wv = {"track": track if track is not None else vehicle["width"], "front": vehicle["front"],
          "wheelbase": vehicle["wheelbase"]}
    if vehicle.get("trailer"):
        wv["trailer"] = [vehicle["trailer"]["kingpin"], vehicle["trailer"]["wheelbase"]]
    rules = {c["pi"]: c for c in d["method"].get("curves", [])} if d["method"]["method"] == "curves" else {}
    rows, unturnable = [], []
    for r in b.reports:
        radius = float(r["radius"])
        if not radius > 0:
            continue
        st = [float(x) for x in r["stations"]]
        c = {"radius": radius, "vehicle": wv, "road": road}
        if step_ is not None or least is not None:
            c["round"] = {k: v for k, v in (("step", step_), ("least", least)) if v is not None}
        got = wref.reference(c)
        if "error" in got:
            unturnable.append(r["pi"])
            continue
        row = {"pi": r["pi"], "from": st[0], "to": st[3], "radius": radius}
        row.update({key: got[key] for key in ("offtracking", "track", "overhang", "allowance", "width", "widening")})
        if "rounded" in got:
            row["suggested"] = got["rounded"]
        else:
            row["suggested"] = got["widening"] if got["widening"] > 0 else 0.0
        row["current"] = float(rules.get(r["pi"], {}).get("widening", 0.0))
        if drive_:
            row["driven"] = drive(b, vehicle, st[0], st[3])
        rows.append(row)
    warns = []
    if unturnable:
        warns.append({"code": "widening_unturnable",
                      "message": f"Taşıt {', '.join(unturnable)} kurbunda kararlı dönemez: yarıçap dingil aralıklarının "
                                 f"karelerinin toplamının kökünden küçük.",
                      "path": "vehicle"})
    if d.get("alignmentRevision") != pcmd.REV[d["alignment"]]:
        warns.append(sup.stale_warning(d))
    out = {"superelevation": d["id"], "lanes": n, "tangent": wn, "method": method, "curves": rows,
           "unturnable": unturnable}
    return out, warns


def cases():
    out = []
    tk1, tk2 = scmd.TK1, scmd.TK2

    def q(name, input_, d, t, base=None, within=None, **kw):
        result, warns = answer(d, t, **kw)
        b = base or sup.setup()
        c = case(name, [step("execute", input_, completed(result, warns), {**UNCHANGED, "road": b["settings"]["road"]})],
                 base)
        # The widening is read at 40 digits and KentOS's in doubles: equal within 1e-9 m; the drive within 5 mm.
        c["steps"][0]["within"] = within or 1e-9
        out.append(c)

    def no(name, input_, result, base=None):
        b = base or sup.setup()
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": b["settings"].get("road")})], base))

    bad = lambda path, words: failed("invalid_widening", words, path)  # noqa: E731
    base_in = {"superelevation": "d-1", "template": "tk-1", "vehicle": WB19, "clearance": 0.9}
    q("Çekici ve römork, 80 km/sa", {**base_in, "speed": 80.0}, D1, tk1, vehicle=WB19, clearance=0.9, speed=80.0)
    q("Basit yöntem: N(U + C)", {**base_in, "speed": 80.0, "method": "simple"}, D1, tk1, vehicle=WB19, clearance=0.9,
      speed=80.0, method="simple")
    q("Hızsız: sürüş payı yok", base_in, D1, tk1, vehicle=WB19, clearance=0.9)
    q("İz genişliği ayrı verilir", {**base_in, "track": 2.44, "speed": 80.0}, D1, tk1, vehicle=WB19, clearance=0.9,
      track=2.44, speed=80.0)
    q("0,1 m'ye yukarı yuvarlanır", {**base_in, "speed": 80.0, "step": 0.1}, D1, tk1, vehicle=WB19, clearance=0.9,
      speed=80.0, step_=0.1)
    q("0,25 m'ye yuvarlanır, en az 0,6 m", {**base_in, "speed": 80.0, "step": 0.25, "least": 0.6}, D1, tk1,
      vehicle=WB19, clearance=0.9, speed=80.0, step_=0.25, least=0.6)
    q("En az genişletmenin altında öneri yok", {**base_in, "clearance": 0.6, "least": 2.0}, D1, tk1, vehicle=WB19,
      clearance=0.6, least=2.0)
    q("Tek gövdeli kamyon", {**base_in, "vehicle": SU9, "clearance": 0.75, "speed": 60.0}, D1, tk1, vehicle=SU9,
      clearance=0.75, speed=60.0)
    q("Otobüs, adıyla", {**base_in, "vehicle": BUS, "clearance": 0.75, "speed": 50.0}, D1, tk1, vehicle=BUS,
      clearance=0.75, speed=50.0)
    q("Çizelge ve tip kesit adıyla, büyük küçük harf ayrımsız",
      {**base_in, "superelevation": "Ana aks deveri", "template": "TIP 1", "speed": 70.0}, D1, tk1,
      vehicle=WB19, clearance=0.9, speed=70.0)
    q("Üç şeritli tip kesit", {**base_in, "template": "tk-2", "speed": 80.0}, D1, tk2, vehicle=WB19, clearance=0.9,
      speed=80.0)
    q("Sürüş: çekici ve römork kurpta", {**base_in, "speed": 80.0, "drive": True}, D1, tk1, within=DRIVE_WITHIN,
      vehicle=WB19, clearance=0.9, speed=80.0, drive_=True)
    q("Sürüş: tek gövdeli kamyon", {**base_in, "vehicle": SU9, "clearance": 0.75, "drive": True}, D1, tk1,
      within=DRIVE_WITHIN, vehicle=SU9, clearance=0.75, drive_=True)
    q("Kurpsuz aks: öneri yok, çizelge eskimiş", {**base_in, "superelevation": "d-2", "speed": 60.0}, D2, tk1,
      vehicle=WB19, clearance=0.9, speed=60.0)
    q("Kararlı dönemeyen taşıt", {**base_in, "vehicle": LONG, "speed": 40.0}, D1, tk1, vehicle=LONG, clearance=0.9,
      speed=40.0)
    r, w = answer(D1, tk1, vehicle=WB19, clearance=0.9, speed=90.0, step_=0.1)
    plan_in = {**base_in, "speed": 90.0, "step": 0.1}
    out.append(case("Plan cevabı verir, hiçbir şey yazmaz",
                    [{**step("plan", plan_in, completed(r, w), {**UNCHANGED, "road": sup.setup()["settings"]["road"]}),
                      "within": 1e-9}]))
    r, w = answer(D1, tk1, vehicle=LONG, clearance=0.9)
    out.append(case("Doğrulama uyarıları verir",
                    [step("validate", {**base_in, "vehicle": LONG}, completed(None, w),
                          {**UNCHANGED, "road": sup.setup()["settings"]["road"]})]))
    no("Yol projesi değil", base_in, NOT_ROAD, sup.setup(work="map"))
    no("Olmayan dever çizelgesi", {**base_in, "superelevation": "yok"},
       failed("unknown_superelevation", "“yok” kimlikli ya da adlı dever çizelgesi projede yok.", "superelevation"))
    no("Olmayan tip kesit", {**base_in, "template": "yok"},
       failed("unknown_road_template", "“yok” kimlikli ya da adlı tip kesit projede yok.", "template"))
    no("Sıfır dingil aralığı", {**base_in, "vehicle": {**SU9, "wheelbase": 0.0}},
       bad("vehicle", "Taşıtın dingil aralığı ve genişliği sıfırdan büyük, çıkıntıları eksi olmayan sonlu sayılar olmalı "
                      "(römorkun da)."))
    no("Sıfır iz genişliği", {**base_in, "track": 0.0}, bad("track", "İz genişliği (track) sıfırdan büyük olmalı (m)."))
    no("Eksi açıklık", {**base_in, "clearance": -0.1},
       bad("clearance", "Yanal açıklık (clearance) eksi olmayan bir sayı olmalı (m)."))
    no("Sıfır hız", {**base_in, "speed": 0.0}, bad("speed", "Proje hızı sıfırdan büyük olmalı (km/sa)."))
    no("Sıfır yuvarlama adımı", {**base_in, "step": 0.0},
       bad("step", "Yuvarlama adımı (step) sıfırdan büyük olmalı (m)."))
    no("Eksi en az genişletme", {**base_in, "least": -1.0},
       bad("least", "En az genişletme (least) eksi olmayan bir sayı olmalı (m)."))
    nl = sup.setup()
    nl["settings"]["road"]["templates"].append(copy.deepcopy(NO_LANE))
    no("Şeritsiz tip kesit", {**base_in, "template": "tk-9"},
       bad("template", "Tip kesitte şerit yok: genişletme şeritlerin sayısından ve genişliğinden hesaplanır."), nl)
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.superelevations.widen", "commandVersion": 1,
            "title": "Genişletme öner",
            "note": "scripts/fixtures/road_widening_command_cases.py yazar; çizim ve çizelgeler "
                    "road_superelevation_command_cases.py'nin, genişletme road_widening_cases.py'nin (kararlı dönüş "
                    "dingil dingil, 40 basamak), sürüş bisiklet modelinin Runge–Kutta tümlevi; KentOS kodu yok.",
            "setup": sup.setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
