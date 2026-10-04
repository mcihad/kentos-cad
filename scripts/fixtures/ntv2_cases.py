#!/usr/bin/env python3
"""Independent reference of NTv2 grid shifts (docs/adr/0168 §3–§4).

Writes synthetic NTv2 grids into fixtures/geodesy/v1/ntv2/ with a writer of its own (the NTv2 layout: an overview of
eleven 16-byte records, each subgrid's header of eleven, then its records of four 32-bit floats, rows from south to
north, each row from east to west, longitudes positive west), and fixtures/geodesy/v1/ntv2.json from PROJ (pyproj's
`hgridshift`) on them, no KentOS code:

- **grids:** a grid over Türkiye (little-endian and the same big-endian), and nested subgrids (a parent, two children,
  a grandchild) whose shifts differ, so the densest one holding a point is seen to be the one used; what each file says
  (datums, axes, subgrids, extent).
- **shifts:** points forward and back through each grid, as PROJ shifts them (radians in and out, as PROJ's pipeline
  takes degrees: `unitconvert`); outside a grid, nothing; just inside an edge within PROJ's tolerance, the edge's value.
- **transforms:** the project's grid choice for ED50 and TUREF between transverse Mercator grids, both ways; a point
  outside the grid; a choice whose grid is not loaded ("noGrid").
- **broken:** files a reader must refuse, with why: too short, not NTv2, not seconds, a header not where it should be,
  an extent PROJ would refuse, a record count its extent does not allow, records missing, a shift that is not a number.

The core (crates/shared/geometry-core/tests/crs_ntv2.rs) and the web through its WASM must give the same: latitudes and
longitudes within 1e-11 degrees, grid points within 1e-6 m; the reasons exactly.
"""

import argparse
import hashlib
import json
import math
import struct
import sys
from pathlib import Path

import pyproj
from pyproj import Transformer

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "geodesy" / "v1" / "ntv2"
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "ntv2.json"

INTL = (6378388.0, 6378388.0 * (1 - 1 / 297.0))
GRS80 = (6378137.0, 6378137.0 * (1 - 1 / 298.257222101))


# ── The writer ────────────────────────────────────────────────────────────────────────────────────────────────────


def record(endian, name, value, kind):
    n = name.ljust(8).encode("ascii")[:8]
    if kind == "i":
        return n + struct.pack(endian + "i", value) + b"\0" * 4
    if kind == "d":
        return n + struct.pack(endian + "d", value)
    return n + value.ljust(8).encode("ascii")[:8]


def overview(endian, subgrids, gs_type="SECONDS", label="NUM_OREC"):
    return b"".join([
        record(endian, label, 11, "i"), record(endian, "NUM_SREC", 11, "i"),
        record(endian, "NUM_FILE", subgrids, "i"), record(endian, "GS_TYPE", gs_type, "s"),
        record(endian, "VERSION", "NTv2.0", "s"), record(endian, "SYSTEM_F", "ED50", "s"),
        record(endian, "SYSTEM_T", "TUREF", "s"), record(endian, "MAJOR_F", INTL[0], "d"),
        record(endian, "MINOR_F", INTL[1], "d"), record(endian, "MAJOR_T", GRS80[0], "d"),
        record(endian, "MINOR_T", GRS80[1], "d"),
    ])


def lat_shift(lat, lon):
    """Seconds north: smooth, not linear, so interpolation shows."""
    return 2.8 + 0.15 * math.sin(math.radians(lon * 7)) + 0.05 * (lat - 39)


def lon_shift(lat, lon):
    """Seconds east."""
    return 3.4 - 0.2 * math.cos(math.radians(lat * 9)) + 0.03 * (lon - 35)


def subgrid(endian, name, parent, west, east, south, north, step, extra=(0.0, 0.0), count=None, label="SUB_NAME",
            poison=None):
    """A subgrid over west..east, south..north (degrees east and north), `step` seconds apart, its shifts the fields
    plus `extra` (seconds north and east)."""
    s, n = south * 3600, north * 3600
    e_long, w_long = -east * 3600, -west * 3600
    rows = round((n - s) / step) + 1
    cols = round((w_long - e_long) / step) + 1
    gs = rows * cols if count is None else count
    head = b"".join([
        record(endian, label, name, "s"), record(endian, "PARENT", parent, "s"),
        record(endian, "CREATED", "20261004", "s"), record(endian, "UPDATED", "20261004", "s"),
        record(endian, "S_LAT", s, "d"), record(endian, "N_LAT", n, "d"), record(endian, "E_LONG", e_long, "d"),
        record(endian, "W_LONG", w_long, "d"), record(endian, "LAT_INC", step, "d"),
        record(endian, "LONG_INC", step, "d"), record(endian, "GS_COUNT", gs, "i"),
    ])
    body = bytearray()
    for r in range(rows):
        lat = (s + r * step) / 3600
        for c in range(cols):
            lon = -(e_long + c * step) / 3600
            a = lat_shift(lat, lon) + extra[0]
            b = -(lon_shift(lat, lon) + extra[1])
            if poison is not None and (r, c) == poison:
                a = float("nan")
            body += struct.pack(endian + "ffff", a, b, 0.05, 0.05)
    return head + bytes(body)


END = record("<", "END", 0, "i")

TR = ("<", [("TURKIYE", "NONE", 25.5, 45.5, 35.5, 42.5, 900.0)])
NESTED = ("<", [
    ("TURKIYE", "NONE", 29.5, 37.5, 36.5, 41.5, 1800.0),
    ("ANKARA", "TURKIYE", 32.0, 33.5, 39.5, 40.5, 450.0, (0.02, -0.03)),
    ("CANKAYA", "ANKARA", 32.75, 33.0, 39.75, 40.0, 112.5, (0.05, 0.04)),
    ("KONYA", "TURKIYE", 32.0, 33.0, 37.5, 38.5, 900.0, (-0.04, 0.02)),
])


def grid_file(endian, subs):
    return overview(endian, len(subs)) + b"".join(subgrid(endian, *s) for s in subs) + END


FILES = {
    "tr.gsb": grid_file(*TR),
    "tr-be.gsb": grid_file(">", TR[1]),
    "ic-ice.gsb": grid_file(*NESTED),
}

ONE = TR[1][0]
BROKEN = [
    # file, bytes, why
    ("kisa.gsb", overview("<", 1)[:100], "notNtv2"),
    ("ntv2-degil.gsb", overview("<", 1, label="NUM_ORE?") + subgrid("<", *ONE), "notNtv2"),
    ("dakika.gsb", overview("<", 1, gs_type="MINUTES") + subgrid("<", *ONE), "notSeconds"),
    ("alt-baslik.gsb", overview("<", 1) + subgrid("<", *ONE, label="SUB_NAMX"), "header"),
    ("alt-yok.gsb", overview("<", 0), "header"),
    ("kapsam.gsb", overview("<", 1) + subgrid("<", "TERS", "NONE", 25.5, 45.5, 42.5, 35.5, -900.0), "extent"),
    ("sayi.gsb", overview("<", 1) + subgrid("<", *ONE, count=29 * 81 - 81), "count"),
    ("kesik.gsb", (overview("<", 1) + subgrid("<", *ONE))[:-160], "truncated"),
    ("sayi-degil.gsb", overview("<", 1) + subgrid("<", *ONE, poison=(3, 4)), "values"),
]


def sha256(b):
    return hashlib.sha256(b).hexdigest()


# ── PROJ ──────────────────────────────────────────────────────────────────────────────────────────────────────────


def shift_pipeline(file, reverse):
    return (f"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step {'+inv ' if reverse else ''}"
            f"+proj=hgridshift +grids={DIR / file} +step +proj=unitconvert +xy_in=rad +xy_out=deg")


def moved(t, p):
    x, y = t.transform(p[0], p[1])
    return [x, y] if math.isfinite(x) and math.isfinite(y) else None


SHIFTS = [
    # name, file, reverse, points (longitude, latitude)
    ("Türkiye ızgarası, ileri", "tr.gsb", False,
     [[32.8597, 39.9334], [43.3729, 38.5012], [26.5623, 41.6818], [30.1, 35.5 - 1e-6], [25.0, 39.0]]),
    ("Türkiye ızgarası, geri", "tr.gsb", True,
     [[32.8606, 39.9342], [45.4999, 40.0], [28.9784, 41.0082]]),
    ("büyük uçlu kopyası, ileri", "tr-be.gsb", False, [[32.8597, 39.9334], [43.3729, 38.5012]]),
    ("büyük uçlu kopyası, geri", "tr-be.gsb", True, [[32.8606, 39.9342]]),
    ("iç içe: torun, çocuk, öbür çocuk, yalnız ata, dışarısı", "ic-ice.gsb", False,
     [[32.85, 39.9], [33.3, 40.3], [32.5, 38.0], [30.0, 37.0], [38.0, 39.0]]),
    ("iç içe, geri: torunun kenarında, çocukta, atada", "ic-ice.gsb", True,
     [[33.0001, 39.9], [33.3, 40.3], [30.0, 37.0]]),
]


def build_shifts():
    out = []
    for name, file, reverse, points in SHIFTS:
        t = Transformer.from_pipeline(shift_pipeline(file, reverse))
        out.append({"name": name, "file": file, "reverse": reverse, "points": points,
                    "expect": [moved(t, p) for p in points]})
    return out


ED50_TM30 = {"kind": "tm", "datum": "ED50", "centralMeridian": 30.0, "scaleFactor": 1.0, "falseEasting": 500000.0,
             "falseNorthing": 0.0}
TUREF_TM30 = dict(ED50_TM30, datum="TUREF")
ED50_TM36 = dict(ED50_TM30, centralMeridian=36.0)
TUREF_TM36 = dict(TUREF_TM30, centralMeridian=36.0)


def tm(s):
    a, rf = {"ED50": (6378388.0, 297.0), "TUREF": (6378137.0, 298.257222101)}[s["datum"]]
    return (f"+proj=tmerc +lat_0=0 +lon_0={s['centralMeridian']!r} +k={s['scaleFactor']!r} "
            f"+x_0={s['falseEasting']!r} +y_0={s['falseNorthing']!r} +a={a!r} +rf={rf!r}")


ED50_TM33 = dict(ED50_TM30, centralMeridian=33.0)
TUREF_TM33 = dict(TUREF_TM30, centralMeridian=33.0)


def on_grid(system, places):
    """Places (longitude, latitude on the system's datum) as the system's grid points, to the millimetre."""
    t = Transformer.from_pipeline(f"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step {tm(system)}")
    return [[round(v, 3) for v in t.transform(lon, lat)] for lon, lat in places]


TRANSFORMS = [
    # name, from, to, file, accuracy, points (the grid's id is the file's; none: a grid not loaded)
    ("ED50 TM30'dan TUREF TM30'a, projenin ızgarasıyla", ED50_TM30, TUREF_TM30, "tr.gsb", 0.3,
     [[412345.678, 4512345.678], [530000.0, 4400000.0]]),
    ("TUREF TM36'dan ED50 TM36'ya, aynı ızgaranın tersi", TUREF_TM36, ED50_TM36, "tr.gsb", 0.3,
     [[486512.34, 4420187.52]]),
    ("doğruluğu yazılmamış ızgara", ED50_TM30, TUREF_TM30, "tr.gsb", None, [[412345.678, 4512345.678]]),
    ("ızgaranın dışında", ED50_TM30, TUREF_TM30, "tr.gsb", 0.3, [[50000.0, 4400000.0]]),
    ("iç içe ızgarayla: torun, çocuk, öbür çocuk, ata", ED50_TM33, TUREF_TM33, "ic-ice.gsb", 0.1,
     on_grid(ED50_TM33, [(32.85, 39.9), (33.3, 40.3), (32.5, 38.0), (34.0, 37.0)])),
    ("iç içe ızgaranın tersi", TUREF_TM33, ED50_TM33, "ic-ice.gsb", 0.1,
     on_grid(TUREF_TM33, [(32.85, 39.9), (33.3, 40.3)])),
    ("ızgarası yüklenmemiş seçim", ED50_TM30, TUREF_TM30, None, 0.3, [[412345.678, 4512345.678]]),
]


def build_transforms():
    out = []
    for name, src, dst, file, accuracy, points in TRANSFORMS:
        grid = {"id": sha256(FILES[file]) if file else "0" * 64}
        if accuracy is not None:
            grid["accuracy"] = accuracy
        choice = {"from": "ED50", "to": "TUREF", "name": "ED50 → TUREF: ızgara", "grid": grid}
        case = {"name": name, "from": src, "to": dst, "choices": [choice], "points": points}
        if file is None:
            case["expect"] = [{"error": "noGrid"} for _ in points]
        else:
            reverse = src["datum"] == "TUREF"
            t = Transformer.from_pipeline(
                f"+proj=pipeline +step +inv {tm(src)} +step {'+inv ' if reverse else ''}+proj=hgridshift "
                f"+grids={DIR / file} +step {tm(dst)}")
            expect = []
            for p in points:
                q = moved(t, p)
                expect.append({"error": "outsideGrid"} if q is None else {
                    "point": q, "accuracy": accuracy, "via": choice["name"], "unofficial": False})
            case["expect"] = expect
        out.append(case)
    return out


def describe(file):
    endian, subs = {"tr.gsb": TR, "tr-be.gsb": (">", TR[1]), "ic-ice.gsb": NESTED}[file]
    tops = [s for s in subs if s[1] == "NONE"]
    extent = [min(s[2] for s in tops), min(s[4] for s in tops), max(s[3] for s in tops), max(s[5] for s in tops)]
    return {"file": file, "id": sha256(FILES[file]), "from": "ED50", "to": "TUREF", "fromAxes": list(INTL),
            "toAxes": list(GRS80), "subgrids": len(subs), "extent": extent}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the files are not what the writer and PROJ give")
    args = ap.parse_args()
    files = dict(FILES)
    files.update({f: b for f, b, _ in BROKEN})
    if not args.check:
        DIR.mkdir(parents=True, exist_ok=True)
        for f, b in files.items():
            (DIR / f).write_bytes(b)
    else:
        for f, b in files.items():
            if not (DIR / f).exists() or (DIR / f).read_bytes() != b:
                print(f"{(DIR / f).relative_to(ROOT)} yazıcının verdiği değil; yeniden yazın: "
                      f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
                return 1
    data = {"format": "kentos.ntv2", "version": 1, "proj": pyproj.proj_version_str,
            "grids": [describe(f) for f in FILES],
            "broken": [{"file": f, "error": why} for f, _, why in BROKEN],
            "shifts": build_shifts(), "transforms": build_transforms()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} ve ızgaralar PROJ'dan ve yazıcıdan çıkanla aynı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(FILES)} ızgara, {len(BROKEN)} bozuk dosya, "
          f"{len(data['shifts'])} kayma, {len(data['transforms'])} dönüşüm.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
