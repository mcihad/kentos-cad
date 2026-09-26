"""Independent reference reader of the GeoJSON and Shapefile imports (fixtures/formats/v1/gis).

Reads a GeoJSON file (RFC 7946) or a Shapefile set (.shp, .dbf and the optional
.prj and .cpg; ESRI Shapefile Technical Description, July 1998; dBASE III+) by
the reading rules the Rust reader in crates/shared/formats follows too, and
gives their canonical JSON: the declared SRID, the encoding (Shapefile only)
and the objects in reading order. Python's standard library only, never KentOS
code: the rules and the public specifications are its only sources, so the
Rust reader can be compared with it float for float.

    python3 tools/formats/gis.py read PATH [--layer NAME]

PATH is a GeoJSON file or a `.shp`; the .dbf, .prj and .cpg beside a .shp are
read when present (the .shx is not needed: records are read from the .shp in
order). The layer name the caller gives (`--layer`) defaults to the file name
without its extension. The output is the text of the fixtures'
`<name>.expected.json`: indented by one space, UTF-8, keys in the rules' order.
A file the rules refuse gives an error and no output.

As a module: read_geojson(data, layer), read_shapefile(shp, shx, dbf, prj_text,
cpg_text, layer), read_shapefile_files(files, layer) and read_path(path, layer)
give the canonical dict, dumps(result) its text.
"""
import argparse
import json
import math
import re
import struct
import sys
from pathlib import Path


class GisError(Exception):
    """A file the rules refuse: there is no canonical output for it."""


# ── Canonical output ────────────────────────────────────────────────────


def make(kind, layer, geometry, label, attrs):
    """One object, keys in the rules' order; `label` only when the source gave one."""
    obj = {"kind": kind, "layer": layer, **geometry}
    if label is not None:
        obj["label"] = label
    obj["attrs"] = dict(attrs)
    return obj


def point_geometry(x, y, z):
    return {"p": [x, y]} if z is None else {"p": [x, y], "z": z}


def polygon_geometry(outline, holes):
    return {"pts": outline, "holes": holes} if holes else {"pts": outline}


def open_ring(ring):
    """A ring without its closing position: the last one goes when it equals the first (x and y, exactly)."""
    if ring and ring[-1][0] == ring[0][0] and ring[-1][1] == ring[0][1]:
        return ring[:-1]
    return ring


def finite(*values):
    return all(v is None or math.isfinite(v) for v in values)


def dumps(result):
    """The canonical text: indent 1, characters as themselves (UTF-8), floats by repr (shortest round trip)."""
    return json.dumps(result, indent=1, ensure_ascii=False, allow_nan=False) + "\n"


class Target:
    """Where a feature's (or a record's) objects go, with the layer, label and attributes each carries."""

    __slots__ = ("out", "layer", "label", "attrs")

    def __init__(self, out, layer, label, attrs):
        self.out, self.layer, self.label, self.attrs = out, layer, label, attrs

    def add(self, kind, geometry):
        self.out.append(make(kind, self.layer, geometry, self.label, self.attrs))


# ── GeoJSON (RFC 7946) ──────────────────────────────────────────────────


class Num:
    """A JSON number kept as the text the file wrote it with (`1.50`, `1e3`, `-0.0`)."""

    __slots__ = ("text",)

    def __init__(self, text):
        self.text = text

    def __repr__(self):
        return f"Num({self.text!r})"


class Obj(dict):
    """A JSON object: a dict in which a repeated key keeps its last value, remembering every member in file order, repeats too."""

    def __init__(self, pairs):
        super().__init__(pairs)
        self.pairs = pairs


def _not_json(name):
    raise ValueError(f"{name} JSON'da bir değer değil")


_ESCAPED_SURROGATE = re.compile(r"\\u[dD][89a-fA-F]")
_REPLACEMENT = chr(0xFFFD)


def lone_surrogates_replaced(value):
    """The value with each escaped lone surrogate made U+FFFD (json joins the escaped pairs itself)."""
    if isinstance(value, str):
        return "".join(_REPLACEMENT if 0xD800 <= ord(c) <= 0xDFFF else c for c in value)
    if isinstance(value, list):
        return [lone_surrogates_replaced(v) for v in value]
    if isinstance(value, Obj):
        return Obj([(lone_surrogates_replaced(k), lone_surrogates_replaced(v)) for k, v in value.pairs])
    return value


def load_json(data):
    """The JSON value: numbers as `Num`, objects as `Obj`."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as e:
        raise GisError(f"GeoJSON UTF-8 değil (bayt {e.start}); dosyayı UTF-8 olarak kaydedin") from None
    try:
        value = json.loads(text, parse_float=Num, parse_int=Num, parse_constant=_not_json, object_pairs_hook=Obj)
    except (ValueError, RecursionError) as e:
        raise GisError(f"GeoJSON, JSON olarak okunamadı: {e}") from None
    return lone_surrogates_replaced(value) if _ESCAPED_SURROGATE.search(text) else value


_ESCAPES = {'"': '\\"', "\\": "\\\\", "\b": "\\b", "\f": "\\f", "\n": "\\n", "\r": "\\r", "\t": "\\t"}


def quote(text):
    """A JSON string the rules' way: the seven short escapes, \\u00xx (lowercase) for the rest below U+0020, every other character as itself."""
    parts = ['"']
    for ch in text:
        escaped = _ESCAPES.get(ch)
        if escaped is not None:
            parts.append(escaped)
        elif ch < "\x20":
            parts.append(f"\\u{ord(ch):04x}")
        else:
            parts.append(ch)
    parts.append('"')
    return "".join(parts)


def compact(value):
    """Compact JSON text of an attribute's object or array: no whitespace, members in file order (repeats kept), numbers as written."""
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, Num):
        return value.text
    if isinstance(value, str):
        return quote(value)
    if isinstance(value, list):
        return "[" + ",".join(compact(v) for v in value) + "]"
    if isinstance(value, dict):
        members = value.pairs if isinstance(value, Obj) else value.items()
        return "{" + ",".join(quote(k) + ":" + compact(v) for k, v in members) + "}"
    raise TypeError(type(value))


def geojson_attrs(properties):
    """A feature's attributes: each member of an object `properties` (a repeated key: the last), null ones left out, as text."""
    if not isinstance(properties, dict):
        return {}
    attrs = {}
    for key, value in properties.items():
        if value is None:
            continue
        if isinstance(value, str):
            attrs[key] = value
        elif isinstance(value, Num):
            attrs[key] = value.text
        elif value is True or value is False:
            attrs[key] = "true" if value else "false"
        else:
            attrs[key] = compact(value)
    return attrs


def position(value):
    """[x, y] or [x, y, z] as floats; None when the value is not an array of at least two numbers or x, y or z is not finite."""
    if not isinstance(value, list) or len(value) < 2 or not all(isinstance(n, Num) for n in value):
        return None
    numbers = [float(n.text) for n in value[:3]]  # numbers after the third are ignored
    return numbers if finite(*numbers) else None


def gj_point(coords, target):
    p = position(coords)
    if p is not None:
        target.add("point", point_geometry(p[0], p[1], p[2] if len(p) > 2 else None))


def gj_line_string(coords, target):
    if not isinstance(coords, list):
        return
    positions = [position(p) for p in coords]
    if any(p is None for p in positions):
        return  # a bad position: the whole LineString gives nothing
    if len(positions) == 2:
        target.add("line", {"a": positions[0][:2], "b": positions[1][:2]})
    elif len(positions) > 2:
        target.add("polyline", {"pts": [p[:2] for p in positions]})


def gj_polygon(coords, target):
    if not isinstance(coords, list) or not coords or not all(isinstance(ring, list) for ring in coords):
        return
    rings = [[position(p) for p in ring] for ring in coords]
    if any(p is None for ring in rings for p in ring):
        return  # anything bad anywhere: the whole Polygon gives nothing
    outline = open_ring([p[:2] for p in rings[0]])
    if len(outline) < 3:
        return
    holes = [h for h in (open_ring([p[:2] for p in ring]) for ring in rings[1:]) if len(h) >= 3]
    target.add("polygon", polygon_geometry(outline, holes))


def gj_each(read):
    """A Multi* geometry: an array whose members are read on their own, each as the single geometry."""

    def each(coords, target):
        if isinstance(coords, list):
            for member in coords:
                read(member, target)

    return each


COORDINATE_READERS = {
    "Point": gj_point,
    "MultiPoint": gj_each(gj_point),
    "LineString": gj_line_string,
    "MultiLineString": gj_each(gj_line_string),
    "Polygon": gj_polygon,
    "MultiPolygon": gj_each(gj_polygon),
}
GEOMETRY_TYPES = (*COORDINATE_READERS, "GeometryCollection")


def gj_geometry(geometry, target):
    """A geometry's objects; not an object, an unknown type or a missing (or non-array) member gives nothing."""
    if not isinstance(geometry, dict):
        return
    kind = geometry.get("type")
    if kind == "GeometryCollection":
        members = geometry.get("geometries")
        if isinstance(members, list):
            for member in members:
                gj_geometry(member, target)
        return
    read = COORDINATE_READERS.get(kind) if isinstance(kind, str) else None
    if read is not None:
        read(geometry.get("coordinates"), target)


def gj_feature(feature, default_layer, out):
    if not isinstance(feature, dict) or feature.get("type") != "Feature":
        return  # not a Feature: nothing
    layer, label = default_layer, None
    kentos = feature.get("kentos")
    if isinstance(kentos, dict):
        if isinstance(kentos.get("layer"), str) and kentos["layer"]:
            layer = kentos["layer"]
        if isinstance(kentos.get("label"), str) and kentos["label"]:
            label = kentos["label"]
    gj_geometry(feature.get("geometry"), Target(out, layer, label, geojson_attrs(feature.get("properties"))))


_EPSG_NAME = re.compile(r"EPSG::?([0-9]+)\Z", re.IGNORECASE | re.ASCII)


def geojson_srid(root):
    """The SRID the legacy top-level `crs` member declares; 4326 without one (RFC 7946)."""
    if "crs" not in root:
        return 4326
    crs = root["crs"]
    if not isinstance(crs, dict) or not isinstance(crs.get("properties"), dict):
        return None
    kind, props = crs.get("type"), crs["properties"]
    if kind == "name":
        name = props.get("name")
        if not isinstance(name, str):
            return None
        match = _EPSG_NAME.search(name)
        if match:
            return int(match.group(1))
        return 4326 if "CRS84" in name else None
    if kind == "EPSG":
        code = props.get("code")
        if isinstance(code, Num) and re.fullmatch(r"-?[0-9]+", code.text):
            return int(code.text)
        if isinstance(code, str) and re.fullmatch(r"[0-9]+", code):
            return int(code)
    return None


def read_geojson(data, layer):
    """A GeoJSON file's canonical dict; `layer` is the caller's name for the default layer."""
    root = load_json(data)
    if not isinstance(root, dict):
        raise GisError("GeoJSON'un kökü bir nesne olmalı")
    name = root.get("name")
    default_layer = name if isinstance(name, str) and name else layer
    out = []
    kind = root.get("type")
    if kind == "FeatureCollection":
        features = root.get("features", [])  # without `features`: no objects
        if not isinstance(features, list):
            raise GisError("FeatureCollection'ın features üyesi bir dizi olmalı")
        for feature in features:
            gj_feature(feature, default_layer, out)
    elif kind == "Feature":
        gj_feature(root, default_layer, out)
    elif kind in GEOMETRY_TYPES:
        gj_geometry(root, Target(out, default_layer, None, {}))
    else:
        raise GisError(f"GeoJSON'un kök türü tanınmıyor: {kind!r}")
    return {"declaredSrid": geojson_srid(root), "objects": out}


# ── Shapefile (ESRI Shapefile Technical Description, July 1998) ─────────

POINTS = (1, 11, 21)  # Point, PointZ, PointM
MULTIPOINTS = (8, 18, 28)
POLYLINES = (3, 13, 23)
POLYGONS = (5, 15, 25)


def needed(shape_type, base, n):
    """The bytes a shape of `n` points needs: `base` up to its x, y values, then the z block of a Z type (its m
    block is optional) or the m block of an M type; a block is a range (min, max) and one value per point."""
    return base if shape_type in (8, 3, 5) else base + 16 + 8 * n


def shp_parts(content, shape_type):
    """The parts of a PolyLine or Polygon record, each a list of [x, y]; None when the record gives nothing."""
    if len(content) < 44:
        return None
    nparts, npoints = struct.unpack_from("<2i", content, 36)
    if nparts < 0 or npoints < 0 or len(content) < needed(shape_type, 44 + 4 * nparts + 16 * npoints, npoints):
        return None
    starts = [*struct.unpack_from(f"<{nparts}i", content, 44), npoints]
    if nparts and (starts[0] != 0 or any(starts[i] > starts[i + 1] for i in range(nparts))):
        return None  # parts must run from 0, non-decreasing, up to the point count
    xy = struct.unpack_from(f"<{2 * npoints}d", content, 44 + 4 * nparts)
    points = [[xy[2 * i], xy[2 * i + 1]] for i in range(npoints)]
    return [points[starts[i] : starts[i + 1]] for i in range(nparts)]


def signed_area(ring):
    """Half the shoelace sum over the ring without its closing point: negative is clockwise."""
    total = 0.0
    n = len(ring)
    for i in range(n):
        x0, y0 = ring[i]
        x1, y1 = ring[(i + 1) % n]
        total += x0 * y1 - x1 * y0
    return total / 2


def contains(ring, point):
    """Even-odd ray casting written exactly as the rules give it (edge i runs from vertex i-1, cyclic)."""
    px, py = point
    inside = False
    j = len(ring) - 1
    for i in range(len(ring)):
        xi, yi = ring[i]
        xj, yj = ring[j]
        if (yi > py) != (yj > py) and px < (xj - xi) * (py - yi) / (yj - yi) + xi:
            inside = not inside
        j = i
    return inside


def shp_polygons(parts):
    """A record's rings as (outline, holes): clockwise rings are outlines, counter-clockwise ones go to the
    first clockwise outline holding their first vertex or stand alone; zero-area and short rings give nothing."""
    shells, holes = [], []
    for index, part in enumerate(parts):
        ring = open_ring(part)
        if len(ring) < 3:
            continue
        area = signed_area(ring)
        if area < 0:
            shells.append((index, ring, []))
        elif area > 0:
            holes.append((index, ring))
    outlines = list(shells)  # only clockwise rings take holes
    for index, ring in holes:
        owner = next((s for s in outlines if contains(s[1], ring[0])), None)
        if owner is None:
            shells.append((index, ring, []))
        else:
            owner[2].append(ring)
    shells.sort(key=lambda s: s[0])
    return [(ring, owned) for _, ring, owned in shells]


def shp_objects(content):
    """The (kind, geometry) pairs one record gives, in order; a record shorter than its shape needs gives nothing."""
    if len(content) < 4:
        return []
    shape_type = struct.unpack_from("<i", content, 0)[0]
    if shape_type in POINTS:
        if len(content) < (20 if shape_type == 1 else 28):  # PointM needs its m, PointZ its z (the m after it is optional)
            return []
        x, y = struct.unpack_from("<2d", content, 4)
        z = struct.unpack_from("<d", content, 20)[0] if shape_type == 11 else None
        return [("point", point_geometry(x, y, z))] if finite(x, y, z) else []
    if shape_type in MULTIPOINTS:
        if len(content) < 40:
            return []
        n = struct.unpack_from("<i", content, 36)[0]
        if n < 0 or len(content) < needed(shape_type, 40 + 16 * n, n):
            return []
        xy = struct.unpack_from(f"<{2 * n}d", content, 40)
        zs = struct.unpack_from(f"<{n}d", content, 40 + 16 * n + 16) if shape_type == 18 else (None,) * n
        points = [(xy[2 * i], xy[2 * i + 1], zs[i]) for i in range(n)]
        return [("point", point_geometry(x, y, z)) for x, y, z in points if finite(x, y, z)]  # a bad point: that point nothing
    if shape_type in POLYLINES:
        pairs = []
        for part in shp_parts(content, shape_type) or []:
            if not all(finite(x, y) for x, y in part):
                continue  # a bad point: that part nothing
            if len(part) == 2:
                pairs.append(("line", {"a": part[0], "b": part[1]}))
            elif len(part) > 2:
                pairs.append(("polyline", {"pts": part}))
        return pairs
    if shape_type in POLYGONS:
        parts = shp_parts(content, shape_type)
        if not parts or not all(finite(x, y) for part in parts for x, y in part):
            return []  # a bad point: the whole record nothing
        return [("polygon", polygon_geometry(ring, holes)) for ring, holes in shp_polygons(parts)]
    return []  # Null, MultiPatch and unknown shape types give nothing


def shp_records(shp):
    """The .shp records' contents in file order; reading stops at a record that runs past the end of the file."""
    if len(shp) < 100 or struct.unpack_from(">i", shp, 0)[0] != 9994:
        raise GisError(".shp değil: 100 baytlık başlığı ya da 9994 dosya kodu yok")
    records, at = [], 100
    while at + 8 <= len(shp):
        words = struct.unpack_from(">i", shp, at + 4)[0]
        end = at + 8 + 2 * words
        if words < 0 or end > len(shp):
            break
        records.append(shp[at + 8 : end])
        at = end
    return records


# ── dBASE III table, encoding ───────────────────────────────────────────

CODECS = {
    "UTF-8": "utf-8",
    "Windows-1254": "cp1254",
    "Windows-1252": "cp1252",
    "ISO-8859-9": "iso8859_9",
    "CP857": "cp857",
    "CP850": "cp850",
    "CP437": "cp437",
}
_CPG_LABELS = {
    "UTF-8": ("UTF-8", "UTF8", "65001"),
    "Windows-1254": ("1254", "CP1254", "WINDOWS-1254", "ANSI 1254"),
    "Windows-1252": ("1252", "CP1252", "WINDOWS-1252", "ANSI 1252"),
    "ISO-8859-9": ("ISO-8859-9", "ISO8859-9", "28599", "LATIN5"),
    "CP857": ("857", "CP857", "IBM857", "OEM 857"),
    "CP850": ("850", "CP850", "IBM850"),
    "CP437": ("437", "CP437", "IBM437"),
}


def cpg_key(text):
    """A .cpg text compared the rules' way: whitespace removed, ASCII letters upper-cased."""
    return "".join(c.upper() if c.isascii() else c for c in text if c not in " \t\r\n\v\f")


CPG = {cpg_key(label): name for name, labels in _CPG_LABELS.items() for label in labels}
LANGUAGE_DRIVERS = {0xCA: "Windows-1254", 0x6B: "CP857", 0x88: "CP857", 0x03: "Windows-1252", 0x58: "Windows-1252", 0x59: "Windows-1252"}
LANGUAGE_DRIVERS.update(dict.fromkeys((0x02, 0x0A, 0x0E, 0x10, 0x12, 0x14, 0x16, 0x1A, 0x1D, 0x25, 0x37), "CP850"))
LANGUAGE_DRIVERS.update(dict.fromkeys((0x01, 0x09, 0x0B, 0x0D, 0x0F, 0x11, 0x15, 0x18, 0x19, 0x1B), "CP437"))


def shapefile_encoding(cpg_text, dbf):
    """The .cpg's encoding when there is one (unknown: Windows-1254), else the .dbf's language driver byte."""
    if cpg_text is not None:
        return CPG.get(cpg_key(cpg_text), "Windows-1254")
    if dbf is not None and len(dbf) > 29:
        return LANGUAGE_DRIVERS.get(dbf[29], "Windows-1254")
    return "Windows-1254"


def dbf_value(kind, raw, codec):
    """One field's attribute text, or None when the rules leave it out."""
    # Padding is spaces, or NULs as some writers put.
    if kind == "C":
        return raw.decode(codec, errors="replace").rstrip(" \0") or None
    text = raw.decode("ascii", errors="replace").strip(" \0")
    if kind in ("N", "F"):
        return text or None
    if kind == "L":
        return "true" if text in ("T", "t", "Y", "y") else "false" if text in ("F", "f", "N", "n") else None
    if kind == "D":
        if text in ("", "00000000"):
            return None
        return f"{text[:4]}-{text[4:6]}-{text[6:]}" if re.fullmatch(r"[0-9]{8}", text) else text
    return None  # any other field type is left out


def dbf_rows(dbf, codec):
    """The .dbf's complete records in order, as (deleted, attrs)."""
    if len(dbf) < 32:
        raise GisError(".dbf başlığı 32 bayttan kısa")
    count = struct.unpack_from("<I", dbf, 4)[0]
    header_length, record_length = struct.unpack_from("<2H", dbf, 8)
    fields, at = [], 32
    while True:
        if at >= len(dbf):
            raise GisError(".dbf alan tanımlarının sonunda 0x0D yok")
        if dbf[at] == 0x0D:
            break
        if at + 32 > len(dbf):
            raise GisError(".dbf alan tanımı yarım")
        field = dbf[at : at + 32]
        fields.append((field[:11].split(b"\0", 1)[0].decode(codec, errors="replace"), chr(field[11]), field[16]))
        at += 32
    if 1 + sum(width for _, _, width in fields) > record_length:
        raise GisError(".dbf alanları kayıt uzunluğuna sığmıyor")
    rows = []
    for n in range(count):
        start = header_length + n * record_length
        if start + record_length > len(dbf):
            break  # only complete records count
        record = dbf[start : start + record_length]
        attrs, pos = {}, 1
        for name, kind, width in fields:
            value = dbf_value(kind, record[pos : pos + width], codec)
            pos += width
            if name and value is not None:  # a field without a name is left out
                attrs[name] = value
        rows.append((record[0] == 0x2A, attrs))  # `*`: deleted
    return rows


# ── .prj: WKT 1 (ESRI or OGC) ───────────────────────────────────────────

_WKT_TOKEN = re.compile(
    r'\s*(?:(?P<word>[A-Za-z_][A-Za-z0-9_]*)|(?P<number>[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)'
    r'|"(?P<text>(?:[^"]|"")*)"|(?P<mark>[\[\](),]))'
)
_OPEN = {"[": "]", "(": ")"}


class Wkt:
    """A WKT node: its keyword (upper-cased) and arguments, each a Wkt or a ("word" | "number" | "text", value) pair."""

    __slots__ = ("keyword", "args")

    def __init__(self, keyword, args):
        self.keyword, self.args = keyword, args

    def children(self, keyword):
        return [a for a in self.args if isinstance(a, Wkt) and a.keyword == keyword]

    def child(self, keyword):
        found = self.children(keyword)
        return found[0] if found else None

    def text(self, i):
        a = self.args[i] if i < len(self.args) else None
        return a[1] if isinstance(a, tuple) and a[0] == "text" else None

    def number(self, i):
        a = self.args[i] if i < len(self.args) else None
        return float(a[1]) if isinstance(a, tuple) and a[0] == "number" else None


def parse_wkt(source):
    tokens, at = [], 0
    while True:
        match = _WKT_TOKEN.match(source, at)
        if match is None:
            if source[at:].strip():
                raise GisError("WKT çözümlenemedi")
            break
        kind = match.lastgroup
        value = match.group(kind)
        tokens.append((kind, value.replace('""', '"') if kind == "text" else value))
        at = match.end()
    node, at = _wkt_node(tokens, 0)
    if at != len(tokens):
        raise GisError("WKT'nin ardında fazladan içerik var")
    return node


def _wkt_node(tokens, at):
    if at + 1 >= len(tokens) or tokens[at][0] != "word" or tokens[at + 1][0] != "mark" or tokens[at + 1][1] not in _OPEN:
        raise GisError("WKT düğümü bekleniyordu")
    keyword, close = tokens[at][1].upper(), _OPEN[tokens[at + 1][1]]
    args, at = [], at + 2
    while True:
        if at >= len(tokens):
            raise GisError("WKT yarım")
        kind, value = tokens[at]
        if kind == "word" and at + 1 < len(tokens) and tokens[at + 1][0] == "mark" and tokens[at + 1][1] in _OPEN:
            arg, at = _wkt_node(tokens, at)
        elif kind in ("word", "number", "text"):
            arg, at = (kind, value), at + 1
        else:
            raise GisError("WKT'de değer bekleniyordu")
        args.append(arg)
        if at < len(tokens) and tokens[at] == ("mark", ","):
            at += 1
        elif at < len(tokens) and tokens[at] == ("mark", close):
            return Wkt(keyword, args), at + 1
        else:
            raise GisError("WKT'de ',' ya da kapanış bekleniyordu")


def wkt_name(text):
    """A WKT name compared the rules' way: lowercase, ASCII letters and digits only."""
    return "".join(c.lower() for c in text if c.isascii() and c.isalnum())


def datum_family(name):
    n = wkt_name(name)
    if any(k in n for k in ("turef", "turkishnationalreferenceframe", "itrf96", "itrf1996")):
        return "TUREF"
    if any(k in n for k in ("european1950", "europeandatum1950", "ed50")):  # ESRI writes D_European_1950
        return "ED50"
    if "wgs1984" in n or "wgs84" in n:
        return "WGS84"
    return None


TM3 = {27.0: 0, 30.0: 1, 33.0: 2, 36.0: 3, 39.0: 4, 42.0: 5, 45.0: 6}  # central meridian: (cm - 27) / 3
UTM = {27.0: 0, 33.0: 1, 39.0: 2, 45.0: 3}  # central meridian: (cm - 27) / 6


def transverse_mercator(projcs, family):
    """Rule 3: a metre Transverse Mercator zone of TUREF, ED50 or WGS 84 by its parameters."""
    projection, unit = projcs.child("PROJECTION"), projcs.child("UNIT")  # the PROJCS's own children only
    if projection is None or projection.text(0) is None or "transversemercator" not in wkt_name(projection.text(0)):
        return None
    if unit is None or unit.number(1) != 1.0:
        return None
    params = {}
    for parameter in projcs.children("PARAMETER"):
        name, value = parameter.text(0), parameter.number(1)
        if name is None or value is None:
            return None
        params[wkt_name(name)] = value
    get = params.get  # a missing parameter counts as 0, the scale factor as 1
    if get("falseeasting", 0.0) != 500000.0 or get("falsenorthing", 0.0) != 0.0 or get("latitudeoforigin", 0.0) != 0.0:
        return None
    scale, cm = get("scalefactor", 1.0), get("centralmeridian", 0.0)
    if family == "TUREF" and scale == 1.0 and cm in TM3:
        return 5253 + TM3[cm]
    if family == "ED50" and scale == 1.0 and cm in TM3:
        return 2319 + TM3[cm]
    if family == "ED50" and scale == 0.9996 and cm in UTM:
        return 23035 + UTM[cm]
    if family == "WGS84" and scale == 0.9996 and cm in UTM:
        return 32635 + UTM[cm]
    return None


def prj_srid(text):
    """The SRID a .prj declares, or None (no .prj, unreadable or unrecognised WKT)."""
    if text is None:
        return None
    try:
        root = parse_wkt(text)
    except (GisError, RecursionError):
        return None
    if root.keyword not in ("PROJCS", "GEOGCS"):
        return None
    for authority in root.children("AUTHORITY"):  # 1. a direct child of the outermost node
        code = authority.text(1)
        if len(authority.args) == 2 and authority.text(0) == "EPSG" and code is not None and re.fullmatch(r"[0-9]+", code):
            return int(code)
    geogcs = root if root.keyword == "GEOGCS" else root.child("GEOGCS")
    datum = geogcs.child("DATUM") if geogcs is not None else None
    name = datum.text(0) if datum is not None else None
    family = datum_family(name) if name is not None else None  # 2. the datum
    if root.keyword == "GEOGCS":  # 4. a geographic system alone
        return {"WGS84": 4326, "TUREF": 5252}.get(family)
    return transverse_mercator(root, family)  # 3.


# ── Reading a set ───────────────────────────────────────────────────────


def read_shapefile(shp, shx, dbf, prj_text, cpg_text, layer):
    """A Shapefile set's canonical dict; dbf, prj_text and cpg_text are None when the file is absent.
    `shx` is accepted for the set's sake but not read: the records come from the .shp in order."""
    encoding = shapefile_encoding(cpg_text, dbf)
    rows = dbf_rows(dbf, CODECS[encoding]) if dbf is not None else []
    out = []
    for n, content in enumerate(shp_records(shp)):
        deleted, attrs = rows[n] if n < len(rows) else (False, {})  # the n-th .dbf record gives the n-th shape's attrs
        if not deleted:
            target = Target(out, layer, None, attrs)
            for kind, geometry in shp_objects(content):
                target.add(kind, geometry)
    return {"declaredSrid": prj_srid(prj_text), "encoding": encoding, "objects": out}


def read_shapefile_files(files, layer):
    """A set given as {".shp": bytes, ".shx": …, ".dbf": …, ".prj": …, ".cpg": …}; the text files decoded as the CLI does."""
    prj, cpg = files.get(".prj"), files.get(".cpg")
    return read_shapefile(
        files[".shp"],
        files.get(".shx"),
        files.get(".dbf"),
        None if prj is None else prj.decode("utf-8", errors="replace"),
        None if cpg is None else cpg.decode("latin-1"),
        layer,
    )


def read_path(path, layer):
    """A GeoJSON file, or a .shp with the set's other files beside it."""
    path = Path(path)
    if path.suffix.lower() != ".shp":
        return read_geojson(path.read_bytes(), layer)
    files = {".shp": path.read_bytes()}
    for ext in (".shx", ".dbf", ".prj", ".cpg"):
        for candidate in (path.with_suffix(ext), path.with_suffix(ext.upper())):
            if candidate.is_file():
                files[ext] = candidate.read_bytes()
                break
    return read_shapefile_files(files, layer)


def main(argv=None):
    parser = argparse.ArgumentParser(prog="gis.py", description="GeoJSON ya da Shapefile'ı ortak kurallarla okur, kanonik JSON'unu yazar.")
    commands = parser.add_subparsers(dest="command", required=True)
    read = commands.add_parser("read", help="bir .geojson ya da .shp dosyasını okur")
    read.add_argument("path")
    read.add_argument("--layer", help="varsayılan katmanın adı (yoksa dosyanın uzantısız adı)")
    args = parser.parse_args(argv)
    path = Path(args.path)
    try:
        result = read_path(path, path.stem if args.layer is None else args.layer)
    except (GisError, OSError) as e:
        print(f"{path}: {e}", file=sys.stderr)
        return 1
    sys.stdout.buffer.write(dumps(result).encode("utf-8"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
