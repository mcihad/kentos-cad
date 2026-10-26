#!/usr/bin/env python3
"""The shared cases of cad.pointclouds.describe and cad.pointclouds.query (docs/adr/0536; plan/gis.md CBS-32).

    uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj python3 scripts/fixtures/pointcloud_describe_cases.py           # writes
    uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj python3 scripts/fixtures/pointcloud_describe_cases.py --check   # compares
    uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj python3 scripts/fixtures/pointcloud_describe_cases.py --files   # writes meta-*.las

No KentOS code is used. The files are fixtures/pointcloud/v1/files (laspy's and LASzip's) and two written here by
laspy for what those lack: meta-wkt.las (LAS 1.4, a compound WKT with a vertical system, a GUID, standard GPS time,
an extra dimension, a source id, a creation day) and meta-geokeys.las (LAS 1.2, GeoTIFF keys with a vertical key 4096,
GPS week time). What a source is comes from laspy's header (version, format, record length, count, bounds, scale,
offset, creation day, system and software, source id), the raw header bytes (the GUID as a GUID's little-endian
fields), the VLRs read here (the WKT by pyproj: its horizontal part's name and EPSG code, the vertical part's name;
the GeoTIFF keys by their numbers), the LAS 1.4 R15 tables (each format's dimensions, the extra bytes' types) and
laspy's points (classes counted with numpy). A COPC's nodes and their cubes are read from its info and hierarchy
EVLRs by the COPC 1.0 specification. A query's answer is the nearest point in plan within the reach over every point
laspy reads; a point's index is its place in the file (a COPC's: by its nodes' keys, each node's points in order),
the points read are those of the sources (and COPC nodes) whose box meets the search square.
"""
import base64
import json
import math
import struct
import sys
import uuid
from pathlib import Path

import laspy
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
FILES = ROOT / "fixtures/pointcloud/v1/files"
OUT = ROOT / "fixtures/commands/v1"
REL = "pointcloud/v1/files"
ANY = "$any"


def fx(name):
    return f"$fixture:{REL}/{name}"


# ── The two files written here ─────────────────────────────────────────

META_GUID = uuid.UUID("6f1c2a9e-4b3d-4c5e-9f00-112233445566")


def write_files():
    from pyproj import CRS

    rng = np.random.default_rng(536)
    # LAS 1.4, format 6: a compound system (TUREF / TM30 + EGM96 height), standard GPS time, an extra dimension.
    h = laspy.LasHeader(point_format=6, version="1.4")
    h.scales = np.array([0.01, 0.01, 0.001])
    h.offsets = np.array([500000.0, 4400000.0, 0.0])
    h.add_extra_dim(laspy.ExtraBytesParams(name="derinlik", type=np.float32, description="su derinligi"))
    h.add_crs(CRS.from_user_input("EPSG:5254+5773"))
    h.system_identifier = "KentOS fixtures"
    h.generating_software = "laspy 2.7.0 (ADR 0536)"
    h.file_source_id = 7
    h.uuid = META_GUID
    import datetime
    h.creation_date = datetime.date(2026, 3, 15)
    h.global_encoding.gps_time_type = laspy.header.GpsTimeType.STANDARD
    n = 40
    las = laspy.LasData(h)
    las.x = 500100 + rng.integers(0, 2000, n) / 100
    las.y = 4400100 + rng.integers(0, 2000, n) / 100
    las.z = 100 + rng.integers(0, 5000, n) / 1000
    las.classification = np.array([2, 2, 3, 6, 6, 6, 2, 9] * 5, dtype=np.uint8)
    las.intensity = rng.integers(0, 65535, n).astype(np.uint16)
    las.return_number = np.ones(n, dtype=np.uint8)
    las.number_of_returns = np.ones(n, dtype=np.uint8)
    las.gps_time = 1.4e9 + np.arange(n) * 0.25
    las.derinlik = rng.random(n).astype(np.float32)
    las.write(FILES / "meta-wkt.las")
    # LAS 1.2, format 1: GeoTIFF keys (projected 5254, vertical 5773), GPS week time.
    h = laspy.LasHeader(point_format=1, version="1.2")
    h.scales = np.array([0.01, 0.01, 0.01])
    h.offsets = np.array([500000.0, 4400000.0, 0.0])
    keys = [1, 1, 0, 3, 1024, 0, 1, 1, 3072, 0, 1, 5254, 4096, 0, 1, 5773]
    vlr = laspy.vlrs.known.GeoKeyDirectoryVlr()
    vlr.parse_record_data(struct.pack("<16H", *keys))
    h.vlrs.append(vlr)
    h.system_identifier = "KentOS fixtures"
    h.generating_software = "laspy 2.7.0 (ADR 0536)"
    h.creation_date = datetime.date(2025, 12, 31)
    n = 20
    las = laspy.LasData(h)
    las.x = 500200 + rng.integers(0, 1000, n) / 100
    las.y = 4400200 + rng.integers(0, 1000, n) / 100
    las.z = 50 + rng.integers(0, 1000, n) / 100
    las.classification = np.array([1, 2] * 10, dtype=np.uint8)
    las.return_number = np.ones(n, dtype=np.uint8)
    las.number_of_returns = np.ones(n, dtype=np.uint8)
    las.gps_time = 300000 + np.arange(n) * 1.5
    las.write(FILES / "meta-geokeys.las")
    write_tiles()
    write_grid()
    print("yazıldı: meta-wkt.las, meta-geokeys.las, tile-a … tile-f.las, area-grid.las")


# A 41 × 41 lattice (docs/adr/0552): points every half unit from (1000, 2000), every coordinate exact in binary, so
# the cases can put a ring's edge, a hole's edge, a corridor's edge and a station's end exactly on points.
GRID_FILE = "area-grid.las"


def write_grid():
    h = laspy.LasHeader(point_format=1, version="1.2")
    h.scales = np.array([0.5, 0.5, 0.01])
    h.offsets = np.array([1000.0, 2000.0, 0.0])
    h.system_identifier = "KentOS fixtures"
    h.generating_software = "laspy 2.7.0 (ADR 0552)"
    ii, jj = np.meshgrid(np.arange(41), np.arange(41), indexing="ij")
    i, j = ii.ravel(), jj.ravel()
    las = laspy.LasData(h)
    las.x = 1000.0 + i * 0.5
    las.y = 2000.0 + j * 0.5
    las.z = 100.0 + ((i * 7 + j * 3) % 50) * 0.1
    las.classification = np.array([2, 3, 5, 6], dtype=np.uint8)[(i + j) % 4]
    las.intensity = ((i * 41 + j * 5) % 1000).astype(np.uint16)
    las.return_number = np.ones(len(i), dtype=np.uint8)
    las.number_of_returns = np.ones(len(i), dtype=np.uint8)
    las.gps_time = 500000.0 + np.arange(len(i)) * 0.25
    las.write(FILES / GRID_FILE)


# Six tiles of a virtual cloud (docs/adr/0552): the same LAS 1.2 format 1, GeoTIFF keys, 40 points each in a 10 × 10 box
# at x0 = 600000 + 10 k (so none cover the same ground); b is a's twin, c names another horizontal system, d adds a
# vertical one, e an extra dimension, f a finer z scale.
TILES = {
    "tile-a.las": {}, "tile-b.las": {}, "tile-c.las": {"epsg": 5253}, "tile-d.las": {"vertical": 5773},
    "tile-e.las": {"extra": True}, "tile-f.las": {"zscale": 0.001},
}


def write_tiles():
    import datetime

    rng = np.random.default_rng(552)
    for k, (name, o) in enumerate(TILES.items()):
        x0 = 600000.0 + 10 * k
        h = laspy.LasHeader(point_format=1, version="1.2")
        h.scales = np.array([0.01, 0.01, o.get("zscale", 0.01)])
        h.offsets = np.array([x0, 4400000.0, 0.0])
        keys = [1, 1, 0, 3 if "vertical" in o else 2, 1024, 0, 1, 1, 3072, 0, 1, o.get("epsg", 5254)]
        if "vertical" in o:
            keys += [4096, 0, 1, o["vertical"]]
        vlr = laspy.vlrs.known.GeoKeyDirectoryVlr()
        vlr.parse_record_data(struct.pack(f"<{len(keys)}H", *keys))
        h.vlrs.append(vlr)
        if o.get("extra"):
            h.add_extra_dim(laspy.ExtraBytesParams(name="ek", type=np.float32))
        h.system_identifier = "KentOS fixtures"
        h.generating_software = "laspy 2.7.0 (ADR 0552)"
        h.creation_date = datetime.date(2026, 10, 20)
        n = 40
        las = laspy.LasData(h)
        las.x = x0 + rng.integers(0, 1000, n) / 100
        las.y = 4400000.0 + rng.integers(0, 1000, n) / 100
        las.z = 20 + rng.integers(0, 2000, n) / 100
        las.classification = np.array([2, 5] * (n // 2), dtype=np.uint8)
        las.return_number = np.ones(n, dtype=np.uint8)
        las.number_of_returns = np.ones(n, dtype=np.uint8)
        las.gps_time = 400000 + np.arange(n) * 0.5
        if o.get("extra"):
            las.ek = rng.random(n).astype(np.float32)
        las.write(FILES / name)


# ── What a file is ─────────────────────────────────────────────────────

LEGACY = ["x", "y", "z", "intensity", "returnNumber", "numberOfReturns", "scanDirectionFlag", "edgeOfFlightLine",
          "classification", "scanAngleRank", "userData", "pointSourceId"]
WIDE = ["x", "y", "z", "intensity", "returnNumber", "numberOfReturns", "classificationFlags", "scannerChannel",
        "scanDirectionFlag", "edgeOfFlightLine", "classification", "userData", "scanAngle", "pointSourceId", "gpsTime"]


def dimensions(fmt):
    """The LAS 1.4 R15 tables 7–17: each format's fields, camel cased."""
    out = list(WIDE if fmt >= 6 else LEGACY)
    if fmt in (1, 3, 4, 5):
        out.append("gpsTime")
    if fmt in (2, 3, 5, 7, 8, 10):
        out += ["red", "green", "blue"]
    if fmt in (8, 10):
        out.append("nir")
    if fmt in (4, 5, 9, 10):
        out += ["wavePacketDescriptorIndex", "byteOffsetToWaveformData", "waveformPacketSize",
                "returnPointWaveformLocation", "xT", "yT", "zT"]
    return out


EXTRA_TYPES = {1: ("u8", 1), 2: ("i8", 1), 3: ("u16", 2), 4: ("i16", 2), 5: ("u32", 4), 6: ("i32", 4),
               7: ("u64", 8), 8: ("i64", 8), 9: ("f32", 4), 10: ("f64", 8)}


def vlrs_of(path):
    """Every VLR and EVLR: (user, record, data), read from the bytes by the LAS 1.4 header's layout."""
    b = path.read_bytes()
    header_size = struct.unpack_from("<H", b, 94)[0]
    count = struct.unpack_from("<I", b, 100)[0]
    out, at = [], header_size
    for _ in range(count):
        user = b[at + 2:at + 18].split(b"\0")[0].decode()
        record, length = struct.unpack_from("<HH", b, at + 18)
        out.append((user, record, b[at + 54:at + 54 + length]))
        at += 54 + length
    minor = b[25]
    if minor >= 4:
        start, n = struct.unpack_from("<QI", b, 235)
        at = start
        for _ in range(n):
            user = b[at + 2:at + 18].split(b"\0")[0].decode()
            record = struct.unpack_from("<H", b, at + 18)[0]
            length = struct.unpack_from("<Q", b, at + 20)[0]
            out.append((user, record, b[at + 60:at + 60 + length]))
            at += 60 + length
    return out


def extras(vlrs):
    for user, record, data in vlrs:
        if user == "LASF_Spec" and record == 4:
            out = []
            for k in range(len(data) // 192):
                d = data[k * 192:(k + 1) * 192]
                t, options = d[2], d[3]
                name = d[4:36].split(b"\0")[0].decode()
                if t == 0:
                    kind, size = "bytes", options
                elif t <= 10:
                    kind, size = EXTRA_TYPES[t]
                elif t <= 20:
                    k0, s0 = EXTRA_TYPES[t - 10]
                    kind, size = f"{k0}[2]", 2 * s0
                else:
                    k0, s0 = EXTRA_TYPES[t - 20]
                    kind, size = f"{k0}[3]", 3 * s0
                out.append({"name": name, "kind": kind, "size": size})
            return out
    return []


def systems(vlrs):
    """(epsg, crs name, geographic, vertical): a WKT by pyproj (its horizontal and vertical parts), else the keys."""
    from pyproj import CRS

    for user, record, data in vlrs:
        if user == "LASF_Projection" and record == 2112:
            text = data.split(b"\0")[0].decode()
            crs = CRS.from_wkt(text)
            parts = crs.sub_crs_list or [crs]
            hz = parts[0]
            vertical = next((p.name for p in parts[1:] if p.is_vertical), None)
            # The horizontal part's own code: its outermost AUTHORITY or ID (pyproj's, as WKT 1 and 2 write it),
            # else the registry's system it is.
            auth = hz.to_json_dict().get("id")
            epsg = auth["code"] if auth and auth.get("authority") == "EPSG" else hz.to_epsg()
            return epsg, hz.name, hz.is_geographic, vertical
    for user, record, data in vlrs:
        if user == "LASF_Projection" and record == 34735:
            w = struct.unpack(f"<{len(data) // 2}H", data)
            keys = {w[4 + 4 * k]: (w[5 + 4 * k], w[7 + 4 * k]) for k in range(w[3])}
            epsg, geo = None, False
            if 3072 in keys and keys[3072][0] == 0 and keys[3072][1] not in (0, 32767):
                epsg = keys[3072][1]
            elif 2048 in keys and keys[2048][0] == 0 and keys[2048][1] not in (0, 32767):
                epsg, geo = keys[2048][1], True
            v = keys.get(4096)
            vertical = f"EPSG:{v[1]}" if v and v[0] == 0 and v[1] not in (0, 32767) else None
            return epsg, None, geo, vertical
    return None, None, False, None


def copc_nodes(path):
    """A COPC's info (centre, half size) and its hierarchy's entries: key, count (pages followed)."""
    b = path.read_bytes()
    vl = vlrs_of(path)
    info = next(d for u, r, d in vl if u == "copc" and r == 1)
    cx, cy, cz, half = struct.unpack_from("<4d", info, 0)
    root_off, root_size = struct.unpack_from("<QQ", info, 40)
    nodes, pages = [], [(root_off, root_size)]
    while pages:
        off, size = pages.pop(0)
        for k in range(size // 32):
            d, x, y, z, eoff, ebytes, count = struct.unpack_from("<4iQii", b, off + 32 * k)
            if count == -1:
                pages.append((eoff, ebytes))
            else:
                nodes.append(((d, x, y, z), count))
    return (cx, cy, cz, half), nodes


def describe(name, classes=False):
    path = FILES / name
    with laspy.open(path) as r:
        h = r.header
        pts = r.read() if classes else None
    vl = vlrs_of(path)
    raw = path.read_bytes()
    copc = any(u == "copc" for u, _, _ in vl)
    epsg, crs_name, geographic, vertical = systems(vl)
    fmt = h.point_format.id
    gps = fmt in (1, 3, 4, 5) or fmt >= 6
    guid = None if raw[8:24] == bytes(16) else str(uuid.UUID(bytes_le=raw[8:24]))
    assert guid is None or guid == str(h.uuid), (name, guid, h.uuid)
    by_return = [int(v) for v in h.number_of_points_by_return]
    while by_return and by_return[-1] == 0:
        by_return.pop()
    out = {
        "file": fx(name), "read": True,
        "kind": "copc" if copc else ("laz" if name.endswith(".laz") else "las"),
        "version": f"{h.version.major}.{h.version.minor}", "format": fmt, "recordLen": h.point_format.size,
        "count": int(h.point_count),
        "bounds": [float(v) for v in [*h.mins, *h.maxs]],
        "scale": [float(v) for v in h.scales], "offset": [float(v) for v in h.offsets],
    }
    if epsg is not None:
        out["epsg"] = epsg
    if crs_name is not None:
        out["crsName"] = crs_name
    out["geographic"] = geographic if False else None
    out.pop("geographic")
    if vertical is not None:
        out["vertical"] = vertical
    if h.creation_date is not None:
        out["created"] = h.creation_date.isoformat()
    if gps:
        out["gpsTime"] = "standard" if int(h.global_encoding.value) & 1 else "week"
    for key, v in (("system", h.system_identifier), ("software", h.generating_software)):
        v = v.strip("\0").strip()
        if v:
            out[key] = v
    out["sourceId"] = int(h.file_source_id)
    if guid:
        out["guid"] = guid
    out["dimensions"] = dimensions(fmt)
    ex = extras(vl)
    if ex:
        out["extra"] = ex
    if by_return:
        out["byReturn"] = by_return
    if copc:
        out["nodes"] = len(copc_nodes(path)[1])
    if classes:
        codes, counts = np.unique(np.asarray(pts.classification, dtype=np.uint8), return_counts=True)
        out["classes"] = {str(int(c)): int(n) for c, n in sorted(zip(codes, counts), key=lambda p: str(int(p[0])))}
    return out


def source_record(name):
    """The cloud's record of a source (as Nokta bulutu ekle writes it): its format, count and bounds."""
    with laspy.open(FILES / name) as r:
        h = r.header
    vl = vlrs_of(FILES / name)
    copc = any(u == "copc" for u, _, _ in vl)
    fmt = "copc" if copc else ("laz" if name.endswith(".laz") else "las")
    return {"file": fx(name), "format": fmt, "count": int(h.point_count),
            "bounds": [float(v) for v in [*h.mins, *h.maxs]]}


# ── The drawing ────────────────────────────────────────────────────────

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
TILE_FILES = list(TILES)
CLOUD_FILES = ["las12-f0.las", "las12-f1.laz", "las12-f2.las", "las12-f3.laz", "las13-f1.las", "las14-f6.laz",
               "las14-f7.las", "las14-f8.laz", "index-sample.laz", "index-sample.copc.laz", "many-chunks.laz",
               "meta-wkt.las", "meta-geokeys.las"]
ASSET = "pointcloud-0536053605360536"
TEXT_BOUNDS = [500000.0, 4400000.0, 100.0, 500100.0, 4400100.0, 150.0]


def layer(i, name, visible=True):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": False, "expanded": True,
            "style": STYLE, "children": []}


def cloud(id_, layer_id, sources, count=None):
    b = [s["bounds"] for s in sources]
    return {"kind": "pointcloud", "id": id_, "layerId": layer_id, "attrs": {}, "sources": sources,
            "bounds": [min(x[0] for x in b), min(x[1] for x in b), min(x[2] for x in b),
                       max(x[3] for x in b), max(x[4] for x in b), max(x[5] for x in b)],
            "count": sum(s["count"] for s in sources) if count is None else count,
            "srid": 5254, "style": {"render": "classification", "size": 2}}


def drawing():
    """Ids 1–13 the files' clouds (in CLOUD_FILES' order), 14 a text cloud, 15 a missing file, 16 an address,
    17 a broken file, 18 an embedded las12-f0, 19 a line, 20 a cloud recorded with too many points, 21 meta-wkt
    on a hidden layer, 22 two sources (meta-wkt and meta-geokeys)."""
    ents = [cloud(k + 1, "bulut", [source_record(n)]) for k, n in enumerate(CLOUD_FILES)]
    ents.append(cloud(14, "bulut", [{"file": fx("bulut.xyz"), "format": "xyz", "count": 50, "bounds": TEXT_BOUNDS}]))
    ents.append(cloud(15, "bulut", [{**source_record("las12-f0.las"), "file": fx("yok.las")}]))
    ents.append(cloud(16, "bulut", [{"url": "https://ornek.invalid/bulut.copc.laz", "format": "copc", "count": 10,
                                     "bounds": TEXT_BOUNDS}]))
    ents.append(cloud(17, "bulut", [{**source_record("las12-f0.las"), "file": fx("broken/not-las.las")}]))
    s = source_record("las12-f0.las")
    del s["file"]
    ents.append(cloud(18, "bulut", [{**s, "asset": ASSET}]))
    ents.append({"kind": "line", "id": 19, "layerId": "bulut", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 1}})
    ents.append(cloud(20, "bulut", [source_record("las12-f0.las")], count=300_000_000))
    ents.append(cloud(21, "gizli", [source_record("meta-wkt.las")]))
    ents.append(cloud(22, "bulut", [source_record("meta-wkt.las"), source_record("meta-geokeys.las")]))
    # 23: three files over the same ground (formats 0, 1, 2); 24: two tiles; 25: one file twice; 26: a file and one
    # not found; 27: tile a whose record is stale (points, box); 28: a text source and a tile; 29–32: tile a with c
    # (system), d (vertical system), e (extra dimension), f (scale); 33: the six tiles.
    s0 = source_record("las12-f0.las")
    ents.append(cloud(23, "bulut", [source_record(n) for n in ["las12-f0.las", "las12-f1.laz", "las12-f2.las"]]))
    ents.append(cloud(24, "bulut", [source_record("tile-a.las"), source_record("tile-b.las")]))
    ents.append(cloud(25, "bulut", [s0, s0]))
    ents.append(cloud(26, "bulut", [s0, {**s0, "file": fx("yok.las")}]))
    stale = source_record("tile-a.las")
    stale["count"] += 5
    stale["bounds"] = [stale["bounds"][0] - 1.0, *stale["bounds"][1:]]
    ents.append(cloud(27, "bulut", [stale, source_record("tile-b.las")]))
    ents.append(cloud(28, "bulut", [{"file": fx("bulut.xyz"), "format": "xyz", "count": 50, "bounds": TEXT_BOUNDS},
                                    source_record("tile-a.las")]))
    for k, other in enumerate(["tile-c.las", "tile-d.las", "tile-e.las", "tile-f.las"]):
        ents.append(cloud(29 + k, "bulut", [source_record("tile-a.las"), source_record(other)]))
    ents.append(cloud(33, "bulut", [source_record(n) for n in TILE_FILES]))
    # 34: the lattice; 35: the lattice recorded with too many points (a call that would read them refuses).
    ents.append(cloud(34, "bulut", [source_record(GRID_FILE)]))
    big = source_record(GRID_FILE)
    big["count"] = 300_000_000
    ents.append(cloud(35, "bulut", [big]))
    data = base64.b64encode((FILES / "las12-f0.las").read_bytes()).decode()
    return {
        "format": "kentos.document", "version": 1, "name": "Nokta bulutlarının bilgisi",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis"},
        "origin": {"x": 500000.0, "y": 4400000.0},
        "layers": [layer("bulut", "Bulut"), layer("gizli", "Gizli", visible=False)],
        "activeLayer": "bulut", "entities": ents,
        "styles": {"items": [{"kind": "asset", "id": ASSET, "name": "las12-f0.las",
                              "data": f"data:application/octet-stream;base64,{data}"}], "categories": []},
    }


# ── The cases ──────────────────────────────────────────────────────────

def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result, "expect": {"revision": "same"}}


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def num(v):
    """A float as Rust's `{}` writes it (positional, no trailing .0)."""
    t = repr(float(v))
    return t[:-2] if t.endswith(".0") else t


def summary_ref(records, srcs):
    """The sources of a cloud of several files summed up (docs/adr/0552): totals, the system they share, and where
    they disagree, in the ADR's words."""
    b = list(srcs[0]["bounds"])
    for x in srcs[1:]:
        b = [min(b[k], x["bounds"][k]) for k in range(3)] + [max(b[k], x["bounds"][k]) for k in range(3, 6)]
    headed = [i for i, x in enumerate(srcs) if x["read"] and x["kind"] != "text"]
    epsg = None
    if headed and all(srcs[i].get("epsg") == srcs[headed[0]].get("epsg") for i in headed):
        epsg = srcs[headed[0]].get("epsg")
    mism = []
    if len(headed) >= 2:
        def differ(kind, what, vals):
            if any(v != vals[0] for v in vals):
                said = ", ".join(f"{i + 1}. kaynak {v}" for i, v in zip(headed, vals))
                mism.append({"kind": kind, "sources": list(headed), "message": f"Kaynakların {what} aynı değil: {said}."})
        differ("epsg", "yatay sistemi", [f"EPSG:{srcs[i]['epsg']}" if "epsg" in srcs[i] else "yok" for i in headed])
        differ("vertical", "düşey sistemi", [srcs[i].get("vertical", "yok") for i in headed])
        differ("point_format", "nokta biçimi", [str(srcs[i]["format"]) for i in headed])
        differ("gps_time", "GPS zamanı", [srcs[i].get("gpsTime", "yok") for i in headed])
        differ("scale", "ölçeği", [" ".join(num(v) for v in srcs[i]["scale"]) for i in headed])
        differ("extra", "ek boyutları", ["/".join(e["name"] for e in srcs[i].get("extra", [])) or "yok" for i in headed])
    off = [i for i in headed if records[i]["count"] != srcs[i]["count"]]
    if off:
        said = ", ".join(f"{i + 1}. kaynak (kayıtta {records[i]['count']}, başlıkta {srcs[i]['count']})" for i in off)
        mism.append({"kind": "count", "sources": off,
                     "message": f"Bulutun kaydındaki nokta sayısı kaynağın başlığındakinden farklı: {said}."})
    off = [i for i in headed if records[i]["bounds"] != srcs[i]["bounds"]]
    if off:
        said = ", ".join(f"{i + 1}. kaynak" for i in off)
        mism.append({"kind": "bounds", "sources": off,
                     "message": f"Bulutun kaydındaki kutu kaynağın başlığındakinden farklı: {said}."})
    pairs = 0
    for a, i in enumerate(headed):
        for j in headed[a + 1:]:
            p, q = srcs[i]["bounds"], srcs[j]["bounds"]
            if max(p[0], q[0]) < min(p[3], q[3]) and max(p[1], q[1]) < min(p[4], q[4]) and pairs < 50:
                pairs += 1
                mism.append({"kind": "overlap", "sources": [i, j],
                             "message": f"{i + 1}. ve {j + 1}. kaynağın kutuları aynı yeri kaplıyor."})
    out = {"sources": len(srcs), "readable": sum(1 for x in srcs if x["read"]),
           "count": sum(x["count"] for x in srcs), "bounds": b}
    if epsg is not None:
        out["epsg"] = epsg
    if mism:
        out["mismatches"] = mism
    return out


def described(id_, sources, count=None, layer_id="bulut"):
    ent = next(e for e in drawing()["entities"] if e["id"] == id_)
    out = {"uid": f"$uidOf:{id_}", "layer": layer_id, "srid": 5254, "count": ent["count"] if count is None else count,
           "sources": sources}
    if len(sources) > 1:
        out["summary"] = summary_ref(ent["sources"], sources)
    return out


def answer(clouds):
    return {"clouds": clouds, "revision": "$current"}


def unread(rec, why):
    out = {k: rec[k] for k in ("file", "url", "asset") if k in rec}
    out.update({"read": False, "why": why, "kind": rec["format"] if rec["format"] != "xyz" else "text",
                "count": rec["count"], "bounds": rec["bounds"]})
    return out


def warn(why, at="clouds[0].sources[0]", k=1):
    return {"code": "cloud_source_unreadable", "message": f"Bulutun {k}. kaynağı okunamadı: {why}.", "path": at}


UID_TEXT = ("geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
            "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.")
GONE = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4eff"


def describe_cases():
    out = []
    for k, n in enumerate(CLOUD_FILES):
        out.append(case(f"{n}: başlığı, sistemi, boyutları", step(
            {"uids": [f"$uidOf:{k + 1}"]}, done(answer([described(k + 1, [describe(n)])])))))
    for n in ["las12-f0.las", "meta-wkt.las", "index-sample.copc.laz", "many-chunks.laz"]:
        k = CLOUD_FILES.index(n) + 1
        out.append(case(f"{n}: sınıflar sayılarak", step(
            {"uids": [f"$uidOf:{k}"], "classes": True}, done(answer([described(k, [describe(n, True)])])))))
    text = {"file": fx("bulut.xyz"), "read": True, "kind": "text", "count": 50, "bounds": TEXT_BOUNDS,
            "dimensions": ["x", "y", "z"]}
    out.append(case("metin bulutunun başlığı yok: sayısı ve kutusu bulutun kaydından", step(
        {"uids": ["$uidOf:14"]}, done(answer([described(14, [text])])))))
    rec = drawing()["entities"][14]["sources"][0]
    missing = unread(rec, ANY)
    out.append(case("dosyası olmayan kaynak okunmaz, söylenir", step(
        {"uids": ["$uidOf:15"]}, done(answer([described(15, [missing])]), [ANY]))))
    rec = drawing()["entities"][15]["sources"][0]
    # The address is .invalid: a host with no network says so, one with a network that the name is not found
    # (docs/adr/0645 §5); either way the source is not read and said.
    out.append(case("adresteki kaynak okunamazsa söylenir", step(
        {"uids": ["$uidOf:16"]}, done(answer([described(16, [unread(rec, ANY)])]), [ANY]))))
    rec = drawing()["entities"][16]["sources"][0]
    out.append(case("bozuk dosya okunmaz, söylenir", step(
        {"uids": ["$uidOf:17"]}, done(answer([described(17, [unread(rec, ANY)])]), [ANY]))))
    emb = describe("las12-f0.las")
    del emb["file"]
    emb = {"asset": ASSET, **emb}
    out.append(case("gömülü bulut kitaplıktan okunur", step(
        {"uids": ["$uidOf:18"]}, done(answer([described(18, [emb])])))))
    out.append(case("iki kaynaklı bulut: kaynakları sırasıyla", step(
        {"uids": ["$uidOf:22"]}, done(answer([described(22, [describe("meta-wkt.las"), describe("meta-geokeys.las")])])))))
    out.append(case("gizli katmandaki bulut adıyla sorulunca okunur", step(
        {"uids": ["$uidOf:21"]}, done(answer([described(21, [describe("meta-wkt.las")], layer_id="gizli")])))))
    out += summary_cases()
    out.append(case("aynı bulut iki kez verilse bir kez söylenir", step(
        {"uids": ["$uidOf:1", "$uidOf:1"]}, done(answer([described(1, [describe("las12-f0.las")])])))))
    out.append(case("plan aynı cevabı yazmadan verir", step(
        {"uids": ["$uidOf:12"]}, done(answer([described(12, [describe("meta-wkt.las")])])), op="plan")))
    out.append(case("validate yalnız denetler", step(
        {"uids": ["$uidOf:12"], "classes": True}, {"status": "completed", "output": None, "warnings": []}, op="validate")))
    out += [
        case("geçersiz kimlik", step({"uids": ["bulut"]}, failed("invalid_uid", f"“bulut” {UID_TEXT}", "uids[0]"))),
        case("çizimde olmayan nesne", step({"uids": [GONE]}, failed(
            "entity_not_found",
            f"“{GONE}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
            "uids[0]"))),
        case("nokta bulutu olmayan nesne", step({"uids": ["$uidOf:19"]}, failed(
            "not_a_pointcloud", "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.",
            "uids[0]"))),
        case("sınıflar sayılırken çok nokta okunurdu", step({"uids": ["$uidOf:20"], "classes": True}, failed(
            "too_many_cloud_points",
            "Sınıflar sayılırken 300000000 nokta okunurdu; bir çağrı en çok 200000000 nokta sayar. Bulutları ayrı ayrı sorun.",
            "classes"))),
    ]
    return out


def summary_cases():
    """The summary of a cloud of several files (docs/adr/0552): where the files disagree and where they do not."""
    d = describe
    out = []

    def one(name, id_, names):
        out.append(case(name, step({"uids": [f"$uidOf:{id_}"]}, done(answer([described(id_, [d(n) for n in names])])))))

    one("özet: üç dosya aynı yerde, nokta biçimleri ve GPS zamanı farklı", 23, ["las12-f0.las", "las12-f1.laz", "las12-f2.las"])
    one("özet: yan yana iki karo, uyuşmazlık yok", 24, ["tile-a.las", "tile-b.las"])
    one("özet: aynı dosya iki kez, yalnız kutuları örtüşür", 25, ["las12-f0.las", "las12-f0.las"])
    rec = drawing()["entities"][25]["sources"][1]
    out.append(case("özet: biri okunamayan kaynak toplamda sayılır, karşılaştırmaya girmez", step(
        {"uids": ["$uidOf:26"]},
        done(answer([described(26, [d("las12-f0.las"), unread(rec, ANY)])]), [ANY]))))
    one("özet: kayıttaki sayı ve kutu başlıktan farklı", 27, ["tile-a.las", "tile-b.las"])
    text = {"file": fx("bulut.xyz"), "read": True, "kind": "text", "count": 50, "bounds": TEXT_BOUNDS,
            "dimensions": ["x", "y", "z"]}
    out.append(case("özet: metin kaynağı sayılır, başlığı olmadığından karşılaştırılmaz", step(
        {"uids": ["$uidOf:28"]}, done(answer([described(28, [text, d("tile-a.las")])])))))
    one("özet: yatay sistem farklı", 29, ["tile-a.las", "tile-c.las"])
    one("özet: düşey sistem var, öbüründe yok", 30, ["tile-a.las", "tile-d.las"])
    one("özet: ek boyut yalnız birinde", 31, ["tile-a.las", "tile-e.las"])
    one("özet: ölçek farklı", 32, ["tile-a.las", "tile-f.las"])
    one("özet: altı karo, dört ayrı uyuşmazlık sırasıyla", 33, TILE_FILES)
    return out


def points_of(name):
    """Every point of a file in the source's order: its index, x, y, z and record; a COPC's by its nodes' keys."""
    with laspy.open(FILES / name) as r:
        p = r.read()
    order = np.arange(len(p.x))
    vl = vlrs_of(FILES / name)
    if any(u == "copc" for u, _, _ in vl):
        # The file's chunks are its nodes in the hierarchy's file order; the source's order is by key.
        _, nodes = copc_nodes(FILES / name)
        with_points = [n for n in nodes if n[1] > 0]
        b = (FILES / name).read_bytes()
        info = next(d for u, r_, d in vl if u == "copc" and r_ == 1)
        offsets = {}
        root_off, root_size = struct.unpack_from("<QQ", info, 40)
        pages = [(root_off, root_size)]
        while pages:
            off, size = pages.pop(0)
            for k in range(size // 32):
                d, x, y, z, eoff, ebytes, count = struct.unpack_from("<4iQii", b, off + 32 * k)
                if count == -1:
                    pages.append((eoff, ebytes))
                elif count > 0:
                    offsets[(d, x, y, z)] = (eoff, count)
        in_file = sorted(offsets.items(), key=lambda kv: kv[1][0])
        start, firsts = 0, {}
        for key, (_, count) in in_file:
            firsts[key] = start
            start += count
        # by_key[j]: the file place of the j-th point in key order; order[file place] = its key-order place.
        by_key = np.concatenate([np.arange(firsts[key], firsts[key] + count)
                                 for key, count in sorted((k, c) for k, c in with_points)])
        order = np.empty_like(by_key)
        order[by_key] = np.arange(len(by_key))
    return p, order


class TextPoints:
    """A text cloud's points as laspy's arrays (x, y, z, intensity, classification), in its records' order."""

    def __init__(self, name):
        def number(t):
            try:
                v = float(t)
            except ValueError:
                return None
            return v if math.isfinite(v) else None

        rows, columns = [], None
        for line in (FILES / name).read_text("utf-8").splitlines():
            fields = [f for f in line.replace(",", " ").replace(";", " ").split() if f]
            values = [number(f) for f in fields]
            if columns is None:
                if len(values) < 3 or any(v is None for v in values[:3]):
                    continue  # a header line
                n = len(values)
                columns = "xyz" if n == 3 else "xyzi" if n in (4, 5) else "xyzrgb" if n == 6 else "xyzirgb"
            need = {"xyz": 3, "xyzi": 4, "xyzrgb": 6, "xyzirgb": 7}[columns]
            if len(values) < need or any(v is None for v in values[:3]):
                continue
            i = 0
            if columns in ("xyzi", "xyzirgb") and len(values) > 3 and values[3] is not None:
                i = int(values[3])
            rows.append((values[0], values[1], values[2], i))
        a = np.array(rows, dtype=float).reshape(-1, 4)
        self.x, self.y, self.z = a[:, 0], a[:, 1], a[:, 2]
        self.intensity = a[:, 3].astype(int)
        n = len(rows)
        # Records of format 6: return 1 of 1, class 0, no source id, no GPS time.
        self.classification = np.zeros(n, dtype=int)
        self.return_number = np.ones(n, dtype=int)
        self.number_of_returns = np.ones(n, dtype=int)
        self.point_source_id = np.zeros(n, dtype=int)
        self.gps_time = np.zeros(n)
        self.point_format = type("Format", (), {"id": 6 if columns in ("xyz", "xyzi") else 7})()


def nearest(clouds, at, reach):
    """The nearest point over (cloud id, source index, name) in their order, each source's points in its order;
    the points read: the sources whose box meets the square (a COPC's nodes that meet it)."""
    best, read = None, 0
    for cid, si, name in clouds:
        text = name.endswith(".xyz")
        b = TEXT_BOUNDS if text else source_record(name)["bounds"]
        if at[0] + reach < b[0] or at[0] - reach > b[3] or at[1] + reach < b[1] or at[1] - reach > b[4]:
            continue
        if text:
            # A text cloud is read whole, its points in its records' order (docs/adr/0645 §6).
            p = TextPoints(name)
            order = np.arange(len(p.x))
            read += len(p.x)
            keep = np.ones(len(p.x), dtype=bool)
        else:
            p, order = points_of(name)
        vl = [] if text else vlrs_of(FILES / name)
        if text:
            pass
        elif any(u == "copc" for u, _, _ in vl):
            (cx, cy, cz, half), nodes = copc_nodes(FILES / name)
            low = (cx - half, cy - half)
            meets = set()
            for (d, x, y, z), count in nodes:
                side = 2 * half / (1 << d)
                lx, ly = low[0] + x * side, low[1] + y * side
                if not (at[0] + reach < lx or at[0] - reach > lx + side or at[1] + reach < ly or at[1] - reach > ly + side):
                    if count > 0:
                        meets.add((d, x, y, z))
            read += sum(c for k, c in nodes if k in meets)
            # Only the points of nodes that meet the square are looked at.
            keep = node_mask(name, meets)
        else:
            read += len(p.x)
            keep = np.ones(len(p.x), dtype=bool)
        xs, ys = np.asarray(p.x), np.asarray(p.y)
        d2 = (xs - at[0]) ** 2 + (ys - at[1]) ** 2
        for i in sorted(np.flatnonzero(keep & (d2 <= reach * reach)), key=lambda i: order[i]):
            if best is None or d2[i] < best[0]:
                best = (d2[i], cid, si, name, int(order[i]), int(i), p)
    return best, read


def node_mask(name, keys):
    """Which points of a COPC (in laspy's file order) lie in the nodes `keys`."""
    b = (FILES / name).read_bytes()
    vl = vlrs_of(FILES / name)
    info = next(d for u, r_, d in vl if u == "copc" and r_ == 1)
    root_off, root_size = struct.unpack_from("<QQ", info, 40)
    pages, entries = [(root_off, root_size)], []
    while pages:
        off, size = pages.pop(0)
        for k in range(size // 32):
            d, x, y, z, eoff, ebytes, count = struct.unpack_from("<4iQii", b, off + 32 * k)
            if count == -1:
                pages.append((eoff, ebytes))
            elif count > 0:
                entries.append(((d, x, y, z), eoff, count))
    entries.sort(key=lambda e: e[1])
    mask = []
    for key, _, count in entries:
        mask += [key in keys] * count
    return np.array(mask, dtype=bool)


def point_out(best):
    d2, cid, si, name, index, i, p = best
    fmt = p.point_format.id
    out = {"cloud": f"$uidOf:{cid}", "source": si, "index": index,
           "x": float(p.x[i]), "y": float(p.y[i]), "z": float(p.z[i]), "distance": float(np.sqrt(d2)),
           "class": int(p.classification[i]), "intensity": int(p.intensity[i]),
           "returnNumber": int(p.return_number[i]), "numberOfReturns": int(p.number_of_returns[i]),
           "pointSourceId": int(p.point_source_id[i])}
    if fmt in (1, 3, 4, 5) or fmt >= 6:
        out["gpsTime"] = float(p.gps_time[i])
    if fmt in (2, 3, 5, 7, 8, 10):
        out["rgb"] = [int(p.red[i]), int(p.green[i]), int(p.blue[i])]
    return out


def queried(clouds, at, reach, inp=None, warnings=(), op="execute"):
    best, read = nearest(clouds, at, reach)
    out = {"read": read, "revision": "$current"}
    if best is not None:
        out = {"point": point_out(best), **out}
    i = inp or {"at": {"x": at[0], "y": at[1]}, "radius": reach, "uids": [f"$uidOf:{c}" for c, _, _ in clouds]}
    return step(i, done(out, warnings), op=op)


def near_point(name, k, dx=0.003, dy=-0.002):
    """A place next to a file's k-th point (its index in laspy's order)."""
    with laspy.open(FILES / name) as r:
        p = r.read()
    return [float(p.x[k]) + dx, float(p.y[k]) + dy]


def query_cases():
    out = []
    for n, k, reach in [("las12-f0.las", 17, 0.5), ("las12-f1.laz", 120, 0.5), ("las12-f2.las", 5, 0.2),
                        ("las12-f3.laz", 250, 1.0), ("las13-f1.las", 33, 0.5), ("las14-f6.laz", 77, 0.5),
                        ("las14-f7.las", 199, 0.5), ("las14-f8.laz", 3, 0.5), ("index-sample.laz", 4321, 0.3),
                        ("index-sample.copc.laz", 1234, 0.3), ("many-chunks.laz", 99999, 0.3),
                        ("meta-wkt.las", 11, 0.3), ("meta-geokeys.las", 7, 0.3)]:
        cid = CLOUD_FILES.index(n) + 1
        at = near_point(n, k)
        out.append(case(f"{n}: {k}. noktanın yanında en yakın nokta", queried([(cid, 0, n)], at, reach)))
    at = near_point("las12-f0.las", 17)
    out.append(case("erimin içinde nokta yoksa nokta yok", queried([(1, 0, "las12-f0.las")], [at[0] + 3000, at[1]], 0.5)))
    out.append(case("arama karesi kaynağın kutusuna değmezse dosya okunmaz", queried(
        [(1, 0, "las12-f0.las")], [400000.0, 4300000.0], 1.0)))
    at = near_point("meta-geokeys.las", 7)
    out.append(case("iki kaynaklı bulut: kaynağın sırası noktanın kimliğinde", queried(
        [(22, 0, "meta-wkt.las"), (22, 1, "meta-geokeys.las")], at, 0.3)))
    at = near_point("meta-wkt.las", 11)
    # Without uids every visible cloud is looked at: 12 and 22 hold meta-wkt.las, 21 (hidden) is not looked at.
    visible = [(c + 1, 0, n) for c, n in enumerate(CLOUD_FILES)] + [(14, 0, "bulut.xyz")] + [
        (18, 0, "las12-f0.las"), (20, 0, "las12-f0.las"), (22, 0, "meta-wkt.las"), (22, 1, "meta-geokeys.las")]
    # The clouds of several files (23–33): every source that is a file that is there (a text one too).
    for e in drawing()["entities"][22:]:
        for si, src in enumerate(e["sources"]):
            name = src["file"].rsplit("/", 1)[-1]
            if name != "yok.las":
                visible.append((e["id"], si, name))
    out.append(case("uids verilmezse görünen bulutlar: gizli katmandaki bakılmaz, eşit uzaklıkta ilk bulut", queried(
        visible, at, 0.3, inp={"at": {"x": at[0], "y": at[1]}, "radius": 0.3}, warnings=[ANY] * 0)))
    out[-1]["steps"][0]["result"]["warnings"] = ANY
    out.append(case("gizli katmandaki bulut adıyla sorulunca bakılır", queried([(21, 0, "meta-wkt.las")], at, 0.3)))
    out.append(case("plan aynı cevabı verir", queried([(12, 0, "meta-wkt.las")], at, 0.3, op="plan")))
    out += [
        case("metin bulutu: satırlarından okunur, en yakın noktası satırının sırasıyla", queried(
            [(14, 0, "bulut.xyz")], [500013.383, 4400092.830], 0.5)),
        case("dosyası olmayan kaynak söylenir", step(
            {"at": {"x": near_point("las12-f0.las", 17)[0], "y": near_point("las12-f0.las", 17)[1]}, "radius": 1.0,
             "uids": ["$uidOf:15"]},
            done({"read": 0, "revision": "$current"},
                 [{"code": "cloud_source_unreadable", "message": ANY, "path": "clouds[0].sources[0]"}]))),
        case("metin bulutu: erimde nokta yok, bütün dosya okunur", queried(
            [(14, 0, "bulut.xyz")], [500050.0, 4400050.0], 1.0)),
        case("metin kaynağı ve döşeme: ikisinde de aranır", queried(
            [(28, 0, "bulut.xyz"), (28, 1, "tile-a.las")], [500013.383, 4400092.830], 2.0)),
        case("erim sıfır", step({"at": {"x": 500050.0, "y": 4400050.0}, "radius": 0.0}, failed(
            "invalid_reach", "Aramanın erimi sıfırdan büyük olmalı; noktanın en çok ne kadar uzakta aranacağını verin.",
            "radius"))),
        case("eksi erim", step({"at": {"x": 500050.0, "y": 4400050.0}, "radius": -2.0}, failed(
            "invalid_reach", "Aramanın erimi sıfırdan büyük olmalı; noktanın en çok ne kadar uzakta aranacağını verin.",
            "radius"))),
        case("geçersiz kimlik", step({"at": {"x": 0.0, "y": 0.0}, "radius": 1.0, "uids": ["bulut"]},
                                     failed("invalid_uid", f"“bulut” {UID_TEXT}", "uids[0]"))),
        case("çizimde olmayan bulut", step({"at": {"x": 0.0, "y": 0.0}, "radius": 1.0, "uids": [GONE]}, failed(
            "entity_not_found",
            f"“{GONE}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
            "uids[0]"))),
        case("nokta bulutu olmayan nesne", step({"at": {"x": 0.0, "y": 0.0}, "radius": 1.0, "uids": ["$uidOf:19"]}, failed(
            "not_a_pointcloud", "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.",
            "uids[0]"))),
    ]
    return out


NOTE = ("ADR 0536, 0552. Bağımsız başvuru scripts/fixtures/pointcloud_describe_cases.py: dosyalar laspy'yle ve başlık "
        "baytlarından, sistemler pyproj'la, boyutlar LAS 1.4 R15'in tablolarından, COPC düğümleri COPC 1.0 belirtiminden; "
        "KentOS kodu yok.")


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": drawing(), "cases": cases}


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    files = {
        "cad.pointclouds.describe": ("Nokta bulutunun bilgisi", describe_cases),
        "cad.pointclouds.query": ("XYZ sor", query_cases),
    }
    bad = 0
    for command, (title, make) in files.items():
        out = OUT / f"{command}.json"
        text = json.dumps(build(command, title, make()), ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv:
            if not out.exists() or out.read_text("utf-8") != text:
                print(f"{out.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad += 1
            continue
        out.write_text(text, "utf-8")
        print(f"yazıldı: {out.relative_to(ROOT)}")
    if "--check" in sys.argv and not bad:
        print("Nokta bulutu bilgisi ve XYZ sor durumları tutarlı")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
