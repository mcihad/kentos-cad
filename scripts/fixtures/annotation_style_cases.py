"""The shared cases of named text and dimension styles (docs/adr/0183): the
style tables' rule, a text's face and a dimension's look checked, a style
applied, and an object following its style when the style changes.

    python3 scripts/fixtures/annotation_style_cases.py           # writes the file
    python3 scripts/fixtures/annotation_style_cases.py --check   # writes nothing; compares

Writes fixtures/text/v1/styles.json. The rules are written here from the ADR
on their own, not from an implementation's output; the contract's rules
(crates/shared/contracts/src/annotation.rs) and the web's
(apps/web/src/model/annotationStyles.ts) are held to them.

- A table (text or dimension styles) is checked style by style, in order:
  an empty id, an id seen before; a name empty when trimmed, a name with
  spaces at its ends, longer than 64 letters, with a control character
  (a line break), the reserved “Standart” (whatever its letters' case), a
  name seen before (lower-cased by Unicode's default rule); then each
  style's values, in order.
- A text style's values: a slant (degrees) not finite, 0, or 85 or more
  either way; a height (paper mm) not over 0 or over 1000; a width factor not
  over 0, over 100, or 1 (one spelling: 1 is no width factor); an empty
  font file.
- A dimension style's values: a height not over 0 or over 1000 mm; the
  arrowhead's size not over 0 (the gaps below 0), over 1000 mm, or over 100
  times the height; more than 8 decimals; a prefix or a suffix empty, over 32
  letters, or with a control character.
- A text's face: an empty style id; bold, italic or a slant without a
  typeface (the first of them named); a slant out of its bounds.
- A dimension's look: an empty style id; a size (times the height) not finite,
  over 100, the arrowhead's not over 0, the gaps below 0; more than 8
  decimals; a prefix or a suffix as a style's.
- Applying a text style: the style's face (its id, typeface, bold, italic,
  slant) and width factor; the height mm / 1000 × the scale when the style
  has one, else the text's own. Standart: no face, no width factor, the
  height kept.
- A text following its style from `old` to `new`: each face field and the
  width factor (absent counting as 1) equal to the old style's take the new
  one; the height moves when it is the old style's height at the scale and
  the new style has a height.
- Applying a dimension style: its look, each size as mm / the style's height,
  left out when it is the default (the tick 0.6, any other arrowhead 1, the
  gaps 0.5, the value 0.35); the height mm / 1000 × the scale. Standart: no
  look, the height the project's dimension height (2.5 mm unless a case
  names `standardMm`, docs/adr/0205 §1).
- A dimension's lines (docs/adr/0205 §6): its line's, extension lines' and
  value's colours `#RRGGBB`, its line's and extension lines' weights 0 to 100
  paper mm, their types; a style carries them into a dimension's look as
  they are, and following its style moves them as the other look fields.
- A dimension following its style: each look field equal to the old style's
  look takes the new one; the height moves when it is the old style's.
- A dimension's value as written (docs/adr/0183 §3): its look's prefix, its
  kind's own prefix, the number, its look's suffix. A length or a
  coordinate: metres times the look's unit's (else the project's) units per
  metre, in float64, shown with the look's decimals (else the project's
  length decimals) by the display rule (docs/adr/0149: seven decimals first,
  then the digits shown, a half away from zero, no sign on zero). A slope's
  percentage: the look's decimals, else 2; no unit.
"""
import json
import math
import sys
from decimal import ROUND_HALF_UP, Decimal, getcontext
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/text/v1/styles.json"

MAX_OBLIQUE = 85.0
MAX_NAME = 64
MAX_MM = 1000.0
MAX_RATIO = 100.0
MAX_DECIMALS = 8
MAX_AFFIX = 32
STANDARD = "Standart"
DEFAULTS = {"tick": 0.6, "arrow": 1.0, "extOffset": 0.5, "extBeyond": 0.5, "textGap": 0.35}
STANDARD_DIMENSION_MM = 2.5
MAX_LINE_WEIGHT = 100.0
LINE_FIELDS = ("dimLineColor", "dimLineWeight", "dimLineType", "extColor", "extWeight", "extLineType", "textColor")


def hex_colour(c):
    return len(c) == 7 and c[0] == "#" and all(ch in "0123456789abcdefABCDEF" for ch in c[1:])


def line_problem(l):
    """The line colours and weights' refusal: the field and its words, or None."""
    for key, what in [("dimLineColor", "ölçü çizgisinin rengi"), ("extColor", "uzatma çizgilerinin rengi"), ("textColor", "değerinin rengi")]:
        c = l.get(key)
        if c is not None and not hex_colour(c):
            return [key, f"Ölçünün {what} #RRGGBB biçiminde olmalı; “{c}” verildi. Rengi #RRGGBB olarak verin ya da alanı kaldırın (nesnenin rengi)."]
    for key, what in [("dimLineWeight", "ölçü çizgisinin kalınlığı"), ("extWeight", "uzatma çizgilerinin kalınlığı")]:
        w = l.get(key)
        if w is not None and not (math.isfinite(w) and 0 <= w <= MAX_LINE_WEIGHT):
            return [key, f"Ölçünün {what} kâğıtta 0 ile {num(MAX_LINE_WEIGHT)} mm arasında olmalı; {num(w)} verildi. Bu aralıkta verin ya da alanı kaldırın (kılcal)."]
    return None


def num(x):
    """A number as Rust's Display and JavaScript's String write it here."""
    if isinstance(x, bool):
        raise TypeError(x)
    if isinstance(x, int):
        return str(x)
    if math.isnan(x):
        return "NaN"
    if math.isinf(x):
        return "inf" if x > 0 else "-inf"
    if x == int(x) and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def control(s):
    return any(ord(c) < 32 or 0x7F <= ord(c) <= 0x9F for c in s)


def names_problem(kind, styles):
    seen_ids, seen_names = [], []
    for s in styles:
        sid, name = s["id"], s["name"]
        if sid == "":
            return f"{kind} kimliği boş"
        if sid in seen_ids:
            return f"“{sid}” kimlikli {kind} iki kez var"
        trimmed = name.strip()
        if trimmed == "":
            return f"{kind} adı boş"
        if trimmed != name:
            return f"“{name}” {kind} adının başında ya da sonunda boşluk var"
        if len(trimmed) > MAX_NAME:
            return f"“{trimmed}” {kind} adı {MAX_NAME} harften uzun"
        if control(trimmed):
            return f"“{trimmed}” {kind} adında satır sonu ya da denetim karakteri var"
        folded = trimmed.lower()
        if folded == STANDARD.lower():
            return f"“{trimmed}” adı Standart'ındır; {kind} başka bir ad almalı"
        if folded in seen_names:
            return f"“{trimmed}” adlı {kind} iki kez var"
        seen_ids.append(sid)
        seen_names.append(folded)
    return None


def oblique_holds(o):
    return math.isfinite(o) and o != 0 and abs(o) < MAX_OBLIQUE


def mm_holds(mm, positive):
    return math.isfinite(mm) and mm <= MAX_MM and (mm > 0 if positive else mm >= 0)


def affix_problem(s):
    n = len(s)
    if n == 0:
        return "boş"
    if n > MAX_AFFIX:
        return f"{n} harf; en çok {MAX_AFFIX}"
    if control(s):
        return "satır sonu ya da denetim karakteri var"
    return None


def text_style_problem(s):
    name = s["name"]
    o = s.get("oblique")
    if o is not None and not oblique_holds(o):
        return f"“{name}” yazı stilinin eğikliği {num(o)}; −{num(MAX_OBLIQUE)} ile {num(MAX_OBLIQUE)} arasında ve sıfırdan farklı olmalı"
    h = s.get("height")
    if h is not None and not mm_holds(h, True):
        return f"“{name}” yazı stilinin yüksekliği {num(h)} mm; sıfırdan büyük, en çok {num(MAX_MM)} olmalı"
    w = s.get("widthFactor")
    if w is not None and not (0 < w <= 100 and w != 1):
        return f"“{name}” yazı stilinin genişlik çarpanı {num(w)}; sıfırdan büyük, en çok 100 ve 1'den farklı olmalı (1 yazılmaz)"
    if s.get("fontFile") == "":
        return f"“{name}” yazı stilinin yazı tipi dosyası boş"
    return None


def dimension_style_problem(s):
    name, height = s["name"], s["height"]
    if not mm_holds(height, True):
        return f"“{name}” ölçü stilinin değer yüksekliği {num(height)} mm; sıfırdan büyük, en çok {num(MAX_MM)} olmalı"
    for what, key, positive in [
        ("ok boyu", "arrowSize", True),
        ("uzatma çizgisinin boşluğu", "extOffset", False),
        ("uzatma çizgisinin aşması", "extBeyond", False),
        ("değerin çizgiden yüksekliği", "textGap", False),
    ]:
        v = s.get(key)
        if v is not None and not (mm_holds(v, positive) and v / height <= MAX_RATIO):
            floor = "sıfırdan büyük" if positive else "0 ya da büyük"
            return f"“{name}” ölçü stilinin {what} {num(v)} mm; {floor}, en çok {num(MAX_MM)} ve değer yüksekliğinin {num(MAX_RATIO)} katı olmalı"
    d = s.get("decimals")
    if d is not None and d > MAX_DECIMALS:
        return f"“{name}” ölçü stilinin basamak sayısı {d}; en çok {MAX_DECIMALS} olmalı"
    for what, key in [("öneki", "prefix"), ("soneki", "suffix")]:
        v = s.get(key)
        if v is not None:
            why = affix_problem(v)
            if why:
                return f"“{name}” ölçü stilinin {what} yazılamaz: {why}"
    line = line_problem(s)
    return f"“{name}” ölçü stili: {line[1]}" if line else None


def text_styles_problem(styles):
    return names_problem("yazı stili", styles) or next(
        (p for p in map(text_style_problem, styles) if p), None
    )


def dimension_styles_problem(styles):
    return names_problem("ölçü stili", styles) or next(
        (p for p in map(dimension_style_problem, styles) if p), None
    )


def face_problem(f):
    if f.get("textStyle") == "":
        return ["textStyle", "Yazının stil kimliği boş. Stilin kimliğini verin ya da alanı kaldırın (Standart)."]
    if f.get("font") is None:
        for key in ("bold", "italic"):
            if f.get(key):
                return [key, "Kalın, eğik ve yatık yazı bir yazı tipiyle olur; yazının yazı tipi yok. Yazıya bir yazı tipi verin ya da bu alanları kaldırın (yazı projenin yazı tipiyle çizilir)."]
        if f.get("oblique") is not None:
            return ["oblique", "Kalın, eğik ve yatık yazı bir yazı tipiyle olur; yazının yazı tipi yok. Yazıya bir yazı tipi verin ya da bu alanları kaldırın (yazı projenin yazı tipiyle çizilir)."]
    o = f.get("oblique")
    if o is not None and not oblique_holds(o):
        return ["oblique", f"Yazının eğikliği −{num(MAX_OBLIQUE)} ile {num(MAX_OBLIQUE)} derece arasında ve sıfırdan farklı olmalı; {num(o)} verildi. Eğikliği bu aralıkta verin ya da alanı kaldırın (dik)."]
    return None


def look_problem(l):
    if l.get("dimStyle") == "":
        return ["dimStyle", "Ölçünün stil kimliği boş. Stilin kimliğini verin ya da alanı kaldırın (Standart)."]
    for key, what, positive in [
        ("arrowSize", "ok boyu", True),
        ("extOffset", "uzatma çizgisinin boşluğu", False),
        ("extBeyond", "uzatma çizgisinin aşması", False),
        ("textGap", "değerin çizgiden yüksekliği", False),
    ]:
        v = l.get(key)
        if v is not None and not (math.isfinite(v) and v <= MAX_RATIO and (v > 0 if positive else v >= 0)):
            floor = "sıfırdan büyük" if positive else "0 ya da büyük"
            return [key, f"Ölçünün {what} değer yüksekliğinin katıdır: {floor}, en çok {num(MAX_RATIO)} olmalı; {num(v)} verildi. Bu aralıkta verin ya da alanı kaldırın (Standart'ınki)."]
    d = l.get("decimals")
    if d is not None and d > MAX_DECIMALS:
        return ["decimals", f"Ölçünün basamak sayısı en çok {MAX_DECIMALS}; {d} verildi. Daha az basamak verin ya da alanı kaldırın (projenin basamakları)."]
    for key, what in [("prefix", "öneki"), ("suffix", "soneki")]:
        v = l.get(key)
        if v is not None:
            why = affix_problem(v)
            if why:
                return [key, f"Ölçünün {what} yazılamaz: {why}. Tek satır, en çok {MAX_AFFIX} harf verin ya da alanı kaldırın."]
    return line_problem(l)


FACE = ("textStyle", "font", "bold", "italic", "oblique")


def face_of(style):
    """A text style's face, as an object's fields (absent: left out)."""
    out = {"textStyle": style["id"], "font": style["font"]}
    if style.get("bold"):
        out["bold"] = True
    if style.get("italic"):
        out["italic"] = True
    if style.get("oblique") is not None:
        out["oblique"] = style["oblique"]
    return out


def height_at(mm, scale):
    return mm / 1000 * scale


def apply_text(style, text, scale):
    out = {k: v for k, v in text.items() if k not in FACE and k != "widthFactor"}
    if style is None:
        return out
    out.update(face_of(style))
    if style.get("widthFactor") is not None:
        out["widthFactor"] = style["widthFactor"]
    if style.get("height") is not None:
        out["height"] = height_at(style["height"], scale)
    return out


def follow_text(old, new, text, scale):
    out = dict(text)
    was, now = face_of(old), face_of(new)
    for key in ("font", "oblique"):
        if text.get(key) == was.get(key):
            out.pop(key, None)
            if now.get(key) is not None:
                out[key] = now[key]
    for key in ("bold", "italic"):
        if bool(text.get(key)) == bool(was.get(key)):
            out.pop(key, None)
            if now.get(key):
                out[key] = True
    if text.get("widthFactor", 1) == old.get("widthFactor", 1):
        out.pop("widthFactor", None)
        if new.get("widthFactor") is not None:
            out["widthFactor"] = new["widthFactor"]
    if old.get("height") is not None and new.get("height") is not None and text["height"] == height_at(old["height"], scale):
        out["height"] = height_at(new["height"], scale)
    return out


LOOK = ("dimStyle", "arrow", "arrowSize", "extOffset", "extBeyond", "textGap", "textPlace", "decimals", "unit", "prefix", "suffix", "font") + LINE_FIELDS


def look_of(style):
    out = {"dimStyle": style["id"]}
    if style.get("arrow") is not None:
        out["arrow"] = style["arrow"]
    tick = style.get("arrow") is None
    for key, default in [
        ("arrowSize", DEFAULTS["tick"] if tick else DEFAULTS["arrow"]),
        ("extOffset", DEFAULTS["extOffset"]),
        ("extBeyond", DEFAULTS["extBeyond"]),
        ("textGap", DEFAULTS["textGap"]),
    ]:
        if style.get(key) is not None:
            r = style[key] / style["height"]
            if r != default:
                out[key] = r
    for key in ("textPlace", "decimals", "unit", "prefix", "suffix", "font") + LINE_FIELDS:
        if style.get(key) is not None:
            out[key] = style[key]
    return out


def apply_dimension(style, dim, scale, standard=STANDARD_DIMENSION_MM):
    out = {k: v for k, v in dim.items() if k not in LOOK}
    if style is None:
        out["height"] = height_at(standard, scale)
        return out
    out.update(look_of(style))
    out["height"] = height_at(style["height"], scale)
    return out


def follow_dimension(old, new, dim, scale):
    out = dict(dim)
    was, now = look_of(old), look_of(new)
    for key in LOOK:
        if key == "dimStyle":
            continue
        if dim.get(key) == was.get(key):
            out.pop(key, None)
            if now.get(key) is not None:
                out[key] = now[key]
    if dim["height"] == height_at(old["height"], scale):
        out["height"] = height_at(new["height"], scale)
    return out


ADA = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101", "name": "Ada no", "font": "arimo", "bold": True, "height": 3.5}
YOL = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0102", "name": "Yol adı", "font": "overpass", "italic": True, "oblique": 15.0, "widthFactor": 0.8}
NOT = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0103", "name": "Not", "font": "barlow"}
OLCU = {
    "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0201",
    "name": "Mimari",
    "height": 3.5,
    "arrow": "closed",
    "arrowSize": 3.5,
    "extOffset": 1.0,
    "extBeyond": 1.75,
    "textGap": 0.875,
    "decimals": 1,
    "unit": "cm",
    "prefix": "~",
    "suffix": " cm",
    "font": "arimo",
}
CENTIK = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0202", "name": "Harita", "height": 2.5, "arrowSize": 1.5, "textPlace": "centre"}


getcontext().prec = 200
NOISE_DECIMALS = 7
PER_METRE = {"mm": 1000.0, "cm": 100.0, "m": 1.0}


def shown(v, d):
    """The display rule (docs/adr/0149), as scripts/fixtures/numeric_display.py writes it."""
    x = Decimal(abs(v))
    if d >= NOISE_DECIMALS:
        y = x.quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    else:
        y = x.quantize(Decimal(1).scaleb(-NOISE_DECIMALS), rounding=ROUND_HALF_UP)
        y = y.quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    text = format(y, "f")
    if v < 0 and any(c in "123456789" for c in text):
        text = "-" + text
    return text


def dimension_value(measured, look, project):
    """A dimension's value as written: prefixes, the number, the suffix."""
    kind = measured["unit"]
    if kind == "percent":
        number = shown(measured["value"], look.get("decimals", 2))
    else:
        unit = look.get("unit", project["unit"])
        number = shown(measured["value"] * PER_METRE[unit], look.get("decimals", project["lengthDecimals"]))
    return look.get("prefix", "") + measured["prefix"] + number + look.get("suffix", "")


def cases():
    out = []

    def add(name, op, inp, expect):
        out.append({"name": name, "op": op, **inp, "expect": expect})

    # ── The tables' rule ────────────────────────────────────────────────
    for name, styles in [
        ("Boş tablo", []),
        ("Üç yazı stili", [ADA, YOL, NOT]),
        ("Kimliği boş", [dict(ADA, id="")]),
        ("Aynı kimlik iki kez", [ADA, dict(YOL, id=ADA["id"])]),
        ("Adı boşluklardan", [dict(ADA, name="   ")]),
        ("Adının başında boşluk", [dict(ADA, name=" Ada no")]),
        ("Adı 65 harf", [dict(ADA, name="A" * 65)]),
        ("Adı 64 harf", [dict(ADA, name="Ç" * 64)]),
        ("Adında satır sonu", [dict(ADA, name="Ada\nno")]),
        ("Standart ayrılmıştır, büyük küçük harf ayırmadan", [dict(ADA, name="STANDART")]),
        ("Aynı ad, büyük küçük harf ayırmadan", [ADA, dict(YOL, name="ADA NO")]),
        ("İ'nin küçüğü i̇: İL ile il ayrıdır (Unicode'un kuralı)", [dict(ADA, name="İL"), dict(YOL, name="il")]),
        ("Eğiklik 85", [dict(YOL, oblique=85.0)]),
        ("Eğiklik −84,5", [dict(YOL, oblique=-84.5)]),
        ("Eğiklik 0 yazılmaz", [dict(YOL, oblique=0.0)]),
        ("Yükseklik 0", [dict(ADA, height=0.0)]),
        ("Yükseklik 1001 mm", [dict(ADA, height=1001.0)]),
        ("Genişlik çarpanı 1 yazılmaz", [dict(YOL, widthFactor=1.0)]),
        ("Genişlik çarpanı 0", [dict(YOL, widthFactor=0.0)]),
        ("Yazı tipi dosyası boş", [dict(NOT, fontFile="")]),
        ("Yazı tipi dosyası", [dict(NOT, fontFile="romans.shx")]),
    ]:
        add(name, "textStylesProblem", {"styles": styles}, text_styles_problem(styles))
    for name, styles in [
        ("İki ölçü stili", [OLCU, CENTIK]),
        ("Değer yüksekliği 0", [dict(OLCU, height=0.0)]),
        ("Ok boyu 0", [dict(OLCU, arrowSize=0.0)]),
        ("Boşluk −1 mm", [dict(OLCU, extOffset=-1.0)]),
        ("Aşma 0 olabilir", [dict(OLCU, extBeyond=0.0)]),
        ("Değerin yüksekliği yüksekliğin 120 katı", [dict(CENTIK, textGap=300.0)]),
        ("9 basamak", [dict(OLCU, decimals=9)]),
        ("Önek boş", [dict(OLCU, prefix="")]),
        ("Sonek 33 harf", [dict(OLCU, suffix="m" * 33)]),
        ("Sonekte sekme", [dict(OLCU, suffix="\tm")]),
        ("Standart adı ölçüde de ayrılmıştır", [dict(CENTIK, name="standart")]),
        ("Çizgileri olan stil", [dict(OLCU, dimLineColor="#C0392B", dimLineWeight=0.35, dimLineType="dashed", extColor="#7F8C8D", extWeight=0.18, extLineType="dotted", textColor="#1F4E79")]),
        ("Ölçü çizgisinin rengi #RRGGBB değil", [dict(OLCU, dimLineColor="kırmızı")]),
        ("Uzatma çizgilerinin kalınlığı 101 mm", [dict(OLCU, extWeight=101.0)]),
    ]:
        add(name, "dimensionStylesProblem", {"styles": styles}, dimension_styles_problem(styles))

    # ── A face and a look ───────────────────────────────────────────────
    for name, face in [
        ("Yüzsüz yazı", {}),
        ("Stilin yüzü", face_of(YOL)),
        ("Yazı tipi olmadan kalın", {"bold": True}),
        ("Yazı tipi olmadan eğik", {"italic": True}),
        ("Yazı tipi olmadan yatık", {"oblique": 10.0}),
        ("Yatıklık 90", {"font": "arimo", "oblique": 90.0}),
        ("Stil kimliği boş", {"textStyle": ""}),
        ("Stili var, yüzü yok", {"textStyle": ADA["id"]}),
    ]:
        add(name, "faceProblem", {"face": face}, face_problem(face))
    for name, look in [
        ("Görünüşsüz ölçü", {}),
        ("Stilin görünüşü", look_of(OLCU)),
        ("Ok boyu 0", {"arrowSize": 0.0}),
        ("Boşluk −0,1", {"extOffset": -0.1}),
        ("Değer 101 kat yukarıda", {"textGap": 101.0}),
        ("9 basamak", {"decimals": 9}),
        ("Önek boş", {"prefix": ""}),
        ("Sonekte sekme", {"suffix": "\tm"}),
        ("Stil kimliği boş", {"dimStyle": ""}),
        ("Çizgiler", {"dimLineColor": "#C0392B", "dimLineWeight": 0.35, "dimLineType": "dashdot", "extColor": "#7f8c8d", "extWeight": 0.0, "extLineType": "dashed", "textColor": "#1F4E79"}),
        ("Değerin rengi kısa", {"textColor": "#F00"}),
        ("Ölçü çizgisi −0,1 mm", {"dimLineWeight": -0.1}),
    ]:
        add(name, "lookProblem", {"look": look}, look_problem(look))

    # ── Applying and following ──────────────────────────────────────────
    text = {"height": 2.0}
    styles = {"Ada no": ADA, "Yol adı": YOL, "Not": NOT}
    for name, style, t, scale in [
        ("Ada no 1:1000: kalın Arimo, 3,5 mm", ADA, text, 1000.0),
        ("Ada no 1:500", ADA, {"height": 2.0, "widthFactor": 0.7}, 500.0),
        ("Yol adı: yüksekliği yok, yazınınki kalır", YOL, {"height": 1.25}, 1000.0),
        ("Standart: yüz ve genişlik gider, yükseklik kalır", None, dict(face_of(YOL), height=1.25, widthFactor=0.8), 1000.0),
    ]:
        add(name, "applyText", {"style": style["id"] if style else None, "styles": [ADA, YOL, NOT], "plotScale": scale, "text": t}, apply_text(style, t, scale))
    ada2 = dict(ADA, font="overpass", bold=False, italic=True, oblique=-10.0, height=4.0, widthFactor=0.9)
    for name, t in [
        ("Hepsi stilin: hepsi yeni", apply_text(ADA, text, 1000.0)),
        ("Kendi genişliği kalır", dict(apply_text(ADA, text, 1000.0), widthFactor=0.6)),
        ("Kendi yüksekliği kalır", dict(apply_text(ADA, text, 1000.0), height=5.0)),
        ("Kendi yazı tipi kalır", dict(apply_text(ADA, text, 1000.0), font="plex-mono")),
    ]:
        add(name, "followText", {"old": ADA, "new": ada2, "plotScale": 1000.0, "text": t}, follow_text(ADA, ada2, t, 1000.0))
    add("Eski stilin yüksekliği yoktu: yükseklik kalır", "followText", {"old": YOL, "new": dict(YOL, height=3.0), "plotScale": 1000.0, "text": apply_text(YOL, {"height": 1.0}, 1000.0)}, follow_text(YOL, dict(YOL, height=3.0), apply_text(YOL, {"height": 1.0}, 1000.0), 1000.0))
    ada_free = {k: v for k, v in ADA.items() if k != "height"}
    add("Yeni stilin yüksekliği yok: yükseklik kalır", "followText", {"old": ADA, "new": ada_free, "plotScale": 1000.0, "text": apply_text(ADA, text, 1000.0)}, follow_text(ADA, ada_free, apply_text(ADA, text, 1000.0), 1000.0))

    dim = {"height": 1.0}
    for name, style, scale in [
        ("Mimari 1:50: dolu ok, cm, bir basamak", OLCU, 50.0),
        ("Harita: çentik 1,5 mm (varsayılan oran yazılmaz), değer ortada", CENTIK, 1000.0),
        ("Standart 1:1000: görünüş gider, değer 2,5 mm", None, 1000.0),
    ]:
        add(name, "applyDimension", {"style": style["id"] if style else None, "styles": [OLCU, CENTIK], "plotScale": scale, "dimension": dict(look_of(CENTIK), height=0.3)}, apply_dimension(style, dict(look_of(CENTIK), height=0.3), scale))
    # Standart's height is the project's (docs/adr/0205 §1); a style's lines come with its look (§6).
    add("Standart 1:500, projenin ölçü yüksekliği 3,5 mm", "applyDimension", {"style": None, "styles": [OLCU, CENTIK], "plotScale": 500.0, "standardMm": 3.5, "dimension": dict(look_of(CENTIK), height=0.3)}, apply_dimension(None, dict(look_of(CENTIK), height=0.3), 500.0, 3.5))
    lined = dict(CENTIK, dimLineColor="#C0392B", dimLineWeight=0.35, extLineType="dotted", textColor="#1F4E79")
    add("Çizgileri olan stil: renkler, kalınlık ve tip görünüşe geçer", "applyDimension", {"style": lined["id"], "styles": [OLCU, lined], "plotScale": 1000.0, "dimension": {"height": 0.3}}, apply_dimension(lined, {"height": 0.3}, 1000.0))
    olcu2 = dict(OLCU, height=2.5, arrow="dot", arrowSize=2.5, unit="mm", decimals=0, prefix=None, suffix=" mm")
    olcu2 = {k: v for k, v in olcu2.items() if v is not None}
    for name, d in [
        ("Hepsi stilin: hepsi yeni", apply_dimension(OLCU, dim, 50.0)),
        ("Kendi oku ve öneki kalır", dict(apply_dimension(OLCU, dim, 50.0), arrow="open", prefix="≈")),
        ("Kendi yüksekliği kalır", dict(apply_dimension(OLCU, dim, 50.0), height=0.2)),
    ]:
        add(name, "followDimension", {"old": OLCU, "new": olcu2, "plotScale": 50.0, "dimension": d}, follow_dimension(OLCU, olcu2, d, 50.0))
    # The lines follow as the other look fields (docs/adr/0205 §6): the style's change, an own colour kept.
    lined_old = dict(OLCU, dimLineColor="#C0392B", extWeight=0.18)
    lined_new = dict(OLCU, dimLineColor="#1F4E79", extWeight=0.25, dimLineType="dashed")
    for name, d in [
        ("Çizgiler stilin: hepsi yeni", apply_dimension(lined_old, dim, 50.0)),
        ("Kendi rengi kalır, kalınlık ve tip yeni", dict(apply_dimension(lined_old, dim, 50.0), dimLineColor="#000000")),
    ]:
        add(name, "followDimension", {"old": lined_old, "new": lined_new, "plotScale": 50.0, "dimension": d}, follow_dimension(lined_old, lined_new, d, 50.0))

    # ── A dimension's value as written ──────────────────────────────────
    metres = {"unit": "m", "lengthDecimals": 3}
    for name, measured, look, project in [
        ("Standart: projenin birimi ve basamakları", {"prefix": "", "unit": "length", "value": 12.3456}, {}, metres),
        ("cm, bir basamak, sonekle", {"prefix": "", "unit": "length", "value": 1.2345}, {"unit": "cm", "decimals": 1, "suffix": " cm"}, metres),
        ("mm, sıfır basamak: 0,5 mm yukarı", {"prefix": "", "unit": "length", "value": 0.0125}, {"unit": "mm", "decimals": 0}, metres),
        ("Yarıçapın öneki stilin önekinden sonra", {"prefix": "R ", "unit": "length", "value": 2.5}, {"prefix": "≈", "decimals": 2}, metres),
        ("Çap, sekiz basamak", {"prefix": "Ø ", "unit": "length", "value": 0.1 + 0.2}, {"decimals": 8}, metres),
        ("Yerel projenin cm'si; stilin birimi m", {"prefix": "", "unit": "length", "value": 3.25}, {"unit": "m", "decimals": 2}, {"unit": "cm", "lengthDecimals": 1}),
        ("Yerel projenin mm'si, stilde birim yok", {"prefix": "", "unit": "length", "value": 3.25}, {"decimals": 0}, {"unit": "mm", "lengthDecimals": 1}),
        ("Koordinat cm'de", {"prefix": "X=", "unit": "coordinate", "value": 120.456}, {"unit": "cm", "decimals": 0}, metres),
        ("Koordinatın eksi değeri", {"prefix": "Y=", "unit": "coordinate", "value": -0.0004}, {"decimals": 3}, metres),
        ("Eğim: basamak yoksa 2", {"prefix": "%", "unit": "percent", "value": 1.255}, {"suffix": " eğim"}, metres),
        ("Eğim dört basamak; birim eğime uygulanmaz", {"prefix": "%", "unit": "percent", "value": 1.25549}, {"decimals": 4, "unit": "mm"}, metres),
    ]:
        add(name, "dimensionValue", {"measured": measured, "look": look, "project": project}, dimension_value(measured, look, project))
    return out


def main():
    doc = {
        "format": "kentos.annotation-style-cases",
        "version": 1,
        "title": "Yazı ve ölçü stilleri: tabloların kuralı, yüz ve görünüş denetimi, stili uygulamak ve stile uymak",
        "note": "ADR 0183. scripts/fixtures/annotation_style_cases.py kurallardan yazar; sözleşmenin (annotation.rs) ve web'in (model/annotationStyles.ts) kuralları bunu geçer. Yükseklikler metrede (kâğıtta mm / 1000 × ölçek), ölçünün boyları değer yüksekliğinin katı.",
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
