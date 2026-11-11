#!/usr/bin/env python3
"""The parking places' test building (docs/adr/8045 §6), M36's (docs/adr/8036) again: ten flats of 100 m² (a living
room and a bedroom each) and a 60 m² shop on Zemin kat, a common 10 × 10 m Otopark and a 5 × 5 m Sığınak south of
them. Otopark Yönetmeliği Ek-1 asks seven places (5.00 + 1.20, rounded up); the Otopark's 100 m² holds five (4/1-ç).

Writes fixtures/arch/v1/parking-building.kcad with the independent reference writer, without KentOS code; `--check`
compares. The real-window acceptance (fixtures/interaction/real/otopark-yeri.txt) reads it."""
import argparse
import sys
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as ref  # noqa: E402

OUT = ROOT / "fixtures/arch/v1/parking-building.kcad"


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


units = [{"id": f"d{i}", "number": str(i), "nature": "Daire"} for i in range(1, 11)]
units.append({"id": "s1", "number": "11", "nature": "Dükkân"})
units.append({"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common"})
d = {"format": "kentos.document", "version": 2, "name": "Otopark binası",
     "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                  "plotScale": 100.0, "workspace": "cad", "cadWork": "architecture",
                  "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}],
                           "program": {"units": units}}},
     "origin": {"x": 0.0, "y": 0.0},
     "layers": [layer("0", "0"), layer("cizim", "Çizim")],
     "activeLayer": "cizim", "styles": {"items": [], "categories": []}}
ents, uids = [], []


def uid(i):
    return str(uuid.UUID(int=0x0192f5a1_4245_7000_8000_000000000000 + i))


def add(e):
    i = len(ents) + 1
    e["id"] = i
    e.setdefault("layerId", "cizim")
    e.setdefault("attrs", {})
    ents.append(e)
    uids.append(uid(i))


def P(x, y):
    return {"x": float(x), "y": float(y)}


def rect(x0, y0, x1, y1):
    return {"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}


def room(x0, y0, x1, y1, name, unit):
    add({"kind": "space", "parts": [rect(x0, y0, x1, y1)], "name": name, "unit": unit, "arch": {"level": "zemin"}})


for i in range(10):
    x = 10.0 * i
    room(x, 0.0, x + 6.0, 10.0, "Salon", f"d{i + 1}")
    room(x + 6.0, 0.0, x + 10.0, 10.0, "Yatak odası", f"d{i + 1}")
room(100.0, 0.0, 106.0, 10.0, "Dükkân", "s1")
room(0.0, -12.0, 10.0, -2.0, "Otopark", "ortak")
room(12.0, -12.0, 17.0, -7.0, "Sığınak", "ortak")
d["entities"], d["uids"] = ents, uids


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    data = ref.container(ref.document(d))
    if a.check:
        if not OUT.exists() or OUT.read_bytes() != data:
            print(f"{OUT.relative_to(ROOT)} başvuruyla aynı değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(ents)} nesne, başvuruyla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(data)
    print(f"{OUT.relative_to(ROOT)}: {len(ents)} nesne yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
