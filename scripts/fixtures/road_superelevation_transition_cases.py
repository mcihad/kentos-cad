#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.superelevations.transitions.json: the shared cases of
`road.superelevations.transitions` (plan/road.md Y34, docs/adr/0940), their expectations built without KentOS code:
the drawing and schedules are scripts/fixtures/road_superelevation_command_cases.py's (alignments mpmath at 40
digits, schedules exact fractions) with a winding alignment of three curves added here; a curve's relative gradient
scripts/fixtures/road_superelevation_physics_cases.py's on the schedule with that curve alone; the rest the ADR's
rules written here:

- the runoff by the relative gradient: L·G(L)/Δ (every change of the transition runs over L or L·n/e);
- the clothoid by comfort: v·(v²/R − g·e)/C, v = V/3.6, g = 9.80665, none below 0; by its shift √(24·p·R);
- on a clothoid the runoff is at least the longer clothoid and lies on it (share 0); without one the share given;
- rounded up to the step; the runout L·|n|/e of the outside's normal slope n; the length at full superelevation
  (ST − TS) − 2·(1 − share)·L; two curves' transitions on the tangent between them: each runout + share·L.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_superelevation_transition_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_grades  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_superelevation_check_cases as chk  # noqa: E402
import road_superelevation_command_cases as sup  # noqa: E402
import road_superelevation_physics_cases as phys  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.superelevations.transitions.json"
completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED = pcmd.UNCHANGED
G = 9.80665
NOT_ROAD = failed("not_a_road_project", "Dever yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")

# A winding alignment: a curve without clothoids, a reverse one with them, a third turning back (docs/adr/0940).
G5 = {"id": "g-5", "name": "Kıvrımlı aks", "revision": 1,
      "pis": [pcmd.acmd.pi("a", 0, 0), pcmd.acmd.pi("p1", 250, 0, 180), pcmd.acmd.pi("p2", 400, 200, 120, 35, 35),
              pcmd.acmd.pi("p3", 650, 180, 250), pcmd.acmd.pi("b", 900, 400)]}
pcmd.ALIGNMENTS.append(G5)
pcmd.ALIGNED["g-5"] = pcmd.acmd.build(pcmd.ALIGNMENTS, G5)
pcmd.REV["g-5"] = 1
D1 = scmd.D1
D5 = {"id": "d-5", "name": "Kıvrımlı aks deveri", "alignment": "g-5", "alignmentRevision": 1, "revision": 1,
      "normalLeft": -2.5, "normalRight": -2.5,
      "method": {"method": "curves", "curves": [
          {"pi": "p1", "rate": 6.0, "runoff": 40.0, "onTangent": 0.67},
          {"pi": "p2", "rate": 8.0, "runoff": 35.0, "onTangent": 0.0, "widening": 0.6},
          {"pi": "p3", "rate": 5.0, "runoff": 30.0, "onTangent": 0.6}]}}
SUPERS = (D1, scmd.D2, D5)


def round_up(v, step_):
    return math.ceil(v / step_ - 1e-9) * step_ if step_ else v


def gradient_now(d, rule, t):
    """The largest relative gradient of the curve's own transition: the schedule with that curve alone."""
    single = copy.deepcopy(d)
    single["method"] = {"method": "curves", "curves": [rule]}
    got, why = sup.build(single)
    if why:
        return None
    way, _ = chk.way_of(single, t)
    b = pcmd.ALIGNED[d["alignment"]]
    rows = phys.gradients(way, d.get("pivot", "centerline"), 0, F(float(b.length)))
    return max([max(g["left"], g["right"]) for g in rows] + [0.0])


def answer(d, t, q):
    b = pcmd.ALIGNED[d["alignment"]]
    curves = {c["pi"]: c for c in b.reports}
    rows, taken = [], []
    for rule in d["method"]["curves"]:
        c = curves.get(rule["pi"])
        if c is None:
            continue
        ts, st = float(c["stations"][0]), float(c["stations"][3])
        radius, sin, sout = float(c["radius"]), float(c["spiralIn"]), float(c["spiralOut"])
        g = gradient_now(d, rule, t)
        rg = q.get("mostRelative")
        runoff_gradient = rule["runoff"] * g / rg if (rg is not None and g is not None and g > 0) else None
        e = rule["rate"] / 100
        comfort = None
        if q.get("speed") is not None and q.get("comfort") is not None:
            v = q["speed"] / 3.6
            comfort = max(0.0, v * (v * v / radius - G * e) / q["comfort"])
        shift = math.sqrt(24 * q["shift"] * radius) if q.get("shift") is not None else None
        most = math.sqrt(24 * q["mostShift"] * radius) if q.get("mostShift") is not None else None
        spiral = max(x for x in (comfort, shift) if x is not None) if (comfort is not None or shift is not None) else None
        clothoid = sin > 0 or sout > 0
        base = max(runoff_gradient or 0.0, max(sin, sout)) if clothoid else runoff_gradient
        runoff = round_up(base, q.get("step")) if base is not None else None
        share = 0.0 if clothoid else q.get("onTangent")
        n_out = d["normalRight"] if float(c["delta"]) > 0 else d["normalLeft"]
        l_eff = runoff if runoff is not None else rule["runoff"]
        s_eff = share if share is not None else rule["onTangent"]
        row = {"pi": rule["pi"], "from": ts, "to": st, "radius": radius, "rate": rule["rate"], "spiralIn": sin,
               "spiralOut": sout, "runoffNow": rule["runoff"], "onTangentNow": rule["onTangent"]}
        for k, v in (("gradientNow", g), ("runoffGradient", runoff_gradient), ("spiralComfort", comfort),
                     ("spiralShift", shift), ("spiralMost", most), ("spiral", spiral), ("runoff", runoff),
                     ("onTangent", share)):
            if v is not None:
                row[k] = v
        row["runout"] = l_eff * abs(n_out) / rule["rate"]
        row["full"] = (st - ts) - 2 * (1 - s_eff) * l_eff
        rows.append(row)
        taken.append((l_eff, s_eff, n_out, rule["rate"]))
    order = sorted(range(len(rows)), key=lambda i: rows[i]["from"])
    rows, taken = [rows[i] for i in order], [taken[i] for i in order]
    findings = []
    for r in rows:
        if r.get("spiral") is not None:
            for ln, side in ((r["spiralIn"], "giriş"), (r["spiralOut"], "çıkış")):
                if 0 < ln < r["spiral"] - 1e-9:
                    findings.append({"kind": "spiralShort", "pi": r["pi"], "value": ln, "limit": r["spiral"],
                                     "message": f"{r['pi']} kurbunun {side} klotoidi {ln:.2f} m; konfor ve kaydırma en az "
                                                f"{r['spiral']:.2f} m ister."})
        if r.get("spiralMost") is not None:
            for ln, side in ((r["spiralIn"], "giriş"), (r["spiralOut"], "çıkış")):
                if ln > r["spiralMost"] + 1e-9:
                    findings.append({"kind": "spiralLong", "pi": r["pi"], "value": ln, "limit": r["spiralMost"],
                                     "message": f"{r['pi']} kurbunun {side} klotoidi {ln:.2f} m; en büyük kaydırma en çok "
                                                f"{r['spiralMost']:.2f} m'ye izin verir."})
        if r["full"] < -1e-9:
            findings.append({"kind": "full", "pi": r["pi"], "value": r["full"], "limit": 0.0,
                             "message": f"{r['pi']} kurbu tam devere varamıyor: giriş ve çıkış geçişleri kurpta "
                                        f"{-r['full']:.2f} m üst üste biniyor."})
    for (a, ta), (b2, tb) in zip(zip(rows, taken), list(zip(rows, taken))[1:]):
        room = b2["from"] - a["to"]
        need = sum(lr * abs(n) / rate + s * lr for lr, s, n, rate in (ta, tb))
        if need > room + 1e-9:
            findings.append({"kind": "tangent", "pi": f"{a['pi']}–{b2['pi']}", "value": need, "limit": room,
                             "message": f"{a['pi']} ile {b2['pi']} arasındaki {room:.2f} m teğete iki kurbun geçişleri "
                                        f"{need:.2f} m ister ({need - room:.2f} m eksik)."})
    warns = []
    if any(f["kind"] in ("full", "tangent") for f in findings):
        warns.append({"code": "superelevation_transition_conflict",
                      "message": f"“{d['name']}” dever çizelgesine önerilen geçişler kurulamıyor; bulgular yerlerini söyler.",
                      "path": "superelevation"})
    return {"superelevation": d["id"], "rows": rows, "findings": findings}, warns


def setup(supers=SUPERS, work="road"):
    return sup.setup(supers=supers, work=work)


def cases():
    out = []
    tk1 = scmd.TK1

    def q(name, input_, d, supers=SUPERS, op="execute"):
        result, warns = answer(d, tk1, input_)
        base = setup(supers)
        res = completed(None, warns) if op == "validate" else completed(result, warns)
        out.append(case(name, [step(op, input_, res, {**UNCHANGED, "road": base["settings"]["road"]})],
                        None if supers == SUPERS else base))

    def no(name, input_, result, base=None):
        b = base or setup()
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": b["settings"].get("road")})], base))

    bad = lambda path, words: failed("invalid_superelevation_transition", words, path)  # noqa: E731
    one = {"superelevation": "d-1", "template": "tk-1"}
    five = {"superelevation": "d-5", "template": "tk-1"}
    q("Sınırsız: yalnız şimdiki geçiş ve göreli eğim", one, D1)
    q("1:200 göreli eğimden geçiş, klotoid üstünde", {**one, "mostRelative": 0.005}, D1)
    q("1:300 göreli eğim, 5 m'ye yuvarla", {**one, "mostRelative": 1 / 300, "step": 5.0}, D1)
    q("Konfor (Shortt) 80 km/sa C 0,6", {**one, "speed": 80.0, "comfort": 0.6}, D1)
    q("Kaydırma 0,20–1,0 m", {**one, "shift": 0.2, "mostShift": 1.0}, D1)
    q("Konfor ve kaydırma birlikte: kısa klotoid bulunur", {**one, "speed": 100.0, "comfort": 0.4, "shift": 0.2}, D1)
    q("Uzun klotoid: en büyük kaydırma aşılır", {**one, "mostShift": 0.3}, D1)
    q("Deverin dengelediği hız: konfor klotoidi sıfır", {**one, "speed": 30.0, "comfort": 0.6}, D1)
    q("Kıvrımlı aks: üç kurp, sınırsız", five, D5)
    q("Kıvrımlı aks: 1:200 ve teğet payı 0,67", {**five, "mostRelative": 0.005, "onTangent": 0.67}, D5)
    q("Kıvrımlı aks: 1:400, geçişler teğete sığmaz", {**five, "mostRelative": 1 / 400, "onTangent": 0.7, "step": 1.0},
      D5)
    q("Kıvrımlı aks: hepsi birlikte", {**five, "mostRelative": 1 / 250, "speed": 70.0, "comfort": 0.6, "shift": 0.2,
                                        "mostShift": 1.0, "onTangent": 0.67, "step": 5.0}, D5)
    q("Kıvrımlı aks: 1:150 ve teğet payı 0, geçişler sığar", {**five, "mostRelative": 1 / 150, "onTangent": 0.0}, D5)
    for pivot in ("insideEdge",):
        dp = {**D5, "pivot": pivot}
        q(f"Dönme ekseni {pivot}: kenar eğimi geçişi uzatır", {**five, "mostRelative": 1 / 200}, dp,
          supers=(D1, scmd.D2, dp))
    q("Adıyla dever ve tip kesit", {"superelevation": "Kıvrımlı aks deveri", "template": "Tip 1", "mostRelative": 0.006}, D5)
    q("Plan cevabı verir, hiçbir şey yazmaz", {**five, "mostRelative": 1 / 400, "onTangent": 0.7}, D5, op="plan")
    q("Doğrulama uyarıları verir", {**five, "mostRelative": 1 / 400, "onTangent": 0.7}, D5, op="validate")
    no("Yol projesi değil", one, NOT_ROAD, setup(work="map"))
    no("Olmayan dever çizelgesi", {"superelevation": "yok", "template": "tk-1"},
       failed("unknown_superelevation", "“yok” kimlikli ya da adlı dever çizelgesi projede yok.", "superelevation"))
    no("Olmayan tip kesit", {"superelevation": "d-1", "template": "yok"},
       failed("unknown_road_template", "“yok” kimlikli ya da adlı tip kesit projede yok.", "template"))
    no("Km satırlarıyla yazılmış çizelge", {"superelevation": "d-2", "template": "tk-1"},
       bad("superelevation", "Geçiş önerisi kurplardan kurulan dever çizelgesinde olur; bu çizelge km satırlarıyla yazılmış."))
    no("Sıfır göreli eğim", {**one, "mostRelative": 0.0},
       bad("mostRelative", "En büyük göreli eğim 0 ile 1 arası olmalı (oran; 0,005 = 1:200)."))
    no("Hızsız konfor", {**one, "comfort": 0.6}, bad("speed", "Konforlu klotoid bir hızda okunur: proje hızını (speed) verin."))
    no("Sıfır konfor", {**one, "speed": 80.0, "comfort": 0.0},
       bad("comfort", "Konfor ölçütü (ivmenin değişme hızı) 0'dan büyük olmalı (m/s³)."))
    no("En büyük kaydırma en küçükten küçük", {**one, "shift": 0.5, "mostShift": 0.2},
       bad("mostShift", "En büyük kaydırma en küçükten küçük olamaz."))
    no("Teğet payı 1'den büyük", {**one, "onTangent": 1.5}, bad("onTangent", "Geçişin teğetteki payı 0 ile 1 arası olmalı."))
    no("Sıfır adım", {**one, "step": 0.0}, bad("step", "Yuvarlama adımı 0'dan büyük, en çok 100 m olmalı."))
    nl = setup()
    nl["settings"]["road"]["templates"].append(copy.deepcopy(chk.NO_LANE))
    no("Bir yanında şerit olmayan tip kesit", {"superelevation": "d-1", "template": "tk-9"},
       bad("template", "Tip kesitin iki yanında da şerit olmalı: göreli eğim şerit kenarında okunur."), nl)
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.superelevations.transitions",
            "commandVersion": 1, "title": "Geçiş öner",
            "note": "scripts/fixtures/road_superelevation_transition_cases.py yazar; çizim ve çizelgeler "
                    "road_superelevation_command_cases.py'nin (üç kurplu kıvrımlı aks burada eklenir), göreli eğim "
                    "road_superelevation_physics_cases.py'nin kurbun tek başına kurulan çizelgesinde; geçiş, klotoid, "
                    "runout ve teğet ADR 0940'ın kurallarıyla; KentOS kodu yok.",
            "setup": setup(), "cases": cases()}
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
