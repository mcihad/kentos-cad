#!/usr/bin/env python3
"""The shared cases of `road.volumes.history` (plan/road.md Y44, docs/adr/0950; ROAD-VOL-17).

The volumes, their approvals (numbered, ended into the history by a revocation or by the next approval) and today's
figures are road_volume_command_cases.py's (its own reference, no KentOS code); each approval is set against the one
before it and today against the last, by the same stretch matching the approval's comparison uses (stations within
1e-6 m, figures beyond 1e-6 m³).

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_volume_history_command_cases.py [--check]
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.volumes.history.json"
completed, failed, step, case = vcmd.completed, vcmd.failed, vcmd.step, vcmd.case
UNCHANGED = vcmd.UNCHANGED
W = 1e-6
SAME = 1e-6


def change(before, after):
    """`after` against `before`, each (totals [cut, fill, factoredCut, factoredFill], rows [from, to, cut, fill])."""
    (bt, br), (at, ar) = before, after
    same = lambda x, y: abs(x - y) <= SAME  # noqa: E731
    rows, used = [], [False] * len(ar)
    for p in br:
        i = next((k for k, t in enumerate(ar) if same(t[0], p[0]) and same(t[1], p[1])), None)
        if i is None:
            rows.append({"from": p[0], "to": p[1], "approvedCut": p[2], "approvedFill": p[3]})
            continue
        used[i] = True
        t = ar[i]
        if not same(t[2], p[2]) or not same(t[3], p[3]):
            rows.append({"from": p[0], "to": p[1], "approvedCut": p[2], "approvedFill": p[3], "cut": t[2], "fill": t[3]})
    rows += [{"from": t[0], "to": t[1], "cut": t[2], "fill": t[3]} for t, u in zip(ar, used) if not u]
    rows.sort(key=lambda r: (r["from"], r["to"]))
    ch = [a - b for a, b in zip(at, bt)]
    return {"changed": bool(rows) or any(abs(c) > SAME for c in ch), "cutChange": ch[0], "fillChange": ch[1],
            "factoredCutChange": ch[2], "factoredFillChange": ch[3], "rows": rows}


def of_approval(a):
    return [a["cut"], a["fill"], a["factoredCut"], a["factoredFill"]], a["rows"]


def history(v, corridors):
    approvals = [(a, False) for a in v.get("history", [])] + ([(v["approved"], True)] if v.get("approved") else [])
    entries = []
    for i, (a, standing) in enumerate(approvals):
        e = {"approval": a, "standing": standing}
        if i > 0:
            e["against"] = change(of_approval(approvals[i - 1][0]), of_approval(a))
        entries.append(e)
    out = {"volume": v["id"], "name": v["name"], "approvals": entries}
    try:
        r, _ = vcmd.compute(v, corridors)
    except vcmd.Refused as e:
        out["problem"] = e.result["error"]["message"]
        return out
    out["today"] = {"cut": r["cut"], "fill": r["fill"], "factoredCut": r["factoredCut"], "factoredFill": r["factoredFill"]}
    if approvals:
        today = [[x["from"], x["to"], x["cut"], x["fill"]] for x in r["rows"] if "cut" in x]
        out["since"] = change(of_approval(approvals[-1][0]), ([r["cut"], r["fill"], r["factoredCut"], r["factoredFill"]],
                                                               today))
    return out


def cases():
    out = []
    V1, V2, START, CORR = vcmd.V1, vcmd.V2, vcmd.START, vcmd.CORRIDORS
    dense = [{**ccmd.C1, "tangent": 20.0}, ccmd.C2]
    short = [{**ccmd.C1, "regions": [{"from": 0.0, "to": 300.0, "template": "tk-1"}]}, ccmd.C2]

    def ok(name, input_, volumes=START, corridors=CORR, op="execute"):
        v = next(x for x in volumes if x["id"] == input_["volume"] or x["name"].lower() == input_["volume"].lower())
        o = history(v, corridors)
        result = completed(None, []) if op == "validate" else completed(o, [])
        default = volumes is START and corridors is CORR
        s = step(op, input_, result, {**UNCHANGED, "road": vcmd.road_of(volumes, corridors)})
        s["within"] = W
        out.append(case(name, [s], None if default else vcmd.setup(volumes, corridors)))
        return o

    def no(name, input_, result, setup_=None):
        road = None if setup_ is not None else vcmd.road_of(START)
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    a1 = vcmd.approved(V1, "2026-10-01", "1. hakediş")
    ok("Onaysız kübaj: geçmiş yok", {"volume": "v-1"})
    o = ok("Tek onay, bugün aynı", {"volume": "v-1"}, [a1, V2])
    assert not o["since"]["changed"]
    o = ok("Onaydan sonra koridor sıklaştı", {"volume": "v-1"}, [a1, V2], dense)
    assert o["since"]["changed"] and o["since"]["rows"]
    a2 = vcmd.approved(a1, "2026-10-15", "2. hakediş", corridors=dense)
    o = ok("İki onay: ikincisi sıklaşan koridorla", {"volume": "v-1"}, [a2, V2], dense)
    assert o["approvals"][1]["against"]["changed"] and not o["since"]["changed"]
    a3 = vcmd.approved(a2, "2026-11-01", "3. hakediş", corridors=short)
    ok("Üç onay: bölge kısaldı, kalkan aralıklar", {"volume": "v-1"}, [a3, V2], short)
    ok("Üç onay, bugün eski koridorla", {"volume": "v-1"}, [a3, V2])
    r1 = vcmd.revoked(a1, "2026-10-10", "Zemin raporu değişti")
    o = ok("Kaldırılmış onay: geçerli onay yok", {"volume": "v-1"}, [r1, V2])
    assert not o["approvals"][0]["standing"]
    ok("Kaldırılmış onay, koridor sıklaşınca", {"volume": "v-1"}, [r1, V2], dense)
    ok("Kaldırıldıktan sonra yeniden onay", {"volume": "v-1"}, [vcmd.approved(r1, "2026-10-12", corridors=dense), V2], dense)
    legacy = {**a1, "approved": {k: val for k, val in a1["approved"].items() if k != "number"}}
    ok("Numarasız eski onay", {"volume": "v-1"}, [legacy, V2])
    ok("Numarasız eski onayın yerine gelen", {"volume": "v-1"}, [vcmd.approved(legacy, "2026-10-20", corridors=dense), V2],
       dense)
    a2v = vcmd.approved(V2, "2026-10-30")
    ok("Yüzeyler arası kübajın onayı, adıyla", {"volume": "YAN kübaj"}, [V1, a2v])
    ok("Yüzeyler arası kübajın kaldırılan onayı", {"volume": "v-2"}, [V1, vcmd.revoked(a2v, "2026-11-02")])
    vs = {**V1, "strip": [{"from": 0.0, "to": 600.0, "depth": 0.3}], "bench": {"slope": 3.0, "height": 0.5}}
    ok("Sıyırma ve diş kazısıyla onaylı", {"volume": "v-1"}, [vcmd.approved(vs, "2026-10-05"), V2])
    many = V1
    for k, (day, corr) in enumerate([("2026-09-01", CORR), ("2026-09-15", dense), ("2026-10-01", CORR),
                                     ("2026-10-15", short), ("2026-11-01", CORR)]):
        many = vcmd.approved(many, day, f"{k + 1}. hakediş", corridors=corr)
    o = ok("Beş hakediş", {"volume": "v-1"}, [many, V2])
    assert [e["approval"]["number"] for e in o["approvals"]] == [1, 2, 3, 4, 5]
    broken_corr = {**ccmd.C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"},
                                          {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    o = ok("Bugün hesaplanmayan kübajın geçmişi", {"volume": "v-1"}, [a2, V2], [broken_corr, ccmd.C2])
    assert "problem" in o and "since" not in o
    ok("Planı", {"volume": "v-1"}, [a2, V2], dense, op="plan")
    ok("Doğrulama", {"volume": "v-1"}, [a2, V2], dense, op="validate")
    no("Yol projesi değil", {"volume": "v-1"}, vcmd.NOT_ROAD, vcmd.setup(work="map"))
    no("Olmayan kübaj", {"volume": "yok"},
       failed("unknown_volume", "“yok” kimlikli ya da adlı kübaj hesabı projede yok.", "volume"))
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.volumes.history", "commandVersion": 1,
            "title": "Onay geçmişi",
            "note": "scripts/fixtures/road_volume_history_command_cases.py yazar; kübajlar, onayları ve bugünkü hesap "
                    "road_volume_command_cases.py'nin; her onay öncekiyle, bugün son onayla aynı aralık eşlemesiyle "
                    "karşılaştırılır. KentOS kodu yok.",
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
            ap = [(e["approval"].get("number", 0), e["standing"], e.get("against", {}).get("changed"),
                   len(e.get("against", {}).get("rows", []))) for e in o["approvals"]]
            s = o.get("since")
            print(f"  {c['name'][:42]:42} {ap} bugün {s and (s['changed'], round(s['cutChange'], 2), len(s['rows']))} "
                  f"{'sorun' if 'problem' in o else ''}")
        else:
            print(f"  {c['name'][:42]:42} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
