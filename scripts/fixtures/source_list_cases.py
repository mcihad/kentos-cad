#!/usr/bin/env python3
"""Kaynaklar's folder listing (docs/adr/0199 §7): the shared cases of what a folder of the panel shows, written from the
ADR without KentOS code. Given a folder's entries (names, and whether each is a folder), the panel shows:

- its folders, but those whose name begins with a dot, in the natural order (`text::natural`, as Noktalar sorts names);
- its files of the kinds the panel adds as layers, but those whose name begins with a dot, in the natural order; the
  kind by the extension, case aside: GeoJSON (.geojson, .json), Shapefile (.shp), DXF (.dxf), Netcad NCZ (.ncz), GNSS
  (.gpx, .nmea, .nma), coordinate list (.ncn, .txt, .csv, .xyz, .dat, .asc);
- a Shapefile as its .shp, with the files of the same name (exactly) and the extensions .shx, .dbf, .prj and .cpg (case
  aside) as its parts, in that order; those files are not shown on their own, nor without their .shp.

The natural order is the one scripts/fixtures/point_editor_cases.py checks the core's against.

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
}
LABELS = {
    "geojson": "GeoJSON",
    "shapefile": "Shapefile",
    "dxf": "DXF",
    "ncz": "Netcad NCZ",
    "gnss": "GNSS (GPX, NMEA)",
    "coords": "Koordinat listesi",
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
              "parsel.shp", "parsel.shx", "parsel.dbf", "parsel.prj", "kayit.log"])),
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
]


def build():
    return {
        "format": "kentos.source-list-cases",
        "version": 1,
        "source": SOURCE,
        "labels": LABELS,
        "cases": [{"name": n, "entries": e, "expect": listing(e)} for n, e in CASES],
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
