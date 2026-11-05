#!/usr/bin/env python3
"""The shared cases of `road.volumes.ledger` (plan/road.md Y43, docs/adr/0949; ROAD-VOL-16, ROAD-VOL-18).

The volume's report is road_volume_command_cases.py's (its own reference, no KentOS code); the ledger is taken apart
here in exact fractions by road_volume_ledger_cases.py's functions: the stretches grouped by their middle, the groups'
sums, the largest stretches with their length and mean area over the medians, the densest-first concentration, the
rounded stretches against the rounded total.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_volume_ledger_command_cases.py [--check]
"""

import copy
import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402
import road_volume_ledger_cases as lref  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.volumes.ledger.json"
completed, failed, step, case = vcmd.completed, vcmd.failed, vcmd.step, vcmd.case
UNCHANGED = vcmd.UNCHANGED
W = 1e-6


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, words):
    return Refused("invalid_volume_ledger", f"Kübaj defteri sorusu geçerli değil: {words}.", path)


def ledger(input_, volumes, corridors):
    by = input_["by"]
    if by["kind"] == "stations" and not 1 <= by["step"] <= 100000:
        raise bad("by.step", "km aralığı 1 ile 100000 m arası olur")
    top = input_.get("top", 10)
    if not 1 <= top <= 100:
        raise bad("top", "en büyük aralıkların sayısı 1 ile 100 arası olur")
    decimals = input_.get("decimals", 2)
    if decimals > 3:
        raise bad("decimals", "yuvarlama en çok 3 basamak olur")
    name = input_["volume"]
    v = next((x for x in volumes if x["id"] == name), None) or next(
        (x for x in volumes if x["name"].lower() == name.lower()), None)
    if v is None:
        raise Refused("unknown_volume", f"“{name}” kimlikli ya da adlı kübaj hesabı projede yok.", "volume")
    try:
        report, warnings = vcmd.compute(v, corridors)
    except vcmd.Refused as r:
        raise Refused(r.result["error"]["code"], r.result["error"]["message"], r.result["error"].get("path"))
    c = next(x for x in corridors if x["id"] == v["corridor"])
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    columns = [("cut", "Kazı"), ("fill", "Dolgu"), ("factoredCut", "Düzeltilmiş kazı"),
               ("factoredFill", "Düzeltilmiş dolgu")]
    if "strip" in report:
        columns.append(("strip", "Bitkisel toprak"))
    if "bench" in report:
        columns.append(("bench", "Diş kazısı"))
    layers = report.get("layers", [])
    columns += [(f"layer:{l['name']}", l["name"]) for l in layers]
    stretches = []
    for r in report["rows"]:
        cut, fill = r.get("cut"), r.get("fill")
        vals = [cut, fill, None if cut is None else cut * r.get("cutFactor", 1.0),
                None if fill is None else fill * r.get("fillFactor", 1.0)]
        if "strip" in report:
            vals.append(r.get("strip"))
        if "bench" in report:
            vals.append(r.get("bench"))
        ls = r.get("layers", [])
        vals += [ls[k] if k < len(ls) else None for k in range(len(layers))]
        stretches.append({"from": r["from"], "to": r["to"], "values": vals})
    width = len(columns)
    totals = [sum((F(s["values"][k]) for s in stretches if s["values"][k] is not None), F(0)) for k in range(width)]
    cols_out = []
    for k, (key, words) in enumerate(columns):
        present = [s["values"][k] for s in stretches if s["values"][k] is not None]
        rows_r, total_r, residue = lref.rounding(present, decimals)
        cols_out.append({"key": key, "words": words, "total": float(totals[k]), "rowsRounded": rows_r,
                         "totalRounded": total_r, "residue": residue})
    if by["kind"] == "stations":
        if stretches:
            ranges = lref.steps(min(s["from"] for s in stretches), max(s["to"] for s in stretches), by["step"])
            names = [f"{text(a)} – {text(b)}" for a, b in ranges]
        else:
            ranges, names = [], []
    else:
        ranges = [[rg["from"], rg["to"]] for rg in c["regions"]]
        names = []
        for k, rg in enumerate(c["regions"]):
            t = next((x for x in ccmd.TEMPLATES if x["id"] == rg["template"]), None)
            names.append(f"Bölge {k + 1} · {t['name'] if t else rg['template']}")
    groups = []
    grouped = lref.group(stretches, ranges) if stretches else []
    for g in grouped:
        m0 = (F(stretches[g["members"][0]]["from"]) + F(stretches[g["members"][0]]["to"])) / 2
        k = next(i for i, (a, b) in enumerate(ranges) if F(a) <= m0 <= F(b))
        sums = [sum((F(stretches[i]["values"][c_]) for i in g["members"] if stretches[i]["values"][c_] is not None),
                    F(0)) for c_ in range(width)]
        groups.append({"label": names[k], "from": g["from"], "to": g["to"], "fromText": text(g["from"]),
                       "toText": text(g["to"]), "stretches": len(g["members"]), "values": [float(x) for x in sums],
                       "shares": [float(x / t) if t > 0 else None for x, t in zip(sums, totals)]})
    residual = [float(sum((F(g["values"][k]) for g in grouped), F(0)) - totals[k]) if grouped else float(-totals[k])
                for k in range(width)]
    lengths = [s["to"] - s["from"] for s in stretches]
    tops, crowd = [], []
    for k, key in ((0, "cut"), (1, "fill")):
        if totals[k] <= 0:
            continue
        vols = [s["values"][k] or 0.0 for s in stretches]
        why = lref.reasons(lengths, vols)
        order = sorted((i for i in range(len(vols)) if vols[i] > lref.NONE), key=lambda i: (-vols[i], i))[:top]
        run = F(0)
        out = []
        for i in order:
            run += F(vols[i])
            r = why[i]
            out.append({"from": stretches[i]["from"], "to": stretches[i]["to"], "fromText": text(stretches[i]["from"]),
                        "toText": text(stretches[i]["to"]), "volume": vols[i], "share": float(F(vols[i]) / totals[k]),
                        "cumulative": float(run / totals[k]), "length": lengths[i],
                        "meanArea": float(F(vols[i]) / F(lengths[i])), "lengthFactor": r["length"],
                        "areaFactor": r["area"], "reason": "section" if r["area"] >= r["length"] else "length"})
        tops.append({"column": key, "stretches": out})
        cc = lref.concentration(lengths, vols)
        if cc:
            crowd.append({"column": key, **cc})
    return {"volume": v["id"], "name": v["name"], "decimals": decimals, "columns": cols_out, "groups": groups,
            "residual": residual, "top": tops, "concentration": crowd, "gaps": report["gaps"]}, warnings


def cases():
    out = []
    V1, V2, START, CORR = vcmd.V1, vcmd.V2, vcmd.START, vcmd.CORRIDORS

    def ok(name, input_, volumes=START, corridors=CORR, op="execute"):
        o, warns = ledger(input_, volumes, corridors)
        result = completed(None, warns) if op == "validate" else completed(o, warns)
        default = volumes is START and corridors is CORR and ccmd.TEMPLATES == [vcmd.scmd.TK1, vcmd.scmd.TK2]
        s = step(op, input_, result, {**UNCHANGED, "road": vcmd.road_of(volumes, corridors)})
        s["within"] = W
        out.append(case(name, [s], None if default else vcmd.setup(volumes, corridors)))
        return o

    def no(name, input_, volumes=START, corridors=CORR, setup_=None):
        if setup_ is not None:
            result = vcmd.NOT_ROAD
        else:
            try:
                ledger(input_, volumes, corridors)
            except Refused as r:
                result = r.result
            else:
                raise AssertionError(name)
        road = None if setup_ is not None else vcmd.road_of(volumes, corridors)
        default = volumes is START and corridors is CORR
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                        setup_ or (None if default else vcmd.setup(volumes, corridors))))

    def v(**kw):
        o = copy.deepcopy(V1)
        for k, val in kw.items():
            if val is None:
                o.pop(k, None)
            else:
                o[k] = val
        return o

    by100 = {"kind": "stations", "step": 100.0}
    a = ok("100 m'lik dilimler", {"volume": "v-1", "by": by100})
    assert all(abs(x) < 1e-9 for x in a["residual"])
    ok("Bölgelere göre", {"volume": "Ana kübaj", "by": {"kind": "regions"}})
    ok("Yüzeyler arası kübaj, 50 m", {"volume": "v-2", "by": {"kind": "stations", "step": 50.0}})
    strips = [{"from": 0.0, "to": 300.0, "depth": 0.3}, {"from": 300.0, "to": 600.0, "depth": 0.2}]
    ok("Bitkisel toprak ve diş kazısı sütunları", {"volume": "v-1", "by": by100},
       [v(strip=strips, bench={"slope": 3.0, "height": 0.5}), V2])
    with vcmd.paved_templates():
        ok("Üstyapı tabakaları sütun", {"volume": "v-1", "by": {"kind": "regions"}})
    c_gap = {**ccmd.C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"},
                                    {"from": 250.0, "to": 400.0, "template": "tk-2"}]}
    ok("Koridorun boşluğu: bölgeler ayrı", {"volume": "v-1", "by": {"kind": "regions"}}, START, [c_gap, ccmd.C2])
    ok("En büyük üç aralık", {"volume": "v-1", "by": by100, "top": 3})
    ok("Tam sayıya yuvarlama", {"volume": "v-1", "by": by100, "decimals": 0})
    ok("Üç basamağa yuvarlama", {"volume": "v-1", "by": by100, "decimals": 3})
    ok("Prizmoidal", {"volume": "v-1", "by": by100}, [v(method="prismoidal"), V2])
    ok("Katsayısız: düzeltilmiş ham", {"volume": "v-1", "by": by100}, [v(factors=None), V2])
    ok("Uzun dilim: tek grup", {"volume": "v-1", "by": {"kind": "stations", "step": 10000.0}})
    ok("25 m'lik dilimler", {"volume": "v-1", "by": {"kind": "stations", "step": 25.0}, "top": 20})
    c_dense = {**ccmd.C1, "tangent": 20.0}
    ok("Onaydan sonra koridor sıklaştı", {"volume": "v-1", "by": by100}, [vcmd.approved(V1), V2], [c_dense, ccmd.C2])
    ok("Planı", {"volume": "v-1", "by": by100}, op="plan")
    ok("Doğrulama", {"volume": "v-2", "by": by100}, op="validate")
    no("Yol projesi değil", {"volume": "v-1", "by": by100}, setup_=vcmd.setup(work="map"))
    no("Olmayan kübaj", {"volume": "yok", "by": by100})
    no("Sıfır dilim", {"volume": "v-1", "by": {"kind": "stations", "step": 0.0}})
    no("Çok uzun dilim", {"volume": "v-1", "by": {"kind": "stations", "step": 200000.0}})
    no("Sıfır en büyük", {"volume": "v-1", "by": by100, "top": 0})
    no("Yüz bir en büyük", {"volume": "v-1", "by": by100, "top": 101})
    no("Dört basamak", {"volume": "v-1", "by": by100, "decimals": 4})
    broken_corr = {**ccmd.C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"},
                                          {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    no("Hesaplanamayan kübaj", {"volume": "v-1", "by": by100}, START, [broken_corr, ccmd.C2])
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.volumes.ledger", "commandVersion": 1,
            "title": "Kübaj defteri",
            "note": "scripts/fixtures/road_volume_ledger_command_cases.py yazar; kübajın raporu "
                    "road_volume_command_cases.py'nin, defter road_volume_ledger_cases.py'nin tam kesirli işlevleriyle; "
                    "KentOS kodu yok.",
            "setup": vcmd.setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")
    for c in data["cases"]:
        r = c["steps"][0]["result"]
        o = r.get("output")
        if o:
            cc = {x["column"]: round(x["gini"], 3) for x in o["concentration"]}
            t = o["top"][0]["stretches"][0] if o["top"] and o["top"][0]["stretches"] else None
            print(f"  {c['name'][:34]:34} gruplar {len(o['groups'])} sütun {len(o['columns'])} gini {cc} "
                  f"en büyük {t and (t['fromText'], round(t['share'], 3), t['reason'])} "
                  f"artık {[round(x, 3) for x in o['columns'][0:2] and [col['residue'] for col in o['columns'][:2]]]} "
                  f"{[w['code'] for w in r.get('warnings', [])]}")
        else:
            print(f"  {c['name'][:34]:34} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
