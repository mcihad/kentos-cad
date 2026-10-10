#!/usr/bin/env python3
"""Mesh ve çok boyutlu veri (docs/adr/0243): NetCDF classic files, their CF grids and UGRID meshes read as rasters, and 2DM
with ASCII DAT made one UGRID file, written from the ADR with Python, without KentOS code; the cases the formats core plays
(crates/shared/formats/tests/all/multidim.rs).

- The files: written by netcdf_classic.py (this reference's own NetCDF writer, from Unidata's format notes) and by hand
  (2DM, DAT, broken files).
- Grids: the ADR's rules (§3): the axes, the regular step, the half-cell shift, rising y turned round, values unpacked
  (raw × scale + offset in float64 in that order), fills, the default fill and the valid range made NaN, the samples' type;
  coarse levels the mean of the 2 × 2 finer ones (NaN left out, in their order). Cross-checked against GDAL's netCDF
  driver (libnetcdf) where GDAL reads the same: its geotransform, nodata and samples.
- Meshes (§4, §5): the faces' triangles (a convex face fanned from its first node after a clockwise one is turned round;
  concave faces only carry face values in the cases, whatever their triangles), a pixel's centre (the engine's float64
  formula) located exactly with fractions (the lowest triangle whose closed area holds it), node values interpolated
  exactly, a vector's magnitude at 50 digits; the raster's samples within one unit in the last place of their type.
  Cross-checked against QGIS's MDAL (2DM, DAT and UGRID values at points) when QGIS is installed.
- 2DM and DAT (§4): read here; the UGRID file written here with the ADR's layout, byte for byte what the engine writes.

    python3 scripts/fixtures/multidim_cases.py          # write fixtures/multidim/v1 (files and cases.json)
    python3 scripts/fixtures/multidim_cases.py --check  # compare
"""

import json
import math
import os
import struct
import subprocess
import sys
import tempfile
from fractions import Fraction
from pathlib import Path

import mpmath

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netcdf_classic as nc  # noqa: E402

mpmath.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/multidim/v1"
FILES = DIR / "files"
OUT = DIR / "cases.json"
NAN = math.nan
F_FILL = struct.unpack(">f", struct.pack(">f", 9.9692099683868690e36))[0]
D_FILL = 9.9692099683868690e36
TILE = 256
DAY = 86_400_000


def f32(v):
    return struct.unpack(">f", struct.pack(">f", v))[0]


def jnum(v):
    """A number as JSON holds it: NaN as null."""
    return None if isinstance(v, float) and math.isnan(v) else v


# ---------------------------------------------------------------------------------------------------------------------------
# Time (§7)

def days_from_civil(y, m, d):
    y = y - 1 if m <= 2 else y
    era = y // 400
    yoe = y - era * 400
    mp = (m + 9) % 12
    doy = (153 * mp + 2) // 5 + d - 1
    doe = yoe * 365 + yoe // 4 - yoe // 100 + doy
    return era * 146097 + doe - 719468


def days_in(y, m):
    if m == 2:
        return 29 if (y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)) else 28
    return 30 if m in (4, 6, 9, 11) else 31


UNITS = {**{k: 1 for k in ("millisecond", "milliseconds", "msec", "msecs", "ms")},
         **{k: 1000 for k in ("second", "seconds", "sec", "secs", "s")},
         **{k: 60_000 for k in ("minute", "minutes", "min", "mins")},
         **{k: 3_600_000 for k in ("hour", "hours", "hr", "hrs", "h")},
         **{k: DAY for k in ("day", "days", "d")}}


def round_away(x):
    return math.floor(x + 0.5) if x >= 0 else -math.floor(-x + 0.5)


def origin_ms(text):
    text = text.strip()
    cut = min([i for i in (text.find(" "), text.find("T")) if i >= 0], default=-1)
    date, rest = (text, "") if cut < 0 else (text[:cut], text[cut + 1:].strip())
    try:
        y, m, d = (int(p) for p in date.split("-"))
    except ValueError:
        return None
    if not (1 <= m <= 12) or d < 1 or d > days_in(y, m):
        return None
    ms = days_from_civil(y, m, d) * DAY
    if not rest:
        return ms
    cut = min([i for i, c in enumerate(rest) if c in " Zz+-"], default=len(rest))
    clock, zone = rest[:cut], rest[cut:].strip()
    f = clock.split(":")
    try:
        h = int(f[0])
        mi = int(f[1]) if len(f) > 1 else 0
        sec = float(f[2]) if len(f) > 2 else 0.0
    except ValueError:
        return None
    if len(f) > 3 or not (0 <= h <= 23) or not (0 <= mi <= 59) or not (0 <= sec < 60):
        return None
    ms += h * 3_600_000 + mi * 60_000 + round_away(sec * 1000.0)
    z = zone.upper()
    if z in ("", "Z", "UTC", "GMT"):
        off = 0
    else:
        sign = {"+": 1, "-": -1}.get(z[0])
        if sign is None:
            return None
        digits = z[1:]
        if ":" in digits:
            zh, zm = (int(p) for p in digits.split(":"))
        elif len(digits) > 2:
            zh, zm = int(digits[:-2]), int(digits[-2:])
        else:
            zh, zm = int(digits), 0
        off = sign * (zh * 3_600_000 + zm * 60_000)
    return ms - off


def cf_time(units, calendar):
    cal = (calendar or "").strip().lower()
    if cal not in ("", "standard", "gregorian", "proleptic_gregorian"):
        return None
    low = units.strip().lower()
    at = low.find(" since ")
    if at < 0:
        return None
    unit = UNITS.get(units.strip()[:at].strip().lower())
    origin = origin_ms(units.strip()[at + 7:])
    if unit is None or origin is None:
        return None
    if cal != "proleptic_gregorian" and origin < days_from_civil(1582, 10, 15) * DAY:
        return None
    return unit, origin


def moment(t, value):
    unit, origin = t
    return float(origin + round_away(value * unit))


# ---------------------------------------------------------------------------------------------------------------------------
# The files

def var(name, dims, kind, values, attrs=None):
    return nc.Var(name, dims, kind, values, attrs or {})


def grid_files():
    """The grids' files: (name, bytes, what they hold for the cases)."""
    out = {}
    # 1. CDF-1: lat rising (turned round), lon, a record time (3 hours), a level (2); packed short with a fill and a
    #    valid range; a float without a fill holding the default fill; a raw int with a fill; a transposed variable;
    #    a variable without coordinates on its last two dimensions.
    lat = [36.0, 36.25, 36.5]
    lon = [32.0, 32.25, 32.5, 32.75]
    t2m = []
    for t in range(3):
        for lev in range(2):
            for j in range(3):
                for i in range(4):
                    t2m.append(-20 + t * 7 + lev * 3 + j * 4 + i)
    t2m[5] = -30000          # the fill
    t2m[17] = 9999           # past the valid range
    pr = [float(k) * 0.5 for k in range(3 * 3 * 4)]
    pr[7] = F_FILL
    land = [(k * 7) % 5 for k in range(12)]
    land[3] = -1
    vs = [
        var("lat", ["lat"], "float", lat, {"units": "degrees_north", "standard_name": "latitude"}),
        var("lon", ["lon"], "float", lon, {"units": "degrees_east"}),
        var("level", ["level"], "double", [850.0, 500.0], {"units": "hPa", "positive": "down"}),
        var("time", ["time"], "double", [0.0, 6.0, 12.5], {"units": "hours since 2024-05-01 00:00:00", "calendar": "gregorian"}),
        var("t2m", ["time", "level", "lat", "lon"], "short", t2m,
            {"long_name": "2 metre sıcaklık", "units": "K", "scale_factor": ("float", [0.5]), "add_offset": ("float", [280.0]),
             "_FillValue": ("short", [-30000]), "valid_range": ("short", [-1000, 1000])}),
        var("pr", ["time", "lat", "lon"], "float", pr, {"units": "mm"}),
        var("land", ["lat", "lon"], "int", land, {"_FillValue": ("int", [-1])}),
        var("swapped", ["lon", "lat"], "float", [float(k) for k in range(12)]),
    ]
    out["grid-cdf1.nc"] = nc.write(None, 1, [("lat", 3), ("lon", 4), ("level", 2), ("time", 0)],
                                   {"Conventions": "CF-1.8", "title": "deneme"}, vs, numrecs=3)
    # 2. CDF-2: a projected grid (x rising, y falling: north first), its system in crs_wkt, days with a zone, a double
    #    packed by double attributes, NaN and missing_value.
    xs = [500010.0, 500030.0, 500050.0, 500070.0, 500090.0]
    ys = [4420050.0, 4420030.0, 4420010.0]
    dem = []
    for t in range(2):
        for j in range(3):
            for i in range(5):
                dem.append(100.0 + t * 1000 + j * 10 + i + 0.25)
    dem[3] = NAN
    dem[9] = -9999.0
    wkt = ('PROJCS["TUREF / TM33",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,298.257222101]],'
           'PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","5252"]],'
           'PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",33],'
           'PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],'
           'UNIT["metre",1,AUTHORITY["EPSG","9001"]],AUTHORITY["EPSG","5254"]]')
    vs = [
        var("crs", [], "int", [0], {"grid_mapping_name": "transverse_mercator", "crs_wkt": wkt}),
        var("x", ["x"], "double", xs, {"standard_name": "projection_x_coordinate", "units": "m"}),
        var("y", ["y"], "double", ys, {"standard_name": "projection_y_coordinate", "units": "m"}),
        var("t", ["t"], "double", [0.5, 1.0], {"units": "days since 2000-01-01T00:00:00+03:00"}),
        var("dem", ["t", "y", "x"], "double", dem, {"grid_mapping": "crs", "missing_value": ("double", [-9999.0])}),
        var("slope", ["y", "x"], "short", [k * 10 for k in range(15)],
            {"grid_mapping": "crs", "scale_factor": ("double", [0.01]), "add_offset": ("double", [0.0])}),
    ]
    out["grid-cdf2.nc"] = nc.write(None, 2, [("x", 5), ("y", 3), ("t", 2)], {"Conventions": "CF-1.8"}, vs)
    # 3. CDF-5: the unsigned and 64-bit types, an unknown calendar (the dimension is no time), streaming record count.
    vs = [
        var("y", ["y"], "double", [10.0, 20.0], {"axis": "Y"}),
        var("x", ["x"], "double", [1.0, 2.0, 3.0], {"axis": "X"}),
        var("step", ["step"], "int", [0, 1], {"units": "days since 2000-01-01", "calendar": "noleap"}),
        var("u8", ["step", "y", "x"], "ubyte", [250, 251, 252, 253, 254, 255, 0, 1, 2, 3, 4, 5]),
        var("u16", ["y", "x"], "ushort", [65530, 1, 2, 3, 4, 5], {"_FillValue": ("ushort", [65535])}),
        var("u32", ["y", "x"], "uint", [4000000000, 1, 2, 3, 4, 5]),
        var("i64", ["y", "x"], "int64", [-(2 ** 40), 2 ** 40, 7, 8, 9, 10]),
    ]
    out["grid-cdf5.nc"] = nc.write(None, 5, [("y", 2), ("x", 3), ("step", 0)], {}, vs, numrecs=2, streaming=True)
    return out


# A small mesh: two convex quads, triangles, a concave quad (face values only in the cases) and a clockwise triangle;
# the notch of the concave quad is no face's.
MESH_NODES = [
    (500000.0, 4420000.0), (500020.0, 4420000.0), (500040.0, 4420000.0),
    (500000.0, 4420020.0), (500020.0, 4420020.0), (500040.0, 4420020.0),
    (500000.0, 4420040.0), (500020.0, 4420040.0), (500034.0, 4420026.0), (500040.0, 4420040.0),
]
# Faces with UGRID's start index 1; -1 pads a triangle.
MESH_FACES = [
    [1, 2, 5, 4],      # convex quad
    [2, 3, 6, -1],     # triangle
    [2, 6, 5, -1],     # triangle
    [4, 5, 8, 7],      # convex quad
    [5, 6, 10, 9],     # concave quad (node 9 is reflex)
    [5, 8, 10, -1],    # clockwise triangle
]


def ugrid_file():
    n = len(MESH_NODES)
    f = len(MESH_FACES)
    steps = 3
    depth = []
    for t in range(steps):
        for k in range(n):
            depth.append(float(k) * 0.25 + t)
    depth[n + 3] = F_FILL   # node 4 has no value at step 2
    level = [float(k) for k in range(f)] * steps
    ux = [float(k % 3) for k in range(n)] * steps
    uy = [float((k * 2) % 5) - 2.0 for k in range(n)] * steps
    active = [1] * f + [1, 0, 1, 1, 1, 1] + [1, 1, 1, 0, 1, 1]
    vs = [
        var("mesh", [], "int", [0], {"cf_role": "mesh_topology", "topology_dimension": ("int", [2]),
                                     "node_coordinates": "node_x node_y", "face_node_connectivity": "faces",
                                     "grid_mapping": "crs"}),
        var("crs", [], "int", [0], {"epsg_code": "EPSG:5254"}),
        var("node_x", ["node"], "double", [p[0] for p in MESH_NODES], {"standard_name": "projection_x_coordinate"}),
        var("node_y", ["node"], "double", [p[1] for p in MESH_NODES], {"standard_name": "projection_y_coordinate"}),
        var("faces", ["face", "nmax"], "int", [v for face in MESH_FACES for v in face],
            {"cf_role": "face_node_connectivity", "start_index": ("int", [1]), "_FillValue": ("int", [-1])}),
        var("time", ["time"], "double", [0.0, 1800.0, 3600.0], {"units": "seconds since 2024-05-01 06:00:00"}),
        var("depth", ["time", "node"], "float", depth, {"mesh": "mesh", "location": "node", "long_name": "Su derinliği",
                                                        "_FillValue": ("float", [F_FILL]), "kentos_mask": "active"}),
        var("active", ["time", "face"], "byte", active, {"mesh": "mesh", "location": "face"}),
        var("level", ["time", "face"], "float", level, {"mesh": "mesh", "location": "face"}),
        var("ucx", ["time", "node"], "float", ux, {"mesh": "mesh", "location": "node"}),
        var("ucy", ["time", "node"], "float", uy, {"mesh": "mesh", "location": "node"}),
    ]
    dims = [("node", n), ("face", f), ("nmax", 4), ("time", steps)]
    return nc.write(None, 2, dims, {"Conventions": "CF-1.8 UGRID-1.0"}, vs)


def ugrid_calc_file():
    """The mesh of ugrid.nc with what Mesh hesaplayıcı reads (§10): a static dataset, a face dataset masked by a DAT's
    activity, a vector, and a layered dataset it refuses."""
    n, f, steps = len(MESH_NODES), len(MESH_FACES), 3
    depth = [float(k) * 0.25 + t for t in range(steps) for k in range(n)]
    depth[n + 3] = F_FILL
    bed = [100.0 + 0.125 * k for k in range(n)]
    stage = [10.0 + k * 0.5 + t for t in range(steps) for k in range(f)]
    active = [1] * f + [1, 0, 1, 1, 1, 1] + [1, 1, 1, 0, 1, 1]
    level = [float(k) for k in range(f)] * steps
    ux = [float(k % 3) for k in range(n)] * steps
    uy = [float((k * 2) % 5) - 2.0 for k in range(n)] * steps
    salt = [30.0 + k + 0.5 * lay + t for t in range(steps) for lay in range(2) for k in range(n)]
    vs = [
        var("mesh", [], "int", [0], {"cf_role": "mesh_topology", "topology_dimension": ("int", [2]),
                                     "node_coordinates": "node_x node_y", "face_node_connectivity": "faces",
                                     "grid_mapping": "crs"}),
        var("crs", [], "int", [0], {"epsg_code": "EPSG:5254"}),
        var("node_x", ["node"], "double", [q[0] for q in MESH_NODES], {"standard_name": "projection_x_coordinate"}),
        var("node_y", ["node"], "double", [q[1] for q in MESH_NODES], {"standard_name": "projection_y_coordinate"}),
        var("faces", ["face", "nmax"], "int", [v for face in MESH_FACES for v in face],
            {"cf_role": "face_node_connectivity", "start_index": ("int", [1]), "_FillValue": ("int", [-1])}),
        var("time", ["time"], "double", [0.0, 1800.0, 3600.0], {"units": "seconds since 2024-05-01 06:00:00"}),
        var("depth", ["time", "node"], "float", depth, {"mesh": "mesh", "location": "node", "long_name": "Su derinliği",
                                                        "_FillValue": ("float", [F_FILL])}),
        var("bed", ["node"], "double", bed, {"mesh": "mesh", "location": "node", "long_name": "Taban kotu"}),
        var("stage", ["time", "face"], "float", stage, {"mesh": "mesh", "location": "face", "kentos_mask": "active"}),
        var("active", ["time", "face"], "byte", active),
        var("level", ["time", "face"], "float", level, {"mesh": "mesh", "location": "face"}),
        var("ucx", ["time", "node"], "float", ux, {"mesh": "mesh", "location": "node"}),
        var("ucy", ["time", "node"], "float", uy, {"mesh": "mesh", "location": "node"}),
        var("salt", ["time", "layer", "node"], "float", salt, {"mesh": "mesh", "location": "node"}),
    ]
    dims = [("node", n), ("face", f), ("nmax", 4), ("time", steps), ("layer", 2)]
    return nc.write(None, 2, dims, {"Conventions": "CF-1.8 UGRID-1.0"}, vs)


# The 2DM QGIS's MDAL reads as the engine does: nodes and elements numbered in order (MDAL refuses nodes out of order and
# DATs on numbers with gaps, and keeps elements in the file's order where the engine takes them by number).
MESH_2DM = """MESH2D
MESHNAME "deneme"
NUM_MATERIALS_PER_ELEM 1
E4Q 1 1 2 5 4 1
E3T 2 2 3 6 1
E3T 3 2 6 5 1
ND 1 0.0 0.0 10.0
ND 2 20.0 0.0 11.5
ND 3 40.0 0.0 12.0
ND 4 0.0 20.0 9.0
ND 5 20.0 20.0 9.5
ND 6 40.0 20.0 8.25
"""

# The engine's own rules where MDAL refuses or differs: numbers with gaps and out of order, elements by number, a line
# element left out, a second-order triangle by its corners.
MESH_2DM_QUIRKS = """MESH2D
E3T 7 3 9 5 2
E4Q 2 1 3 5 4 1
E6T 4 3 10 11 12 13 9 1
E2L 9 1 3 1
ND 4 0 10 1
ND 1 0 0 2
ND 3 10 0 3
ND 5 10 10 4
ND 9 20 0 5
ND 10 25 0 0
ND 11 30 0 0
ND 12 30 5 0
ND 13 25 5 0
"""

DAT_SCALAR = """DATASET
OBJTYPE "mesh2d"
BEGSCL
ND 6
NC 3
NAME "Su derinliği"
TIMEUNITS Hours
TS 0 0.0
0.0
0.5
1.0
1.5
2.0
2.5
TS 1 1.5
1
0
1
0.1
0.6
1.1
1.6
2.1
2.6
ENDDS
"""

DAT_VECTOR = """DATASET
OBJTYPE "mesh2d"
BEGVEC
ND 6
NC 3
NAME "Hız"
TIMEUNITS Minutes
RT_JULIAN 2460431.75
TS 0 0
1.0 0.0
0.0 2.0
3.0 4.0
-1.0 -1.0
0.5 0.5
2.0 -2.0
TS 0 30
2.0 0.0
0.0 3.0
3.0 -4.0
1.0 1.0
0.0 0.0
1.0 1.0
ENDDS
"""


def gdal_files():
    """Files GDAL's netCDF driver wrote (libnetcdf): made once and kept, so the cases read a real writer's conventions."""
    out = {}
    for name in ("gdal-cdf1.nc", "gdal-cdf2.nc"):
        if (FILES / name).exists():
            out[name] = (FILES / name).read_bytes()
    if len(out) == 2:
        return out
    import numpy as np
    from osgeo import gdal, osr
    gdal.UseExceptions()
    with tempfile.TemporaryDirectory() as tmp:
        # A GeoTIFF in TUREF / TM33 made a classic NetCDF by gdal.Translate (bottom-up rows, crs_wkt, GeoTransform).
        src = gdal.GetDriverByName("MEM").Create("", 5, 4, 1, gdal.GDT_Int16)
        src.SetGeoTransform([500000.0, 10.0, 0.0, 4420040.0, 0.0, -10.0])
        srs = osr.SpatialReference()
        srs.ImportFromEPSG(5254)
        src.SetProjection(srs.ExportToWkt())
        band = src.GetRasterBand(1)
        band.WriteArray(np.array([[k + 5 * j for k in range(5)] for j in range(4)], dtype=np.int16))
        band.SetNoDataValue(-999)
        band.WriteArray(np.array([[-999, 1, 2, 3, 4], [5, 6, 7, 8, 9], [10, 11, -999, 13, 14], [15, 16, 17, 18, 19]], dtype=np.int16))
        path = os.path.join(tmp, "a.nc")
        gdal.Translate(path, src, format="netCDF", creationOptions=["FORMAT=NC"])
        out["gdal-cdf1.nc"] = Path(path).read_bytes()
        # The multidimensional API: time, y and x, a float with a fill, its system.
        path = os.path.join(tmp, "b.nc")
        ds = gdal.GetDriverByName("netCDF").CreateMultiDimensional(path, [], ["FORMAT=NC2"])
        rg = ds.GetRootGroup()
        dt = rg.CreateDimension("time", "TEMPORAL", None, 3)
        dy = rg.CreateDimension("y", "HORIZONTAL_Y", None, 3)
        dx = rg.CreateDimension("x", "HORIZONTAL_X", None, 4)
        f64 = gdal.ExtendedDataType.Create(gdal.GDT_Float64)
        f32 = gdal.ExtendedDataType.Create(gdal.GDT_Float32)
        txt = gdal.ExtendedDataType.CreateString()
        t = rg.CreateMDArray("time", [dt], f64)
        t.Write(np.array([0.0, 30.0, 60.0]))
        a = t.CreateAttribute("units", [], txt)
        a.Write("minutes since 2024-05-01 06:00:00")
        y = rg.CreateMDArray("y", [dy], f64)
        y.Write(np.array([4420025.0, 4420015.0, 4420005.0]))
        a = y.CreateAttribute("standard_name", [], txt)
        a.Write("projection_y_coordinate")
        x = rg.CreateMDArray("x", [dx], f64)
        x.Write(np.array([500005.0, 500015.0, 500025.0, 500035.0]))
        a = x.CreateAttribute("standard_name", [], txt)
        a.Write("projection_x_coordinate")
        v = rg.CreateMDArray("temp", [dt, dy, dx], f32)
        v.SetNoDataValueDouble(-1.0)
        v.SetSpatialRef(srs)
        v.Write(np.arange(36, dtype=np.float32).reshape(3, 3, 4) * 0.5 - np.float32(1.0) * (np.arange(36).reshape(3, 3, 4) == 7))
        ds = None
        out["gdal-cdf2.nc"] = Path(path).read_bytes()
    return out


def broken_files():
    good = grid_files()["grid-cdf2.nc"]
    return {
        "bad-hdf5.nc": b"\x89HDF\r\n\x1a\n" + b"\0" * 32,
        "bad-grib.nc": b"GRIB" + b"\0" * 32,
        "bad-other.nc": b"PK\x03\x04" + b"\0" * 32,
        "bad-header.nc": good[:40],
        "bad-data.nc": good[:len(good) - 16],
        "bad-type.nc": bad_type(good),
    }


def bad_type(good):
    """The first variable's type made 9 (uint) in a CDF-2 file, which has no such type."""
    h = nc.read_header(good)
    # The type of `crs` sits after its name, dims and attributes: find its code (int 4) right before its vsize and begin.
    b = bytearray(good)
    first = h.vars[0]
    # Search the header for the int code 4 followed by vsize 4 and begin (8 bytes) = first["begin"].
    pat = struct.pack(">i", 4) + struct.pack(">i", 4) + struct.pack(">q", first["begin"])
    at = bytes(b).find(pat)
    b[at:at + 4] = struct.pack(">i", 9)
    return bytes(b)


# ---------------------------------------------------------------------------------------------------------------------------
# Grids (§3)

def unpack_rule(attrs, kind):
    """The engine's unpacking: (packed, scale, offset, fills, valid, sample)."""
    sc, of = attrs.get("scale_factor"), attrs.get("add_offset")
    packed = sc is not None or of is not None
    fills = []
    for k in ("_FillValue", "missing_value"):
        if k in attrs:
            fills += list(attrs[k])
    if not fills and kind in ("float", "double"):
        fills = [F_FILL if kind == "float" else D_FILL]
    valid = None
    if "valid_range" in attrs and len(attrs["valid_range"]) >= 2:
        valid = (attrs["valid_range"][0], attrs["valid_range"][1])
    elif "valid_min" in attrs or "valid_max" in attrs:
        valid = (attrs.get("valid_min", [-math.inf])[0], attrs.get("valid_max", [math.inf])[0])
    if packed:
        akind = (sc or of)[0] if isinstance(sc or of, tuple) else None
        sample = "f32" if akind == "float" else "f64"
    else:
        sample = {"byte": "i8", "ubyte": "u8", "char": "u8", "short": "i16", "ushort": "u16", "int": "i32", "uint": "u32",
                  "float": "f32", "double": "f64", "int64": "f64", "uint64": "f64"}[kind]
    def num(a):
        return a[1][0] if isinstance(a, tuple) else a[0]
    return {
        "packed": packed,
        "scale": num(sc) if sc is not None else 1.0,
        "offset": num(of) if of is not None else 0.0,
        "fills": fills,
        "valid": valid,
        "sample": sample,
        "nodata": None if sample.startswith("f") else (fills[0] if fills else None),
    }


def unpacked(rule, raw):
    if rule["sample"].startswith("f"):
        if math.isnan(raw) or raw in rule["fills"] or (rule["valid"] and (raw < rule["valid"][0] or raw > rule["valid"][1])):
            return NAN
        v = raw * rule["scale"] + rule["offset"] if rule["packed"] else raw
        return f32(v) if rule["sample"] == "f32" else v
    return raw


def attr_values(a):
    """An attribute as the writer took it, its numbers as a list."""
    if isinstance(a, tuple):
        return list(a[1])
    if isinstance(a, list):
        return a
    return a


# ---------------------------------------------------------------------------------------------------------------------------
# Meshes (§4, §5)

def orient(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def triangles(nodes, faces):
    """(triangle nodes counter-clockwise, its face) in the engine's order; a concave face's triangles any valid ones."""
    out = []
    for fi, face in enumerate(faces):
        pts = [tuple(Fraction(c) for c in nodes[k]) for k in face]
        twice = sum(orient(pts[0], pts[i], pts[i + 1]) for i in range(1, len(pts) - 1))
        ring = list(face) if twice > 0 else list(reversed(face))
        p = [tuple(Fraction(c) for c in nodes[k]) for k in ring]
        m = len(ring)
        convex = all(orient(p[i], p[(i + 1) % m], p[(i + 2) % m]) >= 0 for i in range(m))
        if convex:
            for i in range(1, m - 1):
                t = (ring[0], ring[i], ring[i + 1])
                if orient(*(tuple(Fraction(c) for c in nodes[k]) for k in t)) > 0:
                    out.append((t, fi, True))
        else:
            # Ear clipping; the cases use these triangles only for face values.
            idx = list(range(m))
            guard = 0
            while len(idx) > 3 and guard < 1000:
                guard += 1
                for a in range(len(idx)):
                    i0, i1, i2 = idx[a - 1], idx[a], idx[(a + 1) % len(idx)]
                    if orient(p[i0], p[i1], p[i2]) <= 0:
                        continue
                    if any(inside(p[j], p[i0], p[i1], p[i2]) for j in idx if j not in (i0, i1, i2)):
                        continue
                    out.append(((ring[i0], ring[i1], ring[i2]), fi, False))
                    idx.pop(a)
                    break
            out.append(((ring[idx[0]], ring[idx[1]], ring[idx[2]]), fi, False))
    return out


def inside(q, a, b, c):
    return orient(a, b, q) >= 0 and orient(b, c, q) >= 0 and orient(c, a, q) >= 0


def locate(nodes, tris, x, y):
    q = (Fraction(x), Fraction(y))
    for ti, (t, fi, exact) in enumerate(tris):
        a, b, c = (tuple(Fraction(v) for v in nodes[k]) for k in t)
        if inside(q, a, b, c):
            return ti
    return None


def mesh_value(nodes, tris, data, x, y):
    """The value at (x, y) as an exact Fraction or mpf (a vector), None when nothing; and whether it is exact."""
    ti = locate(nodes, tris, x, y)
    if ti is None:
        return None
    t, fi, convex = tris[ti]
    if data.get("mask") is not None and not data["mask"][fi]:
        return None
    if data["location"] == "face":
        u = data["x"][fi]
        if u is None:
            return None
        if data.get("y") is None:
            return Fraction(u)
        v = data["y"][fi]
        if v is None:
            return None
        return mpmath.sqrt(mpmath.mpf(u) ** 2 + mpmath.mpf(v) ** 2)
    if not convex:
        raise ValueError("node values in a concave face depend on its triangles")
    q = (Fraction(x), Fraction(y))
    a, b, c = (tuple(Fraction(v) for v in nodes[k]) for k in t)
    wa, wb, wc = orient(q, b, c), orient(q, c, a), orient(q, a, b)
    s = wa + wb + wc

    def lerp(vals):
        vs = [vals[k] for k in t]
        if any(v is None for v in vs):
            return None
        return (wa * Fraction(vs[0]) + wb * Fraction(vs[1]) + wc * Fraction(vs[2])) / s

    u = lerp(data["x"])
    if u is None:
        return None
    if data.get("y") is None:
        return u
    v = lerp(data["y"])
    if v is None:
        return None
    return mpmath.sqrt(mpmath.mpf(u.numerator) / u.denominator * mpmath.mpf(u.numerator) / u.denominator
                       + mpmath.mpf(v.numerator) / v.denominator * mpmath.mpf(v.numerator) / v.denominator)


def to_float(v):
    if v is None:
        return NAN
    if isinstance(v, Fraction):
        return v.numerator / v.denominator if abs(v.denominator) < 2 ** 1000 else float(v)
    return float(v)


def grid_tile(nodes, tris, data, affine, level, tx, ty, lw, lh, sample):
    f = float(1 << level)
    x0, a, b, y0, c, d = affine
    tw = min(TILE, lw - tx * TILE)
    th = min(TILE, lh - ty * TILE)
    out = []
    for j in range(th):
        v = (float(ty * TILE + j) + 0.5) * f
        for i in range(tw):
            u = (float(tx * TILE + i) + 0.5) * f
            px = x0 + a * u + b * v
            py = y0 + c * u + d * v
            val = to_float(mesh_value(nodes, tris, data, px, py))
            out.append(f32(val) if sample == "f32" and not math.isnan(val) else val)
    return tw, th, out


def level_sizes(w, h):
    sizes = [(w, h)]
    while sizes[-1][0] > TILE or sizes[-1][1] > TILE:
        lw, lh = sizes[-1]
        sizes.append((max(1, (lw + 1) // 2), max(1, (lh + 1) // 2)))
    return sizes


# ---------------------------------------------------------------------------------------------------------------------------
# 2DM, DAT and the UGRID file the engine writes (§4)

def read_2dm(text):
    nodes, elements, skipped = [], [], 0
    lines = text.splitlines()
    assert lines[0].split()[0].upper() == "MESH2D"
    corners = {"E3T": (3, [0, 1, 2]), "E4Q": (4, [0, 1, 2, 3]), "E6T": (6, [0, 2, 4]), "E8Q": (8, [0, 2, 4, 6]),
               "E9Q": (9, [0, 2, 4, 6])}
    for line in lines[1:]:
        f = line.split()
        if not f:
            continue
        card = f[0].upper()
        if card == "ND":
            nodes.append((int(f[1]), float(f[2]), float(f[3]), float(f[4]) if len(f) > 4 else 0.0))
        elif card in corners:
            n, cs = corners[card]
            ids = [int(v) for v in f[2:2 + n]]
            elements.append((int(f[1]), [ids[c] for c in cs]))
        elif card in ("E2L", "E3L"):
            skipped += 1
    nodes.sort()
    index = {t[0]: k for k, t in enumerate(nodes)}
    elements.sort()
    return {
        "x": [t[1] for t in nodes], "y": [t[2] for t in nodes], "z": [t[3] for t in nodes],
        "faces": [[index[c] for c in cs] for _, cs in elements], "skipped": skipped,
    }


def read_dat(text):
    out, cur = [], None
    lines = text.splitlines()
    i = 0
    units = {"hours": 3_600_000, "hour": 3_600_000, "h": 3_600_000, "minutes": 60_000, "minute": 60_000, "min": 60_000,
             "seconds": 1000, "second": 1000, "sec": 1000, "s": 1000, "days": DAY, "day": DAY, "d": DAY}
    while i < len(lines):
        f = lines[i].split()
        i += 1
        if not f:
            continue
        card = f[0].upper()
        if card == "DATASET":
            cur = {"name": "", "vector": False, "count": 0, "cells": 0, "unit": 3_600_000, "reference": None, "steps": []}
        elif card in ("BEGSCL", "BEGVEC"):
            cur["vector"] = card == "BEGVEC"
        elif card == "ND":
            cur["count"] = int(f[1])
        elif card == "NC":
            cur["cells"] = int(f[1])
        elif card == "NAME":
            cur["name"] = lines[i - 1].strip()[4:].strip().strip('"').strip()
        elif card == "TIMEUNITS":
            cur["unit"] = units[f[1].lower()]
        elif card == "RT_JULIAN":
            cur["reference"] = float(round_away((float(f[1]) - 2440587.5) * 86_400_000.0))
        elif card == "TS":
            istat, time = int(f[1]), float(f[2])
            active = None
            if istat == 1:
                active = []
                while len(active) < cur["cells"]:
                    active += [1 if int(v) != 0 else 0 for v in lines[i].split()]
                    i += 1
                active = active[:cur["cells"]]
            per = 2 if cur["vector"] else 1
            values = []
            while len(values) < cur["count"] * per:
                values += [f32(float(v)) for v in lines[i].split()]
                i += 1
            cur["steps"].append({"time": time, "values": values[:cur["count"] * per], "active": active})
        elif card == "ENDDS":
            out.append(cur)
            cur = None
    if cur:
        out.append(cur)
    return out


def variable_name(shown):
    table = {"ç": "c", "Ç": "C", "ğ": "g", "Ğ": "G", "ı": "i", "İ": "I", "ö": "o", "Ö": "O", "ş": "s", "Ş": "S", "ü": "u", "Ü": "U"}
    s = "".join(table.get(c, c if (c.isascii() and (c.isalnum() or c == "_")) else "_") for c in shown.strip())
    if not (s[:1].isascii() and s[:1].isalpha()):
        s = "v_" + s
    return s[:200]


def ugrid_bytes(x, y, faces, geographic, epsg, times, datasets, title):
    """The engine's UGRID layout (write.rs): its dimensions, variables, attributes and values in its order."""
    n, f = len(x), len(faces)
    most = max(3, max(len(face) for face in faces))
    dims = [("nMesh2d_node", n), ("nMesh2d_face", f), ("max_nMesh2d_face_nodes", most)]
    time_dims = []
    for k, t in enumerate(times):
        dims.append(("time" if k == 0 else f"time_{k + 1}", len(t["values"])))
        time_dims.append(dims[-1][0])
    vs = [var("Mesh2d", [], "int", [0], {"cf_role": "mesh_topology", "long_name": "Topology data of 2D mesh",
                                          "topology_dimension": ("int", [2]),
                                          "node_coordinates": "Mesh2d_node_x Mesh2d_node_y",
                                          "face_node_connectivity": "Mesh2d_face_nodes", "face_dimension": "nMesh2d_face"})]
    xs, ys, ux, uy = ("longitude", "latitude", "degrees_east", "degrees_north") if geographic else (
        "projection_x_coordinate", "projection_y_coordinate", "m", "m")
    vs.append(var("Mesh2d_node_x", ["nMesh2d_node"], "double", list(x), {"standard_name": xs, "units": ux}))
    vs.append(var("Mesh2d_node_y", ["nMesh2d_node"], "double", list(y), {"standard_name": ys, "units": uy}))
    conn = [v for face in faces for v in list(face) + [-1] * (most - len(face))]
    vs.append(var("Mesh2d_face_nodes", ["nMesh2d_face", "max_nMesh2d_face_nodes"], "int", conn,
                  {"cf_role": "face_node_connectivity", "start_index": ("int", [0]), "_FillValue": ("int", [-1])}))
    if epsg is not None:
        vs.append(var("crs", [], "int", [0], {"epsg_code": f"EPSG:{epsg}"}))
        vs[0].attrs["grid_mapping"] = "crs"
    for k, t in enumerate(times):
        attrs = {}
        if t["absolute"]:
            attrs["standard_name"] = "time"
        attrs["long_name"] = "Zaman"
        attrs["units"] = "seconds since 1970-01-01 00:00:00" if t["absolute"] else "hours"
        if t["absolute"]:
            attrs["calendar"] = "standard"
        vals = [ms / 1000.0 for ms in t["values"]] if t["absolute"] else list(t["values"])
        vs.append(var(time_dims[k], [time_dims[k]], "double", vals, attrs))
    for d in datasets:
        vd = ([time_dims[d["time"]]] if d["time"] is not None else []) + (["nMesh2d_node"] if d["location"] == "node" else ["nMesh2d_face"])
        kind = "byte" if d["is_mask"] else ("double" if d["double"] else "float")
        attrs = {"long_name": d["long_name"], "mesh": "Mesh2d", "location": d["location"]}
        if not d["is_mask"]:
            attrs["_FillValue"] = (kind, [F_FILL if kind == "float" else D_FILL])
        if d["mask"]:
            attrs["kentos_mask"] = d["mask"]
        if epsg is not None:
            attrs["grid_mapping"] = "crs"
        fill = F_FILL if kind == "float" else D_FILL
        values = [fill if (isinstance(v, float) and math.isnan(v)) else v for v in d["values"]]
        if kind == "byte":
            values = [int(v) for v in values]
        vs.append(var(d["name"], vd, kind, values, attrs))
    gattrs = {"Conventions": "CF-1.8 UGRID-1.0", "title": title, "source": "KentOS"}
    return nc.write(None, 2, dims, gattrs, vs)


def sms_ugrid(mesh_text, dat_texts, start, epsg):
    """2DM and its DATs as the engine writes them (write.rs `write_sms`), and its report."""
    mesh = read_2dm(mesh_text)
    n, f = len(mesh["x"]), len(mesh["faces"])
    report = {"nodes": n, "faces": f, "datasets": ["Taban kotu"], "notes": []}
    if mesh["skipped"]:
        report["notes"].append(f"{mesh['skipped']} çizgi elemanı atlandı.")
    alls = []
    for text in dat_texts:
        for d in read_dat(text):
            loc = "node" if d["count"] == n else "face"
            alls.append((d, loc))
    times, axis_of = [], []
    for d, _ in alls:
        base = start if start is not None else d["reference"]
        vals = [(base + round_away(s["time"] * d["unit"])) if base is not None else s["time"] * d["unit"] / 3_600_000.0
                for s in d["steps"]]
        t = {"values": [float(v) for v in vals], "absolute": base is not None}
        if t in times:
            axis_of.append(times.index(t))
        else:
            times.append(t)
            axis_of.append(len(times) - 1)
    if any(not t["absolute"] for t in times) and alls:
        report["notes"].append("Başlangıç zamanı verilmedi: zamanlar başlangıçtan saat; zaman sürgüsü izlenemez.")
    taken = ["taban_kotu"]
    fixed = {"Mesh2d", "Mesh2d_node_x", "Mesh2d_node_y", "Mesh2d_face_nodes", "crs", "time"}

    def unique(base):
        name, k = base, 2
        while name in taken or name in fixed or name.startswith("time_"):
            name = f"{base}_{k}"
            k += 1
        taken.append(name)
        return name

    datasets = [{"name": "taban_kotu", "long_name": "Taban kotu", "location": "node", "double": True, "time": None,
                 "mask": None, "is_mask": False, "values": list(mesh["z"])}]
    for k, (d, loc) in enumerate(alls):
        base = unique(variable_name(d["name"]))
        masked = any(s["active"] is not None for s in d["steps"])
        mask = f"{base}_etkin" if masked else None
        comps = [(0, "_x"), (1, "_y")] if d["vector"] else [(0, "")]
        per = 2 if d["vector"] else 1
        for c, suffix in comps:
            values = [v for s in d["steps"] for v in s["values"][c::per]]
            datasets.append({"name": base + suffix,
                             "long_name": f"{d['name']} ({'x' if c == 0 else 'y'})" if d["vector"] else d["name"],
                             "location": loc, "double": False, "time": axis_of[k], "mask": mask, "is_mask": False,
                             "values": values})
        if mask:
            values = [v for s in d["steps"] for v in (s["active"] if s["active"] is not None else [1] * d["cells"])]
            datasets.append({"name": mask, "long_name": f"{d['name']} etkinliği", "location": "face", "double": False,
                             "time": axis_of[k], "mask": None, "is_mask": True, "values": values})
        report["datasets"].append(f"{d['name']} (vektör)" if d["vector"] else d["name"])
    data = ugrid_bytes(mesh["x"], mesh["y"], mesh["faces"], False, epsg, times, datasets, "2DM ve DAT")
    return data, report


# ---------------------------------------------------------------------------------------------------------------------------
# The reference's own reading of a grid file (§3), from the header its reader gives

EAST = {"degrees_east", "degree_east", "degree_e", "degrees_e", "degreee", "degreese"}
NORTH = {"degrees_north", "degree_north", "degree_n", "degrees_n", "degreen", "degreesn"}


def text_attr(v, k):
    a = v["attrs"].get(k)
    return a.rstrip("\0") if isinstance(a, str) else None


def axis_by_name(name):
    n = name.lower()
    if n in ("x", "easting"):
        return "X"
    if n in ("y", "northing"):
        return "Y"
    if n in ("lon", "longitude", "long"):
        return "Lon"
    if n in ("lat", "latitude"):
        return "Lat"
    return "Other"


def axis_of_var(v):
    units = (text_attr(v, "units") or "").strip().lower()
    std = (text_attr(v, "standard_name") or "").strip().lower()
    ax = (text_attr(v, "axis") or "").strip().upper()
    if std == "longitude" or units in EAST:
        return "Lon"
    if std == "latitude" or units in NORTH:
        return "Lat"
    if std in ("projection_x_coordinate", "grid_longitude") or ax == "X":
        return "X"
    if std in ("projection_y_coordinate", "grid_latitude") or ax == "Y":
        return "Y"
    return axis_by_name(v["name"])


def coordinate_of(h, d):
    name = h.dims[d][0]
    for v in h.vars:
        if v["name"] == name and v["dims"] == [d] and v["kind"] != 2:
            return v
    return None


def axis_of(h, d):
    c = coordinate_of(h, d)
    if c is not None:
        a = axis_of_var(c)
        if a != "Other":
            return a
    return axis_by_name(h.dims[d][0])


def is_x(a):
    return a in ("X", "Lon")


def is_y(a):
    return a in ("Y", "Lat")


def regular(values):
    n = len(values)
    if n < 2 or not all(math.isfinite(v) for v in values):
        return None
    step = (values[-1] - values[0]) / (n - 1)
    if not (math.isfinite(step) and step != 0):
        return None
    tol = abs(step) * 1e-3
    if all(abs(v - (values[0] + k * step)) <= tol for k, v in enumerate(values)):
        return values[0], step
    return None


def epsg_of_wkt(wkt):
    depth, found, i = 0, None, 0
    while i < len(wkt):
        c = wkt[i]
        if c in "[(":
            depth += 1
        elif c in "])":
            depth = max(0, depth - 1)
        elif c == '"':
            i += 1
            while i < len(wkt):
                if wkt[i] == '"':
                    if i + 1 < len(wkt) and wkt[i + 1] == '"':
                        i += 1
                    else:
                        break
                i += 1
        elif depth == 1:
            up = wkt[i:i + 10].upper()
            k = 10 if up.startswith("AUTHORITY[") else (3 if up.startswith("ID[") else None)
            if k:
                inner = wkt[i + k:]
                inner = inner[:inner.find("]")] if "]" in inner else inner
                parts = [p.strip().strip('"') for p in inner.split(",")]
                if parts and parts[0].upper() == "EPSG" and len(parts) > 1:
                    try:
                        found = int(parts[1])
                    except ValueError:
                        found = None
        i += 1
    return found


def crs_of(h, v):
    gm = text_attr(v, "grid_mapping")
    if not gm:
        return None
    name = gm.strip().replace(":", " ").split()[0]
    m = next((x for x in h.vars if x["name"] == name), None)
    if m is None:
        return None
    wkt = text_attr(m, "crs_wkt") or text_attr(m, "spatial_ref")
    if wkt and epsg_of_wkt(wkt) is not None:
        return epsg_of_wkt(wkt)
    code = text_attr(m, "epsg_code")
    if code and code.strip().upper().startswith("EPSG:"):
        try:
            return int(code.strip()[5:])
        except ValueError:
            return None
    a = m["attrs"].get("epsg_code")
    return int(a[0]) if isinstance(a, list) and a else None


KIND_NAMES = {1: "byte", 2: "char", 3: "short", 4: "int", 5: "float", 6: "double", 7: "ubyte", 8: "ushort", 9: "uint",
              10: "int64", 11: "uint64"}


def attrs_for_rule(v):
    """A variable's attributes as unpack_rule takes them: numbers as lists, packing kinds kept."""
    out = {}
    for k, a in v["attrs"].items():
        out[k] = a
    sc, of = v["attrs"].get("scale_factor"), v["attrs"].get("add_offset")
    # unpack_rule reads the packing's kind from a (kind, values) tuple: the header gives lists, so mark them.
    return out


def rule_of(v, kinds):
    """unpack_rule from the header's attributes and their kinds."""
    attrs = {}
    for k, a in v["attrs"].items():
        if isinstance(a, list):
            attrs[k] = (KIND_NAMES[kinds[(v["name"], k)]], a) if k in ("scale_factor", "add_offset") else a
    return unpack_rule(attrs, KIND_NAMES[v["kind"]])


def attr_kinds(data):
    """The kinds of every numeric attribute of every variable, read from the header bytes again."""
    h = nc.read_header(data)
    five = h.version == 5
    kinds = {}
    # Walk the header once more, noting each variable attribute's type.
    pos = 4 + (8 if five else 4)

    def take(fmt):
        nonlocal pos
        v = struct.unpack_from(">" + fmt, data, pos)[0]
        pos += struct.calcsize(">" + fmt)
        return v

    def count():
        return take("q") if five else take("i")

    def name():
        nonlocal pos
        n = count()
        s = data[pos:pos + n].decode("utf-8")
        pos += nc.pad4(n)
        return s

    def attrs(owner):
        nonlocal pos
        take("i")
        n = count()
        for _ in range(n):
            k = name()
            kind = take("i")
            m = count()
            size = nc.TYPES[kind][1]
            pos += nc.pad4(m * size)
            kinds[(owner, k)] = kind

    take("i")
    for _ in range(count()):
        name()
        count()
    attrs(None)
    take("i")
    for _ in range(count()):
        vn = name()
        for _ in range(count()):
            count()
        attrs(vn)
        take("i")
        count()
        take("q") if h.version != 1 else take("i")
    return kinds


def show_time(t, seconds):
    """The time slider's `show` (time.rs): GG.AA.YYYY SS:DD, with :ss when seconds are shown."""
    ms = int(t)
    days, into = divmod(ms, DAY)
    z = days + 719468
    era = z // 146097
    doe = z - era * 146097
    yoe = (doe - doe // 1460 + doe // 36524 - doe // 146096) // 365
    doy = doe - (365 * yoe + yoe // 4 - yoe // 100)
    mp = (5 * doy + 2) // 153
    d = doy - (153 * mp + 2) // 5 + 1
    m = mp + 3 if mp < 10 else mp - 9
    y = yoe + era * 400 + (1 if m <= 2 else 0)
    h, mi, s = into // 3_600_000, into % 3_600_000 // 60_000, into % 60_000 // 1000
    day = f"{d:02}.{m:02}.{y:04}"
    return f"{day} {h:02}:{mi:02}:{s:02}" if seconds else f"{day} {h:02}:{mi:02}"


def shortest(v):
    """The shortest decimal that reads back to `v`, positional (Rust's `{}` of a float)."""
    import numpy as np
    s = np.format_float_positional(v, unique=True, trim="-")
    return "0" if s in ("0", "-0") else s


def dim_labels(values, time, units):
    if time:
        seconds = any(int(t) % 60_000 != 0 for t in values)
        return [show_time(t, seconds) for t in values]
    return [f"{shortest(v)} {units}" if units else shortest(v) for v in values]


def slice_dim(h, data, d):
    n = h.dims[d][1] if h.dims[d][1] else h.numrecs
    c = coordinate_of(h, d)
    units = (text_attr(c, "units") or "").strip() if c else ""
    units = units or None
    cal = text_attr(c, "calendar") if c else None
    raw = list(nc.read_var(data, h, c["name"])) if c else [float(k) for k in range(n)]
    raw = [float(x) for x in raw]
    t = cf_time(units, cal) if (c and units) else None
    if t is not None:
        m = [moment(t, x) for x in raw]
        if all(m[k] <= m[k + 1] for k in range(len(m) - 1)):
            return {"name": h.dims[d][0], "values": m, "time": True, "labels": dim_labels(m, True, None)}
    out = {"name": h.dims[d][0], "values": raw, "time": False}
    if units:
        out["units"] = units
    out["labels"] = dim_labels(raw, False, units)
    return out


def grid_infos(data):
    """The engine's CubeInfo of a grid file, by the ADR's rules (meshes listed by mesh_infos)."""
    h = nc.read_header(data)
    kinds = attr_kinds(data)
    grids, notes = [], []
    for v in h.vars:
        if v["kind"] == 2 or len(v["dims"]) < 2 or "mesh" in v["attrs"] or "cf_role" in v["attrs"]:
            continue
        a, b = axis_of(h, v["dims"][-2]), axis_of(h, v["dims"][-1])
        if not (is_y(a) and is_x(b)):
            if is_x(a) and is_y(b):
                notes.append(f"“{v['name']}”: son iki boyut X ve Y sırasında; Y ve X olmalı.")
            elif not (a == "Other" and b == "Other"):
                notes.append(f"“{v['name']}”: son iki boyutu Y ve X değil.")
            continue
        yd, xd = v["dims"][-2], v["dims"][-1]
        w = h.dims[xd][1] or h.numrecs
        hh = h.dims[yd][1] or h.numrecs
        cx, cy = coordinate_of(h, xd), coordinate_of(h, yd)
        affine = None
        if cx and cy:
            xs = [float(x) for x in nc.read_var(data, h, cx["name"])]
            ys = [float(x) for x in nc.read_var(data, h, cy["name"])]
            rx, ry = regular(xs), regular(ys)
            if rx and ry:
                flip = ry[1] > 0
                top = ys[-1] if flip else ry[0]
                step = abs(ry[1])
                affine = [rx[0] - rx[1] / 2.0, rx[1], 0.0, top + step / 2.0, 0.0, -step]
        geo = axis_of(h, xd) == "Lon" and axis_of(h, yd) == "Lat"
        epsg = crs_of(h, v)
        if epsg is None and geo:
            epsg = 4326
        rule = rule_of(v, kinds)
        g = {"variable": v["name"]}
        if text_attr(v, "long_name"):
            g["longName"] = text_attr(v, "long_name")
        if text_attr(v, "units"):
            g["units"] = text_attr(v, "units")
        g.update({"width": w, "height": hh})
        if affine:
            g["affine"] = affine
        if epsg is not None:
            g["epsg"] = epsg
        g.update({"geographic": geo, "sample": rule["sample"],
                  "dims": [slice_dim(h, data, d) for d in v["dims"][:-2]]})
        grids.append(g)
    return {"version": h.version, "grids": grids, "notes": notes}


def grid_region(data, variable, slice_idx, level, x, y, w, hgt):
    """Region (level, x, y, w, h) of a grid slice, its edges repeated (the engine's region())."""
    h = nc.read_header(data)
    kinds = attr_kinds(data)
    v = next(q for q in h.vars if q["name"] == variable)
    rule = rule_of(v, kinds)
    yd, xd = v["dims"][-2], v["dims"][-1]
    W = h.dims[xd][1] or h.numrecs
    H = h.dims[yd][1] or h.numrecs
    cy = coordinate_of(h, yd)
    ys = [float(q) for q in nc.read_var(data, h, cy["name"])] if cy else None
    flip = bool(ys and regular(ys) and regular(ys)[1] > 0)
    allv = nc.read_var(data, h, variable)
    # The slice's slab (C order over the slice dims).
    dims = [h.dims[d][1] or h.numrecs for d in v["dims"]]
    off = 0
    for k, i in enumerate(slice_idx):
        off = off * dims[k] + i
    off *= W * H
    level0 = []
    for r in range(H):
        fr = H - 1 - r if flip else r
        for i in range(W):
            level0.append(unpacked(rule, float(allv[off + fr * W + i])))
    float_sample = rule["sample"].startswith("f")
    assert level < len(level_sizes(W, H)), f"{variable}: no level {level}"
    levels = [(W, H, level0)]
    while len(levels) <= level:
        lw, lh, prev = levels[-1]
        nw, nh = (lw + 1) // 2, (lh + 1) // 2
        nxt = []
        nodata = rule["nodata"]
        for j in range(nh):
            for i in range(nw):
                s, n = 0.0, 0
                for di, dj in ((0, 0), (1, 0), (0, 1), (1, 1)):
                    si, sj = 2 * i + di, 2 * j + dj
                    if si >= lw or sj >= lh:
                        continue
                    val = prev[sj * lw + si]
                    if math.isnan(val) or (nodata is not None and val == nodata):
                        continue
                    s += val
                    n += 1
                if n:
                    m = s / n
                    nxt.append(f32(m) if rule["sample"] == "f32" else (m if float_sample else int(m)))
                elif nodata is not None:
                    nxt.append(nodata)
                else:
                    nxt.append(NAN if float_sample else 0)
        levels.append((nw, nh, nxt))
    lw, lh, vals = levels[level]
    out = []
    for j in range(y, y + hgt):
        for i in range(x, x + w):
            ci, cj = min(max(i, 0), lw - 1), min(max(j, 0), lh - 1)
            out.append(vals[cj * lw + ci])
    return rule["sample"], out


def big_grid():
    """A grid past 256 cells, y rising, NaN scattered and a block: for the coarse levels."""
    W, H = 520, 260
    xs = [400000.0 + 5.0 * i + 2.5 for i in range(W)]
    ys = [4500000.0 + 5.0 * j + 2.5 for j in range(H)]
    vals = []
    for j in range(H):
        for i in range(W):
            v = f32(i * 0.5 + j * 0.25 + (i * j % 7) * 0.125)
            if (j * W + i) % 37 == 0 or (100 <= i < 103 and 50 <= j < 53):
                v = NAN
            vals.append(v)
    vs = [
        var("x", ["x"], "double", xs, {"standard_name": "projection_x_coordinate", "units": "m"}),
        var("y", ["y"], "double", ys, {"standard_name": "projection_y_coordinate", "units": "m"}),
        var("h", ["y", "x"], "float", vals, {"_FillValue": ("float", [NAN]), "units": "m"}),
    ]
    return nc.write(None, 2, [("x", W), ("y", H)], {}, vs)


# Files of gigabytes whose header and coordinates alone are kept (§2): a month of hourly values over the globe at 0.25° (ERA5's
# grid; a variable of 2.9 GiB, its vsize past the 32-bit sign) and a longer run past 4 GiB (vsize 2³² − 1, as CDF-2 allows for
# its last fixed variable). The cases give the sizes the files would have; GDAL (libnetcdf) opens sparse copies of that size.
LARGE = {"grid-large.nc": 744, "grid-huge.nc": 1240}


def large_grid(steps):
    """A CDF-2 file of hourly 2 m temperatures over the globe at 0.25° for `steps` hours, its values left out."""
    vs = [
        var("lon", ["lon"], "double", [-180.0 + 0.25 * i for i in range(1440)], {"units": "degrees_east", "standard_name": "longitude"}),
        var("lat", ["lat"], "double", [90.0 - 0.25 * j for j in range(721)], {"units": "degrees_north", "standard_name": "latitude"}),
        var("time", ["time"], "double", [float(k) for k in range(steps)],
            {"units": "hours since 2024-01-01 00:00:00", "standard_name": "time"}),
        var("t2m", ["time", "lat", "lon"], "float", None, {"units": "K", "long_name": "2 metre temperature"}),
    ]
    return nc.write(None, 2, [("lon", 1440), ("lat", 721), ("time", steps)], {}, vs)


def full_size(data):
    """The size a file whose last values were left out would have: that variable's begin and its values' bytes."""
    h = nc.read_header(data)
    v = max(h.vars, key=lambda v: v["begin"])
    n = nc.TYPES[v["kind"]][1]
    for d in v["dims"]:
        n *= h.dims[d][1]
    return v["begin"] + n


def long_header_grid():
    vs = [
        var("y", ["y"], "double", [1.0, 2.0], {"axis": "Y"}),
        var("x", ["x"], "double", [1.0, 2.0], {"axis": "X"}),
        var("v", ["y", "x"], "float", [1.0, 2.0, 3.0, 4.0]),
    ]
    return nc.write(None, 2, [("y", 2), ("x", 2)], {"history": "çok uzun geçmiş " * 6000}, vs)


# ---------------------------------------------------------------------------------------------------------------------------
# The cases

MESH_AFFINE = [500000.0, 0.125, 0.0, 4420040.0, 0.0, -0.125]
MESH_SIZE = (320, 320)


def mesh_data(data, name, step):
    """A UGRID dataset's values at a step as the reference reads them (fills None), with its mask."""
    h = nc.read_header(data)
    v = next(q for q in h.vars if q["name"] == name)
    allv = nc.read_var(data, h, name)
    n = h.dims[v["dims"][-1]][1]
    vals = [float(x) for x in allv[step * n:(step + 1) * n]]
    fills = set()
    for k in ("_FillValue", "missing_value"):
        if k in v["attrs"]:
            fills |= set(v["attrs"][k])
    if not fills and v["kind"] in (5, 6):
        fills = {F_FILL if v["kind"] == 5 else D_FILL}
    vals = [None if (x in fills or math.isnan(x)) else x for x in vals]
    mask = None
    if "kentos_mask" in v["attrs"]:
        m = text_attr(v, "kentos_mask")
        mv = next(q for q in h.vars if q["name"] == m)
        f = h.dims[mv["dims"][-1]][1]
        mask = [float(x) != 0 for x in nc.read_var(data, h, m)[step * f:(step + 1) * f]]
    return vals, mask


def mesh_nodes_faces():
    nodes = MESH_NODES
    faces = [[k - 1 for k in f if k > 0] for f in MESH_FACES]
    return nodes, faces


def mesh_cases(data):
    nodes, faces = mesh_nodes_faces()
    tris = triangles(nodes, faces)
    sets = [
        ("depth-0", {"variable": "depth", "slice": [0]}, "node", [("depth", 0)]),
        ("depth-1", {"variable": "depth", "slice": [1]}, "node", [("depth", 1)]),
        ("depth-2", {"variable": "depth", "slice": [2]}, "node", [("depth", 2)]),
        ("level-2", {"variable": "level", "slice": [2]}, "face", [("level", 2)]),
        ("vector-1", {"variable": "ucx", "vector": "ucy", "slice": [1]}, "node", [("ucx", 1), ("ucy", 1)]),
    ]
    out = []
    for name, part, loc, comps in sets:
        xv, mask = mesh_data(data, comps[0][0], comps[0][1])
        dd = {"location": loc, "x": xv, "mask": mask}
        if len(comps) > 1:
            dd["y"], _ = mesh_data(data, comps[1][0], comps[1][1])
        part = {**part, "mesh": "mesh", "affine": MESH_AFFINE, "width": MESH_SIZE[0], "height": MESH_SIZE[1]}
        sizes = level_sizes(*MESH_SIZE)
        tiles = []
        for level, tx, ty, stride in ((0, 0, 0, 9), (0, 1, 0, 5), (0, 0, 1, 5), (1, 0, 0, 3)):
            lw, lh = sizes[level]
            tw = min(TILE, lw - tx * TILE)
            th = min(TILE, lh - ty * TILE)
            pixels, values = [], []
            f = float(1 << level)
            x0, a, b, y0, c, d = MESH_AFFINE
            picks = [(i, j) for j in range(0, th, stride) for i in range(0, tw, stride)]
            # Pixels on the diagonal edges too (their centres lie on them).
            picks += [(i, 479 - (tx * TILE + i) - ty * TILE) for i in range(0, tw, 11)
                      if 0 <= 479 - (tx * TILE + i) - ty * TILE < th] if level == 0 else []
            for i, j in picks:
                u = (float(tx * TILE + i) + 0.5) * f
                v = (float(ty * TILE + j) + 0.5) * f
                px, py = x0 + a * u + b * v, y0 + c * u + d * v
                try:
                    val = mesh_value(nodes, tris, dd, px, py)
                except ValueError:
                    continue
                fv = to_float(val)
                pixels.append([i, j])
                values.append(jnum(f32(fv) if not math.isnan(fv) else NAN))
            tiles.append({"level": level, "tx": tx, "ty": ty, "pixels": pixels, "values": values})
        points = [(500020.0, 4420020.0), (500010.0, 4420010.0), (500030.0, 4420010.0), (500020.0, 4420000.0),
                  (500005.0, 4420035.0), (500036.0, 4420030.0), (500031.0, 4420025.0), (500025.0, 4420035.0),
                  (500039.0, 4420021.0), (499999.0, 4420010.0), (500041.0, 4420041.0), (500030.0, 4420024.0),
                  (500022.5, 4420003.125)]
        pv, pts = [], []
        for x, y in points:
            try:
                val = mesh_value(nodes, tris, dd, x, y)
            except ValueError:
                continue
            pts.append([x, y])
            pv.append(jnum(to_float(val)))
        out.append({"name": f"mesh-{name}", "kind": "mesh", "file": "ugrid.nc", "part": part, "tiles": tiles,
                    "points": pts, "values": pv})
    return out


def mesh_info(data):
    nodes, faces = mesh_nodes_faces()
    area = Fraction(0)
    for face in faces:
        p = [tuple(Fraction(c) for c in nodes[k]) for k in face]
        area += abs(sum(orient(p[0], p[i], p[i + 1]) for i in range(1, len(p) - 1))) / 2
    mean = math.sqrt(float(area) / len(faces)) / 8.0
    e = math.floor(math.log10(mean))
    base = 10.0 ** e
    m = mean / base
    cell = (5.0 if m >= 5 else 2.0 if m >= 2 else 1.0) * base
    h = nc.read_header(data)
    tdim = next(k for k, d in enumerate(h.dims) if d[0] == "time")
    time = slice_dim(h, data, tdim)
    xs = [p[0] for p in nodes]
    ys = [p[1] for p in nodes]
    ds = [
        {"variable": "depth", "longName": "Su derinliği", "location": "node", "sample": "f32", "dims": [time]},
        {"variable": "level", "location": "face", "sample": "f32", "dims": [time]},
        {"variable": "ucx", "vector": "ucy", "location": "node", "sample": "f32", "dims": [time]},
    ]
    return {"version": 2, "grids": [], "meshes": [{
        "name": "mesh", "nodes": len(nodes), "faces": len(faces), "bbox": [min(xs), min(ys), max(xs), max(ys)],
        "cell": cell, "epsg": 5254, "geographic": False, "datasets": ds}], "notes": []}


TIME_CASES = [
    ("hours since 2024-05-01 00:00:00", None, [0.0, 6.0, 12.5]),
    ("days since 2000-01-01T00:00:00+03:00", None, [0.5, 1.0]),
    ("seconds since 1970-01-01 00:00:00 UTC", "proleptic_gregorian", [-1.5, 0.0004, 1e9]),
    ("minutes since 1990-1-1 0:0:0", "standard", [1.0, 2.5]),
    ("days since 1-1-1", "proleptic_gregorian", [0.0, 738000.0]),
    ("days since 1-1-1", "standard", [0.0]),
    ("days since 2000-01-01", "noleap", [1.0]),
    ("hours", None, [1.0]),
    ("fortnights since 2000-01-01", None, [1.0]),
    ("hours since 2000-02-30", None, [1.0]),
    ("hours since 2000-01-01 25:00", None, [1.0]),
    ("ms since 2020-03-01 12:30:15.25 -0530", None, [0.0, 1.5]),
    ("Hours Since 2024-01-01", "GREGORIAN", [1.0]),
    ("days since 2000-01-01 00:00:00 +14:30", None, [0.0]),
]


# ---------------------------------------------------------------------------------------------------------------------------
# Kesit, Zaman serisi and Mesh hesaplayıcı (§8–§10)

def grouped(n):
    s = str(n)
    return "".join(("." if i and (len(s) - i) % 3 == 0 else "") + ch for i, ch in enumerate(s))


def walk_polyline(pts, step):
    """The engine's points along a straight polyline (profile.rs): i·step short of the end, then the end: (s, x, y)."""
    cum = [0.0]
    for k in range(1, len(pts)):
        cum.append(cum[-1] + math.hypot(pts[k][0] - pts[k - 1][0], pts[k][1] - pts[k - 1][1]))
    total = cum[-1]
    ss, i = [], 0
    while True:
        at = i * step
        if i > 0 and at >= total - 1e-9:
            break
        ss.append(min(at, total))
        i += 1
        if total <= 1e-9:
            break
    if total > 1e-9:
        ss.append(total)
    out = []
    for at in ss:
        k = next((k for k in range(1, len(pts)) if at <= cum[k]), len(pts) - 1)
        seg = cum[k] - cum[k - 1]
        t = (at - cum[k - 1]) / seg if seg > 0 else 0.0
        out.append((at, pts[k - 1][0] + (pts[k][0] - pts[k - 1][0]) * t, pts[k - 1][1] + (pts[k][1] - pts[k - 1][1]) * t))
    return out


def grid_point_value(data, variable, slice_idx, x, y):
    """A grid slice's value at a drawing point: its cell's (the raster placed by the grid's own affine), the integer fill
    and NaN nothing (the analysis input's rule)."""
    g = next(q for q in grid_infos(data)["grids"] if q["variable"] == variable)
    x0, a, b, y0, c, d = g["affine"]
    det = a * d - b * c
    dx, dy = x - x0, y - y0
    u, v = (d * dx - b * dy) / det, (a * dy - c * dx) / det
    i, j = math.floor(u), math.floor(v)
    if not (0 <= i < g["width"] and 0 <= j < g["height"]):
        return NAN
    _, vals = grid_region(data, variable, slice_idx, 0, i, j, 1, 1)
    h = nc.read_header(data)
    rule = rule_of(next(q for q in h.vars if q["name"] == variable), attr_kinds(data))
    val = float(vals[0])
    return NAN if math.isnan(val) or (rule["nodata"] is not None and val == rule["nodata"]) else val


def mesh_point_value(data, part, step, x, y):
    """A mesh dataset's value at a point at a step (the mesh's interpolation, not a pixel's), as a float."""
    nodes, faces = mesh_nodes_faces()
    tris = triangles(nodes, faces)
    h = nc.read_header(data)
    loc = text_attr(next(q for q in h.vars if q["name"] == part["variable"]), "location")
    xv, mask = mesh_data(data, part["variable"], step)
    dd = {"location": loc, "x": xv, "mask": mask}
    if part.get("vector"):
        dd["y"], _ = mesh_data(data, part["vector"], step)
    return to_float(mesh_value(nodes, tris, dd, x, y))


PROFILES = [
    ("grid-cdf2.nc", {"variable": "dem", "slice": [0]}, 7.0,
     [[(500005.0, 4420045.0), (500095.0, 4420045.0)],
      [(499990.0, 4420025.0), (500050.0, 4420025.0), (500050.0, 4419990.0)]]),
    ("grid-cdf1.nc", {"variable": "t2m", "slice": [0, 1]}, 0.1,
     [[(31.95, 36.05), (32.7, 36.55)]]),
    ("ugrid.nc", {"variable": "depth", "slice": [1], "mesh": "mesh"}, 3.0,
     [[(500001.0, 4420003.0), (500039.0, 4420017.0)], [(500003.0, 4420019.0), (500019.0, 4420001.0)]]),
    ("ugrid.nc", {"variable": "level", "slice": [2], "mesh": "mesh"}, 2.5,
     [[(500030.0, 4420021.0), (500030.0, 4420039.0)], [(500001.0, 4420003.0), (500039.0, 4420017.0)]]),
    ("ugrid.nc", {"variable": "ucx", "vector": "ucy", "slice": [1], "mesh": "mesh"}, 4.0,
     [[(500001.0, 4420003.0), (500039.0, 4420017.0)]]),
]


def profile_cases(files):
    out = []
    for file, part, step, lines in PROFILES:
        data = files[file]
        stations, values = [], []
        for k, line in enumerate(lines):
            for at, x, y in walk_polyline(line, step):
                if "mesh" in part:
                    v = mesh_point_value(data, part, part["slice"][0], x, y)
                else:
                    v = grid_point_value(data, part["variable"], part["slice"], x, y)
                stations.append([k + 1, at, x, y])
                values.append(jnum(v))
        empty = sum(1 for v in values if v is None)
        summary = f"{grouped(len(lines))} çizgi, {grouped(len(stations))} nokta" + (f" ({grouped(empty)} değersiz)" if empty else "") + "."
        mpart = {**part, "affine": MESH_AFFINE, "width": MESH_SIZE[0], "height": MESH_SIZE[1]} if "mesh" in part else part
        out.append({"name": f"profile {file} {part['variable']} {part['slice']}", "kind": "profile", "file": file,
                    "part": mpart, "step": step, "lines": [[list(q) for q in line] for line in lines],
                    "expect": {"stations": stations, "values": values, "summary": summary}})
    return out


SERIES = [
    ("grid-cdf1.nc", {"variable": "t2m", "slice": [0, 1]},
     [("A", 32.1, 36.1), (None, 32.25, 36.25), ("A", 32.6, 36.4), ("Dışarıda", 31.0, 36.0)]),
    ("grid-cdf1.nc", {"variable": "pr", "slice": [2]}, [(None, 32.75, 36.25), (None, 32.0, 36.5)]),
    ("grid-cdf2.nc", {"variable": "dem", "slice": [1]},
     [("K1", 500010.0, 4420050.0), ("K2", 500070.0, 4420030.0), ("K3", 500090.0, 4420010.0)]),
    ("ugrid.nc", {"variable": "depth", "slice": [0], "mesh": "mesh"},
     [("P1", 500010.0, 4420006.0), ("P2", 500028.0, 4420004.0), ("P3", 500037.0, 4420014.0), ("P4", 500012.0, 4420031.0),
      ("P5", 499990.0, 4420010.0)]),
    ("ugrid.nc", {"variable": "level", "slice": [1], "mesh": "mesh"},
     [("F1", 500010.0, 4420006.0), ("F2", 500037.0, 4420014.0), ("F3", 500033.0, 4420031.0)]),
    ("ugrid.nc", {"variable": "ucx", "vector": "ucy", "slice": [0], "mesh": "mesh"},
     [("V1", 500010.0, 4420006.0), ("V2", 500028.0, 4420004.0)]),
]


def series_names(points):
    out = []
    for k, (name, _, _) in enumerate(points):
        base = name.strip() if name and name.strip() else f"Nokta {k + 1}"
        cand, i = base, 2
        while cand in out:
            cand = f"{base} ({i})"
            i += 1
        out.append(cand)
    return out


def series_cases(files):
    out = []
    for file, part, points in SERIES:
        data = files[file]
        h = nc.read_header(data)
        v = next(q for q in h.vars if q["name"] == part["variable"])
        dims = v["dims"][:-1] if "mesh" in part else v["dims"][:-2]
        sds = [slice_dim(h, data, d) for d in dims]
        tk = next(k for k, sd in enumerate(sds) if sd["time"])
        times = sds[tk]["values"]
        rows = []
        for st in range(len(times)):
            sl = list(part["slice"])
            sl[tk] = st
            row = []
            for _, x, y in points:
                if "mesh" in part:
                    val = mesh_point_value(data, part, st, x, y)
                else:
                    val = grid_point_value(data, part["variable"], sl, x, y)
                row.append(jnum(val))
            rows.append(row)
        mpart = {**part, "affine": MESH_AFFINE, "width": MESH_SIZE[0], "height": MESH_SIZE[1]} if "mesh" in part else part
        summary = f"{grouped(len(points))} nokta, {grouped(len(times))} adım."
        out.append({"name": f"series {file} {part['variable']} {part['slice']}", "kind": "series", "file": file,
                    "part": mpart, "points": [[n, x, y] for n, x, y in points],
                    "expect": {"times": times, "labels": dim_labels(times, True, None), "names": series_names(points),
                               "values": rows, "summary": summary}})
    return out


def calc_cases(files):
    """Mesh hesaplayıcı's runs over ugrid-calc.nc: the values worked out here, the file written by the reference's UGRID
    writer (ugrid_bytes)."""
    data = files["ugrid-calc.nc"]
    nodes, faces = mesh_nodes_faces()
    h = nc.read_header(data)
    n, f = len(nodes), len(faces)
    steps = 3
    t = cf_time("seconds since 2024-05-01 06:00:00", None)
    times = [moment(t, v) for v in (0.0, 1800.0, 3600.0)]

    def slab(name, st, count):
        vals = [float(x) for x in nc.read_var(data, h, name)]
        return vals[st * count:(st + 1) * count] if len(vals) > count else vals

    def nothing(xs):
        return [NAN if (x == F_FILL or math.isnan(x)) else x for x in xs]

    depth = [nothing(slab("depth", st, n)) for st in range(steps)]
    bed = slab("bed", 0, n)
    active = [slab("active", st, f) for st in range(steps)]
    stage = [[NAN if active[st][k] == 0 else x for k, x in enumerate(slab("stage", st, f))] for st in range(steps)]
    level = [slab("level", st, f) for st in range(steps)]
    speed = [[math.hypot(a, b) for a, b in zip(slab("ucx", st, n), slab("ucy", st, n))] for st in range(steps)]

    def summed(per_step, how):
        out = []
        for k in range(len(per_step[0])):
            vals = [row[k] for row in per_step if math.isfinite(row[k])]
            if not vals:
                out.append(NAN)
            elif how == "max":
                out.append(max(vals))
            elif how == "min":
                out.append(min(vals))
            else:
                acc = 0.0
                for x in vals:
                    acc += x
                out.append(acc / len(vals) if how == "mean" else acc)
        return out

    runs = [
        ("Derinlik × 2 + 1", "depth * 2 + 1", "none", "depth", "node",
         [[x * 2 + 1 for x in depth[st]] for st in range(steps)]),
        ("En büyük kot", "[Su derinliği] + bed", "max", "depth", "node",
         [[a + b for a, b in zip(depth[st], bed)] for st in range(steps)]),
        ("Ortalama fark", "stage - level", "mean", "stage", "face",
         [[a - b for a, b in zip(stage[st], level[st])] for st in range(steps)]),
        ("Toplam hız", "ucx * 10", "sum", "ucx", "node", [[x * 10 for x in speed[st]] for st in range(steps)]),
    ]
    words = {"max": "en büyüğü", "min": "en küçüğü", "mean": "ortalaması", "sum": "toplamı"}
    cases, outs = [], {}
    for k, (name, expr, how, first, loc, per_step) in enumerate(runs):
        per_step = [[NAN if not math.isfinite(x) else x for x in row] for row in per_step]
        if how == "none":
            slabs = [[f32(x) if math.isfinite(x) else NAN for x in row] for row in per_step]
            tout = [{"values": times, "absolute": True}]
        else:
            slabs = [[f32(x) if math.isfinite(x) else NAN for x in summed(per_step, how)]]
            tout = []
        flat = [x for row in slabs for x in row]
        empty = sum(1 for x in flat if math.isnan(x))
        places = n if loc == "node" else f
        word = "düğüm" if loc == "node" else "yüz"
        summary = f"“{name}”: {grouped(places)} {word}, {grouped(steps)} adım" + ("" if how == "none" else f"ın {words[how]}")
        if empty:
            summary += f" ({grouped(empty)} değersiz)"
        summary += "."
        datasets = [{"name": variable_name(name), "long_name": name, "location": loc, "double": False,
                     "time": 0 if how == "none" else None, "mask": None, "is_mask": False, "values": flat}]
        file = f"calc-{k + 1}.nc"
        outs[file] = ugrid_bytes([q[0] for q in nodes], [q[1] for q in nodes], faces, False, 5254, tout, datasets, name)
        cases.append({"name": f"calc {expr} {how}", "kind": "calc", "file": "ugrid-calc.nc",
                      "part": {"variable": first, "slice": [0], "mesh": "mesh", "affine": MESH_AFFINE,
                               "width": MESH_SIZE[0], "height": MESH_SIZE[1]},
                      "spec": {"expression": expr, "summary": how, "name": name},
                      "expect": {"file": file, "summary": summary, "variable": variable_name(name)}})
    listed = ", ".join(f"[{q}]" for q in ("depth", "bed", "stage", "level", "ucx", "salt"))
    errors = [
        ("depth + level", "x", "İfadenin andığı veri setleri aynı konumda olmalı: “depth” düğümlerde, “level” yüzlerde."),
        ("salt * 2", "x", "“salt” veri seti katmanlı (time, layer); Mesh hesaplayıcı katmanlı veri setini okumaz."),
        ("[yok] + 1", "x", f"“yok” adında veri seti yok. Veri setleri: {listed}."),
        ("1 + 2", "x", "İfade bir veri seti anmalı (örnek: [depth] * 2)."),
        ("depth * 2", " ", "Veri setinin adı 1–256 harf olmalı."),
    ]
    for expr, name, message in errors:
        cases.append({"name": f"calc refuses {expr!r} named {name!r}", "kind": "calc", "file": "ugrid-calc.nc",
                      "part": {"variable": "depth", "slice": [0], "mesh": "mesh", "affine": MESH_AFFINE,
                               "width": MESH_SIZE[0], "height": MESH_SIZE[1]},
                      "spec": {"expression": expr, "summary": "none", "name": name}, "expect": {"error": message}})
    return cases, outs


def build():
    files = {}
    files.update(grid_files())
    files.update(gdal_files())
    files["grid-big.nc"] = big_grid()
    files["grid-long-header.nc"] = long_header_grid()
    for name, steps in LARGE.items():
        files[name] = large_grid(steps)
    files["ugrid.nc"] = ugrid_file()
    files["ugrid-calc.nc"] = ugrid_calc_file()
    files["mesh.2dm"] = MESH_2DM.encode()
    files["quirks.2dm"] = MESH_2DM_QUIRKS.encode()
    files["scalar.dat"] = DAT_SCALAR.encode()
    files["vector.dat"] = DAT_VECTOR.encode()
    files.update(broken_files())
    cases = []
    for units, cal, values in TIME_CASES:
        t = cf_time(units, cal)
        cases.append({"name": f"time {units} {cal or ''}".strip(), "kind": "time", "units": units, "calendar": cal,
                      "values": values, "expect": None if t is None else [moment(t, v) for v in values]})
    for name in ("grid-cdf1.nc", "grid-cdf2.nc", "grid-cdf5.nc", "grid-big.nc", "gdal-cdf1.nc", "gdal-cdf2.nc"):
        cases.append({"name": f"info {name}", "kind": "info", "file": name, "expect": {**grid_infos(files[name]), "meshes": []}})
    cases.append({"name": "info ugrid.nc", "kind": "info", "file": "ugrid.nc", "expect": mesh_info(files["ugrid.nc"])})
    for name in LARGE:
        cases.append({"name": f"large {name}", "kind": "large", "file": name, "size": full_size(files[name]),
                      "expect": {**grid_infos(files[name]), "meshes": []}})
    regions = [
        ("grid-cdf1.nc", "t2m", [1, 0], 0, -1, -1, 6, 5),
        ("grid-cdf1.nc", "t2m", [0, 1], 0, 0, 0, 4, 3),
        ("grid-cdf1.nc", "t2m", [2, 1], 0, 0, 0, 4, 3),
        ("grid-cdf1.nc", "pr", [0], 0, 0, 0, 4, 3),
        ("grid-cdf1.nc", "land", [], 0, 0, 0, 4, 3),
        ("grid-cdf2.nc", "dem", [0], 0, -1, 0, 6, 3),
        ("grid-cdf2.nc", "dem", [1], 0, 0, 0, 5, 3),
        ("grid-cdf2.nc", "slope", [], 0, 0, 0, 5, 3),
        ("grid-cdf5.nc", "u8", [1], 0, 0, 0, 3, 2),
        ("grid-cdf5.nc", "u16", [], 0, 0, 0, 3, 2),
        ("grid-cdf5.nc", "u32", [], 0, 0, 0, 3, 2),
        ("grid-cdf5.nc", "i64", [], 0, 0, 0, 3, 2),
        ("grid-big.nc", "h", [], 0, 95, 45, 12, 12),
        ("grid-big.nc", "h", [], 1, 40, 20, 20, 15),
        ("grid-big.nc", "h", [], 2, 0, 0, 20, 20),
        ("grid-big.nc", "h", [], 2, 120, 55, 12, 12),
        ("grid-long-header.nc", "v", [], 0, 0, 0, 2, 2),
        ("gdal-cdf1.nc", "Band1", [], 0, -1, -1, 7, 6),
        ("gdal-cdf2.nc", "temp", [0], 0, 0, 0, 4, 3),
        ("gdal-cdf2.nc", "temp", [2], 0, 0, 0, 4, 3),
    ]
    for file, variable, sl, level, x, y, w, hg in regions:
        sample, values = grid_region(files[file], variable, sl, level, x, y, w, hg)
        cases.append({"name": f"region {file} {variable} {sl} level {level}", "kind": "region", "file": file,
                      "part": {"variable": variable, "slice": sl}, "region": [level, x, y, w, hg],
                      "expect": {"sample": sample, "values": [jnum(v) for v in values]}})
    cases += mesh_cases(files["ugrid.nc"])
    errors = {
        "bad-hdf5.nc": "Bu NetCDF-4 (HDF5) dosyası; KentOS klasik NetCDF okur. `nccopy -k cdf5 girdi.nc çıktı.nc` ya da `cdo -f nc5 copy girdi.nc çıktı.nc` ile çevirin.",
        "bad-grib.nc": "Bu bir GRIB dosyası; KentOS klasik NetCDF okur. `cdo -f nc5 copy girdi.grib çıktı.nc` ile çevirin.",
        "bad-other.nc": "Dosya NetCDF değil.",
        "bad-header.nc": "NetCDF dosyası kesik: başlığı bitmeden dosya bitiyor.",
        "bad-data.nc": "NetCDF dosyası kesik: “slope” değişkeninin değerleri dosyanın dışına uzanıyor.",
        "bad-type.nc": "NetCDF'in “crs” değişkeninin türü (9) bilinmiyor.",
    }
    for file, message in errors.items():
        cases.append({"name": f"error {file}", "kind": "error", "file": file, "expect": message})
    q = read_2dm(MESH_2DM_QUIRKS)
    cases.append({"name": "2dm quirks", "kind": "read2dm", "file": "quirks.2dm", "expect": q})
    size = len(files["grid-long-header.nc"])
    cases.append({"name": "parse grows its prefix", "kind": "parse", "file": "grid-long-header.nc",
                  "expect": [min(65536, size), min(131072, size)]})
    for name, start in (("sms.nc", None), ("sms-start.nc", float(days_from_civil(2024, 5, 1) * DAY + 3 * 3_600_000))):
        data, report = sms_ugrid(MESH_2DM, [DAT_SCALAR, DAT_VECTOR], start, 5254)
        files[name] = data
        cases.append({"name": f"2dm and dat to {name}", "kind": "sms", "mesh": "mesh.2dm", "dats": ["scalar.dat", "vector.dat"],
                      "start": start, "epsg": 5254, "expect": {"file": name, "report": report}})
    cases += profile_cases(files)
    cases += series_cases(files)
    calc, outs = calc_cases(files)
    cases += calc
    files.update(outs)
    return files, {"version": 1, "cases": cases}


# ---------------------------------------------------------------------------------------------------------------------------
# Cross-checks: GDAL (libnetcdf) for the grids, QGIS (MDAL) for the meshes

def gdal_check(files):
    try:
        from osgeo import gdal
    except ImportError:
        print("GDAL yok: ızgaraların çapraz denetimi atlandı")
        return
    gdal.UseExceptions()
    checked = 0
    with tempfile.TemporaryDirectory() as tmp:
        for name in ("grid-cdf1.nc", "grid-cdf2.nc", "grid-big.nc", "gdal-cdf1.nc", "gdal-cdf2.nc"):
            path = os.path.join(tmp, name)
            Path(path).write_bytes(files[name])
            info = grid_infos(files[name])
            for g in info["grids"]:
                try:
                    ds = gdal.Open(f'NETCDF:"{path}":{g["variable"]}')
                except RuntimeError as e:
                    print(f"  GDAL {name}:{g['variable']} açamadı ({e}); atlandı")
                    continue
                if "affine" in g:
                    gt = ds.GetGeoTransform()
                    assert all(abs(p - q) <= 1e-9 * max(1.0, abs(q)) for p, q in zip(gt, g["affine"])), (name, g["variable"], gt, g["affine"])
                assert (ds.RasterXSize, ds.RasterYSize) == (g["width"], g["height"])
                dims = g["dims"]
                for b in range(1, ds.RasterCount + 1):
                    band = ds.GetRasterBand(b)
                    md = band.GetMetadata()
                    sl = []
                    for d in dims:
                        key = f"NETCDF_DIM_{d['name']}"
                        raw = float(md[key])
                        # The band names its coordinate; our slice is that value's index (times compare as raw values).
                        h = nc.read_header(files[name])
                        dim_id = next(k for k, dd in enumerate(h.dims) if dd[0] == d["name"])
                        c = coordinate_of(h, dim_id)
                        coords = [float(x) for x in nc.read_var(files[name], h, c["name"])] if c else list(range(len(d["values"])))
                        sl.append(min(range(len(coords)), key=lambda k: abs(coords[k] - raw)))
                    _, ours = grid_region(files[name], g["variable"], sl, 0, 0, 0, g["width"], g["height"])
                    arr = band.ReadAsArray().astype(float).flatten().tolist()
                    scale, offset = band.GetScale(), band.GetOffset()
                    nodata = band.GetNoDataValue()
                    h = nc.read_header(files[name])
                    v = next(q for q in h.vars if q["name"] == g["variable"])
                    rule = rule_of(v, attr_kinds(files[name]))
                    theirs = [unpacked(rule, x) for x in arr]
                    for p, q in zip(ours, theirs):
                        assert (math.isnan(p) and math.isnan(q)) or p == q, (name, g["variable"], sl, p, q, scale, offset, nodata)
                    checked += 1
        # The large files as sparse copies of their full size (the values never written): their grid as libnetcdf reads it.
        large = 0
        for name in LARGE:
            path = os.path.join(tmp, name)
            with open(path, "wb") as f:
                f.write(files[name])
                f.truncate(full_size(files[name]))
            g = grid_infos(files[name])["grids"][0]
            ds = gdal.Open(f'NETCDF:"{path}":{g["variable"]}')
            gt = ds.GetGeoTransform()
            assert all(abs(p - q) <= 1e-9 * max(1.0, abs(q)) for p, q in zip(gt, g["affine"])), (name, gt, g["affine"])
            assert (ds.RasterXSize, ds.RasterYSize, ds.RasterCount) == (g["width"], g["height"], len(g["dims"][0]["values"])), name
            large += 1
    print(f"GDAL: {checked} bant aynı, {large} büyük dosyanın ızgarası aynı "
          "(CDF-5 değil: bu GDAL'ın libnetcdf'i CDF-5'siz; o dosya yalnız başvurunun okuyucusuyla)")


QGIS_PROBE = r"""
import json, sys
from qgis.core import QgsApplication, QgsMeshLayer, QgsMeshDatasetIndex, QgsPointXY
app = QgsApplication([], False); app.initQgis()
job = json.loads(sys.stdin.read())
out = {}
lay = QgsMeshLayer(job["mesh"], "m", "mdal")
for extra in job.get("datasets", []):
    lay.addDatasets(extra)
lay.updateTriangularMesh()
for g in range(lay.datasetGroupCount()):
    md = lay.datasetGroupMetadata(QgsMeshDatasetIndex(g, 0))
    n = lay.datasetCount(QgsMeshDatasetIndex(g, 0))
    vals = []
    for k in range(n):
        row = []
        for x, y in job["points"]:
            v = lay.datasetValue(QgsMeshDatasetIndex(g, k), QgsPointXY(x, y))
            s = v.scalar()
            row.append(None if s != s else s)
        vals.append(row)
    out[md.name()] = vals
print("JSON" + json.dumps(out))
"""


def qgis_values(mesh_path, datasets, points):
    # MDAL reads numbers by the C library's locale: a Turkish LC_NUMERIC reads “0.5” as 0. The C locale reads them as written.
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen", PYTHONPATH="/usr/share/qgis/python", LC_ALL="C.UTF-8")
    r = subprocess.run([sys.executable, "-c", QGIS_PROBE], input=json.dumps({"mesh": mesh_path, "datasets": datasets, "points": points}),
                       capture_output=True, text=True, env=env, timeout=300)
    line = next((ln for ln in r.stdout.splitlines() if ln.startswith("JSON")), None)
    if line is None:
        raise RuntimeError(r.stderr[-2000:])
    return json.loads(line[4:])


def close(p, q):
    if p is None or q is None:
        return p is None and q is None
    return abs(p - q) <= 1e-6 * max(1.0, abs(q))


def qgis_check(files):
    if not Path("/usr/share/qgis/python/qgis").exists():
        print("QGIS yok: ağların çapraz denetimi atlandı")
        return
    nodes, faces = mesh_nodes_faces()
    tris = triangles(nodes, faces)
    convex = {f for t, f, cv in tris if cv}
    # Away from the edges: on a shared edge MDAL may take the other triangle (the engine takes the lowest).
    pts = [(500005.0, 4420003.0), (500012.5, 4420017.5), (500031.0, 4420004.0), (500027.0, 4420012.0),
           (500006.0, 4420031.0), (500024.0, 4420036.0), (500037.0, 4420022.0), (500021.0, 4420019.0)]
    checked = 0
    with tempfile.TemporaryDirectory() as tmp:
        ug = os.path.join(tmp, "ugrid.nc")
        Path(ug).write_bytes(files["ugrid.nc"])
        got = qgis_values(ug, [], pts)
        for name, shown, loc in (("depth", "Su derinliği", "node"), ("level", "level", "face")):
            series = got.get(shown)
            if series is None:
                print(f"  QGIS'te “{name}” grubu yok: {list(got)}")
                continue
            for step, row in enumerate(series):
                xv, mask = mesh_data(files["ugrid.nc"], name, step)
                for (x, y), qv in zip(pts, row):
                    ti = locate(nodes, tris, x, y)
                    if ti is None:
                        continue
                    face = tris[ti][1]
                    if mask is not None and not mask[face]:
                        continue  # MDAL knows no kentos_mask
                    if loc == "node" and face not in convex:
                        continue
                    ours = to_float(mesh_value(nodes, tris, {"location": loc, "x": xv, "mask": None}, x, y))
                    assert close(None if math.isnan(ours) else ours, qv), ("ugrid", name, step, x, y, ours, qv)
                    checked += 1
        # 2DM with its DATs, and the UGRID file the engine writes of them.
        for label, mesh_file, dats in (("2dm", "mesh.2dm", ["scalar.dat", "vector.dat"]), ("sms", "sms.nc", [])):
            for f in [mesh_file] + dats:
                Path(os.path.join(tmp, f)).write_bytes(files[f] if f in files else b"")
            got = qgis_values(os.path.join(tmp, mesh_file), [os.path.join(tmp, d) for d in dats], SMS_POINTS)
            mesh = read_2dm(MESH_2DM)
            snodes = list(zip(mesh["x"], mesh["y"]))
            stris = triangles(snodes, mesh["faces"])
            dat = read_dat(DAT_SCALAR)[0]
            series = got.get("Su derinliği")
            if series is None:
                print(f"  QGIS {label}: derinlik grubu yok: {list(got)}")
                continue
            for step, row in enumerate(series):
                st = dat["steps"][step]
                for (x, y), qv in zip(SMS_POINTS, row):
                    ti = locate(snodes, stris, x, y)
                    if ti is None:
                        continue
                    face = stris[ti][1]
                    if st["active"] is not None and not st["active"][face] and label == "sms":
                        continue  # MDAL reads no kentos_mask
                    mask = None if st["active"] is None else [bool(a) for a in st["active"]]
                    ours = to_float(mesh_value(snodes, stris, {"location": "node", "x": st["values"], "mask": mask}, x, y))
                    assert close(None if math.isnan(ours) else ours, qv), (label, step, x, y, ours, qv)
                    checked += 1
            bed = next((v for k, v in got.items() if "bed" in k.lower() or "taban" in k.lower()), None)
            if bed:
                for (x, y), qv in zip(SMS_POINTS, bed[0]):
                    ours = to_float(mesh_value(snodes, stris, {"location": "node", "x": mesh["z"], "mask": None}, x, y))
                    assert close(None if math.isnan(ours) else ours, qv), (label, "bed", x, y, ours, qv)
                    checked += 1
    print(f"QGIS (MDAL): {checked} değer aynı")


SMS_POINTS = [(5.0, 5.0), (12.0, 15.0), (25.0, 3.0), (35.0, 15.0), (22.0, 18.0), (30.0, 10.0), (45.0, 5.0)]


def main():
    check = "--check" in sys.argv
    files, cases = build()
    text = json.dumps(cases, ensure_ascii=False, indent=1, allow_nan=False) + "\n"
    bad = []
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            bad.append(str(OUT.relative_to(ROOT)))
        for name, data in files.items():
            p = FILES / name
            if not p.exists() or p.read_bytes() != data:
                bad.append(str(p.relative_to(ROOT)))
    else:
        FILES.mkdir(parents=True, exist_ok=True)
        OUT.write_text(text)
        for name, data in files.items():
            (FILES / name).write_bytes(data)
    gdal_check(files)
    qgis_check(files)
    if bad:
        print("Farklı:", *bad, sep="\n  ")
        sys.exit(1)
    print(f"{len(cases['cases'])} durum {'aynı' if check else 'yazıldı'}")


if __name__ == "__main__":
    main()
