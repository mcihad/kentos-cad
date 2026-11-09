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

No KentOS code.

    python3 scripts/fixtures/road_terrain_compare_section_cases.py [--check]
"""

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
        else:
            cut += length * (c0 + c1) / 2
            fill += length * (f0 + f1) / 2
    return rows, cut, fill


def answer(inp):
    plain = {k: v for k, v in inp.items() if k not in ("cutFactor", "fillFactor")}
    out, warns = cmp.answer(plain)
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


def map_setup():
    """The drawing as a Harita project: its surfaces alone (a map project keeps no road design)."""
    s = cmp.setup()
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
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.compare", "commandVersion": 1,
           "title": "Yüzeyleri karşılaştır: kesit yöntemi (ADR 4340)",
           "note": "scripts/fixtures/road_terrain_compare_section_cases.py'den: kesit doğrusu iki kafesin çizgilerinde "
                   "kırılır, aralarında fark doğrusal, kesin integral; ortalama alan ya da prizmoidal; kesin TIN "
                   "road_terrain_compare_cases.py'nin. KentOS kodu yok.",
           "setup": cmp.setup(), "cases": found}
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
