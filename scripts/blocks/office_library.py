#!/usr/bin/env python3
"""KentOS's published component library `kentos-ofis` (docs/adr/1100 §2): a few office symbols each carrying a
publisher identity (family, type and release), so an office standard's block reference (`resources.blocks`) has a
library KentOS can read to install it from. The door's two releases stand side by side, as a publisher's archive
keeps them: a standard names the release it approved, never “the latest”. Drawn here from plain measurements; no one
else's rights are on them.

    python3 scripts/blocks/office_library.py           # writes catalog/v1/blocks/kentos-ofis.kblok
    python3 scripts/blocks/office_library.py --check   # writes nothing; compares

The file is a `kentos.blocks` file of version 2 (publication identities, docs/adr/0314 §3); `cad.blocks.edit`'s
import takes it by its name (`name: kentos-ofis`) and `cad.standards.install` resolves references against it.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "catalog" / "v1" / "blocks" / "kentos-ofis.kblok"

# Publisher identities: stable whatever a drawing calls the block.
FAMILY_DOOR = "0192f5b1-0000-7000-8000-0000000f0001"
TYPE_DOOR = "0192f5b1-0000-7000-8000-0000000e0001"
FAMILY_MARK = "0192f5b1-0000-7000-8000-0000000f0002"
TYPE_NORTH = "0192f5b1-0000-7000-8000-0000000e0002"
TYPE_LEVEL = "0192f5b1-0000-7000-8000-0000000e0003"


def P(x, y):
    return {"x": float(x), "y": float(y)}


class Block:
    def __init__(self, n, name, category, tags, description, family, type_, type_name, release):
        self.data = {"id": f"0192f5b1-0000-7000-8000-{n:012x}", "name": name, "base": P(0, 0), "entities": [],
                     "description": description, "units": "m",
                     "card": {"publication": {"familyId": family, "typeId": type_, "typeName": type_name,
                                              "release": release},
                              "category": category, "tags": tags, "version": 1}}

    def add(self, kind, **fields):
        e = {"kind": kind, "id": len(self.data["entities"]) + 1, "layerId": "", "attrs": {}}
        e.update(fields)
        self.data["entities"].append(e)

    def line(self, a, b):
        self.add("line", a=P(*a), b=P(*b))


def door(n, release):
    b = Block(n, f"Ofis kapı 90 · yayın {release}", "Mimari/Kapılar", ["kapı", "tek kanat", "ofis"],
              "Tek kanatlı 90 cm kapı; taban noktası menteşe, duvar x boyunca."
              + (" Yayın 2: eşik çizgisi eklendi." if release == 2 else ""),
              FAMILY_DOOR, TYPE_DOOR, "Tek kanatlı kapı 90", release)
    b.line((0, 0), (0, 0.9))
    b.add("arc", c=P(0, 0), r=0.9, a0=0.0, a1=math.pi / 2)
    b.line((0, -0.1), (0, 0))
    b.line((0.9, -0.1), (0.9, 0))
    if release == 2:
        b.line((0, -0.05), (0.9, -0.05))
    return b.data


def north():
    b = Block(3, "Ofis kuzey oku", "İşaretler/Yön", ["kuzey", "ok", "ofis"], "Kuzey oku; taban noktası dairenin merkezi.",
              FAMILY_MARK, TYPE_NORTH, "Kuzey oku", 1)
    b.add("circle", c=P(0, 0), r=0.5)
    b.add("polyline", pts=[P(0, 0.6), P(-0.15, -0.3), P(0, -0.15), P(0.15, -0.3)], closed=True)
    return b.data


def level():
    b = Block(4, "Ofis kot işareti", "İşaretler/Kot", ["kot", "ofis"], "Kot işareti; taban noktası üçgenin ucu.",
              FAMILY_MARK, TYPE_LEVEL, "Kot işareti", 1)
    b.add("polyline", pts=[P(0, 0), P(-0.15, 0.25), P(0.15, 0.25)], closed=True)
    b.line((-0.15, 0.25), (0.9, 0.25))
    return b.data


def build():
    return {"format": "kentos.blocks", "version": 2, "blocks": [door(1, 1), door(2, 2), north(), level()]}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['blocks'])} blok)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
