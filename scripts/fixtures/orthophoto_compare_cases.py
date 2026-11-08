#!/usr/bin/env python3
"""The shared cases of `cad.orthophoto.compare` (Ortofoto karşılaştırması; plan/map-cad.md MAP-12, docs/adr/1780),
without KentOS code.

    python3 scripts/fixtures/orthophoto_compare_cases.py           # writes fixtures/commands/v1/cad.orthophoto.compare.json
    python3 scripts/fixtures/orthophoto_compare_cases.py --check   # writes nothing; compares

The rules, by hand (BÖHHBÜY, RG 26.06.2018/30460 Mükerrer, Article 67(6): the misalignment between a mosaic's
orthophotos is at most ±2·GSD; Articles 54, 91(2) for each image's own check, as in orthophoto_check_cases.py):
- the points read on the first image and on the second matched by name (the first of a name on each side); a point's
  shift is the second's place − the first's, its length math.hypot;
- the shift's statistics as the check's: RMSE in X and Y, sqrt(Σ(dx² + dy²) / 2n), the radial RMSE, the means (the
  systematic shift), the largest; no heights;
- a point's limit is the package's `mosaic` × the larger of the two ground sampling distances (the square root of a
  pixel's area); a point over it is flagged, `passed` is every point within it (none without points);
- with the ground's points each image is checked on its own, the check's rules (fit points from its georeference).

The seam (docs/adr/2460): three synthetic 8-bit GeoTIFFs written here byte by byte (uncompressed, one strip), the
scene an analytic texture f(x, y), a sum of sines, sampled at the pixels' centres; the second image shows the scene
displaced by a known shift s (B(x, y) = f(x − sx, y − sy)), so the place P of the first is found at P + s. A flat
patch in the first and a changed patch in the second. The measurement is the ADR's, written here again: places every
step metres inside the overlap by half the window and the search, the first image's patch sought in the second by
zero-mean normalized cross-correlation (sums in row order, as written), the peak refined by a parabola. Each ok
place's shift is asserted within 0.05 pixel of s: the known shift is the independent truth.
"""

import copy
import json
import math
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.orthophoto.compare.json"
RULES = json.loads((ROOT / "catalog/v1/rules/ortofoto-bohhbuy-2018.json").read_text("utf-8"))
X0, Y0 = 487000.0, 4420000.0


def uid(n):
    return f"0192f5c0-1780-7000-8000-{n:012x}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


GEOREF = {"method": "thinPlate",
          "points": [{"name": "Y1", "pixel": [0.0, 0.0], "target": [X0 + 10, Y0 + 290]},
                     {"name": "Y2", "pixel": [2900.0, 0.0], "target": [X0 + 290, Y0 + 290]},
                     {"name": "Y3", "pixel": [2900.0, 2900.0], "target": [X0 + 290, Y0 + 10]}],
          "source": {"width": 3000, "height": 3000, "asset": "raster-1be365b685ba77d1",
                     "digest": "1be365b685ba77d1d05f8c076a61e8f65572a688a0e6e8a78b563499da8b289b"},
          "output": {"affine": [X0, 0.1, 0.0, Y0 + 300, 0.0, -0.1], "width": 3000, "height": 3000, "pixel": 0.1,
                     "resampling": "nearest"}}


def raster(file, pixel, srid=5254, georef=False):
    r = {"kind": "raster", "layerId": "altlik", "attrs": {}, "affine": [X0, pixel, 0.0, Y0 + 300, 0.0, -pixel],
         "width": 3000, "height": 3000, "bands": 3, "sample": "u8", "srid": srid,
         "style": {"render": "rgb", "bands": [1, 2, 3]}, "file": file}
    if georef:
        r["georef"] = copy.deepcopy(GEOREF)
        # The fit's output grid is the raster's (a .kcad rule).
        r["georef"]["output"].update(affine=list(r["affine"]), pixel=pixel)
    return r


def first_raster(**kw):
    return raster("ortofoto-2024.tif", kw.pop("pixel", 0.10), georef=kw.pop("georef", True), **kw)


def second_raster(**kw):
    return raster("ortofoto-2026.tif", kw.pop("pixel", 0.08), **kw)


# Name, ground (x, y, z), the first image's reading error (dx, dy, dz), the second's (dx, dy) in metres. The first
# image (2024, 10 cm) fitted to Y1–Y3; the second (2026, 8 cm) read without heights, about 6 cm east and 4 cm north.
POINTS = [
    ("D1", (20.0, 30.0, 851.20), (0.064, -0.048, 0.06), (0.090, 0.010)),
    ("D2", (80.0, 40.0, 851.90), (-0.080, 0.032, -0.08), (0.020, 0.070)),
    ("D3", (150.0, 60.0, 852.40), (0.048, 0.096, 0.10), (0.100, 0.020)),
    ("D4", (220.0, 90.0, None), (-0.032, -0.112, None), (0.050, 0.030)),
    ("D5", (260.0, 150.0, 853.00), (0.128, 0.016, 0.05), (0.080, 0.060)),
    ("D6", (200.0, 220.0, 853.60), (-0.096, -0.064, -0.12), (0.030, 0.000)),
    ("D7", (120.0, 250.0, 852.80), (0.016, 0.080, 0.04), (0.070, 0.080)),
    ("D8", (40.0, 200.0, 852.10), (0.080, -0.080, 0.09), (0.040, 0.050)),
    ("Y1", (10.0, 290.0, 852.00), (0.004, -0.003, 0.01), (0.060, 0.030)),
    ("Y2", (290.0, 290.0, 854.00), (-0.002, 0.005, None), (0.050, 0.050)),
]


def point(n, layer_, x, y, z, name, field="Ad"):
    e = {"kind": "point", "id": n, "layerId": layer_, "attrs": {field: name}, "p": {"x": X0 + x, "y": Y0 + y}}
    if z is not None:
        e["z"] = z
    return e


def setup(points=POINTS, first=None, second=None, field="Ad", extra=()):
    ents = [dict(first or first_raster(), id=1), dict(second or second_raster(), id=2)]
    n = 3
    for name, (x, y, z), _, _ in points:
        ents.append(point(n, "gnss", x, y, z, name, field))
        n += 1
    for name, (x, y, z), (dx, dy, dz), _ in points:
        ents.append(point(n, "okuma-2024", x + dx, y + dy, None if z is None or dz is None else z + dz, name, field))
        n += 1
    for name, (x, y, _), _, (dx, dy) in points:
        ents.append(point(n, "okuma-2026", x + dx, y + dy, None, name, field))
        n += 1
    for e in extra:
        ents.append(dict(e, id=n))
        n += 1
    return {"format": "kentos.document", "version": 2, "name": "Ortofoto karşılaştırması",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("altlik", "Altlık"), layer("gnss", "GNSS denetim noktaları"),
                       layer("okuma-2024", "2024 ortofotosunda okunan"),
                       layer("okuma-2026", "2026 ortofotosunda okunan")],
            "activeLayer": "okuma-2026", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


def named(s, layer_, field):
    out = []
    for e in s["entities"]:
        if e["layerId"] != layer_ or e["kind"] != "point":
            continue
        name = e["attrs"].get(field, "").strip()
        if name and name not in [o[0] for o in out]:
            out.append((name, e["p"]["x"], e["p"]["y"], e.get("z")))
    return out


def stats(ps):
    n = len(ps)
    if n == 0:
        return {"n": 0, "rmseX": 0.0, "rmseY": 0.0, "kohXy": 0.0, "rmseR": 0.0, "meanX": 0.0, "meanY": 0.0,
                "maxD": 0.0, "nZ": 0}
    sx, sy = sum(p["dx"] ** 2 for p in ps), sum(p["dy"] ** 2 for p in ps)
    zs = [p["dz"] for p in ps if "dz" in p]
    out = {"n": n, "rmseX": math.sqrt(sx / n), "rmseY": math.sqrt(sy / n), "kohXy": math.sqrt((sx + sy) / (2 * n)),
           "rmseR": math.sqrt((sx + sy) / n), "meanX": sum(p["dx"] for p in ps) / n,
           "meanY": sum(p["dy"] for p in ps) / n, "maxD": max(p["d"] for p in ps), "nZ": len(zs)}
    if zs:
        out["kohZ"] = math.sqrt(sum(z * z for z in zs) / len(zs))
    return out


def gsd(ras):
    af = ras["affine"]
    return math.sqrt(abs(af[1] * af[5] - af[2] * af[4]))


def check(s, ras, known, observed, field, scale, rules):
    """One image against the ground's points: the check's rules (orthophoto_check_cases.py)."""
    scale = scale or round(s["settings"]["plotScale"])
    k, o = named(s, known, field), named(s, observed, field)
    fitted = [p["name"].strip() for p in (ras.get("georef") or {}).get("points", []) if not p.get("check") and
              p.get("name")]
    points, unmatched = [], []
    for name, x, y, z in k:
        m = [q for q in o if q[0] == name]
        if not m:
            unmatched.append(name)
            continue
        _, ox, oy, oz = m[0]
        p = {"name": name, "role": "fit" if name in fitted else "check", "dx": ox - x, "dy": oy - y}
        if oz is not None and z is not None:
            p["dz"] = oz - z
        p["d"] = math.hypot(p["dx"], p["dy"])
        points.append(p)
    for name, *_ in o:
        if name not in [q[0] for q in k]:
            unmatched.append(name)
    chk = stats([p for p in points if p["role"] == "check"])
    fits = [p for p in points if p["role"] == "fit"]
    S = float(scale)
    a, b, c = rules["xy"]
    limit_xy = (a * S * S + b * S + c) / 100.0
    limit_z = rules["zFactor"] * limit_xy
    g = gsd(ras)
    gl = [x["gsd"] for x in rules["gsd"] if x["scale"] == scale]
    notes = [f"KOH_X,Y = √(Σ(dx² + dy²) / 2n): X ve Y'nin ortak karesel ortalama hatası; sınırı {rules['id']} (S = {scale}).",
             "Uyum noktaları (görüntünün oturtulduğu kontrol noktaları) doğruluk göstermez; yalnız bağımsız denetim "
             "noktaları sayılır.",
             "Denetim noktalarının sayısı ve dağılımı (BÖHHBÜY m. 91(1): detay noktalarının en az %5'i) bu denetimde "
             "bilinmez; elle gözden geçirin."]
    if not gl:
        notes.append(f"Kural paketi 1/{scale} ölçeği için yer örnekleme aralığı vermiyor.")
    if unmatched:
        notes.append(f"Eşi bulunmayan noktalar: {', '.join(unmatched)}.")
    out = {"rules": rules["id"], "source": rules["source"], "scale": scale, "gsd": g}
    if gl:
        out["gsdLimit"] = gl[0]
        out["gsdOk"] = g <= gl[0] * (1 + rules["gsdTolerance"]) + 1e-12
    out.update({"points": points, "unmatched": unmatched, "check": chk})
    if fits:
        out["fit"] = stats(fits)
    out.update({"limitXy": limit_xy, "limitZ": limit_z})
    if chk["n"] > 0:
        out["passedXy"] = chk["kohXy"] <= limit_xy
    if "kohZ" in chk:
        out["passedZ"] = chk["kohZ"] <= limit_z
    out["belowPixel"] = chk["n"] > 0 and chk["kohXy"] < g / 2
    out["notes"] = notes
    return out


def num(v):
    """A number as Rust's `{}` writes it: 2 for 2.0."""
    t = repr(float(v))
    return t[:-2] if t.endswith(".0") else t


def compare(s, first, second, first_points, second_points, known=None, field="Ad", scale=None, rules=None):
    by_uid = dict(zip(s["uids"], s["entities"]))
    r1, r2 = by_uid[first.strip()], by_uid[second.strip()]
    rules = rules or RULES
    g1, g2 = gsd(r1), gsd(r2)
    limit = rules["mosaic"] * max(g1, g2)
    a, b = named(s, first_points, field), named(s, second_points, field)
    pairs, shifts, unmatched = [], [], []
    for name, x, y, _ in a:
        m = [q for q in b if q[0] == name]
        if not m:
            unmatched.append(name)
            continue
        _, bx, by, _ = m[0]
        dx, dy = bx - x, by - y
        d = math.hypot(dx, dy)
        pairs.append({"name": name, "dx": dx, "dy": dy, "d": d, "over": d > limit})
        shifts.append({"name": name, "role": "check", "dx": dx, "dy": dy, "d": d})
    for name, *_ in b:
        if name not in [q[0] for q in a]:
            unmatched.append(name)
    out = {"rules": rules["id"], "source": rules["source"], "gsd": [g1, g2], "limit": limit, "pairs": pairs,
           "unmatched": unmatched, "shift": stats(shifts)}
    if pairs:
        out["passed"] = all(not p["over"] for p in pairs)
    if known is not None:
        out["firstCheck"] = check(s, r1, known, first_points, field, scale, rules)
        out["secondCheck"] = check(s, r2, known, second_points, field, scale, rules)
    notes = [f"Kayma = ikinci − birinci görüntüde okunan yer; nokta başına sınır {num(rules['mosaic'])}·YÖA = "
             f"{limit:.3f} m ({rules['id']}), iki görüntüden YÖA'sı iri olanınki (KentOS'un okuması).",
             "Ortalama kayma görüntüler arasındaki sistematik farktır; iki zamanın farkı kendiliğinden deformasyon ya "
             "da yer değiştirme değildir, görüntülerin kendi doğruluğuyla birlikte okunur."]
    if unmatched:
        notes.append(f"Eşi bulunmayan noktalar: {', '.join(unmatched)}.")
    if known is None:
        notes.append("Arazi noktaları verilmedi: görüntülerin kendi doğruluğu denetlenmedi.")
    out["notes"] = notes
    return out


SAME = {"revision": "same", "canUndo": False}
BASE = setup()
DEFAULT = {"first": uid(1), "second": uid(2), "firstPoints": "okuma-2024", "secondPoints": "okuma-2026"}


def ok(name, s=BASE, op="plan", **kw):
    inp = dict(DEFAULT)
    inp.update(kw)
    out = compare(s, inp["first"], inp["second"], inp["firstPoints"], inp["secondPoints"], inp.get("known"),
                  inp.get("field", "Ad"), inp.get("scale"), inp.get("rules"))
    case = {"name": name, "steps": [{"op": op, "input": inp, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}
    if s is not BASE:
        case["setup"] = s
    return case


def refused(name, inp, code, message, path_, s=None):
    case = {"name": name, "steps": [{"op": "execute", "input": dict(DEFAULT, **inp),
                                     "result": {"status": "failed", "error": {"code": code, "message": message,
                                                                              "path": path_}}, "expect": SAME}]}
    if s is not None:
        case["setup"] = s
    return case


# ── The seam (docs/adr/2460) ──────────────────────────────────────────────────────────────────────────────────────
FILES = ROOT / "fixtures" / "orthophoto" / "v1" / "files"
PIX = 0.1
W_A, H_A = 200, 120
A0 = (X0 + 500.0, Y0 + 120.0)         # the first image's top-left corner
B0 = (X0 + 510.0, Y0 + 120.0)         # the second's: 10 m east, a 10 m wide overlap
S_B = (0.034, -0.021)                 # the second's scene shift (0.34, −0.21 pixel)
S_C = (0.27, 0.0)                     # the third's (2.7 pixels east)
FLAT = (511.0, 514.0, 113.0, 117.0)   # a flat patch in the first (local x0, x1, y0, y1)
CHANGED = (515.0, 518.0, 110.0, 113.0)  # a changed patch in the second
WAVES = [(0.61, 0.17, 0.3, 40.0), (-0.23, 0.83, 1.1, 30.0), (1.37, -0.59, 2.3, 22.0), (0.41, 1.21, 0.7, 18.0),
         (1.93, 0.71, 1.9, 12.0)]


def scene(x, y):
    """The analytic texture at local (x, y) in metres."""
    return sum(a * math.sin(2 * math.pi * (u * x + v * y) + phi) for u, v, phi, a in WAVES)


def changed(x, y):
    return 60.0 * math.sin(2 * math.pi * (3.1 * x - 2.7 * y)) + 40.0 * math.cos(2 * math.pi * (2.3 * y))


def image(origin, shift, flat=None, change=None):
    """Rows of 8-bit values: each pixel's centre (local), the scene at it less the shift, 128 + value, rounded."""
    rows = []
    for j in range(H_A):
        row = []
        for i in range(W_A):
            x = origin[0] - X0 + (i + 0.5) * PIX
            y = origin[1] - Y0 - (j + 0.5) * PIX
            if flat and flat[0] <= x <= flat[1] and flat[2] <= y <= flat[3]:
                v = 128.0
            elif change and change[0] <= x <= change[1] and change[2] <= y <= change[3]:
                v = 128.0 + changed(x, y)
            else:
                v = 128.0 + scene(x - shift[0], y - shift[1])
            row.append(max(0, min(255, int(math.floor(v + 0.5)))))
        rows.append(row)
    return rows


def geotiff(rows, origin, srid=5254):
    """A baseline little-endian TIFF: 8-bit grey, one strip, uncompressed; GeoTIFF's pixel scale, tie point (the
    top-left corner) and keys (projected, pixel is area, the EPSG code)."""
    h, w = len(rows), len(rows[0])
    data = bytes(v for row in rows for v in row)
    keys = [1, 1, 0, 3, 1024, 0, 1, 1, 1025, 0, 1, 1, 3072, 0, 1, srid]
    scale = struct.pack("<3d", PIX, PIX, 0.0)
    tie = struct.pack("<6d", 0.0, 0.0, 0.0, origin[0], origin[1], 0.0)
    geokeys = struct.pack(f"<{len(keys)}H", *keys)
    entries = [(256, 4, 1, w), (257, 4, 1, h), (258, 3, 1, 8), (259, 3, 1, 1), (262, 3, 1, 1), (273, 4, 1, None),
               (277, 3, 1, 1), (278, 4, 1, h), (279, 4, 1, len(data)), (284, 3, 1, 1),
               (33550, 12, 3, scale), (33922, 12, 6, tie), (34735, 3, len(keys), geokeys)]
    ifd_at = 8
    ifd_size = 2 + 12 * len(entries) + 4
    extra_at = ifd_at + ifd_size
    blobs = b""
    packed = []
    for tag, kind, count, value in entries:
        if isinstance(value, bytes):
            packed.append((tag, kind, count, struct.pack("<I", extra_at + len(blobs))))
            blobs += value
        else:
            packed.append((tag, kind, count, value))
    data_at = extra_at + len(blobs)
    out = b"II*\x00" + struct.pack("<I", ifd_at) + struct.pack("<H", len(packed))
    for tag, kind, count, value in packed:
        if tag == 273:
            value = data_at
        field = value if isinstance(value, bytes) else (struct.pack("<HH", value, 0) if kind == 3 else
                                                        struct.pack("<I", value))
        out += struct.pack("<HHI", tag, kind, count) + field
    out += struct.pack("<I", 0) + blobs + data
    return out


IMAGES = {"dikis-a.tif": (A0, (0.0, 0.0), FLAT, None), "dikis-b.tif": (B0, S_B, None, CHANGED),
          "dikis-c.tif": (B0, S_C, None, None)}


def seam_raster(name, origin, pixel=PIX):
    return {"kind": "raster", "layerId": "altlik", "attrs": {}, "affine": [origin[0], pixel, 0.0, origin[1], 0.0, -pixel],
            "width": W_A, "height": H_A, "bands": 1, "sample": "u8", "srid": 5254,
            "style": {"render": "gray", "bands": [1]}, "file": f"$fixture:orthophoto/v1/files/{name}"}


def seam_setup(second="dikis-b.tif", second_origin=B0, pixel=PIX, extra=()):
    return setup(points=[], first=seam_raster("dikis-a.tif", A0), second=seam_raster(second, second_origin, pixel),
                 extra=extra)


# docs/adr/2860: a seam line drawn across the overlap, from north of it to south of it with a bend: its places every
# step along it, those inside the overlap (with the window's and the search's margin) measured.
SEAM_LINE = [(515.0, 121.5), (516.2, 114.0), (514.4, 106.5)]


def polyline(pts, bulges=None):
    e = {"kind": "polyline", "layerId": "0", "attrs": {}, "pts": [{"x": X0 + x, "y": Y0 + y} for x, y in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def along(pts, s):
    """The point at distance s along pts (edges of no length skipped; past the end, the end)."""
    walked = 0.0
    for (ax, ay), (bx, by) in zip(pts, pts[1:]):
        length = math.hypot(bx - ax, by - ay)
        if length == 0.0:
            continue
        if s <= walked + length:
            t = (s - walked) / length
            return (ax + t * (bx - ax), ay + t * (by - ay))
        walked += length
    return pts[-1]


def ncc(a, b):
    n = len(a)
    ma, mb = sum(a) / n, sum(b) / n
    sab = saa = sbb = 0.0
    for x, y in zip(a, b):
        p, q = x - ma, y - mb
        sab += p * q
        saa += p * p
        sbb += q * q
    return sab / math.sqrt(saa * sbb) if saa > 0 and sbb > 0 else None


def vertex(l, c, r):
    den = l - 2 * c + r
    return (l - r) / (2 * den) if den < 0 else 0.0


def seam(pixels, ask, limit, truth, line=None):
    """The overlap measured as docs/adr/2460 says; `pixels` the two images' rows, `truth` the second's known shift;
    `line` (docs/adr/2860): (uid, world vertices) of a seam line measured along instead of the grid."""
    (ra, oa), (rb, ob) = pixels
    w, r, step = ask["window"], ask["search"], ask["step"]
    half = w // 2
    reach = half + r + 1
    ox0, ox1 = max(oa[0], ob[0]) + reach * PIX, min(oa[0], ob[0]) + W_A * PIX - reach * PIX
    ox1 = min(oa[0] + W_A * PIX, ob[0] + W_A * PIX) - reach * PIX
    oy0 = max(oa[1] - H_A * PIX, ob[1] - H_A * PIX) + reach * PIX
    oy1 = min(oa[1], ob[1]) - reach * PIX
    patches = []
    places, line_out = [], None
    if line is not None:
        uid_, pts = line
        length = sum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:]))
        total = outside = 0
        k = 0
        while True:
            st = k * step
            if st > length:
                break
            px_, py_ = along(pts, st)
            total += 1
            if ox0 <= px_ <= ox1 and oy0 <= py_ <= oy1:
                places.append(((px_, py_), st))
            else:
                outside += 1
            k += 1
        line_out = {"uid": uid_, "length": length, "places": total, "outside": outside}
    else:
        y = oy1
        while y >= oy0:
            x = ox0
            while x <= ox1:
                places.append(((x, y), None))
                x += step
            y -= step
    for (x, y), station in places:
        if True:
            ci, cj = math.floor((x - oa[0]) / PIX), math.floor((y - oa[1]) / -PIX)
            at = [oa[0] + (ci + 0.5) * PIX, oa[1] + (cj + 0.5) * -PIX]
            bi = math.floor((at[0] - ob[0]) / PIX - 0.5 + 0.5)
            bj = math.floor((at[1] - ob[1]) / -PIX - 0.5 + 0.5)
            first = [float(ra[cj - half + j][ci - half + i]) for j in range(w) for i in range(w)]
            scores = {}
            for v in range(-r, r + 1):
                for u in range(-r, r + 1):
                    second = [float(rb[bj + v - half + j][bi + u - half + i]) for j in range(w) for i in range(w)]
                    scores[(u, v)] = ncc(first, second)
            best = None
            for v in range(-r, r + 1):
                for u in range(-r, r + 1):
                    c = scores[(u, v)]
                    if c is not None and (best is None or c > best[2]):
                        best = (u, v, c)
            here = {} if station is None else {"station": station}
            if best is None:
                patches.append({"at": at, "state": "flat", "over": False, **here})
            else:
                u, v, c = best
                if abs(u) == r or abs(v) == r:
                    patches.append({"at": at, "correlation": c, "state": "edge", "over": False, **here})
                else:
                    sc = lambda du, dv: scores[(u + du, v + dv)] if scores[(u + du, v + dv)] is not None else c
                    fu, fv = vertex(sc(-1, 0), c, sc(1, 0)), vertex(sc(0, -1), c, sc(0, 1))
                    xb = ob[0] + (bi + u + fu + 0.5) * PIX
                    yb = ob[1] + (bj + v + fv + 0.5) * -PIX
                    dx, dy = xb - at[0], yb - at[1]
                    d = math.hypot(dx, dy)
                    ok_ = ask.get("correlation") is None or c >= ask["correlation"]
                    patches.append({"at": at, "dx": dx, "dy": dy, "d": d, "correlation": c,
                                    "state": "ok" if ok_ else "low", "over": ok_ and d > limit, **here})
    good = [p for p in patches if p["state"] == "ok"]
    out = {"patches": patches, **{k: sum(1 for p in patches if p["state"] == k) for k in ("ok", "low", "edge", "flat")}}
    if good:
        out["shift"] = stats([{"name": str(k), "role": "check", "dx": p["dx"], "dy": p["dy"], "d": p["d"]}
                              for k, p in enumerate(good)])
        out["passed"] = all(not p["over"] for p in patches)
    if line_out is not None:
        out["line"] = line_out
    return out, good


def seam_case(name, ask, second="dikis-b.tif", truth=S_B, expect_true=True, line=None):
    """`line` (docs/adr/2860): the seam line's local vertices, drawn as the drawing's third object and measured along."""
    s = seam_setup(second, extra=() if line is None else (polyline(line),))
    if line is not None:
        ask = dict(ask, line=uid(3))
    inp = dict(DEFAULT, seam=ask)
    out = compare(s, uid(1), uid(2), "okuma-2024", "okuma-2026")
    limit = out["limit"]
    pa = (ROWS["dikis-a.tif"], A0)
    pb = (ROWS[second], B0)
    world = None if line is None else (uid(3), [(X0 + x, Y0 + y) for x, y in line])
    measured, good = seam((pa, pb), ask, limit, truth, world)
    if expect_true:
        # Places whose window and search stay in the clean texture (off the flat and the changed patches).
        reach = (ask["window"] // 2 + ask["search"] + 1) * PIX
        def clean(at):
            x, y = at[0] - X0, at[1] - Y0
            return all(not (r[0] - reach <= x <= r[1] + reach and r[2] - reach <= y <= r[3] + reach) for r in (FLAT, CHANGED))
        errs = [max(abs(p["dx"] - truth[0]), abs(p["dy"] - truth[1])) / PIX for p in good if clean(p["at"])]
        assert errs and max(errs) < 0.05, (name, len(errs), max(errs))
        ERRORS[name] = (len(errs), max(errs))
    out["seam"] = measured
    where_ = (f"örtüşmede {len(measured['patches'])} yer" if line is None else
              f"dikiş hattı boyunca ({measured['line']['length']:.2f} m) örtüşmede {len(measured['patches'])} yer, "
              f"{measured['line']['outside']} yer örtüşmenin dışında")
    out["notes"].append(
        f"Dikiş ölçümü: {where_}; {measured['ok']} sayıldı, {measured['low']} "
        f"korelasyonu düşük, {measured['edge']} tepe arama sınırında, {measured['flat']} dokusuz. Yerler görüntülerin "
        f"kendisinden normalize çapraz korelasyonla ölçüldü (piksel altı, parabolle).")
    case = {"name": name, "setup": s, "steps": [{"op": "plan", "input": inp, "result": {
        "status": "completed", "output": out, "warnings": []}, "expect": SAME, "within": 1e-9}]}
    return case, measured


ROWS = {}
ERRORS = {}


def seam_cases():
    for name, (origin, shift, flat, change) in IMAGES.items():
        ROWS[name] = image(origin, shift, flat, change)
    ask = {"step": 1.0, "window": 21, "search": 5, "correlation": 0.8}
    c1, m1 = seam_case("dikiş: örtüşmede ölçülen kayma bilinen sentetik kaymadır; dokusuz ve değişen yerler ayrı", ask)
    assert m1["ok"] > 20 and m1["flat"] > 0 and m1["low"] > 0 and m1["passed"], (m1["ok"], m1["flat"], m1["low"])
    c2, m2 = seam_case("dikiş: korelasyon sınırı olmadan değişen yerler de sayılır", {k: v for k, v in ask.items()
                                                                                        if k != "correlation"},
                       expect_true=False)
    assert m2["low"] == 0 and m2["ok"] == m1["ok"] + m1["low"]
    c3, m3 = seam_case("dikiş: 2,7 piksellik kayma sınırı aşar", dict(ask, search=5), "dikis-c.tif", S_C)
    assert m3["passed"] is False and m3["ok"] > 0
    c4, m4 = seam_case("dikiş: arama dar, tepe sınırda kalır", dict(ask, search=2), "dikis-c.tif", S_C, False)
    assert m4["edge"] > 0 and m4["ok"] == 0
    # docs/adr/2860: along a drawn seam line.
    c5, m5 = seam_case("dikiş hattı boyunca: örtüşmedeki yerleri ölçülür, dışındakiler sayılır", ask, line=SEAM_LINE)
    assert m5["line"]["outside"] > 0 and m5["ok"] > 3 and len(m5["patches"]) + m5["line"]["outside"] == m5["line"]["places"]
    assert all(p["station"] <= m5["line"]["length"] for p in m5["patches"])
    seam_in = lambda **kw: {"seam": dict(ask, **kw)}
    bad = "invalid_input"
    ss = seam_setup()
    lined = lambda e: seam_setup(extra=(e,))  # noqa: E731
    return [c1, c2, c3, c4, c5,
            refused("dikiş: sıfır adım", seam_in(step=0.0), bad, "Dikiş ölçümünün adımı 0'dan büyük bir uzunluk olmalı (m).",
                    "seam.step", ss),
            refused("dikiş: çift pencere", seam_in(window=20), bad, "Pencere tek sayıda piksel olmalı, 5 ile 255 arasında.",
                    "seam.window", ss),
            refused("dikiş: sıfır arama", seam_in(search=0), bad, "Arama 1 ile 64 piksel arasında olmalı.", "seam.search",
                    ss),
            refused("dikiş: birden büyük korelasyon", seam_in(correlation=1.5), bad,
                    "En az korelasyon 0'dan büyük, en çok 1 olmalı; her yer sayılacaksa boş bırakın.", "seam.correlation",
                    ss),
            refused("dikiş: pikseller farklı boyda", seam_in(), bad,
                    "Dikiş ölçümü pikselleri aynı boyda ve kuzeye bakan (dönüksüz) iki görüntüde yapılır; birini "
                    "öbürünün ızgarasına yeniden örnekleyin.", "seam", seam_setup(pixel=0.08)),
            refused("dikiş: örtüşme yok", seam_in(), bad,
                    "Görüntüler örtüşmüyor ya da örtüşmeleri pencere ve arama için dar; pencereyi ya da aramayı "
                    "küçültün.", "seam", seam_setup(second_origin=(X0 + 900.0, Y0 + 120.0))),
            refused("dikiş: dosyası olmayan görüntü", seam_in(), "file_not_found", "$any", "second",
                    seam_setup(second="yok.tif")),
            refused("dikiş hattı: çizimde yok", seam_in(line=uid(9)), bad,
                    f"“{uid(9)}” kimlikli dikiş hattı çizimde yok.", "seam.line", ss),
            refused("dikiş hattı: nokta", seam_in(line=uid(3)), bad, "Dikiş hattı bir çizgi ya da çoklu çizgi olmalı.",
                    "seam.line", lined(point(3, "0", 515.0, 114.0, None, "N"))),
            refused("dikiş hattı: yaylı", seam_in(line=uid(3)), bad,
                    "Dikiş hattı düz kenarlı olmalı; yaylı çoklu çizgi bu ölçümde okunmaz.", "seam.line",
                    lined(polyline(SEAM_LINE, [0.2, 0.0, 0.0]))),
            refused("dikiş hattı: örtüşmeden geçmiyor", seam_in(line=uid(3)), bad,
                    "Dikiş hattının hiçbir yeri görüntülerin örtüşmesinde değil (pencere ve arama payıyla); hattı "
                    "örtüşmeden geçirin ya da adımı küçültün.", "seam.line",
                    lined(polyline([(502.0, 119.0), (504.0, 110.0)]))),
            ]


def cases():
    first = compare(BASE, uid(1), uid(2), "okuma-2024", "okuma-2026", "gnss")
    assert first["passed"] and len(first["pairs"]) == 10 and first["firstCheck"]["fit"]["n"] == 2, first
    assert "fit" not in first["secondCheck"] and first["secondCheck"]["check"]["nZ"] == 0, first["secondCheck"]
    shifted = [(n, g, e1, (e2[0] + 0.12, e2[1])) for n, g, e1, e2 in POINTS]
    blunder = [(n, g, e1, (0.31, -0.22) if n == "D6" else e2) for n, g, e1, e2 in POINTS]
    coarse = setup(second=second_raster(pixel=0.25))
    over_coarse = setup(blunder, second=second_raster(pixel=0.25))
    one_and_half = dict(copy.deepcopy(RULES), id="deneme-mozaik-1-5", source="Deneme paketi (sentetik): 1,5·YÖA",
                        mosaic=1.5)
    no_mosaic = {k: v for k, v in copy.deepcopy(RULES).items() if k != "mosaic"}
    extra = [point(0, "okuma-2024", 5, 5, None, "Yalniz-2024"), point(0, "okuma-2026", 6, 6, None, "Yalniz-2026")]
    alone = compare(over_coarse, uid(1), uid(2), "okuma-2024", "okuma-2026")
    assert alone["passed"], alone
    over = compare(setup(blunder), uid(1), uid(2), "okuma-2024", "okuma-2026")
    assert over["passed"] is False and [p["name"] for p in over["pairs"] if p["over"]] == ["D6"], over
    return [
        ok("iki zaman, arazi noktalarıyla: kayma sınırın içinde; her görüntü kendi denetimiyle", known="gnss"),
        ok("uygula: salt okunur", op="execute", known="gnss"),
        ok("doğrulama", op="validate", known="gnss"),
        ok("arazi noktaları olmadan: yalnız iki görüntü"),
        ok("sistematik kayma: 12 cm doğuya kayan ikinci görüntüde üç nokta sınırı aşar", setup(shifted)),
        ok("kaba okunmuş nokta sınırı aşar", setup(blunder)),
        ok("ikinci görüntü 25 cm: sınır iri olanın 2·YÖA'sı, aynı nokta içinde kalır", over_coarse),
        ok("ikinci görüntü 25 cm, arazi noktalarıyla: kendi denetiminde YÖA 1/1000'e kaba", coarse, known="gnss"),
        ok("1/500: görüntülerin kendi denetimi bu ölçeğin sınırıyla", known="gnss", scale=500),
        ok("başka kural paketi: mozaik katsayısı 1,5; D4 sınırı aşar", rules=one_and_half),
        ok("sıra değişince kayma işaret değiştirir", first=uid(2), second=uid(1), firstPoints="okuma-2026",
           secondPoints="okuma-2024"),
        ok("başka ad alanı", setup(field="No"), field="No", known="gnss"),
        ok("eşi olmayan noktalar söylenir", setup(extra=extra), known="gnss"),
        ok("eşleşen nokta yok: sonuç yok", secondPoints="0"),
        ok("raster sistemi 0: projeninki sayılır", setup(second=second_raster(srid=0))),
        ok("kimliğin çevresindeki boşluk okunmaz", first=f" {uid(1)} "),
        refused("ikinci raster projenin sisteminde değil", {}, "crs_mismatch",
                "Ortofotonun koordinat sistemi EPSG:5256, projeninki EPSG:5254; farklı sistemdeki görüntü "
                "karşılaştırılmaz. Önce projeye çevirin (Raster ekle › Projenin sistemine çevir ve ekle).", "second",
                setup(second=second_raster(srid=5256))),
        refused("birinci raster değil", {"first": uid(3)}, "not_a_raster",
                f"“{uid(3)}” bir raster değil; ortofotoyu seçin.", "first"),
        refused("ikinci raster değil", {"second": uid(4)}, "not_a_raster",
                f"“{uid(4)}” bir raster değil; ortofotoyu seçin.", "second"),
        refused("kimlik UUID değil", {"first": "x"}, "invalid_uid", "“x” geçerli bir nesne kimliği değil.", "first"),
        refused("çizimde olmayan raster", {"second": uid(999)}, "entity_not_found",
                f"“{uid(999)}” kimlikli nesne çizimde yok.", "second"),
        refused("aynı ortofoto iki kez", {"second": f" {uid(1)}"}, "invalid_input",
                "Aynı ortofoto iki kez verilmiş; iki ayrı görüntü seçin.", "second"),
        refused("birinci okuma katmanı yok", {"firstPoints": "yok"}, "layer_not_found", "“yok” katmanı çizimde yok.",
                "firstPoints"),
        refused("ikinci okuma katmanı yok", {"secondPoints": "yok"}, "layer_not_found",
                "“yok” katmanı çizimde yok.", "secondPoints"),
        refused("arazi katmanı yok", {"known": "yok"}, "layer_not_found", "“yok” katmanı çizimde yok.", "known"),
        refused("ölçek 0", {"scale": 0}, "invalid_input", "Ölçek paydası 0 olamaz.", "scale"),
        refused("kural paketinin kaynağı boş", {"rules": dict(copy.deepcopy(RULES), source=" ")}, "invalid_input",
                "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan karşılaştırma yapılmaz.",
                "rules"),
        refused("kural paketinde mozaik sınırı yok", {"rules": no_mosaic}, "invalid_input",
                "Kural paketi iki görüntü arasındaki kaymanın sınırını (mosaic: YÖA'nın pozitif katı) vermiyor; "
                "dayanağı yazılmadan karşılaştırma yapılmaz.", "rules"),
        refused("mozaik katsayısı 0", {"rules": dict(copy.deepcopy(RULES), mosaic=0.0)}, "invalid_input",
                "Kural paketi iki görüntü arasındaki kaymanın sınırını (mosaic: YÖA'nın pozitif katı) vermiyor; "
                "dayanağı yazılmadan karşılaştırma yapılmaz.", "rules"),
    ] + seam_cases()


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.orthophoto.compare", "commandVersion": 1,
           "title": "Ortofoto karşılaştırması",
           "note": "ADR 1780. Bağımsız başvuru scripts/fixtures/orthophoto_compare_cases.py: BÖHHBÜY m. 67(6) "
                   "(±2·YÖA) ve her görüntünün m. 54, 91(2) denetimi elle; KentOS kodu yok. Noktalar sentetiktir. "
                   "ADR 2460: dikiş ölçümünün görüntüleri burada bayt bayt yazılır, bilinen kayma bağımsız doğrudur.",
           "setup": BASE, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    files = {name: geotiff(ROWS[name], IMAGES[name][0]) for name in IMAGES}
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text and all(
            (FILES / n).exists() and (FILES / n).read_bytes() == b for n, b in files.items())
        print("Ortofoto karşılaştırması ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    FILES.mkdir(parents=True, exist_ok=True)
    for n, b in files.items():
        (FILES / n).write_bytes(b)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
