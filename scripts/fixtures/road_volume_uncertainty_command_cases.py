#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.volumes.uncertainty.json: the shared cases of `road.volumes.uncertainty`
(plan/road.md Y35.2, docs/adr/0941), their expectations built without KentOS code: the drawing, the corridors and the
volumes are scripts/fixtures/road_volume_command_cases.py's (the volume's report, the other method's too); the
corridor's top surface scripts/fixtures/road_corridor_command_cases.py's; the ground's TIN the profile reference's;
each ground point's weight scripts/fixtures/road_volume_uncertainty_cases.py's (exact fractions, checked there by
differences and trials). σ√Σw², β·Σw, |β·Σw| + 1.96σ, n·u·Σ|v| as the ADR writes them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_volume_uncertainty_command_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_cases as cref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402
import road_volume_uncertainty_cases as uref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.volumes.uncertainty.json"
completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED = pcmd.UNCHANGED
Z95 = 1.959963984540054
U = 2.0 ** -53
NEAR_SHARE = 0.5
VERTICES = [(x, y, z) for (x, y), z in zip(pcmd.TIN.xy, pcmd.TIN.z)]
TRIANGLES = [list(t) for t in pcmd.TIN.tris]
NOT_ROAD = failed("not_a_road_project", "Kübaj yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")


def spread(w, sigma):
    return sigma * math.sqrt(sum(x * x for x in w))


def answer(v, q, volumes, corridors=vcmd.CORRIDORS):
    rep, warns = vcmd.compute(v, corridors)
    c = next(x for x in corridors if x["id"] == v["corridor"])
    if c["surface"] != "t-1":
        raise vcmd.Refused("invalid_volume", f"“{c['name']}” koridorunun yüzeyi kurulamıyor; kotların hacme etkisi okunamaz.",
                           "corridor")
    _, _, tris, regs = ccmd.make(c)
    sigma, beta = q["sigma"], q.get("systematic") or 0.0
    near = 2 * sigma
    wc, wf, areas = uref.weights(tris, VERTICES, TRIANGLES, near=near)
    wc, wf = [float(x) for x in wc], [float(x) for x in wf]
    wn = [a - b for a, b in zip(wc, wf)]

    def spread_of(value, w):
        s = spread(w, sigma)
        b = beta * sum(w)
        bound = abs(b) + Z95 * s
        out = {"value": value, "sigma": s, "bias": b, "bound": bound}
        if q.get("accept") is not None:
            out["accepted"] = bound <= q["accept"] / 100 * abs(value) + 1e-9
        return out

    cut, fill = spread_of(rep["cut"], wc), spread_of(rep["fill"], wf)
    net = spread_of(rep["cut"] - rep["fill"], wn)
    covered = float(areas["covered"])
    share = float(areas["near"]) / covered if covered > 0 else 0.0
    other = {"endArea": "prismoidal"}.get(v["method"], "endArea")
    od = copy.deepcopy(v)
    od["method"] = other
    if other != "endArea":
        od.pop("curve", None)
    try:
        orep, _ = vcmd.compute(od, corridors)
    except vcmd.Refused:
        orep = None
    terms = [x for r in rep["rows"] for x in (r.get("cut"), r.get("fill")) if x is not None]
    numeric = len(terms) * U * sum(abs(x) for x in terms)
    stretches = []
    for reg in regs:
        t = reg["template"]
        for x, y in zip(reg["crosses"], reg["crosses"][1:]):
            if x["section"] is None or y["section"] is None:
                continue
            got = cref.strip(cref.top(x, t), cref.top(y, t))
            if any(cref.area(g) <= 0 for g in got):
                continue
            sc, sf, _ = uref.weights(got, VERTICES, TRIANGLES, near=near)
            stretches.append({"from": x["station"], "to": y["station"],
                              "fromText": ccmd.km_text(c["alignment"], x["station"]),
                              "toText": ccmd.km_text(c["alignment"], y["station"]),
                              "cutSigma": spread([float(a) for a in sc], sigma),
                              "fillSigma": spread([float(a) for a in sf], sigma)})
    out = {"volume": v["id"], "method": v["method"], "sigma": sigma}
    if q.get("systematic") is not None:
        out["systematic"] = q["systematic"]
    out.update({"cut": cut, "fill": fill, "net": net, "cutArea": float(areas["cut"]), "fillArea": float(areas["fill"]),
                "nearShare": share, "points": sum(1 for a, b in zip(wc, wf) if abs(a) + abs(b) > 0)})
    if orep is not None:
        out.update({"otherMethod": other, "otherCut": orep["cut"], "otherFill": orep["fill"]})
    out.update({"numeric": numeric, "stretches": stretches})
    warns = list(warns)
    if share > NEAR_SHARE:
        warns.append({"code": "volume_uncertainty_near_zero",
                      "message": f"“{v['name']}” kübajında alanın %{100 * share:.0f}'i sıfır çizgisinin 2σ yakınında: kazı ve "
                                 "dolgunun σ'sı ayrı ayrı yalnız yaklaşık (netinki kesin).", "path": "sigma"})
    over = [w for w, s in (("kazı", cut), ("dolgu", fill)) if s.get("accepted") is False]
    if over:
        acc = q["accept"]
        acc_text = str(int(acc)) if float(acc).is_integer() else repr(float(acc))
        warns.append({"code": "volume_uncertainty_over",
                      "message": f"“{v['name']}” kübajında {' ve '.join(over)} %95 sınırı kabul payını (%{acc_text}) aşıyor.",
                      "path": "accept"})
    return out, warns


def cases():
    out = []
    start = vcmd.START

    def q(name, input_, volumes=start, corridors=vcmd.CORRIDORS, op="execute"):
        v = next(x for x in volumes if x["id"] == input_["volume"] or x["name"].lower() == input_["volume"].lower())
        result, warns = answer(v, input_, volumes, corridors)
        res = completed(None, warns) if op == "validate" else completed(result, warns)
        default = volumes is start and corridors is vcmd.CORRIDORS
        out.append(case(name, [step(op, input_, res, {**UNCHANGED, "road": vcmd.road_of(volumes, corridors)})],
                        None if default else vcmd.setup(volumes, corridors)))

    def no(name, input_, result, setup_=None):
        road = setup_["settings"].get("road") if setup_ is not None else vcmd.road_of(start)
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    def with_(**kw):
        v = copy.deepcopy(vcmd.V1)
        for k, val in kw.items():
            if val is None:
                v.pop(k, None)
            else:
                v[k] = val
        return [v, vcmd.V2]

    bad = lambda path, words: failed("invalid_volume_uncertainty", words, path)  # noqa: E731
    q("Ortalama alan, σ 5 cm", {"volume": "v-1", "sigma": 0.05})
    q("Adıyla, σ 5 cm ve 2 cm kayma", {"volume": "ANA kübaj", "sigma": 0.05, "systematic": 0.02})
    q("Eksi kayma: arazi aşağıda ölçülmüş", {"volume": "v-1", "sigma": 0.03, "systematic": -0.04})
    q("Kabul %3: tutar", {"volume": "v-1", "sigma": 0.05, "systematic": 0.02, "accept": 3.0})
    q("Kabul %0,5: aşar", {"volume": "v-1", "sigma": 0.05, "systematic": 0.02, "accept": 0.5})
    q("Yüzeyler arası kübaj", {"volume": "v-2", "sigma": 0.05})
    q("Prizmoidal: öbür yöntem ortalama alan", {"volume": "v-1", "sigma": 0.05}, with_(method="prismoidal"))
    q("Yüzeyler arası ana koridorda", {"volume": "v-1", "sigma": 0.05}, with_(method="surfaces"))
    q("σ 1 cm: GNSS RTK", {"volume": "v-1", "sigma": 0.01})
    q("σ 30 cm: sıfır çizgisi yakını büyür", {"volume": "v-1", "sigma": 0.3})
    q("σ 1 m: yakın alan yarının altında", {"volume": "v-1", "sigma": 1.0})
    q("σ 2 m: kazı ve dolgu ayrı yaklaşık", {"volume": "v-1", "sigma": 2.0})
    q("Katsayılı kübaj: değerler doğal (yerindeki)", {"volume": "v-1", "sigma": 0.05},
      with_(factors=[{"from": 0.0, "to": 600.0, "cut": 1.2, "fill": 0.9}]))
    q("Kurp düzeltmeli kübaj", {"volume": "v-1", "sigma": 0.05}, with_(curve=True))
    q("Plan cevabı verir", {"volume": "v-1", "sigma": 0.05, "accept": 0.5}, op="plan")
    q("Doğrulama uyarıları verir", {"volume": "v-1", "sigma": 0.05, "accept": 0.5}, op="validate")
    broken = [dict(c, surface="t-2") if c["id"] == "c-1" else c for c in vcmd.CORRIDORS]
    try:
        answer(vcmd.V1, {"volume": "v-1", "sigma": 0.05}, start, broken)
    except vcmd.Refused as r:
        no("Koridorun yüzeyi kurulamıyor", {"volume": "v-1", "sigma": 0.05}, r.result, vcmd.setup(start, broken))
    no("Yol projesi değil", {"volume": "v-1", "sigma": 0.05}, NOT_ROAD, vcmd.setup(work="map"))
    no("Olmayan kübaj", {"volume": "yok", "sigma": 0.05},
       failed("unknown_volume", "“yok” kimlikli ya da adlı kübaj hesabı projede yok.", "volume"))
    no("Sıfır σ", {"volume": "v-1", "sigma": 0.0}, bad("sigma", "Kotların standart sapması 0'dan büyük, en çok 10 m olmalı."))
    no("Çok büyük σ", {"volume": "v-1", "sigma": 20.0}, bad("sigma", "Kotların standart sapması 0'dan büyük, en çok 10 m olmalı."))
    no("Çok büyük kayma", {"volume": "v-1", "sigma": 0.05, "systematic": 12.0},
       bad("systematic", "Sistematik kayma en çok 10 m olabilir."))
    no("Sıfır kabul payı", {"volume": "v-1", "sigma": 0.05, "accept": 0.0},
       bad("accept", "Kabul payı 0'dan büyük, en çok yüzde 100 olmalı."))
    no("Yüzde 100'den büyük kabul", {"volume": "v-1", "sigma": 0.05, "accept": 150.0},
       bad("accept", "Kabul payı 0'dan büyük, en çok yüzde 100 olmalı."))
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.volumes.uncertainty", "commandVersion": 1,
            "title": "Kübajın belirsizliği",
            "note": "scripts/fixtures/road_volume_uncertainty_command_cases.py yazar; çizim, koridorlar ve kübajlar "
                    "road_volume_command_cases.py'nin, arazi noktalarının ağırlıkları road_volume_uncertainty_cases.py'nin "
                    "(kesirli bindirme, farklarla ve Monte Carlo denemeleriyle sınanmış); KentOS kodu yok.",
            "setup": vcmd.setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text("utf-8") != text:
            sys.exit(f"güncel değil: {OUT.relative_to(ROOT)}")
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")


if __name__ == "__main__":
    main()
