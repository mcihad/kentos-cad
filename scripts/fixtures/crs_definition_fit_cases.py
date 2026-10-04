#!/usr/bin/env python3
"""Independent reference of Özel koordinat sistemi's Ortak noktalardan hesapla (docs/adr/0168 §1, §6; step 4d).

Writes fixtures/crs/v1/definition-fit.json from the rules alone, no KentOS code: a local system's plane (this system →
its base) worked out from points known in both, by least squares, as Vektör oturtma solves its pairs
(scripts/fixtures/fit_cases.py: exact fractions of the float64 inputs, the pairs' centred frames), then written as the
window's plane: a similarity's shift east and north, turn (degrees, counter-clockwise) and scale, or an affine's six
coefficients (base east = a·east + b·north + c, base north = d·east + e·north + f).

- A row is this system's east and north and the base's east and north, as typed, and Kullan ("0" leaves it out). An
  empty row is skipped; a row with all four read as the Hesap windows read numbers (trimmed, the first comma a point)
  is a pair; any other row is left out of the solution, as Vektör oturtma leaves a row being typed, and named
  (“Satır 3 hesaba katılmadı …”).
- A similarity needs 2 used pairs, an affine 3; this system's points all in one place, or (affine) on one line, give
  no plane.
- The plane, exactly from the centred solution (`from` the used sources' centre, `to` their targets'): a similarity
  X̄ = a·x̄ − b·ȳ, Ȳ = b·x̄ + a·ȳ is scale √(a² + b²), turn atan2(b, a) in degrees, east to.x − (a·from.x − b·from.y),
  north to.y − (b·from.x + a·from.y); an affine X̄ = a·x̄ + c·ȳ, Ȳ = b·x̄ + d·ȳ (the core's order) is a, c,
  to.x − a·from.x − c·from.y, b, d, to.y − b·from.x − d·from.y. Square roots and the turn to 50 digits (mpmath).
- Every pair's residual (transformed source less target: east, north, length) and m0 = √([vv] / (2n − u)), u 4 or 6;
  none without redundancy.

The desktop (kentos_project::definition_form::fit_plane) and the web (model/definitionForm.ts `fitPlane`) solve every
case with the shared core and must give these within the file's tolerances, or the same words.
"""

import argparse
import json
import math
import re
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fit_cases import fit  # noqa: E402  (the independent least-squares reference)

mpmath.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "crs" / "v1" / "definition-fit.json"
NUMBER = re.compile(r"^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$", re.IGNORECASE)
KINDS = {"similarity": ("helmert", "Benzerlik"), "affine": ("affine", "Afin")}

TEXTS = {
    "skipped": "Satır {rows} hesaba katılmadı: dört değer de sayı olmalı (bu sistemde ve tabanda sağa ve yukarı).",
    "tooFew": "{kind} için en az {need} kullanılan ortak nokta gerekir; şimdi {n}.",
    "coincident": "Bu sistemdeki noktaların hepsi aynı yerde; düzlem bulunamaz.",
    "collinear": "Bu sistemdeki noktalar bir doğru üstünde; afin bulunamaz. Doğrunun dışında bir nokta ekleyin.",
}


def number(text):
    t = text.strip().replace(",", ".", 1)
    return float(t) if t and NUMBER.match(t) else None


def mp(x):
    return mpmath.mpf(x.numerator) / mpmath.mpf(x.denominator)


def solve(rows, plane):
    pairs, at, skipped = [], [], []
    for i, r in enumerate(rows):
        values = r[:4]
        if all(not v.strip() for v in values):
            continue
        read = [number(v) for v in values]
        if any(v is None for v in read):
            skipped.append(i)
            continue
        pairs.append({"source": {"x": read[0], "y": read[1]}, "target": {"x": read[2], "y": read[3]}, "used": r[4] != "0"})
        at.append(i)
    kind, word = KINDS[plane]
    got = fit(pairs, kind)
    if "error" in got:
        if got["error"] == "too_few":
            n = sum(1 for p in pairs if p["used"])
            return {"problem": TEXTS["tooFew"].format(kind=word, need=got["need"], n=n)}
        return {"problem": TEXTS[got["error"]]}
    # The centred solution again, exactly, for the plane's numbers.
    used = [p for p in pairs if p["used"]]
    n = len(used)
    x0 = sum(F(p["source"]["x"]) for p in used) / n
    y0 = sum(F(p["source"]["y"]) for p in used) / n
    X0 = sum(F(p["target"]["x"]) for p in used) / n
    Y0 = sum(F(p["target"]["y"]) for p in used) / n
    src = [(F(p["source"]["x"]) - x0, F(p["source"]["y"]) - y0) for p in used]
    dst = [(F(p["target"]["x"]) - X0, F(p["target"]["y"]) - Y0) for p in used]
    if plane == "similarity":
        s = sum(x * x + y * y for x, y in src)
        a = sum(x * X + y * Y for (x, y), (X, Y) in zip(src, dst)) / s
        b = sum(x * Y - y * X for (x, y), (X, Y) in zip(src, dst)) / s
        out_plane = {
            "kind": "similarity",
            "east": float(X0 - (a * x0 - b * y0)),
            "north": float(Y0 - (b * x0 + a * y0)),
            "rotation": float(mpmath.degrees(mpmath.atan2(mp(b), mp(a)))),
            "scale": float(mpmath.sqrt(mp(a * a + b * b))),
        }
    else:
        sxx = sum(x * x for x, _ in src)
        syy = sum(y * y for _, y in src)
        sxy = sum(x * y for x, y in src)
        det = sxx * syy - sxy * sxy
        sxX = sum(x * X for (x, _), (X, _) in zip(src, dst))
        syX = sum(y * X for (_, y), (X, _) in zip(src, dst))
        sxY = sum(x * Y for (x, _), (_, Y) in zip(src, dst))
        syY = sum(y * Y for (_, y), (_, Y) in zip(src, dst))
        a = (sxX * syy - syX * sxy) / det
        c = (syX * sxx - sxX * sxy) / det
        b = (sxY * syy - syY * sxy) / det
        d = (syY * sxx - sxY * sxy) / det
        out_plane = {
            "kind": "affine",
            "a": float(a),
            "b": float(c),
            "c": float(X0 - a * x0 - c * y0),
            "d": float(b),
            "e": float(d),
            "f": float(Y0 - b * x0 - d * y0),
        }
    return {"plane": out_plane, "rows": at, "skipped": skipped, "residuals": got["residuals"], "m0": got["m0"]}


def true_similarity(east, north, deg, scale):
    th = math.radians(deg)
    c, s = math.cos(th), math.sin(th)

    def apply(x, y):
        return (scale * (c * x - s * y) + east, scale * (s * x + c * y) + north)

    return apply


def rows_of(points, transform, noise=(), used=()):
    """Rows of site points carried by `transform`, the base written to the millimetre, with noise (m) added."""
    out = []
    for i, (x, y) in enumerate(points):
        X, Y = transform(x, y)
        dx, dy = noise[i] if i < len(noise) else (0.0, 0.0)
        out.append([f"{x:.3f}", f"{y:.3f}", f"{X + dx:.3f}", f"{Y + dy:.3f}", "0" if i in used else "1"])
    return out


def cases():
    site = [(1000.0, 2000.0), (1250.5, 2010.25), (1240.0, 2300.75), (990.125, 2280.5), (1120.0, 2150.0)]
    sim = true_similarity(412_000.123, 4_521_000.456, 0.45, 1.000_012_3)
    noise = [(0.004, -0.003), (-0.002, 0.005), (0.001, -0.002), (-0.003, 0.0), (0.002, 0.001)]

    def affine(x, y):
        return (1.000_021_5 * x - 0.000_387_1 * y + 412_000.0, 0.000_412_9 * x + 0.999_983_2 * y + 4_521_000.0)

    out = [
        ("benzerlik: iki nokta tam geçer", "similarity", rows_of(site[:2], sim)),
        ("benzerlik: beş nokta, gürültülü", "similarity", rows_of(site, sim, noise)),
        ("benzerlik: biri kullanılmıyor", "similarity", rows_of(site, sim, noise, used={2})),
        ("afin: üç nokta tam geçer", "affine", rows_of(site[:3], affine)),
        ("afin: beş nokta, gürültülü", "affine", rows_of(site, affine, noise)),
        ("boş satırlar atlanır, virgül nokta sayılır", "similarity",
         [["", "", "", "", "1"]] + [[v.replace(".", ",", 1) if j < 4 else v for j, v in enumerate(r)] for r in rows_of(site[:3], sim, noise)] + [["  ", "", "", "", "1"]]),
        ("benzerlik: tek nokta yetmez", "similarity", rows_of(site[:1], sim)),
        ("afin: iki nokta yetmez", "affine", rows_of(site[:2], affine)),
        ("afin: biri kullanılmayınca iki kalır", "affine", rows_of(site[:3], affine, used={1})),
        ("benzerlik: noktalar aynı yerde", "similarity", [["1000", "2000", "412000", "4521000", "1"], ["1000", "2000", "412001", "4521001", "1"]]),
        ("afin: noktalar bir doğru üstünde", "affine", [["0", "0", "1", "1", "1"], ["10", "10", "11", "11", "1"], ["20", "20", "21", "21", "1"]]),
        ("eksik satır", "similarity", rows_of(site[:2], sim) + [["1100", "", "412100", "4521100", "1"]]),
        ("sayı olmayan değer", "affine", rows_of(site[:3], affine) + [["1100", "2100", "412100,5,1", "4521100", "1"]]),
    ]
    return out


def build():
    built = []
    for name, plane, rows in cases():
        got = solve(rows, plane)
        built.append({"name": name, "plane": plane, "typed": rows, **({"expect": got} if "problem" not in got else got)})
    return {
        "format": "kentos.crs-definition-fit",
        "version": 1,
        "source": "scripts/fixtures/crs_definition_fit_cases.py (docs/adr/0168 §1, §6; fit_cases.py's solution)",
        "tolerance": {"metres": 1e-9, "relative": 1e-12, "degrees": 1e-12},
        "texts": TEXTS,
        "cases": built,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = parser.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
