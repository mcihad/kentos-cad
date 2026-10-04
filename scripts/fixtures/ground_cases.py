#!/usr/bin/env python3
"""Independent reference of plane, ellipsoid and ground values (docs/adr/0171).

Writes fixtures/geodesy/v1/ground.json from PROJ and GeographicLib's C library (both through pyproj) and mpmath, no
KentOS code:

- a point's projection scale: PROJ's `get_factors` (a transverse Mercator's is one in every direction); a local
  system on a similarity, its base's over the similarity's scale; none for a geographic system, the Pseudo-Mercator
  (its meridian and parallel scales differ on the ellipsoid) and an affine local plane;
- a line's scale: Simpson's rule, (k₁ + 4kₘ + k₂)/6 at its ends and its plane middle;
- a line's height factor R/(R + h): R Euler's radius MN/(N cos²α + M sin²α) at the geodesic's middle (GeographicLib's
  direct problem from its start, half its length) in the geodesic's direction there, h the project's mean
  ellipsoidal height;
- a path's or an area's values (ADR §1): the points taken from the project's plane to its own datum's latitude and
  longitude the way PROJ takes them (no datum shift); a straight segment the geodesic between its ends, an arc
  segment cut into the pieces of docs/adr/0167 §2 (crs_measure_cases.py's rule, sagitta at most 0.1 mm) whose
  geodesics are summed and taken to the arc, times its exact length (r·|θ|, mpmath) over its chords' sum in the
  plane; on the ground each piece's geodesic times (R + h)/R as above, taken to the arc likewise; a ring's ellipsoid
  area the region its geodesics and its arcs bound: its boundary dense on the ellipsoid (a straight segment's geodesic
  in points at most 1 m apart, GeographicLib's; an arc's pieces as above) and its area the shoelace area on PROJ's
  Lambert azimuthal equal-area projection of the ellipsoid about the ring (mpmath), times its exact plane area
  (shoelace and circular segments, mpmath) over its chords' polygon's when it has arcs; holes taken from the outer
  ring; on the ground times (M + h)(N + h)/(MN) at the outer ring's middle latitude ((lowest + highest)/2); the scale
  the plane's value over the ellipsoid's (an area's ratio's square root), the height factor the ellipsoid's over the
  ground's (an area's √(MN/((M + h)(N + h)))).

An area is not GeographicLib's polygon of the dense points: its area is a sum of each edge's area to the equator, and
its rounding grows with the edges. Its polygon of an area's vertices is what the core takes (and the arcs beside it);
`areaNoise` is how far that polygon may move when each of its coordinates moves by one ulp (the sum of their moves),
the core's floor: its latitudes and longitudes are its own projection's, a few ulps from PROJ's.

The geometry core (crates/shared/geometry-core/tests/all/crs_ground.rs) and the web through its WASM
(apps/web/src/wasm/crsGround.wasm.test.ts) must give the same: point and line scales within 1e-10 (PROJ's factors are
numerical derivatives), lengths within 1e-6 m, areas within 1e-6 m² for every 100 m of their perimeter and four times
their `areaNoise`, height factors within 1e-10; a measure's scale as its values allow (1e-10 and a path's length's
tolerance over its length, or half an area's relative tolerance).
"""

import argparse
import json
import math
import sys
from pathlib import Path

import mpmath as mp
from pyproj import CRS, Geod, Proj, Transformer

sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_measure_cases as pieces  # noqa: E402  (the arcs' pieces: docs/adr/0167 §2)
import crs_transform_cases as registry  # noqa: E402  (the registry's systems as the core reads them)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "ground.json"

mp.mp.dps = 40

# The registry's datums by EPSG's geographic systems: their ellipsoids are PROJ's.
GEOGRAPHIC = {"TUREF": "EPSG:5252", "ED50": "EPSG:4230", "WGS84": "EPSG:4326"}

BESSEL = {"name": "Bessel datumu", "ellipsoid": {"name": "Bessel 1841", "semiMajor": 6377397.155, "inverseFlattening": 299.1528128},
          "toWgs84": {"translation": [598.1, 73.7, 418.2], "rotation": [0.202, 0.045, -2.455], "scale": 6.7,
                      "convention": "positionVector", "accuracy": 1.5}}


def tm(datum, cm, k=1.0, fe=500000.0, fn=0.0, lat0=None):
    s = {"kind": "tm", "datum": datum, "centralMeridian": cm, "scaleFactor": k, "falseEasting": fe, "falseNorthing": fn}
    return s if lat0 is None else {**s, "latitudeOfOrigin": lat0}


def entry(srid):
    for e in json.loads(registry.REGISTRY.read_text(encoding="utf-8"))["systems"]:
        if e["srid"] == srid:
            return registry.system(e)
    raise SystemExit(f"no {srid} in the registry")


def ellipsoid(datum):
    """(a, 1/f) of a datum: the registry's from PROJ, the project's as written."""
    if isinstance(datum, dict):
        return datum["ellipsoid"]["semiMajor"], datum["ellipsoid"]["inverseFlattening"]
    e = CRS(GEOGRAPHIC[datum]).ellipsoid
    return e.semi_major_metre, e.inverse_flattening


def datum_of(system):
    if system["kind"] == "local":
        return datum_of(system["base"])
    return "WGS84" if system["kind"] == "mercator" else system["datum"]


def grid_crs(system):
    """PROJ's projected system of a TM or the Pseudo-Mercator: EPSG's for the registry's, a PROJ string otherwise."""
    if system["kind"] == "mercator":
        return CRS("EPSG:3857")
    for e in json.loads(registry.REGISTRY.read_text(encoding="utf-8"))["systems"]:
        if e["kind"] != "local" and registry.system(e) == system:
            return CRS(f"EPSG:{e['srid']}")
    a, rf = ellipsoid(system["datum"])
    return CRS(f"+proj=tmerc +lat_0={system.get('latitudeOfOrigin', 0.0)!r} +lon_0={system['centralMeridian']!r} "
               f"+k_0={system['scaleFactor']!r} +x_0={system['falseEasting']!r} +y_0={system['falseNorthing']!r} "
               f"+a={a!r} +rf={rf!r} +units=m +no_defs +type=crs")


def plane_forward(plane, x, y):
    """A local point in its base, PROJ's `affine` step's sums."""
    if plane["kind"] == "similarity":
        t = math.radians(plane["rotation"])
        s = plane["scale"]
        a, b, c, d, e, f = s * math.cos(t), -s * math.sin(t), plane["east"], s * math.sin(t), s * math.cos(t), plane["north"]
    else:
        a, b, c, d, e, f = (plane[k] for k in "abcdef")
    return a * x + b * y + c, d * x + e * y + f


def unproject(system, x, y):
    """(latitude, longitude) on the system's own datum; None where PROJ gives no point."""
    if system["kind"] == "local":
        return unproject(system["base"], *plane_forward(system["plane"], x, y))
    if system["kind"] == "geographic":
        return (y, x) if abs(y) <= 90 else None
    crs = grid_crs(system)
    lon, lat = Transformer.from_crs(crs, crs.geodetic_crs, always_xy=True).transform(x, y)
    return (lat, lon) if math.isfinite(lat) and math.isfinite(lon) else None


def project(system, lat, lon):
    """A latitude and longitude on the system's datum into its plane (to place the cases' points)."""
    if system["kind"] == "local":
        raise SystemExit("local points are written as local coordinates")
    if system["kind"] == "geographic":
        return [lon, lat]
    crs = grid_crs(system)
    x, y = Transformer.from_crs(crs.geodetic_crs, crs, always_xy=True).transform(lon, lat)
    return [x, y]


def point_scale(system, x, y):
    if system["kind"] == "local":
        plane = system["plane"]
        if plane["kind"] != "similarity":
            return None
        k = point_scale(system["base"], *plane_forward(plane, x, y))
        return None if k is None else k / plane["scale"]
    if system["kind"] != "tm":
        return None
    lat, lon = unproject(system, x, y)
    f = Proj(grid_crs(system)).get_factors(lon, lat)
    # Conformal: one scale in every direction.
    assert abs(f.meridional_scale - f.parallel_scale) < 1e-10, (system, x, y, f)
    return f.meridional_scale


def radii(a, rf, lat):
    f = 1 / mp.mpf(rf)
    e2 = f * (2 - f)
    s = mp.sin(mp.radians(lat))
    w2 = 1 - e2 * s * s
    return a * (1 - e2) / (w2 * mp.sqrt(w2)), a / mp.sqrt(w2)


def euler(a, rf, lat, azimuth):
    m, n = radii(a, rf, lat)
    t = mp.radians(azimuth)
    return m * n / (n * mp.cos(t) ** 2 + m * mp.sin(t) ** 2)


def geodesic(geod, p, q):
    """The geodesic from p to q (latitude, longitude): its length and its middle's latitude and azimuth."""
    az12, _az21, dist = geod.inv(p[1], p[0], q[1], q[0])
    _lon, lat, back = geod.fwd(p[1], p[0], az12, dist / 2)
    return dist, lat, back + 180.0


def line_factors(system, a, b, height):
    ea, rf = ellipsoid(datum_of(system))
    geod = Geod(a=ea, rf=rf)
    p, q = unproject(system, *a), unproject(system, *b)
    if p is None or q is None:
        return None
    dist, lat, azi = geodesic(geod, p, q)
    out = {"ellipsoidLength": dist}
    ks = [point_scale(system, *a), point_scale(system, (a[0] + b[0]) / 2, (a[1] + b[1]) / 2), point_scale(system, *b)]
    if None not in ks:
        out["scale"] = (ks[0] + 4 * ks[1] + ks[2]) / 6
        # Simpson's rule against the line's own ratio: the plane over the geodesic.
        ratio = math.hypot(b[0] - a[0], b[1] - a[1]) / dist
        assert abs(out["scale"] - ratio) < 1e-9, (system, a, b, out["scale"], ratio)
    if height is not None:
        r = euler(ea, rf, lat, azi)
        out["heightFactor"] = float(r / (r + height))
    return out


def arc_of(a, b, bulge):
    """An arc segment's radius and signed sweep (mpmath), or None for a straight one (crs_measure_cases' rule)."""
    if abs(bulge) <= 1e-12:
        return None
    chord = mp.hypot(mp.mpf(b[0]) - a[0], mp.mpf(b[1]) - a[1])
    if chord < 1e-12:
        return None
    return chord * (1 + mp.mpf(bulge) ** 2) / (4 * abs(mp.mpf(bulge))), 4 * mp.atan(bulge)


def shoelace(points):
    """The signed shoelace area about the first point."""
    o = points[0]
    rel = [(mp.mpf(x) - o[0], mp.mpf(y) - o[1]) for x, y in points]
    return mp.fsum(p[0] * q[1] - q[0] * p[1] for p, q in zip(rel, rel[1:] + rel[:1])) / 2


def equal_area(system, ring, geod, ea, rf):
    """The ring's area on the ellipsoid: its boundary dense (geodesics in points at most 1 m apart, arcs in their
    pieces), shoelace on PROJ's Lambert azimuthal equal-area projection about the ring."""
    pts, bulges = ring["pts"], ring.get("bulges")
    n = len(pts)
    dense = []
    for k in range(n):
        a, b = tuple(pts[k]), tuple(pts[(k + 1) % n])
        bulge = bulges[k] if bulges and k < len(bulges) else 0.0
        if arc_of(a, b, bulge) is not None:
            dense.extend(unproject(system, *q) for q in pieces.arc_points(a, b, bulge)[:-1])
            continue
        p, q = unproject(system, *a), unproject(system, *b)
        _az12, _az21, dist = geod.inv(p[1], p[0], q[1], q[0])
        dense.append(p)
        steps = math.ceil(dist / 1.0)
        if steps > 1:
            dense.extend((lat, lon) for lon, lat in geod.npts(p[1], p[0], q[1], q[0], steps - 1))
    lat0 = sum(lat for lat, _ in dense) / len(dense)
    lon0 = sum(lon for _, lon in dense) / len(dense)
    laea = Proj(proj="laea", lat_0=lat0, lon_0=lon0, a=ea, rf=rf)
    xs, ys = laea([lon for _, lon in dense], [lat for lat, _ in dense])
    return abs(shoelace(list(zip(xs, ys))))


def polygon_noise(geod, vertices):
    """How far GeographicLib's polygon of the vertices may move when each of their coordinates moves by one ulp: the
    sum over the coordinates of its larger move either way."""
    if len(vertices) < 3:
        return 0.0
    lats, lons = [lat for lat, _ in vertices], [lon for _, lon in vertices]
    base, _ = geod.polygon_area_perimeter(lons, lats)
    total = 0.0
    for i in range(len(vertices)):
        for values in (lats, lons):
            most = 0.0
            for way in (math.inf, -math.inf):
                kept = values[i]
                values[i] = math.nextafter(kept, way)
                moved, _ = geod.polygon_area_perimeter(lons, lats)
                values[i] = kept
                most = max(most, abs(moved - base))
            total += most
    return total


def measures(system, rings, closed, height):
    ea, rf = ellipsoid(datum_of(system))
    geod = Geod(a=ea, rf=rf)
    rings = rings if closed else rings[:1]
    has_plane = system["kind"] != "geographic"
    plane_len, plane_ar, ell_len, ell_ar, ground_len = [], [], [], [], []
    lats = []
    noise = 0.0
    for i, ring in enumerate(rings):
        pts, bulges = ring["pts"], ring.get("bulges")
        n = len(pts)
        plane_pieces, geo, arcs = [tuple(pts[0])], [unproject(system, *pts[0])], False
        if geo[0] is None:
            return None
        for k in range(n if closed else n - 1):
            a, b = tuple(pts[k]), tuple(pts[(k + 1) % n])
            bulge = bulges[k] if bulges and k < len(bulges) else 0.0
            chords, s, ground = [], [], []
            for p, q in zip(pieces.arc_points(a, b, bulge), pieces.arc_points(a, b, bulge)[1:]):
                g = unproject(system, *q)
                if g is None:
                    return None
                dist, lat, azi = geodesic(geod, geo[-1], g)
                chords.append(mp.hypot(mp.mpf(q[0]) - p[0], mp.mpf(q[1]) - p[1]))
                s.append(mp.mpf(dist))
                if height is not None:
                    r = euler(ea, rf, lat, azi)
                    ground.append(dist * (r + height) / r)
                plane_pieces.append(q)
                geo.append(g)
            arc = arc_of(a, b, bulge)
            chord_sum = mp.fsum(chords)
            if arc is not None and chord_sum > 0:
                arcs = True
                exact = arc[0] * abs(arc[1])
                ratio = exact / chord_sum
            else:
                exact, ratio = chord_sum, 1
            plane_len.append(exact)
            ell_len.append(mp.fsum(s) * ratio)
            ground_len.append(mp.fsum(ground) * ratio)
        if not closed:
            continue
        plane_pieces.pop()
        geo.pop()
        if i == 0:
            lats = [lat for lat, _ in geo]
        exact = shoelace(pts)
        for k in range(n):
            arc = arc_of(tuple(pts[k]), tuple(pts[(k + 1) % n]), bulges[k] if bulges and k < len(bulges) else 0.0)
            if arc is not None:
                r, sweep = arc
                exact += r * r / 2 * (sweep - mp.sin(sweep))
        exact = abs(exact)
        on_ellipsoid = equal_area(system, ring, geod, ea, rf)
        chordal = abs(shoelace(plane_pieces))
        if arcs and chordal > 0:
            on_ellipsoid *= exact / chordal
        vertices = [unproject(system, *v) for v in pts]
        ring_noise = polygon_noise(geod, vertices)
        noise += ring_noise
        if not arcs and len(vertices) > 2:
            # Without arcs the region is GeographicLib's polygon of the vertices: the two ways agree within its floor.
            polygon, _ = geod.polygon_area_perimeter([lon for _, lon in vertices], [lat for lat, _ in vertices])
            assert abs(abs(polygon) - on_ellipsoid) <= ring_noise + 1e-7, (system, ring, polygon, on_ellipsoid)
        sign = 1 if i == 0 else -1
        plane_ar.append(sign * exact)
        ell_ar.append(sign * on_ellipsoid)
    out = {}
    pl, ell, gl = mp.fsum(plane_len), mp.fsum(ell_len), mp.fsum(ground_len)
    if has_plane:
        out["planeLength"] = pl
        if closed:
            out["planeArea"] = mp.fsum(plane_ar)
    out["ellipsoidLength"] = ell
    if closed:
        out["ellipsoidArea"] = mp.fsum(ell_ar)
    area_factor = None
    if height is not None:
        out["groundLength"] = gl
        if closed and lats:
            m, n = radii(ea, rf, (min(lats) + max(lats)) / 2)
            area_factor = m * n / ((m + height) * (n + height))
            out["groundArea"] = out["ellipsoidArea"] / area_factor
    if has_plane:
        if closed and out["ellipsoidArea"] > 0:
            out["scale"] = mp.sqrt(out["planeArea"] / out["ellipsoidArea"])
        elif not closed and ell > 0:
            out["scale"] = pl / ell
    if height is not None:
        if closed and area_factor is not None:
            out["heightFactor"] = mp.sqrt(area_factor)
        elif not closed and gl > 0:
            out["heightFactor"] = ell / gl
    out = {k: float(v) for k, v in out.items()}
    return out, noise


def at(origin, pts):
    return [[origin[0] + x, origin[1] + y] for x, y in pts]


def build():
    tm30, tm33, tm36 = entry(5254), entry(5255), entry(5256)
    ed36, utm36, geo, merc = entry(2322), entry(32636), entry(5252), entry(3857)
    ed33, utm37 = entry(2321), entry(23037)
    custom = tm("TUREF", 34.5, 0.9999, 200000.0, 100000.0, 36.0)
    bessel = tm(BESSEL, 33.0)
    similar = {"kind": "local", "base": tm33,
               "plane": {"kind": "similarity", "east": 492345.678, "north": 4422345.678, "rotation": 12.5, "scale": 1.000012}}
    similar_custom = {"kind": "local", "base": custom,
                      "plane": {"kind": "similarity", "east": 1000.0, "north": 2000.0, "rotation": -3.25, "scale": 1.0}}
    affine = {"kind": "local", "base": entry(2321),
              "plane": {"kind": "affine", "a": 1.00002, "b": -0.0003, "c": 492000.0, "d": 0.00025, "e": 0.99998, "f": 4422500.0}}

    scales = []
    for name, system, place in [
        ("TUREF / TM30, orta meridyende", tm30, (40.0, 30.0)),
        ("TUREF / TM30, doğu kenarında", tm30, (40.0, 31.5)),
        ("TUREF / TM30, batı kenarında, güneyde", tm30, (36.0, 28.5)),
        ("TUREF / TM30, kuzeyde", tm30, (42.0, 31.0)),
        ("ED50 / TM33", ed33, (39.0, 34.4)),
        ("WGS 84 / UTM 36N, Ankara", utm36, (39.9334, 32.8597)),
        ("WGS 84 / UTM 36N, dilimin batı kenarı", utm36, (38.0, 30.0)),
        ("ED50 / UTM 37N, dilimin dışında (Van)", utm37, (38.5012, 43.3729)),
        ("başlangıcı ve ölçeği farklı TM", custom, (36.5, 35.2)),
        ("Bessel datumlu TM33", bessel, (39.5, 34.0)),
    ]:
        x, y = project(system, *place)
        scales.append({"name": name, "system": system, "point": [x, y], "expect": point_scale(system, x, y)})
    for name, system, point in [
        ("yerel sistem, benzerlikle TM33'e", similar, [100.0, 200.0]),
        ("yerel sistem, benzerlikle TM33'e, uzakta", similar, [2500.0, -1500.0]),
        ("yerel sistem, özel TM'ye", similar_custom, [-350.0, 820.0]),
        ("yerel sistem, afinle: tek ölçeği yok", affine, [100.0, 200.0]),
        ("coğrafi: düzlemi yok", geo, [32.8597, 39.9334]),
        ("Pseudo-Mercator: tek ölçeği yok", merc, [3225000.0, 5010000.0]),
    ]:
        scales.append({"name": name, "system": system, "point": point, "expect": point_scale(system, *point)})

    lines = []
    for name, system, a, b, height in [
        ("TM30'un orta meridyeninde 1 km, h 0", tm30, project(tm30, 40.0, 30.0), project(tm30, 40.009, 30.0), 0.0),
        ("TM30'un doğu kenarında 10 km doğu-batı, h 1000", tm30, project(tm30, 40.0, 31.38), project(tm30, 40.0, 31.497), 1000.0),
        ("TM30'un batı kenarında 5 km kuzey-güney, h 2000", tm30, project(tm30, 38.0, 28.6), project(tm30, 38.045, 28.6), 2000.0),
        ("UTM 36N'de 3 km çapraz, h 850", utm36, project(utm36, 39.9, 32.8), project(utm36, 39.92, 32.825), 850.0),
        ("ED50 / TM36'da, h −30", ed36, project(ed36, 36.8, 36.2), project(ed36, 36.81, 36.21), -30.0),
        ("yerel sistemde, h 500", similar, [100.0, 200.0], [1350.0, -410.0], 500.0),
        ("Bessel datumlu TM33'te, yükseklik yok", bessel, project(bessel, 39.5, 34.0), project(bessel, 39.52, 34.03), None),
        ("coğrafide, h 100", geo, [32.85, 39.93], [32.86, 39.935], 100.0),
        ("Pseudo-Mercator'da, h 0", merc, [3225000.0, 5010000.0], [3226000.0, 5011000.0], 0.0),
        ("coğrafide kutbun ötesi", geo, [32.85, 39.93], [32.86, 95.0], 100.0),
    ]:
        lines.append({"name": name, "system": system, "a": a, "b": b, "height": height,
                      "expect": line_factors(system, a, b, height)})

    ist, ank = project(tm30, 41.0082, 28.9784), project(tm36, 39.9334, 32.8597)
    edge = project(tm30, 40.0, 31.45)
    ank_utm = project(utm36, 39.9334, 32.8597)
    ank_ed = project(ed36, 39.9334, 32.8597)
    ist_merc = project(merc, 41.0082, 28.9784)
    bes = project(bessel, 39.5, 34.0)
    hole = at(ank, [(40, 30), (60, 30), (60, 50), (40, 50)])
    shapes = [
        ("açık yol, üç köşe", tm30, [{"pts": at(ist, [(0, 0), (60.25, 12.5), (84.75, -30.125)])}], False, [None, 100.0]),
        ("açık yol, yaylı kenarla", tm30, [{"pts": at(ist, [(0, 0), (40, 0), (40, 30)]), "bulges": [0.0, 0.4]}], False, [850.0]),
        ("kapalı alan, 40 × 25 m", tm30, [{"pts": at(ist, [(0, 0), (40, 0), (40, 25), (0, 25)])}], True, [0.0, 1000.0]),
        ("kapalı alan, yaylı kenarlı ve delikli", tm36,
         [{"pts": at(ank, [(0, 0), (120, 0), (120, 80), (0, 80)]), "bulges": [0.0, 0.25, 0.0, -0.1]}, {"pts": hole}], True, [900.0]),
        ("TM30'un doğu kenarında 1 km'lik parsel", tm30, [{"pts": at(edge, [(0, 0), (1000, 0), (1000, 1000), (0, 1000)])}], True, [1500.0]),
        ("UTM 36N'de 1 km'lik parsel", utm36, [{"pts": at(ank_utm, [(0, 0), (1000, 0), (1000, 1000), (0, 1000)])}], True, [950.0]),
        ("ED50 / TM36'da yol", ed36, [{"pts": at(ank_ed, [(0, 0), (250, 40), (400, -120), (520, 10)])}], False, [-30.0]),
        ("coğrafide 0,01°'lik dörtgen", geo,
         [{"pts": [[32.85, 39.93], [32.86, 39.93], [32.86, 39.94], [32.85, 39.94]]}], True, [None, 1200.0]),
        ("Pseudo-Mercator'da parsel", merc, [{"pts": at(ist_merc, [(0, 0), (80, 0), (80, 50), (0, 50)])}], True, [50.0]),
        ("yerel sistemde (benzerlik) parsel", similar, [{"pts": [[100, 200], [180, 210], [175, 290], [95, 280]]}], True, [0.0, 500.0]),
        ("yerel sistemde (afin) yol", affine, [{"pts": [[100, 200], [400, 260], [520, 30]]}], False, [200.0]),
        ("Bessel datumlu TM33'te parsel", bessel, [{"pts": at(bes, [(0, 0), (300, 0), (300, 200), (0, 200)])}], True, [300.0]),
        ("aynı noktada iki köşeli yol", tm30, [{"pts": at(ist, [(0, 0), (0, 0)])}], False, [100.0]),
        ("daire, 20 m yarıçaplı (iki yaylı kenar)", tm36, [{"pts": at(ank, [(0, 0), (40, 0)]), "bulges": [1.0, 1.0]}], True, [1000.0]),
        ("TM30'un doğu kenarında 10 km'lik büyük alan", tm30,
         [{"pts": at(edge, [(0, 0), (10000, 0), (10000, 10000), (0, 10000)])}], True, [0.0]),
        ("coğrafide kutbun ötesinde köşe", geo, [{"pts": [[32.85, 39.93], [32.86, 95.0]]}], False, [100.0]),
    ]
    cases = []
    for name, system, rings, closed, heights in shapes:
        for h in heights:
            label = name if h is None else f"{name}, h {h:g}"
            got = measures(system, rings, closed, h)
            expect, noise = got if got else (None, 0.0)
            case = {"name": label, "system": system, "rings": rings, "closed": closed, "height": h, "expect": expect}
            if expect is None:
                case["why"] = "unreachable"
            elif closed:
                case["areaNoise"] = noise
            cases.append(case)
    import pyproj
    return {"format": "kentos.ground", "version": 1, "proj": pyproj.proj_version_str, "chord": pieces.CHORD,
            "scales": scales, "lines": lines, "measures": cases}


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
    data = json.loads(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(data['scales'])} ölçek, {len(data['lines'])} çizgi, {len(data['measures'])} ölçü.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
