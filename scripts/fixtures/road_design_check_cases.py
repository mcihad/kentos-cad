#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.alignments.check.json and road.profiles.check.json: the shared cases of KentOS
Yol's design checks (plan/road.md Y21, docs/adr/0922), their expectations built here without KentOS code. An
alignment is road_alignment_cases.py's (mpmath at 40 digits): its curves' radii, circular lengths, clothoids and TS,
SC, CS, ST stations, the tangents between them; a profile is road_vertical_cases.py's (exact fractions, circles in
mpmath): its grades, curves and K. The rules and their limits are the cases' own: KentOS gives none.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_design_check_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_alignment_command_cases as acmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
MEET = 1e-3
UNCHANGED = acmd.UNCHANGED
failed = acmd.failed
pi = acmd.pi

# A reverse pair (left, then right) with clothoids on the first; a broken-back pair (left, left); a joined compound
# pair (left 45°, left 45°, the tangent lengths meeting exactly); a kink.
T1, T2 = 100 * math.tan(math.radians(22.5)), 250 * math.tan(math.radians(22.5))
P1 = (200.0, 0.0)
P2 = (P1[0] + (T1 + T2) * math.cos(math.radians(45)), P1[1] + (T1 + T2) * math.sin(math.radians(45)))
ALIGNMENTS = [
    {"id": "g-1", "name": "Ters", "revision": 1,
     "pis": [pi("a", 0, 0), pi("p1", 300, 0, 200, 40, 40), pi("p2", 450, 150, 150), pi("b", 800, 150)]},
    {"id": "g-2", "name": "Kırık sırt", "revision": 1,
     "pis": [pi("a", 0, 0), pi("p1", 300, 0, 300), pi("p2", 500, 100, 300), pi("b", 700, 300)]},
    {"id": "g-3", "name": "Bitişik", "revision": 1,
     "pis": [pi("a", 0, 0), pi("p1", P1[0], P1[1], 100), pi("p2", P2[0], P2[1], 250), pi("b", P2[0], P2[1] + 300)]},
    {"id": "g-4", "name": "Kırıklı", "revision": 1, "pis": [pi("a", 0, 0), pi("p1", 200, 0), pi("b", 400, 50)]},
    {"id": "g-5", "name": "Paralel", "revision": 1, "offsetOf": {"alignment": "g-1", "offset": 3.5}},
]


def pv(id_, s, z, **curve):
    return pcmd.pv(id_, s, z, **curve)


PROFILES = [
    {"id": "b-1", "name": "Proje kotu", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
     "pvis": [pv("a", 0, 100), pv("v1", 150, 106, length=80), pv("v2", 350, 98, length=120), pv("b", 700, 104)]},
    {"id": "b-2", "name": "Daire", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
     "pvis": [pv("a", 0, 100), pv("v1", 300, 112, radius=2000), pv("b", 600, 103)]},
    {"id": "b-3", "name": "Eski", "alignment": "g-2", "revision": 1,
     "pvis": [pv("a", 0, 100), pv("v1", 200, 104, length=60), pv("b", 400, 100)]},
]


def setup(work="road"):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work == "road":
        settings["road"] = road()
    return {"format": "kentos.document", "version": 2, "name": "Yol tasarım denetimi", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": copy.deepcopy(pcmd.LAYERS[:1]), "activeLayer": "0",
            "entities": [], "uids": [], "styles": {"items": [], "categories": []}}


def road():
    return {"alignments": copy.deepcopy(ALIGNMENTS), "profiles": copy.deepcopy(PROFILES)}


def fmt(v):
    v = float(v)
    return str(int(v)) if v.is_integer() else repr(v)


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


# ── road.alignments.check ─────────────────────────────────────────────────────────────────────────────────────────
A_RULES = (("leastRadius", "radius"), ("leastArc", "arc"), ("leastSpiral", "spiral"), ("leastTangent", "tangent"),
           ("leastReverse", "reverse"), ("leastBrokenBack", "brokenBack"))
A_ORDER = ["radius", "arc", "spiral", "tangent", "reverse", "brokenBack", "radiusRatio", "kink"]


def a_words(rule, of, value, limit):
    a, b = of[0], (of[1] if len(of) > 1 else "")
    if limit is None:
        return f"{a} PI'sinde yön {value:.4f}° dönüyor, kurp yok."
    return {
        "radius": f"{a} kurbunun yarıçapı {value:.3f} m, en az {fmt(limit)} m.",
        "arc": f"{a} kurbunun daire boyu {value:.3f} m, en az {fmt(limit)} m.",
        "spiral": f"{a} kurbunun klotoidi {value:.3f} m, en az {fmt(limit)} m.",
        "tangent": f"{a} ile {b} arasındaki aliyman {value:.3f} m, en az {fmt(limit)} m.",
        "reverse": f"{a} ile {b} ters dönüyor; aradaki aliyman {value:.3f} m, en az {fmt(limit)} m.",
        "brokenBack": f"{a} ile {b} aynı yöne dönüyor; aradaki aliyman {value:.3f} m, en az {fmt(limit)} m.",
        "radiusRatio": f"{a} ile {b} bitişik; yarıçapların oranı {value:.3f}, en çok {fmt(limit)}.",
    }[rule]


def a_answer(inp):
    name = inp["alignment"]
    d = next((a for a in ALIGNMENTS if a["id"] == name), None) or next(
        (a for a in ALIGNMENTS if a["name"].lower() == name.lower()), None)
    if d is None:
        raise Refused("unknown_alignment", f"“{name}” kimlikli ya da adlı aks projede yok.", "alignment")
    if "offsetOf" in d:
        raise Refused("invalid_design_check", f"“{d['name']}” paralel aks; kendi PI'si yok, kaynağını denetleyin.",
                      "alignment")
    if not any(k in inp for k, _ in A_RULES) and "mostRadiusRatio" not in inp and not inp.get("kinks"):
        raise Refused("invalid_design_check", "Denetlenecek bir kural verin: tasarım standardının değerleri "
                                              "kullanıcınındır.", "alignment")
    for k, _ in A_RULES:
        if k in inp and not inp[k] > 0:
            raise Refused("invalid_design_check", f"{k} 0'dan büyük olmalı.", k)
    if "mostRadiusRatio" in inp and not inp["mostRadiusRatio"] >= 1:
        raise Refused("invalid_design_check", "mostRadiusRatio 1'den küçük olamaz: büyük yarıçap küçüğe bölünür.",
                      "mostRadiusRatio")
    els, _, reports, _ = aref.build([acmd.full(p) for p in d["pis"]])
    found = []

    def add(rule, of, frm, to, value, limit):
        found.append({"rule": rule, "of": of, "from": float(frm), "to": float(to), "value": float(value),
                      "limit": limit, "message": a_words(rule, of, float(value), limit)})

    for c in reports:
        st = c["stations"]
        if "leastRadius" in inp and c["radius"] < inp["leastRadius"]:
            add("radius", [c["pi"]], st[0], st[3], c["radius"], inp["leastRadius"])
        if "leastArc" in inp and c["arc"] < inp["leastArc"]:
            add("arc", [c["pi"]], st[1], st[2], c["arc"], inp["leastArc"])
        for length, frm, to in ((c["spiralIn"], st[0], st[1]), (c["spiralOut"], st[2], st[3])):
            if length > 0 and "leastSpiral" in inp and length < inp["leastSpiral"]:
                add("spiral", [c["pi"]], frm, to, length, inp["leastSpiral"])
    pairs = []
    for a, b in zip(reports, reports[1:]):
        tangent = max(b["stations"][0] - a["stations"][3], 0.0)
        reverse = (a["delta"] > 0) != (b["delta"] > 0)
        joined = tangent < MEET
        pairs.append({"between": [a["pi"], b["pi"]], "reverse": reverse, "joined": joined, "tangent": tangent,
                      "from": a["stations"][3], "to": b["stations"][0]})
        of = [a["pi"], b["pi"]]
        if not joined and "leastTangent" in inp and tangent < inp["leastTangent"]:
            add("tangent", of, a["stations"][3], b["stations"][0], tangent, inp["leastTangent"])
        if reverse and "leastReverse" in inp and tangent < inp["leastReverse"]:
            add("reverse", of, a["stations"][3], b["stations"][0], tangent, inp["leastReverse"])
        if not reverse and not joined and "leastBrokenBack" in inp and tangent < inp["leastBrokenBack"]:
            add("brokenBack", of, a["stations"][3], b["stations"][0], tangent, inp["leastBrokenBack"])
        if joined and "mostRadiusRatio" in inp:
            ratio = max(a["radius"], b["radius"]) / min(a["radius"], b["radius"])
            if ratio > inp["mostRadiusRatio"]:
                add("radiusRatio", of, a["stations"][0], b["stations"][3], ratio, inp["mostRadiusRatio"])
    if inp.get("kinks"):
        P = d["pis"]
        for i in range(1, len(P) - 1):
            p = P[i]
            if p.get("radius", 0) > 0:
                continue
            a, b = P[i - 1], P[i + 1]
            turn = aref.wrap(mp.atan2(b["y"] - p["y"], b["x"] - p["x"]) - mp.atan2(p["y"] - a["y"], p["x"] - a["x"]))
            if turn == 0:
                continue
            at = next(float(e.station) for e in els if abs(e.start[0] - p["x"]) + abs(e.start[1] - p["y"]) < 1e-9)
            found.append({"rule": "kink", "of": [p["id"]], "from": at, "to": at, "value": float(abs(turn) * 180 / mp.pi),
                          "message": a_words("kink", [p["id"]], float(abs(turn) * 180 / mp.pi), None)})
    found.sort(key=lambda f: (f["from"], A_ORDER.index(f["rule"])))
    warns = []
    if found:
        warns.append({"code": "design_rule_broken",
                      "message": f"“{d['name']}” aksında {len(found)} yer verilen tasarım kurallarına uymuyor.",
                      "path": "alignment"})
    return {"alignment": d["id"], "pairs": pairs, "findings": found}, warns


# ── road.profiles.check ───────────────────────────────────────────────────────────────────────────────────────────
P_RULES = ("mostGrade", "leastGrade", "leastCrestK", "leastSagK", "leastCurve", "leastGradeLength")
P_ORDER = ["grade", "leastGrade", "crestK", "sagK", "curve", "gradeLength"]


def p_words(rule, of, value, limit):
    a, b = of[0], (of[1] if len(of) > 1 else "")
    return {
        "grade": f"{a}–{b} eğimi %{100 * value:.3f}, en çok %{100 * limit:.3f}.",
        "leastGrade": f"{a}–{b} eğimi %{100 * value:.3f}, en az %{100 * limit:.3f}.",
        "crestK": f"{a} tepe kurbunun K'si {value:.3f}, en az {fmt(limit)}.",
        "sagK": f"{a} çukur kurbunun K'si {value:.3f}, en az {fmt(limit)}.",
        "curve": f"{a} düşey kurbu {value:.3f} m, en az {fmt(limit)} m.",
        "gradeLength": f"{a} ile {b} kurpları arasındaki düz eğim {value:.3f} m, en az {fmt(limit)} m.",
    }[rule]


def p_answer(inp):
    name = inp["profile"]
    d = next((p for p in PROFILES if p["id"] == name), None) or next(
        (p for p in PROFILES if p["name"].lower() == name.lower()), None)
    if d is None:
        raise Refused("unknown_road_profile", f"“{name}” kimlikli ya da adlı boykesit projede yok.", "profile")
    if not any(k in inp for k in P_RULES):
        raise Refused("invalid_design_check", "Denetlenecek bir kural verin: tasarım standardının değerleri "
                                              "kullanıcınındır.", "profile")
    for k in P_RULES:
        if k in inp and not inp[k] > 0:
            raise Refused("invalid_design_check", f"{k} 0'dan büyük olmalı.", k)
    pvis = d["pvis"]
    vp = pcmd.to_vref(pvis)
    curves, _, _, _ = pcmd.vref.solve(vp)
    rows = pcmd.vref.report(vp, curves)
    found = []

    def add(rule, of, frm, to, value, limit):
        found.append({"rule": rule, "of": of, "from": float(frm), "to": float(to), "value": float(value),
                      "limit": limit, "message": p_words(rule, of, float(value), limit)})

    for a, b in zip(pvis, pvis[1:]):
        g = abs((b["elevation"] - a["elevation"]) / (b["station"] - a["station"]))
        if "mostGrade" in inp and g > inp["mostGrade"]:
            add("grade", [a["id"], b["id"]], a["station"], b["station"], g, inp["mostGrade"])
        if "leastGrade" in inp and g < inp["leastGrade"]:
            add("leastGrade", [a["id"], b["id"]], a["station"], b["station"], g, inp["leastGrade"])
    for r in rows:
        frm, to = r["start"][0], r["end"][0]
        key = "leastCrestK" if r["crest"] else "leastSagK"
        if r["k"] is not None and key in inp and r["k"] < inp[key]:
            add("crestK" if r["crest"] else "sagK", [r["pvi"]], frm, to, r["k"], inp[key])
        if "leastCurve" in inp and to - frm < inp["leastCurve"]:
            add("curve", [r["pvi"]], frm, to, to - frm, inp["leastCurve"])
    if "leastGradeLength" in inp:
        for a, b in zip(rows, rows[1:]):
            length = max(b["start"][0] - a["end"][0], 0.0)
            if length < inp["leastGradeLength"]:
                add("gradeLength", [a["pvi"], b["pvi"]], a["end"][0], b["start"][0], length, inp["leastGradeLength"])
    found.sort(key=lambda f: (f["from"], P_ORDER.index(f["rule"])))
    warns = []
    rev = next(a["revision"] for a in ALIGNMENTS if a["id"] == d["alignment"])
    if rev != d.get("alignmentRevision", 0):
        warns.append({"code": "profile_stale",
                      "message": f"“{d['name']}” boykesitinin aksı değişti (boykesit {d.get('alignmentRevision', 0)}. "
                                 f"revizyona göre kurulu, aks şimdi {rev}.); PVI'ları denetleyip aksa yeniden bağlayın.",
                      "path": "profile"})
    if found:
        warns.append({"code": "design_rule_broken",
                      "message": f"“{d['name']}” boykesitinde {len(found)} yer verilen tasarım kurallarına uymuyor.",
                      "path": "profile"})
    return {"profile": d["id"], "findings": found}, warns


def cases_of(answer):
    out = []
    unchanged = {**UNCHANGED, "road": road()}

    def q(name, inp, result=None, setup_=None):
        if result is None:
            try:
                o, w = answer(inp)
                result = {"status": "completed", "output": o, "warnings": w}
            except Refused as r:
                result = r.result
        expect = {**UNCHANGED, "road": None} if setup_ else unchanged
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": expect, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)
    return out, q


def alignment_cases():
    out, q = cases_of(a_answer)
    q("Ters kurplar arasında kısa aliyman", {"alignment": "g-1", "leastReverse": 60.0})
    q("Ters kurplar yeterince ayrık", {"alignment": "g-1", "leastReverse": 30.0})
    q("Yarıçap ve klotoid", {"alignment": "g-1", "leastRadius": 180.0, "leastSpiral": 50.0})
    q("Daire boyu", {"alignment": "Ters", "leastArc": 120.0})
    q("Aliyman kuralı", {"alignment": "g-1", "leastTangent": 100.0})
    q("Kırık sırt", {"alignment": "g-2", "leastBrokenBack": 150.0, "leastReverse": 300.0})
    q("Kırık sırt yeterince ayrık", {"alignment": "g-2", "leastBrokenBack": 50.0})
    q("Bitişik kurplar, oran", {"alignment": "g-3", "mostRadiusRatio": 2.0, "leastTangent": 10.0,
                                "leastBrokenBack": 10.0})
    q("Bitişik kurplar, oran tutar", {"alignment": "Bitişik", "mostRadiusRatio": 3.0})
    q("Kurpsuz kırık", {"alignment": "g-4", "kinks": True})
    q("Kırık sorulmadı", {"alignment": "g-4", "leastRadius": 100.0})
    q("Hepsi birden", {"alignment": "g-1", "leastRadius": 180.0, "leastArc": 120.0, "leastSpiral": 50.0,
                       "leastTangent": 100.0, "leastReverse": 60.0, "leastBrokenBack": 60.0, "mostRadiusRatio": 1.2,
                       "kinks": True})
    q("Kırık sırtta ters kural işlemez", {"alignment": "g-2", "leastReverse": 500.0})
    q("Bitişikte aliyman kuralı işlemez", {"alignment": "g-3", "leastTangent": 500.0})
    # Refusals.
    q("Kural yok", {"alignment": "g-1"})
    q("Sıfır yarıçap sınırı", {"alignment": "g-1", "leastRadius": 0.0})
    q("Eksi aliyman sınırı", {"alignment": "g-1", "leastTangent": -5.0})
    q("Birden küçük oran", {"alignment": "g-1", "mostRadiusRatio": 0.5})
    q("Paralel aks", {"alignment": "g-5", "leastRadius": 100.0})
    q("Olmayan aks", {"alignment": "yok", "leastRadius": 100.0})
    q("Yol projesi değil", {"alignment": "g-1", "leastRadius": 100.0},
      failed("not_a_road_project", "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
      setup_=setup("map"))
    return out


def profile_cases():
    out, q = cases_of(p_answer)
    q("Eğim en çok", {"profile": "b-1", "mostGrade": 0.035})
    q("Eğim en az", {"profile": "b-1", "leastGrade": 0.02})
    q("Tepe ve çukur K", {"profile": "b-1", "leastCrestK": 15.0, "leastSagK": 25.0})
    q("K tutar", {"profile": "b-1", "leastCrestK": 5.0, "leastSagK": 10.0})
    q("Düşey kurp boyu", {"profile": "b-1", "leastCurve": 100.0})
    q("Kurplar arası eğim", {"profile": "b-1", "leastGradeLength": 120.0})
    q("Hepsi birden", {"profile": "Proje kotu", "mostGrade": 0.035, "leastGrade": 0.02, "leastCrestK": 15.0,
                       "leastSagK": 25.0, "leastCurve": 100.0, "leastGradeLength": 120.0})
    q("Daire kurbun K'si", {"profile": "b-2", "leastCrestK": 30.0, "leastCurve": 300.0})
    q("Daire, eğimler", {"profile": "b-2", "mostGrade": 0.035, "leastGrade": 0.035})
    q("Eski boykesit", {"profile": "b-3", "leastCrestK": 20.0})
    q("Eski boykesit, kural tutar", {"profile": "b-3", "mostGrade": 0.5})
    q("Tek kurpta kurplar arası yok", {"profile": "b-2", "leastGradeLength": 500.0})
    # Refusals.
    q("Kural yok", {"profile": "b-1"})
    q("Sıfır eğim sınırı", {"profile": "b-1", "mostGrade": 0.0})
    q("Eksi K", {"profile": "b-1", "leastCrestK": -1.0})
    q("Sıfır kurp boyu", {"profile": "b-1", "leastCurve": 0.0})
    q("Olmayan boykesit", {"profile": "yok", "mostGrade": 0.05})
    q("Ad büyük küçük harf", {"profile": "PROJE KOTU", "mostGrade": 0.05})
    q("Kısa eğim yok", {"profile": "b-1", "leastGradeLength": 50.0})
    q("Yatık eğim yok", {"profile": "b-2", "leastGrade": 0.01})
    q("Yol projesi değil", {"profile": "b-1", "mostGrade": 0.05},
      failed("not_a_road_project", "Boykesit yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
      setup_=setup("map"))
    return out


def main():
    docs = {
        "road.alignments.check.json": ("road.alignments.check", "Aksı denetle: kullanıcının tasarım kurallarıyla",
                                       alignment_cases()),
        "road.profiles.check.json": ("road.profiles.check", "Boykesiti denetle: kullanıcının tasarım kurallarıyla",
                                     profile_cases()),
    }
    stale = []
    for name, (command, title, found) in docs.items():
        doc = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
               "note": "scripts/fixtures/road_design_check_cases.py'den; aksın kurpları road_alignment_cases.py'nin "
                       "40 basamaklı hesabıyla, boykesit road_vertical_cases.py'nin kesirleriyle; KentOS kodu olmadan.",
               "setup": setup(), "cases": found}
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if not path.exists() or path.read_text() != text:
                stale.append(name)
        else:
            path.write_text(text)
        print(f"{name}: {len(found)} durum")
    if stale:
        sys.exit(f"güncel değil: {stale}")


if __name__ == "__main__":
    main()
