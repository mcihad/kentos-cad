#!/usr/bin/env python3
"""Independent reference of the survey windows' lengths between ground and grid (docs/adr/0171 §4).

Writes fixtures/geodesy/v1/ground-survey.json from PROJ and GeographicLib's C library (through pyproj) and mpmath, no
KentOS code. The cases are built backwards from known points on the grid: a line's ground length is its grid length over
its factors, the projection's scale along it (Simpson's rule on PROJ's `get_factors` at its ends and middle; a local
system on a similarity, its base's over the similarity's scale) times the height's factor R/(R + h) (R Euler's radius
at the geodesic's middle in its direction there, ground_cases.py's rule), at the project's mean ellipsoidal height h.

- Kutupsal alım: from a station oriented on a back point, the readings of the known points (their grid bearings less
  the back's, in the unit) and their ground lengths (horizontal, or slope with a zenith angle); the core, given the
  grid, must place the points where they are (within 1e-6 m) and give their grid lengths and factors.
- Aplikasyon: the known points' grid distances and their ground distances with the factors.
- Poligon hesabı: a connected traverse through known points, its angles from the grid and its legs' ground lengths;
  the core must give the points back (within 1e-6 m) with no misclosure (within 1e-6 m).

Lengths agree within 1e-6 m, factors within 1e-10 (PROJ's factors are numerical derivatives).
"""

import argparse
import json
import math
import sys
from pathlib import Path

import mpmath as mp
from pyproj import Geod

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ground_cases as ground  # noqa: E402  (the systems, the unprojection, PROJ's point scales, Euler's radius)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "ground-survey.json"

mp.mp.dps = 40
TURN = {"grad": 400.0, "deg": 360.0}


def factors(system, height, a, b):
    """The line from a to b's scale (Simpson) and height factor."""
    ea, rf = ground.ellipsoid(ground.datum_of(system))
    geod = Geod(a=ea, rf=rf)
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    k = (ground.point_scale(system, *a) + 4 * ground.point_scale(system, *m) + ground.point_scale(system, *b)) / 6
    p, q = ground.unproject(system, *a), ground.unproject(system, *b)
    _dist, lat, azi = ground.geodesic(geod, p, q)
    r = ground.euler(ea, rf, lat, azi)
    return k, float(r / (r + height))


def bearing(a, b, unit):
    """The grid bearing from a to b, clockwise from north, in the unit."""
    t = math.atan2(b[0] - a[0], b[1] - a[1])
    return (t if t >= 0 else t + 2 * math.pi) * TURN[unit] / (2 * math.pi)


def dist(a, b):
    return float(mp.hypot(mp.mpf(b[0]) - a[0], mp.mpf(b[1]) - a[1]))


def polar(name, system, height, unit, station, back, targets, zenith=None):
    turn = TURN[unit]
    shots, expect = [], []
    for t in targets:
        grid_len = dist(station, t)
        k, hf = factors(system, height, station, t)
        ground_len = grid_len / (k * hf)
        reading = (bearing(station, t, unit) - bearing(station, back, unit)) % turn
        shot = {"reading": reading, "distance": ground_len}
        if zenith is not None:
            # A slope length whose horizontal part is the ground length.
            z = zenith * 2 * math.pi / turn
            shot = {"reading": reading, "distance": ground_len / math.sin(z), "zenith": zenith}
        shots.append(shot)
        expect.append({"p": t, "grid": grid_len, "scale": k, "heightFactor": hf})
    return {"name": name, "input": {"unit": unit, "station": {"x": station[0], "y": station[1]},
                                    "back": {"x": back[0], "y": back[1]}, "backReading": 0.0, "shots": shots,
                                    "grid": {"system": system, "height": height}}, "expect": expect}


def stakeout(name, system, height, unit, station, back, targets):
    expect = []
    for t in targets:
        k, hf = factors(system, height, station, t)
        d = dist(station, t)
        expect.append({"distance": d, "ground": d / (k * hf), "scale": k, "heightFactor": hf})
    return {"name": name, "input": {"unit": unit, "station": {"x": station[0], "y": station[1]},
                                    "back": {"x": back[0], "y": back[1]},
                                    "targets": [{"x": t[0], "y": t[1]} for t in targets],
                                    "grid": {"system": system, "height": height}}, "expect": expect}


def traverse(name, system, height, unit, back, points, fore):
    """A connected traverse through `points` (its first the start, its last the end), oriented on `back` and `fore`."""
    turn = TURN[unit]
    angles, lengths, legs = [], [], []
    prev = back
    for i in range(len(points)):
        here = points[i]
        nxt = points[i + 1] if i + 1 < len(points) else fore
        # The angle at a station, clockwise from the previous point to the next.
        angles.append((bearing(here, nxt, unit) - bearing(here, prev, unit)) % turn)
        if i + 1 < len(points):
            k, hf = factors(system, height, here, nxt)
            grid_len = dist(here, nxt)
            lengths.append(grid_len / (k * hf))
            legs.append({"distance": grid_len, "scale": k, "heightFactor": hf})
        prev = here
    return {"name": name, "input": {"unit": unit, "start": {"x": points[0][0], "y": points[0][1]},
                                    "back": {"x": back[0], "y": back[1]},
                                    "end": {"x": points[-1][0], "y": points[-1][1]},
                                    "fore": {"x": fore[0], "y": fore[1]}, "angles": angles, "distances": lengths,
                                    "grid": {"system": system, "height": height}},
            "expect": {"points": [{"x": p[0], "y": p[1]} for p in points[1:-1]], "legs": legs}}


def at(o, pts):
    return [(o[0] + x, o[1] + y) for x, y in pts]


def build():
    tm30, utm36 = ground.entry(5254), ground.entry(32636)
    custom = ground.tm("TUREF", 34.5, 0.9999, 200000.0, 100000.0, 36.0)
    similar = {"kind": "local", "base": ground.entry(5255),
               "plane": {"kind": "similarity", "east": 492345.678, "north": 4422345.678, "rotation": 12.5, "scale": 1.000012}}
    edge = ground.project(tm30, 40.0, 31.45)
    centre = ground.project(tm30, 40.0, 30.0)
    ank = ground.project(utm36, 39.9334, 32.8597)
    cus = ground.project(custom, 36.4, 35.1)
    polars = [
        polar("TM30'un doğu kenarında, h 850, gon", tm30, 850.0, "grad", edge, (edge[0], edge[1] + 500.0),
              at(edge, [(120.5, 340.25), (-780.0, 410.0), (1500.0, -2200.0), (35.0, -12.0)])),
        polar("TM30'un orta meridyeninde, h 0, derece", tm30, 0.0, "deg", centre, (centre[0] + 300.0, centre[1]),
              at(centre, [(250.0, 250.0), (-1000.0, 20.0)])),
        polar("UTM 36N'de, h 1200, eğik uzunlukla", utm36, 1200.0, "grad", ank, (ank[0] - 400.0, ank[1] + 300.0),
              at(ank, [(600.0, 150.0), (-220.0, -880.0)]), zenith=97.5),
        polar("başlangıcı farklı TM'de, h −30", custom, -30.0, "grad", cus, (cus[0], cus[1] - 250.0),
              at(cus, [(400.0, 400.0), (-650.0, 120.0)])),
        polar("benzerlikle bağlı yerel sistemde, h 500", similar, 500.0, "grad", (100.0, 200.0), (100.0, 700.0),
              [(900.0, 650.0), (-450.0, -300.0)]),
    ]
    stakeouts = [
        stakeout("TM30'un doğu kenarında, h 850", tm30, 850.0, "grad", edge, (edge[0], edge[1] + 500.0),
                 at(edge, [(120.5, 340.25), (-780.0, 410.0), (1500.0, -2200.0)])),
        stakeout("UTM 36N'de, h 1200", utm36, 1200.0, "deg", ank, (ank[0] - 400.0, ank[1] + 300.0),
                 at(ank, [(600.0, 150.0), (-220.0, -880.0)])),
    ]
    traverses = [
        traverse("TM30'un doğu kenarında bağlı poligon, h 850", tm30, 850.0, "grad", (edge[0] - 300.0, edge[1] - 400.0),
                 at(edge, [(0, 0), (650.0, 180.0), (1210.0, -90.0), (1720.0, 260.0), (2350.0, 210.0)]),
                 (edge[0] + 2900.0, edge[1] + 600.0)),
        traverse("UTM 36N'de bağlı poligon, h 1200, derece", utm36, 1200.0, "deg", (ank[0] + 200.0, ank[1] - 500.0),
                 at(ank, [(0, 0), (-420.0, 380.0), (-910.0, 520.0), (-1300.0, 1040.0)]),
                 (ank[0] - 1800.0, ank[1] + 1500.0)),
    ]
    import pyproj
    return {"format": "kentos.ground-survey", "version": 1, "proj": pyproj.proj_version_str,
            "polar": polars, "stakeout": stakeouts, "traverse": traverses}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what PROJ, GeographicLib and the rule give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} PROJ'dan, GeographicLib'den ve kuraldan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} PROJ'dan, GeographicLib'den ve kuraldan çıkanla aynı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    d = json.loads(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(d['polar'])} kutupsal alım, {len(d['stakeout'])} aplikasyon, "
          f"{len(d['traverse'])} poligon.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
