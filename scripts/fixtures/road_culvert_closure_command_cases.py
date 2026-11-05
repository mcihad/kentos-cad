#!/usr/bin/env python3
"""The shared cases of `road.culverts.closure` (plan/road.md Y42, docs/adr/0948; ROAD-DRAIN-08).

The culvert as read is road_culvert_hydraulics_command_cases.py's (its ends, length and the road's crest over it); the
closure road_culvert_closure_cases.py's (each criterion a limit on the flow per metre on the crest, its threshold flow
in one root of the culvert's headwater, the hours exact on the linear hydrograph), the hydraulics HDS-5 in mpmath at 40
digits. No KentOS code. The hydrographs and coefficients are the cases' own.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_culvert_closure_command_cases.py [--check]
"""

import json
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_culvert_cases as rc  # noqa: E402
import road_culvert_closure_cases as cl  # noqa: E402
import road_culvert_hydraulics_command_cases as hc  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.culverts.closure.json"
completed, failed, step, case = rc.completed, rc.failed, rc.step, rc.case
UNCHANGED, NOT_ROAD, START = rc.UNCHANGED, rc.NOT_ROAD, rc.START
W = 1e-6
VEHICLE_WORDS = ["Küçük binek araç", "Büyük binek araç", "Büyük 4×4 araç"]


def bad(path, words):
    return hc.bad(path, words)


def model(input_, culverts):
    """road.culverts.hydraulics' checks and model, without a flow."""
    probe = {**input_, "flow": 1.0}
    probe.pop("hydrograph", None)
    probe.pop("range", None)
    m = rc.find(culverts, input_["culvert"])
    tw = input_.get("tailwater", 0.0)
    if not 0 <= tw <= 100:
        raise bad("tailwater", "çıkıştaki su derinliği 0 ile 100 m arası olur")
    if not 0 <= input_["entranceLoss"] <= 2:
        raise bad("entranceLoss", "giriş kaybı katsayısı 0 ile 2 arası olur")
    n = input_.get("roughness", m.get("roughness"))
    if n is None:
        raise bad("roughness", "menfezin Manning pürüzlülüğü yok; roughness ile verin")
    if not 0 < n <= 0.1:
        raise bad("roughness", "Manning pürüzlülüğü 0'dan büyük, en çok 0.1 olur")
    w = input_["weir"]
    if not 0 < w["cd"] <= 3:
        raise bad("weir.cd", "savak katsayısı 0'dan büyük, en çok 3 olur")
    if not 0 < w["length"] <= 10000:
        raise bad("weir.length", "savak boyu 0'dan büyük, en çok 10000 m olur")
    return m, probe


def answer(input_, culverts=START):
    m, probe = model(input_, culverts)
    h = input_["hydrograph"]
    if not 2 <= len(h) <= 1000:
        raise bad("hydrograph", "hidrograf 2 ile 1000 nokta olur")
    for k, p in enumerate(h):
        if not (0 <= p["time"] <= 8760 and 0 <= p["flow"] <= 10000):
            raise bad(f"hydrograph.{k}", "hidrografın saati 0 ile 8760, debisi 0 ile 10000 m³/s arası olur")
    for k in range(1, len(h)):
        if h[k]["time"] <= h[k - 1]["time"]:
            raise bad(f"hydrograph.{k}", "hidrografın saatleri artarak gider")
    if all(p["flow"] <= 0 for p in h):
        raise bad("hydrograph", "hidrografta debi yok")
    cd = input_["weir"]["cd"]
    r = input_.get("range")
    if r is not None:
        if not (0 < r["flowLow"] <= 1 and 1 <= r["flowHigh"] <= 10):
            raise bad("range", "debi çarpanının alt sınırı 0 ile 1, üst sınırı 1 ile 10 arası olur")
        if not (0 < r["cdLow"] <= cd and cd <= r["cdHigh"] <= 3):
            raise bad("range", "Cd'nin alt sınırı 0 ile savağın Cd'si, üst sınırı savağın Cd'si ile 3 arası olur")
    # The hydraulics' answer at any flow gives the culvert as read, its crest and warnings.
    read, warnings = hc.answer(probe, culverts)
    warnings = [w for w in warnings if w["code"] != "culvert_overtopped"]
    sh = m["shape"]
    section = ({"kind": "circle", "diameter": sh["diameter"]} if sh["kind"] == "pipe"
               else {"kind": "box", "width": sh["width"], "height": sh["height"]})
    c = {"section": section, "cells": read["cells"], "length": read["length"], "slope": read["slope"],
         "n": read["roughness"], "inlet": hc.INLETS[input_["inlet"]], "ke": input_["entranceLoss"]}
    inlet, crest = read["inletInvert"], read["crest"]
    weir = {"crest": crest - inlet, "length": input_["weir"]["length"], "cd": cd}
    tw = input_.get("tailwater", 0.0)
    points = [(p["time"], p["flow"]) for p in h]
    cases_ = [("asked", 1.0, cd)] + ([("low", r["flowLow"], r["cdLow"]), ("high", r["flowHigh"], r["cdHigh"])]
                                    if r is not None else [])
    floods = []
    for name, factor, kcd in cases_:
        scaled = [(t, mp.mpf(q) * mp.mpf(factor)) for t, q in points]
        wk = {**weir, "cd": kcd}
        peak = max(q for _, q in scaled)

        def crit(code, words, qstar):
            q = cl.threshold(c, wk, tw, qstar, peak)
            out = {"criterion": code, "words": words}
            if q is None:
                return {**out, "hours": 0.0, "spans": []}
            hours, spans = cl.above(scaled, q)
            return {**out, "flow": float(q), "hours": float(hours),
                    "spans": [{"from": float(a), "to": float(b)} for a, b in spans]}
        criteria = [crit("submerged", "Su altında", None)]
        criteria += [crit(v[0], VEHICLE_WORDS[k], cl.q_limit(*v[1:])) for k, v in enumerate(cl.VEHICLES)]
        criteria += [crit(f"H{k + 2}", hc.WORDS[k + 2], cl.q_limit(d, v, p)) for k, (p, d, v) in enumerate(cl.HAZARD)]
        floods.append({"case": name, "factor": factor, "cd": kcd,
                       "peak": hc.flow_read(c, wk, float(peak), tw, inlet), "criteria": criteria})
    a = floods[0]["criteria"]
    rng = (lambda k: f" (aralıkta {floods[1]['criteria'][k]['hours']:.2f}–{floods[2]['criteria'][k]['hours']:.2f} saat)"
           if len(floods) == 3 else "")
    if a[0]["hours"] > 0:
        first, last = a[0]["spans"][0]["from"], a[0]["spans"][-1]["to"]
        warnings.append({"code": "culvert_overtopped", "path": "hydrograph",
                         "message": f"“{m['name']}” menfezinin üstündeki yol taşkında {a[0]['hours']:.2f} saat su altında: "
                                    f"{first:.2f}. saatte başlar, {last:.2f}. saatte biter{rng(0)}."})
    if a[1]["hours"] > 0:
        warnings.append({"code": "culvert_road_closed", "path": "hydrograph",
                         "message": f"“{m['name']}” menfezinin üstündeki yol küçük binek araçlara {a[1]['hours']:.2f} "
                                    f"saat kapalı{rng(1)}."})
    return {"culvert": m["id"], "inletInvert": inlet, "crest": crest, "floods": floods}, warnings


def tri(qp, tp, tb, base=0.0):
    return [{"time": 0.0, "flow": base}, {"time": tp, "flow": qp}, {"time": tb, "flow": base}]


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

    box = {"culvert": "m-1", "inlet": "boxWingwall30To75", "entranceLoss": 0.4, "weir": {"cd": 1.66, "length": 60.0}}
    flood = {**box, "hydrograph": tri(60.0, 3.0, 12.0)}
    rng = {"flowLow": 0.8, "flowHigh": 1.2, "cdLow": 1.5, "cdHigh": 1.8}
    a = ok("Üçgen hidrograf, kutu menfez", flood)
    hours = [c["hours"] for c in a["floods"][0]["criteria"][:4]]
    assert hours[0] > hours[1] > 0, "under water longer than closed to small cars"
    b = ok("Debi ve Cd aralığıyla", {**flood, "range": rng})
    mid, lo, hi = (f["criteria"][1]["hours"] for f in b["floods"])
    assert lo < mid < hi, "the range holds the asked"
    ok("Aşmayan taşkın", {**box, "hydrograph": tri(8.0, 2.0, 6.0)})
    ok("Dar çukur: 4×4 de kapalı", {**flood, "weir": {"cd": 1.66, "length": 10.0}})
    ok("Kret verilmiş", {**flood, "weir": {"cd": 1.66, "length": 60.0, "crest": 108.5}})
    ok("İki tepeli hidrograf", {**box, "hydrograph": [{"time": 0.0, "flow": 0.0}, {"time": 2.0, "flow": 45.0},
                                                      {"time": 4.0, "flow": 10.0}, {"time": 6.0, "flow": 55.0},
                                                      {"time": 12.0, "flow": 0.0}]})
    ok("Taban debili hidrograf", {**box, "hydrograph": tri(50.0, 4.0, 18.0, 3.0)})
    ok("Başta yüksek hidrograf", {**box, "hydrograph": [{"time": 0.0, "flow": 50.0}, {"time": 6.0, "flow": 0.0}]})
    ok("Yüksek kuyruk suyu", {**flood, "tailwater": 2.0})
    ok("Pürüzlülük girdiden", {**flood, "roughness": 0.02})
    ok("Eskiyen boru menfez", {"culvert": "yan MENFEZ", "inlet": "pipeSquareHeadwall", "entranceLoss": 0.5,
                               "roughness": 0.013, "weir": {"cd": 1.6, "length": 30.0},
                               "hydrograph": tri(14.0, 1.0, 5.0)})
    ok("Planı", {**flood, "range": rng}, op="plan")
    ok("Doğrulama", flood, op="validate")
    no("Yol projesi değil", flood, result=NOT_ROAD, setup_=rc.setup(work="map"))
    no("Olmayan menfez", {**flood, "culvert": "yok"})
    no("Tek noktalı hidrograf", {**box, "hydrograph": [{"time": 0.0, "flow": 5.0}]})
    no("Saatler geri gidiyor", {**box, "hydrograph": [{"time": 0.0, "flow": 0.0}, {"time": 3.0, "flow": 20.0},
                                                      {"time": 2.0, "flow": 0.0}]})
    no("Eksi debi", {**box, "hydrograph": [{"time": 0.0, "flow": -1.0}, {"time": 3.0, "flow": 20.0}]})
    no("Debisiz hidrograf", {**box, "hydrograph": [{"time": 0.0, "flow": 0.0}, {"time": 3.0, "flow": 0.0}]})
    no("Debi çarpanı sıfır", {**flood, "range": {**rng, "flowLow": 0.0}})
    no("Cd aralığı savağın Cd'sini içermiyor", {**flood, "range": {**rng, "cdHigh": 1.6}})
    no("Savak katsayısı sıfır", {**flood, "weir": {"cd": 0.0, "length": 60.0}})
    no("Pürüzlülüğü olmayan menfez", {**flood, "culvert": "m-2", "inlet": "pipeSquareHeadwall"})
    yarma = START + [{**rc.M1, "id": "m-3", "name": "Yarma", "station": 60.0}]
    no("Yarmadaki menfez", {**flood, "culvert": "m-3"}, culverts=yarma)
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.culverts.closure", "commandVersion": 1,
            "title": "Taşkında yol kapanması",
            "note": "scripts/fixtures/road_culvert_closure_command_cases.py yazar; menfezin okunuşu "
                    "road_culvert_hydraulics_command_cases.py'nin, kapanma road_culvert_closure_cases.py'nin (her ölçüt "
                    "sırtta metre başına debi sınırı), hidrolik 40 basamaklı mpmath; KentOS kodu yok. Hidrograflar ve "
                    "katsayılar durumlarındır.",
            "setup": rc.setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")
    for c in data["cases"]:
        r = c["steps"][0]["result"]
        o = r.get("output")
        if r.get("status") == "completed" and o:
            for f in o["floods"]:
                hs = " ".join(f"{x['hours']:.2f}" for x in f["criteria"][:4])
                print(f"  {c['name'][:36]:36} {f['case']:5} tepe {f['peak']['flow']:.1f} saat: {hs} "
                      f"{[w['code'] for w in r.get('warnings', [])]}")
        else:
            print(f"  {c['name'][:36]:36} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
