#!/usr/bin/env python3
"""The shared cases of `road.culverts.hydraulics` (plan/road.md Y41, docs/adr/0947; ROAD-DRAIN-08).

The culvert as read (its ends' inverts, its barrel's length, the road's top over it) is road_culvert_cases.py's, its
corridor's cross road_stakeout_cases.py's; the hydraulics are road_culvert_hydraulics_cases.py's, mpmath at 40 digits
from HDS-5's equations and AIDR's hazard limits. No KentOS code. The coefficients and flows are the cases' own.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_culvert_hydraulics_command_cases.py [--check]
"""

import json
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_culvert_cases as rc  # noqa: E402
import road_culvert_hydraulics_cases as hy  # noqa: E402
import road_grades  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402
import road_stakeout_cases as so  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.culverts.hydraulics.json"
completed, failed, step, case = rc.completed, rc.failed, rc.step, rc.case
UNCHANGED = rc.UNCHANGED
W = 1e-6
NOT_ROAD = rc.NOT_ROAD
START = rc.START

INLETS = {"pipeSquareHeadwall": "pipe-square-headwall", "pipeGrooveHeadwall": "pipe-groove-headwall",
          "pipeGrooveProjecting": "pipe-groove-projecting", "boxWingwall30To75": "box-wingwall-30-75",
          "boxWingwall90And15": "box-wingwall-90-15", "boxWingwall0": "box-wingwall-0",
          "boxHeadwallChamfer": "box-headwall-chamfer"}
WORDS = {1: "H1: araç, insan ve yapı için genelde güvenli", 2: "H2: küçük araçlar için güvensiz",
         3: "H3: araçlar, çocuklar ve yaşlılar için güvensiz", 4: "H4: araçlar ve insanlar için güvensiz",
         5: "H5: araçlar ve insanlar için güvensiz, yapılar hasar görebilir",
         6: "H6: araçlar ve insanlar için güvensiz, yapılar yıkılabilir"}


def bad(path, words):
    return rc.Refused("invalid_culvert_hydraulics", f"Menfez hidroliği sorusu geçerli değil: {words}.", path)


def crest_of(m):
    """The highest of the road's top between the hinges at the culvert's station (as road_culvert_cases.read finds
    them)."""
    c = grc.corridor(m["corridor"])
    table, _ = so.table(c, {"corridor": c["id"], "from": m["station"], "to": m["station"], "step": 1.0})
    region = table["rows"][0]["region"]
    rows = [r for r in table["rows"] if r["region"] == region]
    hinge = lambda w: next(r for r in rows if r["code"] == f"{w}.hinge")["offset"]  # noqa: E731
    lo, hi = hinge("left"), hinge("right")
    return max(r["z"] for r in rows if lo - 1e-9 <= r["offset"] <= hi + 1e-9)


def flow_read(c, weir, q, tw, invert):
    p = hy.perform(c, weir, q, tw)
    f = lambda v: float(v)  # noqa: E731
    out = {"flow": q, "headwater": f(p["headwater"]), "level": invert + f(p["headwater"]),
           "inletControl": p["control"] == "inlet", "inletHeadwater": f(p["inlet"]), "outletHeadwater": f(p["outlet"]),
           "through": f(p["culvert"]), "overtopping": f(p["overtopping"])}
    if p["hazard"]:
        out["over"] = {"head": f(p["head"]), "depth": f(p["depth"]), "speed": f(p["speed"]), "unit": f(p["unit"]),
                       "hazard": p["hazard"], "words": WORDS[p["hazard"]]}
    return out


def answer(input_, culverts=START):
    m = rc.find(culverts, input_["culvert"])
    q = input_["flow"]
    if not 0 < q <= 10000:
        raise bad("flow", "debi 0'dan büyük, en çok 10000 m³/s olur")
    tw = input_.get("tailwater", 0.0)
    if not 0 <= tw <= 100:
        raise bad("tailwater", "çıkıştaki su derinliği 0 ile 100 m arası olur")
    ke = input_["entranceLoss"]
    if not 0 <= ke <= 2:
        raise bad("entranceLoss", "giriş kaybı katsayısı 0 ile 2 arası olur")
    n = input_.get("roughness", m.get("roughness"))
    if n is None:
        raise bad("roughness", "menfezin Manning pürüzlülüğü yok; roughness ile verin")
    if not 0 < n <= 0.1:
        raise bad("roughness", "Manning pürüzlülüğü 0'dan büyük, en çok 0.1 olur")
    w = input_.get("weir")
    if w is not None:
        if not 0 < w["cd"] <= 3:
            raise bad("weir.cd", "savak katsayısı 0'dan büyük, en çok 3 olur")
        if not 0 < w["length"] <= 10000:
            raise bad("weir.length", "savak boyu 0'dan büyük, en çok 10000 m olur")
    k = input_.get("curve")
    if k is not None and not 2 <= k <= 200:
        raise bad("curve", "eğrinin nokta sayısı 2 ile 200 arası olur")
    report, warnings, _, _ = rc.read(m)
    ends = {e["side"]: e["invert"] for e in report["ends"]}
    inlet_side = report["inlet"]
    inlet, outlet = ends[inlet_side], ends["right" if inlet_side == "left" else "left"]
    length = report["length"]
    slope = (inlet - outlet) / length
    sh = m["shape"]
    if sh["kind"] == "pipe":
        section, cells = {"kind": "circle", "diameter": sh["diameter"]}, sh["count"]
    else:
        section, cells = {"kind": "box", "width": sh["width"], "height": sh["height"]}, sh["cells"]
    c = {"section": section, "cells": cells, "length": length, "slope": slope, "n": n,
         "inlet": INLETS[input_["inlet"]], "ke": ke}
    crest = None
    if w is not None:
        crest = w["crest"] if w.get("crest") is not None else crest_of(m)
        if crest <= inlet:
            raise bad("weir.crest", f"savak kreti ({crest:.3f}) menfezin giriş kotunun ({inlet:.3f}) üstünde olmalı")
        weir = {"crest": crest - inlet, "length": w["length"], "cd": w["cd"]}
    else:
        weir = {"crest": mp.inf, "length": 0, "cd": 0}
    at = flow_read(c, weir, q, tw, inlet)
    out = {"culvert": m["id"], "inletInvert": inlet, "outletInvert": outlet, "length": length, "slope": slope,
           "cells": cells, "roughness": n}
    if crest is not None:
        out["crest"] = crest
    out["at"] = at
    if k is not None:
        out["curve"] = [flow_read(c, weir, 1.5 * q * i / (k - 1), tw, inlet) for i in range(k)]
    if "over" in at:
        o = at["over"]
        warnings = warnings + [{
            "code": "culvert_overtopped", "path": "flow",
            "message": f"“{m['name']}” menfezinde {rc.num(q)} m³/s debide su yolu aşıyor: yoldan "
                       f"{at['overtopping']:.3f} m³/s geçer, kret üstünde su {o['head']:.3f} m; {o['words']}."}]
    return out, warnings


def cases():
    out = []

    def ok(name, input_, op="execute", culverts=START):
        o, warns = answer(input_, culverts)
        result = completed(None, warns) if op == "validate" else completed(o, warns)
        s = step(op, input_, result, {**UNCHANGED, "road": rc.road_of(culverts)})
        s["within"] = W
        out.append(case(name, [s], None if culverts is START else rc.setup(culverts)))
        return o

    def no(name, input_, culverts=START, result=None, setup_=None):
        if result is None:
            try:
                answer(input_, culverts)
            except rc.Refused as r:
                result = r.result
            else:
                raise AssertionError(f"{name}: refuses")
        road = None if setup_ is not None else rc.road_of(culverts)
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                        setup_ if setup_ is not None else (None if culverts is START else rc.setup(culverts))))

    box = {"culvert": "m-1", "inlet": "boxWingwall30To75", "entranceLoss": 0.4}
    road = {"cd": 1.66, "length": 60.0}
    pipe = {"culvert": "yan MENFEZ", "inlet": "pipeSquareHeadwall", "entranceLoss": 0.5, "roughness": 0.013}
    a = ok("Kutu menfez, küçük debi: (dc + D)/2 çıkış kontrolü", {**box, "flow": 4.0})
    assert not a["at"]["inletControl"], "at a small flow the outlet's (dc + D)/2 sets the headwater"
    ok("Yol savak, aşılmıyor", {**box, "flow": 12.0, "weir": road})
    b = ok("Yol aşılıyor", {**box, "flow": 60.0, "weir": road})
    assert "over" in b["at"] and b["at"]["inletControl"], "60 m³/s overtops the road, the inlet controlling"
    ok("Kısa taşma boyu, tehlike yüksek", {**box, "flow": 60.0, "weir": {"cd": 1.66, "length": 10.0}})
    ok("Kret verilmiş", {**box, "flow": 20.0, "weir": {**road, "crest": 108.5}})
    ok("Performans eğrisi", {**box, "flow": 40.0, "weir": road, "curve": 13})
    ok("İki noktalı eğri, savaksız", {**box, "flow": 10.0, "curve": 2})
    o = ok("Yüksek kuyruk suyu: çıkış kontrollü", {**box, "flow": 8.0, "tailwater": 3.0})
    assert not o["at"]["inletControl"], "a high tailwater sets the outlet's control"
    ok("Pürüzlülük girdiden", {**box, "flow": 8.0, "tailwater": 2.0, "roughness": 0.024})
    ok("Kutu 90° ve 15° kanat, batmış giriş", {**box, "flow": 24.0, "inlet": "boxWingwall90And15"})
    ok("Kutu kanatsız, batmış giriş", {**box, "flow": 24.0, "inlet": "boxWingwall0", "entranceLoss": 0.5})
    ok("Pahlı başlıklı kutu", {**box, "flow": 12.0, "inlet": "boxHeadwallChamfer", "entranceLoss": 0.5})
    ok("Adıyla, eskiyen boru menfez", {**pipe, "flow": 1.5})
    ok("Oluklu ağızlı boru", {**pipe, "flow": 2.5, "inlet": "pipeGrooveHeadwall", "entranceLoss": 0.2})
    ok("Çıkıntılı oluklu boru, yolu aşıyor", {**pipe, "flow": 12.0, "inlet": "pipeGrooveProjecting",
                                              "entranceLoss": 0.2, "weir": {"cd": 1.6, "length": 30.0}})
    ok("Az örtü uyarısıyla", {**box, "flow": 4.0}, culverts=[{**rc.M1, "cover": 2.0}])
    ok("Planı", {**box, "flow": 60.0, "weir": road}, op="plan")
    ok("Doğrulama", {**box, "flow": 60.0, "weir": road}, op="validate")
    no("Yol projesi değil", {**box, "flow": 4.0}, result=NOT_ROAD, setup_=rc.setup(work="map"))
    no("Olmayan menfez", {**box, "culvert": "yok", "flow": 4.0})
    no("Sıfır debi", {**box, "flow": 0.0})
    no("Eksi kuyruk suyu", {**box, "flow": 4.0, "tailwater": -1.0})
    no("Giriş kaybı 3", {**box, "flow": 4.0, "entranceLoss": 3.0})
    no("Pürüzlülüğü olmayan menfez", {k: v for k, v in {**pipe, "flow": 1.0}.items() if k != "roughness"})
    no("Pürüzlülük 0.2", {**box, "flow": 4.0, "roughness": 0.2})
    no("Savak katsayısı sıfır", {**box, "flow": 4.0, "weir": {"cd": 0.0, "length": 60.0}})
    no("Savak boyu sıfır", {**box, "flow": 4.0, "weir": {"cd": 1.66, "length": 0.0}})
    no("Tek noktalı eğri", {**box, "flow": 4.0, "curve": 1})
    no("Kret girişin altında", {**box, "flow": 4.0, "weir": {**road, "crest": 100.0}})
    yarma = START + [{**rc.M1, "id": "m-3", "name": "Yarma", "station": 60.0}]
    no("Yarmadaki menfez", {**box, "culvert": "m-3", "flow": 4.0}, culverts=yarma)
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.culverts.hydraulics", "commandVersion": 1,
            "title": "Menfez hidroliği",
            "note": "scripts/fixtures/road_culvert_hydraulics_command_cases.py yazar; menfezin okunuşu "
                    "road_culvert_cases.py'nin, hidrolik road_culvert_hydraulics_cases.py'nin 40 basamaklı mpmath "
                    "başvurusu (HDS-5, AIDR); KentOS kodu yok. Katsayılar ve debiler durumlarındır.",
            "setup": rc.setup(), "cases": cases()}
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")
    for c in data["cases"]:
        s = c["steps"][0]
        r = s["result"]
        if r.get("status") == "completed" and r.get("output"):
            o = r["output"]
            at = o["at"]
            print(f"  {c['name'][:40]:40} giriş {o['inletInvert']:.3f} kret {o.get('crest', 0):.3f} L {o['length']:.2f} "
                  f"S {o['slope']:.4f} HW {at['headwater']:.3f} ({'giriş' if at['inletControl'] else 'çıkış'}) "
                  f"taşan {at['overtopping']:.2f} H{at.get('over', {}).get('hazard', 0)} "
                  f"{[w['code'] for w in r.get('warnings', [])]}")
        else:
            print(f"  {c['name'][:40]:40} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
