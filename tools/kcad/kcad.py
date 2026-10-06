#!/usr/bin/env python3
"""Independent reader of KCAD v2, the KentOS project file (`.kcad`).

Written from docs/specs/kcad-v2.md with Python's standard library only, never
from KentOS code, so the file format is held to an outside implementation
(TODOS.md FILE-23, docs/adr/0025): the Rust codec (crates/shared/kcad), its
browser build (crates/wasm/formats-wasm) and this reader read the same
fixtures (fixtures/kcad/v2, expected.json) and must agree on every one.

    python3 tools/kcad/kcad.py inspect FILE       # header and a summary of the drawing
    python3 tools/kcad/kcad.py validate FILE...   # valid, or the error code and its message
    python3 tools/kcad/kcad.py dump FILE          # the drawing as JSON, in the contract's form
    python3 tools/kcad/kcad.py sniff FILE...      # what a file is by its content

Exit status: 0 when every file is valid, 1 when one is not, 2 on a usage error.

The reader checks the container (signature, versions, lengths, SHA-256),
decodes the payload by the KentOS CBOR profile (definite lengths, shortest
integers, binary64 floats only, text keys in RFC 8949 §4.2.1 order, no
duplicates, no tags, size and depth limits) and then checks the document
schema, the block rules with it (§6.9). The document rules the apps add on
top (layer references, known CRS, vertex counts; spec §6.11) are not checked
here.
"""

import hashlib
import json
import math
import re
import struct
import sys
from collections import Counter

# ── Container (spec §2, §3) ─────────────────────────────────────────────

MAGIC = b"\x89KCAD\r\n\x1a\n"
MAGIC_PREFIX = MAGIC[:5]
FIXED_HEADER = 36
MAJOR = 2
MINOR = 0
ENCODING_CBOR_PROFILE_1 = 1
CODEC_NONE = 0
HASH_SIZE = 32
MAX_PAYLOAD = 1 << 30
MAX_EXTENSIONS = 64
MAX_EXTENSION_NAME = 64
EXTENSION_CHARS = set(b"abcdefghijklmnopqrstuvwxyz0123456789._-")

# ── CBOR profile 1 (spec §5) ────────────────────────────────────────────

MAX_DEPTH = 64
MAX_ITEMS = 1 << 24
MAX_STRING = 1 << 24

DOCUMENT_FORMAT = "kentos.document"
DOCUMENT_VERSION = 2
# Schema 3: schema 2 and an object's own line weight (`lineWeight`, mm; docs/adr/0139).
SCHEMA_WITH_LINE_WEIGHTS = 3
MAX_LINE_WEIGHT = 100.0
# Schema 4: schema 3 and vertex elevations (`za`, `zb` of a line, `zs` of a polyline, a polygon and a hole; docs/adr/0142).
SCHEMA_WITH_ELEVATIONS = 4
# Schema 5: schema 4 and multi-part areas (`parts` of a polygon; docs/adr/0143).
SCHEMA_WITH_PARTS = 5
# Schema 6: schema 5 and blocks (the document's `blocks`, the `insert` kind; docs/adr/0144).
SCHEMA_WITH_BLOCKS = 6
# Schema 7: schema 6 and a text's `align`, `widthFactor` and `mask`, an attribute definition's `align` and
# `widthFactor` (docs/adr/0145).
SCHEMA_WITH_TEXT_EXTRAS = 7
# Schema 8: schema 7 and the `leader` kind (docs/adr/0146).
SCHEMA_WITH_LEADERS = 8
# Schema 9: schema 8 and the dimension's new kinds, its `mask`, a slope's `za` and `zb` (docs/adr/0147).
SCHEMA_WITH_DIMENSIONS = 9
# Schema 10: schema 9 and a layer's own snapping, a layer node's `snap` (docs/adr/0163 §4).
SCHEMA_WITH_LAYER_SNAP = 10
# Schema 11: schema 10 and a local project's drawing unit, the settings' `drawingUnit` (docs/adr/0165 §2).
SCHEMA_WITH_DRAWING_UNIT = 11
# Schema 12: schema 11 and the project's second coordinate system, the settings' `secondSrid` (docs/adr/0167 §1).
SCHEMA_WITH_SECOND_SRID = 12
# Schema 13: schema 12 and the project's own coordinate systems and datum choices, the settings' `customCrs`,
# `secondCustomCrs` and `datumTransforms` (docs/adr/0168).
SCHEMA_WITH_CUSTOM_CRS = 13
# Schema 14: schema 13 and the project's survey constants and tolerances, the settings' `survey` (docs/adr/0169 §3).
SCHEMA_WITH_SURVEY = 14
# Schema 15: schema 14 and the survey settings' traverse tolerances, `twoWay`, `traverseAngle`, `traverseCoord` (docs/adr/0169 §3).
SCHEMA_WITH_TRAVERSE_TOLERANCES = 15
# Schema 16: schema 15 and the survey settings' ground, `groundHeight` and `reduceToGrid` (docs/adr/0171 §2, §4).
SCHEMA_WITH_GROUND = 16
# Schema 17: schema 16 and multi-part polylines and points, a polyline's and a point's `parts` (docs/adr/0174).
SCHEMA_WITH_LINE_PARTS = 17
# Schema 18: schema 17 and the text that writes an object's label, a text's `labelOf` and `labelScale` (docs/adr/0175 §4).
SCHEMA_WITH_LINKED_TEXTS = 18
# Schema 19: schema 18 and the project's named layer states, the settings' `layerStates` (docs/adr/0177 §4).
SCHEMA_WITH_LAYER_STATES = 19
# Schema 20: schema 19 and a multi-line text's `boxWidth`, `lineSpacing` and `runs` (docs/adr/0182 §1).
SCHEMA_WITH_PARAGRAPHS = 20
# A multi-line text's line spacing's bounds (AutoCAD's own).
MIN_LINE_SPACING = 0.25
MAX_LINE_SPACING = 4.0
REGISTRY_DATUMS = ("TUREF", "ED50", "WGS84")
# The kinds a layer may keep to; `endpoint` brings the quadrants with it, so `quadrant` is none of them.
LAYER_SNAP_KINDS = ("endpoint", "midpoint", "center", "node", "intersection", "perpendicular", "tangent", "nearest", "centroid", "extension", "parallel", "grid")
# A dimension's kinds; schema 9 added the last five.
DIMENSION_STYLES = ("aligned", "linear", "angular", "radius", "diameter", "ordinate", "arcLength", "jogged", "azimuth", "slope")
# A leader's arrowheads (spec §6.6); the filled arrow is the field's absence, no value.
LEADER_ARROWS = ("open", "dot", "none")
# A text's alignments (spec §6.6); the left of the baseline is the field's absence, no value.
TEXT_ALIGNS = ("baselineCenter", "baselineRight", "bottomLeft", "bottomCenter", "bottomRight", "middleLeft", "middleCenter", "middleRight", "topLeft", "topCenter", "topRight")
# The widest a text's letters may be drawn, times their width.
MAX_WIDTH_FACTOR = 100.0
MAX_BLOCK_DEPTH = 16
SCHEMAS = (DOCUMENT_VERSION, SCHEMA_WITH_LINE_WEIGHTS, SCHEMA_WITH_ELEVATIONS, SCHEMA_WITH_PARTS, SCHEMA_WITH_BLOCKS, SCHEMA_WITH_TEXT_EXTRAS, SCHEMA_WITH_LEADERS, SCHEMA_WITH_DIMENSIONS, SCHEMA_WITH_LAYER_SNAP, SCHEMA_WITH_DRAWING_UNIT, SCHEMA_WITH_SECOND_SRID, SCHEMA_WITH_CUSTOM_CRS, SCHEMA_WITH_SURVEY, SCHEMA_WITH_TRAVERSE_TOLERANCES, SCHEMA_WITH_GROUND, SCHEMA_WITH_LINE_PARTS, SCHEMA_WITH_LINKED_TEXTS, SCHEMA_WITH_LAYER_STATES, SCHEMA_WITH_PARAGRAPHS)
# Unicode's White_Space characters: a block's name is not made of these alone.
WHITE_SPACE = set("\t\n\x0b\x0c\r \x85\xa0\u1680\u2028\u2029\u202f\u205f\u3000") | {chr(c) for c in range(0x2000, 0x200B)}


# ── The project's coordinate systems' rules (spec §6.4.1; kentos_contracts::crs) ────────────────────────────────


def finite_all(values):
    return all(math.isfinite(x) for x in values)


def helmert_problem(h):
    if not finite_all(h["translation"] + h["rotation"] + [h["scale"]]):
        return "yedi parametre sonlu sayılar olmalı"
    if "accuracy" in h and not (math.isfinite(h["accuracy"]) and h["accuracy"] >= 0):
        return "doğruluk 0 ya da büyük bir sayı olmalı"
    return None


def datum_problem(s):
    """A definition's datum: exactly one of `datum` and `customDatum`, the project's by its rules."""
    if ("datum" in s) == ("customDatum" in s):
        return "datum ya kayıttaki bir ad ya projenin datumu olur, yalnız biri"
    if "datum" in s:
        return None
    d = s["customDatum"]
    e = d["ellipsoid"]
    if not d["name"].strip() or not e["name"].strip():
        return "datumun ve elipsoidin adı boş olamaz"
    ok = math.isfinite(e["semiMajor"]) and e["semiMajor"] > 0 and math.isfinite(e["inverseFlattening"]) and e["inverseFlattening"] > 1
    if not ok:
        return "elipsoidin büyük yarı ekseni 0'dan, ters basıklığı 1'den büyük olmalı"
    return helmert_problem(d["toWgs84"]) if "toWgs84" in d else None


def plane_problem(p):
    if p["kind"] == "similarity":
        if not finite_all([p["east"], p["north"], p["rotation"], p["scale"]]) or p["scale"] <= 0:
            return "benzerliğin değerleri sonlu, ölçeği 0'dan büyük olmalı"
        return None
    if not finite_all([p[k] for k in "abcdef"]) or p["a"] * p["e"] - p["b"] * p["d"] == 0:
        return "afinin katsayıları sonlu olmalı, düzlemi katlamamalı"
    return None


def crs_problem(d, base=False):
    """What is wrong with a definition (kentos_contracts::CrsDefinition::problem), or None."""
    if not d["name"].strip():
        return "tanımın adı boş olamaz"
    s = d["system"]
    if s["kind"] == "tm":
        lat0 = s.get("latitudeOfOrigin", 0.0)
        if not finite_all([lat0, s["centralMeridian"], s["scaleFactor"], s["falseEasting"], s["falseNorthing"]]):
            return "izdüşümün değerleri sonlu sayılar olmalı"
        if s["scaleFactor"] <= 0:
            return "ölçek 0'dan büyük olmalı"
        if abs(s["centralMeridian"]) > 180 or abs(lat0) > 90:
            return "orta meridyen ya da başlangıç enlemi aralık dışında"
        return datum_problem(s)
    if base:
        return "yerel sistemin tabanı bir TM izdüşümü olmalı"
    if s["kind"] == "geographic":
        return datum_problem(s)
    b = s["base"]
    if ("srid" in b) == ("definition" in b):
        return "yerel sistemin tabanı ya bir EPSG kodu ya bir tanımdır, yalnız biri"
    if "srid" in b:
        return "yerel sistemin tabanı kayıttaki bir sistem olmalı" if b["srid"] == 0 else plane_problem(s["plane"])
    return crs_problem(b["definition"], True) or plane_problem(s["plane"])


def choices_problem(choices):
    """What is wrong with a project's datum choices together (kentos_contracts::crs::choices_problem), or None."""
    pairs = []
    for t in choices:
        if t["from"] == t["to"]:
            return "datum seçimi iki ayrı datum arasında olur"
        if not t["name"].strip():
            return "datum seçiminin adı boş olamaz"
        if ("helmert" in t) == ("grid" in t):
            return "datum seçimi ya yedi parametre ya ızgaradır, yalnız biri"
        if "helmert" in t:
            problem = helmert_problem(t["helmert"])
            if problem:
                return problem
        else:
            g = t["grid"]
            if not re.fullmatch(r"[0-9a-f]{64}", g["id"]):
                return "ızgaranın kimliği küçük harfli 64 onaltılık rakamla SHA-256 olmalı"
            if g["size"] == 0:
                return "ızgaranın boyu 0 olamaz"
            if "accuracy" in g and not (math.isfinite(g["accuracy"]) and g["accuracy"] >= 0):
                return "doğruluk 0 ya da büyük bir sayı olmalı"
        pair = frozenset((t["from"], t["to"]))
        if pair in pairs:
            return "bir datum çifti için en çok bir seçim olur"
        pairs.append(pair)
    return None


class KcadError(Exception):
    """A file the reader refuses: `code` is from the specification's table (§9)."""

    def __init__(self, code, message):
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


def sniff(data):
    """What a file is by content (spec §8): kcad, kcad-damaged, json, empty or foreign."""
    if data[: len(MAGIC)] == MAGIC:
        return "kcad"
    if data[: len(MAGIC_PREFIX)] == MAGIC_PREFIX:
        return "kcad-damaged"
    rest = data[3:] if data[:3] == b"\xef\xbb\xbf" else data
    rest = rest.lstrip(b" \t\r\n")
    if not rest:
        return "empty"
    if rest[:1] == b"{":
        return "json"
    return "foreign"


def _u16(data, at):
    return int.from_bytes(data[at : at + 2], "little")


def _u64(data, at):
    return int.from_bytes(data[at : at + 8], "little")


def read_header(data):
    """Checks the container in the specification's order (§4); returns the header and the payload."""
    size = len(data)
    if size == 0:
        raise KcadError("empty", "Dosya boş; içinde çizim yok.")
    if data[: len(MAGIC)] != MAGIC:
        if data[: len(MAGIC_PREFIX)] == MAGIC_PREFIX:
            if size < len(MAGIC) and MAGIC.startswith(bytes(data)):
                raise KcadError("truncated", f"Dosya eksik: {size} bayt, imza bile tamamlanmıyor.")
            raise KcadError("damaged_signature", "KCAD imzası bozuk: dosya metin olarak aktarılmış (satır sonları değişmiş) olabilir.")
        raise KcadError("not_kcad", "KCAD (v2) çizim dosyası değil: imza yok.")
    if size < FIXED_HEADER:
        raise KcadError("truncated", f"Dosya eksik: başlık {FIXED_HEADER} bayt olmalı, dosya {size} bayt.")
    major, minor, min_reader = data[9], data[10], data[11]
    if major != MAJOR:
        raise KcadError("unsupported_version", f"KCAD ana sürümü {major}; bu okuyucu yalnız {MAJOR}'yi okur.")
    if min_reader > MINOR:
        raise KcadError("newer_version", f"Dosya en az KCAD {MAJOR}.{min_reader} okuyucusu istiyor; bu okuyucu {MAJOR}.{MINOR}.")
    header_length = _u16(data, 12)
    encoding, codec = data[14], data[15]
    payload_length = _u64(data, 16)
    decoded_length = _u64(data, 24)
    flags = _u16(data, 32)
    count = _u16(data, 34)
    if header_length < FIXED_HEADER:
        raise KcadError("bad_header", f"Başlık uzunluğu {header_length}; en az {FIXED_HEADER} olmalı.")
    if payload_length > MAX_PAYLOAD or decoded_length > MAX_PAYLOAD:
        raise KcadError("too_large", f"Yük {max(payload_length, decoded_length)} bayt; sınır {MAX_PAYLOAD}.")
    total = header_length + payload_length + HASH_SIZE
    if size < total:
        raise KcadError("truncated", f"Dosya eksik: {total} bayt olmalı, {size} bayt var.")
    if size > total:
        raise KcadError("trailing_data", f"Dosyanın sonunda {size - total} fazla bayt var.")
    if hashlib.sha256(data[: size - HASH_SIZE]).digest() != bytes(data[size - HASH_SIZE :]):
        raise KcadError("hash_mismatch", "SHA-256 özeti tutmuyor: dosya bozulmuş.")
    if minor < min_reader:
        raise KcadError("bad_header", f"Küçük sürüm {minor}, en az okuyucu sürümünden ({min_reader}) küçük.")
    if flags != 0:
        raise KcadError("bad_header", f"Bilinmeyen başlık bayrakları: {flags:#06x}.")
    if count > MAX_EXTENSIONS:
        raise KcadError("bad_header", f"{count} zorunlu uzantı; en çok {MAX_EXTENSIONS}.")
    extensions = []
    at = FIXED_HEADER
    for _ in range(count):
        if at >= header_length:
            raise KcadError("bad_header", "Uzantı listesi başlığın dışına taşıyor.")
        length = data[at]
        name = bytes(data[at + 1 : at + 1 + length])
        if not 1 <= length <= MAX_EXTENSION_NAME or at + 1 + length > header_length or not set(name) <= EXTENSION_CHARS:
            raise KcadError("bad_header", "Uzantı adı geçersiz.")
        extensions.append(name.decode("ascii"))
        at += 1 + length
    if at != header_length:
        raise KcadError("bad_header", "Başlık uzunluğu uzantı listesiyle bitmiyor.")
    if encoding != ENCODING_CBOR_PROFILE_1:
        raise KcadError("unknown_encoding", f"Yük kodlaması {encoding} tanımsız.")
    if codec != CODEC_NONE:
        raise KcadError("unknown_codec", f"Sıkıştırma kodeki {codec} tanımsız.")
    if decoded_length != payload_length:
        raise KcadError("bad_header", "Sıkıştırmasız yükte çözülmüş uzunluk yük uzunluğuna eşit olmalı.")
    if extensions:
        raise KcadError("unknown_extension", f"Tanınmayan zorunlu uzantı: {', '.join(extensions)}.")
    header = {
        "major": major,
        "minor": minor,
        "minReader": f"{MAJOR}.{min_reader}",
        "headerLength": header_length,
        "encoding": encoding,
        "codec": codec,
        "payloadLength": payload_length,
        "sha256": bytes(data[size - HASH_SIZE :]).hex(),
    }
    return header, bytes(data[header_length : header_length + payload_length])


class _Cbor:
    """A decoder of the KentOS CBOR profile: every rule of spec §5 is checked while reading."""

    def __init__(self, data):
        self.data = data
        self.pos = 0

    def fail(self, code, message):
        raise KcadError(code, f"{message} (yükün {self.pos}. baytı)")

    def need(self, n):
        if self.pos + n > len(self.data):
            self.fail("cbor_truncated", "Yük bir öğenin ortasında bitiyor")

    def head(self):
        self.need(1)
        initial = self.data[self.pos]
        self.pos += 1
        major, info = initial >> 5, initial & 31
        if info < 24:
            return major, info, info
        if info in (28, 29, 30):
            self.fail("malformed", f"Ayrılmış ek bilgi {info}")
        if info == 31:
            if major in (2, 3, 4, 5):
                self.fail("indefinite_length", "Belirsiz uzunluk kullanılamaz")
            self.fail("malformed", "Yerinde olmayan belirsiz uzunluk ya da kırma baytı")
        if major == 7:
            return major, info, None
        size = {24: 1, 25: 2, 26: 4, 27: 8}[info]
        self.need(size)
        arg = int.from_bytes(self.data[self.pos : self.pos + size], "big")
        self.pos += size
        if arg < {24: 24, 25: 256, 26: 65536, 27: 1 << 32}[info]:
            self.fail("non_shortest", f"{arg} en kısa biçimde yazılmamış")
        return major, info, arg

    def item(self, depth):
        major, info, arg = self.head()
        if major == 0:
            return arg
        if major == 1:
            return -1 - arg
        if major in (2, 3):
            if arg > MAX_STRING:
                self.fail("too_long", f"{arg} baytlık dizgi; sınır {MAX_STRING}")
            self.need(arg)
            raw = self.data[self.pos : self.pos + arg]
            self.pos += arg
            if major == 2:
                return bytes(raw)
            try:
                return bytes(raw).decode("utf-8")
            except UnicodeDecodeError:
                self.fail("invalid_utf8", "Metin geçerli UTF-8 değil")
        if major in (4, 5):
            if depth > MAX_DEPTH:
                self.fail("too_deep", f"İç içe derinlik {MAX_DEPTH}'ı aşıyor")
            if arg > MAX_ITEMS:
                self.fail("too_long", f"{arg} öğe; sınır {MAX_ITEMS}")
            if (arg if major == 4 else 2 * arg) > len(self.data) - self.pos:
                self.fail("cbor_truncated", f"{arg} öğe yükte kalan baytlara sığmıyor")
            if major == 4:
                return [self.item(depth + 1) for _ in range(arg)]
            result = {}
            previous = None
            for _ in range(arg):
                self.need(1)
                if self.data[self.pos] >> 5 != 3:
                    self.fail("non_text_key", "Harita anahtarı metin değil")
                start = self.pos
                key = self.item(depth + 1)
                raw = bytes(self.data[start : self.pos])
                if previous is not None:
                    if raw == previous:
                        self.fail("duplicate_key", f"“{key}” anahtarı iki kez yazılmış")
                    if raw < previous:
                        self.fail("unsorted_keys", f"“{key}” anahtarı sırasında değil")
                previous = raw
                result[key] = self.item(depth + 1)
            return result
        if major == 6:
            self.fail("tag", "CBOR etiketi kullanılamaz")
        if info == 20:
            return False
        if info == 21:
            return True
        if info == 22:
            return None
        if info in (25, 26):
            self.fail("narrow_float", "Kayan noktalı sayı binary64 (8 bayt) yazılmalı")
        if info == 27:
            self.need(8)
            (value,) = struct.unpack(">d", self.data[self.pos : self.pos + 8])
            self.pos += 8
            if math.isnan(value) or math.isinf(value):
                self.fail("non_finite", "NaN ya da sonsuz sayı yazılamaz")
            return value
        self.fail("simple_value", "Bu basit değer kullanılamaz (yalnız true, false, null)")


def decode_payload(payload):
    """The payload as Python values: dict, list, str, bytes, int, float, bool, None."""
    decoder = _Cbor(payload)
    value = decoder.item(1)
    if decoder.pos != len(payload):
        decoder.fail("cbor_trailing", "Yükte belgeden sonra fazladan bayt var")
    return value


# ── Document schemas 2 to 10 (spec §6) ──────────────────────────────────


class _Schema:
    """Checks the decoded payload and turns it into the contract's JSON form (`DocumentSnapshotV2`)."""

    def __init__(self):
        self.path = []
        self.uids = []
        self.seen = set()
        # What the payload's schema lets an object hold: its own line weight (schema 3 and up), vertex elevations
        # (schema 4 and up), an area's parts (schema 5 and up), blocks (schema 6 and up), a text's extras (schema 7).
        self.weights = False
        self.elevations = False
        self.parts = False
        self.line_parts = False
        self.linked_texts = False
        self.paragraphs = False
        self.blocks = False
        self.texts = False
        # While a block definition's objects are read: how many so far (they have no persistent ids).
        self.inside = None
        # The definitions' ids and places, once read; where each insert inside them is.
        self.index = {}

    def where(self):
        return "/".join(self.path) or "(kök)"

    def fail(self, code, message):
        raise KcadError(code, f"{self.where()}: {message}")

    def at(self, segment, check, value):
        self.path.append(str(segment))
        try:
            return check(value)
        finally:
            self.path.pop()

    # Scalars.

    def text(self, v):
        if type(v) is not str:
            self.fail("wrong_type", "metin olmalı")
        return v

    def float(self, v):
        if type(v) is not float:
            self.fail("wrong_type", "float64 olmalı")
        return v

    def uint(self, bits):
        def check(v):
            if type(v) is not int:
                self.fail("wrong_type", "tam sayı olmalı")
            if not 0 <= v < (1 << bits):
                self.fail("bad_value", f"{v} aralık dışında (0…{(1 << bits) - 1})")
            return v

        return check

    def bool(self, v):
        if type(v) is not bool:
            self.fail("wrong_type", "true ya da false olmalı")
        return v

    def enum(self, values):
        def check(v):
            self.text(v)
            if v not in values:
                self.fail("bad_value", f"“{v}” bilinmiyor ({', '.join(values)})")
            return v

        return check

    def id16(self, v):
        if type(v) is not bytes:
            self.fail("wrong_type", "16 baytlık bayt dizgisi olmalı")
        if len(v) != 16:
            self.fail("bad_value", f"{len(v)} bayt; kimlik 16 bayt olmalı")
        if v == bytes(16):
            self.fail("bad_value", "kimlik boş (sıfır) olamaz")
        return uuid_text(v)

    def sha256(self, v):
        if type(v) is not bytes:
            self.fail("wrong_type", "32 baytlık bayt dizgisi olmalı")
        if len(v) != 32:
            self.fail("bad_value", f"{len(v)} bayt; SHA-256 32 bayt olmalı")
        return v.hex()

    # Compounds.

    def array(self, item):
        def check(v):
            if type(v) is not list:
                self.fail("wrong_type", "dizi olmalı")
            return [self.at(i, item, x) for i, x in enumerate(v)]

        return check

    def point(self, v):
        if type(v) is not list:
            self.fail("wrong_type", "nokta [x, y] dizisi olmalı")
        if len(v) != 2:
            self.fail("bad_value", f"nokta 2 sayı olmalı, {len(v)} var")
        return {"x": self.at(0, self.float, v[0]), "y": self.at(1, self.float, v[1])}

    def bounds(self, v):
        if type(v) is not list:
            self.fail("wrong_type", "sınırlar [minX, minY, maxX, maxY] dizisi olmalı")
        if len(v) != 4:
            self.fail("bad_value", f"sınırlar 4 sayı olmalı, {len(v)} var")
        names = ("minX", "minY", "maxX", "maxY")
        return {n: self.at(i, self.float, x) for i, (n, x) in enumerate(zip(names, v))}

    def attrs(self, v):
        if type(v) is not dict:
            self.fail("wrong_type", "harita olmalı")
        return {k: self.at(k, self.text, x) for k, x in v.items()}

    def any(self, v):
        """An opaque part: JSON-compatible values, integers within JSON's 64-bit range."""
        if v is None or type(v) in (bool, str, float):
            return v
        if type(v) is int:
            if not -(1 << 63) <= v < (1 << 64):
                self.fail("bad_value", f"{v} 64 bit tam sayı aralığının dışında")
            return v
        if type(v) is list:
            return [self.at(i, self.any, x) for i, x in enumerate(v)]
        if type(v) is dict:
            return {k: self.at(k, self.any, x) for k, x in v.items()}
        self.fail("wrong_type", "JSON'la gösterilebilen bir değer olmalı (bayt dizgisi olamaz)")

    def not_null(self, v):
        if v is None:
            self.fail("bad_value", "null yazılmaz; değer yoksa alan hiç yazılmaz")
        return self.any(v)

    def fields(self, table, rename=None):
        """A map with the given fields: {key: (check, required)}; unknown keys and missing required ones are refused."""

        def check(v):
            if type(v) is not dict:
                self.fail("wrong_type", "harita olmalı")
            for key in v:
                if key not in table:
                    self.path.append(key)
                    self.fail("unknown_field", "bilinmeyen alan")
            for key, (_, required) in table.items():
                if required and key not in v:
                    self.path.append(key)
                    self.fail("missing_field", "zorunlu alan yok")
            return {(rename or {}).get(k, k): self.at(k, table[k][0], x) for k, x in v.items()}

        return check

    # The document.

    def root(self, v):
        if type(v) is not dict:
            self.fail("wrong_type", "yük bir harita olmalı")
        if v.get("format") != DOCUMENT_FORMAT:
            self.fail("schema_format", f"KentOS çizimi değil (format ≠ {DOCUMENT_FORMAT})")
        version = v.get("version")
        if type(version) is not int or version not in SCHEMAS:
            self.fail("schema_version", f"belge şeması sürümü {version!r} okunamıyor (desteklenen: {', '.join(map(str, SCHEMAS))})")
        # In an older schema these fields are unknown ones.
        self.weights = version >= SCHEMA_WITH_LINE_WEIGHTS
        self.elevations = version >= SCHEMA_WITH_ELEVATIONS
        self.parts = version >= SCHEMA_WITH_PARTS
        self.line_parts = version >= SCHEMA_WITH_LINE_PARTS
        self.linked_texts = version >= SCHEMA_WITH_LINKED_TEXTS
        self.paragraphs = version >= SCHEMA_WITH_PARAGRAPHS
        self.blocks = version >= SCHEMA_WITH_BLOCKS
        self.texts = version >= SCHEMA_WITH_TEXT_EXTRAS
        self.leaders = version >= SCHEMA_WITH_LEADERS
        self.dimensions = version >= SCHEMA_WITH_DIMENSIONS
        self.layer_snap = version >= SCHEMA_WITH_LAYER_SNAP
        self.drawing_unit = version >= SCHEMA_WITH_DRAWING_UNIT
        self.second_srid = version >= SCHEMA_WITH_SECOND_SRID
        self.custom_crs = version >= SCHEMA_WITH_CUSTOM_CRS
        self.survey_fields = version >= SCHEMA_WITH_SURVEY
        self.traverse_tolerances = version >= SCHEMA_WITH_TRAVERSE_TOLERANCES
        self.ground = version >= SCHEMA_WITH_GROUND
        self.layer_states = version >= SCHEMA_WITH_LAYER_STATES
        checked = self.fields({"format": (self.text, True), "version": (self.uint(32), True), "document": (self.document, True)})(v)
        return checked["document"]

    def document(self, v):
        d = self.fields(
            {
                "name": (self.text, True),
                **({"blocks": (self.definitions, False)} if self.blocks else {}),
                "layers": (self.array(self.layer), True),
                "origin": (self.point, True),
                "styles": (self.fields({"items": (self.array(self.any), True), "categories": (self.array(self.any), True)}), True),
                "entities": (self.array(self.entity), True),
                "homeView": (self.bounds, False),
                "settings": (self.settings, True),
                "projectId": (self.id16, False),
                "activeLayer": (self.text, True),
                "migratedFrom": (self.source, False),
            }
        )(v)
        out = {
            "format": DOCUMENT_FORMAT,
            "version": DOCUMENT_VERSION,
            "name": d["name"],
            "settings": d["settings"],
            "origin": d["origin"],
        }
        if "homeView" in d:
            out["homeView"] = d["homeView"]
        out["layers"] = d["layers"]
        out["activeLayer"] = d["activeLayer"]
        out["entities"] = d["entities"]
        out["uids"] = list(self.uids)
        out["styles"] = d["styles"]
        if "blocks" in d:
            out["blocks"] = d["blocks"]
        if "projectId" in d:
            out["projectId"] = d["projectId"]
        if "migratedFrom" in d:
            out["migratedFrom"] = d["migratedFrom"]
        return out

    def settings(self, v):
        s = self.fields(
            {
                "srid": (self.uint(32), True),
                "areaUnit": (self.enum(("m2", "donum", "ha")), True),
                "angleUnit": (self.enum(("grad", "deg")), True),
                "plotScale": (self.float, True),
                "workspace": (self.enum(("hybrid", "cad", "gis", "plan3d", "disaster")), False),
                **({"secondSrid": (self.uint(32), False)} if self.second_srid else {}),
                "drawingFont": (self.enum(("barlow", "arimo", "overpass", "quicksand", "architects-daughter", "courier-prime", "plex-mono")), False),
                **({"drawingUnit": (self.enum(("mm", "cm", "m")), False)} if self.drawing_unit else {}),
                "areaDecimals": (self.uint(32), True),
                "lengthDecimals": (self.uint(32), True),
                **(
                    {
                        "customCrs": (self.crs_definition, False),
                        "secondCustomCrs": (self.crs_definition, False),
                        "datumTransforms": (self.datum_transforms, False),
                    }
                    if self.custom_crs
                    else {}
                ),
                **({"survey": (self.survey, False)} if self.survey_fields else {}),
                **({"layerStates": (self.layer_states_, False)} if self.layer_states else {}),
            }
        )(v)
        # A second coordinate system is another system than the project's own; a project without one has none (a
        # definition of its own is one, docs/adr/0168 §1).
        has_system = s["srid"] != 0 or "customCrs" in s
        second = s.get("secondSrid")
        if second is not None and (second == 0 or not has_system or second == s["srid"]):
            self.path.append("secondSrid")
            self.fail("bad_value", "ikinci koordinat sistemi projeninkinden başka bir sistem olmalı; yerel projenin ikinci sistemi olmaz")
        if "customCrs" in s and s["srid"] != 0:
            self.path.append("customCrs")
            self.fail("bad_value", "projenin kendi tanımı yalnız EPSG kodu olmayan (srid 0) projede olur")
        if "secondCustomCrs" in s and ("secondSrid" in s or not has_system):
            self.path.append("secondCustomCrs")
            self.fail("bad_value", "ikinci sistem ya EPSG kodu ya tanımdır; koordinat sistemi olmayan projenin ikinci sistemi olmaz")
        return s

    def layer_states_(self, v):
        # The project's named layer states (schema 19, spec §6.4.3, docs/adr/0177 §4): an id and a name, neither empty
        # nor twice (a name as written, the spaces at its ends aside); in each, the nodes, none empty or twice, each
        # with its visibility and, when kept, its lock and a layer's style.
        node = self.fields({"node": (self.text, True), "style": (self.layer_style, False), "locked": (self.bool, False), "visible": (self.bool, True)})
        states = self.array(self.fields({"id": (self.text, True), "name": (self.text, True), "nodes": (self.array(node), True)}))(v)
        ids, names = set(), set()
        for s in states:
            name = s["name"].strip()
            if not s["id"]:
                self.fail("bad_value", "katman durumunun kimliği boş")
            if s["id"] in ids:
                self.fail("bad_value", f"“{s['id']}” kimlikli katman durumu iki kez var")
            if not name:
                self.fail("bad_value", "katman durumunun adı boş")
            if name in names:
                self.fail("bad_value", f"“{name}” adlı katman durumu iki kez var")
            ids.add(s["id"])
            names.add(name)
            seen = set()
            for n in s["nodes"]:
                if not n["node"]:
                    self.fail("bad_value", f"“{name}” durumunda düğüm kimliği boş")
                if n["node"] in seen:
                    self.fail("bad_value", f"“{name}” durumunda “{n['node']}” düğümü iki kez var")
                seen.add(n["node"])
        return states

    def survey(self, v):
        # The project's survey constants and tolerances (schema 14, spec §6.4.2, docs/adr/0169 §3): at least one, k
        # within [−1, 1], tolerances above zero; schema 16's ground height within [−500, 9000] m, the reduction to the
        # grid only with one (docs/adr/0171).
        table = {"index": (self.float, False), "faceHz": (self.float, False), "faceSlope": (self.float, False), "refraction": (self.float, False)}
        if self.traverse_tolerances:
            # Schema 15: the traverse tolerances.
            table.update({"twoWay": (self.float, False), "traverseAngle": (self.float, False), "traverseCoord": (self.float, False)})
        if self.ground:
            # Schema 16: the mean ellipsoidal height and the reduction to the grid (docs/adr/0171).
            table.update({"groundHeight": (self.float, False), "reduceToGrid": (self.bool, False)})
        s = self.fields(table)(v)
        if not s:
            self.fail("bad_value", "ölçme ayarları boş; ayarı olmayan proje alanı yazmaz")
        k = s.get("refraction")
        if k is not None and not -1.0 <= k <= 1.0:
            self.fail("bad_value", f"kırılma katsayısı k {k}; −1 ile 1 arasında olmalı")
        h = s.get("groundHeight")
        if h is not None and not -500.0 <= h <= 9000.0:
            self.fail("bad_value", f"ortalama elipsoit yüksekliği {h} m; −500 ile 9000 arasında olmalı")
        if s.get("reduceToGrid") is True and h is None:
            self.fail("bad_value", "uzunlukları projeksiyona indirmek ortalama elipsoit yüksekliği ister")
        for key in ("faceHz", "index", "faceSlope", "twoWay", "traverseAngle", "traverseCoord"):
            if key in s and not s[key] > 0.0:
                self.fail("bad_value", f"{key} toleransı {s[key]}; sıfırdan büyük olmalı")
        return s

    def source(self, v):
        s = self.fields({"format": (self.text, True), "version": (self.uint(32), True), "sourceSha256": (self.sha256, True)})(v)
        if s["format"] != DOCUMENT_FORMAT or s["version"] != 1:
            self.fail("bad_value", "göç kaynağı yalnız kentos.document sürüm 1 olabilir")
        return s

    def layer(self, v):
        n = self.fields(
            {
                "id": (self.text, True),
                "name": (self.text, True),
                **({"snap": (self.layer_snap_, False)} if self.layer_snap else {}),
                "type": (self.enum(("group", "layer")), True),
                "style": (self.layer_style, True),
                "locked": (self.bool, True),
                "visible": (self.bool, True),
                "children": (self.array(self.layer), True),
                "expanded": (self.bool, True),
            }
        )(v)
        if "snap" in n and n["type"] == "group":
            self.fail("bad_value", "grubun keneti olmaz; kenet yalnız katmanındır")
        return n

    # The project's coordinate systems and datum choices (schema 13, spec §6.4.1, docs/adr/0168).

    def three(self, v):
        a = self.array(self.float)(v)
        if len(a) != 3:
            self.fail("bad_value", f"3 sayı olmalı, {len(a)} var")
        return a

    def helmert(self, v):
        h = self.fields(
            {
                "scale": (self.float, True),
                "accuracy": (self.float, False),
                "rotation": (self.three, True),
                "convention": (self.enum(("positionVector", "coordinateFrame")), True),
                "translation": (self.three, True),
            }
        )(v)
        return {k: h[k] for k in ("translation", "rotation", "scale", "convention", "accuracy") if k in h}

    def custom_datum(self, v):
        ellipsoid = self.fields({"name": (self.text, True), "semiMajor": (self.float, True), "inverseFlattening": (self.float, True)})
        d = self.fields({"name": (self.text, True), "toWgs84": (self.helmert, False), "ellipsoid": (ellipsoid, True)})(v)
        e = d["ellipsoid"]
        out = {"name": d["name"], "ellipsoid": {"name": e["name"], "semiMajor": e["semiMajor"], "inverseFlattening": e["inverseFlattening"]}}
        if "toWgs84" in d:
            out["toWgs84"] = d["toWgs84"]
        return out

    def crs_plane(self, v):
        similarity = ("east", "north", "rotation", "scale")
        table = {"kind": (self.enum(("similarity", "affine")), True), **{k: (self.float, False) for k in "abcdef"},
                 **{k: (self.float, False) for k in similarity}}
        p = self.fields(table)(v)
        affine = p["kind"] == "affine"
        if any(k in p for k in (similarity if affine else tuple("abcdef"))):
            self.fail("unknown_field", "düzlem dönüşümünde türünün olmayan bir alanı var")
        for k in (tuple("abcdef") if affine else similarity):
            if k not in p:
                self.path.append(k)
                self.fail("missing_field", "zorunlu alan yok")
        return {"kind": p["kind"], **{k: p[k] for k in (tuple("abcdef") if affine else similarity)}}

    def crs_base(self, v):
        b = self.fields({"srid": (self.uint(32), False), "definition": (self.crs_definition_raw, False)})(v)
        return {k: b[k] for k in ("srid", "definition") if k in b}

    def crs_system(self, v):
        allowed = {
            "tm": ("kind", "datum", "customDatum", "latitudeOfOrigin", "centralMeridian", "scaleFactor", "falseEasting", "falseNorthing"),
            "geographic": ("kind", "datum", "customDatum"),
            "local": ("kind", "base", "plane"),
        }
        s = self.fields(
            {
                "kind": (self.enum(("tm", "geographic", "local")), True),
                "datum": (self.enum(REGISTRY_DATUMS), False),
                "customDatum": (self.custom_datum, False),
                "latitudeOfOrigin": (self.float, False),
                "centralMeridian": (self.float, False),
                "scaleFactor": (self.float, False),
                "falseEasting": (self.float, False),
                "falseNorthing": (self.float, False),
                "base": (self.crs_base, False),
                "plane": (self.crs_plane, False),
            }
        )(v)
        kind = s["kind"]
        stray = [k for k in s if k not in allowed[kind]]
        if stray:
            self.path.append(stray[0])
            self.fail("unknown_field", f"“{kind}” sisteminde bu alan olmaz")
        required = {"tm": ("centralMeridian", "scaleFactor", "falseEasting", "falseNorthing"), "geographic": (), "local": ("base", "plane")}[kind]
        for k in required:
            if k not in s:
                self.path.append(k)
                self.fail("missing_field", "zorunlu alan yok")
        return {"kind": kind, **{k: s[k] for k in allowed[kind][1:] if k in s}}

    def crs_definition_raw(self, v):
        d = self.fields({"name": (self.text, True), "system": (self.crs_system, True)})(v)
        return {"name": d["name"], "system": d["system"]}

    def crs_definition(self, v):
        d = self.crs_definition_raw(v)
        problem = crs_problem(d)
        if problem:
            self.fail("bad_value", problem)
        return d

    def datum_transform(self, v):
        grid = self.fields({"id": (self.text, True), "file": (self.text, True), "size": (self.uint(64), True), "accuracy": (self.float, False)})
        t = self.fields({"to": (self.enum(REGISTRY_DATUMS), True), "from": (self.enum(REGISTRY_DATUMS), True), "grid": (grid, False),
                         "name": (self.text, True), "helmert": (self.helmert, False)})(v)
        out = {"from": t["from"], "to": t["to"], "name": t["name"]}
        if "helmert" in t:
            out["helmert"] = t["helmert"]
        if "grid" in t:
            g = t["grid"]
            out["grid"] = {k: g[k] for k in ("id", "file", "size", "accuracy") if k in g}
        return out

    def datum_transforms(self, v):
        all_ = self.array(self.datum_transform)(v)
        problem = choices_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    def layer_snap_(self, v):
        """A layer's own snapping (schema 10): exactly one of `off` (true only) and a non-empty list of known,
        distinct kinds."""
        s = self.fields({"off": (self.bool, False), "kinds": (self.array(self.text), False)})(v)
        if s.get("off") is False:
            self.fail("bad_value", "off yalnız true yazılır")
        if ("off" in s) == ("kinds" in s):
            self.fail("bad_value", "katmanın keneti ya off ya kinds taşır, tam biri")
        kinds = s.get("kinds", [])
        if "kinds" in s and not kinds:
            self.fail("bad_value", "kenet türleri boş olamaz")
        for i, k in enumerate(kinds):
            if k not in LAYER_SNAP_KINDS:
                self.fail("bad_value", f"bilinmeyen kenet türü: {k}")
            if k in kinds[:i]:
                self.fail("bad_value", f"kenet türü iki kez yazılmış: {k}")
        return s

    def layer_style(self, v):
        return self.fields(
            {
                "fill": (self.text, False),
                "color": (self.text, True),
                "label": (self.label_style, False),
                "point": (self.fields({"size": (self.float, True), "symbol": (self.enum(("ring", "cross", "triangle")), True)}), False),
                "lineType": (self.enum(("continuous", "dashed", "dashdot", "dotted")), True),
                "renderer": (self.not_null, False),
                "lineWeight": (self.float, True),
                "pickInterior": (self.bool, False),
            }
        )(v)

    def label_style(self, v):
        return self.fields(
            {
                "ink": (self.enum(("fg", "fg-dim", "label")), False),
                "grow": (self.float, False),
                "size": (self.float, True),
                "weight": (self.uint(16), False),
                "maxSize": (self.float, False),
                "maxScale": (self.float, False),
                "minScale": (self.float, False),
                "template": (self.text, False),
                "placement": (self.enum(("center", "corner", "beside", "along")), True),
                "minFeaturePx": (self.float, False),
            }
        )(v)

    def uid(self, v):
        text = self.id16(v)
        if text in self.seen:
            self.fail("duplicate_uid", f"kalıcı kimlik {text} iki nesnede var")
        self.seen.add(text)
        self.uids.append(text)
        return text

    def entity(self, v):
        if type(v) is not dict:
            self.fail("wrong_type", "nesne tek anahtarlı bir harita olmalı")
        if len(v) != 1:
            self.fail("bad_value", f"nesne haritasında tek anahtar (tür) olmalı, {len(v)} var")
        ((kind, body),) = v.items()
        if kind not in ENTITY_KINDS or (kind == "insert" and not self.blocks) or (kind == "leader" and not self.leaders):
            self.path.append(kind)
            self.fail("unknown_kind", f"“{kind}” nesne türü bilinmiyor")
        table = dict(self.common())
        table.update(ENTITY_KINDS[kind](self))
        self.path.append(kind)
        try:
            fields = self.fields(table)(body)
            if kind in ("polyline", "polygon"):
                self.same_length(fields)
            if kind == "dimension":
                self.dimension_rules(fields)
            if kind == "text" and ("labelOf" in fields) != ("labelScale" in fields):
                self.path.append("labelOf" if "labelOf" in fields else "labelScale")
                self.fail("bad_value", "bağlı yazının nesnesi ve ölçeği birlikte verilir")
            if kind == "text" and "runs" in fields:
                self.path.append("runs")
                self.runs_rules(fields["text"], fields["runs"])
                self.path.pop()
            # The drawing's own insert names a definition read before it (`blocks` comes before `entities`).
            if kind == "insert" and self.inside is None and fields["block"] not in self.index:
                self.path.append("block")
                self.fail("unknown_block", "yerleştirilen blok çizimde tanımlı değil")
        finally:
            self.path.pop()
        if self.inside is None:
            fields.pop("uid")
            return {"kind": kind, "id": len(self.uids), **fields}
        self.inside += 1
        return {"kind": kind, "id": self.inside, **fields}

    # Blocks (§6.9, docs/adr/0144).

    def definitions(self, v):
        if type(v) is list and not v:
            self.fail("bad_value", "blok listesi boş; tanımı olmayan çizimde alan yazılmaz")
        blocks = self.array(self.definition)(v)
        self.check_blocks(blocks)
        self.index = {b["id"]: i for i, b in enumerate(blocks)}
        return blocks

    def definition(self, v):
        def content(x):
            self.inside = 0
            try:
                return self.array(self.entity)(x)
            finally:
                self.inside = None

        def attributes(x):
            if type(x) is list and not x:
                self.fail("bad_value", "öznitelik listesi boş; öznitelik tanımı yoksa alan yazılmaz")
            return self.array(self.attribute)(x)

        d = self.fields(
            {
                "id": (self.id16, True),
                "base": (self.point, True),
                "name": (self.text, True),
                "entities": (content, True),
                "attributes": (attributes, False),
                "description": (self.text, False),
            }
        )(v)
        return {k: d[k] for k in ("id", "name", "base", "entities", "attributes", "description") if k in d}

    def attribute(self, v):
        table = {
            "p": (self.point, True),
            "tag": (self.text, True),
            "value": (self.text, False),
            "height": (self.float, True),
            "prompt": (self.text, False),
            "rotation": (self.float, True),
        }
        if self.texts:
            table.update(text_extras(self, mask=False))
        a = self.fields(table)(v)
        return {k: a[k] for k in ("tag", "prompt", "value", "p", "height", "rotation", "align", "widthFactor") if k in a}

    def width_factor(self, v):
        x = self.float(v)
        if not (0.0 < x <= MAX_WIDTH_FACTOR):
            self.fail("bad_value", f"genişlik çarpanı {x}; 0'dan büyük, en çok {MAX_WIDTH_FACTOR:g} olmalı")
        return x

    def mask(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "zemin false yazılmaz; zeminsiz yazıda alan yoktur")
        return True

    def box_width(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"çok satırlı yazının kutu genişliği {x:g}; sıfırdan büyük olmalı")
        return x

    def line_spacing(self, v):
        x = self.float(v)
        if not MIN_LINE_SPACING <= x <= MAX_LINE_SPACING:
            self.fail("bad_value", f"çok satırlı yazının satır aralığı {x:g}; {MIN_LINE_SPACING:g} ile {MAX_LINE_SPACING:g} arasında olmalı")
        return x

    def true_flag(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "false yazılmaz; biçimsiz dilimde alan yoktur")
        return True

    def runs(self, v):
        """A multi-line text's letter formats (§6.6, docs/adr/0182): a list that is not empty, each run its range and format."""
        if type(v) is list and not v:
            self.fail("bad_value", "biçim dilimi listesi boş; dilimsiz yazıda alan yazılmaz")
        run = self.fields(
            {
                "start": (self.uint(32), True),
                "end": (self.uint(32), True),
                "bold": (self.true_flag, False),
                "italic": (self.true_flag, False),
                "underline": (self.true_flag, False),
                "script": (self.enum(("super", "sub")), False),
                "color": (self.text, False),
            }
        )
        return self.array(run)(v)

    def runs_rules(self, text, runs):
        """The runs inside the text's letters (Unicode scalar values), in order, apart, each with a format, touching runs
        of one format joined (docs/adr/0182 §1)."""
        letters = len(text)
        keys = ("bold", "italic", "underline", "script", "color")
        before = None
        for i, r in enumerate(runs):
            n = i + 1
            if r["start"] >= r["end"] or r["end"] > letters:
                self.fail("bad_value", f"{n}. biçim dilimi {r['start']}–{r['end']}; yazının {letters} harfi içinde, başı sonundan önce olmalı")
            if not any(k in r for k in keys):
                self.fail("bad_value", f"{n}. biçim diliminin biçimi yok; biçimsiz dilim yazılmaz")
            if r.get("color") == "":
                self.fail("bad_value", f"{n}. biçim diliminin rengi boş")
            if before is not None:
                if r["start"] < before["end"]:
                    self.fail("bad_value", f"{n}. biçim dilimi öncekiyle örtüşüyor; dilimler sıralı ve ayrı olmalı")
                if r["start"] == before["end"] and all(r.get(k) == before.get(k) for k in keys):
                    self.fail("bad_value", f"{n}. biçim dilimi aynı biçimdeki öncekine bitişik; ikisi tek dilimdir")
            before = r

    def label_scale(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"bağlı yazının ölçeği 1:{x:g}; sıfırdan büyük olmalı")
        return x

    def dimension_mask(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "zemin false yazılmaz; zeminsiz ölçüde alan yoktur")
        return True

    def dimension_rules(self, d):
        """What a dimension's style needs, and what only a slope has (docs/adr/0147)."""
        style = d.get("style")
        needs = ["c"] if style in ("arcLength", "jogged") else ["za", "zb"] if style == "slope" else []
        for key in needs:
            if key not in d:
                self.path.append(key)
                self.fail("missing_field", "zorunlu alan yok")
        if style != "slope":
            for key in ("za", "zb"):
                if key in d:
                    self.path.append(key)
                    self.fail("bad_value", "kot (za, zb) yalnız eğim ölçüsünde yazılır")
        if style == "ordinate" and "angle" in d and d["angle"] not in (0.0, 90.0):
            self.path.append("angle")
            self.fail("bad_value", f"koordinat ölçüsünün ekseni {d['angle']}; 0 (Y) ya da 90 (X) olmalı")

    def leader_points(self, v):
        pts = self.array(self.point)(v)
        if len(pts) < 2:
            self.fail("bad_value", f"kılavuzun {len(pts)} köşesi var; en az iki olmalı")
        return pts

    def leader_height(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"kılavuzun yüksekliği {x}; 0'dan büyük olmalı")
        return x

    def note(self, v):
        t = self.text(v)
        if t == "":
            self.fail("bad_value", "kılavuzun notu boş; notsuz kılavuzun not alanı yazılmaz")
        return t

    def scale(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"blok ölçeği {x}; pozitif olmalı")
        return x

    def mirror(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "aynalama false yazılmaz; aynalı olmayan yerleştirmede alan yoktur")
        return True

    def check_blocks(self, blocks):
        """The block rules over the whole list, in their order: each definition's name, id and tags; the inserts in
        the definitions; then the nesting. The first broken one is said at its definition."""
        ids, names = {}, {}
        for i, b in enumerate(blocks):
            self.path.append(str(i))
            if all(c in WHITE_SPACE for c in b["name"]):
                self.path.append("name")
                self.fail("bad_value", "blok adı boş ya da yalnız boşluk olamaz")
            if b["id"] in ids:
                self.path.append("id")
                self.fail("duplicate_block", f"blok kimliği {b['id']} {ids[b['id']] + 1}. tanımda da var")
            ids[b["id"]] = i
            key = name_key(b["name"])
            if key in names:
                self.path.append("name")
                self.fail("duplicate_block", f"“{b['name']}” adı {names[key] + 1}. tanımda da var")
            names[key] = i
            tags = {}
            for a, attribute in enumerate(b.get("attributes", [])):
                self.path += ["attributes", str(a), "tag"]
                if not attribute["tag"]:
                    self.fail("bad_value", "öznitelik etiketi boş")
                if attribute["tag"] in tags:
                    self.fail("bad_value", f"“{attribute['tag']}” etiketi {tags[attribute['tag']] + 1}. öznitelikte de var")
                tags[attribute["tag"]] = a
                del self.path[-3:]
            self.path.pop()
        for i, b in enumerate(blocks):
            for j, e in enumerate(b["entities"]):
                if e["kind"] == "insert" and e["block"] not in ids:
                    self.path += [str(i), "entities", str(j), "insert", "block"]
                    self.fail("unknown_block", "yerleştirilen blok çizimde tanımlı değil")
        depth = [0] * len(blocks)
        stack = []

        def visit(i):
            stack.append(i)
            deepest = 0
            for e in blocks[i]["entities"]:
                if e["kind"] != "insert":
                    continue
                k = ids[e["block"]]
                if depth[k] == 0:
                    if k in stack:
                        self.path.append(str(k))
                        self.fail("block_cycle", f"“{blocks[k]['name']}” bloğu kendini içeriyor")
                    if len(stack) == MAX_BLOCK_DEPTH:
                        self.path.append(str(stack[0]))
                        self.fail("block_too_deep", f"bloklar {MAX_BLOCK_DEPTH} düzeyden derin iç içe")
                    visit(k)
                if len(stack) + depth[k] > MAX_BLOCK_DEPTH:
                    self.path.append(str(stack[0]))
                    self.fail("block_too_deep", f"bloklar {MAX_BLOCK_DEPTH} düzeyden derin iç içe")
                deepest = max(deepest, depth[k])
            stack.pop()
            depth[i] = deepest + 1

        for i in range(len(blocks)):
            if depth[i] == 0:
                visit(i)

    def common(self):
        return {
            **({"uid": (self.uid, True)} if self.inside is None else {}),
            "attrs": (self.attrs, True),
            "color": (self.text, False),
            "label": (self.text, False),
            "symbol": (self.text, False),
            "layerId": (self.text, True),
            **({"lineWeight": (self.line_weight, False)} if self.weights else {}),
        }

    def line_weight(self, v):
        w = self.float(v)
        if not 0.0 <= w <= MAX_LINE_WEIGHT:
            self.fail("bad_value", f"çizgi kalınlığı {w} mm; 0 ile {MAX_LINE_WEIGHT:g} arasında olmalı")
        return w

    def elevation(self, v):
        """A vertex's elevation: a float, or `null` for a vertex without one (§6.6)."""
        return None if v is None else self.float(v)

    def same_length(self, obj):
        """One elevation per vertex (§6.6): `zs` as long as `pts`. Keys come in encoded order, `zs` before `pts`, so this is checked once the map is read."""
        if "zs" in obj and len(obj["zs"]) != len(obj["pts"]):
            self.path.append("zs")
            self.fail("bad_value", f"{len(obj['zs'])} kot var ama {len(obj['pts'])} köşe var; her köşenin bir kotu olmalı (kotsuz köşe için null)")

    def ring(self, v):
        table = {"pts": (self.array(self.point), True), "bulges": (self.array(self.float), False)}
        if self.elevations:
            table["zs"] = (self.array(self.elevation), False)
        ring = self.fields(table)(v)
        self.same_length(ring)
        return ring

    def line_part(self, v):
        """A part of a multi-part polyline past its first (schema 17, §6.6): two vertices or more, its arcs and
        elevations; no holes."""
        table = {"pts": (self.array(self.point), True), "bulges": (self.array(self.float), False)}
        if self.elevations:
            table["zs"] = (self.array(self.elevation), False)
        part = self.fields(table)(v)
        if len(part["pts"]) < 2:
            self.path.append("pts")
            self.fail("bad_value", f"çoklu çizginin parçasının {len(part['pts'])} köşesi var; en az iki olmalı")
        self.same_length(part)
        return part

    def point_part(self, v):
        """A point of a multi-point object past its first (schema 17, §6.6): its place and elevation."""
        return self.fields({"p": (self.point, True), "z": (self.float, False)})(v)

    def part(self, v):
        """A part of a multi-part area past its first (§6.6): its ring, arcs, holes and elevations, as the area's own."""
        table = {"pts": (self.array(self.point), True), "bulges": (self.array(self.float), False), "holes": (self.array(self.ring), False)}
        if self.elevations:
            table["zs"] = (self.array(self.elevation), False)
        part = self.fields(table)(v)
        self.same_length(part)
        return part


def _path(s, holes):
    table = {"pts": (s.array(s.point), True), "bulges": (s.array(s.float), False)}
    if holes:
        table["holes"] = (s.array(s.ring), False)
        # An area has parts from schema 5 on.
        if s.parts:
            table["parts"] = (s.array(s.part), False)
    elif s.line_parts:
        # A polyline from schema 17 on (docs/adr/0174).
        table["parts"] = (s.array(s.line_part), False)
    if s.elevations:
        table["zs"] = (s.array(s.elevation), False)
    return table


ENTITY_KINDS = {
    "point": lambda s: {"p": (s.point, True), "z": (s.float, False), **({"parts": (s.array(s.point_part), False)} if s.line_parts else {})},
    # A line's end without an elevation has no key: `null` is not written there (§5.2).
    "line": lambda s: {"a": (s.point, True), "b": (s.point, True), **({"za": (s.float, False), "zb": (s.float, False)} if s.elevations else {})},
    "polyline": lambda s: _path(s, holes=False),
    "polygon": lambda s: _path(s, holes=True),
    "circle": lambda s: {"c": (s.point, True), "r": (s.float, True)},
    "arc": lambda s: {"c": (s.point, True), "r": (s.float, True), "a0": (s.float, True), "a1": (s.float, True)},
    "ellipse": lambda s: {"c": (s.point, True), "major": (s.point, True), "ratio": (s.float, True), "t0": (s.float, True), "t1": (s.float, True)},
    "spline": lambda s: {"pts": (s.array(s.point), True), "closed": (s.bool, True)},
    "xline": lambda s: {"p": (s.point, True), "dir": (s.point, True)},
    "ray": lambda s: {"p": (s.point, True), "dir": (s.point, True)},
    "text": lambda s: {
        "p": (s.point, True),
        "text": (s.text, True),
        "height": (s.float, True),
        "rotation": (s.float, True),
        **(text_extras(s, mask=True) if s.texts else {}),
        # Schema 18 (docs/adr/0175 §4): the object whose label it writes and the scale, only where objects have ids.
        **({"labelOf": (s.id16, False), "labelScale": (s.label_scale, False)} if s.linked_texts and s.inside is None else {}),
        # Schema 20 (docs/adr/0182 §1): a multi-line text's box, line spacing and letter formats.
        **({"boxWidth": (s.box_width, False), "lineSpacing": (s.line_spacing, False), "runs": (s.runs, False)} if s.paragraphs else {}),
    },
    # Schema 9 (docs/adr/0147): five more kinds, `mask` only when true, a slope's two elevations.
    "dimension": lambda s: {
        "a": (s.point, True),
        "b": (s.point, True),
        "c": (s.point, False),
        "text": (s.text, False),
        "angle": (s.float, False),
        "style": (s.enum(DIMENSION_STYLES if s.dimensions else DIMENSION_STYLES[:5]), False),
        "height": (s.float, True),
        "offset": (s.float, True),
        **({"mask": (s.dimension_mask, False), "za": (s.float, False), "zb": (s.float, False)} if s.dimensions else {}),
    },
    "hatch": lambda s: {
        "ring": (s.array(s.point), True),
        "holes": (s.array(s.array(s.point)), False),
        "pattern": (s.fields({"type": (s.enum(("solid", "lines", "cross")), True), "angle": (s.float, True), "spacing": (s.float, True)}), True),
    },
    # Schema 6 (docs/adr/0144): `mirror` only when true.
    "insert": lambda s: {"block": (s.id16, True), "p": (s.point, True), "scale": (s.scale, True), "rotation": (s.float, True), "mirror": (s.mirror, False)},
    # Schema 8 (docs/adr/0146): two vertices or more, a positive height, a note that is not empty, `mask` only when true.
    "leader": lambda s: {
        "pts": (s.leader_points, True),
        "text": (s.note, False),
        "height": (s.leader_height, True),
        "rotation": (s.float, True),
        "arrow": (s.enum(LEADER_ARROWS), False),
        "mask": (s.mask, False),
    },
}


def text_extras(s, mask):
    """Schema 7's fields of a text (`mask` too) or an attribute definition (docs/adr/0145)."""
    table = {"align": (s.enum(TEXT_ALIGNS), False), "widthFactor": (s.width_factor, False)}
    if mask:
        table["mask"] = (s.mask, False)
    return table


def name_key(name):
    """A block's name as names are compared: each character lowercased, Turkish I (I → ı, İ → i)."""
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in name)


def uuid_text(raw):
    h = raw.hex()
    return f"{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}"


def read(data):
    """Reads a whole file: (header, document in the contract's JSON form); raises KcadError."""
    header, payload = read_header(data)
    return header, _Schema().root(decode_payload(payload))


# ── Command line ────────────────────────────────────────────────────────


def _summary(header, doc):
    kinds = Counter(e["kind"] for e in doc["entities"])
    versions = Counter(int(u[14], 16) for u in doc["uids"])
    layers = []

    def walk(nodes):
        for n in nodes:
            layers.append(n)
            walk(n["children"])

    walk(doc["layers"])
    lines = [
        f"KCAD {header['major']}.{header['minor']} (en az okuyucu {header['minReader']}), kodlama {header['encoding']} (CBOR profili 1), sıkıştırma yok",
        f"Yük: {header['payloadLength']} bayt, SHA-256 doğru ({header['sha256'][:16]}…)",
        f"Belge: {doc['format']} sürüm {doc['version']}",
        f"Ad: {doc['name']}",
        f"Koordinat sistemi: EPSG:{doc['settings']['srid']}, ölçek 1:{doc['settings']['plotScale']:g}",
        f"Katmanlar: {sum(1 for n in layers if n['type'] == 'layer')} katman, {sum(1 for n in layers if n['type'] == 'group')} grup; etkin: {doc['activeLayer']}",
        f"Nesneler: {len(doc['entities'])}" + (" — " + ", ".join(f"{k} {n}" for k, n in sorted(kinds.items())) if kinds else ""),
        f"Kalıcı kimlikler: {len(doc['uids'])}, benzersiz" + (" (" + ", ".join(f"v{v}: {n}" for v, n in sorted(versions.items())) + ")" if versions else ""),
        f"Proje kimliği: {doc.get('projectId', 'yok')}",
    ]
    source = doc.get("migratedFrom")
    lines.append(f"Göç kaynağı: {source['format']} sürüm {source['version']}, sha256 {source['sourceSha256']}" if source else "Göç kaynağı: yok")
    lines.append(f"Proje stilleri: {len(doc['styles']['items'])} öğe, {len(doc['styles']['categories'])} kategori")
    return "\n".join(lines)


def _json_default(v):
    raise TypeError(f"JSON'a yazılamıyor: {type(v).__name__}")


def main(argv):
    usage = "Kullanım: kcad.py inspect DOSYA | validate DOSYA… | dump DOSYA | sniff DOSYA…"
    if len(argv) < 2 or argv[0] not in ("inspect", "validate", "dump", "sniff"):
        print(usage, file=sys.stderr)
        return 2
    command, files = argv[0], argv[1:]
    if command in ("inspect", "dump") and len(files) != 1:
        print(usage, file=sys.stderr)
        return 2
    status = 0
    for name in files:
        try:
            with open(name, "rb") as f:
                data = f.read()
        except OSError as e:
            print(f"{name}: okunamadı ({e.strerror})", file=sys.stderr)
            status = 1
            continue
        if command == "sniff":
            print(f"{name}: {sniff(data)}")
            continue
        try:
            header, doc = read(data)
        except KcadError as e:
            print(f"{name}: {e.code}: {e.message}", file=sys.stderr if command != "validate" else sys.stdout)
            status = 1
            continue
        if command == "validate":
            print(f"{name}: geçerli ({len(doc['entities'])} nesne)")
        elif command == "inspect":
            print(_summary(header, doc))
        else:
            json.dump(doc, sys.stdout, ensure_ascii=False, indent=2, default=_json_default)
            print()
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
