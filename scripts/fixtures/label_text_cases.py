#!/usr/bin/env python3
"""Independent reference of Etiketleri yazıya çevir (docs/adr/0175 §1).

Writes fixtures/label-text/v1/cases.json from the rule alone, with Python's
standard library and no KentOS code. The geometry core
(`ops::label_text::label_texts`, crates/shared/geometry-core/tests/all/label_text.rs)
and the web through its WASM must give the same: places and heights within
1e-9 m, rotations within 1e-9°, the same texts and counts.

Input: the labels in the drawing's order, each where its object puts it
(`placement`: center or beside at the object's anchor `p`, corner at its
box's top left `p`, along between two vertices `p` and `q`), the smaller
side of the object's box in metres (`feature`), its text (the template
filled), the text's width in em in the drawing's typeface (`em`), and its
layer's label style (size, grow, maxSize in CSS px; minScale, maxScale in
px per metre; minFeaturePx); the scale 1:N; whether overlapping labels are
thinned.

The rule (the sheet's, apps/web/src/app/sheet/mapLabels.ts):

1. k = 96 / 0.0254 / N CSS px per ground metre on the paper (96 px an inch).
2. An empty text is no text and is counted nowhere.
3. Out of the style's range (k < minScale or k > maxScale): not written,
   counted as out of scale.
4. feature · k < minFeaturePx: not written, counted as small.
5. The size s = min(maxSize, size + grow · k) px, maxSize defaulting to size
   and grow to 0; s ≤ 0 is counted as small. The text's height is s / k m.
6. Where, on the paper (X = x · k, Y = −y · k, px; w = em · s):
   - center: at p, aligned middle centre, box X ± w/2, Y ± s/2;
   - corner: at p moved 8 px right and 14 px down (x + 8/k, y − 14/k),
     aligned middle left, box X + 8 … X + 8 + w, Y + 14 ± s/2;
   - beside: at p moved 7 px right and 7 px up (x + 7/k, y + 7/k), aligned
     middle left, box X + 7 … X + 7 + w, Y − 7 ± s/2;
   - along: a = atan2(q − p) on the ground; past ±90° it turns half a turn
     (to stay readable); at the middle of p and q, rotated a (degrees, in
     (−180°, 180°]), aligned middle centre; the box about the middle M,
     half widths (|cos a|·w + |sin a|·s)/2 and (|sin a|·w + |cos a|·s)/2.
7. Thinning (unless every label is wanted): the paper in 8 px cells from the
   drawing's origin; a label whose box's cells (floor of each edge / 8, both
   ends included) touch a taken cell is not written and is counted as
   overlapping, else it takes them. Labels come in the given order.

Every case keeps each box edge at least 1e-6 px off a cell's edge, so a
rounding in the last place cannot move a label between cells.

    python3 scripts/fixtures/label_text_cases.py           # writes the cases
    python3 scripts/fixtures/label_text_cases.py --check   # compares with the file
"""

import argparse
import json
import math
import random
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "label-text" / "v1" / "cases.json"
E0, N0 = 487000.0, 4420000.0
CELL = 8
MARGIN = 1e-6


class Fragile(Exception):
    """A box edge too near a cell's edge for the case to be fair."""


def px_per_m(n):
    return F(480000) / (127 * F(n))


def floor_cell(edge):
    q = edge / CELL
    if abs(float(q - round(q))) * CELL < MARGIN:
        raise Fragile(float(edge))
    return math.floor(q)


def solve(items, n, thin):
    k = px_per_m(n)
    texts = []
    out_of_scale = small = overlapping = 0
    taken = set()
    for i, it in enumerate(items):
        if not it["text"]:
            continue
        kf = float(k)
        if (it.get("minScale") is not None and kf < it["minScale"]) or (it.get("maxScale") is not None and kf > it["maxScale"]):
            out_of_scale += 1
            continue
        if it.get("minFeaturePx") is not None and float(F(it["feature"]) * k) < it["minFeaturePx"]:
            small += 1
            continue
        size = F(it["size"])
        top = F(it["maxSize"]) if it.get("maxSize") is not None else size
        s = min(top, size + F(it.get("grow") or 0) * k)
        if s <= 0:
            small += 1
            continue
        w = F(it["em"]) * s
        px, py = F(it["p"]["x"]), F(it["p"]["y"])
        X, Y = px * k, -py * k
        rotation = 0.0
        place = it["placement"]
        if place == "center":
            at, align = (px, py), "middleCenter"
            box = (X - w / 2, Y - s / 2, X + w / 2, Y + s / 2)
        elif place == "corner":
            at, align = (px + 8 / k, py - 14 / k), "middleLeft"
            box = (X + 8, Y + 14 - s / 2, X + 8 + w, Y + 14 + s / 2)
        elif place == "beside":
            at, align = (px + 7 / k, py + 7 / k), "middleLeft"
            box = (X + 7, Y - 7 - s / 2, X + 7 + w, Y - 7 + s / 2)
        else:
            qx, qy = F(it["q"]["x"]), F(it["q"]["y"])
            a = math.atan2(float(qy - py), float(qx - px))
            if a > math.pi / 2 or a < -math.pi / 2:
                a += math.pi
            rotation = math.degrees(a)
            if rotation > 180:
                rotation -= 360
            at, align = ((px + qx) / 2, (py + qy) / 2), "middleCenter"
            mx, my = (X + qx * k) / 2, (Y - qy * k) / 2
            c, sn = F(abs(math.cos(a))), F(abs(math.sin(a)))
            hx, hy = (c * w + sn * s) / 2, (sn * w + c * s) / 2
            box = (mx - hx, my - hy, mx + hx, my + hy)
        if thin:
            c0, r0, c1, r1 = (floor_cell(e) for e in box)
            cells = [(c, r) for c in range(c0, c1 + 1) for r in range(r0, r1 + 1)]
            if any(cell in taken for cell in cells):
                overlapping += 1
                continue
            taken.update(cells)
        texts.append({"item": i, "text": it["text"], "p": P(float(at[0]), float(at[1])), "height": float(s / k), "rotation": rotation, "align": align})
    return {"texts": texts, "outOfScale": out_of_scale, "small": small, "overlapping": overlapping}


def P(x, y):
    return {"x": x, "y": y}


def item(placement, p, text, em, size, feature=100.0, q=None, **style):
    it = {"placement": placement, "p": P(*p), "q": P(*q) if q is not None else None, "feature": feature, "text": text, "em": em, "size": size}
    for key in ("grow", "maxSize", "minScale", "maxScale", "minFeaturePx"):
        it[key] = style.get(key)
    return it


# The layers' default labels (apps/web/src/viewport/storeRecords.ts DEFAULT_LABELS), the commonest styles.
PARCEL = dict(size=10, grow=1, maxSize=14, minFeaturePx=26)
POINT = dict(size=10.5, minScale=2)
ROAD = dict(size=10, minScale=1.6)


def hand():
    cases = []
    c = lambda name, items, n, thin=True: cases.append((name, items, n, thin))
    c("merkez: çapada, ortanın ortası", [item("center", (10.3, 20.1), "101", 1.7, 10)], 1000)
    c("büyüme ve üst sınır, 1:1000 (10 + k px)", [item("center", (10.3, 20.1), "101", 1.7, **PARCEL)], 1000)
    c("büyüme üst sınıra dayanır, 1:500 (14 px)", [item("center", (10.3, 20.1), "101", 1.7, **PARCEL)], 500)
    c("büyüme, 1:5000", [item("center", (10.3, 20.1), "101", 1.7, **PARCEL)], 5000)
    c("üst sınırsız büyüme boyutu değiştirmez", [item("center", (10.3, 20.1), "101", 1.7, 10, grow=1)], 1000)
    c("köşe: kutunun sol üstünden 8 px sağa, 14 px aşağıya", [item("corner", (3.1, 40.3), "Ada 12", 2.9, 11)], 1000)
    c("yan: 7 px sağa, 7 px yukarıya", [item("beside", (5.2, 5.3), "P12", 1.5, **POINT)], 1000)
    c("nokta ölçek aralığının altında (k < 2)", [item("beside", (5.2, 5.3), "P12", 1.5, **POINT)], 2000)
    c("üst ölçek sınırı", [item("center", (10.3, 20.1), "101", 1.7, 10, maxScale=3)], 1000)
    c("üst ölçek sınırının içinde", [item("center", (10.3, 20.1), "101", 1.7, 10, maxScale=3)], 2000)
    c("en küçük nesneden küçük", [item("center", (10.3, 20.1), "101", 1.7, feature=6.5, **PARCEL)], 1000)
    c("en küçük nesneden büyük", [item("center", (10.3, 20.1), "101", 1.7, feature=7.5, **PARCEL)], 1000)
    c("boyunca: doğuya", [item("along", (0.3, 0.2), "Atatürk Cd.", 5.2, q=(30.3, 0.2), **ROAD)], 1000)
    c("boyunca: batıya giden yol okunur yöne döner", [item("along", (30.3, 0.2), "Atatürk Cd.", 5.2, q=(0.3, 0.2), **ROAD)], 1000)
    c("boyunca: kuzeye 90°", [item("along", (0.3, 0.2), "Sokak", 2.6, q=(0.3, 30.2), **ROAD)], 1000)
    c("boyunca: güneye −90°", [item("along", (0.3, 30.2), "Sokak", 2.6, q=(0.3, 0.2), **ROAD)], 1000)
    c("boyunca: kuzeybatıya giden döner (−45°)", [item("along", (20.3, 0.2), "Sokak", 2.6, q=(0.3, 20.2), **ROAD)], 1000)
    c("boyunca: güneybatıya giden döner (45°)", [item("along", (20.3, 20.2), "Sokak", 2.6, q=(0.3, 0.2), **ROAD)], 1000)
    c("boyunca: kuzeydoğuya", [item("along", (0.3, 0.2), "Sokak", 2.6, q=(20.3, 10.2), **ROAD)], 1000)
    c("boyunca: ölçek aralığının altında", [item("along", (0.3, 0.2), "Sokak", 2.6, q=(20.3, 10.2), **ROAD)], 5000)
    c("boş yazı yazılmaz, sayılmaz", [item("center", (10.3, 20.1), "", 1.7, 10), item("center", (40.3, 20.1), "102", 1.7, 10)], 1000)
    c("sıfır boyut küçük sayılır", [item("center", (10.3, 20.1), "101", 1.7, 10, grow=-3, maxSize=20)], 1000)
    same = [item("center", (10.3, 20.1), "101", 1.7, 10), item("center", (10.4, 20.2), "102", 1.7, 10)]
    c("aynı yerdeki ikinci etiket örtüşür", same, 1000)
    c("örtüşenler de: ikisi de yazılır", same, 1000, False)
    apart = [item("center", (10.3, 20.1), "101", 1.7, 10), item("center", (40.3, 20.1), "102", 1.7, 10)]
    c("uzak etiketler ikisi de", apart, 1000)
    c("1:10000'de aynı etiketler örtüşür", apart, 10000)
    c("yakın kutular ayrı hücrelerde: ikisi de", [item("center", (10.3, 20.1), "101", 1.7, 10), item("center", (16.3, 20.1), "102", 1.7, 10)], 1000)
    c("kutular kısmen örtüşür", [item("center", (10.3, 20.1), "101", 1.7, 10), item("center", (14.3, 20.6), "102", 1.7, 10)], 1000)
    # Boxes 3 px apart in the same cell: on the paper (X right, Y down) the first ends at 802 px, the second starts at
    # 805 px, both in cell 100; rows −409 … −399 px.
    k = px_per_m(1000)
    x1, x2, y = float(F(7935, 10) / k), float(F(8135, 10) / k), float(F(404) / k)
    c("örtüşmeyen ama aynı hücreye düşen kutular", [item("center", (x1, y), "101", 1.7, 10), item("center", (x2, y), "102", 1.7, 10)], 1000)
    c("ilk gelen kalır, sonrakiler sayılır", [item("center", (10.3, 20.1), "101", 1.7, 10), item("beside", (9.3, 18.6), "P1", 1.5, 10), item("center", (10.5, 20.3), "103", 1.7, 10)], 1000)
    return cases


def moved(case, east, north):
    """The case near (east, north), moved by whole cells at its scale so that it thins as it did."""
    name, items, n, thin = case
    cell = F(CELL) / px_per_m(n)
    de, dn = (float(round(F(v) / cell) * cell) for v in (east, north))
    mv = lambda p: None if p is None else P(p["x"] + de, p["y"] + dn)
    return (name + ", TM koordinatlarında", [{**it, "p": mv(it["p"]), "q": mv(it["q"])} for it in items], n, thin)


def sample(seed):
    """Parcels on a jittered grid (centre or corner labels), points beside, roads along; a random scale."""
    rnd = random.Random(seed)
    r3 = lambda v: round(v, 3)
    x0, y0 = E0 + rnd.uniform(-500, 500), N0 + rnd.uniform(-500, 500)
    items = []
    parcel = rnd.choice([PARCEL, dict(size=11, minFeaturePx=20), dict(size=9, grow=0.5, maxSize=16)])
    place = rnd.choice(["center", "corner"])
    for i in range(rnd.randint(2, 6)):
        for j in range(rnd.randint(1, 4)):
            w, h = rnd.uniform(8, 30), rnd.uniform(8, 30)
            x, y = x0 + i * 30, y0 + j * 30
            p = (r3(x + w / 2 + rnd.uniform(-1, 1)), r3(y + h / 2 + rnd.uniform(-1, 1))) if place == "center" else (r3(x), r3(y + h))
            items.append(item(place, p, str(100 + len(items)), r3(rnd.uniform(1.2, 2.6)), feature=r3(min(w, h)), **parcel))
    for _ in range(rnd.randint(0, 6)):
        p = (r3(x0 + rnd.uniform(0, 150)), r3(y0 + rnd.uniform(0, 120)))
        items.append(item("beside", p, f"P{rnd.randint(1, 300)}", r3(rnd.uniform(1.0, 2.2)), feature=0.0, **POINT))
    for _ in range(rnd.randint(0, 3)):
        a = (r3(x0 + rnd.uniform(-20, 170)), r3(y0 + rnd.uniform(-20, 140)))
        t = rnd.uniform(-math.pi, math.pi)
        length = rnd.uniform(5, 60)
        b = (r3(a[0] + math.cos(t) * length), r3(a[1] + math.sin(t) * length))
        items.append(item("along", a, rnd.choice(["Cumhuriyet Cd.", "1203. Sk.", "Dere"]), r3(rnd.uniform(2, 7)), q=b, feature=r3(length / 3), **ROAD))
    rnd.shuffle(items)
    n = rnd.choice([200, 500, 1000, 2000, 2500, 5000])
    return (f"rastgele etiketler {seed}, 1:{n}", items, n, rnd.random() < 0.8)


def build():
    out = []
    add = lambda name, items, n, thin, expected: out.append({"name": name, "items": items, "scale": n, "thin": thin, "expected": expected})
    for case in hand():
        for name, items, n, thin in (case, moved(case, E0, N0)):
            add(name, items, n, thin, solve(items, n, thin))
    # Random samples; one with a box edge on a cell's edge is passed over (see Fragile).
    made, seed = 0, 1
    while made < 24:
        name, items, n, thin = sample(seed)
        seed += 1
        try:
            expected = solve(items, n, thin)
        except Fragile:
            continue
        add(name, items, n, thin, expected)
        made += 1
    return {"format": "kentos.label-text-fixtures", "version": 1, "cases": out}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/label_text_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
