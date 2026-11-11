#!/usr/bin/env python3
"""The shared cases of `road.terrains.compare`'s section method (docs/adr/4340; plan/map-cad.md MAP-15; Civil 3D's
sample line volumes, Average End Area and Prismoidal).

The drawing and the exact TIN–TIN are road_terrain_compare_cases.py's: every lattice is z = g(x) + h(y), each square
one plane. A section is the line across the axis (azimuth from north, clockwise) at a distance along it; inside both
lattices' rectangle it crosses their x and y lines, and between two crossings both surfaces are planes, so the
difference d = z_comparison − z_base is linear there: its fill (d > 0) and cut (d < 0) are integrated exactly, split at
its zero. The sections are the comparison's first place along the axis, then every interval while short of its last
place, then the last. Between two sections the volume is the average end area L·(A₀ + A₁)/2 or, prismoidal, with the
section at the middle, L/6·(A₀ + 4·Aₘ + A₁). The plan areas, extremes and the exact volumes beside are the TIN's.

Along a drawn axis (docs/adr/4760): the axis is a line or an open polyline whose segments may be arcs (bulge b: the
arc's angle 4·atan(b), counterclockwise above 0, its centre left of the chord at (c/2)/tan(angle/2) from its middle). The
stations are the axis's start, every interval while short of its end, its end and, asked for, its inner corners; at a
station inside a segment the section is square to it (on an arc along the radius), at an inner corner square to the
bisector of the two segments' ways. A section runs `left` metres left of the axis and `right` metres right, cut back to
both lattices' rectangle; its areas are found as above on that segment. Between two sections the length is their
stations' difference.

No KentOS code.

    python3 scripts/fixtures/road_terrain_compare_section_cases.py [--check]
"""

import bisect
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_compare_cases as cmp  # noqa: E402

analysis = cmp.analysis
OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
UNCHANGED = cmp.UNCHANGED
QUARTERS = {0.0: (0.0, 1.0), 90.0: (1.0, 0.0), 180.0: (0.0, -1.0), 270.0: (-1.0, 0.0)}


def axis(az):
    return QUARTERS.get(az) or (math.sin(math.radians(az)), math.cos(math.radians(az)))


def span_of(n, p, rect):
    """The run of t where p + t·n is inside the closed rectangle; none when it is not."""
    lo, hi = -math.inf, math.inf
    for k in (0, 1):
        a, b = rect[k]
        if n[k] == 0:
            if p[k] < a or p[k] > b:
                return None
        else:
            t0, t1 = (a - p[k]) / n[k], (b - p[k]) / n[k]
            lo, hi = max(lo, min(t0, t1)), min(hi, max(t0, t1))
    return (lo, hi) if hi > lo else None


def section(comparison, base, p, n):
    """The cut and fill areas of the section through p along n."""
    _, fc, cxs, cys = comparison
    _, fb, bxs, bys = base
    rect = ((max(cxs[0], bxs[0]), min(cxs[-1], bxs[-1])), (max(cys[0], bys[0]), min(cys[-1], bys[-1])))
    if rect[0][0] > rect[0][1] or rect[1][0] > rect[1][1]:
        return 0.0, 0.0
    run = span_of(n, p, rect)
    if run is None:
        return 0.0, 0.0
    lo, hi = run
    ts = {lo, hi}
    for k, lines in ((0, cxs + bxs), (1, cys + bys)):
        if n[k] != 0:
            ts |= {(c - p[k]) / n[k] for c in lines if lo < (c - p[k]) / n[k] < hi}
    ts = sorted(ts)
    d = [fc(p[0] + t * n[0], p[1] + t * n[1]) - fb(p[0] + t * n[0], p[1] + t * n[1]) for t in ts]
    cut = fill = 0.0
    for i in range(len(ts) - 1):
        d0, d1, length = d[i], d[i + 1], ts[i + 1] - ts[i]
        if d0 >= 0 and d1 >= 0:
            fill += (d0 + d1) / 2 * length
        elif d0 <= 0 and d1 <= 0:
            cut -= (d0 + d1) / 2 * length
        else:
            f = d0 / (d0 - d1)
            first, second = d0 * f * length / 2, d1 * (1 - f) * length / 2
            if d0 > 0:
                fill, cut = fill + first, cut - second
            else:
                cut, fill = cut - first, fill + second
    return cut, fill


def section_span(comparison, base, p, n, lo, hi):
    """The cut and fill areas of the section through p along n from t = lo to t = hi (docs/adr/4760)."""
    _, fc, cxs, cys = comparison
    _, fb, bxs, bys = base
    rect = ((max(cxs[0], bxs[0]), min(cxs[-1], bxs[-1])), (max(cys[0], bys[0]), min(cys[-1], bys[-1])))
    if rect[0][0] > rect[0][1] or rect[1][0] > rect[1][1]:
        return 0.0, 0.0
    run = span_of(n, p, rect)
    if run is None:
        return 0.0, 0.0
    a, b = max(run[0], lo), min(run[1], hi)
    if not b > a:
        return 0.0, 0.0
    ts = {a, b}
    for k, lines in ((0, cxs + bxs), (1, cys + bys)):
        if n[k] != 0:
            ts |= {(c - p[k]) / n[k] for c in lines if a < (c - p[k]) / n[k] < b}
    ts = sorted(ts)
    d = [fc(p[0] + t * n[0], p[1] + t * n[1]) - fb(p[0] + t * n[0], p[1] + t * n[1]) for t in ts]
    cut = fill = 0.0
    for i in range(len(ts) - 1):
        d0, d1, length = d[i], d[i + 1], ts[i + 1] - ts[i]
        if d0 >= 0 and d1 >= 0:
            fill += (d0 + d1) / 2 * length
        elif d0 <= 0 and d1 <= 0:
            cut -= (d0 + d1) / 2 * length
        else:
            f = d0 / (d0 - d1)
            first, second = d0 * f * length / 2, d1 * (1 - f) * length / 2
            if d0 > 0:
                fill, cut = fill + first, cut - second
            else:
                cut, fill = cut - first, fill + second
    return cut, fill


def segment(a, b, bulge):
    """An axis segment: ('line', a, b, length) or ('arc', centre, radius, start angle, sweep, length); none if no length."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    if not chord > 0:
        return None
    if bulge == 0:
        return ("line", a, b, chord)
    sweep = 4 * math.atan(bulge)
    r = chord / abs(2 * math.sin(sweep / 2))
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    along = (chord / 2) / math.tan(sweep / 2)
    left = (-dy / chord, dx / chord)
    c = (mid[0] + left[0] * along, mid[1] + left[1] * along)
    return ("arc", c, r, math.atan2(a[1] - c[1], a[0] - c[0]), sweep, r * abs(sweep))


def segment_at(seg, length):
    """The place and unit way `length` metres along a segment."""
    if seg[0] == "line":
        _, a, b, ln = seg
        u = ((b[0] - a[0]) / ln, (b[1] - a[1]) / ln)
        return (b, u) if length == ln else ((a[0] + u[0] * length, a[1] + u[1] * length), u)
    _, c, r, start, sweep, _ = seg
    turn = 1.0 if sweep > 0 else -1.0
    angle = start + turn * length / r
    return (c[0] + r * math.cos(angle), c[1] + r * math.sin(angle)), (-turn * math.sin(angle), turn * math.cos(angle))


def axis_sections(comparison, base, pts, bulges, interval, left, right, corners=False, formula="endArea"):
    """The sections along a drawn axis (the module's note): rows (station, at, left end, right end, cut, fill), each
    window's (cut, fill) volume, the sums and the axis's length."""
    segs, starts, s = [], [], 0.0
    for k in range(len(pts) - 1):
        seg = segment(pts[k], pts[k + 1], bulges[k] if k < len(bulges) else 0.0)
        if seg is not None:
            starts.append(s)
            s += seg[-1]
            segs.append(seg)
    length = starts[-1] + segs[-1][-1]

    def place(st):
        k = min(max(bisect.bisect_right(starts, st) - 1, 0), len(segs) - 1)
        p, u = segment_at(segs[k], min(st - starts[k], segs[k][-1]))
        if k > 0 and st == starts[k]:
            _, before = segment_at(segs[k - 1], segs[k - 1][-1])
            w = (before[0] + u[0], before[1] + u[1])
            nw = math.hypot(*w)
            if nw > 0:
                u = (w[0] / nw, w[1] / nw)
        return p, u

    stations, k = [], 0
    while k * interval < length:
        stations.append(k * interval)
        k += 1
    stations.append(length)
    if corners:
        stations += starts[1:]
    stations = sorted(set(stations))

    def row(st):
        p, u = place(st)
        n = (u[1], -u[0])
        cut, fill = section_span(comparison, base, p, n, -left, right)
        return (st, p, (p[0] - n[0] * left, p[1] - n[1] * left), (p[0] + n[0] * right, p[1] + n[1] * right), cut, fill)

    rows = [row(st) for st in stations]
    between = []
    for r0, r1 in zip(rows, rows[1:]):
        length_ = r1[0] - r0[0]
        if formula == "prismoidal":
            m = row((r0[0] + r1[0]) / 2)
            between.append((length_ / 6 * (r0[4] + 4 * m[4] + r1[4]), length_ / 6 * (r0[5] + 4 * m[5] + r1[5])))
        elif formula == "frustum":
            # ADR 5140: the frustum's L/3·(A₀ + A₁ + √(A₀·A₁)), cut and fill each.
            between.append((frustum(r0[4], r1[4], length_), frustum(r0[5], r1[5], length_)))
        else:
            between.append((length_ * (r0[4] + r1[4]) / 2, length_ * (r0[5] + r1[5]) / 2))
    return rows, between, sum(b[0] for b in between), sum(b[1] for b in between), length


def frustum(a0, a1, length):
    """ADR 5140: a frustum's volume between two sections: L/3·(A₀ + A₁ + √(A₀·A₁))."""
    return length / 3 * (a0 + a1 + math.sqrt(a0 * a1))


def by_sections(inp):
    """The sections and volumes of `inp` (its comparison and base by name or id)."""
    find = lambda s: next(i for i, n in cmp.NAMES.items() if s in (i, n))  # noqa: E731
    comparison, base = cmp.LATTICES[find(inp["comparison"])], cmp.LATTICES[find(inp["base"])]
    az, interval = inp.get("azimuth", 0.0), inp["interval"]
    u = axis(az)
    n = (u[1], -u[0])
    _, _, cxs, cys = comparison
    corners = [(x, y) for x in (cxs[0], cxs[-1]) for y in (cys[0], cys[-1])]
    s = [x * u[0] + y * u[1] for x, y in corners]
    lo, hi = min(s), max(s)
    span = hi - lo
    rel, k = [], 0
    while k * interval < span:
        rel.append(k * interval)
        k += 1
    rel.append(span)
    at = lambda r: section(comparison, base, (u[0] * (lo + r), u[1] * (lo + r)), n)  # noqa: E731
    rows = [(r, *at(r)) for r in rel]
    cut = fill = 0.0
    for (r0, c0, f0), (r1, c1, f1) in zip(rows, rows[1:]):
        length = r1 - r0
        if inp.get("formula") == "prismoidal":
            cm, fm = at((r0 + r1) / 2)
            cut += length / 6 * (c0 + 4 * cm + c1)
            fill += length / 6 * (f0 + 4 * fm + f1)
        elif inp.get("formula") == "frustum":
            cut += frustum(c0, c1, length)
            fill += frustum(f0, f1, length)
        else:
            cut += length * (c0 + c1) / 2
            fill += length * (f0 + f1) / 2
    return rows, cut, fill


def answer(inp):
    plain = {k: v for k, v in inp.items() if k not in ("cutFactor", "fillFactor", "axis", "left", "right", "corners")}
    out, warns = cmp.answer(plain)
    if "axis" in inp:
        return axis_answer(inp, out, warns)
    rows, cut, fill = by_sections(inp)
    out["exact"] = {"cut": out["cut"], "fill": out["fill"], "net": out["net"]}
    out["cut"], out["fill"], out["net"] = cut, fill, fill - cut
    out["interval"] = inp["interval"]
    out["azimuth"] = inp.get("azimuth", 0.0)
    out["formula"] = inp.get("formula", "endArea")
    out["sections"] = [{"station": r, "cutArea": c, "fillArea": f} for r, c, f in rows]
    if "cutFactor" in inp or "fillFactor" in inp:
        cf, ff = inp.get("cutFactor", 1.0), inp.get("fillFactor", 1.0)
        out["adjusted"] = {"cutFactor": cf, "fillFactor": ff, "cut": cut * cf, "fill": fill * ff, "net": fill * ff - cut * cf}
    return out, warns


# ── docs/adr/4760: sections along a drawn axis ──────────────────────────────────────────────────────────────────────
ARC = math.tan(math.pi / 8)  # a quarter circle's bulge
AXES = {
    # A straight line across the terrace from west to east: the azimuth 90° sections again.
    "duz": ("line", [(8000.0, 5.0), (8010.0, 5.0)], []),
    # Straight, then a quarter circle counterclockwise round (8005, 6), then straight north.
    "yay": ("polyline", [(8001.0, 2.0), (8005.0, 2.0), (8009.0, 6.0), (8009.0, 9.5)], [0.0, ARC, 0.0]),
    # A corner: east, then north.
    "kose": ("polyline", [(8001.0, 1.0), (8006.0, 1.0), (8006.0, 9.0)], [0.0, 0.0]),
    # A closed polyline: no axis.
    "kapali": ("closed", [(8001.0, 1.0), (8003.0, 1.0), (8003.0, 3.0)], []),
}
AXIS_FIRST = None


def axis_setup():
    """cmp's drawing with an axis layer and the AXES after its objects (their ids follow)."""
    global AXIS_FIRST
    s = cmp.setup()
    s["layers"].append({"id": "eksen", "name": "Eksen", "type": "layer", "visible": True, "locked": False,
                        "expanded": True, "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25},
                        "children": []})
    k = max(e["id"] for e in s["entities"])
    AXIS_FIRST = k + 1
    for key, (kind, pts, bulges) in AXES.items():
        k += 1
        if kind == "line":
            geo = {"kind": "line", "a": {"x": pts[0][0], "y": pts[0][1]}, "b": {"x": pts[1][0], "y": pts[1][1]}}
        else:
            geo = {"kind": "polyline", "pts": [{"x": x, "y": y} for x, y in pts], **({"closed": True} if kind == "closed" else {})}
            if any(b != 0.0 for b in bulges):
                geo["bulges"] = list(bulges) + [0.0]
        s["entities"].append({**geo, "id": k, "layerId": "eksen", "attrs": {}})
        s["uids"].append(cmp.UID.format(k))
    return s


SETUP = axis_setup()


def axis_uid(key):
    return cmp.UID.format(AXIS_FIRST + list(AXES).index(key))


def axis_answer(inp, out, warns):
    """`answer` along a drawn axis."""
    find = lambda s: next(i for i, n in cmp.NAMES.items() if s in (i, n))  # noqa: E731
    comparison, base = cmp.LATTICES[find(inp["comparison"])], cmp.LATTICES[find(inp["base"])]
    key = next(k for k in AXES if axis_uid(k) == inp["axis"])
    _, pts, bulges = AXES[key]
    rows, _, cut, fill, length = axis_sections(comparison, base, pts, bulges, inp["interval"], inp["left"],
                                               inp["right"], inp.get("corners", False),
                                               inp.get("formula", "endArea"))
    out["exact"] = {"cut": out["cut"], "fill": out["fill"], "net": out["net"]}
    out["cut"], out["fill"], out["net"] = cut, fill, fill - cut
    out["interval"] = inp["interval"]
    out["formula"] = inp.get("formula", "endArea")
    out["axis"] = inp["axis"]
    out["length"] = length
    out["sections"] = [{"station": st, "cutArea": c, "fillArea": f, "at": list(p), "ends": [list(a), list(b)]}
                       for st, p, a, b, c, f in rows]
    if "cutFactor" in inp or "fillFactor" in inp:
        cf, ff = inp.get("cutFactor", 1.0), inp.get("fillFactor", 1.0)
        out["adjusted"] = {"cutFactor": cf, "fillFactor": ff, "cut": cut * cf, "fill": fill * ff, "net": fill * ff - cut * cf}
    return out, warns


def map_setup():
    """The drawing as a Harita project: its surfaces alone (a map project keeps no road design)."""
    s = axis_setup()
    s["settings"]["cadWork"] = "map"
    s["settings"]["road"] = {"surfaces": s["settings"]["road"]["surfaces"]}
    return s


def failed(message, path):
    return analysis.failed("invalid_analysis", message, path)


def cases():
    out = []

    def q(name, inp, result=None, op="execute", setup_=None, note=None):
        if result is None:
            o, w = answer(inp)
            result = {"status": "completed", "output": None if op == "validate" else o, "warnings": w}
        c = {"name": name, **({"note": note} if note else {}),
             "steps": [{"op": op, "input": inp, "result": result, "expect": UNCHANGED, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    kd = {"base": "Düz", "comparison": "Kazı", "method": "sections"}
    st = {"base": "Teras", "comparison": "Set", "method": "sections"}
    q("Kazı düzlemin üstünde, kesitler doğu–batı her 2,5 m", {**kd, "interval": 2.5},
      note="semt 0: eksen kuzeye, kesitler y sabit; yarısı tabanın dışında")
    q("Kesitler kuzey–güney (semt 90°) her 2,5 m", {**kd, "interval": 2.5, "azimuth": 90.0})
    q("Kesit aralığı kafese denk gelmez: 3 m", {**kd, "interval": 3.0, "azimuth": 90.0},
      note="son kesit karşılaştırmanın son yerinde, aradaki boy kısa")
    q("Ters yönde eksen (semt 270°): kesitler öbür uçtan başlar", {**kd, "interval": 3.0, "azimuth": 270.0})
    q("Semt 180°: kuzeyden güneye", {**kd, "interval": 3.0, "azimuth": 180.0})
    q("Prizmoidal: ortadaki kesitle", {**kd, "interval": 2.5, "azimuth": 90.0, "formula": "prismoidal"})
    q("Prizmoidal, 3 m", {**kd, "interval": 3.0, "formula": "prismoidal"})
    q("Ortalama alan açıkça", {**kd, "interval": 2.5, "formula": "endArea"})
    # ADR 5140: the frustum's formula.
    q("Kesik koni: (A₀ + A₁ + √(A₀·A₁))·L/3", {**kd, "interval": 2.5, "azimuth": 90.0, "formula": "frustum"})
    q("Kesik koni, 3 m, katsayılarla", {**kd, "interval": 3.0, "formula": "frustum", "cutFactor": 1.25,
                                        "fillFactor": 0.9})
    q("Set terasın üstünde, her metre kafeste", {**st, "interval": 1.0, "azimuth": 90.0},
      note="kazı ve dolgu aynı kesitte, fark sıfırında bölünür")
    q("Set terasın üstünde, 4 m: kaba kesit, yöntem farkı büyür", {**st, "interval": 4.0, "azimuth": 90.0})
    q("Set terasın üstünde, prizmoidal 4 m", {**st, "interval": 4.0, "azimuth": 90.0, "formula": "prismoidal"})
    q("Eğik eksen: semt 30°", {**st, "interval": 2.0, "azimuth": 30.0})
    q("Eğik eksen ve yarısı dışarıda: semt 120°", {**kd, "interval": 1.5, "azimuth": 120.0})
    q("Ters yönde kazı ve dolgu yer değiştirir", {"base": "Set", "comparison": "Teras", "method": "sections",
                                                  "interval": 2.0, "azimuth": 90.0})
    q("Kesit ve katsayılar", {**st, "interval": 2.0, "azimuth": 90.0, "cutFactor": 1.25, "fillFactor": 1.1})
    q("Plan da aynısını söyler", {**kd, "interval": 2.5}, op="plan")
    q("Doğrulama", {**kd, "interval": 2.5}, op="validate")
    q("Harita projesinde", {**kd, "interval": 2.5, "azimuth": 90.0}, setup_=map_setup())
    # Refusals (invalid_analysis), in their order.
    q("Kesitte aralık yok", dict(kd), failed("Kesit yönteminde kesitlerin aralığını (interval, m) verin.", "interval"))
    q("Sıfır aralık", {**kd, "interval": 0.0}, failed("Kesit aralığı 0'dan büyük bir uzunluk olmalı.", "interval"))
    q("Eksi aralık", {**kd, "interval": -2.0}, failed("Kesit aralığı 0'dan büyük bir uzunluk olmalı.", "interval"))
    azimuth = "Eksenin semti 0 ile 360 derece arasında olmalı (kuzeyden saat yönünde; 360 hariç)."
    q("Semt 360", {**kd, "interval": 2.5, "azimuth": 360.0}, failed(azimuth, "azimuth"))
    q("Eksi semt", {**kd, "interval": 2.5, "azimuth": -10.0}, failed(azimuth, "azimuth"))
    q("Kesitte fark aralığı istenmez", {**kd, "interval": 2.5, "ranges": [-1.0, 0.0, 1.0]},
      failed("Fark aralıkları TIN yöntemiyle verilir; kesit yönteminde aralık istenmez.", "ranges"))
    q("Kesitte malzeme bölgesi istenmez", {**kd, "interval": 2.5, "zones": [{"name": "Kaya", "boundary": cmp.UID.format(1)}]},
      failed("Malzeme bölgeleri TIN yöntemiyle ölçülür; kesit yönteminde bölge istenmez.", "zones"))
    q("Formül kesit yönteminindir", {"base": "Düz", "comparison": "Kazı", "formula": "prismoidal"},
      failed("Formül (ortalama alan ya da prizmoidal) kesit yönteminindir; yöntemi kesit seçin ya da formülü kaldırın.",
             "formula"))
    q("Çok sık kesit", {**kd, "interval": 0.0005},
      failed("Bu aralıkla 10 000'den çok kesit olur; kesit aralığını büyütün.", "interval"))
    # Docs/adr/4760: along a drawn axis.
    q("Düz eksen boyunca: semt 90°'nin kesitleri, uçlarıyla", {**st, "interval": 2.0, "axis": axis_uid("duz"),
                                                              "left": 5.0, "right": 5.0},
      note="eksen doğuya: sol uç kuzeyde, sağ uç güneyde; kesitler kafesin bütün eninde")
    q("Yaylı eksen: kurpta kesitler yarıçap doğrultusunda", {**st, "interval": 1.5, "axis": axis_uid("yay"),
                                                            "left": 1.5, "right": 1.5},
      note="kurbun dışındaki sağ uç kafesin dışına taşar: yalnız iki yüzeyin örttüğü kısım sayılır")
    q("Yaylı eksen, prizmoidal", {**st, "interval": 1.5, "axis": axis_uid("yay"), "left": 1.5, "right": 1.5,
                                  "formula": "prismoidal"})
    q("Yaylı eksen, kesik koni", {**st, "interval": 1.5, "axis": axis_uid("yay"), "left": 1.5, "right": 1.5,
                                  "formula": "frustum"})
    q("Köşeli eksen: eksen noktasında açıortaya dik kesit", {**st, "interval": 3.0, "axis": axis_uid("kose"),
                                                             "left": 1.0, "right": 2.0, "corners": True})
    q("Köşeli eksen, eksen noktası istenmeden", {**st, "interval": 3.0, "axis": axis_uid("kose"), "left": 1.0,
                                                  "right": 2.0})
    q("Tek yana genişlik: solda 0", {**kd, "interval": 2.5, "axis": axis_uid("yay"), "left": 0.0, "right": 3.0},
      note="eksen kazının dışında: kesitler boş")
    q("Eksenle katsayılar", {**st, "interval": 2.0, "axis": axis_uid("duz"), "left": 5.0, "right": 5.0,
                             "cutFactor": 1.2, "fillFactor": 0.9})
    widths = "Kesitin sol ve sağ genişliği 0 ya da daha büyük bir uzunluk olmalı; ikisi birden 0 olamaz."
    q("Eksen ve semt birlikte", {**st, "interval": 2.0, "axis": axis_uid("duz"), "left": 5.0, "right": 5.0,
                                 "azimuth": 90.0},
      failed("Kesitlerin ekseni ya semtle ya da çizimdeki bir eksenle verilir; ikisinden birini kaldırın.", "azimuth"))
    q("Eksen kesit yöntemi olmadan", {"base": "Teras", "comparison": "Set", "axis": axis_uid("duz")},
      failed("Çizimdeki eksen kesit yönteminindir; yöntemi kesit seçin ya da ekseni kaldırın.", "axis"))
    q("Genişlik eksensiz", {**st, "interval": 2.0, "left": 5.0},
      failed("Kesitlerin genişlikleri ve köşe kesitleri çizimdeki eksenle verilir; ekseni (axis) verin.", "left"))
    q("Eksen noktaları eksensiz", {**st, "interval": 2.0, "corners": True},
      failed("Kesitlerin genişlikleri ve köşe kesitleri çizimdeki eksenle verilir; ekseni (axis) verin.", "corners"))
    q("Eksenle genişlik yok", {**st, "interval": 2.0, "axis": axis_uid("duz"), "right": 5.0},
      failed("Eksenle kesitlerin sol ve sağ genişliğini (left, right, m) verin.", "left"))
    q("Eksi genişlik", {**st, "interval": 2.0, "axis": axis_uid("duz"), "left": -1.0, "right": 5.0},
      failed(widths, "left"))
    q("İki genişlik 0", {**st, "interval": 2.0, "axis": axis_uid("duz"), "left": 0.0, "right": 0.0}, failed(widths, "left"))
    q("Olmayan eksen", {**st, "interval": 2.0, "axis": cmp.UID.format(99999), "left": 5.0, "right": 5.0},
      analysis.failed("entity_not_found", f"“{cmp.UID.format(99999)}” kimlikli nesne çizimde yok.", "axis"))
    q("Kapalı çoklu çizgi eksen olamaz", {**st, "interval": 2.0, "axis": axis_uid("kapali"), "left": 5.0, "right": 5.0},
      failed(f"“{axis_uid('kapali')}” kesitlerin ekseni olamaz: bir çizgi ya da tek parçalı açık bir çoklu çizgi olmalı.",
             "axis"))
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.compare", "commandVersion": 1,
           "title": "Yüzeyleri karşılaştır: kesit yöntemi (ADR 4340)",
           "note": "scripts/fixtures/road_terrain_compare_section_cases.py'den: kesit doğrusu iki kafesin çizgilerinde "
                   "kırılır, aralarında fark doğrusal, kesin integral; ortalama alan ya da prizmoidal; kesin TIN "
                   "road_terrain_compare_cases.py'nin. KentOS kodu yok.",
           "setup": SETUP, "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.compare.sections.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.compare.sections.json")
    else:
        path.write_text(text)
    print(f"road.terrains.compare.sections.json: {len(found)} durum")


if __name__ == "__main__":
    main()
