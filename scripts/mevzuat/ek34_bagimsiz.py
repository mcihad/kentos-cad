#!/usr/bin/env python3
"""EK-3 and EK-4 checked on a DXF without KentOS's code (docs/adr/8064, plan/bim-plan.md BIM-03 "bağımsız başvuru"): the
file read with ezdxf as another program reads it, the regulation's rules written again from its text, the discipline
codes taken from the annexes' transcription (mevzuat/bim/ek-2…, Tablo 2.2). It prints each rule's verdict, "pass" or
"fail", as JSON; the desktop's test compares them with the standards package's rule on the same drawing.

    uv run --offline --no-project --with ezdxf==1.4.5 python -I scripts/mevzuat/ek34_bagimsiz.py plan.dxf

What a DXF can say: the layers, their objects and line types, the hatches, the blocks inserted, the rooms' numbers as
the texts on the room layer (MM-XXXX-MAHAL) or the tags' (MM-XXXX-ETIKET) write them, at their text's place. Not in a
DXF: a room's storey (so no storey code) and the drawing's own typefaces (KentOS writes TrueType text as arial.ttf,
docs/adr/8046 §2), so neither is judged here.
"""

import json
import math
import re
import sys
from pathlib import Path

import ezdxf

ROOT = Path(__file__).resolve().parents[2]
EK2 = ROOT / "mevzuat" / "bim" / "ek-2-dosya-isimlendirme.md"

NUMBER = re.compile(r"^([A-Z0-9]{3})_([A-Z0-9]{3})_([0-9]{3})$")
FIELD = re.compile(r"^[A-Z0-9]+$")


def disciplines():
    """Tablo 2.2's two-letter codes, as the transcription's rows write them (“| Mimari | MM |”)."""
    text = EK2.read_text(encoding="utf-8")
    table = text.split("## Tablo 2.2", 1)[1].split("\n## ", 1)[0]
    return {m.group(1) for m in re.finditer(r"^\|[^|]+\|\s*([A-Z]{2})\s*\|", table, re.M)}


def layer_ok(name, codes):
    """4.11 and Tablo 4.1–4.3: capitals without Turkish letters, fields by single hyphens; discipline, a four-letter
    field and a material of at most 15; a "-T" (4.14.5), a depth 1–4 (4.14.3) or a renovation state after it."""
    if name != name.upper() or re.search(r"[ÇĞİÖŞÜçğıöşü\s]", name):
        return False
    fields = name.split("-")
    if any(not f or not FIELD.match(f) for f in fields):
        return False
    while len(fields) > 3 and fields[-1] in ("T", "1", "2", "3", "4", "MEVCUT", "IPTAL", "ILAVE", "GECICI"):
        fields = fields[:-1]
    if fields[0] not in codes:
        return False
    if fields[0] == "PY":
        return len(fields) in (3, 4) and len(fields[1]) == 4 and len(fields[2]) == 4
    return len(fields) == 3 and len(fields[1]) == 4 and fields[1].isalpha() and len(fields[2]) <= 15


def block_ok(name, codes):
    """Tablo 4.4: discipline, item (4), an optional material (≤ 15) and size (≤ 10), capitals, hyphens."""
    if name != name.upper() or re.search(r"[ÇĞİÖŞÜçğıöşü\s]", name):
        return False
    fields = name.split("-")
    if any(not f or not FIELD.match(f) for f in fields) or fields[0] not in codes:
        return False
    if not 2 <= len(fields) <= 4 or len(fields[1]) != 4:
        return False
    return not (len(fields) >= 3 and len(fields[2]) > 15) and not (len(fields) == 4 and len(fields[3]) > 10)


def text_of(e):
    if e.dxftype() == "MTEXT":
        return e.plain_text().strip()
    return (e.dxf.text or "").strip()


def clockwise(points):
    """The points' order clockwise from north round their middle."""
    cx = sum(p[1][0] for p in points) / len(points)
    cy = sum(p[1][1] for p in points) / len(points)
    return sorted(points, key=lambda p: (math.atan2(p[1][0] - cx, p[1][1] - cy) + 2 * math.pi) % (2 * math.pi))


def main(path):
    doc = ezdxf.readfile(path)
    msp = doc.modelspace()
    codes = disciplines()
    used = {}
    for e in msp:
        used.setdefault(e.dxf.layer, []).append(e)
    verdict = {}
    named = [name for name in used if name != "0" and name.upper() != "DEFPOINTS"]
    verdict["layer_name"] = "pass" if all(layer_ok(n, codes) for n in named) else "fail"
    verdict["layer_zero"] = "fail" if used.get("0") else "pass"
    hatches = [e for e in msp if e.dxftype() == "HATCH"]
    verdict["hatch_layer"] = "pass" if all(h.dxf.layer.endswith("-T") for h in hatches) else "fail"
    emsal = [n for n in named if n.split("-")[1:2] == ["EMSL"]]
    verdict["emsal_layer"] = "pass" if all(n.split("-")[2] in ("DAHIL", "HARIC", "DIGER") for n in emsal) else "fail"
    axis = [n for n in named if n.split("-")[1:2] == ["AKSS"]]
    linetypes = {n: doc.layers.get(n).dxf.linetype.upper() for n in axis}
    verdict["axis_line"] = "pass" if all(("DASHDOT" in lt or "CENTER" in lt) for lt in linetypes.values()) else "fail"
    inserted = {e.dxf.name for e in msp if e.dxftype() == "INSERT"}
    verdict["block_name"] = "pass" if all(block_ok(n, codes) for n in inserted) else "fail"
    # EK-3: the numbers the room and tag texts write, at their places.
    numbers = []
    for layer in ("MM-XXXX-MAHAL", "MM-XXXX-ETIKET"):
        for e in used.get(layer, []):
            if e.dxftype() in ("TEXT", "MTEXT"):
                for line in text_of(e).splitlines():
                    line = line.strip()
                    if "_" in line or re.fullmatch(r"[A-Z0-9]{3,}", line):
                        p = e.dxf.insert
                        numbers.append((line, (p.x, p.y)))
    # A room's number written twice (its own text and its tag's) is one number.
    seen = {}
    for n, p in numbers:
        seen.setdefault(n, p)
    numbers = sorted(seen.items())
    verdict["number_pattern"] = "pass" if all(NUMBER.match(n) for n, _ in numbers) else "fail"
    groups = {}
    for n, p in numbers:
        m = NUMBER.match(n)
        if m:
            groups.setdefault((m.group(1), m.group(2)), []).append((int(m.group(3)), p))
    sequence, turn = "pass", "pass"
    for members in groups.values():
        if sorted(s for s, _ in members) != list(range(1, len(members) + 1)):
            sequence = "fail"
            continue
        if len(members) >= 3:
            order = [s for s, _ in clockwise(members)]
            k = order.index(1)
            if order[k:] + order[:k] != sorted(order):
                turn = "fail"
    verdict["sequence"] = sequence
    verdict["clockwise"] = turn
    verdict["_numbers"] = [n for n, _ in numbers]
    print(json.dumps(verdict, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main(sys.argv[1])
