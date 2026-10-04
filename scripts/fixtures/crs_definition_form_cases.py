#!/usr/bin/env python3
"""Independent reference of the Özel koordinat sistemi window's rules (docs/adr/0168 §1–§2, §6; step 4b).

Writes fixtures/crs/v1/definition-form.json from the ADR's rules and the registry (fixtures/crs/v1/registry.json), no
KentOS code: what is typed in the window turns into the project's definition (`CrsDefinition`, as the contract writes
it) or says, field by field, what is wrong; a definition the registry has already is said (it does not stop Tamam).

- The name is required (trimmed).
- A transverse Mercator: the central meridian (−180…180°) and the false easting and northing are required numbers;
  the origin's latitude (−90…90°) is 0 when empty and written only when not 0; the scale is 1 when empty, and above 0.
- A datum is the registry's (TUREF, ED50, WGS 84) or the project's: its name (required), its ellipsoid (one of the
  classic ones by name, or its semi-major axis above 0 and inverse flattening above 1), and, when it is bound to WGS
  84, its seven parameters (the translations required, an empty rotation or scale difference 0), their convention and
  their accuracy (empty: unknown; else 0 or more).
- A local system: its base is a projected system of the registry by its code; its plane is a similarity (east and north
  required, the rotation 0 when empty, the scale 1 when empty and above 0) or an affine (all six required, a·e − b·d not
  0).
- Numbers are read as the Hesap windows read them: trimmed, the first comma a point.
- The same system as one of the registry's (a TM on its datum with its values, or the geographic system of its datum):
  “EPSG:5254 (TUREF / TM30) ile aynı; kayıttakini seçin.”

The desktop (kentos_project::definition_form) and the web (model/definitionForm.ts) must give exactly these.
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "crs" / "v1" / "definition-form.json"
REGISTRY = json.loads((ROOT / "fixtures" / "crs" / "v1" / "registry.json").read_text(encoding="utf-8"))["systems"]

NUMBER = re.compile(r"^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$", re.IGNORECASE)

# The classic ellipsoids by name: semi-major axis and inverse flattening (crs::text's table).
ELLIPSOIDS = {
    "GRS 1980": (6378137.0, 298.257222101),
    "WGS 84": (6378137.0, 298.257223563),
    "International 1924": (6378388.0, 297.0),
    "Bessel 1841": (6377397.155, 299.1528128),
    "Krasovski 1940": (6378245.0, 298.3),
    "Clarke 1880 (RGS)": (6378249.145, 293.465),
}

TEXTS = {
    "name": "Adını yazın: sistem bu adla görünür.",
    "number": "Sayı yazın.",
    "meridian": "−180 ile 180 arasında bir derece yazın.",
    "latitude": "−90 ile 90 arasında bir derece yazın.",
    "positive": "0'dan büyük bir sayı yazın.",
    "datumName": "Datumun adını yazın.",
    "semiMajor": "Büyük yarı ekseni metre olarak, 0'dan büyük yazın.",
    "inverseFlattening": "Ters basıklığı 1'den büyük yazın.",
    "accuracy": "0 ya da büyük bir sayı yazın; bilinmiyorsa boş bırakın.",
    "base": "Kayıttaki projeksiyonlu bir sistem seçin.",
    "folds": "Bu katsayılar düzlemi katlıyor (a·e − b·d = 0).",
}

PARAMETERS = ("tx", "ty", "tz", "rx", "ry", "rz", "ds")


def number(text):
    t = text.strip().replace(",", ".", 1)
    return float(t) if t and NUMBER.match(t) else None


class Reader:
    """Reads the form's fields, keeping the problems by field."""

    def __init__(self, form):
        self.form = form
        self.problems = {}

    def text(self, key):
        return self.form[key]

    def number(self, key, default=None, check=None, problem="number"):
        text = self.form[key].strip()
        if not text and default is not None:
            return default
        v = number(text)
        if v is None:
            self.problems[key] = TEXTS["number"]
            return None
        if check and not check(v):
            self.problems[key] = TEXTS[problem]
            return None
        return v


def datum_of(r):
    f = r.form
    if f["datum"] != "custom":
        return {"datum": f["datum"]}
    name = f["datumName"].strip()
    if not name:
        r.problems["datumName"] = TEXTS["datumName"]
    if f["ellipsoid"] in ELLIPSOIDS:
        a, rf = ELLIPSOIDS[f["ellipsoid"]]
        ellipsoid = {"name": f["ellipsoid"], "semiMajor": a, "inverseFlattening": rf}
    else:
        a = r.number("semiMajor", check=lambda v: v > 0, problem="semiMajor")
        rf = r.number("inverseFlattening", check=lambda v: v > 1, problem="inverseFlattening")
        ellipsoid = {"name": f"a={f['semiMajor'].strip()}, 1/f={f['inverseFlattening'].strip()}", "semiMajor": a,
                     "inverseFlattening": rf}
    d = {"name": name, "ellipsoid": ellipsoid}
    if f["linked"]:
        values = []
        for i, key in enumerate(PARAMETERS):
            text = f["parameters"][key].strip()
            if not text and i >= 3:
                values.append(0.0)
                continue
            v = number(text)
            if v is None:
                r.problems[key] = TEXTS["number"]
            values.append(v)
        h = {"translation": values[0:3], "rotation": values[3:6], "scale": values[6], "convention": f["convention"]}
        if f["accuracy"].strip():
            acc = number(f["accuracy"])
            if acc is None or acc < 0:
                r.problems["accuracy"] = TEXTS["accuracy"]
            else:
                h["accuracy"] = acc
        d["toWgs84"] = h
    return {"customDatum": d}


def tm_of(r):
    out = {"kind": "tm"}
    out.update(datum_of(r))
    lat0 = r.number("latitudeOfOrigin", default=0.0, check=lambda v: -90 <= v <= 90, problem="latitude")
    if lat0:
        out["latitudeOfOrigin"] = lat0
    out["centralMeridian"] = r.number("centralMeridian", check=lambda v: -180 <= v <= 180, problem="meridian")
    out["scaleFactor"] = r.number("scaleFactor", default=1.0, check=lambda v: v > 0, problem="positive")
    out["falseEasting"] = r.number("falseEasting")
    out["falseNorthing"] = r.number("falseNorthing")
    return out


def local_of(r):
    f = r.form
    entry = next((s for s in REGISTRY if str(s["srid"]) == f["base"].strip() and s["kind"] == "projected"), None)
    if entry is None:
        r.problems["base"] = TEXTS["base"]
    if f["plane"] == "similarity":
        plane = {"kind": "similarity", "east": r.number("east"), "north": r.number("north"),
                 "rotation": r.number("rotation", default=0.0),
                 "scale": r.number("scale", default=1.0, check=lambda v: v > 0, problem="positive")}
    else:
        plane = {"kind": "affine", **{k: r.number(k) for k in "abcdef"}}
        if all(plane[k] is not None for k in "abde") and plane["a"] * plane["e"] - plane["b"] * plane["d"] == 0:
            r.problems["a"] = TEXTS["folds"]
    return {"kind": "local", "base": {"srid": entry["srid"] if entry else 0}, "plane": plane}


def registry_system(entry):
    if entry["kind"] == "geographic":
        return {"kind": "geographic", "datum": entry["datum"]}
    if entry.get("projection") == "Pseudo-Mercator":
        return None
    return {"kind": "tm", "datum": entry["datum"], "centralMeridian": float(entry["centralMeridian"]),
            "scaleFactor": float(entry["scaleFactor"]), "falseEasting": float(entry["falseEasting"]),
            "falseNorthing": float(entry["falseNorthing"])}


def same_as_registry(system):
    """The registry's system this definition is: its datum by name, every value the same."""
    if system["kind"] == "local" or "datum" not in system:
        return None
    for entry in REGISTRY:
        if entry["kind"] == "local":
            continue
        if registry_system(entry) == system:
            return f"EPSG:{entry['srid']} ({entry['name']}) ile aynı; kayıttakini seçin."
    return None


def build(form):
    r = Reader(form)
    name = form["name"].strip()
    if not name:
        r.problems["name"] = TEXTS["name"]
    if form["kind"] == "tm":
        system = tm_of(r)
    elif form["kind"] == "geographic":
        system = {"kind": "geographic"}
        system.update(datum_of(r))
    else:
        system = local_of(r)
    if r.problems:
        return {"problems": r.problems}
    out = {"definition": {"name": name, "system": system}}
    note = same_as_registry(system)
    if note:
        out["note"] = note
    return out


EMPTY = {k: "" for k in PARAMETERS}


def form(kind, name="Şantiye", **fields):
    """A form as both platforms hold it: every field, as typed."""
    f = {"name": name, "kind": kind, "latitudeOfOrigin": "", "centralMeridian": "", "scaleFactor": "",
         "falseEasting": "", "falseNorthing": "", "datum": "TUREF", "datumName": "", "ellipsoid": "GRS 1980",
         "semiMajor": "", "inverseFlattening": "", "linked": True, "parameters": dict(EMPTY),
         "convention": "positionVector", "accuracy": "", "base": "", "plane": "similarity", "east": "", "north": "",
         "rotation": "", "scale": "", "a": "", "b": "", "c": "", "d": "", "e": "", "f": ""}
    parameters = fields.pop("parameters", None)
    f.update(fields)
    if parameters:
        f["parameters"] = dict(EMPTY, **parameters)
    return f


BESSEL = {"tx": "598.1", "ty": "73,7", "tz": "418.2", "rx": "0.202", "ry": "0.045", "rz": "-2.455", "ds": "6.7"}

CASES = [
    ("başlangıcı ve ölçeği farklı TM, TUREF'te",
     form("tm", "Kent TM", latitudeOfOrigin="36", centralMeridian="34,5", scaleFactor="0.9999",
          falseEasting="200000", falseNorthing="100000")),
    ("kayıttaki TM30 ile aynı: söylenir", form("tm", "TM30 kopyası", centralMeridian="30", falseEasting="500000",
                                               falseNorthing="0")),
    ("Bessel datumlu TM, iki kuraldan konum vektörü, doğruluklu",
     form("tm", "Bessel TM", centralMeridian="33", falseEasting="500000", falseNorthing="0", datum="custom",
          datumName="Bessel datumu", ellipsoid="Bessel 1841", parameters=BESSEL, accuracy="1.5")),
    ("elipsoidi yazılan, WGS 84'e bağı olmayan coğrafi sistem",
     form("geographic", "Eski coğrafi", datum="custom", datumName="Bağsız datum", ellipsoid="custom",
          semiMajor="6378249.145", inverseFlattening="293.465", linked=False)),
    ("TUREF coğrafi: kayıttaki ile aynı", form("geographic", "TUREF coğrafi", datum="TUREF")),
    ("ED50 coğrafi: kayıtta yok, söylenmez", form("geographic", "ED50 coğrafi", datum="ED50")),
    ("benzerlikle TM33'e bağlı yerel sistem, dönüklük ve ölçek boş",
     form("local", base="5255", east="492345.678", north="4422345.678")),
    ("afinle TM30'a bağlı yerel sistem",
     form("local", "Belediye", base="5254", plane="affine", a="1.00002", b="-0.0003", c="1250", d="0.00025",
          e="0.99998", f="-830.5")),
    ("adsız, okunmayan orta meridyen, sıfır ölçek",
     form("tm", "  ", centralMeridian="otuz", scaleFactor="0", falseEasting="500000", falseNorthing="0")),
    ("sınır dışı enlem ve meridyen", form("tm", latitudeOfOrigin="95", centralMeridian="190", falseEasting="0",
                                          falseNorthing="0")),
    ("özel datumun adı, ekseni ve ötelemesi eksik",
     form("geographic", datum="custom", ellipsoid="custom", semiMajor="-1", inverseFlattening="0.5",
          parameters={"tx": "", "ty": "1", "tz": "2"}, accuracy="-0.1")),
    ("tabanı coğrafi ya da kayıtta yok", form("local", base="4326", east="0", north="0")),
    ("katlanan afin", form("local", base="5254", plane="affine", a="1", b="2", c="0", d="2", e="4", f="0")),
]


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what the rules give")
    args = ap.parse_args()
    cases = [{"name": name, "form": f, **build(f)} for name, f in CASES]
    data = {"format": "kentos.crs-definition-form", "version": 1, "texts": TEXTS,
            "ellipsoids": [{"name": k, "semiMajor": a, "inverseFlattening": rf} for k, (a, rf) in ELLIPSOIDS.items()],
            "cases": cases}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(cases)} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
