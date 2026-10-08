#!/usr/bin/env python3
"""Independent reference of Proje ayarları › Ölçme's form (docs/adr/0169 §3; step 3a; docs/adr/0171 §2).

Writes fixtures/project/v1/survey-form.json from the rules alone, no KentOS code: the project's survey settings as the
form's fourteen texts (the refraction coefficient k, the two faces' horizontal reading difference, the index error, the
two faces' slope distance difference; a traverse leg's two-way difference, a traverse's angular and linear misclosure;
the project's mean ellipsoidal height; a network's a priori standard deviations, docs/adr/0203 §1) and the texts read
back into settings, with what is said of a text that does not hold.

The rules:

1. A text is read as the Hesap windows read a number: trimmed, its first comma a point,
   ^[-+]?(\\d+(\\.\\d*)?|\\.\\d+)(e[-+]?\\d+)?$ (any case); an empty text is no value.
2. k: within [−1, 1]; empty is the default 0.13, and so is 0.13 itself (a project keeps only another k).
3. A tolerance: above zero; empty is not checked. The angles are typed in cc (a ten-thousandth of a gon) in a gon
   project, in arc seconds (″) in a degree one, and kept in radians: cc × π / 2 000 000, ″ × π / 648 000 (each one
   multiplication, then one division, in this order). The lengths are typed in millimetres and kept in metres (÷ 1000).
4. The mean ellipsoidal height: metres, within [−500, 9000]; empty is none (the ground values are not given).
5. A value written for the form: k, the tolerances and the height back in their units (rad × 2 000 000 / π,
   rad × 648 000 / π, m × 1000; the height as it is), by the display rule (docs/adr/0149) with four decimals, trailing
   zeros and a bare point dropped; an absent value (k's default too) is an empty text.
6. What a text that does not hold says: "Sayı yazın." for one that is no number, k's range, a tolerance's sign or the
   height's range otherwise. The settings read are the values that hold; the form is saved only without a problem.
7. The a priori standard deviations (docs/adr/0203 §1): a direction's and a zenith angle's typed as a tolerance's angle,
   a distance's constant part, the centering and levelling's per √km in millimetres (kept in metres), the parts per
   million as typed. A direction, a distance, a zenith angle and levelling are above zero, the parts per million and the
   centering not below; a value is kept as typed even when it is the default. An empty field shows its default (the
   contract's SIGMA_DEFAULTS) as its placeholder, written by rule 5.
"""

import argparse
import json
import math
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from numeric_display import shown  # noqa: E402  (the display rule's own reference)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "project" / "v1" / "survey-form.json"
FIELDS = ("refraction", "faceHz", "index", "faceSlope", "twoWay", "traverseAngle", "traverseCoord", "groundHeight",
          "sigmaDirection", "sigmaDistance", "sigmaPpm", "sigmaCentering", "sigmaZenith", "sigmaLevelling")
LENGTHS = ("faceSlope", "twoWay", "traverseCoord", "sigmaDistance", "sigmaCentering", "sigmaLevelling")
# Kept as typed: k, the height, the parts per million.
PLAIN = ("refraction", "groundHeight", "sigmaPpm")
# The a priori standard deviations (docs/adr/0203 §1): above zero; the parts per million and the centering not below.
SIGMAS = ("sigmaDirection", "sigmaDistance", "sigmaZenith", "sigmaLevelling")
PARTS = ("sigmaPpm", "sigmaCentering")
# Their defaults (the contract's SIGMA_DEFAULTS): the placeholders.
SIGMA_DEFAULTS = {"sigmaDirection": math.pi / 200000, "sigmaDistance": 0.002, "sigmaPpm": 2.0, "sigmaCentering": 0.001,
                  "sigmaZenith": math.pi / 200000, "sigmaLevelling": 0.002}
TEXTS = {
    "number": "Sayı yazın.",
    "refraction": "−1 ile 1 arasında bir sayı yazın; boş bırakılırsa 0.13.",
    "tolerance": "Sıfırdan büyük bir sayı yazın; denetlenmeyecekse boş bırakın.",
    "height": "−500 ile 9000 m arasında bir yükseklik yazın; zemin değerleri gerekmiyorsa boş bırakın.",
    "sigma": "Sıfırdan büyük bir sayı yazın; boş bırakılırsa varsayılan.",
    "part": "Sıfır ya da sıfırdan büyük bir sayı yazın; boş bırakılırsa varsayılan.",
}
REFRACTION = 0.13
NUMBER = re.compile(r"^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$", re.IGNORECASE)


def number(text):
    t = text.strip().replace(",", ".", 1)
    return float(t) if t and NUMBER.match(t) else None


def to_stored(field, v, unit):
    if field in PLAIN:
        return v
    if field in LENGTHS:
        return v / 1000.0
    return v * math.pi / 2000000.0 if unit == "grad" else v * math.pi / 648000.0


def to_typed(field, v, unit):
    if field in PLAIN:
        return v
    if field in LENGTHS:
        return v * 1000.0
    return v * 2000000.0 / math.pi if unit == "grad" else v * 648000.0 / math.pi


def trimmed(s):
    if "." in s:
        s = s.rstrip("0").rstrip(".")
    return "0" if s in ("", "-0") else s


def texts(survey, unit):
    out = []
    for f in FIELDS:
        v = (survey or {}).get(f)
        out.append("" if v is None else trimmed(shown(to_typed(f, v, unit), 4)))
    return out


def read(typed, unit):
    survey, problems = {}, {}
    for f, t in zip(FIELDS, typed):
        if not t.strip():
            continue
        v = number(t)
        if v is None or not math.isfinite(v):
            problems[f] = TEXTS["number"]
            continue
        if f == "refraction":
            if not -1.0 <= v <= 1.0:
                problems[f] = TEXTS["refraction"]
            elif v != REFRACTION:
                survey[f] = v
            continue
        if f == "groundHeight":
            if not -500.0 <= v <= 9000.0:
                problems[f] = TEXTS["height"]
            else:
                survey[f] = v
            continue
        if f in PARTS:
            stored = to_stored(f, v, unit)
            if not (v >= 0.0 and math.isfinite(stored) and stored >= 0.0):
                problems[f] = TEXTS["part"]
            else:
                survey[f] = stored
            continue
        said = TEXTS["sigma"] if f in SIGMAS else TEXTS["tolerance"]
        if not v > 0.0:
            problems[f] = said
            continue
        stored = to_stored(f, v, unit)
        if not (math.isfinite(stored) and stored > 0.0):
            problems[f] = said
            continue
        survey[f] = stored
    return {"survey": survey or None, "problems": problems}


def placeholders(unit):
    """What an empty field shows: the a priori standard deviations' defaults in the units they are typed in."""
    return [trimmed(shown(to_typed(f, SIGMA_DEFAULTS[f], unit), 4)) if f in SIGMA_DEFAULTS else "" for f in FIELDS]


def cases():
    cc20 = 20 * math.pi / 2000000.0
    cc10 = 10 * math.pi / 2000000.0
    s6 = 6.48 * math.pi / 648000.0
    shows = [
        ("hiçbiri", None, "grad"),
        ("poligon toleransları, gon", {"twoWay": 0.01, "traverseAngle": 30 * math.pi / 2000000.0, "traverseCoord": 0.05}, "grad"),
        ("poligon toleransları, derece", {"twoWay": 0.01, "traverseAngle": 30 * math.pi / 2000000.0, "traverseCoord": 0.05}, "deg"),
        ("k ve üç tolerans, gon", {"refraction": 0.14, "faceHz": cc20, "index": cc10, "faceSlope": 0.005}, "grad"),
        ("aynısı, derece", {"refraction": 0.14, "faceHz": cc20, "index": cc10, "faceSlope": 0.005}, "deg"),
        ("eksi k, yalnız uzunluk farkı", {"refraction": -0.2, "faceSlope": 0.0025}, "grad"),
        ("saniyeyle yazılmış, derece", {"faceHz": s6}, "deg"),
        ("dört ondalıktan uzun", {"refraction": 0.123456, "faceHz": 12.345678 * math.pi / 2000000.0}, "grad"),
        ("ortalama yükseklik", {"groundHeight": 850.25}, "grad"),
        ("eksi ve uzun ondalıklı yükseklik, k ile", {"refraction": 0.14, "groundHeight": -27.123456}, "deg"),
        ("sıfır yükseklik", {"groundHeight": 0.0}, "grad"),
        ("önsel doğruluklar, gon", {"sigmaDirection": 5 * math.pi / 2000000.0, "sigmaDistance": 0.001, "sigmaPpm": 1.5,
                                    "sigmaCentering": 0.0005, "sigmaZenith": 15 * math.pi / 2000000.0, "sigmaLevelling": 0.0008}, "grad"),
        ("önsel doğruluklar, derece", {"sigmaDirection": 1.0 * math.pi / 648000.0, "sigmaPpm": 0.0, "sigmaCentering": 0.0}, "deg"),
    ]
    reads = [
        ("boş", ["", "", "", "", "", "", "", ""], "grad"),
        ("hepsi, gon", ["0.14", "20", "10", "5", "10", "30", "50", ""], "grad"),
        ("hepsi, derece", ["0,14", "6.48", "3,24", "2.5", "10", "9.72", "50", ""], "deg"),
        ("varsayılan k yazılmaz", ["0.13", "", "", "", "", "", "", ""], "grad"),
        ("boşluklar ve üs", [" -0.2 ", "1e1", "", " 5 ", "", "", "", ""], "grad"),
        ("aralık dışı k, sıfır ve eksi tolerans", ["1.5", "0", "-3", "4", "0", "-1", "x", ""], "grad"),
        ("sayı olmayanlar", ["k", "20cc", "1,5,0", ".", "", "", "", ""], "deg"),
        ("sınırlar: −1 ve 1", ["-1", "", "", "", "", "", "", ""], "grad"),
        ("sınırlar: 1", ["1", "", "0.0001", "", "", "", "", ""], "deg"),
        ("yükseklik", ["", "", "", "", "", "", "", "850.25"], "grad"),
        ("yükseklik virgülle ve boşlukla", ["", "", "", "", "", "", "", " 1250,5 "], "deg"),
        ("yükseklik sınırları", ["", "", "", "", "", "", "", "-500"], "grad"),
        ("yükseklik sınırları: 9000", ["", "", "", "", "", "", "", "9000"], "grad"),
        ("aralık dışı yükseklik", ["0.14", "", "", "", "", "", "", "9000.5"], "grad"),
        ("sayı olmayan yükseklik", ["", "", "", "", "", "", "", "850 m"], "grad"),
        ("sıfır yükseklik yazılır", ["", "", "", "", "", "", "", "0"], "grad"),
        ("önsel doğruluklar, gon", ["", "", "", "", "", "", "", "", "5", "1", "1,5", "0.5", "15", "0.8"], "grad"),
        ("önsel doğruluklar, derece", ["", "", "", "", "", "", "", "", "1", "3", "0", "0", "2.5", "1"], "deg"),
        ("varsayılana eşit de yazılır", ["", "", "", "", "", "", "", "", "10", "2", "2", "1", "10", "2"], "grad"),
        ("sıfır ve eksi önsel doğruluk", ["", "", "", "", "", "", "", "", "0", "-1", "-0.5", "-1", "x", "0"], "grad"),
    ]
    out = {"texts": [], "reads": []}
    for name, survey, unit in shows:
        out["texts"].append({"name": name, "unit": unit, "survey": survey, "texts": texts(survey, unit)})
    for name, typed, unit in reads:
        typed = typed + [""] * (len(FIELDS) - len(typed))
        out["reads"].append({"name": name, "unit": unit, "texts": typed, **read(typed, unit)})
    out["placeholders"] = {unit: placeholders(unit) for unit in ("grad", "deg")}
    return out


def build():
    return {"format": "kentos.survey-form", "version": 1, "source": "scripts/fixtures/survey_form_cases.py (docs/adr/0169 §3, docs/adr/0171 §2, docs/adr/0203 §1)", "fields": list(FIELDS), "messages": TEXTS, **cases()}


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
    b = build()
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(b['texts'])} gösterim, {len(b['reads'])} okuma durumu.")


if __name__ == "__main__":
    main()
