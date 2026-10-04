#!/usr/bin/env python3
"""Independent reference of Proje ayarları' Datum dönüşümleri (docs/adr/0168 §3, §6; step 4a).

Writes fixtures/crs/v1/choice-form.json from the ADR's rules, no KentOS code: for each of the registry's three datum
pairs the form says what EPSG's way is, and turns what is typed into the project's choice (`DatumTransform`, as the
contract writes it) or says, field by field, what is wrong.

- A pair is ED50–TUREF, ED50–WGS 84 or TUREF–WGS 84; its method is EPSG's way (no choice), seven parameters or an
  NTv2 grid of the device's library; its direction is the pair's order or the reverse.
- The name is what the values rest on: required (trimmed).
- Seven parameters: the translations are required numbers; an empty rotation or scale difference is 0 (three
  parameters); the convention is the position vector's or the coordinate frame's. Numbers are read as the Hesap
  windows read them: trimmed, the first comma a point.
- An NTv2 grid: one of the library's (its SHA-256, file and size come with it) or the one the choice already names
  when the device does not have it; required.
- The accuracy (m): empty is unknown, else 0 or more.

The desktop (kentos_project::choice_form) and the web (model/choiceForm.ts) must give exactly these.
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "crs" / "v1" / "choice-form.json"

PAIRS = [("ED50", "TUREF"), ("ED50", "WGS84"), ("TUREF", "WGS84")]
# EPSG's way for each pair, as the transforms say it (ADR 0167 §3).
EPSG = {
    ("ED50", "TUREF"): "±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil",
    ("ED50", "WGS84"): "±2 m, EPSG:1784; resmî dönüşüm değil",
    ("TUREF", "WGS84"): "±1 m, EPSG:5261",
}

NUMBER = re.compile(r"^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$", re.IGNORECASE)

TEXTS = {
    "name": "Adını yazın: değerlerin yanında dayanağı olarak görünür.",
    "number": "Sayı yazın, ör. -84.1.",
    "accuracy": "0 ya da büyük bir sayı yazın; bilinmiyorsa boş bırakın.",
    "grid": "Bir ızgara seçin; listede yoksa Izgaralar'dan ekleyin.",
}


def number(text):
    t = text.strip().replace(",", ".", 1)
    return float(t) if t and NUMBER.match(t) else None


def build_choice(pair, form, library):
    """The choice the form gives, or the problems by field; none for EPSG's way."""
    if form["method"] == "epsg":
        return None, {}
    a, b = pair
    frm, to = (b, a) if form["reversed"] else (a, b)
    problems = {}
    name = form["name"].strip()
    if not name:
        problems["name"] = TEXTS["name"]
    accuracy = None
    acc_text = form["accuracy"].strip()
    if acc_text:
        accuracy = number(acc_text)
        if accuracy is None or accuracy < 0:
            problems["accuracy"] = TEXTS["accuracy"]
    out = {"from": frm, "to": to, "name": name}
    if form["method"] == "helmert":
        values = {}
        for key in ("tx", "ty", "tz", "rx", "ry", "rz", "ds"):
            text = form["parameters"][key].strip()
            if not text and key in ("rx", "ry", "rz", "ds"):
                values[key] = 0.0
                continue
            v = number(text)
            if v is None:
                problems[key] = TEXTS["number"]
            else:
                values[key] = v
        if not problems:
            h = {"translation": [values["tx"], values["ty"], values["tz"]],
                 "rotation": [values["rx"], values["ry"], values["rz"]],
                 "scale": values["ds"], "convention": form["convention"]}
            if accuracy is not None:
                h["accuracy"] = accuracy
            out["helmert"] = h
    else:
        entry = next((e for e in library if e["id"] == form["grid"]), None)
        if entry is None:
            problems["grid"] = TEXTS["grid"]
        elif not problems:
            g = {"id": entry["id"], "file": entry["file"], "size": entry["size"]}
            if accuracy is not None:
                g["accuracy"] = accuracy
            out["grid"] = g
    return (None, problems) if problems else (out, {})


LIBRARY = [{"id": "b328bc7d133b48ffdacab572e7ccad7510e49fd7a78eb1194f81912711659b4a", "file": "tr.gsb", "size": 37952}]

HELMERT = {"tx": "-158.785", "ty": "-109,965", "tz": " -50.768 ", "rx": "1.4275", "ry": "-3.0873", "rz": "0.5505",
           "ds": "-5.1814"}
EMPTY = {k: "" for k in ("tx", "ty", "tz", "rx", "ry", "rz", "ds")}


def form(method, name="", reversed=False, parameters=None, convention="positionVector", grid="", accuracy=""):
    """A form as both platforms hold it: every field, as typed."""
    return {"method": method, "reversed": reversed, "name": name, "parameters": dict(EMPTY, **(parameters or {})),
            "convention": convention, "grid": grid, "accuracy": accuracy}

CASES = [
    ("EPSG'nin yolu: seçim yok", 0, form("epsg")),
    ("yedi parametre, virgüllü sayı ve boşluklar", 0,
     form("helmert", " ED50 → TUREF: Bölge 7 ", parameters=HELMERT, convention="coordinateFrame", accuracy="0.3")),
    ("üç parametre: dönüklük ve ölçek boş", 1,
     form("helmert", "ED50 → WGS 84: saha", parameters={"tx": "-87", "ty": "-98", "tz": "-121"})),
    ("ters yönde, doğruluğu yazılmamış", 0,
     form("helmert", "TUREF → ED50: ters", reversed=True, parameters=HELMERT, convention="coordinateFrame")),
    ("adsız ve okunmayan öteleme", 0, form("helmert", "  ", parameters=dict(HELMERT, ty="yüz"))),
    ("eksi doğruluk", 0, form("helmert", "Bölge", parameters=HELMERT, accuracy="-1")),
    ("kitaplıktaki ızgara, doğruluğuyla", 2,
     form("grid", "TUREF → WGS 84: ızgara", grid=LIBRARY[0]["id"], accuracy="0,05")),
    ("ızgara seçilmemiş", 1, form("grid", "ED50 → WGS 84: ızgara")),
]


def build():
    cases = []
    for name, pair_index, typed in CASES:
        pair = PAIRS[pair_index]
        choice, problems = build_choice(pair, typed, LIBRARY)
        case = {"name": name, "pair": list(pair), "form": typed}
        if problems:
            case["problems"] = problems
        else:
            case["choice"] = choice
        cases.append(case)
    return {"format": "kentos.crs-choice-form", "version": 1, "texts": TEXTS,
            "epsg": [{"pair": list(p), "text": EPSG[p]} for p in PAIRS], "library": LIBRARY, "cases": cases}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what the rules give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(CASES)} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
