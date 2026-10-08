#!/usr/bin/env python3
"""Point clouds' scene (docs/adr/0207): a small, made-up village's airborne LiDAR for the screenshots and the traces.

fixtures/interaction/v1/pointclouds/koy.laz: LAS 1.4, point format 7 (colours), compressed by LASzip itself (laspy's
binding), TUREF / TM36's WKT: a 120 × 90 m village on a gentle slope, about 71 000 points: the ground (class 2) in grass
and earth, a road (11) of asphalt, six houses (6) with gable roofs of tiles, trees (3, 4 and 5 by their height above the
ground) whose crowns return twice, a few low noise points (7); intensities by surface, GPS times by flight line.
fixtures/interaction/v1/pointclouds.kcad: a GIS project in TUREF / TM36 at 1:500 with the cloud on its layer and the
village's parcels over it.

Everything comes from a fixed seed: running it again writes the same bytes.

Needs laspy with the LASzip backend; when this Python lacks it the script runs itself with .run/pyref
(python3 -m venv .run/pyref && .run/pyref/bin/pip install 'laspy[laszip]==2.7.0' numpy).

    python3 scripts/fixtures/pointcloud_scene.py          # write
    python3 scripts/fixtures/pointcloud_scene.py --check  # compare with what is on disk
"""

import argparse
import io
import json
import math
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
try:
    import laspy
    import numpy as np
    from laspy import LazBackend
except ImportError:
    venv = ROOT / ".run" / "pyref"
    py = venv / "bin" / "python"
    if py.exists() and Path(sys.prefix).resolve() != venv.resolve():
        os.execv(str(py), [str(py), *sys.argv])
    sys.exit("laspy[laszip] gerekli: python3 -m venv .run/pyref && .run/pyref/bin/pip install 'laspy[laszip]==2.7.0' numpy")

DIR = ROOT / "fixtures" / "interaction" / "v1"
OUT = DIR / "pointclouds"
SRID = 5256
X0, Y0 = 487600.0, 4420300.0
W, H = 120.0, 90.0
WKT = (
    'PROJCS["TUREF / TM36",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,'
    '298.257222101,AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","1057"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],'
    'UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","5252"]],'
    'PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",36],'
    'PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],'
    'UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Northing",NORTH],AXIS["Easting",EAST],AUTHORITY["EPSG","5256"]]'
)

# The houses: centre (from the lower left), size along and across the ridge, the ridge's turn (degrees), eaves height.
HOUSES = [
    (22.0, 66.0, 13.0, 9.0, 8.0, 5.6),
    (44.0, 70.0, 11.0, 8.0, 12.0, 5.2),
    (78.0, 68.0, 14.0, 10.0, -6.0, 6.0),
    (100.0, 64.0, 10.0, 8.0, 3.0, 5.0),
    (30.0, 22.0, 12.0, 9.0, -14.0, 5.4),
    (86.0, 24.0, 15.0, 10.0, 9.0, 6.2),
]


def ground(x, y):
    """The slope's height at local x, y (metres): rising north-east, a soft swell and a hollow."""
    return (842.0 + 0.045 * x + 0.06 * y + 1.4 * np.sin(x / 19.0) * np.cos(y / 23.0)
            - 1.1 * np.exp(-((x - 60.0) ** 2 + (y - 46.0) ** 2) / 300.0))


def road_y(x):
    """The road's middle at x: a gentle curve across the village."""
    return 45.0 + 6.0 * np.sin(x / 26.0)


def in_house(x, y, h):
    cx, cy, a, b, turn, _ = h
    t = math.radians(turn)
    u = (x - cx) * math.cos(t) + (y - cy) * math.sin(t)
    v = -(x - cx) * math.sin(t) + (y - cy) * math.cos(t)
    return (np.abs(u) <= a / 2) & (np.abs(v) <= b / 2), u, v


def points():
    rng = np.random.default_rng(207)
    spacing = 0.42
    gx, gy = np.meshgrid(np.arange(0.0, W, spacing), np.arange(0.0, H, spacing))
    x = gx.ravel() + rng.uniform(-0.14, 0.14, gx.size)
    y = gy.ravel() + rng.uniform(-0.14, 0.14, gy.size)
    keep = (x >= 0) & (x <= W) & (y >= 0) & (y <= H)
    x, y = x[keep], y[keep]
    z = ground(x, y) + rng.normal(0.0, 0.025, x.size)
    cls = np.full(x.size, 2, np.uint8)
    # Grass and earth: green where the swell is, browner in the hollow, speckled.
    t = np.clip((z - 840.0) / 14.0, 0, 1)
    speck = rng.normal(0.0, 12.0, x.size)
    r = 96 + 60 * (1 - t) + speck
    g = 128 + 34 * t + speck
    b = 70 + 14 * t + speck * 0.6
    inten = 26000 + rng.normal(0, 2500, x.size)
    nret = np.ones(x.size, np.uint8)
    ret = np.ones(x.size, np.uint8)
    # The road: asphalt, 7 m wide, its edges lighter.
    d = np.abs(y - road_y(x))
    on_road = d <= 3.5
    cls[on_road] = 11
    edge = on_road & (d > 3.1)
    r[on_road] = 92 + speck[on_road] * 0.3
    g[on_road] = 94 + speck[on_road] * 0.3
    b[on_road] = 98 + speck[on_road] * 0.3
    r[edge] = g[edge] = b[edge] = 205
    inten[on_road] = 14000 + rng.normal(0, 1200, on_road.sum())
    inten[edge] = 40000
    # The houses' roofs replace the ground under them.
    for h in HOUSES:
        inside, u, v = in_house(x, y, h)
        _, _, a, b_, _, eaves = h
        base = ground(h[0], h[1])
        rise = 0.55 * (b_ / 2 - np.abs(v[inside]))
        z[inside] = base + eaves + rise + rng.normal(0.0, 0.02, inside.sum())
        cls[inside] = 6
        light = np.where(v[inside] > 0, 1.0, 0.78)
        r[inside] = (176 + rng.normal(0, 8, inside.sum())) * light
        g[inside] = (84 + rng.normal(0, 6, inside.sum())) * light
        b[inside] = (58 + rng.normal(0, 5, inside.sum())) * light
        inten[inside] = 43000 + rng.normal(0, 3000, inside.sum())
    xs, ys, zs = [x], [y], [z]
    cs, rs, gs, bs, ins, nrs, rts = [cls], [r], [g], [b], [inten], [nret], [ret]
    # Trees: crowns of returns over their ground, twice where the beam went on to the ground.
    trees = []
    while len(trees) < 34:
        tx, ty = rng.uniform(4, W - 4), rng.uniform(4, H - 4)
        if abs(ty - road_y(tx)) < 6.5:
            continue
        if any(in_house(np.array([tx]), np.array([ty]), (h[0], h[1], h[2] + 5, h[3] + 5, h[4], h[5]))[0][0]
               for h in HOUSES):
            continue
        trees.append((tx, ty, rng.uniform(1.6, 3.8), rng.uniform(2.5, 12.0)))
    for tx, ty, radius, height in trees:
        n = int(170 * radius * radius / 4)
        ang = rng.uniform(0, 2 * math.pi, n)
        rad = radius * np.sqrt(rng.uniform(0, 1, n))
        px, py = tx + rad * np.cos(ang), ty + rad * np.sin(ang)
        top = ground(np.array([tx]), np.array([ty]))[0] + height
        pz = top - (rad / radius) ** 2 * radius * 0.9 + rng.normal(0, 0.12, n)
        above = pz - ground(px, py)
        c = np.where(above <= 0.5, 3, np.where(above <= 2.0, 4, 5)).astype(np.uint8)
        shade = rng.uniform(0.75, 1.1, n)
        xs.append(px)
        ys.append(py)
        zs.append(pz)
        cs.append(c)
        rs.append(46 * shade)
        gs.append((92 + 30 * (pz - pz.min()) / max(np.ptp(pz), 0.1)) * shade)
        bs.append(38 * shade)
        ins.append(12000 + rng.normal(0, 2000, n))
        nrs.append(np.full(n, 2, np.uint8))
        rts.append(np.ones(n, np.uint8))
    # Noise: a few low points under the ground.
    k = 24
    nx, ny = rng.uniform(0, W, k), rng.uniform(0, H, k)
    xs.append(nx)
    ys.append(ny)
    zs.append(ground(nx, ny) - rng.uniform(2.0, 6.0, k))
    cs.append(np.full(k, 7, np.uint8))
    rs.append(np.full(k, 230.0))
    gs.append(np.full(k, 40.0))
    bs.append(np.full(k, 200.0))
    ins.append(np.full(k, 3000.0))
    nrs.append(np.ones(k, np.uint8))
    rts.append(np.ones(k, np.uint8))
    x, y, z = np.concatenate(xs), np.concatenate(ys), np.concatenate(zs)
    # The flight's order: lines along x, south to north, alternating.
    line = np.floor(y / 15.0)
    order = np.lexsort((np.where(line % 2 == 0, x, -x), line))
    cols = [np.concatenate(v)[order] for v in (cs, rs, gs, bs, ins, nrs, rts)]
    return x[order], y[order], z[order], cols


def cloud_bytes():
    x, y, z, (cls, r, g, b, inten, nret, ret) = points()
    header = laspy.LasHeader(point_format=7, version="1.4")
    header.offsets = [X0, Y0, 800.0]
    header.scales = [0.001, 0.001, 0.001]
    header.system_identifier = "KentOS sahnesi"
    header.generating_software = "pointcloud_scene.py"
    header.vlrs.append(laspy.VLR("LASF_Projection", 2112, "OGC Coordinate System WKT", WKT.encode() + b"\0"))
    header.global_encoding.wkt = True
    las = laspy.LasData(header)
    las.x = X0 + x
    las.y = Y0 + y
    las.z = z
    las.classification = cls
    las.intensity = np.clip(inten, 0, 65535).astype(np.uint16)
    las.number_of_returns = nret
    las.return_number = ret
    las.red = (np.clip(r, 0, 255).astype(np.uint16)) * 257
    las.green = (np.clip(g, 0, 255).astype(np.uint16)) * 257
    las.blue = (np.clip(b, 0, 255).astype(np.uint16)) * 257
    las.gps_time = 400000.0 + np.arange(x.size) * 2e-5
    las.point_source_id = np.full(x.size, 17, np.uint16)
    out = io.BytesIO()
    las.write(out, do_compress=True, laz_backend=LazBackend.Laszip)
    return out.getvalue(), las


def drawing(las):
    """The scene's GIS project: the cloud on its layer, the village's parcels over it."""
    layer = lambda id_, name, color: {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False,
                                      "expanded": True, "style": {"color": color, "lineType": "continuous",
                                                                  "lineWeight": 0.25}, "children": []}
    h = las.header
    bounds = [float(v) for v in (*h.mins, *h.maxs)]
    count = int(h.point_count)
    z = np.asarray(las.z)
    lo, hi = (float(np.percentile(z, 2)), float(np.percentile(z, 98)))
    entities = [
        {"kind": "pointcloud", "id": 1, "layerId": "koy", "attrs": {},
         "sources": [{"file": "pointclouds/koy.laz", "format": "laz", "count": count, "bounds": bounds}],
         "bounds": bounds, "count": count, "srid": SRID,
         "style": {"render": "rgb", "min": round(lo, 2), "max": round(hi, 2), "size": 3}},
    ]
    # The parcels: north and south of the road, west to east.
    for k, (a, b_, y1, y2) in enumerate([(4, 36, 54, 88), (36, 62, 56, 88), (62, 92, 55, 88), (92, 118, 52, 88),
                                         (4, 56, 4, 36), (56, 118, 4, 38)]):
        entities.append({"kind": "polygon", "id": len(entities) + 1, "layerId": "parsel",
                         "attrs": {"Parsel": f"{211 + k}"},
                         "pts": [{"x": X0 + a, "y": Y0 + y1}, {"x": X0 + b_, "y": Y0 + y1},
                                 {"x": X0 + b_, "y": Y0 + y2}, {"x": X0 + a, "y": Y0 + y2}]})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Köy",
        "settings": {"srid": SRID, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 500.0, "workspace": "gis"},
        "origin": {"x": X0 + 60.0, "y": Y0 + 45.0},
        "layers": [layer("parsel", "Parsel", "#E8B04A"), layer("koy", "Köy (LiDAR)", "#5B8FCF"), layer("0", "0", "fg")],
        "activeLayer": "parsel",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    data, _ = cloud_bytes()
    # What the file's header says, as the window reads it.
    las = laspy.read(io.BytesIO(data))
    doc = json.dumps(drawing(las), ensure_ascii=False, indent=1) + "\n"
    targets = [(OUT / "koy.laz", data), (DIR / "pointclouds.kcad", doc.encode())]
    if args.check:
        bad = [str(p.relative_to(ROOT)) for p, b in targets if not p.exists() or p.read_bytes() != b]
        if bad:
            sys.exit("farklı: " + ", ".join(bad))
        print("pointcloud_scene: aynı")
        return
    OUT.mkdir(parents=True, exist_ok=True)
    for p, b in targets:
        p.write_bytes(b)
    print(f"{len(data)} bayt, {int(las.header.point_count)} nokta")


if __name__ == "__main__":
    main()
