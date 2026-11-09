#!/usr/bin/env python3
"""KentOS's zoning symbols (docs/adr/4240; plan/map-cad.md MAP-09): which MPYY symbol a decision area's use takes at
each plan level, and the area symbols a user may choose from, read from the system library and the profile.

    python3 scripts/cad/zoning_symbols.py           # writes catalog/v1/profiles/imar-plani-semboller.json
    python3 scripts/cad/zoning_symbols.py --check   # writes nothing; compares

The rule: at a level (uip: MPYY › Uygulama imar planı, nip: MPYY › Nazım imar planı) a use of the profile's
`plan-kullanim` list takes the area (fill) symbol whose name is the use's own name, written alike (Turkish lower case,
“î” as “i”, a trailing “ (…)” of a legend that takes a name left out); exactly one such symbol, else none. A use
without one is left open: the user chooses its symbol from the level's area symbols; KentOS does not guess (MPYY has,
say, a Yerleşik and a Gelişme konut alanı, the profile one Konut alanı).
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
LIBRARY = ROOT / "crates/native/style/assets/system-library.kstil"
PROFILE = ROOT / "catalog/v1/profiles/imar-plani.json"
OUT = ROOT / "catalog/v1/profiles/imar-plani-semboller.json"
LEVELS = [("uip", "Uygulama imar planı"), ("nip", "Nazım imar planı")]


def written(name):
    """A name as it is compared: Turkish lower case, î as i, a legend's trailing “ (…)” left out."""
    name = name.replace("İ", "i").replace("I", "ı").lower().replace("î", "i")
    return name[:-4].strip() if name.endswith(" (…)") else name.strip()


def table():
    library = json.loads(LIBRARY.read_text("utf-8"))["items"]
    profile = json.loads(PROFILE.read_text("utf-8"))
    uses = next(d for d in profile["domains"] if d["id"] == "plan-kullanim")["values"]
    levels = []
    for level, branch in LEVELS:
        fills = [i for i in library if i["kind"] == "symbol" and i["path"][:2] == ["MPYY", branch]
                 and (i.get("symbol") or {}).get("type") == "fill"]
        mapped = []
        for u in uses:
            same = [i for i in fills if written(i["name"]) == written(u["label"])]
            if len(same) == 1:
                mapped.append({"code": u["code"], "symbol": same[0]["id"]})
        levels.append({"level": level, "library": f"MPYY › {branch}", "uses": mapped,
                       "choices": [{"id": i["id"], "name": i["name"]} for i in fills]})
    return {
        "format": "kentos.zoning-symbols",
        "version": 1,
        "profile": "imar-plani",
        "basis": "KentOS'un eşlemesi (ADR 4240): kademenin MPYY alan sembollerinden (sistem kitaplığı, yönetmelik "
                 "eklerinin lejant adları) adı kullanımın adıyla aynı yazılan tek sembol. Birebir eşi olmayan kullanım "
                 "açık kalır; sembolünü kullanıcı seçer, KentOS tahmin etmez. Sembol karar değildir: karar alanın "
                 "Kullanım değeridir.",
        "levels": levels,
    }


def main():
    text = json.dumps(table(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("İmar sembolleri güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    t = json.loads(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ("
          + ", ".join(f"{lv['level']}: {len(lv['uses'])} kullanım, {len(lv['choices'])} sembol" for lv in t["levels"])
          + ")")
    return 0


if __name__ == "__main__":
    sys.exit(main())
