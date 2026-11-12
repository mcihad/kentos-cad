#!/usr/bin/env python3
"""The shared cases of `map.work.query` and `map.work.edit` (plan/map-cad.md MAP-01, docs/adr/1200), written from the
ADR without KentOS code, and the source files they read (fixtures/map-work/v1).

    python3 scripts/fixtures/map_work_cases.py           # writes fixtures/commands/v1/map.work.*.json and the files
    python3 scripts/fixtures/map_work_cases.py --check   # writes nothing; compares

The drawing is a map project (cadWork map) with a GNSS campaign (its points imported and contours drawn from it), an
orthophoto and an old orthophoto (rasters showing their files; the old one on a locked layer), a cadastral CAD
reference held by a layer group, a design surface and a free drawing layer. An item's revision is the SHA-256 of its
file (hashlib here); the rules, their order and the sentences are ADR 1200 §2–§3's, written here by hand:

- An object is in the item that names its layer, else the one naming the nearest group above it (the layer profile).
  Its role: `derived` when a derivation names it; else `source` on a campaign's or a reference's layer; else `free`.
- A derivation is out of date when its recorded revision is not its source's revision; read deep, the source's
  revision is its file's content now. Some objects gone: `missing`.
- An item: `missing` (its file is not there), `changed` (deep: its file is not its revision), `incomplete` (a layer it
  names is not in the drawing), `stale` (approved, with out-of-date derived objects on its layers); the first that
  holds is its state, every one says why.
- Each edit is one undo step; the work structure is written whole after it (status `draft` and empty lists unwritten).

ADR 1201 adds: an item's file not at its path is looked for, given the open project's file (`drawing`), at its relative
path from that file's folder, then in the project's search paths (`xrefPaths`: first its relative path there, then its
file name); the file row says `via` (path, relative, searchPath, none), `at` when found elsewhere and the kept
`relative`. `release` lets objects go of their derivations; a relink takes the external references on the old file along
when they loaded the same content, and warns about the others.

ADR 2080 adds, written here from the ADR with hashlib and json:

- An item without a file has a content revision: SHA-256 of the compact JSON list `[[uid, digest], …]` of the objects
  its layers hold, sorted by uid, where an object's digest is SHA-256 of its JSON with sorted keys and without its slot
  id (`kentos.object-digest/1`). A derivation from such an item is judged against it. A quick read gives it on the row
  only where it is judged (a derivation's source, an approved item with a stamp); a deep read on every such row.
- An approval stamps the item's revision (its file's, or its content revision) in `approved`; leaving the approved
  status takes it away; approving an approved item that moved on stamps it again. An approved item whose revision now
  is not its stamp is `revised`, after the other states.
- An object its own layers hold is not derived from an item without a file (`own_content`).

ADR 5280 adds, written here from the regulations (not from KentOS's rules file, which the cases thus check):

- A shift limit: the user's own value; BÖHHBÜY m. 46(1)'s detail position ±7 cm; m. 78(1)(h)'s ±0.2 mm on the drawing
  times the scale; the error limit regulation's m. 9(1) table (read from its image, RG 27.09.2022/31966). Each changed
  object's shift is beyond it when its largest vertex move (paired) or its Hausdorff distance (not paired) is larger;
  how many vertices are, when paired; the item's changes count the listed objects beyond it and its row says so.
- `suggest`: an item without a profile lists the profiles (KentOS's, then the drawing's copies) with the classes whose
  `work` holds its kind.
"""

import copy
import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# docs/adr/4880: an item's typed layers are a data profile's classes; `cad.profiles`' own independent reference writes
# and checks them (scripts/fixtures/profile_cases.py, no KentOS code).
PR = _load("profile_cases")
PROFILES = {n: json.loads((ROOT / f"catalog/v1/profiles/{n}.json").read_text("utf-8"))
            for n in ("tucbs-temel", "harita-varliklari", "imar-plani")}
FILES = ROOT / "fixtures/map-work/v1"

# docs/adr/5280: the position accuracy values, transcribed from the regulations' text and the m. 9(1) table's image.
DETAIL = (0.07, "BÖHHBÜY m. 46(1): detay konumu ±7 cm")
DRAWING_MM = 0.2
DRAWING_SOURCE = "BÖHHBÜY m. 78(1)(h): çizimde nokta konumu ±0,2 mm"
TABLE_SOURCE = "Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik m. 9(1)"
TABLE_SCALES = [200, 250, 500, 1000, 2000, 2500, 3000, 4000, 5000, 10000]
_ = None
TABLE = [
    ("prizmatik-olcu", "Prizmatik, ölçü değerlerine göre", [0.21] * 10),
    ("prizmatik-pafta", "Prizmatik, paftadan okunan", [0.21, 0.22, 0.25, 0.35, 0.6, 0.74, 1.11, 1.28, 1.43, 2.01]),
    ("takeometrik-yakin", "Takeometrik, poligona uzaklık 100 m'ye dek", [0.5] * 10),
    ("takeometrik-uzak", "Takeometrik, poligona uzaklık 100 m'den çok", [1] * 10),
    ("takeometrik-pafta", "Takeometrik, paftadan okunan", [1, 1, 1.01, 1.04, 1.15, 1.22, 1.31, 1.51, 1.73, 3]),
    ("fotogrametrik", "Fotogrametrik", [_, _, _, _, 0.6, 0.75, 0.9, 1.2, 1.5, 3]),
    ("sayisal-fotogrametrik", "Sayısal fotogrametrik", [_, _, _, 0.09, 0.15, 0.15, 0.2, 0.25, 0.3, 0.4]),
    ("fotoplan", "Fotoplan", [_, _, _, _, 0.65, 0.8, 0.95, 1.35, 1.75, 3.5]),
    ("sayisal", "Sayısal", [0.09] * 10),
]


def num_words(x):
    """A number as the shortest text that reads back to it, without a trailing `.0` (Rust's `Display`)."""
    t = repr(float(x))
    return t[:-2] if t.endswith(".0") else t


def scale_words(n):
    return "1/" + f"{n:,}".replace(",", ".")


def limit_of(lim):
    """docs/adr/5280: the limit's (metres, source), or (path, message) refused."""
    basis = lim["basis"]
    if basis == "own":
        m = lim.get("metres")
        if m is None:
            return None, ("limit.metres", "Kendi sınırınız için metres verin (ör. 0,05).")
        if not (0 < m <= 1000):
            return None, ("limit.metres", f"Sınır {num_words(m)} m: 0'dan büyük, en çok 1000 m olmalı.")
        return (float(m), "kullanıcının değeri"), None
    if basis == "detail":
        return DETAIL, None

    def scale(what):
        sc = lim.get("scale")
        if sc is None:
            return None, ("limit.scale", f"{what} için ölçek gerekir (1000: 1/1.000).")
        if not (1 <= sc <= 1_000_000):
            return None, ("limit.scale", f"Ölçek paydası {sc}: 1 ile 1000000 arasında olmalı.")
        return sc, None
    if basis == "drawing":
        sc, bad = scale("Çizimde nokta konumu")
        if bad:
            return None, bad
        return (DRAWING_MM * sc / 1000.0, f"{DRAWING_SOURCE}, {scale_words(sc)}"), None
    ids = ", ".join(t[0] for t in TABLE)
    mid = lim.get("method")
    if mid is None:
        return None, ("limit.method", f"Üretim yöntemini verin: {ids}.")
    row = next((t for t in TABLE if t[0] == mid), None)
    if row is None:
        return None, ("limit.method", f"“{mid}” yöntemi tabloda yok; yöntemler: {ids}.")
    sc, bad = scale("Üretim yöntemi")
    if bad:
        return None, bad
    if sc not in TABLE_SCALES:
        have = ", ".join(scale_words(x) for x in TABLE_SCALES)
        return None, ("limit.scale", f"{scale_words(sc)} tabloda yok (tablonun ölçekleri {have}); tabloda olmayan "
                                     "ölçeğin değerini Genel Müdürlük belirler (m. 9(3)).")
    v = row[2][TABLE_SCALES.index(sc)]
    if v is None:
        return None, ("limit.scale", f"Tablo {scale_words(sc)} için “{row[1]}” değeri vermiyor.")
    return (float(v), f"{TABLE_SOURCE}: {row[1]}, {scale_words(sc)}"), None

# ── The source files ─────────────────────────────────────────────────────────────────────────────────────────────────

NMEA = (ROOT / "fixtures/gnss/v1/sample.nmea").read_bytes()
# The campaign's later log: the same receiver's file with one more fix (another length, another SHA-256).
NMEA_LATER = NMEA + b"$GPGGA,093410.00,3954.9640000,N,03550.5540000,E,4,12,0.80,1063.500,M,36.874,M,1.0,0001*4A\n"
DXF = ("0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nYAPI\n10\n500020.0\n20\n4400010.0\n30\n0.0\n11\n500030.0\n21\n4400010.0\n"
       "31\n0.0\n0\nENDSEC\n0\nEOF\n").encode()
WRITTEN = {"kampanya.nmea": NMEA, "kampanya-2026-11.nmea": NMEA_LATER, "kadastro.dxf": DXF}
ORTHO = ROOT / "fixtures/raster/v1/files/i8.tif"
OLD_ORTHO = ROOT / "fixtures/raster/v1/files/geographic.tif"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def digest(e):
    """An object's digest (`kentos.object-digest/1`, ADR 1320): SHA-256 of its JSON with sorted keys, no slot id."""
    v = {k: x for k, x in e.items() if k != "id"}
    return sha(json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode())


# docs/adr/3280: an object's four parts, its fields split by name (the slot id in none); each part's digest the first
# 16 digits of the SHA-256 of its fields as JSON with sorted keys.
PARTS = ("geometry", "attributes", "layer", "look")
ATTRIBUTE_FIELDS = ("attrs", "attachments")
LOOK_FIELDS = ("color", "label", "symbol", "lineWeight", "labelPins", "lineType", "linePattern", "lineTypeScale",
               "transparency", "order", "byLayer", "wipeout")


def value(e):
    """docs/adr/3680: an object's value as an approval keeps it: the very text its digest is the SHA-256 of."""
    v = {k: x for k, x in e.items() if k != "id"}
    return json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def attributes_changed(then, now):
    """docs/adr/3680: the attributes (`attrs`) that differ between an object's value as approved and now, by name,
    with their values then and now (none: not there); and whether its attachments differ."""
    a, b = then.get("attrs", {}), now.get("attrs", {})
    out = []
    for k in sorted(set(a) | set(b)):
        if a.get(k) != b.get(k):
            out.append({"name": k, **({"approved": a[k]} if k in a else {}), **({"now": b[k]} if k in b else {})})
    return out, then.get("attachments") != now.get("attachments")


def parts(e):
    split = [{}, {}, {}, {}]
    for k, v in e.items():
        if k == "id":
            continue
        split[1 if k in ATTRIBUTE_FIELDS else 2 if k == "layerId" else 3 if k in LOOK_FIELDS else 0][k] = v
    return [sha(json.dumps(p, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode())[:16] for p in split]


S_GNSS, S_LATER, S_DXF = sha(NMEA), sha(NMEA_LATER), sha(DXF)
S_ORTHO, S_OLD = sha(ORTHO.read_bytes()), sha(OLD_ORTHO.read_bytes())
P_GNSS, P_LATER, P_DXF = "$fixture:map-work/v1/kampanya.nmea", "$fixture:map-work/v1/kampanya-2026-11.nmea", \
    "$fixture:map-work/v1/kadastro.dxf"
P_ORTHO, P_OLD = "$fixture:raster/v1/files/i8.tif", "$fixture:raster/v1/files/geographic.tif"
CONTENT = {P_GNSS: S_GNSS, P_LATER: S_LATER, P_DXF: S_DXF, P_ORTHO: S_ORTHO, P_OLD: S_OLD}

# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────────


def uid(n):
    return f"0192f5c0-1200-7000-8000-{n:012x}"


G1, G2, G3, R1, A1, T1, T2, F1, K1 = (uid(n) for n in range(1, 10))
GHOST = uid(0x999)
XREF_SAME, XREF_OTHER, XREF_ELSE = "0192f5c0-1201-7000-8000-000000000001", "0192f5c0-1201-7000-8000-000000000002", \
    "0192f5c0-1201-7000-8000-000000000003"


def xref_of(path, digest):
    return {"path": path, "digest": digest}


def xref_block(id_, name, path, digest):
    """An external reference's definition (ADR 0348): one line of its source, its source's path and digest."""
    return {"id": id_, "name": name, "base": {"x": 0.0, "y": 0.0},
            "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0.0, "y": 0.0},
                          "b": {"x": 1.0, "y": 0.0}}],
            "xref": xref_of(path, digest)}
X0, Y0 = 500000.0, 4400000.0


def layer(id_, name, kind="layer", locked=False, children=()):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": list(children)}


def point(slot, layer_id, name, x, y):
    return {"kind": "point", "id": slot, "layerId": layer_id, "attrs": {"Ad": name}, "p": {"x": X0 + x, "y": Y0 + y}}


def line(slot, layer_id, a, b):
    return {"kind": "line", "id": slot, "layerId": layer_id, "attrs": {},
            "a": {"x": X0 + a[0], "y": Y0 + a[1]}, "b": {"x": X0 + b[0], "y": Y0 + b[1]}}


def raster(slot, layer_id, file):
    return {"kind": "raster", "id": slot, "layerId": layer_id, "attrs": {},
            "affine": [X0, 1.0, 0.0, Y0 + 10.0, 0.0, -1.0], "width": 10, "height": 10, "bands": 1, "sample": "u8",
            "srid": 5256, "style": {"render": "gray", "bands": [1]}, "file": file}


def item(id_, kind, name, layers=(), file=None, status="draft"):
    out = {"id": id_, "kind": kind, "name": name}
    if status != "draft":
        out["status"] = status
    if layers:
        out["layers"] = list(layers)
    if file is not None:
        out["file"] = {"path": file[0], "revision": file[1]}
    return out


def folder_of(path):
    """A `$tmp:`/`$fixture:` file's folder."""
    return path.rsplit("/", 1)[0]


def resolve(folder, rel):
    """`rel` (parts by "/", ".." only at the start) from `folder`, lexically, keeping the `$tmp:` root."""
    root, rest = folder.split(":", 1)
    parts = [p for p in rest.split("/") if p]
    for part in rel.split("/"):
        if part == "..":
            parts.pop()
        elif part not in ("", "."):
            parts.append(part)
    return root + ":" + "/".join(parts)


def derivation(id_, source, revision, operation, uids):
    return {"id": id_, "source": source, "revision": revision, "operation": operation, "uids": list(uids)}


IMPORT, CONTOURS = "GNSS içe aktarma", "Eş yükselti eğrileri"


def start_work():
    return {
        "items": [
            item("is-1", "campaign", "GNSS ölçümü 2026-10", ["gnss"], (P_GNSS, S_GNSS)),
            item("is-2", "reference", "Ortofoto 2025", ["ortofoto"], (P_ORTHO, S_ORTHO)),
            item("is-3", "reference", "Kadastro CAD altlığı", ["altlik"], (P_DXF, S_DXF)),
            item("is-4", "design", "Tasarım yüzeyi", ["tasarim"], status="review"),
            item("is-5", "reference", "Eski ortofoto", ["kilitli"], (P_OLD, S_OLD)),
        ],
        "derivations": [
            derivation("tr-1", "is-1", S_GNSS, IMPORT, [G1, G2]),
            derivation("tr-2", "is-1", S_GNSS, CONTOURS, [T1, T2]),
        ],
    }


def setup(work="map", map_work="start"):
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work}
    if map_work == "start":
        map_work = start_work()
    if map_work is not None:
        settings["mapWork"] = map_work
    return {
        "format": "kentos.document", "version": 2, "name": "Harita işi", "settings": settings,
        "origin": {"x": X0, "y": Y0},
        "layers": [layer("0", "0"), layer("gnss", "GNSS noktaları"), layer("ortofoto", "Ortofoto"),
                   layer("altlik", "Kadastro altlığı", "group", children=[layer("altlik-yapi", "Yapılar")]),
                   layer("tasarim", "Tasarım yüzeyi"), layer("cizim", "Çizim"),
                   layer("kilitli", "Kilitli", locked=True), layer("bos", "Boş")],
        "activeLayer": "cizim",
        "entities": [point(1, "gnss", "G1", 1.0, 2.0), point(2, "gnss", "G2", 5.0, 2.0), point(3, "gnss", "G3", 9.0, 2.0),
                     raster(4, "ortofoto", P_ORTHO), line(5, "altlik-yapi", (20.0, 10.0), (30.0, 10.0)),
                     line(6, "tasarim", (0.0, 0.0), (10.0, 0.0)), line(7, "tasarim", (0.0, 5.0), (10.0, 5.0)),
                     line(8, "cizim", (0.0, 20.0), (10.0, 25.0)), raster(9, "kilitli", P_OLD)],
        "uids": [G1, G2, G3, R1, A1, T1, T2, F1, K1],
        "styles": {"items": [], "categories": []},
    }


# ── How far a shape moved (docs/adr/4880) ───────────────────────────────────────────────────────────────────────────
#
# Vertex by vertex when both states are of one kind and have as many vertices (a point's place, a line's two ends, a
# polyline's corners, an area's ring then its holes): the largest move (the first of equal ones), the moves' root mean
# square. Always the Hausdorff distance of the two outlines (a shape without edges is its points), exactly: along a
# straight edge the distance to the other outline is the least of convex functions (the distance to each of its
# edges), so its largest value is at the edge's ends or where two of those functions are equal: two points (a line),
# a point and a line (a quadratic), two lines (their bisectors). Every such place is tried. The fixtures' outlines are
# straight.


def _sub(p, q):
    return (p[0] - q[0], p[1] - q[1])


def _dot(p, q):
    return p[0] * q[0] + p[1] * q[1]


def _xy(p):
    return (p["x"], p["y"])


def outline_of(e):
    """(vertices, edges) of an object's snapshot form."""
    k = e["kind"]
    if k == "point":
        return [_xy(e["p"])], []
    if k == "line":
        a, b = _xy(e["a"]), _xy(e["b"])
        return [a, b], [(a, b)]
    if k in ("polyline", "polygon"):
        assert not e.get("bulges"), "the fixtures' outlines are straight"
        rings = [[_xy(p) for p in e["pts"]]] + [[_xy(p) for p in h["pts"]] for h in e.get("holes", [])]
        closed = k == "polygon" or e.get("closed", False)
        verts, edges = [], []
        for r in rings:
            verts += r
            edges += list(zip(r, r[1:])) + ([(r[-1], r[0])] if closed and len(r) > 2 else [])
        return verts, edges
    return [], []


def _near_seg(p, a, b):
    v = _sub(b, a)
    l2 = _dot(v, v)
    if l2 == 0.0:
        return a
    t = min(1.0, max(0.0, _dot(_sub(p, a), v) / l2))
    return (a[0] + v[0] * t, a[1] + v[1] * t)


def _nearest(p, edges, points):
    best = (math.inf, p)
    for q in [_near_seg(p, a, b) for a, b in edges] + list(points):
        d = math.dist(p, q)
        if d < best[0]:
            best = (d, q)
    return best


def _crossings(a, b, edges, points):
    """The places along a → b (0 … 1) where two of the other outline's distance functions are equal."""
    v = _sub(b, a)
    verts = list(points) + [x for e in edges for x in e]
    lines = []
    for p, q in edges:
        w = _sub(q, p)
        n = math.hypot(*w)
        if n > 0:
            lines.append((p, (-w[1] / n, w[0] / n)))
    ts = []
    for i, P in enumerate(verts):
        for Q in verts[i + 1:]:
            den = 2.0 * _dot(v, _sub(Q, P))
            if den != 0.0:
                ts.append((_dot(_sub(a, Q), _sub(a, Q)) - _dot(_sub(a, P), _sub(a, P))) / den)
        for L, n in lines:
            nv, na = _dot(n, v), _dot(n, _sub(a, L))
            qa = _dot(v, v) - nv * nv
            qb = 2.0 * (_dot(v, _sub(a, P)) - na * nv)
            qc = _dot(_sub(a, P), _sub(a, P)) - na * na
            if abs(qa) < 1e-300:
                if qb != 0.0:
                    ts.append(-qc / qb)
                continue
            disc = qb * qb - 4.0 * qa * qc
            if disc >= 0.0:
                r = math.sqrt(disc)
                ts += [(-qb - r) / (2.0 * qa), (-qb + r) / (2.0 * qa)]
    for i, (L1, n1) in enumerate(lines):
        for L2, n2 in lines[i + 1:]:
            for sign in (1.0, -1.0):
                den = _dot(n1, v) - sign * _dot(n2, v)
                if den != 0.0:
                    ts.append((sign * _dot(n2, _sub(a, L2)) - _dot(n1, _sub(a, L1))) / den)
    return [t for t in ts if 0.0 < t < 1.0]


def directed(a_edges, a_points, b_edges, b_points):
    """The farthest a point of A is from B: (d, from, to)."""
    best = (-1.0, None, None)
    tries = list(a_points)
    for a, b in a_edges:
        tries += [a, b]
        v = _sub(b, a)
        tries += [(a[0] + v[0] * t, a[1] + v[1] * t) for t in _crossings(a, b, b_edges, b_points)]
    for p in tries:
        d, q = _nearest(p, b_edges, b_points)
        if d > best[0]:
            best = (d, p, q)
    return best


def shift_of(then, now):
    """docs/adr/4880: how far `now` moved from `then` (both snapshot forms)."""
    vt, et = outline_of(then)
    vn, en = outline_of(now)
    paired = then["kind"] == now["kind"] and len(vt) == len(vn) and len(vt) > 0
    out = {"paired": paired, "vertices": [len(vt), len(vn)]}
    if paired:
        top, total = None, 0.0
        for k, (p, q) in enumerate(zip(vt, vn)):
            dx, dy = q[0] - p[0], q[1] - p[1]
            d = math.hypot(dx, dy)
            total += d * d
            if top is None or d > top["d"]:
                top = {"index": k, "dx": dx, "dy": dy, "d": d}
        out["largest"] = top
        out["rms"] = math.sqrt(total / len(vt))
    pt, pn = ([] if et else vt), ([] if en else vn)
    if (et or pt) and (en or pn):
        d1, f1, t1 = directed(et, pt, en, pn)
        d2, f2, t2 = directed(en, pn, et, pt)
        # The way from the approved outline first; the other way only when it is farther.
        d, f, t = (d2, t2, f2) if d2 > d1 else (d1, f1, t1)
        out["hausdorff"] = {"d": d, "from": list(f), "to": list(t)}
    return out


# ── The rules (ADR 1200 §2) ──────────────────────────────────────────────────────────────────────────────────────────


class Drawing:
    """The drawing a case's steps leave: its layers' tree, its objects (uid → layer) and the work structure; the files
    that steps put in `$tmp:` (path → content's SHA-256)."""

    def __init__(self, d, files=None):
        self.d = copy.deepcopy(d)
        self.objects = dict(zip(self.d["uids"], (e["layerId"] for e in self.d["entities"])))
        self.entities = dict(zip(self.d["uids"], copy.deepcopy(self.d["entities"])))
        self.files = dict(CONTENT, **(files or {}))
        self.parents = {}

        def walk(nodes, parent):
            for n in nodes:
                self.parents[n["id"]] = parent
                walk(n["children"], n["id"])
        walk(self.d["layers"], None)

    @property
    def work(self):
        return self.d["settings"].get("mapWork") or {"items": [], "derivations": []}

    def items(self):
        return self.work.get("items", [])

    def derivations(self):
        return self.work.get("derivations", [])

    def get(self, id_):
        return next((i for i in self.items() if i["id"] == id_), None)

    def item_of_layer(self, layer_id):
        at = layer_id
        while at is not None:
            for i in self.items():
                if at in i.get("layers", []):
                    return i
            at = self.parents.get(at)
        return None

    def leaves(self):
        return [lid for lid in self.parents if not self.node(lid)["children"] and self.node(lid)["type"] == "layer"]

    def node(self, lid):
        def find(nodes):
            for n in nodes:
                if n["id"] == lid:
                    return n
                got = find(n["children"])
                if got:
                    return got
            return None
        return find(self.d["layers"])

    def look(self, f, drawing=None):
        """Where an item's file is (ADR 1201 §1): its path, its relative path from the drawing's folder, the search
        paths (each: the relative path there, then the file's name). (where, via)."""
        if f["path"] in self.files:
            return f["path"], "path"
        rel = f.get("relative")
        if drawing is not None and rel is not None:
            at = resolve(folder_of(drawing), rel)
            if at in self.files:
                return at, "relative"
        for base in self.d["settings"].get("xrefPaths", []):
            if not base.startswith("$"):
                if drawing is None:
                    continue
                base = resolve(folder_of(drawing), base)
            if rel is not None and resolve(base, rel) in self.files:
                return resolve(base, rel), "searchPath"
            named = base + "/" + f["path"].rsplit("/", 1)[-1]
            if named in self.files:
                return named, "searchPath"
        return None, "none"

    def file_state(self, i, deep, drawing=None):
        f = i["file"]
        at, via = self.look(f, drawing)
        if at is None:
            return "missing", None, via, None
        if not deep:
            return "found", None, via, at
        now = self.files[at]
        return ("current" if now == f["revision"] else "changed"), now, via, at

    def pairs(self, item_id):
        """ADR 2080 §1: [persistent id, object digest] of the objects an item's layers hold, by id (the list an approval
        keeps since docs/adr/2880)."""
        return sorted([u, digest(self.entities[u])] for u, lid in self.objects.items()
                      if (self.item_of_layer(lid) or {}).get("id") == item_id)

    def parts_of(self, item_id):
        """docs/adr/3280: the parts of the objects of `pairs(item_id)`, in its order."""
        return [parts(self.entities[u]) for u, _ in self.pairs(item_id)]

    def values_of(self, item_id):
        """docs/adr/3680: the values of the objects of `pairs(item_id)`, in its order."""
        return [value(self.entities[u]) for u, _ in self.pairs(item_id)]

    def content(self, item_id):
        """ADR 2080 §1: the content revision of an item without a file."""
        return sha(json.dumps(self.pairs(item_id), separators=(",", ":")).encode())

    def kept(self, i):
        """The revision an approval stamps and is judged against: its file's, or its content revision."""
        return i["file"]["revision"] if "file" in i else self.content(i["id"])

    def source_now(self, d, deep, drawing=None):
        i = self.get(d["source"])
        if "file" not in i:
            return self.content(i["id"])
        if deep:
            state, now, _, _ = self.file_state(i, True, drawing)
            if state == "changed":
                return now
        return i["file"]["revision"]

    def stale(self, d, deep, drawing=None):
        return self.source_now(d, deep, drawing) != d["revision"]

    def stale_objects_in(self, it, deep):
        n = 0
        for d in self.derivations():
            if not self.stale(d, deep):
                continue
            for u in d["uids"]:
                if u in self.objects:
                    held = self.item_of_layer(self.objects[u])
                    n += held is not None and held["id"] == it["id"]
        return n

    def suggested(self, kind):
        """docs/adr/5280: KentOS's profiles (tucbs-temel, harita-varliklari, imar-plani), then the drawing's copies,
        each with its classes whose `work` holds `kind`."""
        profiles = [PROFILES[n] for n in ("tucbs-temel", "harita-varliklari", "imar-plani")]
        profiles += [json.loads(f["text"]) for f in self.d["settings"].get("profileFiles", [])
                     if f["kind"] == "profile"]
        out = []
        for p in profiles:
            classes = [c["id"] for c in p["classes"] if kind in c.get("work", [])]
            if classes:
                out.append({"name": p["name"], "title": p["title"], "classes": classes})
        return out

    def tree(self, deep=False, of=None, drawing=None, limit=None, suggest=False):
        rows, derived, stale, problems = [], [], 0, 0
        for d in self.derivations():
            present = sum(u in self.objects for u in d["uids"])
            why = []
            old = self.stale(d, deep, drawing)
            if old:
                why.append(f"Kaynağı “{self.get(d['source'])['name']}” bu nesneler üretildikten sonra değişti.")
            gone = len(d["uids"]) - present
            if gone:
                why.append(f"{gone} nesnesi çizimde yok.")
                problems += 1
            state = "stale" if old else ("missing" if gone else "current")
            stale += old
            row = {"id": d["id"], "source": d["source"], "operation": d["operation"], "revision": d["revision"],
                   "objects": present, "recorded": len(d["uids"]), "state": state}
            if why:
                row["why"] = why
            derived.append(row)
        counts = {}
        for lid in self.leaves():
            held = self.item_of_layer(lid)
            if held:
                counts[held["id"]] = counts.get(held["id"], 0) + sum(1 for v in self.objects.values() if v == lid)
        for i in self.items():
            why, state = [], "current"

            def worse(s, words):
                nonlocal state
                why.append(words)
                if state == "current":
                    state = s
            row = {"id": i["id"], "kind": i["kind"], "name": i["name"], "status": i.get("status", "draft"),
                   "objects": counts.get(i["id"], 0)}
            if i.get("layers"):
                row["layers"] = list(i["layers"])
            if "file" in i:
                fs, _, via, at = self.file_state(i, deep, drawing)
                row["file"] = {"path": i["file"]["path"], "revision": i["file"]["revision"], "state": fs, "via": via}
                if at is not None and at != i["file"]["path"]:
                    row["file"]["at"] = at
                if "relative" in i["file"]:
                    row["file"]["relative"] = i["file"]["relative"]
                if fs == "missing":
                    worse("missing", f"Dosyası yok: {i['file']['path']}. Yeniden bağla ile yeni yerini gösterin.")
                elif fs == "changed":
                    worse("changed", "Dosyası projenin kabul ettiği içerikten farklı: Yenile ile kabul edin ya da "
                                     "Yeniden bağla ile başka dosyayı gösterin.")
            for lid in i.get("layers", []):
                if lid not in self.parents:
                    worse("incomplete", f"“{lid}” katmanı çizimde yok.")
            if i.get("status") == "approved":
                n = self.stale_objects_in(i, deep)
                if n:
                    worse("stale", f"Onaylı, ama katmanlarındaki {n} türetilmiş nesnenin kaynağı onaydan sonra değişti.")
            judged = "approved" in i or any(d["source"] == i["id"] for d in self.derivations())
            if "file" not in i and (deep or judged):
                row["content"] = self.content(i["id"])
            if "approved" in i:
                row["approved"] = i["approved"]
                now = self.kept(i)
                if i.get("status") == "approved" and now != i["approved"]:
                    # docs/adr/2880: what changed, object by object, when the approval kept its objects' digests.
                    if "file" not in i and i.get("approvedObjects"):
                        then = {u: dg for u, dg in i["approvedObjects"]}
                        nowp = {u: dg for u, dg in self.pairs(i["id"])}
                        added = sorted(u for u in nowp if u not in then)
                        changed_ = sorted(u for u in nowp if u in then and then[u] != nowp[u])
                        removed = [u for u in then if u not in nowp]
                        ch = {"added": len(added), "removed": len(removed), "changed": len(changed_)}
                        listed = sorted(added + changed_)[:1000]
                        if listed:
                            ch["uids"] = listed
                        # docs/adr/3280: with the approval's parts, each changed object's parts against its parts now.
                        if i.get("approvedParts"):
                            then_parts = {u: ps for (u, _), ps in zip(i["approvedObjects"], i["approvedParts"])}
                            by_part, part_objects = dict.fromkeys(PARTS, 0), []
                            # docs/adr/3680: with the approval's values, the attributes by name, then and now.
                            then_values = ({u: json.loads(v) for (u, _), v in zip(i["approvedObjects"],
                                                                                 i["approvedValues"])}
                                           if i.get("approvedValues") else None)
                            for u in changed_:
                                now_parts = parts(self.entities[u])
                                differ = [PARTS[k] for k in range(4) if then_parts[u][k] != now_parts[k]]
                                for name in differ:
                                    by_part[name] += 1
                                o = {"uid": u, "parts": differ}
                                if then_values is not None:
                                    now_v = {k: x for k, x in self.entities[u].items() if k != "id"}
                                    attrs, attached = attributes_changed(then_values[u], now_v)
                                    if attrs:
                                        o["attributes"] = attrs
                                    if attached:
                                        o["attachments"] = True
                                    # docs/adr/4480: a changed geometry's object as approved; docs/adr/4880: how
                                    # far it moved.
                                    if "geometry" in differ:
                                        o["approved"] = then_values[u]
                                        o["shift"] = shift_of(then_values[u], self.entities[u])
                                        if limit is not None:
                                            sh = o["shift"]
                                            d = sh["largest"]["d"] if sh["paired"] else sh.get("hausdorff", {}).get("d")
                                            if d is not None:
                                                sh["beyond"] = d > limit[0]
                                            if sh["paired"]:
                                                vt, vn = outline_of(then_values[u])[0], outline_of(self.entities[u])[0]
                                                sh["over"] = sum(math.hypot(q[0] - p[0], q[1] - p[1]) > limit[0]
                                                                 for p, q in zip(vt, vn))
                                part_objects.append(o)
                            ch["parts"] = by_part
                            if part_objects:
                                ch["objects"] = part_objects[:1000]
                            if then_values is not None and removed:
                                ch["removedObjects"] = [{"uid": u, "approved": then_values[u]} for u in removed][:1000]
                        if limit is not None:
                            ch["beyond"] = sum(o.get("shift", {}).get("beyond") is True
                                               for o in ch.get("objects", []))
                        row["changes"] = ch
                    what = "dosyası" if "file" in i else "içeriği"
                    worse("revised", f"Onaydan sonra {what} değişti (onaylanan {i['approved'][:12]}, şimdi {now[:12]}): "
                                     "değişikliği denetleyip yeniden onaylayın.")
                    k = row.get("changes", {}).get("beyond", 0)
                    if limit is not None and k:
                        why.append(f"{k} nesne seçilen sınırın dışına kaydı: {num_words(limit[0]).replace('.', ',')} m "
                                   f"({limit[1]}).")
            # docs/adr/4880: the typed layers' profile and how its classes stand.
            if "profile" in i:
                row["profile"] = self.profile_row(i, drawing)
            elif suggest:
                got = self.suggested(i["kind"])
                if got:
                    row["suggested"] = got
            problems += state != "current"
            row["state"] = state
            if why:
                row["why"] = why
            rows.append(row)
        out = {"items": rows, "derivations": derived, "stale": stale, "problems": problems, "deep": deep,
               "revision": "$current"}
        if limit is not None:
            out["limit"] = {"metres": limit[0], "source": limit[1]}
        if of is not None:
            out["object"] = self.object(of, deep)
        return out

    def doc_now(self):
        """The drawing as a document dict with its objects now (for `cad.profiles`' reference)."""
        d = copy.deepcopy(self.d)
        d["entities"] = [copy.deepcopy(self.entities[u]) for u in self.d["uids"] if u in self.entities]
        return d

    def profile_row(self, i, drawing=None):
        """docs/adr/4880: the item's profile narrowed to its classes, checked as cad.profiles.check checks it."""
        pr = i["profile"]
        if pr["name"] not in PROFILES:
            return {"name": pr["name"], "found": False}
        p = narrowed(PROFILES[pr["name"]], pr.get("classes", []))
        rep = PR.report(self.doc_now(), p)
        mine = [f for f in rep["findings"] if "class" in f][:1000]
        typed = {c["layer"] for c in rep["classes"] if "layer" in c}
        free = sorted({lid for lid in self.objects.values()
                       if lid not in typed and (self.item_of_layer(lid) or {}).get("id") == i["id"]})
        out = {"name": pr["name"], "found": True, "title": rep["title"], "version": rep["version"]}
        if rep["classes"]:
            out["classes"] = rep["classes"]
        if mine:
            out["findings"] = mine
        if free:
            out["free"] = free
        return out

    def object(self, u, deep):
        held = self.item_of_layer(self.objects[u])
        out = {"uid": u, "role": "free"}
        if held:
            out["item"] = held["id"]
        d = next((d for d in self.derivations() if u in d["uids"]), None)
        if d:
            out.update(role="derived", derivation=d["id"], source=d["source"], revision=d["revision"],
                       operation=d["operation"], current=not self.stale(d, deep))
        elif held and held["kind"] in ("campaign", "reference"):
            out.update(role="source", source=held["id"], revision=self.kept(held))
        return out

    def stale_count(self):
        return sum(self.stale(d, False) for d in self.derivations())

    def remove(self, u):
        """An object taken out of the drawing (`cad.entities.delete`)."""
        del self.objects[u]
        del self.entities[u]

    def set_work(self, work):
        work = {k: v for k, v in work.items() if v}
        if work:
            self.d["settings"]["mapWork"] = work
        else:
            self.d["settings"].pop("mapWork", None)


def narrowed(p, classes):
    """docs/adr/4880: a profile with only `classes` (none: all), the value lists they bind, the relations between two of
    them; no address data unless every class is kept."""
    if not classes:
        return copy.deepcopy(p)
    out = copy.deepcopy(p)
    out["classes"] = [c for c in p["classes"] if c["id"] in classes]
    out["relations"] = [r for r in p.get("relations", []) if r["parent"] in classes and r["child"] in classes]
    bound = {f["domain"] for c in out["classes"] for f in c["fields"] if f.get("domain")}
    out["domains"] = [d for d in p.get("domains", []) if d["id"] in bound]
    out.pop("address", None)
    return out


# ── Steps and answers ────────────────────────────────────────────────────────────────────────────────────────────────

UNCHANGED = {"revision": "same", "canUndo": False}
NOT_MAP = {"status": "failed", "error": {
    "code": "not_a_map_project",
    "message": "Harita işi yalnız harita projesinde (KentOS Harita) olur; bu çizim harita projesi değil."}}
BAD_REVISION = {"status": "failed", "error": {
    "code": "invalid_revision",
    "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
               "ya da komutun planından alın.", "path": "expectedRevision"}}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict",
    "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
               "yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def refused(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_, result, expect=None, command=None):
    s = {"op": op, "input": input_, "result": result}
    if command:
        s["command"] = command
    if expect is not None:
        s["expect"] = expect
    return s


def case(name, steps, note=None, d=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["steps"] = steps
    if d is not None:
        c["setup"] = d
    return c


def copy_file(src, dst):
    return {"op": "copyFile", "from": src, "to": dst}


def ask(dr, deep=False, of=None, op="execute", expect=None, drawing=None, within=None, limit=None, suggest=False):
    """The query's answer; it writes nothing (after a write in the case, the undo step stays: only the revision). With
    `within`, its numbers are held to that many metres (a shift's, docs/adr/4880). `limit` and `suggest`: ADR 5280."""
    input_ = {}
    if deep:
        input_["deep"] = True
    if drawing is not None:
        input_["drawing"] = drawing
    if of is not None:
        input_["uid"] = of
    if limit is not None:
        input_["limit"] = limit
    if suggest:
        input_["suggest"] = True
    used = None
    if limit is not None:
        used, bad = limit_of(limit)
        assert bad is None, bad
    s = step(op, input_, done(dr.tree(deep, of, drawing, used, suggest)), expect or UNCHANGED, command="map.work.query")
    if within is not None:
        s["within"] = within
    return s


# ── map.work.query ───────────────────────────────────────────────────────────────────────────────────────────────────


def query_cases():
    out = []
    base = setup()
    dr = Drawing(base)
    out.append(case("Bütün öğeler ve türetmeler: dosyalar yerinde, hepsi güncel", [ask(dr)],
                    "Hızlı okuma dosyaya yalnız bakar (found); her öğenin katmanlarındaki nesneler sayılır, grubun "
                    "altındaki katman grubun öğesinindir."))
    out.append(case("Derin okuma her dosyanın SHA-256'sını revizyonuyla karşılaştırır", [ask(dr, deep=True)]))
    for name, u in (("GNSS'ten içe aktarılan nokta türetilmiştir: kaynak, revizyon, işlem, güncel", G1),
                    ("Türetme kaydında olmayan arazi ölçümü noktası kaynaktır", G3),
                    ("Ortofotonun rasteri kaynaktır", R1),
                    ("Grubun altındaki katmanın nesnesi grubun öğesinindir (CAD altlığı)", A1),
                    ("Tasarım yüzeyinin eğrisi arazi ölçümünden türetilmiştir; tasarım öğesinin katmanındadır", T1),
                    ("Hiçbir öğenin katmanında olmayan çizgi kullanıcının çizimidir", F1)):
        out.append(case(name, [ask(dr, of=u)]))
    out.append(case("Kimlik UUID değil", [step("execute", {"uid": "G1"}, refused(
        "invalid_uid", "“G1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uid"),
        UNCHANGED)]))
    out.append(case("Çizimde olmayan nesne", [step("execute", {"uid": GHOST}, refused(
        "entity_not_found", f"“{GHOST}” kimlikli nesne çizimde yok.", "uid"), UNCHANGED)]))
    out.append(case("Harita projesi değil", [step("execute", {}, NOT_MAP, UNCHANGED)],
                    d=setup(work="architecture", map_work=None)))
    empty = setup(map_work=None)
    out.append(case("İş yapısı olmayan harita projesi: boş ağaç", [ask(Drawing(empty))], d=empty))
    gone_file = setup()
    gone_file["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:tasindi-1/kampanya.nmea"
    out.append(case("Arazi ölçümünün dosyası yerinde değil: öğe missing, yeniden bağlanmalı", [ask(Drawing(gone_file))],
                    d=gone_file))
    changed = setup()
    changed["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:degisti-1/kampanya.nmea"
    later = Drawing(changed, {"$tmp:degisti-1/kampanya.nmea": S_LATER})
    out.append(case("Arazi ölçümünün dosyası sonradan değişti: hızlı okuma görmez, derin okuma changed ve türevleri güncel değil",
                    [copy_file(P_LATER, "$tmp:degisti-1/kampanya.nmea"), ask(later), ask(later, deep=True),
                     ask(later, deep=True, of=G2)], d=changed))
    no_layer = setup()
    no_layer["settings"]["mapWork"]["items"][3]["layers"] = ["tasarim", "silinen"]
    out.append(case("Öğenin adlandırdığı katman çizimde yok: incomplete", [ask(Drawing(no_layer))], d=no_layer))
    deleted = Drawing(base)
    del deleted.objects[G2]
    out.append(case("Türetilmiş bir nesne silindi: kaydı missing; geri alınca güncel", [
        step("execute", {"uids": [G2]}, "$any", command="cad.entities.delete"),
        ask(deleted, expect={"revision": "same"}), {"op": "undo", "returns": "Sil"}, ask(dr, expect={"revision": "same"})]))
    moved = setup()
    moved["settings"]["mapWork"]["items"][0]["file"]["revision"] = S_LATER
    moved["settings"]["mapWork"]["items"][3]["status"] = "approved"
    md = Drawing(moved)
    out.append(case("Arazi ölçümü yenilendi: iki türetme güncel değil, onaylı tasarım öğesi stale", [ask(md)], d=moved))
    out.append(case("Güncel olmayan türetmenin nesnesi: current false", [ask(md, of=T2)], d=moved))
    out.append(case("Derin okumada dosya kayıtlı revizyonla aynıysa türetme yine güncel değildir (kayıt öğenin dosyasına göre)",
                    [ask(md, deep=True)], d=moved,
                    note="Öğenin revizyonu dosyanın içeriği değil; derin okuma dosyayı changed der, türetmeyi dosyanın "
                         "bugünkü içeriğiyle karşılaştırır: içerik türetmenin revizyonu, türetme güncel."))
    # ADR 1201 §1: a project moved with its data.
    moved_project = setup()
    f0 = moved_project["settings"]["mapWork"]["items"][0]["file"]
    f0.update(path="$tmp:eski-1/kampanya.nmea", relative="veri/kampanya.nmea")
    there = Drawing(moved_project, {"$tmp:proje-1/veri/kampanya.nmea": S_GNSS})
    out.append(case("Taşınan proje: dosya kayıtlı yolunda yok, proje klasöründen göreli yoluyla bulunur (relative)",
                    [copy_file(P_GNSS, "$tmp:proje-1/veri/kampanya.nmea"),
                     ask(there, drawing="$tmp:proje-1/ana.kcad"), ask(there, deep=True, drawing="$tmp:proje-1/ana.kcad"),
                     ask(there, drawing=None)], d=moved_project,
                    note="Göreli yol yalnız açık projenin dosyası (drawing) verilince aranır; verilmeyince dosya missing."))
    beside = setup()
    beside["settings"]["mapWork"]["items"][0]["file"].update(path="$tmp:eski-2/kampanya.nmea",
                                                            relative="../ortak/kampanya.nmea")
    up = Drawing(beside, {"$tmp:ortak/kampanya.nmea": S_GNSS})
    out.append(case("Göreli yol proje klasörünün dışına da çıkabilir (../ortak)",
                    [copy_file(P_GNSS, "$tmp:ortak/kampanya.nmea"), ask(up, drawing="$tmp:proje-2/ana.kcad")],
                    d=beside))
    searched = setup()
    searched["settings"]["xrefPaths"] = ["$tmp:arama-1"]
    searched["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:eski-3/kampanya.nmea"
    found = Drawing(searched, {"$tmp:arama-1/kampanya.nmea": S_GNSS})
    out.append(case("Göreli yolu olmayan dosya projenin arama yolunda adıyla bulunur (searchPath)",
                    [copy_file(P_GNSS, "$tmp:arama-1/kampanya.nmea"), ask(found, drawing="$tmp:proje-3/ana.kcad")],
                    d=searched))
    newer = setup()
    newer["settings"]["mapWork"]["items"][0]["file"].update(path="$tmp:eski-4/kampanya.nmea",
                                                           relative="veri/kampanya.nmea")
    other = Drawing(newer, {"$tmp:proje-4/veri/kampanya.nmea": S_LATER})
    out.append(case("Göreli yolda başka içerik: derin okuma changed, türevler güncel değil",
                    [copy_file(P_LATER, "$tmp:proje-4/veri/kampanya.nmea"),
                     ask(other, deep=True, drawing="$tmp:proje-4/ana.kcad")], d=newer))
    out.append(case("Açık projenin dosyası mutlak yol değil", [step("execute", {"drawing": "proje/ana.kcad"}, refused(
        "invalid_file_path", "“proje/ana.kcad” mutlak bir yol değil; açık projenin dosyasının tam yolunu verin.",
        "drawing"), UNCHANGED)]))
    # ADR 2080: a design's content revision, a derivation from it and an approval's stamp.
    staked = setup()
    staked["settings"]["mapWork"]["derivations"].append(
        derivation("tr-3", "is-4", Drawing(staked).content("is-4"), "Aplikasyon", [F1]))
    s0 = Drawing(staked)
    s1 = Drawing(staked)
    s1.entities[T1]["attrs"] = {"Kot": "1060"}
    out.append(case("Tasarımdan türetilen çizgi: tasarımın içeriği değişmedikçe güncel; bir çizgisinin özniteliği değişince "
                    "güncel değil; geri alınca yine güncel", [
                        ask(s0), ask(s0, of=F1),
                        step("execute", {"uids": [T1], "attrs": {"Kot": "1060"}, "operation": "attributes"}, "$any",
                             command="cad.entities.set"),
                        ask(s1, expect={"revision": "same"}), ask(s1, of=F1, expect={"revision": "same"}),
                        {"op": "undo", "returns": "Değiştir"}, ask(s0, expect={"revision": "same"})], d=staked,
                    note="Dosyası olmayan öğenin revizyonu katmanlarındaki nesnelerin özetidir (kentos.work-content/1); "
                         "türetme onunla karşılaştırılır."))
    approved = setup()
    approved["settings"]["mapWork"]["items"][3].update(status="approved", approved=Drawing(approved).content("is-4"))
    a0 = Drawing(approved)
    a1 = Drawing(approved)
    a1.entities[T2]["attrs"] = {"Kot": "1061"}
    out.append(case("Onaylı tasarımın damgası: içeriği aynıyken güncel; onaydan sonra bir çizgisi değişince revised", [
        ask(a0), step("execute", {"uids": [T2], "attrs": {"Kot": "1061"}, "operation": "attributes"}, "$any",
                      command="cad.entities.set"),
        ask(a1, expect={"revision": "same"})], d=approved))
    # docs/adr/2880: an approval that kept its objects' digests says what changed since, object by object: here the
    # approved list had the first object with another digest, lacked the second (added since) and held one not there
    # now (removed since).
    later = setup()
    now4 = Drawing(later).pairs("is-4")
    assert len(now4) >= 2, now4
    earlier = sorted([[now4[0][0], "0" * 64]] + [list(x) for x in now4[2:]]
                     + [["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff", "1" * 64]])
    later["settings"]["mapWork"]["items"][3].update(
        status="approved", approved=sha(json.dumps(earlier, separators=(",", ":")).encode()), approvedObjects=earlier)
    lt = Drawing(later)
    row4 = next(r for r in lt.tree(False)["items"] if r["id"] == "is-4")
    assert row4["changes"] == {"added": 1, "removed": 1, "changed": 1, "uids": sorted([now4[0][0], now4[1][0]])}, row4
    out.append(case("Onaydan sonra değişen nesneler: biri değişti, biri eklendi, biri silindi (onay nesne başına özet "
                    "tuttu)", [ask(lt)], d=later))
    # docs/adr/3280: the same approval with its objects' parts: the changed object's geometry and look parts were others.
    with_parts = copy.deepcopy(later)
    lp = Drawing(with_parts)
    by = {u: parts(lp.entities[u]) for u in lp.entities}
    kept_parts = [list(by[u]) if u in by else ["1" * 16] * 4 for u, _ in earlier]
    kept_parts[[u for u, _ in earlier].index(now4[0][0])][0] = "0" * 16
    kept_parts[[u for u, _ in earlier].index(now4[0][0])][3] = "0" * 16
    with_parts["settings"]["mapWork"]["items"][3]["approvedParts"] = kept_parts
    wp = Drawing(with_parts)
    row4 = next(r for r in wp.tree(False)["items"] if r["id"] == "is-4")
    assert row4["changes"]["parts"] == {"geometry": 1, "attributes": 0, "layer": 0, "look": 1}, row4
    assert row4["changes"]["objects"] == [{"uid": now4[0][0], "parts": ["geometry", "look"]}], row4
    out.append(case("Onaydan sonra değişen nesnenin değişen parçaları: geometrisi ve görünüşü (onay parçaları tuttu)",
                    [ask(wp)], d=with_parts))
    # An attribute set after an approval that kept the parts: the attributes part, and only it.
    attributed = setup()
    attributed["settings"]["mapWork"]["items"][3].update(status="approved", approved=Drawing(attributed).content("is-4"),
                                                         approvedObjects=Drawing(attributed).pairs("is-4"),
                                                         approvedParts=Drawing(attributed).parts_of("is-4"))
    at0 = Drawing(attributed)
    at1 = Drawing(attributed)
    at1.entities[T2]["attrs"] = {"Kot": "1061"}
    row4 = next(r for r in at1.tree(False)["items"] if r["id"] == "is-4")
    assert row4["changes"]["objects"] == [{"uid": T2, "parts": ["attributes"]}], row4
    out.append(case("Onaydan sonra özniteliği değişen nesne: yalnız öznitelikleri", [
        ask(at0), step("execute", {"uids": [T2], "attrs": {"Kot": "1061"}, "operation": "attributes"}, "$any",
                       command="cad.entities.set"),
        ask(at1, expect={"revision": "same"})], d=attributed))
    # docs/adr/3680: an approval that kept its objects' values: the changed attribute by name with its value as approved
    # and now; a removed object as it was approved.
    valued = setup()
    # The line T2 carries its height when approved; it is changed after.
    valued["entities"][valued["uids"].index(T2)]["attrs"] = {"Kot": "1060"}
    vd = Drawing(valued)
    valued["settings"]["mapWork"]["items"][3].update(status="approved", approved=vd.content("is-4"),
                                                     approvedObjects=vd.pairs("is-4"), approvedParts=vd.parts_of("is-4"),
                                                     approvedValues=vd.values_of("is-4"))
    v0 = Drawing(valued)
    v1 = Drawing(valued)
    v1.entities[T2]["attrs"] = {"Kot": "1061"}
    row4 = next(r for r in v1.tree(False)["items"] if r["id"] == "is-4")
    assert row4["changes"]["objects"][0]["attributes"] == [{"name": "Kot", "approved": "1060", "now": "1061"}], row4
    out.append(case("Onaydan sonra değişen özniteliğin adı ve onaylanan değeri (onay değerleri tuttu)", [
        ask(v0), step("execute", {"uids": [T2], "attrs": {"Kot": "1061"}, "operation": "attributes"}, "$any",
                      command="cad.entities.set"),
        ask(v1, expect={"revision": "same"})], d=valued))
    v2 = Drawing(valued)
    gone = v2.pairs("is-4")[0][0]
    v2.remove(gone)
    row4 = next(r for r in v2.tree(False)["items"] if r["id"] == "is-4")
    assert [o["uid"] for o in row4["changes"]["removedObjects"]] == [gone], row4
    out.append(case("Onaydan sonra silinen nesnenin onaylanan hâli (onay değerleri tuttu)", [
        ask(v0), step("execute", {"uids": [gone]}, "$any", command="cad.entities.delete"),
        ask(v2, expect={"revision": "same"})], d=valued))
    # docs/adr/4480: the line T2 moved after an approval that kept its objects' values: its geometry changed, the answer
    # carries it as approved (the shape Harita işi draws over the drawing); its height, unchanged, is not listed.
    v3 = Drawing(valued)
    for end in ("a", "b"):
        p_ = v3.entities[T2][end]
        v3.entities[T2][end] = {"x": p_["x"] + 0.25, "y": p_["y"] - 0.1}
    row4 = next(r for r in v3.tree(False)["items"] if r["id"] == "is-4")
    moved_t2 = next(o for o in row4["changes"]["objects"] if o["uid"] == T2)
    assert moved_t2["parts"] == ["geometry"] and "attributes" not in moved_t2, moved_t2
    assert moved_t2["approved"]["a"] == {k: v for k, v in vd.entities[T2].items() if k != "id"}["a"], moved_t2
    out.append(case("Onaydan sonra geometrisi değişen nesnenin onaylanan hâli (onay değerleri tuttu)", [
        ask(v0), step("execute", {"uids": [T2], "transform": {"kind": "move", "dx": 0.25, "dy": -0.1}}, "$any",
                      command="cad.entities.transform"),
        ask(v3, expect={"revision": "same"}, within=1e-9)], d=valued))
    # Both its geometry and its height changed: the attribute named and the object as approved.
    v4 = Drawing(valued)
    for end in ("a", "b"):
        p_ = v4.entities[T2][end]
        v4.entities[T2][end] = {"x": p_["x"] + 0.25, "y": p_["y"] - 0.1}
    v4.entities[T2]["attrs"] = {"Kot": "1061"}
    row4 = next(r for r in v4.tree(False)["items"] if r["id"] == "is-4")
    both_t2 = next(o for o in row4["changes"]["objects"] if o["uid"] == T2)
    assert both_t2["parts"] == ["geometry", "attributes"] and "approved" in both_t2, both_t2
    out.append(case("Onaydan sonra hem geometrisi hem özniteliği değişen nesne: ikisi de söylenir", [
        ask(v0), step("execute", {"uids": [T2], "transform": {"kind": "move", "dx": 0.25, "dy": -0.1}}, "$any",
                      command="cad.entities.transform"),
        step("execute", {"uids": [T2], "attrs": {"Kot": "1061"}, "operation": "attributes"}, "$any",
             command="cad.entities.set"),
        ask(v4, expect={"revision": "same"}, within=1e-9)], d=valued))
    # docs/adr/4880: the line T1 became a polyline with a corner 1 m off it: its vertices do not pair; the outlines are 1 m
    # apart at that corner (the line is at most 0.98 m from the bent one).
    bent = copy.deepcopy(valued)
    at1 = bent["uids"].index(T1)
    bent["entities"][at1] = {"kind": "polyline", "id": bent["entities"][at1]["id"], "layerId": "tasarim", "attrs": {},
                             "pts": [{"x": X0, "y": Y0}, {"x": X0 + 4.0, "y": Y0 + 1.0}, {"x": X0 + 10.0, "y": Y0}]}
    bd = Drawing(bent)
    row4 = next(r for r in bd.tree(False)["items"] if r["id"] == "is-4")
    bent_t1 = next(o for o in row4["changes"]["objects"] if o["uid"] == T1)["shift"]
    assert not bent_t1["paired"] and abs(bent_t1["hausdorff"]["d"] - 1.0) < 1e-12, bent_t1
    assert bent_t1["hausdorff"]["to"] == [X0 + 4.0, Y0 + 1.0], bent_t1
    out.append(case("Onaydan sonra köşe eklenen çizgi: köşeler eşlenmez, iki şeklin Hausdorff uzaklığı eklenen köşede",
                    [ask(bd, within=1e-9)], d=bent))
    # docs/adr/4880: the design item's typed layers: the control points' class of a data profile on its layer; a point
    # there without the class's fields; the design surface's layer stays a free drawing layer.
    typed = setup()
    typed["layers"].append(layer("hv", "Harita varlıkları", "group", children=[layer("hv-kn", "Kontrol noktaları")]))
    typed["entities"].append({"kind": "point", "id": 10, "layerId": "hv-kn", "attrs": {},
                              "p": {"x": X0 + 3.0, "y": Y0 + 3.0}})
    typed["uids"].append(uid(10))
    typed["settings"]["mapWork"]["items"][3].update(layers=["tasarim", "hv-kn"],
                                                    profile={"name": "harita-varliklari",
                                                             "classes": ["kontrol-noktasi"]})
    td = Drawing(typed)
    row4 = next(r for r in td.tree(False)["items"] if r["id"] == "is-4")
    assert row4["profile"]["free"] == ["tasarim"] and row4["profile"]["findings"], row4
    out.append(case("Tipli katman profili: öğenin sınıfları, bulguları ve serbest katmanı", [ask(td)], d=typed))
    unknown = setup()
    unknown["settings"]["mapWork"]["items"][3]["profile"] = {"name": "yok-profil"}
    out.append(case("Öğenin profili bulunmuyor: found false", [ask(Drawing(unknown))], d=unknown))
    same = setup()
    same["settings"]["mapWork"]["items"][3].update(status="approved", approved=Drawing(same).content("is-4"),
                                                   approvedObjects=Drawing(same).pairs("is-4"),
                                                   approvedParts=Drawing(same).parts_of("is-4"),
                                                   approvedValues=Drawing(same).values_of("is-4"))
    out.append(case("Onaylı ve nesneleri aynı: değişen yok, güncel", [ask(Drawing(same))], d=same))
    ortho = setup()
    ortho["settings"]["mapWork"]["items"][1].update(status="approved", approved=S_OLD)
    out.append(case("Onaylı ortofotonun damgası başka bir dosya revizyonu: onaydan sonra dosyası değişti (revised)",
                    [ask(Drawing(ortho))], d=ortho))
    out.extend(limit_cases(valued, v3, bent))
    out.extend(suggest_cases(typed))
    out.append(case("Plan yazmaz, aynı ağacı söyler", [ask(dr, op="plan")]))
    out.append(case("Doğrulama yazmaz", [step("validate", {}, done(None), UNCHANGED)]))
    out.append(case("Doğrulama kimliği de denetler", [step("validate", {"uid": GHOST}, refused(
        "entity_not_found", f"“{GHOST}” kimlikli nesne çizimde yok.", "uid"), UNCHANGED)]))
    return out


def limit_cases(valued, moved, bent):
    """docs/adr/5280: the line T2 moved 0.25 m east and 0.1 m south (0.269 m) after an approval that kept its objects'
    values, set against limits on either side; the bent line T1 (not paired) by its Hausdorff distance, 1 m."""
    out = []
    t2 = math.hypot(0.25, 0.1)
    move = step("execute", {"uids": [T2], "transform": {"kind": "move", "dx": 0.25, "dy": -0.1}}, "$any",
                command="cad.entities.transform")
    for name, lim, beyond in [
        ("kendi değeri 0,5 m: içinde", {"basis": "own", "metres": 0.5}, False),
        ("kendi değeri 0,2 m: dışında, iki köşesi de", {"basis": "own", "metres": 0.2}, True),
        ("detay konumu ±7 cm (BÖHHBÜY m. 46(1)): dışında", {"basis": "detail"}, True),
        ("çizimde ±0,2 mm, 1/1000 (0,2 m): dışında", {"basis": "drawing", "scale": 1000}, True),
        ("çizimde ±0,2 mm, 1/2000 (0,4 m): içinde", {"basis": "drawing", "scale": 2000}, False),
        ("sayısal üretim, 1/1000 (0,09 m): dışında", {"basis": "method", "method": "sayisal", "scale": 1000}, True),
        ("takeometrik, poligona yakın, 1/1000 (0,5 m): içinde",
         {"basis": "method", "method": "takeometrik-yakin", "scale": 1000}, False),
        ("prizmatik paftadan, 1/500 (0,25 m): dışında",
         {"basis": "method", "method": "prizmatik-pafta", "scale": 500}, True),
        ("prizmatik paftadan, 1/1000 (0,35 m): içinde",
         {"basis": "method", "method": "prizmatik-pafta", "scale": 1000}, False),
        ("sayısal fotogrametrik, 1/5000 (0,30 m): içinde",
         {"basis": "method", "method": "sayisal-fotogrametrik", "scale": 5000}, False),
    ]:
        used, _ = limit_of(lim)
        assert (t2 > used[0]) == beyond, (name, used)
        row = next(r for r in moved.tree(False, limit=used)["items"] if r["id"] == "is-4")
        sh = next(o for o in row["changes"]["objects"] if o["uid"] == T2)["shift"]
        assert sh["beyond"] == beyond and sh["over"] == (2 if beyond else 0), sh
        out.append(case(f"Kayma sınırı, {name}", [move, ask(moved, limit=lim, expect={"revision": "same"},
                                                                  within=1e-9)], d=valued))
    bd = Drawing(bent)
    used, _ = limit_of({"basis": "own", "metres": 0.9})
    row = next(r for r in bd.tree(False, limit=used)["items"] if r["id"] == "is-4")
    sh = next(o for o in row["changes"]["objects"] if o["uid"] == T1)["shift"]
    assert sh["beyond"] is True and "over" not in sh, sh
    out.append(case("Kayma sınırı: köşeleri eşlenmeyen çizgi Hausdorff uzaklığıyla (1 m > 0,9 m)",
                    [ask(bd, limit={"basis": "own", "metres": 0.9}, within=1e-9)], d=bent))
    out.append(case("Kayma sınırı, değişen nesne yok: yalnız sınır söylenir",
                    [ask(Drawing(setup()), limit={"basis": "detail"})]))
    out.append(case("Kayma sınırıyla plan yazmaz",
                    [move, ask(moved, op="plan", limit={"basis": "own", "metres": 0.2}, expect={"revision": "same"},
                               within=1e-9)], d=valued))
    for name, lim in [
        ("kendi değerinde metres yok", {"basis": "own"}),
        ("kendi değeri 0", {"basis": "own", "metres": 0}),
        ("kendi değeri 2000 m", {"basis": "own", "metres": 2000}),
        ("çizim doğruluğunda ölçek yok", {"basis": "drawing"}),
        ("ölçek paydası 0", {"basis": "drawing", "scale": 0}),
        ("üretim yöntemi verilmemiş", {"basis": "method", "scale": 1000}),
        ("tabloda olmayan yöntem", {"basis": "method", "method": "lidar", "scale": 1000}),
        ("üretim yönteminde ölçek yok", {"basis": "method", "method": "sayisal"}),
        ("tabloda olmayan ölçek 1/1500", {"basis": "method", "method": "sayisal", "scale": 1500}),
        ("tablonun boş hücresi: fotogrametrik 1/1000", {"basis": "method", "method": "fotogrametrik", "scale": 1000}),
    ]:
        _, (path, message) = limit_of(lim)
        out.append(case(f"Kayma sınırı reddi: {name}", [step("execute", {"limit": lim}, refused(
            "invalid_input", message, path), UNCHANGED, command="map.work.query")]))
    _, (path, message) = limit_of({"basis": "own", "metres": -1})
    out.append(case("Kayma sınırı kimlikten önce denetlenir; doğrulama da reddeder", [step("validate", {
        "limit": {"basis": "own", "metres": -1}, "uid": GHOST}, refused("invalid_input", message, path), UNCHANGED)]))
    out.append(case("Harita projesi olmayan çizimde önce proje reddi", [step("execute", {
        "limit": {"basis": "own"}}, NOT_MAP, UNCHANGED, command="map.work.query")],
        d=setup(work="road", map_work=None)))
    return out


def suggest_cases(typed):
    """docs/adr/5280: suggested profiles by the item's kind."""
    out = []
    base = setup()
    dr = Drawing(base)
    rows = {r["id"]: r for r in dr.tree(False, suggest=True)["items"]}
    assert rows["is-1"]["suggested"][0]["name"] == "harita-varliklari" and "suggested" not in rows["is-2"], rows
    assert rows["is-4"]["suggested"] == [{"name": "imar-plani", "title": PROFILES["imar-plani"]["title"],
                                          "classes": [c["id"] for c in PROFILES["imar-plani"]["classes"]]}], rows
    out.append(case("Önerilen profil: arazi ölçümüne harita varlıkları, tasarıma imar planı, altlığa yok",
                    [ask(dr, suggest=True)]))
    more = setup()
    more["settings"]["mapWork"]["items"] += [item("is-6", "working", "Çalışma verisi", ["cizim"]),
                                             item("is-7", "delivery", "Teslim paketi", ["bos"])]
    md = Drawing(more)
    rows = {r["id"]: r for r in md.tree(False, suggest=True)["items"]}
    assert rows["is-7"]["suggested"][0]["name"] == "tucbs-temel" and "suggested" not in rows.get("is-3", {}), rows
    out.append(case("Önerilen profil: çalışma verisine harita varlıkları, teslime TUCBS", [ask(md, suggest=True)],
                    d=more))
    out.append(case("Profili olan öğeye öneri yok", [ask(Drawing(typed), suggest=True)], d=typed))
    # An office's own profile carried inside the drawing: its boundary point class for survey campaigns and checks.
    office = copy.deepcopy(narrowed(PROFILES["harita-varliklari"], ["kontrol-noktasi", "duvar"]))
    office.update(name="kurum-olcum", title="Belediye ölçüm profili")
    office["classes"][0]["work"] = ["campaign", "check"]
    office["classes"][1].pop("work", None)
    kurum = setup()
    kurum["settings"]["profileFiles"] = [{"kind": "profile", "name": "kurum-olcum",
                                         "text": json.dumps(office, ensure_ascii=False)}]
    kurum["settings"]["mapWork"]["items"].append(item("is-6", "check", "Denetim", ["bos"]))
    kd = Drawing(kurum)
    rows = {r["id"]: r for r in kd.tree(False, suggest=True)["items"]}
    assert [x["name"] for x in rows["is-1"]["suggested"]] == ["harita-varliklari", "kurum-olcum"], rows
    assert rows["is-6"]["suggested"] == [{"name": "kurum-olcum", "title": "Belediye ölçüm profili",
                                          "classes": ["kontrol-noktasi"]}], rows
    out.append(case("Kurumun profili çizimin içinde: önerilere eklenir, yalnız türüne uyan sınıfıyla",
                    [ask(kd, suggest=True)], d=kurum))
    out.append(case("Önerisiz sorguda öneri yok", [ask(kd)], d=kurum))
    return out


# ── map.work.edit ────────────────────────────────────────────────────────────────────────────────────────────────────


def edit(input_, out, expect, op="execute"):
    return step(op, input_, out, expect, command="map.work.edit")


def write_step(before, after, input_, fields, label, warnings=(), extra=None, then_undo=True):
    """The execute step and, by default, its undo back to `before`."""
    expect = {"revision": "changed", "canUndo": True, "settings": {"mapWork": after.d["settings"].get("mapWork")}}
    expect.update(extra or {})
    steps = [edit(input_, done(edited(after, **fields), warnings), expect)]
    if then_undo:
        steps.append({"op": "undo", "returns": label,
                      "expect": {"canRedo": True, "settings": {"mapWork": before.d["settings"].get("mapWork")}}})
    return steps


def fail_step(input_, code, message, path):
    return edit(input_, refused(code, message, path), {"revision": "same", "canUndo": False})


def op(o, **kw):
    return {"operation": dict(o, **kw)}


def edit_cases():
    out = []
    base = setup()
    dr = Drawing(base)

    def changed(fn, files=None):
        nd = Drawing(base, files)
        w = copy.deepcopy(nd.work)
        fn(w)
        nd.set_work(w)
        return nd

    # add
    after = changed(lambda w: w["items"].append(item("is-6", "campaign", "GNSS ölçümü 2026-11", ["bos"],
                                                     (P_LATER, S_LATER))))
    out.append(case("Ekle: dosyasıyla yeni arazi ölçümü is-6, revizyonu dosyanın SHA-256'sı; tek adımda geri alınır",
                    write_step(dr, after, op({"op": "add", "kind": "campaign", "name": "GNSS ölçümü 2026-11",
                                              "layers": ["bos"], "file": P_LATER}),
                               {"item": "is-6", "file_revision": S_LATER}, "Harita işine ekle")))
    after = changed(lambda w: w["items"].append(item("is-6", "delivery", "Teslim paketi A")))
    out.append(case("Ekle: dosyasız, katmansız teslim paketi",
                    write_step(dr, after, op({"op": "add", "kind": "delivery", "name": "Teslim paketi A"}),
                               {"item": "is-6"}, "Harita işine ekle")))
    out.append(case("Ekle: başka öğenin katmanı", [fail_step(
        op({"op": "add", "kind": "working", "name": "Çalışma", "layers": ["bos", "gnss"]}), "layer_in_work_item",
        "“gnss” katmanı harita işinin “GNSS ölçümü 2026-10” öğesinde; bir katman tek öğenin olur.",
        "operation.layers[1]")]))
    out.append(case("Ekle: aynı katman iki kez", [fail_step(
        op({"op": "add", "kind": "working", "name": "Çalışma", "layers": ["bos", "bos"]}), "layer_in_work_item",
        "“bos” katmanı iki kez verilmiş.", "operation.layers[1]")]))
    out.append(case("Ekle: çizimde olmayan katman", [fail_step(
        op({"op": "add", "kind": "working", "name": "Çalışma", "layers": ["yok"]}), "layer_not_found",
        "“yok” kimlikli katman çizimde yok.", "operation.layers[0]")]))
    for name, value in (("boş", ""), ("boşlukla başlayan", " Çalışma")):
        out.append(case(f"Ekle: {name} ad", [fail_step(
            op({"op": "add", "kind": "working", "name": value}), "invalid_work_text",
            "Öğenin adı boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.", "operation.name")]))
    out.append(case("Ekle: göreli yol", [fail_step(
        op({"op": "add", "kind": "reference", "name": "Altlık", "file": "veri/plan.dxf"}), "invalid_file_path",
        "“veri/plan.dxf” mutlak bir dosya yolu değil; dosyanın tam yolunu verin.", "operation.file")]))
    out.append(case("Ekle: olmayan dosya", [fail_step(
        op({"op": "add", "kind": "reference", "name": "Altlık", "file": "$tmp:yok/plan.dxf"}), "file_not_found",
        "“$tmp:yok/plan.dxf” dosyası yok.", "operation.file")]))
    # set
    stamp4 = dr.content("is-4")
    pairs4 = dr.pairs("is-4")
    parts4 = dr.parts_of("is-4")
    values4 = dr.values_of("is-4")
    # docs/adr/2880: an approval of an item without a file keeps its objects' digests; docs/adr/3280: and their parts;
    # docs/adr/3680: and their values.
    after = changed(lambda w: w["items"][3].update(status="approved", approved=stamp4, approvedObjects=pairs4,
                                                   approvedParts=parts4, approvedValues=values4))
    out.append(case("Durum: denetlenen tasarım onaylı teslim revizyonu olur (ofisin kaydı); onay içerik revizyonunu "
                    "damgalar", write_step(dr, after, op({"op": "set", "item": "is-4", "status": "approved"}),
                                           {"item": "is-4", "content": stamp4, "approved": stamp4},
                                           "Harita işi öğesini değiştir")))
    after = changed(lambda w: w["items"][1].update(status="approved", approved=S_ORTHO))
    out.append(case("Onay: dosyası olan ortofotonun damgası dosya revizyonudur",
                    write_step(dr, after, op({"op": "set", "item": "is-2", "status": "approved"}),
                               {"item": "is-2", "file_revision": S_ORTHO, "approved": S_ORTHO},
                               "Harita işi öğesini değiştir")))
    moved_on = setup()
    moved_on["settings"]["mapWork"]["items"][3].update(status="approved", approved="0" * 64)
    mo = Drawing(moved_on)
    mo_after = Drawing(moved_on)
    w = copy.deepcopy(mo_after.work)
    w["items"][3]["approved"] = stamp4
    w["items"][3]["approvedObjects"] = pairs4
    w["items"][3]["approvedParts"] = parts4
    w["items"][3]["approvedValues"] = values4
    mo_after.set_work(w)
    out.append(case("Yeniden onayla: onaydan sonra değişen tasarım bugünkü içerik revizyonuyla yeniden damgalanır",
                    write_step(mo, mo_after, op({"op": "set", "item": "is-4", "status": "approved"}),
                               {"item": "is-4", "content": stamp4, "approved": stamp4}, "Harita işi öğesini değiştir"),
                    d=moved_on))
    kept = setup()
    kept["settings"]["mapWork"]["items"][3].update(status="approved", approved=stamp4)
    # docs/adr/2880: an unchanged item approved before its objects were kept gains them when approved again.
    ka = Drawing(kept)
    ka_after = Drawing(kept)
    w = copy.deepcopy(ka_after.work)
    w["items"][3]["approvedObjects"] = pairs4
    w["items"][3]["approvedParts"] = parts4
    w["items"][3]["approvedValues"] = values4
    ka_after.set_work(w)
    out.append(case("Yeniden onayla: değişmemiş, nesneleri tutulmadan onaylanmış öğe nesne başına özetini kazanır",
                    write_step(ka, ka_after, op({"op": "set", "item": "is-4", "status": "approved"}),
                               {"item": "is-4", "content": stamp4, "approved": stamp4}, "Harita işi öğesini değiştir"),
                    d=kept))
    # docs/adr/3280: approved with its objects' digests but not their parts (schema 210): approving again keeps the parts.
    no_parts = setup()
    no_parts["settings"]["mapWork"]["items"][3].update(status="approved", approved=stamp4, approvedObjects=pairs4)
    np = Drawing(no_parts)
    np_after = Drawing(no_parts)
    w = copy.deepcopy(np_after.work)
    w["items"][3]["approvedParts"] = parts4
    w["items"][3]["approvedValues"] = values4
    np_after.set_work(w)
    out.append(case("Yeniden onayla: değişmemiş, parçaları tutulmadan (şema 210) onaylanmış öğe parçalarını kazanır",
                    write_step(np, np_after, op({"op": "set", "item": "is-4", "status": "approved"}),
                               {"item": "is-4", "content": stamp4, "approved": stamp4}, "Harita işi öğesini değiştir"),
                    d=no_parts))
    # docs/adr/3680: approved with its objects' parts but not their values (schema 211): approving again keeps them.
    no_values = setup()
    no_values["settings"]["mapWork"]["items"][3].update(status="approved", approved=stamp4, approvedObjects=pairs4,
                                                        approvedParts=parts4)
    nv = Drawing(no_values)
    nv_after = Drawing(no_values)
    w = copy.deepcopy(nv_after.work)
    w["items"][3]["approvedValues"] = values4
    nv_after.set_work(w)
    out.append(case("Yeniden onayla: değişmemiş, değerleri tutulmadan (şema 211) onaylanmış öğe değerlerini kazanır",
                    write_step(nv, nv_after, op({"op": "set", "item": "is-4", "status": "approved"}),
                               {"item": "is-4", "content": stamp4, "approved": stamp4}, "Harita işi öğesini değiştir"),
                    d=no_values))
    whole = setup()
    whole["settings"]["mapWork"]["items"][3].update(status="approved", approved=stamp4, approvedObjects=pairs4,
                                                    approvedParts=parts4, approvedValues=values4)
    out.append(case("Yeniden onayla: değişmemiş onaylı öğe", [fail_step(
        op({"op": "set", "item": "is-4", "status": "approved"}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")], d=whole))
    kd = Drawing(whole)
    kd_after = Drawing(whole)
    w = copy.deepcopy(kd_after.work)
    w["items"][3]["status"] = "review"
    del w["items"][3]["approved"]
    # docs/adr/2880: its objects' digests go with the stamp; docs/adr/3280: and their parts.
    del w["items"][3]["approvedObjects"]
    del w["items"][3]["approvedParts"]
    del w["items"][3]["approvedValues"]
    kd_after.set_work(w)
    out.append(case("Onaydan çıkan öğenin damgası ve nesne başına özeti kalkar",
                    write_step(kd, kd_after, op({"op": "set", "item": "is-4", "status": "review"}), {"item": "is-4"},
                               "Harita işi öğesini değiştir"), d=whole))
    after = changed(lambda w: w["items"][1].update(name="Ortofoto 2025 (30 cm)", layers=["ortofoto", "bos"]))
    out.append(case("Değiştir: ortofotonun adı ve katmanları",
                    write_step(dr, after, op({"op": "set", "item": "is-2", "name": "Ortofoto 2025 (30 cm)",
                                              "layers": ["ortofoto", "bos"]}),
                               {"item": "is-2", "file_revision": S_ORTHO}, "Harita işi öğesini değiştir")))
    out.append(case("Değiştir: aynı değerler", [fail_step(
        op({"op": "set", "item": "is-4", "status": "review"}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")]))
    out.append(case("Değiştir: olmayan öğe", [fail_step(
        op({"op": "set", "item": "is-9", "status": "review"}), "unknown_work_item",
        "Harita işinde “is-9” kimlikli öğe yok.", "operation.item")]))
    renewed = setup()
    renewed["settings"]["mapWork"]["items"][0]["file"]["revision"] = S_LATER
    out.append(case("Onay: katmanlarında güncel olmayan türetilmiş nesne olan öğe onaylanmaz", [fail_step(
        op({"op": "set", "item": "is-4", "status": "approved"}), "approval_stale",
        "“Tasarım yüzeyi” onaylanamaz: katmanlarındaki 2 türetilmiş nesnenin kaynağı değişti; önce onları güncelleyin.",
        "operation.status")], d=renewed))
    # remove
    after = changed(lambda w: (w["items"].pop(0), w.update(derivations=[])))
    out.append(case("Çıkar: arazi ölçümü ve ondan türetme kayıtları gider, nesneler kalır",
                    write_step(dr, after, op({"op": "remove", "item": "is-1"}), {"item": "is-1", "released": 2},
                               "Harita işinden çıkar", extra={"ids": list(range(1, 10))})))
    # relink
    moved = "$tmp:tasindi-2/i8.tif"
    after = changed(lambda w: w["items"][1]["file"].update(path=moved), {moved: S_ORTHO})
    steps = [copy_file(P_ORTHO, moved)]
    steps += write_step(dr, after, op({"op": "relink", "item": "is-2", "file": moved}),
                        {"item": "is-2", "file_revision": S_ORTHO, "relinked": 1}, "Kaynağı yeniden bağla",
                        extra={"fields": {"4": {"file": moved}}})
    steps[-1]["expect"]["fields"] = {"4": {"file": P_ORTHO}}
    out.append(case("Yeniden bağla: taşınan ortofoto tek yerden; rasteri yeni dosyayı gösterir, içerik aynı, türev "
                    "güncel; geri alma ikisini birlikte geri getirir", steps))
    after = changed(lambda w: w["items"][0]["file"].update(path=P_LATER, revision=S_LATER))
    out.append(case("Yeniden bağla: içeriği farklı yeni kayıt dosyası; iki türetme güncel değil",
                    write_step(dr, after, op({"op": "relink", "item": "is-1", "file": P_LATER}),
                               {"item": "is-1", "file_revision": S_LATER}, "Kaynağı yeniden bağla",
                               warnings=[{"code": "source_content_changed", "path": "operation.file", "message":
                                          "Yeni dosyanın içeriği öncekinden farklı: “GNSS ölçümü 2026-10” "
                                          "kaynağından türetilmiş 2 kayıt güncel değil."}])
                    + [{"op": "redo", "returns": "Kaynağı yeniden bağla"}, ask(after, of=G1, expect={"revision": "same"})]))
    out.append(case("Yeniden bağla: aynı dosya", [fail_step(
        op({"op": "relink", "item": "is-1", "file": P_GNSS}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")]))
    out.append(case("Yeniden bağla: dosyası olmayan öğe", [fail_step(
        op({"op": "relink", "item": "is-4", "file": P_GNSS}), "work_item_no_file",
        "“Tasarım yüzeyi” öğesinin dosyası yok; yalnız dosyası olan bir kaynak yeniden bağlanır ya da yenilenir.",
        "operation.item")]))
    out.append(case("Yenile: dosyası olmayan öğe", [fail_step(
        op({"op": "refresh", "item": "is-4"}), "work_item_no_file",
        "“Tasarım yüzeyi” öğesinin dosyası yok; yalnız dosyası olan bir kaynak yeniden bağlanır ya da yenilenir.",
        "operation.item")]))
    old_moved = "$tmp:tasindi-3/geographic.tif"
    out.append(case("Yeniden bağla: rasteri kilitli katmanda", [copy_file(P_OLD, old_moved), fail_step(
        op({"op": "relink", "item": "is-5", "file": old_moved}), "layer_locked",
        "“Kilitli” katmanı kilitli: üstündeki resim yeni dosyaya bağlanamaz. Kilidi açın.", "operation.item")]))
    out.append(case("Yeniden bağla: yeni yerde dosya yok", [fail_step(
        op({"op": "relink", "item": "is-2", "file": "$tmp:yok/i8.tif"}), "file_not_found",
        "“$tmp:yok/i8.tif” dosyası yok.", "operation.file")]))
    # refresh
    refresh = setup()
    refresh["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:yenile-1/kampanya.nmea"
    rd = Drawing(refresh, {"$tmp:yenile-1/kampanya.nmea": S_LATER})
    rafter = Drawing(refresh, {"$tmp:yenile-1/kampanya.nmea": S_LATER})
    w = copy.deepcopy(rafter.work)
    w["items"][0]["file"]["revision"] = S_LATER
    rafter.set_work(w)
    out.append(case("Yenile: dosyanın bugünkü içeriği revizyon olur; türevleri güncel değil",
                    [copy_file(P_LATER, "$tmp:yenile-1/kampanya.nmea")]
                    + write_step(rd, rafter, op({"op": "refresh", "item": "is-1"}),
                                 {"item": "is-1", "file_revision": S_LATER}, "Kaynağı yenile"), d=refresh))
    out.append(case("Yenile: dosya değişmedi", [fail_step(
        op({"op": "refresh", "item": "is-1"}), "nothing_to_set", "Değişecek bir şey yok.", "operation")]))
    gone = setup()
    gone["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:tasindi-4/kampanya.nmea"
    out.append(case("Yenile: dosya yok", [fail_step(
        op({"op": "refresh", "item": "is-1"}), "file_not_found",
        "“$tmp:tasindi-4/kampanya.nmea” dosyası yok; Yeniden bağla ile yeni yerini gösterin.", "operation.item")],
        d=gone))
    # derive
    after = changed(lambda w: w["derivations"].append(derivation("tr-3", "is-3", S_DXF, "Altlıktan kopya", [F1])))
    out.append(case("Türet: çizgi CAD altlığından kopyalandı diye kaydedilir; nesnenin kaynağı bulunur",
                    write_step(dr, after, op({"op": "derive", "source": "is-3", "uids": [F1, F1],
                                              "operation": "Altlıktan kopya"}),
                               {"item": "is-3", "derivation": "tr-3", "file_revision": S_DXF}, "Kaynaktan türet")
                    + [{"op": "redo", "returns": "Kaynaktan türet"}, ask(after, of=F1, expect={"revision": "same"})]))
    out.append(case("Türet: başka kayıttaki nesne", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [F1, G1], "operation": "Altlıktan kopya"}), "already_derived",
        f"“{G1}” nesnesi “tr-1” türetme kaydında; önce o kaydı silin.", "operation.uids[1]")]))
    after = changed(lambda w: w["derivations"].append(derivation("tr-3", "is-4", stamp4, "Aplikasyon", [F1])))
    out.append(case("Türet: aplikasyon çizgisi dosyası olmayan tasarımdan türetilir; revizyonu tasarımın içerik revizyonu",
                    write_step(dr, after, op({"op": "derive", "source": "is-4", "uids": [F1], "operation": "Aplikasyon"}),
                               {"item": "is-4", "derivation": "tr-3", "content": stamp4}, "Kaynaktan türet")
                    + [{"op": "redo", "returns": "Kaynaktan türet"}, ask(after, of=F1, expect={"revision": "same"})]))
    own = setup()
    own["settings"]["mapWork"]["derivations"].pop(1)
    out.append(case("Türet: dosyası olmayan öğenin kendi katmanındaki nesne", [fail_step(
        op({"op": "derive", "source": "is-4", "uids": [F1, T1], "operation": "Kopya"}), "own_content",
        f"“{T1}” nesnesi “Tasarım yüzeyi” öğesinin kendi katmanında; dosyası olmayan öğe kendi nesnelerinin kaynağı "
        "olamaz.", "operation.uids[1]")], d=own))
    out.append(case("Türet: olmayan kaynak", [fail_step(
        op({"op": "derive", "source": "is-9", "uids": [F1], "operation": "Kopya"}), "unknown_work_item",
        "Harita işinde “is-9” kimlikli öğe yok.", "operation.source")]))
    out.append(case("Türet: nesne yok", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [], "operation": "Kopya"}), "no_entities",
        "Türetilen nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "operation.uids")]))
    out.append(case("Türet: kimlik UUID değil", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": ["F1"], "operation": "Kopya"}), "invalid_uid",
        "“F1” geçerli bir nesne kimliği değil.", "operation.uids[0]")]))
    out.append(case("Türet: çizimde olmayan nesne", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [GHOST], "operation": "Kopya"}), "entity_not_found",
        f"“{GHOST}” kimlikli nesne çizimde yok.", "operation.uids[0]")]))
    out.append(case("Türet: boşluklu işlem adı", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [F1], "operation": "Kopya "}), "invalid_work_text",
        "Üretim işlemi boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.", "operation.operation")]))
    # forget
    after = changed(lambda w: w["derivations"].pop(0))
    out.append(case("Kaydı sil: noktalar yine arazi ölçümünün, artık kaynak olarak",
                    write_step(dr, after, op({"op": "forget", "derivation": "tr-1"}), {"derivation": "tr-1"},
                               "Türetme kaydını sil")
                    + [{"op": "redo", "returns": "Türetme kaydını sil"}, ask(after, of=G1, expect={"revision": "same"})]))
    out.append(case("Kaydı sil: olmayan kayıt", [fail_step(
        op({"op": "forget", "derivation": "tr-9"}), "unknown_derivation",
        "Harita işinde “tr-9” kimlikli türetme kaydı yok.", "operation.derivation")]))
    # project, revision, plan
    out.append(case("Harita projesi değil", [edit(op({"op": "add", "kind": "delivery", "name": "Paket"}), NOT_MAP,
                                                   {"revision": "same", "canUndo": False})],
                    d=setup(work="architecture", map_work=None)))
    out.append(case("İş yapısı olmayan harita projesinde ilk öğe", write_step(
        Drawing(setup(map_work=None)),
        Drawing(setup(map_work={"items": [item("is-1", "reference", "Ortofoto", ["ortofoto"], (P_ORTHO, S_ORTHO))]})),
        op({"op": "add", "kind": "reference", "name": "Ortofoto", "layers": ["ortofoto"], "file": P_ORTHO}),
        {"item": "is-1", "file_revision": S_ORTHO}, "Harita işine ekle"), d=setup(map_work=None)))
    out.append(case("Geçersiz beklenen sürüm", [edit(dict(op({"op": "refresh", "item": "is-1"}), expectedRevision="abc"),
                                                      BAD_REVISION, {"revision": "same", "canUndo": False})]))
    out.append(case("Beklenen sürüm eski", [edit(dict(op({"op": "refresh", "item": "is-1"}), expectedRevision="999"),
                                                  CONFLICT, {"revision": "same", "canUndo": False})]))
    # release (ADR 1201 §3)
    after = changed(lambda w: w["derivations"][0].update(uids=[G1]))
    out.append(case("Bağımsız yap: G2 kaydından çıkar, kayıt G1 ile kalır; tek adımda geri alınır",
                    write_step(dr, after, op({"op": "release", "uids": [G2]}), {"freed": 1}, "Bağımsız yap")
                    + [{"op": "redo", "returns": "Bağımsız yap"}, ask(after, of=G2, expect={"revision": "same"})]))
    after = changed(lambda w: w.update(derivations=[w["derivations"][0]]))
    out.append(case("Bağımsız yap: kaydın bütün nesneleri; boşalan kayıt kalkar",
                    write_step(dr, after, op({"op": "release", "uids": [T1, T2, T1]}), {"freed": 2, "released": 1},
                               "Bağımsız yap")))
    after = changed(lambda w: w.update(derivations=[]))
    out.append(case("Bağımsız yap: iki kayıttan nesneler birlikte",
                    write_step(dr, after, op({"op": "release", "uids": [G1, T1, G2, T2]}), {"freed": 4, "released": 2},
                               "Bağımsız yap")))
    out.append(case("Bağımsız yap: kullanıcının kendi çizimi", [fail_step(
        op({"op": "release", "uids": [G1, F1]}), "not_derived",
        f"“{F1}” nesnesi hiçbir türetme kaydında değil; zaten bağımsız çizim.", "operation.uids[1]")]))
    out.append(case("Bağımsız yap: kaynak nokta türetilmiş değil", [fail_step(
        op({"op": "release", "uids": [G3]}), "not_derived",
        f"“{G3}” nesnesi hiçbir türetme kaydında değil; zaten bağımsız çizim.", "operation.uids[0]")]))
    out.append(case("Bağımsız yap: nesne verilmedi", [fail_step(
        op({"op": "release", "uids": []}), "no_entities",
        "Bağımsız yapılacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "operation.uids")]))
    out.append(case("Bağımsız yap: kimlik UUID değil", [fail_step(
        op({"op": "release", "uids": ["G1"]}), "invalid_uid", "“G1” geçerli bir nesne kimliği değil.",
        "operation.uids[0]")]))
    # relink takes along the external references on the old file (ADR 1201 §2)
    with_refs = setup()
    with_refs["blocks"] = [xref_block(XREF_SAME, "Kadastro", P_DXF, S_DXF),
                           xref_block(XREF_OTHER, "Kadastro eski", P_DXF, S_LATER),
                           xref_block(XREF_ELSE, "Başka plan", P_GNSS, S_GNSS)]
    rd = Drawing(with_refs)
    moved_dxf = "$tmp:tasindi-3/kadastro.dxf"
    after_refs = Drawing(with_refs, {moved_dxf: S_DXF})
    w = copy.deepcopy(after_refs.work)
    w["items"][2]["file"]["path"] = moved_dxf
    after_refs.set_work(w)
    steps = [copy_file(P_DXF, moved_dxf)]
    steps += write_step(rd, after_refs, op({"op": "relink", "item": "is-3", "file": moved_dxf}),
                        {"item": "is-3", "file_revision": S_DXF, "links": 1}, "Kaynağı yeniden bağla",
                        warnings=[{"code": "link_not_followed", "path": "operation.file", "message":
                                   "“Kadastro eski” dış başvurusu başka bir içerik yüklemiş; yeni dosyaya Dış başvurular › "
                                   "Yolu değiştir ile geçirin."}],
                        extra={"xrefs": {"Kadastro": xref_of(moved_dxf, S_DXF), "Kadastro eski": xref_of(P_DXF, S_LATER),
                                         "Başka plan": xref_of(P_GNSS, S_GNSS)}})
    steps[-1]["expect"]["xrefs"] = {"Kadastro": xref_of(P_DXF, S_DXF)}
    out.append(case("Yeniden bağla: aynı içeriği yükleyen dış başvuru öğeyle birlikte yeni dosyaya geçer, başka içerik "
                    "yükleyen kalır ve söylenir; geri alma hepsini birlikte geri getirir", steps, d=with_refs))
    # docs/adr/4880: an item's typed layers' profile. The classes' layers the drawing lacks are added as the profile
    # places them (cad.profiles' own reference writes them) and join the item; its other layers stay free.
    p4 = narrowed(PROFILES["harita-varliklari"], ["kontrol-noktasi", "rogar"])
    d4 = dr.doc_now()
    applied, warned = PR.apply(d4, p4)
    class_ids = [PR.find(d4["layers"], c["layer"])[1] for c in p4["classes"]]
    d4["settings"]["mapWork"]["items"][3]["layers"] = ["tasarim"] + class_ids
    d4["settings"]["mapWork"]["items"][3]["profile"] = {"name": "harita-varliklari",
                                                        "classes": ["kontrol-noktasi", "rogar"]}
    after = Drawing(d4)
    steps = [edit(op({"op": "profile", "item": "is-4", "name": "harita-varliklari",
                      "classes": ["kontrol-noktasi", "rogar"]}),
                  done(edited(after, item="is-4", layers=applied["layers"], fields=applied["fields"]), warned),
                  {"revision": "changed", "canUndo": True, "layers": d4["layers"],
                   "settings": {"mapWork": after.d["settings"]["mapWork"]}}),
             {"op": "undo", "returns": "Harita işi öğesine profil ver",
              "expect": {"canRedo": True, "layers": base["layers"], "settings": {"mapWork": dr.work}}}]
    out.append(case("Profil: kontrol noktası ve rogar sınıflarının katmanları eklenir ve tasarım öğesine girer; tasarım "
                    "yüzeyinin katmanı serbest kalır; tek adımda geri alınır", steps))
    out.append(case("Profil: adı bilinmeyen profil", [fail_step(
        op({"op": "profile", "item": "is-4", "name": "yok-profil"}), "unknown_profile",
        "“yok-profil” adlı veri profili yok; KentOS'un profilleri: tucbs-temel, harita-varliklari, imar-plani.",
        "operation.name")]))
    have = ", ".join(c["id"] for c in PROFILES["harita-varliklari"]["classes"])
    out.append(case("Profil: profilde olmayan sınıf", [fail_step(
        op({"op": "profile", "item": "is-4", "name": "harita-varliklari", "classes": ["rogar", "kopru"]}),
        "unknown_profile_class", f"“harita-varliklari” profilinde “kopru” sınıfı yok; sınıfları: {have}.",
        "operation.classes[1]")]))
    out.append(case("Profil: aynı sınıf iki kez", [fail_step(
        op({"op": "profile", "item": "is-4", "name": "harita-varliklari", "classes": ["rogar", "rogar"]}),
        "invalid_work_text", "“rogar” sınıfı iki kez verilmiş.", "operation.classes[1]")]))
    with_profile = setup()
    with_profile["settings"]["mapWork"]["items"][3]["profile"] = {"name": "imar-plani"}
    wpd = Drawing(with_profile)
    no_profile = Drawing(with_profile)
    w = copy.deepcopy(no_profile.work)
    del w["items"][3]["profile"]
    no_profile.set_work(w)
    out.append(case("Profili kaldır: katmanlar öğede kalır", write_step(
        wpd, no_profile, op({"op": "profile", "item": "is-4"}), {"item": "is-4"},
        "Harita işi öğesinin profilini kaldır"), d=with_profile))
    out.append(case("Profili olmayan öğeden profil kaldırılmaz", [fail_step(
        op({"op": "profile", "item": "is-4"}), "nothing_to_set", "Değişecek bir şey yok.", "operation")]))
    # Approving an item whose typed layers break its profile: refused with the count and the first finding.
    typed = setup()
    typed["layers"].append(layer("hv", "Harita varlıkları", "group", children=[layer("hv-kn", "Kontrol noktaları")]))
    typed["entities"].append({"kind": "point", "id": 10, "layerId": "hv-kn", "attrs": {},
                              "p": {"x": X0 + 3.0, "y": Y0 + 3.0}})
    typed["uids"].append(uid(10))
    typed["settings"]["mapWork"]["items"][3].update(layers=["tasarim", "hv-kn"],
                                                    profile={"name": "harita-varliklari",
                                                             "classes": ["kontrol-noktasi"]})
    rep_ = PR.report(Drawing(typed).doc_now(), narrowed(PROFILES["harita-varliklari"], ["kontrol-noktasi"]))
    mine = [f for f in rep_["findings"] if "class" in f]
    out.append(case("Onay: tipli katmanlarında profiline uymayan nesne olan öğe onaylanmaz", [fail_step(
        op({"op": "set", "item": "is-4", "status": "approved"}), "approval_profile",
        f"“Tasarım yüzeyi” onaylanamaz: tipli katmanlarında “{rep_['title']}” profiline uymayan {len(mine)} bulgu var; "
        f"ilki: {mine[0]['message']} Profili denetle ile düzeltin.", "operation.status")], d=typed))
    # The class's layer another item holds: refused, nothing written.
    taken = copy.deepcopy(typed)
    taken["settings"]["mapWork"]["items"][3].update(layers=["tasarim"])
    del taken["settings"]["mapWork"]["items"][3]["profile"]
    taken["settings"]["mapWork"]["items"][0]["layers"] = ["gnss", "hv-kn"]
    out.append(case("Profil: sınıfın katmanı başka öğede", [fail_step(
        op({"op": "profile", "item": "is-4", "name": "harita-varliklari", "classes": ["kontrol-noktasi"]}),
        "layer_in_work_item",
        "Profilin “Harita varlıkları / Kontrol noktaları” katmanı harita işinin “GNSS ölçümü 2026-10” öğesinde; bir "
        "katman tek öğenin olur.", "operation.name")], d=taken))
    plan_after = changed(lambda w: w["items"].append(item("is-6", "check", "Topoloji denetimi")))
    for o in ("plan", "validate"):
        result = done(edited(plan_after, item="is-6")) if o == "plan" else done(None)
        out.append(case(f"{'Plan' if o == 'plan' else 'Doğrulama'} yazmaz", [edit(
            op({"op": "add", "kind": "check", "name": "Topoloji denetimi"}), result,
            {"revision": "same", "canUndo": False, "settings": {"mapWork": dr.work}}, op=o)]))
    return out


def edited(dr, item=None, derivation=None, file_revision=None, relinked=0, released=0, links=0, freed=0, content=None,
           approved=None, layers=(), fields=()):
    o = {}
    if item is not None:
        o["item"] = item
    if derivation is not None:
        o["derivation"] = derivation
    if file_revision is not None:
        o["fileRevision"] = file_revision
    if content is not None:
        o["content"] = content
    if approved is not None:
        o["approved"] = approved
    if relinked:
        o["relinked"] = relinked
    if released:
        o["released"] = released
    if links:
        o["links"] = links
    if freed:
        o["freed"] = freed
    o["stale"] = dr.stale_count()
    if layers:
        o["layers"] = list(layers)
    if fields:
        o["fields"] = list(fields)
    o["revision"] = "$current"
    return o


def doc(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": setup(), "cases": cases}


NOTE = ("ADR 1200. Bağımsız başvuru scripts/fixtures/map_work_cases.py: kurallar ve cümleler ADR'den elle yazılır; "
        "revizyon dosyanın SHA-256'sı (hashlib); kaynak dosyaları fixtures/map-work/v1 ve fixtures/raster/v1/files; "
        "KentOS kodu yok.")


def main():
    texts = {
        OUT / "map.work.query.json": doc("map.work.query", "Harita işini sorgula", query_cases()),
        OUT / "map.work.edit.json": doc("map.work.edit", "Harita işini düzenle", edit_cases()),
    }
    texts = {p: json.dumps(v, ensure_ascii=False, indent=1) + "\n" for p, v in texts.items()}
    files = {FILES / name: data for name, data in WRITTEN.items()}
    if "--check" in sys.argv[1:]:
        bad = [p for p, b in files.items() if not p.exists() or p.read_bytes() != b]
        bad += [p for p, t in texts.items() if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        if bad:
            return 1
        print("Harita işi ortak durumları güncel.")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    for p, b in files.items():
        p.write_bytes(b)
    for p, t in texts.items():
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
