#!/usr/bin/env python3
"""Independent reference of the field book's reduction (docs/adr/0169 §3; step 1).

Writes fixtures/field/v1/reduce.json from the rules alone, no KentOS code: an instrument's observations at a station
reduced as a Turkish field book is (karne indirgemesi). Every number is worked out with mpmath to 50 digits from the
float64 inputs, and only the answers are rounded to float64.

The rules:

1. Angles are in the book's unit, a full turn of 400 (grad, gon) or 360 (deg); a horizontal reading runs clockwise; the
   zenith angle is 0 straight up. Distances are metres.
2. A face: zenith in (0, half a turn) is face I, in (half, full) face II. A zenith of 0, half or a full turn, or out of
   (0, full), is no observation: its line says so. An observation without a zenith (a direction only, as an
   orientation's back sight) is a single direction, neither paired nor reduced to a distance.
3. Pairs: at a station, a face I observation and a face II observation of the same target pair up in order, the first
   unpaired of each; an observation left alone is single.
4. A pair: the face II reading turned by half a turn, d = (I − II′) wrapped into (−half, half], the mean reading
   I − d/2 in [0, full); the index error i = (V_I + V_II − full) / 2 and the mean zenith V_I − i; the slope distance
   the mean of the two (and their difference); the target height face I's.
5. A single face II: its reading turned by half a turn, its zenith full − V.
6. The horizontal distance S·sin(Z); the height difference station mark → target mark
   S·cos(Z) + (1 − k)·D² / (2R) + i_h − t_h, with R = 6 371 000 m and the book's k (refraction); an observation with no
   slope distance has neither.
7. The rows come in the order of each pair's (or single's) first observation.
8. Each observation's face is said: 1 (face I), 2 (face II), 0 (a direction only), none (no observation).
9. Tolerances (the project's, radians and metres; each may be absent) are checked on a pair: |d| against the faces'
   horizontal tolerance, |i| against the index error's, |S_I − S_II| against the faces' slope tolerance, each turned
   into the book's unit (an angle × full / 2π); a value above its tolerance is said (over), one equal is not.
10. Kutupsal alım is filled from a reduction with a back sight (one of its rows): its target and reading are the
   orientation; every other row with a slope distance and a zenith is a shot (its name, reading, slope distance, zenith
   and target height); a row without them is left out, named. The angles go into the project's unit (gon to degrees
   × 9/10, degrees to gon × 10/9).
"""

import argparse
import json
import sys
from pathlib import Path

import mpmath

mpmath.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "reduce.json"
R = mpmath.mpf(6371000)
FULL = {"grad": 400, "deg": 360}


def mp(x):
    return mpmath.mpf(x) if x is not None else None


def wrap(v, full):
    """Into (−half, half]."""
    half = mpmath.mpf(full) / 2
    m = mpmath.fmod(v, full)
    if m <= -half:
        m += full
    elif m > half:
        m -= full
    return m


def positive(v, full):
    m = mpmath.fmod(v, full)
    return m + full if m < 0 else m


def reduce(setup, unit, k, tolerances=None):
    full = mpmath.mpf(FULL[unit])
    half = full / 2
    rad = 2 * mpmath.pi / full
    problems = []
    obs = []
    faces = []
    for i, o in enumerate(setup["observations"]):
        z = mp(o.get("zenith"))
        if z is None:
            obs.append((i, o, 0))
            faces.append(0)
            continue
        if not (0 < z < full) or z == half:
            problems.append({"observation": i, "problem": "zenith"})
            faces.append(None)
            continue
        obs.append((i, o, 1 if z < half else 2))
        faces.append(1 if z < half else 2)
    # The tolerances in the book's unit (angles) and metres.
    tol = {key: mp(v) / rad if key != "faceSlope" else mp(v) for key, v in (tolerances or {}).items()}
    # Pairs in order: the first unpaired face I with the first unpaired face II of the same target.
    used = set()
    rows = []
    for at, (i, o, face) in enumerate(obs):
        if i in used:
            continue
        mate = None if face == 0 else next((j for j, q, f in obs[at + 1 :] if j not in used and q["target"] == o["target"] and f not in (0, face)), None)
        used.add(i)
        if mate is not None:
            used.add(mate)
            q = setup["observations"][mate]
            first, second = (o, q) if face == 1 else (q, o)
            one = (i, mate) if face == 1 else (mate, i)
            hz1, hz2 = mp(first["hz"]), positive(mp(second["hz"]) - half, full)
            d = wrap(hz1 - hz2, full)
            hz = positive(hz1 - d / 2, full)
            index = (mp(first["zenith"]) + mp(second["zenith"]) - full) / 2
            zen = mp(first["zenith"]) - index
            s1, s2 = mp(first.get("slope")), mp(second.get("slope"))
            slope = (s1 + s2) / 2 if s1 is not None and s2 is not None else (s1 if s1 is not None else s2)
            slope_diff = s1 - s2 if s1 is not None and s2 is not None else None
            th = first.get("targetHeight", second.get("targetHeight"))
            over = [key for key, v in (("faceHz", d), ("index", index), ("faceSlope", slope_diff)) if key in tol and v is not None and abs(v) > tol[key]]
            row = {"target": o["target"], "faces": 2, "observations": list(one), "hz": hz, "zenith": zen, "slope": slope, "hzDiff": d, "index": index, "slopeDiff": slope_diff, "targetHeight": th, "over": over}
        else:
            hz = mp(o["hz"]) if face != 2 else positive(mp(o["hz"]) - half, full)
            zen = None if face == 0 else mp(o["zenith"]) if face == 1 else full - mp(o["zenith"])
            row = {"target": o["target"], "faces": 1, "observations": [i], "hz": hz, "zenith": zen, "slope": mp(o.get("slope")), "hzDiff": None, "index": None, "slopeDiff": None, "targetHeight": o.get("targetHeight"), "over": []}
        if row["slope"] is not None and row["zenith"] is not None:
            s, zr = row["slope"], row["zenith"] * rad
            hd = s * mpmath.sin(zr)
            ih = mp(setup.get("instrumentHeight") or 0)
            th = mp(row["targetHeight"] or 0)
            row["horizontal"] = hd
            row["dh"] = s * mpmath.cos(zr) + (1 - mp(k)) * hd * hd / (2 * R) + ih - th
        else:
            row["horizontal"] = None
            row["dh"] = None
        rows.append(row)
    out = []
    for r in rows:
        out.append({k: (float(v) if isinstance(v, mpmath.mpf) else v) for k, v in r.items()})
    polar = {to: transfer(rows, 0, unit, to) for to in ("grad", "deg")} if rows else None
    return {"rows": out, "problems": problems, "faces": faces, "polar": polar}


def transfer(rows, back, frm, to):
    """Kutupsal alım's fields from the rows with `back` as the back sight, the angles in `to`."""
    conv = mpmath.mpf(1) if frm == to else (mpmath.mpf(9) / 10 if frm == "grad" else mpmath.mpf(10) / 9)
    b = rows[back]
    shots, left = [], []
    for i, r in enumerate(rows):
        if i == back:
            continue
        if r["slope"] is None or r["zenith"] is None:
            left.append(r["target"])
            continue
        shot = {"name": r["target"], "reading": float(r["hz"] * conv), "slope": float(r["slope"]), "zenith": float(r["zenith"] * conv)}
        if r["targetHeight"] is not None:
            shot["targetHeight"] = float(r["targetHeight"])
        shots.append(shot)
    return {"back": b["target"], "backReading": float(b["hz"] * conv), "shots": shots, "left": left}


def obs(target, hz, zenith, slope=None, th=None):
    o = {"target": target, "hz": hz, "zenith": zenith}
    if slope is not None:
        o["slope"] = slope
    if th is not None:
        o["targetHeight"] = th
    return o


def cases():
    return [
        ("tek durum, gon", "grad", 0.13, {"station": "P1", "instrumentHeight": 1.552, "observations": [obs("P2", 0.0, 99.8765, 245.678, 1.7), obs("101", 87.4321, 101.2345, 63.214, 1.7)]}),
        ("iki durum, gon", "grad", 0.13, {"station": "P1", "instrumentHeight": 1.552, "observations": [
            obs("P2", 0.0012, 99.8765, 245.678, 1.7), obs("101", 87.4321, 101.2345, 63.214, 1.7),
            obs("101", 287.4337, 298.7671, 63.216, 1.7), obs("P2", 200.0026, 300.1251, 245.676, 1.7)]}),
        ("iki durum, sıfırdan geçen okuma", "grad", 0.13, {"station": "S", "instrumentHeight": 1.5, "observations": [
            obs("A", 399.9990, 100.0040, 120.0, 1.3), obs("A", 199.9996, 299.9962, 120.002, 1.3)]}),
        ("derece, iki durum", "deg", 0.13, {"station": "S", "instrumentHeight": 1.45, "observations": [
            obs("A", 45.1234, 89.5, 512.345, 2.0), obs("A", 225.1246, 270.502, 512.349, 2.0)]}),
        ("yalnız II. durum", "grad", 0.13, {"station": "S", "observations": [obs("B", 250.0, 301.0, 88.8)]}),
        ("refraksiyonsuz (k = 1)", "grad", 1.0, {"station": "S", "instrumentHeight": 1.6, "observations": [obs("C", 10.0, 95.0, 900.0, 1.6)]}),
        ("uzunluksuz gözlem", "grad", 0.13, {"station": "S", "observations": [obs("D", 123.4567, 100.0)]}),
        ("başucu açısız doğrultu (geri bakış)", "grad", 0.13, {"station": "S", "observations": [{"target": "BS", "hz": 0.0}, {"target": "BS", "hz": 200.0006, "slope": 50.0}]}),
        ("geçersiz başucu açıları", "grad", 0.13, {"station": "S", "observations": [obs("E", 1.0, 0.0, 10.0), obs("F", 2.0, 200.0, 10.0), obs("G", 3.0, 99.0, 10.0)]}),
        ("iki set: her çift ayrı satır", "grad", 0.13, {"station": "S", "instrumentHeight": 1.5, "observations": [
            obs("A", 10.0000, 99.0, 100.0, 1.5), obs("A", 210.0010, 301.0004, 100.002, 1.5),
            obs("A", 60.0000, 99.0002, 100.001, 1.5), obs("A", 259.9994, 300.9996, 100.001, 1.5)]}),
        ("toleranslar: A'nın yatay farkı ve uzunluğu aşar, indeksi aşmaz; B aşmaz (gon)", "grad", 0.13, {"station": "S", "instrumentHeight": 1.5, "observations": [
            obs("A", 10.0000, 99.0, 100.0, 1.5), obs("A", 210.0030, 301.0004, 100.006, 1.5),
            obs("B", 60.0000, 99.0002, 100.001, 1.5), obs("B", 259.9994, 300.9996, 100.001, 1.5)]},
         {"faceHz": 20 * 3.141592653589793 / 2000000.0, "index": 10 * 3.141592653589793 / 2000000.0, "faceSlope": 0.005}),
        ("toleranslar, derece: yalnız uzunluk", "deg", 0.13, {"station": "S", "instrumentHeight": 1.45, "observations": [
            obs("A", 45.1234, 89.5, 512.345, 2.0), obs("A", 225.1246, 270.502, 512.352, 2.0)]},
         {"faceSlope": 0.004}),
    ]


def build():
    out = []
    for name, unit, k, setup, *more in cases():
        tolerances = more[0] if more else None
        case = {"name": name, "unit": unit, "k": k, "setup": setup}
        if tolerances:
            case["tolerances"] = tolerances
        case["expect"] = reduce(setup, unit, k, tolerances)
        out.append(case)
    return {
        "format": "kentos.field-reduce",
        "version": 1,
        "source": "scripts/fixtures/field_reduce_cases.py (docs/adr/0169 §3)",
        "earthRadius": 6371000.0,
        "tolerance": {"metres": 1e-9, "angle": 1e-11},
        "cases": out,
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
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
