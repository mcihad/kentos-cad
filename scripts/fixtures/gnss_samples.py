#!/usr/bin/env python3
"""Writes the hand-made sample GNSS files of fixtures/gnss/v1 (docs/adr/0169 §6): the GNSS import window's pictures and
flow tests on both platforms read them.

A receiver's session at the sample drawing (fixtures/document/v1/sample.json, TUREF / TM36): the parcel 101/7's four
corners measured as named waypoints and a walk along its western edge as a track, then the same corners logged by an
RTK receiver as NMEA. The positions are the drawing's TM36 coordinates (and a few centimetres off, as a measurement is)
taken to WGS 84 with PROJ (pyproj) by EPSG's way KentOS takes, so the imported points fall on the drawing.

- sample.gpx: GPX 1.1 with four waypoints (heights above the geoid and its separation; one without the separation), a
  track of five points with heights and times, a waypoint without a fix (fix none) and one whose latitude is not read.
- sample.nmea: NMEA 0183 with RMC (the date) and GGA sentences (RTK fixed and float, DGPS), a GSA sentence it passes
  over, a GGA without a fix (quality 0) and one whose checksum does not hold.

    python3 scripts/fixtures/gnss_samples.py           # writes the files
    python3 scripts/fixtures/gnss_samples.py --check   # writes nothing; compares
"""
import argparse
import sys
from functools import reduce
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from crs_transform_cases import transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "gnss" / "v1"

# The parcel's corners as measured (east, north in TUREF / TM36, height above the geoid), the geoid's separation there.
CORNERS = [
    ("101/7-1", 486512.352, 4420187.531, 1061.284),
    ("101/7-2", 486535.749, 4420188.715, 1061.902),
    ("101/7-3", 486538.233, 4420218.977, 1063.418),
    ("101/7-4", 486514.338, 4420220.546, 1062.775),
]
GEOID = 36.874
# A walk along the western edge, north to south.
TRACK = [(486514.0 - 0.4 * i, 4420218.0 - 7.5 * i, 1062.6 - 0.32 * i) for i in range(5)]

# TUREF / TM36 to WGS 84 by EPSG's way, the one KentOS takes (docs/adr/0167: TUREF to WGS 84 (1), EPSG:5261).
TO_WGS84, _, _ = transformer({"srid": 5256, "datum": "TUREF", "name": "TUREF / TM36"}, {"srid": 4326, "datum": "WGS84", "name": "WGS 84"})


def lat_lon(east, north):
    lon, lat = TO_WGS84.transform(east, north)
    return lat, lon


def gpx():
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<gpx version="1.1" creator="KentOS örnek" xmlns="http://www.topografix.com/GPX/1/1">',
        "  <metadata><name>101/7 parseli</name><time>2026-10-04T09:30:00Z</time></metadata>",
    ]
    for i, (name, e, n, h) in enumerate(CORNERS):
        lat, lon = lat_lon(e, n)
        geoid = "" if i == 3 else f"<geoidheight>{GEOID:.3f}</geoidheight>"
        lines += [
            f'  <wpt lat="{lat:.9f}" lon="{lon:.9f}">',
            f"    <ele>{h:.3f}</ele><time>2026-10-04T09:{31 + i:02d}:10Z</time>{geoid}",
            f"    <name>{name}</name><fix>3d</fix><sat>{15 - i}</sat><hdop>{0.6 + 0.1 * i:.1f}</hdop>",
            "  </wpt>",
        ]
    lat, lon = lat_lon(486530.0, 4420200.0)
    lines += [
        f'  <wpt lat="{lat:.9f}" lon="{lon:.9f}"><name>KONUMSUZ</name><fix>none</fix></wpt>',
        '  <wpt lat="39,93" lon="35,84"><name>BOZUK</name></wpt>',
        "  <trk><name>Batı sınırı</name><trkseg>",
    ]
    for i, (e, n, h) in enumerate(TRACK):
        lat, lon = lat_lon(e, n)
        lines.append(f'    <trkpt lat="{lat:.9f}" lon="{lon:.9f}"><ele>{h:.2f}</ele><time>2026-10-04T09:40:{5 * i:02d}Z</time></trkpt>')
    lines += ["  </trkseg></trk>", "</gpx>"]
    return "\n".join(lines) + "\n"


def nmea_coordinate(value, degrees, hemispheres):
    """ddmm.mmmmmmm or dddmm.mmmmmmm and its hemisphere."""
    a = abs(value)
    d = int(a)
    m = (a - d) * 60
    return f"{d:0{degrees}d}{m:010.7f}", hemispheres[0] if value >= 0 else hemispheres[1]


def sentence(body):
    check = reduce(lambda x, c: x ^ ord(c), body, 0)
    return f"${body}*{check:02X}"


def nmea():
    lat, lon = lat_lon(*CORNERS[0][1:3])
    la, ns = nmea_coordinate(lat, 2, "NS")
    lo, ew = nmea_coordinate(lon, 3, "EW")
    out = [sentence(f"GPRMC,093100.00,A,{la},{ns},{lo},{ew},0.02,0.0,041026,,,R")]
    qualities = [(4, 14, 0.62), (4, 13, 0.70), (5, 11, 0.95), (2, 9, 1.40)]
    for i, (name, e, n, h) in enumerate(CORNERS):
        lat, lon = lat_lon(e + 0.004, n - 0.003)
        la, ns = nmea_coordinate(lat, 2, "NS")
        lo, ew = nmea_coordinate(lon, 3, "EW")
        q, sats, hdop = qualities[i]
        out.append(sentence(f"GPGGA,09{31 + i:02d}10.00,{la},{ns},{lo},{ew},{q},{sats:02d},{hdop:.2f},{h:.3f},M,{GEOID:.3f},M,1.0,0001"))
        if i == 1:
            out.append(sentence("GPGSA,A,3,02,05,12,13,15,18,20,24,25,29,,,1.10,0.70,0.85"))
    lat, lon = lat_lon(486530.0, 4420200.0)
    la, ns = nmea_coordinate(lat, 2, "NS")
    lo, ew = nmea_coordinate(lon, 3, "EW")
    out.append(sentence(f"GPGGA,093500.00,{la},{ns},{lo},{ew},0,04,9.90,,M,,M,,"))
    broken = sentence(f"GPGGA,093510.00,{la},{ns},{lo},{ew},4,12,0.80,1062.000,M,{GEOID:.3f},M,1.0,0001")
    out.append(broken[:-2] + ("00" if broken[-2:] != "00" else "01"))
    return "\r\n".join(out) + "\r\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the files instead of writing them")
    args = parser.parse_args()
    files = {"sample.gpx": gpx(), "sample.nmea": nmea()}
    stale = []
    for name, text in files.items():
        path = DIR / name
        if args.check:
            if not path.exists() or path.read_bytes() != text.encode("utf-8"):
                stale.append(name)
        else:
            path.write_bytes(text.encode("utf-8"))
    if args.check:
        if stale:
            print(f"fixtures/gnss/v1: {', '.join(stale)} kurallardan çıkanla aynı değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print("fixtures/gnss/v1 örnekleri kurallardan çıkanla aynı.")
        return
    print(f"fixtures/gnss/v1: {', '.join(files)} yazıldı.")


if __name__ == "__main__":
    main()
