#!/usr/bin/env python3
"""The exact curve lengths of fixtures/measure/v1/cases.json checked again with OpenCASCADE (docs/adr/0259 §1).

ADR 0149's lengths of circles, circular arcs, ellipses and elliptical arcs, and of KentOS's Catmull–Rom curves, are
written by scripts/fixtures/measure_cases.py with 50-digit quadrature; the shared core must meet them. Here
OpenCASCADE (tools/occt/occt_ref.cpp, its own curves and its own GCPnts_AbscissaPoint) measures the same curves
without KentOS code: a circle or arc as Geom2d_Circle trimmed, an ellipse as Geom2d_Ellipse trimmed, a Catmull–Rom
span as the cubic Bézier it is (its ends and its end derivatives from the curve's definition, ADR 0057). The two
references must agree within each case's own bounds; so the core is held to curves, never to the chords it draws.

    python3 scripts/fixtures/curves_occt.py --check
"""

import argparse
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CASES = ROOT / "fixtures/measure/v1/cases.json"
OCCT = ROOT / ".run/occt_ref"
sys.path.insert(0, str(Path(__file__).resolve().parent))


def catmull_beziers(pts, closed):
    """The centripetal Catmull–Rom curve's spans as cubic Bézier control points (Barry and Goldman's pyramid)."""
    n = len(pts)

    def at(i):
        if closed:
            return pts[i % n]
        if i < 0:
            return (2 * pts[0][0] - pts[1][0], 2 * pts[0][1] - pts[1][1])
        if i >= n:
            return (2 * pts[n - 1][0] - pts[n - 2][0], 2 * pts[n - 1][1] - pts[n - 2][1])
        return pts[i]

    out = []
    for i in range(n if closed else n - 1):
        p0, p1, p2, p3 = at(i - 1), at(i), at(i + 1), at(i + 2)

        def knot(a, b):
            return math.sqrt(math.hypot(b[0] - a[0], b[1] - a[1]))

        t0 = 0.0
        t1 = t0 + knot(p0, p1)
        t2 = t1 + knot(p1, p2)
        t3 = t2 + knot(p2, p3)

        def lerp(a, b, ta, tb, t):
            u, w = (tb - t) / (tb - ta), (t - ta) / (tb - ta)
            return (a[0] * u + b[0] * w, a[1] * u + b[1] * w)

        def dlerp(a, b, da, db, ta, tb, t):
            u, w = (tb - t) / (tb - ta), (t - ta) / (tb - ta)
            k = 1 / (tb - ta)
            return (da[0] * u + db[0] * w + (b[0] - a[0]) * k, da[1] * u + db[1] * w + (b[1] - a[1]) * k)

        def derivative(t):
            zero = (0.0, 0.0)
            a1, a2, a3 = lerp(p0, p1, t0, t1, t), lerp(p1, p2, t1, t2, t), lerp(p2, p3, t2, t3, t)
            da1, da2, da3 = dlerp(p0, p1, zero, zero, t0, t1, t), dlerp(p1, p2, zero, zero, t1, t2, t), dlerp(p2, p3, zero, zero, t2, t3, t)
            b1, b2 = lerp(a1, a2, t0, t2, t), lerp(a2, a3, t1, t3, t)
            db1, db2 = dlerp(a1, a2, da1, da2, t0, t2, t), dlerp(a2, a3, da2, da3, t1, t3, t)
            return dlerp(b1, b2, db1, db2, t1, t2, t)

        h = t2 - t1
        d1, d2 = derivative(t1), derivative(t2)
        out.append([p1, (p1[0] + d1[0] * h / 3, p1[1] + d1[1] * h / 3), (p2[0] - d2[0] * h / 3, p2[1] - d2[1] * h / 3), p2])
    return out


class Occt:
    def __init__(self):
        self.proc = subprocess.Popen([str(OCCT)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def ask(self, line):
        self.proc.stdin.write(line + "\n")
        self.proc.stdin.flush()
        answer = self.proc.stdout.readline().strip()
        assert not answer.startswith("error"), answer
        return [float(x) for x in answer.split()] if answer else []

    def length(self):
        lo, hi = self.ask("domain")
        return self.ask(f"length {lo!r} {hi!r}")[0]


def measure(occt, e):
    kind = e["kind"]
    if kind == "circle":
        occt.ask(f"circle {e['c']['x']!r} {e['c']['y']!r} {e['r']!r} 0 0")
        return occt.length()
    if kind == "arc":
        occt.ask(f"circle {e['c']['x']!r} {e['c']['y']!r} {e['r']!r} {e['a0']!r} {e['a1']!r}")
        return occt.length()
    if kind == "ellipse":
        occt.ask(f"ellipse {e['c']['x']!r} {e['c']['y']!r} {e['major']['x']!r} {e['major']['y']!r} {e['ratio']!r} {e['t0']!r} {e['t1']!r}")
        return occt.length()
    if kind == "spline":
        pts = [(p["x"], p["y"]) for p in e["pts"]]
        if len(pts) < 3:
            return math.hypot(pts[1][0] - pts[0][0], pts[1][1] - pts[0][1]) if len(pts) == 2 else 0.0
        total = 0.0
        for span in catmull_beziers(pts, e["closed"]):
            nums = " ".join(f"{x!r} {y!r} 1" for x, y in span)
            occt.ask(f"nurbs 3 4 {nums} 8 0 0 0 0 1 1 1 1")
            total += occt.length()
        return total
    return None


def main():
    argparse.ArgumentParser().add_argument("--check", action="store_true")
    if not OCCT.exists():
        print("OpenCASCADE başvurusu yok (.run/occt_ref): önce scripts/fixtures/nurbs_cases.py ya da tools/occt/README.md'deki komutla derleyin.")
        return 1
    occt = Occt()
    cases = json.loads(CASES.read_text())["cases"]
    checked, worst = 0, 0.0
    failures = []
    for c in cases:
        if c["op"] != "entityLength" or c["args"][0].get("kind") not in ("circle", "arc", "ellipse", "spline"):
            continue
        got = measure(occt, c["args"][0])
        exact = float(c["exact"])
        bound = max(c["abs"], c["rel"] * abs(exact))
        err = abs(got - exact)
        worst = max(worst, err / bound)
        if err > bound:
            failures.append(f"{c['what']}: OpenCASCADE {got!r}, kesin {exact!r} (fark {err:.3g}, sınır {bound:.3g})")
        checked += 1
    if failures:
        print("\n".join(failures))
        return 1
    print(f"{checked} eğri uzunluğu OpenCASCADE'le kesin değerlerle aynı (en büyük fark sınırın {worst:.2g} katı).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
