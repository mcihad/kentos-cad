"""EPSG's area of use polygons of the CRS registry's systems (docs/adr/4900), from the EPSG Dataset's own interface. No
KentOS code.

Writes fixtures/crs/v1/area-polygons.json: for each extent of a registry system (fixtures/crs/v1/areas.json) whose
polygon is not just its box, the polygon as the EPSG Dataset gives it (apps.epsg.org's `Extent/<code>/polygon`, GeoJSON,
longitude and latitude in degrees), unchanged: its rings (a multipolygon's parts one after another, read even-odd) and
the systems that use it. Left out, said in the file: the extents whose polygon is its box (the WGS 84 UTM bands, the
world) and the ED50 UTM 35N extent (1638: 25 000 corners over Europe, a megabyte; its box is read).

The EPSG Dataset is owned by IOGP; its Terms of Use (https://epsg.org/terms-of-use.html) let the data be copied and
included in a package with that ownership acknowledged and the terms passed on, unchanged.

    python3 scripts/fixtures/crs_area_polygons.py           # fetches from apps.epsg.org and writes the file
    python3 scripts/fixtures/crs_area_polygons.py --check   # offline: the file's rings closed, its systems the registry's
"""

import json
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AREAS = ROOT / "fixtures/crs/v1/areas.json"
OUT = ROOT / "fixtures/crs/v1/area-polygons.json"
API = "https://apps.epsg.org/api/v1"
VERSION = "v13.106"
READ = "2026-11-10"
# The ED50 UTM 35N extent: 25 000 corners over Europe and the Mediterranean, a megabyte; its box is read.
LEFT_OUT = {1638: "ED50 / UTM 35N'nin alanı: 25 000 köşe, 1 MB; kutusu okunur"}


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "kentos-fixtures"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.loads(r.read())


def rings_of(g):
    parts = g["coordinates"] if g["type"] == "MultiPolygon" else [g["coordinates"]]
    return [r for p in parts for r in p]


def box_only(rings):
    xs = [p[0] for r in rings for p in r]
    ys = [p[1] for r in rings for p in r]
    w, e, s, n = min(xs), max(xs), min(ys), max(ys)
    return all(p[0] in (w, e) or p[1] in (s, n) for r in rings for p in r)


def fetch():
    by_extent, names = {}, {}
    for a in json.loads(AREAS.read_text("utf-8"))["areas"]:
        d = get(f"{API}/CoordRefSystem/{a['srid']}/")
        for u in d["Usage"]:
            code = u["Extent"]["Code"]
            by_extent.setdefault(code, []).append(a["srid"])
            names[code] = u["Extent"]["Name"]
    extents, skipped = [], []
    for code, srids in by_extent.items():
        if code in LEFT_OUT:
            skipped.append({"code": code, "srids": srids, "why": LEFT_OUT[code]})
            continue
        rings = rings_of(get(f"{API}/Extent/{code}/polygon/"))
        if box_only(rings):
            skipped.append({"code": code, "srids": srids, "why": "çokgeni kutusudur"})
            continue
        extents.append({"code": code, "name": names[code], "srids": srids, "rings": rings})
    return extents, skipped


def check(data):
    registry = {a["srid"] for a in json.loads(AREAS.read_text("utf-8"))["areas"]}
    seen = set()
    for x in data["extents"]:
        assert x["rings"], x["code"]
        for r in x["rings"]:
            assert len(r) >= 4 and r[0] == r[-1], (x["code"], "a ring is closed")
            assert all(-180 <= p[0] <= 180 and -90 <= p[1] <= 90 for p in r), x["code"]
        for s in x["srids"]:
            assert s in registry and s not in seen, (x["code"], s)
            seen.add(s)


def main():
    if "--check" in sys.argv:
        data = json.loads(OUT.read_text("utf-8"))
        check(data)
        print(f"area-polygons.json: {len(data['extents'])} alan, {sum(len(x['srids']) for x in data['extents'])} sistem")
        return 0
    extents, skipped = fetch()
    data = {"format": "kentos.crs-area-polygons", "version": 1,
            "source": f"EPSG Dataset {VERSION} (IOGP'nin; kullanım koşulları https://epsg.org/terms-of-use.html), "
                      f"apps.epsg.org'dan {READ}'da değiştirilmeden alındı (scripts/fixtures/crs_area_polygons.py)",
            "extents": extents, "skipped": skipped}
    check(data)
    OUT.write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")) + "\n", "utf-8")
    print(f"area-polygons.json: {len(extents)} alan yazıldı, {len(skipped)} atlandı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
