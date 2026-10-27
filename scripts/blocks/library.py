#!/usr/bin/env python3
"""KentOS's own block library (docs/adr/0318 §4; plan/cad.md CAD-BLOCK-13): doors, windows, furniture, wet rooms,
drawing marks and a title block, drawn here from plain measurements, so KentOS may give them out with no one else's
rights on them. ARCH-LIB-01 (docs/adr/0702) adds real-size beds, seats, tables, storage, wet-room fixtures with an
optional clearance outline, white goods and kitchen units.

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

# ── ARCH-LIB-01 (docs/adr/0702): real-size furniture, wet-room and kitchen symbols. Wall-standing ones have their
# insertion point at the middle of their back on the wall, their front toward +y; the rest at their back's left corner.
# A fixture's clearance is a dashed outline shown in the visibility state “Boşluk zarfı”; it is a drawing aid, not a
# regulation's check. ──


def dashed(block, eid):
    block.data["entities"][eid - 1]["lineType"] = "dashed"
    return eid


def clearance(block, x0, y0, x1, y1):
    """The dashed clearance outline and the visibility parameter that hides it by default."""
    env = dashed(block, block.rect(x0, y0, x1, y1))
    return env, {"kind": "visibility", "name": "Görünüş",
                 "states": [{"name": "Sade", "hidden": [env]}, {"name": "Boşluk zarfı"}]}


single = Block(17, "Tek kişilik yatak", "Mobilya/Yatak", ["yatak", "tek", "genç"],
               "Tek kişilik yatak 90 × 200 cm; taban noktası başucu sol köşe, yatak +y yönünde. En 80–140 cm; Boyut tablosu 90/100/120.")
frame = single.rect(0, 0, 0.9, 2.0)
pillow = single.rect(0.1, 0.08, 0.8, 0.4)
cover = single.line((0, 0.65), (0.9, 0.65))
single.dynamic(
    [{"kind": "linear", "name": "En", "from": P(0, 0), "to": P(0.9, 0), "rule": {"min": 0.8, "max": 1.4, "step": 0.1}},
     {"kind": "lookup", "name": "Boyut", "rows": [{"label": "Tek 90 × 200", "values": {"En": 0.9}},
                                                 {"label": "Tek 100 × 200", "values": {"En": 1.0}},
                                                 {"label": "Genç 120 × 200", "values": {"En": 1.2}}]}],
    [{"kind": "stretch", "parameter": "En", "pieces": [frame, pillow, cover], "min": P(0.75, -0.1), "max": P(1.0, 2.1)}])
blocks.append(single)

nightstand = Block(18, "Komodin", "Mobilya/Yatak", ["komodin", "başucu"], "Başucu komodini 50 × 40 cm, lambasıyla; taban noktası arka sol köşe.")
nightstand.rect(0, 0, 0.5, 0.4)
nightstand.circle((0.25, 0.2), 0.1)
blocks.append(nightstand)

armchair = Block(19, "Koltuk", "Mobilya/Oturma", ["koltuk", "berjer", "tekli"], "Tekli koltuk 85 × 85 cm; taban noktası arkasının ortası, oturağı +y yönünde.")
armchair.rect(-0.425, 0, 0.425, 0.85)
armchair.line((-0.275, 0.2), (0.275, 0.2))
armchair.rect(-0.425, 0.2, -0.275, 0.85)
armchair.rect(0.275, 0.2, 0.425, 0.85)
blocks.append(armchair)

loveseat = Block(20, "İkili kanepe", "Mobilya/Oturma", ["kanepe", "ikili"], "İkili kanepe 150 × 85 cm; taban noktası arka sol köşe.")
loveseat.rect(0, 0, 1.5, 0.85)
loveseat.line((0.15, 0.2), (1.35, 0.2))
loveseat.line((0.15, 0.2), (0.15, 0.85))
loveseat.line((1.35, 0.2), (1.35, 0.85))
loveseat.line((0.75, 0.2), (0.75, 0.85))
blocks.append(loveseat)

round_table = Block(21, "Yuvarlak masa", "Mobilya/Masalar", ["masa", "yuvarlak", "yemek"],
                    "Yuvarlak masa Ø100 cm, dört sandalyesiyle; taban noktası masanın merkezi. Görünüş: Sandalyeli / Yalnız masa.")
round_table.circle((0, 0), 0.5)
seats = []
for k in range(4):
    a = k * Q
    cx, cy = math.cos(a) * 0.62, math.sin(a) * 0.62
    ux, uy = math.cos(a), math.sin(a)
    vx, vy = -uy, ux
    corners = [(cx - ux * 0.0 + vx * sx * 0.21 + ux * sy, cy - uy * 0.0 + vy * sx * 0.21 + uy * sy)
               for sx, sy in ((-1, 0), (1, 0), (1, 0.42), (-1, 0.42))]
    seats.append(round_table.path([(round(x, 12), round(y, 12)) for x, y in corners], closed=True))
round_table.dynamic(
    [{"kind": "visibility", "name": "Görünüş", "states": [{"name": "Sandalyeli"}, {"name": "Yalnız masa", "hidden": seats}]}], [])
blocks.append(round_table)

desk = Block(22, "Çalışma masası", "Mobilya/Masalar", ["masa", "çalışma", "ofis"], "Çalışma masası 120 × 60 cm; uzunluk 100–180 cm. Taban noktası arka sol köşe.")
desk_top = desk.rect(0, 0, 1.2, 0.6)
desk.dynamic(
    [{"kind": "linear", "name": "Uzunluk", "from": P(0, 0), "to": P(1.2, 0), "rule": {"min": 1.0, "max": 1.8, "step": 0.2}}],
    [{"kind": "stretch", "parameter": "Uzunluk", "pieces": [desk_top], "min": P(1.1, -0.1), "max": P(1.3, 0.7)}])
blocks.append(desk)

tv = Block(23, "TV ünitesi", "Mobilya/Depolama", ["tv", "ünite", "salon"], "TV ünitesi 180 × 45 cm; uzunluk 120–240 cm. Taban noktası arka sol köşe.")
tv_body = tv.rect(0, 0, 1.8, 0.45)
tv_screen = tv.line((0.3, 0.06), (1.5, 0.06))
tv.dynamic(
    [{"kind": "linear", "name": "Uzunluk", "from": P(0, 0), "to": P(1.8, 0), "rule": {"min": 1.2, "max": 2.4, "step": 0.3}}],
    [{"kind": "stretch", "parameter": "Uzunluk", "pieces": [tv_body, tv_screen], "min": P(1.4, -0.1), "max": P(1.9, 0.55)}])
blocks.append(tv)

shelf = Block(24, "Kitaplık", "Mobilya/Depolama", ["kitaplık", "raf"], "Kitaplık 90 × 35 cm; genişlik 60–240 cm, 30 cm adımla. Taban noktası arka sol köşe.")
shelf_body = shelf.rect(0, 0, 0.9, 0.35)
shelf_back = shelf.line((0, 0.03), (0.9, 0.03))
shelf.dynamic(
    [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(0.9, 0), "rule": {"min": 0.6, "max": 2.4, "step": 0.3}}],
    [{"kind": "stretch", "parameter": "Genişlik", "pieces": [shelf_body, shelf_back], "min": P(0.8, -0.1), "max": P(1.0, 0.45)}])
blocks.append(shelf)

hung = Block(25, "Asma klozet", "Islak hacim/WC", ["klozet", "asma", "wc", "banyo"],
             "Asma klozet 37 × 54 cm, gömme rezervuarlı; taban noktası duvarda ortası. Boşluk zarfı önünde 80 × 60 cm.")
hung.rect(-0.25, 0, 0.25, 0.08)
hung.ellipse((0, 0.31), (0, 0.23), 0.8)
_, look = clearance(hung, -0.4, 0.54, 0.4, 1.14)
hung.dynamic([look], [])
blocks.append(hung)

urinal = Block(26, "Pisuvar", "Islak hacim/WC", ["pisuvar", "wc"], "Pisuvar 36 × 35 cm; taban noktası duvarda ortası. Boşluk zarfı önünde 80 × 60 cm.")
urinal.path([(-0.18, 0), (0.18, 0), (0.18, 0.2), (0.12, 0.35), (-0.12, 0.35), (-0.18, 0.2)], closed=True)
urinal.circle((0, 0.15), 0.03)
_, look = clearance(urinal, -0.4, 0.35, 0.4, 0.95)
urinal.dynamic([look], [])
blocks.append(urinal)

corner = Block(27, "Köşe duş", "Islak hacim/Duş", ["duş", "köşe", "oval", "banyo"],
               "Oval köşe duşakabin 90 × 90 cm; taban noktası köşe, iki duvar x ve y boyunca. Boşluk zarfı girişin önünde.")
corner.path([(0, 0), (0.9, 0), (0.9, 0.35)])
corner.arc((0.35, 0.35), 0.55, 0, Q)
corner.path([(0.35, 0.9), (0, 0.9), (0, 0)])
corner.circle((0.25, 0.25), 0.04)
_, look = clearance(corner, 0.55, 0.55, 1.35, 1.35)
corner.dynamic([look], [])
blocks.append(corner)

vanity = Block(28, "Dolaplı lavabo", "Islak hacim/Lavabo", ["lavabo", "dolap", "banyo"],
               "Banyo dolabı ve lavabo 80 × 46 cm; taban noktası duvarda ortası. Boşluk zarfı önünde 80 × 70 cm.")
vanity.rect(-0.4, 0, 0.4, 0.46)
vanity.ellipse((0, 0.25), (0.22, 0), 0.6)
vanity.circle((0, 0.06), 0.02)
_, look = clearance(vanity, -0.4, 0.46, 0.4, 1.16)
vanity.dynamic([look], [])
blocks.append(vanity)


def appliance(n, name, category, tags, description, label, width=0.6, depth=0.6):
    """A white good under or beside the counter: its box, its front, its label."""
    b = Block(n, name, category, tags, description)
    b.rect(-width / 2, 0, width / 2, depth)
    b.line((-width / 2, depth - 0.04), (width / 2, depth - 0.04))
    b.text((-0.08, depth / 2 - 0.05), label, 0.1)
    return b


washer = appliance(29, "Çamaşır makinesi", "Islak hacim/Beyaz eşya", ["çamaşır", "makine", "beyaz eşya"],
                   "Çamaşır makinesi 60 × 60 cm; taban noktası duvarda ortası. Kapağı önde.", "ÇM")
washer.circle((0, 0.3), 0.18)
blocks.append(washer)

dryer = appliance(30, "Kurutma makinesi", "Islak hacim/Beyaz eşya", ["kurutma", "makine", "beyaz eşya"],
                  "Kurutma makinesi 60 × 60 cm; taban noktası duvarda ortası.", "KM")
blocks.append(dryer)

sink = Block(31, "Evye", "Mutfak/Evye", ["evye", "eviye", "mutfak"],
             "Tek hazneli, süzgeçli evye 80 × 50 cm, tezgâha gömme; taban noktası duvarda sol köşe. Süzgeç solda çevrilir.")
sink.rect(0, 0, 0.8, 0.5)
bowl = sink.rect(0.38, 0.07, 0.76, 0.43)
drain = sink.circle((0.57, 0.25), 0.03)
ribs = [sink.line((0.06 + 0.06 * k, 0.1), (0.06 + 0.06 * k, 0.4)) for k in range(5)]
tap = sink.circle((0.57, 0.035), 0.02)
sink.dynamic(
    [{"kind": "flip", "name": "Süzgeç sağda", "a": P(0.4, 0), "b": P(0.4, 1)}],
    [{"kind": "flip", "parameter": "Süzgeç sağda", "pieces": [bowl, drain, tap] + ribs}])
blocks.append(sink)

hob = Block(32, "Ocak", "Mutfak/Pişirme", ["ocak", "mutfak"], "Dört gözlü ankastre ocak 60 × 52 cm; taban noktası duvarda ortası.")
hob.rect(-0.3, 0, 0.3, 0.52)
for (x, y, r) in ((-0.14, 0.14, 0.09), (0.14, 0.14, 0.07), (-0.14, 0.37, 0.07), (0.14, 0.37, 0.09)):
    hob.circle((x, y), r)
blocks.append(hob)

fridge = appliance(33, "Buzdolabı", "Mutfak/Beyaz eşya", ["buzdolabı", "beyaz eşya", "mutfak"],
                   "Buzdolabı 60 × 65 cm; taban noktası duvarda ortası. Kapı açılımı önünde.", "BZ", depth=0.65)
fridge.arc((-0.3, 0.65), 0.6, 0, Q)
blocks.append(fridge)

dishwasher = appliance(34, "Bulaşık makinesi", "Mutfak/Beyaz eşya", ["bulaşık", "makine", "beyaz eşya", "mutfak"],
                       "Ankastre bulaşık makinesi 60 × 60 cm, tezgâh altı; taban noktası duvarda ortası.", "BM")
blocks.append(dishwasher)

oven = appliance(35, "Fırın", "Mutfak/Pişirme", ["fırın", "ankastre", "mutfak"],
                 "Ankastre fırın 60 × 60 cm, tezgâh altı; taban noktası duvarda ortası.", "F")
blocks.append(oven)

base_unit = Block(36, "Alt dolap", "Mutfak/Dolap", ["dolap", "tezgâh", "mutfak"],
                  "Mutfak alt dolabı ve tezgâh, 60 cm derin; genişlik 30–120 cm. Taban noktası duvarda sol köşe.")
counter = base_unit.rect(0, 0, 0.6, 0.6)
front = base_unit.line((0, 0.56), (0.6, 0.56))
base_unit.dynamic(
    [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(0.6, 0), "rule": {"min": 0.3, "max": 1.2, "step": 0.15}}],
    [{"kind": "stretch", "parameter": "Genişlik", "pieces": [counter, front], "min": P(0.5, -0.1), "max": P(0.7, 0.7)}])
blocks.append(base_unit)

wall_unit = Block(37, "Üst dolap", "Mutfak/Dolap", ["dolap", "üst", "mutfak"],
                  "Mutfak üst dolabı 35 cm derin, planda kesikli; genişlik 30–120 cm. Taban noktası duvarda sol köşe.")
upper = dashed(wall_unit, wall_unit.rect(0, 0, 0.6, 0.35))
upper_cross = dashed(wall_unit, wall_unit.line((0, 0), (0.6, 0.35)))
wall_unit.dynamic(
    [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(0.6, 0), "rule": {"min": 0.3, "max": 1.2, "step": 0.15}}],
    [{"kind": "stretch", "parameter": "Genişlik", "pieces": [upper, upper_cross], "min": P(0.5, -0.1), "max": P(0.7, 0.45)}])
blocks.append(wall_unit)

LIBRARY = {"format": "kentos.blocks", "version": 1, "blocks": [b.data for b in blocks]}
TEXT = json.dumps(LIBRARY, ensure_ascii=False, indent=1) + "\n"


# The real sizes the library's descriptions give (ARCH-LIB-01, docs/adr/0702): the extent of what a block shows in its
# default state, texts and swing arcs aside, in metres, x then y as drawn.
SIZES = {
    "Tek kişilik yatak": (0.9, 2.0), "Komodin": (0.5, 0.4), "Koltuk": (0.85, 0.85), "İkili kanepe": (1.5, 0.85),
    "Yuvarlak masa": (2.08, 2.08), "Çalışma masası": (1.2, 0.6), "TV ünitesi": (1.8, 0.45), "Kitaplık": (0.9, 0.35),
    "Asma klozet": (0.5, 0.54), "Pisuvar": (0.36, 0.35), "Köşe duş": (0.9, 0.9), "Dolaplı lavabo": (0.8, 0.46),
    "Çamaşır makinesi": (0.6, 0.6), "Kurutma makinesi": (0.6, 0.6), "Evye": (0.8, 0.5), "Ocak": (0.6, 0.52),
    "Buzdolabı": (0.6, 0.65), "Bulaşık makinesi": (0.6, 0.6), "Fırın": (0.6, 0.6), "Alt dolap": (0.6, 0.6),
    "Üst dolap": (0.6, 0.35),
}


def extent(block):
    """What a block shows in its default visibility state, texts and arcs aside: its width and depth."""
    hidden = set()
    for p in block.get("dynamic", {}).get("parameters", []):
        if p["kind"] == "visibility":
            hidden.update(p["states"][0].get("hidden", []))
    xs, ys = [], []
    for e in block["entities"]:
        if e["id"] in hidden or e["kind"] in ("text", "arc"):
            continue
        if e["kind"] == "polyline":
            pts = [(q["x"], q["y"]) for q in e["pts"]]
        elif e["kind"] == "line":
            pts = [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])]
        elif e["kind"] == "circle":
            pts = [(e["c"]["x"] + e["r"] * math.cos(t * math.pi / 2), e["c"]["y"] + e["r"] * math.sin(t * math.pi / 2)) for t in range(4)]
        elif e["kind"] == "ellipse":
            mx, my = e["major"]["x"], e["major"]["y"]
            pts = [(e["c"]["x"] + mx * math.cos(t) - my * e["ratio"] * math.sin(t),
                    e["c"]["y"] + my * math.cos(t) + mx * e["ratio"] * math.sin(t)) for t in (k * math.pi / 180 for k in range(360))]
        else:
            raise AssertionError(e["kind"])
        xs += [q[0] for q in pts]
        ys += [q[1] for q in pts]
    return max(xs) - min(xs), max(ys) - min(ys)


for b in blocks:
    if b.data["name"] in SIZES:
        w, d = extent(b.data)
        ew, ed = SIZES[b.data["name"]]
        assert abs(w - ew) < 1e-3 and abs(d - ed) < 1e-3, (b.data["name"], w, d, ew, ed)
assert set(SIZES) <= {b.data["name"] for b in blocks}


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
