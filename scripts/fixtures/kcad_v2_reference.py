"""Independent reference writer of the KCAD v2 fixtures (docs/specs/kcad-v2.md, docs/adr/0025).

Writes fixtures/kcad/v2 with Python's standard library only, never with KentOS
code: every valid file is encoded here from a drawing written by hand in the
contract's JSON form (`*.json`, the `DocumentSnapshotV2` shape), every broken
one is built byte by byte to break exactly one rule of the specification.
The Rust codec must write the valid files byte for byte from the same
drawings and refuse the broken ones with the same error codes; the browser's
build of it and tools/kcad/kcad.py (the independent reader) read them too.
expected.json, written by hand from the specification, says what each file is.

    python3 scripts/fixtures/kcad_v2_reference.py           # writes the fixtures
    python3 scripts/fixtures/kcad_v2_reference.py --check   # writes nothing; compares and reads

`--check` rebuilds every file in memory and compares it with the one on disk,
then reads every file listed in expected.json with the reader and compares the
outcome: the sniffed kind, the error code, or the drawing (floats bit for bit).

migrated.json is not written by hand: it is fixtures/document/v1/sample.json
(the web app's recorded v1 sample) with the ids the independent v1 reference
derived for it (fixtures/document/v1/identity/expected.json, docs/adr/0014),
the project id and the migration source, its former Hibrit mode left out (a
project type not asked yet, docs/adr/0165 §1); so it is what opening that v1
file and saving it as v2 must give on every platform.
"""
import base64
import copy
import hashlib
import json
import math
import re
import unicodedata
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/kcad/v2"
sys.dont_write_bytecode = True  # no __pycache__ in the tree
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402  (the independent reader, for --check)

MAGIC = b"\x89KCAD\r\n\x1a\n"

# ── CBOR, KentOS profile 1 (spec §5) ────────────────────────────────────


# A text's alignments (spec §6.6, docs/adr/0145); the left of the baseline is the field's absence.
TEXT_ALIGNS = ("baselineCenter", "baselineRight", "bottomLeft", "bottomCenter", "bottomRight", "middleLeft", "middleCenter", "middleRight", "topLeft", "topCenter", "topRight")
# A dimension's kinds (spec §6.6); schema 9 added the last five (docs/adr/0147).
DIMENSION_STYLES = ("aligned", "linear", "angular", "radius", "diameter", "ordinate", "arcLength", "jogged", "azimuth", "slope")
SCHEMA_9_STYLES = DIMENSION_STYLES[5:]
# A leader's arrowheads (spec §6.6, docs/adr/0146); the filled arrow is the field's absence. Schema 30 added AutoCAD's
# others (docs/adr/0205 §7).
LEADER_ARROWS = ("open", "dot", "none")
LEADER_ARROWS_30 = ("closed", "open30", "open90", "dotSmall", "dotBlank", "oblique", "archTick", "boxFilled", "boxBlank", "datumFilled")
# Schema 30 (docs/adr/0205): the annotation heights' kinds, a dimension's line fields and their bounds.
ANNOTATION_KINDS = ("text", "leader", "dimension", "table", "coordinate", "station", "measure")
LINE_TYPES = ("continuous", "dashed", "dashdot", "dotted")
DIMENSION_LINES = ("dimLineColor", "dimLineWeight", "dimLineType", "extColor", "extWeight", "extLineType", "textColor")
# Schema 21 (docs/adr/0183): the drawing typefaces and a dimension's arrowheads by name.
DRAWING_FONTS = ("barlow", "arimo", "overpass", "quicksand", "architects-daughter", "courier-prime", "plex-mono")
DIMENSION_ARROWS = ("closed", "open", "dot", "none")
# Schema 23 (docs/adr/0186): a hatch's pattern kinds and a gradient's shapes by name.
HATCH_TYPES = ("solid", "lines", "cross", "pattern", "gradient")
GRADIENT_SHAPES = ("linear", "cylinder", "spherical")


def head(major, arg):
    """An initial byte and its argument in the shortest form."""
    if arg < 24:
        return bytes([major << 5 | arg])
    if arg < 1 << 8:
        return bytes([major << 5 | 24, arg])
    if arg < 1 << 16:
        return bytes([major << 5 | 25]) + arg.to_bytes(2, "big")
    if arg < 1 << 32:
        return bytes([major << 5 | 26]) + arg.to_bytes(4, "big")
    return bytes([major << 5 | 27]) + arg.to_bytes(8, "big")


def uint(n):
    assert type(n) is int and 0 <= n < 1 << 64
    return head(0, n)


def integer(n):
    assert type(n) is int and -(1 << 64) <= n < 1 << 64
    return head(0, n) if n >= 0 else head(1, -1 - n)


def f64(x):
    x = float(x)
    assert math.isfinite(x), "NaN ya da sonsuz yazılamaz"
    return b"\xfb" + struct.pack(">d", x)


def text(s):
    raw = s.encode("utf-8")
    return head(3, len(raw)) + raw


def blob(b):
    return head(2, len(b)) + b


def array(items):
    return head(4, len(items)) + b"".join(items)


def cmap(entries):
    """A map with text keys in RFC 8949 §4.2.1 order: by the bytes of the encoded key."""
    keyed = sorted((text(k), v) for k, v in entries.items())
    return head(5, len(keyed)) + b"".join(k + v for k, v in keyed)


def raw_map(pairs):
    """A map written in the given order (for broken fixtures)."""
    return head(5, len(pairs)) + b"".join(k + v for k, v in pairs)


def opaque(v):
    """A JSON value of the opaque parts (styles, renderers): integers and floats stay apart."""
    if v is None:
        return b"\xf6"
    if v is True:
        return b"\xf5"
    if v is False:
        return b"\xf4"
    if type(v) is int:
        return integer(v)
    if type(v) is float:
        return f64(v)
    if type(v) is str:
        return text(v)
    if type(v) is list:
        return array([opaque(x) for x in v])
    if type(v) is dict:
        return cmap({k: opaque(x) for k, x in v.items()})
    raise TypeError(type(v))


# ── Document schemas 2 to 6 (spec §6), from the contract's JSON form ────


def uid_bytes(u):
    raw = bytes.fromhex(u.replace("-", ""))
    assert len(raw) == 16 and u == kcad.uuid_text(raw), u
    return raw


def point(p):
    assert set(p) == {"x", "y"}, p
    return array([f64(p["x"]), f64(p["y"])])


def points(list_):
    return array([point(p) for p in list_])


def floats(list_):
    return array([f64(x) for x in list_])


def fields(obj, table, where):
    """The encoded fields of `obj` by `table` {name: (encode, required)}; unknown or missing ones stop the writer."""
    unknown = set(obj) - set(table)
    assert not unknown, f"{where}: bilinmeyen alan {sorted(unknown)}"
    out = {}
    for name, (encode, required) in table.items():
        if name in obj:
            out[name] = encode(obj[name])
        else:
            assert not required, f"{where}: {name} eksik"
    return out


def enum(values):
    def encode(v):
        assert v in values, v
        return text(v)

    return encode


# ── The project's coordinate systems and datum choices (schema 13, spec §6.4.1, docs/adr/0168) ──────────────────

REGISTRY_DATUMS = ("TUREF", "ED50", "WGS84")


def finite(v):
    assert isinstance(v, float) and math.isfinite(v), v
    return f64(v)


def three(v):
    assert len(v) == 3, v
    return array([finite(x) for x in v])


def helmert(h):
    assert h["convention"] in ("positionVector", "coordinateFrame"), h
    assert "accuracy" not in h or h["accuracy"] >= 0, h
    return cmap(fields(h, {"translation": (three, True), "rotation": (three, True), "scale": (finite, True),
                           "convention": (text, True), "accuracy": (finite, False)}, "helmert"))


def custom_datum(d):
    e = d["ellipsoid"]
    assert d["name"].strip() and e["name"].strip() and e["semiMajor"] > 0 and e["inverseFlattening"] > 1, d
    ellipsoid = cmap(fields(e, {"name": (text, True), "semiMajor": (finite, True), "inverseFlattening": (finite, True)},
                            "ellipsoid"))
    return cmap(fields(d, {"name": (text, True), "ellipsoid": (lambda _: ellipsoid, True), "toWgs84": (helmert, False)},
                       "customDatum"))


def plane(p):
    if p["kind"] == "similarity":
        assert p["scale"] > 0, p
        table = {k: (finite, True) for k in ("east", "north", "rotation", "scale")}
    else:
        assert p["kind"] == "affine" and p["a"] * p["e"] - p["b"] * p["d"] != 0, p
        table = {k: (finite, True) for k in "abcdef"}
    return cmap(fields(p, {"kind": (text, True), **table}, "plane"))


def crs_base(b):
    assert ("srid" in b) != ("definition" in b), b
    if "srid" in b:
        assert b["srid"] != 0, b
        return cmap({"srid": uint(b["srid"])})
    assert b["definition"]["system"]["kind"] == "tm", b
    return cmap({"definition": crs_definition(b["definition"])})


def crs_system(s):
    kind = s["kind"]
    if kind in ("tm", "geographic"):
        assert ("datum" in s) != ("customDatum" in s), s
    if kind == "tm":
        assert s["scaleFactor"] > 0 and -180 <= s["centralMeridian"] <= 180 and -90 <= s.get("latitudeOfOrigin", 0.0) <= 90, s
        table = {"kind": (text, True), "datum": (enum(REGISTRY_DATUMS), False), "customDatum": (custom_datum, False),
                 "latitudeOfOrigin": (finite, False), "centralMeridian": (finite, True), "scaleFactor": (finite, True),
                 "falseEasting": (finite, True), "falseNorthing": (finite, True)}
    elif kind == "geographic":
        table = {"kind": (text, True), "datum": (enum(REGISTRY_DATUMS), False), "customDatum": (custom_datum, False)}
    else:
        assert kind == "local", s
        table = {"kind": (text, True), "base": (crs_base, True), "plane": (plane, True)}
    return cmap(fields(s, table, "system"))


def crs_definition(d):
    assert d["name"].strip(), d
    return cmap(fields(d, {"name": (text, True), "system": (crs_system, True)}, "definition"))


def grid_choice(g):
    assert re.fullmatch(r"[0-9a-f]{64}", g["id"]) and g["size"] > 0 and ("accuracy" not in g or g["accuracy"] >= 0), g
    return cmap(fields(g, {"id": (text, True), "file": (text, True), "size": (uint, True), "accuracy": (finite, False)},
                       "grid"))


def datum_transforms(ts):
    pairs = []
    out = []
    for t in ts:
        assert t["from"] in REGISTRY_DATUMS and t["to"] in REGISTRY_DATUMS and t["from"] != t["to"], t
        assert ("helmert" in t) != ("grid" in t) and t["name"].strip(), t
        pair = frozenset((t["from"], t["to"]))
        assert pair not in pairs, "bir datum çifti için en çok bir seçim"
        pairs.append(pair)
        out.append(cmap(fields(t, {"from": (text, True), "to": (text, True), "name": (text, True),
                                   "helmert": (helmert, False), "grid": (grid_choice, False)}, "datumTransform")))
    return array(out)


def settings(s):
    second = s.get("secondSrid")
    has_system = s["srid"] != 0 or "customCrs" in s
    # A second coordinate system is another system than the project's own; a project without one has none (docs/adr/0167
    # §1; a definition of its own is one, 0168 §1).
    assert second is None or (second != 0 and has_system and second != s["srid"]), "ikinci sistem projeninkinden başka olmalı"
    assert "customCrs" not in s or s["srid"] == 0, "kendi tanımı yalnız srid 0 projede"
    assert "lengthStyle" not in s or s.get("drawingUnit") in ("in", "ft"), "ayak ve inç biçimi yalnız inç ya da ayak biriminde"
    assert "secondCustomCrs" not in s or (second is None and has_system), "ikinci tanım ikinci EPSG koduyla birlikte olmaz"
    return cmap(
        fields(
            s,
            {
                "srid": (uint, True),
                "lengthDecimals": (uint, True),
                "areaDecimals": (uint, True),
                "areaUnit": (enum(("m2", "donum", "ha")), True),
                "angleUnit": (enum(("grad", "deg")), True),
                "plotScale": (f64, True),
                "workspace": (enum(("hybrid", "cad", "gis", "plan3d", "disaster")), False),
                "drawingFont": (enum(("barlow", "arimo", "overpass", "quicksand", "architects-daughter", "courier-prime", "plex-mono")), False),
                # Schema 50 (docs/adr/0262 §1): inches and feet too, written only in schema 50.
                "drawingUnit": (enum(("mm", "cm", "m", "in", "ft")), False),
                "secondSrid": (uint, False),
                "customCrs": (crs_definition, False),
                "secondCustomCrs": (crs_definition, False),
                "datumTransforms": (datum_transforms, False),
                "survey": (survey, False),
                "layerStates": (layer_states, False),
                "textStyles": (text_styles, False),
                "dimensionStyles": (dimension_styles, False),
                "topology": (topology, False),
                "annotation": (annotation, False),
                # Schema 32 (docs/adr/0208 §2): the project's connections to map services, without their secrets.
                "connections": (connections, False),
                # Schema 33 (docs/adr/0209 §2): the project's networks.
                "networks": (networks, False),
                # Schema 37 (docs/adr/0214 §2.3): the project's variables.
                "variables": (variables, False),
                # Schema 42 (docs/adr/0229 §2): the project's address data.
                "addressLocators": (address_locators, False),
                # Schema 49 (docs/adr/0261 §1): the current UCS (none: the world) and the named ones.
                "ucs": (ucs, False),
                "namedUcs": (named_ucs, False),
                # Schema 52 (docs/adr/0285 §1): the project's named views.
                "namedViews": (named_views, False),
                # Schema 50 (docs/adr/0262 §2): how lengths are written; decimal is no setting.
                "lengthStyle": (enum(("architectural", "engineering", "fractional")), False),
                # Schema 56 (docs/adr/0313 §1): the project's line types with patterns and its line type scale.
                "lineTypes": (line_types, False),
                "lineTypeScale": (line_type_scale, False),
            },
            "settings",
        )
    )


def annotation(h):
    """The project's annotation heights (schema 30, spec §6.4.6, docs/adr/0205 §1): a kind's height on paper, over 0 and
    at most 100 mm; not empty."""
    assert h and all(0 < h[k] <= 100 for k in h), h
    return cmap(fields(h, {k: (finite, False) for k in ANNOTATION_KINDS}, "annotation"))


def hex_colour(c):
    assert len(c) == 7 and c[0] == "#" and all(ch in "0123456789abcdefABCDEF" for ch in c[1:]), c
    return text(c)


def line_weight(w):
    assert 0 <= w <= 100, w
    return finite(w)


# A dimension's and a dimension style's lines (schema 30, docs/adr/0205 §6).
LINE_FIELDS = {
    "dimLineColor": (hex_colour, False),
    "dimLineWeight": (line_weight, False),
    "dimLineType": (enum(LINE_TYPES), False),
    "extColor": (hex_colour, False),
    "extWeight": (line_weight, False),
    "extLineType": (enum(LINE_TYPES), False),
    "textColor": (hex_colour, False),
}


def leader_arrow_size(x):
    assert 0.1 <= x <= 10, x
    return finite(x)


# A point cloud's formats and looks (schema 31, docs/adr/0207 §5): pixels and round points are the fields' absence.
CLOUD_FORMATS = ("las", "laz", "copc", "xyz")
CLOUD_RENDERS = ("rgb", "classification", "elevation", "intensity", "returns", "single")


def cloud_source(f):
    """A point cloud's file (schema 31, docs/adr/0207 §3): its one source, format, points and bounds."""
    table = {
        "asset": (text, False),
        "file": (text, False),
        "url": (text, False),
        "format": (enum(CLOUD_FORMATS), True),
        "count": (uint, True),
        "bounds": (floats, True),
    }
    return cmap(fields(f, table, "nokta bulutunun dosyası"))


def cloud_style(st):
    """A point cloud's look (schema 31, docs/adr/0207 §5): `render` and `size`, the rest only when not their default."""
    table = {
        "render": (enum(CLOUD_RENDERS), True),
        "ramp": (enum(RASTER_RAMPS), False),
        "invert": (lambda b: boolean(b) if b is True else None, False),
        "min": (f64, False),
        "max": (f64, False),
        "hidden": (lambda hs: array([uint(h) for h in hs]) if hs else None, False),
        "rgb8": (lambda b: boolean(b) if b is True else None, False),
        "size": (f64, True),
        "sizeUnit": (enum(("m",)), False),
        "shape": (enum(("square",)), False),
    }
    out = fields(st, table, "nokta bulutunun görünüşü")
    assert all(out.get(k, True) is not None for k in ("invert", "rgb8", "hidden")), "varsayılanlar alan değildir"
    return cmap(out)


def cloud_shape(e):
    """The contract's point cloud rule, as far as an example needs it held (docs/adr/0207 §3)."""
    assert 1 <= len(e["sources"]) <= 4096, "nokta bulutunun dosyaları"
    assert all(sum(k in f for k in ("asset", "file", "url")) == 1 for f in e["sources"]), "her dosyanın tek kaynağı olmalı"
    assert sum(f["count"] for f in e["sources"]) == e["count"], "nokta sayısı dosyalarınkinin toplamı"
    st = e["style"]
    assert 0.5 <= st["size"] <= 32.0, "noktanın boyu"
    assert st["render"] not in ("elevation", "intensity") or ("min" in st and "max" in st and st["min"] < st["max"]), "rampanın aralığı"
    assert 0.1 <= e.get("opacity", 1.0) <= 1.0, "donukluk 0,1 ile 1 arasında"


TOPOLOGY_KINDS = {
    "mustNotOverlap": (False, None),
    "mustNotHaveGaps": (False, None),
    "mustNotHaveSlivers": (False, "length"),
    "mustNotHaveDuplicates": (False, None),
    "mustNotHaveDangles": (False, None),
    "mustNotHaveShortEdges": (False, "length"),
    "mustNotHaveSmallAngles": (False, "angle"),
    "mustBeValid": (False, None),
    "mustNotHaveMissingVertices": (False, None),
    "mustNotOverlapWith": (True, None),
    "mustBeCoveredBy": (True, None),
    "boundaryMustBeCoveredBy": (True, None),
    "mustBeOnEndOf": (True, None),
}


def topology(t):
    """The project's topology settings (schema 27, spec §6.4.5, docs/adr/0202 §7): a tolerance within [1e-6, 1] m; rules
    with an id neither empty nor twice, a known kind, a layer, another layer only between two layers (neither empty nor
    the rule's own), a value only for a kind that takes one (above zero, an angle below a right angle); exceptions of a
    rule there is, with objects (16-byte ids) and a place; not all empty."""
    assert t.get("rules") or t.get("exceptions") or "tolerance" in t, "boş topoloji ayarı yazılmaz"
    assert "tolerance" not in t or 1e-6 <= t["tolerance"] <= 1, t
    ids = set()
    rules = []
    for r in t.get("rules", []):
        between, value = TOPOLOGY_KINDS[r["kind"]]
        assert r["id"] and r["id"] not in ids and r["layer"], r
        ids.add(r["id"])
        assert ("other" in r) == between and (not between or (r["other"] and r["other"] != r["layer"])), r
        assert "value" not in r or (value is not None and r["value"] > 0 and (value != "angle" or r["value"] < math.pi / 2)), r
        rules.append(cmap(fields(r, {"id": (text, True), "kind": (enum(tuple(TOPOLOGY_KINDS)), True), "layer": (text, True),
                                     "other": (text, False), "value": (finite, False)}, "topologyRule")))
    exceptions = []
    for x in t.get("exceptions", []):
        assert x["rule"] in ids and x["objects"], x
        exceptions.append(cmap({"at": point(x["at"]), "rule": text(x["rule"]), "objects": array([blob(uid_bytes(u)) for u in x["objects"]])}))
    out = {}
    if rules:
        out["rules"] = array(rules)
    if "tolerance" in t:
        out["tolerance"] = finite(t["tolerance"])
    if exceptions:
        out["exceptions"] = array(exceptions)
    assert set(t) <= {"rules", "tolerance", "exceptions"}, t
    return cmap(out)


def style_names(styles):
    """The style tables' name rule (schema 21, docs/adr/0183 §5): an id neither empty nor twice; a name not empty, without
    spaces at its ends, at most 64 letters, without a control character, not “Standart” and not twice, whatever its
    letters' case."""
    assert styles, "boş liste yazılmaz"
    ids, names = set(), set()
    for st in styles:
        name = st["name"]
        assert st["id"] and st["id"] not in ids, "stilin kimliği boş ya da iki kez"
        assert name.strip() == name and name and len(name) <= 64, "stilin adı boş, boşluklu ya da uzun"
        assert name.lower() != "standart" and name.lower() not in names, "stilin adı ayrılmış ya da iki kez"
        ids.add(st["id"])
        names.add(name.lower())


def text_styles(styles):
    """The project's named text styles (schema 21, docs/adr/0183 §2): a typeface by name, bold and italic only when true,
    the slant, the height on paper (mm), the width factor (never 1) and the DXF typeface file."""
    style_names(styles)
    flag = (lambda b: boolean(b) if b is True else None, False)
    table = {
        "id": (text, True),
        "name": (text, True),
        "font": (enum(DRAWING_FONTS), True),
        "bold": flag,
        "italic": flag,
        "oblique": (f64, False),
        "height": (f64, False),
        "widthFactor": (f64, False),
        "fontFile": (text, False),
    }
    for st in styles:
        assert st.get("widthFactor", 2.0) != 1.0, "genişlik çarpanı 1 yazılmaz"
        assert st.get("oblique", 1.0) != 0.0 and abs(st.get("oblique", 1.0)) < 85.0, "eğiklik sınır dışında"
    return array([cmap(fields(st, table, "textStyle")) for st in styles])


def dimension_styles(styles):
    """The project's named dimension styles (schema 21, docs/adr/0183 §3): the value's height on paper (mm), the
    arrowheads by name and the sizes (mm), the value's place, decimals, unit, prefix, suffix and typeface."""
    style_names(styles)
    table = {
        "id": (text, True),
        "name": (text, True),
        "height": (f64, True),
        "arrow": (enum(DIMENSION_ARROWS), False),
        "arrowSize": (f64, False),
        "extOffset": (f64, False),
        "extBeyond": (f64, False),
        "textGap": (f64, False),
        "textPlace": (enum(("centre",)), False),
        "decimals": (uint, False),
        "unit": (enum(("mm", "cm", "m")), False),
        "prefix": (text, False),
        "suffix": (text, False),
        "font": (enum(DRAWING_FONTS), False),
        **LINE_FIELDS,
    }
    return array([cmap(fields(st, table, "dimensionStyle")) for st in styles])


def layer_states(states):
    """The project's named layer states (schema 19, docs/adr/0177 §4): an id and a name, neither empty nor twice (a name
    as written, the spaces at its ends aside); in each, the nodes, none empty or twice, each with its visibility and,
    when kept, its lock and a layer's style."""
    assert states, "boş liste yazılmaz"
    ids, names, out = set(), set(), []
    for st in states:
        name = st["name"].strip()
        assert st["id"] and st["id"] not in ids, "katman durumunun kimliği boş ya da iki kez"
        assert name and name not in names, "katman durumunun adı boş ya da iki kez"
        ids.add(st["id"])
        names.add(name)
        seen, nodes = set(), []
        for n in st["nodes"]:
            assert n["node"] and n["node"] not in seen, "düğüm boş ya da iki kez"
            seen.add(n["node"])
            # Schema 55 (docs/adr/0311 §3): the node's own freeze and printing, kept by every state since.
            nodes.append(cmap(fields(n, {"node": (text, True), "style": (layer_style, False), "locked": (boolean, False), "visible": (boolean, True),
                                         "frozen": (boolean, False), "plot": (boolean, False)}, "layerStateNode")))
        out.append(cmap(fields(st, {"id": (text, True), "name": (text, True), "nodes": (lambda _: array(nodes), True)}, "layerState")))
    return array(out)


def survey(s):
    """The project's survey settings (docs/adr/0169 §3): at least one, k within [−1, 1], tolerances above zero; the
    ground height within [−500, 9000] m and the reduction to the grid only with one (schema 16, docs/adr/0171)."""
    assert s, "ölçme ayarları boş olamaz"
    assert "refraction" not in s or -1.0 <= s["refraction"] <= 1.0, "k −1 ile 1 arasında olmalı"
    assert all(s[k] > 0.0 for k in ("faceHz", "index", "faceSlope", "twoWay", "traverseAngle", "traverseCoord") if k in s), "tolerans sıfırdan büyük olmalı"
    assert "groundHeight" not in s or -500.0 <= s["groundHeight"] <= 9000.0, "ortalama yükseklik −500 ile 9000 m arasında olmalı"
    assert s.get("reduceToGrid") is not True or "groundHeight" in s, "projeksiyona indirme yükseklik ister"
    assert all(s[k] > 0.0 for k in ("sigmaDirection", "sigmaDistance", "sigmaZenith", "sigmaLevelling") if k in s), "önsel doğruluk sıfırdan büyük olmalı"
    assert all(s[k] >= 0.0 for k in ("sigmaPpm", "sigmaCentering") if k in s), "ppm ve merkezleme sıfırdan küçük olamaz"
    return cmap(fields(s, {"index": (f64, False), "faceHz": (f64, False), "faceSlope": (f64, False), "refraction": (f64, False),
                           "twoWay": (f64, False), "traverseAngle": (f64, False), "traverseCoord": (f64, False),
                           "groundHeight": (f64, False), "reduceToGrid": (boolean, False),
                           **{k: (f64, False) for k in SIGMA_KEYS}}, "survey"))


# The survey settings' a priori standard deviations (schema 28, docs/adr/0203 §1).
SIGMA_KEYS = ("sigmaDirection", "sigmaDistance", "sigmaPpm", "sigmaCentering", "sigmaZenith", "sigmaLevelling")


# The label engine's fields of a label style (schema 36, docs/adr/0212 §2), each with its encoder.
LABEL_ENGINE = {
    "text": (text, False),
    "color": (text, False),
    "italic": (lambda b: boolean(b), False),
    "align": (enum(("left", "center", "right")), False),
    "point": (enum(("around", "center")), False),
    "line": (enum(("parallel", "curved", "horizontal", "contour")), False),
    "area": (enum(("horizontal", "free", "perimeter", "boundary", "parcel", "corner")), False),
    "position": (enum(("on", "above", "below", "sides")), False),
    "distance": (f64, False),
    "repeat": (f64, False),
    "maxAngle": (f64, False),
    "curved": (lambda b: boolean(b), False),
    "mergeLines": (lambda b: boolean(b), False),
    "inside": (lambda b: boolean(b), False),
    "outside": (lambda b: boolean(b), False),
    "halo": (lambda h: cmap(fields(h, {"width": (f64, True), "color": (text, False)}, "halo")), False),
    "background": (lambda b: cmap(fields(b, {"shape": (enum(("rect", "round", "ellipse")), True), "fill": (text, False),
                                               "stroke": (text, False), "padding": (f64, False)}, "background")), False),
    "shadow": (lambda h: cmap(fields(h, {"dx": (f64, True), "dy": (f64, True), "color": (text, False), "opacity": (f64, False)}, "shadow")), False),
    "callout": (lambda c: cmap(fields(c, {"kind": (enum(("straight", "manhattan")), True), "color": (text, False), "width": (f64, False),
                                            "minLength": (f64, False)}, "callout")), False),
    "stack": (lambda k: cmap(fields(k, {"mode": (enum(("ifNeeded", "always")), True), "chars": (uint, True), "at": (text, False)}, "stack")), False),
    "abbreviate": (lambda a: cmap(fields(a, {"always": (lambda b: boolean(b), False), "words": (
        lambda ws: array([cmap(fields(w, {"word": (text, True), "short": (text, True)}, "word")) for w in ws]), True)}, "abbreviate")), False),
    "shrink": (f64, False),
    "priority": (uint, False),
    "overlap": (enum(("never", "ifNeeded", "always")), False),
    "duplicates": (f64, False),
}


def label_style(s):
    """A label style: its first fields every schema writes; the label engine's (schema 36, docs/adr/0212 §2)."""
    return cmap(
        fields(
            s,
            {
                "placement": (enum(("center", "corner", "beside", "along")), True),
                "size": (f64, True),
                "grow": (f64, False),
                "maxSize": (f64, False),
                "weight": (uint, False),
                "template": (text, False),
                "minFeaturePx": (f64, False),
                "minScale": (f64, False),
                "maxScale": (f64, False),
                "ink": (enum(("fg", "fg-dim", "label")), False),
                **LABEL_ENGINE,
            },
            "label",
        )
    )


def layer_labels(l):
    """A layer style's labelling (schema 36, docs/adr/0212 §2): its mode, its rules' classes (an empty list not
    written) and its objects as obstacles."""
    assert l.get("classes", [None]), "boş sınıf listesi yazılmaz"
    cls = lambda c: cmap(fields(c, {"name": (text, True), "when": (text, False), "style": (label_style, True)}, "class"))
    return cmap(fields(l, {"mode": (enum(("single", "rules", "off")), True), "classes": (lambda cs: array([cls(c) for c in cs]), False),
                           "obstacle": (lambda o: cmap(fields(o, {"weight": (uint, True), "kind": (enum(("interior", "boundary")), False)}, "obstacle")), False)},
                       "labels"))


def label_pins(ps):
    """An object's labels pinned by hand (schema 36, docs/adr/0212 §2): an empty list not written, hidden only true."""
    assert ps, "boş iğne listesi yazılmaz"
    assert all(p.get("hidden", True) is True for p in ps), "hidden yalnız true yazılır"
    return array([cmap(fields(p, {"class": (text, False), "at": (point, False), "rotation": (f64, False), "hidden": (lambda b: boolean(b), False)}, "pin"))
                  for p in ps])


def attachments(xs):
    """An object's photos and documents (schema 40, docs/adr/0225 §1): an empty list not written; each its name and one
    source (an embedded asset, a linked file or a web address), its note when given."""
    assert xs, "boş ek listesi yazılmaz"
    assert len(xs) <= 64, "en çok 64 ek"
    for a in xs:
        assert sum(k in a for k in ("asset", "file", "url")) == 1, f"ekin tek kaynağı olmalı: {a}"
        assert a["name"].strip(), "ekin adı boş"
    return array([cmap(fields(a, {"name": (text, True), "asset": (text, False), "file": (text, False), "url": (text, False), "note": (text, False)}, "attachment"))
                  for a in xs])


def layer_actions(xs):
    """A layer's actions (schema 40, docs/adr/0225 §4): an empty list not written; each its name and target."""
    assert xs, "boş eylem listesi yazılmaz"
    assert len({a["name"].lower() for a in xs}) == len(xs), "aynı adlı iki eylem"
    return array([cmap(fields(a, {"name": (text, True), "target": (text, True)}, "action")) for a in xs])


def info_card(c):
    """A layer's information card (schema 40, docs/adr/0225 §5): its title, fields and text when given, its switches only
    when off."""
    assert c.get("fields", [None]), "boş alan listesi yazılmaz"
    assert c.get("attachments", False) is False and c.get("actions", False) is False, "kartın düğmeleri yalnız kapalıyken yazılır"
    return cmap(fields(c, {"title": (text, False), "fields": (lambda fs: array([text(f) for f in fs]), False), "text": (text, False),
                           "attachments": (boolean, False), "actions": (boolean, False)}, "card"))


BLEND_MODES = ("multiply", "screen", "overlay", "darken", "lighten", "colorDodge", "colorBurn", "hardLight", "softLight", "difference", "exclusion", "addition")


def layer_mask(m):
    """A layer's mask (schema 41, docs/adr/0226 §3): which of its labels and markers cut (each written only when on, one
    at least), how far they grow (0–10 paper mm) and the 1–64 layers they cut into, each once."""
    assert m.get("labels") is True or m.get("markers") is True, "maskenin kaynağı yok"
    assert all(m.get(k, True) is True for k in ("labels", "markers")), "maskenin kaynağı yalnız açıkken yazılır"
    assert 0 <= m["size"] <= 10, "maskenin boyu 0–10 mm"
    ts = m["targets"]
    assert 1 <= len(ts) <= 64 and len(set(ts)) == len(ts) and all(t.strip() for t in ts), f"maskelenenler: {ts}"
    return cmap(fields(m, {"size": (f64, True), "labels": (boolean, False), "markers": (boolean, False), "targets": (lambda xs: array([text(x) for x in xs]), True)}, "mask"))


def layer_compose(c):
    """A layer's composition (schema 41, docs/adr/0226 §3, §4): its opacity when not 1 (0–1), its blending mode when not
    normal, its mask; one of them at least."""
    assert c, "katman çizimi boş"
    assert "opacity" not in c or 0 <= c["opacity"] < 1, "saydamlık 0 ile 1 arasında; 1 yazılmaz"
    assert "blend" not in c or c["blend"] in BLEND_MODES, "normal kip yazılmaz"
    return cmap(fields(c, {"opacity": (f64, False), "blend": (enum(BLEND_MODES), False), "mask": (layer_mask, False)}, "compose"))


def not_null(v):
    assert v is not None
    return opaque(v)


def line_type_scale(x):
    """Schema 56's LTSCALE (docs/adr/0313 §3): above 0, at most 1000, never 1 (its absence)."""
    assert 0 < x <= 1000 and x != 1.0, "çizgi tipi ölçeği 0'dan büyük, en çok 1000, 1 yazılmaz"
    return f64(x)


def pattern_place(p):
    """A complex element's placing: only what is not its default (scale 1, the rest 0 or false)."""
    assert p.get("scale", 1.0) > 0, "ölçek 0'dan büyük"
    return {"scale": (lambda x: f64(x) if x != 1.0 else None, False), "rotation": (lambda x: f64(x) if x != 0.0 else None, False),
            "absolute": (lambda b: boolean(b) if b else None, False), "upright": (lambda b: boolean(b) if b else None, False),
            "x": (lambda x: f64(x) if x != 0.0 else None, False), "y": (lambda x: f64(x) if x != 0.0 else None, False)}


def pattern_element(el):
    """A pattern's element (docs/adr/0313 §1): a length; a text or a shape takes none (0)."""
    assert ("text" not in el and "shape" not in el) or el["length"] == 0.0, "yazı ya da şeklin uzunluğu 0"
    def text_el(t):
        return cmap(fields(t, {"text": (text, True), "style": (text, True), **pattern_place(t)}, "patternText"))
    def shape_el(s):
        return cmap(fields(s, {"name": (text, True), "file": (text, True), **pattern_place(s)}, "patternShape"))
    return cmap(fields(el, {"length": (f64, True), "text": (text_el, False), "shape": (shape_el, False)}, "patternElement"))


def line_types(types):
    """Schema 56's line types (docs/adr/0313 §1): not empty; names neither empty nor twice (case aside); at most 12
    elements, the first that draws not a gap; a unit, `paper` or `drawing`."""
    assert types, "boş liste yazılmaz"
    seen = set()
    out = []
    for t in types:
        name = t["name"]
        assert name and name.strip() == name and name.lower() not in seen, "çizgi tipinin adı boş ya da iki kez"
        seen.add(name.lower())
        assert len(t["elements"]) <= 12, "en çok 12 öğe"
        drawn = [e["length"] for e in t["elements"] if "text" not in e and "shape" not in e]
        assert not drawn or drawn[0] >= 0, "desen bir çizgi ya da noktayla başlar"
        out.append(cmap(fields(t, {"name": (text, True), "description": (lambda s: text(s) if s else None, False),
                                   "elements": (lambda es: array([pattern_element(e) for e in es]), True),
                                   "unit": (enum(("paper", "drawing")), True), "source": (text, False)}, "lineType")))
    return array(out)


def layer_style(s):
    return cmap(
        fields(
            s,
            {
                "color": (text, True),
                "lineType": (enum(("continuous", "dashed", "dashdot", "dotted")), True),
                "lineWeight": (f64, True),
                "fill": (text, False),
                "point": (lambda p: cmap(fields(p, {"symbol": (enum(("ring", "cross", "triangle")), True), "size": (f64, True)}, "point")), False),
                "label": (label_style, False),
                # Schema 36 (docs/adr/0212 §2): the layer's labelling.
                "labels": (layer_labels, False),
                "pickInterior": (lambda b: b"\xf5" if b else b"\xf4", False),
                "renderer": (not_null, False),
                # Schema 56 (docs/adr/0313 §1): a project line type by its name.
                "linePattern": (text, False),
            },
            "style",
        )
    )


def boolean(b):
    assert type(b) is bool
    return b"\xf5" if b else b"\xf4"


LAYER_SNAP_KINDS = ("endpoint", "midpoint", "center", "node", "intersection", "perpendicular", "tangent", "nearest", "centroid", "extension", "parallel", "grid")


def layer_snap(s):
    """A layer's own snapping (schema 10, docs/adr/0163 §4): exactly one of `off` (true only) and a non-empty list of
    known, distinct kinds."""
    assert set(s) <= {"off", "kinds"} and (s.get("off") is True) != ("kinds" in s) and s.get("off", True) is True, f"katman keneti: {s}"
    if "kinds" in s:
        k = s["kinds"]
        assert k and len(set(k)) == len(k) and all(x in LAYER_SNAP_KINDS for x in k), f"katman keneti: {k}"
        return cmap({"kinds": array([text(x) for x in k])})
    return cmap({"off": boolean(True)})


FIELD_KINDS = ("text", "integer", "decimal", "date", "boolean")


def layer_field(f):
    """A layer's field (schema 26, docs/adr/0199 §1): its name and kind, then what it has; `required` only when true."""
    assert f.get("required", True) is True, f"alan {f.get('name')}: required yalnız true yazılır"
    choice = lambda c: cmap(fields(c, {"code": (text, True), "label": (text, True)}, "değer"))
    table = {
        "name": (text, True),
        "alias": (text, False),
        "kind": (enum(FIELD_KINDS), True),
        "length": (uint, False),
        "scale": (uint, False),
        "min": (text, False),
        "max": (text, False),
        "values": (lambda vs: array([choice(c) for c in vs]), False),
        "required": (boolean, False),
        "default": (text, False),
    }
    return cmap(fields(f, table, f"alan {f.get('name')}"))


# Schema 32 (docs/adr/0208 §2): a layer's map service, where its objects came from, and the project's connections.
SERVICE_KINDS = ("xyz", "wms", "wmts", "ogcTiles", "arcgis", "google", "vector")
FEED_KINDS = ("wfs", "ogcFeatures", "arcgis", "geojson")
AUTH_KINDS = ("none", "query", "header", "basic", "bearer", "arcgis", "oauth2", "google")


def only_true(b):
    """A flag of schema 32: written only when true."""
    assert b is True, "bayrak yalnız true iken yazılır"
    return b"\xf5"


def nonempty(encode):
    """A list of schema 32: written only when it is not empty."""

    def enc(list_):
        assert list_, "boş liste yazılmaz"
        return array([encode(x) for x in list_])

    return enc


def tile_matrix(m):
    return cmap(fields(m, {"id": (text, True), "x0": (f64, True), "y0": (f64, True), "resolution": (f64, True),
                           "tileWidth": (uint, True), "tileHeight": (uint, True), "matrixWidth": (uint, True),
                           "matrixHeight": (uint, True)}, f"matris {m.get('id')}"))


def tile_grid(g):
    return cmap(fields(g, {"srid": (uint, True), "matrices": (nonempty(tile_matrix), True)}, "karo ızgarası"))


def service(s):
    """A layer's map service (schema 32, docs/adr/0208 §2): its kind and address, then what it has; a Google layer has
    no address of its own (its tiles' address comes from its session) and its key is a connection's."""
    assert (s["url"] == "") == (s["kind"] == "google"), f"servis {s['kind']}: adres"
    assert s["kind"] != "google" or "connection" in s, "Google katmanının bağlantısı olmalı"
    param = lambda p: cmap(fields(p, {"name": (text, True), "value": (text, True)}, "parametre"))
    table = {
        "kind": (enum(SERVICE_KINDS), True),
        "url": (text, True),
        "layers": (nonempty(text), False),
        "style": (text, False),
        "format": (text, False),
        "srid": (uint, False),
        "grid": (tile_grid, False),
        "matrixSet": (text, False),
        "template": (text, False),
        "tileSize": (uint, False),
        "minZoom": (uint, False),
        "maxZoom": (uint, False),
        "subdomains": (nonempty(text), False),
        "yFlip": (only_true, False),
        "transparent": (only_true, False),
        "version": (text, False),
        "params": (nonempty(param), False),
        "dynamic": (only_true, False),
        "attribution": (text, False),
        "opacity": (f64, False),
        "connection": (text, False),
        "preset": (text, False),
        # The service's extent in WGS 84 degrees: west, south, east, north.
        "bbox": (floats, False),
    }
    if "bbox" in s:
        w, so, e, n = s["bbox"]
        assert -180 <= w <= e <= 180 and -90 <= so <= n <= 90, "servisin kapsamı WGS 84 derecesinde"
    return cmap(fields(s, table, f"servis {s['kind']}"))


def feed(f):
    """Where a layer's objects came from (schema 32, docs/adr/0208 §10): its kind and address, then what it has."""
    assert "bbox" not in f or len(f["bbox"]) == 4, "istenen alan dört sayı"
    table = {
        "kind": (enum(FEED_KINDS), True),
        "url": (text, True),
        "name": (text, False),
        "srid": (uint, False),
        "filter": (text, False),
        "bbox": (floats, False),
        "limit": (uint, False),
        "version": (text, False),
        "key": (text, False),
        "connection": (text, False),
        "fetched": (text, False),
    }
    return cmap(fields(f, table, f"kaynak {f['kind']}"))


def connection(c):
    """A project's connection (schema 32, docs/adr/0208 §2): where its secret is used and how, never the secret."""
    assert ("names" in c) == (c["auth"] in ("query", "header")), f"bağlantı {c['id']}: adlar"
    table = {
        "id": (text, True),
        "name": (text, True),
        "origin": (text, True),
        "auth": (enum(AUTH_KINDS), True),
        "names": (nonempty(text), False),
        "tokenUrl": (text, False),
        "scope": (text, False),
    }
    return cmap(fields(c, table, f"bağlantı {c['id']}"))


def connections(list_):
    ids = [c["id"] for c in list_]
    assert len(set(ids)) == len(ids), "bağlantının kimliği bir kez"
    return nonempty(connection)(list_)


# Schema 33 (docs/adr/0209 §2): the project's networks; a list only when it is not empty, the direction's field and
# values only with a field direction.
NETWORK_KINDS = ("road", "utility")
NETWORK_CONNECTS = ("ends", "vertices")
JUNCTION_ROLES = ("junction", "source", "valve")
NETWORK_COST_KINDS = ("speed", "field")
DIRECTION_KINDS = ("both", "digitized", "field")


def network_layer(e):
    return cmap(fields(e, {"layer": (text, True), "filter": (text, False)}, "kenar katmanı"))


def junction_layer(j):
    table = {"layer": (text, True), "role": (enum(JUNCTION_ROLES), True), "filter": (text, False), "closed": (text, False)}
    return cmap(fields(j, table, "düğüm katmanı"))


def network_direction(d):
    if d["kind"] != "field":
        assert set(d) == {"kind"}, "yönün alanı ve değerleri yalnız alanla yönde"
    lists = {k: v for k, v in d.items() if k in ("forward", "backward", "closed") and v}
    rest = {k: v for k, v in d.items() if k not in ("forward", "backward", "closed")}
    table = {"kind": (enum(DIRECTION_KINDS), True), "field": (text, d["kind"] == "field")}
    out = fields(rest, table, "yön")
    for k, v in lists.items():
        out[k] = nonempty(text)(v)
    return cmap(out)


def network_cost(c):
    assert c.get("unit", "x") != "", "boş birim yazılmaz"
    table = {"name": (text, True), "kind": (enum(NETWORK_COST_KINDS), True), "field": (text, True), "unit": (text, False), "speed": (finite, False)}
    return cmap(fields(c, table, f"maliyet {c.get('name')}"))


def network(n):
    table = {
        "id": (text, True),
        "name": (text, True),
        "kind": (enum(NETWORK_KINDS), True),
        "edges": (nonempty(network_layer), True),
        "junctions": (nonempty(junction_layer), False),
        "connect": (enum(NETWORK_CONNECTS), True),
        "tolerance": (finite, True),
        "direction": (network_direction, True),
        "costs": (nonempty(network_cost), False),
        "closed": (text, False),
    }
    return cmap(fields(n, table, f"ağ {n.get('id')}"))


def networks(list_):
    ids = [n["id"] for n in list_]
    assert len(set(ids)) == len(ids), "ağın kimliği bir kez"
    return nonempty(network)(list_)


# Schema 42 (docs/adr/0229 §2): the project's address data; an optional field only when it has a value, the areas only
# when there are some, a field's name never empty.
AREA_LEVELS = ("neighbourhood", "district", "province")


def field_name(t):
    assert t, "boş alan adı yazılmaz"
    return text(t)


def door_source(d):
    optional = ("neighbourhood", "district", "province", "postcode", "building", "code")
    table = {"layer": (text, True), "street": (field_name, True), "door": (field_name, True), **{k: (field_name, False) for k in optional}}
    return cmap(fields(d, table, "kapı noktaları"))


def street_source(s_):
    table = {"layer": (text, True), "name": (field_name, True), "neighbourhood": (field_name, False), "district": (field_name, False)}
    return cmap(fields(s_, table, "yollar"))


def area_source(a):
    table = {"layer": (text, True), "level": (enum(AREA_LEVELS), True), "name": (field_name, True), "district": (field_name, False)}
    return cmap(fields(a, table, "bölge"))


def address_locator(l_):
    table = {
        "id": (text, True),
        "name": (text, True),
        "doors": (door_source, False),
        "streets": (street_source, False),
        "areas": (nonempty(area_source), False),
        "minScore": (uint, True),
        "reach": (finite, True),
    }
    return cmap(fields(l_, table, f"adres verisi {l_.get('id')}"))


def address_locators(list_):
    ids = [l_["id"] for l_ in list_]
    assert len(set(ids)) == len(ids), "adres verisinin kimliği bir kez"
    return nonempty(address_locator)(list_)


# Schema 49 (docs/adr/0261 §1): a UCS is an origin within 10 000 km and an angle in [0, 2π); the current one is never
# the world's (that is no setting); names trimmed, 1 to 64 characters without control characters, unique, at most 100.
UCS_REACH = 1.0e7
TAU = 2.0 * math.pi


def ucs_frame(u):
    o = u["origin"]
    assert all(math.isfinite(v) and abs(v) <= UCS_REACH for v in (o["x"], o["y"])), u
    assert math.isfinite(u["angle"]) and 0.0 <= u["angle"] < TAU, u
    return {"origin": (point, True), "angle": (f64, True)}


def ucs(u):
    assert not (u["origin"]["x"] == 0 and u["origin"]["y"] == 0 and u["angle"] == 0), "dünya UCS'i yazılmaz"
    return cmap(fields(u, ucs_frame(u), "ucs"))


def named_ucs(list_):
    names = [n["name"] for n in list_]
    assert len(names) <= 100 and len(set(names)) == len(names), names
    for n in names:
        assert n == n.strip() and 0 < len(n) <= 64 and not any(unicodedata.category(c) == "Cc" for c in n), n

    def one(n):
        return cmap(fields(n, {"name": (text, True), **ucs_frame(n)}, "namedUcs"))

    return nonempty(one)(list_)


# Schema 52 (docs/adr/0285 §1): a named view is a middle within 10 000 km, a width and a height above 0 and at most
# 20 000 km, a twist in [0, 2π); names trimmed, 1 to 64 characters without control characters, unique, at most 100.
VIEW_SIZE = 2.0e7


def named_views(list_):
    names = [v["name"] for v in list_]
    assert len(names) <= 100 and len(set(names)) == len(names), names
    for v in list_:
        n = v["name"]
        assert n == n.strip() and 0 < len(n) <= 64 and not any(unicodedata.category(c) == "Cc" for c in n), n
        c = v["center"]
        assert all(math.isfinite(x) and abs(x) <= UCS_REACH for x in (c["x"], c["y"])), v
        assert all(math.isfinite(v[k]) and 0.0 < v[k] <= VIEW_SIZE for k in ("width", "height")), v
        assert math.isfinite(v["twist"]) and 0.0 <= v["twist"] < TAU, v

    def one(v):
        return cmap(fields(v, {"name": (text, True), "center": (point, True), "width": (f64, True), "height": (f64, True), "twist": (f64, True)}, "namedViews"))

    return nonempty(one)(list_)


# Schema 37 (docs/adr/0214 §2.3): the project's `@` variables; a label only when not empty, a value only when given.
VARIABLE_KINDS = ("text", "number", "bool", "date")


def variable_value(v):
    if v is None:
        return b"\xf6"
    if type(v) is bool:
        return boolean(v)
    if type(v) is float:
        return f64(v)
    return text(v)


def variable(v):
    assert v.get("label", "x") != "", "boş etiket yazılmaz"
    table = {"name": (text, True), "label": (text, False), "kind": (enum(VARIABLE_KINDS), True), "value": (variable_value, False)}
    return cmap(fields(v, table, f"değişken {v.get('name')}"))


def variables(list_):
    return nonempty(variable)(list_)


def layer_time(t):
    """A layer's time setting (schema 34, docs/adr/0210 §2): the start's field, the end's and the key's when given,
    Birikimli only when true."""
    assert t.get("cumulative", True) is True, "cumulative yalnız true yazılır"
    return cmap(fields(t, {"start": (text, True), "end": (text, False), "key": (text, False), "cumulative": (only_true, False)}, "time"))


def scenario(s):
    """A group's scenario (schema 34, docs/adr/0210 §2): its note when given."""
    return cmap(fields(s, {"note": (text, False)}, "scenario"))


def layer_filter(f):
    """A layer's filter (schema 35, docs/adr/0211 §2): its objects' ids as 16-byte strings and its condition, each when
    given; at least one, an empty list not written."""
    assert "expression" in f or f.get("objects"), "süzgeçte ifade ya da liste olmalı"
    assert f.get("objects", [None]), "boş liste yazılmaz"
    return cmap(fields(f, {"objects": (lambda xs: array([blob(uid_bytes(x)) for x in xs]), False), "expression": (text, False)}, "filter"))


def layer(n):
    assert "snap" not in n or n["type"] == "layer", f"layer {n.get('id')}: grubun keneti olmaz"
    assert "fields" not in n or (n["type"] == "layer" and n["fields"]), f"layer {n.get('id')}: alanlar yalnız katmanın, boş değil"
    assert not ({"service", "feed"} & set(n)) or n["type"] == "layer", f"layer {n.get('id')}: servis ve kaynak yalnız katmanın"
    assert not {"service", "feed"} <= set(n), f"layer {n.get('id')}: servis ve kaynak birlikte olmaz"
    assert "filter" not in n or (n["type"] == "layer" and "service" not in n), f"layer {n.get('id')}: süzgeç yalnız nesnesi olan katmanın"
    assert not ({"actions", "card"} & set(n)) or (n["type"] == "layer" and "service" not in n), f"layer {n.get('id')}: eylemler ve kart yalnız nesnesi olan katmanın"
    assert "compose" not in n or n["type"] == "layer", f"layer {n.get('id')}: katman çizimi yalnız katmanın"
    return cmap(layer_entries(n))


def layer_entries(n):
    """A layer's or group's encoded fields by key."""
    return (
        fields(
            n,
            {
                "id": (text, True),
                "name": (text, True),
                "snap": (layer_snap, False),
                # Schema 26 (docs/adr/0199 §1): the schema of its objects' attributes.
                "fields": (lambda fs: array([layer_field(f) for f in fs]), False),
                # Schema 32 (docs/adr/0208 §2, §10): drawn from a map service, or its objects from a source.
                "service": (service, False),
                "feed": (feed, False),
                # Schema 34 (docs/adr/0210 §2): a layer's time setting, a group's scenario, a scenario layer's base layer.
                "time": (layer_time, False),
                "scenario": (scenario, False),
                "replaces": (text, False),
                # Schema 35 (docs/adr/0211 §2): a layer's filter.
                "filter": (layer_filter, False),
                # Schema 40 (docs/adr/0225 §4, §5): a layer's actions and information card.
                "actions": (layer_actions, False),
                "card": (info_card, False),
                # Schema 41 (docs/adr/0226 §3, §4): a layer's opacity, blending mode and mask.
                "compose": (layer_compose, False),
                # Schema 48 (docs/adr/0260 §4): a layer or group not printed; only `false` is written.
                "plot": (lambda b: boolean(b) if b is False else None, False),
                # Schema 55 (docs/adr/0311 §1): a layer or group frozen; only `true` is written.
                "frozen": (lambda b: boolean(b) if b is True else None, False),
                "type": (enum(("group", "layer")), True),
                "visible": (boolean, True),
                "locked": (boolean, True),
                "expanded": (boolean, True),
                "style": (layer_style, True),
                "children": (lambda c: array([layer(x) for x in c]), True),
            },
            f"layer {n.get('id')}",
        )
    )


def elevations(list_):
    """A list of elevations (schema 4, docs/adr/0142): a float, or `null` for a vertex without one."""
    return array([b"\xf6" if z is None else f64(z) for z in list_])


def ring(r):
    assert "zs" not in r or len(r["zs"]) == len(r["pts"]), "delik: kot sayısı köşe sayısına eşit olmalı"
    return cmap(fields(r, {"pts": (points, True), "bulges": (floats, False), "zs": (elevations, False)}, "ring"))


def part(pt):
    """A part of a multi-part area past its first (schema 5, docs/adr/0143): its ring, arcs, holes and elevations."""
    assert "zs" not in pt or len(pt["zs"]) == len(pt["pts"]), "parça: kot sayısı köşe sayısına eşit olmalı"
    table = {"pts": (points, True), "bulges": (floats, False), "holes": (lambda h: array([ring(r) for r in h]), False), "zs": (elevations, False)}
    return cmap(fields(pt, table, "part"))


def line_part(pt):
    """A part of a multi-part polyline past its first (schema 17, docs/adr/0174): two vertices or more, its arcs and
    elevations; no holes."""
    assert len(pt["pts"]) >= 2, "çoklu çizginin parçası: en az iki köşe"
    assert "zs" not in pt or len(pt["zs"]) == len(pt["pts"]), "parça: kot sayısı köşe sayısına eşit olmalı"
    assert "ms" not in pt or len(pt["ms"]) == len(pt["pts"]), "parça: ölçü sayısı köşe sayısına eşit olmalı"
    # Schema 39 (docs/adr/0224): its measures, as its elevations.
    return cmap(fields(pt, {"pts": (points, True), "bulges": (floats, False), "zs": (elevations, False), "ms": (elevations, False)}, "line part"))


def point_part(pp):
    """A point of a multi-point object past its first (schema 17, docs/adr/0174): its place and elevation."""
    return cmap(fields(pp, {"p": (point, True), "z": (f64, False)}, "point part"))


def text_run(r):
    """A multi-line text's run (schema 20, docs/adr/0182): its range, a flag only when true, its script and colour."""
    flag = (lambda b: boolean(b) if b is True else None, False)
    table = {"start": (uint, True), "end": (uint, True), "bold": flag, "italic": flag, "underline": flag, "script": (enum(("super", "sub")), False), "color": (text, False)}
    return cmap(fields(r, table, "run"))


def path_fields(holes):
    table = {"pts": (points, True), "bulges": (floats, False), "zs": (elevations, False)}
    if holes:
        table["holes"] = (lambda h: array([ring(r) for r in h]), False)
        # Schema 5 (docs/adr/0143): an area's parts.
        table["parts"] = (lambda ps: array([part(pt) for pt in ps]), False)
    else:
        # Schema 17 (docs/adr/0174): a polyline's parts.
        table["parts"] = (lambda ps: array([line_part(pt) for pt in ps]), False)
        # Schema 39 (docs/adr/0224): a polyline's measures, a route's; an area has none.
        table["ms"] = (elevations, False)
        # Schema 46 (docs/adr/0258): a closed line, written only when closed; each vertex's edge's widths.
        table["closed"] = (lambda b: boolean(b) if b is True else None, False)
        table["widths"] = (lambda ws: array([floats(w) for w in ws]), False)
    return table


def polyline_extras(e):
    """A closed and a wide polyline's rules (schema 46, docs/adr/0258), as a writer keeps them."""
    closed, widths = e.get("closed", False), e.get("widths")
    assert closed is not False or "closed" not in e, "kapalılık false yazılmaz"
    if closed or widths is not None:
        assert "parts" not in e, "kapalı ya da genişlikli çoklu çizgi tek parçalıdır"
    assert not closed or len(e["pts"]) >= 2, "kapalı çoklu çizginin en az iki köşesi olmalı"
    if widths is not None:
        assert len(widths) == len(e["pts"]), "her köşeye bir genişlik çifti"
        assert all(len(w) == 2 and w[0] >= 0 and w[1] >= 0 for w in widths), "genişlik sıfır ya da büyük"
        assert closed or widths[-1] == [0.0, 0.0], "açık çizginin son genişliği [0, 0]"
        assert any(a != 0 or b != 0 for a, b in widths), "bütün genişlikler sıfır yazılmaz"


def nurbs_rule(e):
    """A NURBS curve's numbers make a curve (schema 47, docs/adr/0259 §1), as a writer keeps them."""
    p, ctrl, knots = e["degree"], e["ctrl"], e["knots"]
    n = len(ctrl)
    assert type(p) is int and 1 <= p <= 11, "derece 1 ile 11 arasında"
    assert p + 1 <= n <= 100_000, "en az derece + 1 denetim noktası"
    assert len(knots) == n + p + 1, "düğüm sayısı denetim noktası + derece + 1"
    assert all(math.isfinite(k) for k in knots) and all(a <= b for a, b in zip(knots, knots[1:])), "düğümler sonlu, azalmaz"
    assert knots[n] > knots[p], "eğrinin aralığı boş değil"
    inner = knots[1:-1]
    run = 1
    for a, b in zip(inner, inner[1:]):
        run = run + 1 if a == b else 1
        assert run <= p, "iç düğüm en çok derece kez"
    if "weights" in e:
        assert len(e["weights"]) == n and all(math.isfinite(w) and w > 0 for w in e["weights"]), "ağırlıklar pozitif"
    assert "tangents" not in e or len(e["tangents"]) == 2, "iki teğet"
    assert e.get("fitTolerance", 0.0) >= 0.0, "geçiş toleransı sıfır ya da büyük"
    assert e.get("closed", True) is True and e.get("periodic", True) is True, "closed ve periodic yalnız true yazılır"


# A table's columns' alignments and lines (schema 22, docs/adr/0184): all lines is the field's absence, no value.
TABLE_ALIGNS = ("left", "center", "right")
TABLE_GRIDS = ("outer", "rows", "none")


def table_source(src):
    """Where a table's rows came from (schema 22): a schedule's kind and its objects' persistent ids, or a file's name
    and sheet."""
    if src["kind"] == "file":
        return cmap(fields(src, {"kind": (text, True), "name": (text, True), "sheet": (text, False)}, "kaynak"))
    assert src["objects"], "kaynağın nesnesi olmalı"
    return cmap(fields(src, {"kind": (enum(("coordinates", "areas", "attributes")), True), "objects": (lambda os: array([blob(uid_bytes(u)) for u in os]), True)}, "kaynak"))


def table_shape(e):
    """The contract's table rule, as far as an example needs it held (docs/adr/0184 §1)."""
    n, m = len(e["rows"]), len(e["columns"])
    assert 0 < n and 0 < m and len(e["cells"]) == n and all(len(r) == m for r in e["cells"]), "hücreler satır ve sütun sayısı kadar"
    assert all(not any(ord(c) < 32 or 0x7F <= ord(c) <= 0x9F for c in w) for r in e["cells"] for w in r), "hücre tek satır"
    assert e.get("merges", [None]), "boş birleşik alan listesi yazılmaz"
    assert len(e.get("aligns", e["columns"])) == m, "her sütunun hizası"
    if "frame" in e:
        assert 0 < 2 * e["frame"] < min(sum(e["columns"]), sum(e["rows"])), "çerçeve tablonun içinde"


def image_shape(e):
    """The contract's picture rule, as far as an example needs it held (docs/adr/0192 §1)."""
    assert 0 < e["width"] <= 1e7 and 0 < e["height"] <= 1e7, "resmin boyu sıfırdan büyük"
    assert ("asset" in e) != ("file" in e), "resmin tek kaynağı olmalı"
    assert all(0 <= q["x"] <= 1 and 0 <= q["y"] <= 1 for q in e.get("clip", [])) and len(e.get("clip", [None] * 3)) >= 3, "kırpma resmin içinde"
    assert 0.1 <= e.get("opacity", 1.0) <= 1.0, "donukluk 0,1 ile 1 arasında"


# A raster's samples, looks, stretches and resamplings (schema 29, docs/adr/0204): no stretch and the bilinear are the
# fields' absence, no value.
RASTER_SAMPLES = ("u8", "i8", "u16", "i16", "u32", "i32", "f32", "f64")
RASTER_RENDERS = ("rgb", "gray", "palette", "ramp", "hillshade", "rampShade")
RASTER_STRETCHES = ("minMax", "percent", "manual")
RASTER_RAMPS = ("Gri", "Arazi", "Spektral", "Viridis", "Mavi-kırmızı", "Sıcaklık")


def raster_style(st):
    """A raster's look (schema 29, docs/adr/0204 §4): `render` and `bands`, the rest only when not their default."""
    table = {
        "render": (enum(RASTER_RENDERS), True),
        "bands": (lambda bs: array([uint(b) for b in bs]), True),
        "stretch": (enum(RASTER_STRETCHES), False),
        "min": (f64, False),
        "max": (f64, False),
        "ramp": (enum(RASTER_RAMPS), False),
        "invert": (lambda b: boolean(b) if b is True else None, False),
        "azimuth": (f64, False),
        "altitude": (f64, False),
        "zFactor": (f64, False),
        "nodata": (f64, False),
        "resampling": (enum(("nearest",)), False),
        # Schema 38 (docs/adr/0243 §5): a mesh's lines.
        "edges": (text, False),
    }
    out = fields(st, table, "rasterin görünüşü")
    assert out.get("invert", True) is not None, "invert yalnız true yazılır"
    return cmap(out)


def raster_dataset(d):
    """A raster's NetCDF variable and slice (schema 38, docs/adr/0243 §6): `variable`; `vector`, `mesh`, `dims` and
    `followTime` only when given (no empty list, no false)."""
    def dim(x):
        table = {
            "name": (text, True),
            "index": (uint, True),
            "values": (floats, True),
            "time": (lambda b: boolean(b) if b is True else None, False),
            "units": (text, False),
        }
        return cmap(fields(x, table, "veri setinin boyutu"))

    assert d.get("dims", [None]), "boş dilim boyutu listesi yazılmaz"
    table = {
        "variable": (text, True),
        "vector": (text, False),
        "mesh": (text, False),
        "dims": (lambda ds: array([dim(x) for x in ds]), False),
        "followTime": (lambda b: boolean(b) if b is True else None, False),
    }
    return cmap(fields(d, table, "rasterin veri seti"))


def raster_shape(e):
    """The contract's raster rule, as far as an example needs it held (docs/adr/0204 §2)."""
    _, a, b, _, c, d = e["affine"]
    assert a * d - b * c != 0.0, "rasterin dönüşümü tersinmeli"
    assert 1 <= e["width"] <= 4_000_000 and 1 <= e["height"] <= 4_000_000 and 1 <= e["bands"] <= 255, "rasterin boyu ve bantları"
    assert sum(k in e for k in ("asset", "file", "url")) == 1, "rasterin tek kaynağı olmalı"
    st = e["style"]
    assert len(st["bands"]) in ((3, 4) if st["render"] == "rgb" else (1,)) and all(1 <= x <= e["bands"] for x in st["bands"]), "görünüşün bantları"
    assert st.get("stretch") != "manual" or st["min"] < st["max"], "elle gerdirmenin aralığı"
    assert 0.1 <= e.get("opacity", 1.0) <= 1.0, "donukluk 0,1 ile 1 arasında"


KINDS = {
    # Schema 17 (docs/adr/0174): a multi-point object's points past its first.
    "point": {"p": (point, True), "z": (f64, False), "parts": (lambda ps: array([point_part(pp) for pp in ps]), False)},
    # A line's end without an elevation has no key (spec §5.2): `null` is only for a vertex in `zs`.
    # Schema 39 (docs/adr/0224): its ends' measures, as its elevations.
    "line": {"a": (point, True), "b": (point, True), "za": (f64, False), "zb": (f64, False), "ma": (f64, False), "mb": (f64, False)},
    "polyline": path_fields(False),
    "polygon": path_fields(True),
    "circle": {"c": (point, True), "r": (f64, True)},
    "arc": {"c": (point, True), "r": (f64, True), "a0": (f64, True), "a1": (f64, True)},
    "ellipse": {"c": (point, True), "major": (point, True), "ratio": (f64, True), "t0": (f64, True), "t1": (f64, True)},
    "spline": {"pts": (points, True), "closed": (boolean, True)},
    # Schema 47 (docs/adr/0259): a NURBS curve, its numbers as given; `closed` and `periodic` only when true; the fit
    # points' end tangents a pair, `null` where none was given.
    "nurbs": {
        "degree": (uint, True),
        "ctrl": (points, True),
        "knots": (floats, True),
        "weights": (floats, False),
        "closed": (lambda b: boolean(b) if b is True else None, False),
        "periodic": (lambda b: boolean(b) if b is True else None, False),
        "fit": (points, False),
        "tangents": (lambda t: array([b"\xf6" if x is None else point(x) for x in t]), False),
        "fitTolerance": (f64, False),
    },
    "xline": {"p": (point, True), "dir": (point, True)},
    "ray": {"p": (point, True), "dir": (point, True)},
    # Schema 7 (docs/adr/0145): an alignment by name, a width factor, `mask` only when true.
    "text": {
        "p": (point, True),
        "text": (text, True),
        "height": (f64, True),
        "rotation": (f64, True),
        "align": (enum(TEXT_ALIGNS), False),
        "widthFactor": (f64, False),
        "mask": (lambda b: boolean(b) if b is True else None, False),
        # Schema 18 (docs/adr/0175 §4): the object whose label it writes, its persistent id, and the scale's denominator.
        "labelOf": (lambda u: blob(uid_bytes(u)), False),
        "labelScale": (f64, False),
        # Schema 20 (docs/adr/0182 §1): a multi-line text's box, line spacing and letter formats.
        "boxWidth": (f64, False),
        "lineSpacing": (f64, False),
        "runs": (lambda rs: array([text_run(r) for r in rs]), False),
        # Schema 21 (docs/adr/0183 §2): its style, typeface, bold and italic only when true, slant.
        "textStyle": (text, False),
        "font": (enum(DRAWING_FONTS), False),
        "bold": (lambda b: boolean(b) if b is True else None, False),
        "italic": (lambda b: boolean(b) if b is True else None, False),
        "oblique": (f64, False),
        # Schema 25 (docs/adr/0196 §1): the curve its letters stand on, in its frame; bulges only when an edge bends.
        "path": (lambda c: cmap(fields(c, {"pts": (points, True), "bulges": (lambda b: array([f64(x) for x in b]), False)}, "eğri")), False),
    },
    # Schema 9 (docs/adr/0147): five more kinds by name, `mask` only when true, a slope's two elevations.
    "dimension": {
        "a": (point, True),
        "b": (point, True),
        "offset": (f64, True),
        "height": (f64, True),
        "text": (text, False),
        "style": (enum(DIMENSION_STYLES), False),
        "angle": (f64, False),
        "c": (point, False),
        "mask": (lambda b: boolean(b) if b is True else None, False),
        "za": (f64, False),
        "zb": (f64, False),
        # Schema 21 (docs/adr/0183 §3): its style, arrowheads by name, sizes (times its height), the value's place,
        # decimals, unit, prefix, suffix and typeface.
        "dimStyle": (text, False),
        "arrow": (enum(DIMENSION_ARROWS), False),
        "arrowSize": (f64, False),
        "extOffset": (f64, False),
        "extBeyond": (f64, False),
        "textGap": (f64, False),
        "textPlace": (enum(("centre",)), False),
        "decimals": (uint, False),
        "unit": (enum(("mm", "cm", "m")), False),
        "prefix": (text, False),
        "suffix": (text, False),
        "font": (enum(DRAWING_FONTS), False),
        # Schema 30 (docs/adr/0205 §6): its lines.
        **LINE_FIELDS,
        # Schema 43 (docs/adr/0249 §4): what its points follow; only in the drawing.
        "refs": (lambda r: dimension_refs(r), False),
    },
    # Schema 23 (docs/adr/0186): a pattern's name, scale and families, a gradient; the objects it follows, only in the drawing.
    "hatch": {
        "ring": (points, True),
        "holes": (lambda h: array([points(r) for r in h]), False),
        "pattern": (lambda p: hatch_pattern(p), True),
        "assoc": (lambda a: hatch_assoc(a), False),
    },
    # Schema 6 (docs/adr/0144): the definition's id; `mirror` only when true.
    "insert": {"block": (lambda u: blob(uid_bytes(u)), True), "p": (point, True), "scale": (f64, True), "rotation": (f64, True), "mirror": (lambda b: boolean(b) if b is True else None, False),
               # Schema 57 (docs/adr/0315 §3): its own y factor, only when it is not the x one.
               "yScale": (f64, False)},
    # Schema 22 (docs/adr/0184): one-line cells row by row, merged ranges, alignments, a heading row only when true, lines
    # by name, a frame's width, a text's face, a source; only in the drawing.
    "table": {
        "p": (point, True),
        "rotation": (f64, True),
        "height": (f64, True),
        "rows": (floats, True),
        "columns": (floats, True),
        "cells": (lambda rs: array([array([text(w) for w in r]) for r in rs]), True),
        "merges": (lambda ms: array([cmap({k: uint(g[k]) for k in ("row", "col", "rows", "cols")}) for g in ms]), False),
        "aligns": (lambda a: array([enum(TABLE_ALIGNS)(x) for x in a]), False),
        "header": (lambda b: boolean(b) if b is True else None, False),
        "grid": (enum(TABLE_GRIDS), False),
        "frame": (f64, False),
        "textStyle": (text, False),
        "font": (enum(DRAWING_FONTS), False),
        "bold": (lambda b: boolean(b) if b is True else None, False),
        "italic": (lambda b: boolean(b) if b is True else None, False),
        "oblique": (f64, False),
        "source": (table_source, False),
    },
    # Schema 24 (docs/adr/0192): a picture's frame, one source, its clip in its own fractions, its opacity; `mirror`
    # only when true; only in the drawing.
    "image": {
        "p": (point, True),
        "width": (f64, True),
        "height": (f64, True),
        "rotation": (f64, True),
        "mirror": (lambda b: boolean(b) if b is True else None, False),
        "asset": (text, False),
        "file": (text, False),
        "clip": (points, False),
        "opacity": (f64, False),
    },
    # Schema 29 (docs/adr/0204 §2): a raster's affine, size, bands and samples, one source, its file's system, its look
    # and opacity; only in the drawing.
    "raster": {
        "affine": (floats, True),
        "width": (uint, True),
        "height": (uint, True),
        "bands": (uint, True),
        "sample": (enum(RASTER_SAMPLES), True),
        "asset": (text, False),
        "file": (text, False),
        # Schema 31 (docs/adr/0207 §1): an address read by ranges.
        "url": (text, False),
        "srid": (uint, True),
        "style": (raster_style, True),
        "opacity": (f64, False),
        # Schema 38 (docs/adr/0243 §6): a NetCDF variable and its slice.
        "dataset": (raster_dataset, False),
    },
    # Schema 31 (docs/adr/0207 §3): a point cloud's files, their bounds and points together, the files' system, its look
    # and opacity; only in the drawing.
    "pointcloud": {
        "sources": (lambda fs: array([cloud_source(f) for f in fs]), True),
        "bounds": (floats, True),
        "count": (uint, True),
        "srid": (uint, True),
        "style": (cloud_style, True),
        "opacity": (f64, False),
    },
    # Schema 8 (docs/adr/0146): two vertices or more, a note only when there is one, an arrowhead by name, `mask` only when true.
    # Schema 30 (docs/adr/0205 §7): AutoCAD's arrowheads, the arrowhead's size times the note's height.
    "leader": {
        "pts": (points, True),
        "text": (text, False),
        "height": (f64, True),
        "rotation": (f64, True),
        "arrow": (enum(LEADER_ARROWS + LEADER_ARROWS_30), False),
        "mask": (lambda b: boolean(b) if b is True else None, False),
        "arrowSize": (leader_arrow_size, False),
    },
}


def pattern_line(l):
    """A pattern's family (schema 23, docs/adr/0186 §1): its angle, origin, offset and, only when it has any, its dashes."""
    assert l.get("dashes", [None]), "boş kesik listesi yazılmaz"
    pair = lambda xy: point({"x": xy[0], "y": xy[1]})
    return cmap(fields(l, {"angle": (f64, True), "origin": (pair, True), "offset": (pair, True), "dashes": (floats, False)}, "desen ailesi"))


def hatch_gradient(g):
    """A gradient (schema 23): its shape by name, its second colour, `inverted` only when true."""
    out = fields(g, {"shape": (enum(GRADIENT_SHAPES), True), "color2": (text, True), "inverted": (lambda b: boolean(b) if b is True else None, False)}, "degrade")
    assert out.get("inverted", True) is not None, "inverted yalnız true yazılır"
    return cmap(out)


def hatch_pattern(p):
    """A hatch's pattern: its kind by name, angle and spacing; schema 23's name, scale, families and gradient."""
    table = {
        "type": (enum(HATCH_TYPES), True),
        "angle": (f64, True),
        "spacing": (f64, True),
        "name": (text, False),
        "scale": (f64, False),
        "lines": (lambda ls: array([pattern_line(l) for l in ls]), False),
        "gradient": (hatch_gradient, False),
    }
    return cmap(fields(p, table, "pattern"))


def hatch_assoc(a):
    """What a hatch's region follows (schema 23, docs/adr/0186 §6): objects by persistent id, empty lists not written; the seed."""
    ids = lambda us: array([blob(uid_bytes(u)) for u in us])
    assert a.get("islands", [None]) and a.get("cutouts", [None]), "boş liste yazılmaz"
    return cmap(fields(a, {"outer": (lambda u: blob(uid_bytes(u)), True), "islands": (ids, False), "cutouts": (ids, False), "seed": (point, True)}, "assoc"))


# A reference's places and the numbers each has (docs/adr/0249 §1); the codes of a broken one.
SUBREF_PLACES = {"node": ("part",), "start": (), "end": (), "center": (), "vertex": ("part", "ring", "index"), "mid": ("part", "ring", "index"), "along": ("part", "ring", "index")}
SUBREF_BROKEN = ("object_gone", "kind_changed", "no_such_place", "vertex_gone", "ambiguous", "piece_gone")


def subref(r):
    """A reference (schema 43, docs/adr/0249 §4): the object's persistent id, the place (its kind, then the numbers it
    has, as unsigned integers; an `along` place's `t` last, a float in [0, 1], schema 51, docs/adr/0263 §1), its block
    path when it names a piece inside an insert (unsigned integers, 1 to 16, schema 51, §3), why it broke only when it
    did."""
    place = r["place"]
    numbers = SUBREF_PLACES[place["kind"]]
    extra = ("t",) if place["kind"] == "along" else ()
    assert set(place) == {"kind", *numbers, *extra}, place
    assert all(0 <= place[k] <= 10_000_000 for k in numbers), place
    assert not extra or 0.0 <= place["t"] <= 1.0, place
    assert r.get("broken", SUBREF_BROKEN[0]) in SUBREF_BROKEN, r
    assert "path" not in r or 1 <= len(r["path"]) <= 16, r
    encoded_place = cmap({"kind": text(place["kind"]), **{k: uint(place[k]) for k in numbers}, **{k: f64(place[k]) for k in extra}})
    path = lambda xs: array([uint(x) for x in xs])  # noqa: E731
    return cmap(fields(r, {"uid": (lambda u: blob(uid_bytes(u)), True), "place": (lambda _: encoded_place, True), "path": (path, False), "broken": (text, False)}, "başvuru"))


def dimension_refs(refs):
    """What a dimension's points follow (schema 43, docs/adr/0249 §4): `a`, `b`, `c`; none, no field."""
    assert refs, "boş başvurular yazılmaz"
    return cmap(fields(refs, {"a": (subref, False), "b": (subref, False), "c": (subref, False)}, "refs"))


LOOK_PARTS = ("color", "lineType", "lineWeight", "transparency")


def look_rule(e, in_block):
    """An object's look (schema 48, docs/adr/0260), as a writer keeps it."""
    assert "lineTypeScale" not in e or 0 < e["lineTypeScale"] <= 1000, "çizgi tipi ölçeği 0'dan büyük, en çok 1000"
    assert "transparency" not in e or 0 <= e["transparency"] <= 0.9, "saydamlık 0 ile 0,9 arası"
    assert "order" not in e or (not in_block and type(e["order"]) is int and -(2**31) <= e["order"] < 2**31), "çizim sırası çizimin nesnesinin, tam sayı"
    parts = e.get("byLayer", [])
    assert not parts or in_block, "katmana göre parçalar yalnız bloğun nesnesinde"
    assert "byLayer" not in e or parts, "boş liste yazılmaz"
    assert all(p in LOOK_PARTS for p in parts) and [LOOK_PARTS.index(p) for p in parts] == sorted(set(LOOK_PARTS.index(p) for p in parts)), "sırayla ve birer kez"
    assert all(p not in e for p in parts), "kendi değeri olan parça katmana göre olamaz"
    # Schema 54 (docs/adr/0310): an area's mask; only an area has it.
    assert "wipeout" not in e or (e["kind"] == "polygon" and set(e["wipeout"]) == {"frame"} and type(e["wipeout"]["frame"]) is bool), "maske yalnız alanda, { frame }"


def entity(e, uid, index):
    """An object; `uid` None for a block definition's object, which has no persistent id (schema 6)."""
    kind = e["kind"]
    assert e["id"] == index + 1, f"nesne {index}: kimlik {e['id']}, dosya sırası {index + 1} vermeli"
    assert "zs" not in e or len(e["zs"]) == len(e["pts"]), f"nesne {index} ({kind}): kot sayısı köşe sayısına eşit olmalı"
    assert "ms" not in e or len(e["ms"]) == len(e["pts"]), f"nesne {index} ({kind}): ölçü sayısı köşe sayısına eşit olmalı"
    table = {
        "layerId": (text, True),
        "color": (text, False),
        "attrs": (lambda a: cmap({k: text(v) for k, v in a.items()}), True),
        "label": (text, False),
        "symbol": (text, False),
        # Schema 3 (docs/adr/0139): an object's own line weight, mm.
        "lineWeight": (f64, False),
        # Schema 36 (docs/adr/0212 §2): its labels pinned by hand; the drawing's objects only.
        "labelPins": (label_pins, False),
        # Schema 40 (docs/adr/0225 §1): its photos and documents; the drawing's objects only.
        "attachments": (attachments, False),
        # Schema 48 (docs/adr/0260): its line type, pattern scale, transparency, drawing order (the drawing's
        # objects) and the parts that follow its layer (a block's objects).
        "lineType": (enum(LINE_TYPES), False),
        "lineTypeScale": (f64, False),
        "transparency": (f64, False),
        "order": (integer, False),
        "byLayer": (lambda ps: array([text(p) for p in ps]), False),
        # Schema 54 (docs/adr/0310): an area's mask.
        "wipeout": (lambda w: cmap({"frame": boolean(w["frame"])}), False),
        # Schema 56 (docs/adr/0313 §1): its own project line type by its name.
        "linePattern": (text, False),
        **KINDS[kind],
    }
    look_rule(e, uid is None)
    assert "labelPins" not in e or uid is not None, "blok tanımının nesnesinde etiket iğnesi olmaz"
    assert "attachments" not in e or uid is not None, "blok tanımının nesnesinde ek olmaz"
    if kind == "table":
        assert uid is not None, "blok tanımında tablo olamaz"
        table_shape(e)
    if kind == "image":
        assert uid is not None, "blok tanımında resim olamaz"
        image_shape(e)
    if kind == "raster":
        assert uid is not None, "blok tanımında raster olamaz"
        raster_shape(e)
    if kind == "pointcloud":
        assert uid is not None, "blok tanımında nokta bulutu olamaz"
        cloud_shape(e)
    if kind == "polyline":
        polyline_extras(e)
    if kind == "nurbs":
        nurbs_rule(e)
    # A block definition's objects have no persistent ids to name (docs/adr/0186 §6).
    assert "assoc" not in e or uid is not None, "blok tanımının taraması nesnelere bağlı olamaz"
    assert "refs" not in e or uid is not None, "blok tanımının ölçüsü nesnelere bağlı olamaz"
    body = fields({k: v for k, v in e.items() if k not in ("kind", "id")}, table, f"nesne {index} ({kind})")
    assert body.get("header", True) is not None, f"nesne {index}: header yalnız true yazılır"
    assert body.get("mirror", True) is not None, f"nesne {index}: mirror yalnız true yazılır"
    assert kind != "insert" or e.get("yScale", None) != e["scale"], f"nesne {index}: x'e eşit y ölçeği yazılmaz"
    assert body.get("mask", True) is not None, f"nesne {index}: mask yalnız true yazılır"
    assert body.get("closed", True) is not None, f"nesne {index}: closed yalnız true yazılır"
    if uid is not None:
        body["uid"] = blob(uid_bytes(uid))
    return cmap({kind: cmap(body)})


def attribute(a):
    table = {"tag": (text, True), "prompt": (text, False), "value": (text, False), "p": (point, True), "height": (f64, True), "rotation": (f64, True)}
    # Schema 7 (docs/adr/0145): an alignment and a width factor, as a text's.
    table.update({"align": (enum(TEXT_ALIGNS), False), "widthFactor": (f64, False)})
    return cmap(fields(a, table, "öznitelik"))


def definition(b):
    """A block definition (schema 6, docs/adr/0144): its objects written as the drawing's, without persistent ids."""
    assert b.get("attributes", [None]), "boş öznitelik listesi yazılmaz"
    table = {
        "id": (lambda u: blob(uid_bytes(u)), True),
        "name": (text, True),
        "base": (point, True),
        "entities": (lambda es: array([entity(e, None, i) for i, e in enumerate(es)]), True),
        "attributes": (lambda xs: array([attribute(a) for a in xs]), False),
        "description": (text, False),
        # Schema 57 (docs/adr/0315 §4): the unit it is drawn in.
        "units": (enum(("mm", "cm", "m", "in", "ft", "unitless")), False),
    }
    return cmap(fields(b, table, f"blok {b.get('name')}"))


def sheets(s):
    """The project's sheet book (schema 44, docs/adr/0251): the book an opaque map, as the sheet core keeps it; its
    pictures' bytes (base64 in the JSON) under the SHA-256 the book names them by, none given twice; no pictures, no
    field."""
    assert type(s["book"]) is dict, "pafta kitabı bir harita olmalı"
    assert s.get("pictures", [None]), "boş resim listesi yazılmaz"
    seen = set()

    def picture(p):
        raw = base64.b64decode(p["bytes"], validate=True)
        assert hashlib.sha256(raw).hexdigest() == p["sha256"], "resmin baytları SHA-256'sına uymuyor"
        assert p["sha256"] not in seen, "aynı resim iki kez verilmiş"
        seen.add(p["sha256"])
        return cmap({"bytes": blob(raw), "sha256": blob(bytes.fromhex(p["sha256"]))})

    return cmap(fields(s, {"book": (opaque, True), "pictures": (lambda ps: array([picture(p) for p in ps]), False)}, "sheets"))


def assoc_arrays(arrs):
    """The associative arrays (schema 53, docs/adr/0300 §1): each its id, source block, first place, layout (its kind's own
    fields; `fit` only when true) and items (an insert's id when there is one, `kept` only when true); not empty."""
    assert arrs, "boş dizi listesi yazılmaz"

    def layout(lay):
        kind = lay["kind"]
        out = {"kind": text(kind)}
        if kind == "grid":
            out.update(rows=uint(lay["rows"]), cols=uint(lay["cols"]), dx=f64(lay["dx"]), dy=f64(lay["dy"]))
        elif kind == "polar":
            out.update(center=point(lay["center"]), count=uint(lay["count"]), fill=f64(lay["fill"]), rotate=boolean(lay["rotate"]))
        elif kind == "line":
            out.update(count=uint(lay["count"]), dx=f64(lay["dx"]), dy=f64(lay["dy"]))
            if lay.get("fit"):
                out["fit"] = boolean(True)
        else:
            out.update(path=blob(uid_bytes(lay["path"])), count=uint(lay["count"]), align=boolean(lay["align"]))
            if "spacing" in lay:
                out["spacing"] = f64(lay["spacing"])
        return cmap(out)

    def item(it):
        out = {}
        if "uid" in it:
            out["uid"] = blob(uid_bytes(it["uid"]))
        if it.get("kept"):
            out["kept"] = boolean(True)
        return cmap(out)

    def one(a):
        out = {
            "p": point(a["p"]),
            "id": blob(uid_bytes(a["id"])),
            "block": blob(uid_bytes(a["block"])),
            "items": array([item(it) for it in a["items"]]),
            "scale": f64(a["scale"]),
            "layout": layout(a["layout"]),
            "layerId": text(a["layerId"]),
            "rotation": f64(a["rotation"]),
        }
        if a.get("mirror"):
            out["mirror"] = boolean(True)
        return cmap(out)

    return array([one(a) for a in arrs])


def requests(rs):
    """The request ledger (schema 45, docs/adr/0253 §6): the requests a drawing executed, in their order; each its id,
    its fingerprint (32 hex digits, 16 bytes in the file), the command, its version, the revision after it (decimal
    text), its status and the answer then; none twice; no requests, no field."""
    assert rs, "boş istek listesi yazılmaz"
    assert len(rs) <= 256, "en çok 256 istek yazılır"
    seen = set()

    def request(r):
        rid = r["requestId"]
        assert 1 <= len(rid) <= 128 and all(c.isascii() and (c.isalnum() or c in "._:-") for c in rid), rid
        assert rid not in seen, "aynı istek iki kez yazılmış"
        seen.add(rid)
        assert len(r["fingerprint"]) == 32 and all(c in "0123456789abcdef" for c in r["fingerprint"])
        assert r["command"] and r["version"] >= 1 and r["revision"].isdigit()
        assert r["status"] in ("done", "undone")
        assert len(r.get("answer", "").encode("utf-8")) <= 64 * 1024
        return cmap(
            fields(
                r,
                {
                    "requestId": (text, True),
                    "fingerprint": (lambda h: blob(bytes.fromhex(h)), True),
                    "command": (text, True),
                    "version": (uint, True),
                    "revision": (text, True),
                    "status": (text, True),
                    "answer": (text, False),
                },
                "istek",
            )
        )

    return array([request(r) for r in rs])


def document(d):
    """The payload of a drawing given in the contract's JSON form (`DocumentSnapshotV2`)."""
    assert d["format"] == "kentos.document" and d["version"] == 2
    assert len(d["uids"]) == len(d["entities"])
    body = fields(
        {k: v for k, v in d.items() if k not in ("format", "version", "uids")},
        {
            "name": (text, True),
            "settings": (settings, True),
            "origin": (point, True),
            "homeView": (lambda b: array([f64(b[k]) for k in ("minX", "minY", "maxX", "maxY")]), False),
            "layers": (lambda ls: array([layer(n) for n in ls]), True),
            "activeLayer": (text, True),
            "entities": (lambda es: array([entity(e, u, i) for i, (e, u) in enumerate(zip(es, d["uids"]))]), True),
            "blocks": (lambda bs: array([definition(b) for b in bs]), False),
            "styles": (lambda s: cmap(fields(s, {"items": (lambda xs: array([opaque(x) for x in xs]), True), "categories": (lambda xs: array([opaque(x) for x in xs]), True)}, "styles")), True),
            "projectId": (lambda u: blob(uid_bytes(u)), False),
            "sheets": (sheets, False),
            "arrays": (assoc_arrays, False),
            "requests": (requests, False),
            "migratedFrom": (
                lambda m: cmap(fields(m, {"format": (text, True), "version": (uint, True), "sourceSha256": (lambda h: blob(bytes.fromhex(h)), True)}, "migratedFrom")),
                False,
            ),
        },
        "belge",
    )
    assert d.get("blocks", [None]), "boş blok listesi yazılmaz"
    return root(cmap(body), version=uint(schema_of(d["entities"], d.get("blocks"), d["layers"], d["settings"], "sheets" in d, "requests" in d, "arrays" in d)))


def schema_of(entities, blocks=None, layers=(), settings=None, has_sheets=False, has_requests=False, has_arrays=False):
    """The oldest schema that holds the drawing: 53 with associative arrays (docs/adr/0300), 44 with the project's sheet book (docs/adr/0251), 43 with a dimension's references (docs/adr/0249 §4), 42 with the project's address data (docs/adr/0229 §2), 41 with a layer's composition (docs/adr/0226 §3, §4), 40 with an object's
    attachments or a layer's actions or card (docs/adr/0225), 39 with vertex measures (docs/adr/0224), 38 with a raster's
    dataset or its look's edges (docs/adr/0243 §6), 37 with the settings' variables (docs/adr/0214 §2.3), 36 with the
    label engine's fields, a layer's labelling or an object's
    labels pinned by hand (docs/adr/0212 §2), 35 with a layer's filter (docs/adr/0211 §2), 34 with a layer's time
    setting, a scenario or a scenario layer's base
    layer (docs/adr/0210 §2), 33 with the project's networks (docs/adr/0209 §2), 32 with a layer drawn
    from a map service or one whose objects came from
    a source, or the project's connections (docs/adr/0208 §2), 31 with a point cloud or a raster read from an address, only in the
    drawing (docs/adr/0207), 30 with the project's annotation heights, a dimension style's or a
    dimension's lines, a leader's arrowhead's size or one of AutoCAD's arrowheads, in the drawing or a block definition
    (docs/adr/0205), 29 with a raster, only in the drawing (docs/adr/0204 §2), 28 with the
    survey settings' a priori standard deviations (docs/adr/0203 §1), 27 with the project's topology settings (docs/adr/0202 §7), 26 with a
    layer's fields (docs/adr/0199 §1), 25 with a text along a
    curve, in the drawing or a block definition
    (docs/adr/0196), 24 with a picture, only in the drawing (docs/adr/0192), 23 with a hatch's pattern of families or gradient (a field of theirs) or
    its tie, in the drawing or a block definition (docs/adr/0186), 22 with a table, only in the drawing (docs/adr/0184), 21 with a text or a dimension style or a text's face or a dimension's look,
    in the drawing or a block definition (docs/adr/0183), 20 with a multi-line text's box, line spacing or letter formats, in the
    drawing or a block definition (docs/adr/0182 §1), 19 with layer states (docs/adr/0177 §4), 18 with a text that writes an
    object's label (docs/adr/0175 §4), 17
    with a multi-part polyline or a multi-point object, in the drawing or
    a block definition (docs/adr/0174), 16 with the survey settings' ground height or reduction to the grid
    (docs/adr/0171), 15 with their traverse tolerances, 14 with survey settings (docs/adr/0169 §3), 13 with the project's
    own systems or datum choices (docs/adr/0168), 12 with a second coordinate system (docs/adr/0167 §1), 11 with a local
    project's drawing unit (docs/adr/0165 §2), 10 with a layer's own snapping (docs/adr/0163 §4), 9 with one of
    schema 9's dimension kinds, a dimension's mask or a
    slope's elevations, in the drawing or a block definition (docs/adr/0147), 8 with a leader, in the drawing or a
    block definition (docs/adr/0146), 7 with a text's or an attribute definition's alignment, width factor or mask
    (docs/adr/0145), 6 with block definitions (docs/adr/0144), 5 with an area's parts (docs/adr/0143), 4 with a
    vertex elevation (docs/adr/0142), 3 with an object's own line weight (docs/adr/0139), else 2: a drawing without
    any stays as it was, byte for byte."""
    def snaps(nodes):
        return any("snap" in n or snaps(n["children"]) for n in nodes)

    def line_parts(es):
        return any(e["kind"] in ("polyline", "point") and "parts" in e for e in es)

    def paragraphs(es):
        return any(e["kind"] == "text" and any(k in e for k in ("boxWidth", "lineSpacing", "runs")) for e in es)

    face = ("textStyle", "font", "bold", "italic", "oblique")
    look = ("dimStyle", "arrow", "arrowSize", "extOffset", "extBeyond", "textGap", "textPlace", "decimals", "unit", "prefix", "suffix", "font")

    def styled(es):
        return any(
            (e["kind"] == "text" and any(k in e for k in face)) or (e["kind"] == "dimension" and any(k in e for k in look)) for e in es
        )

    def patterned(es):
        defined = ("name", "scale", "lines", "gradient")
        return any(
            e["kind"] == "hatch" and ("assoc" in e or e["pattern"]["type"] in ("pattern", "gradient") or any(k in e["pattern"] for k in defined))
            for e in es
        )

    def schemas(nodes):
        return any("fields" in n or schemas(n["children"]) for n in nodes)

    def annotated(es):
        return any(
            (e["kind"] == "dimension" and any(k in e for k in DIMENSION_LINES))
            or (e["kind"] == "leader" and ("arrowSize" in e or e.get("arrow") in LEADER_ARROWS_30))
            for e in es
        )

    def served(nodes):
        return any("service" in n or "feed" in n or served(n["children"]) for n in nodes)

    def temporal(nodes):
        return any("time" in n or "scenario" in n or "replaces" in n or temporal(n["children"]) for n in nodes)

    def filtered(nodes):
        return any("filter" in n or filtered(n["children"]) for n in nodes)

    def engine(style):
        return "labels" in style or any(k in LABEL_ENGINE for k in style.get("label", {}))

    def labelled(nodes):
        return any(engine(n["style"]) or labelled(n["children"]) for n in nodes)

    def measured(es):
        return any(any(k in e for k in ("ma", "mb", "ms")) or any("ms" in pt for pt in e.get("parts", []) if e["kind"] == "polyline") for e in es)

    def informed(nodes):
        return any("actions" in n or "card" in n or informed(n["children"]) for n in nodes)

    def composed(nodes):
        return any("compose" in n or composed(n["children"]) for n in nodes)

    # Schema 56 (docs/adr/0313): a line type with a pattern, a line type scale, a pattern named by a layer, a state's
    # style or an object.
    def line_patterned(nodes):
        return any("linePattern" in n.get("style", {}) or line_patterned(n.get("children", [])) for n in nodes)

    # Schema 57 (docs/adr/0315 §3, §4): an insert's own y factor, of the drawing or of a definition, or a block's unit.
    stretched = lambda es: any(e["kind"] == "insert" and "yScale" in e for e in es)  # noqa: E731
    if stretched(entities) or any("units" in b or stretched(b["entities"]) for b in blocks or []):
        return 57
    named_pattern = lambda es: any("linePattern" in e for e in es)  # noqa: E731
    s_ = settings or {}
    if ("lineTypes" in s_ or "lineTypeScale" in s_ or line_patterned(layers)
            or any("linePattern" in n.get("style", {}) for st in s_.get("layerStates", []) for n in st["nodes"])
            or named_pattern(entities) or any(named_pattern(b["entities"]) for b in blocks or [])):
        return 56
    # Schema 55 (docs/adr/0311): a frozen node, or a layer state keeping freezes and printing.
    def frozen(nodes):
        return any(n.get("frozen") is True or frozen(n["children"]) for n in nodes)

    if frozen(layers) or any("frozen" in n or "plot" in n for st in (settings or {}).get("layerStates", []) for n in st["nodes"]):
        return 55
    # Schema 54 (docs/adr/0310): an area's mask, a block definition's too.
    masked = lambda es: any("wipeout" in e for e in es)  # noqa: E731
    if masked(entities) or any(masked(b["entities"]) for b in blocks or []):
        return 54
    # Schema 53 (docs/adr/0300): an associative array.
    if has_arrays:
        return 53
    # Schema 52 (docs/adr/0285): a named view.
    if settings and "namedViews" in settings:
        return 52
    # Schema 51 (docs/adr/0263): a dimension's reference along an edge or inside a block.
    def extended(r):
        return r["place"]["kind"] == "along" or "path" in r

    if any(e["kind"] == "dimension" and any(extended(r) for r in e.get("refs", {}).values()) for e in entities):
        return 51
    # Schema 50 (docs/adr/0262): a drawing in inches or feet, or a length style.
    if settings and (settings.get("drawingUnit") in ("in", "ft") or "lengthStyle" in settings):
        return 50
    # Schema 49 (docs/adr/0261): the current UCS or a named one.
    if settings and ("ucs" in settings or "namedUcs" in settings):
        return 49
    # Schema 48 (docs/adr/0260): an object's look, a block definition's too, or a layer not printed.
    looked = lambda es: any(k in e for e in es for k in ("lineType", "lineTypeScale", "transparency", "order", "byLayer"))  # noqa: E731

    def unprinted(nodes):
        return any(n.get("plot") is False or unprinted(n["children"]) for n in nodes)

    if looked(entities) or any(looked(b["entities"]) for b in blocks or []) or unprinted(layers):
        return 48
    # Schema 47 (docs/adr/0259): a NURBS curve, a block definition's too.
    def curved(es):
        return any(e["kind"] == "nurbs" for e in es)

    if curved(entities) or any(curved(b["entities"]) for b in blocks or []):
        return 47
    # Schema 46 (docs/adr/0258): a closed or a wide polyline, a block definition's too.
    def extras(es):
        return any(e["kind"] == "polyline" and ("closed" in e or "widths" in e) for e in es)

    if extras(entities) or any(extras(b["entities"]) for b in blocks or []):
        return 46
    if has_requests:
        return 45
    if has_sheets:
        return 44
    if any(e["kind"] == "dimension" and "refs" in e for e in entities):
        return 43
    if settings and "addressLocators" in settings:
        return 42
    if composed(layers):
        return 41
    if any("attachments" in e for e in entities) or informed(layers):
        return 40
    if measured(entities) or any(measured(b["entities"]) for b in blocks or []):
        return 39
    if any(e["kind"] == "raster" and ("dataset" in e or "edges" in e["style"]) for e in entities):
        return 38
    if settings and "variables" in settings:
        return 37
    if (labelled(layers) or any("labelPins" in e for e in entities)
            or any("style" in n and engine(n["style"]) for st in (settings or {}).get("layerStates", []) for n in st["nodes"])):
        return 36
    if filtered(layers):
        return 35
    if temporal(layers):
        return 34
    if settings and "networks" in settings:
        return 33
    if served(layers) or (settings and "connections" in settings):
        return 32
    if any(e["kind"] == "pointcloud" or (e["kind"] == "raster" and "url" in e) for e in entities):
        return 31
    if settings and ("annotation" in settings or any(k in st for st in settings.get("dimensionStyles", []) for k in DIMENSION_LINES)):
        return 30
    if annotated(entities) or any(annotated(b["entities"]) for b in blocks or []):
        return 30
    if any(e["kind"] == "raster" for e in entities):
        return 29
    if settings and any(k in settings.get("survey", {}) for k in SIGMA_KEYS):
        return 28
    if settings and "topology" in settings:
        return 27
    if schemas(layers):
        return 26
    if any(e["kind"] == "text" and "path" in e for e in [*entities, *(x for b in blocks or [] for x in b["entities"])]):
        return 25
    if any(e["kind"] == "image" for e in entities):
        return 24
    if patterned(entities) or any(patterned(b["entities"]) for b in blocks or []):
        return 23
    if any(e["kind"] == "table" for e in entities):
        return 22
    if settings and (settings.get("textStyles") or settings.get("dimensionStyles")):
        return 21
    if styled(entities) or any(styled(b["entities"]) for b in blocks or []):
        return 21
    if paragraphs(entities) or any(paragraphs(b["entities"]) for b in blocks or []):
        return 20
    if settings and settings.get("layerStates"):
        return 19
    # A block definition's texts have no link (their objects have no persistent ids).
    if any(e["kind"] == "text" and ("labelOf" in e or "labelScale" in e) for e in entities):
        return 18
    if line_parts(entities) or any(line_parts(b["entities"]) for b in blocks or []):
        return 17
    if settings and any(k in settings.get("survey", {}) for k in ("groundHeight", "reduceToGrid")):
        return 16
    if settings and any(k in settings.get("survey", {}) for k in ("twoWay", "traverseAngle", "traverseCoord")):
        return 15
    if settings and "survey" in settings:
        return 14
    if settings and any(k in settings for k in ("customCrs", "secondCustomCrs", "datumTransforms")):
        return 13
    if settings and "secondSrid" in settings:
        return 12
    if settings and "drawingUnit" in settings:
        return 11
    if snaps(layers):
        return 10

    def new_dimensions(es):
        return any(
            e["kind"] == "dimension" and (e.get("style") in SCHEMA_9_STYLES or any(k in e for k in ("mask", "za", "zb")))
            for e in es
        )

    if new_dimensions(entities) or any(new_dimensions(b["entities"]) for b in blocks or []):
        return 9
    if any(e["kind"] == "leader" for e in entities) or any(e["kind"] == "leader" for b in blocks or [] for e in b["entities"]):
        return 8
    extras = ("align", "widthFactor", "mask")

    def texts(es):
        return any(e["kind"] == "text" and any(k in e for k in extras) for e in es)

    if texts(entities) or any(texts(b["entities"]) or any(k in a for a in b.get("attributes", []) for k in extras) for b in blocks or []):
        return 7
    if blocks:
        return 6

    def elevated(e):
        holes = e.get("holes", []) if e["kind"] == "polygon" else []
        return any(k in e for k in ("za", "zb", "zs")) or any("zs" in h for h in holes)

    if any(e["kind"] == "polygon" and "parts" in e for e in entities):
        return 5
    if any(elevated(e) for e in entities):
        return 4
    return 3 if any("lineWeight" in e for e in entities) else 2


def root(document_bytes, version=b"\x02"):
    """The payload's top map: format, version, document (already in canonical order)."""
    return head(5, 3) + text("format") + text("kentos.document") + text("version") + version + text("document") + document_bytes


# ── Container (spec §3) ─────────────────────────────────────────────────


def container(payload, *, major=2, minor=0, min_reader=0, encoding=1, codec=0, flags=0, extensions=(), payload_length=None, decoded_length=None):
    ext = b"".join(bytes([len(n)]) + n.encode("ascii") for n in extensions)
    header = (
        MAGIC
        + bytes([major, minor, min_reader])
        + (36 + len(ext)).to_bytes(2, "little")
        + bytes([encoding, codec])
        + (len(payload) if payload_length is None else payload_length).to_bytes(8, "little")
        + (len(payload) if decoded_length is None else decoded_length).to_bytes(8, "little")
        + flags.to_bytes(2, "little")
        + len(extensions).to_bytes(2, "little")
        + ext
    )
    body = header + payload
    return body + hashlib.sha256(body).digest()


# ── The drawings ────────────────────────────────────────────────────────


def load(name):
    return json.loads((DIR / name).read_text("utf-8"))


def migrated():
    """fixtures/document/v1/sample.json opened (derived ids, ADR 0014) and saved as v2: its content in the contract's JSON form."""
    v1 = json.loads((ROOT / "fixtures/document/v1/sample.json").read_text("utf-8"))
    ids = json.loads((ROOT / "fixtures/document/v1/identity/expected.json").read_text("utf-8"))["cases"][0]
    assert ids["name"] == "sample"
    assert [e["id"] for e in v1["entities"]] == [e["id"] for e in ids["entities"]] == list(range(1, len(v1["entities"]) + 1))

    def typed(v, fields_):
        # The v1 JSON writes whole floats as integers ("plotScale": 1000); the contract reads them as float64.
        return {k: (float(x) if k in fields_ else x) for k, x in v.items()}

    def layer_(n):
        style = typed(n["style"], {"lineWeight"})
        if "point" in style:
            style["point"] = typed(style["point"], {"size"})
        if "label" in style:
            style["label"] = typed(style["label"], {"size", "grow", "maxSize", "minFeaturePx", "minScale", "maxScale"})
        return {**n, "style": style, "children": [layer_(c) for c in n["children"]]}

    floats_ = {"z", "r", "a0", "a1", "ratio", "t0", "t1", "height", "rotation", "offset", "angle"}

    def entity_(e):
        out = {}
        for k, v in e.items():
            if k in ("p", "a", "b", "c", "major", "dir"):
                v = {"x": float(v["x"]), "y": float(v["y"])}
            elif k in ("pts", "ring"):
                v = [{"x": float(p["x"]), "y": float(p["y"])} for p in v]
            elif k == "bulges":
                v = [float(x) for x in v]
            elif k == "holes" and e["kind"] == "polygon":
                v = [{**r, "pts": [{"x": float(p["x"]), "y": float(p["y"])} for p in r["pts"]], **({"bulges": [float(x) for x in r["bulges"]]} if "bulges" in r else {})} for r in v]
            elif k == "holes":
                v = [[{"x": float(p["x"]), "y": float(p["y"])} for p in r] for r in v]
            elif k == "pattern":
                v = typed(v, {"angle", "spacing"})
            elif k in floats_:
                v = float(v)
            out[k] = v
        return out

    settings = typed(v1["settings"], {"plotScale"})
    # The former Hibrit mode is a project type not asked yet: not written (docs/adr/0165 §1).
    if settings.get("workspace") == "hybrid":
        del settings["workspace"]
    return {
        "format": "kentos.document",
        "version": 2,
        "name": v1["name"],
        "settings": settings,
        "origin": {"x": float(v1["origin"]["x"]), "y": float(v1["origin"]["y"])},
        "homeView": {k: float(x) for k, x in v1["homeView"].items()},
        "layers": [layer_(n) for n in v1["layers"]],
        "activeLayer": v1["activeLayer"],
        "entities": [entity_(e) for e in v1["entities"]],
        "uids": [e["uid"] for e in ids["entities"]],
        "styles": v1["styles"],
        "projectId": ids["project"],
        "migratedFrom": {"format": "kentos.document", "version": 1, "sourceSha256": ids["sourceSha256"]},
    }


def json_text(v):
    return json.dumps(v, ensure_ascii=False, indent=2) + "\n"


# ── Broken files: each breaks exactly one rule ──────────────────────────


def minimal_parts(content):
    """The minimal drawing's document map as {key: encoded value}, to break one entry at a time."""
    d = content
    return {
        "name": text(d["name"]),
        "layers": array([layer(n) for n in d["layers"]]),
        "origin": point(d["origin"]),
        "styles": cmap({"items": array([]), "categories": array([])}),
        "entities": array([entity(e, u, i) for i, (e, u) in enumerate(zip(d["entities"], d["uids"]))]),
        "settings": settings(d["settings"]),
        "activeLayer": text(d["activeLayer"]),
    }


def with_parts(parts):
    return root(cmap(parts))


def broken(minimal_content, minimal_file):
    m = minimal_content
    files = {}
    good = minimal_file
    payload = document(m)

    # The container.
    files["bad-magic.kcad"] = b"\x88" + good[1:]
    files["line-endings.kcad"] = good[:5] + b"\n\x1a\n" + good[9:]
    files["empty.kcad"] = b""
    files["signature-only.kcad"] = good[:7]
    files["header-only.kcad"] = good[:20]
    files["truncated.kcad"] = good[:-10]
    files["trailing-data.kcad"] = good + b"\x00"
    flipped = bytearray(good)
    flipped[36 + 20] ^= 0x01
    files["bad-hash.kcad"] = bytes(flipped)
    files["major-3.kcad"] = container(payload, major=3)
    files["newer-minor.kcad"] = container(payload, minor=1, min_reader=1)
    files["flags.kcad"] = container(payload, flags=1)
    files["unknown-encoding.kcad"] = container(payload, encoding=2)
    files["unknown-codec.kcad"] = container(payload, codec=1)
    files["decoded-length.kcad"] = container(payload, decoded_length=len(payload) + 1)
    files["unknown-extension.kcad"] = container(payload, extensions=("kentos.blocks",))
    files["too-large.kcad"] = container(payload, payload_length=(1 << 30) + 1)

    # The CBOR profile.
    parts = minimal_parts(m)
    files["duplicate-key.kcad"] = container(head(5, 3) + text("format") + text("kentos.document") + text("format") + text("kentos.document") + text("document") + cmap(parts))
    files["unsorted-keys.kcad"] = container(root(raw_map([(text("layers"), parts["layers"]), (text("name"), parts["name"])] + [(text(k), parts[k]) for k in ("origin", "styles", "entities", "settings", "activeLayer")])))
    deep = array([])
    for _ in range(70):
        deep = array([deep])
    files["too-deep.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([deep]), "categories": array([])})}))
    files["too-long.kcad"] = container(with_parts({**parts, "styles": cmap({"items": b"\x9a\x01\x00\x00\x01" + b"\xf6" * 8, "categories": array([])})}))
    files["too-long-text.kcad"] = container(with_parts({**parts, "name": b"\x7a\x01\x00\x00\x01" + b"a" * 8}))
    files["array-past-end.kcad"] = container(with_parts({**parts, "styles": cmap({"items": b"\x99\xff\xff", "categories": array([])})}))
    files["truncated-cbor.kcad"] = container(payload[:-3])
    files["trailing-cbor.kcad"] = container(payload + b"\x00")
    files["non-shortest-int.kcad"] = container(root(cmap(parts), version=b"\x18\x02"))
    files["non-shortest-length.kcad"] = container(with_parts({**parts, "name": b"\x78\x04" + "Boş".encode("utf-8")}))
    files["narrow-float.kcad"] = container(with_parts({**parts, "origin": array([b"\xfa" + struct.pack(">f", 500000.5), f64(4400000.0)])}))
    files["half-float.kcad"] = container(with_parts({**parts, "origin": array([b"\xf9\x3c\x00", f64(4400000.0)])}))
    files["nan.kcad"] = container(with_parts({**parts, "origin": array([b"\xfb\x7f\xf8\x00\x00\x00\x00\x00\x00", f64(4400000.0)])}))
    files["infinity.kcad"] = container(with_parts({**parts, "origin": array([f64(500000.0), b"\xfb\x7f\xf0\x00\x00\x00\x00\x00\x00"])}))
    files["indefinite.kcad"] = container(with_parts({**parts, "entities": b"\x9f\xff"}))
    tagged = cmap({"uid": b"\xd8\x25" + blob(uid_bytes(m["uids"][0])), "attrs": cmap({}), "layerId": text("0"), "p": point(m["entities"][0]["p"])})
    files["tag.kcad"] = container(with_parts({**parts, "entities": array([cmap({"point": tagged})])}))
    files["undefined.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([b"\xf7"]), "categories": array([])})}))
    files["reserved-info.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([b"\x1c"]), "categories": array([])})}))
    files["invalid-utf8.kcad"] = container(with_parts({**parts, "name": b"\x62\xc3\x28"}))
    files["non-text-key.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([head(5, 1) + b"\x01" + b"\xf6"]), "categories": array([])})}))

    # The document schema.
    files["wrong-format.kcad"] = container(head(5, 3) + text("format") + text("kentos.style") + text("version") + b"\x02" + text("document") + cmap(parts))
    files["schema-version-58.kcad"] = container(root(cmap(parts), version=uint(58)))
    files["unknown-field.kcad"] = container(with_parts({**parts, "extra": text("?")}))
    files["missing-field.kcad"] = container(with_parts({k: v for k, v in parts.items() if k != "activeLayer"}))
    files["int-for-float.kcad"] = container(with_parts({**parts, "settings": cmap({**{k: v for k, v in settings_parts(m["settings"]).items()}, "plotScale": uint(1000)})}))
    one = m["entities"][0]
    body = {"attrs": cmap({k: text(v) for k, v in one["attrs"].items()}), "layerId": text(one["layerId"]), "p": point(one["p"])}
    files["short-uid.kcad"] = container(with_parts({**parts, "entities": array([cmap({"point": cmap({**body, "uid": blob(bytes(range(1, 16)))})})])}))
    files["nil-uid.kcad"] = container(with_parts({**parts, "entities": array([cmap({"point": cmap({**body, "uid": blob(bytes(16))})})])}))
    same = cmap({"point": cmap({**body, "uid": blob(uid_bytes(m["uids"][0]))})})
    files["duplicate-uid.kcad"] = container(with_parts({**parts, "entities": array([same, same])}))
    # An object's own line weight is schema 3's: in schema 2 it is an unknown field; beyond 0…100 mm a bad value.
    weighed = cmap({"point": cmap({**body, "uid": blob(uid_bytes(m["uids"][0])), "lineWeight": f64(0.35)})})
    files["line-weight-in-schema-2.kcad"] = container(with_parts({**parts, "entities": array([weighed])}))
    heavy = cmap({"point": cmap({**body, "uid": blob(uid_bytes(m["uids"][0])), "lineWeight": f64(100.5)})})
    files["line-weight-too-heavy.kcad"] = container(root(cmap({**parts, "entities": array([heavy])}), version=b"\x03"))
    # Vertex elevations are schema 4's (docs/adr/0142): in schema 2 or 3 they are unknown fields. `zs` has one entry per
    # vertex, a finite float or `null`; a line's end without an elevation has no key, so `null` there is a wrong type.
    common = {"attrs": cmap({}), "layerId": text(one["layerId"]), "uid": blob(uid_bytes(m["uids"][0]))}

    def line(**extra):
        ends = {"a": point(one["p"]), "b": point({"x": one["p"]["x"] + 10.0, "y": one["p"]["y"]})}
        return cmap({"line": cmap({**common, **ends, **extra})})

    def path(kind, vertices, **extra):
        pts = array([point({"x": one["p"]["x"] + i, "y": one["p"]["y"]}) for i in range(vertices)])
        return cmap({kind: cmap({**common, "pts": pts, **extra})})

    def in_schema(n, entity_bytes):
        return container(root(cmap({**parts, "entities": array([entity_bytes])}), version=uint(n)))

    nan, infinity = b"\xfb\x7f\xf8" + bytes(5), b"\xfb\xff\xf0" + bytes(5)
    square_line = lambda dx: array([point({"x": one["p"]["x"] + dx + x, "y": one["p"]["y"]}) for x in (0, 1)])
    files["elevation-in-schema-2.kcad"] = in_schema(2, line(za=f64(105.25)))
    files["elevation-in-schema-3.kcad"] = in_schema(3, path("polyline", 3, zs=elevations([101.5, None, 103.25])))
    files["elevation-wrong-length.kcad"] = in_schema(4, path("polyline", 3, zs=elevations([101.5, 103.25])))
    hole = cmap({"pts": array([point({"x": one["p"]["x"] + i, "y": one["p"]["y"] + 1.0}) for i in range(3)]), "zs": elevations([1.0, 2.0, 3.0, 4.0])})
    files["hole-elevation-wrong-length.kcad"] = in_schema(4, path("polygon", 4, holes=array([hole])))
    files["elevation-nan.kcad"] = in_schema(4, path("polyline", 3, zs=array([f64(101.5), nan, f64(103.25)])))
    files["elevation-infinity.kcad"] = in_schema(4, line(zb=infinity))
    files["elevation-int.kcad"] = in_schema(4, path("polyline", 3, zs=array([f64(101.5), uint(102), f64(103.25)])))
    files["line-elevation-null.kcad"] = in_schema(4, line(za=b"\xf6"))
    # Vertex measures are schema 39's (docs/adr/0224): in schema 38 unknown fields; an area has none (an unknown
    # field there); `ms` has one entry per vertex, a line's end without one no key, as the elevations.
    files["measures-in-schema-38.kcad"] = in_schema(38, line(ma=f64(0.0), mb=f64(10.0)))
    files["measures-in-polygon.kcad"] = in_schema(39, path("polygon", 4, ms=elevations([0.0, 1.0, 2.0, 3.0])))
    files["measure-wrong-length.kcad"] = in_schema(39, path("polyline", 3, ms=elevations([0.0, 1.0])))
    files["line-measure-null.kcad"] = in_schema(39, line(mb=b"\xf6"))
    files["part-measure-wrong-length.kcad"] = in_schema(39, path("polyline", 3, parts=array([cmap({"pts": square_line(5), "ms": elevations([1.0])})])))
    # Attachments are schema 40's (docs/adr/0225 §1): in schema 39 an unknown field; an empty list is not written; each
    # has its name and exactly one source, an address is http or https; the contract's rules broken one at a time.
    attached = lambda xs, n=40: in_schema(n, cmap({"point": cmap({**common, "p": point(one["p"]), "attachments": array(xs)})}))
    tapu = cmap({"name": text("Tapu"), "file": text("belgeler/101-5.pdf")})
    files["attachments-in-schema-39.kcad"] = attached([tapu], 39)
    files["attachments-empty.kcad"] = attached([])
    files["attachment-two-sources.kcad"] = attached([cmap({"name": text("Tapu"), "file": text("a.pdf"), "url": text("https://a.org/a.pdf")})])
    files["attachment-no-source.kcad"] = attached([cmap({"name": text("Tapu")})])
    files["attachment-without-name.kcad"] = attached([cmap({"file": text("a.pdf")})])
    files["attachment-name-blank.kcad"] = attached([cmap({"name": text(" "), "file": text("a.pdf")})])
    files["attachment-ftp.kcad"] = attached([cmap({"name": text("Tapu"), "url": text("ftp://a.org/a.pdf")})])
    files["attachment-note-control.kcad"] = attached([cmap({"name": text("Tapu"), "file": text("a.pdf"), "note": text("a\tb")})])
    files["attachment-unknown-field.kcad"] = attached([cmap({"name": text("Tapu"), "file": text("a.pdf"), "size": uint(10)})])
    # An area's parts are schema 5's (docs/adr/0143): in schema 4 an unknown field, on a polyline too; a part's `zs`
    # has one entry per vertex, as the area's own.
    square = lambda dx: array([point({"x": one["p"]["x"] + dx + x, "y": one["p"]["y"] + y}) for x, y in ((0, 0), (1, 0), (1, 1), (0, 1))])
    files["parts-in-schema-4.kcad"] = in_schema(4, path("polygon", 4, parts=array([cmap({"pts": square(5)})])))
    files["parts-on-polyline.kcad"] = in_schema(5, path("polyline", 3, parts=array([cmap({"pts": square(5)})])))
    files["part-elevation-wrong-length.kcad"] = in_schema(5, path("polygon", 4, parts=array([cmap({"pts": square(5), "zs": elevations([1.0, 2.0])})])))
    files["part-without-points.kcad"] = in_schema(5, path("polygon", 4, parts=array([cmap({"bulges": array([f64(0.5)])})])))
    # A polyline's and a point's parts are schema 17's (docs/adr/0174): in schema 16 unknown fields. A polyline's part
    # has two vertices or more and no holes; a point's part its place.
    two = lambda dx: array([point({"x": one["p"]["x"] + dx + x, "y": one["p"]["y"]}) for x in (0, 1)])
    files["polyline-parts-in-schema-16.kcad"] = in_schema(16, path("polyline", 3, parts=array([cmap({"pts": two(5)})])))
    point_with = lambda **extra: cmap({"point": cmap({**common, "p": point(one["p"]), **extra})})
    files["point-parts-in-schema-16.kcad"] = in_schema(16, point_with(parts=array([cmap({"p": point({"x": one["p"]["x"] + 1.0, "y": one["p"]["y"]})})])))
    files["polyline-part-one-point.kcad"] = in_schema(17, path("polyline", 3, parts=array([cmap({"pts": array([point(one["p"])])})])))
    files["polyline-part-holes.kcad"] = in_schema(17, path("polyline", 3, parts=array([cmap({"pts": two(5), "holes": array([])})])))
    files["point-part-without-place.kcad"] = in_schema(17, point_with(parts=array([cmap({"z": f64(100.0)})])))
    # A text's link to the object whose label it writes is schema 18's (docs/adr/0175 §4): in schema 17 unknown
    # fields; both or neither; a scale finite and over 0; an id of 16 bytes, not nil; no link in a block definition.
    linked = lambda **extra: cmap({"text": cmap({**common, "p": point(one["p"]), "text": text("101"), "height": f64(2.5), "rotation": f64(0.0), **extra})})
    of = blob(uid_bytes("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001"))
    files["text-label-of-in-schema-17.kcad"] = in_schema(17, linked(labelOf=of, labelScale=f64(1000.0)))
    files["text-label-of-without-scale.kcad"] = in_schema(18, linked(labelOf=of))
    files["text-label-scale-without-of.kcad"] = in_schema(18, linked(labelScale=f64(1000.0)))
    files["text-label-scale-zero.kcad"] = in_schema(18, linked(labelOf=of, labelScale=f64(0.0)))
    files["text-label-scale-nan.kcad"] = in_schema(18, linked(labelOf=of, labelScale=b"\xfb\x7f\xf8\x00\x00\x00\x00\x00\x00"))
    files["text-label-of-short.kcad"] = in_schema(18, linked(labelOf=blob(bytes(8)), labelScale=f64(1000.0)))
    files["text-label-of-nil.kcad"] = in_schema(18, linked(labelOf=blob(bytes(16)), labelScale=f64(1000.0)))
    # Blocks are schema 6's (docs/adr/0144): in schema 5 `blocks` is an unknown field and `insert` an unknown kind.
    # A definition's objects have no persistent id; its name (Turkish case folded) and id are once in a drawing; an
    # insert names a definition, its scale is positive, `mirror` is written only when true; no definition holds
    # itself, and nesting is at most 16 levels; the lists the writer leaves out when empty are never empty.
    def block_id(n):
        return f"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{n:04x}"

    def insert_of(n, **extra):
        return cmap({"insert": cmap({**common, "block": blob(uid_bytes(block_id(n))), "p": point(one["p"]), "scale": f64(1.0), "rotation": f64(0.0), **extra})})

    def content_insert(n, **extra):
        return cmap({"insert": cmap({"attrs": cmap({}), "layerId": text("0"), "block": blob(uid_bytes(block_id(n))), "p": point({"x": 0.0, "y": 0.0}), "scale": f64(1.0), "rotation": f64(0.0), **extra})})

    circle = cmap({"circle": cmap({"attrs": cmap({}), "layerId": text("0"), "c": point({"x": 0.0, "y": 0.0}), "r": f64(0.5)})})

    def block(n, name, inside=None, **extra):
        return cmap({"id": blob(uid_bytes(block_id(n))), "base": point({"x": 0.0, "y": 0.0}), "name": text(name), "entities": array(inside if inside is not None else [circle]), **extra})

    def with_blocks(blocks, entities=None, version=6):
        entries = {**parts, "blocks": array(blocks), "entities": array(entities if entities is not None else [insert_of(1)])}
        return container(root(cmap(entries), version=uint(version)))

    # A block definition's text names no object: its objects have no persistent ids (docs/adr/0175 §4).
    block_text = cmap({"text": cmap({"attrs": cmap({}), "layerId": text("0"), "p": point({"x": 0.0, "y": 0.0}), "text": text("A"), "height": f64(1.0), "rotation": f64(0.0), "labelOf": of, "labelScale": f64(1000.0)})})
    files["block-text-label-of.kcad"] = with_blocks([block(1, "Pafta", [block_text])], version=18)
    # Nor is a definition's object's label pinned (docs/adr/0212 §2): its objects have no persistent ids.
    block_pinned = cmap({"circle": cmap({"attrs": cmap({}), "layerId": text("0"), "c": point({"x": 0.0, "y": 0.0}), "r": f64(0.5),
                                         "labelPins": array([cmap({"hidden": boolean(True)})])})})
    files["label-pins-in-block.kcad"] = with_blocks([block(1, "Rögar", [block_pinned])], version=36)
    # Nor has it attachments (docs/adr/0225 §1): they are the drawing's objects'.
    block_attached = cmap({"circle": cmap({"attrs": cmap({}), "layerId": text("0"), "c": point({"x": 0.0, "y": 0.0}), "r": f64(0.5),
                                           "attachments": array([cmap({"name": text("Foto"), "file": text("a.jpg")})])})})
    files["attachments-in-block.kcad"] = with_blocks([block(1, "Rögar", [block_attached])], version=40)
    files["blocks-in-schema-5.kcad"] = with_blocks([block(1, "Rögar")], entities=[], version=5)
    files["insert-in-schema-5.kcad"] = in_schema(5, insert_of(1))
    files["unknown-block.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(2)])
    files["unknown-block-inside.kcad"] = with_blocks([block(1, "Rögar", [content_insert(3)])])
    files["duplicate-block-name.kcad"] = with_blocks([block(1, "Direk"), block(2, "DİREK")])
    files["duplicate-block-id.kcad"] = with_blocks([block(1, "Direk"), block(1, "Lamba")])
    files["blank-block-name.kcad"] = with_blocks([block(1, " \u3000")])
    files["block-cycle.kcad"] = with_blocks([block(1, "A", [content_insert(2)]), block(2, "B", [circle, content_insert(1)])])
    # A thousand inserts of a block of a thousand circles is a million pieces; one more is too many (docs/adr/0316 §4).
    files["block-too-large.kcad"] = with_blocks([block(1, "Bin", [circle] * 1000), block(2, "Milyon", [content_insert(1)] * 1001)], entities=[])
    files["block-too-deep.kcad"] = with_blocks([block(n, f"K{n}", [content_insert(n + 1)] if n < 17 else None) for n in range(1, 18)])
    files["block-object-uid.kcad"] = with_blocks([block(1, "Rögar", [cmap({"circle": cmap({"attrs": cmap({}), "layerId": text("0"), "uid": blob(uid_bytes(m["uids"][0])), "c": point({"x": 0.0, "y": 0.0}), "r": f64(0.5)})})])])
    files["insert-scale-zero.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(1, scale=f64(0.0))])
    files["insert-mirror-false.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(1, mirror=boolean(False))])
    # An insert's y factor and a block's unit are schema 57's (docs/adr/0315 §3, §4): unknown fields in 56; the factor
    # positive and not the x one, the unit one of six words.
    files["insert-y-scale-in-schema-56.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(1, yScale=f64(2.0))], version=56)
    files["insert-y-scale-equal.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(1, yScale=f64(1.0))], version=57)
    files["insert-y-scale-zero.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(1, yScale=f64(0.0))], version=57)
    files["block-units-in-schema-56.kcad"] = with_blocks([block(1, "Rögar", units=text("mm"))], version=56)
    files["block-units-unknown.kcad"] = with_blocks([block(1, "Rögar", units=text("dm"))], version=57)
    files["empty-blocks.kcad"] = with_blocks([], entities=[])
    files["empty-attributes.kcad"] = with_blocks([block(1, "Rögar", attributes=array([]))])
    files["duplicate-attribute-tag.kcad"] = with_blocks([block(1, "Rögar", attributes=array([cmap({"p": point({"x": 0.0, "y": 0.0}), "tag": text("NO"), "height": f64(0.5), "rotation": f64(0.0)})] * 2))])
    # A text's alignment, width factor and mask are schema 7's (docs/adr/0145): in schema 6 unknown fields, on a text
    # and on an attribute definition; the left of the baseline is no value but the field's absence; a width factor is
    # over 0 and at most 100; `mask` is written only when true.
    def text_of(**extra):
        return cmap({"text": cmap({**common, "p": point(one["p"]), "text": text("R-12"), "height": f64(2.5), "rotation": f64(0.0), **extra})})

    files["text-align-in-schema-6.kcad"] = in_schema(6, text_of(align=text("middleCenter")))
    files["attribute-align-in-schema-6.kcad"] = with_blocks([block(1, "Rögar", attributes=array([cmap({"p": point({"x": 0.0, "y": 0.0}), "tag": text("NO"), "align": text("middleLeft"), "height": f64(0.5), "rotation": f64(0.0)})]))])
    files["text-align-baseline-left.kcad"] = in_schema(7, text_of(align=text("baselineLeft")))
    files["text-align-unknown.kcad"] = in_schema(7, text_of(align=text("center")))
    files["text-width-factor-zero.kcad"] = in_schema(7, text_of(widthFactor=f64(0.0)))
    files["text-width-factor-too-wide.kcad"] = in_schema(7, text_of(widthFactor=f64(100.5)))
    files["text-width-factor-nan.kcad"] = in_schema(7, text_of(widthFactor=b"\xfb\x7f\xf8\x00\x00\x00\x00\x00\x00"))
    files["text-mask-false.kcad"] = in_schema(7, text_of(mask=boolean(False)))
    # A multi-line text's box, line spacing and runs are schema 20's (docs/adr/0182 §1): in schema 19 unknown fields;
    # a box over 0, a spacing from 0.25 to 4, runs in the text's letters (Unicode scalar values), in order, apart, each
    # with a format (a flag written only when true, a colour not empty), touching runs of one format one; the list is
    # not empty.
    def run_of(start, end, **extra):
        return cmap({"start": uint(start), "end": uint(end), **extra})

    files["paragraph-in-schema-19.kcad"] = in_schema(19, text_of(boxWidth=f64(20.0)))
    files["paragraph-box-zero.kcad"] = in_schema(20, text_of(boxWidth=f64(0.0)))
    files["paragraph-spacing-too-wide.kcad"] = in_schema(20, text_of(lineSpacing=f64(4.5)))
    files["paragraph-runs-empty.kcad"] = in_schema(20, text_of(runs=array([])))
    files["paragraph-run-past-text.kcad"] = in_schema(20, text_of(runs=array([run_of(2, 5, bold=boolean(True))])))
    files["paragraph-run-backwards.kcad"] = in_schema(20, text_of(runs=array([run_of(3, 1, bold=boolean(True))])))
    files["paragraph-run-overlap.kcad"] = in_schema(20, text_of(runs=array([run_of(0, 3, bold=boolean(True)), run_of(2, 4, italic=boolean(True))])))
    files["paragraph-run-touching-same.kcad"] = in_schema(20, text_of(runs=array([run_of(0, 2, bold=boolean(True)), run_of(2, 4, bold=boolean(True))])))
    files["paragraph-run-formatless.kcad"] = in_schema(20, text_of(runs=array([run_of(0, 2)])))
    files["paragraph-run-bold-false.kcad"] = in_schema(20, text_of(runs=array([run_of(0, 2, bold=boolean(False))])))
    files["paragraph-run-empty-color.kcad"] = in_schema(20, text_of(runs=array([run_of(0, 2, color=text(""))])))
    files["paragraph-run-script-unknown.kcad"] = in_schema(20, text_of(runs=array([run_of(0, 2, script=text("upper"))])))
    files["paragraph-run-unknown-field.kcad"] = in_schema(20, text_of(runs=array([run_of(0, 2, size=f64(2.0))])))
    # A text's face and a dimension's look are schema 21's (docs/adr/0183): in schema 20 unknown fields; a typeface by
    # one of its names, bold and italic written only when true and only with a typeface, a slant not 0 and under 85
    # either way; a dimension's arrowhead by name, its sizes (times its height) from 0 (the arrowhead's over 0) to
    # 100, at most 8 decimals, a prefix and a suffix of one line, 1 to 32 letters.
    def dimension_of(**extra):
        b = {"x": one["p"]["x"] + 10.0, "y": one["p"]["y"]}
        return cmap({"dimension": cmap({**common, "a": point(one["p"]), "b": point(b), "offset": f64(2.0), "height": f64(0.5), **extra})})

    files["style-face-in-schema-20.kcad"] = in_schema(20, text_of(font=text("arimo")))
    files["style-face-font-unknown.kcad"] = in_schema(21, text_of(font=text("arial")))
    files["style-face-bold-false.kcad"] = in_schema(21, text_of(font=text("arimo"), bold=boolean(False)))
    files["style-face-bold-without-font.kcad"] = in_schema(21, text_of(bold=boolean(True)))
    files["style-face-oblique-zero.kcad"] = in_schema(21, text_of(font=text("arimo"), oblique=f64(0.0)))
    files["style-face-oblique-steep.kcad"] = in_schema(21, text_of(font=text("arimo"), oblique=f64(85.0)))
    files["style-face-style-empty.kcad"] = in_schema(21, text_of(textStyle=text("")))
    files["style-look-in-schema-20.kcad"] = in_schema(20, dimension_of(arrow=text("closed")))
    files["style-look-arrow-unknown.kcad"] = in_schema(21, dimension_of(arrow=text("tick")))
    files["style-look-arrow-size-zero.kcad"] = in_schema(21, dimension_of(arrowSize=f64(0.0)))
    files["style-look-gap-negative.kcad"] = in_schema(21, dimension_of(extOffset=f64(-0.5)))
    files["style-look-gap-too-wide.kcad"] = in_schema(21, dimension_of(textGap=f64(100.5)))
    files["style-look-decimals-nine.kcad"] = in_schema(21, dimension_of(decimals=uint(9)))
    files["style-look-unit-unknown.kcad"] = in_schema(21, dimension_of(unit=text("km")))
    files["style-look-prefix-empty.kcad"] = in_schema(21, dimension_of(prefix=text("")))
    files["style-look-suffix-line-break.kcad"] = in_schema(21, dimension_of(suffix=text("m\n2")))
    files["style-look-place-unknown.kcad"] = in_schema(21, dimension_of(textPlace=text("below")))

    # The settings' style tables (schema 21): each style's fields; the tables' name rule.
    def with_styles(version, **tables):
        return container(root(cmap({**parts, "settings": cmap({**settings_parts(m["settings"]), **tables})}), version=uint(version)))

    def text_style(**extra):
        return cmap({"id": text("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101"), "name": text("Ada no"), "font": text("arimo"), **extra})

    def dimension_style(**extra):
        return cmap({"id": text("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0201"), "name": text("Mimari"), "height": f64(3.5), **extra})

    files["style-table-in-schema-20.kcad"] = with_styles(20, textStyles=array([text_style()]))
    files["style-table-name-twice.kcad"] = with_styles(21, textStyles=array([text_style(), text_style(id=text("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0102"), name=text("ADA NO"))]))
    files["style-table-standart.kcad"] = with_styles(21, textStyles=array([text_style(name=text("Standart"))]))
    files["style-table-name-padded.kcad"] = with_styles(21, textStyles=array([text_style(name=text(" Ada no"))]))
    files["style-table-id-empty.kcad"] = with_styles(21, dimensionStyles=array([dimension_style(id=text(""))]))
    files["style-table-font-missing.kcad"] = with_styles(21, textStyles=array([cmap({"id": text("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101"), "name": text("Ada no")})]))
    files["style-table-width-one.kcad"] = with_styles(21, textStyles=array([text_style(widthFactor=f64(1.0))]))
    files["style-table-height-zero.kcad"] = with_styles(21, dimensionStyles=array([dimension_style(height=f64(0.0))]))
    files["style-table-ratio-too-big.kcad"] = with_styles(21, dimensionStyles=array([dimension_style(textGap=f64(400.0))]))
    files["style-table-unknown-field.kcad"] = with_styles(21, textStyles=array([text_style(color=text("red"))]))
    # A table is schema 22's (docs/adr/0184): in schema 21 an unknown kind; only in the drawing; its rows, columns and
    # cells as many as each other, a cell of one line, a merged range inside it and apart from the others with only its
    # top left cell's words, a heading row and merged ranges written only when there are, a frame inside it, lines by
    # name (all: no field), a source naming objects or a file.
    def table_of(**extra):
        cells = array([array([text("Ad"), text("Alan")]), array([text("7"), text("600.00")])])
        body = {**common, "p": point(one["p"]), "rotation": f64(0.0), "height": f64(1.25), "rows": floats([2.5, 2.5]), "columns": floats([6.0, 8.0]), "cells": cells, **extra}
        return cmap({"table": cmap(body)})

    one_line = lambda words: array([array([text("Ad"), text("Alan")]), array([text(words), text("600.00")])])  # noqa: E731
    files["table-in-schema-21.kcad"] = in_schema(21, table_of())
    files["table-in-block.kcad"] = with_blocks([block(1, "Çizelge", [cmap({"table": cmap({"attrs": cmap({}), "layerId": text("0"), "p": point({"x": 0.0, "y": 0.0}), "rotation": f64(0.0), "height": f64(1.25), "rows": floats([2.5]), "columns": floats([6.0]), "cells": array([array([text("A")])])})})])], version=22)
    files["table-cells-short.kcad"] = in_schema(22, table_of(cells=array([array([text("Ad"), text("Alan")])])))
    files["table-cell-line-break.kcad"] = in_schema(22, table_of(cells=one_line("7\n8")))
    files["table-merge-outside.kcad"] = in_schema(22, table_of(merges=array([cmap({"row": uint(1), "col": uint(1), "rows": uint(2), "cols": uint(1)})])))
    files["table-merge-hides-words.kcad"] = in_schema(22, table_of(merges=array([cmap({"row": uint(0), "col": uint(0), "rows": uint(1), "cols": uint(2)})])))
    files["table-merges-empty.kcad"] = in_schema(22, table_of(merges=array([])))
    files["table-header-false.kcad"] = in_schema(22, table_of(header=boolean(False)))
    files["table-grid-all.kcad"] = in_schema(22, table_of(grid=text("all")))
    files["table-frame-too-wide.kcad"] = in_schema(22, table_of(frame=f64(2.5)))
    files["table-aligns-short.kcad"] = in_schema(22, table_of(aligns=array([text("left")])))
    files["table-source-nil.kcad"] = in_schema(22, table_of(source=cmap({"kind": text("areas"), "objects": array([blob(bytes(16))])})))
    # A hatch's patterns, gradients and ties are schema 23's (docs/adr/0186): in schema 22 the kinds are unknown values
    # and the fields unknown ones; a pattern has a name, a scale and families within their bounds, each family's lines
    # apart and a dash with a length; a gradient a #RRGGBB second colour, `inverted` only when true; a tie names each
    # object once, only in the drawing.
    square = array([point({"x": one["p"]["x"] + x, "y": one["p"]["y"] + y}) for x, y in ((0.0, 0.0), (4.0, 0.0), (4.0, 4.0))])
    family = lambda **extra: cmap({"angle": f64(45.0), "origin": point({"x": 0.0, "y": 0.0}), "offset": point({"x": 0.0, "y": 3.175}), **extra})

    def hatch_of(pattern, **extra):
        return cmap({"hatch": cmap({**common, "ring": square, "pattern": cmap(pattern), **extra})})

    ansi = {"type": text("pattern"), "angle": f64(0.0), "spacing": f64(1.0), "name": text("ANSI31"), "scale": f64(0.5), "lines": array([family()])}
    grad = lambda **g: {"type": text("gradient"), "angle": f64(0.0), "spacing": f64(1.0), "gradient": cmap({"shape": text("linear"), "color2": text("#FFFFFF"), **g})}
    tie = lambda **extra: cmap({"outer": blob(uid_bytes(m["uids"][0])), "seed": point(one["p"]), **extra})
    files["hatch-pattern-in-schema-22.kcad"] = in_schema(22, hatch_of({"type": text("gradient"), "angle": f64(0.0), "spacing": f64(1.0)}))
    files["hatch-name-in-schema-22.kcad"] = in_schema(22, hatch_of({"type": text("lines"), "angle": f64(45.0), "spacing": f64(1.0), "name": text("ANSI31")}))
    files["hatch-assoc-in-schema-22.kcad"] = in_schema(22, hatch_of({"type": text("solid"), "angle": f64(0.0), "spacing": f64(1.0)}, assoc=tie()))
    files["hatch-pattern-without-lines.kcad"] = in_schema(23, hatch_of({k: v for k, v in ansi.items() if k != "lines"}))
    files["hatch-pattern-without-name.kcad"] = in_schema(23, hatch_of({k: v for k, v in ansi.items() if k != "name"}))
    files["hatch-pattern-scale-zero.kcad"] = in_schema(23, hatch_of({**ansi, "scale": f64(0.0)}))
    files["hatch-pattern-lines-apart-zero.kcad"] = in_schema(23, hatch_of({**ansi, "lines": array([family(offset=point({"x": 3.0, "y": 0.0}))])}))
    files["hatch-pattern-dashes-zero.kcad"] = in_schema(23, hatch_of({**ansi, "lines": array([family(dashes=floats([0.0, 0.0]))])}))
    files["hatch-pattern-dashes-empty.kcad"] = in_schema(23, hatch_of({**ansi, "lines": array([family(dashes=array([]))])}))
    files["hatch-pattern-many-dashes.kcad"] = in_schema(23, hatch_of({**ansi, "lines": array([family(dashes=floats([1.0, -1.0] * 9))])}))
    files["hatch-lines-with-name.kcad"] = in_schema(23, hatch_of({"type": text("lines"), "angle": f64(45.0), "spacing": f64(1.0), "name": text("ANSI31")}))
    files["hatch-gradient-without-gradient.kcad"] = in_schema(23, hatch_of({"type": text("gradient"), "angle": f64(0.0), "spacing": f64(1.0)}))
    files["hatch-gradient-colour-name.kcad"] = in_schema(23, hatch_of(grad(color2=text("beyaz"))))
    files["hatch-gradient-inverted-false.kcad"] = in_schema(23, hatch_of(grad(inverted=boolean(False))))
    files["hatch-gradient-shape-unknown.kcad"] = in_schema(23, hatch_of(grad(shape=text("curved"))))
    files["hatch-assoc-twice.kcad"] = in_schema(23, hatch_of(ansi, assoc=tie(islands=array([blob(uid_bytes(m["uids"][0]))]))))
    files["hatch-assoc-seed-nan.kcad"] = in_schema(23, hatch_of(ansi, assoc=cmap({"outer": blob(uid_bytes(m["uids"][0])), "seed": array([nan, f64(0.0)])})))
    files["hatch-assoc-islands-empty.kcad"] = in_schema(23, hatch_of(ansi, assoc=tie(islands=array([]))))
    files["hatch-assoc-in-block.kcad"] = with_blocks([block(1, "Tarama", [cmap({"hatch": cmap({"attrs": cmap({}), "layerId": text("0"), "ring": square, "pattern": cmap(ansi), "assoc": tie()})})])], version=23)
    # A dimension's references are schema 43's (docs/adr/0249 §4): in schema 42 an unknown field, in a block definition
    # too (its objects have no persistent ids to name); no references, no field; an id not nil; a place's kind and a
    # broken one's code known; a field neither the references nor a reference nor a place has, unknown.
    corner = lambda i, **extra: cmap({"uid": blob(uid_bytes(m["uids"][0])), "place": cmap({"kind": text("vertex"), "part": uint(0), "ring": uint(0), "index": uint(i)}), **extra})
    dim_body = {"a": point(one["p"]), "b": point({"x": one["p"]["x"] + 10.0, "y": one["p"]["y"]}), "offset": f64(2.0), "height": f64(2.5)}

    def dimension_of(refs):
        return cmap({"dimension": cmap({**common, **dim_body, "refs": refs})})

    files["dimension-refs-in-schema-42.kcad"] = in_schema(42, dimension_of(cmap({"a": corner(0)})))
    files["dimension-refs-empty.kcad"] = in_schema(43, dimension_of(cmap({})))
    files["dimension-refs-zero-uid.kcad"] = in_schema(43, dimension_of(cmap({"a": cmap({"uid": blob(bytes(16)), "place": cmap({"kind": text("start")})})})))
    files["dimension-refs-unknown-kind.kcad"] = in_schema(43, dimension_of(cmap({"a": cmap({"uid": blob(uid_bytes(m["uids"][0])), "place": cmap({"kind": text("edge")})})})))
    files["dimension-refs-unknown-broken.kcad"] = in_schema(43, dimension_of(cmap({"b": corner(1, broken=text("lost"))})))
    files["dimension-refs-unknown-field.kcad"] = in_schema(43, dimension_of(cmap({"a": corner(0), "d": corner(1)})))
    # A reference along an edge and inside a block are schema 51's (docs/adr/0263): in schema 50 `along` is an unknown
    # place and `path` an unknown field; t within [0, 1]; a path neither empty nor deeper than 16.
    along = lambda t, **extra: cmap({"uid": blob(uid_bytes(m["uids"][0])), "place": cmap({"kind": text("along"), "part": uint(0), "ring": uint(0), "index": uint(1), "t": f64(t)}), **extra})  # noqa: E731
    files["dimension-refs-along-in-schema-50.kcad"] = in_schema(50, dimension_of(cmap({"a": along(0.5)})))
    files["dimension-refs-path-in-schema-50.kcad"] = in_schema(50, dimension_of(cmap({"a": corner(0, path=array([uint(0)]))})))
    files["dimension-refs-along-t-above-one.kcad"] = in_schema(51, dimension_of(cmap({"a": along(1.5)})))
    files["dimension-refs-path-empty.kcad"] = in_schema(51, dimension_of(cmap({"a": corner(0, path=array([]))})))
    files["dimension-refs-path-deep.kcad"] = in_schema(51, dimension_of(cmap({"a": corner(0, path=array([uint(0)] * 17))})))
    files["dimension-refs-in-block.kcad"] = with_blocks([block(1, "Ölçü", [cmap({"dimension": cmap({"attrs": cmap({}), "layerId": text("0"), **dim_body, "refs": cmap({"a": corner(0)})})})])], version=43)
    # A closed and a wide polyline are schema 46's (docs/adr/0258): in schema 45 unknown fields; an area has neither;
    # `closed` only true; widths one pair of two numbers for each vertex, not below 0, the last pair 0 on an open
    # line, not all 0; a closed or wide polyline of one part.
    pair = lambda a, b: floats([a, b])  # noqa: E731
    files["polyline-closed-in-schema-45.kcad"] = in_schema(45, path("polyline", 3, closed=boolean(True)))
    files["polyline-widths-in-schema-45.kcad"] = in_schema(45, path("polyline", 2, widths=array([pair(1.0, 1.0), pair(0.0, 0.0)])))
    files["polyline-closed-false.kcad"] = in_schema(46, path("polyline", 3, closed=boolean(False)))
    files["polyline-widths-count.kcad"] = in_schema(46, path("polyline", 3, widths=array([pair(1.0, 1.0), pair(0.0, 0.0)])))
    files["polyline-widths-negative.kcad"] = in_schema(46, path("polyline", 2, widths=array([pair(-1.0, 1.0), pair(0.0, 0.0)])))
    files["polyline-widths-all-zero.kcad"] = in_schema(46, path("polyline", 2, widths=array([pair(0.0, 0.0), pair(0.0, 0.0)])))
    files["polyline-widths-open-last.kcad"] = in_schema(46, path("polyline", 2, widths=array([pair(1.0, 1.0), pair(1.0, 1.0)])))
    files["polyline-widths-three.kcad"] = in_schema(46, path("polyline", 2, widths=array([floats([1.0, 1.0, 1.0]), pair(0.0, 0.0)])))
    files["polyline-widths-nan.kcad"] = in_schema(46, path("polyline", 2, widths=array([array([nan, f64(1.0)]), pair(0.0, 0.0)])))
    files["polyline-closed-parts.kcad"] = in_schema(46, path("polyline", 3, closed=boolean(True), parts=array([cmap({"pts": square_line(5.0)})])))
    files["polygon-closed.kcad"] = in_schema(46, path("polygon", 3, closed=boolean(True)))
    files["polygon-widths.kcad"] = in_schema(46, path("polygon", 3, widths=array([pair(1.0, 1.0)] * 3)))
    # A NURBS curve is schema 47's (docs/adr/0259): in schema 46 an unknown kind; its numbers a curve's (the degree 1 to
    # 11, as many knots as control points + degree + 1, never decreasing, an inner knot at most degree times, positive
    # weights one per control point, two tangents); `closed` and `periodic` only true.
    def nurbs_of(**extra):
        ctrl = array([point({"x": one["p"]["x"] + i, "y": one["p"]["y"] + (i % 2)}) for i in range(4)])
        body = {**common, "degree": uint(2), "ctrl": ctrl, "knots": floats([0.0, 0.0, 0.0, 1.0, 2.0, 2.0, 2.0])}
        return cmap({"nurbs": cmap({**body, **extra})})

    # An object's look is schema 48's (docs/adr/0260): in schema 47 unknown fields; a scale above 0 and at most 1000,
    # a transparency from 0 to 0.9, a whole order on the drawing's objects only, by-layer parts on a block's only, in
    # order, once, never one with a value of its own; a layer's plot only false.
    files["look-in-schema-47.kcad"] = in_schema(47, line(lineType=text("dashed")))
    files["look-line-type-unknown.kcad"] = in_schema(48, line(lineType=text("hidden")))
    files["look-scale-zero.kcad"] = in_schema(48, line(lineTypeScale=f64(0.0)))
    files["look-transparency-high.kcad"] = in_schema(48, line(transparency=f64(0.95)))
    files["look-order-float.kcad"] = in_schema(48, line(order=f64(1.5)))
    files["look-by-layer-in-drawing.kcad"] = in_schema(48, line(byLayer=array([text("color")])))
    # An area's mask is schema 54's (docs/adr/0310 §1): only an area's, `{ frame }` with frame a boolean and nothing else.
    files["wipeout-on-a-line.kcad"] = in_schema(54, line(wipeout=cmap({"frame": boolean(True)})))
    files["wipeout-without-frame.kcad"] = in_schema(54, path("polygon", 3, wipeout=cmap({})))
    files["wipeout-frame-not-bool.kcad"] = in_schema(54, path("polygon", 3, wipeout=cmap({"frame": uint(1)})))
    files["wipeout-unknown-field.kcad"] = in_schema(54, path("polygon", 3, wipeout=cmap({"fill": boolean(True), "frame": boolean(True)})))
    files["plot-true.kcad"] = container(root(cmap({**parts, "layers": array([cmap({**layer_entries(m["layers"][0]), "plot": boolean(True)})])}), version=uint(48)))
    # A layer's freeze is schema 55's (docs/adr/0311 §1): only true is written.
    files["frozen-false.kcad"] = container(root(cmap({**parts, "layers": array([cmap({**layer_entries(m["layers"][0]), "frozen": boolean(False)})])}), version=uint(55)))
    # The UCS settings are schema 49's (docs/adr/0261 §1): in schema 48 unknown fields; the world's never written as the
    # current one, an angle in [0, 2π), an origin within 10 000 km, names that hold and are unique, an empty list refused.
    def with_ucs(extra, schema=49):
        return container(root(cmap({**parts, "settings": cmap({**settings_parts(m["settings"]), **extra})}), version=uint(schema)))

    def frame_of(x=10.0, y=20.0, a=1.0, **more):
        return cmap({**more, "origin": point({"x": x, "y": y}), "angle": f64(a)})

    files["ucs-in-schema-48.kcad"] = with_ucs({"ucs": frame_of()}, schema=48)
    # Inches, feet and a length style are schema 50's (docs/adr/0262): in schema 49 an inch is an unknown unit and the
    # style an unknown field; a style other than the three refused, decimal too (it is no setting).
    local = {"srid": uint(0)}
    files["drawing-unit-in-in-schema-49.kcad"] = with_ucs({**local, "drawingUnit": text("in")}, schema=49)
    files["length-style-in-schema-49.kcad"] = with_ucs({**local, "drawingUnit": text("mm"), "lengthStyle": text("fractional")}, schema=49)
    files["length-style-decimal.kcad"] = with_ucs({**local, "drawingUnit": text("in"), "lengthStyle": text("decimal")}, schema=50)
    files["length-style-metric.kcad"] = with_ucs({**local, "drawingUnit": text("mm"), "lengthStyle": text("architectural")}, schema=50)
    files["ucs-world.kcad"] = with_ucs({"ucs": frame_of(0.0, 0.0, 0.0)})
    files["ucs-angle-tau.kcad"] = with_ucs({"ucs": frame_of(a=TAU)})
    files["ucs-far.kcad"] = with_ucs({"ucs": frame_of(x=2.0e7)})
    files["named-ucs-empty.kcad"] = with_ucs({"namedUcs": array([])})
    files["named-ucs-twice.kcad"] = with_ucs({"namedUcs": array([frame_of(name=text("A")), frame_of(name=text("A"))])})
    files["named-ucs-blank.kcad"] = with_ucs({"namedUcs": array([frame_of(name=text(" "))])})
    files["named-ucs-unknown-field.kcad"] = with_ucs({"namedUcs": array([frame_of(name=text("A"), note=text("x"))])})
    # The named views are schema 52's (docs/adr/0285 §1): in schema 51 an unknown field; a middle within 10 000 km, a
    # width and a height above 0 and at most 20 000 km, a twist in [0, 2π), names that hold and are unique, an empty
    # list refused, every field required.
    def view_of(name="A", x=10.0, y=20.0, w=30.0, h=20.0, t=0.5, drop=(), **more):
        f = {"name": text(name), "center": point({"x": x, "y": y}), "width": f64(w), "height": f64(h), "twist": f64(t), **more}
        return cmap({k: v for k, v in f.items() if k not in drop})

    files["views-in-schema-51.kcad"] = with_ucs({"namedViews": array([view_of()])}, schema=51)
    files["named-views-empty.kcad"] = with_ucs({"namedViews": array([])}, schema=52)
    files["named-views-twice.kcad"] = with_ucs({"namedViews": array([view_of(), view_of(x=0.0)])}, schema=52)
    files["named-views-blank.kcad"] = with_ucs({"namedViews": array([view_of(name=" ")])}, schema=52)
    files["named-views-width-zero.kcad"] = with_ucs({"namedViews": array([view_of(w=0.0)])}, schema=52)
    files["named-views-twist-tau.kcad"] = with_ucs({"namedViews": array([view_of(t=TAU)])}, schema=52)
    files["named-views-far.kcad"] = with_ucs({"namedViews": array([view_of(x=2.0e7)])}, schema=52)
    files["named-views-unknown-field.kcad"] = with_ucs({"namedViews": array([view_of(note=text("x"))])}, schema=52)
    files["named-views-no-twist.kcad"] = with_ucs({"namedViews": array([view_of(drop=("twist",))])}, schema=52)
    files["nurbs-in-schema-46.kcad"] = in_schema(46, nurbs_of())
    files["nurbs-closed-false.kcad"] = in_schema(47, nurbs_of(closed=boolean(False)))
    files["nurbs-degree-12.kcad"] = in_schema(47, nurbs_of(degree=uint(12)))
    files["nurbs-knot-count.kcad"] = in_schema(47, nurbs_of(knots=floats([0.0, 0.0, 0.0, 1.0, 2.0, 2.0])))
    files["nurbs-knots-decreasing.kcad"] = in_schema(47, nurbs_of(knots=floats([0.0, 0.0, 0.0, 2.0, 1.0, 2.0, 2.0])))
    files["nurbs-inner-knot.kcad"] = in_schema(47, nurbs_of(degree=uint(1), knots=floats([0.0, 0.0, 1.0, 1.0, 2.0, 2.0])))
    files["nurbs-weight-zero.kcad"] = in_schema(47, nurbs_of(weights=floats([1.0, 0.0, 1.0, 1.0])))
    files["nurbs-weight-count.kcad"] = in_schema(47, nurbs_of(weights=floats([1.0, 1.0, 1.0])))
    files["nurbs-knot-nan.kcad"] = in_schema(47, nurbs_of(knots=array([f64(0.0)] * 3 + [nan] + [f64(2.0)] * 3)))
    files["nurbs-tangents-three.kcad"] = in_schema(47, nurbs_of(tangents=array([b"\xf6"] * 3)))
    # A picture is schema 24's (docs/adr/0192): in schema 23 an unknown kind; only in the drawing; a positive size, one
    # source, a clip of three corners or more inside the picture, an opacity from 0.1 to 1, `mirror` only when true.
    def image_of(**extra):
        body = {**common, "p": point(one["p"]), "width": f64(8.0), "height": f64(6.0), "rotation": f64(0.0), "asset": text("resim-0011223344556677"), **extra}
        return cmap({"image": cmap({k: v for k, v in body.items() if v is not None})})

    unit_square = lambda *xy: array([point({"x": x, "y": y}) for x, y in xy])  # noqa: E731
    files["image-in-schema-23.kcad"] = in_schema(23, image_of())
    files["image-in-block.kcad"] = with_blocks([block(1, "Logo", [cmap({"image": cmap({"attrs": cmap({}), "layerId": text("0"), "p": point({"x": 0.0, "y": 0.0}), "width": f64(8.0), "height": f64(6.0), "rotation": f64(0.0), "asset": text("resim-0011223344556677")})})])], version=24)
    files["image-two-sources.kcad"] = in_schema(24, image_of(file=text("logo.png")))
    files["image-no-source.kcad"] = in_schema(24, image_of(asset=None))
    files["image-width-zero.kcad"] = in_schema(24, image_of(width=f64(0.0)))
    files["image-clip-outside.kcad"] = in_schema(24, image_of(clip=unit_square((0.0, 0.0), (1.5, 0.0), (1.5, 1.0))))
    files["image-clip-two-corners.kcad"] = in_schema(24, image_of(clip=unit_square((0.0, 0.0), (1.0, 1.0))))
    files["image-opacity-low.kcad"] = in_schema(24, image_of(opacity=f64(0.05)))
    files["image-mirror-false.kcad"] = in_schema(24, image_of(mirror=boolean(False)))
    files["image-file-line-break.kcad"] = in_schema(24, image_of(asset=None, file=text("foto/\nsaha.jpg")))
    # A raster is schema 29's (docs/adr/0204 §2): in schema 28 an unknown kind; only in the drawing; an affine that
    # inverts, a size and bands within their bounds, one source, a look that suits its bands with its fields' defaults
    # left out, an opacity from 0.1 to 1.
    raster_affine = floats([487100.0, 0.5, 0.0, 4420800.0, 0.0, -0.5])

    def raster_of(style=None, **extra):
        look = style if style is not None else cmap({"render": text("rgb"), "bands": array([uint(1), uint(2), uint(3)])})
        body = {**common, "affine": raster_affine, "width": uint(2000), "height": uint(1500), "bands": uint(3), "sample": text("u8"), "file": text("pafta-12.tif"), "srid": uint(5256), "style": look, **extra}
        return cmap({"raster": cmap({k: v for k, v in body.items() if v is not None})})

    gray = lambda **extra: cmap({"render": text("gray"), "bands": array([uint(1)]), **extra})  # noqa: E731
    files["raster-in-schema-28.kcad"] = in_schema(28, raster_of())
    files["raster-in-block.kcad"] = with_blocks([block(1, "Orto", [cmap({"raster": cmap({"attrs": cmap({}), "layerId": text("0"), "affine": raster_affine, "width": uint(2000), "height": uint(1500), "bands": uint(3), "sample": text("u8"), "file": text("pafta-12.tif"), "srid": uint(5256), "style": cmap({"render": text("rgb"), "bands": array([uint(1), uint(2), uint(3)])})})})])], version=29)
    files["raster-affine-flat.kcad"] = in_schema(29, raster_of(affine=floats([487100.0, 0.5, 0.5, 4420800.0, 0.5, 0.5])))
    files["raster-affine-five.kcad"] = in_schema(29, raster_of(affine=floats([487100.0, 0.5, 0.0, 4420800.0, 0.0])))
    files["raster-width-zero.kcad"] = in_schema(29, raster_of(width=uint(0)))
    files["raster-two-sources.kcad"] = in_schema(29, raster_of(asset=text("raster-0011223344556677")))
    files["raster-no-source.kcad"] = in_schema(29, raster_of(file=None))
    files["raster-sample-unknown.kcad"] = in_schema(29, raster_of(sample=text("u12")))
    files["raster-rgb-two-bands.kcad"] = in_schema(29, raster_of(style=cmap({"render": text("rgb"), "bands": array([uint(1), uint(2)])})))
    files["raster-band-missing.kcad"] = in_schema(29, raster_of(style=gray(bands=array([uint(4)]))))
    files["raster-stretch-none.kcad"] = in_schema(29, raster_of(style=gray(stretch=text("none"))))
    files["raster-manual-upside-down.kcad"] = in_schema(29, raster_of(style=gray(stretch=text("manual"), min=f64(10.0), max=f64(2.0))))
    files["raster-ramp-unknown.kcad"] = in_schema(29, raster_of(style=gray(render=text("ramp"), ramp=text("Gökkuşağı"))))
    files["raster-light-below.kcad"] = in_schema(29, raster_of(style=gray(render=text("hillshade"), altitude=f64(-5.0))))
    files["raster-invert-false.kcad"] = in_schema(29, raster_of(style=gray(invert=boolean(False))))
    files["raster-bilinear-written.kcad"] = in_schema(29, raster_of(style=gray(resampling=text("bilinear"))))
    files["raster-opacity-low.kcad"] = in_schema(29, raster_of(opacity=f64(0.05)))
    # Schema 31 (docs/adr/0207): a raster's address beside its file is two sources; an address that is not HTTP; in
    # schema 30 an unknown field.
    files["raster-url-in-schema-30.kcad"] = in_schema(30, raster_of(file=None, url=text("https://ornek.org/orto.tif")))
    files["raster-url-and-file.kcad"] = in_schema(31, raster_of(url=text("https://ornek.org/orto.tif")))
    files["raster-url-ftp.kcad"] = in_schema(31, raster_of(file=None, url=text("ftp://ornek.org/orto.tif")))
    # A raster's NetCDF variable and its look's mesh lines are schema 38's (docs/adr/0243 §6): in schema 37 unknown
    # fields; one band; names given; no empty list, no false; an index within its values; one time dimension, its values
    # not falling; following the slider needs it; lines only on a mesh, as #RRGGBB.
    def dim_of(**extra):
        body = {"name": text("time"), "index": uint(0), "values": floats([0.0, 3600000.0]), "time": boolean(True), **extra}
        return cmap({k: v for k, v in body.items() if v is not None})

    def dataset_of(**extra):
        body = {"variable": text("depth"), "mesh": text("mesh"), "dims": array([dim_of()]), **extra}
        return cmap({k: v for k, v in body.items() if v is not None})

    one_band = dict(bands=uint(1), style=gray())
    files["dataset-in-schema-37.kcad"] = in_schema(37, raster_of(dataset=dataset_of(), **one_band))
    files["dataset-three-bands.kcad"] = in_schema(38, raster_of(dataset=dataset_of()))
    files["dataset-follow-false.kcad"] = in_schema(38, raster_of(dataset=dataset_of(followTime=boolean(False)), **one_band))
    files["dataset-dims-empty.kcad"] = in_schema(38, raster_of(dataset=dataset_of(dims=array([])), **one_band))
    files["dataset-index-out.kcad"] = in_schema(38, raster_of(dataset=dataset_of(dims=array([dim_of(index=uint(2))])), **one_band))
    files["dataset-two-times.kcad"] = in_schema(38, raster_of(dataset=dataset_of(dims=array([dim_of(), dim_of(name=text("t2"))])), **one_band))
    files["dataset-time-falling.kcad"] = in_schema(38, raster_of(dataset=dataset_of(dims=array([dim_of(values=floats([3600000.0, 0.0]))])), **one_band))
    files["dataset-follow-without-time.kcad"] = in_schema(38, raster_of(dataset=dataset_of(dims=array([dim_of(time=None)]), followTime=boolean(True)), **one_band))
    files["dataset-variable-empty.kcad"] = in_schema(38, raster_of(dataset=dataset_of(variable=text(" ")), **one_band))
    files["edges-in-schema-37.kcad"] = in_schema(37, raster_of(bands=uint(1), style=gray(edges=text("#203040"))))
    files["edges-without-mesh.kcad"] = in_schema(38, raster_of(bands=uint(1), style=gray(edges=text("#203040")), dataset=dataset_of(mesh=None)))
    files["edges-not-a-colour.kcad"] = in_schema(38, raster_of(bands=uint(1), style=gray(edges=text("lacivert")), dataset=dataset_of()))
    # A point cloud is schema 31's (docs/adr/0207 §3): in schema 30 an unknown kind; only in the drawing; files each
    # with one source, the points their sum, bounds in order, a look whose range a ramp needs, its defaults left out,
    # a size from 0.5 to 32, an opacity from 0.1 to 1.
    cloud_bounds = floats([487600.0, 4420300.0, 838.9, 487720.0, 4420390.0, 862.7])

    def cloud_file(**extra):
        body = {"file": text("koy.laz"), "format": text("laz"), "count": uint(71212), "bounds": cloud_bounds, **extra}
        return cmap({k: v for k, v in body.items() if v is not None})

    def cloud_of(style=None, sources=None, **extra):
        look = style if style is not None else cmap({"render": text("rgb"), "size": f64(2.0)})
        body = {**common, "sources": array(sources if sources is not None else [cloud_file()]), "bounds": cloud_bounds, "count": uint(71212), "srid": uint(5256), "style": look, **extra}
        return cmap({"pointcloud": cmap({k: v for k, v in body.items() if v is not None})})

    files["pointcloud-in-schema-30.kcad"] = in_schema(30, cloud_of())
    files["pointcloud-in-block.kcad"] = with_blocks([block(1, "Bulut", [cmap({"pointcloud": cmap({"attrs": cmap({}), "layerId": text("0"), "sources": array([cloud_file()]), "bounds": cloud_bounds, "count": uint(71212), "srid": uint(5256), "style": cmap({"render": text("rgb"), "size": f64(2.0)})})})])], version=31)
    files["pointcloud-no-files.kcad"] = in_schema(31, cloud_of(sources=[], count=uint(0)))
    files["pointcloud-two-sources.kcad"] = in_schema(31, cloud_of(sources=[cloud_file(url=text("https://ornek.org/koy.laz"))]))
    files["pointcloud-no-source.kcad"] = in_schema(31, cloud_of(sources=[cloud_file(file=None)]))
    files["pointcloud-url-ftp.kcad"] = in_schema(31, cloud_of(sources=[cloud_file(file=None, url=text("ftp://ornek.org/koy.laz"))]))
    files["pointcloud-format-unknown.kcad"] = in_schema(31, cloud_of(sources=[cloud_file(format=text("e57"))]))
    files["pointcloud-count-not-sum.kcad"] = in_schema(31, cloud_of(count=uint(71213)))
    files["pointcloud-bounds-five.kcad"] = in_schema(31, cloud_of(bounds=floats([487600.0, 4420300.0, 838.9, 487720.0, 4420390.0])))
    files["pointcloud-bounds-upside-down.kcad"] = in_schema(31, cloud_of(bounds=floats([487720.0, 4420300.0, 838.9, 487600.0, 4420390.0, 862.7])))
    files["pointcloud-ramp-without-range.kcad"] = in_schema(31, cloud_of(style=cmap({"render": text("elevation"), "size": f64(2.0)})))
    files["pointcloud-size-zero.kcad"] = in_schema(31, cloud_of(style=cmap({"render": text("rgb"), "size": f64(0.0)})))
    files["pointcloud-hidden-unsorted.kcad"] = in_schema(31, cloud_of(style=cmap({"render": text("classification"), "hidden": array([uint(7), uint(2)]), "size": f64(2.0)})))
    files["pointcloud-hidden-empty.kcad"] = in_schema(31, cloud_of(style=cmap({"render": text("classification"), "hidden": array([]), "size": f64(2.0)})))
    files["pointcloud-invert-false.kcad"] = in_schema(31, cloud_of(style=cmap({"render": text("rgb"), "invert": boolean(False), "size": f64(2.0)})))
    files["pointcloud-px-written.kcad"] = in_schema(31, cloud_of(style=cmap({"render": text("rgb"), "size": f64(2.0), "sizeUnit": text("px")})))
    files["pointcloud-round-written.kcad"] = in_schema(31, cloud_of(style=cmap({"render": text("rgb"), "size": f64(2.0), "shape": text("round")})))
    files["pointcloud-opacity-low.kcad"] = in_schema(31, cloud_of(opacity=f64(0.05)))
    # A text's curve is schema 25's (docs/adr/0196 §1): in schema 24 an unknown field; a vertex at least, as many
    # bulges as vertices, some length, a text of one line, not linked; only `pts` and `bulges` in it.
    def curve(pts, bulges=None, **extra):
        body = {"pts": array([point({"x": x, "y": y}) for x, y in pts]), **({"bulges": array([f64(b) for b in bulges])} if bulges is not None else {}), **extra}
        return cmap(body)

    files["text-path-in-schema-24.kcad"] = in_schema(24, text_of(path=curve([(20.0, 0.0)])))
    files["text-path-empty.kcad"] = in_schema(25, text_of(path=curve([])))
    files["text-path-bulges-count.kcad"] = in_schema(25, text_of(path=curve([(10.0, 0.0), (20.0, 2.0)], [0.2])))
    files["text-path-zero-length.kcad"] = in_schema(25, text_of(path=curve([(0.0, 0.0)])))
    files["text-path-line-break.kcad"] = in_schema(25, text_of(text=text("R-12\nAda"), path=curve([(20.0, 0.0)])))
    files["text-path-box.kcad"] = in_schema(25, text_of(boxWidth=f64(10.0), path=curve([(20.0, 0.0)])))
    files["text-path-linked.kcad"] = in_schema(25, text_of(labelOf=blob(uid_bytes("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001")), labelScale=f64(500.0), path=curve([(20.0, 0.0)])))
    files["text-path-unknown-field.kcad"] = in_schema(25, text_of(path=curve([(20.0, 0.0)], turn=f64(1.0))))
    files["text-path-without-pts.kcad"] = in_schema(25, text_of(path=cmap({"bulges": array([f64(0.1)])})))
    files["table-source-file-objects.kcad"] = in_schema(22, table_of(source=cmap({"kind": text("file"), "name": text("a.csv"), "objects": array([blob(uid_bytes(m["uids"][0]))])})))
    files["table-bold-without-font.kcad"] = in_schema(22, table_of(bold=boolean(True)))
    files["attribute-width-factor-negative.kcad"] = with_blocks([block(1, "Rögar", attributes=array([cmap({"p": point({"x": 0.0, "y": 0.0}), "tag": text("NO"), "height": f64(0.5), "rotation": f64(0.0), "widthFactor": f64(-1.0)})]))], version=7)
    # A leader is schema 8's (docs/adr/0146): in schema 7 an unknown kind; two vertices or more, a positive height,
    # a note that is not empty, an arrowhead by one of its names (the filled arrow has none), `mask` only when true.
    def leader_of(**extra):
        tip = point(one["p"])
        end = point({"x": one["p"]["x"] + 8.0, "y": one["p"]["y"] + 6.0})
        return cmap({"leader": cmap({**common, "pts": array([tip, end]), "text": text("Mevcut bina"), "height": f64(2.5), "rotation": f64(0.0), **extra})})

    files["leader-in-schema-7.kcad"] = in_schema(7, leader_of())
    files["leader-one-vertex.kcad"] = in_schema(8, leader_of(pts=array([point(one["p"])])))
    files["leader-zero-height.kcad"] = in_schema(8, leader_of(height=f64(0.0)))
    files["leader-empty-note.kcad"] = in_schema(8, leader_of(text=text("")))
    files["leader-filled-arrow.kcad"] = in_schema(8, leader_of(arrow=text("filled")))
    files["leader-mask-false.kcad"] = in_schema(8, leader_of(mask=boolean(False)))
    # The new dimensions are schema 9's (docs/adr/0147): in schema 8 the kinds are unknown values and the fields
    # unknown fields; an arc length and a jogged radius have their centre, a slope its two elevations and nothing
    # else has them; an ordinate's axis is 0 or 90; `mask` is written only when true.
    def dimension_of(**extra):
        a = point(one["p"])
        b = point({"x": one["p"]["x"] + 10.0, "y": one["p"]["y"]})
        return cmap({"dimension": cmap({**common, "a": a, "b": b, "offset": f64(2.0), "height": f64(2.5), **extra})})

    centre = point({"x": one["p"]["x"] + 5.0, "y": one["p"]["y"] - 5.0})
    files["dimension-ordinate-in-schema-8.kcad"] = in_schema(8, dimension_of(style=text("ordinate"), angle=f64(0.0)))
    files["dimension-mask-in-schema-8.kcad"] = in_schema(8, dimension_of(mask=boolean(True)))
    files["dimension-arc-length-without-centre.kcad"] = in_schema(9, dimension_of(style=text("arcLength")))
    files["dimension-jogged-without-centre.kcad"] = in_schema(9, dimension_of(style=text("jogged")))
    files["dimension-slope-without-elevations.kcad"] = in_schema(9, dimension_of(style=text("slope"), za=f64(105.0)))
    files["dimension-elevations-off-slope.kcad"] = in_schema(9, dimension_of(style=text("arcLength"), c=centre, za=f64(105.0)))
    files["dimension-ordinate-axis.kcad"] = in_schema(9, dimension_of(style=text("ordinate"), angle=f64(45.0)))
    files["dimension-mask-false.kcad"] = in_schema(9, dimension_of(mask=boolean(False)))
    files["unknown-kind.kcad"] = container(with_parts({**parts, "entities": array([cmap({"block": cmap({**body, "uid": blob(uid_bytes(m["uids"][0]))})})])}))
    files["two-kinds.kcad"] = container(with_parts({**parts, "entities": array([cmap({"line": cmap({}), "point": cmap({**body, "uid": blob(uid_bytes(m["uids"][0]))})})])}))
    files["point-three-numbers.kcad"] = container(with_parts({**parts, "origin": array([f64(1.0), f64(2.0), f64(3.0)])}))
    top = m["layers"][0]
    nulled = cmap(
        {
            "id": text(top["id"]),
            "name": text(top["name"]),
            "type": text(top["type"]),
            "visible": boolean(top["visible"]),
            "locked": boolean(top["locked"]),
            "expanded": boolean(top["expanded"]),
            "style": cmap({**layer_style_parts(top["style"]), "renderer": b"\xf6"}),
            "children": array([]),
        }
    )
    files["null-renderer.kcad"] = container(with_parts({**parts, "layers": array([nulled])}))
    # A layer's own snapping is schema 10's (docs/adr/0163 §4): in schema 9 an unknown field; on a group, `off: false`,
    # both or neither, an empty list, a repeated or an unknown kind (quadrant comes with endpoint) a bad value.
    def snapped(snap, kind="layer"):
        node = {
            "id": text(top["id"]),
            "name": text(top["name"]),
            "snap": snap,
            "type": text(kind),
            "visible": boolean(top["visible"]),
            "locked": boolean(top["locked"]),
            "expanded": boolean(top["expanded"]),
            "style": cmap(layer_style_parts(top["style"])),
            "children": array([]),
        }
        return cmap({**parts, "layers": array([cmap(node)])})

    off = cmap({"off": boolean(True)})
    files["layer-snap-in-schema-9.kcad"] = container(root(snapped(off), version=uint(9)))
    files["layer-snap-on-group.kcad"] = container(root(snapped(off, "group"), version=uint(10)))
    files["layer-snap-off-false.kcad"] = container(root(snapped(cmap({"off": boolean(False)})), version=uint(10)))
    files["layer-snap-both.kcad"] = container(root(snapped(cmap({"off": boolean(True), "kinds": array([text("endpoint")])})), version=uint(10)))
    files["layer-snap-empty.kcad"] = container(root(snapped(cmap({})), version=uint(10)))
    files["layer-snap-empty-kinds.kcad"] = container(root(snapped(cmap({"kinds": array([])})), version=uint(10)))
    files["layer-snap-unknown-kind.kcad"] = container(root(snapped(cmap({"kinds": array([text("quadrant")])})), version=uint(10)))
    files["layer-snap-repeated-kind.kcad"] = container(root(snapped(cmap({"kinds": array([text("endpoint"), text("endpoint")])})), version=uint(10)))
    # A layer's fields are schema 26's (docs/adr/0199 §1): in schema 25 an unknown field; on a group, an empty list, a
    # name twice (folded), a default that is not the canonical text, `required: false`, an unknown kind, a fraction
    # digit count on a text, a negative length a bad value; an unknown key, a field without its kind, a value without
    # its label refused as such.
    def with_fields(field_list, kind="layer", version=26):
        node = {
            "id": text(top["id"]),
            "name": text(top["name"]),
            "fields": array(field_list),
            "type": text(kind),
            "visible": boolean(top["visible"]),
            "locked": boolean(top["locked"]),
            "expanded": boolean(top["expanded"]),
            "style": cmap(layer_style_parts(top["style"])),
            "children": array([]),
        }
        return container(root(cmap({**parts, "layers": array([cmap(node)])}), version=uint(version)))

    def field(name="Ada", kind="text", **extra):
        return cmap({"name": text(name), "kind": text(kind), **extra})

    files["layer-fields-in-schema-25.kcad"] = with_fields([field()], version=25)
    files["layer-fields-on-group.kcad"] = with_fields([field()], kind="group")
    files["layer-fields-empty.kcad"] = with_fields([])
    files["layer-fields-repeated-name.kcad"] = with_fields([field("İl"), field("il")])
    files["layer-fields-bad-default.kcad"] = with_fields([field("Kat", "integer", default=text("03"))])
    files["layer-fields-required-false.kcad"] = with_fields([field(required=boolean(False))])
    files["layer-fields-unknown-kind.kcad"] = with_fields([field(kind="time")])
    files["layer-fields-scale-on-text.kcad"] = with_fields([field(scale=uint(2))])
    files["layer-fields-negative-length.kcad"] = with_fields([field(length=integer(-1))])
    files["layer-fields-unknown-field.kcad"] = with_fields([field(unit=text("m"))])
    files["layer-fields-without-kind.kcad"] = with_fields([cmap({"name": text("Ada")})])
    files["layer-fields-choice-without-label.kcad"] = with_fields([field(values=array([cmap({"code": text("K")})]))])

    # A layer's map service, where its objects came from and the project's connections are schema 32's
    # (docs/adr/0208 §2, §10): in schema 31 unknown fields; a service or a source on a group, both on one layer, an
    # object on a service layer, a connection named that the project has not, and every rule of the contract's
    # `problem`s broken one at a time; flags written false and empty lists refused as such.
    osm = {"kind": text("xyz"), "url": text("https://tile.openstreetmap.org/{z}/{x}/{y}.png")}

    def on_layer(extra, node_type="layer", schema=32, settings_extra=None, entities=None):
        node = {
            "id": text(top["id"]),
            "name": text(top["name"]),
            **extra,
            "type": text(node_type),
            "visible": boolean(top["visible"]),
            "locked": boolean(top["locked"]),
            "expanded": boolean(top["expanded"]),
            "style": cmap(layer_style_parts(top["style"])),
            "children": array([]),
        }
        body = {**parts, "layers": array([cmap(node)])}
        body["entities"] = array([]) if entities is None else entities
        if settings_extra:
            body["settings"] = cmap({**settings_parts(m["settings"]), **settings_extra})
        return container(root(cmap(body), version=uint(schema)))

    def served(schema=32, node_type="layer", entities=None, **fields_):
        return on_layer({"service": cmap({**osm, **fields_})}, node_type=node_type, schema=schema, entities=entities)

    def fed(schema=32, node_type="layer", **fields_):
        return on_layer({"feed": cmap({"kind": text("wfs"), "url": text("https://cbs.example.com/wfs"), "name": text("tkgm:parsel"), **fields_})}, node_type=node_type, schema=schema)

    def connected(conns, schema=32, **service_fields):
        extra = {"service": cmap({**osm, **service_fields})} if service_fields else {}
        return on_layer(extra, schema=schema, settings_extra={"connections": array(conns)})

    def conn(cid="hgm", auth="query", **extra):
        if auth in ("query", "header") and "names" not in extra:
            extra["names"] = array([text("apikey")])
        return cmap({"id": text(cid), "name": text("HGM ATLAS"), "origin": text("https://atlas.harita.gov.tr"), "auth": text(auth), **extra})

    grid = lambda **m_: cmap({"srid": uint(5256), "matrices": array([cmap({"id": text("0"), "resolution": f64(128.0), "x0": f64(200000.0), "y0": f64(4700000.0), "tileWidth": uint(256), "tileHeight": uint(256), "matrixWidth": uint(20), "matrixHeight": uint(12), **m_})])})
    files["service-in-schema-31.kcad"] = served(schema=31)
    files["feed-in-schema-31.kcad"] = fed(schema=31)
    files["connections-in-schema-31.kcad"] = connected([conn()], schema=31)
    files["service-on-group.kcad"] = served(node_type="group")
    files["feed-on-group.kcad"] = fed(node_type="group")
    files["service-and-feed.kcad"] = on_layer({"service": cmap(osm), "feed": cmap({"kind": text("geojson"), "url": text("https://data.example.com/a.geojson")})})
    point_on = cmap({"point": cmap({"uid": blob(uid_bytes(m["uids"][0])), "attrs": cmap({}), "layerId": text(top["id"]), "p": point(m["entities"][0]["p"])})})
    files["service-holds-object.kcad"] = served(entities=array([point_on]))
    files["service-unknown-connection.kcad"] = served(connection=text("hgm"))
    files["service-unknown-kind.kcad"] = served(kind=text("tms"))
    files["service-unknown-field.kcad"] = served(apiKey=text("gizli"))
    files["service-without-url.kcad"] = on_layer({"service": cmap({"kind": text("xyz")})})
    files["service-url-ftp.kcad"] = served(url=text("ftp://tile.example.com/{z}/{x}/{y}.png"))
    files["service-xyz-no-y.kcad"] = served(url=text("https://tile.example.com/{z}/{x}.png"))
    files["service-xyz-subdomains-missing.kcad"] = served(url=text("https://{s}.tile.example.com/{z}/{x}/{y}.png"))
    files["service-wms-no-layers.kcad"] = served(kind=text("wms"), url=text("https://cbs.example.com/wms"), srid=uint(5256))
    files["service-wms-no-srid.kcad"] = served(kind=text("wms"), url=text("https://cbs.example.com/wms"), layers=array([text("imar")]))
    files["service-wms-version.kcad"] = served(kind=text("wms"), url=text("https://cbs.example.com/wms"), layers=array([text("imar")]), srid=uint(5256), version=text("1.2.0"))
    files["service-wmts-no-grid.kcad"] = served(kind=text("wmts"), url=text("https://cbs.example.com/wmts"), layers=array([text("orto")]), matrixSet=text("TM33"))
    files["service-wmts-two-layers.kcad"] = served(kind=text("wmts"), url=text("https://cbs.example.com/wmts"), layers=array([text("orto"), text("yol")]), matrixSet=text("TM33"), grid=grid())
    files["service-grid-empty.kcad"] = served(kind=text("wmts"), url=text("https://cbs.example.com/wmts"), layers=array([text("orto")]), matrixSet=text("TM33"), grid=cmap({"srid": uint(5256), "matrices": array([])}))
    files["service-grid-zero-resolution.kcad"] = served(kind=text("wmts"), url=text("https://cbs.example.com/wmts"), layers=array([text("orto")]), matrixSet=text("TM33"), grid=grid(resolution=f64(0.0)))
    files["service-grid-big-tile.kcad"] = served(kind=text("wmts"), url=text("https://cbs.example.com/wmts"), layers=array([text("orto")]), matrixSet=text("TM33"), grid=grid(tileWidth=uint(8192)))
    files["service-google-url.kcad"] = served(kind=text("google"), style=text("satellite"), connection=text("google"))
    files["service-google-map-type.kcad"] = served(kind=text("google"), url=text(""), style=text("streets"))
    files["service-layers-empty.kcad"] = served(layers=array([]))
    files["service-params-empty.kcad"] = served(params=array([]))
    files["service-param-name.kcad"] = served(params=array([cmap({"name": text("a b"), "value": text("1")})]))
    files["service-yflip-false.kcad"] = served(yFlip=boolean(False))
    files["service-zoom-order.kcad"] = served(minZoom=uint(18), maxZoom=uint(12))
    files["service-zoom-big.kcad"] = served(maxZoom=uint(31))
    files["service-tile-size-small.kcad"] = served(tileSize=uint(32))
    files["service-opacity-zero.kcad"] = served(opacity=f64(0.0))
    # The extent's west east of its east, and three numbers.
    files["service-bbox-order.kcad"] = served(bbox=array([f64(v) for v in (44.9, 35.8, 25.6, 42.2)]))
    files["service-bbox-three.kcad"] = served(bbox=array([f64(v) for v in (25.6, 35.8, 44.9)]))
    files["service-style-empty.kcad"] = served(style=text(""))
    files["feed-no-name.kcad"] = on_layer({"feed": cmap({"kind": text("wfs"), "url": text("https://cbs.example.com/wfs")})})
    files["feed-bbox-three.kcad"] = fed(bbox=floats([1.0, 2.0, 3.0]))
    files["feed-bbox-order.kcad"] = fed(bbox=floats([10.0, 0.0, 0.0, 10.0]))
    files["feed-version-geojson.kcad"] = on_layer({"feed": cmap({"kind": text("geojson"), "url": text("https://data.example.com/a.geojson"), "version": text("2.0.0")})})
    files["feed-limit-zero.kcad"] = fed(limit=uint(0))
    files["feed-unknown-connection.kcad"] = fed(connection=text("kurum"))
    files["connections-empty.kcad"] = connected([])
    files["connection-origin-path.kcad"] = connected([conn(origin=text("https://atlas.harita.gov.tr/wms"))])
    files["connection-origin-upper.kcad"] = connected([conn(origin=text("https://Atlas.harita.gov.tr"))])
    files["connection-duplicate.kcad"] = connected([conn(), conn()])
    files["connection-query-no-names.kcad"] = connected([conn(names=array([]))])
    files["connection-basic-names.kcad"] = connected([conn(auth="basic", names=array([text("user")]))])
    files["connection-oauth2-no-token-url.kcad"] = connected([conn(auth="oauth2")])
    files["connection-scope-not-oauth2.kcad"] = connected([conn(auth="bearer", scope=text("read"))])
    files["connection-secret.kcad"] = connected([conn(auth="basic", password=text("gizli"))])

    # The project's variables are schema 37's (docs/adr/0214 §2.3): in schema 36 an unknown field; every rule of the
    # contract's `variables_problem` broken one at a time; an empty list and an unknown field refused as such.
    def var(**fields_):
        base = {"kind": text("text"), "name": text("is_no")}
        base.update(fields_)
        return cmap(base)

    def with_variables(list_, schema=37):
        return on_layer({}, schema=schema, settings_extra={"variables": array(list_)})

    files["variables-in-schema-36.kcad"] = with_variables([var()], schema=36)
    files["variables-empty.kcad"] = with_variables([])
    files["variable-duplicate.kcad"] = with_variables([var(), var(name=text("IS_NO"))])
    files["variable-bad-name.kcad"] = with_variables([var(name=text("1a"))])
    files["variable-builtin-name.kcad"] = with_variables([var(name=text("Proje_Adı"))])
    files["variable-number-text.kcad"] = with_variables([var(kind=text("number"), value=text("12"))])
    files["variable-bad-date.kcad"] = with_variables([var(kind=text("date"), value=text("2026-02-29"))])
    files["variable-unknown-kind.kcad"] = with_variables([var(kind=text("list"))])
    files["variable-unknown-field.kcad"] = with_variables([var(note=text("x"))])
    files["variable-no-kind.kcad"] = with_variables([cmap({"name": text("is_no")})])

    # The project's networks are schema 33's (docs/adr/0209 §2): in schema 32 an unknown field; every rule of the
    # contract's `networks_problem` broken one at a time; an empty list and a direction's field or values with another
    # kind than a field refused as such.
    def net(**fields_):
        base = {"id": text("ag-1"), "name": text("Yollar"), "kind": text("road"), "edges": array([cmap({"layer": text(top["id"])})]),
                "connect": text("ends"), "tolerance": f64(0.01), "direction": cmap({"kind": text("both")})}
        base.update(fields_)
        return cmap(base)

    def networked(nets, schema=33):
        return on_layer({}, schema=schema, settings_extra={"networks": array(nets)})

    def by_field(**lists):
        return cmap({"kind": text("field"), "field": text("yon"), **lists})

    def cost(name="Süre", kind="speed", **extra):
        if kind == "speed" and "speed" not in extra:
            extra["speed"] = f64(50.0)
        return cmap({"name": text(name), "kind": text(kind), "field": text("hiz"), **extra})

    files["networks-in-schema-32.kcad"] = networked([net()], schema=32)
    files["networks-empty.kcad"] = networked([])
    files["network-duplicate-id.kcad"] = networked([net(), net(name=text("Yollar 2"))])
    files["network-duplicate-name.kcad"] = networked([net(), net(id=text("ag-2"), name=text("YOLLAR"))])
    files["network-bad-id.kcad"] = networked([net(id=text("Ağ 1"))])
    files["network-name-untrimmed.kcad"] = networked([net(name=text(" Yollar"))])
    files["network-no-edges.kcad"] = networked([net(edges=array([]))])
    files["network-edge-twice.kcad"] = networked([net(edges=array([cmap({"layer": text("yol")}), cmap({"layer": text("yol")})]))])
    files["network-tolerance.kcad"] = networked([net(tolerance=f64(20.0))])
    files["network-unknown-kind.kcad"] = networked([net(kind=text("rail"))])
    files["network-unknown-field.kcad"] = networked([net(speed=f64(50.0))])
    files["network-junctions-empty.kcad"] = networked([net(junctions=array([]))])
    files["network-junction-role.kcad"] = networked([net(junctions=array([cmap({"layer": text("vana"), "role": text("pump")})]))])
    files["network-direction-no-values.kcad"] = networked([net(direction=by_field())])
    files["network-direction-value-twice.kcad"] = networked([net(direction=by_field(forward=array([text("FT")]), closed=array([text("ft")])))])
    files["network-direction-list-empty.kcad"] = networked([net(direction=by_field(forward=array([text("FT")]), backward=array([])))])
    files["network-direction-field-on-both.kcad"] = networked([net(direction=cmap({"kind": text("both"), "field": text("yon")}))])
    files["network-cost-length-name.kcad"] = networked([net(costs=array([cost(name="uzunluk")]))])
    files["network-cost-twice.kcad"] = networked([net(costs=array([cost(), cost(name="SÜRE")]))])
    files["network-cost-speed-unit.kcad"] = networked([net(costs=array([cost(unit=text("dk"))]))])
    files["network-cost-speed-zero.kcad"] = networked([net(costs=array([cost(speed=f64(0.0))]))])
    files["network-cost-field-speed.kcad"] = networked([net(costs=array([cost(name="Ücret", kind="field", speed=f64(5.0))]))])
    files["network-costs-empty.kcad"] = networked([net(costs=array([]))])
    files["network-closed-blank.kcad"] = networked([net(closed=text("  "))])

    # The project's address data is schema 42's (docs/adr/0229 §2): in schema 41 an unknown field; every rule of the
    # contract's `address_locators_problem` broken one at a time; an empty list, an empty area list and a field's empty
    # name refused as such.
    def locator(**fields_):
        base = {"id": text("adres-1"), "name": text("Numarataj"), "minScore": uint(85), "reach": f64(100.0),
                "doors": cmap({"layer": text(top["id"]), "street": text("YOL"), "door": text("KAPI")})}
        base.update({k: v for k, v in fields_.items() if v is not None})
        for k in [k for k, v in fields_.items() if v is None]:
            base.pop(k)
        return cmap(base)

    def addressed(list_, schema=42):
        return on_layer({}, schema=schema, settings_extra={"addressLocators": array(list_)})

    def area_(level="neighbourhood", **extra):
        return cmap({"layer": text("mahalle"), "level": text(level), "name": text("AD"), **extra})

    # The project's sheet book is schema 44's (docs/adr/0251): in schema 43 an unknown field; the book a map and
    # required; a picture's bytes held to its 32-byte SHA-256, none given twice; an empty picture list refused; a field
    # neither the book's map nor a picture has, unknown.
    logo = b"\x89PNG\r\n\x1a\n KentOS"
    logo_sha = hashlib.sha256(logo).digest()

    def picture_(raw=logo, sha=None, **extra):
        return cmap({"bytes": blob(raw), "sha256": blob(hashlib.sha256(raw).digest() if sha is None else sha), **extra})

    def sheeted(sheets_map, schema=44):
        return container(root(cmap({**parts, "sheets": sheets_map}), version=uint(schema)))

    book_ = opaque({"schema": "kentos.sheet/1", "sheets": []})
    files["sheets-in-schema-43.kcad"] = sheeted(cmap({"book": book_}), schema=43)
    files["sheets-book-not-a-map.kcad"] = sheeted(cmap({"book": opaque(["kentos.sheet/1"])}))
    files["sheets-book-missing.kcad"] = sheeted(cmap({"pictures": array([picture_()])}))
    files["sheets-unknown-field.kcad"] = sheeted(cmap({"book": book_, "assets": array([])}))
    files["sheets-pictures-empty.kcad"] = sheeted(cmap({"book": book_, "pictures": array([])}))
    files["sheets-picture-digest.kcad"] = sheeted(cmap({"book": book_, "pictures": array([picture_(sha=hashlib.sha256(b"baska").digest())])}))
    files["sheets-picture-twice.kcad"] = sheeted(cmap({"book": book_, "pictures": array([picture_(), picture_()])}))
    files["sheets-picture-sha256-length.kcad"] = sheeted(cmap({"book": book_, "pictures": array([picture_(sha=logo_sha[:31])])}))
    files["sheets-picture-unknown-field.kcad"] = sheeted(cmap({"book": book_, "pictures": array([picture_(name=text("logo.png"))])}))

    # The request ledger is schema 45's (docs/adr/0253 §6): in schema 44 an unknown field; not empty; each request's
    # fields its own, all of them but the answer required; the id by its rule and once; the fingerprint 16 bytes; the
    # status done or undone; the revision decimal digits; the version from 1.
    def request_(**over):
        base = {
            "status": text("done"),
            "command": text("cad.entities.delete"),
            "version": uint(1),
            "revision": text("3"),
            "requestId": text("istek-1"),
            "fingerprint": blob(bytes(range(16))),
        }
        return cmap({k: v for k, v in {**base, **over}.items() if v is not None})

    def requested(entries, schema=45):
        return container(root(cmap({**parts, "requests": array(entries)}), version=uint(schema)))

    files["requests-in-schema-44.kcad"] = requested([request_()], schema=44)
    files["requests-empty.kcad"] = requested([])
    files["requests-unknown-field.kcad"] = requested([request_(note=text("?"))])
    files["requests-missing-field.kcad"] = requested([request_(fingerprint=None)])
    files["requests-fingerprint-length.kcad"] = requested([request_(fingerprint=blob(bytes(15)))])
    files["requests-status.kcad"] = requested([request_(status=text("lost"))])
    files["requests-twice.kcad"] = requested([request_(), request_(revision=text("4"))])
    files["requests-id.kcad"] = requested([request_(requestId=text("boşluk var"))])
    files["requests-revision.kcad"] = requested([request_(revision=text("x1"))])
    files["requests-version-zero.kcad"] = requested([request_(version=uint(0))])
    files["addresses-in-schema-41.kcad"] = addressed([locator()], schema=41)
    files["addresses-empty.kcad"] = addressed([])
    files["address-duplicate-id.kcad"] = addressed([locator(), locator(name=text("Numarataj 2"))])
    files["address-duplicate-name.kcad"] = addressed([locator(), locator(id=text("adres-2"), name=text("NUMARATAJ"))])
    files["address-bad-id.kcad"] = addressed([locator(id=text("Adres 1"))])
    files["address-name-untrimmed.kcad"] = addressed([locator(name=text("Numarataj "))])
    files["address-no-source.kcad"] = addressed([locator(doors=None)])
    files["address-door-field-empty.kcad"] = addressed([locator(doors=cmap({"layer": text(top["id"]), "street": text("YOL"), "door": text("")}))])
    files["address-door-field-untrimmed.kcad"] = addressed([locator(doors=cmap({"layer": text(top["id"]), "street": text(" YOL"), "door": text("KAPI")}))])
    files["address-door-without-street.kcad"] = addressed([locator(doors=cmap({"layer": text(top["id"]), "door": text("KAPI")}))])
    files["address-areas-empty.kcad"] = addressed([locator(areas=array([]))])
    files["address-area-level-twice.kcad"] = addressed([locator(areas=array([area_(), area_()]))])
    files["address-area-unknown-level.kcad"] = addressed([locator(areas=array([area_(level="village")]))])
    files["address-district-area-district.kcad"] = addressed([locator(areas=array([area_(level="district", district=text("ILCE"))]))])
    files["address-min-score.kcad"] = addressed([locator(minScore=uint(40))])
    files["address-reach-zero.kcad"] = addressed([locator(reach=f64(0.0))])
    files["address-reach-far.kcad"] = addressed([locator(reach=f64(20000.0))])
    files["address-unknown-field.kcad"] = addressed([locator(note=text("x"))])
    files["address-no-min-score.kcad"] = addressed([locator(minScore=None)])

    # A layer's time setting, a group's scenario and a scenario layer's base layer are schema 34's (docs/adr/0210 §2):
    # in schema 33 unknown fields; every rule of the contract's `LayerTime::problem`, `ScenarioInfo::problem` and
    # `scenarios_problem` broken one at a time.
    def timed(**f):
        base = {"start": text("baslangic"), "end": text("bitis"), **f}
        return cmap({k: v for k, v in base.items() if v is not None})

    files["time-in-schema-33.kcad"] = on_layer({"time": timed()}, schema=33)
    files["time-on-group.kcad"] = on_layer({"time": timed()}, node_type="group", schema=34)
    files["time-start-blank.kcad"] = on_layer({"time": timed(start=text(" baslangic"))}, schema=34)
    files["time-start-missing.kcad"] = on_layer({"time": cmap({"end": text("bitis")})}, schema=34)
    files["time-same-fields.kcad"] = on_layer({"time": timed(end=text("baslangic"))}, schema=34)
    files["time-cumulative-false.kcad"] = on_layer({"time": timed(cumulative=boolean(False))}, schema=34)
    files["time-unknown-field.kcad"] = on_layer({"time": timed(format=text("ISO"))}, schema=34)
    files["time-field-long.kcad"] = on_layer({"time": timed(key=text("k" * 65))}, schema=34)
    files["time-on-service-layer.kcad"] = on_layer({"time": timed(), "service": cmap(osm)}, schema=34)
    files["scenario-on-layer.kcad"] = on_layer({"scenario": cmap({})}, schema=34)
    files["scenario-note-blank.kcad"] = on_layer({"scenario": cmap({"note": text("")})}, node_type="group", schema=34)
    files["replaces-outside-scenario.kcad"] = on_layer({"replaces": text("yol")}, schema=34)

    # A layer's filter is schema 35's (docs/adr/0211 §2): in schema 34 an unknown field; every rule of the contract's
    # `LayerFilter::problem` and `filters_problem` broken one at a time.
    one_id = blob(uid_bytes("0192f5a1-7777-7000-8000-000000000601"))

    def filtered_(**f):
        base = {"expression": text("Nitelik = 'Arsa'"), **f}
        return cmap({k: v for k, v in base.items() if v is not None})

    files["filter-in-schema-34.kcad"] = on_layer({"filter": filtered_()}, schema=34)
    # A layer's actions and card are schema 40's (docs/adr/0225 §4, §5): in schema 39 unknown fields; on a layer that
    # holds objects only; an empty list is not written, a card's switch only off; the contract's rules one at a time.
    act = lambda name="Parsel sorgu", target="https://parselsorgu.example/{Ada}/{Parsel}": cmap({"name": text(name), "target": text(target)})
    files["actions-in-schema-39.kcad"] = on_layer({"actions": array([act()])}, schema=39)
    files["actions-on-group.kcad"] = on_layer({"actions": array([act()])}, node_type="group", schema=40)
    files["actions-on-service-layer.kcad"] = on_layer({"actions": array([act()]), "service": cmap(osm)}, schema=40)
    files["actions-empty.kcad"] = on_layer({"actions": array([])}, schema=40)
    files["action-name-twice.kcad"] = on_layer({"actions": array([act(), act(name="PARSEL SORGU")])}, schema=40)
    files["action-target-blank.kcad"] = on_layer({"actions": array([act(target=" ")])}, schema=40)
    files["action-without-target.kcad"] = on_layer({"actions": array([cmap({"name": text("A")})])}, schema=40)
    files["card-in-schema-39.kcad"] = on_layer({"card": cmap({"title": text("Parsel {Ada}")})}, schema=39)
    files["card-on-group.kcad"] = on_layer({"card": cmap({"title": text("Parsel {Ada}")})}, node_type="group", schema=40)
    files["card-switch-on.kcad"] = on_layer({"card": cmap({"attachments": boolean(True)})}, schema=40)
    files["card-fields-empty.kcad"] = on_layer({"card": cmap({"fields": array([])})}, schema=40)
    files["card-fields-twice.kcad"] = on_layer({"card": cmap({"fields": array([text("Malik"), text("Malik")])})}, schema=40)
    files["card-title-line-break.kcad"] = on_layer({"card": cmap({"title": text("Parsel\n{Ada}")})}, schema=40)
    files["card-unknown-field.kcad"] = on_layer({"card": cmap({"color": text("#E5484D")})}, schema=40)
    # A layer's composition is schema 41's (docs/adr/0226 §3, §4): in schema 40 an unknown field; on a layer only; what
    # says nothing is not written (an opacity of 1, the normal mode, a mask's switch off); the contract's rules one at a
    # time. A target the tree lacks is skipped when drawn, not refused (the reader's tests read one).
    def composed_(compose, node_type="layer", schema=41):
        node = lambda nid, name, t, extra: cmap({
            "id": text(nid), "name": text(name), **extra, "type": text(t), "visible": boolean(True), "locked": boolean(False),
            "expanded": boolean(True), "style": cmap(layer_style_parts(top["style"])), "children": array([]),
        })
        tree = [node(top["id"], top["name"], node_type, {"compose": compose}), node("hat", "Hatlar", "layer", {})]
        return container(root(cmap({**parts, "layers": array(tree), "entities": array([])}), version=uint(schema)))

    def mask_(**f):
        base = {"labels": boolean(True), "size": f64(1.0), "targets": array([text("hat")]), **f}
        return cmap({k: v for k, v in base.items() if v is not None})

    files["compose-in-schema-40.kcad"] = composed_(cmap({"opacity": f64(0.5)}), schema=40)
    files["compose-on-group.kcad"] = composed_(cmap({"opacity": f64(0.5)}), node_type="group")
    files["compose-empty.kcad"] = composed_(cmap({}))
    files["compose-opacity-one.kcad"] = composed_(cmap({"opacity": f64(1.0)}))
    files["compose-opacity-above-one.kcad"] = composed_(cmap({"opacity": f64(1.5)}))
    files["compose-blend-normal.kcad"] = composed_(cmap({"blend": text("normal")}))
    files["compose-blend-unknown.kcad"] = composed_(cmap({"blend": text("dissolve")}))
    files["compose-unknown-field.kcad"] = composed_(cmap({"opacity": f64(0.5), "shadow": boolean(True)}))
    files["mask-without-source.kcad"] = composed_(cmap({"mask": mask_(labels=None)}))
    files["mask-switch-off.kcad"] = composed_(cmap({"mask": mask_(markers=boolean(False))}))
    files["mask-size-above-ten.kcad"] = composed_(cmap({"mask": mask_(size=f64(10.5))}))
    files["mask-without-targets.kcad"] = composed_(cmap({"mask": mask_(targets=None)}))
    files["mask-targets-empty.kcad"] = composed_(cmap({"mask": mask_(targets=array([]))}))
    files["mask-target-twice.kcad"] = composed_(cmap({"mask": mask_(targets=array([text("hat"), text("hat")]))}))
    files["filter-on-group.kcad"] = on_layer({"filter": filtered_()}, node_type="group", schema=35)
    files["filter-on-service-layer.kcad"] = on_layer({"filter": filtered_(), "service": cmap(osm)}, schema=35)
    files["filter-empty.kcad"] = on_layer({"filter": cmap({})}, schema=35)
    files["filter-expression-blank.kcad"] = on_layer({"filter": filtered_(expression=text(" Nitelik = 'Arsa'"))}, schema=35)
    files["filter-expression-long.kcad"] = on_layer({"filter": filtered_(expression=text("a" * 10_001))}, schema=35)
    files["filter-objects-empty.kcad"] = on_layer({"filter": filtered_(objects=array([]))}, schema=35)
    files["filter-objects-twice.kcad"] = on_layer({"filter": filtered_(objects=array([one_id, one_id]))}, schema=35)
    files["filter-object-short.kcad"] = on_layer({"filter": filtered_(objects=array([blob(bytes(range(1, 16)))]))}, schema=35)
    files["filter-object-nil.kcad"] = on_layer({"filter": filtered_(objects=array([blob(bytes(16))]))}, schema=35)
    files["filter-unknown-field.kcad"] = on_layer({"filter": filtered_(scope=text("view"))}, schema=35)

    # The label engine is schema 36's (docs/adr/0212 §2): in schema 35 a layer style's `labels`, a label style's engine
    # fields and an object's `labelPins` are unknown fields; every rule of the contract's `style_problem`,
    # `layer_labels_problem`, `pins_problem` and `labels_problem` broken one at a time.
    def labelled_(labels=None, label=None, node_type="layer", schema=36, entities=None):
        style = {**layer_style_parts(top["style"])}
        if labels is not None:
            style["labels"] = labels
        if label is not None:
            style["label"] = label
        node = {
            "id": text(top["id"]),
            "name": text(top["name"]),
            "type": text(node_type),
            "visible": boolean(top["visible"]),
            "locked": boolean(top["locked"]),
            "expanded": boolean(top["expanded"]),
            "style": cmap(style),
            "children": array([]),
        }
        body = {**parts, "layers": array([cmap(node)])}
        body["entities"] = array([]) if entities is None else entities
        return container(root(cmap(body), version=uint(schema)))

    def rules_(*classes, **more):
        return cmap({"mode": text("rules"), "classes": array(list(classes)), **more})

    def class_(name="Ada", **style):
        st = {"placement": text("center"), "size": f64(10.0), **style}
        return cmap({"name": text(name), "style": cmap(st)})

    def style_(**engine):
        return cmap({"placement": text("center"), "size": f64(10.0), **engine})

    files["labels-in-schema-35.kcad"] = labelled_(cmap({"mode": text("off")}), schema=35)
    files["label-engine-in-schema-35.kcad"] = labelled_(label=style_(text=text("Ada")), schema=35)
    files["labels-on-group.kcad"] = labelled_(cmap({"mode": text("off")}), node_type="group")
    files["labels-rules-empty.kcad"] = labelled_(rules_())
    files["labels-rules-without-classes.kcad"] = labelled_(cmap({"mode": text("rules")}))
    files["labels-classes-in-single.kcad"] = labelled_(cmap({"mode": text("single"), "classes": array([class_()])}))
    files["labels-class-twice.kcad"] = labelled_(rules_(class_(), class_()))
    files["labels-class-name-blank.kcad"] = labelled_(rules_(class_(" Ada")))
    files["labels-class-when-blank.kcad"] = labelled_(rules_(cmap({"name": text("Ada"), "when": text(""), "style": style_()})))
    files["labels-mode-unknown.kcad"] = labelled_(cmap({"mode": text("blocking")}))
    files["labels-unknown-field.kcad"] = labelled_(cmap({"mode": text("off"), "z": uint(1)}))
    files["labels-obstacle-weight.kcad"] = labelled_(cmap({"mode": text("off"), "obstacle": cmap({"weight": uint(0)})}))
    files["label-size-out.kcad"] = labelled_(label=cmap({"placement": text("center"), "size": f64(0.5), "text": text("Ada")}))
    files["label-color-bad.kcad"] = labelled_(label=style_(color=text("kırmızı")))
    files["label-text-blank.kcad"] = labelled_(label=style_(text=text(" Ada")))
    files["label-priority-high.kcad"] = labelled_(label=style_(priority=uint(11)))
    files["label-shrink-out.kcad"] = labelled_(label=style_(shrink=f64(0.3)))
    files["label-stack-chars.kcad"] = labelled_(label=style_(stack=cmap({"mode": text("always"), "chars": uint(1)})))
    files["label-abbreviate-empty.kcad"] = labelled_(label=style_(abbreviate=cmap({"words": array([])})))
    files["label-abbreviate-twice.kcad"] = labelled_(label=style_(abbreviate=cmap({"words": array(
        [cmap({"word": text("Sokak"), "short": text("Sk.")}), cmap({"word": text("Sokak"), "short": text("S.")})])})))
    files["label-halo-wide.kcad"] = labelled_(label=style_(halo=cmap({"width": f64(12.0)})))
    files["label-unknown-engine-field.kcad"] = labelled_(label=style_(glow=f64(1.0)))
    pinned_ = lambda pins, schema=36: labelled_(cmap({"mode": text("off")}), schema=schema, entities=array([cmap({"point": cmap({
        "uid": blob(uid_bytes(m["uids"][0])), "attrs": cmap({}), "layerId": text(top["id"]), "p": point(m["entities"][0]["p"]),
        "labelPins": pins})})]))
    files["label-pins-in-schema-35.kcad"] = labelled_(None, schema=35, entities=array([cmap({"point": cmap({
        "uid": blob(uid_bytes(m["uids"][0])), "attrs": cmap({}), "layerId": text(top["id"]), "p": point(m["entities"][0]["p"]),
        "labelPins": array([cmap({"hidden": boolean(True)})])})})]))
    files["label-pins-empty.kcad"] = pinned_(array([]))
    files["label-pin-rotation-alone.kcad"] = pinned_(array([cmap({"rotation": f64(10.0), "hidden": boolean(True)})]))
    files["label-pin-hidden-false.kcad"] = pinned_(array([cmap({"at": point({"x": 1.0, "y": 2.0}), "hidden": boolean(False)})]))
    files["label-pin-nothing.kcad"] = pinned_(array([cmap({"class": text("Ada")})]))
    files["label-pin-class-twice.kcad"] = pinned_(array([cmap({"hidden": boolean(True)}), cmap({"hidden": boolean(True)})]))
    files["label-pin-far.kcad"] = pinned_(array([cmap({"at": point({"x": 2e7, "y": 0.0})})]))
    files["label-pin-unknown-field.kcad"] = pinned_(array([cmap({"hidden": boolean(True), "size": f64(2.0)})]))

    def node_(nid, node_type="layer", children=(), **extra):
        return cmap({"id": text(nid), "name": text(nid), **extra, "type": text(node_type), "visible": boolean(True),
                     "locked": boolean(False), "expanded": boolean(True), "style": cmap(layer_style_parts(top["style"])),
                     "children": array(list(children))})

    def treed(nodes):
        return container(root(cmap({**parts, "layers": array(nodes), "entities": array([]), "activeLayer": text("yol")}), version=uint(34)))

    def alternative(children, nid="alt-a"):
        return node_(nid, "group", children, scenario=cmap({}))

    files["scenario-nested.kcad"] = treed([alternative([alternative([], "alt-b")]), node_("yol")])
    files["replaces-on-group.kcad"] = treed([alternative([node_("grup", "group", replaces=text("yol"))]), node_("yol")])
    files["replaces-self.kcad"] = treed([alternative([node_("yol-a", replaces=text("yol-a"))]), node_("yol")])
    files["replaces-empty.kcad"] = treed([alternative([node_("yol-a", replaces=text(""))]), node_("yol")])
    files["replaces-scenario-layer.kcad"] = treed([alternative([node_("yol-a", replaces=text("yol-b")), node_("yol-b")]), node_("yol")])
    files["replaces-twice.kcad"] = treed([alternative([node_("yol-a", replaces=text("yol")), node_("yol-b", replaces=text("yol"))]), node_("yol")])
    files["replaces-group-target.kcad"] = treed([alternative([node_("yol-a", replaces=text("grup"))]), node_("grup", "group", [node_("yol")])])
    files["big-negative.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([b"\x3b" + b"\xff" * 8]), "categories": array([])})}))
    files["bytes-in-opaque.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([blob(b"\x01")]), "categories": array([])})}))
    files["bad-enum.kcad"] = container(with_parts({**parts, "settings": cmap({**settings_parts(m["settings"]), "areaUnit": text("acre")})}))
    # A local project's drawing unit is schema 11's (docs/adr/0165 §2): in schema 10 it is an unknown field; only
    # mm, cm and m are units.
    files["drawing-unit-in-schema-10.kcad"] = container(root(cmap({**parts, "settings": cmap({**settings_parts(m["settings"]), "drawingUnit": text("mm")})}), version=uint(10)))
    files["drawing-unit-unknown.kcad"] = container(root(cmap({**parts, "settings": cmap({**settings_parts(m["settings"]), "drawingUnit": text("km")})}), version=uint(11)))
    # A second coordinate system is schema 12's (docs/adr/0167 §1): in schema 11 it is an unknown field; it is another
    # system than the project's own, and a local project has none.
    with_second = lambda second, **more: cmap({**parts, "settings": cmap({**settings_parts({**m["settings"], **more}), "secondSrid": uint(second)})})
    files["second-srid-in-schema-11.kcad"] = container(root(with_second(2322), version=uint(11)))
    files["second-srid-zero.kcad"] = container(root(with_second(0), version=uint(12)))
    files["second-srid-same.kcad"] = container(root(with_second(m["settings"]["srid"]), version=uint(12)))
    files["second-srid-local.kcad"] = container(root(with_second(2322, srid=0), version=uint(12)))
    files["srid-range.kcad"] = container(with_parts({**parts, "settings": cmap({**settings_parts(m["settings"]), "srid": uint(1 << 32)})}))
    # The project's own systems and datum choices are schema 13's (docs/adr/0168): unknown fields in schema 12; a
    # definition of the project's own only without an EPSG code, a second one only instead of a second EPSG code and with
    # a system to be the second of; every definition and choice by its rules.
    tm = {"kind": text("tm"), "datum": text("TUREF"), "centralMeridian": f64(30.0), "scaleFactor": f64(1.0),
          "falseEasting": f64(200000.0), "falseNorthing": f64(0.0)}
    definition = lambda system, name="Tanım": cmap({"name": text(name), "system": cmap(system)})
    with_settings = lambda version, srid=0, **more: container(root(cmap({**parts, "settings": cmap(
        {**settings_parts({**m["settings"], "srid": srid}), **more})}), version=uint(version)))
    files["custom-crs-in-schema-12.kcad"] = with_settings(12, customCrs=definition(tm))
    files["custom-crs-with-srid.kcad"] = with_settings(13, srid=5254, customCrs=definition(tm))
    files["second-custom-with-second-srid.kcad"] = with_settings(13, srid=5254, secondSrid=uint(2320),
                                                                 secondCustomCrs=definition(tm))
    files["second-custom-without-system.kcad"] = with_settings(13, srid=0, secondCustomCrs=definition(tm))
    files["custom-crs-kind-unknown.kcad"] = with_settings(13, customCrs=definition({**tm, "kind": text("lambert")}))
    files["custom-crs-field-of-other-kind.kcad"] = with_settings(13, customCrs=definition(
        {**tm, "plane": cmap({"kind": text("similarity"), "east": f64(0.0), "north": f64(0.0), "rotation": f64(0.0),
                              "scale": f64(1.0)})}))
    files["custom-crs-two-datums.kcad"] = with_settings(13, customCrs=definition({**tm, "customDatum": cmap(
        {"name": text("D"), "ellipsoid": cmap({"name": text("E"), "semiMajor": f64(6378000.0),
                                                "inverseFlattening": f64(298.0)})})}))
    files["custom-crs-flat-ellipsoid.kcad"] = with_settings(13, customCrs=definition({**{k: v for k, v in tm.items() if k != "datum"},
        "customDatum": cmap({"name": text("D"), "ellipsoid": cmap({"name": text("E"), "semiMajor": f64(6378000.0),
                                                                    "inverseFlattening": f64(0.5)})})}))
    folded = cmap({"kind": text("affine"), **{k: f64(v) for k, v in zip("abcdef", (1.0, 2.0, 0.0, 2.0, 4.0, 0.0))}})
    files["custom-crs-folded-plane.kcad"] = with_settings(13, customCrs=definition(
        {"kind": text("local"), "base": cmap({"srid": uint(5254)}), "plane": folded}))
    local_base = cmap({"definition": definition({"kind": text("local"), "base": cmap({"srid": uint(5254)}),
                                                  "plane": cmap({"kind": text("similarity"), "east": f64(0.0), "north": f64(0.0),
                                                                 "rotation": f64(0.0), "scale": f64(1.0)})})})
    files["custom-crs-local-base.kcad"] = with_settings(13, customCrs=definition(
        {"kind": text("local"), "base": local_base, "plane": cmap({"kind": text("similarity"), "east": f64(0.0),
                                                                   "north": f64(0.0), "rotation": f64(0.0), "scale": f64(1.0)})}))
    files["custom-crs-rotation-two.kcad"] = with_settings(13, customCrs=definition({**{k: v for k, v in tm.items() if k != "datum"},
        "customDatum": cmap({"name": text("D"), "ellipsoid": cmap({"name": text("E"), "semiMajor": f64(6378000.0),
                                                                    "inverseFlattening": f64(298.0)}),
                             "toWgs84": cmap({"translation": array([f64(1.0)] * 3), "rotation": array([f64(0.0)] * 2),
                                              "scale": f64(0.0), "convention": text("positionVector")})})}))
    seven = cmap({"translation": array([f64(1.0)] * 3), "rotation": array([f64(0.0)] * 3), "scale": f64(0.0),
                  "convention": text("positionVector")})
    choice = lambda a, b, **more: cmap({"from": text(a), "to": text(b), "name": text("Seçim"), **more})
    files["datum-transform-same.kcad"] = with_settings(13, srid=5254, datumTransforms=array([choice("ED50", "ED50", helmert=seven)]))
    files["datum-transform-twice.kcad"] = with_settings(13, srid=5254, datumTransforms=array(
        [choice("ED50", "TUREF", helmert=seven), choice("TUREF", "ED50", helmert=seven)]))
    files["datum-transform-both.kcad"] = with_settings(13, srid=5254, datumTransforms=array([choice(
        "ED50", "TUREF", helmert=seven, grid=cmap({"id": text("a" * 64), "file": text("g.gsb"), "size": uint(10)}))]))
    files["datum-transform-grid-id.kcad"] = with_settings(13, srid=5254, datumTransforms=array([choice(
        "ED50", "TUREF", grid=cmap({"id": text("A" * 64), "file": text("g.gsb"), "size": uint(10)}))]))
    # The survey settings are schema 14's (docs/adr/0169 §3): an unknown field in schema 13; at least one, k within
    # [−1, 1], tolerances above zero, no other key.
    files["survey-in-schema-13.kcad"] = with_settings(13, survey=cmap({"refraction": f64(0.14)}))
    files["survey-empty.kcad"] = with_settings(14, survey=cmap({}))
    files["survey-refraction-range.kcad"] = with_settings(14, survey=cmap({"refraction": f64(1.5)}))
    files["survey-tolerance-zero.kcad"] = with_settings(14, survey=cmap({"faceHz": f64(0.0)}))
    files["survey-unknown-key.kcad"] = with_settings(14, survey=cmap({"closure": f64(0.01)}))
    # The traverse tolerances are schema 15's: unknown fields in schema 14, above zero.
    files["survey-traverse-in-schema-14.kcad"] = with_settings(14, survey=cmap({"twoWay": f64(0.01)}))
    files["survey-two-way-zero.kcad"] = with_settings(15, survey=cmap({"twoWay": f64(0.0)}))
    # The ground is schema 16's (docs/adr/0171): unknown fields in schema 15; a height within [−500, 9000] m, the
    # reduction to the grid only with one, a bool.
    files["survey-ground-in-schema-15.kcad"] = with_settings(15, survey=cmap({"groundHeight": f64(850.0)}))
    files["survey-ground-range.kcad"] = with_settings(16, survey=cmap({"groundHeight": f64(9500.0)}))
    files["survey-reduce-without-height.kcad"] = with_settings(16, survey=cmap({"reduceToGrid": b"\xf5"}))
    files["survey-reduce-not-bool.kcad"] = with_settings(16, survey=cmap({"groundHeight": f64(850.0), "reduceToGrid": uint(1)}))
    # The a priori standard deviations are schema 28's (docs/adr/0203 §1): unknown fields in schema 27; a deviation
    # above zero, the parts per million not below.
    files["survey-sigma-in-schema-27.kcad"] = with_settings(27, survey=cmap({"sigmaDirection": f64(1.5e-5)}))
    files["survey-sigma-zero.kcad"] = with_settings(28, survey=cmap({"sigmaDistance": f64(0.0)}))
    files["survey-sigma-ppm-negative.kcad"] = with_settings(28, survey=cmap({"sigmaPpm": f64(-1.0)}))
    # The layer states are schema 19's (docs/adr/0177 §4): an unknown field in schema 18; an id and a name, neither empty
    # nor twice; in a state, no node empty or twice; a visibility that is a bool.
    node = lambda n, **more: cmap({"node": text(n), "visible": b"\xf5", **more})
    state = lambda i, name, nodes=None: cmap({"id": text(i), "name": text(name), "nodes": array(nodes if nodes is not None else [node("0")])})
    files["layer-states-in-schema-18.kcad"] = with_settings(18, layerStates=array([state("a", "Görünüm")]))
    files["layer-states-empty-id.kcad"] = with_settings(19, layerStates=array([state("", "Görünüm")]))
    files["layer-states-same-id.kcad"] = with_settings(19, layerStates=array([state("a", "Görünüm"), state("a", "Baskı")]))
    files["layer-states-empty-name.kcad"] = with_settings(19, layerStates=array([state("a", "  ")]))
    files["layer-states-same-name.kcad"] = with_settings(19, layerStates=array([state("a", "Görünüm"), state("b", " Görünüm")]))
    files["layer-states-same-node.kcad"] = with_settings(19, layerStates=array([state("a", "Görünüm", [node("0"), node("0")])]))
    files["layer-states-visible-not-bool.kcad"] = with_settings(19, layerStates=array([state("a", "Görünüm", [cmap({"node": text("0"), "visible": uint(1)})])]))
    files["layer-states-without-visible.kcad"] = with_settings(19, layerStates=array([state("a", "Görünüm", [cmap({"node": text("0")})])]))
    # A state node's freeze and printing are schema 55's (docs/adr/0311 §3); its path only an exported file's.
    files["layer-states-frozen-in-schema-54.kcad"] = with_settings(54, layerStates=array([state("a", "Görünüm", [node("0", frozen=b"\xf4")])]))
    files["layer-states-path.kcad"] = with_settings(55, layerStates=array([state("a", "Görünüm", [node("0", path=text("0"))])]))
    # The line types and their scale are schema 56's (docs/adr/0313 §1): unknown fields in 55; the scale never 1, the
    # list never empty, a name once.
    def line_type_of(name):
        return cmap({"name": text(name), "unit": text("paper"), "elements": array([cmap({"length": f64(1.0)}), cmap({"length": f64(-1.0)})])})
    files["line-type-scale-in-schema-55.kcad"] = with_settings(55, lineTypeScale=f64(2.0))
    files["line-type-scale-one.kcad"] = with_settings(56, lineTypeScale=f64(1.0))
    files["line-types-empty.kcad"] = with_settings(56, lineTypes=array([]))
    files["line-types-same-name.kcad"] = with_settings(56, lineTypes=array([line_type_of("KESIK"), line_type_of("kesik")]))
    # The topology settings are schema 27's (docs/adr/0202 §7): an unknown field in schema 26; not empty, a tolerance in
    # range, a rule's id neither empty nor twice, a known kind, another layer only and always between two layers and
    # never the rule's own, a value only where a kind takes one, an angle below a right angle, an exception's rule there.
    trule = lambda i="r1", kind="mustNotOverlap", **more: cmap({"id": text(i), "kind": text(kind), "layer": text("0"), **more})
    files["topology-in-schema-26.kcad"] = with_settings(26, topology=cmap({"rules": array([trule()])}))
    files["topology-empty.kcad"] = with_settings(27, topology=cmap({}))
    files["topology-tolerance-range.kcad"] = with_settings(27, topology=cmap({"tolerance": f64(2.0)}))
    files["topology-rule-same-id.kcad"] = with_settings(27, topology=cmap({"rules": array([trule(), trule()])}))
    files["topology-rule-unknown-kind.kcad"] = with_settings(27, topology=cmap({"rules": array([trule(kind="mustBeNice")])}))
    files["topology-rule-without-other.kcad"] = with_settings(27, topology=cmap({"rules": array([trule(kind="mustBeCoveredBy")])}))
    files["topology-rule-other-itself.kcad"] = with_settings(27, topology=cmap({"rules": array([trule(kind="mustBeCoveredBy", other=text("0"))])}))
    files["topology-rule-other-on-one-layer.kcad"] = with_settings(27, topology=cmap({"rules": array([trule(other=text("1"))])}))
    files["topology-rule-value-not-taken.kcad"] = with_settings(27, topology=cmap({"rules": array([trule(value=f64(1.0))])}))
    files["topology-rule-angle-too-large.kcad"] = with_settings(27, topology=cmap({"rules": array([trule(kind="mustNotHaveSmallAngles", value=f64(2.0))])}))
    files["topology-exception-unknown-rule.kcad"] = with_settings(27, topology=cmap({"rules": array([trule()]), "exceptions": array([cmap({
        "at": array([f64(500000.0), f64(4400000.0)]), "rule": text("r9"), "objects": array([blob(uid_bytes(m["uids"][0]))])})])}))
    # The annotation heights, a dimension's lines and a leader's AutoCAD arrowheads are schema 30's (docs/adr/0205): in
    # schema 29 unknown fields and unknown values; heights not empty, of a known kind, floats over 0 and at most 100 mm;
    # colours #RRGGBB, weights 0 to 100 mm, line types by name; an arrowhead's size 0.1 to 10 times the note's height.
    files["annotation-in-schema-29.kcad"] = with_settings(29, annotation=cmap({"text": f64(3.5)}))
    files["annotation-empty.kcad"] = with_settings(30, annotation=cmap({}))
    files["annotation-zero.kcad"] = with_settings(30, annotation=cmap({"leader": f64(0.0)}))
    files["annotation-too-tall.kcad"] = with_settings(30, annotation=cmap({"dimension": f64(101.0)}))
    files["annotation-unknown-kind.kcad"] = with_settings(30, annotation=cmap({"label": f64(2.0)}))
    files["annotation-int.kcad"] = with_settings(30, annotation=cmap({"text": uint(3)}))
    files["dimension-lines-in-schema-29.kcad"] = in_schema(29, dimension_of(dimLineColor=text("#C0392B")))
    files["dimension-line-colour-not-hex.kcad"] = in_schema(30, dimension_of(dimLineColor=text("red")))
    files["dimension-ext-weight-negative.kcad"] = in_schema(30, dimension_of(extWeight=f64(-0.1)))
    files["dimension-line-type-unknown.kcad"] = in_schema(30, dimension_of(dimLineType=text("zigzag")))
    files["dimension-style-lines-in-schema-29.kcad"] = with_styles(29, dimensionStyles=array([dimension_style(textColor=text("#1F4E79"))]))
    files["dimension-style-text-colour-short.kcad"] = with_styles(30, dimensionStyles=array([dimension_style(textColor=text("#F00"))]))
    files["dimension-style-line-weight-too-big.kcad"] = with_styles(30, dimensionStyles=array([dimension_style(dimLineWeight=f64(101.0))]))
    files["leader-arrow-closed-in-schema-29.kcad"] = in_schema(29, leader_of(arrow=text("closed")))
    files["leader-arrow-size-in-schema-29.kcad"] = in_schema(29, leader_of(arrowSize=f64(1.5)))
    files["leader-arrow-size-too-small.kcad"] = in_schema(30, leader_of(arrowSize=f64(0.05)))
    files["leader-arrow-size-too-big.kcad"] = in_schema(30, leader_of(arrowSize=f64(10.5)))
    files["leader-arrow-unknown.kcad"] = in_schema(30, leader_of(arrow=text("filled")))
    files["bad-source.kcad"] = container(with_parts({**parts, "migratedFrom": cmap({"format": text("kentos.document"), "version": uint(1), "sourceSha256": blob(bytes(31))})}))
    return files


def settings_parts(s):
    out = {"srid": uint(s["srid"]), "lengthDecimals": uint(s["lengthDecimals"]), "areaDecimals": uint(s["areaDecimals"]), "areaUnit": text(s["areaUnit"]), "angleUnit": text(s["angleUnit"]), "plotScale": f64(s["plotScale"])}
    for k in ("workspace", "drawingFont", "drawingUnit"):
        if k in s:
            out[k] = text(s[k])
    return out


def layer_style_parts(s):
    return {"color": text(s["color"]), "lineType": text(s["lineType"]), "lineWeight": f64(s["lineWeight"])}


# ── Writing and checking ────────────────────────────────────────────────


def build():
    """Every fixture file: {path relative to DIR: bytes}."""
    out = {}
    minimal = load("minimal.json")
    drawing = load("drawing.json")
    moved = migrated()
    out["migrated.json"] = json_text(moved).encode("utf-8")
    out["minimal.kcad"] = container(document(minimal))
    out["drawing.kcad"] = container(document(drawing))
    out["migrated.kcad"] = container(document(moved))
    out["line-weights.kcad"] = container(document(load("line-weights.json")))
    out["elevations.kcad"] = container(document(load("elevations.json")))
    out["parts.kcad"] = container(document(load("parts.json")))
    out["blocks.kcad"] = container(document(load("blocks.json")))
    out["texts.kcad"] = container(document(load("texts.json")))
    out["leaders.kcad"] = container(document(load("leaders.json")))
    out["dimensions.kcad"] = container(document(load("dimensions.json")))
    out["layer-snap.kcad"] = container(document(load("layer-snap.json")))
    out["drawing-unit.kcad"] = container(document(load("drawing-unit.json")))
    out["second-crs.kcad"] = container(document(load("second-crs.json")))
    out["custom-crs.kcad"] = container(document(load("custom-crs.json")))
    out["custom-second-crs.kcad"] = container(document(load("custom-second-crs.json")))
    out["custom-geographic.kcad"] = container(document(load("custom-geographic.json")))
    out["survey.kcad"] = container(document(load("survey.json")))
    out["survey-traverse.kcad"] = container(document(load("survey-traverse.json")))
    out["survey-ground.kcad"] = container(document(load("survey-ground.json")))
    out["survey-sigmas.kcad"] = container(document(load("survey-sigmas.json")))
    out["multi-part-lines.kcad"] = container(document(load("multi-part-lines.json")))
    out["linked-texts.kcad"] = container(document(load("linked-texts.json")))
    out["layer-states.kcad"] = container(document(load("layer-states.json")))
    out["paragraphs.kcad"] = container(document(load("paragraphs.json")))
    out["styles.kcad"] = container(document(load("styles.json")))
    out["tables.kcad"] = container(document(load("tables.json")))
    out["hatches.kcad"] = container(document(load("hatches.json")))
    out["images.kcad"] = container(document(load("images.json")))
    out["text-paths.kcad"] = container(document(load("text-paths.json")))
    out["layer-fields.kcad"] = container(document(load("layer-fields.json")))
    out["topology.kcad"] = container(document(load("topology.json")))
    out["rasters.kcad"] = container(document(load("rasters.json")))
    out["annotation.kcad"] = container(document(load("annotation.json")))
    out["pointclouds.kcad"] = container(document(load("pointclouds.json")))
    out["services.kcad"] = container(document(load("services.json")))
    out["networks.kcad"] = container(document(load("networks.json")))
    out["scenarios.kcad"] = container(document(load("scenarios.json")))
    out["filters.kcad"] = container(document(load("filters.json")))
    out["labels.kcad"] = container(document(load("labels.json")))
    out["variables.kcad"] = container(document(load("variables.json")))
    out["multidim.kcad"] = container(document(load("multidim.json")))
    out["measures.kcad"] = container(document(load("measures.json")))
    out["attachments.kcad"] = container(document(load("attachments.json")))
    out["compose.kcad"] = container(document(load("compose.json")))
    out["addresses.kcad"] = container(document(load("addresses.json")))
    out["dimension-refs.kcad"] = container(document(load("dimension-refs.json")))
    out["sheets.kcad"] = container(document(load("sheets.json")))
    out["requests.kcad"] = container(document(load("requests.json")))
    out["polyline-extras.kcad"] = container(document(load("polyline-extras.json")))
    out["nurbs.kcad"] = container(document(load("nurbs.json")))
    out["look.kcad"] = container(document(load("look.json")))
    out["ucs.kcad"] = container(document(load("ucs.json")))
    out["imperial.kcad"] = container(document(load("imperial.json")))
    out["subref-extras.kcad"] = container(document(load("subref-extras.json")))
    out["views.kcad"] = container(document(load("views.json")))
    out["arrays.kcad"] = container(document(load("arrays.json")))
    # The associative arrays are schema 53's (docs/adr/0300 §1): in schema 52 an unknown field; an unknown block, items not
    # one per place, an insert in two items or not in the drawing, a scale of zero refused.
    def arrays_broken(change):
        d = copy.deepcopy(load("arrays.json"))
        change(d)
        return container(document(d))

    payload = document(load("arrays.json"))
    assert payload.count(b"\x67version\x18\x35") == 1
    out["broken/arrays-in-schema-52.kcad"] = container(payload.replace(b"\x67version\x18\x35", b"\x67version\x18\x34"))
    out["broken/arrays-block-unknown.kcad"] = arrays_broken(lambda d: d["arrays"][0].update(block="0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0bff"))
    out["broken/arrays-items-short.kcad"] = arrays_broken(lambda d: d["arrays"][0]["items"].pop())
    out["broken/arrays-item-twice.kcad"] = arrays_broken(lambda d: d["arrays"][0]["items"][2].update(uid=d["arrays"][0]["items"][0]["uid"]))
    out["broken/arrays-item-missing.kcad"] = arrays_broken(lambda d: d["arrays"][0]["items"][2].update(uid="0192f5a2-2222-7000-8000-0000000000ff"))
    out["broken/arrays-scale-zero.kcad"] = arrays_broken(lambda d: d["arrays"][1].update(scale=0.0))
    out["wipeouts.kcad"] = container(document(load("wipeouts.json")))
    payload = document(load("wipeouts.json"))
    assert payload.count(b"\x67version\x18\x36") == 1
    out["broken/wipeouts-in-schema-53.kcad"] = container(payload.replace(b"\x67version\x18\x36", b"\x67version\x18\x35"))
    out["block-extras.kcad"] = container(document(load("block-extras.json")))
    payload = document(load("block-extras.json"))
    assert payload.count(b"\x67version\x18\x39") == 1
    out["broken/block-extras-in-schema-56.kcad"] = container(payload.replace(b"\x67version\x18\x39", b"\x67version\x18\x38"))
    out["patterns.kcad"] = container(document(load("patterns.json")))
    payload = document(load("patterns.json"))
    assert payload.count(b"\x67version\x18\x38") == 1
    out["broken/patterns-in-schema-55.kcad"] = container(payload.replace(b"\x67version\x18\x38", b"\x67version\x18\x37"))
    out["frozen.kcad"] = container(document(load("frozen.json")))
    payload = document(load("frozen.json"))
    assert payload.count(b"\x67version\x18\x37") == 1
    out["broken/frozen-in-schema-54.kcad"] = container(payload.replace(b"\x67version\x18\x37", b"\x67version\x18\x36"))
    # A newer writer that used no newer feature: 2.0 readers read it.
    out["readable-minor.kcad"] = container(document(minimal), minor=7, min_reader=0)
    for name, data in broken(minimal, out["minimal.kcad"]).items():
        out[f"broken/{name}"] = data
    return out


def bits(v):
    """A value compared bit for bit: floats by their binary64 bytes, so -0.0 ≠ 0.0."""
    if type(v) is float:
        return ("f", struct.pack(">d", v))
    if type(v) is list:
        return [bits(x) for x in v]
    if type(v) is dict:
        return {k: bits(x) for k, x in v.items()}
    return v


def normalized(content):
    """A drawing given by hand, with its typed float fields made floats (as the contract reads them)."""
    return kcad.read(container(document(content)))[1]


def check(files):
    problems = []
    for rel, data in files.items():
        path = DIR / rel
        if not path.exists() or path.read_bytes() != data:
            problems.append(f"{rel}: diskteki dosya yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
    expected = json.loads((DIR / "expected.json").read_text("utf-8"))
    for case in expected["files"]:
        name = case["file"]
        data = (DIR / name).read_bytes()
        got = kcad.sniff(data)
        if got != case["sniff"]:
            problems.append(f"{name}: koklama {got}, beklenen {case['sniff']}")
        try:
            _, doc = kcad.read(data)
        except kcad.KcadError as e:
            if case.get("error") != e.code:
                problems.append(f"{name}: {e.code} ({e.message}), beklenen {case.get('error') or 'geçerli'}")
            continue
        if "error" in case:
            problems.append(f"{name}: okundu, beklenen hata {case['error']}")
            continue
        if "content" in case:
            want = normalized(load(case["content"]))
            if bits(doc) != bits(want):
                problems.append(f"{name}: okunan çizim {case['content']} ile bit bit aynı değil")
        elif len(doc["entities"]) != case["entities"]:
            problems.append(f"{name}: {len(doc['entities'])} nesne, beklenen {case['entities']}")
        # A 2.0 writer writes the same drawing to the same bytes; `rewrite: false` marks a file another writer version wrote.
        if case.get("rewrite", True) and container(document(doc)) != data:
            problems.append(f"{name}: yeniden yazınca aynı baytlar çıkmıyor")
    listed = {c["file"] for c in expected["files"]}
    for rel in files:
        if rel.endswith(".kcad") and rel not in listed:
            problems.append(f"{rel}: expected.json'da yok")
    return problems


def main():
    files = build()
    if "--check" in sys.argv[1:]:
        problems = check(files)
        for p in problems:
            print(p, file=sys.stderr)
        if problems:
            return 1
        print(f"KCAD v2 fixture'ları tutarlı: {len(files)} dosya, expected.json'daki her dosya okuyucuyla aynı sonucu veriyor.")
        return 0
    for rel, data in files.items():
        path = DIR / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    print(f"{len(files)} dosya yazıldı: {DIR.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
