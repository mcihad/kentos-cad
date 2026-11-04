#!/usr/bin/env python3
"""An earthwork's progress between its original ground and a later survey against its design, shared cases
(docs/adr/0942, plan/road.md Y36.1), from an independent reference in exact fractions: each place where a design, an
original and a survey triangle overlap (scripts/fixtures/road_volume_cases.py's clipping), cut along z_o = z_d,
z_n = z_d and z_o = z_n, a piece on such a line throughout kept on one side; on each piece the ADR's quantities at the
centroid times the area. No KentOS code.

Before writing, the cases with planes are checked against closed forms (a 2 m cut surveyed at 1 m: done = left = area,
and so on), and wherever the survey covers the whole overlap, done + left is checked against the design's own cut and
fill on the original ground by Y8's pairwise reference (scripts/fixtures/road_volume_cases.py `between`): at each point
done + left = |z_o − z_d| whatever the survey says, so the split may move volume between the two but never make or
lose any.

    python3 scripts/fixtures/road_volume_progress_cases.py
    ... --check   # fails when fixtures/road/v1/volume_progress.json is not what this writes
"""

import json
import random
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_volume_cases as vref  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "volume_progress.json"
KEYS = ("cutDone", "cutLeft", "cutOver", "fillDone", "fillLeft", "fillOver")


def tri(t):
    return vref.ccw([tuple(map(F, p)) for p in t])


def clip_by(poly, g):
    for k in range(3):
        poly = vref.clip(poly, g[k], g[(k + 1) % 3])
        if len(poly) < 3:
            return []
    return poly


def box(pts):
    return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))


def meets(a, b):
    return not (a[2] < b[0] or b[2] < a[0] or a[3] < b[1] or b[3] < a[1])


def split3(poly, f, above):
    val = lambda z: f[0] * z[0] + f[1] * z[1] + f[2] * z[2]  # noqa: E731
    keep = (lambda v: v >= 0) if above else (lambda v: v <= 0)
    out = []
    for i, (p, z) in enumerate(poly):
        q, w = poly[(i + 1) % len(poly)]
        vp, vq = val(z), val(w)
        if keep(vp):
            out.append((p, z))
        if (vp > 0 > vq) or (vp < 0 < vq):
            t = vp / (vp - vq)
            out.append(((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t), tuple(z[k] + (w[k] - z[k]) * t for k in range(3))))
    return out


def mean3(poly):
    area, z = F(0), [F(0)] * 3
    for i in range(1, len(poly) - 1):
        a, b, c = poly[0], poly[i], poly[i + 1]
        s = vref.area2(a[0], b[0], c[0]) / 2
        area += s
        for k in range(3):
            z[k] += s * (a[1][k] + b[1][k] + c[1][k]) / 3
    return (area, [v / area for v in z]) if area > 0 else (F(0), [F(0)] * 3)


def progress(design, original, now):
    out = {k: F(0) for k in KEYS + ("covered", "uncovered")}
    os_ = [t for t in (tri(t) for t in original) if t]
    ns = [t for t in (tri(t) for t in now) if t]
    o_box, n_box = [box(t) for t in os_], [box(t) for t in ns]
    for t in design:
        d = tri(t)
        if not d:
            continue
        whole = vref.area2(d[0], d[1], d[2]) / 2
        covered = F(0)
        d_box = box(d)
        for o, ob in zip(os_, o_box):
            # A triangle whose box misses another's meets none of it.
            if not meets(d_box, ob):
                continue
            base = clip_by([(p[0], p[1]) for p in d], o)
            if not base:
                continue
            b_box = box(base)
            for n, nb in zip(ns, n_box):
                if not meets(b_box, nb):
                    continue
                poly = clip_by(base, n)
                if not poly:
                    continue
                piece = [(p, (vref.z_at(d, p), vref.z_at(o, p), vref.z_at(n, p))) for p in poly]
                covered += mean3(piece)[0]
                parts = [piece]
                for f in ((-1, 1, 0), (-1, 0, 1), (0, 1, -1)):
                    nxt = []
                    for p in parts:
                        if all(f[0] * z[0] + f[1] * z[1] + f[2] * z[2] == 0 for _, z in p):
                            nxt.append(p)
                        else:
                            nxt += [split3(p, f, True), split3(p, f, False)]
                    parts = [p for p in nxt if len(p) >= 3]
                for p in parts:
                    area, (zd, zo, zn) = mean3(p)
                    if area <= 0:
                        continue
                    if zo > zd:
                        out["cutDone"] += area * max(F(0), zo - max(zn, zd))
                        out["cutLeft"] += area * max(F(0), min(zn, zo) - zd)
                        out["cutOver"] += area * max(F(0), zd - zn)
                    elif zd > zo:
                        out["fillDone"] += area * max(F(0), min(zn, zd) - zo)
                        out["fillLeft"] += area * max(F(0), zd - max(zn, zo))
                        out["fillOver"] += area * max(F(0), zn - zd)
        out["covered"] += covered
        out["uncovered"] += max(whole - covered, F(0))
    return out


# ---- the surfaces --------------------------------------------------------------------------------------------------

def grid(n, m, step, z, x0=0, y0=0):
    """An n × m grid of corners as triangles, each cell two."""
    tris = []
    for j in range(m - 1):
        for i in range(n - 1):
            a, b = (x0 + i * step, y0 + j * step), (x0 + (i + 1) * step, y0 + j * step)
            c, d = (x0 + (i + 1) * step, y0 + (j + 1) * step), (x0 + i * step, y0 + (j + 1) * step)
            p = lambda q: [q[0], q[1], z(q[0], q[1])]  # noqa: E731
            tris += [[p(a), p(b), p(c)], [p(a), p(c), p(d)]]
    return tris


def plane(z0, gx=0.0, gy=0.0):
    return lambda x, y: z0 + gx * x + gy * y


def bumpy(seed, base, amp):
    rnd = random.Random(seed)
    table = {}

    def z(x, y):
        k = (x, y)
        if k not in table:
            table[k] = round(base + amp * (rnd.random() - 0.5), 3)
        return table[k]
    return z


def case(name, design, original, now, closed=None):
    got = progress(design, original, now)
    if closed is not None:
        for k, v in closed.items():
            assert got[k] == F(v).limit_denominator(10**9) or abs(float(got[k]) - v) < 1e-9, (name, k, float(got[k]), v)
    pair = vref.between(design, original)
    if abs(float(got["uncovered"]) - pair["uncovered"]) < 1e-9:
        assert abs(float(got["cutDone"] + got["cutLeft"]) - pair["cut"]) < 1e-9, (name, "cut", pair["cut"])
        assert abs(float(got["fillDone"] + got["fillLeft"]) - pair["fill"]) < 1e-9, (name, "fill", pair["fill"])
    return {"name": name, "design": design, "original": original, "now": now,
            "expected": {k: float(v) for k, v in got.items()}}


def cases():
    out = []
    A = 400.0  # a 20 × 20 m design
    des = lambda z: grid(3, 3, 10, z)  # noqa: E731
    org = lambda z: grid(4, 4, 10, z, -5, -5)  # noqa: E731
    out.append(case("Yarma 2 m, ölçü 1 m inmiş: yapılan = kalan", des(plane(98)), org(plane(100)), org(plane(99)),
                    {"cutDone": A, "cutLeft": A, "cutOver": 0}))
    out.append(case("Yarma bitti, 0,5 m fazla kazılmış", des(plane(98)), org(plane(100)), org(plane(97.5)),
                    {"cutDone": 2 * A, "cutLeft": 0, "cutOver": 0.5 * A}))
    out.append(case("Yarmaya dokunulmamış", des(plane(98)), org(plane(100)), org(plane(100)),
                    {"cutDone": 0, "cutLeft": 2 * A, "cutOver": 0}))
    out.append(case("Yarmanın üstüne yığın: sayılmaz", des(plane(98)), org(plane(100)), org(plane(101)),
                    {"cutDone": 0, "cutLeft": 2 * A, "cutOver": 0}))
    out.append(case("Ölçü tasarıma ulaşmış: yarma tamam", des(plane(98)), org(plane(100)), des(plane(98)),
                    {"cutDone": 2 * A, "cutLeft": 0, "cutOver": 0}))
    out.append(case("Dolgu 2 m, 1 m serilmiş", des(plane(102)), org(plane(100)), org(plane(101)),
                    {"fillDone": A, "fillLeft": A, "fillOver": 0}))
    out.append(case("Dolgu 0,5 m fazla serilmiş", des(plane(102)), org(plane(100)), org(plane(102.5)),
                    {"fillDone": 2 * A, "fillLeft": 0, "fillOver": 0.5 * A}))
    out.append(case("Dolgu yerinden kazılmış: sayılmaz", des(plane(102)), org(plane(100)), org(plane(99)),
                    {"fillDone": 0, "fillLeft": 2 * A, "fillOver": 0}))
    out.append(case("Ölçü tasarıma ulaşmış: dolgu tamam", des(plane(102)), org(plane(100)), des(plane(102)),
                    {"fillDone": 2 * A, "fillLeft": 0, "fillOver": 0}))
    out.append(case("Eğik tasarım yarmadan dolguya, ölçü yarı yolda", des(plane(98, 0.2, 0)), org(plane(100)),
                    org(plane(99, 0.1, 0))))
    out.append(case("Eğik arazi, düz tasarım, ölçü dalgalı", des(plane(101)), org(plane(99, 0.0, 0.15)),
                    org(bumpy(1, 100.5, 1.0))))
    out.append(case("Dalgalı arazi, düz tasarım, ölçü dalgalı", des(plane(100)), org(bumpy(2, 100, 2.0)),
                    org(bumpy(3, 100, 1.5))))
    out.append(case("Ölçü ilk araziyle aynı: hiçbir şey yapılmamış", des(plane(100)), org(bumpy(4, 100, 2.0)),
                    org(bumpy(4, 100, 2.0))))
    out.append(case("Ölçü yalnız yarısını örtüyor", des(plane(98)), org(plane(100)), grid(2, 3, 15, plane(99), -5, -5),
                    {"cutDone": 200.0, "cutLeft": 200.0}))
    out.append(case("Farklı ızgaralar: ölçü 7 m, arazi 10 m", des(plane(99.5)), org(bumpy(5, 100, 1.0)),
                    grid(5, 5, 7, bumpy(6, 99.8, 0.8), -4, -4)))
    out.append(case("Ölçü tasarımın içinden geçiyor: yer yer fazla", des(plane(99)), org(plane(101)),
                    org(plane(98, 0.1, 0.0))))
    out.append(case("Üç yüzey birbirini kesiyor", des(plane(99, 0.1, 0.05)), org(plane(100, -0.05, 0.1)),
                    org(plane(99.8, 0.02, -0.03))))
    out.append(case("Tek üçgen tasarım", [[(2, 2, 98), (18, 2, 98.5), (2, 18, 97.5)]], org(plane(100)),
                    org(bumpy(7, 99, 1.0))))
    out.append(case("Tasarım arazinin dışına taşıyor", grid(3, 3, 15, plane(98)), org(plane(100)), org(plane(99))))
    out.append(case("Ötelenmiş koordinatlar", grid(3, 3, 10, plane(98), 5000, 4000),
                    grid(4, 4, 10, plane(100), 4995, 3995), grid(4, 4, 10, bumpy(8, 99, 1.0), 4995, 3995)))
    out.append(case("Ters dönük üçgenler", [[(0, 0, 98), (20, 20, 98), (20, 0, 98)], [(0, 0, 98), (0, 20, 98), (20, 20, 98)]],
                    org(plane(100)), org(plane(99)), {"cutDone": A, "cutLeft": A}))
    return out


def main():
    data = {"cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what scripts/fixtures/road_volume_progress_cases.py writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        e = c["expected"]
        print(f"  {c['name'][:46]:46} " + " ".join(f"{k} {e[k]:.1f}" for k in KEYS + ("uncovered",)))


if __name__ == "__main__":
    main()
