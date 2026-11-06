#!/usr/bin/env python3
"""AutoCAD plot style tables (`.ctb`/`.stb`) for `cad.plotstyles.import` (docs/adr/1130), without KentOS code.

Writes the test tables in fixtures/commands/v1/plotstyles/ from the documented file format and
fixtures/commands/v1/cad.plotstyles.import.json from this file's own reader; `--check` compares both and writes nothing.
`--read FILE` prints what the reader makes of any table (a profile and its report) as JSON.

The file format (as the format's reference reader and writer, ezdxf `addons/acadctb`, give it; MIT): 48 bytes
`PIAFILEVERSION_2.0,CTBVER1,compress\\r\\npmzlibcodec` (or `STBVER1`), three little-endian u32 — the Adler-32 of the
compressed body, the inflated length (the text and its closing NUL) and the compressed length — then a zlib stream of
text: lines `key=value`, blocks `name{` … `}`; a string value opens with `"` and has no closing one; another value may
carry a ` (…)` appendix, dropped. A CTB has `aci_table_available=TRUE`, 255 styles `0{`…`254{` for ACI 1–255; a STB
`FALSE` and named styles, Normal first. A style's fields: color (−1 the object's; else the signed 32-bit 0xTTRRGGBB,
TT 0xC2 RGB, 0xC3 ACI), mode_color, color_policy (bits 1 dither, 2 grayscale, 4 named colour), physical_pen_number,
virtual_pen_number (0 automatic), screen (0–100), linepattern_size, linetype (0 solid, 1–30 AutoCAD's patterns, 31 the
object's), adaptive_linetype, lineweight (an index of custom_lineweight_table, millimetres), fill_style (64–72 patterns,
73 the object's), end_style (0 butt, 1 square, 2 round, 3 diamond, 4 the object's), join_style (0 miter, 1 bevel,
2 round, 3 diamond, 5 the object's).

KentOS's mapping (ADR 1130 §2–§3), each rule written here again:

- colour: the object's; RGB/ACI-typed → `#rrggbb`; another type → the object's (`colorKind`, its type as two hex
  digits); a mode_color that differs → `modeColor`; grayscale with the object's colour → `grayscale` (approximate),
  with an RGB → its grey `(2126 R + 7152 G + 722 B + 5000) // 10000` (`grayscaleColor`, approximate); unknown policy
  bits → `policyBits`;
- screen < 100: with dither the screening (`dither`, approximate), without it full intensity (`screenWithoutDither`,
  a note: AutoCAD screens only with dither);
- a pen or virtual pen number ≠ 0 → `physicalPen` / `virtualPen` (lost);
- linetype 0 → continuous (`dash: []`), 1–30 → `linetype` (lost, `n: Name`), 31 → the object's;
- lineweight: the table's millimetres to µm, exact to three decimals, else half away from zero
  (`lineweightRounded`, approximate); 0 µm is the object's lineweight; an index outside the table → the object's
  (`lineweightOutside`, lost);
- ends and joins: butt/square/round, miter/bevel/round; diamond → `endStyle` / `joinStyle` (lost);
- fill 64–72 → `fillStyle` (lost); a style's non-empty description → `description` (lost);
- a STB's Normal is the object's look: not an entry; a Normal that draws otherwise → `normalChanged`;
- unknown fields → `unknownField` (the field's name); `apply_factor=TRUE` → `scaleFactor`; a header Adler-32 or length
  that does not match → `checksum` / `length` (notes);
- a style is exact when no lost or approximate line names it; the report's lines in the kinds' order, their styles and
  values each once in first-seen order.
"""

import argparse
import copy
import hashlib
import json
import math
import re
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILES = OUT / "plotstyles"
COMMAND = "cad.plotstyles.import"
BASE = "$fixture:commands/v1/plotstyles"
HEAD = {"ctb": b"PIAFILEVERSION_2.0,CTBVER1,compress\r\npmzlibcodec",
        "stb": b"PIAFILEVERSION_2.0,STBVER1,compress\r\npmzlibcodec"}
MAX_FILE = 4 * 1024 * 1024
MAX_TEXT = 8 * 1024 * 1024
LINETYPES = ["Solid", "Dashed", "Dotted", "Dash Dot", "Short Dash", "Medium Dash", "Long Dash", "Short Dash x2",
             "Medium Dash x2", "Long Dash x2", "Medium Long Dash", "Medium Dash Short Dash Short Dash",
             "Long Dash Short Dash", "Long Dash Dot Dot", "Long Dash Dot", "Medium Dash Dot Short Dash Dot", "Sparse Dot",
             "ISO Dash", "ISO Dash Space", "ISO Long Dash Dot", "ISO Long Dash Double Dot", "ISO Long Dash Triple Dot",
             "ISO Dot", "ISO Long Dash Short Dash", "ISO Long Dash Double Short Dash", "ISO Dash Dot",
             "ISO Double Dash Dot", "ISO Dash Double Dot", "ISO Double Dash Double Dot", "ISO Dash Triple Dot",
             "ISO Double Dash Triple Dot"]
# The format's default lineweight table, millimetres (ezdxf DEFAULT_LINE_WEIGHTS).
WEIGHTS = ["0.00", "0.05", "0.09", "0.10", "0.13", "0.15", "0.18", "0.20", "0.25", "0.30", "0.35", "0.40", "0.45",
           "0.50", "0.53", "0.60", "0.65", "0.70", "0.80", "0.90", "1.00", "1.06", "1.20", "1.40", "1.58", "2.00",
           "2.11"]
KINDS = ["colorKind", "modeColor", "grayscale", "grayscaleColor", "dither", "screenWithoutDither", "policyBits",
         "physicalPen", "virtualPen", "linetype", "lineweightOutside", "lineweightRounded", "endStyle", "joinStyle",
         "fillStyle", "description", "normalChanged", "unknownField", "scaleFactor", "checksum", "length"]
SEVERITY = {k: "lost" for k in KINDS}
SEVERITY.update({k: "approximate" for k in ["grayscale", "grayscaleColor", "dither", "lineweightRounded"]})
SEVERITY.update({k: "note" for k in ["screenWithoutDither", "checksum", "length"]})
FIELDS = ["name", "localized_name", "description", "color", "mode_color", "color_policy", "physical_pen_number",
          "virtual_pen_number", "screen", "linepattern_size", "linetype", "adaptive_linetype", "lineweight",
          "fill_style", "end_style", "join_style"]
TOP = ["description", "aci_table_available", "scale_factor", "apply_factor", "custom_lineweight_display_units",
       "aci_table", "plot_style", "custom_lineweight_table"]
INT = re.compile(r"[+-]?[0-9]+\Z")


class Refused(Exception):
    def __init__(self, code):
        super().__init__(code)
        self.code = code


def bad():
    return Refused("bad_plot_style_file")


# ---- writing test tables from the documented format ---------------------------------------------------------------

def rgb(r, g, b, kind=0xC2):
    """A colour as the file writes it: the signed 32-bit value of 0xTTRRGGBB."""
    u = (kind << 24) | (r << 16) | (g << 8) | b
    return u - (1 << 32) if u >= 1 << 31 else u


def pen(name, **f):
    """A style's lines in the writer's order; `f` overrides a field (None leaves it out), `extra` adds lines."""
    fields = {"name": f'"{name}', "localized_name": f'"{name}', "description": '"', "color": -1, "color_policy": 1,
              "physical_pen_number": 0, "virtual_pen_number": 0, "screen": 100, "linepattern_size": 0.5,
              "linetype": 31, "adaptive_linetype": "TRUE", "lineweight": 0, "fill_style": 73, "end_style": 4,
              "join_style": 5}
    extra = f.pop("extra", [])
    for k, v in f.items():
        fields[k] = v
    if "mode_color" not in f and fields["color"] != -1:
        fields = dict(list(fields.items())[:4] + [("mode_color", fields["color"])] + list(fields.items())[4:])
    return [f"{k}={v}" for k, v in fields.items() if v is not None] + extra


def text(kind, styles, weights=WEIGHTS, description="", apply_factor="FALSE", extra_top=()):
    out = [f'description="{description}', f"aci_table_available={'TRUE' if kind == 'ctb' else 'FALSE'}",
           "scale_factor=1.0", f"apply_factor={apply_factor}", "custom_lineweight_display_units=0"]
    out += list(extra_top)
    if kind == "ctb":
        out.append("aci_table{")
        out += [f' {i}="Color_{i + 1}' for i in range(len(styles))]
        out.append("}")
    out.append("plot_style{")
    for i, lines in enumerate(styles):
        out.append(f" {i}{{")
        out += [f"  {x}" for x in lines]
        out.append(" }")
    out.append("}")
    if weights is not None:
        out.append("custom_lineweight_table{")
        out += [f" {i}={w}" for i, w in enumerate(weights)]
        out.append("}")
    return "\n".join(out) + "\n"


def table(kind, body_text, encoding="utf-8", sum_fix=None, length_fix=None, packed_fix=None, body_bytes=None):
    """The file: header, the three numbers, the zlib stream of the text and its closing NUL."""
    body = (body_bytes if body_bytes is not None else body_text.encode(encoding)) + b"\0"
    packed = zlib.compress(body, 9)
    s = zlib.adler32(packed) if sum_fix is None else sum_fix
    n = len(body) if length_fix is None else length_fix
    k = len(packed) if packed_fix is None else packed_fix
    return HEAD[kind] + struct.pack("<III", s, n, k) + packed


def ctb_pens(over=None):
    """255 pens, `over` replacing some by their ACI (1–255)."""
    over = over or {}
    return [over.get(i + 1, pen(f"Color_{i + 1}")) for i in range(255)]


def files():
    """Every test table, by name."""
    black = ctb_pens({i + 1: pen(f"Color_{i + 1}", color=rgb(0, 0, 0)) for i in range(255)})
    black[0] = pen("Color_1", color=rgb(0, 0, 0), lineweight=13)
    black[1] = pen("Color_2", color=rgb(0, 0, 0), lineweight=8)
    black[6] = pen("Color_7")
    w = WEIGHTS + ["0.0076200000000 (+7.Z+\"8V?S_LC )"]
    many = ctb_pens({
        1: pen("Color_1", color=rgb(255, 0, 0), color_policy=3),
        2: pen("Color_2", color_policy=3),
        3: pen("Color_3", color=rgb(0, 0, 255), screen=50),
        4: pen("Color_4", color=rgb(0, 128, 0), color_policy=0, screen=30),
        5: pen("Color_5", linetype=2),
        6: pen("Color_6", linetype=0),
        7: pen("Color_7", lineweight=27),
        8: pen("Color_8", lineweight=255),
        9: pen("Color_9", end_style=3),
        10: pen("Color_10", join_style=3),
        11: pen("Color_11", fill_style=66),
        12: pen("Color_12", physical_pen_number=5),
        13: pen("Color_13", virtual_pen_number=9),
        14: pen("Color_14", color=rgb(10, 20, 30, 0xC4)),
        15: pen("Color_15", color=rgb(1, 2, 3), mode_color=rgb(9, 9, 9)),
        16: pen("Color_16", extra=["foo=1"]),
        17: pen("Color_17", description='"Aks'),
        18: pen("Color_18", color_policy=9),
        19: pen("Color_19", end_style=2, join_style=2, lineweight=10),
        20: pen("Color_20", color=rgb(255, 255, 0, 0xC3)),
    })
    named = [pen("Normal"), pen("Kalın", color=rgb(0, 0, 0), lineweight=17, end_style=0, join_style=1),
             pen("İnce", color=rgb(128, 128, 128), lineweight=4, screen=40),
             pen("Kesik", linetype=1, adaptive_linetype="FALSE")]
    out = {
        "siyah.ctb": table("ctb", text("ctb", black, description="Siyah kalem")),
        "kalemler.ctb": table("ctb", text("ctb", many, weights=w, apply_factor="TRUE",
                                          extra_top=["yeni_alan=3"])),
        "stiller.stb": table("stb", text("stb", named, description="Ofis stilleri")),
        "normal-degisik.stb": table("stb", text("stb", [pen("Normal", lineweight=13), pen("A")])),
        "saglama.ctb": table("ctb", text("ctb", ctb_pens()), sum_fix=7, length_fix=12),
        "metin.ctb": b"description=\"bu bir tablo degil\n",
        "kesik.ctb": table("ctb", text("ctb", ctb_pens()))[:-40],
        "zlib-bozuk.ctb": HEAD["ctb"] + struct.pack("<III", 0, 10, 12) + b"not zlib!!!!",
        "latin.stb": table("stb", text("stb", [pen("Normal"), pen("Kalın")]), encoding="cp1254"),
        "tur-celiski.ctb": table("ctb", text("stb", [pen("Normal")])),
        "cift.stb": table("stb", text("stb", [pen("Normal"), pen("A"), pen("A")])),
        "eksik.ctb": table("ctb", text("ctb", ctb_pens()[:254])),
        "tarama-150.ctb": table("ctb", text("ctb", ctb_pens({3: pen("Color_3", screen=150)}))),
        "satir.stb": table("stb", text("stb", [pen("Normal")]).replace("scale_factor=1.0", "olcek 1.0")),
        "kapanmayan.stb": table("stb", text("stb", [pen("Normal")]).rstrip("}\n")),
        "cok.stb": table("stb", text("stb", [pen("Normal")] + [pen(f"S{i}") for i in range(4096)])),
        "sira.ctb": table("ctb", text("ctb", ctb_pens()).replace(" 7{", " 8{", 1)),
    }
    return out


# ---- the independent reader ----------------------------------------------------------------------------------------

def value(raw):
    v = raw.strip()
    if v.startswith('"'):
        return v[1:]
    if v.endswith(")"):
        return v.split(" ")[0]
    return v


def parse(t):
    stack = [("", [])]
    for line in t.split("\n"):
        s = line.strip()
        if not s:
            continue
        if s == "}":
            if len(stack) < 2:
                raise bad()
            name, items = stack.pop()
            stack[-1][1].append((name, items))
        elif s.endswith("{"):
            if len(stack) > 4:
                raise bad()
            stack.append((s[:-1].strip(), []))
        elif "=" in s:
            k, v = s.split("=", 1)
            stack[-1][1].append((k.strip(), value(v)))
        else:
            raise bad()
    if len(stack) != 1:
        raise bad()
    return stack[0][1]


def integer(v):
    if not INT.match(v):
        raise bad()
    return int(v)


def boolean(v):
    u = v.upper()
    if u not in ("TRUE", "FALSE"):
        raise bad()
    return u == "TRUE"


def finite(v):
    try:
        x = float(v)
    except ValueError:
        raise bad()
    if not math.isfinite(x):
        raise bad()


def micrometres(v):
    whole, _, frac = v.partition(".")
    if not whole or not whole.isdigit() or (frac and not frac.isdigit()) or not v.isascii():
        raise bad()
    d = [int(c) for c in frac] + [0, 0, 0, 0]
    um = int(whole) * 1000 + d[0] * 100 + d[1] * 10 + d[2]
    if d[3] >= 5:
        um += 1
    if um > 100000:
        raise bad()
    return um, any(x != 0 for x in d[3:])


class Report:
    def __init__(self):
        self.lines = {}

    def add(self, kind, style=None, value=None):
        styles, values = self.lines.setdefault(kind, ([], []))
        if style is not None and style not in styles:
            styles.append(style)
        if value is not None and value not in values:
            values.append(value)

    def names(self, key):
        return sum(1 for k, (st, _) in self.lines.items() if SEVERITY[k] != "note" and key in st)

    def done(self):
        return [{"kind": k, "severity": SEVERITY[k], "words": "$any", "styles": self.lines[k][0],
                 "values": self.lines[k][1]} for k in KINDS if k in self.lines]


def first(fields, name):
    for k, v in fields:
        if k == name and isinstance(v, str):
            return v
    return None


def style(fields, key, weights, report):
    before = report.names(key)
    for k, v in fields:
        if k not in FIELDS or not isinstance(v, str):
            report.add("unknownField", key, k)
    out = {"color": {"type": "object"}, "screening": 100}
    get = lambda n: first(fields, n)
    color = integer(get("color")) if get("color") is not None else -1
    if not (-(1 << 31) <= color < (1 << 32)):
        raise bad()
    rgb_ = None
    if color not in (-1, -1006632961):
        u = color & 0xFFFFFFFF
        kind = u >> 24
        if kind in (0xC2, 0xC3):
            rgb_ = ((u >> 16) & 255, (u >> 8) & 255, u & 255)
        else:
            report.add("colorKind", key, f"{kind:02X}")
        if get("mode_color") is not None and integer(get("mode_color")) != color:
            report.add("modeColor", key)
    policy = integer(get("color_policy")) if get("color_policy") is not None else 1
    if not 0 <= policy <= 255:
        raise bad()
    if policy & ~7:
        report.add("policyBits", key, str(policy))
    dither, gray = bool(policy & 1), bool(policy & 2)
    if rgb_ is None and gray:
        report.add("grayscale", key)
        out["color"] = {"type": "grayscale"}
    elif rgb_ is not None and not gray:
        out["color"] = {"type": "color", "value": "#%02x%02x%02x" % rgb_}
    elif rgb_ is not None:
        report.add("grayscaleColor", key)
        y = (2126 * rgb_[0] + 7152 * rgb_[1] + 722 * rgb_[2] + 5000) // 10000
        out["color"] = {"type": "color", "value": "#%02x%02x%02x" % (y, y, y)}
    screen = integer(get("screen")) if get("screen") is not None else 100
    if not 0 <= screen <= 100:
        raise bad()
    if screen < 100:
        if dither:
            report.add("dither", key)
            out["screening"] = screen
        else:
            report.add("screenWithoutDither", key, str(screen))
    for field, kind in (("physical_pen_number", "physicalPen"), ("virtual_pen_number", "virtualPen")):
        n = integer(get(field)) if get(field) is not None else 0
        if not 0 <= n <= 255:
            raise bad()
        if n:
            report.add(kind, key, str(n))
    lt = integer(get("linetype")) if get("linetype") is not None else 31
    if lt == 0:
        out["dash"] = []
    elif 1 <= lt <= 30:
        report.add("linetype", key, f"{lt}: {LINETYPES[lt]}")
    elif lt != 31:
        raise bad()
    if get("adaptive_linetype") is not None:
        boolean(get("adaptive_linetype"))
    if get("linepattern_size") is not None:
        finite(get("linepattern_size"))
    if get("lineweight") is not None:
        i = integer(get("lineweight"))
        if i < 0:
            raise bad()
        if i < len(weights):
            um, rounded = weights[i]
            if um != 0:
                out["lineWeight"] = um
                if rounded:
                    report.add("lineweightRounded", key, str(i))
        else:
            report.add("lineweightOutside", key, str(i))
    end = integer(get("end_style")) if get("end_style") is not None else 4
    if end in (0, 1, 2):
        out["cap"] = ["butt", "square", "round"][end]
    elif end == 3:
        report.add("endStyle", key)
    elif end != 4:
        raise bad()
    join = integer(get("join_style")) if get("join_style") is not None else 5
    if join in (0, 1, 2):
        out["join"] = ["miter", "bevel", "round"][join]
    elif join == 3:
        report.add("joinStyle", key)
    elif join != 5:
        raise bad()
    fill = integer(get("fill_style")) if get("fill_style") is not None else 73
    if 64 <= fill <= 72:
        report.add("fillStyle", key, str(fill))
    elif fill != 73:
        raise bad()
    if get("description"):
        report.add("description", key)
    return out, report.names(key) > before


def read(data, pid, name, file):
    """A table's bytes read: (profile, description, styles, exact, losses), or Refused."""
    if len(data) > MAX_FILE:
        raise Refused("plot_style_file_too_large")
    if data.startswith(HEAD["ctb"]):
        fmt = "ctb"
    elif data.startswith(HEAD["stb"]):
        fmt = "stb"
    else:
        raise Refused("unsupported_plot_style_file")
    if len(data) < 60:
        raise bad()
    s, n, k = struct.unpack("<III", data[48:60])
    body = data[60:]
    if k != len(body):
        raise bad()
    try:
        d = zlib.decompressobj()
        t = d.decompress(body, MAX_TEXT + 1)
    except zlib.error:
        raise bad()
    if len(t) > MAX_TEXT or not d.eof:
        raise bad()
    report = Report()
    if zlib.adler32(body) != s:
        report.add("checksum")
    if n != len(t):
        report.add("length", None, f"{n} ≠ {len(t)}")
    t = t.rstrip(b"\0")
    try:
        t = t.decode("utf-8")
    except UnicodeDecodeError:
        raise Refused("plot_style_encoding")
    top = parse(t)
    for key, _ in top:
        if key not in TOP:
            report.add("unknownField", None, key)
    get = lambda n: first(top, n)
    block = lambda n: next((v for k, v in top if k == n and isinstance(v, list)), None)
    if get("aci_table_available") is None:
        raise bad()
    if boolean(get("aci_table_available")) != (fmt == "ctb"):
        raise Refused("plot_style_kind_mismatch")
    if get("scale_factor") is not None:
        finite(get("scale_factor"))
    if get("apply_factor") is not None and boolean(get("apply_factor")):
        report.add("scaleFactor", None, get("scale_factor"))
    weights = []
    for k_, v in block("custom_lineweight_table") or []:
        if not isinstance(v, str) or integer(k_) != len(weights):
            raise bad()
        weights.append(micrometres(v))
    styles = block("plot_style")
    if styles is None:
        raise bad()
    entries, exact, names = [], 0, set()
    for i, (k_, fields) in enumerate(styles):
        if not isinstance(fields, list) or integer(k_) != i:
            raise bad()
        if fmt == "ctb":
            key = str(i + 1)
        else:
            key = first(fields, "name")
            if key is None:
                raise bad()
        if fmt == "stb" and key.lower() == "normal":
            scratch = Report()
            st, _ = style(fields, key, weights, scratch)
            if st != {"color": {"type": "object"}, "screening": 100} or any(
                    SEVERITY[x] != "note" for x in scratch.lines):
                report.add("normalChanged", key)
            else:
                exact += 1
            continue
        st, lost = style(fields, key, weights, report)
        if not lost:
            exact += 1
        if fmt == "ctb":
            entries.append({"key": {"type": "aci", "index": i + 1}, "style": st})
        else:
            if not key.strip() or len(key.encode()) > 256 or any(ord(c) < 32 or 127 <= ord(c) < 160 for c in key):
                raise bad()
            if key in names:
                raise Refused("duplicate_plot_style")
            names.add(key)
            entries.append({"key": {"type": "named", "name": key}, "style": st})
    if fmt == "ctb" and len(styles) != 255:
        raise bad()
    if len(styles) > 4096:
        raise Refused("plot_profile_too_large")
    profile = {"id": pid, "name": name, "kind": "colorDependent" if fmt == "ctb" else "named", "entries": entries,
               "assignments": [], "source": {"format": fmt, "sha256": hashlib.sha256(data).hexdigest(), "file": file}}
    return profile, get("description") or "", len(styles), exact, report.done()


# ---- the command's cases -------------------------------------------------------------------------------------------

def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def sheet(i):
    return {"export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "svg"}, "guides": [], "id": i,
            "items": [{"bindings": [], "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
                       "frame": {"height": 230000, "left": 20000, "top": 10000, "width": 390000}, "hidden": False,
                       "id": f"{i}-harita", "kind": {"clipToAtlasFeature": False, "grids": [], "labelBand": 0,
                                                     "layers": {"type": "all"}, "overviewFrame": {
                               "cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
                                                     "type": "map", "view": {"center": {"x": 0.0, "y": 0.0},
                                                                             "rotation": 0, "scale": 100,
                                                                             "type": "fixed"}},
                       "locked": False, "name": "Harita", "opacity": 100, "padding": 0, "printable": True,
                       "rotation": 0}],
            "name": i.upper(), "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000},
                                        "orientation": "landscape", "paper": "a3",
                                        "size": {"height": 297000, "width": 420000}},
            "snapGrid": {"enabled": False, "spacing": 5000, "visible": False}, "variables": [], "variants": []}


def setup(book=True, profiles=()):
    d = {"format": "kentos.document", "version": 2, "name": "Baskı",
         "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 100.0, "workspace": "cad", "drawingUnit": "m"},
         "origin": {"x": 0.0, "y": 0.0}, "layers": [layer("0", "0"), layer("duvar", "Duvar"), layer("aks", "Aks")],
         "activeLayer": "0", "entities": [], "uids": [], "styles": {"items": [], "categories": []}}
    if book:
        b = {"schema": "kentos.sheet/1", "sheets": [sheet("a-101"), sheet("a-102")], "masters": [], "assets": []}
        if profiles:
            b["plotProfiles"] = list(profiles)
        d["sheets"] = {"book": b, "pictures": []}
    return d


SETUP = setup()


def pid(data):
    return ("stb" if data.startswith(HEAD["stb"]) else "ctb") + "-" + hashlib.sha256(data).hexdigest()[:12]


class Cases:
    def __init__(self, tables):
        self.cases, self.tables = [], tables

    def add(self, name, steps, case_setup=None):
        c = {"name": name, "steps": steps}
        if case_setup is not None:
            c["setup"] = case_setup
        self.cases.append(c)

    def done(self, file, name=None, assignments=(), sheets=(), op="execute", before=(), extra=None):
        """A completed import: its output from this reader, the book's profiles after it."""
        data = self.tables[file]
        stem = name if name is not None else file.rsplit(".", 1)[0]
        profile, description, styles, exact, losses = read(data, pid(data), stem, file)
        profile["assignments"] = list(assignments)
        applied = []
        for s in sheets:
            if s not in applied:
                applied.append(s)
        inp = {"path": file, "base": BASE}
        if name is not None:
            inp["name"] = name
        if assignments:
            inp["assignments"] = list(assignments)
        if sheets:
            inp["sheets"] = list(sheets)
        warn = [{"code": "plot_style_losses", "message": "$any"}] if any(
            l["severity"] != "note" for l in losses) else []
        out = {"inverse": "$any", "label": "$any", "revision": "$current", "sheets": "$any", "profile": profile,
               "format": profile["source"]["format"], "description": description, "styles": styles,
               "exact": exact, "losses": losses, "applied": applied}
        written = op == "execute"
        after = list(before) + ([profile] if written else [])
        expect = {"plotProfiles": after, "revision": "changed" if written else "same",
                  "sheetProfiles": {"a-101": profile["id"] if written and "a-101" in applied else None,
                                    "a-102": profile["id"] if written and "a-102" in applied else None}}
        if extra:
            expect.update(extra)
        step = {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warn},
                "expect": expect}
        if op == "validate":
            step["result"] = {"status": "completed", "output": None, "warnings": "$any"}
        return step

    def refused(self, inp, code, path, before=(), status="failed"):
        i = {"base": BASE}
        i.update(inp)
        error = {"code": code, "message": "$any", "path": path}
        if status == "conflict":
            error["revision"] = "0"
        return {"op": "execute", "input": i,
                "result": {"status": status, "error": error},
                "expect": {"plotProfiles": list(before), "revision": "same",
                           "sheetProfiles": {"a-101": None, "a-102": None}}}


def cases(tables):
    c = Cases(tables)
    duvar = {"target": {"type": "modelLayer", "id": "duvar"}, "key": {"type": "aci", "index": 1}}
    aks = {"target": {"type": "modelLayer", "id": "aks"}, "key": {"type": "aci", "index": 2}}
    harita = {"target": {"type": "paperItem", "id": "a-101-harita"}, "key": {"type": "aci", "index": 7}}
    c.add("Siyah CTB: 255 kalem aynen alınır, kitapta profil olur", [c.done("siyah.ctb")])
    c.add("Kalemler katmanlara açıkça verilir ve paftaya uygulanır",
          [c.done("siyah.ctb", assignments=[duvar, aks], sheets=["a-101"])])
    c.add("Kâğıt öğesine kalem ve iki paftaya uygulama; tekrarlanan pafta bir kez",
          [c.done("siyah.ctb", name="Siyah", assignments=[harita], sheets=["a-101", "a-102", "a-101"])])
    c.add("Plan kitabı yazmaz", [c.done("siyah.ctb", op="plan", sheets=["a-101"])])
    c.add("Doğrula kitabı yazmaz", [c.done("siyah.ctb", op="validate")])
    c.add("Kitapsız projede kitap profille kurulur", [c.done("siyah.ctb", extra={"sheetProfiles": {}})],
          case_setup=setup(book=False))
    c.add("Her eşlenmeyen alan adıyla raporlanır", [c.done("kalemler.ctb")])
    c.add("Adlı STB: Normal nesnenin görünüşüdür, öbür stiller adlarıyla", [c.done("stiller.stb")])
    c.add("STB stili katmana verilir",
          [c.done("stiller.stb", assignments=[{"target": {"type": "modelLayer", "id": "duvar"},
                                               "key": {"type": "named", "name": "Kalın"}}], sheets=["a-102"])])
    c.add("Değişik Normal stili raporlanır", [c.done("normal-degisik.stb")])
    c.add("Üst bilginin sağlaması ve uzunluğu tutmasa da zlib denetimi geçen tablo okunur (not)",
          [c.done("saglama.ctb")])
    existing = read(tables["siyah.ctb"], pid(tables["siyah.ctb"]), "siyah", "siyah.ctb")[0]
    other = dict(existing, id="baska", source=None)
    other.pop("source")
    c.add("Aynı adlı profil varsa reddedilir",
          [c.refused({"path": "stiller.stb", "name": "siyah"}, "duplicate_plot_profile", "name", before=[other])],
          case_setup=setup(profiles=[other]))
    c.add("Aynı dosya başka adla ikinci kez alınmaz",
          [c.refused({"path": "siyah.ctb", "name": "Yine"}, "duplicate_plot_profile", "path", before=[existing])],
          case_setup=setup(profiles=[existing]))
    c.add("Olmayan katmana atama", [c.refused({"path": "siyah.ctb", "assignments": [
        {"target": {"type": "modelLayer", "id": "yok"}, "key": {"type": "aci", "index": 1}}]},
        "layer_not_found", "assignments[0].target.id")])
    c.add("Olmayan kâğıt öğesine atama", [c.refused({"path": "siyah.ctb", "assignments": [
        {"target": {"type": "paperItem", "id": "yok"}, "key": {"type": "aci", "index": 1}}]},
        "unknown_item", "assignments[0].target.id")])
    c.add("Aynı katmana iki atama", [c.refused({"path": "siyah.ctb", "assignments": [duvar, dict(duvar, key={
        "type": "aci", "index": 3})]}, "bad_plot_assignment", "assignments")])
    c.add("Tabloda olmayan STB stili atanamaz", [c.refused({"path": "stiller.stb", "assignments": [
        {"target": {"type": "modelLayer", "id": "duvar"}, "key": {"type": "named", "name": "Yok"}}]},
        "bad_plot_assignment", "assignments")])
    c.add("Okunamayan atama", [c.refused({"path": "siyah.ctb", "assignments": [{"target": "duvar"}]},
                                         "invalid_input", "assignments[0]")])
    c.add("Olmayan pafta", [c.refused({"path": "siyah.ctb", "sheets": ["a-999"]}, "unknown_sheet", "sheets[0]")])
    c.add("Boş yol", [c.refused({"path": " "}, "invalid_input", "path")])
    c.add("Boş ad", [c.refused({"path": "siyah.ctb", "name": "  "}, "invalid_input", "name")])
    c.add("Olmayan dosya", [c.refused({"path": "yok.ctb"}, "file_not_found", "path")])
    c.add("Klasör dosya değildir", [c.refused({"path": "."}, "file_not_found", "path")])
    for file, code, what in [
        ("metin.ctb", "unsupported_plot_style_file", "üst bilgisi olmayan dosya"),
        ("kesik.ctb", "bad_plot_style_file", "kesilmiş dosya: sıkıştırılmış uzunluk tutmaz"),
        ("zlib-bozuk.ctb", "bad_plot_style_file", "zlib akışı bozuk"),
        ("latin.stb", "plot_style_encoding", "UTF-8 olmayan metin"),
        ("tur-celiski.ctb", "plot_style_kind_mismatch", "CTB üst bilgisi, adlı tablo metni"),
        ("cift.stb", "duplicate_plot_style", "iki kez adlanan stil"),
        ("eksik.ctb", "bad_plot_style_file", "254 kalemli CTB"),
        ("tarama-150.ctb", "bad_plot_style_file", "tarama 0–100 dışında"),
        ("satir.stb", "bad_plot_style_file", "ad=değer olmayan satır"),
        ("kapanmayan.stb", "bad_plot_style_file", "kapanmayan blok"),
        ("cok.stb", "plot_profile_too_large", "4096'dan çok stil"),
        ("sira.ctb", "bad_plot_style_file", "stil sırası atlıyor"),
    ]:
        try:
            read(tables[file], pid(tables[file]), "x", file)
            raise SystemExit(f"{file} reddedilmeliydi")
        except Refused as e:
            assert e.code == code, (file, e.code, code)
        c.add(f"Ret: {what}", [c.refused({"path": file}, code, "path")])
    c.add("Geçersiz beklenen sürüm",
          [c.refused({"path": "siyah.ctb", "expectedRevision": "on"}, "invalid_revision", "expectedRevision")])
    c.add("Sürüm çakışması: hiçbir şey yazılmaz",
          [c.refused({"path": "siyah.ctb", "expectedRevision": "999"}, "revision_conflict", "expectedRevision",
                     status="conflict")])
    return c


def doc(cs):
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "AutoCAD baskı stili tablosunu (CTB/STB) baskı profili olarak içe aktar",
            "note": "scripts/fixtures/plot_style_cases.py yazar: tabloları belgelenen biçimden kurar, beklentileri "
                    "ADR 1130'un kurallarıyla kendi okuyucusundan çıkarır; KentOS kodu yok.",
            "setup": SETUP, "cases": cs.cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--read")
    args = ap.parse_args()
    if args.read:
        data = Path(args.read).read_bytes()
        name = Path(args.read).stem
        try:
            p, description, styles, exact, losses = read(data, pid(data), name, Path(args.read).name)
        except Refused as e:
            print(json.dumps({"refused": e.code}))
            return 1
        print(json.dumps({"profile": p, "description": description, "styles": styles, "exact": exact,
                          "losses": losses}, ensure_ascii=False, indent=2))
        return 0
    tables = files()
    d = doc(cases(tables))
    assert len(d["cases"]) >= 20, len(d["cases"])
    path = OUT / f"{COMMAND}.json"
    text_ = json.dumps(d, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        wrong = [n for n, b in tables.items() if not (FILES / n).exists() or (FILES / n).read_bytes() != b]
        if wrong or not path.exists() or path.read_text() != text_:
            print(f"{path.relative_to(ROOT)} ya da tablolar ({', '.join(wrong)}) başvurudan farklı; --check'siz "
                  "yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND}: {len(d['cases'])} durum ve {len(tables)} tablo başvuruyla aynı.")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    for n, b in tables.items():
        (FILES / n).write_bytes(b)
    path.write_text(text_)
    print(f"{path.relative_to(ROOT)}: {len(d['cases'])} durum, {len(tables)} tablo yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
