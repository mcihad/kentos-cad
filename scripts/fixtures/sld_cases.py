#!/usr/bin/env python3
"""SLD'den görünüş al'ın ortak durumları ve SLD'nin QGIS 4.2 ile iki yönlü denetimi (docs/adr/0516, 0571), KentOS kodu olmadan.

    python3 scripts/fixtures/sld_cases.py --files    # QGIS 4.2'nin SLD'lerini ve elle yazılanları fixtures/sld/v1'e yazar
    python3 scripts/fixtures/sld_cases.py            # fixtures/commands/v1/file.import.sld.json'u yazar
    python3 scripts/fixtures/sld_cases.py --check    # yazılı durumlar güncel mi; KentOS'un SLD'leri QGIS'te ne oluyor

Okuma kuralları ADR'den, bu betikte yeniden yazılmıştır (xml.etree ile): bir alanın değerlerine eşit kurallar
kategorili, bir alanın aralıkları aralıklı görünüş, süzgeçsiz tek kural tek simge, ötekiler kurallı görünüş; SE'nin
pikseli piksel, metresi metre; renkler ve saydamlıkları #rrggbb(aa); ADR 0571'den `GraphicFill` nokta deseni, SVG (`base64:`
ve satır içi), `ttf://` yazı işareti, `PropertyName` ve `Recode`/`Categorize` renkleri. Alınamayanların cümleleri de ADR'nin
sözleridir.

KentOS'un yazdığı SLD'leri (`KENTOS_WRITE_SLD=1 cargo test -p kentos-headless --test headless sld` yazar,
fixtures/sld/v1/out) QGIS 4.2 açar: görünüşün türü, sınıflar, renkler ve kalınlıklar beklenenle denetlenir.
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
DIR = ROOT / "fixtures/sld/v1"
CASES = ROOT / "fixtures/commands/v1/file.import.sld.json"
QGIS = Path("/Applications/QGIS-final-4_2_2.app/Contents")

# ─── The reader, from the ADR ──────────────────────────────────────────────────────────────────────────


def local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def child(n, name):
    for c in n:
        if local(c.tag) == name:
            return c
    return None


def kids(n, name):
    return [c for c in n if local(c.tag) == name]


def text_of(n) -> str:
    return "".join(n.itertext()).strip()


def param_node(n, name):
    for c in n:
        if local(c.tag) in ("SvgParameter", "CssParameter") and c.get("name") == name:
            return c
    return None


def param(n, name):
    """A parameter's literal text; none when it holds a property name or a function (see paint)."""
    p = param_node(n, name)
    if p is None or len(list(p)) > 0:
        return None
    return text_of(p)


def fnum(s):
    try:
        return float(s)
    except (TypeError, ValueError):
        return None


def num(v: float) -> str:
    if v == math.trunc(v) and abs(v) < 1e15:
        return str(int(v))
    return repr(v)


def size_in(v: float, uom):
    if uom and uom.endswith("/metre"):
        return v, "m"
    if uom and uom.endswith("/foot"):
        return v * 0.3048, "m"
    return v, "px"


def colour_with(rgb: str, opacity):
    if opacity is not None and opacity < 1.0:
        o = min(max(opacity, 0.0), 1.0) * 255.0
        return rgb.lower() + "%02x" % int(math.floor(o + 0.5))
    return rgb.lower()


def read_filter(n, lost):
    tag = local(n.tag)
    ops = {
        "PropertyIsEqualTo": "=", "PropertyIsNotEqualTo": "<>", "PropertyIsLessThan": "<",
        "PropertyIsLessThanOrEqualTo": "<=", "PropertyIsGreaterThan": ">", "PropertyIsGreaterThanOrEqualTo": ">=",
    }

    def prop(x):
        p = child(x, "PropertyName")
        return text_of(p) if p is not None else None

    def lit(x):
        literal = child(x, "Literal")
        if literal is None:
            return None
        t = text_of(literal)
        v = fnum(t)
        if v is not None and math.isfinite(v) and not t.startswith("+"):
            return ("num", v)
        return ("text", t)

    if tag in ops:
        f, li = prop(n), lit(n)
        if f is None or li is None:
            return None
        return ("cmp", ops[tag], f, li)
    if tag in ("And", "Or"):
        parts = []
        for c in n:
            p = read_filter(c, lost)
            if p is None:
                return None
            parts.append(p)
        return ("and" if tag == "And" else "or", parts)
    if tag == "Not":
        inner = next(iter(n), None)
        if inner is None:
            return None
        f = read_filter(inner, lost)
        return None if f is None else ("not", f)
    if tag == "PropertyIsNull":
        f = prop(n)
        return None if f is None else ("null", f)
    if tag == "PropertyIsBetween":
        lo, hi = child(n, "LowerBoundary"), child(n, "UpperBoundary")
        a = fnum(text_of(child(lo, "Literal"))) if lo is not None and child(lo, "Literal") is not None else None
        b = fnum(text_of(child(hi, "Literal"))) if hi is not None and child(hi, "Literal") is not None else None
        f = prop(n)
        if a is None or b is None or f is None:
            return None
        return ("between", f, a, b)
    if tag == "PropertyIsLike":
        wild = n.get("wildCard", "%")
        single = n.get("singleChar") or n.get("singleCharacter") or "_"
        literal = child(n, "Literal")
        f = prop(n)
        if literal is None or f is None:
            return None
        return ("like", f, text_of(literal).replace(wild, "%").replace(single, "_"))
    lost.append(f"“{tag}” süzgeci KentOS'un ifadesine çevrilemedi")
    return None


def lit_text(li):
    kind, v = li
    return num(v) if kind == "num" else "'" + v.replace("'", "''") + "'"


def filter_text(f) -> str:
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
        return " ve ".join(f"({filter_text(p)})" if p[0] == "or" else filter_text(p) for p in f[1])
    if k == "or":
        return " veya ".join(f"({filter_text(p)})" if p[0] == "and" else filter_text(p) for p in f[1])
    if k == "not":
        inner = f[1]
        if inner[0] == "null":
            return f"[{inner[1]}] boş değil"
        return f"değil ({filter_text(inner)})"
    raise ValueError(k)


def literal_expr(t):
    t = t.strip()
    v = fnum(t)
    if v is not None and math.isfinite(v):
        return num(v)
    return "'" + t.replace("'", "''") + "'"


def function_expr(f, fallback, lost, what):
    """Recode and Categorize of one field as a `durum` chain whose values are colours."""
    name = f.get("name") or ""
    args = list(f)
    field = text_of(args[0]) if args and local(args[0].tag) == "PropertyName" else None
    lits = [text_of(a) for a in args[1:]] if all(local(a.tag) == "Literal" for a in args[1:]) else None
    if field is None or lits is None:
        lost.append(f"{what}: SE'nin “{name}” işlevi KentOS'un ifadesine çevrilemedi; sabit değeri alındı")
        return None

    def q(c):
        return "'" + colour_with(c.strip(), None) + "'"

    out = "durum"
    if name == "Recode" and len(lits) >= 2 and len(lits) % 2 == 0:
        for i in range(0, len(lits), 2):
            out += f" eğer [{field}] = {literal_expr(lits[i])} ise {q(lits[i + 1])}"
        out += f" yoksa '{fallback}' son"
    elif name == "Categorize" and len(lits) >= 3 and len(lits) % 2 == 1:
        rest = lits[1:]
        for k in range(0, len(rest), 2):
            out += f" eğer [{field}] < {literal_expr(rest[k])} ise {q(lits[k])}"
        out += f" yoksa {q(lits[-1])} son"
    else:
        lost.append(f"{what}: SE'nin “{name}” işlevi KentOS'un ifadesine çevrilemedi; sabit değeri alındı")
        return None
    return out


def paint(n, name, default, lost, what):
    """A colour parameter as the layer's value: fixed, or an expression with the colour as its fallback."""
    opacity = fnum(param(n, name + "-opacity"))
    rgb = param(n, name)
    fixed = colour_with(rgb, opacity) if rgb is not None else None
    p = param_node(n, name)
    if p is None or len(list(p)) == 0:
        if fixed is not None:
            return fixed
        return colour_with(default, opacity) if default is not None else None
    fallback = colour_with(default, opacity) if default is not None else ""
    e = list(p)[0]
    tag = local(e.tag)
    if tag == "PropertyName":
        expr = f"[{text_of(e)}]"
    elif tag == "Function":
        expr = function_expr(e, fallback, lost, what)
    else:
        lost.append(f"{what}: “{name}” değerinin “{tag}” ifadesi KentOS'ta yok; sabit değeri alındı")
        expr = None
    if expr is not None:
        return {"expr": expr, "fallback": fallback} if fallback else {"expr": expr}
    return fallback if fallback else None


def read_stroke(s, uom, ident, lost):
    color = paint(s, "stroke", "#000000", lost, "Çizgi")
    if color is None:
        color = "#000000"
    w = fnum(param(s, "stroke-width"))
    width, unit = size_in(1.0 if w is None else w, uom)
    layer = {"id": ident, "type": "simpleLine", "color": color, "width": width, "unit": unit}
    dash = param(s, "stroke-dasharray")
    if dash is not None:
        parts = [fnum(p.strip()) for p in dash.replace(",", " ").split(" ")]
        d = [size_in(v, uom)[0] for v in parts if v is not None]
        if d:
            layer["dash"] = d
    j = param(s, "stroke-linejoin")
    if j in ("miter", "round", "bevel"):
        layer["join"] = j
    c = param(s, "stroke-linecap")
    if c in ("butt", "round", "square"):
        layer["cap"] = c
    return layer


def read_fill(f, uom, ident, lost):
    gf = child(f, "GraphicFill")
    if gf is not None:
        g = child(gf, "Graphic")
        marker = read_mark(g, uom, lost) if g is not None else None
        if marker is None:
            lost.append("desenli dolgunun (GraphicFill) işareti KentOS'un işaretine çevrilemedi; dolgu alınmadı")
            return None
        size = marker.get("size", 6.0)
        lost.append("desenli dolgu (GraphicFill): SE işareti kendi boyunda döşer; KentOS'un deseninin aralığı işaretin boyu alındı")
        return {"id": ident, "type": "patternFill", "marker": {"type": "marker", "layers": [marker]},
                "spacingX": size, "spacingY": size, "unit": marker.get("unit", "px")}
    color = paint(f, "fill", "#808080", lost, "Dolgu")
    if color is None:
        return None
    return {"id": ident, "type": "simpleFill", "color": color}


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


def decode_svg(text):
    clean = "".join(text.split()).replace("=", "")
    try:
        t = base64.b64decode(clean + "=" * (-len(clean) % 4), validate=True).decode("utf-8", "replace")
    except Exception:
        return None
    return t if looks_like_svg(t) else None


def href_of(n):
    for k, v in n.attrib.items():
        if local(k) == "href":
            return v
    return None


def read_external(g, lost):
    seen = False
    for e in kids(g, "ExternalGraphic"):
        seen = True
        text = None
        o = child(e, "OnlineResource")
        h = href_of(o) if o is not None else None
        if h is not None:
            for pre in ("base64:", "data:image/svg+xml;base64,"):
                if h.startswith(pre):
                    text = h[len(pre):]
                    break
        if text is None:
            ic = child(e, "InlineContent")
            if ic is not None:
                text = text_of(ic)
        svg = decode_svg(text) if text is not None else None
        if svg is not None:
            return asset_id("sld", svg)
    if seen:
        lost.append("dış resimli işaret (ExternalGraphic): çizimin içindeki bir SVG değil (dosya adresi ya da başka bir resim); KentOS çizimin içindeki SVG'yi alır, işaret alınmadı")
    return None


def font_mark(mark):
    font, code = None, None
    o = child(mark, "OnlineResource")
    if o is not None:
        h = href_of(o)
        if h is not None and h.startswith("ttf://"):
            font = h[len("ttf://"):]
    mi = child(mark, "MarkIndex")
    if mi is not None:
        t = text_of(mi)
        if t.isdigit() and int(t) < 0x110000 and not 0xD800 <= int(t) < 0xE000:
            code = chr(int(t))
    w = child(mark, "WellKnownName")
    if w is not None and text_of(w).startswith("ttf://"):
        r = text_of(w)[len("ttf://"):]
        f, _, c = r.partition("#")
        font = f
        if code is None:
            v = None
            if c.startswith("0x"):
                try:
                    v = int(c[2:], 16)
                except ValueError:
                    v = None
            elif c.isdigit():
                v = int(c)
            if v is not None and v < 0x110000 and not 0xD800 <= v < 0xE000:
                code = chr(v)
            elif v is None and len(c) == 1:
                code = c
    if font is None or code is None:
        return None
    return font, code


def read_mark(g, uom, lost):
    sz = child(g, "Size")
    sv = fnum(text_of(sz)) if sz is not None else None
    size, unit = size_in(6.0 if sv is None else sv, uom)
    rot = child(g, "Rotation")
    rotation = fnum(text_of(rot)) if rot is not None else None
    asset = read_external(g, lost)
    if asset is not None:
        m = {"id": "v", "type": "svg", "asset": asset, "size": size, "unit": unit}
        if rotation is not None:
            m["rotation"] = -rotation
        return m
    mark = child(g, "Mark")
    if mark is None:
        return None
    fm = font_mark(mark)
    if fm is not None:
        m = {"id": "t", "type": "text", "text": fm[1], "font": fm[0], "size": size, "unit": unit}
        fill = child(mark, "Fill")
        if fill is not None:
            c = paint(fill, "fill", None, lost, "Yazı işareti")
            if c is not None:
                m["color"] = c
        st = child(mark, "Stroke")
        if st is not None and param(st, "stroke") is not None:
            sw = fnum(param(st, "stroke-width"))
            m["halo"] = {"color": colour_with(param(st, "stroke"), fnum(param(st, "stroke-opacity"))),
                         "width": size_in(1.0 if sw is None else sw, uom)[0]}
        if rotation is not None:
            m["rotation"] = -rotation
        return m
    w = child(mark, "WellKnownName")
    wkn = text_of(w) if w is not None else "square"
    if wkn in ("circle", "square", "triangle", "star", "cross", "x"):
        shape = wkn
    else:
        lost.append(f"“{wkn}” işareti KentOS'ta yok; kare yazıldı")
        shape = "square"
    m = {"id": "p", "type": "shape", "shape": shape, "size": size, "unit": unit, "fill": None}
    fill = child(mark, "Fill")
    if fill is not None:
        c = paint(fill, "fill", "#808080", lost, "İşaret")
        if c is not None:
            m["fill"] = c
    st = child(mark, "Stroke")
    if st is not None:
        c = paint(st, "stroke", "#000000", lost, "İşaret")
        m["stroke"] = c if c is not None else "#000000"
        sw = fnum(param(st, "stroke-width"))
        m["strokeWidth"] = size_in(1.0 if sw is None else sw, uom)[0]
    if rotation is not None:
        m["rotation"] = -rotation
    return m


def read_symbols(rule, lost):
    fill, line, marker = [], [], []
    for s in rule:
        uom = s.get("uom")
        tag = local(s.tag)
        if tag == "PolygonSymbolizer":
            f = child(s, "Fill")
            if f is not None:
                got = read_fill(f, uom, f"f{len(fill)}", lost)
                if got is not None:
                    fill.append(got)
            st = child(s, "Stroke")
            if st is not None:
                fill.append(read_stroke(st, uom, f"o{len(fill)}", lost))
            if child(s, "Displacement") is not None or child(s, "PerpendicularOffset") is not None:
                lost.append("alanın kaydırması KentOS'un dolgusuna çevrilmedi")
        elif tag == "LineSymbolizer":
            st = child(s, "Stroke")
            if st is not None:
                if child(st, "GraphicStroke") is not None:
                    lost.append("işaretli çizgi (GraphicStroke) KentOS'un çizgisine çevrilmedi")
                    continue
                layer = read_stroke(st, uom, f"l{len(line)}", lost)
                off = child(s, "PerpendicularOffset")
                if off is not None and fnum(text_of(off)) is not None:
                    layer["offset"] = size_in(fnum(text_of(off)), uom)[0]
                line.append(layer)
        elif tag == "PointSymbolizer":
            g = child(s, "Graphic")
            if g is not None:
                m = read_mark(g, uom, lost)
                if m is not None:
                    marker.append(m)
        elif tag == "TextSymbolizer":
            lost.append("etiket (TextSymbolizer) görünüşe çevrilmedi; KentOS'ta etiketler ayrıdır")
        elif tag == "RasterSymbolizer":
            lost.append("raster görünüşü (RasterSymbolizer) çevrilmedi")
    out = {}
    if fill:
        out["fill"] = {"type": "fill", "layers": fill}
    if line:
        out["line"] = {"type": "line", "layers": line}
    if marker:
        out["marker"] = {"type": "marker", "layers": marker}
    return out


def as_categories(rules):
    field, cats, other = None, [], None
    for r in rules:
        if r["min"] is not None or r["max"] is not None:
            return None
        f = r["filter"]
        if f is not None and f[0] == "cmp" and f[1] == "=":
            if field is not None and field != f[2]:
                return None
            field = f[2]
            kind, v = f[3]
            cats.append({"value": num(v) if kind == "num" else v, "label": r["label"], "symbols": r["symbols"]})
        elif f is None and r["else"] and other is None:
            other = r["symbols"]
        else:
            return None
    if field is None:
        return None
    out = {"type": "categorized", "expr": f"[{field}]", "categories": cats}
    if other is not None:
        out["other"] = other
    return out


def as_classes(rules):
    field, classes = None, []
    for r in rules:
        if r["else"] or r["min"] is not None or r["max"] is not None:
            return None
        f = r["filter"]
        if f is not None and f[0] == "and" and len(f[1]) == 2:
            a, b = f[1]
            if not (a[0] == "cmp" and b[0] == "cmp" and a[2] == b[2] and a[1] in (">=", ">") and b[1] in ("<", "<=")
                    and a[3][0] == "num" and b[3][0] == "num"):
                return None
            fld, lo, hi = a[2], a[3][1], b[3][1]
        elif f is not None and f[0] == "between":
            fld, lo, hi = f[1], f[2], f[3]
        else:
            return None
        if field is not None and field != fld:
            return None
        field = fld
        classes.append({"min": lo, "max": hi, "label": r["label"], "symbols": r["symbols"]})
    if field is None:
        return None
    return {"type": "graduated", "expr": f"[{field}]", "classes": classes}


def read(text: str):
    """(renderer, lost, rules) or the refusal's words."""
    try:
        root = ET.fromstring(text)
    except ET.ParseError:
        return "Dosya okunabilir bir XML değil."
    styles = [n for n in root.iter() if local(n.tag) == "FeatureTypeStyle"]
    if not styles:
        return "Dosyada SLD'nin görünüşü (FeatureTypeStyle) yok."
    lost = []
    if len(styles) > 1:
        lost.append(f"{len(styles)} görünüşten yalnız ilki alındı")
    rules = []
    for r in kids(styles[0], "Rule"):
        d = child(r, "Description")
        title = child(d, "Title") if d is not None else None
        label = text_of(title) if title is not None and text_of(title) else None
        if label is None:
            nm = child(r, "Name")
            label = text_of(nm) if nm is not None else f"Kural {len(rules) + 1}"
        filt = None
        fe = child(r, "Filter")
        if fe is not None:
            inner = next(iter(fe), None)
            if inner is not None:
                filt = read_filter(inner, lost)
                if filt is None:
                    lost.append(f"“{label}” kuralının süzgeci okunamadı; kural alınmadı")
                    continue

        def scale(tag):
            c = child(r, tag)
            return fnum(text_of(c)) if c is not None else None

        symbols = read_symbols(r, lost)
        if not symbols:
            continue
        rules.append({"label": label, "else": child(r, "ElseFilter") is not None, "filter": filt,
                      "min": scale("MinScaleDenominator"), "max": scale("MaxScaleDenominator"), "symbols": symbols})
    if not rules:
        return "SLD'de KentOS'un çizebileceği bir kural yok."
    r0 = rules[0]
    if len(rules) == 1 and r0["filter"] is None and not r0["else"] and r0["min"] is None and r0["max"] is None:
        renderer = {"type": "single", "symbols": r0["symbols"]}
    else:
        renderer = as_categories(rules) or as_classes(rules)
        if renderer is None:
            out = []
            for k, r in enumerate(rules):
                v = {"id": f"r{k + 1}", "label": r["label"], "symbols": r["symbols"]}
                if r["filter"] is not None:
                    v["filter"] = filter_text(r["filter"])
                if r["else"]:
                    v["isElse"] = True
                if r["min"] is not None:
                    v["minScale"] = r["min"]
                if r["max"] is not None:
                    v["maxScale"] = r["max"]
                out.append(v)
            renderer = {"type": "rules", "rules": out}
    return renderer, lost, len(rules)


# ─── The files ─────────────────────────────────────────────────────────────────────────────────────────

HAND = {
    "geoserver-10.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.0.0" xmlns="http://www.opengis.net/sld" xmlns:ogc="http://www.opengis.net/ogc">
  <NamedLayer><Name>imar</Name>
    <UserStyle><Title>İmar</Title>
      <FeatureTypeStyle>
        <Rule><Title>Konut</Title>
          <ogc:Filter><ogc:PropertyIsEqualTo><ogc:PropertyName>kullanim</ogc:PropertyName><ogc:Literal>Konut</ogc:Literal></ogc:PropertyIsEqualTo></ogc:Filter>
          <PolygonSymbolizer><Fill><CssParameter name="fill">#FFFF99</CssParameter><CssParameter name="fill-opacity">0.6</CssParameter></Fill>
            <Stroke><CssParameter name="stroke">#996633</CssParameter><CssParameter name="stroke-width">0.5</CssParameter></Stroke></PolygonSymbolizer>
        </Rule>
        <Rule><Title>Ticaret</Title>
          <ogc:Filter><ogc:PropertyIsEqualTo><ogc:PropertyName>kullanim</ogc:PropertyName><ogc:Literal>Ticaret</ogc:Literal></ogc:PropertyIsEqualTo></ogc:Filter>
          <PolygonSymbolizer><Fill><CssParameter name="fill">#FF6666</CssParameter></Fill></PolygonSymbolizer>
        </Rule>
        <Rule><Title>Öteki</Title><ElseFilter/>
          <PolygonSymbolizer><Fill><CssParameter name="fill">#CCCCCC</CssParameter></Fill></PolygonSymbolizer>
        </Rule>
      </FeatureTypeStyle>
    </UserStyle>
  </NamedLayer>
</StyledLayerDescriptor>
""",
    "metre.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc">
  <NamedLayer><se:Name>yol</se:Name><UserStyle><se:FeatureTypeStyle>
    <se:Rule><se:Name>Yol</se:Name>
      <se:LineSymbolizer uom="http://www.opengeospatial.org/se/units/metre">
        <se:Stroke><se:SvgParameter name="stroke">#888888</se:SvgParameter><se:SvgParameter name="stroke-width">7</se:SvgParameter></se:Stroke>
      </se:LineSymbolizer>
      <se:LineSymbolizer uom="http://www.opengeospatial.org/se/units/metre">
        <se:Stroke><se:SvgParameter name="stroke">#FFFFFF</se:SvgParameter><se:SvgParameter name="stroke-width">0.15</se:SvgParameter><se:SvgParameter name="stroke-dasharray">3 3</se:SvgParameter></se:Stroke>
        <se:PerpendicularOffset>1.75</se:PerpendicularOffset>
      </se:LineSymbolizer>
    </se:Rule>
  </se:FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
""",
    "kayipli.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc" xmlns:xlink="http://www.w3.org/1999/xlink">
  <NamedLayer><se:Name>durak</se:Name><UserStyle>
    <se:FeatureTypeStyle>
      <se:Rule><se:Name>Durak</se:Name>
        <se:PointSymbolizer><se:Graphic>
          <se:ExternalGraphic><se:OnlineResource xlink:type="simple" xlink:href="durak.svg"/><se:Format>image/svg+xml</se:Format></se:ExternalGraphic>
          <se:Mark><se:WellKnownName>shape://vertline</se:WellKnownName><se:Fill><se:SvgParameter name="fill">#0055AA</se:SvgParameter></se:Fill></se:Mark>
          <se:Size>12</se:Size>
        </se:Graphic></se:PointSymbolizer>
        <se:TextSymbolizer><se:Label><ogc:PropertyName>ad</ogc:PropertyName></se:Label></se:TextSymbolizer>
      </se:Rule>
      <se:Rule><se:Name>Kutuda</se:Name>
        <ogc:Filter><ogc:BBOX><ogc:PropertyName>geom</ogc:PropertyName></ogc:BBOX></ogc:Filter>
        <se:PointSymbolizer><se:Graphic><se:Mark><se:WellKnownName>circle</se:WellKnownName></se:Mark></se:Graphic></se:PointSymbolizer>
      </se:Rule>
    </se:FeatureTypeStyle>
    <se:FeatureTypeStyle><se:Rule><se:PointSymbolizer><se:Graphic><se:Mark/></se:Graphic></se:PointSymbolizer></se:Rule></se:FeatureTypeStyle>
  </UserStyle></NamedLayer>
</StyledLayerDescriptor>
""",
    "desenli.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc">
  <NamedLayer><se:Name>orman</se:Name><UserStyle><se:FeatureTypeStyle>
    <se:Rule><se:Name>Orman</se:Name>
      <se:PolygonSymbolizer>
        <se:Fill><se:GraphicFill><se:Graphic><se:Mark><se:WellKnownName>circle</se:WellKnownName></se:Mark><se:Size>4</se:Size></se:Graphic></se:GraphicFill></se:Fill>
        <se:Stroke><se:SvgParameter name="stroke">#2E7D32</se:SvgParameter><se:SvgParameter name="stroke-width">2</se:SvgParameter><se:SvgParameter name="stroke-opacity">0.5</se:SvgParameter></se:Stroke>
      </se:PolygonSymbolizer>
      <se:LineSymbolizer><se:Stroke><se:GraphicStroke><se:Graphic><se:Mark><se:WellKnownName>triangle</se:WellKnownName></se:Mark></se:Graphic></se:GraphicStroke></se:Stroke></se:LineSymbolizer>
    </se:Rule>
  </se:FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
""",
    "suzgecler.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc">
  <NamedLayer><se:Name>parsel</se:Name><UserStyle><se:FeatureTypeStyle>
    <se:Rule><se:Name>Boş not</se:Name>
      <ogc:Filter><ogc:PropertyIsNull><ogc:PropertyName>not</ogc:PropertyName></ogc:PropertyIsNull></ogc:Filter>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">#EEEEEE</se:SvgParameter></se:Fill></se:PolygonSymbolizer>
    </se:Rule>
    <se:Rule><se:Name>Ka ile başlayan</se:Name>
      <ogc:Filter><ogc:PropertyIsLike wildCard="*" singleChar="?" escapeChar="\\"><ogc:PropertyName>ada</ogc:PropertyName><ogc:Literal>Ka*</ogc:Literal></ogc:PropertyIsLike></ogc:Filter>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">#AADDAA</se:SvgParameter></se:Fill></se:PolygonSymbolizer>
    </se:Rule>
    <se:Rule><se:Name>Orta</se:Name>
      <ogc:Filter><ogc:Or>
        <ogc:PropertyIsBetween><ogc:PropertyName>alan</ogc:PropertyName><ogc:LowerBoundary><ogc:Literal>100</ogc:Literal></ogc:LowerBoundary><ogc:UpperBoundary><ogc:Literal>200.5</ogc:Literal></ogc:UpperBoundary></ogc:PropertyIsBetween>
        <ogc:Not><ogc:PropertyIsEqualTo><ogc:PropertyName>ada</ogc:PropertyName><ogc:Literal>O'Neil</ogc:Literal></ogc:PropertyIsEqualTo></ogc:Not>
      </ogc:Or></ogc:Filter>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill">#5599FF</se:SvgParameter></se:Fill></se:PolygonSymbolizer>
    </se:Rule>
  </se:FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
""",
    # SE's own forms: a Recode and a Categorize of a field, a field's colour in a stroke and a mark, a font mark by its
    # WellKnownName, an SVG inline, a graphic that is a picture file.
    "renk-islevleri.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc" xmlns:xlink="http://www.w3.org/1999/xlink">
  <NamedLayer><se:Name>parsel</se:Name><UserStyle><se:FeatureTypeStyle>
    <se:Rule><se:Name>İşlevli</se:Name>
      <se:PolygonSymbolizer>
        <se:Fill><se:SvgParameter name="fill"><ogc:Function name="Recode"><ogc:PropertyName>tür</ogc:PropertyName>
          <ogc:Literal>konut</ogc:Literal><ogc:Literal>#FF0000</ogc:Literal><ogc:Literal>3</ogc:Literal><ogc:Literal>#00FF00</ogc:Literal>
        </ogc:Function></se:SvgParameter></se:Fill>
        <se:Stroke><se:SvgParameter name="stroke"><ogc:Function name="Categorize"><ogc:PropertyName>alan</ogc:PropertyName>
          <ogc:Literal>#111111</ogc:Literal><ogc:Literal>100</ogc:Literal><ogc:Literal>#555555</ogc:Literal><ogc:Literal>500</ogc:Literal><ogc:Literal>#999999</ogc:Literal>
        </ogc:Function></se:SvgParameter><se:SvgParameter name="stroke-width">2</se:SvgParameter></se:Stroke>
      </se:PolygonSymbolizer>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill"><ogc:Function name="strConcat"><ogc:PropertyName>a</ogc:PropertyName><ogc:Literal>b</ogc:Literal></ogc:Function></se:SvgParameter></se:Fill></se:PolygonSymbolizer>
      <se:PolygonSymbolizer><se:Fill><se:SvgParameter name="fill"><ogc:Function name="Recode"><ogc:PropertyName>tür</ogc:PropertyName><ogc:PropertyName>başka</ogc:PropertyName></ogc:Function></se:SvgParameter></se:Fill></se:PolygonSymbolizer>
      <se:PointSymbolizer><se:Graphic>
        <se:Mark><se:WellKnownName>circle</se:WellKnownName>
          <se:Fill><se:SvgParameter name="fill"><ogc:PropertyName>renk</ogc:PropertyName></se:SvgParameter><se:SvgParameter name="fill-opacity">0.5</se:SvgParameter></se:Fill>
          <se:Stroke><se:SvgParameter name="stroke"><ogc:PropertyName>kenar</ogc:PropertyName></se:SvgParameter></se:Stroke></se:Mark>
        <se:Size>8</se:Size></se:Graphic></se:PointSymbolizer>
    </se:Rule>
  </se:FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
""",
    "yazi-isaretleri.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se" xmlns:ogc="http://www.opengis.net/ogc" xmlns:xlink="http://www.w3.org/1999/xlink">
  <NamedLayer><se:Name>nokta</se:Name><UserStyle><se:FeatureTypeStyle>
    <se:Rule><se:Name>Yazı</se:Name>
      <se:PointSymbolizer><se:Graphic>
        <se:Mark><se:WellKnownName>ttf://Webdings#0x41</se:WellKnownName><se:Fill><se:SvgParameter name="fill">#00AA00</se:SvgParameter></se:Fill></se:Mark>
        <se:Size>14</se:Size><se:Rotation>45</se:Rotation></se:Graphic></se:PointSymbolizer>
      <se:PointSymbolizer><se:Graphic>
        <se:Mark><se:WellKnownName>ttf://Arial#66</se:WellKnownName></se:Mark><se:Size>10</se:Size></se:Graphic></se:PointSymbolizer>
      <se:PointSymbolizer><se:Graphic>
        <se:ExternalGraphic><se:InlineContent encoding="base64">PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA0IDQiPjxjaXJjbGUgY3g9IjIiIGN5PSIyIiByPSIyIi8+PC9zdmc+</se:InlineContent><se:Format>image/svg+xml</se:Format></se:ExternalGraphic>
        <se:Size>12</se:Size></se:Graphic></se:PointSymbolizer>
      <se:PointSymbolizer><se:Graphic>
        <se:ExternalGraphic><se:OnlineResource xlink:type="simple" xlink:href="durak.png"/><se:Format>image/png</se:Format></se:ExternalGraphic>
        <se:Mark><se:WellKnownName>square</se:WellKnownName></se:Mark><se:Size>9</se:Size></se:Graphic></se:PointSymbolizer>
    </se:Rule>
  </se:FeatureTypeStyle></UserStyle></NamedLayer>
</StyledLayerDescriptor>
""",
    "bos.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld"><NamedLayer><Name>x</Name></NamedLayer></StyledLayerDescriptor>
""",
    "kuralsiz.sld": """<?xml version="1.0" encoding="UTF-8"?>
<StyledLayerDescriptor version="1.1.0" xmlns="http://www.opengis.net/sld" xmlns:se="http://www.opengis.net/se"><NamedLayer><se:Name>x</se:Name><UserStyle><se:FeatureTypeStyle>
<se:Rule><se:Name>Yalnız etiket</se:Name><se:TextSymbolizer/></se:Rule></se:FeatureTypeStyle></UserStyle></NamedLayer></StyledLayerDescriptor>
""",
    "bozuk.sld": "<StyledLayerDescriptor><NamedLayer>",
}

QGIS_STYLES = r"""
from qgis.core import *
from qgis.PyQt.QtGui import QColor
QgsApplication.setPrefixPath('{q}/MacOS', True)
app = QgsApplication([], False); app.initQgis()
G = '{g}'
out = '{out}/'
parsel = QgsVectorLayer(G + '|layername=parsel', 'parsel', 'ogr')
cats = []
for v, c in [('A', '#e41a1c'), ('B', '#377eb8')]:
    cats.append(QgsRendererCategory(v, QgsFillSymbol.createSimple({{'color': c, 'outline_color': '#222222', 'outline_width': '0.4'}}), 'Ada ' + v))
parsel.setRenderer(QgsCategorizedSymbolRenderer('ada', cats)); parsel.saveSldStyle(out + 'qgis-kategorili.sld')
ranges = []
for lo, hi, c in [(0, 500, '#fee5d9'), (500, 1000, '#fcae91'), (1000, 5000, '#de2d26')]:
    ranges.append(QgsRendererRange(lo, hi, QgsFillSymbol.createSimple({{'color': c, 'outline_color': '#555555', 'outline_width': '0.26'}}), f'{{lo}} - {{hi}}'))
parsel.setRenderer(QgsGraduatedSymbolRenderer('alan', ranges)); parsel.saveSldStyle(out + 'qgis-aralikli.sld')
root = QgsRuleBasedRenderer.Rule(None)
r1 = QgsRuleBasedRenderer.Rule(QgsFillSymbol.createSimple({{'color': '#1b9e77'}}), 0, 0, '"tapulu" = 1', 'Tapulu')
r2 = QgsRuleBasedRenderer.Rule(QgsFillSymbol.createSimple({{'color': '#d95f02', 'outline_style': 'dash'}}), 500, 10000, '"alan" > 100 AND "ada" <> \'B\'', 'Büyük')
r3 = QgsRuleBasedRenderer.Rule(QgsFillSymbol.createSimple({{'color': '#7570b3'}}), 0, 0, 'ELSE', 'Diğer'); r3.setIsElse(True)
for r in (r1, r2, r3): root.appendChild(r)
parsel.setRenderer(QgsRuleBasedRenderer(root)); parsel.saveSldStyle(out + 'qgis-kurallar.sld')
yol = QgsVectorLayer(G + '|layername=yol', 'yol', 'ogr')
yol.setRenderer(QgsSingleSymbolRenderer(QgsLineSymbol.createSimple({{'line_color': '#3366cc', 'line_width': '0.8', 'line_style': 'dash', 'capstyle': 'round', 'joinstyle': 'round'}})))
yol.saveSldStyle(out + 'qgis-cizgi.sld')
nokta = QgsVectorLayer(G + '|layername=nokta', 'nokta', 'ogr')
nokta.setRenderer(QgsSingleSymbolRenderer(QgsMarkerSymbol.createSimple({{'name': 'triangle', 'color': '#ff0000', 'outline_color': '#000000', 'size': '3', 'angle': '30'}})))
nokta.saveSldStyle(out + 'qgis-nokta.sld')
# ADR 0571: a point pattern, an embedded SVG marker, a font marker, data-defined colours.
pp = QgsPointPatternFillSymbolLayer(); pp.setDistanceX(5); pp.setDistanceY(6)
pp.setSubSymbol(QgsMarkerSymbol.createSimple({{'name': 'circle', 'color': '#00aa00', 'size': '2', 'outline_color': '#003300', 'outline_width': '0.2'}}))
fs = QgsFillSymbol(); fs.changeSymbolLayer(0, pp)
parsel.setRenderer(QgsSingleSymbolRenderer(fs)); parsel.saveSldStyle(out + 'qgis-desen.sld')
svg = 'base64:PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA0IDQiPjxjaXJjbGUgY3g9IjIiIGN5PSIyIiByPSIyIi8+PC9zdmc+'
sv = QgsSvgMarkerSymbolLayer(svg, 5, 20)
ms = QgsMarkerSymbol(); ms.changeSymbolLayer(0, sv)
nokta.setRenderer(QgsSingleSymbolRenderer(ms)); nokta.saveSldStyle(out + 'qgis-svg.sld')
fm = QgsFontMarkerSymbolLayer('Arial', 'A', 6, QColor('#aa0000')); fm.setStrokeColor(QColor('#ffffff')); fm.setStrokeWidth(0.4)
ms = QgsMarkerSymbol(); ms.changeSymbolLayer(0, fm)
nokta.setRenderer(QgsSingleSymbolRenderer(ms)); nokta.saveSldStyle(out + 'qgis-yazi.sld')
fl = QgsFillSymbol.createSimple({{'color': '#ff0000', 'outline_color': '#000000', 'outline_width': '0.4'}})
fl.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.FillColor, QgsProperty.fromField('renk'))
fl.symbolLayer(0).setDataDefinedProperty(QgsSymbolLayer.Property.StrokeColor, QgsProperty.fromField('kenar'))
parsel.setRenderer(QgsSingleSymbolRenderer(fl)); parsel.saveSldStyle(out + 'qgis-renkler.sld')
print('ok')
"""


def qgis_env():
    return dict(os.environ, QT_QPA_PLATFORM="offscreen", PYTHONHOME=str(QGIS / "Frameworks"),
                PYTHONPATH=f"{QGIS}/Resources/qgis/python:{QGIS}/Frameworks/lib/python3.12/site-packages",
                PROJ_DATA=str(QGIS / "Resources/qgis/proj"))


def write_files() -> None:
    DIR.mkdir(parents=True, exist_ok=True)
    for name, text in HAND.items():
        (DIR / name).write_text(text, encoding="utf-8")
    code = QGIS_STYLES.format(q=QGIS, g=ROOT / "fixtures/geopackage/v1/qgis.gpkg", out=DIR)
    r = subprocess.run([str(QGIS / "MacOS/python3.12"), "-c", code], env=qgis_env(), capture_output=True, text=True,
                       timeout=120)
    if "ok" not in r.stdout:
        sys.exit(f"QGIS SLD yazamadı: {r.stderr[-600:]}")
    print(f"{len(HAND)} elle yazılmış ve 9 QGIS SLD'si: {DIR}")


# ─── The cases ─────────────────────────────────────────────────────────────────────────────────────────

LAYER_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def layer(ident, name, **extra):
    return {"id": ident, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": dict(LAYER_STYLE), "children": [], **extra}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "SLD'den görünüş",
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
    """The drawing's layers with `ident`'s renderer."""
    layers = json.loads(json.dumps(SETUP["layers"]))

    def walk(nodes):
        for n in nodes:
            if n["id"] == ident:
                n["style"]["renderer"] = renderer
            walk(n["children"])

    walk(layers)
    return layers


def imported(ident, name, extra_steps=()):
    text = (DIR / name).read_text(encoding="utf-8")
    renderer, lost, rules = read(text)
    output = {"layer": ident, "kind": renderer["type"], "rules": rules, "renderer": renderer, "changed": True,
              "revision": "$current"}
    if lost:
        output["lost"] = lost
    warnings = [] if not lost else [{"code": "import_losses", "message": "SLD'nin bazı parçaları alınamadı: "
                                     + "; ".join(lost) + ".", "path": "path"}]
    return renderer, output, warnings


def ok_case(title, ident, name, by_text=False):
    renderer, output, warnings = imported(ident, name)
    path = {"text": (DIR / name).read_text(encoding="utf-8")} if by_text else {"path": f"$fixture:sld/v1/{name}"}
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


def build_cases():
    cases = [
        ok_case("QGIS'in kategorili görünüşü: Ada A ve B, dolgu ve 0,4 mm'lik kenar (1 piksel)", "parsel",
                "qgis-kategorili.sld"),
        ok_case("QGIS'in aralıklı görünüşü: alanın üç aralığı", "parsel", "qgis-aralikli.sld"),
        ok_case("QGIS'in kurallı görünüşü: süzgeçler KentOS'un ifadesine, ölçek aralığı, Diğer", "parsel",
                "qgis-kurallar.sld"),
        ok_case("QGIS'in çizgisi: kesikli, yuvarlak uç ve köşe, piksel", "yol", "qgis-cizgi.sld"),
        ok_case("QGIS'in noktası: üçgen işaret, döndürülmüş, dolgu ve kenar", "durak", "qgis-nokta.sld"),
        ok_case("GeoServer'ın SLD 1.0'ı: CssParameter, saydam dolgu, ElseFilter kategorili görünüşün Diğerleri olur",
                "imar-alan", "geoserver-10.sld"),
        ok_case("Metre biriminde iki çizgi: yolun dolgusu ve kaydırılmış kesikli şerit çizgisi", "yol", "metre.sld"),
        ok_case("Alınamayanlar söylenir: dış resim, bilinmeyen işaret, etiket, BBOX süzgeci, ikinci görünüş",
                "durak", "kayipli.sld"),
        ok_case("Desenli dolgu nokta deseni olur (aralığı işaretin boyu); işaretli çizgi alınmaz; saydam kenar alınır",
                "parsel", "desenli.sld"),
        ok_case("QGIS'in nokta deseni: işaret ve kenarı, GraphicFill'in işareti kendi boyunda döşer", "parsel",
                "qgis-desen.sld"),
        ok_case("QGIS'in SVG işareti: parametreli adres alınmaz, düz base64 adresi kitaplığa girer; döndürme", "durak",
                "qgis-svg.sld"),
        ok_case("QGIS'in yazı işareti: ttf adresi ve MarkIndex; renk", "durak", "qgis-yazi.sld"),
        ok_case("QGIS'in veriye bağlı rengi SLD'ye yazmadığı görünüş (sabit renkler) olduğu gibi okunur", "parsel",
                "qgis-renkler.sld"),
        ok_case("SE'nin işlevleri: Recode ve Categorize durum ifadesine, başka işlev söylenir; alanın rengi", "parsel",
                "renk-islevleri.sld"),
        ok_case("Yazı işaretleri: WellKnownName'deki ttf adresi (onaltılık, ondalık), satır içi SVG, resim dosyası "
                "söylenir ve işarete düşer", "durak", "yazi-isaretleri.sld"),
        ok_case("Süzgeçler: boş, gibi (joker karakterler çevrilir), arasında, değil ve veya; tırnak iki kez",
                "parsel", "suzgecler.sld"),
        ok_case("SLD dosya yerine metin olarak verilir", "parsel", "qgis-kategorili.sld", by_text=True),
    ]
    # The same look twice: the second leaves the layer as it is, no step.
    renderer, output, warnings = imported("parsel", "qgis-kategorili.sld")
    same = dict(output, changed=False)
    cases.append({"name": "Aynı görünüş ikinci kez: değişmez, adım yazılmaz", "steps": [
        {"op": "execute", "input": {"layer": "parsel", "path": "$fixture:sld/v1/qgis-kategorili.sld"},
         "result": {"status": "completed", "output": output, "warnings": warnings},
         "expect": {"canUndo": True, "revision": "changed"}},
        {"op": "execute", "input": {"layer": "parsel", "path": "$fixture:sld/v1/qgis-kategorili.sld"},
         "result": {"status": "completed", "output": same, "warnings": warnings},
         "expect": {"revision": "same", "layers": layer_after("parsel", renderer)}},
    ]})
    cases.append({"name": "Plan okur, hiçbir şey yazmaz", "steps": [
        {"op": "plan", "input": {"layer": "parsel", "path": "$fixture:sld/v1/qgis-aralikli.sld"},
         "result": {"status": "completed", "output": imported("parsel", "qgis-aralikli.sld")[1],
                    "warnings": []},
         "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same", "layers": SETUP["layers"]}},
    ]})
    cases.append({"name": "Geri alınır ve yinelenir", "steps": [
        {"op": "execute", "input": {"layer": "yol", "path": "$fixture:sld/v1/qgis-cizgi.sld"},
         "result": {"status": "completed", "output": imported("yol", "qgis-cizgi.sld")[1], "warnings": []}},
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": SETUP["layers"]}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False,
                                  "layers": layer_after("yol", imported("yol", "qgis-cizgi.sld")[0])}},
    ]})
    cases += [
        failed("Ne yol ne metin", {"layer": "parsel"}, "invalid_input",
               "SLD'nin ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.", "path"),
        failed("Hem yol hem metin", {"layer": "parsel", "path": "$fixture:sld/v1/metre.sld", "text": "<x/>"},
               "invalid_input", "SLD'nin ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.", "path"),
        failed("Olmayan dosya", {"layer": "parsel", "path": "$fixture:sld/v1/yok.sld"}, "file_unreadable",
               "“$fixture:sld/v1/yok.sld” okunamadı: No such file or directory (os error 2). Dosyanın yerinde olduğunu denetleyin.",
               "path"),
        failed("XML olmayan dosya", {"layer": "parsel", "path": "$fixture:sld/v1/bozuk.sld"}, "invalid_sld",
               "Dosya okunabilir bir XML değil.", "path"),
        failed("Görünüşü olmayan SLD", {"layer": "parsel", "path": "$fixture:sld/v1/bos.sld"}, "invalid_sld",
               "Dosyada SLD'nin görünüşü (FeatureTypeStyle) yok.", "path"),
        failed("Çizilebilecek kuralı olmayan SLD (yalnız etiket)", {"layer": "parsel", "path": "$fixture:sld/v1/kuralsiz.sld"},
               "invalid_sld", "SLD'de KentOS'un çizebileceği bir kural yok.", "path"),
        failed("Olmayan katman", {"layer": "yok", "path": "$fixture:sld/v1/metre.sld"}, "layer_not_found",
               "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"),
        failed("Katman grubu", {"layer": "imar", "path": "$fixture:sld/v1/metre.sld"}, "not_a_layer",
               "“İmar” bir katman grubu; işleyici yalnız katmanın olur. Grubun bir katmanını verin.", "layer"),
        failed("Servis katmanı", {"layer": "altlik", "path": "$fixture:sld/v1/metre.sld"}, "service_layer",
               "“OpenStreetMap” servisten çizilir ve nesne tutmaz; işleyici nesneleri olan katmanın olur.", "layer"),
    ]
    cases.append({"name": "Eski sürüme göre hazırlanmış istek: hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": {"layer": "parsel", "path": "$fixture:sld/v1/metre.sld", "expectedRevision": "999"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
             "path": "expectedRevision", "revision": "$current"}},
         "expect": {"canUndo": False, "revision": "same"}},
    ]})
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.sld", "commandVersion": 1,
        "title": "SLD'den görünüş al",
        "note": "ADR 0516. QGIS 4.2'nin yazdığı ve elle yazılmış SLD'ler (fixtures/sld/v1); görünüşler ADR'nin okuma kuralıyla burada, xml.etree'yle kurulur. Bağımsız başvuru: scripts/fixtures/sld_cases.py; elle düzenlemeyin.",
        "setup": SETUP,
        "cases": cases,
    }


# ─── KentOS's SLDs in QGIS ─────────────────────────────────────────────────────────────────────────────

QGIS_READ = r"""
import json
from qgis.core import *
from qgis.PyQt.QtGui import QColor
QgsApplication.setPrefixPath('{q}/MacOS', True)
app = QgsApplication([], False); app.initQgis()
G = '{g}'
def layer_info(sl):
    t = sl.layerType()
    if t == 'PointPatternFill':
        sub = sl.subSymbol().symbolLayer(0)
        return [t, sl.distanceX(), sl.distanceY(), sub.layerType(), sub.color().name()]
    if t == 'SvgMarker':
        return [t, sl.path()[:7], sl.path()[7:].__len__() > 40, round(sl.size(), 4), round(sl.angle(), 4)]
    if t == 'FontMarker':
        return [t, sl.fontFamily(), sl.character(), round(sl.size(), 4), round(sl.angle(), 4), sl.color().name(),
                sl.strokeColor().name()]
    return [t, sl.color().name()]
out = {{}}
for name, table in {files}:
    l = QgsVectorLayer(G + '|layername=' + table, table, 'ogr')
    msg, ok = l.loadSldStyle('{dir}/' + name)
    r = l.renderer()
    info = {{'ok': ok, 'type': r.type()}}
    if r.type() == 'categorizedSymbol':
        info['field'] = r.classAttribute()
        info['classes'] = [[c.value(), c.symbol().color().name()] for c in r.categories()]
    elif r.type() == 'graduatedSymbol':
        info['field'] = r.classAttribute()
        info['classes'] = [[g.lowerValue(), g.upperValue(), g.symbol().color().name()] for g in r.ranges()]
    elif r.type() == 'RuleRenderer':
        info['rules'] = [[x.label(), x.filterExpression(), x.symbol().color().name() if x.symbol() else None]
                         for x in r.rootRule().children()]
    else:
        s = r.symbol()
        info['colour'] = s.color().name()
        sl = s.symbolLayer(0)
        info['layer'] = sl.layerType()
        if hasattr(sl, 'width'):
            info['width'] = [sl.width(), int(sl.widthUnit())]
        if hasattr(sl, 'strokeWidth'):
            info['width'] = [sl.strokeWidth(), int(sl.strokeWidthUnit())]
        info['layers'] = [layer_info(sl) for sl in [s.symbolLayer(i) for i in range(s.symbolLayerCount())]]
    out[name] = info
print(json.dumps(out))
"""

# What KentOS's SLDs (written by the headless test) must be in QGIS 4.2. QGIS reads every SLD as a rule-based
# renderer (its own categorized SLD too): the rules' labels, filters and colours, the symbol layer and its width.
WANT = {
    "kentos-kategorili.sld": ("parsel", {"type": "RuleRenderer", "rules": [
        ["Ada A", "ada = 'A'", "#e41a1c"], ["Ada B", "ada = 'B'", "#377eb8"]]}),
    "kentos-aralikli.sld": ("parsel", {"type": "RuleRenderer", "rules": [
        ["küçük", "alan >= 0 AND alan < 500", "#fee5d9"],
        ["orta", "alan >= 500 AND alan < 1000", "#fcae91"],
        ["büyük", "alan >= 1000 AND alan <= 5000", "#de2d26"]]}),
    # ADR 0571: a point pattern (a GraphicFill of a mark; the gradient is not written), an SVG marker with its drawing
    # and a font marker (angles turn clockwise), a colour of a field (QGIS reads no field in a colour: grey).
    "kentos-desen.sld": ("parsel", {"type": "singleSymbol", "layer": "PointPatternFill",
                                    "layers": [["PointPatternFill", 7.143, 7.143, "SimpleMarker", "#00aa00"]]}),
    "kentos-isaretler.sld": ("nokta", {"type": "singleSymbol", "layers": [
        ["SvgMarker", "base64:", True, 17.857, 20.0], ["FontMarker", "Arial", "A", 21.429, 15.0, "#aa0000", "#ffffff"]]}),
    "kentos-renk.sld": ("parsel", {"type": "singleSymbol", "layers": [["SimpleFill", "#808080"]]}),
    "kentos-sade.sld": ("yol", {"type": "singleSymbol", "colour": "#3366cc", "layer": "SimpleLine", "width": [2.857, 2]}),
}


def check_out():
    out_dir = DIR / "out"
    missing = [n for n in WANT if not (out_dir / n).exists()]
    if missing:
        return [f"{n} yok: önce KENTOS_WRITE_SLD=1 ile KentOS'un testini çalıştırın" for n in missing]
    py = QGIS / "MacOS/python3.12"
    if not py.exists():
        print("QGIS 4.2 yok; KentOS'un SLD'leri QGIS'te denetlenmedi.")
        return []
    files = [(n, t) for n, (t, _) in WANT.items()]
    code = QGIS_READ.format(q=QGIS, g=ROOT / "fixtures/geopackage/v1/qgis.gpkg", files=files, dir=out_dir)
    r = subprocess.run([str(py), "-c", code], env=qgis_env(), capture_output=True, text=True, timeout=120)
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
                problems.append(f"{name}: QGIS'te {k} {g.get(k)!r}, beklenen {v!r}")
    return problems


def main() -> None:
    if "--files" in sys.argv:
        write_files()
        return
    data = build_cases()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        problems = []
        if CASES.read_text(encoding="utf-8") != text:
            problems.append(f"{CASES.relative_to(ROOT)} güncel değil")
        problems += check_out()
        if problems:
            sys.exit("\n".join(problems))
        print(f"{len(data['cases'])} durum güncel; KentOS'un SLD'leri QGIS 4.2'de beklenen görünüşte.")
        return
    CASES.write_text(text, encoding="utf-8")
    print(f"{len(data['cases'])} durum yazıldı: {CASES.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
