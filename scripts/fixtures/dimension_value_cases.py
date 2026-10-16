"""The shared cases of a dimension's written value (docs/adr/0321 §2, §3; plan/cad.md CAD-ANNO-09, CAD-ANNO-10).

    python3 scripts/fixtures/dimension_value_cases.py           # writes the file
    python3 scripts/fixtures/dimension_value_cases.py --check   # writes nothing; compares

Writes fixtures/text/v1/dimension-values.json: a measured value (metres; an angle in radians; a slope in per cent), a
dimension's look and the project's units, and the value as it is written. The rules, from the ADR, without KentOS code:

1. A length or a coordinate: rounded to the look's increment (`round`, metres; half away from zero), then in the
   look's unit (else the project's): with the look's `lengthStyle` (else the project's, unless the look names its own
   unit) in feet and inches when the unit is the inch or the foot and the style not decimal (docs/adr/0262 §2), else
   with its decimals (else the project's) by the display rule (docs/adr/0149).
2. Zeros (`zeros`): `trailing` takes the decimal digits' trailing zeros (and a bare point) away, `leading` the zero
   before the point (`0.50` → `.50`); `both` both. Not on feet and inches.
3. A tolerance (`tolerance`, metres): `symmetric` writes ` ±upper`; `deviation` ` +upper -lower`; `limits` writes the
   largest and the smallest value instead of the number, `max/min`. In the number's unit, the tolerance's decimals
   (else the number's).
4. The look's prefix, the kind's prefix (R, Ø, Y=, X=, t=, %), the number, the tolerance, the look's suffix; then the
   alternate unit (`alt`) in brackets after a space: ` [n]`, the measured value in the alternate unit and decimals (else
   the project's), not rounded by the increment.
5. An angle is written as the project's bearing (degrees here: four decimals and `°`), a slope's
   per cent with the look's decimals (else 2); neither takes the increment, the alternate unit nor the tolerance.
6. The text written over the value (`text`): `<>` in it is the value as above; a text without `<>` is shown as it is
   and the value is not (an override).
"""
import json
import math
import sys
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from annotation_style_cases import shown  # noqa: E402  (beside this file)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/text/v1/dimension-values.json"
RATIO = {"mm": (1.0, 1000.0), "cm": (1.0, 100.0), "m": (1.0, 1.0), "in": (254.0, 10000.0), "ft": (3048.0, 10000.0)}


def per_metre(unit):
    mul, div = RATIO[unit]
    return div / mul


def half_away(q):
    """The nearest whole number to the double q, a half away from zero."""
    return float(Decimal(q).quantize(Decimal(1), rounding=ROUND_HALF_UP))


def js_round(x):
    """JavaScript's Math.round: a half up."""
    return math.floor(x + 0.5)


def num(v):
    """A whole double as its integer's digits (Rust's `{}` of f64)."""
    return str(int(v)) if v == int(v) else repr(v)


def fraction(a, den):
    whole = math.floor(a)
    n = js_round((a - whole) * den)
    if n == den:
        return whole + 1, 0, den
    d = den
    while n and n % 2 == 0 and d > 1:
        n //= 2
        d //= 2
    return whole, n, d


def inches_text(w, n, d):
    return num(w) if n == 0 else (f"{n}/{d}" if w == 0 else f"{num(w)} {n}/{d}")


def format_inches(inches, style, decimals):
    neg, a = inches < 0, abs(inches)
    den = 1 << min(decimals, 8)
    if style == "fractional":
        w, n, d = fraction(a, den)
        body = inches_text(w, n, d) + '"'
    elif style == "architectural":
        w, n, d = fraction(a, den)
        feet = math.floor(w / 12.0)
        rest = w - feet * 12.0
        body = f"{num(feet)}'-{num(rest) if n == 0 else num(rest) + f' {n}/{d}'}\""
    else:
        scale = 10.0 ** min(decimals, 8)
        rounded = js_round(a * scale) / scale
        feet = math.floor(rounded / 12.0)
        rest = rounded - feet * 12.0
        body = f"{num(feet)}'-{shown(rest, decimals)}\""
    return "-" + body if neg and any(c in "123456789" for c in body) else body


def strip_zeros(text, how):
    if how in ("trailing", "both") and "." in text:
        text = text.rstrip("0").rstrip(".")
    if how in ("leading", "both"):
        if text.startswith("0."):
            text = text[1:]
        elif text.startswith("-0."):
            text = "-" + text[2:]
    return text


def number(metres, look, project, decimals=None):
    """A length in the look's unit and style."""
    o = look.get("options", {})
    unit = look.get("unit", project["unit"])
    style = o.get("lengthStyle", project.get("lengthStyle", "decimal") if "unit" not in look else "decimal")
    d = look.get("decimals", project["lengthDecimals"]) if decimals is None else decimals
    if unit in ("in", "ft") and style != "decimal":
        return format_inches(metres * 10000.0 / 254.0, style, d)
    return strip_zeros(shown(metres * per_metre(unit), d), o.get("zeros"))


def value(measured, look, project, text=None):
    kind = measured["unit"]
    v = measured["value"]
    o = look.get("options", {})
    if kind == "angle":
        # The project's bearing: radians to grads, then degrees, four decimals.
        grad = v * 200.0 / math.pi
        body = shown(grad, 4) + " g" if project.get("angleUnit") == "grad" else shown(grad / (200.0 / 180.0), 4) + "°"
        core = look.get("prefix", "") + measured["prefix"] + body + look.get("suffix", "")
    elif kind == "percent":
        core = look.get("prefix", "") + measured["prefix"] + shown(v, look.get("decimals", 2)) + look.get("suffix", "")
    else:
        r = o.get("round")
        rounded = half_away(v / r) * r if r else v
        tol = o.get("tolerance")
        tol_text = ""
        if tol and tol["kind"] == "limits":
            td = tol.get("decimals")
            body = number(rounded + tol["upper"], look, project, td) + "/" + number(rounded - tol["lower"], look, project, td)
        else:
            body = number(rounded, look, project)
            if tol:
                td = tol.get("decimals")
                if tol["kind"] == "symmetric":
                    tol_text = " ±" + number(tol["upper"], look, project, td)
                else:
                    tol_text = " +" + number(tol["upper"], look, project, td) + " -" + number(tol["lower"], look, project, td)
        core = look.get("prefix", "") + measured["prefix"] + body + tol_text + look.get("suffix", "")
        alt = o.get("alt")
        if alt:
            alt_look = {"unit": alt["unit"], "decimals": alt.get("decimals", project["lengthDecimals"])}
            core += " [" + number(v, alt_look, project) + "]"
    if text is None:
        return core
    return text.replace("<>", core)


METRIC = {"unit": "m", "lengthDecimals": 3, "angleUnit": "deg"}
MM = {"unit": "mm", "lengthDecimals": 0, "angleUnit": "deg"}
INCH = {"unit": "in", "lengthDecimals": 4, "lengthStyle": "architectural", "angleUnit": "deg"}
OWN = ("unit", "decimals", "prefix", "suffix")
L = lambda v, prefix="": {"value": v, "unit": "length", "prefix": prefix}  # noqa: E731


def cases():
    out = []

    def add(name, measured, flat, project, text=None):
        # The look's own fields, and its options (docs/adr/0321) as the contract nests them.
        look = {k: v for k, v in flat.items() if k in OWN}
        options = {k: v for k, v in flat.items() if k not in OWN}
        if options:
            look["options"] = options
        case = {"name": name, "measured": measured, "look": look, "project": project}
        if text is not None:
            case["text"] = text
        case["expect"] = value(measured, look, project, text)
        out.append(case)

    add("Uzunluk projenin birimi ve basamağıyla", L(3.24567), {}, METRIC)
    add("Yuvarlama artışı 5 cm: 3,24567 → 3,25", L(3.24567), {"round": 0.05}, METRIC)
    add("Yuvarlama artışı 0,5 m, yarım uzağa: 3,25 → 3,5", L(3.25), {"round": 0.5}, METRIC)
    add("Yuvarlama artışı 5 mm, mm biriminde", L(1.2374), {"round": 0.005, "unit": "mm", "decimals": 0}, METRIC)
    add("Sondaki sıfırlar: 2.500 → 2.5", L(2.5), {"zeros": "trailing"}, METRIC)
    add("Sondaki sıfırlar: tam sayı noktasız", L(4.0), {"zeros": "trailing"}, METRIC)
    add("Baştaki sıfır: 0.450 → .450", L(0.45), {"zeros": "leading"}, METRIC)
    add("İkisi: 0.450 → .45", L(0.45), {"zeros": "both"}, METRIC)
    add("Eksi koordinatın baştaki sıfırı", {"value": -0.25, "unit": "coordinate", "prefix": "Y="}, {"zeros": "leading"}, METRIC)
    add("Simetrik tolerans", L(2.5), {"tolerance": {"kind": "symmetric", "upper": 0.01}}, METRIC)
    add("Sapma toleransı, kendi basamağıyla", L(2.5), {"tolerance": {"kind": "deviation", "upper": 0.02, "lower": 0.005, "decimals": 4}},
        METRIC)
    add("Sınırlar: en büyük ve en küçük", L(2.5), {"tolerance": {"kind": "limits", "upper": 0.01, "lower": 0.005}}, METRIC)
    add("Sınırlar mm'de", L(1.2), {"unit": "mm", "decimals": 1, "tolerance": {"kind": "limits", "upper": 0.0002, "lower": 0.0001}}, METRIC)
    add("Alternatif birim mm, köşeli ayraçta", L(2.5), {"alt": {"unit": "mm", "decimals": 0}}, METRIC)
    add("Alternatif birim inç, yuvarlanmamış değerle", L(1.0), {"round": 0.5, "alt": {"unit": "in", "decimals": 2}}, METRIC)
    add("Ön ek, yarıçap ön eki, tolerans, son ek ve alternatif", L(0.75, "R "),
        {"prefix": "~", "suffix": " m", "tolerance": {"kind": "symmetric", "upper": 0.001}, "alt": {"unit": "cm", "decimals": 1}}, METRIC)
    add("İnç projesi: mimari biçim projeden", L(3.8227), {}, INCH)
    add("Ölçünün kendi biçimi: kesirli inç", L(3.8227), {"lengthStyle": "fractional", "decimals": 3}, INCH)
    add("Ölçünün kendi biçimi: mühendislik", L(3.8227), {"lengthStyle": "engineering", "decimals": 2}, INCH)
    add("Ölçü kendi birimini (mm) adlandırınca projenin mimari biçimi uygulanmaz", L(3.8227), {"unit": "mm", "decimals": 1}, INCH)
    add("mm projesi, basamaksız", L(1.23456), {}, MM)
    add("Açı derecede; artış ve tolerans almaz", {"value": math.pi / 3, "unit": "angle", "prefix": ""}, {"round": 0.5}, METRIC)
    add("Eğim yüzde, kendi basamağıyla", {"value": 12.3456, "unit": "percent", "prefix": "%"}, {"decimals": 1}, METRIC)
    add("Elle yazılan değer <> ile: ölçülen değer içinde", L(2.5), {}, METRIC, text="<> (tipik)")
    add("Elle yazılan değer <> olmadan: ölçülen gösterilmez", L(2.5), {}, METRIC, text="2.600")
    add("Elle yazılanda iki <>", L(1.25), {"zeros": "trailing"}, METRIC, text="<> + <>")
    return out


def main():
    doc = {
        "format": "kentos.dimension-value-cases",
        "version": 1,
        "title": "Ölçünün yazılan değeri: yuvarlama, sıfırlar, ayak ve inç, tolerans, alternatif birim, elle yazılan",
        "note": "ADR 0321. scripts/fixtures/dimension_value_cases.py kurallardan yazar; Format::dimension_value bunu geçer.",
        "cases": cases(),
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
