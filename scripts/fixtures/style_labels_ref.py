"""Stil dosyalarında etiketlerin ortak kuralları (docs/adr/0635 §1), KentOS kodu olmadan: sınıfların metni (alanlar ve
aralarındaki yazılar), okunan sınıfların KentOS'un etiketlemesi olması, ölçeğin piksel/metre ile ölçek paydası arasındaki
çevirisi. SLD'nin, QML'in ve MapLibre'nin kendi sözcükleri sld_cases.py, qml_cases.py ve maplibre_cases.py'dedir; bu modül
onların ortak parçasıdır.

Kurallar ADR'den: tek etiket koşulsuz ve “Etiket” adlı tek sınıf olarak yazılır ve öyle okunan tek sınıf yine tek etikettir;
öbür sınıflar kurallı etiketlemedir, adları birer kez (ikinci “Ad” “Ad (2)”). Etiketin boyu CSS pikselidir. Ölçek etiket
motorununki (ekranda metre başına piksel), dosyanınki 96 dpi'de (0,00026458 m/piksel) ölçek paydası: payda dört basamakla,
geri okunan ölçek beş anlamlı basamakla.
"""

from __future__ import annotations

import math

SINGLE = "Etiket"
METRES_PER_PX = 0.00026458


def rnd(x: float) -> float:
    """Rust's `round`: half away from zero."""
    return math.copysign(math.floor(abs(x) + 0.5), x)


def r3(v: float) -> float:
    return rnd(v * 1000.0) / 1000.0


def round4(v: float) -> float:
    return rnd(v * 10_000.0) / 10_000.0


def denominator(ppm: float) -> float:
    return rnd((1.0 / (ppm * METRES_PER_PX)) * 10_000.0) / 10_000.0


def ppm(n: float) -> float:
    v = 1.0 / (n * METRES_PER_PX)
    if not math.isfinite(v) or v <= 0.0:
        return v
    digits = 4 - math.floor(math.log10(v))
    f = 10.0 ** digits
    return rnd(v * f) / f


def template(t: str):
    """A template's pieces: `{field}` a field, the rest words."""
    out = []
    rest = t
    while "{" in rest:
        o = rest.index("{")
        c = rest.find("}", o)
        if c < 0:
            break
        if o > 0:
            out.append(("text", rest[:o]))
        field = rest[o + 1:c].strip()
        if field:
            out.append(("field", field))
        rest = rest[c + 1:]
    if rest:
        out.append(("text", rest))
    return out


KEYWORDS = {"ve", "veya", "değil", "boş", "gibi", "arasında", "and", "or", "not", "is", "null", "like", "between",
            "doğru", "yanlış", "true", "false"}


def bare_field(w: str, keywords) -> bool:
    return w.lower() not in keywords


def concatenation(src: str, qgis: bool, keywords=KEYWORDS):
    """Fields and texts joined by `||`: KentOS's (`[a]` a field, `'t'` or `"t"` a text) or QGIS's (`"a"` a field,
    `'t'` a text; a bare word a field in both); or None."""
    c = list(src)
    out = []
    i = 0
    want = True

    def quoted(i, q):
        s = []
        j = i + 1
        while True:
            if j >= len(c):
                return None
            x = c[j]
            if x == q:
                if j + 1 < len(c) and c[j + 1] == q:
                    s.append(q)
                    j += 2
                    continue
                return "".join(s), j + 1
            s.append(x)
            j += 1

    while i < len(c):
        ch = c[i]
        if ch.isspace():
            i += 1
            continue
        if not want:
            if ch == "|" and i + 1 < len(c) and c[i + 1] == "|":
                want = True
                i += 2
                continue
            return None
        if not qgis and ch == "[":
            try:
                j = c.index("]", i + 1)
            except ValueError:
                return None
            name = "".join(c[i + 1:j]).strip()
            if not name:
                return None
            out.append(("field", name))
            i = j + 1
        elif ch == "'" or (not qgis and ch == '"'):
            q = quoted(i, ch)
            if q is None:
                return None
            out.append(("text", q[0]))
            i = q[1]
        elif qgis and ch == '"':
            q = quoted(i, '"')
            if q is None or not q[0]:
                return None
            out.append(("field", q[0]))
            i = q[1]
        elif ch.isalpha() or ch == "_":
            j = i + 1
            while j < len(c) and (c[j].isalnum() or c[j] == "_"):
                j += 1
            w = "".join(c[i:j])
            nxt = next((x for x in c[j:] if not x.isspace()), None)
            if not bare_field(w, keywords) or nxt == "(":
                return None
            out.append(("field", w))
            i = j
        else:
            return None
        want = False
    return out if out and not want else None


def kentos_text(pieces) -> str:
    def q(s):
        return "'" + s.replace("'", "''") + "'"
    return " || ".join(f"[{v}]" if k == "field" else q(v) for k, v in pieces)


def style_in(size: float, text: str, more: dict) -> dict:
    point, line, area = more.get("point"), more.get("line"), more.get("area")
    if line is not None and point is None and area is None:
        placement = "along"
    elif point == "center" or (point is None and area is not None):
        placement = "center"
    else:
        placement = "beside"
    s = {"placement": placement, "size": float(size), "text": text}
    s.update(more)
    return s


MAX_CLASSES = 64
HEAT_RAMP = ["#2b83ba00", "#2b83ba", "#abdda4", "#ffffbf", "#fdae61", "#d7191c"]


def heat_ramp(ramp, what, lost):
    """A heat map's ramp as KentOS holds it: none the ready-made Isı ramp, one colour twice, the first 16 of more."""
    if not ramp:
        lost.append(f"{what}: renk rampası yok; KentOS'un Isı rampası alındı")
        return list(HEAT_RAMP)
    if len(ramp) == 1:
        return ramp + ramp
    if len(ramp) > 16:
        lost.append(f"{what}: rampanın {len(ramp)} renginden ilk 16'sı alındı")
        return ramp[:16]
    return ramp


def num(v: float) -> str:
    if math.isfinite(v) and v == math.trunc(v) and abs(v) < 1e15:
        return str(int(v))
    return repr(v)


RANGES = [("size", "boyu", 1.0, 200.0), ("distance", "uzaklığı", 0.0, 500.0), ("repeat", "yinelemesi", 20.0, 100_000.0),
          ("maxAngle", "harfler arası en çok açısı", 5.0, 90.0), ("minFeaturePx", "en küçük nesne boyu", 0.0, 100_000.0),
          ("minScale", "en küçük ölçeği", 0.0, 1e9), ("maxScale", "en büyük ölçeği", 0.0, 1e9)]


def colour_fit(c: str):
    if not c.startswith("#"):
        return None
    h = c[1:]
    if not h or not all(x in "0123456789abcdefABCDEF" for x in h):
        return None
    if len(h) == 3:
        return "#" + "".join(x + x for x in h).lower()
    if len(h) in (6, 8):
        return c.lower()
    return None


def fit_number(m, key, word, lo, hi, what, lost):
    v = m.get(key)
    if not isinstance(v, (int, float)) or isinstance(v, bool):
        return
    if lo <= v <= hi:
        return
    c = min(max(v, lo), hi) if math.isfinite(v) else lo
    m[key] = float(c)
    lost.append(f"{what}: {word} {num(v)} KentOS'un aralığına ({num(lo)}–{num(hi)}) çekildi")


def class_in(name, when, size, text, more, lost):
    style = style_in(size, text, more)
    what = f"“{name}” etiketi"
    for key, word, lo, hi in RANGES:
        fit_number(style, key, word, lo, hi, what, lost)
    c = style.get("color")
    if isinstance(c, str):
        f = colour_fit(c)
        if f is None:
            del style["color"]
            lost.append(f"{what}: “{c}” rengi okunamadı; alınmadı")
        else:
            style["color"] = f
    h = style.get("halo")
    if isinstance(h, dict):
        fit_number(h, "width", "halesinin genişliği", 0.0, 10.0, what, lost)
        c = h.get("color")
        if isinstance(c, str):
            f = colour_fit(c)
            if f is None:
                del h["color"]
                lost.append(f"{what}: halesinin “{c}” rengi okunamadı; alınmadı")
            else:
                h["color"] = f
    return {"name": name, "when": when, "style": style}


def labelling(classes, lost):
    """(label, labels, count) the read classes give, or None: classes are dicts of name, when, style."""
    if not classes:
        return None
    if len(classes) > MAX_CLASSES:
        lost.append(f"{len(classes)} etiket sınıfından ilk {MAX_CLASSES}'ü alındı")
        classes = classes[:MAX_CLASSES]
    if len(classes) == 1 and classes[0]["when"] is None and classes[0]["name"] == SINGLE:
        return classes[0]["style"], {"mode": "single"}, 1
    seen, out = [], []
    for c in classes:
        base = c["name"].strip()[:90]
        if not base:
            base = f"Etiket {len(out) + 1}"
        name, k = base, 2
        while name in seen:
            name = f"{base} ({k})"
            k += 1
        seen.append(name)
        v = {"name": name, "style": c["style"]}
        if c["when"] is not None:
            v["when"] = c["when"]
        out.append(v)
    return None, {"mode": "rules", "classes": out}, len(out)


def families_lost(families):
    return (f"etiketin yazı tipi ({', '.join(families)}) alınmadı; KentOS etiketleri projenin yazı tipiyle "
            "çizer")


def apply_to_style(style: dict, got) -> dict:
    """A layer's style after the labelling `got` (label, labels, count) is written."""
    label, labels, _ = got
    if label is None:
        style.pop("label", None)
    else:
        style["label"] = label
    style["labels"] = labels
    return style


def output_of(got) -> dict:
    """The command's `labels` and `label` for `got`."""
    if got is None:
        return {}
    label, labels, _ = got
    out = {"labels": labels}
    if label is not None:
        out["label"] = label
    return out
