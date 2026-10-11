#!/usr/bin/env python3
"""Independent reference of Konumlu fotoğraflar's points (docs/adr/0225 §3).

Writes fixtures/photos/v1/import.json from the ADR's rules and PROJ (pyproj), no KentOS code: photos whose EXIF says
where they were taken, as the points the import writes into the project. The photos' EXIF values are those of
fixtures/exif/v1/cases.json (scripts/fixtures/exif_cases.py).

The rules:

1. A photo with a place (latitude and longitude) moves from WGS 84 into the project's system by the way Koordinat
   dönüştür takes (docs/adr/0167; scripts/fixtures/crs_transform_cases.py), in the photos' order. One without a place
   is said and left out: “{name}: {why} Eklenmedi.”, why its EXIF's problem.
2. The point's label and Ad are the file's name without its last extension; its attributes Tarih (the EXIF's moment as
   written), Yön (degrees, two decimals by the display rule, “ M” after it when magnetic), Yükseklik (metres, two
   decimals), Kamera, each only when the EXIF gives it, and Dönüşüm as the GNSS import writes it
   (scripts/fixtures/gnss_import_cases.py). The height is never the point's elevation.
3. Its attachment is the photo: its name the file's, its source the window's (an embedded library image or the linked
   file).
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from crs_transform_cases import transformer  # noqa: E402
from gnss_import_cases import how, registry  # noqa: E402
from numeric_display import shown  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "photos" / "v1" / "import.json"
EXIF = ROOT / "fixtures" / "exif" / "v1" / "cases.json"
NO_PLACE = "{name}: {why} Eklenmedi."


def exif_of(file):
    case = next(c for c in json.loads(EXIF.read_text(encoding="utf-8"))["cases"] if c["file"] == file)
    return {k: v for k, v in case.items() if k != "file"}


def stem(name):
    k = name.rfind(".")
    return name[:k] if k > 0 else name


def place(photos, srid):
    reg = registry()
    src, dst = reg[4326], reg[srid]
    t, _, _ = transformer(src, dst)
    way = f"WGS 84 → {dst['name']}: {how(src, dst)}"
    placed, skipped = [], []
    for i, ph in enumerate(photos, 1):
        e = ph["exif"]
        if "latitude" not in e or "longitude" not in e:
            skipped.append({"line": i, "message": NO_PLACE.format(name=ph["name"], why=e.get("problem", "Fotoğrafta GPS konumu yok."))})
            continue
        x, y = t.transform(e["longitude"], e["latitude"])
        name = stem(ph["name"])
        attrs = {"Ad": name}
        if "taken" in e:
            attrs["Tarih"] = e["taken"]
        if "direction" in e:
            attrs["Yön"] = shown(e["direction"], 2) + (" M" if e.get("magnetic") else "")
        if "altitude" in e:
            attrs["Yükseklik"] = shown(e["altitude"], 2)
        if "camera" in e:
            attrs["Kamera"] = e["camera"]
        attrs["Dönüşüm"] = way
        attachment = {"name": ph["name"], **{k: ph[k] for k in ("asset", "file") if k in ph}}
        placed.append({"index": i, "name": name, "p": [x, y], "attrs": dict(sorted(attrs.items())), "attachment": attachment})
    return {"placed": placed, "skipped": skipped}


PHOTOS = [
    {"name": "IMG_0012.JPG", "exif": exif_of("konum.jpg"), "asset": "resim-1a2b3c4d5e6f7081"},
    {"name": "gpssiz.jpg", "exif": exif_of("gpssiz.jpg"), "asset": "resim-0000000000000002"},
    {"name": "saha.foto.3.jpeg", "exif": exif_of("gps-saati.jpg"), "file": "foto/saha.foto.3.jpeg"},
    {"name": "bozuk.jpg", "exif": exif_of("sifir-payda.jpg"), "file": "foto/bozuk.jpg"},
    # Magnetic north and below sea level (guney-bati.jpg's way), at a place every system here reaches.
    {"name": "manyetik.jpg", "exif": {**exif_of("guney-bati.jpg"), "latitude": 40.7539083, "longitude": 29.38539}, "file": "/veri/manyetik.jpg"},
    {"name": "dakika.jpg", "exif": exif_of("ondalik-dakika.jpg"), "asset": "resim-0000000000000006"},
]


def build():
    cases = []
    for name, srid in (("TUREF / TM30", 5254), ("ED50 / TM30: resmî olmayan yol", 2320), ("WGS 84 / UTM 35N", 32635)):
        cases.append({"name": name, "srid": srid, "photos": PHOTOS, "expect": place(PHOTOS, srid)})
    return {"format": "kentos.photo-import", "version": 1, "source": "scripts/fixtures/photo_import_cases.py (docs/adr/0225 §3; PROJ)",
            "tolerance": {"metres": 1e-6}, "cases": cases}


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
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
