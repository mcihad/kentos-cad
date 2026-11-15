#!/usr/bin/env python3
"""The rule catalog compared row by row with an independent transcription of the annexes (docs/adr/8062 §6;
plan/bim-plan.md BIM-01): an open-source transcription's data files (MIT; docs/adr/8062 §8 names it; EK-2's disciplines, EK-5's classes,
EK-6's 68 mandatory tables, EK-7's mandatory tables, EK-9's 21 questions). Neither is the legal text: a row where the
two differ is read again from the annexes PDF's page and closed as “the catalog was wrong” (the transcription is
fixed), “the other was wrong” or “a reading”.

    python3 scripts/mevzuat/karsilastir.py DIR      # DIR holds ek2_disiplinler.json … ek9_form.json

It prints the differences as a Markdown list; KentOS code is not used, the catalog is read as data.
"""
from __future__ import annotations

import json
import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IFC = ROOT / "crates" / "shared" / "ifc" / "data" / "temel-2026.json"
DIGITAL = ROOT / "crates" / "shared" / "digital-project" / "data" / "temel-2026.json"

KIND = {"M": "Metin", "S": "Sayı", "B": "Mantıksal", "SM": "Sayı veya Metin"}


def norm(s: str | None) -> str:
    """Words compared without case, quotes, spacing or punctuation differences."""
    s = unicodedata.normalize("NFC", s or "")
    s = s.replace("“", '"').replace("”", '"').replace("’", "'")
    s = re.sub(r"[\s\"'`.,;:()\-]+", " ", s)
    return s.strip().casefold()


def canon(s: str) -> str:
    return re.sub(r"[^A-Z0-9]", "", s.upper())


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    other = Path(argv[0])
    ours = json.loads(IFC.read_text(encoding="utf-8"))
    digital = json.loads(DIGITAL.read_text(encoding="utf-8"))
    out: list[str] = []
    counts = {"compared": 0, "differ": 0}

    def differ(line: str) -> None:
        counts["differ"] += 1
        out.append(f"- {line}")

    # EK-2 Tablo 2.2.
    theirs = json.loads((other / "ek2_disiplinler.json").read_text(encoding="utf-8"))["disiplinler"]
    mine = {c["code"]: c for c in digital["codes"] if c["list"] == "discipline"}
    out.append("### EK-2 Tablo 2.2 (disiplinler)")
    for code in sorted(set(theirs) | set(mine)):
        counts["compared"] += 1
        if code not in mine:
            differ(f"{code}: yalnız öbür dökümde ({theirs[code]})")
        elif code not in theirs:
            differ(f"{code}: yalnız katalogda ({mine[code]['name']}, s. {mine[code]['page']})")
        elif norm(theirs[code]) != norm(mine[code]["name"]):
            differ(f"{code}: katalog “{mine[code]['name']}”, öbürü “{theirs[code]}” (s. {mine[code]['page']})")

    # EK-5 Tablo 5.1–5.2.
    theirs5 = json.loads((other / "ek5_siniflar.json").read_text(encoding="utf-8"))
    out.append("### EK-5 Tablo 5.1–5.2 (sınıflar)")
    mine5 = {c["class"]: c for c in ours["classes"]}
    for level, key in (("mandatory", "zorunlu"), ("optional", "istege_bagli")):
        for t in theirs5[key]:
            counts["compared"] += 1
            m = mine5.get(t["ifc"])
            if m is None:
                differ(f"{t['ifc']}: yalnız öbür dökümde ({key})")
                continue
            notes = []
            if m["level"] != level:
                notes.append(f"katalogda {m['level']}, öbüründe {level}")
            if (m["code"] if m["code"] not in ("-", "") else None) != t["kod"]:
                notes.append(f"kod katalogda {m['code']}, öbüründe {t['kod']}")
            if norm(m["tr"]) != norm(t["turkce"]):
                notes.append(f"Türkçe ad katalogda “{m['tr']}”, öbüründe “{t['turkce']}”")
            if m["row"] != t["sira"]:
                notes.append(f"sıra katalogda {m['row']}, öbüründe {t['sira']}")
            if notes:
                differ(f"{t['ifc']} (Tablo {m['table']}, s. {m['page']}): " + "; ".join(notes))
    theirs_names = {t["ifc"] for k in ("zorunlu", "istege_bagli") for t in theirs5[k]}
    for name, m in mine5.items():
        if name not in theirs_names:
            counts["compared"] += 1
            differ(f"{name}: yalnız katalogda (Tablo {m['table']}, s. {m['page']})")

    # EK-6 6.1–6.68 and EK-7.
    def compare_tables(file: str, title: str) -> None:
        theirs6 = json.loads((other / file).read_text(encoding="utf-8"))["tablolar"]
        out.append(f"### {title}")
        tables = {t["table"]: t for t in ours["tables"]}
        for t in theirs6:
            counts["compared"] += 1
            m = tables.get(t["tablo"])
            if m is None:
                differ(f"Tablo {t['tablo']} ({t['ifc']}): yalnız öbür dökümde")
                continue
            if m["class"] != t["ifc"]:
                differ(f"Tablo {t['tablo']}: sınıf katalogda {m['class']}, öbüründe {t['ifc']} (s. {m['pages'][0]})")
            if t.get("sayfa") and t["sayfa"] not in m["pages"]:
                differ(f"Tablo {t['tablo']} ({m['class']}): sayfa katalogda {m['pages']}, öbüründe {t['sayfa']}")
            if "onTanimliTipler" in t:
                a = sorted(canon(x) for x in m["predefined"])
                b = sorted(canon(x) for x in t["onTanimliTipler"])
                if a != b:
                    only_m = sorted(set(a) - set(b))
                    only_t = sorted(set(b) - set(a))
                    differ(
                        f"Tablo {t['tablo']} ({m['class']}) ön tanımlı tipler: yalnız katalogda {only_m or '—'}, "
                        f"yalnız öbüründe {only_t or '—'} (s. {m['pages'][0]})"
                    )
            rows = [r for r in ours["requirements"] if r["table"] == t["tablo"]]

            def key_of_mine(r):
                if r["kind"] in ("property", "quantity"):
                    return ("set", r["set"], r["name"])
                if r["kind"] == "material":
                    return ("material", r["name"])
                if r["kind"] == "relation":
                    return ("relation", r["path"])
                return ("attribute", r["name"])

            def key_of_theirs(g):
                if g.get("set"):
                    return ("set", g["set"], g["ad"])
                if g["tur"] == "malzeme":
                    return ("material", "Name")
                if g["tur"] == "katman":
                    return ("material", g["ad"].split(".")[-1])
                if g["tur"] == "iliski":
                    return ("relation", g["ad"])
                return ("attribute", g["ad"])

            mine_keys = {key_of_mine(r): r for r in rows}
            their_keys = {key_of_theirs(g): g for g in t["gerekenler"]}
            for k in sorted(set(their_keys) - set(mine_keys)):
                differ(f"Tablo {t['tablo']} ({t['ifc']}): yalnız öbür dökümde {'.'.join(k[1:])}")
            for k in sorted(set(mine_keys) - set(their_keys)):
                r = mine_keys[k]
                differ(f"Tablo {t['tablo']} ({t['ifc']}): yalnız katalogda {'.'.join(k[1:])} (satır {r['row']}, s. {r['page']})")
            for k in sorted(set(mine_keys) & set(their_keys)):
                r, g = mine_keys[k], their_keys[k]
                kind = KIND.get(g.get("veriTipi", ""), g.get("veriTipi"))
                if kind and norm(kind) != norm(r["datatype"]):
                    differ(
                        f"Tablo {t['tablo']} ({t['ifc']}) {'.'.join(k[1:])}: veri tipi katalogda “{r['datatype']}”, "
                        f"öbüründe “{kind}” (satır {r['row']}, s. {r['page']})"
                    )

    compare_tables("ek6_ozellikler.json", "EK-6 Tablo 6.1–6.68 (zorunlu)")
    compare_tables("ek7_proje.json", "EK-7 (zorunlu proje sınıfları)")

    # EK-9 Tablo 9.1.
    theirs9 = json.loads((other / "ek9_form.json").read_text(encoding="utf-8"))["maddeler"]
    out.append("### EK-9 Tablo 9.1 (21 soru)")
    mine9 = {q["no"]: q for q in ours["questions"] if q["form"] == "9.1"}
    for t in theirs9:
        counts["compared"] += 1
        m = mine9.get(t["no"])
        if m is None:
            differ(f"{t['no']}: yalnız öbür dökümde")
            continue
        notes = []
        if norm(m["title"]) != norm(t["baslik"]):
            notes.append(f"başlık katalogda “{m['title']}”, öbüründe “{t['baslik']}”")
        if norm(m["question"]) != norm(t["soru"]):
            notes.append(f"soru katalogda “{m['question']}”, öbüründe “{t['soru']}”")
        if norm(m["group"]) != norm(t.get("bolum")):
            notes.append(f"bölüm katalogda “{m['group']}”, öbüründe “{t.get('bolum')}”")
        if norm(m["discipline"]) != norm(t.get("kategori")):
            notes.append(f"kategori katalogda “{m['discipline']}”, öbüründe “{t.get('kategori')}”")
        if notes:
            differ(f"Soru {t['no']} (s. {m['pages']}): " + "; ".join(notes))

    print(f"Karşılaştırılan: {counts['compared']} satır/tablo; ayrışan: {counts['differ']}\n")
    print("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
