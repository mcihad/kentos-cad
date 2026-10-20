#!/usr/bin/env python3
"""The shared cases of cad.pointclouds.region and cad.pointclouds.profile (docs/adr/0552; plan/gis.md CBS-32).

    uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj --with mpmath python3 scripts/fixtures/pointcloud_area_cases.py           # writes
    uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj --with mpmath python3 scripts/fixtures/pointcloud_area_cases.py --check   # compares

No KentOS code is used. The files and the drawing are those of pointcloud_describe_cases.py (laspy's and LASzip's files,
the lattice area-grid.las written there). What an answer is comes from laspy's points (their scaled coordinates, the
raw heights and classes), and from the rules of the ADR:

- region: a point is inside when it lies in the closed outer ring and not strictly inside a hole; the test is the
  crossing number against the ring's edges to the point's east, the sign of every edge's cross product decided exactly
  with fractions where floats cannot (a point on an edge is inside). Statistics: counts, the inside points' box,
  heights' and intensities' smallest, largest and mean (math.fsum), classes counted, the area exactly (fractions) and
  the density.
- profile: a point is in the corridor when its distance to the path (the nearest segment's, the earlier on a tie) is at
  most half the width, decided exactly (the squared distance is rational); its station is that segment's start plus the
  foot's place on it, its offset positive on the left, both with mpmath at 40 digits; stations are bins of `step`
  (floor, the last bin holding the path's end).
- the points read: a source whose box meets one of the area's boxes (a corridor's: each segment's box widened by half
  the width) is read whole; a COPC reads only the nodes whose cube meets one.
"""
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

import mpmath
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pointcloud_describe_cases as pd  # noqa: E402

mpmath.mp.dps = 40

ROOT = pd.ROOT
OUT = pd.OUT
ANY = pd.ANY
WITHIN = 1e-9
TEXT_WARN = "Bulutun {}. kaynağı metin bulutu; noktası burada aranmaz."
MOST = 200_000_000


# ── Who is read ────────────────────────────────────────────────────────

def entity(cid):
    return pd.drawing()["entities"][cid - 1]


def kind_of(src):
    """('file', name) a readable file; 'text'; 'unreadable' (an address, a missing or a broken file)."""
    if "url" in src:
        return "unreadable", None
    if "asset" in src:
        return "file", "las12-f0.las"
    name = src["file"].rsplit("/", 1)[-1]
    if src["format"] == "xyz":
        return "text", None
    if name in ("yok.las", "not-las.las"):
        return "unreadable", None
    return "file", name


def visible_clouds():
    return [e["id"] for e in pd.drawing()["entities"] if e["kind"] == "pointcloud" and e["layerId"] == "bulut"]


def meets(a, b):
    """Boxes [x1, y1, x2, y2] that touch."""
    return not (a[2] < b[0] or a[0] > b[2] or a[3] < b[1] or a[1] > b[3])


def plan_of(clouds, boxes):
    """(cloud place, id, source place, kind, name, record) for every source whose box meets one of the boxes."""
    out = []
    for k, cid in enumerate(clouds):
        for si, src in enumerate(entity(cid)["sources"]):
            b = src["bounds"]
            if not any(meets(x, [b[0], b[1], b[3], b[4]]) for x in boxes):
                continue
            kind, name = kind_of(src)
            out.append((k, cid, si, kind, name, src))
    return out


def warnings_of(plan):
    if any(p[3] == "unreadable" for p in plan):
        return ANY
    return [{"code": "cloud_source_unreadable", "message": TEXT_WARN.format(si + 1),
             "path": f"clouds[{k}].sources[{si}]"} for k, cid, si, kind, name, src in plan if kind == "text"]


def touched(name, boxes):
    """(mask of the file's points that are read in laspy's order, the points read): a COPC's nodes that meet a box."""
    p, order = pd.points_of(name)
    n = len(p.x)
    if not any(u == "copc" for u, _, _ in pd.vlrs_of(pd.FILES / name)):
        return p, order, np.ones(n, dtype=bool), n
    (cx, cy, cz, half), nodes = pd.copc_nodes(pd.FILES / name)
    low = (cx - half, cy - half)
    keys = set()
    for (d, x, y, z), count in nodes:
        side = 2 * half / (1 << d)
        lx, ly = low[0] + x * side, low[1] + y * side
        if count > 0 and any(meets(b, [lx, ly, lx + side, ly + side]) for b in boxes):
            keys.add((d, x, y, z))
    read = sum(c for k, c in nodes if k in keys)
    return p, order, pd.node_mask(name, keys), read


# ── Region ─────────────────────────────────────────────────────────────

def drop_closing(ring):
    ring = [tuple(v) for v in ring]
    return ring[:-1] if len(ring) > 1 and ring[0] == ring[-1] else ring


def exact_sign(ax, ay, bx, by, cx, cy):
    f = Fraction
    v = (f(bx) - f(ax)) * (f(cy) - f(ay)) - (f(by) - f(ay)) * (f(cx) - f(ax))
    return (v > 0) - (v < 0)


def classify(xs, ys, ring):
    """0 outside, 1 on the boundary, 2 strictly inside: the crossing number to the east, edge signs exact."""
    n = len(xs)
    parity = np.zeros(n, dtype=bool)
    boundary = np.zeros(n, dtype=bool)
    m = len(ring)
    for i in range(m):
        (ax, ay), (bx, by) = ring[i], ring[(i + 1) % m]
        cross = (bx - ax) * (ys - ay) - (by - ay) * (xs - ax)
        sign = np.sign(cross)
        tol = 1e-6 * (abs(bx - ax) + abs(by - ay) + 1.0)
        for k in np.flatnonzero(np.abs(cross) <= tol):
            sign[k] = exact_sign(ax, ay, bx, by, xs[k], ys[k])
        on = (sign == 0) & (xs >= min(ax, bx)) & (xs <= max(ax, bx)) & (ys >= min(ay, by)) & (ys <= max(ay, by))
        boundary |= on
        up = (ay <= ys) & (ys < by)
        down = (by <= ys) & (ys < ay)
        parity ^= (up & (sign > 0)) | (down & (sign < 0))
    out = np.where(parity, 2, 0)
    out[boundary] = 1
    return out


def region_mask(xs, ys, ring, holes):
    ok = classify(xs, ys, ring) > 0
    for h in holes:
        ok &= classify(xs, ys, h) != 2
    return ok


def area_of(ring, holes):
    def twice(r):
        s = Fraction(0)
        for i in range(len(r)):
            (ax, ay), (bx, by) = r[i], r[(i + 1) % len(r)]
            s += Fraction(ax) * Fraction(by) - Fraction(bx) * Fraction(ay)
        return abs(s)
    a = twice(ring) - sum((twice(h) for h in holes), Fraction(0))
    return a / 2


class Acc:
    def __init__(self):
        self.x, self.y, self.z, self.i, self.c = [], [], [], [], {}

    def add(self, p, mask):
        self.x += [float(v) for v in np.asarray(p.x)[mask]]
        self.y += [float(v) for v in np.asarray(p.y)[mask]]
        self.z += [float(v) for v in np.asarray(p.z)[mask]]
        self.i += [int(v) for v in np.asarray(p.intensity)[mask]]
        for c in np.asarray(p.classification)[mask]:
            self.c[str(int(c))] = self.c.get(str(int(c)), 0) + 1

    def merge(self, o):
        self.x += o.x
        self.y += o.y
        self.z += o.z
        self.i += o.i
        for k, v in o.c.items():
            self.c[k] = self.c.get(k, 0) + v

    def out(self):
        n = len(self.x)
        out = {"count": n, "classes": dict(self.c)}
        if n:
            out["bounds"] = [min(self.x), min(self.y), min(self.z), max(self.x), max(self.y), max(self.z)]
            out["z"] = {"min": min(self.z), "max": max(self.z), "mean": math.fsum(self.z) / n}
            out["intensity"] = {"min": float(min(self.i)), "max": float(max(self.i)), "mean": sum(self.i) / n}
        return out


def region_answer(clouds, ring, holes=()):
    ring = drop_closing(ring)
    holes = [drop_closing(h) for h in holes]
    xs = [v[0] for v in ring]
    ys = [v[1] for v in ring]
    box = [min(xs), min(ys), max(xs), max(ys)]
    plan = plan_of(clouds, [box])
    accs = [Acc() for _ in clouds]
    reads = [0] * len(clouds)
    for k, cid, si, kind, name, src in plan:
        if kind != "file":
            continue
        p, order, keep, nread = touched(name, [box])
        reads[k] += nread
        px, py = np.asarray(p.x), np.asarray(p.y)
        accs[k].add(p, keep & region_mask(px, py, ring, holes))
    total = Acc()
    for a in accs:
        total.merge(a)
    area = area_of(ring, holes)
    out = {"area": float(area), "read": sum(reads), "total": total.out(),
           "clouds": [{"uid": f"$uidOf:{cid}", "read": reads[k], "stats": accs[k].out()} for k, cid in enumerate(clouds)],
           "revision": "$current"}
    if total.x:
        out["density"] = float(Fraction(len(total.x)) / area)
    return out, warnings_of(plan)


def xy(pts):
    return [{"x": float(x), "y": float(y)} for x, y in pts]


def region_step(clouds, ring, holes=(), uids=True, op="execute"):
    out, warns = region_answer(clouds, ring, holes)
    inp = {"ring": xy(ring)}
    if holes:
        inp["holes"] = [xy(h) for h in holes]
    if uids:
        inp["uids"] = [f"$uidOf:{c}" for c in clouds]
    s = pd.step(inp, pd.done(out, warns if warns != ANY else []), op=op)
    if warns == ANY:
        s["result"]["warnings"] = ANY
    s["within"] = WITHIN
    return s


def rect(x1, y1, x2, y2):
    return [(x1, y1), (x2, y1), (x2, y2), (x1, y2)]


def empty_rect(name, size):
    """A square of the file's box with no point in it (the first found, by whole cells from the box's corner)."""
    p, _ = pd.points_of(name)
    x, y = np.asarray(p.x), np.asarray(p.y)
    x0, y0 = float(x.min()), float(y.min())
    for j in range(int((y.max() - y0) // size)):
        for i in range(int((x.max() - x0) // size)):
            a, b = x0 + 1.0 + i * size, y0 + 1.0 + j * size
            if not ((x >= a) & (x <= a + size) & (y >= b) & (y <= b + size)).any():
                return rect(a, b, a + size, b + size)
    raise ValueError("no empty square")


def region_cases():
    out = []
    cid = lambda n: pd.CLOUD_FILES.index(n) + 1  # noqa: E731
    big = rect(500050.0, 4400050.0, 500250.0, 4400200.0)
    out.append(case("las12-f0: dikdörtgen içindeki noktalar", region_step([cid("las12-f0.las")], big)))
    out.append(case("las12-f1: üçgen", region_step([cid("las12-f1.laz")], [(500000.0, 4400000.0), (500400.0, 4400000.0), (500200.0, 4400300.0)])))
    lshape = [(500000.0, 4400000.0), (500300.0, 4400000.0), (500300.0, 4400100.0), (500100.0, 4400100.0),
              (500100.0, 4400300.0), (500000.0, 4400300.0)]
    out.append(case("las12-f2: L biçimli içbükey alan", region_step([cid("las12-f2.las")], lshape)))
    out.append(case("las12-f3: delikli alan", region_step(
        [cid("las12-f3.laz")], rect(500020.0, 4400020.0, 500380.0, 4400280.0),
        holes=[rect(500100.0, 4400080.0, 500300.0, 4400200.0)])))
    out.append(case("las13-f1: saat yönünde verilen halka", region_step(
        [cid("las13-f1.las")], list(reversed(rect(500100.0, 4400100.0, 500350.0, 4400250.0))))))
    out.append(case("las14-f6: son köşesi birinciyle aynı halka", region_step(
        [cid("las14-f6.laz")], rect(500000.0, 4400000.0, 500200.0, 4400150.0) + [(500000.0, 4400000.0)])))
    out.append(case("las14-f7: sekiz köşeli alan", region_step(
        [cid("las14-f7.las")], [(500050.0, 4400000.0), (500250.0, 4400000.0), (500400.0, 4400100.0), (500400.0, 4400250.0),
                                (500250.0, 4400300.0), (500050.0, 4400300.0), (500000.0, 4400200.0), (500000.0, 4400100.0)])))
    out.append(case("las14-f8: renkli nokta biçimi", region_step([cid("las14-f8.laz")], rect(500200.0, 4400100.0, 500400.0, 4400300.0))))
    out.append(case("index-sample.laz: LAZ'da dosya bütün okunur", region_step(
        [cid("index-sample.laz")], rect(400020.0, 4500020.0, 400060.0, 4500050.0))))
    out.append(case("index-sample.copc.laz: aynı alan, COPC'de yalnız değen düğümler okunur", region_step(
        [cid("index-sample.copc.laz")], rect(400020.0, 4500020.0, 400060.0, 4500050.0))))
    out.append(case("index-sample.copc.laz: bütün alan, bütün düğümler", region_step(
        [cid("index-sample.copc.laz")], rect(399999.0, 4499999.0, 400101.0, 4500101.0))))
    out.append(case("many-chunks.laz: 120 000 nokta", region_step(
        [cid("many-chunks.laz")], [(20.0, 10.0), (180.0, 30.0), (160.0, 140.0), (30.0, 120.0)])))
    out.append(case("meta-wkt.las: GPS zamanı ve ek boyutlu dosya", region_step(
        [cid("meta-wkt.las")], rect(500100.0, 4400100.0, 500110.0, 4400120.0))))
    out.append(case("meta-geokeys.las", region_step([cid("meta-geokeys.las")], rect(500200.0, 4400200.0, 500205.0, 4400210.0))))
    out.append(case("iki bulut: uids sırasıyla, toplam ikisinin birleşimi", region_step(
        [cid("las12-f1.laz"), cid("las12-f0.las")], big)))
    out.append(case("iki kaynaklı bulut: kaynakları birlikte", region_step([22], rect(500100.0, 4400100.0, 500300.0, 4400300.0))))
    out.append(case("gömülü bulut kitaplıktan okunur", region_step([18], big)))
    out.append(case("gizli katmandaki bulut adıyla sorulunca okunur", region_step([21], rect(500100.0, 4400100.0, 500120.0, 4400120.0))))
    out.append(case("hiçbir noktaya değmeyen alan: sıfır nokta, okunan yok", region_step(
        [cid("las12-f0.las")], rect(100.0, 100.0, 200.0, 200.0))))
    out.append(case("alan dosyanın kutusuna değip noktaya değmez: dosya okunur, sıfır nokta", region_step(
        [cid("las12-f0.las")], empty_rect("las12-f0.las", 12.0))))
    text_ring = rect(500000.0, 4400000.0, 500090.0, 4400090.0)
    out.append(case("metin bulutu aranmaz, söylenir", region_step([14], text_ring)))
    out.append(case("dosyası olmayan, bozuk ve adresteki kaynaklar uyarıyla atlanır", region_step([15, 16, 17], text_ring)))
    visible = visible_clouds()
    out.append(case("uids verilmezse görünen bulutlar, gizli katmandaki bakılmaz", region_step(
        visible, rect(500100.0, 4400100.0, 500130.0, 4400130.0), uids=False)))
    out.append(case("plan aynı cevabı yazmadan verir", region_step([cid("las12-f0.las")], big, op="plan")))
    out.append(case("validate yalnız denetler", pd.step(
        {"ring": xy(big)}, {"status": "completed", "output": None, "warnings": []}, op="validate")))
    # The lattice: every coordinate is exact, so the edges lie on points.
    g = 34
    out.append(case("kafes: sınırdaki noktalar alanın içindedir", region_step([g], rect(1002.0, 2002.0, 1010.0, 2008.0))))
    out.append(case("kafes: deliğin sınırındaki nokta alanın içinde, kesin içindeki dışında", region_step(
        [g], rect(1002.0, 2002.0, 1010.0, 2008.0), holes=[rect(1004.0, 2004.0, 1008.0, 2006.0)])))
    out.append(case("kafes: eğik kenar kafesin noktalarından geçer", region_step(
        [g], [(1000.0, 2000.0), (1020.0, 2000.0), (1000.0, 2020.0)])))
    out.append(case("kafes: köşe bir noktanın üstünde, saat yönünde halka", region_step(
        [g], list(reversed([(1005.0, 2005.0), (1015.0, 2005.0), (1015.0, 2015.0), (1005.0, 2015.0)])))))
    out.append(case("kafes: iki delikli alan", region_step(
        [g], rect(1000.0, 2000.0, 1020.0, 2020.0),
        holes=[rect(1002.0, 2002.0, 1006.0, 2006.0), rect(1010.0, 2010.0, 1018.0, 2014.0)])))
    out.append(case("kafes: ince şerit alan", region_step([g], rect(1000.0, 2010.0, 1020.0, 2010.0001))))
    # Refusals.
    big_text = "Halka en az üç köşeli olmalı; alanın köşelerini sırayla verin."
    out.append(case("halka iki köşeli", pd.step({"ring": xy([(0.0, 0.0), (1.0, 1.0)])}, pd.failed("invalid_cloud_area", big_text, "ring"))))
    out.append(case("son köşe birinciyle aynıysa üç köşe sayılır", pd.step(
        {"ring": xy([(0.0, 0.0), (1.0, 1.0), (0.0, 0.0)])}, pd.failed("invalid_cloud_area", big_text, "ring"))))
    zero = "Alanın büyüklüğü sıfırdan büyük olmalı (halka eksi delikler); köşeler bir doğru üzerinde ya da delikler halkayı kaplıyor."
    out.append(case("köşeler bir doğru üzerinde", pd.step(
        {"ring": xy([(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)])}, pd.failed("invalid_cloud_area", zero, "ring"))))
    out.append(case("delik halkayı kaplıyor", pd.step(
        {"ring": xy(rect(0.0, 0.0, 4.0, 4.0)), "holes": [xy(rect(-1.0, -1.0, 5.0, 5.0))]},
        pd.failed("invalid_cloud_area", zero, "ring"))))
    out.append(case("delik iki köşeli", pd.step(
        {"ring": xy(rect(0.0, 0.0, 4.0, 4.0)), "holes": [xy([(1.0, 1.0), (2.0, 2.0)])]},
        pd.failed("invalid_cloud_area", "Delik en az üç köşeli olmalı; alanın köşelerini sırayla verin.", "holes[0]"))))
    out.append(case("geçersiz kimlik", pd.step({"ring": xy(big), "uids": ["bulut"]}, pd.failed(
        "invalid_uid", f"“bulut” {pd.UID_TEXT}", "uids[0]"))))
    out.append(case("çizimde olmayan nesne", pd.step({"ring": xy(big), "uids": [pd.GONE]}, pd.failed(
        "entity_not_found",
        f"“{pd.GONE}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
        "uids[0]"))))
    out.append(case("nokta bulutu olmayan nesne", pd.step({"ring": xy(big), "uids": ["$uidOf:19"]}, pd.failed(
        "not_a_pointcloud", "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.", "uids[0]"))))
    out.append(case("kaydında çok nokta olan dosya okunmaz", pd.step(
        {"ring": xy(rect(1000.0, 2000.0, 1010.0, 2010.0)), "uids": ["$uidOf:35"]}, pd.failed(
            "too_many_cloud_points",
            f"Bu alan bir çağrının okuduğundan çok nokta okutur; bir çağrı en çok {MOST} nokta okur. Küçültün ya da bulutları ayrı ayrı sorun.",
            "ring"))))
    return out


def case(name, *steps):
    return pd.case(name, *steps)


# ── Profile ────────────────────────────────────────────────────────────

class Segment:
    def __init__(self, a, b):
        self.a, self.b = a, b
        fa = [Fraction(v) for v in (*a, *b)]
        self.f = fa
        self.abx, self.aby = fa[2] - fa[0], fa[3] - fa[1]
        self.len2 = self.abx * self.abx + self.aby * self.aby
        self.len = mpmath.sqrt(mpmath.mpf(self.len2.numerator) / self.len2.denominator)


def exact_d2(seg, px, py):
    """The squared distance of (px, py) to a segment, exactly (rational), and the foot's dot and cross."""
    apx, apy = Fraction(px) - seg.f[0], Fraction(py) - seg.f[1]
    dot = apx * seg.abx + apy * seg.aby
    cross = seg.abx * apy - seg.aby * apx
    if dot <= 0:
        return apx * apx + apy * apy, dot, cross
    if dot >= seg.len2:
        bpx, bpy = Fraction(px) - seg.f[2], Fraction(py) - seg.f[3]
        return bpx * bpx + bpy * bpy, dot, cross
    return cross * cross / seg.len2, dot, cross


def profile_answer(clouds, path, width, step, points=False):
    segs = [Segment(path[i], path[i + 1]) for i in range(len(path) - 1)]
    half = Fraction(width) / 2
    half2 = half * half
    starts, at = [], mpmath.mpf(0)
    for s in segs:
        starts.append(at)
        at += s.len
    length = at
    n = max(1, int(mpmath.ceil(length / mpmath.mpf(step))))
    hf = float(half)
    boxes = [[min(s.a[0], s.b[0]) - hf, min(s.a[1], s.b[1]) - hf, max(s.a[0], s.b[0]) + hf, max(s.a[1], s.b[1]) + hf]
             for s in segs]
    plan = plan_of(clouds, boxes)
    bins = [[] for _ in range(n)]
    listed = []
    reads = [0] * len(clouds)
    count = 0
    for k, cid, si, kind, name, src in plan:
        if kind != "file":
            continue
        p, order, keep, nread = touched(name, boxes)
        reads[k] += nread
        px, py = np.asarray(p.x), np.asarray(p.y)
        pz, pc = np.asarray(p.z), np.asarray(p.classification)
        # Float prefilter: a point beyond every segment's widened box is out; the rest are decided exactly.
        near = np.zeros(len(px), dtype=bool)
        for b in boxes:
            near |= (px >= b[0]) & (px <= b[2]) & (py >= b[1]) & (py <= b[3])
        for i in np.flatnonzero(keep & near):
            x, y = float(px[i]), float(py[i])
            best = None
            for j, s in enumerate(segs):
                d2, dot, cross = exact_d2(s, x, y)
                if best is None or d2 < best[0]:
                    best = (d2, j, dot, cross)
            d2, j, dot, cross = best
            if d2 > half2:
                continue
            s = segs[j]
            lenf = s.len
            dotm = mpmath.mpf(dot.numerator) / dot.denominator
            along = 0 if dot <= 0 else (lenf if dot >= s.len2 else dotm / lenf)
            station = starts[j] + along
            offset = (mpmath.mpf(cross.numerator) / cross.denominator) / lenf
            b = min(int(mpmath.floor(station / mpmath.mpf(step))), n - 1)
            count += 1
            bins[b].append(float(pz[i]))
            if points:
                listed.append((float(station), k, si, int(order[i]), {
                    "cloud": f"$uidOf:{cid}", "source": si, "index": int(order[i]), "station": float(station),
                    "offset": float(offset), "x": x, "y": y, "z": float(pz[i]), "class": int(pc[i])}))
    out_bins = []
    for b in range(n):
        zs = bins[b]
        d = {"from": b * step, "to": min((b + 1) * step, float(length)), "count": len(zs)}
        if zs:
            d["z"] = {"min": min(zs), "max": max(zs), "mean": math.fsum(zs) / len(zs)}
        out_bins.append(d)
    out = {"length": float(length), "count": count, "read": sum(reads), "bins": out_bins, "revision": "$current"}
    if points:
        listed.sort(key=lambda t: t[:4])
        out["points"] = [t[4] for t in listed]
    return out, warnings_of(plan)


def profile_step(clouds, path, width, step, points=False, uids=True, op="execute"):
    out, warns = profile_answer(clouds, path, width, step, points)
    inp = {"path": xy(path), "width": width, "step": step}
    if points:
        inp["points"] = True
    if uids:
        inp["uids"] = [f"$uidOf:{c}" for c in clouds]
    s = pd.step(inp, pd.done(out, warns if warns != ANY else []), op=op)
    if warns == ANY:
        s["result"]["warnings"] = ANY
    s["within"] = WITHIN
    return s


def profile_cases():
    out = []
    cid = lambda n: pd.CLOUD_FILES.index(n) + 1  # noqa: E731
    out.append(case("las12-f0: yatay yol, dört istasyon", profile_step(
        [cid("las12-f0.las")], [(500000.0, 4400150.0), (500400.0, 4400150.0)], 40.0, 100.0)))
    out.append(case("las12-f0: aynı koridorun noktaları istasyonlarıyla ve ofsetleriyle", profile_step(
        [cid("las12-f0.las")], [(500000.0, 4400150.0), (500400.0, 4400150.0)], 40.0, 100.0, points=True)))
    out.append(case("las12-f1: eğik yol", profile_step(
        [cid("las12-f1.laz")], [(500000.0, 4400000.0), (500300.0, 4400400.0)], 60.0, 50.0)))
    out.append(case("las12-f2: köşeli yol, iki parça", profile_step(
        [cid("las12-f2.las")], [(500000.0, 4400100.0), (500200.0, 4400100.0), (500200.0, 4400300.0)], 50.0, 80.0, points=True)))
    out.append(case("las12-f3: U biçimli yol, yuvarlak birleşimler", profile_step(
        [cid("las12-f3.laz")], [(500050.0, 4400250.0), (500050.0, 4400050.0), (500350.0, 4400050.0), (500350.0, 4400250.0)],
        30.0, 120.0)))
    out.append(case("las13-f1: ters yönde çizilen yolun ofseti ters işaretli", profile_step(
        [cid("las13-f1.las")], [(500400.0, 4400150.0), (500000.0, 4400150.0)], 40.0, 100.0, points=True)))
    out.append(case("las14-f6", profile_step([cid("las14-f6.laz")], [(500000.0, 4400000.0), (500400.0, 4400300.0)], 50.0, 100.0)))
    out.append(case("las14-f7", profile_step([cid("las14-f7.las")], [(500100.0, 4400300.0), (500100.0, 4400000.0)], 80.0, 75.0, points=True)))
    out.append(case("las14-f8", profile_step([cid("las14-f8.laz")], [(500000.0, 4400100.0), (500400.0, 4400200.0)], 70.0, 150.0)))
    out.append(case("index-sample.laz: dosya bütün okunur", profile_step(
        [cid("index-sample.laz")], [(400010.0, 4500050.0), (400090.0, 4500050.0)], 6.0, 20.0, points=True)))
    out.append(case("index-sample.copc.laz: aynı koridor, yalnız değen düğümler okunur", profile_step(
        [cid("index-sample.copc.laz")], [(400010.0, 4500050.0), (400090.0, 4500050.0)], 6.0, 20.0, points=True)))
    out.append(case("index-sample.copc.laz: köşeli yol, her parçanın kutusuna değen düğümler", profile_step(
        [cid("index-sample.copc.laz")], [(400005.0, 4500005.0), (400005.0, 4500095.0), (400095.0, 4500095.0)], 4.0, 30.0)))
    out.append(case("many-chunks.laz: uzun dar koridor", profile_step(
        [cid("many-chunks.laz")], [(10.0, 20.0), (190.0, 120.0)], 4.0, 25.0)))
    out.append(case("meta-wkt.las", profile_step(
        [cid("meta-wkt.las")], [(500100.0, 4400100.0), (500120.0, 4400120.0)], 6.0, 5.0, points=True)))
    out.append(case("meta-geokeys.las", profile_step(
        [cid("meta-geokeys.las")], [(500200.0, 4400200.0), (500210.0, 4400210.0)], 4.0, 5.0)))
    out.append(case("adım yoldan uzunsa tek istasyon", profile_step(
        [cid("las12-f0.las")], [(500000.0, 4400150.0), (500100.0, 4400150.0)], 40.0, 1000.0)))
    out.append(case("adım yolun uzunluğunu bölmezse son istasyon kısadır", profile_step(
        [cid("las12-f0.las")], [(500000.0, 4400150.0), (500250.0, 4400150.0)], 40.0, 100.0)))
    out.append(case("koridorda nokta yok: boş istasyonlar", profile_step(
        [cid("las12-f0.las")], [(500000.0, 4400500.0), (500400.0, 4400500.0)], 4.0, 100.0)))
    g = 34
    out.append(case("kafes: ofsetin tam yarı genişlik olduğu nokta koridordadır, istasyon sınırındaki sonraki istasyona geçer", profile_step(
        [g], [(1000.0, 2005.0), (1020.0, 2005.0)], 4.0, 5.0, points=True)))
    out.append(case("kafes: 5-12-13 doğrultulu yol", profile_step(
        [g], [(1000.0, 2000.0), (1012.0, 2005.0)], 3.0, 4.0, points=True)))
    out.append(case("kafes: köşeli yol, birleşimdeki noktalar en yakın parçaya", profile_step(
        [g], [(1000.0, 2000.0), (1010.0, 2000.0), (1010.0, 2010.0)], 2.0, 5.0, points=True)))
    out.append(case("kafes: yolun ucundaki yuvarlak kapak", profile_step(
        [g], [(1005.0, 2005.0), (1010.0, 2005.0)], 2.0, 2.5, points=True)))
    out.append(case("iki bulut: uids sırasıyla", profile_step(
        [cid("las12-f1.laz"), cid("las12-f0.las")], [(500000.0, 4400150.0), (500400.0, 4400150.0)], 40.0, 100.0, points=True)))
    out.append(case("iki kaynaklı bulut", profile_step([22], [(500100.0, 4400100.0), (500250.0, 4400250.0)], 30.0, 50.0)))
    out.append(case("gömülü bulut kitaplıktan okunur", profile_step(
        [18], [(500000.0, 4400150.0), (500400.0, 4400150.0)], 40.0, 100.0)))
    visible = visible_clouds()
    out.append(case("uids verilmezse görünen bulutlar, gizli katmandaki bakılmaz", profile_step(
        visible, [(500100.0, 4400100.0), (500130.0, 4400130.0)], 6.0, 20.0, uids=False)))
    out.append(case("metin bulutu aranmaz, söylenir", profile_step(
        [14], [(500000.0, 4400050.0), (500090.0, 4400050.0)], 10.0, 30.0)))
    out.append(case("plan aynı cevabı yazmadan verir", profile_step(
        [cid("las12-f0.las")], [(500000.0, 4400150.0), (500400.0, 4400150.0)], 40.0, 100.0, op="plan")))
    out.append(case("validate yalnız denetler", pd.step(
        {"path": xy([(0.0, 0.0), (10.0, 0.0)]), "width": 2.0, "step": 5.0},
        {"status": "completed", "output": None, "warnings": []}, op="validate")))
    # Refusals.
    ok = [(0.0, 0.0), (10.0, 0.0)]
    out.append(case("yol tek köşeli", pd.step({"path": xy([(0.0, 0.0)]), "width": 2.0, "step": 5.0}, pd.failed(
        "invalid_corridor", "Profilin yolu en az iki köşeli olmalı; yolun köşelerini sırayla verin.", "path"))))
    out.append(case("yolun art arda iki köşesi aynı", pd.step(
        {"path": xy([(0.0, 0.0), (5.0, 0.0), (5.0, 0.0)]), "width": 2.0, "step": 5.0}, pd.failed(
            "invalid_corridor", "Yolun 3. köşesi öncekiyle aynı; uzunluğu olmayan parça olmaz.", "path[2]"))))
    out.append(case("koridorun genişliği sıfır", pd.step({"path": xy(ok), "width": 0.0, "step": 5.0}, pd.failed(
        "invalid_corridor",
        "Koridorun genişliği sıfırdan büyük olmalı; yolun iki yanında toplam genişliği verin.", "width"))))
    out.append(case("koridorun genişliği eksi", pd.step({"path": xy(ok), "width": -1.0, "step": 5.0}, pd.failed(
        "invalid_corridor",
        "Koridorun genişliği sıfırdan büyük olmalı; yolun iki yanında toplam genişliği verin.", "width"))))
    out.append(case("istasyon adımı sıfır", pd.step({"path": xy(ok), "width": 2.0, "step": 0.0}, pd.failed(
        "invalid_corridor", "İstasyon adımı sıfırdan büyük olmalı; yol boyunca adımı verin.", "step"))))
    long_path = [(0.0, 0.0), (300.0, 400.0)]
    stations = math.ceil(500.0 / 0.001)
    out.append(case("çok istasyon", pd.step({"path": xy(long_path), "width": 2.0, "step": 0.001}, pd.failed(
        "too_many_stations",
        f"Profil {stations} istasyon olurdu; bir çağrı en çok 100000 istasyon verir. Adımı büyütün ya da yolu kısaltın.",
        "step"))))
    out.append(case("koridorda çok nokta listelenemez", pd.step(
        {"path": xy([(0.0, 75.0), (200.0, 75.0)]), "width": 200.0, "step": 50.0, "points": True,
         "uids": [f"$uidOf:{cid('many-chunks.laz')}"]}, pd.failed(
            "too_many_listed_points",
            "Koridorda 100000 noktadan çoğu var; bir çağrı en çok bu kadar nokta listeler. Koridoru daraltın ya da points'i kapatın (istasyonlar yine verilir).",
            "points"))))
    out.append(case("geçersiz kimlik", pd.step({"path": xy(ok), "width": 2.0, "step": 5.0, "uids": ["bulut"]}, pd.failed(
        "invalid_uid", f"“bulut” {pd.UID_TEXT}", "uids[0]"))))
    out.append(case("çizimde olmayan nesne", pd.step({"path": xy(ok), "width": 2.0, "step": 5.0, "uids": [pd.GONE]}, pd.failed(
        "entity_not_found",
        f"“{pd.GONE}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
        "uids[0]"))))
    out.append(case("nokta bulutu olmayan nesne", pd.step(
        {"path": xy(ok), "width": 2.0, "step": 5.0, "uids": ["$uidOf:19"]}, pd.failed(
            "not_a_pointcloud", "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.", "uids[0]"))))
    out.append(case("kaydında çok nokta olan dosya okunmaz", pd.step(
        {"path": xy([(1000.0, 2000.0), (1010.0, 2000.0)]), "width": 2.0, "step": 5.0, "uids": ["$uidOf:35"]}, pd.failed(
            "too_many_cloud_points",
            f"Bu profil bir çağrının okuduğundan çok nokta okutur; bir çağrı en çok {MOST} nokta okur. Küçültün ya da bulutları ayrı ayrı sorun.",
            "path"))))
    return out


NOTE = ("ADR 0552. Bağımsız başvuru scripts/fixtures/pointcloud_area_cases.py: noktalar laspy'den, alanın içi kesirlerle "
        "(kenar işaretleri tam), koridorun uzaklığı kesirlerle, istasyon ve ofset mpmath'le 40 basamakta, ortalamalar "
        "math.fsum'la; KentOS kodu yok.")


def main():
    files = {
        "cad.pointclouds.region": ("Nokta bulutu alan istatistiği", region_cases),
        "cad.pointclouds.profile": ("Nokta bulutu profili", profile_cases),
    }
    bad = 0
    for command, (title, make) in files.items():
        doc = pd.build(command, title, make())
        doc["note"] = NOTE
        out = OUT / f"{command}.json"
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv:
            if not out.exists() or out.read_text("utf-8") != text:
                print(f"{out.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad += 1
            continue
        out.write_text(text, "utf-8")
        print(f"yazıldı: {out.relative_to(ROOT)}")
    if "--check" in sys.argv and not bad:
        print("Nokta bulutu alan ve profil durumları tutarlı")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
