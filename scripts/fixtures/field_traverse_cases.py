#!/usr/bin/env python3
"""Independent reference of a field book's traverse (docs/adr/0169 §3; step 4a).

Writes fixtures/field/v1/traverse.json from the rules alone, no KentOS code: a field book's stations, each reduced as
field_reduce_cases.py reduces it (mpmath, 50 digits), turned into Poligon hesabı's angles and legs.

The rules:

1. The traverse runs through the book's stations in order, ST1 … STn (n ≥ 2). ST1 is oriented on the back sight given
   (a target at ST1), STn on the fore sight given (a target at STn) or on none.
2. At each station the back target is the previous station (ST1: the back sight) and the fore target the next one
   (STn: the fore sight); a target is the station's first reduced row of that name. The angle (kırılma açısı) is the
   fore row's reading less the back row's, in [0, a full turn); without either row there is none, and the missing row
   is named (station, target).
3. A leg STi → STi+1: forward its horizontal distance at STi to STi+1, backward at STi+1 to STi (a row that is a
   direction only has none); with both the mean and the difference forward − backward, with one that one alone; a leg
   with neither is named (STi, STi+1) as missing.
4. The angles go into the project's unit (gon to degrees × 9/10, degrees to gon × 10/9).
5. A leg's difference above the project's two-way tolerance (metres) is marked (over); one equal is not.
6. Poligon hesabı's misclosures against the project's tolerances: |fβ| (in the unit) above the angular tolerance
   (radians, turned into the unit: × full / 2π), fs above the linear one (metres); none without a misclosure or a
   tolerance.
"""

import argparse
import json
import sys
from pathlib import Path

import mpmath

sys.path.insert(0, str(Path(__file__).resolve().parent))
import field_reduce_cases as reduction  # noqa: E402  (the reduction's own reference)

mpmath.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "traverse.json"
FULL = {"grad": 400, "deg": 360}


def rows_mp(setup, unit, k):
    """The station's reduced rows with mpmath values (field_reduce_cases.reduce rounds them at the end)."""
    out = reduction.reduce(setup, unit, k)
    return out["rows"]


def first(rows, target):
    return next((r for r in rows if r["target"] == target), None)


def traverse(book, unit, k, back, fore, to, two_way=None):
    full = mpmath.mpf(FULL[unit])
    conv = mpmath.mpf(1) if unit == to else (mpmath.mpf(9) / 10 if unit == "grad" else mpmath.mpf(10) / 9)
    names = [s["station"] for s in book]
    reduced = [rows_mp(s, unit, k) for s in book]
    angles, legs, missing = [], [], []
    for i, rows in enumerate(reduced):
        b = back if i == 0 else names[i - 1]
        f = fore if i == len(book) - 1 else names[i + 1]
        if f is None:
            angles.append(None)
            continue
        rb, rf = first(rows, b), first(rows, f)
        for target, row in ((b, rb), (f, rf)):
            if row is None:
                missing.append({"station": names[i], "target": target})
        if rb is None or rf is None:
            angles.append(None)
            continue
        a = mpmath.fmod(mpmath.mpf(rf["hz"]) - mpmath.mpf(rb["hz"]), full)
        if a < 0:
            a += full
        angles.append(float(a * conv))
    for i in range(len(book) - 1):
        fw = first(reduced[i], names[i + 1])
        bw = first(reduced[i + 1], names[i])
        forward = None if fw is None else fw.get("horizontal")
        backward = None if bw is None else bw.get("horizontal")
        if forward is None and backward is None:
            missing.append({"station": names[i], "target": names[i + 1]})
        leg = {"from": names[i], "to": names[i + 1]}
        if forward is not None:
            leg["forward"] = forward
        if backward is not None:
            leg["backward"] = backward
        over = False
        if forward is not None and backward is not None:
            diff = mpmath.mpf(forward) - mpmath.mpf(backward)
            leg["mean"] = float((mpmath.mpf(forward) + mpmath.mpf(backward)) / 2)
            leg["diff"] = float(diff)
            over = two_way is not None and abs(diff) > mpmath.mpf(two_way)
        elif forward is not None or backward is not None:
            leg["mean"] = forward if forward is not None else backward
        leg["over"] = bool(over)
        legs.append(leg)
    # Each missing row is said once, in the order found.
    seen, said = set(), []
    for m in missing:
        key = (m["station"], m["target"])
        if key not in seen:
            seen.add(key)
            said.append(m)
    return {"stations": names, "angles": angles, "legs": legs, "missing": said}


def obs(target, hz, zenith=None, slope=None, th=None):
    o = {"target": target, "hz": hz}
    if zenith is not None:
        o["zenith"] = zenith
    if slope is not None:
        o["slope"] = slope
    if th is not None:
        o["targetHeight"] = th
    return o


def cases():
    three = [
        {"station": "A", "instrumentHeight": 1.5, "observations": [obs("K", 0.0, 100.0, 300.0, 1.5), obs("B", 150.0, 100.002, 120.004, 1.5),
                                                                   obs("B", 350.0006, 299.998, 120.006, 1.5), obs("K", 200.0004, 300.0, 300.002, 1.5)]},
        {"station": "B", "instrumentHeight": 1.5, "observations": [obs("A", 10.0, 99.998, 120.002, 1.5), obs("C", 190.1234, 100.0, 95.5, 1.5)]},
        {"station": "C", "instrumentHeight": 1.5, "observations": [obs("B", 0.0, 100.0, 95.497, 1.5), obs("L", 222.2222, 100.0, 200.0, 1.5)]},
    ]
    return [
        ("üç istasyon, bağlı (gon)", three, "grad", 0.13, "K", "L", "grad"),
        ("üç istasyon, iki yönden fark toleransı 2,5 mm", three, "grad", 0.13, "K", "L", "grad", 0.0025),
        ("aynısı, tolerans 5 mm", three, "grad", 0.13, "K", "L", "grad", 0.005),
        ("aynısı, derecelere", three, "grad", 0.13, "K", "L", "deg"),
        ("bitişte yöneltme yok", three, "grad", 0.13, "K", None, "grad"),
        ("geri bakış istasyonda yok, bir kenar tek yönden", [
            {"station": "A", "observations": [obs("B", 50.0, 100.0, 80.0)]},
            {"station": "B", "observations": [obs("A", 0.0), obs("C", 123.0, 100.0, 60.0)]},
            {"station": "C", "observations": [obs("B", 10.0, 100.0, 60.004)]},
        ], "grad", 0.13, "X", None, "grad"),
        ("ölçülmemiş kenar", [
            {"station": "A", "observations": [obs("K", 0.0), obs("B", 100.0)]},
            {"station": "B", "observations": [obs("A", 0.0), obs("K", 200.0)]},
        ], "grad", 0.13, "K", "K", "grad"),
        ("derece, iki istasyon", [
            {"station": "S1", "instrumentHeight": 1.45, "observations": [obs("R", 0.0, 90.0, 150.0, 2.0), obs("S2", 120.5, 89.5, 512.345, 2.0)]},
            {"station": "S2", "instrumentHeight": 1.5, "observations": [obs("S1", 300.25, 90.5, 512.349, 2.0), obs("T", 45.125, 90.0, 80.0, 2.0)]},
        ], "deg", 0.13, "R", "T", "deg"),
    ]


def build():
    out = []
    for name, book, unit, k, back, fore, to, *more in cases():
        two_way = more[0] if more else None
        case = {"name": name, "unit": unit, "k": k, "back": back, "fore": fore, "to": to}
        if two_way is not None:
            case["twoWay"] = two_way
        case["book"] = book
        case["expect"] = traverse(book, unit, k, back, fore, to, two_way)
        out.append(case)
    return {"format": "kentos.field-traverse", "version": 1, "source": "scripts/fixtures/field_traverse_cases.py (docs/adr/0169 §3)",
            "tolerance": {"metres": 1e-9, "angle": 1e-11}, "cases": out, "closures": closures()}


def closures():
    """Poligon hesabı's misclosures against the tolerances (rule 6)."""
    out = []
    for unit, fb, fs, angle, coord in [
        ("grad", 0.0025, 0.012, 20 * 3.141592653589793 / 2000000.0, 0.01),
        ("grad", -0.0015, 0.008, 20 * 3.141592653589793 / 2000000.0, 0.01),
        ("deg", 0.001, 0.03, 5 * 3.141592653589793 / 648000.0, 0.05),
        ("deg", -0.002, 0.06, 5 * 3.141592653589793 / 648000.0, 0.05),
        ("grad", None, 0.02, 20 * 3.141592653589793 / 2000000.0, None),
        ("grad", 0.003, None, None, 0.01),
    ]:
        rad = 2 * mpmath.pi / FULL[unit]
        c = {"unit": unit}
        if fb is not None:
            c["angleMisclosure"] = fb
        if fs is not None:
            c["linearMisclosure"] = fs
        if angle is not None:
            c["angle"] = angle
        if coord is not None:
            c["coord"] = coord
        if fb is not None and angle is not None:
            c["angleOver"] = bool(abs(mpmath.mpf(fb)) > mpmath.mpf(angle) / rad)
        if fs is not None and coord is not None:
            c["coordOver"] = bool(mpmath.mpf(fs) > mpmath.mpf(coord))
        out.append(c)
    return out


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
