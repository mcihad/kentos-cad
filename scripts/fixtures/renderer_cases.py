#!/usr/bin/env python3
"""The thematic renderers' rules (docs/adr/0213 §2), from the ADR, not from either platform's code:
fixtures/renderers/v1/cases.json.

- ramps: a ramp's colour at a share (equally spaced stops, channels mixed straight, halves rounded up);
- unclassed: a value's share of [min, max] in 256 steps and its colour (Sürekli renk);
- sizes: Orantılı sembol's size at a value (QGIS's size assistant: Alan, Yarıçap, Flannery) in 256 steps;
- classes: a value's class among breaks (İki değişkenli renk);
- dots: Nokta yoğunluğu's count and dots (SplitMix64 seeded by the seed, the area's vertices and the value's place;
  the box's points kept when inside, even-odd), bit for bit;
- charts: a pie's slices and bars as rings (Grafik), to 1e-9;
- groups: Kümeleme's and Yayma's groups and their centres, bit for bit;
- displaced: where Yayma puts a group's points, to 1e-9;
- heat: Isı haritası's grid, values, maximum (`top`: the given `max`, else the values' largest) and pixels (QGIS's
  quartic kernel, points cut to their cells);
- inverted: Ters alan's region's area by shapely (even-odd: the areas' symmetric difference; non-zero: their union),
  which the core's trapezoids must add up to.

The Rust core (`crates/shared/style-core/tests/all/renderers.rs`) must give the same.

`--check` says whether the file holds what this script writes.
"""
import json
import math
import struct
import sys
from pathlib import Path

from shapely.geometry import Polygon, box as shapely_box
from shapely.ops import unary_union

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/renderers/v1/cases.json"
MASK = (1 << 64) - 1

# ── Colours ─────────────────────────────────────────────────────────────


def parse(c):
    h = c[1:]
    v = [int(h[i:i + 2], 16) for i in range(0, len(h), 2)]
    return v + [255] if len(v) == 3 else v


def hexa(c):
    s = "#" + "".join(f"{v:02X}" for v in c[:3])
    return s if c[3] == 255 else s + f"{c[3]:02X}"


def round_half_up(x):
    return math.floor(x + 0.5)


def ramp_color(stops, t):
    stops = [parse(s) for s in stops]
    if not stops:
        return [0, 0, 0, 255]
    if len(stops) == 1:
        return stops[0]
    t = 0.0 if t != t else min(1.0, max(0.0, t))
    at = t * (len(stops) - 1)
    k = min(int(math.floor(at)), len(stops) - 2)
    f = at - k
    a, b = stops[k], stops[k + 1]
    return [min(255, max(0, round_half_up(a[i] + (b[i] - a[i]) * f))) for i in range(4)]


def share(v, lo, hi):
    if not hi - lo > 0:
        return 0.0
    return min(1.0, max(0.0, (v - lo) / (hi - lo)))


def step(t):
    return round_half_up(255.0 * min(1.0, max(0.0, t)))


EXPONENT = {"area": 0.5, "radius": 1.0, "flannery": 0.57}


def size_at(t, lo, hi, e):
    t = min(1.0, max(0.0, t))
    return lo + (hi - lo) * (t if e == 1.0 else t ** e)


def class_of(v, breaks):
    return sum(1 for b in breaks if v >= b)


# ── Dots ────────────────────────────────────────────────────────────────


class SplitMix64:
    def __init__(self, state):
        self.state = state & MASK

    def next(self):
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        return z ^ (z >> 31)

    def unit(self):
        return (self.next() >> 11) / 9007199254740992.0


def rings_hash(parts):
    h = 0xCBF29CE484222325
    for rings in parts:
        for ring in rings:
            for x, y in ring:
                for v in (x, y):
                    for b in struct.pack("<d", v):
                        h ^= b
                        h = (h * 0x00000100000001B3) & MASK
    return h


def seed_of(seed, area, field):
    return (area ^ ((seed * 0x9E3779B97F4A7C15) & MASK) ^ (((field + 1) * 0xD1B54A32D192ED03) & MASK)) & MASK


def inside(parts, x, y):
    odd = False
    for rings in parts:
        for ring in rings:
            n = len(ring)
            if n < 3:
                continue
            for i in range(n):
                ax, ay = ring[i]
                bx, by = ring[(i + 1) % n]
                if ay == by:
                    continue
                if (ay > y) != (by > y) and x < (bx - ax) * (y - ay) / (by - ay) + ax:
                    odd = not odd
    return odd


def count_of(v, dot_value):
    if v is None or not v > 0 or not dot_value > 0:
        return 0
    return int(math.floor(v / dot_value + 0.5))


def dots(parts, count, state):
    pts = [p for rings in parts for ring in rings if len(ring) >= 3 for p in ring]
    x0, y0 = min(p[0] for p in pts), min(p[1] for p in pts)
    x1, y1 = max(p[0] for p in pts), max(p[1] for p in pts)
    w, h = x1 - x0, y1 - y0
    rng = SplitMix64(state)
    out = []
    for _ in range(50 * count + 1000):
        if len(out) == count:
            break
        x = x0 + rng.unit() * w
        y = y0 + rng.unit() * h
        if inside(parts, x, y):
            out.append([x, y])
    return out


# ── Charts ──────────────────────────────────────────────────────────────

ARC = 5.0 * math.pi / 180.0


def bearing_at(c, r, b):
    return [c[0] + r * math.sin(b), c[1] + r * math.cos(b)]


def pie(c, d, values):
    total = sum(v for v in values if v > 0)
    if not total > 0 or not d > 0:
        return []
    r = d / 2.0
    shown = sum(1 for v in values if v > 0)
    out, start = [], 0.0
    for field, v in enumerate(values):
        if not v > 0:
            continue
        sweep = 2.0 * math.pi * v / total
        if shown == 1:
            n = math.ceil(2.0 * math.pi / ARC)
            out.append({"field": field, "ring": [bearing_at(c, r, 2.0 * math.pi * k / n) for k in range(n)]})
            break
        n = max(1, math.ceil(sweep / ARC))
        ring = [list(c)] + [bearing_at(c, r, start + sweep * k / n) for k in range(n + 1)]
        out.append({"field": field, "ring": ring})
        start += sweep
    return out


def bars(kind, c, h, w, max_value, values):
    if not max_value > 0 or not w > 0:
        return []
    rect = lambda x0, x1, ya, yb: [[x0, ya], [x1, ya], [x1, yb], [x0, yb]]
    out = []
    if kind == "stacked":
        y = c[1]
        for field, v in enumerate(values):
            if not v > 0:
                continue
            top = y + h * v / max_value
            out.append({"field": field, "ring": rect(c[0] - w / 2, c[0] + w / 2, y, top)})
            y = top
    else:
        left = c[0] - w * len(values) / 2.0
        for field, v in enumerate(values):
            if v == 0 or not math.isfinite(v):
                continue
            x0 = left + w * field
            top = c[1] + h * v / max_value
            ya, yb = (c[1], top) if top >= c[1] else (top, c[1])
            out.append({"field": field, "ring": rect(x0, x0 + w, ya, yb)})
    return out


# ── Groups ──────────────────────────────────────────────────────────────


def group(points, d):
    groups = []  # [members, sx, sy]
    grid = {}
    cell = lambda x, y: (math.floor(x / d), math.floor(y / d))
    centre = lambda g: (g[1] / len(g[0]), g[2] / len(g[0]))
    for i, (x, y) in enumerate(points):
        cx, cy = cell(x, y)
        best = None
        for gx in (cx - 1, cx, cx + 1):
            for gy in (cy - 1, cy, cy + 1):
                for g in grid.get((gx, gy), []):
                    gx_, gy_ = centre(groups[g])
                    dist = (gx_ - x) * (gx_ - x) + (gy_ - y) * (gy_ - y)
                    if dist <= d * d and (best is None or dist < best[0] or (dist == best[0] and g < best[1])):
                        best = (dist, g)
        if best is None:
            groups.append([[i], x, y])
            grid.setdefault((cx, cy), []).append(len(groups) - 1)
        else:
            g = best[1]
            before = cell(*centre(groups[g]))
            groups[g][0].append(i)
            groups[g][1] += x
            groups[g][2] += y
            after = cell(*centre(groups[g]))
            if after != before:
                grid[before].remove(g)
                grid.setdefault(after, []).append(g)
    return [{"members": g[0], "centre": list(centre(g))} for g in groups]


def displaced(placement, n, s, c, spacing):
    at = lambda r, a: [r * math.sin(a), r * math.cos(a)]
    out, rings = [], []
    if placement == "ring":
        r = max(s / 2.0, n * s / (2.0 * math.pi)) + spacing
        out = [at(r, 2.0 * math.pi * k / n) for k in range(n)]
        rings = [r]
    elif placement == "rings":
        r, left = c / 2.0 + s / 2.0 + spacing, n
        while left > 0:
            fits = min(max(1, math.floor(2.0 * math.pi * r / s)), left)
            out += [at(r, 2.0 * math.pi * k / fits) for k in range(fits)]
            rings.append(r)
            left -= fits
            r += s + spacing
    else:
        cols = max(1, math.ceil(math.sqrt(n)))
        rows = -(-n // cols)
        a = (c / 2.0 + s / 2.0 + s) / 2.0 + spacing
        w, h = (cols - 1) * a, (rows - 1) * a
        out = [[(k % cols) * a - w / 2.0, h / 2.0 - (k // cols) * a] for k in range(n)]
    return out, rings


# ── Heat ────────────────────────────────────────────────────────────────


def heat(case):
    x0, y0, x1, y1 = case["box"]
    ppm, q = case["pxPerM"], case["quality"]
    w, h = x1 - x0, y1 - y0
    cell = max(max(q / ppm, w / 4096.0), max(h / 4096.0, math.sqrt(w * h / 4194304.0)))
    width, height = max(1, math.ceil(w / cell)), max(1, math.ceil(h / cell))
    r = max(1, case["radius"])
    k = {}
    for dj in range(-r, r + 1):
        for di in range(-r, r + 1):
            d2 = di * di + dj * dj
            if d2 <= r * r:
                t = 1.0 - d2 / (r * r)
                k[(di, dj)] = t * t
    values = [0.0] * (width * height)
    for px, py, wt in case["points"]:
        if not wt > 0:
            continue
        i = math.floor((px - x0) / cell)
        j = math.floor((y1 - py) / cell)
        for dj in range(-r, r + 1):
            row = j + dj
            if row < 0 or row >= height:
                continue
            for di in range(-r, r + 1):
                col = i + di
                if col < 0 or col >= width:
                    continue
                values[row * width + col] += wt * k.get((di, dj), 0.0)
    top = case.get("max") or max(values + [0.0])
    table = []
    for s in range(1024):
        c = ramp_color(case["ramp"], s / 1023)
        table.append(c[:3] + [round_half_up(c[3] * min(1.0, max(0.0, case["opacity"])))])
    pixels = []
    for v in values:
        if not v > 0 or not top > 0:
            pixels += [0, 0, 0, 0]
        else:
            pixels += table[round_half_up(1023 * min(v / top, 1.0))]
    return {"grid": {"width": width, "height": height, "cell": cell}, "values": values, "top": top, "pixels": pixels}


# ── Inverted ────────────────────────────────────────────────────────────


def region_area(frame, areas, rule):
    b = shapely_box(*frame)
    polys = [Polygon(a[0], a[1:]) for a in areas]
    if rule == "nonZero":
        covered = unary_union(polys) if polys else Polygon()
        return b.difference(covered).area
    odd = Polygon()
    for p in polys:
        odd = odd.symmetric_difference(p)
    return b.difference(odd).area


# ── Cases ───────────────────────────────────────────────────────────────

HEAT_RAMP = ["#2B83BA00", "#2B83BA", "#ABDDA4", "#FFFFBF", "#FDAE61", "#D7191C"]
YELLOW_RED = ["#FFF5B8", "#FDB863", "#E66101", "#A50F15"]


def cases():
    out = {
        "format": "kentos.renderers",
        "version": 1,
        "note": "Ek işleyicilerin kuralları (docs/adr/0213 §2), scripts/fixtures/renderer_cases.py'nin ADR'den, KentOS kodu olmadan "
        "yazdığı başvuru. Renkler #RRGGBB ya da #RRGGBBAA; noktalar ve gruplar bit bit, grafikler ve yaymalar 1e-9'a, ısı değerleri "
        "1e-12'ye (göreli), pikseller ±1'e, ters alanın alanı shapely'nin 1e-9'una.",
    }
    out["ramps"] = [
        {"ramp": r, "t": t, "color": hexa(ramp_color(r, t))}
        for r in (YELLOW_RED, ["#2B83BA00", "#2B83BA"], ["#123456"])
        for t in (0.0, 0.1, 1 / 3, 0.5, 0.75, 1.0, -0.2, 1.4)
    ]
    out["unclassed"] = []
    for lo, hi in ((0.0, 100.0), (50.0, 50.0), (-10.0, 30.0)):
        for v in (-5.0, 0.0, 12.3, 50.0, 99.9, 100.0, 250.0):
            k = step(share(v, lo, hi))
            out["unclassed"].append(
                {"min": lo, "max": hi, "ramp": YELLOW_RED, "value": v, "step": k, "color": hexa(ramp_color(YELLOW_RED, k / 255.0))}
            )
    out["sizes"] = []
    for scaling in ("area", "radius", "flannery"):
        for v in (5.0, 10.0, 100.0, 505.0, 1000.0, 2000.0):
            k = step(share(v, 10.0, 1000.0))
            out["sizes"].append(
                {
                    "minValue": 10.0,
                    "maxValue": 1000.0,
                    "minSize": 2.0,
                    "maxSize": 12.0,
                    "scaling": scaling,
                    "value": v,
                    "step": k,
                    "size": size_at(k / 255.0, 2.0, 12.0, EXPONENT[scaling]),
                }
            )
    out["classes"] = [
        {"breaks": br, "value": v, "class": class_of(v, br)}
        for br in ([10.0, 20.0], [1.0, 2.0, 3.0], [0.5])
        for v in (-1.0, 0.5, 1.0, 5.0, 10.0, 15.0, 20.0, 25.0)
    ]

    square = [[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]]
    holed = [
        [[487000.0, 4420000.0], [487060.0, 4420000.0], [487060.0, 4420040.0], [487030.0, 4420040.0],
         [487030.0, 4420020.0], [487000.0, 4420020.0]],
        [[487040.0, 4420005.0], [487050.0, 4420005.0], [487050.0, 4420015.0], [487040.0, 4420015.0]],
    ]
    two_parts = [square, [[[200.0, 0.0], [220.0, 0.0], [210.0, 30.0]]]]
    out["dots"] = []
    for parts, seed, field, dot_value, value in (
        ([square], 0, 0, 10.0, 125.0),
        ([square], 7, 1, 10.0, 125.0),
        ([holed], 3, 0, 2.5, 101.25),
        (two_parts, 11, 2, 1.0, 40.0),
        ([square], 1, 0, 10.0, -4.0),
    ):
        n = count_of(value, dot_value)
        out["dots"].append(
            {
                "parts": parts,
                "seed": seed,
                "field": field,
                "dotValue": dot_value,
                "value": value,
                "count": n,
                "dots": dots(parts, n, seed_of(seed, rings_hash(parts), field)) if n else [],
            }
        )

    out["charts"] = [
        {"kind": "pie", "place": [10.0, 20.0], "size": 4.0, "values": [1.0, 2.0, 3.0], "pieces": pie([10.0, 20.0], 4.0, [1.0, 2.0, 3.0])},
        {"kind": "pie", "place": [0.0, 0.0], "size": 2.0, "values": [0.0, 5.0, -1.0], "pieces": pie([0.0, 0.0], 2.0, [0.0, 5.0, -1.0])},
        {"kind": "pie", "place": [0.0, 0.0], "size": 2.0, "values": [0.0, 0.0], "pieces": []},
        {"kind": "pie", "place": [1.5, -2.0], "size": 10.0, "values": [1.0, 1.0, 1.0, 97.0],
         "pieces": pie([1.5, -2.0], 10.0, [1.0, 1.0, 1.0, 97.0])},
    ]
    for kind, values in (("bar", [3.0, -1.0, 2.0]), ("stacked", [1.0, 2.0, 0.5])):
        out["charts"].append(
            {"kind": kind, "place": [5.0, 5.0], "size": 5.0, "width": 1.0, "maxValue": 4.0, "values": values,
             "pieces": bars(kind, [5.0, 5.0], 5.0, 1.0, 4.0, values)}
        )

    pts = [[0.0, 0.0], [1.0, 0.0], [10.0, 0.0], [3.0, 0.0], [11.0, 1.0], [100.0, 100.0], [4.9, 0.0], [5.5, 0.0], [9.0, 0.5],
           [-4.0, -3.0], [100.0, 100.0], [100.0, 104.9]]
    out["groups"] = [
        {"points": pts, "distance": d, "groups": group(pts, d)} for d in (5.0, 0.5, 50.0)
    ] + [{"points": [[487000.0 + (i % 7) * 3.1, 4420000.0 + (i // 7) * 2.3] for i in range(49)], "distance": 4.0,
          "groups": group([[487000.0 + (i % 7) * 3.1, 4420000.0 + (i // 7) * 2.3] for i in range(49)], 4.0)}]

    out["displaced"] = []
    for placement, n, s, c, spacing in (("ring", 5, 10.0, 0.0, 0.0), ("ring", 2, 8.0, 0.0, 3.0), ("rings", 20, 8.0, 6.0, 0.0),
                                        ("rings", 3, 12.0, 0.0, 1.0), ("grid", 7, 10.0, 0.0, 2.0), ("grid", 4, 6.0, 8.0, 0.0)):
        offs, rings = displaced(placement, n, s, c, spacing)
        out["displaced"].append({"placement": placement, "n": n, "s": s, "c": c, "spacing": spacing, "offsets": offs, "rings": rings})

    heat_cases = [
        {"box": [0.0, 0.0, 40.0, 30.0], "pxPerM": 1.0, "quality": 2, "radius": 4,
         "points": [[5.0, 5.0, 1.0], [5.2, 5.9, 2.0], [30.0, 20.0, 1.0], [-3.0, 10.0, 1.0], [100.0, 100.0, 1.0], [12.0, 29.9, 0.0]],
         "ramp": HEAT_RAMP, "opacity": 0.8},
        {"box": [487000.0, 4420000.0, 487020.0, 4420012.0], "pxPerM": 2.5, "quality": 1, "radius": 6,
         "points": [[487004.0, 4420004.0, 1.0], [487004.4, 4420004.4, 1.0], [487015.0, 4420009.0, 3.0]],
         "max": 2.0, "ramp": HEAT_RAMP, "opacity": 1.0},
    ]
    out["heat"] = []
    for case in heat_cases:
        got = heat(case)
        out["heat"].append({**case, **got})

    sq = lambda x0, y0, x1, y1: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    inv = [
        ([0.0, 0.0, 100.0, 100.0], [[sq(10.0, 10.0, 40.0, 40.0)]]),
        ([0.0, 0.0, 100.0, 100.0], [[sq(10.0, 10.0, 40.0, 40.0)], [sq(30.0, 30.0, 60.0, 60.0)]]),
        ([0.0, 0.0, 100.0, 100.0], [[sq(20.0, 20.0, 80.0, 80.0), sq(40.0, 40.0, 60.0, 60.0)]]),
        ([0.0, 0.0, 100.0, 100.0], [[[[-20.0, 50.0], [50.0, -10.0], [130.0, 120.0]]]]),
        ([0.0, 0.0, 100.0, 100.0], [[[[10.0, 10.0], [90.0, 20.0], [20.0, 90.0], [50.0, 50.0]]], [sq(60.0, 60.0, 70.0, 95.0)]]),
        ([0.0, 0.0, 100.0, 100.0], []),
        ([487000.0, 4420000.0, 487100.0, 4420100.0],
         [[sq(487000.0 + 10 * i + 1, 4420000.0 + 10 * j + 1, 487000.0 + 10 * i + 9, 4420000.0 + 10 * j + 9)] for i in range(10) for j in range(10)]),
    ]
    out["inverted"] = []
    for frame, areas in inv:
        for rule in ("evenOdd", "nonZero"):
            out["inverted"].append({"box": frame, "areas": areas, "rule": rule, "area": region_area(frame, areas, rule)})
    return out


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)}: güncel değil; betiği --check olmadan çalıştırın.")
            sys.exit(1)
        print(f"İşleyicilerin durumları tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı.")


if __name__ == "__main__":
    main()
