#!/usr/bin/env python3
"""KentOS's own block library (docs/adr/0318 §4; plan/cad.md CAD-BLOCK-13): doors, windows, furniture, wet rooms,
drawing marks and a title block, drawn here from plain measurements, so KentOS may give them out with no one else's
rights on them.

    python3 scripts/blocks/library.py           # writes catalog/v1/blocks/kentos-temel.kblok
    python3 scripts/blocks/library.py --check   # writes nothing; compares

The file is a `kentos.blocks` file (docs/adr/0314 §3), the text Bloklar panel's Blokları al… reads; KentOS takes it
in by its name (`cad.blocks.edit`'s import with `name: kentos-temel`). Every block is in metres (the title block at paper size) and has a card (category, tags, version 1); the door, the window, the table, the wardrobe and
the section mark are dynamic.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "catalog" / "v1" / "blocks" / "kentos-temel.kblok"


def P(x, y):
    return {"x": float(x), "y": float(y)}


class Block:
    def __init__(self, n, name, category, tags, description):
        self.data = {"id": f"0192f5b0-0000-7000-8000-{n:012x}", "name": name, "base": P(0, 0), "entities": [],
                     "description": description, "units": "m",
                     "card": {"category": category, "tags": tags, "version": 1}}

    def add(self, kind, **fields):
        e = {"kind": kind, "id": len(self.data["entities"]) + 1, "layerId": "", "attrs": {}}
        e.update(fields)
        self.data["entities"].append(e)
        return e["id"]

    def line(self, a, b):
        return self.add("line", a=P(*a), b=P(*b))

    def path(self, pts, closed=False):
        fields = {"pts": [P(*p) for p in pts]}
        if closed:
            fields["closed"] = True
        return self.add("polyline", **fields)

    def rect(self, x0, y0, x1, y1):
        return self.path([(x0, y0), (x1, y0), (x1, y1), (x0, y1)], closed=True)

    def circle(self, c, r):
        return self.add("circle", c=P(*c), r=float(r))

    def arc(self, c, r, a0, a1):
        return self.add("arc", c=P(*c), r=float(r), a0=float(a0), a1=float(a1))

    def ellipse(self, c, major, ratio):
        return self.add("ellipse", c=P(*c), major=P(*major), ratio=float(ratio), t0=0.0, t1=0.0)

    def text(self, p, text, height):
        return self.add("text", p=P(*p), text=text, height=float(height), rotation=0.0)

    def attribute(self, tag, p, height, value=None, prompt=None, **options):
        a = {"tag": tag}
        if prompt is not None:
            a["prompt"] = prompt
        if value is not None:
            a["value"] = value
        a.update({"p": P(*p), "height": float(height), "rotation": 0.0})
        a.update(options)
        self.data.setdefault("attributes", []).append(a)

    def dynamic(self, parameters, actions):
        self.data["dynamic"] = {"parameters": parameters, "actions": actions}


Q = math.pi / 2
blocks = []

# ── Doors: the insertion point is the hinge, the wall along x ──
door = Block(1, "Kapı", "Mimari/Kapılar", ["kapı", "tek kanat", "iç kapı"],
             "Tek kanatlı kapı; taban noktası menteşe, duvar x boyunca. Genişlik 60–120 cm, açılış yönü ve el çevrilir.")
leaf = door.line((0, 0), (0, 0.9))
swing = door.arc((0, 0), 0.9, 0, Q)
near = door.line((0, -0.1), (0, 0))
far = door.line((0.9, -0.1), (0.9, 0))
door.attribute("NO", (0.1, 0.45), 0.15, prompt="Kapı numarası", movable=True)
door.dynamic(
    [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(0.9, 0), "rule": {"min": 0.6, "max": 1.2, "step": 0.05}},
     {"kind": "flip", "name": "Dışa açılır", "a": P(0, 0), "b": P(1, 0)},
     {"kind": "flip", "name": "Sol el", "a": P(0, 0), "b": P(0, 1)},
     {"kind": "visibility", "name": "Görünüş", "states": [{"name": "Açılım"}, {"name": "Sade", "hidden": [swing]}]},
     {"kind": "lookup", "name": "Tip", "rows": [{"label": "Banyo 70", "values": {"Genişlik": 0.7}},
                                               {"label": "Oda 90", "values": {"Genişlik": 0.9}},
                                               {"label": "Giriş 100", "values": {"Genişlik": 1.0}}]}],
    [{"kind": "scale", "parameter": "Genişlik", "pieces": [leaf, swing]},
     {"kind": "move", "parameter": "Genişlik", "pieces": [far]},
     {"kind": "flip", "parameter": "Dışa açılır", "pieces": [leaf, swing]},
     {"kind": "flip", "parameter": "Sol el", "pieces": [leaf, swing, near, far]}])
blocks.append(door)

double = Block(2, "Çift kanatlı kapı", "Mimari/Kapılar", ["kapı", "çift kanat"],
               "İki kanatlı kapı, 160 cm; taban noktası sol kasa.")
double.line((0, 0), (0, 0.8))
double.arc((0, 0), 0.8, 0, Q)
double.line((1.6, 0), (1.6, 0.8))
double.arc((1.6, 0), 0.8, Q, math.pi)
double.line((0, -0.1), (0, 0))
double.line((1.6, -0.1), (1.6, 0))
blocks.append(double)

# ── Windows: the insertion point is the left end of the wall's outer face ──
window = Block(3, "Pencere", "Mimari/Pencereler", ["pencere", "doğrama"],
               "Pencere, duvar kalınlığı 20 cm; genişlik 60–300 cm, 10 cm adımla esner.")
outer = window.line((0, 0), (1.2, 0))
inner = window.line((0, 0.2), (1.2, 0.2))
glass = window.line((0, 0.1), (1.2, 0.1))
window.line((0, 0), (0, 0.2))
right = window.line((1.2, 0), (1.2, 0.2))
window.dynamic(
    [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(1.2, 0), "rule": {"min": 0.6, "max": 3.0, "step": 0.1}}],
    [{"kind": "stretch", "parameter": "Genişlik", "pieces": [outer, inner, glass, right], "min": P(1.1, -0.05), "max": P(1.3, 0.25)}])
blocks.append(window)

# ── Furniture ──
table = Block(4, "Masa", "Mobilya/Masalar", ["masa", "yemek"], "Dikdörtgen masa, 80 cm derin; uzunluk 120–240 cm.")
top = table.rect(0, 0, 1.6, 0.8)
table.dynamic(
    [{"kind": "linear", "name": "Uzunluk", "from": P(0, 0), "to": P(1.6, 0), "rule": {"min": 1.2, "max": 2.4, "step": 0.2}}],
    [{"kind": "stretch", "parameter": "Uzunluk", "pieces": [top], "min": P(1.5, -0.1), "max": P(1.7, 0.9)}])
blocks.append(table)

chair = Block(5, "Sandalye", "Mobilya/Oturma", ["sandalye"], "Sandalye, 45 × 45 cm; taban noktası oturağın önü ortası.")
chair.rect(-0.225, 0, 0.225, 0.45)
chair.line((-0.225, 0.4), (0.225, 0.4))
blocks.append(chair)

sofa = Block(6, "Kanepe", "Mobilya/Oturma", ["kanepe", "üçlü"], "Üçlü kanepe, 200 × 90 cm.")
sofa.rect(0, 0, 2.0, 0.9)
sofa.line((0.2, 0.2), (1.8, 0.2))
sofa.line((0.2, 0.2), (0.2, 0.9))
sofa.line((1.8, 0.2), (1.8, 0.9))
blocks.append(sofa)

bed = Block(7, "Çift kişilik yatak", "Mobilya/Yatak", ["yatak", "çift"], "Çift kişilik yatak, 160 × 200 cm; taban noktası başucu sol köşe.")
bed.rect(0, 0, 1.6, 2.0)
bed.rect(0.1, 0.1, 0.75, 0.45)
bed.rect(0.85, 0.1, 1.5, 0.45)
blocks.append(bed)

wardrobe = Block(8, "Dolap", "Mobilya/Depolama", ["dolap", "gardırop"], "Gardırop, 60 cm derin; genişlik 60–300 cm.")
body = wardrobe.rect(0, 0, 1.2, 0.6)
wardrobe.line((0, 0.05), (1.2, 0.05))
door_line = wardrobe.line((0.6, 0), (0.6, 0.05))
wardrobe.dynamic(
    [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(1.2, 0), "rule": {"min": 0.6, "max": 3.0, "step": 0.6}}],
    [{"kind": "stretch", "parameter": "Genişlik", "pieces": [body, 2], "min": P(1.1, -0.1), "max": P(1.3, 0.7)},
     {"kind": "array", "parameter": "Genişlik", "pieces": [door_line], "spacing": 0.6}])
blocks.append(wardrobe)

# ── Wet rooms ──
basin = Block(9, "Lavabo", "Islak hacim/Lavabo", ["lavabo", "banyo"], "Lavabo, 55 × 45 cm; taban noktası duvarda ortası.")
basin.rect(-0.275, 0, 0.275, 0.45)
basin.ellipse((0, 0.25), (0.2, 0), 0.65)
basin.circle((0, 0.05), 0.02)
blocks.append(basin)

wc = Block(10, "Klozet", "Islak hacim/WC", ["klozet", "wc", "banyo"], "Klozet ve rezervuar; taban noktası duvarda ortası.")
wc.rect(-0.2, 0, 0.2, 0.18)
wc.ellipse((0, 0.45), (0, 0.24), 0.75)
blocks.append(wc)

shower = Block(11, "Duş teknesi", "Islak hacim/Duş", ["duş", "banyo"], "Duş teknesi, 90 × 90 cm.")
shower.rect(0, 0, 0.9, 0.9)
shower.line((0, 0), (0.9, 0.9))
shower.line((0, 0.9), (0.9, 0))
shower.circle((0.45, 0.45), 0.04)
blocks.append(shower)

tub = Block(12, "Küvet", "Islak hacim/Küvet", ["küvet", "banyo"], "Küvet, 170 × 75 cm.")
tub.rect(0, 0, 1.7, 0.75)
tub.path([(0.1, 0.1), (1.6, 0.1), (1.6, 0.65), (0.1, 0.65)], closed=True)
tub.circle((1.45, 0.375), 0.03)
blocks.append(tub)

# ── Drawing marks (metres at 1:1; scale them with the drawing) ──
level = Block(13, "Kot işareti", "İşaretler/Kot", ["kot", "işaret"], "Kesitte ya da planda kot işareti; değer KOT özniteliğinde.")
level.path([(-0.15, 0.15), (0, 0), (0.15, 0.15)], closed=True)
level.line((0, 0), (0.6, 0))
level.attribute("KOT", (0.2, 0.05), 0.12, value="±0.00", prompt="Kot", movable=True)
blocks.append(level)

north = Block(14, "Kuzey oku", "İşaretler/Yön", ["kuzey", "ok", "yön"], "Kuzey oku; taban noktası dairenin merkezi.")
north.circle((0, 0), 0.5)
north.path([(0, 0.6), (-0.15, -0.3), (0, -0.15), (0.15, -0.3)], closed=True)
north.text((-0.08, 0.7), "K", 0.2)
blocks.append(north)

section = Block(15, "Kesit işareti", "İşaretler/Kesit", ["kesit", "işaret"], "Kesit çizgisinin ucu: bakış yönü oku ve harfi.")
cut = section.line((0, 0), (0, 0.6))
arrow = section.path([(0, 0.6), (0.3, 0.6), (0.2, 0.68), (0.2, 0.52), (0.3, 0.6)])
section.attribute("KESİT", (-0.2, 0.3), 0.2, value="A", prompt="Kesit adı")
section.dynamic(
    [{"kind": "flip", "name": "Ters yön", "a": P(0, 0), "b": P(0, 1)}],
    [{"kind": "flip", "parameter": "Ters yön", "pieces": [arrow]}])
blocks.append(section)

# ── A title block at paper size (0.18 × 0.05 m: 180 × 50 mm on paper); placed scaled by the drawing's scale ──
title = Block(16, "Antet", "Pafta/Antet", ["antet", "pafta"],
              "A3 paftanın sağ alt köşesine 180 × 50 mm antet, kâğıt boyunda: çizime ölçeği kadar büyütülerek yerleşir (1/50'de 50).")
k = 0.001
title.rect(0, 0, 180 * k, 50 * k)
for y in (10, 20, 30, 40):
    title.line((0, y * k), (180 * k, y * k))
title.line((40 * k, 0), (40 * k, 50 * k))
for y, word in ((42, "Proje"), (32, "Pafta"), (22, "Ölçek"), (12, "Tarih"), (2, "Çizen")):
    title.text((2 * k, y * k), word, 3.5 * k)
for y, (tag, prompt, value) in zip((42, 32, 22, 12, 2), (("PROJE", "Proje adı", None), ("PAFTA", "Pafta numarası", "1"),
                                                          ("ÖLÇEK", "Ölçek", "1/50"), ("TARİH", "Tarih", None), ("ÇİZEN", "Çizen", None))):
    title.attribute(tag, (42 * k, y * k), 3.5 * k, value=value, prompt=prompt)
blocks.append(title)

LIBRARY = {"format": "kentos.blocks", "version": 1, "blocks": [b.data for b in blocks]}
TEXT = json.dumps(LIBRARY, ensure_ascii=False, indent=1) + "\n"


def main():
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != TEXT:
            sys.exit("catalog/v1/blocks/kentos-temel.kblok güncel değil")
        print(f"blok kitaplığı tutarlı: {len(blocks)} blok")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(TEXT, encoding="utf-8")
    print(f"blok kitaplığı yazıldı: {len(blocks)} blok")


main()
