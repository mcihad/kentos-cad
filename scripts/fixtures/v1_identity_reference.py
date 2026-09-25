"""Independent reference for the persistent ids of v1 drawings (docs/adr/0014, TODOS.md DOM-04).

A v1 `.kcad` keeps no persistent object ids; they are derived from its content
when it is opened. This computes them with Python's standard library only
(json, hashlib, uuid, decimal), never with KentOS code, so the Rust contracts
(native) and the browser (the formats WASM module) are held to an outside
reference and not to themselves:

1. the drawing is written canonically: the contract's fields in their declared
   order (crates/shared/contracts/src/{document,layer,entity}.rs, mirrored in
   the schema below), no whitespace, float64 fields in serde_json's shortest
   round-trip form (`f64_text`), attributes and the keys of the opaque parts
   (style items, renderers) sorted, fields the contract does not know left out;
2. namespace = uuid5(KENTOS_V1_IMPORT, sha256 of that text as 64 lowercase hex digits);
3. an object's id = uuid5(namespace, "entity/<local id>"); the project's = uuid5(namespace, "project").

Run from the repository root:

    python3 scripts/fixtures/v1_identity_reference.py           # writes the fixture
    python3 scripts/fixtures/v1_identity_reference.py --check   # compares, writes nothing

Writes fixtures/document/v1/identity/: the inputs derived from
fixtures/document/v1/sample.json (the web app's recorded sample), each case's
canonical text (*.canonical.json) and expected.json. minimal.kcad is written by
hand and only read.
"""
import hashlib
import json
import math
import sys
import uuid
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/document/v1/identity"
SAMPLE = ROOT / "fixtures/document/v1/sample.json"

# Never changes (crates/shared/contracts/src/identity.rs): a random v4 drawn once, 25 September 2026.
KENTOS_V1_IMPORT = uuid.UUID("4a5259a1-97f7-4742-88ce-b747287025ed")

# ── The contract's shape ────────────────────────────────────────────────
# ("obj", [(name, type)]) fields in declaration order; ("opt", t) absent or
# null means left out; ("arr", t); ("map", t) keys sorted; "f64", "int",
# "str", "bool"; "any" is opaque JSON; ("ref", name) a named type.


def opt(t):
    return ("opt", t)


def arr(t):
    return ("arr", t)


VEC2 = ("obj", [("x", "f64"), ("y", "f64")])
RING = ("obj", [("pts", arr(VEC2)), ("bulges", opt(arr("f64")))])
PATH = [("pts", arr(VEC2)), ("bulges", opt(arr("f64"))), ("holes", opt(arr(RING)))]
TYPES = {
    "document": ("obj", [
        ("format", "str"),
        ("version", "int"),
        ("name", "str"),
        ("settings", ("obj", [
            ("srid", "int"),
            ("lengthDecimals", "int"),
            ("areaDecimals", "int"),
            ("areaUnit", "str"),
            ("angleUnit", "str"),
            ("plotScale", "f64"),
            ("workspace", opt("str")),
            ("drawingFont", opt("str")),
        ])),
        ("origin", VEC2),
        ("homeView", opt(("obj", [("minX", "f64"), ("minY", "f64"), ("maxX", "f64"), ("maxY", "f64")]))),
        ("layers", arr(("ref", "layer"))),
        ("activeLayer", "str"),
        ("entities", arr("entity")),
        ("styles", ("obj", [("items", arr("any")), ("categories", arr("any"))])),
    ]),
    "layer": ("obj", [
        ("id", "str"),
        ("name", "str"),
        ("type", "str"),
        ("visible", "bool"),
        ("locked", "bool"),
        ("expanded", "bool"),
        ("style", ("obj", [
            ("color", "str"),
            ("lineType", "str"),
            ("lineWeight", "f64"),
            ("fill", opt("str")),
            ("point", opt(("obj", [("symbol", "str"), ("size", "f64")]))),
            ("label", opt(("obj", [
                ("placement", "str"),
                ("size", "f64"),
                ("grow", opt("f64")),
                ("maxSize", opt("f64")),
                ("weight", opt("int")),
                ("template", opt("str")),
                ("minFeaturePx", opt("f64")),
                ("minScale", opt("f64")),
                ("maxScale", opt("f64")),
                ("ink", opt("str")),
            ]))),
            ("pickInterior", opt("bool")),
            ("renderer", opt("any")),
        ])),
        ("children", arr(("ref", "layer"))),
    ]),
}
# An entity: "kind" first (serde's internal tag), the common fields, then the kind's own.
ENTITY_BASE = [("id", "int"), ("layerId", "str"), ("color", opt("str")), ("attrs", ("map", "str")), ("label", opt("str")), ("symbol", opt("str"))]
ENTITY_KINDS = {
    "point": [("p", VEC2), ("z", opt("f64"))],
    "line": [("a", VEC2), ("b", VEC2)],
    "polyline": PATH,
    "polygon": PATH,
    "circle": [("c", VEC2), ("r", "f64")],
    "arc": [("c", VEC2), ("r", "f64"), ("a0", "f64"), ("a1", "f64")],
    "ellipse": [("c", VEC2), ("major", VEC2), ("ratio", "f64"), ("t0", "f64"), ("t1", "f64")],
    "spline": [("pts", arr(VEC2)), ("closed", "bool")],
    "xline": [("p", VEC2), ("dir", VEC2)],
    "ray": [("p", VEC2), ("dir", VEC2)],
    "text": [("p", VEC2), ("text", "str"), ("height", "f64"), ("rotation", "f64")],
    "dimension": [("a", VEC2), ("b", VEC2), ("offset", "f64"), ("height", "f64"), ("text", opt("str")), ("style", opt("str")), ("angle", opt("f64")), ("c", opt(VEC2))],
    "hatch": [("ring", arr(VEC2)), ("holes", opt(arr(arr(VEC2)))), ("pattern", ("obj", [("type", "str"), ("angle", "f64"), ("spacing", "f64")]))],
}

# ── Canonical text ──────────────────────────────────────────────────────


def f64_text(x):
    """A float64 as serde_json writes it: the shortest digits that read back
    to the same value (Python's repr finds the same ones), laid out as its
    formatter does: plain between 1e-5 and 1e16 (always with a fraction,
    "486512.0"), otherwise d.ddde±N ("1e-7", "1e+16")."""
    if x == 0:
        return "-0.0" if math.copysign(1.0, x) < 0 else "0.0"
    sign = "-" if x < 0 else ""
    _, digits, exp = Decimal(repr(abs(x))).as_tuple()
    digits = list(digits)
    while len(digits) > 1 and digits[-1] == 0:
        digits.pop()
        exp += 1
    d = "".join(map(str, digits))
    k = len(d) - 1 + exp  # decimal exponent of the first digit
    if -5 <= k <= 15:
        if len(d) - 1 <= k:
            body = d + "0" * (k - len(d) + 1) + ".0"
        elif k >= 0:
            body = d[: k + 1] + "." + d[k + 1 :]
        else:
            body = "0." + "0" * (-k - 1) + d
    else:
        body = d[0] + ("." + d[1:] if len(d) > 1 else "") + ("e-" if k < 0 else "e+") + str(abs(k))
    return sign + body


def json_str(s):
    # serde_json escapes `"`, `\` and the control characters (\b \t \n \f \r, the
    # rest as lowercase \u00XX) and nothing else; so does json.dumps without ASCII escaping.
    return json.dumps(s, ensure_ascii=False)


def opaque(v):
    if v is None:
        return "null"
    if v is True:
        return "true"
    if v is False:
        return "false"
    if isinstance(v, int):
        assert -(2**63) <= v < 2**64, "serde_json reads a larger integer as a float"
        return str(v)
    if isinstance(v, float):
        return f64_text(v)
    if isinstance(v, str):
        return json_str(v)
    if isinstance(v, list):
        return "[" + ",".join(opaque(x) for x in v) + "]"
    return "{" + ",".join(json_str(k) + ":" + opaque(v[k]) for k in sorted(v)) + "}"


def canon(v, t):
    if isinstance(t, tuple) and t[0] == "ref":
        t = TYPES[t[1]]
    if t == "f64":
        assert isinstance(v, (int, float)) and not isinstance(v, bool)
        return f64_text(float(v))
    if t == "int":
        assert isinstance(v, int) and not isinstance(v, bool) and v >= 0
        return str(v)
    if t == "str":
        assert isinstance(v, str)
        return json_str(v)
    if t == "bool":
        assert isinstance(v, bool)
        return "true" if v else "false"
    if t == "any":
        return opaque(v)
    if t == "entity":
        return fields(v, [("kind", "str")] + ENTITY_BASE + ENTITY_KINDS[v["kind"]])
    if t[0] == "arr":
        return "[" + ",".join(canon(x, t[1]) for x in v) + "]"
    if t[0] == "map":
        return "{" + ",".join(json_str(k) + ":" + canon(v[k], t[1]) for k in sorted(v)) + "}"
    if t[0] == "obj":
        return fields(v, t[1])
    raise ValueError(t)


def fields(v, spec):
    out = []
    for name, t in spec:
        x = v.get(name)
        if isinstance(t, tuple) and t[0] == "opt":
            if x is None:
                continue
            t = t[1]
        elif name not in v:
            raise KeyError(name)
        out.append(json_str(name) + ":" + canon(x, t))
    return "{" + ",".join(out) + "}"


def identities(text):
    doc = json.loads(text.removeprefix("﻿"))
    canonical = canon(doc, TYPES["document"])
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    ns = uuid.uuid5(KENTOS_V1_IMPORT, digest)
    return canonical, {
        "sourceSha256": digest,
        "namespace": str(ns),
        "project": str(uuid.uuid5(ns, "project")),
        "entities": [{"id": e["id"], "uid": str(uuid.uuid5(ns, f"entity/{e['id']}"))} for e in doc["entities"]],
    }


# ── Inputs derived from the recorded sample ─────────────────────────────


def compact(doc):
    """What the web app writes (`JSON.stringify(snapshot) + "\\n"`): the sample
    holds no number whose JavaScript and Python spellings differ."""
    return json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n"


def variant(v, t, depth=0):
    """The same drawing spelled differently: Windows line ends, tab
    indentation, every object's keys in reverse, float64 fields with 17
    significant digits in exponent form ("4.86512340000000000e+05", the
    same value), and floats of the opaque parts too (their integers stay
    integers, which the canonical text keeps apart)."""
    if isinstance(t, tuple) and t[0] == "ref":
        t = TYPES[t[1]]
    if t == "f64":
        return "{:.17e}".format(float(v))
    if t in ("int", "str", "bool"):
        return json.dumps(v, ensure_ascii=False)
    nl = "\r\n" + "\t" * (depth + 1)
    end = "\r\n" + "\t" * depth
    if t == "any":
        if isinstance(v, float):
            return "{:.17e}".format(v)
        if isinstance(v, list):
            return "[" + ",".join(nl + variant(x, "any", depth + 1) for x in v) + end + "]" if v else "[]"
        if isinstance(v, dict):
            return "{" + ",".join(nl + json.dumps(k, ensure_ascii=False) + ": " + variant(v[k], "any", depth + 1) for k in reversed(list(v))) + end + "}" if v else "{}"
        return json.dumps(v, ensure_ascii=False)
    if t == "entity":
        spec = dict([("kind", "str")] + ENTITY_BASE + ENTITY_KINDS[v["kind"]])
    elif t[0] == "arr":
        return "[" + ",".join(nl + variant(x, t[1], depth + 1) for x in v) + end + "]" if v else "[]"
    elif t[0] == "map":
        return "{" + ",".join(nl + json.dumps(k, ensure_ascii=False) + ": " + variant(v[k], t[1], depth + 1) for k in reversed(list(v))) + end + "}" if v else "{}"
    else:
        spec = dict(t[1])
    items = []
    for k in reversed(list(v)):
        ft = spec[k]
        if isinstance(ft, tuple) and ft[0] == "opt":
            ft = ft[1]
        items.append(nl + json.dumps(k, ensure_ascii=False) + ": " + variant(v[k], ft, depth + 1))
    return "{" + ",".join(items) + end + "}"


def main(check):
    sample = json.loads(SAMPLE.read_text(encoding="utf-8"))
    edited = json.loads(SAMPLE.read_text(encoding="utf-8"))
    edited["entities"][0]["p"]["x"] = 486513.342  # one millimetre: another drawing
    derived = {
        "sample.compact.kcad": compact(sample),
        "sample.variant.kcad": variant(sample, TYPES["document"]) + "\r\n",
        "edited.kcad": compact(edited),
    }
    cases = [
        ("sample", ["../sample.json", "sample.compact.kcad", "sample.variant.kcad"]),
        ("edited", ["edited.kcad"]),
        ("minimal", ["minimal.kcad"]),
    ]
    files = dict(derived)
    expected = {
        "producedBy": f"python3 scripts/fixtures/v1_identity_reference.py (Python {sys.version.split()[0]}; json, hashlib, uuid, decimal)",
        "importNamespace": str(KENTOS_V1_IMPORT),
        "cases": [],
    }
    for name, inputs in cases:
        results = []
        for f in inputs:
            text = derived[f] if f in derived else (DIR / f).read_text(encoding="utf-8")
            results.append(identities(text))
        canonical, ids = results[0]
        for other, _ in results[1:]:
            assert other == canonical, f"{name}: the inputs should have one canonical text"
        files[f"{name}.canonical.json"] = canonical
        expected["cases"].append({"name": name, "inputs": inputs, "canonical": f"{name}.canonical.json", **ids})
    files["expected.json"] = json.dumps(expected, ensure_ascii=False, indent=2) + "\n"
    stale = [f for f, text in files.items() if not (DIR / f).exists() or (DIR / f).read_bytes() != text.encode("utf-8")]
    if check:
        if stale:
            sys.exit(f"out of date: {', '.join(stale)} (run without --check, read the diff)")
        print("fixtures/document/v1/identity is current")
        return
    for f in stale:
        (DIR / f).write_bytes(files[f].encode("utf-8"))
    print(f"wrote {len(stale)} file(s) in {DIR.relative_to(ROOT)}")


if __name__ == "__main__":
    main("--check" in sys.argv[1:])
