"""The shared cases of the Blok öznitelikleri window's table (docs/adr/0144 §7):
an attribute definition as a row of texts and back, and where a new row goes.

    python3 scripts/fixtures/attribute_table_cases.py           # writes the file
    python3 scripts/fixtures/attribute_table_cases.py --check   # writes nothing; compares

Writes fixtures/blocks/v1/attribute-table.json. The rules are written here
from the ADR on their own, not from an implementation's output: a number is
shown as JavaScript's `String(+n.toFixed(6))` writes it (six decimals of the
exact binary value, then the shortest text that reads back, never “-0”);
arithmetic is IEEE double, as Python's floats are, in the rule's order. The
web (apps/web/src/ui/blocks/attributeTable.test.ts) and the desktop
(apps/desktop/src/attribute_table.rs) read the file. A definition's alignment
and width factor (docs/adr/0145) have no cells: a row keeps them as they were.
"""
import json
import math
import sys


def number_text(n):
    """JavaScript's String(+n.toFixed(6)) for the moderate numbers used here."""
    fixed = "%.6f" % n  # the exact binary value, correctly rounded (no ties below)
    v = float(fixed)
    if v == 0:
        return "0"
    text = repr(v)
    return text[:-2] if text.endswith(".0") else text


def P(x, y):
    return {"x": x, "y": y}


def definition(tag, p, height, rotation=0, prompt=None, value=None, align=None, width_factor=None):
    d = {"tag": tag}
    if prompt is not None:
        d["prompt"] = prompt
    if value is not None:
        d["value"] = value
    d.update({"p": p, "height": height, "rotation": rotation})
    if align is not None:
        d["align"] = align
    if width_factor is not None:
        d["widthFactor"] = width_factor
    return d


def cells_of(d, base):
    return {
        "tag": d["tag"],
        "prompt": d.get("prompt", ""),
        "value": d.get("value", ""),
        "height": number_text(d["height"]),
        "rotation": number_text(d["rotation"]),
        "y": number_text(d["p"]["x"] - base["x"]),
        "x": number_text(d["p"]["y"] - base["y"]),
    }


def read_number(text):
    """The Hesap tables' reader: trimmed, a comma for the point; None when empty, NaN when no number."""
    t = text.strip().replace(",", ".", 1)
    if not t:
        return None
    try:
        import re
        if not re.fullmatch(r"[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?", t, re.I):
            return math.nan
        return float(t)
    except ValueError:
        return math.nan


def json_number(n):
    return "NaN" if isinstance(n, float) and math.isnan(n) else n


BASE = P(487010.0, 4420008.5)
NO = definition("NO", P(487010.9, 4420008.65), 0.5, prompt="Rögar numarası", value="R-?")
KOT = definition("KOT", P(487010.9, 4420007.95), 0.4, rotation=12.5, prompt="Kapak kotu")
ORTA = definition("NO", P(487010.9, 4420008.65), 0.5, prompt="Rögar numarası", value="R-?", align="middleCenter", width_factor=0.8)

numbers = [[n, number_text(n)] for n in [0.0, 0.5, -0.55, 487010.9 - 487010.0, 4420008.65 - 4420008.5, -1e-9, 12.5, 1234567.1234567, 2.0000004]]

rows = [
    {"name": "tanımın satırı: sayılar tabana göre, altı ondalıkla", "base": BASE, "definition": NO, "cells": cells_of(NO, BASE)},
    {"name": "sorusu ve varsayılanı olmayan, dönük öznitelik", "base": BASE, "definition": KOT, "cells": cells_of(KOT, BASE)},
]


def edited(d, base, **cells):
    out = cells_of(d, base)
    out.update(cells)
    return out


definitions = [
    {"name": "dokunulmamış satır tanımı olduğu gibi verir (yer ve sayılar tam)", "base": BASE, "from": NO, "cells": cells_of(NO, BASE), "definition": NO},
    {
        "name": "yazılanlar: etiket ve soru kırpılır, boş soru ve varsayılan yazılmaz, virgül ondalıktır",
        "base": BASE,
        "from": NO,
        "cells": edited(NO, BASE, tag="  KAPAK ", prompt="  ", value=" Açık ", height="0,75", y="1.25", x="-0.5"),
        "definition": definition("KAPAK", P(BASE["x"] + 1.25, BASE["y"] + -0.5), 0.75, value="Açık"),
    },
    {
        "name": "boş hücreler: yer ve açı 0 (taban noktası), yükseklik yok (NaN)",
        "base": BASE,
        "from": KOT,
        "cells": edited(KOT, BASE, height="", rotation="", y="", x=""),
        "definition": definition("KOT", P(BASE["x"], BASE["y"]), math.nan, rotation=0, prompt="Kapak kotu"),
    },
    {
        "name": "sayı olmayan hücre NaN verir",
        "base": BASE,
        "from": KOT,
        "cells": edited(KOT, BASE, height="abc", rotation="1e1", y="0.4.5"),
        "definition": definition("KOT", P(math.nan, KOT["p"]["y"]), math.nan, rotation=10.0, prompt="Kapak kotu"),
    },
    {
        "name": "hizası ve genişlik çarpanı satırda kalır (hücreleri yok; ADR 0145)",
        "base": BASE,
        "from": ORTA,
        "cells": edited(ORTA, BASE, value="R-1"),
        "definition": definition("NO", ORTA["p"], 0.5, prompt="Rögar numarası", value="R-1", align="middleCenter", width_factor=0.8),
    },
]


def new_row(above, base, extent, height):
    """The rule: below the last one by a line and a half, its height; the first right of the drawing, its top level with it."""
    last = above[-1] if above else None
    h = last["height"] if last and math.isfinite(last["height"]) and last["height"] > 0 else height
    if last and math.isfinite(last["p"]["x"]) and math.isfinite(last["p"]["y"]):
        p = P(last["p"]["x"], last["p"]["y"] - 1.5 * h)
    elif extent:
        p = P(base["x"] + extent["maxX"] + 0.2 * h, base["y"] + extent["maxY"] - h)
    else:
        p = P(base["x"], base["y"])
    return {
        "tag": "",
        "prompt": "",
        "value": "",
        "height": number_text(h),
        "rotation": "0",
        "y": number_text(p["x"] - base["x"]),
        "x": number_text(p["y"] - base["y"]),
    }, p, h


ROGAR = {"minX": -0.75, "minY": -0.75, "maxX": 0.75, "maxY": 0.75}
new_rows = []
for name, above, extent, height in [
    ("ilk satır bloğun çiziminin sağında, yazısının üstü çizimin üstüyle bir", [], ROGAR, 1.25),
    ("çizimi olmayan blokta ilk satır taban noktasında", [], None, 1.25),
    ("sonraki satır öncekinin bir buçuk satır altında, onun yüksekliğiyle", [NO], ROGAR, 1.25),
    ("önceki satırın yüksekliği okunmuyorsa verilen yükseklik", [definition("A", P(487011.0, 4420009.0), math.nan)], ROGAR, 1.25),
]:
    cells, p, h = new_row(above, BASE, extent, height)
    new_rows.append({"name": name, "base": BASE, "above": above, "extent": extent, "height": height, "cells": cells, "p": p, "exactHeight": h})


def nan_safe(v):
    if isinstance(v, float) and math.isnan(v):
        return "NaN"
    if isinstance(v, dict):
        return {k: nan_safe(x) for k, x in v.items()}
    if isinstance(v, list):
        return [nan_safe(x) for x in v]
    return v


fixture = {
    "format": "kentos.attribute-table",
    "version": 1,
    "title": "Blok öznitelikleri penceresinin tablosu: tanım satıra, satır tanıma; yeni satırın yeri (ADR 0144 §7)",
    "note": "Sayı hücrede JavaScript'in String(+n.toFixed(6)) yazdığı gibidir: ikili değerin altı ondalığı, sonra geri okunan en kısa yazı, “-0” değil. Yer taban noktasına göredir: Y doğu, X kuzey. Dokunulmamış hücre gösterdiği tam değeri verir; yazılan hücre Hesap tablolarının okuyucusuyla okunur (kırpılır, virgül noktadır; boş: yer ve açı 0, yükseklik yok; sayı değil: NaN). Etiket, soru ve varsayılan kırpılır; boş soru ve varsayılan yazılmaz. Yeni satır: öncekinin bir buçuk satır altında, onun yüksekliğiyle; ilk satır bloğun çiziminin (taban noktasında, dönmemiş) sağında, yüksekliğin beşte biri ötede, yazısının üstü çizimin üstüyle bir; çizim yoksa taban noktasında. JSON NaN taşıyamaz: \"NaN\" yazılır. Aritmetik IEEE çift duyarlıklı, kuralın sırasıyla.",
    "numbers": numbers,
    "rows": rows,
    "definitions": definitions,
    "newRows": new_rows,
}

text = json.dumps(nan_safe(fixture), ensure_ascii=False, indent=2) + "\n"
path = "fixtures/blocks/v1/attribute-table.json"
if "--check" in sys.argv[1:]:
    with open(path, encoding="utf-8") as f:
        if f.read() != text:
            sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
    print("attribute-table matches")
else:
    import os
    os.makedirs("fixtures/blocks/v1", exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)
    print("attribute-table written")
