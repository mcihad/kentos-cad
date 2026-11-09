#!/usr/bin/env python3
"""Independent reference of the survey windows' lengths between ground and grid (docs/adr/0171 §4).

Writes fixtures/geodesy/v1/ground-survey.json from PROJ and GeographicLib's C library (through pyproj) and mpmath, no
KentOS code. The cases are built backwards from known points on the grid: a line's ground length is its grid length over
its factors, the projection's scale along it (Simpson's rule on PROJ's `get_factors` at its ends and middle; a local
system on a similarity, its base's over the similarity's scale) times the height's factor R/(R + h) (R Euler's radius
at the geodesic's middle in its direction there, ground_cases.py's rule), at the project's mean ellipsoidal height h.

- Kutupsal alım: from a station oriented on a back point, the readings of the known points (their grid bearings less
  the back's, in the unit) and their ground lengths (horizontal, or slope with a zenith angle); the core, given the
  grid, must place the points where they are (within 1e-6 m) and give their grid lengths, factors and the height each
  line was reduced at. With the station's height, the points' heights and the geoid's height above the ellipsoid at
  each (docs/adr/2900 §3), a shot with a zenith angle is reduced at its own line's height: the mean of its two ends'
  ellipsoidal heights, each the point's height plus its undulation; the slope length and the zenith angle are built
  from the ground length and the height difference (no refraction, no instrument or target height). A shot without a
  zenith angle, or a station without a height, is reduced at the project's mean height.
- Aplikasyon: the known points' grid distances and their ground distances with the factors. With the station's height,
  the targets' heights and the undulations at each (docs/adr/3300), a line to a target with a height is taken at its own
  height, the mean of its two ends' ellipsoidal heights; a target without one at the project's mean height.
- Poligon hesabı: a connected traverse through known points, its angles from the grid and its legs' ground lengths;
  the core must give the points back (within 1e-6 m) with no misclosure (within 1e-6 m). With the points' heights and
  the undulations at each (docs/adr/4100), each leg is reduced at its own height, the mean of its ends' ellipsoidal
  heights, and its zenith angle is built from its ground length and the height difference less the instrument's and
  plus the target's height (S·cot z = ΔH − i + t; no refraction): the core must give the heights back too.

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
        expect.append({"p": t, "grid": grid_len, "scale": k, "heightFactor": hf, "height": height})
    return {"name": name, "input": {"unit": unit, "station": {"x": station[0], "y": station[1]},
                                    "back": {"x": back[0], "y": back[1]}, "backReading": 0.0, "shots": shots,
                                    "grid": {"system": system, "height": height}}, "expect": expect}


def polar_heights(name, system, height, unit, station, back, targets, station_z, undulations):
    """Kutupsal alım with the points' heights: each target (east, north, height or None for a horizontal shot), the
    undulations at the station and each target (None: not given), each line reduced at its ends' ellipsoidal heights'
    mean (a shot without a height at the mean height `height`)."""
    turn = TURN[unit]
    shots, expect = [], []
    for i, (x, y, z) in enumerate(targets):
        t = (x, y)
        grid_len = dist(station, t)
        reading = (bearing(station, t, unit) - bearing(station, back, unit)) % turn
        if z is None or station_z is None or undulations is None:
            h = height
        else:
            h = ((station_z + undulations[0]) + (z + undulations[i + 1])) / 2.0
        k, hf = factors(system, h, station, t)
        ground_len = grid_len / (k * hf)
        if z is None:
            shot = {"reading": reading, "distance": ground_len}
        else:
            dz = z - station_z
            slope = math.hypot(ground_len, dz)
            zenith = math.atan2(ground_len, dz) * turn / (2 * math.pi)
            shot = {"reading": reading, "distance": slope, "zenith": zenith}
        shots.append(shot)
        expect.append({"p": t, "grid": grid_len, "scale": k, "heightFactor": hf, "height": h})
    input_ = {"unit": unit, "station": {"x": station[0], "y": station[1]}, "back": {"x": back[0], "y": back[1]},
              "backReading": 0.0, "shots": shots, "grid": {"system": system, "height": height}}
    if station_z is not None:
        input_["stationZ"] = station_z
    if undulations is not None:
        input_["undulations"] = undulations
    return {"name": name, "input": input_, "expect": expect}


def stakeout(name, system, height, unit, station, back, targets, station_z=None, target_zs=None, undulations=None):
    """Aplikasyon; with heights (docs/adr/3300) each line to a target with one at its ends' ellipsoidal heights' mean."""
    expect = []
    for i, t in enumerate(targets):
        z = None if target_zs is None else target_zs[i]
        if z is None or station_z is None or undulations is None:
            h = height
        else:
            h = ((station_z + undulations[0]) + (z + undulations[i + 1])) / 2.0
        k, hf = factors(system, h, station, t)
        d = dist(station, t)
        expect.append({"distance": d, "ground": d / (k * hf), "scale": k, "heightFactor": hf, "height": h})
    input_ = {"unit": unit, "station": {"x": station[0], "y": station[1]}, "back": {"x": back[0], "y": back[1]},
              "targets": [{"x": t[0], "y": t[1]} for t in targets], "grid": {"system": system, "height": height}}
    if station_z is not None:
        input_["stationZ"] = station_z
    if target_zs is not None:
        input_["targetZs"] = target_zs
    if undulations is not None:
        input_["undulations"] = undulations
    return {"name": name, "input": input_, "expect": expect}


def traverse(name, system, height, unit, back, points, fore, heights=None):
    """A connected traverse through `points` (its first the start, its last the end), oriented on `back` and `fore`.
    `heights` (docs/adr/4100): (the points' heights, the undulations at each, the instrument's and the target's height
    at each leg)."""
    turn = TURN[unit]
    angles, lengths, legs, zeniths = [], [], [], []
    prev = back
    for i in range(len(points)):
        here = points[i]
        nxt = points[i + 1] if i + 1 < len(points) else fore
        # The angle at a station, clockwise from the previous point to the next.
        angles.append((bearing(here, nxt, unit) - bearing(here, prev, unit)) % turn)
        if i + 1 < len(points):
            own = None
            if heights is not None:
                hs, ns, _, _ = heights
                own = ((hs[i] + ns[i]) + (hs[i + 1] + ns[i + 1])) / 2
            k, hf = factors(system, height if own is None else own, here, nxt)
            grid_len = dist(here, nxt)
            ground_len = grid_len / (k * hf)
            lengths.append(ground_len)
            legs.append({"distance": grid_len, "scale": k, "heightFactor": hf, **({} if own is None else {"height": own})})
            if heights is not None:
                hs, _, inst, tgt = heights
                # S·cot z = ΔH − i + t: z = 90° − atan((ΔH − i + t)/S), in the unit.
                z = math.pi / 2 - math.atan((hs[i + 1] - hs[i] - inst[i] + tgt[i]) / ground_len)
                zeniths.append(z * turn / (2 * math.pi))
        prev = here
    input_ = {"unit": unit, "start": {"x": points[0][0], "y": points[0][1]}, "back": {"x": back[0], "y": back[1]},
              "end": {"x": points[-1][0], "y": points[-1][1]}, "fore": {"x": fore[0], "y": fore[1]}, "angles": angles,
              "distances": lengths, "grid": {"system": system, "height": height}}
    expect = {"points": [{"x": p[0], "y": p[1]} for p in points[1:-1]], "legs": legs}
    if heights is not None:
        hs, ns, inst, tgt = heights
        input_["heights"] = {"start": hs[0], "end": hs[-1], "zeniths": zeniths, "instruments": inst, "targets": tgt,
                             "undulations": ns}
        expect["heights"] = hs[1:-1]
    return {"name": name, "input": input_, "expect": expect}


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
        polar_heights("TM30'un doğu kenarında, elipsoit yükseklikleri: her çizgi uçlarının ortalamasında", tm30, 850.0,
                      "grad", edge, (edge[0], edge[1] + 500.0),
                      [(edge[0] + 420.0, edge[1] + 310.0, 1150.0), (edge[0] - 900.0, edge[1] + 260.0, 610.25),
                       (edge[0] + 1500.0, edge[1] - 1800.0, 2100.0)],
                      850.0, [0.0, 0.0, 0.0, 0.0]),
        polar_heights("UTM 36N'de ortometrik kotlar ve geoit ayrımları; başucu açısız çizgi ortalamada", utm36, 1200.0,
                      "deg", ank, (ank[0] - 400.0, ank[1] + 300.0),
                      [(ank[0] + 600.0, ank[1] + 150.0, 1180.5), (ank[0] - 220.0, ank[1] - 880.0, 990.0),
                       (ank[0] + 300.0, ank[1] - 300.0, None)],
                      1100.0, [36.21, 36.25, 36.18, 36.2]),
        polar_heights("İstasyon kotu yokken ayrımlar kullanılmaz: ortalama yükseklik", tm30, 850.0, "grad", edge,
                      (edge[0], edge[1] + 500.0), [(edge[0] + 420.0, edge[1] + 310.0, None)], None, [0.0, 0.0]),
    ]
    stakeouts = [
        stakeout("TM30'un doğu kenarında, h 850", tm30, 850.0, "grad", edge, (edge[0], edge[1] + 500.0),
                 at(edge, [(120.5, 340.25), (-780.0, 410.0), (1500.0, -2200.0)])),
        stakeout("UTM 36N'de, h 1200", utm36, 1200.0, "deg", ank, (ank[0] - 400.0, ank[1] + 300.0),
                 at(ank, [(600.0, 150.0), (-220.0, -880.0)])),
        # docs/adr/3300: the lines at their own heights.
        stakeout("TM30'un doğu kenarında, elipsoit yükseklikleri: her çizgi uçlarının ortalamasında", tm30, 850.0,
                 "grad", edge, (edge[0], edge[1] + 500.0),
                 at(edge, [(420.0, 310.0), (-900.0, 260.0), (1500.0, -1800.0)]), 850.0, [1150.0, 610.25, 2100.0],
                 [0.0, 0.0, 0.0, 0.0]),
        stakeout("UTM 36N'de ortometrik kotlar ve geoit ayrımları; kotu olmayan hedef ortalamada", utm36, 1200.0, "deg",
                 ank, (ank[0] - 400.0, ank[1] + 300.0), at(ank, [(600.0, 150.0), (-220.0, -880.0), (300.0, -300.0)]),
                 1100.0, [1180.5, 990.0, None], [36.21, 36.25, 36.18, 36.2]),
        stakeout("İstasyon kotu yokken ayrımlar kullanılmaz: ortalama yükseklik", tm30, 850.0, "grad", edge,
                 (edge[0], edge[1] + 500.0), at(edge, [(420.0, 310.0)]), None, [1150.0], [0.0, 0.0]),
    ]
    traverses = [
        traverse("TM30'un doğu kenarında bağlı poligon, h 850", tm30, 850.0, "grad", (edge[0] - 300.0, edge[1] - 400.0),
                 at(edge, [(0, 0), (650.0, 180.0), (1210.0, -90.0), (1720.0, 260.0), (2350.0, 210.0)]),
                 (edge[0] + 2900.0, edge[1] + 600.0)),
        traverse("UTM 36N'de bağlı poligon, h 1200, derece", utm36, 1200.0, "deg", (ank[0] + 200.0, ank[1] - 500.0),
                 at(ank, [(0, 0), (-420.0, 380.0), (-910.0, 520.0), (-1300.0, 1040.0)]),
                 (ank[0] - 1800.0, ank[1] + 1500.0)),
        # docs/adr/4100: its heights by trigonometric heighting, each leg at its own height.
        traverse("UTM 36N'de bağlı poligon trigonometrik kotlarla: her kenar kendi yüksekliğinde", utm36, 1200.0, "deg",
                 (ank[0] + 200.0, ank[1] - 500.0), at(ank, [(0, 0), (-420.0, 380.0), (-910.0, 520.0), (-1300.0, 1040.0)]),
                 (ank[0] - 1800.0, ank[1] + 1500.0),
                 ([1120.25, 1185.5, 1320.75, 1290.0], [36.21, 36.24, 36.27, 36.3], [1.55, 1.48, 1.6], [1.7, 1.3, 1.65])),
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
