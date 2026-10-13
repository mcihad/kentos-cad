#!/usr/bin/env python3
"""The command protocol's shared cases (docs/adr/0253), written without
KentOS code: canonical JSON and its floats, state stamps of objects, block
definitions and the project's parts, a request's fingerprint, a plan's id
and the summary's rules, and the envelope's, expectations' and requests'
answers. Written to fixtures/protocol/v1/cases.json; `--check` writes
nothing and fails when the file differs.

The Rust side (crates/native/application/tests/all/protocol_cases.rs)
builds each drawing from its v2 snapshot, checks the snapshot reads back as
written (so the values hashed here are the values hashed there), computes
the same stamps, normalizes the inputs, and runs every call through the
protocol (`call::call`).

The rules, as the ADR states them:

- canonical JSON: no white space; keys sorted by their UTF-8 bytes; strings
  escaped as JSON requires (`"`, `\\`, `\\b \\f \\n \\r \\t`, other control
  characters `\\u00XX`), nothing else; integers by their digits; floats by
  their shortest round-trip digits, in decimal notation when
  1e-5 <= |x| < 1e16 (a whole number keeping `.0`), otherwise with an
  exponent signed either way (`1e-6`, `1e+16`); zero `0.0` or `-0.0`. Of
  two shortest candidates equally near, the even one. Here the digits come
  from CPython's `repr` (David Gay's algorithm, ties to even, as
  ECMAScript's Number.prototype.toString); Rust's from its own formatter,
  whose shortest form breaks such a tie up (canonical.rs takes the even).
- a stamp: the first 16 hex digits of the SHA-256 of the canonical text; a
  plan's id and a request's fingerprint: the first 32.
- an object's stamp: of {"uid", "entity"} with the entity's slot (`id`)
  taken out; a block definition's: of the definition; the layer tree's: of
  {"nodes", "active"}; the settings': of {"name", "settings"}; the styles':
  of the library; the sheets': of the book and its pictures' SHA-256s, null
  without a book.
- a request's fingerprint: of {"command", "version", "op", "input"
  (normalized), "expected" (the envelope's, `{}` without one)}.
- a plan's id: of {"command", "version", "input" (normalized, without
  `expectedRevision`), "sources", "result"}, every new object's id written
  `#aN` and every new block's `#bN` by the order the write made them.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import random
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "protocol" / "v1" / "cases.json"

# ── Canonical JSON ──────────────────────────────────────────────────────


def decompose(x: float) -> tuple[str, int]:
    """The shortest round-trip digits of |x| > 0 and where the point goes:
    |x| = 0.DIGITS × 10^point, DIGITS without leading or trailing zeros."""
    r = repr(abs(x))
    mantissa, _, exp = r.partition("e")
    whole, _, frac = mantissa.partition(".")
    digits = whole + frac
    point = len(whole) + (int(exp) if exp else 0)
    stripped = digits.lstrip("0")
    point -= len(digits) - len(stripped)
    digits = stripped.rstrip("0")
    return digits, point


def float_text(x: float) -> str:
    if math.isnan(x) or math.isinf(x):
        return "null"
    sign = "-" if math.copysign(1.0, x) < 0 else ""
    if x == 0:
        return sign + "0.0"
    digits, point = decompose(x)
    if -4 <= point <= 16:
        if point <= 0:
            body = "0." + "0" * -point + digits
        elif point >= len(digits):
            body = digits + "0" * (point - len(digits)) + ".0"
        else:
            body = digits[:point] + "." + digits[point:]
    else:
        exp = point - 1
        body = digits[0] + ("." + digits[1:] if len(digits) > 1 else "") + "e" + ("+" if exp > 0 else "") + str(exp)
    return sign + body


ESCAPES = {'"': '\\"', "\\": "\\\\", "\b": "\\b", "\f": "\\f", "\n": "\\n", "\r": "\\r", "\t": "\\t"}


def string_text(s: str) -> str:
    out = ['"']
    for c in s:
        if c in ESCAPES:
            out.append(ESCAPES[c])
        elif ord(c) < 0x20:
            out.append("\\u%04x" % ord(c))
        else:
            out.append(c)
    out.append('"')
    return "".join(out)


def canonical(v) -> str:
    if v is None:
        return "null"
    if v is True:
        return "true"
    if v is False:
        return "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        return float_text(v)
    if isinstance(v, str):
        return string_text(v)
    if isinstance(v, list):
        return "[" + ",".join(canonical(x) for x in v) + "]"
    if isinstance(v, dict):
        keys = sorted(v, key=lambda k: k.encode("utf-8"))
        return "{" + ",".join(string_text(k) + ":" + canonical(v[k]) for k in keys) + "}"
    raise TypeError(type(v))


def digest(v, n: int) -> str:
    return hashlib.sha256(canonical(v).encode("utf-8")).hexdigest()[:n]


def stamp(v) -> str:
    return digest(v, 16)


def ident(v) -> str:
    return digest(v, 32)


# ── The canonical cases ─────────────────────────────────────────────────

VALUES: list[tuple[str, object]] = [
    ("boş nesne", {}),
    ("boş dizi", []),
    ("null", None),
    ("doğru", True),
    ("tam sayılar", [0, 1, -1, 42, 9007199254740993, -9223372036854775808, 18446744073709551615]),
    ("kayan noktalı sayılar", [1.0, -2.5, 0.1, 1e-5, 1e-6, 1e16, 1e15, -0.0, 0.0, 6.02e23, 5e-324]),
    ("anahtarlar UTF-8 baytlarıyla sıralı", {"b": 1, "a": 2, "B": 3, "ç": 4, "z": 5, "é": 6, "": 7, "😀": 8, "": 9}),
    ("iç içe", {"y": {"q": [1, {"b": None, "a": False}], "p": "x"}, "x": [[], {}]}),
    ("kaçışlar", "\"\\/\b\f\n\r\t\u0000\u0001\u001f\u007f 😀ş"),
    ("Türkçe metin", {"ad": "Çalışma alanı", "not": "ı İ ğ Ğ ü Ü ş Ş ö Ö ç Ç"}),
    ("sayının yazılışı anahtarda değil değerde", {"1": 1, "10": 1.0, "2": 2.5}),
]


def float_cases() -> list[str]:
    """Doubles by their bits, each "bits text": edges, the notation's
    borders on both sides, whole numbers, decimals people write, random
    magnitudes and random bit patterns."""
    rnd = random.Random(20261013)
    xs: list[float] = [
        1.0, 0.5, 0.1, 0.2, 0.3, 0.1 + 0.2, 1 / 3, 2 / 3, 123.456, 12345.678, 487010.123, 4420010.5,
        1e-4, 1e-5, 1e-6, 9.999999999999999e-6, 1.0000000000000001e-5, 1.2345e-5, 1.5e-7,
        1e15, 1e16, 9999999999999998.0, 9007199254740992.0, 9007199254740993.0, 2.5e16, 1.2345678901234568e17,
        2.2250738585072014e-308, 2.2250738585072009e-308, 5e-324, 1.7976931348623157e308,
        100.0, 1000.0, 0.001, 0.025, 1.005, 2.675, 1e21, 1e22, 1e23, 4.35, 0.07, 3.0e-5,
        -0.0, 0.0, -1.0, -1e-5, -1e-6, -1e16, -9999999999999998.0, -5e-324, -1.7976931348623157e308,
    ]
    for k in range(-8, 18):
        base = 10.0**k
        xs += [base, math.nextafter(base, 0.0), math.nextafter(base, math.inf)]
    for _ in range(400):
        xs.append(round(rnd.uniform(-1e7, 1e7), rnd.randint(0, 6)))
    for _ in range(300):
        xs.append(float(rnd.randint(-(2**60), 2**60)))
    while len(xs) < 2000:
        x = struct.unpack("<d", struct.pack("<Q", rnd.getrandbits(64)))[0]
        if math.isfinite(x):
            xs.append(x)
    for _ in range(1200):
        x = rnd.uniform(1.0, 10.0) * 10.0 ** rnd.randint(-12, 20)
        xs.append(-x if rnd.random() < 0.5 else x)
    out, seen = [], set()
    for x in xs:
        bits = struct.unpack("<Q", struct.pack("<d", x))[0]
        if bits not in seen:
            seen.add(bits)
            out.append(f"{bits:016x} {float_text(x)}")
    return out


# ── The drawing ─────────────────────────────────────────────────────────


def uid(n: int) -> str:
    return f"00000000-0000-4000-8000-{n:012x}"


STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def node(id: str, name: str, *, kind: str = "layer", locked: bool = False, children=()) -> dict:
    return {
        "id": id,
        "name": name,
        "type": kind,
        "visible": True,
        "locked": locked,
        "expanded": True,
        "style": dict(STYLE),
        "children": list(children),
    }


SETTINGS = {
    "srid": 5256,
    "lengthDecimals": 3,
    "areaDecimals": 2,
    "areaUnit": "m2",
    "angleUnit": "grad",
    "plotScale": 1000.0,
    "workspace": "cad",
    "drawingUnit": "m",
}

AGAC = uid(0xB1)
BLOCK = {
    "id": AGAC,
    "name": "Ağaç",
    "base": {"x": 0.0, "y": 0.0},
    "entities": [
        {"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": {"x": 0.0, "y": 0.0}, "r": 1.5},
    ],
}


def point(id: int, layer: str, x: float, y: float, **extra) -> dict:
    e = {"kind": "point", "id": id, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y}}
    e.update(extra)
    return e


def polygon(id: int, layer: str, pts, **extra) -> dict:
    e = {"kind": "polygon", "id": id, "layerId": layer, "attrs": {}, "pts": [{"x": x, "y": y} for x, y in pts]}
    e.update(extra)
    return e


P1, P2, PARSEL, YAZI, KILITLI, AGACLAR = uid(1), uid(2), uid(3), uid(4), uid(5), uid(6)

ENTITIES = [
    (P1, point(1, "yapi", 10.0, 20.0)),
    (P2, point(2, "yapi", 30.5, 40.25, color="#E5484D", attrs={"Ad": "N2", "Kod": "SN"})),
    (PARSEL, polygon(3, "parsel", [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)], attrs={"Ada": "101", "Parsel": "5"})),
    (
        YAZI,
        {
            "kind": "text",
            "id": 4,
            "layerId": "yazi",
            "attrs": {},
            "p": {"x": 20.0, "y": 15.0},
            "text": "101/5",
            "height": 2.0,
            "rotation": 0.0,
            "labelOf": PARSEL,
            "labelScale": 1000.0,
        },
    ),
    (KILITLI, point(5, "kilitli", 1.0, 1.0)),
    (AGACLAR, {"kind": "insert", "id": 6, "layerId": "yapi", "attrs": {}, "block": AGAC, "p": {"x": 50.0, "y": 50.0}, "scale": 1.0, "rotation": 0.0}),
]

SNAPSHOT = {
    "format": "kentos.document",
    "version": 2,
    "name": "Protokol",
    "settings": SETTINGS,
    "origin": {"x": 0.0, "y": 0.0},
    "layers": [
        node("0", "0"),
        node("yapi", "Yapı"),
        node("parsel", "Parsel"),
        node("yazi", "Yazı"),
        node("kilitli", "Kilitli", locked=True),
    ],
    "activeLayer": "yapi",
    "entities": [e for _, e in ENTITIES],
    "uids": [u for u, _ in ENTITIES],
    "styles": {"items": [], "categories": []},
    "blocks": [BLOCK],
}


def entity_value(e: dict) -> dict:
    v = copy.deepcopy(e)
    v.pop("id", None)
    return v


def entity_stamp(u: str, e: dict) -> str:
    return stamp({"uid": u, "entity": entity_value(e)})


def snapshot_stamps(s: dict) -> dict:
    return {
        "objects": {u: entity_stamp(u, e) for u, e in zip(s["uids"], s["entities"])},
        "blocks": {b["id"]: stamp(b) for b in s.get("blocks", [])},
        "layers": stamp({"nodes": s["layers"], "active": s["activeLayer"]}),
        "settings": stamp({"name": s["name"], "settings": s["settings"]}),
        "styles": stamp(s["styles"]),
        "sheets": stamp(None),
    }


STAMPS = snapshot_stamps(SNAPSHOT)
BY_UID = dict(ENTITIES)

# ── Plans (§5) ──────────────────────────────────────────────────────────


def bounds_of(e: dict) -> tuple[float, float, float, float] | None:
    if e["kind"] == "point":
        x, y = e["p"]["x"], e["p"]["y"]
        return (x, y, x, y)
    if e["kind"] == "polygon":
        xs = [p["x"] for p in e["pts"]]
        ys = [p["y"] for p in e["pts"]]
        return (min(xs), min(ys), max(xs), max(ys))
    return None


def plan(command: str, version: int, normalized: dict, touched: list[dict], *, named: set[str], blocks_read=(), label: str | None):
    """The plan's summary and id for a write that touched `touched`, in the
    order it touched them: {"uid", "before", "after"} (None: not there)."""
    sources_objects, stamps_objects = {}, {}
    added, changed, removed, dependents = [], [], [], []
    result_added, result_changed, result_removed = [], {}, []
    new_uids = []
    box = None
    for t in touched:
        u, before, after = t["uid"], t["before"], t["after"]
        for e in (before, after):
            if e is not None:
                b = bounds_of(e)
                if b is not None:
                    box = b if box is None else (min(box[0], b[0]), min(box[1], b[1]), max(box[2], b[2]), max(box[3], b[3]))
        if before is None:
            # Its stamp holds its id, which every write makes anew: "new" (16 hex digits there).
            stamps_objects[u] = "new"
            result_added.append(entity_value(after))
            added.append(u)
            new_uids.append(u)
        elif after is None:
            sources_objects[u] = entity_stamp(u, before)
            if u not in named:
                dependents.append(u)
            result_removed.append(u)
            removed.append(u)
        else:
            sources_objects[u] = entity_stamp(u, before)
            stamps_objects[u] = entity_stamp(u, after)
            if u not in named:
                dependents.append(u)
            result_changed[u] = entity_value(after)
            changed.append(u)
    sources_blocks = {b: STAMPS["blocks"][b] for b in blocks_read}
    plain = {k: v for k, v in normalized.items() if k != "expectedRevision"}
    essence = {
        "command": command,
        "version": version,
        "input": plain,
        "sources": {"objects": sources_objects, "blocks": sources_blocks, "layers": None, "settings": None, "styles": None, "sheets": None},
        "result": {
            "added": result_added,
            "changed": result_changed,
            "removed": result_removed,
            "blocks": {"added": [], "changed": {}, "removed": []},
            "layers": None,
            "settings": None,
            "styles": None,
            "sheets": None,
        },
    }
    renamed = rename(essence, {u: f"#a{i}" for i, u in enumerate(new_uids)})
    fresh = {u: f"#a{i}" for i, u in enumerate(new_uids)}
    summary = {
        "added": added,
        "changed": changed,
        "removed": removed,
        "dependents": dependents,
        "counts": {"added": len(added), "changed": len(changed), "removed": len(removed), "dependents": len(dependents)},
        "project": [],
        "undo": {"label": label, "outside": []},
        "sources": {"objects": sources_objects, "blocks": sources_blocks},
        "stamps": {"objects": stamps_objects},
    }
    if box is not None:
        summary["bounds"] = {"minX": box[0], "minY": box[1], "maxX": box[2], "maxY": box[3]}
    # A new object's id differs at every write: the summary says it as the essence does.
    return renamed, ident(renamed), rename(summary, fresh)


def rename(v, names: dict[str, str]):
    if isinstance(v, str):
        return names.get(v, v)
    if isinstance(v, list):
        return [rename(x, names) for x in v]
    if isinstance(v, dict):
        return {names.get(k, k): rename(x, names) for k, x in v.items()}
    return v


def moved(e: dict, dx: float, dy: float) -> dict:
    out = copy.deepcopy(e)
    if "p" in out:
        out["p"] = {"x": out["p"]["x"] + dx, "y": out["p"]["y"] + dy}
    if "pts" in out:
        out["pts"] = [{"x": p["x"] + dx, "y": p["y"] + dy} for p in out["pts"]]
    return out


def plan_cases() -> list[dict]:
    cases = []

    def add(name: str, envelope: dict, normalized: dict, touched, *, named, blocks_read=(), label, note: str):
        essence, plan_id, summary = plan(envelope["command"], envelope.get("version", 1), normalized, touched, named=named, blocks_read=blocks_read, label=label)
        cases.append(
            {"name": name, "note": note, "envelope": envelope, "normalized": normalized, "essence": essence, "planId": plan_id, "summary": summary}
        )

    move = {"uids": [P1], "transform": {"kind": "move", "dx": 2.5, "dy": -1.0}}
    add(
        "taşıma: kaynağın damgası, sonraki içerik",
        {"command": "cad.entities.transform", "op": "plan", "input": move},
        move,
        [{"uid": P1, "before": BY_UID[P1], "after": moved(BY_UID[P1], 2.5, -1.0)}],
        named={P1},
        label="Taşı",
        note="Değişen nesnenin önceki damgası sources'ta, sonraki içeriği result.changed'de; kutu önceki ve sonraki hâlin birleşimi.",
    )
    with_revision = dict(move, expectedRevision="0")
    add(
        "girdinin expectedRevision'ı planın kimliğine girmez",
        {"command": "cad.entities.transform", "op": "plan", "input": with_revision},
        with_revision,
        [{"uid": P1, "before": BY_UID[P1], "after": moved(BY_UID[P1], 2.5, -1.0)}],
        named={P1},
        label="Taşı",
        note="Aynı yazmanın planı, beklenen sürüm verilse de verilmese de aynı kimliği taşır.",
    )
    created = {"layerId": "yapi", "p": {"x": 7.0, "y": 8.0}}
    add(
        "yeni nesne: kimliği sırasıyla #a0",
        {"command": "cad.point.create", "op": "plan", "input": created},
        created,
        [{"uid": "yeni-nokta", "before": None, "after": point(0, "yapi", 7.0, 8.0)}],
        named=set(),
        label="Ekle",
        note="Yeni nesnenin kalıcı kimliği her yürütmede başkadır; planın özünde #a0 yazılır, böylece aynı plan aynı kimliği verir.",
    )
    color = {"uids": [P2], "operation": "color", "color": "#30A46C"}
    after_color = copy.deepcopy(BY_UID[P2])
    after_color["color"] = "#30A46C"
    add(
        "rengi değişen nokta",
        {"command": "cad.entities.set", "op": "plan", "input": color},
        color,
        [{"uid": P2, "before": BY_UID[P2], "after": after_color}],
        named={P2},
        label="Renk değiştir",
        note="Özniteliği değişen nesnenin özü: sonraki hâli bütünüyle.",
    )
    delete = {"uids": [PARSEL]}
    add(
        "silinen parsel ve ona bağlı yazı: izleyen",
        {"command": "cad.entities.delete", "op": "plan", "input": delete},
        delete,
        [
            {"uid": PARSEL, "before": BY_UID[PARSEL], "after": None},
            {"uid": YAZI, "before": BY_UID[YAZI], "after": None},
        ],
        named={PARSEL},
        label="Sil",
        note="Bağlı yazı nesnesiyle silinir (ADR 0175); girdide adı geçmediği için dependents'tadır. Kutu yalnız sınırı bilinen türlerden; bu durumda denetlenmez.",
    )
    cases[-1]["summary"].pop("bounds", None)
    insert_move = {"uids": [AGACLAR], "transform": {"kind": "move", "dx": 1.0, "dy": 1.0}}
    add(
        "blok yerleştirmesi: dayandığı tanımın damgası kaynaklarda",
        {"command": "cad.entities.transform", "op": "plan", "input": insert_move},
        insert_move,
        [{"uid": AGACLAR, "before": BY_UID[AGACLAR], "after": moved(BY_UID[AGACLAR], 1.0, 1.0)}],
        named={AGACLAR},
        blocks_read=[AGAC],
        label="Taşı",
        note="Tanım değişirse plan geçersizdir: tanımın damgası sources.blocks'ta.",
    )
    return cases


# ── Fingerprints (§6) ───────────────────────────────────────────────────


def fingerprint_cases() -> list[dict]:
    cases = []

    def add(name: str, envelope: dict, version: int, normalized: dict, note: str):
        value = {
            "command": envelope["command"],
            "version": version,
            "op": envelope["op"],
            "input": normalized,
            "expected": envelope.get("expected", {}),
        }
        cases.append({"name": name, "note": note, "envelope": envelope, "normalized": normalized, "fingerprint": ident(value)})

    p = {"layerId": "yapi", "p": {"x": 1.0, "y": 2.0}}
    add("nokta", {"command": "cad.point.create", "op": "execute", "requestId": "betik:1", "input": p}, 1, p, "Beklentisiz istek: expected {} sayılır.")
    add(
        "beklentili",
        {"command": "cad.point.create", "op": "execute", "requestId": "betik:2", "input": p, "expected": {"revision": "0", "layers": STAMPS["layers"]}},
        1,
        p,
        "Beklenenler parmak izine girer: aynı girdi başka beklentiyle başka istektir.",
    )
    m = {"uids": [P1], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}
    add(
        "sürüm verilmiş",
        {"command": "cad.entities.transform", "version": 1, "op": "execute", "requestId": "betik:3", "input": m},
        1,
        m,
        "Sürüm zarfta verilse de verilmese de komutun sürümüdür.",
    )
    return cases


# ── Envelopes, expectations and requests ───────────────────────────────


def envelope_cases() -> list[dict]:
    p = {"layerId": "yapi", "p": {"x": 1.0, "y": 2.0}}
    pc = "cad.point.create"

    def case(name, envelope, status, code, path, note):
        return {"name": name, "note": note, "envelope": envelope, "expect": {"status": status, "code": code, "path": path, "retryable": False, "count": 6}}

    return [
        case("validate ile istek kimliği", {"command": pc, "op": "validate", "requestId": "a", "input": p}, "failed", "invalid_call", "requestId", "validate ve plan bir şey yazmaz; istek kimliği yalnız execute ile."),
        case("plan ile istek kimliği", {"command": pc, "op": "plan", "requestId": "a", "input": p}, "failed", "invalid_call", "requestId", "Aynı kural."),
        case("boşluklu istek kimliği", {"command": pc, "op": "execute", "requestId": "iki sözcük", "input": p}, "failed", "invalid_call", "requestId", "Yalnız harf, rakam ve . _ : -"),
        case("129 karakterlik istek kimliği", {"command": pc, "op": "execute", "requestId": "a" * 129, "input": p}, "failed", "invalid_call", "requestId", "1–128 karakter."),
        case("boş istek kimliği", {"command": pc, "op": "execute", "requestId": "", "input": p}, "failed", "invalid_call", "requestId", "1–128 karakter."),
        case("plan ile beklenen plan", {"command": pc, "op": "plan", "input": p, "expected": {"plan": "0" * 32}}, "failed", "invalid_call", "expected.plan", "Beklenen plan yalnız execute ile."),
        case("31 haneli plan", {"command": pc, "op": "execute", "input": p, "expected": {"plan": "0" * 31}}, "failed", "invalid_call", "expected.plan", "32 küçük onaltılık hane."),
        case("büyük harfli plan", {"command": pc, "op": "execute", "input": p, "expected": {"plan": "A" * 32}}, "failed", "invalid_call", "expected.plan", "Küçük harf."),
        case("sayı olmayan sürüm", {"command": pc, "op": "execute", "input": p, "expected": {"revision": "x1"}}, "failed", "invalid_revision", "expected.revision", "Ondalık sayı metni."),
        case(
            "kimlik olmayan anahtar",
            {"command": pc, "op": "execute", "input": p, "expected": {"objects": {"nokta": "0" * 16}}},
            "failed",
            "invalid_call",
            "expected.objects.nokta",
            "Anahtar küçük harfli, tireli UUID.",
        ),
        case(
            "15 haneli damga",
            {"command": pc, "op": "execute", "input": p, "expected": {"objects": {P1: "0" * 15}}},
            "failed",
            "invalid_call",
            f"expected.objects.{P1}",
            "Damga 16 küçük onaltılık hane.",
        ),
        case("katmanların bozuk damgası", {"command": pc, "op": "execute", "input": p, "expected": {"layers": "XYZ"}}, "failed", "invalid_call", "expected.layers", "Proje parçalarının damgası da 16 hane."),
        case("olmayan sürüm", {"command": pc, "version": 99, "op": "execute", "input": p}, "failed", "unknown_version", "version", "Komutun sürümü 1."),
        case("olmayan komut", {"command": "cad.nothing", "op": "execute", "input": p}, "failed", "unknown_command", "command", "Katalogda yok."),
    ]


def expectation_cases() -> list[dict]:
    m = {"uids": [P1], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}
    tr = "cad.entities.transform"
    wrong = "0123456789abcdef"

    def case(name, expected, status, code=None, path=None, uids=None, retryable=None, note=""):
        expect = {"status": status, "count": 6}
        if code:
            expect.update(code=code, path=path, retryable=retryable)
        if uids is not None:
            expect["uids"] = uids
        return {"name": name, "note": note, "envelope": {"command": tr, "op": "execute", "input": m, "expected": expected}, "expect": expect}

    return [
        case("tutan damgalar", {"revision": "0", "objects": {P1: STAMPS["objects"][P1]}, "layers": STAMPS["layers"], "settings": STAMPS["settings"], "styles": STAMPS["styles"], "sheets": STAMPS["sheets"], "blocks": {AGAC: STAMPS["blocks"][AGAC]}}, "completed", note="Bütün beklentiler tutar: yazılır."),
        case("eski nesne damgası", {"objects": {P1: wrong}}, "conflict", "stale_state", f"expected.objects.{P1}", [P1], True, "Tutmayan nesne adıyla söylenir."),
        case("olmayan nesne", {"objects": {uid(0x99): wrong}}, "conflict", "stale_state", f"expected.objects.{uid(0x99)}", [uid(0x99)], True, "Artık olmayan nesne de tutmaz."),
        case("eski blok damgası", {"blocks": {AGAC: wrong}}, "conflict", "stale_state", f"expected.blocks.{AGAC}", [], True, "Tanım değişmişse yazılmaz."),
        case("eski katman damgası", {"layers": wrong}, "conflict", "stale_state", "expected.layers", [], True, "Proje parçası da damgasıyla."),
        case("eski ayar damgası", {"settings": wrong}, "conflict", "stale_state", "expected.settings", [], True, ""),
        case("eski sürüm", {"revision": "7"}, "conflict", "revision_conflict", "expected.revision", None, True, "Sürüm önce denetlenir."),
    ]


def request_cases() -> list[dict]:
    """Sequences: a request sent again, reused, undone; a refusal not kept."""
    pc = "cad.point.create"
    p = {"layerId": "yapi", "p": {"x": 1.0, "y": 2.0}}
    q = {"layerId": "yapi", "p": {"x": 3.0, "y": 4.0}}
    locked = {"layerId": "kilitli", "p": {"x": 1.0, "y": 2.0}}
    return [
        {
            "name": "aynı istek bir kez uygulanır",
            "note": "İkinci gönderim ilk cevabı verir (replayed); çizim değişmez.",
            "steps": [
                {"call": {"command": pc, "op": "execute", "requestId": "r1", "input": p}, "expect": {"status": "completed", "replayed": False, "count": 7}},
                {"call": {"command": pc, "op": "execute", "requestId": "r1", "input": p}, "expect": {"status": "completed", "replayed": True, "sameOutputAs": 0, "count": 7}},
            ],
        },
        {
            "name": "kimlik başka iş için kullanılamaz",
            "note": "Başka girdiyle aynı kimlik: request_reused, hiçbir şey yazılmaz.",
            "steps": [
                {"call": {"command": pc, "op": "execute", "requestId": "r2", "input": p}, "expect": {"status": "completed", "count": 7}},
                {"call": {"command": pc, "op": "execute", "requestId": "r2", "input": q}, "expect": {"status": "failed", "code": "request_reused", "path": "requestId", "retryable": False, "count": 7}},
            ],
        },
        {
            "name": "geri alınan istek geri alındığını söyler",
            "note": "Geri alınmış isteğin cevabı undone: true; yeniden uygulanmaz.",
            "steps": [
                {"call": {"command": pc, "op": "execute", "requestId": "r3", "input": p}, "expect": {"status": "completed", "count": 7}},
                {"undo": "Ekle", "expect": {"count": 6}},
                {"call": {"command": pc, "op": "execute", "requestId": "r3", "input": p}, "expect": {"status": "completed", "replayed": True, "undone": True, "count": 6}},
            ],
        },
        {
            "name": "ret deftere girmez",
            "note": "Reddedilen istek yeniden denenebilir: aynı kimlikle başka girdi request_reused değildir.",
            "steps": [
                {"call": {"command": pc, "op": "execute", "requestId": "r4", "input": locked}, "expect": {"status": "failed", "code": "layer_locked", "count": 6}},
                {"call": {"command": pc, "op": "execute", "requestId": "r4", "input": p}, "expect": {"status": "completed", "replayed": False, "count": 7}},
            ],
        },
        {
            "name": "plan onaylanmış hâliyle yazılır",
            "note": "Planın kimliği beklenirse yazılan plana eşittir; araya giren yazma kaynağı değiştirince plan_changed.",
            "steps": [
                {"call": {"command": "cad.entities.transform", "op": "plan", "input": {"uids": [P1], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}}, "expect": {"status": "completed", "count": 6}, "keepPlan": "a"},
                {"call": {"command": "cad.entities.transform", "op": "execute", "input": {"uids": [P1], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}, "expected": {"plan": "$a"}}, "expect": {"status": "completed", "planId": "$a", "count": 6}},
                {"call": {"command": "cad.entities.transform", "op": "plan", "input": {"uids": [P1], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}}, "expect": {"status": "completed"}, "keepPlan": "b"},
                {"call": {"command": "cad.entities.transform", "op": "execute", "input": {"uids": [P1], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}}, "expect": {"status": "completed"}},
                {"call": {"command": "cad.entities.transform", "op": "execute", "input": {"uids": [P1], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}, "expected": {"plan": "$b"}}, "expect": {"status": "conflict", "code": "plan_changed", "path": "expected.plan", "retryable": True}},
            ],
        },
    ]


# ── The file ────────────────────────────────────────────────────────────


def build() -> dict:
    return {
        "format": "kentos.protocol-cases",
        "version": 1,
        "note": "docs/adr/0253: kanonik JSON, damgalar, parmak izi, planın kimliği ve özeti, zarfın, beklentilerin ve isteklerin cevapları. scripts/fixtures/protocol_cases.py yazar; KentOS kodu kullanılmaz.",
        "canonical": [{"name": n, "value": v, "text": canonical(v), "stamp": stamp(v), "id": ident(v)} for n, v in VALUES],
        "floats": float_cases(),
        "snapshot": SNAPSHOT,
        "stamps": STAMPS,
        "plans": plan_cases(),
        "fingerprints": fingerprint_cases(),
        "envelopes": envelope_cases(),
        "expectations": expectation_cases(),
        "requests": request_cases(),
    }


def text_of(data: dict) -> str:
    return json.dumps(data, ensure_ascii=False, indent=1) + "\n"


def self_checks() -> None:
    """The rule's own examples (canonical.rs's unit tests say the same)."""
    # Pairs, not a dict: 0.0 and -0.0 are one key of a dict.
    want = [
        (1.0, "1.0"), (0.1, "0.1"), (-2.5, "-2.5"), (1e-4, "0.0001"), (1e-5, "0.00001"), (1.2345e-5, "0.000012345"),
        (1e-6, "1e-6"), (1.5e-7, "1.5e-7"), (12345.678, "12345.678"), (1e15, "1000000000000000.0"),
        (9999999999999998.0, "9999999999999998.0"), (1e16, "1e+16"), (2.5e16, "2.5e+16"),
        (1.2345678901234568e17, "1.2345678901234568e+17"), (0.0, "0.0"), (-0.0, "-0.0"),
        (2.2250738585072014e-308, "2.2250738585072014e-308"), (1.7976931348623157e308, "1.7976931348623157e+308"),
    ]
    for x, text in want:
        got = float_text(x)
        if got != text:
            sys.exit(f"float_text({x!r}) = {got}, beklenen {text}")
    for item in float_cases():
        bits, text = item.split(" ")
        x = struct.unpack("<d", struct.pack("<Q", int(bits, 16)))[0]
        if float(text) != x or math.copysign(1, float(text)) != math.copysign(1, x):
            sys.exit(f"{text} {x!r} sayısını geri vermiyor")
    if canonical({"b": 1, "a": [True, -3], "ç": 'ş"\n\u0001', "A": None}) != '{"A":null,"a":[true,-3],"b":1,"ç":"ş\\"\\n\\u0001"}':
        sys.exit("kanonik metin canonical.rs'in örneğini vermiyor")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="dosyayı yazma; farklıysa düş")
    args = parser.parse_args()
    self_checks()
    text = text_of(build())
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/protocol_cases.py")
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")


if __name__ == "__main__":
    main()
