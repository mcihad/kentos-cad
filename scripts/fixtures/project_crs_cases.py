#!/usr/bin/env python3
"""Independent reference of the project's coordinate systems as the transforms read them (docs/adr/0168 §1–§3).

Writes fixtures/geodesy/v1/project.json from the ADR's rules and the registry (fixtures/crs/v1/registry.json), no KentOS
code: a project's settings (`srid`, `customCrs`, `secondSrid`, `secondCustomCrs`, `datumTransforms`, as the contract
writes them) give its own system and its second, each named, and its datum choices, in the core's terms (the systems
of crates/shared/geometry-core/src/crs.rs, the choices of crs/datum.rs).

- The registry's system: its name, the title “TUREF / TM30 (EPSG:5254)”, the code “EPSG:5254”; the core's system of
  the entry (crs_transform_cases.py's rule). SRID 0 without a definition is no system.
- A definition: its name, the title “Şantiye (özel sistem)”, the code “Özel sistem”. A TM or geographic definition's
  datum is the registry's by name (`datum`) or the project's (`customDatum`: its ellipsoid and its way to WGS 84 as
  written). A local system's base is a projected system of the registry by its code, or a TM definition; any other
  base (the registry has no such code, or it is not projected) leaves the definition without a system the transforms
  read, its name kept.
- A datum choice: from, to, name, and its seven parameters, or its grid by SHA-256 with the accuracy given (the
  file's name and size are the device's library's).

The desktop (crates/native/project/src/crs.rs) and the web (apps/web/src/model/projectCrs.ts) must give exactly these.
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "project.json"
REGISTRY = {s["srid"]: s for s in json.loads((ROOT / "fixtures" / "crs" / "v1" / "registry.json").read_text())["systems"]}

BASE = {"lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000}


def registry_system(entry):
    """The core's system of a registry entry; none for the local one."""
    if entry["kind"] == "local":
        return None
    if entry["kind"] == "geographic":
        return {"kind": "geographic", "datum": entry["datum"]}
    if entry.get("projection") == "Pseudo-Mercator":
        return {"kind": "mercator"}
    return {"kind": "tm", "datum": entry["datum"], "centralMeridian": entry["centralMeridian"],
            "scaleFactor": entry["scaleFactor"], "falseEasting": entry["falseEasting"],
            "falseNorthing": entry["falseNorthing"]}


def datum(d):
    """A definition's datum: the registry's name, or the project's own as written."""
    return d["datum"] if "datum" in d else d["customDatum"]


def definition_system(definition):
    """The core's system of a definition; none when its base is not a projected system the registry has."""
    s = definition["system"]
    if s["kind"] == "tm":
        out = {"kind": "tm", "datum": datum(s)}
        if "latitudeOfOrigin" in s:
            out["latitudeOfOrigin"] = s["latitudeOfOrigin"]
        for k in ("centralMeridian", "scaleFactor", "falseEasting", "falseNorthing"):
            out[k] = s[k]
        return out
    if s["kind"] == "geographic":
        return {"kind": "geographic", "datum": datum(s)}
    base = s["base"]
    if "srid" in base:
        entry = REGISTRY.get(base["srid"])
        under = registry_system(entry) if entry and entry["kind"] == "projected" else None
    else:
        under = definition_system(base["definition"])
    return None if under is None else {"kind": "local", "base": under, "plane": s["plane"]}


def registry_named(srid):
    entry = REGISTRY.get(srid)
    if entry is None:
        return None
    return {"name": entry["name"], "title": f"{entry['name']} (EPSG:{srid})", "code": f"EPSG:{srid}",
            "system": registry_system(entry)}


def custom_named(definition):
    return {"name": definition["name"], "title": f"{definition['name']} (özel sistem)", "code": "Özel sistem",
            "system": definition_system(definition)}


def own(settings):
    if settings["srid"] != 0:
        return registry_named(settings["srid"])
    return custom_named(settings["customCrs"]) if "customCrs" in settings else None


def second(settings):
    if own(settings) is None:
        return None
    if "secondSrid" in settings:
        return registry_named(settings["secondSrid"])
    return custom_named(settings["secondCustomCrs"]) if "secondCustomCrs" in settings else None


def choice(t):
    out = {"from": t["from"], "to": t["to"], "name": t["name"]}
    if "helmert" in t:
        out["helmert"] = t["helmert"]
    else:
        g = {"id": t["grid"]["id"]}
        if "accuracy" in t["grid"]:
            g["accuracy"] = t["grid"]["accuracy"]
        out["grid"] = g
    return out


# ── The cases: settings as a project keeps them (sanitized) ───────────────────────────────────────────────────────────

SIMILARITY = {"kind": "similarity", "east": 492345.678, "north": 4422345.678, "rotation": 12.5, "scale": 1.000012}
AFFINE = {"kind": "affine", "a": 1.00002, "b": -0.0003, "c": 1250.0, "d": 0.00025, "e": 0.99998, "f": -830.5}
BESSEL = {"name": "Bessel datumu",
          "ellipsoid": {"name": "Bessel 1841", "semiMajor": 6377397.155, "inverseFlattening": 299.1528128},
          "toWgs84": {"translation": [598.1, 73.7, 418.2], "rotation": [0.202, 0.045, -2.455], "scale": 6.7,
                      "convention": "positionVector", "accuracy": 1.5}}
KRASOVSKI = {"name": "Krasovski datumu",
             "ellipsoid": {"name": "Krasovski 1940", "semiMajor": 6378245.0, "inverseFlattening": 298.3},
             "toWgs84": {"translation": [23.92, -141.27, -80.9], "rotation": [0.0, -0.35, -0.82], "scale": -0.12,
                         "convention": "coordinateFrame"}}
CLARKE = {"name": "Bağsız datum",
          "ellipsoid": {"name": "Clarke 1880 (RGS)", "semiMajor": 6378249.145, "inverseFlattening": 293.465}}
BESSEL_TM = {"name": "Bessel TM", "system": {"kind": "tm", "customDatum": BESSEL, "latitudeOfOrigin": 36.0,
                                             "centralMeridian": 34.5, "scaleFactor": 0.9999,
                                             "falseEasting": 200000.0, "falseNorthing": 100000.0}}
REGION = {"from": "ED50", "to": "TUREF", "name": "ED50 → TUREF: Bölge 7",
          "helmert": {"translation": [-158.785, -109.965, -50.768], "rotation": [1.4275, -3.0873, 0.5505],
                      "scale": -5.1814, "convention": "coordinateFrame", "accuracy": 0.3}}
GRID = {"from": "ED50", "to": "WGS84", "name": "ED50 → WGS 84: ızgara",
        "grid": {"id": "b328bc7d133b48ffdacab572e7ccad7510e49fd7a78eb1194f81912711659b4a", "file": "tr.gsb",
                 "size": 37952, "accuracy": 0.5}}
GRID_UNSURE = {"from": "TUREF", "to": "WGS84", "name": "TUREF → WGS 84: ızgara",
               "grid": {"id": "1f" * 32, "file": "turef.gsb", "size": 1024}}


def settings(**fields):
    return {"srid": fields.pop("srid"), **BASE, **fields}


CASES = [
    ("kayıttaki proje, kayıttaki ikinci sistem", settings(srid=5254, secondSrid=2320)),
    ("koordinat sistemi olmayan proje", settings(srid=0)),
    ("yerel sistem (benzerlik, TM33 tabanlı), ikinci sistem tabanı, iki datum seçimi",
     settings(srid=0, customCrs={"name": "Şantiye", "system": {"kind": "local", "base": {"srid": 5255},
                                                                "plane": SIMILARITY}},
              secondSrid=5255, datumTransforms=[REGION, GRID])),
    ("kayıttaki projenin ikinci sistemi yerel (afin), tabanı Bessel datumlu özel TM",
     settings(srid=5254, secondCustomCrs={"name": "Belediye sistemi",
                                          "system": {"kind": "local", "base": {"definition": BESSEL_TM},
                                                     "plane": AFFINE}})),
    ("bağsız datumlu coğrafi proje, ikinci sistem ED50'de özel TM",
     settings(srid=0, customCrs={"name": "Eski coğrafi", "system": {"kind": "geographic", "customDatum": CLARKE}},
              secondCustomCrs={"name": "ED50 TM33 k=0.9999",
                               "system": {"kind": "tm", "datum": "ED50", "centralMeridian": 33.0,
                                          "scaleFactor": 0.9999, "falseEasting": 500000.0,
                                          "falseNorthing": 0.0}})),
    ("Krasovski datumlu özel TM (koordinat çerçevesi kuralı), ikinci sistem WGS 84, doğruluğu yazılmamış ızgara",
     settings(srid=0, customCrs={"name": "Krasovski TM", "system": {"kind": "tm", "customDatum": KRASOVSKI,
                                                                     "centralMeridian": 39.0, "scaleFactor": 1.0,
                                                                     "falseEasting": 500000.0,
                                                                     "falseNorthing": 0.0}},
              secondSrid=4326, datumTransforms=[GRID_UNSURE])),
    ("kayıttaki coğrafi proje, ikinci sistem Pseudo-Mercator tabanlı yerel sistem",
     settings(srid=4326, secondCustomCrs={"name": "Web yerel", "system": {"kind": "local", "base": {"srid": 3857},
                                                                          "plane": SIMILARITY}})),
    ("tabanı kayıtta olmayan yerel sistem: adı var, sistemi yok",
     settings(srid=0, customCrs={"name": "Bilinmeyen taban", "system": {"kind": "local", "base": {"srid": 99999},
                                                                         "plane": SIMILARITY}},
              secondSrid=5254)),
    ("tabanı coğrafi olan yerel sistem: adı var, sistemi yok",
     settings(srid=5254, secondCustomCrs={"name": "Coğrafi taban", "system": {"kind": "local",
                                                                             "base": {"srid": 4326},
                                                                             "plane": AFFINE}})),
]


def build():
    return [{"name": name, "settings": s, "own": own(s), "second": second(s),
             "choices": [choice(t) for t in s.get("datumTransforms", [])]} for name, s in CASES]


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what the rules give")
    args = ap.parse_args()
    data = {"format": "kentos.project-crs", "version": 1, "cases": build()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(data['cases'])} proje.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
