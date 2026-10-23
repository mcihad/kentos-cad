#!/usr/bin/env python3
"""Kaynaklar's folder listing (docs/adr/0199 §7): the shared cases of what a folder of the panel shows, written from the
ADR without KentOS code. Given a folder's entries (names, and whether each is a folder), the panel shows:

- its folders, but those whose name begins with a dot, in the natural order (`text::natural`, as Noktalar sorts names);
- its files of the kinds the panel adds as layers, but those whose name begins with a dot, in the natural order; the
  kind by the extension, case aside: GeoJSON (.geojson, .json), Shapefile (.shp), DXF (.dxf), Netcad NCZ (.ncz), GNSS
  (.gpx, .nmea, .nma), coordinate list (.ncn, .txt, .csv, .xyz, .dat, .asc), KML (.kml, .kmz; docs/adr/0222);
  GeoPackage (.gpkg), FlatGeobuf (.fgb), GML (.gml), raster (.tif, .tiff, .vrt) and point cloud (.las, .laz) since
  docs/adr/0599 §1 (a GeoPackage's journal files, .gpkg-wal and .gpkg-shm, are not shown);
- a Shapefile as its .shp, with the files of the same name (exactly) and the extensions .shx, .dbf, .prj and .cpg (case
  aside) as its parts, in that order; those files are not shown on their own, nor without their .shp.

The natural order is the one scripts/fixtures/point_editor_cases.py checks the core's against.

The panel's search (docs/adr/0599 §5): the query's words (split at white space) must each be, the Turkish way caseless
(I is ı's, İ is i's, any other letter its lower case when that is one letter; then ı is i, as the panel's words are
also English ones: MULTIPOLYGON, EPSG), somewhere in one of the row's fields; an
empty query takes every row. In the tree (rows in their order with their depths) a row shows when it matches or when a
row under it does (its ancestors show with it); a row's children show only when they match themselves.

The recent and favourite sources (docs/adr/0599 §6): a source used goes to the front of the recent ones, once (an
earlier use of it leaves its place), at most 12 kept; a source starred joins the end of the favourites, starred again it
leaves them, at most 50 (a 51st is refused and the list stays).

    python3 scripts/fixtures/source_list_cases.py          # write
    python3 scripts/fixtures/source_list_cases.py --check  # compare
"""

import json
import sys
from functools import cmp_to_key
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from point_editor_cases import natural_cmp  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "sources" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/source_list_cases.py (docs/adr/0199 §7)"

KINDS = {
    "geojson": "geojson", "json": "geojson",
    "shp": "shapefile",
    "dxf": "dxf",
    "ncz": "ncz",
    "gpx": "gnss", "nmea": "gnss", "nma": "gnss",
    "ncn": "coords", "txt": "coords", "csv": "coords", "xyz": "coords", "dat": "coords", "asc": "coords",
    "kml": "kml", "kmz": "kml",
    "gpkg": "geopackage",
    "fgb": "flatgeobuf",
    "gml": "gml",
    "tif": "raster", "tiff": "raster", "vrt": "raster",
    "las": "pointcloud", "laz": "pointcloud",
}
LABELS = {
    "geojson": "GeoJSON",
    "shapefile": "Shapefile",
    "dxf": "DXF",
    "ncz": "Netcad NCZ",
    "gnss": "GNSS (GPX, NMEA)",
    "coords": "Koordinat listesi",
    "kml": "KML/KMZ",
    "geopackage": "GeoPackage",
    "flatgeobuf": "FlatGeobuf",
    "gml": "GML",
    "raster": "Raster (GeoTIFF, VRT)",
    "pointcloud": "Nokta bulutu (LAS, LAZ)",
}
PARTS = ["shx", "dbf", "prj", "cpg"]


def split(name):
    """The stem and the extension (case aside); no extension without a dot past the first letter."""
    at = name.rfind(".")
    if at <= 0:
        return name, ""
    return name[:at], name[at + 1:].lower()


def natural(names):
    return sorted(names, key=cmp_to_key(natural_cmp))


def listing(entries):
    shown = [e for e in entries if not e["name"].startswith(".")]
    folders = natural([e["name"] for e in shown if e["dir"]])
    files = [e["name"] for e in shown if not e["dir"]]
    out = []
    for name in natural(files):
        stem, ext = split(name)
        kind = KINDS.get(ext)
        if kind is None:
            continue
        parts = [name]
        if kind == "shapefile":
            for p in PARTS:
                parts += [f for f in files if split(f)[0] == stem and split(f)[1] == p][:1]
        out.append({"name": name, "kind": kind, "parts": parts})
    return {"folders": folders, "files": out}


def entries(dirs, files):
    return [{"name": n, "dir": True} for n in dirs] + [{"name": n, "dir": False} for n in files]


CASES = [
    ("desteklenen dosyalar ve klasörler doğal sırayla; gizliler ve desteklenmeyenler yok",
     entries(["Pafta 10", "Pafta 2", ".git", "arşiv"],
             ["yol.dxf", "imar.NCZ", "rota.gpx", "alim.nmea", "eski.nma", "noktalar.ncn", "liste.csv", "not.txt",
              "olcu.xyz", "veri.dat", "kot.asc", "veri.geojson", "ayarlar.json", "rapor.pdf", ".gizli.dxf",
              "parsel.shp", "parsel.shx", "parsel.dbf", "parsel.prj", "kayit.log", "gezi.kml", "Alanlar.KMZ"])),
    ("Shapefile'ın parçaları: aynı ad, uzantının büyük küçük harfi önemsiz; parçası olmayan .dbf gösterilmez",
     entries([], ["Bina.SHP", "Bina.shx", "Bina.DBF", "Bina.prj", "Bina.cpg", "bina.dbf", "binalar.dbf", "Bina.sbn"])),
    ("parçasız Shapefile ve sahipsiz parçalar",
     entries([], ["yalniz.shp", "yetim.shx", "yetim.dbf", "yetim.prj", "yetim.cpg"])),
    ("büyük küçük harfle iki ayrı Shapefile, her biri kendi parçalarıyla",
     entries([], ["a.shp", "A.shp", "a.dbf", "A.dbf", "A.shx"])),
    ("uzantı büyük küçük harfsiz, birden çok nokta, noktasız ad",
     entries([], ["X.GeoJSON", "Y.Json", "pafta.2026.dxf", "dxf", "README", "nokta.", "Tablo.CSV"])),
    ("sayılar değerleriyle, Türkçe harfler",
     entries(["P10", "P2", "Çamlık", "cadde", "İmar", "ılgaz"], ["P10.dxf", "P2.dxf", "P1.dxf", "Çizim.dxf", "cizim.dxf"])),
    ("boş klasör", []),
    ("yalnız desteklenmeyen dosyalar", entries([], ["a.pdf", "b.docx", "c.zip", "d.kcad"])),
    ("GeoPackage, FlatGeobuf, GML, raster ve nokta bulutu (ADR 0599); GeoPackage'ın günlük dosyaları gösterilmez",
     entries(["ortofoto"], ["kadastro.gpkg", "kadastro.gpkg-wal", "kadastro.gpkg-shm", "Yapi.GPKG", "bina.fgb",
                            "tapu.gml", "tapu.xsd", "dem.tif", "Orto.TIFF", "mozaik.vrt", "koy.las", "koy.laz",
                            "koy.copc.laz", "pafta.jpg", "pafta.jgw"])),
]


RECENT_MOST = 12
FAVOURITES_MOST = 50


def fold(text):
    out = []
    for c in text:
        if c == "I":
            out.append("ı")
        elif c == "İ":
            out.append("i")
        else:
            low = c.lower()
            out.append(low if len(low) == 1 else c)
    return "".join(out).replace("ı", "i")


def matches(query, fields):
    words = [fold(w) for w in query.split()]
    folded = [fold(f) for f in fields]
    return all(any(w in f for f in folded) for w in words)


def shown(query, rows):
    """The places of the rows the tree shows."""
    keep = [matches(query, r["fields"]) for r in rows]
    # A row under a kept row keeps its ancestors: walk back up its depths.
    for i, r in enumerate(rows):
        if not keep[i] or not matches(query, r["fields"]):
            continue
        depth = r["depth"]
        for j in range(i - 1, -1, -1):
            if rows[j]["depth"] < depth:
                keep[j] = True
                depth = rows[j]["depth"]
                if depth == 0:
                    break
    return [i for i, k in enumerate(keep) if k]


def tree(*rows):
    return [{"depth": d, "fields": list(f)} for d, *f in rows]


SEARCH_ROWS = tree(
    (0, "Klasörler"),
    (1, "İmar", "/veri/İmar"),
    (2, "kadastro.gpkg", "GeoPackage", "/veri/İmar/kadastro.gpkg"),
    (3, "parsel", "MULTIPOLYGON", "EPSG:5254"),
    (3, "IŞIK_DİREĞİ", "POINT", "EPSG:5254"),
    (3, "malik", "tablo"),
    (2, "Orto 2024.tif", "Raster (GeoTIFF, VRT)", "/veri/İmar/Orto 2024.tif"),
    (1, "Çamlık", "/veri/Çamlık"),
    (2, "yol.dxf", "DXF", "/veri/Çamlık/yol.dxf"),
    (0, "Veritabanları"),
    (1, "postgresql://cbs@sunucu/belediye"),
    (2, "public.parsel", "MULTIPOLYGON", "EPSG:5256", "public"),
    (2, "imar.plan", "POLYGON", "EPSG:5256", "imar"),
    (0, "Servisler"),
    (1, "Hazır altlıklar"),
    (2, "OpenStreetMap", "XYZ / TMS"),
    (1, "Kayıtlı servisler"),
    (2, "TKGM Parsel", "WMS"),
    (3, "parseller", "WMS"),
)
SEARCHES = [
    ("boş arama her satırı gösterir", ""),
    ("ad: parsel, iki tabloda", "parsel"),
    ("Türkçe büyük küçük harf: ışık ve direği", "ışık direği"),
    ("İ'nin küçüğü i: imar", "imar"),
    ("sistem: EPSG kodu", "epsg:5256"),
    ("tür ve sistem birlikte", "multipolygon 5254"),
    ("şema", "public"),
    ("tür: raster", "raster"),
    ("dosyanın yolu klasörün adını taşır", "çamlık"),
    ("eşleşmeyen sözcük", "parsel orto"),
    ("servisin katmanı ve ataları", "parseller"),
    ("noktalı ve noktasız i bir: Imar ve isik", "Imar"),
    ("noktasız ı ile yazılan İngilizce tür", "multıpolygon"),
    ("ş ve ğ yerine s ve g eşleşmez", "isik diregi"),
]

RECENT = [
    ("ilk kullanım öne", [], "a"),
    ("yeni öne geçer", ["a", "b"], "c"),
    ("yeniden kullanılan öne geçer, bir kez", ["a", "b", "c"], "b"),
    ("en çok 12: en eskisi düşer", [str(i) for i in range(12)], "yeni"),
    ("12 iken var olanı kullanmak düşürmez", [str(i) for i in range(12)], "5"),
]


def recent_with(recent, used):
    return ([used] + [r for r in recent if r != used])[:RECENT_MOST]


def starred(favourites, source):
    """The favourites after starring `source`, and whether it was refused."""
    if source in favourites:
        return [f for f in favourites if f != source], False
    if len(favourites) >= FAVOURITES_MOST:
        return list(favourites), True
    return favourites + [source], False


FAVOURITES = [
    ("ilk yıldız", [], "a"),
    ("sona eklenir", ["a", "b"], "c"),
    ("yeniden yıldız çıkarır", ["a", "b", "c"], "b"),
    ("en çok 50: ellibirinci reddedilir", [str(i) for i in range(50)], "yeni"),
    ("50 iken çıkarmak olur", [str(i) for i in range(50)], "7"),
]


def build():
    return {
        "format": "kentos.source-list-cases",
        "version": 1,
        "source": SOURCE,
        "labels": LABELS,
        "cases": [{"name": n, "entries": e, "expect": listing(e)} for n, e in CASES],
        "search": {
            "rows": SEARCH_ROWS,
            "cases": [{"name": n, "query": q, "shown": shown(q, SEARCH_ROWS)} for n, q in SEARCHES],
        },
        "recent": {
            "most": RECENT_MOST,
            "cases": [{"name": n, "recent": r, "used": u, "expect": recent_with(r, u)} for n, r, u in RECENT],
        },
        "favourites": {
            "most": FAVOURITES_MOST,
            "cases": [{"name": n, "favourites": f, "starred": x, "expect": starred(f, x)[0], "refused": starred(f, x)[1]}
                      for n, f, x in FAVOURITES],
        },
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"kaynak listesi durumları tutarlı: {OUT.relative_to(ROOT)}")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
