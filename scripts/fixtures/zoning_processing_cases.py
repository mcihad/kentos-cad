#!/usr/bin/env python3
"""İmar planını denetle's cases from İşlem araç kutusu (docs/adr/3840; plan/map-cad.md MAP-09), without KentOS code.

    python3 scripts/fixtures/zoning_processing_cases.py           # writes fixtures/processing/v1/zoning*.{json,kcad}
    python3 scripts/fixtures/zoning_processing_cases.py --check   # writes nothing; compares

The drawings and what the check finds are `cad.zoning.check`'s shared cases (fixtures/commands/v1/cad.zoning.check.json,
written by scripts/fixtures/zoning_check_cases.py, its own independent reference). Here, by hand, what the tool makes of
them: the table's rows (the kind in words, the objects by their ids in the drawing, the area to the drawing's decimals,
the regions' count, the message), the objects selected in order, and, asked to write, each region an area on the error
layer (its fields; its area by the shoelace, the holes taken off; its holes counted).
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/commands/v1/cad.zoning.check.json"
DIR = ROOT / "fixtures/processing/v1"
OUT = DIR / "zoning.json"
TOOL = "zoning.check"
LABEL = "İmar planını denetle"
LAYER = "İmar bulguları"

ROADS = "yol genişliği çizilenden geniş: yola giren kararlar ve parseller"
U_ROAD = "U biçimli yol karara iki yerden girer"

KIND = {
    "missingLayer": "Katman yok", "noLevel": "Kademe yok", "upperMissing": "Üst plan yok",
    "levelOrder": "Kademe sırası", "outsideUpper": "Üst plan dışına taşma", "noPlan": "Planı yok",
    "noUse": "Kullanım yok", "unknownUse": "Bilinmeyen kullanım", "missingValue": "Değer verilmemiş",
    "declaredValued": "Değer çelişkisi", "kaksReview": "KAKS gözden geçir", "overlap": "Karar çelişkisi",
    "uncovered": "Kararsız kısım", "outside": "Plan dışına taşma", "roadGap": "Kopuk yol",
    "roadWidth": "Yola giren karar", "roadParcel": "Yola giren parsel", "noteMissing": "Plan notu yok",
    "noteUnused": "Anılmayan not",
}


def case_of(name):
    cases = json.loads(SOURCE.read_text("utf-8"))["cases"]
    return next(c for c in cases if c["name"].startswith(name))


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


def shoelace(pts):
    return sum(a["x"] * b["y"] - b["x"] * a["y"] for a, b in zip(pts, pts[1:] + pts[:1])) / 2.0


def ring_area(ring):
    """A ring's signed area; its edges straight (the cases' roads are straight)."""
    assert not any(ring.get("bulges") or []), "a straight ring"
    pts = ring["pts"]
    o = pts[0]
    rel = [{"x": p["x"] - o["x"], "y": p["y"] - o["y"]} for p in pts]
    return shoelace(rel)


def area_text(a, decimals):
    return f"{a:.{decimals}f} m²"


def expected(name, write):
    c = case_of(name)
    setup = c["setup"]
    out = c["steps"][0]["result"]["output"]
    decimals = setup["settings"]["areaDecimals"]
    ids = {u: e["id"] for u, e in zip(setup["uids"], setup["entities"])}
    findings = out["findings"]
    rows = [[KIND[f["kind"]], ", ".join(f"#{ids[u]}" for u in f["uids"]),
             area_text(f["area"], decimals) if f.get("area") is not None else "",
             str(len(f["regions"])) if f.get("regions") else "", f["message"]] for f in findings]
    objects = sorted({ids[u] for f in findings for u in f["uids"]})
    n = len(findings)
    said = f"İmar denetimi {out['plans']} planda, {out['decisions']} karar alanında {n} bulgu verdi"
    shapes = []
    if write:
        for f in findings:
            regions = f.get("regions") or []
            for k, r in enumerate(regions):
                attrs = {"Tür": KIND[f["kind"]], "Nesneler": ", ".join(f["uids"]), "Bölge": f"{k + 1}/{len(regions)}",
                         "İleti": f["message"]}
                if f.get("area") is not None:
                    attrs["Bulgu alanı"] = area_text(f["area"], decimals)
                holes = r.get("holes") or []
                area = abs(ring_area(r["outer"])) - sum(abs(ring_area(h)) for h in holes)
                shapes.append({"kind": "polygon", "layerId": layer_id(LAYER), "attrs": attrs, "parts": 1,
                               "holes": len(holes), "area": round(area, 9)})
        placed = sum(1 for f in findings if f.get("regions"))
        wrote = len(shapes)
        summary = f"{said}; {placed} bulgunun {wrote} bölgesi hata katmanına yazıldı."
        if placed < n:
            summary = f"{said}; {placed} bulgunun {wrote} bölgesi hata katmanına yazıldı, {n - placed} bulgunun yeri yok (tabloda)."
    else:
        summary = f"{said}; bulgulu nesneler seçildi."
    expect = {"status": "ok", "summary": summary, "undo": LABEL if write else None,
              "layers": [{"id": layer_id(LAYER), "name": LAYER, "style": {"color": "#E5484D", "lineType": "continuous",
                                                                         "lineWeight": 0.35, "fill": "#E5484D26"}}] if write else [],
              "addedShapes": shapes,
              "outputs": {"count": n, "objects": objects,
                          "findings": {"columns": ["Tür", "Nesneler", "Alan", "Bölge", "İleti"], "rows": rows}}}
    if not write:
        expect["selection"] = objects
    return expect


def document_name(name):
    return {ROADS: "zoning-yollar.kcad", U_ROAD: "zoning-u-yol.kcad"}[name]


def build():
    cases = [
        {"id": "zoning-check", "title": "İmar planını denetle: yola giren dört karar ve üç parsel tabloda, alanlarıyla; bulgulu nesneler seçilir, çizim değişmez",
         "document": document_name(ROADS), "run": {"tool": TOOL}, "values": {}, "expect": expected(ROADS, False)},
        {"id": "zoning-check-errors", "title": "İmar planını denetle: yola giren her bölge hata katmanına alan olarak yazılır, tek geri alma adımı",
         "document": document_name(ROADS), "run": {"tool": TOOL}, "values": {"write": True, "layer": {"newName": LAYER}},
         "expect": expected(ROADS, True)},
        {"id": "zoning-check-u-road", "title": "İmar planını denetle: U biçimli yolun karara iki yerden girdiği tek bulgu iki alan olur (1/2, 2/2)",
         "document": document_name(U_ROAD), "run": {"tool": TOOL}, "values": {"write": True, "layer": {"newName": LAYER}},
         "expect": expected(U_ROAD, True)},
    ]
    documents = {}
    for name in (ROADS, U_ROAD):
        s = case_of(name)["setup"]["settings"]
        documents[document_name(name)] = {
            "defaults": {"lengthDecimals": s["lengthDecimals"], "areaDecimals": s["areaDecimals"], "angleUnit": s["angleUnit"],
                         "plotScale": s["plotScale"], "drawingFont": "barlow",
                         "activeLayer": case_of(name)["setup"]["activeLayer"], "measureHeightMm": 2},
            "tools": {TOOL: {"tolerance": 0.01, "write": False, "layer": {"newName": LAYER}}},
        }
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": 1e-6, "measureTolerance": 1e-6,
            "documents": documents, "cases": cases}


def main():
    outs = [(OUT, json.dumps(build(), ensure_ascii=False, indent=1) + "\n")]
    for name in (ROADS, U_ROAD):
        outs.append((DIR / document_name(name), json.dumps(case_of(name)["setup"], ensure_ascii=False, indent=1) + "\n"))
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil", file=sys.stderr)
        if not bad:
            print("İmar planını denetle'nin işlem durumları güncel.")
        return 1 if bad else 0
    for p, t in outs:
        p.write_text(t, "utf-8")
    print(f"yazıldı: {', '.join(str(p.relative_to(ROOT)) for p, _ in outs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
