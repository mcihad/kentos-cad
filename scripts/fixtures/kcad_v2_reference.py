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
the project id and the migration source; so it is what opening that v1 file
and saving it as v2 must give on every platform.
"""
import hashlib
import json
import math
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


# ── Document schema 2 (spec §6), from the contract's JSON form ──────────


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


def settings(s):
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
            },
            "settings",
        )
    )


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


def layer(n):
    return cmap(
        fields(
            n,
            {
                "id": (text, True),
                "name": (text, True),
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


def ring(r):
    return cmap(fields(r, {"pts": (points, True), "bulges": (floats, False)}, "ring"))


def path_fields(holes):
    table = {"pts": (points, True), "bulges": (floats, False)}
    if holes:
        table["holes"] = (lambda h: array([ring(r) for r in h]), False)
    return table


KINDS = {
    "point": {"p": (point, True), "z": (f64, False)},
    "line": {"a": (point, True), "b": (point, True)},
    "polyline": path_fields(False),
    "polygon": path_fields(True),
    "circle": {"c": (point, True), "r": (f64, True)},
    "arc": {"c": (point, True), "r": (f64, True), "a0": (f64, True), "a1": (f64, True)},
    "ellipse": {"c": (point, True), "major": (point, True), "ratio": (f64, True), "t0": (f64, True), "t1": (f64, True)},
    "spline": {"pts": (points, True), "closed": (boolean, True)},
    "xline": {"p": (point, True), "dir": (point, True)},
    "ray": {"p": (point, True), "dir": (point, True)},
    "text": {"p": (point, True), "text": (text, True), "height": (f64, True), "rotation": (f64, True)},
    "dimension": {
        "a": (point, True),
        "b": (point, True),
        "offset": (f64, True),
        "height": (f64, True),
        "text": (text, False),
        "style": (enum(("aligned", "linear", "angular", "radius", "diameter")), False),
        "angle": (f64, False),
        "c": (point, False),
    },
    "hatch": {
        "ring": (points, True),
        "holes": (lambda h: array([points(r) for r in h]), False),
        "pattern": (lambda p: cmap(fields(p, {"type": (enum(("solid", "lines", "cross")), True), "angle": (f64, True), "spacing": (f64, True)}, "pattern")), True),
    },
}


def entity(e, uid, index):
    kind = e["kind"]
    assert e["id"] == index + 1, f"nesne {index}: kimlik {e['id']}, dosya sırası {index + 1} vermeli"
    table = {
        "layerId": (text, True),
        "color": (text, False),
        "attrs": (lambda a: cmap({k: text(v) for k, v in a.items()}), True),
        "label": (text, False),
        "symbol": (text, False),
        **KINDS[kind],
    }
    body = fields({k: v for k, v in e.items() if k not in ("kind", "id")}, table, f"nesne {index} ({kind})")
    body["uid"] = blob(uid_bytes(uid))
    return cmap({kind: cmap(body)})


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
            "styles": (lambda s: cmap(fields(s, {"items": (lambda xs: array([opaque(x) for x in xs]), True), "categories": (lambda xs: array([opaque(x) for x in xs]), True)}, "styles")), True),
            "projectId": (lambda u: blob(uid_bytes(u)), False),
            "migratedFrom": (
                lambda m: cmap(fields(m, {"format": (text, True), "version": (uint, True), "sourceSha256": (lambda h: blob(bytes.fromhex(h)), True)}, "migratedFrom")),
                False,
            ),
        },
        "belge",
    )
    return root(cmap(body))


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

    return {
        "format": "kentos.document",
        "version": 2,
        "name": v1["name"],
        "settings": typed(v1["settings"], {"plotScale"}),
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
    files["schema-version-3.kcad"] = container(root(cmap(parts), version=b"\x03"))
    files["unknown-field.kcad"] = container(with_parts({**parts, "extra": text("?")}))
    files["missing-field.kcad"] = container(with_parts({k: v for k, v in parts.items() if k != "activeLayer"}))
    files["int-for-float.kcad"] = container(with_parts({**parts, "settings": cmap({**{k: v for k, v in settings_parts(m["settings"]).items()}, "plotScale": uint(1000)})}))
    one = m["entities"][0]
    body = {"attrs": cmap({k: text(v) for k, v in one["attrs"].items()}), "layerId": text(one["layerId"]), "p": point(one["p"])}
    files["short-uid.kcad"] = container(with_parts({**parts, "entities": array([cmap({"point": cmap({**body, "uid": blob(bytes(range(1, 16)))})})])}))
    files["nil-uid.kcad"] = container(with_parts({**parts, "entities": array([cmap({"point": cmap({**body, "uid": blob(bytes(16))})})])}))
    same = cmap({"point": cmap({**body, "uid": blob(uid_bytes(m["uids"][0]))})})
    files["duplicate-uid.kcad"] = container(with_parts({**parts, "entities": array([same, same])}))
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
    files["big-negative.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([b"\x3b" + b"\xff" * 8]), "categories": array([])})}))
    files["bytes-in-opaque.kcad"] = container(with_parts({**parts, "styles": cmap({"items": array([blob(b"\x01")]), "categories": array([])})}))
    files["bad-enum.kcad"] = container(with_parts({**parts, "settings": cmap({**settings_parts(m["settings"]), "areaUnit": text("acre")})}))
    files["srid-range.kcad"] = container(with_parts({**parts, "settings": cmap({**settings_parts(m["settings"]), "srid": uint(1 << 32)})}))
    files["bad-source.kcad"] = container(with_parts({**parts, "migratedFrom": cmap({"format": text("kentos.document"), "version": uint(1), "sourceSha256": blob(bytes(31))})}))
    return files


def settings_parts(s):
    out = {"srid": uint(s["srid"]), "lengthDecimals": uint(s["lengthDecimals"]), "areaDecimals": uint(s["areaDecimals"]), "areaUnit": text(s["areaUnit"]), "angleUnit": text(s["angleUnit"]), "plotScale": f64(s["plotScale"])}
    for k in ("workspace", "drawingFont"):
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
        want = normalized(load(case["content"]))
        if bits(doc) != bits(want):
            problems.append(f"{name}: okunan çizim {case['content']} ile bit bit aynı değil")
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
