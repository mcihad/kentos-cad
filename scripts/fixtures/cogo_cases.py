"""Independent reference of the recorded measurements' rules (docs/adr/0180), and the cases both platforms play.

- Measured: a line's semt (grads, from north clockwise, 0 ≤ semt < 400) from its start to its end and its plane length; an
  arc's chord semt and length (start to end, counter-clockwise from a0 to a1), its radius and its length along the arc.
- Checked: each recorded value an object has (Kayıtlı semt, Kayıtlı uzunluk; on an arc also Kayıtlı yarıçap and Kayıtlı yay
  uzunluğu) against the measured one: the semt's difference in cc (1 cc = 0.0001 grad), brought between −200 and 200 grads,
  the lengths' in metres; over the tolerance it differs. A value that is no number (a decimal comma is a point) is unreadable.
- Recorded while drawing: a polar point typed as `@d<a` or `d<a` gives its distance as Kayıtlı uzunluk, in metres (a local
  project's millimetres or centimetres by moving the decimal point), and, in a GIS project with grads, its angle as Kayıtlı
  semt; the texts as typed, a decimal comma made a point, a leading plus dropped.

    python3 scripts/fixtures/cogo_cases.py           # writes fixtures/cogo/v1/cases.json
    python3 scripts/fixtures/cogo_cases.py --check   # writes nothing; compares

The rules are written here again, from the ADR, not from either platform's code. Numbers are mpmath's at 50 digits; a
recorded text is read exactly (Fraction) before it meets them.
"""

import json
import re
import sys
from fractions import Fraction
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/cogo/v1/cases.json"

SEMT, LENGTH, RADIUS, ARC = "Kayıtlı semt", "Kayıtlı uzunluk", "Kayıtlı yarıçap", "Kayıtlı yay uzunluğu"


def semt(dx, dy):
    """Grads from north, clockwise, 0 ≤ semt < 400."""
    g = mp.atan2(dx, dy) * 200 / mp.pi
    return g + 400 if g < 0 else g


def measure(shape):
    if shape["kind"] == "line":
        a, b = shape["a"], shape["b"]
        dx, dy = mp.mpf(b["x"]) - mp.mpf(a["x"]), mp.mpf(b["y"]) - mp.mpf(a["y"])
        return {"semt": semt(dx, dy), "length": mp.sqrt(dx * dx + dy * dy)}
    if shape["kind"] == "arc":
        c, r = shape["c"], mp.mpf(shape["r"])
        a0, a1 = mp.mpf(shape["a0"]), mp.mpf(shape["a1"])
        sweep = a1 - a0
        while sweep <= 0:
            sweep += 2 * mp.pi
        while sweep > 2 * mp.pi:
            sweep -= 2 * mp.pi
        sx, sy = mp.mpf(c["x"]) + r * mp.cos(a0), mp.mpf(c["y"]) + r * mp.sin(a0)
        ex, ey = mp.mpf(c["x"]) + r * mp.cos(a1), mp.mpf(c["y"]) + r * mp.sin(a1)
        dx, dy = ex - sx, ey - sy
        return {"semt": semt(dx, dy), "length": mp.sqrt(dx * dx + dy * dy), "radius": r, "arc": r * sweep}
    return None


NUMBER = re.compile(r"^\s*[+-]?([0-9]+([.,][0-9]*)?|[.,][0-9]+)\s*$")


def read(text):
    """A recorded text as an exact number, or None when it is no number."""
    if not NUMBER.match(text):
        return None
    return Fraction(text.strip().replace(",", ".").lstrip("+") or "0")


def check(shape, attrs, tol_length, tol_cc):
    m = measure(shape)
    if m is None:
        return None
    fields = [(SEMT, "semt"), (LENGTH, "length")] + ([(RADIUS, "radius"), (ARC, "arc")] if shape["kind"] == "arc" else [])
    items = []
    for name, key in fields:
        if name not in attrs:
            continue
        value = read(attrs[name])
        measured = m[key]
        if value is None:
            items.append({"field": key, "recorded": attrs[name], "measured": float(measured), "difference": None, "over": False})
            continue
        rec = mp.mpf(value.numerator) / value.denominator
        if key == "semt":
            d = (rec - measured + 200) % 400 - 200
            diff = d * 10000
            over = abs(diff) > tol_cc
        else:
            diff = rec - measured
            over = abs(diff) > tol_length
        items.append({"field": key, "recorded": attrs[name], "measured": float(measured), "difference": float(diff),
                      "over": over})
    if not items:
        return None
    status = "unreadable" if any(i["difference"] is None for i in items) else "differs" if any(i["over"] for i in items) else "ok"
    return {"status": status, "items": items}


POLAR = re.compile(r"^\s*@?\s*([+]?(?:[0-9]+(?:[.,][0-9]*)?|[.,][0-9]+))\s*<\s*([+-]?(?:[0-9]+(?:[.,][0-9]*)?|[.,][0-9]+))\s*$")


def plain(text):
    """A typed number's text as kept: a decimal comma made a point, a leading plus dropped, a bare point given its zero."""
    t = text.replace(",", ".").lstrip("+")
    if t.startswith("."):
        t = "0" + t
    if t.startswith("-."):
        t = "-0" + t[1:]
    if t.endswith("."):
        t = t[:-1]
    return t


def shift(text, places):
    """A non-negative decimal text divided by 10^places, exactly, as text: the point moved left."""
    if places == 0:
        return text
    whole, _, frac = text.partition(".")
    digits = whole + frac
    point = len(whole) - places
    if point <= 0:
        digits = "0" * (1 - point) + digits
        point = 1
    out = digits[:point].lstrip("0") or "0"
    rest = digits[point:]
    return out + ("." + rest if rest else "")


def record(text, convention, angle_unit, length_unit):
    m = POLAR.match(text)
    if not m:
        return None
    out = {LENGTH: shift(plain(m.group(1)), {"m": 0, "cm": 2, "mm": 3}[length_unit])}
    if convention == "gis" and angle_unit == "grad":
        out[SEMT] = plain(m.group(2))
    return out


def xy(x, y):
    return {"x": x, "y": y}


def cases():
    measures = []
    for name, shape in [
        ("Doğuya çizgi: semt 100", {"kind": "line", "a": xy(487000, 4420000), "b": xy(487025.4, 4420000)}),
        ("Kuzeye çizgi: semt 0", {"kind": "line", "a": xy(487000, 4420000), "b": xy(487000, 4420010)}),
        ("Güneybatıya çizgi: semt 250", {"kind": "line", "a": xy(487010, 4420010), "b": xy(487000, 4420000)}),
        ("Kuzeybatıya çizgi (−3, 4)", {"kind": "line", "a": xy(0, 0), "b": xy(-3, 4)}),
        ("Çeyrek yay: kirişin semti, uzunluğu, yarıçap ve yay uzunluğu",
         {"kind": "arc", "c": xy(100, 100), "r": 10, "a0": 0, "a1": 1.5707963267948966}),
        ("Sıfırdan geçen yay", {"kind": "arc", "c": xy(0, 0), "r": 5, "a0": 5.5, "a1": 0.5}),
    ]:
        m = measure(shape)
        measures.append({"name": name, "shape": shape, "measured": {k: float(v) for k, v in m.items()}})
    measures.append({"name": "Başka tür ölçülmez", "shape": {"kind": "point", "p": xy(0, 0)}, "measured": None})

    line = {"kind": "line", "a": xy(487000, 4420000), "b": xy(487025.4, 4420000)}
    arc = {"kind": "arc", "c": xy(100, 100), "r": 10, "a0": 0, "a1": 1.5707963267948966}
    checks = []
    for name, shape, attrs in [
        ("Kayıtlı değerler çizimle aynı: uyuyor", line, {SEMT: "100", LENGTH: "25.40"}),
        ("Uzunluk 4 cm farklı: farklı", line, {SEMT: "100.0000", LENGTH: "25.44"}),
        ("Semt 30 cc farklı, tolerans 50 cc: uyuyor", line, {SEMT: "100.0030", LENGTH: "25,40"}),
        ("Semt 399.9990, çizim 0'a yakın: fark −10 cc değil, dönüşle bulunur",
         {"kind": "line", "a": xy(0, 0), "b": xy(0, 10)}, {SEMT: "399.9990"}),
        ("Sayı olmayan kayıt: okunamadı", line, {LENGTH: "25,4 m"}),
        ("Kayıtlı ölçüsü olmayan çizgi listede yok", line, {"Ad": "Y1"}),
        ("Yayın dört ölçüsü uyuyor", arc, {SEMT: "350", LENGTH: "14.1421", RADIUS: "10", ARC: "15.708"}),
        ("Yayın yarıçapı farklı", arc, {RADIUS: "10.05"}),
        ("Çizgide yarıçap anlamsız: sayılmaz", line, {RADIUS: "5"}),
    ]:
        checks.append({"name": name, "shape": shape, "attrs": attrs, "toleranceLength": 0.01, "toleranceCc": 50,
                       "result": check(shape, attrs, 0.01, 50)})

    records = []
    for name, text, conv, unit, lu in [
        ("CBS, grad: uzunluk ve semt yazıldığı gibi", "@25.40<123.4567", "gis", "grad", "m"),
        ("@ yazılmadan, virgüllü ondalık ve boşluklar", " 25,40 < 123,4567 ", "gis", "grad", "m"),
        ("Artı işareti düşer, noktayla başlayan sayıya sıfır", "+.5<+100", "gis", "grad", "m"),
        ("CBS, derece: yalnız uzunluk", "@10<45", "gis", "deg", "m"),
        ("CAD: açı semt değildir, yalnız uzunluk", "@10<90", "cad", "deg", "m"),
        ("Yerel proje milimetre: nokta üç basamak sola", "2540<100", "gis", "grad", "mm"),
        ("Yerel proje santimetre: sıfırlar eklenir", "5<100", "gis", "grad", "cm"),
        ("Kutupsal olmayan yazı kaydedilmez", "487010,4420000", "gis", "grad", "m"),
        ("Uzaklık eksi olamaz", "-5<100", "gis", "grad", "m"),
    ]:
        records.append({"name": name, "text": text, "convention": conv, "angleUnit": unit, "lengthUnit": lu,
                        "recorded": record(text, conv, unit, lu)})
    return {"format": "kentos.cogo-cases", "version": 1,
            "note": "Written by scripts/fixtures/cogo_cases.py from docs/adr/0180, not from either platform's code. Semts in "
                    "grads, lengths in metres; a check's semt difference in cc, length differences in metres; `recorded` "
                    "is the attributes a typed polar point gives a line (null: none).",
            "measures": measures, "checks": checks, "records": records}


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/cogo_cases.py ile yeniden yazın")
            sys.exit(1)
        print(f"Kayıtlı ölçü durumları tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
