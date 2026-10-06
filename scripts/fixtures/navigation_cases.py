"""Independent reference of Genel bakış and Büyüteç's rules (docs/adr/0181), and the cases both platforms play.

- Cards (§4): where the overview's and the magnifier's cards stand in the drawing area, and the side the magnifier takes as the
  pointer comes near it.
- Overview mapping (§3): the extent fitted in the picture, the view's frame on it, a press turned into the view's new centre.
- Overview picture (§3): the picture's pixels, from the objects' shapes, by the exact pixel rules; written as rows of letters
  (`.` clear, a capital its layer's colour opaque, a small letter the same colour at 64 of 255).

    python3 scripts/fixtures/navigation_cases.py           # writes fixtures/navigation/v1/cases.json
    python3 scripts/fixtures/navigation_cases.py --check   # writes nothing; compares

The rules are written here again from the ADR, not from either platform's code. Straight geometry takes only additions,
subtractions, products and quotients, which IEEE 754 rounds the same everywhere (in the same order, without fused products, as
both platforms do); a number taken from a cosine or a sine (a circle's or an arc's points, an arc's box) is checked to be farther
than 1e-9 from a pixel's edge, so a last-bit difference between two maths libraries cannot move a pixel.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/navigation/v1/cases.json"

MARGIN = 8
GAP = 8
NEAR = 16
NORTH = 52
OVERVIEW = (240, 160)
LENS = (220, 220)
PAD = 6
FILL_ALPHA = 64


# ── Cards ───────────────────────────────────────────────────────────────────────────────────────────────────────────────


def card(x, y, content, header):
    """A card's rectangle and its content's, as [x, y, width, height]."""
    w, h = content
    return {"card": [x, y, w + 2, header + h + 2], "content": [x + 1, y + 1 + header, w, h]}


def cards(area, header, cbs, overview, side):
    W, H = area
    out = {"overview": card(MARGIN, MARGIN, OVERVIEW, header) if overview else None}
    lens_w, lens_h = LENS[0] + 2, header + LENS[1] + 2
    right = card(W - MARGIN - lens_w, MARGIN + (NORTH if cbs else 0), LENS, header)
    if overview:
        below = MARGIN + header + OVERVIEW[1] + 2 + GAP
        if below + lens_h <= H - MARGIN:
            left = card(MARGIN, below, LENS, header)
        else:
            left = card(MARGIN + OVERVIEW[0] + 2 + GAP, MARGIN, LENS, header)
    else:
        left = card(MARGIN, MARGIN, LENS, header)
    out["right"] = right
    out["left"] = left
    return out


def near(rect, p):
    x, y, w, h = rect
    return x - NEAR <= p[0] <= x + w + NEAR and y - NEAR <= p[1] <= y + h + NEAR


def next_side(layout, side, pointer):
    here = layout[side]["card"]
    other = "left" if side == "right" else "right"
    if near(here, pointer) and not near(layout[other]["card"], pointer):
        return other
    return side


# ── Overview mapping ────────────────────────────────────────────────────────────────────────────────────────────────────


def fit(extent, size):
    x0, y0, x1, y1 = extent
    W, H = size
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    w = max(x1 - x0, 1.0)
    h = max(y1 - y0, 1.0)
    k = min((W - 2 * PAD) / w, (H - 2 * PAD) / h)
    return cx, cy, k


def to_card(f, size, x, y):
    cx, cy, k = f
    return size[0] / 2 + (x - cx) * k, size[1] / 2 - (y - cy) * k


def to_world(f, size, u, v):
    cx, cy, k = f
    return cx + (u - size[0] / 2) / k, cy - (v - size[1] / 2) / k


def view_frame(f, size, center, metres_per_pixel, view_px):
    hw = view_px[0] * metres_per_pixel / 2
    hh = view_px[1] * metres_per_pixel / 2
    u0, v0 = to_card(f, size, center[0] - hw, center[1] + hh)
    u1, v1 = to_card(f, size, center[0] + hw, center[1] - hh)
    return [u0, v0, u1, v1], (u1 - u0) < 6 and (v1 - v0) < 6


# ── Overview picture ────────────────────────────────────────────────────────────────────────────────────────────────────


def arc_points(c, r, a0, sweep, kp):
    n = max(4, math.ceil(abs(sweep) * r * kp / 4))
    n = min(n, 720)
    return [(c[0] + r * math.cos(a0 + sweep * i / n), c[1] + r * math.sin(a0 + sweep * i / n)) for i in range(n + 1)]


def arc_bounds(c, r, a0, a1):
    """An arc's exact box, counter-clockwise from a0 to a1."""
    sweep = a1 - a0
    while sweep <= 0:
        sweep += 2 * math.pi
    pts = [(c[0] + r * math.cos(a0), c[1] + r * math.sin(a0)), (c[0] + r * math.cos(a0 + sweep), c[1] + r * math.sin(a0 + sweep))]
    for q in range(8):
        a = q * math.pi / 2
        d = (a - a0) % (2 * math.pi)
        if d <= sweep:
            pts.append((c[0] + r * math.cos(a), c[1] + r * math.sin(a)))
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return [min(xs), min(ys), max(xs), max(ys)]


def bounds(s):
    k = s["kind"]
    if k == "point":
        return [s["p"][0], s["p"][1], s["p"][0], s["p"][1]]
    if k == "line":
        xs, ys = [s["a"][0], s["b"][0]], [s["a"][1], s["b"][1]]
    elif k in ("polyline", "polygon"):
        pts = [p for ring in ([s["points"]] if k == "polyline" else s["rings"]) for p in ring]
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    elif k == "circle":
        return [s["c"][0] - s["r"], s["c"][1] - s["r"], s["c"][0] + s["r"], s["c"][1] + s["r"]]
    elif k == "arc":
        return arc_bounds(s["c"], s["r"], s["a0"], s["a1"])
    else:
        raise ValueError(k)
    return [min(xs), min(ys), max(xs), max(ys)]


def edge_check(v, what):
    """A value a pixel is taken from must not sit on a pixel's edge within 1e-9."""
    f = v - math.floor(v)
    assert 1e-9 < f < 1 - 1e-9, f"{what}: {v} is on a pixel's edge"


def picture(objects, colors, size, dpr):
    extent = None
    for o in objects:
        b = bounds(o["shape"])
        extent = b if extent is None else [min(extent[0], b[0]), min(extent[1], b[1]), max(extent[2], b[2]), max(extent[3], b[3])]
    straight = None
    for o in objects:
        if o["shape"]["kind"] in ("circle", "arc"):
            continue
        b = bounds(o["shape"])
        straight = b if straight is None else [min(straight[0], b[0]), min(straight[1], b[1]), max(straight[2], b[2]),
                                               max(straight[3], b[3])]
    assert straight == extent, "a curve's box must not reach the extent's edge: the store's box of a curve is its own rule"
    cx, cy, k = fit(extent, size)
    Wp = math.floor(size[0] * dpr + 0.5)
    Hp = math.floor(size[1] * dpr + 0.5)
    kp = k * dpr
    d = max(1, math.floor(dpr + 0.5))
    px = [[None] * Wp for _ in range(Hp)]

    def at(x, y, what, curved=False):
        u = Wp / 2 + (x - cx) * kp
        v = Hp / 2 - (y - cy) * kp
        if curved:
            edge_check(u, what)
            edge_check(v, what)
        return math.floor(u), math.floor(v), u, v

    def plot(i, j, layer, alpha):
        if 0 <= i < Wp and 0 <= j < Hp:
            px[j][i] = (layer, alpha)

    def dot(i, j, layer):
        for dj in range(d):
            for di in range(d):
                plot(i + di, j + dj, layer, 255)

    def segment(a, b, layer):
        x0, y0 = a
        x1, y1 = b
        dx = abs(x1 - x0)
        sx = 1 if x0 < x1 else -1
        dy = -abs(y1 - y0)
        sy = 1 if y0 < y1 else -1
        err = dx + dy
        while True:
            plot(x0, y0, layer, 255)
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x0 += sx
            if e2 <= dx:
                err += dx
                y0 += sy

    def fill(rings, layer, curved=False):
        edges = []
        for ring in rings:
            pts = [(Wp / 2 + (x - cx) * kp, Hp / 2 - (y - cy) * kp) for x, y in ring]
            for i in range(len(pts)):
                edges.append((pts[i], pts[(i + 1) % len(pts)]))
        for j in range(Hp):
            yc = j + 0.5
            xs = []
            for (ax, ay), (bx, by) in edges:
                if (ay <= yc) != (by <= yc):
                    x = ax + (yc - ay) * (bx - ax) / (by - ay)
                    if curved:
                        edge_check(x - 0.5, "fill crossing")
                    xs.append(x)
            xs.sort()
            for n in range(0, len(xs) - 1, 2):
                for i in range(max(0, math.ceil(xs[n] - 0.5)), min(Wp, math.ceil(xs[n + 1] - 0.5))):
                    if px[j][i] is None or px[j][i][1] < 255:
                        px[j][i] = (layer, FILL_ALPHA)

    def path(points, closed, layer, what, curved=False):
        cells = [at(x, y, what, curved)[:2] for x, y in points]
        if closed:
            cells.append(cells[0])
        for a, b in zip(cells, cells[1:]):
            segment(a, b, layer)

    for o in objects:
        s, layer = o["shape"], o["layer"]
        b = bounds(s)
        if (b[2] - b[0]) * kp < 2 and (b[3] - b[1]) * kp < 2 or s["kind"] == "point":
            i, j, _, _ = at((b[0] + b[2]) / 2, (b[1] + b[3]) / 2, "dot", s["kind"] in ("circle", "arc"))
            dot(i, j, layer)
            continue
        k_ = s["kind"]
        if k_ == "line":
            path([s["a"], s["b"]], False, layer, "line")
        elif k_ == "polyline":
            path(s["points"], False, layer, "polyline")
        elif k_ == "polygon":
            fill(s["rings"], layer)
            for ring in s["rings"]:
                path(ring, True, layer, "polygon")
        elif k_ == "circle":
            ring = arc_points(s["c"], s["r"], 0.0, 2 * math.pi, kp)[:-1]
            fill([ring], layer, True)
            path(ring, True, layer, "circle", True)
        elif k_ == "arc":
            sweep = s["a1"] - s["a0"]
            while sweep <= 0:
                sweep += 2 * math.pi
            path(arc_points(s["c"], s["r"], s["a0"], sweep, kp), False, layer, "arc", True)

    rows = []
    for j in range(Hp):
        row = ""
        for i in range(Wp):
            p = px[j][i]
            if p is None:
                row += "."
            else:
                letter = "ABCDEFGH"[colors.index(p[0])]
                row += letter if p[1] == 255 else letter.lower()
        rows.append(row)
    return {"extent": extent, "fit": [cx, cy, k], "width": Wp, "height": Hp, "rows": rows}


# ── Cases ───────────────────────────────────────────────────────────────────────────────────────────────────────────────


def cases():
    card_cases = []
    for name, area, header, cbs, overview, side, pointer in [
        ("CBS, Genel bakış kapalı, büyüteç sağda, imleç ortada", (1112, 640), 24, True, False, "right", (556, 320)),
        ("CBS, imleç sağdaki büyütecin yakınında: sola geçer", (1112, 640), 24, True, False, "right", (900, 100)),
        ("CBS, solda iken imleç yaklaşınca sağa döner", (1112, 640), 24, True, False, "left", (40, 120)),
        ("CAD: sağdaki büyüteç kuzey okusuz en üstte", (1112, 640), 24, False, True, "right", (556, 320)),
        ("Genel bakış açık: soldaki büyüteç onun altında", (1112, 640), 24, True, True, "left", (556, 320)),
        ("1100 × 650'de alan alçak: soldaki büyüteç Genel bakışın yanında", (788, 426), 24, True, True, "left", (700, 300)),
        ("Dar alanda iki yan da yakın: yerinde kalır", (480, 300), 24, False, False, "right", (240, 100)),
        ("Büyük yazı: başlık 30 piksel", (1112, 640), 30, True, True, "right", (556, 320)),
    ]:
        layout = cards(area, header, cbs, overview, side)
        card_cases.append({"name": name, "area": list(area), "header": header, "cbs": cbs, "overview": overview, "side": side,
                           "pointer": list(pointer), "layout": layout, "next": next_side(layout, side, pointer)})

    fits = []
    for name, extent, size, center, mpp, view_px, presses in [
        ("Yatay kapsam: genişlik sınırlar", [487000, 4420000, 487600, 4420200], OVERVIEW, [487300, 4420100], 0.5, [800, 500],
         [[120, 80], [6, 6], [234, 154]]),
        ("Dikey kapsam: yükseklik sınırlar", [0, 0, 100, 400], OVERVIEW, [50, 300], 0.125, [800, 500], [[120, 80]]),
        ("Tek nokta: 1 m'ye büyür", [5, 5, 5, 5], OVERVIEW, [5, 5], 0.01, [400, 300], [[120, 80]]),
        ("Görünüm kapsamdan büyük: çerçeve kartı aşar", [0, 0, 100, 50], OVERVIEW, [50, 25], 1.0, [1000, 600], []),
        ("Çok yakın görünüm: artı", [0, 0, 10000, 5000], OVERVIEW, [2500, 1250], 0.01, [800, 500], []),
    ]:
        f = fit(extent, size)
        frame, cross = view_frame(f, size, center, mpp, view_px)
        fits.append({"name": name, "extent": extent, "size": list(size), "center": center, "metresPerPixel": mpp,
                     "viewPx": view_px, "fit": list(f), "frame": frame, "cross": cross,
                     "presses": [{"at": p, "world": list(to_world(f, size, *p))} for p in presses]})

    colors = ["Sınır", "Yol", "Bina"]
    pictures = []
    for name, size, dpr, objects in [
        ("Çizgi, çoklu çizgi ve kapalı alan", (40, 28), 1.0, [
            {"layer": "Sınır", "shape": {"kind": "polygon", "rings": [[[0.13, 0.17], [30.11, 0.17], [30.11, 20.23], [0.13, 20.23]]]}},
            {"layer": "Yol", "shape": {"kind": "line", "a": [2.07, 10.31], "b": [28.03, 12.29]}},
            {"layer": "Bina", "shape": {"kind": "polyline", "points": [[5.03, 3.11], [9.07, 3.11], [9.07, 7.13]]}},
        ]),
        ("Delikli alan: delik dolmaz", (36, 36), 1.0, [
            {"layer": "Sınır", "shape": {"kind": "polygon", "rings": [
                [[0.11, 0.13], [20.17, 0.13], [20.17, 20.19], [0.11, 20.19]],
                [[6.07, 6.09], [14.03, 6.09], [14.03, 14.11], [6.07, 14.11]]]}},
        ]),
        ("Daire, yay ve küçük nesneler nokta (kapsamı düz kenarlı çerçeve verir)", (40, 30), 1.0, [
            {"layer": "Sınır", "shape": {"kind": "polyline",
                                         "points": [[0.03, 0.05], [40.07, 0.05], [40.07, 26.09], [0.03, 26.09], [0.03, 0.05]]}},
            {"layer": "Bina", "shape": {"kind": "circle", "c": [10.07, 10.03], "r": 8.31}},
            {"layer": "Yol", "shape": {"kind": "arc", "c": [30.11, 10.07], "r": 6.29, "a0": 0.3, "a1": 2.7}},
            {"layer": "Sınır", "shape": {"kind": "point", "p": [24.13, 18.17]}},
            {"layer": "Sınır", "shape": {"kind": "line", "a": [2.11, 18.07], "b": [2.19, 18.13]}},
        ]),
        ("İki kat yoğunlukta: noktalar 2 × 2", (24, 16), 2.0, [
            {"layer": "Yol", "shape": {"kind": "line", "a": [0.07, 0.11], "b": [20.03, 9.13]}},
            {"layer": "Bina", "shape": {"kind": "point", "p": [15.17, 2.09]}},
            {"layer": "Sınır", "shape": {"kind": "polygon", "rings": [[[3.09, 5.13], [8.11, 5.13], [5.03, 9.07]]]}},
        ]),
        ("Üstteki alanın dolgusu alttakinin dolgusunu örter, çizgisini örtmez", (32, 24), 1.0, [
            {"layer": "Sınır", "shape": {"kind": "polygon", "rings": [[[0.07, 0.09], [12.11, 0.09], [12.11, 10.13], [0.07, 10.13]]]}},
            {"layer": "Bina", "shape": {"kind": "polygon", "rings": [[[6.03, 4.07], [18.13, 4.07], [18.13, 14.11], [6.03, 14.11]]]}},
        ]),
    ]:
        pic = picture(objects, colors, size, dpr)
        pictures.append({"name": name, "size": list(size), "dpr": dpr, "layers": colors, "objects": objects, **pic})

    return {"format": "kentos.navigation-cases", "version": 1,
            "note": "Written by scripts/fixtures/navigation_cases.py from docs/adr/0181, not from either platform's code. Rectangles are "
                    "[x, y, width, height] in the drawing area's logical pixels; a fit is [cx, cy, k] (k pixels per metre); a "
                    "frame is [u0, v0, u1, v1] in the overview's content pixels; a picture's rows go top down, `.` clear, a capital "
                    "the layer's colour (by its place in `layers`) opaque, a small letter the same at 64 of 255.",
            "cards": card_cases, "fits": fits, "pictures": pictures}


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/navigation_cases.py ile yeniden yazın")
            sys.exit(1)
        print(f"Genel bakış ve büyüteç durumları tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
