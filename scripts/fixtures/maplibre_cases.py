#!/usr/bin/env python3
"""MapLibre stilinden görünüş al'ın ortak durumları ve KentOS'un yazdığı stillerin resmî doğrulayıcıyla denetimi
(docs/adr/0560), KentOS kodu olmadan.

    python3 scripts/fixtures/maplibre_cases.py --files   # elle yazılmış stilleri fixtures/maplibre/v1'e yazar
    python3 scripts/fixtures/maplibre_cases.py           # fixtures/commands/v1/file.import.maplibre.json'u yazar
    python3 scripts/fixtures/maplibre_cases.py --check   # yazılı durumlar güncel mi; KentOS'un stilleri doğrulayıcıda

Okuma kuralları ADR'den, bu betikte yeniden yazılmıştır (json ile): stil katmanları (fill, line, circle), süzgeçler
(ifade ve eski biçim) KentOS'un ifadesine, `match` kategorili, `step` aralıklı, süzgeçli katmanlar kurallı görünüşe;
renkler CSS'ten #rrggbb(aa)'ya; pikseller piksel, yakınlaştırmayla büyüyen genişlikler metre; yakınlaştırma düzeyleri
ölçek paydasına. Alınamayanların cümleleri de ADR'nin sözleridir.

KentOS'un yazdığı stilleri (`KENTOS_WRITE_MAPLIBRE=1 cargo test -p kentos-headless --test headless maplibre` yazar,
fixtures/maplibre/v1/out) iki şeyle denetler:

1. MapLibre'nin resmî stil belirtimi doğrulayıcısı (`@maplibre/maplibre-gl-style-spec` 26.4.4, npm ile
   `.run/maplibre-spec`'e kurulur; depoya bağımlılık eklenmez): stil geçerli olmalı.
2. Aynı paketin ifade değerlendiricisi: stilin süzgeçleri ve boya değerleri örnek nesnelerde değerlendirilir ve
   KentOS'un görünüşünün (burada Python'la yeniden yazılan kuralı) çizeceğiyle karşılaştırılır.
"""

from __future__ import annotations

import json
import math
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import qml_cases as q  # noqa: E402  (the filter text and the numbers' text of the QML reference)

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/maplibre/v1"
CASES = ROOT / "fixtures/commands/v1/file.import.maplibre.json"
SPEC_DIR = ROOT / ".run/maplibre-spec"
SPEC_VERSION = "26.4.4"

METRES_PER_PX_Z0 = 78_271.516_964_020_48
SCALE_Z0 = 279_541_132.014_358_9
PX_PER_MM = 96.0 / 25.4

num = q.num
kentos_text = q.kentos_text


def rnd(x: float, k: float = 1.0) -> float:
    """Round half away from zero, as Rust's `round` does."""
    y = x * k
    return math.copysign(math.floor(abs(y) + 0.5), y) / k


def round4(v):
    return rnd(v, 10000.0)


def round3(v):
    return rnd(v, 1000.0)


# ─── Colours ──────────────────────────────────────────────────────────────────────────────────────────

NAMED = {"black": "#000000", "white": "#ffffff", "red": "#ff0000", "green": "#008000", "lime": "#00ff00",
         "blue": "#0000ff", "yellow": "#ffff00", "cyan": "#00ffff", "aqua": "#00ffff", "magenta": "#ff00ff",
         "fuchsia": "#ff00ff", "gray": "#808080", "grey": "#808080", "orange": "#ffa500", "purple": "#800080",
         "pink": "#ffc0cb", "brown": "#a52a2a"}


def hex_colour(s: str):
    if not s.startswith("#"):
        return None
    h = s[1:]
    if not all(c in "0123456789abcdefABCDEF" for c in h):
        return None
    if len(h) in (3, 4):
        h = "".join(c + c for c in h)
    if len(h) not in (6, 8):
        return None
    rgb = "#" + h[:6].lower()
    a = int(h[6:8], 16) / 255.0 if len(h) == 8 else 1.0
    return rgb, a


def css_in(text: str):
    t = text.strip().lower()
    if t.startswith("#"):
        return hex_colour(t)
    if t in NAMED:
        return NAMED[t], 1.0
    if t == "transparent":
        return "#000000", 0.0
    if "(" not in t or not t.endswith(")"):
        return None
    func, args = t.split("(", 1)
    args = args[:-1]
    import re
    nums = [x for x in re.split(r"[,/\s]+", args) if x]

    def part(s, full):
        if s.endswith("%"):
            try:
                return float(s[:-1]) / 100.0 * full
            except ValueError:
                return None
        s = s[:-3] if s.endswith("deg") else s
        try:
            return float(s)
        except ValueError:
            return None

    def alpha(i):
        if i < len(nums):
            a = part(nums[i], 1.0)
            return None if a is None else min(max(a, 0.0), 1.0)
        return 1.0

    if func in ("rgb", "rgba") and len(nums) >= 3:
        c = [part(nums[i], 255.0) for i in range(3)]
        if any(x is None for x in c):
            return None
        a = alpha(3)
        if a is None:
            return None
        return "#%02x%02x%02x" % tuple(int(min(max(rnd(x), 0), 255)) for x in c), a
    if func in ("hsl", "hsla") and len(nums) >= 3:
        h = part(nums[0], 360.0)
        s = part(nums[1], 1.0)
        l = part(nums[2], 1.0)
        if None in (h, s, l):
            return None
        h = (h % 360.0) / 360.0
        s = min(max(s, 0.0), 1.0)
        l = min(max(l, 0.0), 1.0)
        qq = l * (1.0 + s) if l < 0.5 else l + s - l * s
        p = 2.0 * l - qq

        def channel(t):
            if t < 0.0:
                t += 1.0
            if t > 1.0:
                t -= 1.0
            if t < 1.0 / 6.0:
                v = p + (qq - p) * 6.0 * t
            elif t < 0.5:
                v = qq
            elif t < 2.0 / 3.0:
                v = p + (qq - p) * (2.0 / 3.0 - t) * 6.0
            else:
                v = p
            return int(min(max(rnd(v * 255.0), 0), 255))

        a = alpha(3)
        if a is None:
            return None
        return "#%02x%02x%02x" % (channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)), a
    return None


def colour_with(rgb: str, a):
    if a is not None and a < 1.0:
        return rgb.lower() + "%02x" % int(rnd(min(max(a, 0.0), 1.0) * 255.0))
    return rgb.lower()


def kcolour(c, opacity: float) -> str:
    return colour_with(c[0], min(max(c[1] * opacity, 0.0), 1.0))


def css_rgba(rgb: str, a: float) -> str:
    if a >= 0.9995:
        return rgb
    h = rgb[1:]
    return "rgba(%d,%d,%d,%s)" % (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), num(round3(min(max(a, 0.0), 1.0))))


# ─── Expressions back into filters ────────────────────────────────────────────────────────────────────


def is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def lit_of(v):
    if is_num(v):
        return ("num", float(v))
    if isinstance(v, str):
        return ("text", v)
    if isinstance(v, bool):
        return ("text", "true" if v else "false")
    return None


def operand_field(v):
    if not isinstance(v, list) or not v or not isinstance(v[0], str):
        return None
    h = v[0]
    if h == "get" and len(v) == 2:
        return v[1] if isinstance(v[1], str) else None
    if h in ("to-number", "to-string", "number", "string") and len(v) == 2:
        return operand_field(v[1])
    return None


FLIP = {"<": ">", "<=": ">=", ">": "<", ">=": "<=", "=": "=", "<>": "<>"}
OPS = {"==": "=", "!=": "<>", "<": "<", "<=": "<=", ">": ">", ">=": ">="}


def labels_of(v):
    if isinstance(v, list):
        if len(v) == 2 and v[0] == "literal":
            return labels_of(v[1])
        out = [lit_of(x) for x in v]
        return None if any(x is None for x in out) else out
    li = lit_of(v)
    return None if li is None else [li]


def or_of_eq(field, labels):
    parts = [("cmp", "=", field, li) for li in labels]
    return parts[0] if len(parts) == 1 else ("or", parts)


def geometry_of(v):
    if not isinstance(v, list) or not v or not isinstance(v[0], str):
        return None

    def kinds(lits):
        out = []
        for li in lits:
            if li is None or li[0] != "text":
                return None
            t = li[1]
            while t.startswith("Multi"):
                t = t[5:]
            if t not in out:
                out.append(t)
        return out

    def is_geom(x):
        return x == ["geometry-type"] or x == "$type"

    h = v[0]
    if h == "==" and len(v) == 3:
        if is_geom(v[1]):
            return kinds([lit_of(v[2])])
        if is_geom(v[2]):
            return kinds([lit_of(v[1])])
        return None
    if h == "match" and len(v) == 5 and is_geom(v[1]) and v[3] is True and v[4] is False:
        ls = labels_of(v[2])
        return None if ls is None else kinds(ls)
    if h == "in" and len(v) == 3 and is_geom(v[1]):
        ls = labels_of(v[2])
        return None if ls is None else kinds(ls)
    if h == "in" and len(v) > 2 and v[1] == "$type":
        return kinds([lit_of(x) for x in v[2:]])
    return None


def has_of(v):
    if isinstance(v, list) and len(v) == 2 and v[0] == "has" and isinstance(v[1], str):
        return v[1]
    return None


def null_pattern(not_there, empty):
    if not isinstance(not_there, list) or len(not_there) != 2 or not_there[0] != "!":
        return None
    f = has_of(not_there[1])
    if f is None:
        return None
    if isinstance(empty, list) and len(empty) == 3 and empty[0] == "==" and operand_field(empty[1]) == f \
            and empty[2] == "":
        return ("null", f)
    return None


def like_of(v):
    if not isinstance(v, list) or len(v) != 3:
        return None
    op, x, y = v
    if not isinstance(x, list) or not x or not isinstance(x[0], str):
        return None
    if op == "==" and x[0] == "slice":
        field = operand_field(x[1]) if len(x) > 1 else None
        if field is None or not isinstance(y, str):
            return None
        n = len(y)
        if len(x) == 4 and x[2] == 0 and x[3] == n:
            return ("like", field, y + "%")
        if len(x) == 3 and isinstance(x[2], list) and len(x[2]) == 3 and x[2][0] == "-" and x[2][2] == n \
                and isinstance(x[2][1], list) and x[2][1] and x[2][1][0] == "length":
            return ("like", field, "%" + y)
        return None
    if op == "!=" and x[0] == "index-of" and y == -1 and len(x) > 2:
        field = operand_field(x[2])
        if field is not None and isinstance(x[1], str):
            return ("like", field, "%" + x[1] + "%")
    return None


def mentions(f, field):
    k = f[0]
    if k == "cmp":
        return f[2] == field
    if k in ("between", "null", "like"):
        return f[1] == field
    if k in ("and", "or"):
        return any(mentions(p, field) for p in f[1])
    return mentions(f[1], field)


def tidy(parts):
    kept = []
    for i, p in enumerate(parts):
        if p[0] == "not" and p[1][0] == "null":
            field = p[1][1]
            if any(j != i and mentions(r, field) and not (r[0] == "not" and r[1][0] == "null")
                   for j, r in enumerate(parts)):
                continue
        if p not in kept:
            kept.append(p)
    if len(kept) == 2 and kept[0][0] == "cmp" and kept[1][0] == "cmp" and kept[0][1] == ">=" and kept[1][1] == "<=" \
            and kept[0][2] == kept[1][2] and kept[0][3][0] == "num" and kept[1][3][0] == "num":
        return ("between", kept[0][2], kept[0][3][1], kept[1][3][1])
    return kept[0] if len(kept) == 1 else ("and", kept)


def to_filter(v):
    if not isinstance(v, list) or not v or not isinstance(v[0], str):
        return None
    name = v[0]
    if name in OPS and len(v) == 3:
        op = OPS[name]

        def key(x):
            return x if isinstance(x, str) and not x.startswith("$") else None

        if operand_field(v[1]) is not None:
            field, lit_v = operand_field(v[1]), v[2]
        elif operand_field(v[2]) is not None:
            field, lit_v, op = operand_field(v[2]), v[1], FLIP[op]
        elif key(v[1]) is not None:
            field, lit_v = key(v[1]), v[2]
        else:
            return like_of(v)
        if lit_v is None:
            return ("null", field) if op == "=" else (("not", ("null", field)) if op == "<>" else None)
        li = lit_of(lit_v)
        if li is None:
            return None
        if li[0] == "text" and li[1] == "" and op == "=":
            return ("null", field)
        if li[0] == "text" and li[1] == "" and op == "<>":
            return ("not", ("null", field))
        return ("cmp", op, field, li)
    if name == "all":
        parts = []
        for p in v[1:]:
            f = to_filter(p)
            if f is None:
                return None
            if f[0] == "and":
                parts.extend(f[1])
            else:
                parts.append(f)
        return tidy(parts)
    if name == "any":
        if len(v) == 3:
            n = null_pattern(v[1], v[2])
            if n is not None:
                return n
        parts = [to_filter(p) for p in v[1:]]
        if any(p is None for p in parts):
            return None
        return parts[0] if len(parts) == 1 else ("or", parts)
    if name == "!" and len(v) == 2:
        f = has_of(v[1])
        if f is not None:
            return ("null", f)
        inner = to_filter(v[1])
        return None if inner is None else ("not", inner)
    if name == "has" and len(v) == 2 and isinstance(v[1], str):
        return ("not", ("null", v[1]))
    if name == "!has" and len(v) == 2 and isinstance(v[1], str):
        return ("null", v[1])
    if name in ("in", "!in"):
        if len(v) == 3 and operand_field(v[1]) is not None:
            field, labels = operand_field(v[1]), labels_of(v[2])
        elif len(v) >= 2 and isinstance(v[1], str) and not v[1].startswith("$"):
            field, labels = v[1], [lit_of(x) for x in v[2:]]
            if any(x is None for x in labels):
                labels = None
        else:
            return None
        if labels is None:
            return None
        f = or_of_eq(field, labels)
        return ("not", f) if name == "!in" else f
    if name == "match" and len(v) == 5:
        field = operand_field(v[1])
        if field is None:
            return None
        ls = labels_of(v[2])
        if ls is None:
            return None
        f = or_of_eq(field, ls)
        if v[3] is True and v[4] is False:
            return f
        if v[3] is False and v[4] is True:
            return ("not", f)
        return None
    return None


def filter_in(v):
    """(filter, geometry) or an error text."""
    if v is None or v is True:
        return None, None
    parts = v[1:] if isinstance(v, list) and v and v[0] == "all" else [v]
    geometry = None
    rest = []
    for p in parts:
        g = geometry_of(p)
        if g is not None:
            geometry = g
        else:
            rest.append(p)
    if not rest:
        return None, geometry
    f = to_filter(["all"] + rest)
    if f is None:
        return ("süzgeci KentOS'un ifadesine çevrilemedi (yalnız alanın değerinin karşılaştırmaları, boşluk, in, match "
                "ve all, any, ! okunur)")
    return f, geometry


# ─── Values ───────────────────────────────────────────────────────────────────────────────────────────


def prop_of(v, key, lost, what):
    if not isinstance(v, list) or not v or not isinstance(v[0], str):
        return ("const", v)
    name = v[0]
    if name == "literal":
        return ("const", v[1]) if len(v) > 1 else None
    if name in ("rgb", "rgba") and len(v) >= 4:
        if not all(is_num(x) for x in v[1:4]):
            return None
        c = [int(min(max(rnd(x), 0), 255)) for x in v[1:4]]
        a = v[4] if len(v) > 4 and is_num(v[4]) else 1.0
        rgb = "#%02x%02x%02x" % tuple(c)
        return ("const", css_rgba(rgb, a))
    if name == "to-color" and len(v) == 2 and isinstance(v[1], str):
        return ("const", v[1])
    if name == "match" and len(v) >= 5 and len(v) % 2 == 1:
        field = operand_field(v[1])
        if field is None:
            return None
        arms = []
        i = 2
        while i + 1 < len(v):
            ls = labels_of(v[i])
            if ls is None:
                return None
            arms.append(([num(x[1]) if x[0] == "num" else x[1] for x in ls], v[i + 1]))
            i += 2
        return ("match", field, arms, v[-1])
    if name == "step" and len(v) >= 3 and len(v) % 2 == 1:
        field = operand_field(v[1])
        if field is None:
            return None
        stops = []
        i = 3
        while i + 1 < len(v):
            if not is_num(v[i]):
                return None
            stops.append((float(v[i]), v[i + 1]))
            i += 2
        return ("step", field, v[2], stops)
    if name == "interpolate" and len(v) >= 5 and v[2] == ["zoom"]:
        pairs = v[3:]
        stops = [(float(pairs[k]), float(pairs[k + 1])) for k in range(0, len(pairs) - 1, 2)
                 if is_num(pairs[k]) and is_num(pairs[k + 1])]
        base2 = isinstance(v[1], list) and len(v[1]) > 1 and v[1][0] == "exponential" and v[1][1] == 2.0
        if base2 and len(stops) >= 2 and len(stops) * 2 == len(v) - 3:
            z0, v0 = stops[0]
            w0 = v0 / (2.0 ** z0)
            if all(abs(val - w0 * (2.0 ** z)) <= 1e-6 * max(abs(val), abs(w0), 1e-300) for z, val in stops):
                return ("metres", w0 * METRES_PER_PX_Z0)
        lost.append(f"{what}: “{key}” yakınlaştırmaya göre değişen bir eğri; ilk durağının değeri alındı")
        return ("const", v[4]) if len(v) > 4 else None
    lost.append(f"{what}: “{key}” değerindeki “{name}” ifadesi KentOS'ta yok; değer alınmadı")
    return None


def prop_at(p, cls):
    k = p[0]
    if k == "const":
        return p[1]
    if k == "metres":
        return {"metres": p[1]}
    if k == "match":
        if cls[0] == "value":
            for labels, out in p[2]:
                if cls[1] in labels:
                    return out
        return p[3]
    if k == "step":
        if cls[0] == "low":
            val = p[2]
            for t, out in p[3]:
                if t <= cls[1]:
                    val = out
            return val
        return p[2]
    return None


READ_KEYS = {"fill-color", "fill-opacity", "fill-outline-color", "fill-antialias", "line-color", "line-width",
             "line-opacity", "line-dasharray", "circle-radius", "circle-color", "circle-opacity",
             "circle-stroke-color", "circle-stroke-width", "circle-stroke-opacity", "circle-pitch-alignment",
             "circle-pitch-scale", "fill-sort-key"}
LAYOUT_OK = {"visibility", "line-cap", "line-join", "line-miter-limit", "line-round-limit", "circle-sort-key",
             "fill-sort-key", "line-sort-key"}


def scale_of_zoom(z):
    return rnd(SCALE_Z0 / (2.0 ** z))


class Raw:
    def __init__(self, src, filt, geometry, paint, disabled, min_scale, max_scale):
        self.src, self.filter, self.geometry, self.paint = src, filt, geometry, paint
        self.disabled, self.min_scale, self.max_scale = disabled, min_scale, max_scale

    def value(self, key, cls):
        for k, p in self.paint:
            if k == key:
                return prop_at(p, cls)
        return None

    def number(self, key, cls):
        v = self.value(key, cls)
        return float(v) if is_num(v) else None

    def colour(self, key, cls):
        v = self.value(key, cls)
        return css_in(v) if isinstance(v, str) else None

    def length(self, key, cls):
        v = self.value(key, cls)
        if isinstance(v, dict) and "metres" in v:
            return ("m", v["metres"])
        if is_num(v):
            return ("px", float(v))
        return None


def raw_of(l, lost):
    what = f"“{l['id']}” katmanı"
    kind = l["type"]
    if kind not in ("fill", "line", "circle"):
        if kind == "symbol":
            lost.append(f"{what}: etiket katmanı alınmadı; KentOS'ta etiketler ayrıdır")
        else:
            lost.append(f"{what}: “{kind}” katmanı KentOS'un çizebileceği bir katman değil, alınmadı")
        return None
    fin = filter_in(l.get("filter"))
    if isinstance(fin, str):
        lost.append(f"{what}: {fin}; katman alınmadı")
        return None
    filt, geometry = fin
    paint = []
    ignored = []
    for k in sorted(l.get("paint", {})):
        if k not in READ_KEYS:
            ignored.append(k)
            continue
        p = prop_of(l["paint"][k], k, lost, what)
        if p is not None:
            paint.append((k, p))
    colour_key = {"fill": "fill-color", "line": "line-color"}.get(kind, "circle-color")
    if colour_key in l.get("paint", {}) and not any(k == colour_key for k, _ in paint):
        lost.append(f"{what}: “{colour_key}” okunamadığı için katman alınmadı")
        return None
    layout = l.get("layout", {})
    for k in sorted(layout):
        if k not in LAYOUT_OK:
            ignored.append(k)
    if ignored:
        lost.append(f"{what}: {', '.join(sorted(ignored))} özellikleri alınmadı")
    min_zoom, max_zoom = l.get("minzoom"), l.get("maxzoom")
    return Raw(l, filt, geometry, paint, layout.get("visibility") == "none",
               scale_of_zoom(max_zoom) if max_zoom is not None else None,
               scale_of_zoom(min_zoom) if min_zoom is not None and min_zoom > 0 else None)


# ─── Symbols ──────────────────────────────────────────────────────────────────────────────────────────


def len_parts(l):
    return (round4(l[1]), l[0])


def draw(raw, cls, others_fill, lost):
    src = raw.src
    what = f"“{src['id']}” katmanı"
    hint = src.get("metadata", {}).get("kentos:symbol")
    out = []
    kind = src["type"]
    layout = src.get("layout", {})
    if kind == "fill":
        colour = raw.colour("fill-color", cls) or ("#000000", 1.0)
        opacity = min(max(raw.number("fill-opacity", cls) if raw.number("fill-opacity", cls) is not None else 1.0, 0), 1)
        out.append(("fill", {"type": "simpleFill", "color": kcolour(colour, 1.0)}, opacity))
        o = raw.colour("fill-outline-color", cls)
        if o:
            out.append(("fill", {"type": "simpleLine", "color": kcolour(o, 1.0), "width": 1, "unit": "px"}, opacity))
    elif kind == "line":
        colour = raw.colour("line-color", cls) or ("#000000", 1.0)
        opv = raw.number("line-opacity", cls)
        opacity = min(max(opv if opv is not None else 1.0, 0), 1)
        width = raw.length("line-width", cls) or ("px", 1.0)
        w, unit = len_parts(width)
        l = {"type": "simpleLine", "color": kcolour(colour, 1.0), "width": w, "unit": unit}
        d = raw.value("line-dasharray", cls)
        if isinstance(d, list):
            ds = [float(x) for x in d if is_num(x)]
            if ds:
                l["dash"] = [round4(r * w) for r in ds]
        cap = layout.get("line-cap")
        l["cap"] = "round" if cap == "round" else ("square" if cap == "square" else "butt")
        join = layout.get("line-join")
        l["join"] = "round" if join == "round" else ("bevel" if join == "bevel" else "miter")
        geom = raw.geometry
        if hint == "fill":
            on_polygons = True
        elif hint is not None:
            on_polygons = False
        elif geom is not None:
            on_polygons = "Polygon" in geom and "LineString" not in geom
        else:
            on_polygons = others_fill
        out.append(("fill" if on_polygons else "line", l, opacity))
    else:
        radius = raw.length("circle-radius", cls) or ("px", 5.0)
        r, unit = len_parts(radius)
        fo = raw.number("circle-opacity", cls)
        fill_o = min(max(fo if fo is not None else 1.0, 0), 1)
        so = raw.number("circle-stroke-opacity", cls)
        stroke_o = min(max(so if so is not None else 1.0, 0), 1)
        fill = raw.colour("circle-color", cls) or ("#000000", 1.0)
        stroke = raw.colour("circle-stroke-color", cls) or ("#000000", 1.0)
        sw = raw.length("circle-stroke-width", cls)
        stroke_on = sw is not None and sw[1] > 0 and stroke_o > 0
        m = {"type": "shape", "shape": "circle", "size": round4(r * 2.0), "unit": unit,
             "fill": kcolour(fill, 1.0) if fill_o > 0 else None}
        if stroke_on:
            w, su = len_parts(sw)
            if su != unit:
                lost.append(f"{what}: çemberin kenar kalınlığının birimi boyununkinden başka; kalınlık alınmadı")
            else:
                m["stroke"] = kcolour(stroke, 1.0)
                m["strokeWidth"] = w
        if fill_o > 0 and stroke_on and abs(fill_o - stroke_o) > 1e-9:
            m["fill"] = kcolour(fill, fill_o)
            m["stroke"] = kcolour(stroke, stroke_o)
            opacity = 1.0
        elif fill_o > 0:
            opacity = fill_o
        else:
            opacity = stroke_o
        out.append(("marker", m, opacity))
    return out


def set_of(drawn, disabled):
    result = {}
    for key in ("fill", "line", "marker"):
        mine = [(i, d) for i, d in enumerate(drawn) if d[0] == key]
        if not mine:
            continue
        common = mine[0][1][2]
        same = all(abs(d[2] - common) < 1e-9 for _, d in mine)
        layers = []
        for k, (i, d) in enumerate(mine):
            l = dict(d[1])
            l["id"] = f"{key[0]}{k}"
            if not same and d[2] < 1.0:
                for field in ("color", "fill", "stroke"):
                    v = l.get(field)
                    if isinstance(v, str):
                        c = css_in(v)
                        if c:
                            l[field] = colour_with(c[0], min(max(c[1] * d[2], 0.0), 1.0))
            if disabled[i]:
                l["enabled"] = False
            layers.append(l)
        sym = {"type": key, "layers": layers}
        if same and common < 1.0:
            sym["opacity"] = round3(common)
        result[key] = sym
    return result


def symbols_at(raws, cls, lost):
    fill_on = any(r.src["type"] == "fill" for r in raws)
    drawn, disabled = [], []
    for r in raws:
        for d in draw(r, cls, fill_on, lost):
            drawn.append(d)
            disabled.append(r.disabled)
    return set_of(drawn, disabled)


# ─── Renderers ────────────────────────────────────────────────────────────────────────────────────────


def selector_of(raws, lost):
    found = None
    for r in raws:
        for k, p in r.paint:
            if p[0] not in ("match", "step"):
                continue
            sel = ("cat" if p[0] == "match" else "step", p[1])
            if found is None:
                found = sel
            elif found != sel:
                lost.append(f"“{r.src['id']}” katmanının “{k}” değeri başka bir alana ya da türe bağlı; "
                            "yedek değeri alındı")
    return found


def lit_k(lit):
    return num(lit[1]) if lit[0] == "num" else lit[1]


def split_members(filt, field):
    def members(f):
        def lit(p):
            if p[0] == "cmp" and p[1] == "=" and p[2] == field:
                return lit_k(p[3])
            return None

        if f[0] == "or":
            ms = [lit(p) for p in f[1]]
            return None if any(m is None for m in ms) else ms
        m = lit(f)
        return None if m is None else [m]

    if filt is None:
        return None, None
    m = members(filt)
    if m is not None:
        return m, None
    if filt[0] == "and":
        for i, p in enumerate(filt[1]):
            m = members(p)
            if m is not None:
                rest = [x for j, x in enumerate(filt[1]) if j != i]
                return m, rest[0] if len(rest) == 1 else ("and", rest)
    return None, filt


def split_bounds(filt, field):
    if filt is None:
        return None, None, None
    parts = filt[1] if filt[0] == "and" else [filt]
    lo = hi = None
    rest = []
    for p in parts:
        if p[0] == "cmp" and p[2] == field and p[3][0] == "num" and p[1] == ">=" and lo is None:
            lo = p[3][1]
        elif p[0] == "cmp" and p[2] == field and p[3][0] == "num" and p[1] == "<=" and hi is None:
            hi = (p[3][1], True)
        elif p[0] == "cmp" and p[2] == field and p[3][0] == "num" and p[1] == "<" and hi is None:
            hi = (p[3][1], False)
        elif p[0] == "between" and p[1] == field and lo is None and hi is None:
            lo, hi = p[2], (p[3], True)
        elif p[0] == "not" and p[1][0] == "null" and p[1][1] == field:
            pass
        else:
            rest.append(p)
    r = None if not rest else (rest[0] if len(rest) == 1 else ("and", rest))
    return lo, hi, r


def conj(a, b):
    if a is not None and b is not None:
        return ("and", [a, b])
    return a if b is None else b


def label_of(raw, fallback):
    v = raw.src.get("metadata", {}).get("kentos:class")
    return v if isinstance(v, str) else fallback


def run_key(r):
    return (kentos_text(r.filter) if r.filter else None, r.min_scale, r.max_scale)


def runs(raws):
    out, last = [], object()
    for r in raws:
        k = run_key(r)
        if out and k == last:
            out[-1].append(r)
        else:
            out.append([r])
            last = k
    return out


def rules_value(rules):
    lst = []
    for i, r in enumerate(rules):
        v = {"id": f"r{i + 1}", "label": r["label"], "symbols": r["symbols"]}
        if r["is_else"]:
            v["isElse"] = True
        elif r["filter"] is not None:
            v["filter"] = kentos_text(r["filter"])
        if r["min_scale"] is not None:
            v["minScale"] = r["min_scale"]
        if r["max_scale"] is not None:
            v["maxScale"] = r["max_scale"]
        lst.append(v)
    return {"type": "rules", "rules": lst}


def recognised(rules):
    if len(rules) < 2 or any(r["min_scale"] is not None or r["max_scale"] is not None for r in rules):
        return None
    field = None
    cats = []
    other = None
    ok = True
    for r in rules:
        f = r["filter"]
        if f is not None and f[0] == "cmp" and f[1] == "=" and not r["is_else"]:
            if field is not None and field != f[2]:
                ok = False
                break
            field = f[2]
            cats.append({"value": lit_k(f[3]), "label": r["label"], "symbols": r["symbols"]})
        elif f is None and r["is_else"] and other is None:
            other = r["symbols"]
        else:
            ok = False
            break
    if ok and field is not None:
        v = {"type": "categorized", "expr": f"[{field}]", "categories": cats}
        if other is not None:
            v["other"] = other
        return v
    field = None
    classes = []
    n = len(rules)
    for k, r in enumerate(rules):
        if r["is_else"]:
            return None
        f = r["filter"]
        if f is not None and f[0] == "and" and len(f[1]) == 2 and f[1][0][0] == "cmp" and f[1][1][0] == "cmp" \
                and f[1][0][1] == ">=" and f[1][1][1] == "<" and f[1][0][2] == f[1][1][2] \
                and f[1][0][3][0] == "num" and f[1][1][3][0] == "num":
            fld, lo, hi, closed = f[1][0][2], f[1][0][3][1], f[1][1][3][1], False
        elif f is not None and f[0] == "between":
            fld, lo, hi, closed = f[1], f[2], f[3], True
        else:
            return None
        if closed != (k + 1 == n):
            return None
        if field is not None and field != fld:
            return None
        field = fld
        classes.append({"min": lo, "max": hi, "label": r["label"], "symbols": r["symbols"]})
    return {"type": "graduated", "expr": f"[{field}]", "classes": classes}


def generic(raws, lost):
    groups = runs(raws)
    rules = []
    for i, g in enumerate(groups):
        first = g[0]
        rules.append({"label": label_of(first, f"Kural {i + 1}"), "filter": first.filter, "is_else": False,
                      "min_scale": first.min_scale, "max_scale": first.max_scale,
                      "symbols": symbols_at(g, ("first",), lost)})
    if len(rules) == 1 and rules[0]["filter"] is None and rules[0]["min_scale"] is None \
            and rules[0]["max_scale"] is None:
        return {"type": "single", "symbols": rules[0]["symbols"]}, 1
    n = len(rules)
    rec = recognised(rules)
    return (rec if rec is not None else rules_value(rules)), n


def segments_of(raws, split, scale_ok=True):
    segs = []
    for i in range(len(raws)):
        if segs and split[segs[-1][0]] == split[i] and raws[segs[-1][0]].min_scale == raws[i].min_scale \
                and raws[segs[-1][0]].max_scale == raws[i].max_scale:
            segs[-1].append(i)
        else:
            segs.append([i])
    return segs


def lit_of_value(v):
    try:
        x = float(v)
    except ValueError:
        return ("text", v)
    return ("num", x) if num(x) == v else ("text", v)


def categories(raws, field, lost):
    split = [split_members(r.filter, field) for r in raws]
    segs = segments_of(raws, split)
    labels = {}
    for r in raws:
        m = r.src.get("metadata", {}).get("kentos:labels")
        if isinstance(m, dict):
            labels = m
            break

    def label_of_value(v):
        x = labels.get(v)
        return x if isinstance(x, str) else v

    direct = len(segs) == 1 and split[0][1] is None and raws[0].min_scale is None and raws[0].max_scale is None
    cats, other, rules, count = [], None, [], 0
    for seg in segs:
        members, rest = split[seg[0]]
        layers = [raws[i] for i in seg]
        values = []
        if members is not None:
            values.extend(members)
        else:
            for r in layers:
                for _, p in r.paint:
                    if p[0] == "match" and p[1] == field:
                        for ls, _out in p[2]:
                            for l in ls:
                                if l not in values:
                                    values.append(l)
        for v in values:
            symbols = symbols_at(layers, ("value", v), lost)
            label = label_of_value(v)
            count += 1
            if direct:
                cats.append({"value": v, "label": label, "symbols": symbols})
            else:
                rules.append({"label": label, "filter": conj(rest, ("cmp", "=", field, lit_of_value(v))),
                              "is_else": False, "min_scale": layers[0].min_scale,
                              "max_scale": layers[0].max_scale, "symbols": symbols})
        if members is None:
            symbols = symbols_at(layers, ("other",), lost)
            count += 1
            if direct:
                other = symbols
            else:
                anyf = [("cmp", "=", field, lit_of_value(v)) for v in values]
                none_of = ("not", anyf[0] if len(anyf) == 1 else ("or", anyf)) if anyf else None
                filt = conj(rest, none_of)
                rules.append({"label": "Diğerleri", "filter": filt, "is_else": filt is None,
                              "min_scale": layers[0].min_scale, "max_scale": layers[0].max_scale,
                              "symbols": symbols})
    if direct:
        v = {"type": "categorized", "expr": f"[{field}]", "categories": cats}
        if other is not None:
            v["other"] = other
        return v, count
    return rules_value(rules), count


def steps(raws, field, lost):
    cuts = []
    for r in raws:
        for _, p in r.paint:
            if p[0] == "step" and p[1] == field:
                cuts.extend(t for t, _ in p[3])
    cuts = sorted(set(cuts))
    split = [split_bounds(r.filter, field) for r in raws]
    segs = segments_of(raws, split)
    labels = []
    for r in raws:
        m = r.src.get("metadata", {}).get("kentos:labels")
        if isinstance(m, list):
            labels = [x if isinstance(x, str) else "" for x in m]
            break
    direct = len(segs) == 1 and split[0][0] is not None and split[0][1] is not None and split[0][1][1] \
        and split[0][2] is None and raws[0].min_scale is None and raws[0].max_scale is None
    classes, rules, count = [], [], 0
    for seg in segs:
        lo, hi, rest = split[seg[0]]
        layers = [raws[i] for i in seg]
        inside = [c for c in cuts if (lo is None or c > lo) and (hi is None or c < hi[0])]
        edges = [lo] + inside + [hi[0] if hi is not None else None]
        for k in range(len(edges) - 1):
            cls = ("low", edges[k]) if edges[k] is not None else ("first",)
            symbols = symbols_at(layers, cls, lost)
            a, b = edges[k], edges[k + 1]
            if a is not None and b is not None:
                default_label = f"{num(a)} - {num(b)}"
            elif a is not None:
                default_label = f"≥ {num(a)}"
            elif b is not None:
                default_label = f"< {num(b)}"
            else:
                default_label = "Tümü"
            label = labels[count] if count < len(labels) and labels[count] else default_label
            count += 1
            if direct and a is not None and b is not None:
                classes.append({"min": a, "max": b, "label": label, "symbols": symbols})
                continue
            parts = []
            if a is not None:
                parts.append(("cmp", ">=", field, ("num", a)))
            if b is not None:
                last = k + 2 == len(edges)
                op = "<=" if last and hi is not None and hi[1] else "<"
                parts.append(("cmp", op, field, ("num", b)))
            bound = None if not parts else (parts[0] if len(parts) == 1 else ("and", parts))
            rules.append({"label": label, "filter": conj(rest, bound), "is_else": False,
                          "min_scale": layers[0].min_scale, "max_scale": layers[0].max_scale, "symbols": symbols})
    if direct:
        return {"type": "graduated", "expr": f"[{field}]", "classes": classes}, count
    return rules_value(rules), count


def renderer_of(layers):
    """(renderer, rules, lost) or an error text."""
    lost = []
    raws = [r for r in (raw_of(l, lost) for l in layers) if r is not None]
    if not raws:
        return "Stilde KentOS'un çizebileceği (fill, line, circle) bir katman yok." + (
            "" if not lost else " " + "; ".join(lost))
    if any(r.min_scale is not None or r.max_scale is not None for r in raws):
        lost.append("yakınlaştırma düzeyleri ölçek paydasına yaklaşık çevrildi (OGC'nin 0,28 mm pikseli, ekvator, "
                    "512 pikselli karo)")
    sel = selector_of(raws, lost)
    if sel is None:
        renderer, rules = generic(raws, lost)
    elif sel[0] == "cat":
        renderer, rules = categories(raws, sel[1], lost)
    else:
        renderer, rules = steps(raws, sel[1], lost)
    if rules == 0:
        return "Stilde KentOS'un çizebileceği bir sınıf ya da kural yok."
    return renderer, rules, lost


# ─── The command ──────────────────────────────────────────────────────────────────────────────────────


def names(l, x):
    return l.get("source-layer") == x or l["id"] == x or l["id"].startswith(x + "/")


def matching(style, node, frm):
    out = []
    for l in style["layers"]:
        if frm is not None:
            ok = names(l, frm)
        else:
            k = l.get("metadata", {}).get("kentos:layer")
            ok = (k == node["id"]) if isinstance(k, str) else (names(l, node["id"]) or names(l, node["name"]))
        if ok:
            out.append(l)
    return out


def parse_style(text):
    try:
        root = json.loads(text)
    except ValueError:
        return "Dosya okunabilir bir JSON değil."
    if not isinstance(root, dict):
        return "Dosya bir MapLibre stili değil: kökü bir nesne değil."
    if "version" in root and root["version"] != 8:
        v = json.dumps(root["version"])
        return f"Dosya bir MapLibre stili değil ya da sürümü okunamıyor: version {v}, yalnız 8 okunur."
    if not isinstance(root.get("layers"), list):
        return "Stilde katman listesi (layers) yok."
    layers = [l for l in root["layers"] if isinstance(l, dict) and isinstance(l.get("id"), str)
              and isinstance(l.get("type"), str)]
    return {"layers": layers}


LAYER_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def layer(ident, name, **extra):
    return {"id": ident, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": dict(LAYER_STYLE), "children": [], **extra}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "MapLibre'den görünüş",
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

LEAVES = ["parsel", "yol", "durak", "imar-alan"]  # the layers a style may be for, in tree order


def node_of(ident):
    def walk(nodes):
        for n in nodes:
            if n["id"] == ident:
                return n
            r = walk(n["children"])
            if r:
                return r
        return None

    return walk(SETUP["layers"])


def layers_after(renderers):
    layers = json.loads(json.dumps(SETUP["layers"]))

    def walk(nodes):
        for n in nodes:
            if n["id"] in renderers:
                n["style"]["renderer"] = renderers[n["id"]]
            walk(n["children"])

    walk(layers)
    return layers


def read_text(name):
    return (DIR / name).read_text(encoding="utf-8")


def run_import(name, layer_id=None, frm=None, by_text=False):
    """The command's answer for the style `name`: (output, warnings, renderers) or (error_code, message, path)."""
    text = read_text(name)
    style = parse_style(text)
    if isinstance(style, str):
        return ("invalid_maplibre", style, "path")
    if layer_id is not None:
        node = node_of(layer_id)
        targets = [node]
    else:
        targets = [node_of(i) for i in LEAVES]
    many = layer_id is None
    taken, used, lost = [], set(), []
    for node in targets:
        matched = matching(style, node, frm)
        if not matched:
            continue
        used.update(l["id"] for l in matched)
        res = renderer_of(matched)
        if isinstance(res, str):
            return ("invalid_maplibre", f"“{node['name']}” katmanının stili okunamadı: {res}", "path")
        taken.append((node, matched, res))
    if not taken:
        if frm is not None:
            msg = (f"Stilde “{frm}” adlı (source-layer ya da id) bir katman yok. Stilin katmanlarının adlarına "
                   "bakın.")
        elif layer_id is not None:
            msg = (f"Stilde “{layer_id}” katmanı için bir stil katmanı yok: metadata kentos:layer, source-layer ya "
                   "da id katmanın kimliği ya da adı olmalı; başka bir ad için from verin.")
        else:
            msg = ("Stilde çizimin hiçbir katmanı için bir stil katmanı yok: metadata kentos:layer, source-layer ya "
                   "da id katmanların kimliği ya da adı olmalı.")
        return ("style_layer_not_found", msg, "path")
    if many:
        left = [l["id"] for l in style["layers"] if l["id"] not in used]
        if left:
            lost.append(f"{len(left)} stil katmanı çizimin hiçbir katmanına denk gelmedi, alınmadı: "
                        + ", ".join(left))
    prefixed = many or len(taken) > 1
    out_layers, renderers = [], {}
    for node, matched, (renderer, rules, rlost) in taken:
        out_layers.append({"layer": node["id"], "from": [l["id"] for l in matched], "kind": renderer["type"],
                           "rules": rules, "renderer": renderer, "changed": True})
        renderers[node["id"]] = renderer
        lost.extend(f"“{node['name']}” katmanı: {l}" if prefixed else l for l in rlost)
    output = {"layers": out_layers, "changed": True, "revision": "$current"}
    if lost:
        output["lost"] = lost
    warnings = [] if not lost else [{"code": "import_losses",
                                     "message": "Stilin bazı parçaları alınamadı: " + "; ".join(lost) + ".",
                                     "path": "path"}]
    return output, warnings, renderers


def input_of(name, layer_id=None, frm=None, by_text=False, **extra):
    inp = {}
    if layer_id is not None:
        inp["layer"] = layer_id
    if frm is not None:
        inp["from"] = frm
    if by_text:
        inp["text"] = read_text(name)
    else:
        inp["path"] = f"$fixture:maplibre/v1/{name}"
    inp.update(extra)
    return inp


def ok_case(title, name, layer_id=None, frm=None, by_text=False):
    out, warnings, renderers = run_import(name, layer_id, frm)
    n = len(renderers)
    return {"name": title, "steps": [{
        "op": "execute", "input": input_of(name, layer_id, frm, by_text),
        "result": {"status": "completed", "output": out, "warnings": warnings},
        "expect": {"ids": [1, 2, 3], "canUndo": True, "dirty": True, "revision": "changed",
                   "layers": layers_after(renderers)},
    }]}


def failed(title, inp, code, message, path):
    return {"name": title, "steps": [{
        "op": "execute", "input": inp,
        "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
        "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same"},
    }]}


def refused(title, name, layer_id="parsel", frm=None):
    res = run_import(name, layer_id, frm)
    assert res[0] in ("invalid_maplibre", "style_layer_not_found"), (name, res)
    return failed(title, input_of(name, layer_id, frm), res[0], res[1], res[2])


# ─── The hand-written styles ──────────────────────────────────────────────────────────────────────────

M_5 = 5.0 / METRES_PER_PX_Z0


def metres(m):
    w0 = m / METRES_PER_PX_Z0
    return ["interpolate", ["exponential", 2], ["zoom"], 0, w0, 24, w0 * (2.0 ** 24)]


def style(*layers, **extra):
    return {"version": 8, "name": "deneme", "sources": {}, "layers": list(layers), **extra}


HAND = {
    "kategorili.json": style(
        {"id": "parsel-dolgu", "type": "fill", "source": "s", "source-layer": "parsel",
         "paint": {"fill-color": ["match", ["get", "ada"], "A", "#e41a1c", ["B", "C"], "#377eb8", "#999999"],
                   "fill-opacity": 0.7}},
        {"id": "parsel-kenar", "type": "line", "source": "s", "source-layer": "parsel",
         "paint": {"line-color": "#222222", "line-width": 1.5}}),
    "aralikli.json": style(
        {"id": "alan-sinif", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["all", ["has", "alan"], [">=", ["to-number", ["get", "alan"]], 0],
                    ["<=", ["to-number", ["get", "alan"]], 5000]],
         "paint": {"fill-color": ["step", ["to-number", ["get", "alan"]], "#fee5d9", 500, "#fcae91", 1000, "#de2d26"]}},
        {"id": "alan-kenar", "type": "line", "source": "s", "source-layer": "parsel",
         "filter": ["all", ["has", "alan"], [">=", ["to-number", ["get", "alan"]], 0],
                    ["<=", ["to-number", ["get", "alan"]], 5000]],
         "metadata": {"kentos:symbol": "fill", "kentos:labels": ["Küçük", "Orta", "Büyük"]},
         "paint": {"line-color": ["step", ["to-number", ["get", "alan"]], "#a50f15", 500, "#67000d", 1000, "#000000"],
                   "line-width": 1}}),
    "acik-aralikli.json": style(
        {"id": "acik", "type": "fill", "source": "s", "source-layer": "parsel",
         "paint": {"fill-color": ["step", ["to-number", ["get", "alan"]], "#fee5d9", 500, "#fcae91", 1000, "#de2d26"]}}),
    "kurallar.json": style(
        {"id": "k1", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["==", ["get", "kullanim"], "Konut"], "paint": {"fill-color": "#1b9e77"}},
        {"id": "k2", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["all", [">", ["get", "alan"], 1000], ["!=", ["get", "ada"], "B"]], "minzoom": 14, "maxzoom": 19,
         "paint": {"fill-color": "#d95f02"}},
        {"id": "k3", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["!", ["in", ["get", "kullanim"], ["literal", ["Konut", "Ticaret"]]]],
         "paint": {"fill-color": "#7570b3"}}),
    "cizgi.json": style(
        {"id": "yol-1", "type": "line", "source": "s", "source-layer": "yol",
         "filter": ["==", "$type", "LineString"],
         "layout": {"line-cap": "round", "line-join": "round"},
         "paint": {"line-color": "#3366cc", "line-width": metres(5), "line-dasharray": [2, 1], "line-opacity": 0.8}},
        {"id": "yol-2", "type": "line", "source": "s", "source-layer": "yol",
         "paint": {"line-color": "#ffffff", "line-width": 3, "line-opacity": 0.8}}),
    "nokta.json": style(
        {"id": "durak-1", "type": "circle", "source": "s", "source-layer": "durak",
         "paint": {"circle-radius": 4, "circle-color": "#ff0000", "circle-stroke-color": "#000000",
                   "circle-stroke-width": 1, "circle-opacity": 0.5}},
        {"id": "durak-2", "type": "circle", "source": "s", "source-layer": "durak",
         "paint": {"circle-radius": 6, "circle-color": "#00ff00", "circle-opacity": 0,
                   "circle-stroke-color": "#0000ff", "circle-stroke-width": 2}}),
    "eski.json": style(
        {"id": "e1", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["all", ["==", "ada", "A"], ["has", "alan"]], "paint": {"fill-color": "#111111"}},
        {"id": "e2", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["in", "kullanim", "Konut", "Ticaret"], "paint": {"fill-color": "#222222"}},
        {"id": "e3", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["!has", "tapu"], "paint": {"fill-color": "#333333"}},
        {"id": "e4", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["!in", "ada", "A", "B"], "paint": {"fill-color": "#444444"}},
        {"id": "e5", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["all", [">=", ["to-number", ["get", "kat"]], 2], ["<=", ["to-number", ["get", "kat"]], 4],
                    ["has", "kat"]], "paint": {"fill-color": "#555555"}},
        {"id": "e6", "type": "fill", "source": "s", "source-layer": "parsel",
         "filter": ["all", ["has", "ad"], ["==", ["slice", ["to-string", ["get", "ad"]], 0, 2], "Ba"]],
         "paint": {"fill-color": "#666666"}}),
    "kayipli.json": style(
        {"id": "parsel/etiket", "type": "symbol", "source": "s", "layout": {"text-field": "{ad}"}},
        {"id": "parsel/yukseklik", "type": "fill-extrusion", "source": "s",
         "paint": {"fill-extrusion-height": 10}},
        {"id": "parsel/egri", "type": "fill", "source": "s",
         "paint": {"fill-color": "#abcdef", "fill-pattern": "desen",
                   "fill-opacity": ["interpolate", ["linear"], ["zoom"], 10, 0.2, 18, 1],
                   "fill-translate": [1, 2]}},
        {"id": "parsel/durum", "type": "fill", "source": "s",
         "paint": {"fill-color": ["case", ["has", "alan"], "#ff0000", "#00ff00"]}},
        {"id": "parsel/suzgec", "type": "fill", "source": "s",
         "filter": ["==", ["get", "a"], ["get", "b"]], "paint": {"fill-color": "#123456"}}),
    "renkler.json": style(
        {"id": "r1", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "#abc"}},
        {"id": "r2", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "rgb(255, 128, 0)"}},
        {"id": "r3", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "rgba(0,128,0,0.5)"}},
        {"id": "r4", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "hsl(240, 100%, 50%)"}},
        {"id": "r5", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "hsla(0, 0%, 50%, 0.25)"}},
        {"id": "r6", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "red"}},
        {"id": "r7", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "transparent"}},
        {"id": "r8", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": ["rgba", 10, 20, 30, 0.5]}},
        {"id": "r9", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "#11223344"}}),
    "geometri.json": style(
        {"id": "g1", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "#cccccc"}},
        {"id": "g2", "type": "line", "source-layer": "parsel",
         "filter": ["==", ["geometry-type"], "Polygon"], "paint": {"line-color": "#333333", "line-width": 2}},
        {"id": "g3", "type": "line", "source-layer": "parsel",
         "filter": ["==", ["geometry-type"], "LineString"], "paint": {"line-color": "#0000ff", "line-width": 4}},
        {"id": "g4", "type": "circle", "source-layer": "parsel",
         "filter": ["match", ["geometry-type"], ["Point", "MultiPoint"], True, False],
         "paint": {"circle-radius": 3, "circle-color": "#ff0000"}}),
    "kapali.json": style(
        {"id": "p1", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "#cccccc"}},
        {"id": "p2", "type": "line", "source-layer": "parsel", "layout": {"visibility": "none"},
         "paint": {"line-color": "#000000", "line-width": 1}}),
    "cok-katman.json": style(
        {"id": "arka-plan", "type": "background", "paint": {"background-color": "#ffffff"}},
        {"id": "p-dolgu", "type": "fill", "source-layer": "parsel", "paint": {"fill-color": "#e5d8bd"}},
        {"id": "yol-cizgi", "type": "line", "source-layer": "Yol",
         "paint": {"line-color": "#888888", "line-width": 2}},
        {"id": "durak/1", "type": "circle", "source": "s",
         "paint": {"circle-radius": 5, "circle-color": "#ff5500"}},
        {"id": "imar-alan", "type": "fill", "paint": {"fill-color": "#abd9e9"}},
        {"id": "okyanus", "type": "fill", "source-layer": "okyanus", "paint": {"fill-color": "#0000ff"}}),
    "kentos.json": style(
        {"id": "durak/1", "type": "circle", "source": "durak",
         "metadata": {"kentos:layer": "durak", "kentos:symbol": "marker", "kentos:labels": {"E": "Eczane"}},
         "filter": ["match", ["to-string", ["get", "tur"]], ["E", "H"], True, False],
         "paint": {"circle-radius": ["match", ["to-string", ["get", "tur"]], "E", 5, "H", 8, 8],
                   "circle-color": ["match", ["to-string", ["get", "tur"]], "E", "#e41a1c", "H", "#377eb8", "#377eb8"],
                   "circle-opacity": 1, "circle-stroke-color": "#000000", "circle-stroke-width": 0,
                   "circle-stroke-opacity": 0}}),
    "baska-ad.json": style(
        {"id": "building-fill", "type": "fill", "source": "s", "source-layer": "building",
         "paint": {"fill-color": "#d9d0c9"}},
        {"id": "road", "type": "line", "source": "s", "source-layer": "road",
         "paint": {"line-color": "#ffffff", "line-width": 2}}),
    "eslesmeyen.json": style(
        {"id": "baska", "type": "fill", "source-layer": "baska", "paint": {"fill-color": "#000000"}}),
    "cizilemeyen.json": style(
        {"id": "parsel/etiket", "type": "symbol", "source": "s", "layout": {"text-field": "{ad}"}},
        {"id": "parsel/arka", "type": "background"}),
    "surum7.json": {"version": 7, "sources": {}, "layers": []},
    "katmansiz.json": {"version": 8, "sources": {}},
    "bozuk.json": "{ \"version\": 8, \"layers\": [",
    "dizi.json": [1, 2, 3],
}


def write_files() -> None:
    DIR.mkdir(parents=True, exist_ok=True)
    for name, body in HAND.items():
        text = body if isinstance(body, str) else json.dumps(body, ensure_ascii=False, indent=1) + "\n"
        (DIR / name).write_text(text, encoding="utf-8")
    print(f"{len(HAND)} elle yazılmış stil: {DIR}")


def build_cases():
    cases = [
        ok_case("Üçüncü tarafın kategorili stili: match üç değer, yedek Diğerleri; kenar çizgisi dolgunun kenarı; "
                "saydamlık renge katılır", "kategorili.json", "parsel"),
        ok_case("Aralıklı stil: step, süzgecin sınırları aralığın uçları, KentOS'un yazdığı etiketler", "aralikli.json",
                "parsel"),
        ok_case("Sınırsız step: açık uçlu kurallar olur", "acik-aralikli.json", "parsel"),
        ok_case("Süzgeçli katmanlar kurallı görünüş: ifade süzgeçleri, yakınlaştırma ölçeğe, in'in değili", "kurallar.json",
                "parsel"),
        ok_case("Çizgi: eski $type süzgeci, metre genişliği, kesikler genişlik oranından, uç ve köşe", "cizgi.json",
                "yol"),
        ok_case("Çember: yarıçap çapa, dolgu ve kenarın saydamlıkları ayrı, dolgusuz çember", "nokta.json", "durak"),
        ok_case("Eski süzgeç biçimi: ==, in, !has, !in, bir çift sınır arasında, bir önek", "eski.json", "parsel"),
        ok_case("Alınamayanlar söylenir: etiket, fill-extrusion, zoom eğrisi, case, çevrilemeyen süzgeç, desen",
                "kayipli.json", "parsel"),
        ok_case("CSS renkleri: #abc, rgb, rgba, hsl, hsla, adlı, transparent, rgba ifadesi, sekiz haneli", "renkler.json",
                "parsel"),
        ok_case("Geometri süzgeçleri hangi simgenin olduğunu söyler", "geometri.json", "parsel"),
        ok_case("Görünmez stil katmanı kapalı katman olur", "kapali.json", "parsel"),
        ok_case("KentOS'un yazdığı biçim: metadata kentos:layer, üyelik süzgeci Diğerleri'ni kapatır, etiketler",
                "kentos.json", "durak"),
        ok_case("Başka bir ad: from ile source-layer", "baska-ad.json", "parsel", frm="building"),
        ok_case("Bütün katmanlar: kimliğe, ada, source-layer'a göre; denk gelmeyenler söylenir", "cok-katman.json"),
        ok_case("Stil dosya yerine metin olarak verilir", "kategorili.json", "parsel", by_text=True),
    ]
    out, warnings, renderers = run_import("kapali.json", "parsel")
    cases.append({"name": "Aynı stil ikinci kez: değişmez, adım yazılmaz", "steps": [
        {"op": "execute", "input": input_of("kapali.json", "parsel"),
         "result": {"status": "completed", "output": out, "warnings": warnings},
         "expect": {"canUndo": True, "revision": "changed"}},
        {"op": "execute", "input": input_of("kapali.json", "parsel"),
         "result": {"status": "completed",
                    "output": dict(out, layers=[dict(l, changed=False) for l in out["layers"]], changed=False),
                    "warnings": warnings},
         "expect": {"revision": "same", "layers": layers_after(renderers)}},
    ]})
    plan = run_import("aralikli.json", "parsel")
    cases.append({"name": "Plan okur, hiçbir şey yazmaz", "steps": [
        {"op": "plan", "input": input_of("aralikli.json", "parsel"),
         "result": {"status": "completed", "output": plan[0], "warnings": plan[1]},
         "expect": {"ids": [1, 2, 3], "canUndo": False, "dirty": False, "revision": "same", "layers": SETUP["layers"]}},
    ]})
    allin = run_import("cok-katman.json")
    cases.append({"name": "Birden çok katman tek geri alma adımıdır: geri alınır ve yinelenir", "steps": [
        {"op": "execute", "input": input_of("cok-katman.json"),
         "result": {"status": "completed", "output": allin[0], "warnings": allin[1]}},
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": SETUP["layers"]}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False, "layers": layers_after(allin[2])}},
    ]})
    cases += [
        failed("Ne yol ne metin", {"layer": "parsel"}, "invalid_input",
               "Stilin ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.", "path"),
        failed("Hem yol hem metin", {"layer": "parsel", "path": "$fixture:maplibre/v1/nokta.json", "text": "{}"},
               "invalid_input", "Stilin ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.", "path"),
        failed("Olmayan dosya", {"layer": "parsel", "path": "$fixture:maplibre/v1/yok.json"}, "file_unreadable",
               "“$fixture:maplibre/v1/yok.json” okunamadı: No such file or directory (os error 2). Dosyanın yerinde olduğunu denetleyin.",
               "path"),
        refused("JSON olmayan dosya", "bozuk.json"),
        refused("Nesne olmayan JSON", "dizi.json"),
        refused("Sürümü 8 olmayan stil", "surum7.json"),
        refused("Katman listesi olmayan stil", "katmansiz.json"),
        refused("Çizilebilecek katmanı olmayan stil (yalnız etiket ve arka plan)", "cizilemeyen.json"),
        refused("Katman için stil katmanı olmayan stil", "eslesmeyen.json"),
        refused("from ile olmayan ad", "baska-ad.json", frm="yok"),
        refused("Hiçbir katmana denk gelmeyen stil", "eslesmeyen.json", layer_id=None),
        failed("Olmayan katman", {"layer": "yok", "path": "$fixture:maplibre/v1/nokta.json"}, "layer_not_found",
               "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"),
        failed("Katman grubu", {"layer": "imar", "path": "$fixture:maplibre/v1/nokta.json"}, "not_a_layer",
               "“İmar” bir katman grubu; görünüş katmanındır. Grubun bir katmanını verin.", "layer"),
        failed("Servis katmanı", {"layer": "altlik", "path": "$fixture:maplibre/v1/nokta.json"}, "service_layer",
               "“OpenStreetMap” servisten çizilir ve nesne tutmaz; görünüş nesneleri olan katmanın olur.", "layer"),
    ]
    cases.append({"name": "Eski sürüme göre hazırlanmış istek: hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": input_of("nokta.json", "durak", expectedRevision="999"),
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
             "path": "expectedRevision", "revision": "$current"}},
         "expect": {"canUndo": False, "revision": "same"}},
    ]})
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.maplibre", "commandVersion": 1,
        "title": "MapLibre stilinden görünüş al",
        "note": "ADR 0560. Elle yazılmış MapLibre stilleri (fixtures/maplibre/v1); görünüşler ADR'nin okuma kuralıyla burada, json ile kurulur. Bağımsız başvuru: scripts/fixtures/maplibre_cases.py; elle düzenlemeyin.",
        "setup": SETUP,
        "cases": cases,
    }


# ─── KentOS's styles in the official validator and evaluator ─────────────────────────────────────────

NODE_EVAL = r"""
const spec = require('@maplibre/maplibre-gl-style-spec');
const input = JSON.parse(require('fs').readFileSync(process.argv[2], 'utf8'));
const GEOM = { Point: 1, LineString: 2, Polygon: 3 };
const out = [];
input.style.layers.forEach((layer, index) => {
  const entry = { id: layer.id, type: layer.type, minzoom: layer.minzoom, maxzoom: layer.maxzoom, features: [] };
  const filter = layer.filter ? spec.featureFilter(layer.filter, `layers[${index}].filter`) : null;
  const props = {};
  const group = spec.v8['paint_' + layer.type] || {};
  for (const [key, value] of Object.entries(layer.paint || {})) {
    if (!group[key]) continue;
    const ex = spec.expression.createPropertyExpression(value, `layers[${index}].paint.${key}`, group[key]);
    if (ex.result !== 'success') { props[key] = { error: JSON.stringify(ex.value) }; continue; }
    props[key] = ex.value;
  }
  for (const f of input.features) {
    const feature = { type: GEOM[f.type], properties: f.properties, geometry: [] };
    const visible = filter ? !!filter.filter({ zoom: input.zoom }, feature) : true;
    const values = {};
    for (const [key, p] of Object.entries(props)) {
      if (p.error) { values[key] = { error: p.error }; continue; }
      const v = p.evaluate({ zoom: input.zoom }, feature);
      values[key] = v && typeof v === 'object' && 'a' in v ? v.toString() : v;
    }
    entry.features.push({ visible, values });
  }
  out.push(entry);
});
console.log(JSON.stringify(out));
"""


def ensure_spec():
    marker = SPEC_DIR / "node_modules/@maplibre/maplibre-gl-style-spec/package.json"
    if marker.exists():
        return True
    SPEC_DIR.mkdir(parents=True, exist_ok=True)
    (SPEC_DIR / "package.json").write_text('{"name":"maplibre-spec","private":true}\n')
    r = subprocess.run(["npm", "install", "--no-audit", "--no-fund", "--prefix", str(SPEC_DIR),
                        f"@maplibre/maplibre-gl-style-spec@{SPEC_VERSION}"], capture_output=True, text=True)
    return r.returncode == 0 and marker.exists()


def validate(path: Path):
    r = subprocess.run([str(SPEC_DIR / "node_modules/.bin/gl-style-validate"), str(path)], capture_output=True,
                       text=True)
    return r.returncode == 0, (r.stdout + r.stderr).strip()


# The objects the evaluator draws: typed numbers and the same numbers as text.
FEATURES = [
    {"type": "Polygon", "properties": {"ada": "A", "alan": 420, "tapulu": 1}},
    {"type": "Polygon", "properties": {"ada": "B", "alan": 750, "tapulu": 0}},
    {"type": "Polygon", "properties": {"ada": "C", "alan": 1500, "tapulu": 1}},
    {"type": "Polygon", "properties": {"ada": "A", "alan": 9000}},
    {"type": "Polygon", "properties": {"ada": "D"}},
    {"type": "Polygon", "properties": {"ada": "B", "alan": "750", "tapulu": "0"}},
    {"type": "Polygon", "properties": {"ada": "A", "alan": 5000, "tapulu": "1"}},
    {"type": "Polygon", "properties": {"ada": "A", "alan": 500, "tapulu": 1}},
]


def check_out():
    out_dir = DIR / "out"
    names = ["kentos-kategorili.json", "kentos-aralikli.json", "kentos-kurallar.json", "kentos-cizgi.json",
             "kentos-nokta.json", "kentos-cok.json"]
    missing = [n for n in names if not (out_dir / n).exists()]
    if missing:
        return [f"{n} yok: önce KENTOS_WRITE_MAPLIBRE=1 ile KentOS'un testini çalıştırın" for n in missing]
    if not ensure_spec():
        print("npm ile resmî doğrulayıcı kurulamadı; KentOS'un stilleri doğrulanmadı.")
        return []
    problems = []
    for n in sorted(p.name for p in out_dir.glob("*.json")):
        ok, text = validate(out_dir / n)
        if not ok:
            problems.append(f"{n}: resmî doğrulayıcı: {text[:400]}")
    # The validator must reject what is not a style, or it checks nothing.
    bad = out_dir / "reddedilmeli.tmp.json"
    bad.write_text('{"version": 8, "sources": {}, "layers": [{"id": "x", "type": "fill", "source": "yok"}]}')
    ok, _ = validate(bad)
    bad.unlink()
    if ok:
        problems.append("doğrulayıcı bozuk bir stili geçirdi; denetim bir şey söylemiyor")
    return problems + evaluate_out(out_dir)


def evaluate(style, zoom):
    SPEC_DIR.joinpath("eval.js").write_text(NODE_EVAL)
    inp = SPEC_DIR / "eval-input.json"
    inp.write_text(json.dumps({"style": style, "features": FEATURES, "zoom": zoom}))
    r = subprocess.run(["node", str(SPEC_DIR / "eval.js"), str(inp)], capture_output=True, text=True,
                       cwd=str(SPEC_DIR))
    if r.returncode != 0:
        raise RuntimeError(r.stderr[-800:])
    return json.loads(r.stdout)


def colour_of(text):
    """`rgba(r,g,b,a)` of the evaluator as (#rrggbb, a)."""
    import re
    m = re.match(r"rgba\(([\d.]+),([\d.]+),([\d.]+),([\d.]+)\)", text.replace(" ", ""))
    r, g, b, a = (float(x) for x in m.groups())
    return "#%02x%02x%02x" % (round(r), round(g), round(b)), round(a, 3)


def drawn(layer, i, zoom):
    """Whether the layer draws feature `i` at `zoom`: its filter and its zoom range."""
    if layer.get("minzoom") is not None and zoom < layer["minzoom"]:
        return False
    if layer.get("maxzoom") is not None and zoom >= layer["maxzoom"]:
        return False
    return layer["features"][i]["visible"]


def num_of(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def evaluate_out(out_dir):
    """The styles' filters and paint values at the sample objects, against what KentOS's looks draw there: the
    looks are the headless test's, written again here as plain rules."""
    problems = []

    def get(name, zoom=16):
        style = json.loads((out_dir / name).read_text(encoding="utf-8"))
        return evaluate(style, zoom)

    # Categorized on `ada`: A, B in their colours; the rest the other's (50 % grey at 50 %).
    got = get("kentos-kategorili.json")
    fills = [e for e in got if e["type"] == "fill"]
    lines = [e for e in got if e["type"] == "line"]
    want = {"A": ("#e41a1c", 1.0), "B": ("#377eb8", 1.0)}
    for i, f in enumerate(FEATURES):
        ada = f["properties"].get("ada", "")
        w = want.get(ada, ("#999999", 0.502))
        if not drawn(fills[0], i, 16):
            problems.append(f"kategorili: {i}. nesne çizilmiyor")
            continue
        c = colour_of(fills[0]["features"][i]["values"]["fill-color"])
        if c[0] != w[0] or abs(c[1] - w[1]) > 0.01:
            problems.append(f"kategorili: {i}. nesnenin rengi {c}, beklenen {w}")
        wd = lines[0]["features"][i]["values"]["line-width"]
        if abs(wd - 0.4 * PX_PER_MM) > 1e-3:
            problems.append(f"kategorili: kenar kalınlığı {wd}")
    # Graduated on `alan`: [0, 500), [500, 1000), [1000, 5000]; outside and without the field, nothing.
    got = get("kentos-aralikli.json")
    fills = [e for e in got if e["type"] == "fill"]
    for i, f in enumerate(FEATURES):
        alan = num_of(f["properties"].get("alan"))
        if alan is None or not (0 <= alan <= 5000):
            want_c = None
        elif alan < 500:
            want_c = "#fee5d9"
        elif alan < 1000:
            want_c = "#fcae91"
        else:
            want_c = "#de2d26"
        is_drawn = drawn(fills[0], i, 16)
        if (want_c is None) == is_drawn:
            problems.append(f"aralıklı: {i}. nesne çizimi {is_drawn}, beklenen {want_c}")
        elif want_c is not None:
            c = colour_of(fills[0]["features"][i]["values"]["fill-color"])
            if c[0] != want_c:
                problems.append(f"aralıklı: {i}. nesnenin rengi {c[0]}, beklenen {want_c}")
    # Rules: `tapulu = 1`; its child `alan > 1000 and ada <> 'B'` at scales 500-10000 (zoom 14.8-19.1); the else.
    for zoom, child_on in ((16, True), (12, False)):
        got = get("kentos-kurallar.json", zoom)
        fills = [e for e in got if e["type"] == "fill"]
        for i, f in enumerate(FEATURES):
            p = f["properties"]
            tapulu = num_of(p.get("tapulu")) == 1
            alan = num_of(p.get("alan"))
            r1 = tapulu
            r2 = tapulu and alan is not None and alan > 1000 and p.get("ada", "") != "B" and child_on
            r3 = not tapulu
            for k, w in enumerate((r1, r2, r3)):
                if drawn(fills[k], i, zoom) != w:
                    problems.append(f"kurallar: zoom {zoom}, {i}. nesne, {k + 1}. kural {drawn(fills[k], i, zoom)}, "
                                    f"beklenen {w}")
    # The line: widths from millimetres and metres, the dashes in widths of the line.
    style = json.loads((out_dir / "kentos-cizgi.json").read_text(encoding="utf-8"))
    got = get("kentos-cizgi.json")
    lines = [e for e in got if e["type"] == "line"]
    w0 = lines[0]["features"][0]["values"]["line-width"]
    w1 = lines[1]["features"][0]["values"]["line-width"]
    if abs(w0 - 0.8 * PX_PER_MM) > 1e-3:
        problems.append(f"çizgi: 0,8 mm {w0} px")
    want_m = 2.0 / (METRES_PER_PX_Z0 / 2 ** 16)
    if abs(w1 - want_m) > 1e-6:
        problems.append(f"çizgi: 2 m zoom 16'da {w1} px, beklenen {want_m}")
    dash = style["layers"][0]["paint"].get("line-dasharray")
    if dash != [3.75, 1.875]:
        problems.append(f"çizgi: kesikler {dash}")
    # The marker: a circle's radius is half its size.
    got = get("kentos-nokta.json")
    circles = [e for e in got if e["type"] == "circle"]
    r = circles[0]["features"][0]["values"]["circle-radius"]
    if abs(r - 1.5 * PX_PER_MM) > 1e-3:
        problems.append(f"nokta: yarıçap {r}, beklenen {1.5 * PX_PER_MM}")
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
        print(f"{len(data['cases'])} durum güncel; KentOS'un stilleri resmî doğrulayıcıda geçerli ve beklenen görünüşte.")
        return
    CASES.write_text(text, encoding="utf-8")
    print(f"{len(data['cases'])} durum: {CASES.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
