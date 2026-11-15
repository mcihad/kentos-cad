#!/usr/bin/env python3
"""Twenty rows of the rule catalog drawn at random (seed 8062) for reading against the annexes PDF's pages (docs/adr/8062
§8; plan/bim-plan.md BIM-01's acceptance: “rastgele seçilen 20 satır sayfa görüntüsüyle birebir”). Every row of both
catalogs is a candidate: a code, a template, a clause, a class, a requirement, a form question. It prints each row's id,
page and words; the reading's result is the ADR's.

    python3 scripts/mevzuat/ornekle.py [N] [SEED]
"""
from __future__ import annotations

import json
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IFC = ROOT / "crates" / "shared" / "ifc" / "data" / "temel-2026.json"
DIGITAL = ROOT / "crates" / "shared" / "digital-project" / "data" / "temel-2026.json"


def rows():
    d = json.loads(DIGITAL.read_text(encoding="utf-8"))
    i = json.loads(IFC.read_text(encoding="utf-8"))
    for c in d["clauses"] + i["clauses"]:
        yield c["id"], c["pages"][0], f"Madde {c['clause']}: {c['text'][:160]}"
    for t in d["templates"]:
        yield t["id"], t["page"], f"{t['title']}: " + " · ".join(f"{f['name']} {f['length']}" for f in t["fields"])
    for c in d["codes"]:
        yield c["id"], c["page"], f"Tablo {c['table']} satır {c['row']}: {c['name']} = {c['code']}" + (f" ({c['note']})" if c["note"] else "")
    for f in d["fonts"]:
        yield f["id"], f["page"], f"Tablo 4.7: {f['font']} {f['points']}"
    for m in d["marks"]:
        yield m["id"], m["page"], f"Şekil 4.7: {m['mark']} {m['measure']} {m['sizes']}"
    for c in i["classes"]:
        yield c["id"], c["page"], f"Tablo {c['table']} satır {c['row']}: {c['class']} {c['tr']} {c['code']}"
    for r in i["requirements"]:
        where = f"{r['set']}.{r['name']}" if r["set"] else (r["path"] or r["name"])
        yield r["id"], r["page"], f"Tablo {r['table']} satır {r['row']}: {r['tr']} | {where} | {r['datatype']}"
    for q in i["questions"]:
        yield q["id"], q["pages"][0], f"Tablo {q['form']} soru {q['no']}: {q['title']} — {q['question'][:120]}"


def main(argv: list[str]) -> int:
    n = int(argv[0]) if argv else 20
    seed = int(argv[1]) if len(argv) > 1 else 8062
    all_rows = list(rows())
    picked = random.Random(seed).sample(all_rows, n)
    print(f"{len(all_rows)} satırdan {n} tanesi (tohum {seed}):")
    for k, (rid, page, words) in enumerate(sorted(picked, key=lambda r: r[1]), 1):
        print(f"{k:2}. s. {page:3}  {rid}  —  {words}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
