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
import hashlib
import json
import math
import re
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
# A leader's arrowheads (spec §6.6, docs/adr/0146); the filled arrow is the field's absence.
LEADER_ARROWS = ("open", "dot", "none")


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
                "drawingUnit": (enum(("mm", "cm", "m")), False),
                "secondSrid": (uint, False),
                "customCrs": (crs_definition, False),
                "secondCustomCrs": (crs_definition, False),
                "datumTransforms": (datum_transforms, False),
                "survey": (survey, False),
                "layerStates": (layer_states, False),
            },
            "settings",
        )
    )


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
            nodes.append(cmap(fields(n, {"node": (text, True), "style": (layer_style, False), "locked": (boolean, False), "visible": (boolean, True)}, "layerStateNode")))
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
    return cmap(fields(s, {"index": (f64, False), "faceHz": (f64, False), "faceSlope": (f64, False), "refraction": (f64, False),
                           "twoWay": (f64, False), "traverseAngle": (f64, False), "traverseCoord": (f64, False),
                           "groundHeight": (f64, False), "reduceToGrid": (boolean, False)}, "survey"))


def label_style(s):
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
            },
            "label",
        )
    )


def not_null(v):
    assert v is not None
    return opaque(v)


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
                "pickInterior": (lambda b: b"\xf5" if b else b"\xf4", False),
                "renderer": (not_null, False),
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


def layer(n):
    assert "snap" not in n or n["type"] == "layer", f"layer {n.get('id')}: grubun keneti olmaz"
    return cmap(
        fields(
            n,
            {
                "id": (text, True),
                "name": (text, True),
                "snap": (layer_snap, False),
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
    return cmap(fields(pt, {"pts": (points, True), "bulges": (floats, False), "zs": (elevations, False)}, "line part"))


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
    return table


KINDS = {
    # Schema 17 (docs/adr/0174): a multi-point object's points past its first.
    "point": {"p": (point, True), "z": (f64, False), "parts": (lambda ps: array([point_part(pp) for pp in ps]), False)},
    # A line's end without an elevation has no key (spec §5.2): `null` is only for a vertex in `zs`.
    "line": {"a": (point, True), "b": (point, True), "za": (f64, False), "zb": (f64, False)},
    "polyline": path_fields(False),
    "polygon": path_fields(True),
    "circle": {"c": (point, True), "r": (f64, True)},
    "arc": {"c": (point, True), "r": (f64, True), "a0": (f64, True), "a1": (f64, True)},
    "ellipse": {"c": (point, True), "major": (point, True), "ratio": (f64, True), "t0": (f64, True), "t1": (f64, True)},
    "spline": {"pts": (points, True), "closed": (boolean, True)},
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
    },
    "hatch": {
        "ring": (points, True),
        "holes": (lambda h: array([points(r) for r in h]), False),
        "pattern": (lambda p: cmap(fields(p, {"type": (enum(("solid", "lines", "cross")), True), "angle": (f64, True), "spacing": (f64, True)}, "pattern")), True),
    },
    # Schema 6 (docs/adr/0144): the definition's id; `mirror` only when true.
    "insert": {"block": (lambda u: blob(uid_bytes(u)), True), "p": (point, True), "scale": (f64, True), "rotation": (f64, True), "mirror": (lambda b: boolean(b) if b is True else None, False)},
    # Schema 8 (docs/adr/0146): two vertices or more, a note only when there is one, an arrowhead by name, `mask` only when true.
    "leader": {
        "pts": (points, True),
        "text": (text, False),
        "height": (f64, True),
        "rotation": (f64, True),
        "arrow": (enum(LEADER_ARROWS), False),
        "mask": (lambda b: boolean(b) if b is True else None, False),
    },
}


def entity(e, uid, index):
    """An object; `uid` None for a block definition's object, which has no persistent id (schema 6)."""
    kind = e["kind"]
    assert e["id"] == index + 1, f"nesne {index}: kimlik {e['id']}, dosya sırası {index + 1} vermeli"
    assert "zs" not in e or len(e["zs"]) == len(e["pts"]), f"nesne {index} ({kind}): kot sayısı köşe sayısına eşit olmalı"
    table = {
        "layerId": (text, True),
        "color": (text, False),
        "attrs": (lambda a: cmap({k: text(v) for k, v in a.items()}), True),
        "label": (text, False),
        "symbol": (text, False),
        # Schema 3 (docs/adr/0139): an object's own line weight, mm.
        "lineWeight": (f64, False),
        **KINDS[kind],
    }
    body = fields({k: v for k, v in e.items() if k not in ("kind", "id")}, table, f"nesne {index} ({kind})")
    assert body.get("mirror", True) is not None, f"nesne {index}: mirror yalnız true yazılır"
    assert body.get("mask", True) is not None, f"nesne {index}: mask yalnız true yazılır"
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
    }
    return cmap(fields(b, table, f"blok {b.get('name')}"))


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
            "migratedFrom": (
                lambda m: cmap(fields(m, {"format": (text, True), "version": (uint, True), "sourceSha256": (lambda h: blob(bytes.fromhex(h)), True)}, "migratedFrom")),
                False,
            ),
        },
        "belge",
    )
    assert d.get("blocks", [None]), "boş blok listesi yazılmaz"
    return root(cmap(body), version=uint(schema_of(d["entities"], d.get("blocks"), d["layers"], d["settings"])))


def schema_of(entities, blocks=None, layers=(), settings=None):
    """The oldest schema that holds the drawing: 20 with a multi-line text's box, line spacing or letter formats, in the
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
    files["schema-version-21.kcad"] = container(root(cmap(parts), version=b"\x15"))
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
    files["elevation-in-schema-2.kcad"] = in_schema(2, line(za=f64(105.25)))
    files["elevation-in-schema-3.kcad"] = in_schema(3, path("polyline", 3, zs=elevations([101.5, None, 103.25])))
    files["elevation-wrong-length.kcad"] = in_schema(4, path("polyline", 3, zs=elevations([101.5, 103.25])))
    hole = cmap({"pts": array([point({"x": one["p"]["x"] + i, "y": one["p"]["y"] + 1.0}) for i in range(3)]), "zs": elevations([1.0, 2.0, 3.0, 4.0])})
    files["hole-elevation-wrong-length.kcad"] = in_schema(4, path("polygon", 4, holes=array([hole])))
    files["elevation-nan.kcad"] = in_schema(4, path("polyline", 3, zs=array([f64(101.5), nan, f64(103.25)])))
    files["elevation-infinity.kcad"] = in_schema(4, line(zb=infinity))
    files["elevation-int.kcad"] = in_schema(4, path("polyline", 3, zs=array([f64(101.5), uint(102), f64(103.25)])))
    files["line-elevation-null.kcad"] = in_schema(4, line(za=b"\xf6"))
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
    files["blocks-in-schema-5.kcad"] = with_blocks([block(1, "Rögar")], entities=[], version=5)
    files["insert-in-schema-5.kcad"] = in_schema(5, insert_of(1))
    files["unknown-block.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(2)])
    files["unknown-block-inside.kcad"] = with_blocks([block(1, "Rögar", [content_insert(3)])])
    files["duplicate-block-name.kcad"] = with_blocks([block(1, "Direk"), block(2, "DİREK")])
    files["duplicate-block-id.kcad"] = with_blocks([block(1, "Direk"), block(1, "Lamba")])
    files["blank-block-name.kcad"] = with_blocks([block(1, " \u3000")])
    files["block-cycle.kcad"] = with_blocks([block(1, "A", [content_insert(2)]), block(2, "B", [circle, content_insert(1)])])
    files["block-too-deep.kcad"] = with_blocks([block(n, f"K{n}", [content_insert(n + 1)] if n < 17 else None) for n in range(1, 18)])
    files["block-object-uid.kcad"] = with_blocks([block(1, "Rögar", [cmap({"circle": cmap({"attrs": cmap({}), "layerId": text("0"), "uid": blob(uid_bytes(m["uids"][0])), "c": point({"x": 0.0, "y": 0.0}), "r": f64(0.5)})})])])
    files["insert-scale-zero.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(1, scale=f64(0.0))])
    files["insert-mirror-false.kcad"] = with_blocks([block(1, "Rögar")], entities=[insert_of(1, mirror=boolean(False))])
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
    out["multi-part-lines.kcad"] = container(document(load("multi-part-lines.json")))
    out["linked-texts.kcad"] = container(document(load("linked-texts.json")))
    out["layer-states.kcad"] = container(document(load("layer-states.json")))
    out["paragraphs.kcad"] = container(document(load("paragraphs.json")))
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
