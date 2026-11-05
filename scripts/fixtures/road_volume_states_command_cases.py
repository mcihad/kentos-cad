#!/usr/bin/env python3
"""The shared cases of `road.volumes.states` (plan/road.md Y51, docs/adr/0957; ROAD-VOL-10).

The volumes' stretches are road_volume_command_cases.py's (no KentOS code); over them, here, each factor row's
earthwork in its three states by the conservation of mass: a cut of V m³ in place weighs ρ_bank·V t, loosened it is
V·ρ_bank/ρ_loose m³, compacted V·ρ_bank/ρ_compacted; a fill of V m³ compacted asks V·ρ_compacted/ρ_bank in place and
V·ρ_compacted/ρ_loose loosened. Where a row gives factors instead, the loosened volumes are the factors'. A factor below
1, a loose density above the bank's or a compacted one below the loose, is what no soil does: told.

The densities are the cases' own, of the magnitudes earthwork handbooks list for common earth and clay (a soil
report gives the project's).

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj --with shapely \\
        python3 scripts/fixtures/road_volume_states_command_cases.py [--check]
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.volumes.states.json"
completed, failed, step, case = vcmd.completed, vcmd.failed, vcmd.step, vcmd.case
UNCHANGED = vcmd.UNCHANGED
W = 1e-6
num = ccmd.num


def checks(cut, fill, d):
    out = []
    if d is not None:
        if d["loose"] > d["bank"]:
            out.append(f"gevşek yoğunluk ({num(d['loose'])} t/m³) yerindekinden ({num(d['bank'])} t/m³) büyük: "
                       "kazı gevşeyince küçülmez")
        if d["compacted"] < d["loose"]:
            out.append(f"sıkışık yoğunluk ({num(d['compacted'])} t/m³) gevşekten ({num(d['loose'])} t/m³) küçük: "
                       "sıkıştırılan malzeme gevşekten hafif olmaz")
        return out
    if cut is not None and cut < 1.0:
        out.append(f"kabarma 1'den küçük ({num(cut)}): gevşek hacim yerindekinden az olamaz")
    if fill is not None and fill < 1.0:
        out.append(f"dolgu katsayısı 1'den küçük ({num(fill)}): dolgunun istediği gevşek hacim sıkışık hacminden az "
                   "olamaz; katsayı yerinde hacme göre yazıldıysa kütle ordinatı kazıyı gevşek, dolguyu yerinde sayar")
    return out


def answer(input_, volumes=vcmd.START, corridors=vcmd.CORRIDORS):
    name = input_["volume"]
    v = next((x for x in volumes if x["id"] == name), None) or next(
        (x for x in volumes if x["name"].lower() == name.lower()), None)
    if v is None:
        raise vcmd.Refused("unknown_volume", f"“{name}” kimlikli ya da adlı kübaj hesabı projede yok.", "volume")
    rep, warnings = vcmd.compute(v, corridors)
    c = next(x for x in corridors if x["id"] == v["corridor"])
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    rows_def = v.get("factors", [])
    n = len(rows_def)

    def row_of(m):
        for i, f in enumerate(rows_def):
            if f["from"] <= m and (m < f["to"] or (i + 1 == n and m <= f["to"])):
                return i
        return None
    sums = [[0.0, 0.0] for _ in rows_def]
    un = [0.0, 0.0]
    for r in rep["rows"]:
        if r.get("cut") is None or r.get("fill") is None:
            continue
        k = row_of((r["from"] + r["to"]) / 2)
        target = un if k is None else sums[k]
        target[0] += r["cut"]
        target[1] += r["fill"]
    rows = []
    cut, fill, cut_loose, fill_loose = un[0], un[1], un[0], un[1]
    cut_mass = fill_mass = 0.0
    weighed = un[0] == 0.0 and un[1] == 0.0
    told = []
    for f, (c_, fl) in zip(rows_def, sums):
        d = f.get("densities")
        if d is not None:
            cf, ff = d["bank"] / d["loose"], d["compacted"] / d["loose"]
        else:
            cf, ff = f.get("cut"), f.get("fill")
        row = {"from": f["from"], "to": f["to"], "fromText": text(f["from"]), "toText": text(f["to"])}
        if "source" in f:
            row["source"] = f["source"]
        if d is not None:
            row["densities"] = d
        if cf is not None:
            row["cutFactor"] = cf
        if ff is not None:
            row["fillFactor"] = ff
        row.update({"cut": c_, "fill": fl, "cutLoose": c_ * (cf if cf is not None else 1.0),
                    "fillLoose": fl * (ff if ff is not None else 1.0)})
        if d is not None:
            row.update({"fillBank": fl * d["compacted"] / d["bank"], "cutCompacted": c_ * d["bank"] / d["compacted"],
                        "cutMass": c_ * d["bank"], "fillMass": fl * d["compacted"]})
            cut_mass += row["cutMass"]
            fill_mass += row["fillMass"]
        else:
            weighed = False
        cs = checks(cf, ff, d)
        if cs:
            row["checks"] = cs
        told += [f"km {row['fromText']} – {row['toText']}: {w}" for w in cs]
        cut += c_
        fill += fl
        cut_loose += row["cutLoose"]
        fill_loose += row["fillLoose"]
        rows.append(row)
    if told:
        warnings.append({"code": "volume_factor_physics", "path": "factors",
                         "message": f"“{v['name']}” kübajının katsayıları zeminin yapamayacağını söylüyor: {'; '.join(told)}."})
    out = {"volume": v["id"], "rows": rows, "unfactoredCut": un[0], "unfactoredFill": un[1], "cut": cut, "fill": fill,
           "cutLoose": cut_loose, "fillLoose": fill_loose, "balance": cut_loose - fill_loose}
    if weighed and n > 0:
        out["cutMass"], out["fillMass"] = cut_mass, fill_mass
    return out, warnings


def cases():
    out = []
    V1, V2 = vcmd.V1, vcmd.V2
    earth = {"bank": 1.8, "loose": 1.45, "compacted": 1.95}
    clay = {"bank": 1.75, "loose": 1.3, "compacted": 1.85}
    rock = {"bank": 2.6, "loose": 1.6, "compacted": 2.1}

    def ok(name, input_, volumes=vcmd.START, op="execute", corridors=vcmd.CORRIDORS):
        o, warns = answer(input_, volumes, corridors)
        result = completed(None, warns) if op == "validate" else completed(o, warns)
        s = step(op, input_, result, {**UNCHANGED, "road": vcmd.road_of(volumes, corridors)})
        s["within"] = W
        default = volumes is vcmd.START and corridors is vcmd.CORRIDORS
        out.append(case(name, [s], None if default else vcmd.setup(volumes, corridors)))

    def no(name, input_, volumes=vcmd.START, setup_=None, result=None):
        if result is None:
            try:
                answer(input_, volumes)
            except vcmd.Refused as r:
                result = r.result
            else:
                raise AssertionError(name)
        road = setup_["settings"].get("road") if setup_ is not None else vcmd.road_of(volumes)
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    dense = [{"from": 0.0, "to": 300.0, "densities": earth, "source": "Zemin etüdü raporu, SK-3 sondajı"},
             {"from": 300.0, "to": 600.0, "densities": clay, "source": "Zemin etüdü raporu, SK-7 sondajı"}]
    ok("Ana kübaj: sıkışma 0,9 zeminin yapamayacağını söylüyor", {"volume": "v-1"})
    ok("Yoğunluklu iki satır: kütleler", {"volume": "v-1"}, [{**V1, "factors": dense}, V2])
    ok("Yoğunluk ve katsayı karışık", {"volume": "Ana kübaj"},
       [{**V1, "factors": [dense[0], {"from": 300.0, "to": 600.0, "cut": 1.25, "fill": 1.08, "source": "KGM tablosu"}]}, V2])
    ok("Kaya: sıkışık yerindekinden hafif", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 0.0, "to": 600.0, "densities": rock}]}, V2])
    ok("Gevşek yoğunluk yerindekinden büyük", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 0.0, "to": 600.0, "densities": {"bank": 1.5, "loose": 1.7, "compacted": 1.9}}]}, V2])
    ok("Sıkışık gevşekten hafif", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 0.0, "to": 600.0, "densities": {"bank": 1.8, "loose": 1.6, "compacted": 1.5}}]}, V2])
    ok("Kabarma 1'den küçük", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 0.0, "to": 600.0, "cut": 0.95, "fill": 1.1}]}, V2])
    ok("Katsayısız kübaj", {"volume": "v-1"}, [{k: x for k, x in V1.items() if k != "factors"}, V2])
    ok("Kısmen katsayılı: kalan ham", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 100.0, "to": 300.0, "densities": earth}]}, V2])
    ok("Yalnız kabarma", {"volume": "v-1"}, [{**V1, "factors": [{"from": 0.0, "to": 600.0, "cut": 1.3}]}, V2])
    ok("Yalnız sıkışma", {"volume": "v-1"}, [{**V1, "factors": [{"from": 0.0, "to": 600.0, "fill": 1.15}]}, V2])
    ok("Yüzey yöntemli yan kübaj", {"volume": "v-2"}, [V1, {**V2, "factors": [{"from": 0.0, "to": 1000.0, "densities": clay}]}])
    ok("Prizmoidal ve yoğunluk", {"volume": "v-1"}, [{**V1, "method": "prismoidal", "factors": dense}, V2])
    ok("Üç satır", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 0.0, "to": 200.0, "densities": earth}, {"from": 200.0, "to": 400.0, "cut": 1.25, "fill": 1.1},
                           {"from": 400.0, "to": 600.0, "densities": clay}]}, V2])
    ok("Kaynaklı katsayı", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 0.0, "to": 600.0, "cut": 1.25, "fill": 1.1, "source": "Şartname eki, tablo 3"}]}, V2])
    ok("Eşit yoğunluklar: dönüşüm yok", {"volume": "v-1"},
       [{**V1, "factors": [{"from": 0.0, "to": 600.0, "densities": {"bank": 1.8, "loose": 1.8, "compacted": 1.8}}]}, V2])
    gap = {**ccmd.C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"}, {"from": 250.0, "to": 400.0, "template": "tk-2"}]}
    ok("Kesitsiz aralıklı koridorun kübajı", {"volume": "v-1"}, [{**V1, "factors": dense}, V2], corridors=[gap, ccmd.C2])
    ok("Planı", {"volume": "v-1"}, op="plan")
    ok("Doğrulama", {"volume": "v-1"}, op="validate")
    no("Yol projesi değil", {"volume": "v-1"}, setup_=vcmd.setup(work="map"), result=vcmd.NOT_ROAD)
    no("Olmayan kübaj", {"volume": "yok"})
    broken = {**ccmd.C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"}, {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    no("Kurulamayan koridorun kübajı", {"volume": "v-1"}, setup_=vcmd.setup(vcmd.START, [broken, ccmd.C2]),
       result=failed("invalid_volume", f"“{ccmd.C1['name']}” koridoru kurulamıyor: 1. ve 2. bölge kesişiyor; bölgeler en çok uç uca gelir.",
                     "corridor"))
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.volumes.states", "commandVersion": 1,
            "title": "Hacim halleri",
            "note": "scripts/fixtures/road_volume_states_command_cases.py yazar; kübajın aralıkları "
                    "road_volume_command_cases.py'nin, haller kütlenin korunmasıyla (yoğunluklar ya da katsayılar); "
                    "KentOS kodu yok. Yoğunluklar durumların kendi örnekleridir; projeninkileri zemin raporu verir.",
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
            print(f"  {c['name'][:40]:40} kazı {o['cut']:.0f} gevşek {o['cutLoose']:.0f} dolgu {o['fill']:.0f} "
                  f"gevşek {o['fillLoose']:.0f} denge {o['balance']:.0f} kütle {o.get('cutMass')} "
                  f"{[w['code'] for w in r.get('warnings', [])]}")
        else:
            print(f"  {c['name'][:40]:40} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
