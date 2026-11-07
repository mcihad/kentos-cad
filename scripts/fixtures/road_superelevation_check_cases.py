#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.superelevations.check.json: the shared cases of `road.superelevations.check`
(plan/road.md Y27.2, docs/adr/0931 §2), their expectations built without KentOS code: the drawing and schedules are
scripts/fixtures/road_superelevation_command_cases.py's (alignments mpmath at 40 digits, schedules exact fractions);
the relative gradients scripts/fixtures/road_superelevation_physics_cases.py's (a parabola through each stretch); the
side friction the point-mass balance at 40 digits on the reference alignment's curvature.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_superelevation_check_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_grades  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_superelevation_command_cases as sup  # noqa: E402
import road_superelevation_physics_cases as phys  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.superelevations.check.json"
completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED = pcmd.UNCHANGED
D1, D2 = scmd.D1, scmd.D2
NOT_ROAD = failed("not_a_road_project", "Dever yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
# A template whose left side has no lane: the relative gradient has no edge there.
NO_LANE = {"id": "tk-9", "name": "Şeritsiz", "revision": 1,
           "left": {"components": [scmd.comp("shoulder", 1.0, -4.0)], "cut": 1.0, "fill": 1.5},
           "right": {"components": scmd.ROADWAY, "cut": 1.0, "fill": 1.5}}


def lanes(t):
    side = lambda sd: sum(c["width"] for c in sd["components"] if c["kind"] == "lane")  # noqa: E731
    return side(t["left"]), side(t.get("right") or t["left"])


def way_of(d, t):
    (sch, _), _ = sup.build(d)
    wl, wr = lanes(t)
    slope = lambda pts, n: [[float(a), float(b)] for a, b in pts] if pts else [[0.0, n]]  # noqa: E731
    width = lambda pts, w: [[float(a), w + float(b)] for a, b in pts] if pts else [[0.0, w]]  # noqa: E731
    return {"leftSlope": slope(sch["left"], d["normalLeft"]), "rightSlope": slope(sch["right"], d["normalRight"]),
            "leftWidth": width(sch["leftWidening"], wl), "rightWidth": width(sch["rightWidening"], wr)}, sch


def curvature(els, s):
    e, _ = aref.place_at(els, mp.mpf(s))
    return e.curvature(mp.mpf(s) - e.station)


def frictions(d, sch, speed, step_):
    b = pcmd.ALIGNED[d["alignment"]]
    out = []
    for r in b.reports:
        st = [float(x) for x in r["stations"]]
        frm, to = st[0], st[3]
        if not to > frm:
            continue
        at = list(st)
        for key in ("left", "right"):
            at += [float(p[0]) for p in sch[key] if frm < float(p[0]) < to]
        n = math.ceil((to - frm) / max(step_, 1e-3))
        at += [min(frm + i * step_, to) for i in range(n + 1)]
        at.sort()
        uniq = []
        for x in at:
            if not uniq or abs(x - uniq[-1]) >= 1e-9:
                uniq.append(x)
        best = None
        for s in uniq:
            k = curvature(b.els, s)
            if k == 0:
                k = curvature(b.els, min(s + 1e-6, to))
            if k == 0:
                continue
            sl = phys.along([[a, v] for a, v in sch["left"]], F(s)) if sch["left"] else F(d["normalLeft"])
            sr = phys.along([[a, v] for a, v in sch["right"]], F(s)) if sch["right"] else F(d["normalRight"])
            e = phys.toward(sl, sr, k > 0)
            radius = 1 / abs(k)
            f = phys.friction(speed, radius, e)
            if best is None or f > best["friction"] + 1e-12:
                best = {"pi": r["pi"], "from": frm, "to": to, "friction": f, "at": s, "radius": float(radius), "e": e}
        if best:
            out.append(best)
    return out


def answer(d, t, speed=None, most_friction=None, most_relative=None, step_=1.0):
    way, sch = way_of(d, t)
    b = pcmd.ALIGNED[d["alignment"]]
    pivot = d.get("pivot", "centerline")
    gradients = phys.gradients(way, pivot, 0, F(float(b.length)))
    curves = frictions(d, sch, speed, step_) if speed else []
    findings = []
    if most_relative is not None:
        for g in gradients:
            for value, side in ((g["left"], "sol"), (g["right"], "sağ")):
                if value > most_relative + 1e-12:
                    findings.append({"kind": "relative", "from": g["from"], "to": g["to"], "value": value,
                                     "limit": most_relative,
                                     "message": f"{g['from']:.2f} – {g['to']:.2f} m: {side} kenarın göreli eğimi "
                                                f"1:{1 / value:.0f}, en dik 1:{1 / most_relative:.0f}."})
    if most_friction is not None:
        for c in curves:
            if c["friction"] > most_friction + 1e-12:
                findings.append({"kind": "friction", "from": c["from"], "to": c["to"], "value": c["friction"],
                                 "limit": most_friction,
                                 "message": f"{c['pi']} kurbu {speed:.0f} km/sa'te yanal sürtünme {c['friction']:.3f} ister "
                                            f"(km {c['at']:.2f}, R {c['radius']:.1f} m, dever %{100 * c['e']:.2f}), "
                                            f"en çok {most_friction:.3f}."})
    findings.sort(key=lambda f: (f["from"], f["kind"]))
    warns = []
    if findings:
        warns.append({"code": "superelevation_limit_broken",
                      "message": f"“{d['name']}” dever çizelgesinde {len(findings)} yer verilen sınırları aşıyor.",
                      "path": "superelevation"})
    out = {"superelevation": d["id"], "pivot": pivot, "gradients": gradients, "curves": curves, "findings": findings,
           "step": step_}
    return out, warns


def cases():
    out = []
    tk1, tk2 = scmd.TK1, scmd.TK2

    def q(name, input_, d, t, supers=(D1, D2), templates=None, speeds=None, **kw):
        result, warns = answer(d, t, **kw)
        base = sup.setup(supers=supers)
        if templates:
            base["settings"]["road"]["templates"] = copy.deepcopy(templates)
        if speeds:
            # Docs/adr/0986: g-1's design speed and p1's own (docs/adr/0985).
            g1 = next(a for a in base["settings"]["road"]["alignments"] if a["id"] == "g-1")
            if "design" in speeds:
                g1["designSpeed"] = speeds["design"]
            if "p1" in speeds:
                next(p for p in g1["pis"] if p["id"] == "p1")["speed"] = speeds["p1"]
        out.append(case(name, [step("execute", input_, completed(result, warns),
                                    {**UNCHANGED, "road": base["settings"]["road"]})],
                        None if (supers == (D1, D2) and not templates and not speeds) else base))

    def no(name, input_, result, base=None):
        b = base or sup.setup()
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": b["settings"].get("road")})], base))

    bad = lambda path, words: failed("invalid_superelevation_check", words, path)  # noqa: E731
    q("Göreli eğim: eksen etrafında, sınırsız", {"superelevation": "d-1", "template": "tk-1"}, D1, tk1)
    q("80 km/sa'te kurpların sürtünmesi", {"superelevation": "d-1", "template": "tk-1", "speed": 80.0}, D1, tk1, speed=80.0)
    q("Sınırlar aşılır: 1:300 ve sürtünme 0,05",
      {"superelevation": "Ana aks deveri", "template": "Tip 1", "speed": 80.0, "mostFriction": 0.05, "mostRelative": 1 / 300},
      D1, tk1, speed=80.0, most_friction=0.05, most_relative=1 / 300)
    q("Sınırlar tutar", {"superelevation": "d-1", "template": "tk-1", "speed": 50.0, "mostFriction": 0.2, "mostRelative": 0.02},
      D1, tk1, speed=50.0, most_friction=0.2, most_relative=0.02)
    for pivot in ("insideEdge", "outsideEdge", "leftEdge", "rightEdge"):
        dp = {**D1, "pivot": pivot}
        q(f"Dönme ekseni {pivot}", {"superelevation": "d-1", "template": "tk-1", "mostRelative": 1 / 250}, dp, tk1,
          supers=(dp, D2), most_relative=1 / 250)
    q("Km satırlarından dever, asimetrik tip kesit", {"superelevation": "d-2", "template": "tk-2", "speed": 60.0},
      D2, tk2, speed=60.0)
    q("Klotoid 5 m'de bir okunur", {"superelevation": "d-1", "template": "tk-1", "speed": 90.0, "step": 5.0},
      D1, tk1, speed=90.0, step_=5.0)
    r, w = answer(D1, tk1, speed=70.0, most_friction=0.06)
    out.append(case("Plan cevabı verir, hiçbir şey yazmaz", [step(
        "plan", {"superelevation": "d-1", "template": "tk-1", "speed": 70.0, "mostFriction": 0.06}, completed(r, w),
        {**UNCHANGED, "road": sup.setup()["settings"]["road"]})]))
    out.append(case("Doğrulama uyarıları verir", [step(
        "validate", {"superelevation": "d-1", "template": "tk-1", "speed": 70.0, "mostFriction": 0.06}, completed(None, w),
        {**UNCHANGED, "road": sup.setup()["settings"]["road"]})]))
    no("Yol projesi değil", {"superelevation": "d-1", "template": "tk-1"}, NOT_ROAD, sup.setup(work="map"))
    no("Olmayan dever çizelgesi", {"superelevation": "yok", "template": "tk-1"},
       failed("unknown_superelevation", "“yok” kimlikli ya da adlı dever çizelgesi projede yok.", "superelevation"))
    no("Olmayan tip kesit", {"superelevation": "d-1", "template": "yok"},
       failed("unknown_road_template", "“yok” kimlikli ya da adlı tip kesit projede yok.", "template"))
    no("Sıfır hız", {"superelevation": "d-1", "template": "tk-1", "speed": 0.0}, bad("speed", "Proje hızı 0'dan büyük olmalı (km/sa)."))
    no("Hızsız sürtünme", {"superelevation": "d-1", "template": "tk-1", "mostFriction": 0.14},
       bad("speed", "Yanal sürtünme bir hızda okunur: proje hızını (speed) verin ya da aksa proje hızı yazın."))
    # Docs/adr/0986: without `speed` each curve is read at its own design speed, its PI's else its alignment's; a
    # `speed` given stands for every curve.
    q("Hız verilmez: kurp aksın proje hızıyla", {"superelevation": "d-1", "template": "tk-1", "mostFriction": 0.06},
      D1, tk1, speeds={"design": 70.0}, speed=70.0, most_friction=0.06)
    q("Kurbun kendi hızı aksınkini geçer", {"superelevation": "d-1", "template": "tk-1", "mostFriction": 0.06},
      D1, tk1, speeds={"design": 90.0, "p1": 50.0}, speed=50.0, most_friction=0.06)
    q("Verilen hız kurbunkini geçer", {"superelevation": "d-1", "template": "tk-1", "speed": 80.0, "mostFriction": 0.06},
      D1, tk1, speeds={"p1": 50.0}, speed=80.0, most_friction=0.06)
    no("Bir ya da büyük sürtünme", {"superelevation": "d-1", "template": "tk-1", "speed": 60.0, "mostFriction": 1.0},
       bad("mostFriction", "En büyük yanal sürtünme 0 ile 1 arası olmalı."))
    no("Sıfır göreli eğim", {"superelevation": "d-1", "template": "tk-1", "mostRelative": 0.0},
       bad("mostRelative", "En büyük göreli eğim 0 ile 1 arası olmalı (oran; 0,005 = 1:200)."))
    no("Sıfır adım", {"superelevation": "d-1", "template": "tk-1", "speed": 60.0, "step": 0.0},
       bad("step", "Klotoidin okunma adımı 0'dan büyük, en çok 100 m olmalı."))
    nl = sup.setup()
    nl["settings"]["road"]["templates"].append(copy.deepcopy(NO_LANE))
    no("Bir yanında şerit olmayan tip kesit", {"superelevation": "d-1", "template": "tk-9"},
       bad("template", "Tip kesitin iki yanında da şerit olmalı: göreli eğim şerit kenarında okunur."), nl)
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.superelevations.check", "commandVersion": 1,
            "title": "Deveri denetle",
            "note": "scripts/fixtures/road_superelevation_check_cases.py yazar; çizim ve çizelgeler "
                    "road_superelevation_command_cases.py'nin, göreli eğim road_superelevation_physics_cases.py'nin "
                    "(her aralıkta parabol), sürtünme nokta kütle dengesinin 40 basamakta referans aksın eğriliğiyle; "
                    "KentOS kodu yok.",
            "setup": sup.setup(), "cases": cases()}
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text("utf-8") != text:
            sys.exit(f"güncel değil: {OUT.relative_to(ROOT)}")
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")


if __name__ == "__main__":
    main()
