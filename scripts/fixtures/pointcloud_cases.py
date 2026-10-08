#!/usr/bin/env python3
"""Point clouds (docs/adr/0207): sample files and what reading them gives, written without KentOS code.

fixtures/pointcloud/v1/files/: LAS and LAZ files written by laspy, the LAZ ones compressed by LASzip itself (the C++
library through its Python binding, not laz-rs, which KentOS reads them with): LAS 1.2 point formats 0, 1, 2 and 3
(GeoTIFF keys), 1.3's format 1, 1.4's formats 6, 7 and 8 (WKT 1, WKT 2 and a compound system), extra bytes, a LAZ of
three LASzip chunks; text clouds (XYZ, PTS, CSV, TXT); and broken files, each with the fault the reader must name.

fixtures/pointcloud/v1/cases.json: for each file what the window shows (kind, version, record format and length, extra
bytes, count, scale, offset, the header's bounds, the system's EPSG code) and its points: the raw records' FNV-1a 64
hash (laspy's decoding, LASzip's for LAZ), the first three points' fields, and every record moved to format 6, 7 or 8
by the ADR's rules (§4, LAS 1.4 R15's legacy rules: the scan angle rank × 1000 / 6, the half away from zero) hashed.

Needs laspy with the LASzip backend; when this Python lacks it the script runs itself with .run/pyref
(python3 -m venv .run/pyref && .run/pyref/bin/pip install 'laspy[laszip]==2.7.0' numpy).

    python3 scripts/fixtures/pointcloud_cases.py          # write
    python3 scripts/fixtures/pointcloud_cases.py --check  # compare with what is on disk
"""

import argparse
import json
import os
import struct
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

DIR = ROOT / "fixtures" / "pointcloud" / "v1"
FILES = DIR / "files"
OUT = DIR / "cases.json"

TM30_WKT1 = (
    'PROJCS["TUREF / TM30",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,298.257222101,'
    'AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","1057"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",'
    '0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","5252"]],PROJECTION["Transverse_Mercator"],'
    'PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",30],PARAMETER["scale_factor",1],'
    'PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],'
    'AXIS["Easting",EAST],AXIS["Northing",NORTH],AUTHORITY["EPSG","5254"]]'
)
UTM35_WKT2 = (
    'PROJCRS["WGS 84 / UTM zone 35N",BASEGEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,'
    '298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",4326]],'
    'CONVERSION["UTM zone 35N",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Latitude of natural origin",0,'
    'ANGLEUNIT["degree",0.0174532925199433]],PARAMETER["Longitude of natural origin",27,ANGLEUNIT["degree",0.0174532925199433]],'
    'PARAMETER["Scale factor at natural origin",0.9996,SCALEUNIT["unity",1]],PARAMETER["False easting",500000,'
    'LENGTHUNIT["metre",1]],PARAMETER["False northing",0,LENGTHUNIT["metre",1]]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],'
    'LENGTHUNIT["metre",1]],AXIS["(N)",north,ORDER[2],LENGTHUNIT["metre",1]],ID["EPSG",32635]]'
)
COMPOUND_WKT1 = f'COMPD_CS["TUREF / TM30 + TUDKA-99 height",{TM30_WKT1},VERT_CS["TUDKA-99 height",VERT_DATUM["TUDKA-99",2005],UNIT["metre",1],AXIS["Up",UP]]]'


def fnv(data: bytes) -> str:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def standard_len(fmt: int) -> int:
    return {0: 20, 1: 28, 2: 26, 3: 34, 4: 57, 5: 63, 6: 30, 7: 36, 8: 38, 9: 59, 10: 67}[fmt]


def wide_format(fmt: int) -> int:
    return 7 if fmt in (2, 3, 5, 7) else 8 if fmt in (8, 10) else 6


def rank_to_angle(rank: int) -> int:
    """LAS 1.4 R15's legacy rule as the ADR states it: rank × 1000 / 6, the half away from zero."""
    n = rank * 1000
    q = abs(n) // 6
    m = abs(n) % 6
    if m >= 3:
        q += 1
    return q if n >= 0 else -q


def widen(fmt: int, rec: bytes) -> bytes:
    """A record of `fmt` as format 6, 7 or 8 (docs/adr/0207 §4), its extra bytes kept."""
    to = wide_format(fmt)
    x, y, z, inten = struct.unpack_from("<iiiH", rec, 0)
    if fmt >= 6:
        b14, b15, cls, user, angle, src = struct.unpack_from("<BBBBhH", rec, 14)
        ret, nret = b14 & 0x0F, b14 >> 4
        flags = b15 & 0x0F
        dir_edge = b15 & 0xC0
        gps = struct.unpack_from("<d", rec, 22)[0]
    else:
        b14, b15, rank, user, src = struct.unpack_from("<BBbBH", rec, 14)
        ret, nret = b14 & 0x07, (b14 >> 3) & 0x07
        cls = b15 & 0x1F
        flags = (b15 >> 5) & 0x07
        dir_edge = b14 & 0xC0
        angle = rank_to_angle(rank)
        gps = struct.unpack_from("<d", rec, 20)[0] if fmt in (1, 3, 4, 5) else 0.0
    out = bytearray(struct.pack("<iiiHBBBBhHd", x, y, z, inten, (ret & 0x0F) | ((nret & 0x0F) << 4), flags | dir_edge, cls, user, angle, src, gps))
    rgb_at = {2: 20, 3: 28, 5: 28, 7: 30, 8: 30, 10: 30}.get(fmt)
    if to >= 7:
        out += rec[rgb_at:rgb_at + 6] if rgb_at is not None else bytes(6)
    if to == 8:
        out += rec[36:38] if fmt in (8, 10) else bytes(2)
    out += rec[standard_len(fmt):]
    return bytes(out)


def fields(las, i: int) -> dict:
    p = las.points
    d = {
        "x": int(p.X[i]),
        "y": int(p.Y[i]),
        "z": int(p.Z[i]),
        "intensity": int(p.intensity[i]),
        "returnNumber": int(p.return_number[i]),
        "numberOfReturns": int(p.number_of_returns[i]),
        "class": int(p.classification[i]),
        "userData": int(p.user_data[i]),
        "sourceId": int(p.point_source_id[i]),
    }
    names = set(las.point_format.dimension_names)
    if "gps_time" in names:
        d["gpsTime"] = float(p.gps_time[i])
    if "red" in names:
        d["rgb"] = [int(p.red[i]), int(p.green[i]), int(p.blue[i])]
    if "nir" in names:
        d["nir"] = int(p.nir[i])
    if "scan_angle" in names:
        d["scanAngle"] = int(p.scan_angle[i])
    else:
        d["scanAngleRank"] = int(p.scan_angle_rank[i])
    return d


def points(n: int, seed: int, wide: bool):
    rng = np.random.default_rng(seed)
    xs = rng.integers(0, 40000, n)
    ys = rng.integers(0, 30000, n)
    zs = (5000 + 800 * np.sin(xs / 3000.0) + rng.integers(-200, 200, n)).astype(np.int64)
    nret = rng.integers(1, 5 if not wide else 7, n)
    ret = np.array([rng.integers(1, k + 1) for k in nret])
    classes = rng.choice([1, 2, 3, 4, 5, 6, 9, 17] if not wide else [0, 1, 2, 3, 4, 5, 6, 7, 9, 11, 17, 18, 20, 22, 64, 200], n)
    return dict(
        X=xs, Y=ys, Z=zs,
        intensity=rng.integers(0, 65536, n),
        return_number=ret, number_of_returns=nret,
        classification=classes,
        user_data=rng.integers(0, 256, n),
        point_source_id=rng.integers(1, 4, n),
        scan_direction_flag=rng.integers(0, 2, n),
        edge_of_flight_line=rng.integers(0, 2, n),
        synthetic=rng.integers(0, 2, n),
        key_point=rng.integers(0, 2, n),
        withheld=rng.integers(0, 2, n),
        gps_time=np.cumsum(rng.random(n)) + 3.0e8,
        red=rng.integers(0, 256, n) * 257,
        green=rng.integers(0, 256, n) * 257,
        blue=rng.integers(0, 256, n) * 257,
        nir=rng.integers(0, 65536, n),
        scan_angle_rank=rng.integers(-30, 31, n),
        scan_angle=rng.integers(-5000, 5001, n),
        overlap=rng.integers(0, 2, n),
        scanner_channel=rng.integers(0, 4, n),
    )


def geokeys(epsg: int) -> laspy.VLR:
    keys = [1, 1, 0, 3, 1024, 0, 1, 1, 1025, 0, 1, 1, 3072, 0, 1, epsg]
    return laspy.VLR("LASF_Projection", 34735, "GeoTIFF GeoKeyDirectoryTag", struct.pack(f"<{len(keys)}H", *keys))


def wkt_vlr(text: str) -> laspy.VLR:
    return laspy.VLR("LASF_Projection", 2112, "OGC Coordinate System WKT", text.encode() + b"\0")


SPECS = [
    # name, version, format, compressed, n, system, extra
    ("las12-f0.las", "1.2", 0, False, 300, ("keys", 5254), False),
    ("las12-f1.laz", "1.2", 1, True, 300, ("keys", 5254), False),
    ("las12-f2.las", "1.2", 2, False, 300, None, False),
    ("las12-f3.laz", "1.2", 3, True, 300, ("keys", 32635), True),
    ("las13-f1.las", "1.3", 1, False, 200, ("keys", 5254), False),
    ("las14-f6.laz", "1.4", 6, True, 300, ("wkt", TM30_WKT1), False),
    ("las14-f7.las", "1.4", 7, False, 300, ("wkt", UTM35_WKT2), True),
    ("las14-f8.laz", "1.4", 8, True, 300, ("wkt", COMPOUND_WKT1), False),
]


def build(name, version, fmt, compressed, n, system, extra) -> Path:
    header = laspy.LasHeader(point_format=fmt, version=version)
    header.scales = np.array([0.01, 0.01, 0.001])
    header.offsets = np.array([500000.0, 4400000.0, 0.0])
    header.system_identifier = "KentOS fixtures"
    header.generating_software = "laspy 2.7.0 + LASzip"
    header.creation_date = None
    las = laspy.LasData(header)
    if extra:
        las.add_extra_dims([laspy.ExtraBytesParams("ek1", "u1"), laspy.ExtraBytesParams("ek2", "u2")])
    p = points(n, seed=len(name) * 97 + fmt, wide=fmt >= 6)
    las.X = p["X"]
    las.Y = p["Y"]
    las.Z = p["Z"]
    names = set(las.point_format.dimension_names)
    for k, v in p.items():
        if k in ("X", "Y", "Z") or k not in names:
            continue
        setattr(las, k, v)
    if extra:
        rng = np.random.default_rng(7)
        las.ek1 = rng.integers(0, 256, n)
        las.ek2 = rng.integers(0, 65536, n)
    if system is not None:
        kind, value = system
        if kind == "keys":
            las.header.vlrs.append(geokeys(value))
        else:
            las.header.vlrs.append(wkt_vlr(value))
            las.header.global_encoding.wkt = True
    path = FILES / name
    las.write(str(path), do_compress=compressed, laz_backend=LazBackend.Laszip if compressed else None)
    return path


def build_many_chunks() -> Path:
    """120 000 points: LASzip's default chunks of 50 000 make three."""
    header = laspy.LasHeader(point_format=0, version="1.2")
    header.scales = np.array([0.01, 0.01, 0.01])
    header.offsets = np.array([0.0, 0.0, 0.0])
    header.system_identifier = "KentOS fixtures"
    header.generating_software = "laspy 2.7.0 + LASzip"
    las = laspy.LasData(header)
    n = 120_000
    i = np.arange(n)
    las.X = (i % 400) * 50
    las.Y = (i // 400) * 50
    las.Z = (1000 + 300 * np.sin(i / 700.0)).astype(np.int64)
    las.classification = np.full(n, 2)
    las.intensity = (i * 7) % 65536
    las.return_number = np.ones(n, dtype=np.int64)
    las.number_of_returns = np.ones(n, dtype=np.int64)
    path = FILES / "many-chunks.laz"
    las.write(str(path), do_compress=True, laz_backend=LazBackend.Laszip)
    return path


def build_text():
    rng = np.random.default_rng(11)
    n = 50
    xs = 500000 + rng.integers(0, 100000, n) / 1000.0
    ys = 4400000 + rng.integers(0, 100000, n) / 1000.0
    zs = 100 + rng.integers(0, 50000, n) / 1000.0
    (FILES / "bulut.xyz").write_text("X Y Z\n" + "".join(f"{x:.3f} {y:.3f} {z:.3f}\n" for x, y, z in zip(xs, ys, zs)))
    inten = rng.integers(0, 2048, n)
    rgb = rng.integers(0, 256, (n, 3))
    (FILES / "bulut.pts").write_text(f"{n}\n" + "".join(
        f"{x:.3f} {y:.3f} {z:.3f} {i} {c[0]} {c[1]} {c[2]}\n" for x, y, z, i, c in zip(xs, ys, zs, inten, rgb)))
    (FILES / "bulut.csv").write_text("X,Y,Z,R,G,B\n" + "".join(
        f"{x:.2f},{y:.2f},{z:.2f},{c[0]},{c[1]},{c[2]}\n" for x, y, z, c in zip(xs, ys, zs, rgb)))
    frac = rng.random(n)
    (FILES / "bulut.txt").write_text("".join(f"{x:.3f}\t{y:.3f}\t{z:.3f}\t{f:.4f}\n" for x, y, z, f in zip(xs, ys, zs, frac)))


BROKEN = {
    "not-las.las": "“LASF” ile başlamıyor",
    "version-15.las": "LAS 1.5 okunmuyor",
    "short.las": "Dosya kısa",
    "record-short.las": "Nokta kaydı 10 bayt",
    "scale-zero.las": "Ölçek çarpanları",
    "no-table.laz": "parça tablosu yok",
    "format-11.las": "Nokta kaydı biçimi 11 tanınmıyor",
}


def build_broken():
    b = FILES / "broken"
    b.mkdir(parents=True, exist_ok=True)
    good = (FILES / "las12-f0.las").read_bytes()
    (b / "not-las.las").write_bytes(b"LASX" + good[4:])
    v = bytearray(good)
    v[25] = 5
    (b / "version-15.las").write_bytes(bytes(v))
    (b / "short.las").write_bytes(good[: len(good) - 100])
    r = bytearray(good)
    struct.pack_into("<H", r, 105, 10)
    (b / "record-short.las").write_bytes(bytes(r))
    s = bytearray(good)
    struct.pack_into("<d", s, 131, 0.0)
    (b / "scale-zero.las").write_bytes(bytes(s))
    f = bytearray(good)
    f[104] = 11
    (b / "format-11.las").write_bytes(bytes(f))
    laz = bytearray((FILES / "las12-f1.laz").read_bytes())
    data_offset = struct.unpack_from("<I", laz, 96)[0]
    table_at = struct.unpack_from("<q", laz, data_offset)[0]
    struct.pack_into("<q", laz, data_offset, -1)
    # LASzip also writes the offset in the file's last 8 bytes when it cannot go back; cut the table and those.
    (b / "no-table.laz").write_bytes(bytes(laz[:table_at]))


def raw_vlrs(path: Path):
    """The VLRs as the file holds them: user, record and payload (struct, not laspy's parsed ones)."""
    b = path.read_bytes()
    header_size = struct.unpack_from("<H", b, 94)[0]
    count = struct.unpack_from("<I", b, 100)[0]
    at = header_size
    out = []
    for _ in range(count):
        user = b[at + 2: at + 18].split(b"\0")[0].decode()
        record, length = struct.unpack_from("<HH", b, at + 18)
        out.append((user, record, b[at + 54: at + 54 + length]))
        at += 54 + length
    return out


def system_of(path: Path):
    """The horizontal system's EPSG code: the WKT's (a compound's horizontal part's AUTHORITY), else the
    GeoTIFF keys' ProjectedCSType."""
    import re
    vlrs = raw_vlrs(path)
    for user, record, data in vlrs:
        if user == "LASF_Projection" and record == 2112:
            text = data.split(b"\0")[0].decode()
            if text.startswith("COMPD_CS"):
                inner = text[text.index("PROJCS["):]
                depth = 0
                for i, c in enumerate(inner):
                    depth += c == "["
                    depth -= c == "]"
                    if depth == 0 and c == "]":
                        text = inner[: i + 1]
                        break
            m = re.findall(r'(?:AUTHORITY\["EPSG","(\d+)"\]|ID\["EPSG",(\d+)\])', text)
            last = m[-1]
            return int(last[0] or last[1])
    for user, record, data in vlrs:
        if user == "LASF_Projection" and record == 34735:
            keys = struct.unpack(f"<{len(data) // 2}H", data)
            for k in range(keys[3]):
                kid, loc, cnt, val = keys[4 + 4 * k: 8 + 4 * k]
                if kid == 3072 and loc == 0:
                    return val
    return None


def case(path: Path) -> dict:
    las = laspy.read(str(path))
    h = las.header
    fmt = h.point_format.id
    raw = las.points.array.tobytes()
    rec_len = h.point_format.size
    recs = [raw[i:i + rec_len] for i in range(0, len(raw), rec_len)]
    wide = b"".join(widen(fmt, r) for r in recs)
    epsg = system_of(path)
    return {
        "file": path.name,
        "kind": "laz" if path.suffix == ".laz" else "las",
        "version": f"{h.version.major}.{h.version.minor}",
        "format": fmt,
        "recordLen": rec_len,
        "extra": rec_len - standard_len(fmt),
        "count": int(h.point_count),
        "scale": [float(v) for v in h.scales],
        "offset": [float(v) for v in h.offsets],
        "bounds": [float(h.mins[0]), float(h.mins[1]), float(h.mins[2]), float(h.maxs[0]), float(h.maxs[1]), float(h.maxs[2])],
        "epsg": epsg,
        "rawHash": fnv(raw),
        "wideFormat": wide_format(fmt),
        "wideHash": fnv(wide),
        "first": [fields(las, i) for i in range(3)],
    }


def text_cases() -> list:
    """What a text cloud reads as (docs/adr/0207 §2): columns by their count, the scale from the decimals."""
    out = []
    for name, cols in [("bulut.xyz", "xyz"), ("bulut.pts", "xyzirgb"), ("bulut.csv", "xyzrgb"), ("bulut.txt", "xyzi")]:
        lines = (FILES / name).read_text().splitlines()
        rows = []
        for line in lines:
            parts = line.replace(",", " ").replace(";", " ").split()
            try:
                vals = [float(p) for p in parts]
            except ValueError:
                continue
            if len(vals) < 3:
                continue
            rows.append((parts, vals))
        decimals = max(len(p.split(".")[1]) if "." in p else 0 for parts, _ in rows for p in parts[:3])
        xs = [v[0] for _, v in rows]
        ys = [v[1] for _, v in rows]
        zs = [v[2] for _, v in rows]
        # The offset: each axis's least value's whole part toward minus infinity; the integers by
        # the decimal text, never a float (the ADR's rule).
        from decimal import Decimal, ROUND_FLOOR
        dec = [[Decimal(p) for p in parts[:3]] for parts, _ in rows]
        offset = [int(min(r[k] for r in dec).to_integral_value(rounding=ROUND_FLOOR)) for k in range(3)]
        first = [int((dec[0][k] - offset[k]) * (Decimal(10) ** decimals)) for k in range(3)]
        out.append({
            "file": name,
            "columns": cols,
            "count": len(rows),
            "decimals": decimals,
            "offset": offset,
            "bounds": [min(xs), min(ys), min(zs), max(xs), max(ys), max(zs)],
            "first": rows[0][1],
            "firstInts": first,
        })
    return out


def write_all():
    FILES.mkdir(parents=True, exist_ok=True)
    for spec in SPECS:
        build(*spec)
    build_many_chunks()
    build_text()
    build_broken()


def gather() -> dict:
    files = [case(FILES / spec[0]) for spec in SPECS] + [case(FILES / "many-chunks.laz")]
    return {
        "about": "docs/adr/0207 §2, §4: written by scripts/fixtures/pointcloud_cases.py (laspy + LASzip), not by KentOS",
        "files": files,
        "text": text_cases(),
        "broken": [{"file": f"broken/{k}", "says": v} for k, v in BROKEN.items()],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    if not args.check:
        write_all()
    got = gather()
    text = json.dumps(got, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        on_disk = OUT.read_text()
        if on_disk != text:
            sys.exit("pointcloud cases.json farklı: dosyaları laspy'nin okuduğu yeniden yazılmalı")
        print(f"{OUT.relative_to(ROOT)}: {len(got['files'])} dosya, {len(got['text'])} metin bulutu, {len(got['broken'])} bozuk dosya aynı")
    else:
        OUT.write_text(text)
        print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
