#!/usr/bin/env python3
"""Independent reference of the GNSS import's points and lines (docs/adr/0169 §6, 0222 §6).

Writes fixtures/gnss/v1/import.json from the ADR's rules and PROJ (pyproj), no KentOS code: a GNSS file's positions as
the points the import writes into the project.

The rules:

1. The points of the chosen kinds (wpt, rtept, trkpt of GPX; gga of NMEA), in the file's order, move from WGS 84
   (latitude, longitude) into the project's system by the way Koordinat dönüştür takes (docs/adr/0167: PROJ with the
   ADR's EPSG operations, scripts/fixtures/crs_transform_cases.py). A project without a system imports nothing (its
   window says why).
2. A point's name is the file's when it has one; the others are numbered in order: the prefix, then a number from the
   start, one more for each.
3. Its elevation is its ellipsoidal height when the file gives one (its height plus the geoid's separation), else none:
   a height whose datum the file does not say is never taken for one.
4. Its attributes: Ad; Tür “GNSS noktası”; Kaynak (GPX yol noktası, GPX rota noktası, GPX iz noktası, NMEA GGA); Çözüm;
   Uydu; HDOP (one decimal); Zaman; Enlem and Boylam (decimal degrees, nine decimals, K or G, D or B); Elipsoit
   yüksekliği (m), Yükseklik (dosyada, m), Geoit ayrımı (m) (three decimals); Dönüşüm: “WGS 84 → {system}: {how}”,
   how “kesin, yalnız projeksiyon” within WGS 84, else “±{accuracy} m, {EPSG}”, then “; resmî dönüşüm değil” on ED50's
   way (docs/adr/0167 §5). Numbers are written by the display rule (docs/adr/0149); a value the file does not give
   is no attribute.
5. Lines (docs/adr/0222 §6), when asked for, whatever kinds of points are taken: each route, track or NMEA log of the
   file is one polyline, its parts the file's parts of two points or more (a part of one is left out), each vertex its
   point moved as the points are, its elevation the point's ellipsoidal height when the file gives one. Its label and
   Ad its name when it has one; Tür “GNSS izi” (a track, an NMEA log) or “GNSS rotası”; Kaynak “GPX izi”, “GPX rotası”
   or “NMEA kaydı”; Nokta its vertices' count; Dönüşüm as a point's. A line with no part left, or one of whose points
   the project's system does not reach, is not taken and said: “Satır {line}: {name} çizgisinin iki noktalı parçası
   yok; eklenmedi.”, “Satır {line}: {name} çizgisi projenin sistemine çevrilemedi ({why}); eklenmedi.”, an unnamed
   line named “Adsız iz”, “Adsız rota” or “NMEA kaydı”.
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from crs_transform_cases import PATHS, dd, transformer  # noqa: E402
from numeric_display import shown  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "gnss" / "v1" / "import.json"
REGISTRY = ROOT / "fixtures" / "crs" / "v1" / "registry.json"
SOURCE = {"wpt": "GPX yol noktası", "rtept": "GPX rota noktası", "trkpt": "GPX iz noktası", "gga": "NMEA GGA"}
LINE_KIND = {"trk": ("GNSS izi", "GPX izi", "Adsız iz"), "rte": ("GNSS rotası", "GPX rotası", "Adsız rota"), "nmea": ("GNSS izi", "NMEA kaydı", "NMEA kaydı")}
THIN = "Satır {line}: {name} çizgisinin iki noktalı parçası yok; eklenmedi."

POINTS = [
    {"kind": "wpt", "name": "N1", "lat": 40.7539083, "lon": 29.38539, "height": 105.234, "geoid": 36.123, "ellipsoidal": 141.357,
     "time": "2026-10-04T10:15:00Z", "fix": "3B", "satellites": 14, "hdop": 0.6, "line": 3},
    {"kind": "trkpt", "lat": 40.754, "lon": 29.3854, "height": 104.9, "time": "2026-10-04T10:16:00Z", "line": 12},
    {"kind": "gga", "lat": 40.753909166666666, "lon": 29.385390833333332, "height": 105.24, "geoid": 36.123, "ellipsoidal": 141.363,
     "time": "2026-10-04T10:15:01.00Z", "fix": "RTK kayan", "satellites": 12, "hdop": 0.95, "line": 3},
    {"kind": "rtept", "name": "ANK", "lat": 39.9334, "lon": 32.8597, "line": 9},
    {"kind": "gga", "lat": 36.8969, "lon": 30.7133, "fix": "DGPS", "satellites": 8, "hdop": 1.25, "line": 6},
]


def registry():
    return {e["srid"]: e for e in json.loads(REGISTRY.read_text(encoding="utf-8"))["systems"]}


def number(v):
    """A number as JavaScript and Rust write it in a sentence: no trailing ".0"."""
    return str(int(v)) if v == int(v) else repr(v)


def how(src, dst):
    a, b = src["datum"], dst["datum"]
    if a == b:
        return "kesin, yalnız projeksiyon"
    _, accuracy, via = PATHS[frozenset([a, b])]
    text = f"±{number(accuracy)} m, {via}"
    return text + ("; resmî dönüşüm değil" if "ED50" in (a, b) else "")


def place(points, srid, kinds, prefix, start):
    reg = registry()
    src, dst = reg[4326], reg[srid]
    t, _, _ = transformer(src, dst)
    way = f"WGS 84 → {dst['name']}: {how(src, dst)}"
    n = start
    out = []
    for p in points:
        if p["kind"] not in kinds:
            continue
        x, y = t.transform(p["lon"], p["lat"])
        name = p.get("name")
        if not name:
            name = f"{prefix}{n}"
            n += 1
        attrs = {"Ad": name, "Tür": "GNSS noktası", "Kaynak": SOURCE[p["kind"]]}
        if "fix" in p:
            attrs["Çözüm"] = p["fix"]
        if "satellites" in p:
            attrs["Uydu"] = str(p["satellites"])
        if "hdop" in p:
            attrs["HDOP"] = shown(p["hdop"], 1)
        if "time" in p:
            attrs["Zaman"] = p["time"]
        attrs["Enlem"] = dd(p["lat"], 9, True)
        attrs["Boylam"] = dd(p["lon"], 9, False)
        for key, field in (("Elipsoit yüksekliği (m)", "ellipsoidal"), ("Yükseklik (dosyada, m)", "height"), ("Geoit ayrımı (m)", "geoid")):
            if field in p:
                attrs[key] = shown(p[field], 3)
        attrs["Dönüşüm"] = way
        placed = {"name": name, "p": [x, y]}
        if "ellipsoidal" in p:
            placed["z"] = p["ellipsoidal"]
        placed["attrs"] = attrs
        out.append(placed)
    return out


# The file's lines over POINTS: a track of two parts (the second of one point), a route, a track of one point.
LINES = [
    {"kind": "trk", "name": "Sabah turu", "parts": [[0, 1, 2], [3]], "line": 10},
    {"kind": "rte", "parts": [[3, 4]], "line": 8},
    {"kind": "trk", "parts": [[4]], "line": 14},
]


def place_lines(points, lines, srid):
    reg = registry()
    src, dst = reg[4326], reg[srid]
    t, _, _ = transformer(src, dst)
    way = f"WGS 84 → {dst['name']}: {how(src, dst)}"
    placed, skipped = [], []
    for line in lines:
        kind, source, unnamed = LINE_KIND[line["kind"]]
        name = line.get("name") or unnamed
        parts = [p for p in line["parts"] if len(p) >= 2]
        if not parts:
            skipped.append({"line": line["line"], "message": THIN.format(line=line["line"], name=name)})
            continue
        out = []
        for part in parts:
            run = []
            for i in part:
                p = points[i]
                x, y = t.transform(p["lon"], p["lat"])
                run.append({"p": [x, y], **({"z": p["ellipsoidal"]} if "ellipsoidal" in p else {})})
            out.append(run)
        attrs = {}
        if line.get("name"):
            attrs["Ad"] = line["name"]
        attrs.update({"Tür": kind, "Kaynak": source, "Nokta": str(sum(len(r) for r in out)), "Dönüşüm": way})
        item = {"line": line["line"]}
        if line.get("name"):
            item["name"] = line["name"]
        item["parts"] = out
        item["attrs"] = attrs
        placed.append(item)
    return {"placed": placed, "skipped": skipped}


def cases():
    every = ["wpt", "rtept", "trkpt", "gga"]
    return [
        ("TUREF / TM30: bütün türler, adsızlar G1'den", 5254, every, "G", 1),
        ("ED50 / TM30: yalnız yol noktaları ve GGA, GPS-7'den; resmî olmayan yol", 2320, ["wpt", "gga"], "GPS-", 7),
        ("WGS 84 / UTM 35N: yalnız izdüşüm", 32635, every, "", 100),
    ]


def build():
    out = []
    for name, srid, kinds, prefix, start in cases():
        # The first case takes the lines too (docs/adr/0222 §6); the others only points.
        lines = srid == 5254
        case = {"name": name, "srid": srid, "options": {"kinds": kinds, "prefix": prefix, "start": start, "lines": lines},
                "points": POINTS, "lines": LINES, "expect": place(POINTS, srid, kinds, prefix, start)}
        case["expectLines"] = place_lines(POINTS, LINES, srid) if lines else {"placed": [], "skipped": []}
        out.append(case)
    return {"format": "kentos.gnss-import", "version": 1, "source": "scripts/fixtures/gnss_import_cases.py (docs/adr/0169 §6, 0222 §6; PROJ)",
            "tolerance": {"metres": 1e-6}, "cases": out}


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = parser.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
