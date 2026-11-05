#!/usr/bin/env python3
"""The shared cases of a volume's ledger (docs/adr/0949, plan/road.md Y43; ROAD-VOL-16, ROAD-VOL-18), in exact
fractions, without KentOS code: groups by the stretches' middles, their sums; the densest-first concentration curve, its
Gini index 2·∫C − 1, the length shares holding 1/2 and 4/5 of the volume; each stretch's length and mean area over the
medians (type 7); the rounded rows against the rounded total (half away from zero, on the figures' exact values).

    python3 scripts/fixtures/road_volume_ledger_cases.py [--check]
"""

import json
import sys
from decimal import ROUND_HALF_UP, Decimal
from fractions import Fraction as F
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "volume_ledger.json"
NONE = 1e-6  # a stretch's volume up to 1 cm³ is none (the ADR: sections leave 1e-29 m³ of fill where there is none)


def kept(v):
    return v if v > NONE else 0.0


def median(v):
    v = sorted(v)
    h = F(len(v) - 1, 2)
    lo = int(h)
    hi = min(lo + 1, len(v) - 1)
    return v[lo] + (h - lo) * (v[hi] - v[lo])


def group(stretches, ranges):
    n = len(ranges)
    members = [[] for _ in ranges]
    for i, s in enumerate(stretches):
        m = (F(s["from"]) + F(s["to"])) / 2
        for k, (a, b) in enumerate(ranges):
            if F(a) <= m and (m < F(b) or (k == n - 1 and m <= F(b))):
                members[k].append(i)
                break
    cols = len(stretches[0]["values"])
    out = []
    for m in members:
        if not m:
            continue
        vals = [sum((F(stretches[i]["values"][c]) for i in m if stretches[i]["values"][c] is not None), F(0))
                for c in range(cols)]
        out.append({"from": min(stretches[i]["from"] for i in m), "to": max(stretches[i]["to"] for i in m),
                    "members": m, "values": [float(v) for v in vals]})
    return out


def steps(first, last, step):
    import math
    out = []
    a = math.floor(first / step) * step
    while a < last or not out:
        out.append([a, a + step])
        a += step
    return out


def concentration(lengths, volumes):
    volumes = [kept(v) for v in volumes]
    L, V = sum(map(F, lengths)), sum(map(F, volumes))
    if not (L > 0 and V > 0):
        return None
    # Densest first, densities equal to 9 significant digits in the stretches' order (as KentOS).
    order = sorted((i for i in range(len(lengths)) if lengths[i] > 0),
                   key=lambda i: (-float(f"{volumes[i] / lengths[i]:.8e}"), i))
    curve = [(F(0), F(0))]
    cl = cv = F(0)
    for i in order:
        cl += F(lengths[i])
        cv += F(volumes[i])
        curve.append((cl / L, cv / V))
    area = sum(((b[0] - a[0]) * (a[1] + b[1]) / 2 for a, b in zip(curve, curve[1:])), F(0))

    def reach(t):
        for a, b in zip(curve, curve[1:]):
            if b[1] >= t:
                return a[0] + (b[0] - a[0]) * (t - a[1]) / (b[1] - a[1]) if b[1] > a[1] else a[0]
        return F(1)
    return {"gini": float(2 * area - 1), "half": float(reach(F(1, 2))), "most": float(reach(F(4, 5))),
            "curve": [[float(x), float(y)] for x, y in curve]}


def reasons(lengths, volumes):
    volumes = [kept(v) for v in volumes]
    w = [i for i in range(len(lengths)) if lengths[i] > 0 and volumes[i] > 0]
    ml = median([F(lengths[i]) for i in w])
    ma = median([F(volumes[i]) / F(lengths[i]) for i in w])
    return [{"length": float(F(lengths[i]) / ml), "area": float(F(volumes[i]) / F(lengths[i]) / ma)}
            if lengths[i] > 0 and volumes[i] > 0 else None for i in range(len(lengths))]


def rnd(v, d):
    return Decimal(v).quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)


def rounding(values, d):
    rows = sum((rnd(v, d) for v in values), Decimal(0))
    total = rnd(float(sum(map(F, values), F(0))), d)
    return [float(rows), float(total), float(rows - total)]


def cases():
    out = []

    def case(name, stretches, ranges=None, step=None, decimals=2):
        if step is not None:
            ranges = steps(stretches[0]["from"], stretches[-1]["to"], step)
        lengths = [s["to"] - s["from"] for s in stretches]
        cols = len(stretches[0]["values"])
        conc, why, rd = [], [], []
        for c in range(cols):
            vols = [s["values"][c] or 0.0 for s in stretches]
            conc.append(concentration(lengths, vols))
            why.append(reasons(lengths, vols) if any(v > 0 for v in vols) else None)
            rd.append(rounding([s["values"][c] for s in stretches if s["values"][c] is not None], decimals))
        out.append({"name": name, "stretches": stretches, "ranges": ranges, "step": step, "decimals": decimals,
                    "groups": group(stretches, ranges), "concentration": conc, "reasons": why, "rounding": rd})

    def st(points, cols):
        return [{"from": a, "to": b, "values": list(v)} for (a, b), v in zip(zip(points, points[1:]), cols)]

    even = st([0, 20, 40, 60, 80, 100], [(100.0, 10.0)] * 5)
    case("Düzgün dağılmış hacim", even, step=50.0)
    case("Tek büyük aralık", st([0, 20, 40, 60, 80, 100], [(5.0, 0.0), (5.0, 0.0), (900.0, 0.0), (5.0, 0.0), (5.0, 3.0)]),
         step=40.0)
    case("Farklı boylar", st([0, 10, 35, 40, 90, 120], [(120.0, 4.5), (75.5, 0.0), (300.25, 1.0), (60.0, 80.0),
                                                        (12.0, 230.0)]), step=25.0)
    case("Boşluklu aralıklar", st([0, 20, 40, 60, 80], [(10.0, 1.0), (None, None), (30.0, 3.0), (None, None)]),
         ranges=[[0, 40], [40, 80]])
    case("Bölge sınırında ortası sayılır", st([0, 30, 70, 100], [(1.5, 0.5), (2.5, 0.25), (3.5, 0.125)]),
         ranges=[[0, 45], [45, 100]])
    case("Km'si yüzden başlayan", st([137.5, 160, 200, 251.2, 300], [(40.0, 2.0), (60.0, 0.0), (45.0, 12.0),
                                                                        (8.0, 30.0)]), step=100.0)
    case("Yuvarlama farkı: tam sayıya", st([0, 10, 20, 30], [(0.4, 1.25), (0.4, 2.25), (0.4, 0.0)]), step=100.0,
         decimals=0)
    case("Yuvarlama farkı: bir basamak", st([0, 10, 20, 30], [(0.4, 1.25), (0.4, 2.25), (0.4, 0.0)]), step=100.0,
         decimals=1)
    case("Dolgusuz: yoğunlaşma yok", st([0, 50, 100], [(10.0, 0.0), (20.0, 0.0)]), step=50.0)
    case("Gürültü kadar hacim yok sayılır", st([0, 10, 20, 30, 40], [(50.0, 1e-29), (30.0, 2e-12), (0.0, 40.0),
                                                                  (20.0, 5e-7)]), step=20.0)
    case("Üç sütun, katmanlı", st([0, 25, 50, 75, 100, 125, 150],
                                 [(40.0, 0.0, 6.0), (80.0, 0.0, 6.0), (0.0, 50.0, 6.5), (0.0, 120.0, 6.5),
                                  (15.0, 15.0, 6.0), (220.0, 0.0, 5.75)]), step=50.0, decimals=3)
    return out


def main():
    data = {"cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what {Path(__file__).name} writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        g = c["concentration"][0]
        print(f"  {c['name'][:36]:36} gruplar {len(c['groups'])} gini {g and round(g['gini'], 4)} yarı {g and round(g['half'], 4)} "
              f"yuvarlama {c['rounding']}")


if __name__ == "__main__":
    main()
