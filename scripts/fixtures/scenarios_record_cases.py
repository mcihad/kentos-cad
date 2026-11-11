#!/usr/bin/env python3
"""The shared cases of `cad.scenarios.record` (Ölçüleri çizime yaz; plan/map-cad.md MAP-10, docs/adr/5060), without
KentOS code.

    python3 scripts/fixtures/scenarios_record_cases.py           # writes fixtures/commands/v1/cad.scenarios.record.json
    python3 scripts/fixtures/scenarios_record_cases.py --check   # writes nothing; compares

The comparison is `cad.scenarios.impact`'s independent reference (scenarios_impact_cases.py, the same folder). The
rules, by hand:
- the cells: the heading “Ölçü (<scenario>)”, Birim, Mevcut durum, Senaryo, Fark; a row a measure in the comparison's
  order: its label, its unit (m², m, %, adet), both sides and the difference, areas to the project's area decimals,
  lengths to its length decimals, shares to one, counts whole, a minus zero as zero, a difference above zero that is
  not zero as written with a plus; the last row “Damga”, the base's stamp and the scenario's;
- the table's attributes: SenaryoKimlik, TabanDamgasi, SenaryoDamgasi; its corner the one given, else two table
  heights (2.5 mm at 1:1000) right of the compared layers' objects, at their top;
- refused: the comparison's refusals with their paths under `impact.`, a layer not there, a group, a compared layer.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import scenarios_impact_cases as impact  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.scenarios.record.json"
SAME = {"revision": "same", "canUndo": False}
X0, Y0 = impact.X0, impact.Y0


def fixed(v, decimals):
    text = f"{v:.{decimals}f}"
    return text.lstrip("-") if set(text.lstrip("-")) <= set("0.") else text


def cells(r, s):
    area, length = s["settings"]["areaDecimals"], s["settings"]["lengthDecimals"]
    value = lambda unit, v: fixed(v, {"m2": area, "m": length, "%": 1}.get(unit, 0))  # noqa: E731
    unit = lambda u: {"m2": "m²", "m": "m", "%": "%"}.get(u, "adet")  # noqa: E731
    rows = [[f"Ölçü ({r['name']})", "Birim", "Mevcut durum", "Senaryo", "Fark"]]
    for m in r["measures"]:
        d = value(m["unit"], m["difference"])
        signed = "+" + d if m["difference"] > 0 and any(c.isdigit() and c != "0" for c in d) else d
        rows.append([m["label"], unit(m["unit"]), value(m["unit"], m["base"]), value(m["unit"], m["alternative"]), signed])
    rows.append(["Damga", "", r["stamp"], r["scenarioStamp"], ""])
    return rows


def written(name, s, impact_input, layer="0", at=None, steps_after=()):
    """A case that writes the table: the output, the table as an object (its fitted rows and columns any), then the
    comparison again, which reads it back as current."""
    given = {k: v for k, v in impact_input.items() if k != "scenario"}
    if "impervious" in given:
        given["impervious_"] = given.pop("impervious")
    r = impact.report(s, **given)
    input_ = {"impact": impact_input, "layerId": layer}
    if at is not None:
        input_["at"] = at
    corner = at or [X0 + 130 + 5.0, Y0 + 60]
    n = len(s["entities"]) + 1
    table = {"kind": "table", "id": n, "layerId": layer, "p": {"x": corner[0], "y": corner[1]}, "rotation": 0.0,
             "height": 2.5, "rows": "$any", "columns": "$any", "cells": cells(r, s), "aligns": ["left", "left", "right",
                                                                                                 "right", "right"],
             "header": True, "attrs": {"SenaryoDamgasi": r["scenarioStamp"], "SenaryoKimlik": "alt-a",
                                       "TabanDamgasi": r["stamp"]}}
    again = dict(r, records=[{"table": "$any", "stamp": r["stamp"], "scenarioStamp": r["scenarioStamp"],
                              "baseChanged": False, "scenarioChanged": False}])
    # The slope's parts come from an overlay of floats near 4 420 000 m: within a micrometre, as the comparison's cases.
    within = {"within": 1e-6} if "slope" in impact_input else {}
    return {"name": name, "setup": s, "steps": [
        {"op": "execute", "input": input_, "result": {"status": "completed", "output": {
            "table": "$any", "stamp": r["stamp"], "scenarioStamp": r["scenarioStamp"], "cells": cells(r, s),
            "revision": "$current"}, "warnings": []},
         "expect": {"canUndo": True, "revision": "changed", "entities": {str(n): table}}, **within},
        {"op": "plan", "command": "cad.scenarios.impact", "input": impact_input, "result": {
            "status": "completed", "output": again, "warnings": []}, "expect": {"revision": "same"}, **within},
        *steps_after]}


def refused(name, input_, code, message, path, s=None):
    case = {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": SAME}]}
    if s is not None:
        case["setup"] = s
    return case


def cases():
    base = impact.BASE_SETUP
    r = impact.report(base)
    return [
        {"name": "plan: hücreler ve damgalar, tablo yazılmaz", "steps": [{"op": "plan", "input": {
            "impact": {"scenario": "alt-a"}, "layerId": "0", "at": [X0 + 140, Y0 + 50]}, "result": {
            "status": "completed", "output": {"table": "", "stamp": r["stamp"], "scenarioStamp": r["scenarioStamp"],
                                              "cells": cells(r, base), "revision": "$current"}, "warnings": []},
            "expect": SAME}]},
        written("yaz: tablo 0 katmanında, verilen köşede; sonraki karşılaştırma onu güncel okur", base,
                {"scenario": "alt-a"}, at=[X0 + 140, Y0 + 50]),
        written("köşe verilmezse karşılaştırılan nesnelerin sağında, üstleri hizasında", base, {"scenario": "alt-a"},
                steps_after=[{"op": "undo", "returns": "Ölçüleri çizime yaz", "expect": {
                    "ids": list(range(1, len(base["entities"]) + 1)), "canUndo": False, "canRedo": True}}]),
        written("geçirimsizlik ve erişim ölçüleriyle", base,
                {"scenario": "alt-a", "impervious": impact.SHARES, "access": impact.ACCESS}, at=[X0 + 140, Y0 + 50]),
        written("eğim, sınıflar ve bakıyla", impact.SLOPE_SETUP,
                {"scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0, "classes": [5.0, 15.0, 30.0],
                                                "aspect": True}}, at=[X0 + 140, Y0 + 50]),
        written("altyapıya yakınlıkla", impact.INFRA_SETUP,
                {"scenario": "alt-a", "infrastructure": {"layer": "altyapi", "distance": 40.0}}, at=[X0 + 140, Y0 + 50]),
        written("ağ üzerinden erişimle", impact.NET_SETUP, {"scenario": "alt-a", "access": impact.NET_ACCESS},
                at=[X0 + 140, Y0 + 50]),
        written("aynı senaryo: farklar sıfır", impact.setup(alt=[dict(e, layerId={"karar": "alt-a-karar", "yol": "alt-a-yol"}[
            e["layerId"]]) for e in impact.BASE]), {"scenario": "alt-a"}, at=[X0 + 140, Y0 + 50]),
        stale_after_delete(),
        refused("tablo karşılaştırılan katmana yazılmaz", {"impact": {"scenario": "alt-a"}, "layerId": "karar"},
                "invalid_input", "Ölçü tablosu karşılaştırılan “Karar alanları” katmanına yazılamaz: tablo o katmanın "
                "ölçülerini değiştirir. Başka bir katman seçin.", "layerId"),
        refused("senaryonun katmanına da yazılmaz", {"impact": {"scenario": "alt-a"}, "layerId": "alt-a-yol"},
                "invalid_input", "Ölçü tablosu karşılaştırılan “Yollar” katmanına yazılamaz: tablo o katmanın "
                "ölçülerini değiştirir. Başka bir katman seçin.", "layerId"),
        refused("katman çizimde yok", {"impact": {"scenario": "alt-a"}, "layerId": "notlar"}, "invalid_input",
                "“notlar” kimlikli katman çizimde yok; tablonun yazılacağı bir katmanın kimliğini verin.", "layerId"),
        refused("grup katman değildir", {"impact": {"scenario": "alt-a"}, "layerId": "imar"}, "invalid_input",
                "“imar” kimlikli katman çizimde yok; tablonun yazılacağı bir katmanın kimliğini verin.", "layerId"),
        refused("karşılaştırmanın reddi impact altında", {"impact": {"scenario": "alt-z"}, "layerId": "0"},
                "scenario_not_found", "“alt-z” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin.",
                "impact.scenario"),
        refused("karşılaştırmanın değer reddi impact altında", {"impact": {"scenario": "alt-a", "access": {
            "distance": 0.0, "from": ["konut"], "to": ["park"]}}, "layerId": "0"}, "invalid_input",
                "Erişim uzaklığı 0'dan büyük olmalı (yazılan 0).", "impact.access.distance"),
        refused("yerine geçen katmanı olmayan senaryo", {"impact": {"scenario": "alt-a"}, "layerId": "0"},
                "nothing_to_compare", "“Alternatif A” senaryosunun hiçbir katmanı bir ana katmanın yerine geçmiyor; "
                "karşılaştırılacak bir şey yok.", "impact.scenario", s=impact.setup(layers=no_pairs())),
        refused("eğim yüzeyi projede yok", {"impact": {"scenario": "alt-a", "slope": {"surface": "yok", "threshold": 10.0}},
                                            "layerId": "0"},
                "unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "impact.slope.surface",
                s=impact.SLOPE_SETUP),
        refused("eğim Harita ve Yol projelerinde", {"impact": {"scenario": "alt-a", "slope": {"surface": "t-1",
                                                                                             "threshold": 10.0}},
                                                    "layerId": "0"},
                "not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim "
                "ikisinden biri değil.", "impact",
                s=impact.setup(layers=impact.tree(ground=True), extra=impact.ground_points(), work="architecture")),
        refused("ağ projede yok", {"impact": {"scenario": "alt-a", "access": dict(impact.NET_ACCESS, network="patika")},
                                   "layerId": "0"},
                "unknown_network", "“patika” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin.",
                "impact.access.network", s=impact.NET_SETUP),
        refused("beklenen sürüm sayı değil", {"impact": {"scenario": "alt-a"}, "layerId": "0", "expectedRevision": "dün"},
                "invalid_revision", "Beklenen sürüm “dün” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "çizim değişmişse yazılmaz", "steps": [{"op": "execute", "input": {
            "impact": {"scenario": "alt-a"}, "layerId": "0", "expectedRevision": "999"}, "result": {
            "status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra "
                                            "değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden "
                                            "hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
            "expect": SAME}]},
        refused("kilitli katmana yazılmaz", {"impact": {"scenario": "alt-a"}, "layerId": "0"}, "layer_locked",
                "“0” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId",
                s=impact.setup(layers=locked_zero())),
    ]


def no_pairs():
    t = impact.tree()
    for c in t[2]["children"]:
        c.pop("replaces")
    return t


def locked_zero():
    t = impact.tree()
    t[0]["locked"] = True
    return t


def stale_after_delete():
    """The table written, then the base's road removed: the comparison reads the table back as stale (its base
    changed), its numbers as they were."""
    base = impact.BASE_SETUP
    case = written("taban değişince çizimdeki tablo eskir: tabanın yolu silindi", base, {"scenario": "alt-a"},
                   at=[X0 + 140, Y0 + 50])
    before = impact.report(base)
    road = next(e for e in base["entities"] if e["layerId"] == "yol")
    after = dict(base, entities=[e for e in base["entities"] if e is not road]
                 + [{"kind": "table", "id": len(base["entities"]) + 1, "layerId": "0",
                     "attrs": {"SenaryoKimlik": "alt-a", "TabanDamgasi": before["stamp"],
                               "SenaryoDamgasi": before["scenarioStamp"]}}])
    now = impact.report(after)
    for r in now["records"]:
        r["table"] = "$any"
    assert now["records"][0]["baseChanged"] and not now["records"][0]["scenarioChanged"], now["records"]
    case["steps"] += [
        {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [impact.uid(road["id"])]}, "result": {
            "status": "completed", "output": {"removed": [impact.uid(road["id"])], "locked": [], "revision": "$current"},
            "warnings": []}, "expect": {"revision": "changed"}},
        {"op": "plan", "command": "cad.scenarios.impact", "input": {"scenario": "alt-a"}, "result": {
            "status": "completed", "output": now, "warnings": []}, "expect": {"revision": "same"}}]
    return case


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.scenarios.record", "commandVersion": 1,
           "title": "Ölçüleri çizime yaz",
           "note": "ADR 5060. Bağımsız başvuru scripts/fixtures/scenarios_record_cases.py: karşılaştırma "
                   "scenarios_impact_cases.py'den, hücreler ve damgalar elle; KentOS kodu yok.",
           "setup": impact.BASE_SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Ölçüleri çizime yaz ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
