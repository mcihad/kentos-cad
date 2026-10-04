#!/usr/bin/env python3
"""Independent reference of Özel koordinat sistemi's WKT and PROJ reading (docs/adr/0168 §5, §6; step 4c).

Writes fixtures/crs/v1/definition-text.json from the systems PROJ reads in the texts of fixtures/geodesy/v1/text.json
(scripts/fixtures/crs_text_cases.py checks those against pyproj), the registry (fixtures/crs/v1/registry.json) and the
ADR's rules, no KentOS code: what the window makes of a pasted text.

- A text read is the project's definition (`CrsDefinition`, as the contract writes it), named as the text names it: a
  registry datum by its name, any other as the project's own (its name, ellipsoid and, when the text has them, its
  seven parameters to WGS 84; a text carries no accuracy). A local system's base is the registry's projected system with
  every value the same, else a definition of its own named “<ad> tabanı”.
- The same system as one of the registry's (a TM on its datum with its values, or the geographic system of its datum):
  its code is offered (`same`; the window's “Kayıttakini seç”). A grid the registry has on a datum of the text's own
  (core's `registry_match` not exact): said (`note`).
- A text that gives no system says why: not a definition, something the window does not read (the word it stopped at),
  a unit other than metres and degrees, a prime meridian other than Greenwich.

The desktop (kentos_project::definition_form::read) and the web (model/definitionForm.ts `readDefinition`) must give
exactly these.
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "crs" / "v1" / "definition-text.json"
TEXT = json.loads((ROOT / "fixtures" / "geodesy" / "v1" / "text.json").read_text(encoding="utf-8"))
REGISTRY = json.loads((ROOT / "fixtures" / "crs" / "v1" / "registry.json").read_text(encoding="utf-8"))["systems"]

TEXTS = {
    "syntax": "Metin okunamadı: WKT (PROJCS[…], GEOGCS[…], PROJCRS[…] …) ya da +proj= ile başlayan bir PROJ dizesi yapıştırın.",
    "unsupported": "“{detail}” okunmuyor: yalnız Transverse Mercator (UTM dahil), coğrafi sistem ve afinle türetilmiş yerel sistem tanımlanabilir.",
    "unit": "Birim “{detail}”: yalnız metre ve derece okunur.",
    "meridian": "Başlangıç meridyeni “{detail}”: yalnız Greenwich okunur.",
    "grid": "Izgarası EPSG:{srid} ({name}) ile aynı; datumu metnin kendi datumu.",
    "base": "{name} tabanı",
}


def registry_system(entry):
    """A registry entry as a definition's system writes it (none for the local one and the Pseudo-Mercator)."""
    if entry["kind"] == "geographic":
        return {"kind": "geographic", "datum": entry["datum"]}
    if entry["kind"] != "projected" or entry.get("projection") == "Pseudo-Mercator":
        return None
    return {
        "kind": "tm",
        "datum": entry["datum"],
        "centralMeridian": float(entry["centralMeridian"]),
        "scaleFactor": float(entry["scaleFactor"]),
        "falseEasting": float(entry["falseEasting"]),
        "falseNorthing": float(entry["falseNorthing"]),
    }


def same(system):
    """The registry's entry a system with a registry datum is, every value the same."""
    if system["kind"] == "local" or "datum" not in system:
        return None
    for entry in REGISTRY:
        if registry_system(entry) == system:
            return entry
    return None


def datum(d):
    """A datum read: the registry's by name, or the text's own."""
    if isinstance(d, str):
        return {"datum": d}
    own = {"name": d["name"], "ellipsoid": dict(d["ellipsoid"])}
    if "toWgs84" in d:
        own["toWgs84"] = dict(d["toWgs84"])
    return {"customDatum": own}


def tm(s):
    out = {"kind": "tm"}
    out.update(datum(s["datum"]))
    if s.get("latitudeOfOrigin", 0.0) != 0.0:
        out["latitudeOfOrigin"] = s["latitudeOfOrigin"]
    for k in ("centralMeridian", "scaleFactor", "falseEasting", "falseNorthing"):
        out[k] = s[k]
    return out


def system(s, name):
    if s["kind"] == "geographic":
        out = {"kind": "geographic"}
        out.update(datum(s["datum"]))
        return out
    if s["kind"] == "tm":
        return tm(s)
    base = tm(s["base"])
    entry = same(base)
    return {
        "kind": "local",
        "base": {"srid": entry["srid"]} if entry else {"definition": {"name": TEXTS["base"].format(name=name), "system": base}},
        "plane": dict(s["plane"]),
    }


def outcome(case):
    out = {"name": case["name"], "text": case["text"]}
    if "error" in case:
        why = case["error"]
        out["problem"] = TEXTS[why["kind"]].format(detail=why["detail"])
        return out
    e = case["expect"]
    d = {"name": e["name"], "system": system(e["system"], e["name"])}
    out["definition"] = d
    entry = same(d["system"])
    if entry:
        out["same"] = entry["srid"]
    r = e.get("registry")
    if r and not r["exact"]:
        named = next(x for x in REGISTRY if x["srid"] == r["srid"])
        out["note"] = TEXTS["grid"].format(srid=r["srid"], name=named["name"])
    return out


def build():
    reads = [outcome(c) for c in TEXT["reads"]]
    return {
        "format": "kentos.crs-definition-text",
        "version": 1,
        "source": "scripts/fixtures/crs_definition_text_cases.py (docs/adr/0168 §5, §6)",
        "texts": TEXTS,
        "reads": reads,
    }


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
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['reads'])} metin.")


if __name__ == "__main__":
    main()
