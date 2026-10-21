#!/usr/bin/env python3
"""QML'den görünüş al'ın ortak durumları ve QML'in QGIS 4.2 ile iki yönlü denetimi (docs/adr/0555, 0565, 0571), KentOS kodu olmadan.

    python3 scripts/fixtures/qml_cases.py --files    # QGIS 4.2'nin QML'lerini ve elle yazılanları fixtures/qml/v1'e yazar
    python3 scripts/fixtures/qml_cases.py            # fixtures/commands/v1/file.import.qml.json'u yazar
    python3 scripts/fixtures/qml_cases.py --check    # yazılı durumlar güncel mi; KentOS'un QML'leri QGIS'te ne oluyor

Okuma kuralları ADR'den, bu betikte yeniden yazılmıştır (xml.etree ile): QGIS'in tek simge, kategorili, aralıklı ve
kurallı görünüşü KentOS'un aynı türü; basit dolgu (kenarı ayrı çizgi), basit çizgi (Qt'nin kesik desenleri ya da özel
kesikler), basit işaret, merkez dolgusu, çizgi deseni (tarama) ve işaretli çizgi; veriye bağlı kalınlık, boy ve dönüş; renkler `r,g,b,a`'dan #rrggbb(aa); mm, nokta, inç, piksel ve harita birimi;
kuralların süzgeci QGIS'in ifadesinden KentOS'unkine (bu betiğin kendi ayrıştırıcısıyla); ADR 0571'den degrade (doğrusal,
küre, silindir), nokta deseni, SVG işareti (gömülü `base64:`), yazı işareti ve renklerin veriye bağlı değerleri. Alınamayanların
cümleleri de ADR'nin sözleridir. `--files` QGIS'in dosyalarını yeni renk ve kimliklerle yeniden yazar: var olanlar git'ten geri alınır.

KentOS'un yazdığı QML'leri (`KENTOS_WRITE_QML=1 cargo test -p kentos-headless --test headless qml` yazar,
fixtures/qml/v1/out) QGIS 4.2 `loadNamedStyle` ile açar: görünüşün türü, sınıflar, süzgeçler, renkler, kalınlıklar,
birimler ve saydamlık beklenenle denetlenir.
"""

from __future__ import annotations

import base64
import json
import math
import os
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/qml/v1"
CASES = ROOT / "fixtures/commands/v1/file.import.qml.json"
QGIS = Path("/Applications/QGIS-final-4_2_2.app/Contents")
POINT_MM = 25.4 / 72.0

SHAPES = [("circle", "circle"), ("square", "square"), ("diamond", "diamond"), ("triangle", "triangle"),
          ("pentagon", "pentagon"), ("hexagon", "hexagon"), ("octagon", "octagon"), ("star", "star"),
          ("cross", "cross"), ("x", "cross2"), ("line", "line"), ("arrow", "arrow"), ("arrowhead", "filled_arrowhead"),
          ("chevron", "arrowhead"), ("semicircle", "semi_circle")]
SHAPES_IN = {"equilateral_triangle": "triangle", "quarter_circle": "quartercircle", "cross_fill": "cross"}

# ─── QGIS's expressions and KentOS's, from the ADR ─────────────────────────────────────────────────────

KEYWORDS = {"ve", "veya", "değil", "boş", "gibi", "arasında", "and", "or", "not", "is", "null", "like", "between",
            "doğru", "yanlış", "true", "false"}


def bare_field(w: str) -> bool:
    return bool(w) and (w[0].isalpha() or w[0] == "_") and all(c.isalnum() or c == "_" for c in w) \
        and w.lower() not in KEYWORDS


def tokens(src: str):
    """QGIS's tokens: "x" a field, 'x' a text."""
    out, i, c = [], 0, src
    while i < len(c):
        ch = c[i]
        if ch.isspace():
            i += 1
        elif ch in "'\"":
            q, s, j = ch, "", i + 1
            while True:
                if j >= len(c):
                    return None
                if c[j] == q:
                    if j + 1 < len(c) and c[j + 1] == q:
                        s += q
                        j += 2
                        continue
                    break
                s += c[j]
                j += 1
            out.append(("field", s) if q == '"' else ("text", s))
            i = j + 1
        elif ch.isdigit() or (ch in "-." and i + 1 < len(c) and c[i + 1].isdigit()):
            j = i + 1
            while j < len(c) and (c[j].isdigit() or c[j] in ".eE"):
                j += 1
            try:
                out.append(("num", float(c[i:j])))
            except ValueError:
                return None
            i = j
        elif ch == "(":
            out.append(("open",))
            i += 1
        elif ch == ")":
            out.append(("close",))
            i += 1
        elif ch == ",":
            out.append(("comma",))
            i += 1
        elif ch in "<>=!":
            two = c[i:i + 2]
            if two in ("<=", ">=", "<>", "!=", "=="):
                out.append(("op", {"<=": "<=", ">=": ">=", "<>": "<>", "!=": "<>", "==": "="}[two]))
                i += 2
            elif ch in "<>=":
                out.append(("op", ch))
                i += 1
            else:
                return None
        elif ch.isalpha() or ch == "_":
            j = i + 1
            while j < len(c) and (c[j].isalnum() or c[j] == "_"):
                j += 1
            w = c[i:j]
            if w.lower() == "in":
                out.append(("word", "in"))
            elif bare_field(w):
                out.append(("field", w))
            else:
                out.append(("word", w.lower()))
            i = j
        else:
            return None
    return out


def parse_qgis(src: str):
    t = tokens(src)
    if t is None:
        return None
    at = [0]

    def get(k=0):
        return t[at[0] + k] if at[0] + k < len(t) else None

    def word(tok, words):
        return tok is not None and tok[0] == "word" and tok[1] in words

    def lit(tok):
        if tok is None or tok[0] not in ("num", "text"):
            return None
        return tok

    def or_of():
        parts = [and_of()]
        if parts[0] is None:
            return None
        while word(get(), ("veya", "or")):
            at[0] += 1
            p = and_of()
            if p is None:
                return None
            parts.append(p)
        return parts[0] if len(parts) == 1 else ("or", parts)

    def and_of():
        parts = [not_of()]
        if parts[0] is None:
            return None
        while word(get(), ("ve", "and")):
            at[0] += 1
            p = not_of()
            if p is None:
                return None
            parts.append(p)
        return parts[0] if len(parts) == 1 else ("and", parts)

    def not_of():
        tok = get()
        if word(tok, ("değil", "not")):
            at[0] += 1
            f = not_of()
            return None if f is None else ("not", f)
        if tok == ("open",):
            at[0] += 1
            f = or_of()
            if f is None or get() != ("close",):
                return None
            at[0] += 1
            return f
        if tok is None or tok[0] != "field":
            return None
        name = tok[1]
        at[0] += 1
        nxt = get()
        if nxt is not None and nxt[0] == "op":
            at[0] += 1
            li = lit(get())
            if li is None:
                return None
            at[0] += 1
            return ("cmp", nxt[1], name, li)
        if word(nxt, ("boş", "is")):
            at[0] += 1
            if nxt[1] == "is":
                if word(get(), ("null",)):
                    at[0] += 1
                    return ("null", name)
                if word(get(), ("not",)) and word(get(1), ("null",)):
                    at[0] += 2
                    return ("not", ("null", name))
                return None
            if word(get(), ("değil",)):
                at[0] += 1
                return ("not", ("null", name))
            return ("null", name)
        if word(nxt, ("gibi", "like")):
            at[0] += 1
            p = get()
            if p is None or p[0] != "text":
                return None
            at[0] += 1
            return ("like", name, p[1])
        if word(nxt, ("in",)):
            at[0] += 1
            if get() != ("open",):
                return None
            at[0] += 1
            parts = []
            while True:
                li = lit(get())
                if li is None:
                    return None
                parts.append(("cmp", "=", name, li))
                at[0] += 1
                sep = get()
                if sep == ("comma",):
                    at[0] += 1
                elif sep == ("close",):
                    at[0] += 1
                    break
                else:
                    return None
            return parts[0] if len(parts) == 1 else ("or", parts)
        if word(nxt, ("arasında", "between")):
            at[0] += 1
            a, sep, b = get(), get(1), get(2)
            if a is None or a[0] != "num" or not word(sep, ("ve", "and")) or b is None or b[0] != "num":
                return None
            at[0] += 3
            return ("between", name, a[1], b[1])
        return None

    f = or_of()
    return f if f is not None and at[0] == len(t) else None


def num(v: float) -> str:
    if v == math.trunc(v) and abs(v) < 1e15:
        return str(int(v))
    return repr(v)


def lit_text(li):
    return num(li[1]) if li[0] == "num" else "'" + li[1].replace("'", "''") + "'"


def kentos_text(f) -> str:
    k = f[0]
    if k == "cmp":
        return f"[{f[2]}] {f[1]} {lit_text(f[3])}"
    if k == "between":
        return f"[{f[1]}] arasında {num(f[2])} ve {num(f[3])}"
    if k == "null":
        return f"[{f[1]}] boş"
    if k == "like":
        return f"[{f[1]}] gibi '" + f[2].replace("'", "''") + "'"
    if k == "and":
        return " ve ".join(f"({kentos_text(p)})" if p[0] == "or" else kentos_text(p) for p in f[1])
    if k == "or":
        return " veya ".join(f"({kentos_text(p)})" if p[0] == "and" else kentos_text(p) for p in f[1])
    inner = f[1]
    if inner[0] == "null":
        return f"[{inner[1]}] boş değil"
    return f"değil ({kentos_text(inner)})"


# ─── The reader, from the ADR ──────────────────────────────────────────────────────────────────────────


def child(n, name):
    for c in n:
        if c.tag == name:
            return c
    return None


def props(layer):
    m = {}
    opt = child(layer, "Option")
    if opt is not None:
        for o in opt:
            if o.get("name") is not None and o.get("value") is not None:
                m[o.get("name")] = o.get("value")
    for p in layer:
        if p.tag == "prop" and p.get("k") is not None and p.get("v") is not None:
            m[p.get("k")] = p.get("v")
    return m


def fnum(s):
    try:
        return float(s.strip())
    except (AttributeError, ValueError):
        return None


def round3(v: float) -> float:
    x = v * 1000.0
    return math.copysign(math.floor(abs(x) + 0.5), x) / 1000.0


def colour_in(text):
    if text is None:
        return None
    t = text.strip()
    if t.startswith("#"):
        h = t[1:].lower()
        return "#" + h[:6] + (h[6:8] if len(h) == 8 and h[6:8] != "ff" else "")
    parts = []
    for x in t.split(",")[:4]:
        try:
            v = int(x.strip())
        except ValueError:
            continue
        if 0 <= v <= 255:
            parts.append(v)
    if len(parts) < 3:
        return None
    a = parts[3] if len(parts) > 3 else 255
    if a == 0:
        return None
    rgb = "#%02x%02x%02x" % tuple(parts[:3])
    return rgb if a == 255 else rgb + "%02x" % a


def size_in(v, unit):
    u = unit or "MM"
    if u == "MM":
        return v, "mm"
    if u == "Point":
        return v * POINT_MM, "mm"
    if u == "Inch":
        return v * 25.4, "mm"
    if u == "Pixel":
        return v, "px"
    if u in ("MapUnit", "RenderMetersInMapUnits"):
        return v, "m"
    return None


def size_as(v, unit, to):
    got = size_in(v, unit)
    return got[0] if got is not None and got[1] == to else None


PLACEMENTS = [("interval", "Interval"), ("vertex", "Vertex"), ("innerVertex", "InnerVertices"),
              ("first", "FirstVertex"), ("last", "LastVertex"), ("center", "CentralPoint"),
              ("segmentCenter", "SegmentCenter")]


def value_tokens(src, qgis):
    """A value expression's tokens: fields, numbers, + - * / % and parentheses; anything else says nothing."""
    c, out, i = src, [], 0
    while i < len(c):
        ch = c[i]
        if ch.isspace():
            i += 1
        elif not qgis and ch == "[":
            j = c.find("]", i + 1)
            if j < 0:
                return None
            name = c[i + 1:j].strip()
            if not name:
                return None
            out.append(("field", name))
            i = j + 1
        elif qgis and ch == '"':
            name, j = "", i + 1
            while True:
                if j >= len(c):
                    return None
                if c[j] == '"':
                    if j + 1 < len(c) and c[j + 1] == '"':
                        name += '"'
                        j += 2
                        continue
                    break
                name += c[j]
                j += 1
            out.append(("field", name))
            i = j + 1
        elif ch in "0123456789" or (ch == "." and i + 1 < len(c) and c[i + 1] in "0123456789"):
            j = i + 1
            while j < len(c) and (c[j] in "0123456789."):
                j += 1
            try:
                float(c[i:j])
            except ValueError:
                return None
            out.append(("num", c[i:j]))
            i = j
        elif ch in "+-*/%":
            out.append(("op", ch))
            i += 1
        elif ch == "(":
            out.append(("open",))
            i += 1
        elif ch == ")":
            out.append(("close",))
            i += 1
        elif ch.isalpha() or ch == "_":
            j = i + 1
            while j < len(c) and (c[j].isalnum() or c[j] == "_"):
                j += 1
            w = c[i:j]
            if c[j:].lstrip()[:1] == "(" or not bare_field(w):
                return None
            out.append(("field", w))
            i = j
        else:
            return None
    return out or None


def value_text(toks, field):
    s, prev, sign = "", None, False
    for t in toks:
        if not (prev is None or prev == ("open",) or t == ("close",) or sign):
            s += " "
        sign = t[0] == "op" and t[1] in "-+" and (prev is None or prev[0] == "op" or prev == ("open",))
        if t[0] == "field":
            s += field(t[1])
        elif t[0] in ("num", "op"):
            s += t[1]
        elif t[0] == "open":
            s += "("
        else:
            s += ")"
        prev = t
    return s


def kentos_value(q):
    toks = value_tokens(q, True)
    return None if toks is None else value_text(toks, lambda n: f"[{n}]")


def negated(e):
    if e.startswith("-(") and e.endswith(")"):
        inner, depth, ok = e[2:-1], 0, True
        for ch in inner:
            depth += {"(": 1, ")": -1}.get(ch, 0)
            if depth < 0:
                ok = False
                break
        if ok and depth == 0:
            return inner
    return f"-({e})"


def dd_props(layer):
    """The layer's active data-defined properties in order: name and QGIS expression (a field is "field")."""
    d = child(layer, "data_defined_properties")
    m = child(d, "Option") if d is not None else None
    props_ = next((c for c in m if c.get("name") == "properties"), None) if m is not None else None
    out = []
    if props_ is None:
        return out
    for p in props_:
        get = lambda k: next((c.get("value") for c in p if c.get("name") == k), None)  # noqa: E731
        if get("active") != "true":
            continue
        t, f, e = get("type"), get("field"), get("expression")
        if t == "2" and f is not None:
            q = '"' + f.replace('"', '""') + '"'
        elif e is not None:
            q = e
        elif f is not None:
            q = '"' + f.replace('"', '""') + '"'
        else:
            continue
        out.append((p.get("name") or "", q))
    return out


def take_dd(layer, supported, taken, lost, what):
    for name, q in dd_props(layer):
        found = next((s for s in supported if s[0] == name), None)
        if found is None:
            lost.append(f"{what}: QGIS'in “{name}” veriye bağlı değeri KentOS'un bu katmanında yok; sabit değeri alındı")
            continue
        e = kentos_value(q)
        if e is None:
            lost.append(f"{what}: “{name}” değerinin “{q}” ifadesi KentOS'un ifadesine çevrilemedi; sabit değeri alındı")
            continue
        _, kind, key, neg = found
        target = next((t for t in taken if t.get("type") == kind), None)
        if target is None:
            lost.append(f"{what}: QGIS'in “{name}” veriye bağlı değerinin katmanı alınmadı; değer alınmadı")
            continue
        expr = negated(e) if neg else e
        target[key] = {"expr": expr, "fallback": target[key]} if target.get(key) is not None else {"expr": expr}


def dd_names(layer):
    return [n for n, _ in dd_props(layer)]


def nonzero_pair(text):
    if text is None:
        return False
    for x in text.split(","):
        v = fnum(x)
        if v is not None and v != 0.0:
            return True
    return False


def read_line(m, ident, lost, what, dd_color=False):
    style = m.get("line_style", "solid")
    if style == "no":
        return None
    color = colour_in(m.get("line_color", m.get("color")))
    if color is None and not dd_color:
        return None
    lw = fnum(m.get("line_width"))
    got = size_in(0.26 if lw is None else lw, m.get("line_width_unit"))
    if got is None:
        lost.append(f"{what}: çizginin yüzde boyu KentOS'ta yok; 0,26 mm alındı")
        got = (0.26, "mm")
    width, unit = got
    layer = {"id": ident, "type": "simpleLine"}
    if color is not None:
        layer["color"] = color
    layer["width"] = width
    layer["unit"] = unit
    if m.get("use_custom_dash") == "1":
        raw = m.get("customdash", "")
        d = []
        for p in raw.split(";"):
            v = fnum(p)
            if v is None:
                continue
            w = size_as(v, m.get("customdash_unit"), unit)
            if w is not None:
                d.append(w)
        if not d and raw != "":
            lost.append(f"{what}: kesiklerin birimi çizginin biriminden başka; kesikler alınmadı")
        dash = d or None
    else:
        w = width if width > 0 else 1.0
        pattern = {"dash": [4, 2], "dot": [1, 2], "dash dot": [4, 2, 1, 2], "dash dot dot": [4, 2, 1, 2, 1, 2]}.get(style)
        dash = [round3(p * w) for p in pattern] if pattern else None
    if dash:
        layer["dash"] = dash
    layer["cap"] = {"round": "round", "flat": "butt"}.get(m.get("capstyle", "square"), "square")
    layer["join"] = {"round": "round", "miter": "miter"}.get(m.get("joinstyle", "bevel"), "bevel")
    ring = m.get("ring_filter")
    if ring == "1":
        layer["rings"] = "exterior"
    elif ring == "2":
        layer["rings"] = "interior"
    off = fnum(m.get("offset"))
    if off is not None and off != 0.0:
        lost.append(f"{what}: çizginin paralel kaydırması alınmadı")
    a, b = fnum(m.get("trim_distance_start")), fnum(m.get("trim_distance_end"))
    if (a is not None and a != 0.0) or (b is not None and b != 0.0):
        lost.append(f"{what}: çizginin uçlardan kırpması alınmadı")
    return layer


def read_marker(m, lost, what):
    name = m.get("name", "circle")
    shape = next((k for k, q in SHAPES if q == name), None) or SHAPES_IN.get(name)
    if shape is None:
        lost.append(f"{what}: “{name}” işareti KentOS'ta yok; kare alındı")
        shape = "square"
    sz = fnum(m.get("size"))
    got = size_in(2.0 if sz is None else sz, m.get("size_unit"))
    if got is None:
        lost.append(f"{what}: işaretin yüzde boyu KentOS'ta yok; 2 mm alındı")
        got = (2.0, "mm")
    size, unit = got
    v = {"id": "p", "type": "shape", "shape": shape, "size": size, "unit": unit, "fill": colour_in(m.get("color"))}
    if m.get("outline_style") != "no":
        stroke = colour_in(m.get("outline_color"))
        if stroke is not None:
            v["stroke"] = stroke
            ow = fnum(m.get("outline_width"))
            w = size_as(0.0 if ow is None else ow, m.get("outline_width_unit"), unit)
            if w is None:
                lost.append(f"{what}: işaretin çizgi kalınlığının birimi boyununkinden başka; kalınlık alınmadı")
            else:
                v["strokeWidth"] = w
    angle = fnum(m.get("angle"))
    if angle is not None and angle != 0.0:
        v["rotation"] = -angle
    if nonzero_pair(m.get("offset")):
        lost.append(f"{what}: işaretin kaydırması alınmadı")
    if m.get("horizontal_anchor_point", "1") != "1" or m.get("vertical_anchor_point", "1") != "1":
        lost.append(f"{what}: işaretin bağlantı noktası alınmadı")
    if m.get("scale_method") == "area":
        lost.append(f"{what}: işaretin alana göre ölçeklenmesi alınmadı")
    return v


def marker_pose(m, v, lost, what):
    angle = fnum(m.get("angle"))
    if angle is not None and angle != 0.0:
        v["rotation"] = -angle
    if nonzero_pair(m.get("offset")):
        lost.append(f"{what}: işaretin kaydırması alınmadı")
    if m.get("horizontal_anchor_point", "1") != "1" or m.get("vertical_anchor_point", "1") != "1":
        lost.append(f"{what}: işaretin bağlantı noktası alınmadı")


ASSETS = []


def asset_id(prefix, svg):
    h = 0xcbf29ce484222325
    for b in svg.encode("utf-8"):
        h ^= b
        h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return f"{prefix}-{h:016x}"


def looks_like_svg(t):
    head = t.lower()
    i = head.find("<svg")
    return i >= 0 and (len(head) == i + 4 or head[i + 4] in " \t\r\n>/")


def read_svg_marker(m, lost, what):
    name = m.get("name", "")
    enc = None
    for pre in ("base64:", "data:image/svg+xml;base64,"):
        if name.startswith(pre):
            enc = name[len(pre):]
            break
    svg = None
    if enc is not None:
        clean = "".join(enc.split()).replace("=", "")
        try:
            t = base64.b64decode(clean + "=" * (-len(clean) % 4), validate=True).decode("utf-8", "replace")
            svg = t if looks_like_svg(t) else None
        except Exception:
            svg = None
    if svg is None:
        lost.append(f"{what}: SVG işareti “{name[:60]}” dosyasına bağlı; KentOS çizimin içindeki SVG'yi alır, dosyayı okumaz; işaret alınmadı")
        return None
    ident = asset_id("qml", svg)
    if not any(a["id"] == ident for a in ASSETS):
        ASSETS.append({"id": ident, "name": "QGIS'ten gelen SVG işareti", "svg": svg})
    sz = fnum(m.get("size"))
    got = size_in(2.0 if sz is None else sz, m.get("size_unit"))
    if got is None:
        lost.append(f"{what}: işaretin yüzde boyu KentOS'ta yok; 2 mm alındı")
        got = (2.0, "mm")
    size, unit = got
    v = {"id": "v", "type": "svg", "asset": ident, "size": size, "unit": unit}
    c = colour_in(m.get("color"))
    if c is not None:
        v["fill"] = c
    c = colour_in(m.get("outline_color"))
    if c is not None:
        v["stroke"] = c
    marker_pose(m, v, lost, what)
    return v


def read_font_marker(m, lost, what):
    sz = fnum(m.get("size"))
    got = size_in(2.0 if sz is None else sz, m.get("size_unit"))
    if got is None:
        lost.append(f"{what}: işaretin yüzde boyu KentOS'ta yok; 2 mm alındı")
        got = (2.0, "mm")
    size, unit = got
    v = {"id": "t", "type": "text", "text": m.get("chr", ""), "size": size, "unit": unit}
    if m.get("font"):
        v["font"] = m["font"]
    style = m.get("font_style", "").lower()
    if "bold" in style:
        v["weight"] = 700
    if "italic" in style or "oblique" in style:
        v["italic"] = True
    c = colour_in(m.get("color"))
    if c is not None:
        v["color"] = c
    ow = fnum(m.get("outline_width"))
    c = colour_in(m.get("outline_color"))
    if ow is not None and ow > 0 and c is not None:
        w = size_as(ow, m.get("outline_width_unit"), unit)
        if w is None:
            lost.append(f"{what}: yazı işaretinin çizgi kalınlığının birimi boyununkinden başka; hale alınmadı")
        else:
            v["halo"] = {"color": c, "width": w}
    marker_pose(m, v, lost, what)
    return v


def pair_of(m, key, default):
    t = m.get(key)
    if t is None:
        return default
    parts = [fnum(x) for x in t.split(",")]
    if len(parts) < 2 or parts[0] is None or parts[1] is None:
        return default
    return (parts[0], parts[1])


def read_gradient(layer, m, i, lost, what):
    p1 = pair_of(m, "reference_point1", (0.5, 0.0))
    p2 = pair_of(m, "reference_point2", (0.5, 1.0))
    kind = m.get("type", m.get("gradient_type", "0"))
    spread = m.get("spread", "0")
    c1, c2 = colour_in(m.get("color")), colour_in(m.get("gradient_color2"))
    if m.get("color_type") == "1":
        ramp = child(layer, "colorramp")
        r = props(ramp) if ramp is not None else m
        a, b = colour_in(r.get("color1")), colour_in(r.get("color2"))
        if a is not None and b is not None:
            c1, c2 = a, b
            if r.get("stops"):
                lost.append(f"{what}: degradenin renk rampasının ara durakları alınmadı; ilk ve son rengi alındı")
        else:
            lost.append(f"{what}: degradenin renk rampası okunamadı; iki rengi alındı")

    def half(a, b):
        return a == (0.0, 0.5) and b == (0.5, 0.5)

    if kind == "1":
        shape = "spherical"
    elif kind == "2":
        lost.append(f"{what}: konik degrade KentOS'ta yok; doğrusal alındı")
        shape = "linear"
    elif spread == "1" and half(p1, p2):
        shape = "cylinder"
    else:
        shape = "linear"
    if shape != "cylinder" and spread != "0" and kind != "1":
        lost.append(f"{what}: degradenin yansıtma ve yineleme yayılımı KentOS'ta yok (yalnız kutunun yarısıyla yansıtma silindir olur); alınmadı")
    g = {"id": f"g{i}", "type": "gradientFill", "shape": shape}
    k1, k2 = ("color2", "color") if shape == "spherical" else ("color", "color2")
    if c1 is not None:
        g[k1] = c1
    if c2 is not None:
        g[k2] = c2
    takes = [("fillColor", "gradientFill", k1, False), ("color2", "gradientFill", k2, False)]
    if shape != "spherical":
        along = math.degrees(math.atan2(-(p2[1] - p1[1]), p2[0] - p1[0]))
        a = fnum(m.get("angle"))
        angle = round3((0.0 if a is None else a) + along) % 360.0
        angle = round3(angle)
        if angle != 0.0:
            g["angle"] = angle

        def standard(a, b):
            return ((a, b) in [((0.0, 0.5), (1.0, 0.5)), ((1.0, 0.5), (0.0, 0.5)), ((0.5, 0.0), (0.5, 1.0)),
                               ((0.5, 1.0), (0.5, 0.0))] or half(a, b))

        if not standard(p1, p2) or angle % 90.0 != 0.0:
            lost.append(f"{what}: degradenin uçları QGIS'te kutunun kesirleriyle verilir; KentOS'ta kutunun doğrultudaki izdüşümüdür; yön alındı, renk geçişinin uzunluğu kutuya oturtuldu")
    if m.get("coordinate_mode") == "1":
        lost.append(f"{what}: degradenin görünüm kutusuna göre koordinatı alınmadı (KentOS nesnenin kutusunu kullanır)")
    if nonzero_pair(m.get("offset")):
        lost.append(f"{what}: degradenin kaydırması alınmadı")
    return g, takes


def read_pattern(layer, m, i, lost, what):
    sub = child(layer, "symbol")
    got = read_symbol(sub, lost, what) if sub is not None else None
    if got is None or got[0] != "marker":
        lost.append(f"{what}: nokta deseninin işareti alınmadı")
        return None
    marker = got[1]
    dx = fnum(m.get("distance_x"))
    sx = size_in(15.0 if dx is None else dx, m.get("distance_x_unit"))
    if sx is None:
        lost.append(f"{what}: nokta deseninin yüzde aralığı KentOS'ta yok; desen alınmadı")
        return None
    spx, unit = sx
    dy = fnum(m.get("distance_y"))
    spy = size_as(15.0 if dy is None else dy, m.get("distance_y_unit"), unit)
    if spy is None:
        lost.append(f"{what}: nokta deseninin iki aralığının birimi ayrı; desen alınmadı")
        return None
    v = {"id": f"pt{i}", "type": "patternFill", "marker": marker, "spacingX": spx, "spacingY": spy, "unit": unit}

    def shift(k):
        x = fnum(m.get(k))
        return x if x is not None and x != 0.0 else None

    sdx, sdy = shift("displacement_x"), shift("displacement_y")
    if sdx is not None and sdy is None:
        d = size_as(sdx, m.get("displacement_x_unit"), unit)
        if d is not None and abs(d - spx / 2.0) < 1e-9:
            v["stagger"] = True
        else:
            lost.append(f"{what}: nokta deseninin kayması KentOS'un yarım aralık kaymasından başka; alınmadı")
    elif sdx is None and sdy is None:
        pass
    else:
        lost.append(f"{what}: nokta deseninin kayması KentOS'un yarım aralık kaymasından başka; alınmadı")
    a = fnum(m.get("angle"))
    if a is not None and a != 0.0:
        v["angle"] = -a
    ox = fnum(m.get("offset_x")) or 0.0
    oy = fnum(m.get("offset_y")) or 0.0
    if ox != 0.0 or oy != 0.0:
        x = size_as(ox, m.get("offset_x_unit"), unit)
        y = size_as(oy, m.get("offset_y_unit"), unit)
        if x is not None and y is not None:
            v["offset"] = [x, -y + 0.0]
        else:
            lost.append(f"{what}: nokta deseninin başlangıç kaymasının birimi aralığınkinden başka; alınmadı")
    for k in ("random_deviation_x", "random_deviation_y"):
        d = fnum(m.get(k))
        if d is not None and d != 0.0:
            lost.append(f"{what}: nokta deseninin rastgele sapması KentOS'un sapmasıyla aynı değil; alınmadı")
            break
    return v


def read_hatch(layer, m, i, lost, what):
    dist = fnum(m.get("distance"))
    got = size_in(5.0 if dist is None else dist, m.get("distance_unit"))
    if got is None:
        lost.append(f"{what}: çizgi deseninin yüzde aralığı KentOS'ta yok; tarama alınmadı")
        return None
    spacing, unit = got
    angle = fnum(m.get("angle"))
    if angle is None:
        angle = fnum(m.get("lineangle"))
    if angle is None:
        angle = 45.0
    sub = child(layer, "symbol")
    sym = read_symbol(sub, lost, what) if sub is not None else None
    if sym is None:
        lost.append(f"{what}: çizgi deseninin çizgisi alınmadı")
        return None
    k, body = sym
    first = body["layers"][0] if body["layers"] else None
    if k != "line" or first is None or first.get("type") != "simpleLine":
        lost.append(f"{what}: çizgi deseninin çizgisi basit çizgi değil; tarama alınmadı")
        return None
    if len(body["layers"]) > 1:
        lost.append(f"{what}: çizgi deseninin çizgisinin yalnız ilk katmanı alındı")
    h = {"id": f"h{i}", "type": "hatchFill", "angle": angle, "spacing": spacing, "unit": unit,
         "color": first.get("color", "#000000"), "width": 0.0}
    if first.get("unit") == unit:
        h["width"] = first.get("width", 0.0)
        if "dash" in first:
            h["dash"] = first["dash"]
    elif isinstance(first.get("width"), (int, float)) and first["width"] > 0:
        lost.append(f"{what}: çizgi deseninin çizgisinin birimi aralığınkinden başka; kalınlığı ve kesikleri alınmadı")
    off = fnum(m.get("offset"))
    if off is not None and off != 0.0:
        o = size_as(off, m.get("offset_unit"), unit)
        if o is None:
            lost.append(f"{what}: çizgi deseninin kaydırmasının birimi aralığınkinden başka; alınmadı")
        else:
            h["offset"] = o
    return h


def read_marker_line(layer, m, i, lost, what):
    names = m.get("placements", m.get("placement", "Interval"))
    placement = None
    for n in [x.strip() for x in names.replace(",", "|").split("|")]:
        if not n:
            continue
        found = next((k for k, q in PLACEMENTS if q.lower() == n.lower()), None)
        if found is not None and placement is None:
            placement = found
        else:
            lost.append(f"{what}: işaretli çizginin “{n}” yerleşimi alınmadı (KentOS'ta tek yerleşim)")
    if placement is None:
        lost.append(f"{what}: işaretli çizginin yerleşimi KentOS'ta yok; katman alınmadı")
        return None
    iv = fnum(m.get("interval"))
    got = size_in(3.0 if iv is None else iv, m.get("interval_unit"))
    if got is None:
        lost.append(f"{what}: işaretli çizginin yüzde aralığı KentOS'ta yok; katman alınmadı")
        return None
    interval, unit = got
    sub = child(layer, "symbol")
    sym = read_symbol(sub, lost, what) if sub is not None else None
    if sym is None or sym[0] != "marker":
        lost.append(f"{what}: işaretli çizginin işareti alınmadı")
        return None
    v = {"id": f"m{i}", "type": "markerLine", "marker": sym[1], "placement": placement,
         "interval": interval, "unit": unit, "rotate": m.get("rotate") != "0"}
    oa = fnum(m.get("offset_along_line"))
    if oa is not None and oa != 0.0:
        o = size_as(oa, m.get("offset_along_line_unit"), unit)
        if o is None:
            lost.append(f"{what}: işaretli çizginin baştan uzaklığının birimi aralığınkinden başka; alınmadı")
        else:
            v["offsetAlong"] = o
    ring = m.get("ring_filter")
    if ring == "1":
        v["rings"] = "exterior"
    elif ring == "2":
        v["rings"] = "interior"
    off = fnum(m.get("offset"))
    if off is not None and off != 0.0:
        lost.append(f"{what}: işaretli çizginin paralel kaydırması alınmadı")
    return v


def read_symbol(s, lost, what):
    key = s.get("type", "")
    if key not in ("fill", "line", "marker"):
        lost.append(f"{what}: “{key}” simgesi alınmadı")
        return None
    layers = []
    for i, layer in enumerate([c for c in s if c.tag == "layer"]):
        cls = layer.get("class", "")
        m = props(layer)
        names = dd_names(layer)
        taken = []
        supported = []
        if key == "fill" and cls == "SimpleFill":
            supported = [("fillColor", "simpleFill", "color", False), ("outlineColor", "simpleLine", "color", False),
                         ("outlineWidth", "simpleLine", "width", False)]
            style = m.get("style", "solid")
            if style == "solid":
                c = colour_in(m.get("color"))
                if c is not None or "fillColor" in names:
                    f = {"id": f"f{i}", "type": "simpleFill"}
                    if c is not None:
                        f["color"] = c
                    taken.append(f)
            elif style != "no":
                lost.append(f"{what}: “{style}” fırça deseni KentOS'un dolgusuna çevrilmedi")
            if m.get("outline_style", "solid") != "no":
                line = {k: m[src] for k, src in [("line_color", "outline_color"), ("line_width", "outline_width"),
                                                  ("line_width_unit", "outline_width_unit"),
                                                  ("line_style", "outline_style"), ("joinstyle", "joinstyle")]
                        if src in m}
                got = read_line(line, f"o{i}", lost, what, "outlineColor" in names)
                if got is not None:
                    taken.append(got)
            if nonzero_pair(m.get("offset")):
                lost.append(f"{what}: dolgunun kaydırması alınmadı")
        elif key in ("fill", "line") and cls == "SimpleLine":
            supported = [("outlineWidth", "simpleLine", "width", False), ("outlineColor", "simpleLine", "color", False)]
            got = read_line(m, f"l{i}", lost, what, "outlineColor" in names)
            if got is not None:
                taken.append(got)
        elif key == "marker" and cls == "SimpleMarker":
            supported = [("size", "shape", "size", False), ("angle", "shape", "rotation", True),
                         ("fillColor", "shape", "fill", False), ("outlineColor", "shape", "stroke", False)]
            taken.append(read_marker(m, lost, what))
        elif key == "marker" and cls == "SvgMarker":
            supported = [("size", "svg", "size", False), ("angle", "svg", "rotation", True),
                         ("fillColor", "svg", "fill", False), ("outlineColor", "svg", "stroke", False)]
            got = read_svg_marker(m, lost, what)
            if got is not None:
                taken.append(got)
        elif key == "marker" and cls == "FontMarker":
            supported = [("size", "text", "size", False), ("angle", "text", "rotation", True),
                         ("fillColor", "text", "color", False), ("char", "text", "text", False)]
            taken.append(read_font_marker(m, lost, what))
        elif key == "fill" and cls == "GradientFill":
            g, supported = read_gradient(layer, m, i, lost, what)
            taken.append(g)
        elif key == "fill" and cls == "PointPatternFill":
            got = read_pattern(layer, m, i, lost, what)
            if got is not None:
                taken.append(got)
        elif key == "fill" and cls == "LinePatternFill":
            got = read_hatch(layer, m, i, lost, what)
            if got is not None:
                taken.append(got)
        elif key in ("fill", "line") and cls == "MarkerLine":
            got = read_marker_line(layer, m, i, lost, what)
            if got is not None:
                taken.append(got)
        elif key == "fill" and cls == "CentroidFill":
            sub = child(layer, "symbol")
            got = read_symbol(sub, lost, what) if sub is not None else None
            if got is not None and got[0] == "marker":
                cm = {"id": f"c{i}", "type": "centroidMarker", "marker": got[1]}
                if m.get("point_on_surface") == "0":
                    cm["position"] = "centroid"
                taken.append(cm)
            else:
                lost.append(f"{what}: merkez işaretinin simgesi alınmadı")
        else:
            lost.append(f"{what}: QGIS'in “{cls}” simge katmanı KentOS'un katmanına çevrilmedi")
        take_dd(layer, supported, taken, lost, what)
        if layer.get("enabled") == "0":
            for t in taken:
                t["enabled"] = False
        layers += taken
    if not layers:
        return None
    v = {"type": key, "layers": layers}
    a = fnum(s.get("alpha"))
    if a is not None and a < 1.0:
        v["opacity"] = a
    return key, v


def set_of(sym):
    return {} if sym is None else {sym[0]: sym[1]}


def read_rules(parent, symbols, lost, count):
    out = []
    for r in [c for c in parent if c.tag == "rule"]:
        label = r.get("label") or f"Kural {count[0] + 1}"
        v = {"label": label}
        f = (r.get("filter") or "").strip()
        if f:
            if f.upper() == "ELSE":
                v["isElse"] = True
            else:
                parsed = parse_qgis(f)
                if parsed is None:
                    lost.append(f"“{label}” kuralının “{f}” süzgeci KentOS'un ifadesine çevrilemedi; kural ve alt "
                                "kuralları alınmadı")
                    continue
                v["filter"] = kentos_text(parsed)
        count[0] += 1
        v["id"] = f"r{count[0]}"
        if r.get("symbol") is not None:
            v["symbols"] = set_of(symbols.get(r.get("symbol")))
        for attr, key in (("scalemindenom", "minScale"), ("scalemaxdenom", "maxScale")):
            s = fnum(r.get(attr))
            if s is not None and s > 0:
                v[key] = s
        if r.get("checkstate") == "0":
            v["enabled"] = False
        kids = read_rules(r, symbols, lost, count)
        if kids:
            v["children"] = kids
        out.append(v)
    return out


def read(text: str):
    """(renderer, lost, rules) or the refusal's words."""
    try:
        root = ET.fromstring(text)
    except ET.ParseError:
        return "Dosya okunabilir bir XML değil."
    if root.tag != "qgis":
        return "Dosya bir QGIS stil dosyası (QML) değil: kök öğe <qgis> değil."
    r = child(root, "renderer-v2")
    if r is None:
        return "QML'de QGIS'in görünüşü (renderer-v2) yok."
    lost = []
    if child(root, "labeling") is not None or root.get("labelsEnabled") == "1":
        lost.append("etiketler alınmadı; KentOS'ta etiketler ayrıdır")
    op = child(root, "layerOpacity")
    if op is not None and fnum(op.text) is not None and fnum(op.text) < 1.0:
        lost.append("katmanın saydamlığı alınmadı (Katman çizimi ayrıdır)")
    symbols = {}
    lst = child(r, "symbols")
    if lst is not None:
        for s in [c for c in lst if c.tag == "symbol"]:
            name = s.get("name", "")
            got = read_symbol(s, lost, f"“{name}” simgesi")
            if got is not None:
                symbols[name] = got
    kind = r.get("type", "")

    def attr_field():
        a = (r.get("attr") or "").strip()
        bare = a[1:-1].replace('""', '"') if len(a) >= 2 and a[0] == '"' and a[-1] == '"' else a
        if bare and (a.startswith('"') or all(c.isalnum() or c == "_" for c in bare)):
            return bare
        lost.append(f"görünüşün “{a}” ifadesi bir alan değil; KentOS'un ifadesine çevrilemedi")
        return None

    if kind == "singleSymbol":
        first = next((c for c in lst if c.tag == "symbol"), None) if lst is not None else None
        sym = symbols.get(first.get("name")) if first is not None else None
        if sym is None:
            return "QML'in simgesi KentOS'un çizebileceği bir simge değil."
        renderer, rules = {"type": "single", "symbols": set_of(sym)}, 1
    elif kind == "categorizedSymbol":
        field = attr_field()
        if field is None:
            return "Kategorili görünüşün alanı okunamadı."
        cats, other, n = [], None, 0
        node = child(r, "categories")
        for c in ([x for x in node if x.tag == "category"] if node is not None else []):
            label = c.get("label", "")
            sset = set_of(symbols.get(c.get("symbol")))
            enabled = c.get("render") != "false"
            vals = [v.get("value") for v in c if v.tag == "val" and v.get("value") is not None]
            null = c.get("type") == "NULL" or (not vals and not c.get("value"))
            n += 1
            if null and not vals:
                if other is not None:
                    lost.append(f"ikinci “Diğerleri” sınıfı (“{label}”) alınmadı")
                else:
                    other = sset
                continue
            for v in (vals or [c.get("value", "")]):
                cat = {"value": v, "label": label or v, "symbols": sset}
                if not enabled:
                    cat["enabled"] = False
                cats.append(cat)
        renderer = {"type": "categorized", "expr": f"[{field}]", "categories": cats}
        if other is not None:
            renderer["other"] = other
        rules = n
    elif kind == "graduatedSymbol":
        field = attr_field()
        if field is None:
            return "Aralıklı görünüşün alanı okunamadı."
        classes = []
        node = child(r, "ranges")
        for c in ([x for x in node if x.tag == "range"] if node is not None else []):
            lo, hi = fnum(c.get("lower")), fnum(c.get("upper"))
            if lo is None or hi is None:
                continue
            classes.append({"min": lo, "max": hi, "label": c.get("label", ""),
                            "symbols": set_of(symbols.get(c.get("symbol")))})
            if c.get("render") == "false":
                lost.append(f"kapalı “{c.get('label', '')}” sınıfı açık alındı (KentOS'un aralıklarında kapatma yok)")
        if len(classes) > 1:
            lost.append("QGIS'in aralıkları alt sınırı dışarıda bırakır (alt < değer ≤ üst, ilki hariç), KentOS'unkiler "
                        "üst sınırı (alt ≤ değer < üst, sonuncu hariç); sınıf sınırındaki değer bir üstteki sınıfa geçer")
        ss = child(r, "sizescale")
        if (ss is not None and ss.get("scalemethod") is not None) or r.get("graduatedMethod") == "GraduatedSize":
            lost.append("boya göre aralıklı görünüş renge göre alındı")
        renderer, rules = {"type": "graduated", "expr": f"[{field}]", "classes": classes}, len(classes)
    elif kind == "RuleRenderer":
        count = [0]
        top = child(r, "rules")
        lst_rules = read_rules(top, symbols, lost, count) if top is not None else []
        renderer, rules = {"type": "rules", "rules": lst_rules}, count[0]
    else:
        return (f"QGIS'in “{kind}” görünüşü KentOS'a çevrilmedi (tek simge, kategorili, aralıklı ve kurallı görünüş "
                "alınır).")
    if rules == 0:
        return "QML'de KentOS'un çizebileceği bir sınıf ya da kural yok."
    return renderer, lost, rules


# ─── The files ─────────────────────────────────────────────────────────────────────────────────────────

LINE = """<layer class="SimpleLine" enabled="1" locked="0" pass="0"><Option type="Map">
 <Option name="line_color" type="QString" value="0,0,0,255"/><Option name="line_width" type="QString" value="0.5"/>
 <Option name="line_width_unit" type="QString" value="MM"/></Option></layer>"""

HAND = {
    # Losses: an SVG marker, an unknown shape, a data-defined size, labels, the layer's opacity.
    "kayipli.qml": """<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="3.34.0" labelsEnabled="1"><renderer-v2 type="singleSymbol"><symbols>
<symbol name="0" type="marker" alpha="1">
<layer class="SvgMarker" enabled="1"><Option type="Map"><Option name="name" type="QString" value="x.svg"/></Option></layer>
<layer class="SimpleMarker" enabled="1"><Option type="Map"><Option name="name" type="QString" value="heart"/>
<Option name="color" type="QString" value="255,0,0,255"/><Option name="size" type="QString" value="4"/>
<Option name="outline_style" type="QString" value="no"/><Option name="offset" type="QString" value="1,0"/></Option>
<data_defined_properties><Option type="Map"><Option name="properties" type="Map"><Option name="size" type="Map">
<Option name="active" type="bool" value="true"/><Option name="expression" type="QString" value="&quot;boy&quot;"/>
</Option></Option></Option></data_defined_properties></layer>
</symbol></symbols></renderer-v2><labeling type="simple"/><layerOpacity>0.5</layerOpacity></qgis>
""",
    # QGIS 2's <prop k v>, inches and points; a disabled layer; a hatch brush.
    "qgis2.qml": """<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="2.18.0"><renderer-v2 type="singleSymbol"><symbols><symbol name="0" type="fill" alpha="0.6">
<layer class="SimpleFill" enabled="1"><prop k="color" v="10,20,30,255"/><prop k="style" v="b_diagonal"/>
<prop k="outline_color" v="200,0,0,255"/><prop k="outline_style" v="dash"/><prop k="outline_width" v="1"/>
<prop k="outline_width_unit" v="Point"/></layer>
<layer class="SimpleLine" enabled="0"><prop k="line_color" v="0,0,255,128"/><prop k="line_width" v="0.01"/>
<prop k="line_width_unit" v="Inch"/><prop k="capstyle" v="flat"/><prop k="joinstyle" v="miter"/></layer>
</symbol></symbols></renderer-v2></qgis>
""",
    # A categorized look on an expression, not a field.
    "ifade.qml": """<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2"><renderer-v2 type="categorizedSymbol" attr="upper(&quot;ada&quot;)"><categories>
<category label="A" render="true" symbol="0" type="string" value="A"/></categories><symbols>
<symbol name="0" type="fill" alpha="1">""" + LINE + """</symbol></symbols></renderer-v2></qgis>
""",
    # Rules: a filter KentOS cannot say leaves its rule and its children; IN becomes OR; a disabled rule; ELSE.
    "suzgecler.qml": """<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2"><renderer-v2 type="RuleRenderer"><rules key="{0}">
<rule key="{1}" label="Seçili adalar" filter="&quot;ada&quot; IN ('A', 'C') AND &quot;alan&quot; BETWEEN 100 AND 900" symbol="0">
 <rule key="{2}" label="Tapusuz" filter="&quot;tapulu&quot; IS NULL OR NOT (&quot;tapulu&quot; = 1)" symbol="1"/>
</rule>
<rule key="{3}" label="İşlevli" filter="upper(&quot;ada&quot;) = 'B'" symbol="1"><rule key="{4}" label="Alt" symbol="0"/></rule>
<rule key="{5}" label="Kapalı" filter="&quot;ad&quot; LIKE 'Ç%'" symbol="0" checkstate="0" scalemaxdenom="5000"/>
<rule key="{6}" filter="ELSE" symbol="1"/>
</rules><symbols>
<symbol name="0" type="fill" alpha="1">""" + LINE + """</symbol>
<symbol name="1" type="fill" alpha="1"><layer class="SimpleFill" enabled="1"><Option type="Map">
<Option name="color" type="QString" value="#33a02c"/><Option name="outline_style" type="QString" value="no"/></Option></layer></symbol>
</symbols></renderer-v2></qgis>
""",
    "isi.qml": """<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2"><renderer-v2 type="heatmapRenderer" radius="10"/></qgis>
""",
    "bos.qml": """<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2"><labeling type="simple"/></qgis>
""",
    "yalniz-svg.qml": """<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis version="4.2.2"><renderer-v2 type="singleSymbol"><symbols><symbol name="0" type="marker" alpha="1">
<layer class="SvgMarker" enabled="1"><Option type="Map"/></layer></symbol></symbols></renderer-v2></qgis>
""",
    "sld-degil.qml": """<?xml version="1.0"?><StyledLayerDescriptor version="1.1.0"/>
""",
    "bozuk.qml": "<qgis><renderer-v2",
}

QGIS_STYLES = r"""
from qgis.core import *
from qgis.PyQt.QtCore import QPointF
from qgis.PyQt.QtGui import QColor
QgsApplication.setPrefixPath('{q}/MacOS', True)
app = QgsApplication([], False); app.initQgis()
G = '{g}'
out = '{out}/'
parsel = QgsVectorLayer(G + '|layername=parsel', 'parsel', 'ogr')
cats = []
for v, c in [('A', '#e41a1c'), ('B', '#377eb8')]:
    cats.append(QgsRendererCategory(v, QgsFillSymbol.createSimple({{'color': c, 'outline_color': '#222222', 'outline_width': '0.4'}}), 'Ada ' + v))
cats.append(QgsRendererCategory(['C', 'D'], QgsFillSymbol.createSimple({{'color': '#4daf4a', 'outline_style': 'no'}}), 'C ve D'))
cats.append(QgsRendererCategory(None, QgsFillSymbol.createSimple({{'color': '#999999'}}), 'Diğer'))
parsel.setRenderer(QgsCategorizedSymbolRenderer('ada', cats)); parsel.saveNamedStyle(out + 'qgis-kategorili.qml')
ranges = []
for lo, hi, c in [(0, 500, '#fee5d9'), (500, 1000, '#fcae91'), (1000, 5000, '#de2d26')]:
    ranges.append(QgsRendererRange(lo, hi, QgsFillSymbol.createSimple({{'color': c, 'outline_color': '#555555', 'outline_width': '0.26'}}), f'{{lo}} - {{hi}}'))
parsel.setRenderer(QgsGraduatedSymbolRenderer('alan', ranges)); parsel.saveNamedStyle(out + 'qgis-aralikli.qml')
root = QgsRuleBasedRenderer.Rule(None)
r1 = QgsRuleBasedRenderer.Rule(QgsFillSymbol.createSimple({{'color': '#1b9e77'}}), 0, 0, '"tapulu" = 1', 'Tapulu')
r1b = QgsRuleBasedRenderer.Rule(QgsFillSymbol.createSimple({{'color': '#e7298a'}}), 0, 0, '"alan" >= 1000', 'Tapulu büyük')
r1.appendChild(r1b)
r2 = QgsRuleBasedRenderer.Rule(QgsFillSymbol.createSimple({{'color': '#d95f02', 'outline_style': 'dash'}}), 500, 10000, '"alan" > 100 AND "ada" <> \'B\'', 'Büyük')
r3 = QgsRuleBasedRenderer.Rule(QgsFillSymbol.createSimple({{'color': '#7570b3'}}), 0, 0, 'ELSE', 'Diğer'); r3.setIsElse(True)
for r in (r1, r2, r3): root.appendChild(r)
parsel.setRenderer(QgsRuleBasedRenderer(root)); parsel.saveNamedStyle(out + 'qgis-kurallar.qml')
yol = QgsVectorLayer(G + '|layername=yol', 'yol', 'ogr')
s = QgsLineSymbol.createSimple({{'line_color': '#3366cc', 'line_width': '0.8', 'line_style': 'dash', 'capstyle': 'round', 'joinstyle': 'round'}})
s.appendSymbolLayer(QgsSimpleLineSymbolLayer.create({{'line_color': '#ffffff', 'line_width': '2', 'line_width_unit': 'MapUnit', 'use_custom_dash': '1', 'customdash': '6;3', 'customdash_unit': 'MapUnit'}}))
s.setOpacity(0.75)
yol.setRenderer(QgsSingleSymbolRenderer(s)); yol.saveNamedStyle(out + 'qgis-cizgi.qml')
nokta = QgsVectorLayer(G + '|layername=nokta', 'nokta', 'ogr')
nokta.setRenderer(QgsSingleSymbolRenderer(QgsMarkerSymbol.createSimple({{'name': 'triangle', 'color': '#ff0000', 'outline_color': '#000000', 'outline_width': '0.3', 'size': '3', 'angle': '30'}})))
nokta.saveNamedStyle(out + 'qgis-nokta.qml')
nokta.setRenderer(QgsSingleSymbolRenderer(QgsMarkerSymbol.createSimple({{'name': 'cross2', 'color': '#00000000', 'outline_color': '#0000ff', 'outline_width': '1', 'outline_width_unit': 'Pixel', 'size': '8', 'size_unit': 'Pixel'}})))
nokta.saveNamedStyle(out + 'qgis-piksel.qml')
cen = QgsCentroidFillSymbolLayer(); cen.setPointOnSurface(False)
cen.setSubSymbol(QgsMarkerSymbol.createSimple({{'name': 'star', 'color': '#ffcc00', 'size': '4'}}))
fs = QgsFillSymbol.createSimple({{'color': '#ffffff', 'outline_color': '#000000'}}); fs.appendSymbolLayer(cen)
parsel.setRenderer(QgsSingleSymbolRenderer(fs)); parsel.saveNamedStyle(out + 'qgis-merkez.qml')
lp = QgsLinePatternFillSymbolLayer(); lp.setLineAngle(30); lp.setDistance(3); lp.setOffset(0.5)
lp.setSubSymbol(QgsLineSymbol.createSimple({{'line_color': '#336699', 'line_width': '0.3'}}))
ts = QgsFillSymbol.createSimple({{'color': '#ffffff', 'outline_color': '#000000', 'outline_width': '0.4'}}); ts.appendSymbolLayer(lp)
edge = QgsSimpleLineSymbolLayer.create({{'line_color': '#aa0000', 'line_width': '0.5'}})
edge.setDataDefinedProperty(QgsSymbolLayer.Property.StrokeWidth, QgsProperty.fromExpression('"alan" / 1000'))
edge.setDataDefinedProperty(QgsSymbolLayer.Property.StrokeColor, QgsProperty.fromField('renk'))
ts.appendSymbolLayer(edge)
parsel.setRenderer(QgsSingleSymbolRenderer(ts)); parsel.saveNamedStyle(out + 'qgis-tarama.qml')
ml = QgsMarkerLineSymbolLayer(); ml.setPlacements(Qgis.MarkerLinePlacement.Interval | Qgis.MarkerLinePlacement.FirstVertex)
ml.setInterval(4); ml.setOffsetAlongLine(1)
mk = QgsMarkerSymbol.createSimple({{'name': 'triangle', 'color': '#ff0000', 'outline_style': 'no', 'size': '2'}})
mk.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.Size, QgsProperty.fromExpression('"kat" * 2 + 1'))
mk.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.Angle, QgsProperty.fromField('yon'))
mk.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.FillColor, QgsProperty.fromExpression("if(\"kat\" > 2, 'red', 'blue')"))
ml.setSubSymbol(mk)
ls = QgsLineSymbol.createSimple({{'line_color': '#3366cc', 'line_width': '0.6'}}); ls.appendSymbolLayer(ml)
ls.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.StrokeWidth, QgsProperty.fromExpression('round("genislik", 1)'))
yol.setRenderer(QgsSingleSymbolRenderer(ls)); yol.saveNamedStyle(out + 'qgis-isaretli.qml')
# ADR 0571: gradients, point patterns, SVG and font markers, data-defined colours.
def grad(c1, c2, **kw):
    g = QgsGradientFillSymbolLayer(QColor(c1), QColor(c2))
    for k, v in kw.items():
        getattr(g, 'set' + k)(v)
    fs = QgsFillSymbol(); fs.changeSymbolLayer(0, g)
    return fs
ramp = QgsGradientColorRamp(QColor('#ffffcc'), QColor('#006837'))
dc = [QgsRendererCategory('A', grad('#ff0000', '#0000ff', ReferencePoint1=QPointF(0, 0.5), ReferencePoint2=QPointF(1, 0.5), Angle=30), 'Doğrusal 30°'),
      QgsRendererCategory('B', grad('#ffff00', '#008000', GradientType=Qgis.GradientType.Radial), 'Dairesel'),
      QgsRendererCategory('C', grad('#ffffff', '#000000', GradientSpread=Qgis.GradientSpread.Reflect, ReferencePoint1=QPointF(0, 0.5), ReferencePoint2=QPointF(0.5, 0.5)), 'Silindir'),
      QgsRendererCategory('D', grad('#ff0000', '#0000ff', GradientType=Qgis.GradientType.Conical), 'Konik'),
      QgsRendererCategory('E', grad('#ff0000', '#0000ff', ColorRamp=ramp, GradientColorType=Qgis.GradientColorSource.ColorRamp, ReferencePoint1=QPointF(0.5, 1), ReferencePoint2=QPointF(0.5, 0)), 'Renk rampası, aşağıdan yukarı')]
parsel.setRenderer(QgsCategorizedSymbolRenderer('ada', dc)); parsel.saveNamedStyle(out + 'qgis-degrade.qml')
def patt(marker, **kw):
    pp = QgsPointPatternFillSymbolLayer()
    for k, v in kw.items():
        getattr(pp, 'set' + k)(v)
    pp.setSubSymbol(marker)
    fs = QgsFillSymbol(); fs.changeSymbolLayer(0, pp)
    return fs
dot = lambda c: QgsMarkerSymbol.createSimple({{'name': 'circle', 'color': c, 'size': '1.5', 'outline_style': 'no'}})
pc = [QgsRendererCategory('A', patt(dot('#00aa00'), DistanceX=5, DistanceY=6), 'Izgara 5 × 6'),
      QgsRendererCategory('B', patt(dot('#aa0000'), DistanceX=5, DistanceY=5, DisplacementX=2.5, Angle=30, OffsetX=1, OffsetY=2), 'Kaymalı, dönük, kaydırılmış'),
      QgsRendererCategory('C', patt(dot('#0000aa'), DistanceX=5, DistanceY=5, DisplacementX=1, DisplacementY=1, MaximumRandomDeviationX=0.5), 'Başka kayma, rastgele sapma'),
      QgsRendererCategory('D', patt(dot('#aa00aa'), DistanceX=12, DistanceY=12, DistanceXUnit=Qgis.RenderUnit.Pixels, DistanceYUnit=Qgis.RenderUnit.Pixels), 'Piksel aralığı')]
parsel.setRenderer(QgsCategorizedSymbolRenderer('ada', pc)); parsel.saveNamedStyle(out + 'qgis-desen.qml')
svg = 'base64:PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA0IDQiPjxjaXJjbGUgY3g9IjIiIGN5PSIyIiByPSIyIi8+PC9zdmc+'
sv = QgsSvgMarkerSymbolLayer(svg, 5, 20); sv.setFillColor(QColor('#ff0000')); sv.setStrokeColor(QColor('#000000')); sv.setStrokeWidth(0.3)
sv.setDataDefinedProperty(QgsSymbolLayer.Property.Size, QgsProperty.fromField('boy'))
ms = QgsMarkerSymbol(); ms.changeSymbolLayer(0, sv)
sv2 = QgsSvgMarkerSymbolLayer('/usr/share/icons/yok.svg', 5, 0)
ms2 = QgsMarkerSymbol(); ms2.changeSymbolLayer(0, sv2)
nokta.setRenderer(QgsCategorizedSymbolRenderer('ad', [QgsRendererCategory('A', ms, 'SVG, gömülü'), QgsRendererCategory('B', ms2, 'SVG, dosya yolu')]))
nokta.saveNamedStyle(out + 'qgis-svg.qml')
fm = QgsFontMarkerSymbolLayer('Arial', 'A', 6, QColor('#aa0000')); fm.setStrokeColor(QColor('#ffffff')); fm.setStrokeWidth(0.4)
fm.setAngle(15); fm.setFontStyle('Bold Italic')
fm.setDataDefinedProperty(QgsSymbolLayer.Property.Character, QgsProperty.fromField('harf'))
fm.setDataDefinedProperty(QgsSymbolLayer.Property.FillColor, QgsProperty.fromField('renk'))
ms = QgsMarkerSymbol(); ms.changeSymbolLayer(0, fm)
nokta.setRenderer(QgsSingleSymbolRenderer(ms)); nokta.saveNamedStyle(out + 'qgis-yazi.qml')
fl = QgsFillSymbol.createSimple({{'color': '#ff0000', 'outline_color': '#000000', 'outline_width': '0.4'}})
fl.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.FillColor, QgsProperty.fromField('renk'))
fl.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.StrokeColor, QgsProperty.fromField('kenar'))
parsel.setRenderer(QgsSingleSymbolRenderer(fl)); parsel.saveNamedStyle(out + 'qgis-renkler.qml')
# One symbol each, the layers stacked: a gradient, dots over it and an outline; an SVG marker and a font marker.
sym = QgsFillSymbol.createSimple({{'color': '#ffffff', 'outline_color': '#222222', 'outline_width': '0.4'}})
sym.changeSymbolLayer(0, QgsGradientFillSymbolLayer(QColor('#ff7f00'), QColor('#3366cc')))
sym.symbolLayer(0).setReferencePoint1(QPointF(0, 0.5)); sym.symbolLayer(0).setReferencePoint2(QPointF(1, 0.5)); sym.symbolLayer(0).setAngle(30)
pt = patt(dot('#ffffff'), DistanceX=4, DistanceY=4, DisplacementX=2); sym.appendSymbolLayer(pt.symbolLayer(0).clone())
sym.appendSymbolLayer(QgsSimpleLineSymbolLayer.create({{'line_color': '#222222', 'line_width': '0.5'}}))
parsel.setRenderer(QgsSingleSymbolRenderer(sym)); parsel.saveNamedStyle(out + 'qgis-simgeler-alan.qml')
sv = QgsSvgMarkerSymbolLayer(svg, 6, 0); sv.setFillColor(QColor('#e31a1c'))
ms = QgsMarkerSymbol(); ms.changeSymbolLayer(0, sv)
fm = QgsFontMarkerSymbolLayer('Arial', 'A', 4, QColor('#ffffff')); fm.setFontStyle('Bold')
ms.appendSymbolLayer(fm)
nokta.setRenderer(QgsSingleSymbolRenderer(ms)); nokta.saveNamedStyle(out + 'qgis-simgeler-nokta.qml')
cl = QgsLineSymbol.createSimple({{'line_color': '#3366cc', 'line_width': '0.8'}})
cl.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.StrokeColor, QgsProperty.fromExpression("\"kat\" * 2 + 1"))
yol.setRenderer(QgsSingleSymbolRenderer(cl)); yol.saveNamedStyle(out + 'qgis-renk-ifadesi.qml')
print('ok')
"""


def qgis_env():
    return dict(os.environ, QT_QPA_PLATFORM="offscreen", PYTHONHOME=str(QGIS / "Resources"),
                PYTHONPATH=f"{QGIS}/Resources/python3.12:{QGIS}/Resources/python3.12/site-packages:"
                           f"{QGIS}/Resources/python3.12/lib-dynload",
                PROJ_DATA=str(QGIS / "Resources/qgis/proj"))


def write_files() -> None:
    DIR.mkdir(parents=True, exist_ok=True)
    for name, text in HAND.items():
        (DIR / name).write_text(text, encoding="utf-8")
    code = QGIS_STYLES.format(q=QGIS, g=ROOT / "fixtures/geopackage/v1/qgis.gpkg", out=DIR)
    r = subprocess.run([str(QGIS / "MacOS/python3.12"), "-c", code], env=qgis_env(), capture_output=True, text=True,
                       timeout=300)
    if "ok" not in r.stdout:
        sys.exit(f"QGIS QML yazamadı: {r.stderr[-600:]}")
    print(f"{len(HAND)} elle yazılmış ve 17 QGIS QML'i: {DIR}")


# ─── The cases ─────────────────────────────────────────────────────────────────────────────────────────

LAYER_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def layer(ident, name, **extra):
    return {"id": ident, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": dict(LAYER_STYLE), "children": [], **extra}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "QML'den görünüş",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 487000, "y": 4420000},
    "layers": [
        layer("parsel", "Parsel"), layer("yol", "Yol"), layer("durak", "Durak"),
        {"id": "imar", "name": "İmar", "type": "group", "visible": True, "locked": False, "expanded": True,
         "style": dict(LAYER_STYLE), "children": [layer("imar-alan", "İmar alanı")]},
        layer("altlik", "OpenStreetMap", service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
                                                    "maxZoom": 19}),
    ],
    "activeLayer": "parsel",
    "entities": [
        {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"ada": "A", "alan": "420"},
         "pts": [{"x": 487000, "y": 4420000}, {"x": 487030, "y": 4420000}, {"x": 487030, "y": 4420014}, {"x": 487000, "y": 4420014}]},
        {"kind": "polyline", "id": 2, "layerId": "yol", "attrs": {},
         "pts": [{"x": 487000, "y": 4420020}, {"x": 487060, "y": 4420020}]},
        {"kind": "point", "id": 3, "layerId": "durak", "attrs": {"ad": "Merkez"}, "p": {"x": 487010, "y": 4420025}},
    ],
    "styles": {"items": [], "categories": []},
}


def layer_after(ident, renderer):
    layers = json.loads(json.dumps(SETUP["layers"]))

    def walk(nodes):
        for n in nodes:
            if n["id"] == ident:
                n["style"]["renderer"] = renderer
            walk(n["children"])

    walk(layers)
    return layers


def imported(ident, name):
    renderer, lost, rules = read((DIR / name).read_text(encoding="utf-8"))
    output = {"layer": ident, "kind": renderer["type"], "rules": rules, "renderer": renderer, "changed": True,
              "revision": "$current"}
    if lost:
        output["lost"] = lost
    warnings = [] if not lost else [{"code": "import_losses", "message": "QML'in bazı parçaları alınamadı: "
                                     + "; ".join(lost) + ".", "path": "path"}]
    return renderer, output, warnings


def ok_case(title, ident, name, by_text=False):
    renderer, output, warnings = imported(ident, name)
    path = {"text": (DIR / name).read_text(encoding="utf-8")} if by_text else {"path": f"$fixture:qml/v1/{name}"}
    return {"name": title, "steps": [{
        "op": "execute", "input": {"layer": ident, **path},
        "result": {"status": "completed", "output": output, "warnings": warnings},
        "expect": {"ids": [1, 2, 3], "canUndo": True, "dirty": True, "revision": "changed",
                   "layers": layer_after(ident, renderer)},
    }]}


def failed(title, inp, code, message, path):
    return {"name": title, "steps": [{
        "op": "execute", "input": inp,
        "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
        "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same"},
    }]}


def refused(title, name, layer_id="parsel"):
    words = read((DIR / name).read_text(encoding="utf-8"))
    assert isinstance(words, str), name
    return failed(title, {"layer": layer_id, "path": f"$fixture:qml/v1/{name}"}, "invalid_qml", words, "path")


def build_cases():
    cases = [
        ok_case("QGIS'in kategorili görünüşü: iki değer, iki değerli sınıf her değer için, NULL sınıfı Diğerleri olur",
                "parsel", "qgis-kategorili.qml"),
        ok_case("QGIS'in aralıklı görünüşü: üç aralık, sınır kuralının farkı söylenir", "parsel", "qgis-aralikli.qml"),
        ok_case("QGIS'in kurallı görünüşü: iç içe kural, ölçek aralığı, kesikli kenar, ELSE", "parsel",
                "qgis-kurallar.qml"),
        ok_case("QGIS'in çizgisi: Qt'nin kesik deseni kalınlıkla, özel kesikler harita biriminde, simgenin saydamlığı",
                "yol", "qgis-cizgi.qml"),
        ok_case("QGIS'in noktası: üçgen, saat yönünde 30° KentOS'ta -30°, kenar kalınlığı", "durak", "qgis-nokta.qml"),
        ok_case("Piksel birimi ve saydam dolgu: çarpı işareti dolgusuz", "durak", "qgis-piksel.qml"),
        ok_case("Merkez dolgusu: yıldız ağırlık merkezinde, dolgu ve kenarı ayrı katmanlar", "parsel",
                "qgis-merkez.qml"),
        ok_case("QGIS'in çizgi deseni: tarama açısı, aralığı ve kaydırmasıyla; kenarın veriye bağlı kalınlığı ifadesiyle, "
                "veriye bağlı rengi söylenir", "parsel", "qgis-tarama.qml"),
        ok_case("QGIS'in işaretli çizgisi: ilk yerleşim alınır, ikincisi söylenir; işaretin veriye bağlı boyu ve dönüşü, "
                "çevrilemeyen ifadeler söylenir", "yol", "qgis-isaretli.qml"),
        ok_case("QGIS'in degradeleri: doğrusal 30° (kutunun batısından doğusuna), dairesel (merkez ilk renk, KentOS'ta küre), "
                "silindir (yarım kutuyla yansıtma), konik doğrusal olur, renk rampası aşağıdan yukarı", "parsel",
                "qgis-degrade.qml"),
        ok_case("QGIS'in nokta desenleri: ızgara, yarım aralık kaymalı (saat yönü açı KentOS'ta eksi, y aşağı eksi), başka kayma ve "
                "rastgele sapma söylenir, piksel aralığı", "parsel", "qgis-desen.qml"),
        ok_case("QGIS'in SVG işaretleri: gömülü çizim kitaplığa girer, dosya yolu söylenir; veriye bağlı boy", "durak",
                "qgis-svg.qml"),
        ok_case("QGIS'in yazı işareti: karakter, yazı tipi, kalın eğik, kenar halesi, dönüş; veriye bağlı karakter ve renk",
                "durak", "qgis-yazi.qml"),
        ok_case("Katmanlar üst üste: degrade, üstünde nokta deseni ve kenar çizgisi (tek simge)", "parsel",
                "qgis-simgeler-alan.qml"),
        ok_case("Katmanlar üst üste: SVG işaretinin üstünde yazı işareti (tek simge)", "durak",
                "qgis-simgeler-nokta.qml"),
        ok_case("Veriye bağlı renkler: dolgu ve kenar alandan, KentOS'ta alanın değeri sabit rengin yerine geçer", "parsel",
                "qgis-renkler.qml"),
        ok_case("Alana bağlı olmayan veriye bağlı renk ifadesi söylenir", "yol", "qgis-renk-ifadesi.qml"),
        ok_case("Alınamayanlar söylenir: SVG işareti, bilinmeyen şekil, veriye bağlı boy, kaydırma, etiketler, katmanın "
                "saydamlığı", "durak", "kayipli.qml"),
        ok_case("QGIS 2'nin prop'ları: nokta ve inç mm'ye, kapalı katman kapalı, fırça deseni söylenir", "parsel",
                "qgis2.qml"),
        ok_case("Süzgeçler: IN veya'ya, BETWEEN, IS NULL, NOT; işlevli süzgeç alt kuralıyla alınmaz; kapalı kural",
                "parsel", "suzgecler.qml"),
        ok_case("QML dosya yerine metin olarak verilir", "parsel", "qgis-kategorili.qml", by_text=True),
    ]
    renderer, output, warnings = imported("parsel", "qgis-merkez.qml")
    cases.append({"name": "Aynı görünüş ikinci kez: değişmez, adım yazılmaz", "steps": [
        {"op": "execute", "input": {"layer": "parsel", "path": "$fixture:qml/v1/qgis-merkez.qml"},
         "result": {"status": "completed", "output": output, "warnings": warnings},
         "expect": {"canUndo": True, "revision": "changed"}},
        {"op": "execute", "input": {"layer": "parsel", "path": "$fixture:qml/v1/qgis-merkez.qml"},
         "result": {"status": "completed", "output": dict(output, changed=False), "warnings": warnings},
         "expect": {"revision": "same", "layers": layer_after("parsel", renderer)}},
    ]})
    plan_out = imported("parsel", "qgis-aralikli.qml")
    cases.append({"name": "Plan okur, hiçbir şey yazmaz", "steps": [
        {"op": "plan", "input": {"layer": "parsel", "path": "$fixture:qml/v1/qgis-aralikli.qml"},
         "result": {"status": "completed", "output": plan_out[1], "warnings": plan_out[2]},
         "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same", "layers": SETUP["layers"]}},
    ]})
    line = imported("yol", "qgis-cizgi.qml")
    cases.append({"name": "Geri alınır ve yinelenir", "steps": [
        {"op": "execute", "input": {"layer": "yol", "path": "$fixture:qml/v1/qgis-cizgi.qml"},
         "result": {"status": "completed", "output": line[1], "warnings": line[2]}},
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": SETUP["layers"]}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False, "layers": layer_after("yol", line[0])}},
    ]})
    cases += [
        failed("Ne yol ne metin", {"layer": "parsel"}, "invalid_input",
               "QML'in ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.", "path"),
        failed("Hem yol hem metin", {"layer": "parsel", "path": "$fixture:qml/v1/qgis-nokta.qml", "text": "<x/>"},
               "invalid_input", "QML'in ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.", "path"),
        failed("Olmayan dosya", {"layer": "parsel", "path": "$fixture:qml/v1/yok.qml"}, "file_unreadable",
               "“$fixture:qml/v1/yok.qml” okunamadı: No such file or directory (os error 2). Dosyanın yerinde olduğunu denetleyin.",
               "path"),
        refused("XML olmayan dosya", "bozuk.qml"),
        refused("QML olmayan XML (SLD)", "sld-degil.qml"),
        refused("Görünüşü olmayan QML (yalnız etiket)", "bos.qml"),
        refused("KentOS'ta olmayan görünüş türü (ısı haritası)", "isi.qml"),
        refused("Kategorili görünüş bir ifadeyle, alanla değil", "ifade.qml"),
        refused("Çizilebilecek simgesi olmayan QML (yalnız SVG işareti)", "yalniz-svg.qml", "durak"),
        failed("Olmayan katman", {"layer": "yok", "path": "$fixture:qml/v1/qgis-nokta.qml"}, "layer_not_found",
               "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"),
        failed("Katman grubu", {"layer": "imar", "path": "$fixture:qml/v1/qgis-nokta.qml"}, "not_a_layer",
               "“İmar” bir katman grubu; işleyici yalnız katmanın olur. Grubun bir katmanını verin.", "layer"),
        failed("Servis katmanı", {"layer": "altlik", "path": "$fixture:qml/v1/qgis-nokta.qml"}, "service_layer",
               "“OpenStreetMap” servisten çizilir ve nesne tutmaz; işleyici nesneleri olan katmanın olur.", "layer"),
    ]
    cases.append({"name": "Eski sürüme göre hazırlanmış istek: hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": {"layer": "parsel", "path": "$fixture:qml/v1/qgis-nokta.qml", "expectedRevision": "999"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
             "path": "expectedRevision", "revision": "$current"}},
         "expect": {"canUndo": False, "revision": "same"}},
    ]})
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.qml", "commandVersion": 1,
        "title": "QML'den görünüş al",
        "note": "ADR 0555, 0565. QGIS 4.2'nin yazdığı ve elle yazılmış QML'ler (fixtures/qml/v1); görünüşler ADR'nin okuma kuralıyla burada, xml.etree'yle kurulur. Bağımsız başvuru: scripts/fixtures/qml_cases.py; elle düzenlemeyin.",
        "setup": SETUP,
        "cases": cases,
    }


# ─── KentOS's QMLs in QGIS ─────────────────────────────────────────────────────────────────────────────

QGIS_READ = r"""
import json
from qgis.core import *
from qgis.PyQt.QtGui import QColor
QgsApplication.setPrefixPath('{q}/MacOS', True)
app = QgsApplication([], False); app.initQgis()
G = '{g}'
def sym(s):
    if s is None:
        return None
    out = {{'opacity': round(s.opacity(), 3), 'layers': []}}
    for i in range(s.symbolLayerCount()):
        sl = s.symbolLayer(i)
        d = {{'class': sl.layerType(), 'enabled': sl.enabled(), 'colour': sl.color().name(QColor.NameFormat.HexArgb)}}
        if sl.layerType() == 'SimpleLine':
            d['width'] = [round(sl.width(), 4), QgsUnitTypes.encodeUnit(sl.widthUnit())]
            d['dash'] = [round(x, 4) for x in sl.customDashVector()] if sl.useCustomDashPattern() else []
        if sl.layerType() == 'SimpleMarker':
            d['shape'] = QgsSimpleMarkerSymbolLayerBase.encodeShape(sl.shape())
            d['size'] = [round(sl.size(), 4), QgsUnitTypes.encodeUnit(sl.sizeUnit())]
            d['angle'] = round(sl.angle(), 4)
        if sl.layerType() == 'LinePatternFill':
            d['line'] = [round(sl.lineAngle(), 4), round(sl.distance(), 4), round(sl.offset(), 4),
                         QgsUnitTypes.encodeUnit(sl.distanceUnit())]
            d['sub'] = sym(sl.subSymbol())
        if sl.layerType() == 'GradientFill':
            d['grad'] = [int(sl.gradientType()), sl.color().name(), sl.color2().name(), int(sl.gradientSpread()),
                         [sl.referencePoint1().x(), sl.referencePoint1().y()],
                         [sl.referencePoint2().x(), sl.referencePoint2().y()], round(sl.angle(), 4)]
        if sl.layerType() == 'PointPatternFill':
            d['pat'] = [sl.distanceX(), sl.distanceY(), sl.displacementX(), sl.displacementY(), sl.offsetX(),
                        sl.offsetY(), round(sl.angle(), 4), QgsUnitTypes.encodeUnit(sl.distanceXUnit())]
            d['sub'] = sym(sl.subSymbol())
        if sl.layerType() == 'SvgMarker':
            d['svg'] = [sl.path()[:7], sl.path()[7:].__len__() > 40, round(sl.size(), 4), round(sl.angle(), 4),
                        sl.fillColor().name(), sl.strokeColor().name()]
        if sl.layerType() == 'FontMarker':
            d['font'] = [sl.fontFamily(), sl.character(), round(sl.size(), 4), round(sl.angle(), 4), sl.fontStyle(),
                         sl.color().name(), sl.strokeColor().name(), round(sl.strokeWidth(), 4)]
        if sl.layerType() == 'MarkerLine':
            d['marks'] = [round(sl.interval(), 4), round(sl.offsetAlongLine(), 4), int(sl.placements()),
                          sl.rotateSymbols()]
            d['sub'] = sym(sl.subSymbol())
        dd = sl.dataDefinedProperties()
        found = {{}}
        for k in dd.propertyKeys():
            pr = dd.property(k)
            if pr.isActive():
                found[QgsSymbolLayer.propertyDefinitions()[k].name()] = pr.expressionString()
        if found:
            d['dd'] = found
        out['layers'].append(d)
    return out
def rule(x):
    return {{'label': x.label(), 'filter': x.filterExpression(), 'else': x.isElse(), 'max': x.maximumScale(),
            'min': x.minimumScale(), 'symbol': sym(x.symbol()), 'children': [rule(c) for c in x.children()]}}
out = {{}}
for name, table in {files}:
    l = QgsVectorLayer(G + '|layername=' + table, table, 'ogr')
    msg, ok = l.loadNamedStyle('{dir}/' + name)
    r = l.renderer()
    info = {{'ok': ok, 'type': r.type()}}
    if r.type() == 'categorizedSymbol':
        info['field'] = r.classAttribute()
        info['classes'] = [[c.value(), c.label(), sym(c.symbol())['layers'][0]['colour']] for c in r.categories()]
        info['symbols'] = [sym(c.symbol()) for c in r.categories()]
    elif r.type() == 'graduatedSymbol':
        info['field'] = r.classAttribute()
        info['classes'] = [[g.lowerValue(), g.upperValue(), sym(g.symbol())['layers'][0]['colour']] for g in r.ranges()]
    elif r.type() == 'RuleRenderer':
        info['rules'] = [rule(x) for x in r.rootRule().children()]
    else:
        info['symbol'] = sym(r.symbol())
    out[name] = info
print(json.dumps(out, ensure_ascii=False))
"""

OUTLINE = {"class": "SimpleLine", "enabled": True, "colour": "#ff222222", "width": [0.4, "MM"], "dash": []}


def area(colour):
    return {"opacity": 1.0, "layers": [{"class": "SimpleFill", "enabled": True, "colour": colour}, OUTLINE]}


# What KentOS's QMLs (written by the headless test) must be in QGIS 4.2.
WANT = {
    "kentos-kategorili.qml": ("parsel", {"type": "categorizedSymbol", "field": "ada", "classes": [
        ["A", "Ada A", "#ffe41a1c"], ["B", "Ada B", "#ff377eb8"], [None, "Diğerleri", "#80999999"]]}),
    "kentos-aralikli.qml": ("parsel", {"type": "graduatedSymbol", "field": "alan", "classes": [
        [0.0, 500.0, "#fffee5d9"], [500.0, 1000.0, "#fffcae91"], [1000.0, 5000.0, "#ffde2d26"]]}),
    "kentos-kurallar.qml": ("parsel", {"type": "RuleRenderer", "rules": [
        {"label": "Tapulu", "filter": '"tapulu" = 1', "else": False, "max": 0.0, "min": 0.0, "symbol": area("#ff1b9e77"),
         "children": [{"label": "Büyük", "filter": "\"alan\" > 1000 AND \"ada\" <> 'B'", "else": False, "max": 500.0,
                       "min": 10000.0, "symbol": area("#ffd95f02"), "children": []}]},
        {"label": "Diğer", "filter": "ELSE", "else": True, "max": 0.0, "min": 0.0, "symbol": area("#ff7570b3"),
         "children": []}]}),
    "kentos-cizgi.qml": ("yol", {"type": "singleSymbol", "symbol": {"opacity": 0.8, "layers": [
        {"class": "SimpleLine", "enabled": True, "colour": "#ff3366cc", "width": [0.8, "MM"], "dash": [3.0, 1.5]},
        {"class": "SimpleLine", "enabled": True, "colour": "#ffffffff", "width": [2.0, "RenderMetersInMapUnits"],
         "dash": []}]}}),
    "kentos-tarama.qml": ("parsel", {"type": "singleSymbol", "symbol": {"opacity": 1.0, "layers": [
        {"class": "SimpleFill", "enabled": True, "colour": "#ffffffff"},
        {"class": "LinePatternFill", "enabled": True, "colour": "#ff336699", "line": [30.0, 2.5, 0.5, "MM"],
         "sub": {"opacity": 1.0, "layers": [{"class": "SimpleLine", "enabled": True, "colour": "#ff336699",
                                             "width": [0.2, "MM"], "dash": []}]}},
        {"class": "SimpleLine", "enabled": True, "colour": "#ffaa0000", "width": [0.5, "MM"], "dash": [],
         "dd": {"outlineWidth": '"alan" / 1000'}}]}}),
    "kentos-isaretli.qml": ("yol", {"type": "singleSymbol", "symbol": {"opacity": 1.0, "layers": [
        {"class": "SimpleLine", "enabled": True, "colour": "#ff3366cc", "width": [0.6, "MM"], "dash": []},
        {"class": "MarkerLine", "enabled": True, "colour": "#ffff0000", "marks": [4.0, 1.0, 1, True],
         "sub": {"opacity": 1.0, "layers": [{"class": "SimpleMarker", "enabled": True, "colour": "#ffff0000",
                                             "shape": "triangle", "size": [2.0, "MM"], "angle": 0.0,
                                             "dd": {"size": '"kat" * 2 + 1', "angle": '-("yon")'}}]}}]}}),
    # ADR 0571: gradients (type 0 linear, 1 radial; the sphere's centre is QGIS's first colour, a cylinder reflects a
    # half box), a point pattern (clockwise angle, y down), an embedded SVG, a font marker, data-defined colours.
    "kentos-degrade.qml": ("parsel", {"type": "categorizedSymbol", "field": "ada", "symbols": [
        {"opacity": 1.0, "layers": [{"class": "GradientFill", "enabled": True, "colour": "#ffff0000",
                                     "grad": [0, "#ff0000", "#0000ff", 0, [0.0, 0.5], [1.0, 0.5], 30.0]}]},
        {"opacity": 1.0, "layers": [{"class": "GradientFill", "enabled": True, "colour": "#ff008000",
                                     "grad": [1, "#008000", "#ffff00", 0, [0.5, 0.5], [1.0, 1.0], 0.0]}]},
        {"opacity": 1.0, "layers": [{"class": "GradientFill", "enabled": True, "colour": "#ffffffff",
                                     "grad": [0, "#ffffff", "#000000", 1, [0.0, 0.5], [0.5, 0.5], 90.0]}]}]}),
    "kentos-desen.qml": ("parsel", {"type": "singleSymbol", "symbol": {"opacity": 1.0, "layers": [
        {"class": "PointPatternFill", "enabled": True, "colour": "#ff00aa00",
         "pat": [5.0, 6.0, 2.5, 0.0, 1.0, -2.0, -30.0, "MM"],
         "sub": {"opacity": 1.0, "layers": [{"class": "SimpleMarker", "enabled": True, "colour": "#ff00aa00",
                                             "shape": "circle", "size": [1.5, "MM"], "angle": 0.0}]}}]}}),
    "kentos-svg.qml": ("nokta", {"type": "singleSymbol", "symbol": {"opacity": 1.0, "layers": [
        {"class": "SvgMarker", "enabled": True, "colour": "#ffff0000",
         "svg": ["base64:", True, 5.0, 20.0, "#ff0000", "#000000"]}]}}),
    "kentos-yazi.qml": ("nokta", {"type": "singleSymbol", "symbol": {"opacity": 1.0, "layers": [
        {"class": "FontMarker", "enabled": True, "colour": "#ffaa0000",
         "font": ["Arial", "A", 6.0, 15.0, "Bold Italic", "#aa0000", "#ffffff", 0.4]}]}}),
    "kentos-renkler.qml": ("parsel", {"type": "singleSymbol", "symbol": {"opacity": 1.0, "layers": [
        {"class": "SimpleFill", "enabled": True, "colour": "#ffff0000", "dd": {"fillColor": '"renk"'}},
        {"class": "SimpleLine", "enabled": True, "colour": "#ff000000", "width": [0.4, "MM"], "dash": [],
         "dd": {"outlineColor": '"kenar"'}}]}}),
    "kentos-nokta.qml": ("nokta", {"type": "singleSymbol", "symbol": {"opacity": 1.0, "layers": [
        {"class": "SimpleMarker", "enabled": True, "colour": "#ffff0000", "shape": "triangle", "size": [3.0, "MM"],
         "angle": 30.0}]}}),
}


def check_out():
    out_dir = DIR / "out"
    missing = [n for n in WANT if not (out_dir / n).exists()]
    if missing:
        return [f"{n} yok: önce KENTOS_WRITE_QML=1 ile KentOS'un testini çalıştırın" for n in missing]
    py = QGIS / "MacOS/python3.12"
    if not py.exists():
        print("QGIS 4.2 yok; KentOS'un QML'leri QGIS'te denetlenmedi.")
        return []
    files = [(n, t) for n, (t, _) in WANT.items()]
    code = QGIS_READ.format(q=QGIS, g=ROOT / "fixtures/geopackage/v1/qgis.gpkg", files=files, dir=out_dir)
    r = subprocess.run([str(py), "-c", code], env=qgis_env(), capture_output=True, text=True, timeout=300)
    line = [x for x in r.stdout.splitlines() if x.startswith("{")]
    if not line:
        return [f"QGIS okuyamadı: {r.stderr[-600:]}"]
    got = json.loads(line[-1])
    problems = []
    for name, (_, want) in WANT.items():
        g = got.get(name, {})
        if not g.get("ok"):
            problems.append(f"{name}: QGIS yüklemedi")
        for k, v in want.items():
            if g.get(k) != v:
                problems.append(f"{name}: QGIS'te {k} {json.dumps(g.get(k), ensure_ascii=False)}, beklenen "
                                f"{json.dumps(v, ensure_ascii=False)}")
    return problems


def main() -> None:
    if "--files" in sys.argv:
        write_files()
        return
    data = build_cases()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        problems = []
        if not CASES.exists() or CASES.read_text(encoding="utf-8") != text:
            problems.append(f"{CASES.relative_to(ROOT)} güncel değil")
        problems += check_out()
        if problems:
            sys.exit("\n".join(problems))
        print(f"{len(data['cases'])} durum güncel; KentOS'un QML'leri QGIS 4.2'de beklenen görünüşte.")
        return
    CASES.write_text(text, encoding="utf-8")
    print(f"{len(data['cases'])} durum: {CASES.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
