#!/usr/bin/env python3
"""The EK-3 and EK-4 rules' data for the standards package (docs/adr/8064, plan/bim-plan.md BIM-03; the sheets-from-the-model
rules' sources too, docs/adr/8065), taken from KentOS's
rule catalog (crates/shared/digital-project/data/temel-2026.json, docs/adr/8062): the disciplines and each one's layer
template, the items and sub-disciplines, EK-3 Tablo 3.2's room names and abbreviations, the dimension typefaces of
4.21.2, and each rule's sources cited as the catalog cites them. The package's rule reads this file; it is never edited
by hand.

    python3 scripts/mevzuat/ek34_veri.py          # writes catalog/standards/dijital-proje/src/dijital_proje/data/ek34-2026.json
    python3 scripts/mevzuat/ek34_veri.py --check  # fails when the file is not what the catalog gives
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / "crates" / "shared" / "digital-project" / "data" / "temel-2026.json"
IFC_CATALOG = ROOT / "crates" / "shared" / "ifc" / "data" / "temel-2026.json"
OUT = ROOT / "catalog" / "standards" / "dijital-proje" / "src" / "dijital_proje" / "data" / "ek34-2026.json"

# Each discipline's layer template (EK-4 4.14–4.16): Tablo 4.1 for architecture, interior, survey and soil; 4.2 for
# structure, mechanical, electrical and infrastructure; 4.3 for landscape. The others (fire, solar, acoustics) have none.
LAYER_TEMPLATES = {"T4.1": ["MM", "IM", "HR", "ZE"], "T4.2": ["ST", "MK", "EE", "AL"], "T4.3": ["PY"]}

# The sources each rule cites, by catalog row.
RULES = {
    "number_pattern": ["M3.1.1", "T3.1"],
    "storey_code": ["M3.2.1"],
    "sequence": ["M3.2.3"],
    "clockwise": ["M3.2.3"],
    "room_name": ["M3.3", "T3.2"],
    "layer_name": ["M4.11", "T4.1"],
    "layer_zero": ["M4.12.1"],
    "hatch_layer": ["M4.14.5"],
    "emsal_layer": ["M4.14.2"],
    "axis_line": ["M4.29"],
    "dimension_font": ["M4.21.2"],
    "block_name": ["M4.18", "T4.4"],
    # The sheets drawn from the model (Md. 9/1; EK-9 Tablo 9.1 satır 21, docs/adr/8065).
    "derived_stale": ["T9.1/21"],
    "derived_gone": ["T9.1/21"],
    "free_on_model_layer": ["T9.1/21"],
    "annotation_broken": ["T9.1/21"],
    # The permit package (Md. 4/2, 8; EK-2; docs/adr/8066): the regulation's own articles are cited by the rule.
    "package_name": ["M2.1", "M2.1.1", "T2.1"],
    "package_one_sheet": [],
    "package_paper": [],
    "package_pdfa": [],
}


def pages(ps):
    ps = sorted(set(ps))
    if not ps:
        return ""
    if len(ps) == 1:
        return str(ps[0])
    if ps[-1] - ps[0] + 1 == len(ps):
        return f"{ps[0]}–{ps[-1]}"
    return ", ".join(map(str, ps))


def cite(catalog, row_id, ifc=None):
    """As `kentos_digital_project::Source::cite` says it: “EK-3 m. 3.2.3 · PDF s. 14”, “EK-3 Tablo 3.1 · PDF s. 14”,
    “EK-9 Tablo 9.1 satır 21 · PDF s. 239”."""
    for q in (ifc or {}).get("questions", []):
        if q["id"] == row_id:
            return f"EK-9 Tablo {q['form']} satır {q['no']} · PDF s. {pages(q['pages'])}"
    for c in catalog["clauses"]:
        if c["id"] == row_id:
            return f"EK-{c['annex']} m. {c['clause']} · PDF s. {pages(c['pages'])}"
    for t in catalog["templates"]:
        if t["id"] == row_id:
            return f"EK-{t['table'].split('.')[0]} Tablo {t['table']} · PDF s. {t['page']}"
    if row_id == "T3.2":
        rows = [c for c in catalog["codes"] if c["table"] == "3.2"]
        return f"EK-3 Tablo 3.2 · PDF s. {pages([r['page'] for r in rows])}"
    raise SystemExit(f"{row_id}: no such row in the catalog")


def build(catalog, ifc):
    codes = catalog["codes"]
    clause = next(c for c in catalog["clauses"] if c["id"] == "M4.21.2")
    # 4.21.2 names the typefaces in quotes: “"Times New Roman", "Arial", "Calibri", "Romans" ve "Tahoma"”.
    fonts = re.findall(r'"([^"]+)"', clause["text"])
    if len(fonts) != 5:
        raise SystemExit(f"M4.21.2: {fonts}")
    templates = {d: t for t, ds in LAYER_TEMPLATES.items() for d in ds}
    return {
        "format": "kentos.digitalEk34",
        "version": 1,
        "from": "crates/shared/digital-project/data/temel-2026.json",
        "disciplines": [
            {"code": c["code"], "name": c["name"], "layerTemplate": templates.get(c["code"])}
            for c in codes
            if c["list"] == "discipline"
        ],
        "items": {
            lst.split(":")[1]: [c["code"] for c in codes if c["list"] == lst]
            for lst in sorted({c["list"] for c in codes if c["list"].startswith("item:")})
        },
        "subdisciplines": {
            lst.split(":")[1]: [c["code"] for c in codes if c["list"] == lst]
            for lst in sorted({c["list"] for c in codes if c["list"].startswith("subdiscipline:")})
        },
        "rooms": [{"name": c["name"], "code": c["code"]} for c in codes if c["list"] == "room"],
        "dimensionFonts": fonts,
        "emsal": ["DAHIL", "HARIC", "DIGER"],
        # EK-2's file names (Tablo 2.1; 2.1.1: fields joined by "_"): the fields, the file kinds (2.12) and types (2.13).
        "fileTemplate": [
            {"name": f["name"], "length": int(f["length"]), "kind": f["kind"]}
            for f in next(t for t in catalog["templates"] if t["id"] == "T2.1")["fields"]
        ],
        "fileKinds": [c["code"] for c in codes if c["list"] == "fileKind"],
        "fileTypes": [c["code"] for c in codes if c["list"] == "fileType"],
        "rules": {rule: [cite(catalog, r, ifc) for r in rows] for rule, rows in RULES.items()},
    }


def main():
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    ifc = json.loads(IFC_CATALOG.read_text(encoding="utf-8"))
    text = json.dumps(build(catalog, ifc), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} is not the catalog's: run python3 scripts/mevzuat/ek34_veri.py")
        print("ek34-2026.json is the catalog's")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
