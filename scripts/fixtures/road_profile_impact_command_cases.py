"""Writes fixtures/commands/v1/road.profiles.impact.json: the shared cases of `road.profiles.impact` (plan/road.md Y56,
docs/adr/0962; ROAD-PROFILE-09), their expectations built here without KentOS code.

The grade lines are scripts/fixtures/road_vertical_cases.py's (fractions and mpmath). Where the new line leaves the saved
one: between the breaks of either (PVIs, curve ends) both are one smooth piece, where the zeros of Δz and of the grades'
difference Δg cut |Δz| into monotone runs (400 samples, mpmath's anderson root); the tolerance's crossings and the
greatest |Δz| (at a zero of Δg) follow. KentOS samples 64 points and bisects in doubles: the two meet within 1e-6 m.

The drawing is scripts/fixtures/road_haul_command_cases.py's (corridors on the profile, section sets, volumes, haul
plans) with the main volume and the main haul plan approved.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with scipy \\
        --with shapely python3 scripts/fixtures/road_profile_impact_command_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_haul_approve_cases as hap  # noqa: E402
import road_haul_command_cases as hc  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

mp.mp.dps = 30
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.profiles.impact.json"
completed, failed, step, case = vcmd.completed, vcmd.failed, vcmd.step, vcmd.case
UNCHANGED = vcmd.UNCHANGED
W = 1e-6
TOLERANCE = 1e-3
N = 400

# The haul approval reference's drawing: the main volume approved, the main diagram's plan approved on it.
VOLUMES = [vcmd.approved(vcmd.V1), vcmd.V2, hap.pc.V3]
HAULS = hap.approve({"haul": "h-1", "at": "2026-11-04"}, hap.START, VOLUMES)[0]
PV = pcmd.pv


class Refused(Exception):
    def __init__(self, result):
        super().__init__(result["error"]["message"])
        self.result = result


def line(pvis):
    """The grade line's evaluator, its stations' span and its breaks."""
    curves, at, S, _ = pcmd.vref.solve(pcmd.to_vref(pvis))
    breaks = {mp.mpf(s.numerator) / s.denominator for s in S}
    for b, e, *_ in curves.values():
        breaks |= {mp.mpf(b), mp.mpf(e)}
    return (lambda s: at(float(s))), float(S[0]), float(S[-1]), breaks


def changes(old, new, tol):
    """Where the lines differ by more than `tol`. On each piece between the breaks of either line the zeros of Δz and
    of Δg (the grades' difference) are found among 400 samples by mpmath's anderson root: between two of them |Δz| is
    monotone, so each crossing of the tolerance lies between two consecutive points; the greatest |Δz| of a stretch is
    at an end or at a zero of Δg, the earliest where several are as great (to 1e-9 m)."""
    at0, s0, e0, b0 = line(old)
    at1, s1, e1, b1 = line(new)
    out = []
    lo, hi = max(s0, s1), min(e0, e1)
    ends = []
    for a, b, kind in [(s1, min(s0, e1), "added"), (s0, min(s1, e0), "removed"), (max(e0, s1), e1, "added"),
                       (max(e1, s0), e0, "removed")]:
        if b - a > 1e-9:
            ends.append({"kind": kind, "from": a, "to": b})
    if hi > lo:
        cuts = sorted({mp.mpf(lo), mp.mpf(hi)} | {x for x in b0 | b1 if lo < x < hi})
        for a, b in zip(cuts, cuts[1:]):
            if b - a < mp.mpf("1e-9"):
                continue

            def diff(s, a=a, b=b):
                # Within the piece: its end approached from inside (the evaluators give a break to the next piece).
                q = float(min(max(mp.mpf(s), a), b - (b - a) * mp.mpf("1e-12")))
                z0, z1 = at0(q), at1(q)
                return mp.mpf(z1[0]) - mp.mpf(z0[0]), mp.mpf(z1[1]) - mp.mpf(z0[1])

            def d(s):
                return abs(diff(s)[0])

            def over(s):
                return d(s) - tol

            xs = [a + (b - a) * k / N for k in range(N + 1)]
            pts, peaks = list(xs), [x for x in xs if diff(x)[1] == 0]
            for x0, x1 in zip(xs, xs[1:]):
                for k in (0, 1):
                    f = (lambda s, k=k: diff(s)[k])
                    f0, f1 = f(x0), f(x1)
                    if f0 != 0 and f1 != 0 and (f0 < 0) != (f1 < 0):
                        x = mp.findroot(f, (x0, x1), solver="anderson", verify=False)
                        pts.append(x)
                        if k == 1:
                            peaks.append(x)
            pts.sort()
            start = pts[0] if over(pts[0]) > 0 else None
            for x0, x1 in zip(pts, pts[1:]):
                f0, f1 = over(x0), over(x1)
                if (f0 > 0) != (f1 > 0):
                    x = mp.findroot(over, (x0, x1), solver="anderson", verify=False)
                    if start is None:
                        start = x
                    else:
                        out.append(stretch(start, x, peaks, d))
                        start = None
            if start is not None:
                out.append(stretch(start, b, peaks, d))
    merged = []
    for c in out:
        if merged and c["from"] - merged[-1]["to"] <= 1e-9:
            m = merged[-1]
            m["to"] = c["to"]
            if c["most"] > m["most"] + 1e-9:
                m["most"], m["at"] = c["most"], c["at"]
        else:
            merged.append(c)
    merged += ends
    merged.sort(key=lambda c: c["from"])
    return merged


def stretch(a, b, peaks, d):
    best = (a, d(a))
    for x in [p for p in peaks if a < p < b] + [b]:
        v = d(x)
        if v > best[1] + mp.mpf("1e-9"):
            best = (x, v)
    return {"kind": "changed", "from": float(a), "to": float(b), "most": float(best[1]), "at": float(best[0])}


def answer(inp, volumes=VOLUMES, hauls=HAULS):
    name = inp["profile"]
    old = next((p for p in hc.ccmd.PROFILES if p["id"] == name), None) or next(
        (p for p in hc.ccmd.PROFILES if p["name"].lower() == name.lower()), None)
    if old is None:
        raise Refused(failed("unknown_road_profile", f"“{name}” kimlikli ya da adlı boykesit projede yok.", "profile"))
    tol = inp.get("tolerance", TOLERANCE)
    if not 0 < tol <= 1:
        raise Refused(failed("invalid_profile_impact", "Etki toleransı 0'dan büyük, en çok 1 m olur.", "tolerance"))
    if not 2 <= len(inp["pvis"]) <= 10000:
        raise Refused(pcmd.rule("boykesitin 2 ile 10000 arası PVI'si olur", "pvis"))
    try:
        pcmd.vref.solve(pcmd.to_vref(inp["pvis"]))
        bad = None
    except pcmd.vref.Refused:
        bad = True
    length = float(pcmd.ALIGNED[old["alignment"]].length)
    if bad or any(p["station"] < -1e-9 or p["station"] > length + 1e-9 for p in inp["pvis"]):
        raise Refused(pcmd.vertical_refusal({"alignment": old["alignment"], "pvis": inp["pvis"]}))
    text = lambda s: hc.ccmd.km_text(old["alignment"], s)  # noqa: E731
    found = changes(old["pvis"], inp["pvis"], tol)
    for c in found:
        c["fromText"], c["toText"] = text(c["from"]), text(c["to"])
    inside = lambda s: any(c["from"] - 1e-9 <= s <= c["to"] + 1e-9 for c in found)  # noqa: E731
    corridors = []
    for c in hc.CORRIDORS:
        if c["profile"] != old["id"]:
            continue
        regions = []
        for ri, r in enumerate(c["regions"]):
            for ch in found:
                a, b = max(r["from"], ch["from"]), min(r["to"], ch["to"])
                if b - a > 1e-9:
                    regions.append({"region": ri, "from": a, "to": b, "fromText": text(a), "toText": text(b)})
        if regions:
            corridors.append({"corridor": c["id"], "name": c["name"], "regions": regions})
    sections = []
    for d in [scmd.S1, scmd.S2]:
        if d.get("profile") != old["id"]:
            continue
        st = scmd.stations_of(d)
        n = sum(1 for s in st if inside(s))
        if n:
            sections.append({"set": d["id"], "name": d["name"], "stations": n, "of": len(st)})
    vols = [{"id": v["id"], "name": v["name"], "approved": "approved" in v} for v in volumes
            if any(c["corridor"] == v["corridor"] for c in corridors)]
    hs = [{"id": h["id"], "name": h["name"], "approved": "approved" in h} for h in hauls
          if any(v["id"] == h["volume"] for v in vols)]
    warnings = []
    told = [f"“{v['name']}” kübajı" for v in vols if v["approved"]] + \
           [f"“{h['name']}” taşıma planı" for h in hs if h["approved"]]
    if told:
        warnings.append({"code": "profile_impact_approved",
                         "message": f"“{old['name']}” boykesitinin yeni PVI'ları onaylı hesapları değiştirir: "
                                    f"{', '.join(told)}. Yazılınca onayları eskir; yeni bir hesap onaylanmalı.",
                         "path": "pvis"})
    out = {"profile": old["id"], "changes": found, "corridors": corridors, "sections": sections, "volumes": vols,
           "hauls": hs, "clear": not found}
    return out, warnings


def cases():
    base = hc.setup(HAULS, VOLUMES)
    road = hc.road_of(HAULS, VOLUMES)
    out = []

    def ok(name, inp, op="execute", volumes=VOLUMES, hauls=HAULS, setup_=None):
        o, w = answer(inp, volumes, hauls)
        result = completed(None, w) if op == "validate" else completed(o, w)
        s = step(op, inp, result, {**UNCHANGED, "road": road if setup_ is None else hc.road_of(hauls, volumes)})
        s["within"] = W
        c = case(name, [s])
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

    def no(name, inp, result=None, setup_=None):
        if result is None:
            try:
                answer(inp)
            except Refused as r:
                result = r.result
            else:
                raise AssertionError(name)
        c = case(name, [step("execute", inp, result, {**UNCHANGED, "road": None if setup_ else road})])
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

    main = copy.deepcopy(pcmd.MAIN_PVIS)

    def with_(i, **kw):
        p = copy.deepcopy(main)
        p[i] = {**p[i], **kw}
        return p

    ok("Aynı PVI'lar: değişiklik yok", {"profile": "b-1", "pvis": main})
    ok("v1 1 m yükselir", {"profile": "b-1", "pvis": with_(1, elevation=102.5)})
    ok("v1 adıyla, 20 cm alçalır", {"profile": "Proje kotu", "pvis": with_(1, elevation=101.3)})
    ok("Toleransın altında kalan değişiklik", {"profile": "b-1", "pvis": with_(1, elevation=101.5005)})
    ok("1 cm tolerans: 5 mm görünmez", {"profile": "b-1", "pvis": with_(1, elevation=101.505), "tolerance": 0.01})
    ok("1 mm tolerans: 5 mm görünür", {"profile": "b-1", "pvis": with_(1, elevation=101.505)})
    ok("Parabol boyu uzar", {"profile": "b-1", "pvis": with_(1, length=120.0)})
    ok("Daire yarıçapı büyür", {"profile": "b-1", "pvis": with_(2, radius=1800.0)})
    ok("Asimetrik kolları değişir", {"profile": "b-1", "pvis": with_(3, lengthIn=35.0, lengthOut=50.0)})
    ok("v3 km'si kayar", {"profile": "b-1", "pvis": with_(3, station=530.0)})
    ok("Bitiş yükselir: yalnız son eğim", {"profile": "b-1", "pvis": with_(4, elevation=114.0)})
    ok("Araya PVI eklenir", {"profile": "b-1", "pvis": main[:1] + [PV("n", 80, 103.0)] + main[1:]})
    ok("Kurp kaldırılır", {"profile": "b-1",
                           "pvis": [{k: v for k, v in p.items() if k != "length"} if p["id"] == "v1" else p for p in main]})
    ok("Boykesit kısalır", {"profile": "b-1", "pvis": main[:4] + [PV("b", 600, 110.5)]})
    ok("Boykesit başta kısalır", {"profile": "b-1", "pvis": [PV("a", 40, 101.8824)] + main[1:]})
    ok("Doğrulama", {"profile": "b-1", "pvis": with_(1, elevation=102.5)}, op="validate")
    ok("Planı", {"profile": "b-1", "pvis": with_(1, elevation=102.5)}, op="plan")
    unapproved = [vcmd.V1, vcmd.V2, hap.pc.V3]
    ok("Onaysız kübaj ve plan: uyarı yok", {"profile": "b-1", "pvis": with_(1, elevation=102.5)},
       volumes=unapproved, hauls=hap.START, setup_=hc.setup(hap.START, unapproved))
    ok("Yan boykesit: koridoru ve takımı başka", {"profile": "b-2", "pvis": [
        {**p, "elevation": p["elevation"] + (0.5 if i == 1 else 0.0)}
        for i, p in enumerate(next(p for p in hc.ccmd.PROFILES if p["id"] == "b-2")["pvis"])]})
    no("Olmayan boykesit", {"profile": "yok", "pvis": main})
    no("Sıfır tolerans", {"profile": "b-1", "pvis": main, "tolerance": 0.0})
    no("Çok büyük tolerans", {"profile": "b-1", "pvis": main, "tolerance": 2.0})
    no("Tek PVI", {"profile": "b-1", "pvis": [PV("a", 0, 100)]})
    no("Kurplar biniyor", {"profile": "b-1", "pvis": [PV("a", 0, 100), PV("v1", 200, 104, length=160),
                                                      PV("v2", 300, 102, length=100), PV("b", 500, 106)]})
    no("PVI aksın dışında", {"profile": "b-1", "pvis": [PV("a", -5, 100), PV("b", 300, 104)]})
    no("Yol projesi değil", {"profile": "b-1", "pvis": main}, pcmd.NOT_ROAD, setup_=hc.setup(work="map"))
    return base, out


def main_():
    base, cs = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.profiles.impact", "commandVersion": 1,
           "title": "Boykesit değişikliğinin etkisi",
           "note": "scripts/fixtures/road_profile_impact_command_cases.py yazar; kırmızı kot road_vertical_cases.py'nin "
                   "(kesirler ve mpmath), değişen aralıklar Δz ile Δg sıfırları arasında 400 örnek ve anderson köküyle 30 "
                   "basamakta; KentOS kodu yok. Çizim road_haul_command_cases.py'nindir; ana kübaj ve ana Brükner "
                   "onaylı.",
           "setup": base, "cases": cs}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(cs)} durum güncel")
        return
    OUT.write_text(text)
    for c in cs:
        r = c["steps"][0]["result"]
        o = r.get("output") or {}
        print(f"  {c['name'][:42]:42} {r['status']:9} "
              + (" ".join(f"{x['kind'][0]}[{x['from']:.2f},{x['to']:.2f}]" + (f"≤{x['most']:.4f}" if 'most' in x else "")
                          for x in o.get("changes", [])) if o else r.get("error", {}).get("code", "")))
    print(f"yazıldı: {OUT} ({len(cs)} durum)")


if __name__ == "__main__":
    main_()
