#!/usr/bin/env python3
"""Altyapı bağlarını çiz's cases from İşlem araç kutusu (docs/adr/3860; plan/map-cad.md MAP-10), without KentOS code.

    python3 scripts/fixtures/scenario_links_processing_cases.py           # writes fixtures/processing/v1/scenario-links.*
    python3 scripts/fixtures/scenario_links_processing_cases.py --check   # writes nothing; compares

The drawing and the links are `cad.scenarios.impact`'s shared cases (fixtures/commands/v1/cad.scenarios.impact.json,
written by scripts/fixtures/scenarios_impact_cases.py, its own independent reference: each area's centroid and its
foot on the nearest line). Here, by hand, what the tool makes of them: the table's rows, the summary, and each link a
line from the centroid to the nearest place on the new layer, its fields and its length.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/commands/v1/cad.scenarios.impact.json"
DIR = ROOT / "fixtures/processing/v1"
OUT = DIR / "scenario-links.json"
DOCUMENT = "scenario-links.kcad"
TOOL = "scenarios.links"
LABEL = "Altyapı bağlarını çiz"
LAYER = "Altyapı bağları"
ALL = "altyapıya yakınlık: kanalizasyon hattına 40 m"
HOMES = "altyapıya yakınlık yalnız konut alanlarında"


def case_of(name):
    return next(c for c in json.loads(SOURCE.read_text("utf-8"))["cases"] if c["name"].startswith(name))


def layer_id(name):
    """A new layer's id as the runner makes it: “islem-” and the name folded, lower case, other characters as “-”."""
    fold = str.maketrans("çğıöşüÇĞİÖŞÜâîû", "cgiosuCGIOSUaiu")
    out, dash = "", False
    for ch in name.translate(fold).lower():
        if "a" <= ch <= "z" or "0" <= ch <= "9":
            out += ch
            dash = False
        elif not dash:
            out += "-"
            dash = True
    return "islem-" + out


def length(m, decimals):
    # Not at a half of the last place, where two roundings could part.
    assert abs(m * 10 ** decimals - int(m * 10 ** decimals) - 0.5) > 1e-6, m
    return f"{m:.{decimals}f} m"


def expected(name, side):
    c = case_of(name)
    out = c["steps"][0]["result"]["output"]
    decimals = c["setup"]["settings"]["lengthDecimals"]
    links = []
    if side != "alternative":
        links += [("Mevcut durum", l) for l in out["near"]["baseLinks"]]
    if side != "base":
        links += [("Senaryo", l) for l in out["near"]["alternativeLinks"]]
    rows, shapes = [], []
    for word, l in links:
        drawn = l["distance"] > 0
        rows.append([word, l["area"], l["line"], length(l["distance"], decimals), "evet" if drawn else "boyu yok"])
        if drawn:
            attrs = {"Taraf": word, "Alan": l["area"], "Hat": l["line"], "Uzaklık": length(l["distance"], decimals)}
            shapes.append({"kind": "line", "layerId": layer_id(LAYER), "attrs": attrs, "parts": 1, "holes": 0,
                           "length": l["distance"], "vertices": [{"x": l["from"][0], "y": l["from"][1]},
                                                                 {"x": l["to"][0], "y": l["to"][1]}]})
    wrote, flat = len(shapes), len(links) - len(shapes)
    summary = f"“{out['name']}”: {len(links)} alan en yakın altyapı nesnesine bağlandı; {wrote} bağ yazıldı"
    if flat:
        summary += f", {flat} alanın merkezi hattın üstünde (bağı çizilmedi)"
    summary += "."
    return {"status": "ok", "summary": summary, "undo": LABEL if wrote else None,
            "layers": [{"id": layer_id(LAYER), "name": LAYER, "style": {"color": "#0090FF", "lineType": "continuous",
                                                                       "lineWeight": 0.25}}] if wrote else [],
            "addedShapes": shapes,
            "outputs": {"count": wrote, "links": {"columns": ["Taraf", "Alan", "Hat", "Uzaklık", "Yazıldı"], "rows": rows}}}


def values(name, side):
    infra = case_of(name)["steps"][0]["input"]["infrastructure"]
    return {"scenario": "Alternatif A", "infrastructure": {"scope": "layer", "layerId": infra["layer"]},
            "distance": infra["distance"], "uses": ", ".join(infra.get("from", [])), "side": side,
            "layer": {"newName": LAYER}}


def build():
    setup = case_of(ALL)["setup"]
    assert case_of(HOMES)["setup"] == setup, "one drawing"
    s = setup["settings"]
    cases = [
        {"id": "scenario-links", "title": "Altyapı bağlarını çiz: iki taraftaki on bir alan kanalizasyon hattının en yakın yerine bağlanır, tek geri alma adımı",
         "document": DOCUMENT, "run": {"tool": TOOL}, "values": values(ALL, "both"), "expect": expected(ALL, "both")},
        {"id": "scenario-links-homes", "title": "Altyapı bağlarını çiz: yalnız konut alanları, yalnız senaryo tarafı",
         "document": DOCUMENT, "run": {"tool": TOOL}, "values": values(HOMES, "alternative"),
         "expect": expected(HOMES, "alternative")},
    ]
    documents = {DOCUMENT: {
        "defaults": {"lengthDecimals": s["lengthDecimals"], "areaDecimals": s["areaDecimals"], "angleUnit": s["angleUnit"],
                     "plotScale": s["plotScale"], "drawingFont": "barlow", "activeLayer": setup["activeLayer"],
                     "measureHeightMm": 2},
        "tools": {TOOL: {"scenario": "", "infrastructure": {"scope": "layer", "layerId": setup["activeLayer"]},
                         "distance": 0.0, "uses": "", "side": "both", "layer": {"newName": LAYER}}},
    }}
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": 1e-6, "measureTolerance": 1e-6,
            "documents": documents, "cases": cases}


def main():
    outs = [(OUT, json.dumps(build(), ensure_ascii=False, indent=1) + "\n"),
            (DIR / DOCUMENT, json.dumps(case_of(ALL)["setup"], ensure_ascii=False, indent=1) + "\n")]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil", file=sys.stderr)
        if not bad:
            print("Altyapı bağlarını çiz'in işlem durumları güncel.")
        return 1 if bad else 0
    for p, t in outs:
        p.write_text(t, "utf-8")
    print(f"yazıldı: {', '.join(str(p.relative_to(ROOT)) for p, _ in outs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
