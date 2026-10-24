#!/usr/bin/env python3
"""Independent reference of geoid models and velocity grids in `cad.crs.transform` (docs/adr/0631, plan/gis.md CBS-08).

    .run/geo-py/bin/python scripts/fixtures/geoid_cases.py --files   # writes the grids, fixtures/geoid/v1 (GDAL)
    .run/geo-py/bin/python scripts/fixtures/geoid_cases.py           # writes fixtures/commands/v1/cad.crs.transform.geoid.json
    .run/geo-py/bin/python scripts/fixtures/geoid_cases.py --check   # compares

No KentOS code. The grids are written here: a geoid model over Türkiye as GTX (with a corner of nodes without a value),
the same as GeoTIFF of pixel areas and of pixel points, a global geoid at 1° as GTX (longitudes 0–359, wrapping), a
velocity grid of three bands (east, north, up, mm/yr; PROJ's deformation reads no other). The record of each case is written from the
ADR's rules (crs_vertical_cases.py's way and pipeline, a `vgridshift` with multiplier 1 from the source's orthometric
heights and −1 to the target's, a `deformation` with `+dt` years in Earth-centred coordinates of the source's ellipsoid
before the datum shift) and run through PROJ (pyproj) with the grids under their SHA-256 names: PROJ's answer is the
case's. Every answer is checked a second way: the point and its ellipsoidal height through PROJ's 3D operation, N read
by this script's own bilinear interpolation of the grid's values, H = h − N; the velocity by its own east, north, up
turned into Earth-centred coordinates. Grid coordinates within 1e-6 m, heights within 1e-6 m, degrees within 1e-10.
"""

import hashlib
import json
import math
import shutil
import struct
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_vertical_cases as cv  # noqa: E402
from crs_vertical_cases import n, pt, step, warning, failed, case, SAME, ENGINE  # noqa: E402
import pyproj  # noqa: E402
from pyproj import Transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
GRIDS = ROOT / "fixtures" / "geoid" / "v1"
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.crs.transform.geoid.json"

# The grids: Türkiye's geoid at 0.25°, its velocities at 0.5°, a global geoid at 1°.
TR = (25.0, 35.0, 0.25, 85, 33)          # west, south, step, cols, rows
VEL = (25.0, 35.0, 0.5, 43, 17)
GLOBAL = (0.0, -90.0, 1.0, 360, 181)


def n_tr(lon, lat):
    return 25.0 + 8.0 * math.sin(math.radians((lon - 25.0) * 17.0)) * math.cos(math.radians((lat - 35.0) * 23.0)) \
        + 0.3 * (lon - 25.0)


def n_global(lon, lat):
    return 20.0 * math.sin(math.radians(lon)) * math.cos(math.radians(lat)) + 10.0 * math.cos(math.radians(2 * lat))


def velocity(lon, lat):
    return (-22.0 + 0.4 * (lon - 35.0) + 0.2 * (lat - 39.0), 8.0 - 0.3 * (lat - 39.0), -1.5 + 0.1 * (lon - 35.0))


def f32(v):
    return struct.unpack(">f", struct.pack(">f", v))[0]


def nodes(grid, fn, hole=None):
    """The values row by row from the south (float32), None where a hole leaves no value."""
    west, south, d, cols, rows = grid
    out = []
    for r in range(rows):
        for c in range(cols):
            lon, lat = west + c * d, south + r * d
            out.append(None if hole and hole(lon, lat) else fn(lon, lat))
    return out


def hole(lon, lat):
    return lon >= 45.5 and lat >= 42.5


def write_gtx(path, grid, values):
    west, south, d, cols, rows = grid
    with open(path, "wb") as f:
        f.write(struct.pack(">dddd", south, west, d, d))
        f.write(struct.pack(">ii", rows, cols))
        for v in values:
            f.write(struct.pack(">f", -88.8888 if v is None else v))


def write_tif(path, grid, bands, point):
    from osgeo import gdal, osr
    import numpy as np
    gdal.UseExceptions()
    west, south, d, cols, rows = grid
    ds = gdal.GetDriverByName("GTiff").Create(str(path), cols, rows, len(bands), gdal.GDT_Float32)
    # The pixels' areas round the nodes; a pixel point's file says its node (GDAL writes the tiepoint there).
    north = south + d * (rows - 1)
    ds.SetGeoTransform((west - d / 2, d, 0.0, north + d / 2, 0.0, -d))
    sr = osr.SpatialReference()
    sr.ImportFromEPSG(4326)
    ds.SetProjection(sr.ExportToWkt())
    if point:
        ds.SetMetadataItem("AREA_OR_POINT", "Point")
    names = ["east_velocity", "north_velocity", "up_velocity"]
    for b, fn in enumerate(bands):
        a = np.array([[fn(west + c * d, north - r * d) for c in range(cols)] for r in range(rows)], dtype=np.float32)
        band = ds.GetRasterBand(b + 1)
        band.WriteArray(a)
        if len(bands) > 1:
            band.SetDescription(names[b])
    if len(bands) > 1:
        ds.SetMetadataItem("TYPE", "VELOCITY")
        ds.SetMetadataItem("UNITTYPE", "millimetres per year")
    ds = None


FILES = {
    "tg.gtx": ("geoid", lambda p: write_gtx(p, TR, nodes(TR, n_tr, hole))),
    "tg-alan.tif": ("geoid", lambda p: write_tif(p, TR, [n_tr], False)),
    "tg-nokta.tif": ("geoid", lambda p: write_tif(p, TR, [n_tr], True)),
    "egm-1.gtx": ("geoid", lambda p: write_gtx(p, GLOBAL, nodes(GLOBAL, n_global))),
    "hiz.tif": ("velocity", lambda p: write_tif(p, VEL, [lambda a, b, k=k: velocity(a, b)[k] for k in range(3)], False)),
}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def ext(name):
    return name.rsplit(".", 1)[1]


# ── Its own reading of a grid: bilinear between the nodes, as the ADR says ────────────────────────────────────────


def bilinear(grid, values, lon, lat, wrap=False):
    west, south, d, cols, rows = grid
    fx = ((lon - west) % 360.0) / d if wrap else (lon - west) / d
    fy = (lat - south) / d
    if not wrap and not (0 <= fx <= cols - 1):
        return None
    if not (0 <= fy <= rows - 1):
        return None
    i, j = min(int(math.floor(fx)), cols - 1), min(int(math.floor(fy)), rows - 1)
    i1 = (i + 1) % cols if wrap else min(i + 1, cols - 1)
    j1 = min(j + 1, rows - 1)
    u, v = fx - i, fy - j
    a, b, c, e = values[j * cols + i], values[j * cols + i1], values[j1 * cols + i], values[j1 * cols + i1]
    if None in (a, b, c, e):
        return None
    a, b, c, e = f32(a), f32(b), f32(c), f32(e)
    low, high = a + (b - a) * u, c + (e - c) * u
    return low + (high - low) * v


TR_VALUES = nodes(TR, n_tr, hole)
TR_FULL = nodes(TR, n_tr)
GLOBAL_VALUES = nodes(GLOBAL, n_global)
VEL_VALUES = [nodes(VEL, lambda a, b, k=k: velocity(a, b)[k]) for k in range(3)]


def own_n(file, lon, lat):
    if file == "tg.gtx":
        return bilinear(TR, TR_VALUES, lon, lat)
    if file == "egm-1.gtx":
        return bilinear(GLOBAL, GLOBAL_VALUES, lon, lat, wrap=True)
    return bilinear(TR, TR_FULL, lon, lat)


def own_velocity(file, lon, lat):
    v = [bilinear(VEL, VEL_VALUES[k], lon, lat) for k in range(3)]
    return v


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────


def model(file, name):
    # Before --files has written them the grids are not there yet: a placeholder, never written out.
    return {"name": name, "sha256": sha(GRIDS / file) if (GRIDS / file).exists() else "0" * 64}


TG = model("tg.gtx", "TG-20 (sentetik)")
PROJECT_HEIGHTS = {"kind": "orthometric", "verticalSrid": 5775, "geoid": TG}
SETUP = json.loads(json.dumps(cv.SETUP))
SETUP["name"] = "Geoit ve hız modeli"
SETUP["settings"]["srid"] = 5256
SETUP["settings"]["heights"] = PROJECT_HEIGHTS
del SETUP["settings"]["datumTransforms"]
SETUP["origin"] = {"x": 487200, "y": 4420600}
ELL = {"kind": "ellipsoidal"}


def ortho(v, g=None):
    h = {"kind": "orthometric", "verticalSrid": v}
    if g:
        h["geoid"] = g
    return h


def file_of(g):
    for f in FILES:
        if sha(GRIDS / f) == g["sha256"]:
            return f
    return None


def record(src, dst, keep, from_g, to_g, vel, unit=None):
    """The ADR's record with geoids and a velocity: crs_vertical_cases.operation's way, the grids' steps inserted."""
    rec = cv.operation(src, dst, keep and vel is None, unit)
    pipe = rec["pipeline"].split(" +step ")[1:]
    steps = rec["steps"]
    src_e = cv.entry(src)
    # After the source's unprojection (one step), in the ADR's order.
    at, s_at = 1, (0 if src_e["kind"] == "geographic" else 1)
    extra, extra_steps = [], []
    if keep and vel is not None and not any(p.startswith("+proj=push") for p in pipe):
        extra += ["+proj=push +v_3", "+proj=set +v_3=0"]
    if from_g:
        extra.append(f"+proj=vgridshift +grids={from_g['sha256']}.{ext(file_of(from_g))} +multiplier=1")
        extra_steps.append({"kind": "geoid", "name": from_g["name"], "reverse": False})
    if vel:
        e = cv.ell(cv.ELLIPSOIDS[cv.datum(src)])
        extra += [f"+proj=cart {e}",
                  f"+proj=deformation +grids={vel[0]['sha256']}.{ext(file_of(vel[0]))} +dt={n(vel[1])} {e}",
                  f"+inv +proj=cart {e}"]
        extra_steps.append({"kind": "velocity", "name": vel[0]["name"], "reverse": False})
    pipe[at:at] = extra
    steps[s_at:s_at] = extra_steps
    if keep and vel is not None and not any(p.startswith("+proj=pop") for p in pipe):
        # Before the target's projection (the last step, or the last but the unit's).
        last = len(pipe) - (2 if unit else 1)
        pipe.insert(last, "+proj=pop +v_3")
    if to_g:
        last = len(pipe) - (2 if unit else 1)
        pipe.insert(last, f"+proj=vgridshift +grids={to_g['sha256']}.{ext(file_of(to_g))} +multiplier=-1")
        dst_e = cv.entry(dst)
        k = len(steps) - (1 if dst_e["kind"] != "geographic" else 0) - (1 if unit else 0)
        steps.insert(k, {"kind": "geoid", "name": to_g["name"], "reverse": False})
    rec["pipeline"] = "+proj=pipeline" + "".join(f" +step {x}" for x in pipe)
    grids = []
    if from_g:
        grids.append({"name": from_g["name"], "sha256": from_g["sha256"]})
    if vel:
        grids.append({"name": vel[0]["name"], "sha256": vel[0]["sha256"]})
    if to_g:
        grids.append({"name": to_g["name"], "sha256": to_g["sha256"]})
    rec["grids"] = grids
    if vel:
        rec.pop("accuracy", None)
        rec["epochApplied"] = True
    return rec


def geographic(srid, x, y, h=0.0):
    """A point's latitude and longitude on its own datum (PROJ)."""
    e = cv.entry(srid)
    if e["kind"] == "geographic":
        return x, y
    t = Transformer.from_crs(f"EPSG:{srid}", pyproj.CRS.from_epsg(srid).geodetic_crs, always_xy=True)
    return t.transform(x, y)


def resolve(side):
    side = side or {}
    own = "srid" not in side
    srid = side.get("srid", 5256)
    heights = side.get("heights", PROJECT_HEIGHTS if own else None)
    return srid, heights, side.get("epoch")


def transformed(points, frm=None, to=None, vel=None, op="execute", warn=(), unreached_at=()):
    src, sh, se = resolve(frm)
    dst, dh, de = resolve(to)
    has_z = any("z" in p for p in points)
    from_g = sh.get("geoid") if has_z and sh and sh["kind"] == "orthometric" and dh and dh.get("verticalSrid") != sh.get("verticalSrid") else None
    to_g = dh.get("geoid") if has_z and dh and dh["kind"] == "orthometric" and sh and sh.get("verticalSrid") != dh.get("verticalSrid") else None
    through = has_z and sh is not None and dh is not None and sh["kind"] != "unknown" and (
        sh["kind"] == dh["kind"] == "ellipsoidal" or from_g or to_g)
    method = "none" if not has_z else ("geoid" if (from_g or to_g) else ("ellipsoidal" if through else
                                                                          ("same" if sh and dh else "unchanged")))
    from_m = cv.VERTICAL[sh["verticalSrid"]][3] if from_g else 1.0
    to_m = cv.VERTICAL[dh["verticalSrid"]][3] if to_g else 1.0
    epochs = se is not None and de is not None and se != de
    v = (vel, de - se) if (vel and epochs) else None
    rec = record(src, dst, not through, from_g, to_g, v)
    pipeline = Transformer.from_pipeline(rec["pipeline"])
    geographic_out = cv.entry(dst)["kind"] == "geographic"
    tol = 1e-10 if geographic_out else 1e-6
    out, unreached = [], []
    for i, q in enumerate(points):
        z = q.get("z")
        px, py, pz = pipeline.transform(q["x"], q["y"], (z * from_m) if z is not None else 0.0)
        if i in unreached_at:
            assert not (math.isfinite(px) and math.isfinite(py)) or not math.isfinite(pz) or px == float("inf"), (px, py, pz)
            out.append(None)
            unreached.append({"index": i, "reason": "outsideGrid"})
            continue
        assert math.isfinite(px) and math.isfinite(py), (rec["pipeline"], px, py)
        z2 = (pz / to_m) if (z is not None and through) else z
        # The second way: PROJ's 3D operation with this script's own N and velocity.
        if through:
            lon, lat = geographic(src, q["x"], q["y"])
            h = z * from_m + (own_n(file_of(from_g), lon, lat) if from_g else 0.0)
            x, y = q["x"], q["y"]
            if v:
                x, y, h = own_move(src, x, y, h, v)
            t3 = cv.proj_3d(src, dst)
            ox, oy, oh = t3(x, y, h)
            if to_g:
                tlon, tlat = geographic(dst, ox, oy)
                oh -= own_n(file_of(to_g), tlon, tlat)
            assert abs(ox - px) < tol and abs(oy - py) < tol and abs(oh - pz) < 1e-6, (ox - px, oy - py, oh - pz)
        out.append(pt(px, py, z2))
    rec.update({"from": cv.side_title(src, sh, se), "to": cv.side_title(dst, dh, de), "heights": method})
    if method == "ellipsoidal" and rec.get("accuracy") is not None:
        rec["heightAccuracy"] = rec["accuracy"]
    if method == "same":
        rec["heightAccuracy"] = 0.0
    if se is not None:
        rec["fromEpoch"] = se
    te = de if de is not None else se
    if te is not None:
        rec["toEpoch"] = te
    warnings = list(warn)
    if method == "unchanged":
        warnings.append(cv.HEIGHTS_UNCHANGED)
    if (se is not None or te is not None) and not v:
        warnings.append(cv.EPOCH_NOT_APPLIED)
    if rec["unofficial"]:
        warnings.append(warning("unofficial_operation", cv.UNOFFICIAL, "to"))
    if unreached:
        warnings.append(warning("points_outside", f"{len(unreached)} noktanın hedef sistemde değeri yok (izdüşümün ya "
                                "da ızgaranın dışında); yerleri boş.", "points"))
    report = {"source": {}, "topologyTolerance": 0.001, "displayDecimals": 3, "displayStep": 0.001}
    if rec.get("accuracy") is not None:
        report["transformation"] = rec["accuracy"]
    if rec.get("heightAccuracy") is not None:
        report["transformationVertical"] = rec["heightAccuracy"]
    inp = {"points": points}
    for k, val in (("from", frm), ("to", to), ("velocity", vel)):
        if val is not None:
            inp[k] = val
    return {"op": op, "input": inp,
            "result": {"status": "completed",
                       "output": {"points": out, "unreached": unreached, "operation": rec, "report": report},
                       "warnings": warnings},
            "expect": SAME, "within": tol}


def own_move(srid, x, y, h, v):
    """The point and height moved along the velocity grid: its own reading, turned into Earth-centred coordinates."""
    model, years = v
    lon, lat = geographic(srid, x, y)
    ve, vn, vu = own_velocity(file_of(model), lon, lat)
    e, nn, u = ve / 1000 * years, vn / 1000 * years, vu / 1000 * years
    a, rf = cv.ELLIPSOIDS[cv.datum(srid)]
    cart = pyproj.Transformer.from_pipeline(f"+proj=cart +a={n(a)} +rf={n(rf)}")
    X, Y, Z = cart.transform(lon, lat, h)
    p, l = math.radians(lat), math.radians(lon)
    X += -math.sin(l) * e - math.sin(p) * math.cos(l) * nn + math.cos(p) * math.cos(l) * u
    Y += math.cos(l) * e - math.sin(p) * math.sin(l) * nn + math.cos(p) * math.sin(l) * u
    Z += math.cos(p) * nn + math.sin(p) * u
    lon2, lat2, h2 = cart.transform(X, Y, Z, direction="INVERSE")
    if cv.entry(srid)["kind"] == "geographic":
        return lon2, lat2, h2
    t = Transformer.from_crs(pyproj.CRS.from_epsg(srid).geodetic_crs, f"EPSG:{srid}", always_xy=True)
    x2, y2 = t.transform(lon2, lat2)
    return x2, y2, h2


def refused(points, code, message, path, frm=None, to=None, vel=None):
    inp = {"points": points}
    for k, val in (("from", frm), ("to", to), ("velocity", vel)):
        if val is not None:
            inp[k] = val
    return {"op": "execute", "input": inp, "result": failed(code, message, path), "expect": SAME}


def tm36(lat, lon):
    t = Transformer.from_crs("EPSG:5252", "EPSG:5256", always_xy=True)
    return list(t.transform(lon, lat))


KAYSERI, SIVAS, KARS = (38.7312, 35.4787), (39.7477, 37.0179), (42.95, 45.8)


def cases():
    k, s = tm36(*KAYSERI), tm36(*SIVAS)
    ed50 = cv.place(2320, 39.9, 31.2)
    tg_area, tg_point = model("tg-alan.tif", "TG-20 (GeoTIFF, alan)"), model("tg-nokta.tif", "TG-20 (GeoTIFF, nokta)")
    egm = model("egm-1.gtx", "EGM (sentetik, 1°)")
    hiz = model("hiz.tif", "Anadolu hız alanı (sentetik)")
    out = []
    out.append(case("Elipsoit yüksekliğinden projenin ortometrik kotuna: GTX geoidiyle H = h − N",
                    transformed([pt(*k, 1100.0), pt(*s, 1350.5)], frm={"srid": 5256, "heights": ELL})))
    out.append(case("Aynısının planı: aynı cevap, çizim değişmez",
                    transformed([pt(*k, 1100.0)], frm={"srid": 5256, "heights": ELL}, op="plan")))
    out.append(case("Projenin ortometrik kotundan elipsoit yüksekliğine: h = H + N",
                    transformed([pt(*k, 1072.4)], to={"srid": 5256, "heights": ELL})))
    out.append(case("GeoTIFF geoidi, piksel alanı: düğümler piksellerin ortası",
                    transformed([pt(*k, 1100.0)], frm={"srid": 5256, "heights": ELL},
                                to={"srid": 5256, "heights": ortho(5775, tg_area)})))
    out.append(case("GeoTIFF geoidi, piksel noktası: düğümler bağlama noktaları, aynı cevap",
                    transformed([pt(*k, 1100.0)], frm={"srid": 5256, "heights": ELL},
                                to={"srid": 5256, "heights": ortho(5775, tg_point)})))
    out.append(case("ED50 / TM30 elipsoit yüksekliğinden TUREF / TM36 ortometrik kota: Helmert'ler ve geoit",
                    transformed([pt(*ed50, 900.0)], frm={"srid": 2320, "heights": ELL})))
    out.append(case("İki düşey datum, iki geoit: Antalya kotundan EGM kotuna elipsoit üzerinden",
                    transformed([pt(*k, 1072.4)], to={"srid": 5256, "heights": ortho(3855, egm)})))
    out.append(case("Ayak birimli ortometrik kot (MSL ft) geoitle elipsoit yüksekliğine",
                    transformed([pt(*k, 3500.0)], frm={"srid": 5256, "heights": ortho(8050, TG)},
                                to={"srid": 5256, "heights": ELL})))
    out.append(case("WGS 84 coğrafiden küresel geoide; ızgara 0–359° boylamda, batıdaki boylam 360 eklenerek sarar",
                    transformed([pt(35.5, 39.0, 1000.0), pt(-0.5, 51.5, 80.0), pt(179.5, -33.5, 12.0)],
                                frm={"srid": 4326, "heights": ELL}, to={"srid": 4326, "heights": ortho(3855, egm)})))
    out.append(case("Geoit ızgarasının dışı: değeri yok, öbürü dönüşür",
                    transformed([pt(60.0, 39.0, 500.0), pt(35.5, 39.0, 1000.0)],
                                frm={"srid": 4326, "heights": ELL}, to={"srid": 4326, "heights": ortho(5775, TG)},
                                unreached_at=(0,))))
    out.append(case("Değeri olmayan geoit düğümü (−88,8888): değeri yok",
                    transformed([pt(KARS[1], KARS[0], 2000.0)], frm={"srid": 4326, "heights": ELL},
                                to={"srid": 4326, "heights": ortho(5775, TG)}, unreached_at=(0,))))
    out.append(case("Hız ızgarasıyla 2005,0'dan 2026,5'e, elipsoit yüksekliğiyle: doğu, kuzey ve yukarı",
                    transformed([pt(*k, 1100.0), pt(*s, 1350.5)], frm={"srid": 5256, "heights": ELL, "epoch": 2005.0},
                                to={"srid": 5256, "heights": ELL, "epoch": 2026.5}, vel=hiz)))
    out.append(case("Hız ızgarasıyla, kotsuz: yalnız yatay",
                    transformed([pt(*k)], frm={"srid": 5256, "epoch": 2005.0}, to={"srid": 5256, "epoch": 2026.5},
                                vel=hiz)))
    out.append(case("Hız ve datum: ED50 / TM30'da 1990'dan TUREF / TM36'da 2024'e",
                    transformed([pt(*ed50, 900.0)], frm={"srid": 2320, "heights": ELL, "epoch": 1990.0},
                                to={"srid": 5256, "heights": ELL, "epoch": 2024.0}, vel=hiz)))
    out.append(case("Aynı düşey sistemde ortometrik kot hızla kalır (kenarda tutulur)",
                    transformed([pt(*k, 1072.4)], frm={"epoch": 2005.0}, to={"epoch": 2020.0}, vel=hiz)))
    out.append(case("Aynı epok iki yanda, hız verilmiş: uygulanmaz, uyarıyla",
                    transformed([pt(*k)], frm={"srid": 5256, "epoch": 2023.5}, to={"srid": 5256, "epoch": 2023.5},
                                vel=hiz)))
    # Refusals.
    fake = "ab" * 32
    out.append(case("Ortometrik yanın geoidi yok: geoit gerekir",
                    refused([pt(*k, 1100.0)], "geoid_required",
                            "Kotlar ortometrik, Antalya height (EPSG:5775) ile elipsoit yüksekliği arasında "
                            "çevrilmeli: bunun için to tarafının geoit modeli SHA-256'sıyla gerekir "
                            "(heights.geoid.sha256). Kotlar sıfır geoit ayrımıyla çevrilmez: geoit ızgarasını (GTX ya "
                            "da GeoTIFF) Proje ayarları › Izgaralar'a ekleyip adını ve özetini verin.",
                            "to.heights.geoid", frm={"srid": 5256, "heights": ELL},
                            to={"srid": 5256, "heights": ortho(5775)})))
    out.append(case("Geoidin yalnız adı var, özeti yok: geoit gerekir",
                    refused([pt(*k, 1072.4)], "geoid_required",
                            "Kotlar ortometrik, Antalya height (EPSG:5775), geoit TG-20 ile elipsoit yüksekliği "
                            "arasında çevrilmeli: bunun için from tarafının geoit modeli SHA-256'sıyla gerekir "
                            "(heights.geoid.sha256). Kotlar sıfır geoit ayrımıyla çevrilmez: geoit ızgarasını (GTX ya "
                            "da GeoTIFF) Proje ayarları › Izgaralar'a ekleyip adını ve özetini verin.",
                            "from.heights.geoid", frm={"srid": 5256, "heights": ortho(5775, {"name": "TG-20"})},
                            to={"srid": 5256, "heights": ELL})))
    out.append(case("Adı geçen geoit bu cihazda yok",
                    refused([pt(*k, 1100.0)], "geoid_missing",
                            f"“TG-99” geoit modelinin ızgarası ({fake[:12]}…) bu cihazın ızgara kitaplığında yok. "
                            "Izgarayı Proje ayarları › Izgaralar'a ekleyin; daha düşük doğruluklu bir yola geçilmez.",
                            "to.heights.geoid", frm={"srid": 5256, "heights": ELL},
                            to={"srid": 5256, "heights": ortho(5775, {"name": "TG-99", "sha256": fake})})))
    out.append(case("Geoit diye bir hız ızgarası verilmiş",
                    refused([pt(*k, 1100.0)], "geoid_missing",
                            f"“Yanlış” geoit modelinin ızgarası ({hiz['sha256'][:12]}…) okunamadı: bir hız ızgarası, "
                            "geoit modeli değil. Izgarayı Proje ayarları › Izgaralar'a ekleyin; daha düşük doğruluklu "
                            "bir yola geçilmez.", "to.heights.geoid", frm={"srid": 5256, "heights": ELL},
                            to={"srid": 5256, "heights": ortho(5775, {"name": "Yanlış", "sha256": hiz["sha256"]})})))
    out.append(case("İki düşey datum, yalnız birinin geoidi var",
                    refused([pt(*k, 1072.4)], "vertical_unsupported",
                            "Kotlar “ortometrik, Antalya height (EPSG:5775), geoit TG-20 (sentetik)” ile “ortometrik, "
                            "EGM2008 height (EPSG:3855)” arasında çevrilmeli: iki ayrı düşey datum. Bunun için iki "
                            "yanın da geoit modeli SHA-256'sıyla gerekir (heights.geoid); PROJ'un yaptığı gibi kotları "
                            "olduğu gibi bırakan “kaba” bir yol kullanılmaz.", "to.heights.verticalSrid",
                            to={"srid": 5256, "heights": ortho(3855)})))
    out.append(case("İki ayrı epok, hız verilmemiş",
                    refused([pt(*k)], "epoch_unsupported",
                            "Koordinatlar 2005 epokundan 2024 epokuna taşınmalı: bunun için hız modeli gerekir. Hız "
                            "ızgarasını (doğu, kuzey, yukarı mm/yıl; GeoTIFF) Proje ayarları › Izgaralar'a ekleyip "
                            "velocity ile verin; koordinatlar epok farkı yok sayılarak çevrilmez.", "to.epoch",
                            frm={"srid": 5256, "epoch": 2005.0}, to={"srid": 5256, "epoch": 2024.0})))
    out.append(case("Hız ızgarası bu cihazda yok",
                    refused([pt(*k)], "velocity_missing",
                            f"“TUSAGA hızları” hız modelinin ızgarası ({fake[:12]}…) bu cihazın ızgara kitaplığında "
                            "yok. Izgarayı Proje ayarları › Izgaralar'a ekleyin; daha düşük doğruluklu bir yola geçilmez.",
                            "velocity", frm={"srid": 5256, "epoch": 2005.0}, to={"srid": 5256, "epoch": 2024.0},
                            vel={"name": "TUSAGA hızları", "sha256": fake})))
    out.append(case("Hız diye bir geoit verilmiş",
                    refused([pt(*k)], "velocity_missing",
                            f"“Yanlış” hız modelinin ızgarası ({TG['sha256'][:12]}…) okunamadı: bir geoit modeli, hız "
                            "ızgarası değil. Izgarayı Proje ayarları › Izgaralar'a ekleyin; daha düşük doğruluklu bir "
                            "yola geçilmez.", "velocity", frm={"srid": 5256, "epoch": 2005.0},
                            to={"srid": 5256, "epoch": 2024.0}, vel={"name": "Yanlış", "sha256": TG["sha256"]})))
    out.append(case("Hız ızgarasının özeti geçersiz",
                    refused([pt(*k)], "invalid_epoch",
                            "Hız ızgarasının adı boş olmamalı ve SHA-256'sı küçük harfli 64 onaltılık rakam olmalı.",
                            "velocity", frm={"srid": 5256, "epoch": 2005.0}, to={"srid": 5256, "epoch": 2024.0},
                            vel={"name": "Hız", "sha256": "ABC"})))
    return out


def files():
    GRIDS.mkdir(parents=True, exist_ok=True)
    for name, (_, write) in FILES.items():
        write(GRIDS / name)
        print(f"{(GRIDS / name).relative_to(ROOT)}  {sha(GRIDS / name)}")


def build():
    # PROJ reads the grids by the names the record gives them: their SHA-256.
    tmp = Path(tempfile.mkdtemp(prefix="kentos-geoid-"))
    for name in FILES:
        shutil.copy(GRIDS / name, tmp / f"{sha(GRIDS / name)}.{ext(name)}")
    pyproj.datadir.append_data_dir(str(tmp))
    try:
        return {
            "format": "kentos.command-cases", "version": 1, "command": "cad.crs.transform", "commandVersion": 1,
            "title": "Koordinat dönüştür: geoit modeli ve hız ızgarası",
            "note": "ADR 0631. Izgaralar fixtures/geoid/v1'de (GDAL ve struct), sayılar PROJ'dan (vgridshift, "
                    "deformation; ızgaralar SHA-256 adlarıyla) ve ikinci yoldan: PROJ'un 3B işlemi, betiğin kendi çift "
                    "doğrusal N'i ve hızı. Proje: TUREF / TM36, Antalya kotu (sentetik TG geoidi). Çalıştırıcılar "
                    "ızgaraları bu klasörden okur. Bağımsız başvuru: scripts/fixtures/geoid_cases.py.",
            "grids": {name: sha(GRIDS / name) for name in FILES},
            "setup": SETUP, "cases": cases(),
        }
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main():
    if "--files" in sys.argv[1:]:
        files()
        return 0
    built = build()
    text = json.dumps(built, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"geoit ve hız durumları tutarlı: {len(built['cases'])} durum (PROJ {pyproj.proj_version_str})")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(built['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
