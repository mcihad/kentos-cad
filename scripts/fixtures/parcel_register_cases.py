#!/usr/bin/env python3
"""The shared cases of `cad.parcels.record` and `cad.parcels.records` (Parsel kayıtları; plan/map-cad.md MAP-08,
docs/adr/4220), without KentOS code.

    python3 scripts/fixtures/parcel_register_cases.py           # writes the two files
    python3 scripts/fixtures/parcel_register_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.parcels.record.json and cad.parcels.records.json. The rules, by hand (after ArcGIS's
parcel fabric records: a record creates parcels and retires earlier ones, which stay as history; a line goes historic
once no current parcel uses it):

- a document (its name unique in the drawing; kind, date, note as written) makes parcels from areas and closes earlier
  ones. A parcel's identity is its area's İl/İlçe/Mahalle/Ada/Parsel; its stated area the text given, else the area's
  `Tapu alanı (m²)`, a plain decimal kept as written; its rings, part by part, each outline before its holes, are
  chains of boundaries;
- an edge is a boundary from its lesser corner (by east, then north) to the other, its bulge turned with it. An edge
  with the same two corners (as stored) and a bulge within 1e-9 of a boundary current before the document, or of one
  the document made already, is that boundary, negative in the ring where run the other way; else a new boundary,
  numbered on;
- the closed parcels lose their area; a boundary current before the document that no current parcel uses after it is
  closed by the document;
- reading: a parcel's recorded area is its rings' (shoelace plus each arc's circular segment, holes less), its area now
  its drawing area's; `current` when that area draws each of its boundaries (either way round) and has no other edge,
  `changed` when it draws otherwise, `missing` without an area, `closed`; its neighbours are the current parcels it
  shares a boundary with; `before` the parcels its document closed, `after` those the document that closed it made; a
  boundary is `drawn` when every current parcel using it draws it.

The coordinates are the project's (from X0, Y0); areas and lengths compare within 1e-6 m (`within`).
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"
X0, Y0 = 500000.0, 4400000.0
STEP = "Parsel kaydı"
SAME = {"revision": "same", "canUndo": False}
WITHIN = 1e-6
TOLERANCE = 1e-9
FIELD = "Tapu alanı (m²)"


def uid(n):
    return f"0192f5c0-4220-7000-8000-{n:012x}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def who(il="Ankara", ilce="Çankaya", mahalle="Kızılay", ada="101", parsel="7", tapu=None, **more):
    attrs = {"İl": il, "İlçe": ilce, "Mahalle": mahalle, "Ada": ada, "Parsel": parsel}
    if tapu is not None:
        attrs[FIELD] = tapu
    attrs.update(more)
    return {k: v for k, v in attrs.items() if v is not None}


def area(attrs, pts, bulges=None, holes=None, parts=None):
    e = {"kind": "polygon", "layerId": "parsel", "attrs": attrs, "pts": [P(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": [P(*p) for p in h]} for h in holes]
    if parts:
        e["parts"] = [{"pts": [P(*p) for p in q]} for q in parts]
    return e


# The drawing: Ada 101's parcels and the areas the refusals need. The slot is the object's place, 1 first.
OBJECTS = [
    ("A1", area(who(parsel="7", tapu="200.00"), [(0, 0), (20, 0), (20, 10), (0, 10)])),
    ("A2", area(who(parsel="8", tapu="199,98"), [(20, 0), (40, 0), (40, 10), (20, 10)])),
    # Its north edge an arc bowed out (bulge 0.2 from (20, 20) to (0, 20)); its south edge A1's north one.
    ("A3", area(who(parsel="9", tapu="215,3"), [(0, 10), (20, 10), (20, 20), (0, 20)], bulges=[0, 0, 0.2, 0])),
    # Its west edge runs from (40, 20) to (40, 0) whole: A2's east edge is only part of it (a T junction).
    ("A4", area(who(parsel="10", tapu="300"), [(40, 0), (60, 0), (60, 20), (40, 20)],
                holes=[[(45, 5), (55, 5), (55, 15), (45, 15)]])),
    ("A5", area(who(parsel="11"), [(0, 30), (10, 30), (10, 40), (0, 40)],
                parts=[[(20, 30), (30, 30), (30, 40), (20, 40)]])),
    ("A6", area({"Parsel": "12"}, [(80, 0), (90, 0), (90, 10), (80, 10)])),
    ("A7", area(who(ada="102", parsel="1", tapu="1.250,50"), [(100, 0), (110, 0), (110, 10), (100, 10)])),
    ("A8", area(who(ada="102", parsel="2"), [(120, 0), (130, 0), (130, 0), (130, 10), (120, 10)])),
    ("L9", {"kind": "line", "layerId": "parsel", "attrs": {}, "a": P(0, -10), "b": P(40, -10)}),
    # 101/7 measured again: its east edge 5 cm further.
    ("A10", area(who(parsel="7", tapu="200,50"), [(0, 0), (20.05, 0), (20.05, 10), (0, 10)])),
    # 101/7 divided in two (ifraz) and 101/7 with 101/8 joined (tevhit).
    ("A11", area(who(parsel="13", tapu="100"), [(0, 0), (10, 0), (10, 10), (0, 10)])),
    ("A12", area(who(parsel="14", tapu="100"), [(10, 0), (20, 0), (20, 10), (10, 10)])),
    ("A13", area(who(parsel="15", tapu="399,98"), [(0, 0), (40, 0), (40, 10), (0, 10)])),
    ("A14", area(who(ada="103", parsel="1", mahalle="M" * 121), [(140, 0), (150, 0), (150, 10), (140, 10)])),
]
SLOT = {name: i + 1 for i, (name, _) in enumerate(OBJECTS)}
ENTS = [dict(e, id=i + 1) for i, (_, e) in enumerate(OBJECTS)]


def U(name):
    return uid(SLOT[name])


def setup(register=None, ents=None):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"}
    if register is not None:
        settings["parcelRegister"] = register
    ents = copy.deepcopy(ents if ents is not None else ENTS)
    return {"format": "kentos.document", "version": 2, "name": "Parsel kayıtları", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": [layer("0", "0"), layer("parsel", "Parsel")],
            "activeLayer": "parsel", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


# ── The geometry ─────────────────────────────────────────────────────────


def rings_of(e):
    """An area's rings: (part, hole, places as (x, y), each edge's bulge)."""
    def ring(pts, bulges):
        places = [(p["x"], p["y"]) for p in pts]
        return places, [float(bulges[k]) if bulges and k < len(bulges) else 0.0 for k in range(len(places))]
    out = [(0, False, *ring(e["pts"], e.get("bulges")))]
    out += [(0, True, *ring(h["pts"], h.get("bulges"))) for h in e.get("holes", [])]
    for n, q in enumerate(e.get("parts", []), start=1):
        out.append((n, False, *ring(q["pts"], q.get("bulges"))))
        out += [(n, True, *ring(h["pts"], h.get("bulges"))) for h in q.get("holes", [])]
    return out


def segment(a, b, bulge):
    """The circular segment between an edge's chord and its arc, signed with the bulge (0 for a straight edge)."""
    if bulge == 0:
        return 0.0
    c = math.hypot(b[0] - a[0], b[1] - a[1])
    theta = 4 * math.atan(abs(bulge))
    r = c / (2 * math.sin(theta / 2))
    return math.copysign(r * r / 2 * (theta - math.sin(theta)), bulge)


def ring_area(places, bulges):
    """A ring's area, its shoelace taken from its first vertex (near 4 400 000 m the products would lose the
    millimetres otherwise)."""
    n = len(places)
    ox, oy = places[0]
    rel = [(x - ox, y - oy) for x, y in places]
    shoelace = sum(rel[k][0] * rel[(k + 1) % n][1] - rel[(k + 1) % n][0] * rel[k][1] for k in range(n)) / 2
    return abs(shoelace + sum(segment(places[k], places[(k + 1) % n], bulges[k]) for k in range(n)))


def area_of(rings):
    return sum(-ring_area(p, b) if hole else ring_area(p, b) for _, hole, p, b in rings)


def length(b):
    a, z = (b["from"]["x"], b["from"]["y"]), (b["to"]["x"], b["to"]["y"])
    c = math.hypot(z[0] - a[0], z[1] - a[1])
    if not b.get("bulge"):
        return c
    theta = 4 * math.atan(abs(b["bulge"]))
    return c * theta / (2 * math.sin(theta / 2))


# ── The register, by hand ────────────────────────────────────────────────


def empty():
    return {"documents": [], "boundaries": [], "parcels": []}


def record(reg, document, create=(), close=(), ents=None):
    """The register after a document: `create` (object name, stated area or None), `close` (parcel numbers). The
    answer as `cad.parcels.record` gives it."""
    ents = ents if ents is not None else ENTS
    reg = copy.deepcopy(reg) if reg else empty()
    number = max((d["number"] for d in reg["documents"]), default=0) + 1
    d = {"number": number, "name": document["name"]}
    d.update({k: document[k] for k in ("kind", "date", "note") if document.get(k)})
    reg["documents"].append(d)
    before = [b for b in reg["boundaries"] if "closedBy" not in b]
    earlier = len(reg["boundaries"])
    made = []
    shared = set()
    next_b = max((b["number"] for b in reg["boundaries"]), default=0) + 1
    next_p = max((p["number"] for p in reg["parcels"]), default=0) + 1
    created = []
    for name, stated in create:
        e = ents[SLOT[name] - 1]
        attrs = e["attrs"]
        rings = []
        for part, hole, places, bulges in rings_of(e):
            edges = []
            for k in range(len(places)):
                p, q, bulge = places[k], places[(k + 1) % len(places)], bulges[k]
                frm, to, bulge, back = (p, q, bulge, False) if p <= q else (q, p, -bulge, True)
                hit = next((b for b in before + made if (b["from"]["x"], b["from"]["y"]) == frm
                            and (b["to"]["x"], b["to"]["y"]) == to and abs(b.get("bulge", 0.0) - bulge) <= TOLERANCE),
                           None)
                if hit is None:
                    hit = {"number": next_b, "from": {"x": frm[0], "y": frm[1]}, "to": {"x": to[0], "y": to[1]},
                           "createdBy": number}
                    if bulge != 0:
                        hit["bulge"] = bulge
                    next_b += 1
                    made.append(hit)
                    reg["boundaries"].append(hit)
                elif hit in before:
                    shared.add(hit["number"])
                edges.append(-hit["number"] if back else hit["number"])
            ring = {"edges": edges}
            if part:
                ring["part"] = part
            if hole:
                ring["hole"] = True
            rings.append(ring)
        parcel = {"number": next_p}
        for key, attr in (("il", "İl"), ("ilce", "İlçe"), ("mahalle", "Mahalle"), ("ada", "Ada"), ("parsel", "Parsel")):
            if attrs.get(attr, "").strip():
                parcel[key] = attrs[attr].strip()
        parcel["createdBy"] = number
        text = stated if stated is not None else attrs.get(FIELD, "")
        if text:
            parcel["statedArea"] = text
        parcel["uid"] = U(name)
        parcel["rings"] = rings
        created.append(next_p)
        next_p += 1
        reg["parcels"].append(parcel)
    for p in reg["parcels"]:
        if p["number"] in close:
            p["closedBy"] = number
            p.pop("uid", None)
    used = {abs(e) for p in reg["parcels"] if "closedBy" not in p for r in p["rings"] for e in r["edges"]}
    gone = 0
    for b in reg["boundaries"]:
        if "closedBy" not in b and b["number"] not in used:
            b["closedBy"] = number
            gone += 1
    out = {"document": number, "created": created, "closed": list(close),
           "boundariesCreated": len(reg["boundaries"]) - earlier, "boundariesShared": len(shared),
           "boundariesClosed": gone, "revision": "$current"}
    return reg, out


def label(p):
    return f"{p['ada']}/{p['parsel']}"


def listed(reg, ents, history=False):
    """`cad.parcels.records`'s answer, by hand."""
    if not reg:
        return {"documents": [], "parcels": [], "boundaries": [], "current": 0, "closed": 0, "changed": 0,
                "missing": 0, "moved": 0}
    by = {b["number"]: b for b in reg["boundaries"]}
    drawing = {uid(i + 1): e for i, e in enumerate(ents)}

    def run(e):
        b = by[abs(e)]
        a, z = (b["from"]["x"], b["from"]["y"]), (b["to"]["x"], b["to"]["y"])
        return (a, z, b.get("bulge", 0.0)) if e > 0 else (z, a, -b.get("bulge", 0.0))

    def draws(rings, b):
        a, z, bulge = (b["from"]["x"], b["from"]["y"]), (b["to"]["x"], b["to"]["y"]), b.get("bulge", 0.0)
        for _, _, places, bulges in rings:
            n = len(places)
            for k in range(n):
                p, q, w = places[k], places[(k + 1) % n], bulges[k]
                if (p == a and q == z and abs(w - bulge) <= TOLERANCE) or (p == z and q == a and abs(w + bulge) <= TOLERANCE):
                    return True
        return False

    rows, now = [], {}
    for p in reg["parcels"]:
        recorded = []
        for r in p["rings"]:
            runs = [run(e) for e in r["edges"]]
            recorded.append((r.get("part", 0), r.get("hole", False), [x[0] for x in runs], [x[2] for x in runs]))
        numbers = []
        for r in p["rings"]:
            for e in r["edges"]:
                if abs(e) not in numbers:
                    numbers.append(abs(e))
        e = drawing.get(p.get("uid")) if "uid" in p else None
        rings = rings_of(e) if e is not None and e["kind"] == "polygon" else None
        if "closedBy" in p:
            state = "closed"
        elif rings is None:
            state = "missing"
        else:
            count = sum(len(r["edges"]) for r in p["rings"])
            same = sum(len(x[2]) for x in rings) == count and all(draws(rings, by[abs(x)]) for r in p["rings"]
                                                                   for x in r["edges"])
            state = "current" if same else "changed"
        if rings is not None:
            now[p["number"]] = rings
        row = {"number": p["number"], "il": p.get("il", ""), "ilce": p.get("ilce", ""), "mahalle": p.get("mahalle", ""),
               "ada": p["ada"], "parsel": p["parsel"], "createdBy": p["createdBy"]}
        if "closedBy" in p:
            row["closedBy"] = p["closedBy"]
        if p.get("statedArea"):
            row["statedArea"] = p["statedArea"]
        row["recordedArea"] = area_of(recorded)
        if "uid" in p:
            row["uid"] = p["uid"]
        if rings is not None:
            row["area"] = area_of(rings)
        row["state"] = state
        row["boundaries"] = numbers
        row["neighbours"] = []
        row["before"] = [q["number"] for q in reg["parcels"] if q.get("closedBy") == p["createdBy"]]
        row["after"] = ([q["number"] for q in reg["parcels"] if q["createdBy"] == p["closedBy"]]
                        if "closedBy" in p else [])
        rows.append(row)
    users = {}
    every = {}
    for row in rows:
        for b in row["boundaries"]:
            every.setdefault(b, []).append(row["number"])
            if "closedBy" not in row:
                users.setdefault(b, []).append(row["number"])
    for row in rows:
        if "closedBy" not in row:
            row["neighbours"] = sorted({n for b in row["boundaries"] for n in users.get(b, []) if n != row["number"]})
    lines = []
    for b in reg["boundaries"]:
        current = "closedBy" not in b
        parcels = list((users if current else every).get(b["number"], []))
        line = {"number": b["number"], "from": [b["from"]["x"], b["from"]["y"]], "to": [b["to"]["x"], b["to"]["y"]]}
        if b.get("bulge"):
            line["bulge"] = b["bulge"]
        line["length"] = length(b)
        line["createdBy"] = b["createdBy"]
        if not current:
            line["closedBy"] = b["closedBy"]
        line["parcels"] = parcels
        line["drawn"] = current and all(n in now and draws(now[n], b) for n in parcels)
        lines.append(line)
    documents = []
    for d in reg["documents"]:
        row = dict(d)
        row["created"] = [p["number"] for p in reg["parcels"] if p["createdBy"] == d["number"]]
        row["closed"] = [p["number"] for p in reg["parcels"] if p.get("closedBy") == d["number"]]
        documents.append(row)
    return {
        "documents": documents,
        "parcels": [r for r in rows if history or "closedBy" not in r],
        "boundaries": [b for b in lines if history or "closedBy" not in b],
        "current": sum(1 for r in rows if "closedBy" not in r),
        "closed": sum(1 for r in rows if "closedBy" in r),
        "changed": sum(1 for r in rows if r["state"] == "changed"),
        "missing": sum(1 for r in rows if r["state"] == "missing"),
        "moved": sum(1 for b in lines if "closedBy" not in b and not b["drawn"]),
    }


# ── The cases ────────────────────────────────────────────────────────────

KADASTRO = {"name": "Kadastro 1985", "kind": "Tapu planı", "date": "1985-06-12"}
IFRAZ = {"name": "2026/145 ifraz", "kind": "Değişiklik", "date": "2026-03-14", "note": "Ölçü krokisi 2026/145."}


def doc_input(document):
    return {k: v for k, v in document.items()}


def wanted(create):
    out = []
    for name, stated in create:
        item = {"uid": U(name)}
        if stated is not None:
            item["statedArea"] = stated
        out.append(item)
    return out


def record_case(name, document, create=(), close=(), reg=None, op="execute", note=None):
    after, out = record(reg, document, create, close)
    input_ = {"document": doc_input(document)}
    if create:
        input_["create"] = wanted(create)
    if close:
        input_["close"] = list(close)
    case = {"name": name}
    if note:
        case["note"] = note
    if reg is not None:
        case["setup"] = setup(reg)
    if op != "execute":
        result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
        case["steps"] = [{"op": op, "input": input_, "result": result, "expect": SAME}]
        return case
    case["steps"] = [
        {"op": "execute", "input": input_, "result": {"status": "completed", "output": out, "warnings": []},
         "expect": {"revision": "changed", "canUndo": True, "settings": {"parcelRegister": after}}},
        {"op": "undo", "returns": STEP, "expect": {"settings": {"parcelRegister": reg}}},
    ]
    return case


def refused(name, input_, code, message, path_, reg=None):
    case = {"name": name}
    if reg is not None:
        case["setup"] = setup(reg)
    case["steps"] = [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]
    return case


def first():
    """Kadastro 1985: 101/7, 101/8, 101/9 (parcels 1, 2, 3)."""
    return record(None, KADASTRO, [("A1", None), ("A2", None), ("A3", None)])[0]


def record_cases():
    one, _ = record(None, KADASTRO, [("A1", None)])
    two, _ = record(None, KADASTRO, [("A1", None), ("A2", None)])
    three = first()
    closed_one, _ = record(three, IFRAZ, [("A11", None), ("A12", None)], [1])
    brief = {"name": "2026/12 tescil"}
    return [
        record_case("İlk belge: üç parsel; ortak sınırlar birer kez, bir parselin sınırı öbürüne numarasıyla",
                    KADASTRO, [("A1", None), ("A2", None), ("A3", None)],
                    note="101/7 ile 101/8 doğu-batı kenarını, 101/7 ile 101/9 kuzey-güney kenarını paylaşır; "
                         "101/9'un kuzey kenarı yay."),
        record_case("Delikli alan: deliğin sınırları kendi halkası", KADASTRO, [("A4", None)]),
        record_case("İki parçalı alan: ikinci parçanın halkası part 1", KADASTRO, [("A5", None)]),
        record_case("Kayıtlı sınırı paylaşan yeni parsel: ortak sınır numarasıyla, yeni belgede", brief,
                    [("A2", None)], reg=one),
        record_case("T birleşimi: kenarın bir kısmı ortak olan alan ayrı sınır alır", brief, [("A4", None)],
                    reg=two),
        record_case("Yeniden ölçme: 101/7 kapanır, aynı kimlik yeni ölçülen alandan; batı sınırı sürer, güneyi "
                    "kapanır", {"name": "2026/77 yenileme", "kind": "Yenileme"}, [("A10", None)], [1], reg=three),
        record_case("Aynı alan yeniden kaydedilir: belgedeki alan değişir, sınırlar aynı numarayla", brief,
                    [("A1", "200,05")], [1], reg=one),
        record_case("İfraz: 101/7 kapanır, iki yeni parsel; ortadaki sınır ikisinin, doğu ve batı sınırları sürer",
                    IFRAZ, [("A11", None), ("A12", None)], [1], reg=three),
        record_case("Tevhit: 101/7 ile 101/8 kapanır, bir parsel oluşur; aradaki sınır kapanır",
                    {"name": "2026/200 tevhit", "kind": "Değişiklik"}, [("A13", None)], [1, 2], reg=two),
        record_case("Yalnız kapatma: 101/8 kapanır, yalnız onun sınırları kapanır; ortak sınır 101/7'de sürer",
                    {"name": "2026/9 terkin", "kind": "Terkin"}, (), [2], reg=two),
        record_case("Belgedeki alan verilir: alanın Tapu alanı yerine", brief, [("A1", "199,95")]),
        record_case("Belge alan vermiyor: boş statedArea", brief, [("A1", "")]),
        record_case("İfrazdan sonra yeni belge önceki belgelerin numaralarından sürer", {"name": "2026/300 ifraz"},
                    [("A4", None)], reg=closed_one),
        record_case("Plan yazmaz", KADASTRO, [("A1", None), ("A2", None)], op="plan"),
        record_case("Doğrulama", KADASTRO, [("A1", None)], op="validate"),
        refused("Sürüm yazısı geçersiz", {"document": brief, "create": [{"uid": U("A1")}], "expectedRevision": "abc"},
                "invalid_revision", "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "Beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": {
            "document": brief, "create": [{"uid": U("A1")}], "expectedRevision": "7"}, "result": {
            "status": "conflict", "error": {"code": "revision_conflict", "message":
                                            "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                                            "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                                            "revision": "$current"}}, "expect": SAME}]},
        refused("Belgenin adı boş", {"document": {"name": "  "}, "create": [{"uid": U("A1")}]}, "invalid_input",
                "Belgenin adı boş olmaz; en çok 120 karakterdir, denetim karakteri taşımaz.", "document.name"),
        refused("Belgenin türü 120 karakteri aşıyor", {"document": {"name": "B", "kind": "T" * 121},
                                                      "create": [{"uid": U("A1")}]}, "invalid_input",
                "Belgenin türü en çok 120 karakterdir, denetim karakteri taşımaz.", "document.kind"),
        refused("Ne oluşturuyor ne kapatıyor", {"document": brief}, "invalid_input",
                "Belge ne parsel oluşturuyor ne kapatıyor: oluşturacağı alanları (create) ya da kapatacağı parselleri "
                "(close) verin.", "create"),
        refused("Belgenin adı çizimde var", {"document": {"name": "Kadastro 1985"}, "create": [{"uid": U("A2")}]},
                "document_name_taken", "Parsel kayıtlarında “Kadastro 1985” adlı bir belge var; belgenin adı çizimde "
                "tektir, başka bir ad verin.", "document.name", reg=one),
        refused("Aynı parsel iki kez kapatılıyor", {"document": brief, "close": [1, 1]}, "invalid_input",
                "1 numaralı parsel iki kez verildi.", "close[1]", reg=one),
        refused("Kayıtlarda olmayan parsel", {"document": brief, "close": [9]}, "unknown_parcel_record",
                "Parsel kayıtlarında 9 numaralı parsel yok.", "close[0]", reg=one),
        refused("Kapanmış parsel yeniden kapatılmaz", {"document": brief, "close": [1]}, "parcel_closed",
                "1 numaralı parsel (101/7) “2026/145 ifraz” belgesiyle kapatılmış; ancak güncel bir parsel kapatılır.",
                "close[0]", reg=closed_one),
        refused("Kimlik UUID değil", {"document": brief, "create": [{"uid": "x"}]}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "create[0].uid"),
        refused("Çizimde olmayan nesne", {"document": brief, "create": [{"uid": uid(99)}]}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "create[0].uid"),
        refused("Çizgi parsel değil", {"document": brief, "create": [{"uid": U("L9")}]}, "not_a_parcel",
                f"“{U('L9')}” bir alan değil; parsel yalnız bir alandan kaydedilir.", "create[0].uid"),
        refused("Ada'sı olmayan alan", {"document": brief, "create": [{"uid": U("A1")}, {"uid": U("A6")}]},
                "no_parcel_identity", f"“{U('A6')}” alanının Ada ya da Parsel'i yok; parselin kimliği İl, İlçe, "
                "Mahalle, Ada ve Parsel birliktedir, önce yazın.", "create[1].uid"),
        refused("Aynı alan iki kez", {"document": brief, "create": [{"uid": U("A1")}, {"uid": U("A1")}]},
                "invalid_input", f"“{U('A1')}” alanı iki kez verildi.", "create[1].uid"),
        refused("Kimliğin bir parçası 120 karakteri aşıyor", {"document": brief, "create": [{"uid": U("A14")}]},
                "invalid_input", f"“{U('A14')}” alanının kimliğinin bir parçası en çok 120 karakterdir, denetim "
                "karakteri taşımaz.", "create[0].uid"),
        refused("Aynı yerde iki ardışık köşe", {"document": brief, "create": [{"uid": U("A8")}]}, "invalid_input",
                f"“{U('A8')}” alanının aynı yerde iki ardışık köşesi var; sınır iki ayrı köşe arasındadır, önce "
                "yinelenen köşeyi silin.", "create[0].uid"),
        refused("Belgedeki alan düz ondalık değil", {"document": brief, "create": [{"uid": U("A1"),
                                                                                    "statedArea": "1.250,50"}]},
                "invalid_input", "Belgedeki alan “1.250,50” düz bir ondalık sayı değil: rakamlar, en çok bir nokta ya "
                "da virgül (118,5 ya da 723.525 gibi).", "create[0].statedArea"),
        refused("Alanın Tapu alanı düz ondalık değil", {"document": brief, "create": [{"uid": U("A7")}]},
                "invalid_input", "Alanın “Tapu alanı (m²)” özniteliği “1.250,50”: düz bir ondalık sayı değil; "
                "belgedeki alanı statedArea ile yazın (belge alan vermiyorsa boş verin).", "create[0].statedArea"),
        refused("Alan güncel bir parseli çiziyor, belge kapatmıyor", {"document": brief, "create": [{"uid": U("A1")}]},
                "parcel_recorded", f"“{U('A1')}” alanı 1 numaralı güncel parseli (101/7) çiziyor; belge önce o "
                "parseli kapatmalı (close).", "create[0].uid", reg=one),
        refused("Aynı kimlikte güncel parsel var", {"document": brief, "create": [{"uid": U("A10")}]},
                "parcel_identity_taken", "Ankara/Çankaya/Kızılay/101/7 kimlikli güncel bir parsel var (1 numaralı); belge "
                "önce onu kapatmalı (close).", "create[0].uid", reg=one),
        refused("Aynı kimlik bu belgede iki alanda", {"document": brief, "create": [{"uid": U("A1")},
                                                                                     {"uid": U("A10")}]},
                "parcel_identity_taken", "Ankara/Çankaya/Kızılay/101/7 kimliği bu belgede iki alanda var.", "create[1].uid"),
    ]


def moved(ents, name, old, new):
    """The drawing with one of an area's vertices moved: what was at `old` is at `new`."""
    ents = copy.deepcopy(ents)
    e = ents[SLOT[name] - 1]
    e["pts"] = [P(*new) if (p["x"] - X0, p["y"] - Y0) == old else p for p in e["pts"]]
    return ents


def records_case(name, reg, ents=None, history=False, op="execute", note=None):
    ents = ents if ents is not None else ENTS
    input_ = {"history": True} if history else {}
    out = listed(reg, ents, history)
    case = {"name": name}
    if note:
        case["note"] = note
    case["setup"] = setup(reg, ents)
    case["steps"] = [{"op": op, "input": input_, "result": {"status": "completed",
                                                            "output": None if op == "validate" else out,
                                                            "warnings": []},
                      "within": WITHIN, "expect": SAME}]
    return case


def records_cases():
    three = first()
    # 101/7's north-east corner moved in the drawing: the register keeps its record.
    shifted = moved(ENTS, "A1", (20, 10), (20.4, 10.2))
    # The move recorded by a new document: 101/7 closed and made again from the moved area.
    remeasured, _ = record(three, {"name": "2026/77 yenileme"}, [("A1", "203,10")], [1], ents=shifted)
    divided, _ = record(three, IFRAZ, [("A11", None), ("A12", None)], [1])
    joined, _ = record(record(None, KADASTRO, [("A1", None), ("A2", None)])[0],
                       {"name": "2026/200 tevhit"}, [("A13", None)], [1, 2])
    holed, _ = record(None, KADASTRO, [("A4", None), ("A5", None)])
    removed = copy.deepcopy(three)
    removed["parcels"][0]["uid"] = uid(99)
    line = copy.deepcopy(three)
    line["parcels"][1]["uid"] = U("L9")
    reversed_ = copy.deepcopy(ENTS)
    reversed_[SLOT["A2"] - 1]["pts"] = list(reversed(reversed_[SLOT["A2"] - 1]["pts"]))
    rotated = copy.deepcopy(ENTS)
    pts = rotated[SLOT["A1"] - 1]["pts"]
    rotated[SLOT["A1"] - 1]["pts"] = pts[2:] + pts[:2]
    renamed = copy.deepcopy(ENTS)
    renamed[SLOT["A1"] - 1]["attrs"]["Parsel"] = "70"
    nudged = moved(ENTS, "A2", (40, 10), (40.001, 10))
    inserted = copy.deepcopy(ENTS)
    inserted[SLOT["A2"] - 1]["pts"].insert(1, P(30, 0))
    terminated, _ = record(record(None, KADASTRO, [("A1", None), ("A2", None)])[0], {"name": "2026/9 terkin"}, (), [2])
    return [
        records_case("Kaydı olmayan çizim: boş listeler", None),
        records_case("İlk belgeden sonra: üç güncel parsel, kayıttaki gibi çizili; komşular ortak sınırlardan", three),
        records_case("Çizimde köşe taşınmış: 101/7'nin alanı değişir, belgedeki ve kayıttaki alanı değişmez; "
                     "kenarları kayıttaki yerinde değil", three, shifted,
                     note="Köşe (20, 10) → (20.4, 10.2): 101/7 değişti, 101/8 ve 101/9 kayıttaki gibi; 101/7'nin doğu "
                          "ve kuzey sınırları çizilmiyor."),
        records_case("Taşınan köşe yeni belgeyle kaydedilir: komşuluk ve kayıttaki alan değişir; eski kayıt kapanır",
                     remeasured, shifted),
        records_case("Aynısı, kapananlar da: eski 101/7'nin belgedeki alanı tarihçede", remeasured, shifted,
                     history=True),
        records_case("İfraz sonrası: 101/7 kapandı, 101/13 ve 101/14 güncel", divided),
        records_case("İfraz sonrası, kapananlar da: önceki ve sonraki parseller", divided, history=True),
        records_case("Tevhit sonrası, kapananlar da: iki önceki parsel, bir sonraki", joined, history=True),
        records_case("Yalnız kapatma sonrası, kapananlar da", terminated, history=True),
        records_case("Delikli ve iki parçalı alanlar: kayıttaki alan delik düşülmüş, iki parçanın toplamı", holed),
        records_case("Alanı silinmiş parsel: missing; sınırları çizilmiyor", removed),
        records_case("Alanı artık alan değil (çizgi): missing", line),
        records_case("Ters yönde çizilmiş alan: kayıttaki gibi (yön önemli değil)", three, reversed_),
        records_case("Başka köşeden başlayan alan: kayıttaki gibi", three, rotated),
        records_case("Kimliği çizimde değişmiş alan: kayıt kimliği korur, geometri aynı", three, renamed),
        records_case("1 mm kaymış köşe: changed", three, nudged),
        records_case("Kenarına köşe eklenmiş alan: changed; kenarı kayıttaki gibi çizilmiyor", three, inserted),
        records_case("Yaylı sınır: boyu yayın boyu", record(None, KADASTRO, [("A3", None)])[0]),
        records_case("Kapananlar istenmezse yalnız güncel sınırlar", divided, history=False),
        records_case("Plan aynı cevabı verir", three, op="plan"),
        records_case("Doğrulama", three, op="validate"),
        records_case("İfraz sonrası köşe taşınmış: sayılar", divided, moved(ENTS, "A12", (20, 10), (20.2, 10))),
    ]


def write(command, title, note, cases_, check):
    doc = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
           "note": note, "setup": setup(), "cases": cases_}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / f"{command}.json"
    if check:
        same = path.exists() and path.read_text("utf-8") == text
        print(f"{command} durumları güncel." if same else f"{path.relative_to(ROOT)} güncel değil.")
        return same
    path.write_text(text, "utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(cases_)} durum)")
    return True


def main():
    check = "--check" in sys.argv[1:]
    note = ("ADR 4220. Bağımsız başvuru scripts/fixtures/parcel_register_cases.py: belgeler, sınırların eşlenmesi ve "
            "kapanması, alanlar ve yay boyları betikte; KentOS kodu yok.")
    ok = write("cad.parcels.record", "Parsel kaydı", note, record_cases(), check)
    ok = write("cad.parcels.records", "Parsel kayıtları", note, records_cases(), check) and ok
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
