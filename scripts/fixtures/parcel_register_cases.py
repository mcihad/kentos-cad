#!/usr/bin/env python3
"""The shared cases of `cad.parcels.record`, `cad.parcels.records`, `cad.parcels.approve` and `cad.parcels.withdraw`
(Parsel kayıtları; plan/map-cad.md MAP-08, docs/adr/4220, 4620 and 5020), without KentOS code.

    python3 scripts/fixtures/parcel_register_cases.py           # writes the four files
    python3 scripts/fixtures/parcel_register_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.parcels.record.json, cad.parcels.records.json, cad.parcels.approve.json and
cad.parcels.withdraw.json. The rules, by hand (a document creates parcels and closes earlier ones, which stay as history;
a boundary closes once no current parcel uses it):

- a document (its name unique in the drawing; kind, date, note as written) makes parcels from areas and closes earlier
  ones. A parcel's identity is its area's İl/İlçe/Mahalle/Ada/Parsel; its stated area the text given, else the area's
  `Tapu alanı (m²)`, a plain decimal kept as written; its rings, part by part, each outline before its holes, are
  chains of boundaries;
- an edge is a boundary from its lesser corner (by east, then north) to the other, its bulge turned with it. An edge
  with the same two corners (as stored) and a bulge within 1e-9 of a boundary current before the document, or of one
  the document made already, is that boundary, negative in the ring where run the other way; else a new boundary,
  numbered on;
- ADR 4620: a straight boundary current before the document, used by a parcel current before it that the document
  does not close, inside which (within 1e-6 m of its line, farther than 1e-6 m from its ends) corners of the document's
  new parcels lie, and whose pieces between them (from its lesser corner on) are all current straight boundaries, is
  split: each such parcel's rings take the pieces, in order, each signed by its own direction (backwards and negated
  where the ring ran the boundary the other way);
- ADR 5020, an arc: the same for an arc boundary, the corners within 1e-6 m of its circle and farther than 1e-6 m along
  it from its ends, ordered round it from its lesser corner; a piece is a current arc boundary between its two corners
  whose arc's middle lies on the circle (1e-6 m), on the side of the piece (nearer than the radius to the circle's
  point midway between the two corners);
- ADR 5020, lengths: a document states lengths for its areas' edges (ring, edge, a plain decimal above 0 as written; or
  the area's `Kenar ölçüleri (m)`, the outline's edges in order separated by `;`, an empty or `-` one unstated); each
  edge's boundary gets the length once per document (two edges giving one boundary two values are refused), kept in
  the order the parcels and their lengths come;
- ADR 5020, drafts: with `draft` the document is kept as a draft (numbered apart; its parcels' area, identity, stated
  area, rings as drawn then and lengths; what it closes), nothing current changes, and the answer is what recording it
  would do today with `document` the draft's number and `draft`; approving writes the approval (number, date, who, as
  written) on a document, or writes a draft as the next document with its approval exactly as recording would, once what
  it closes is still current; withdrawing removes a draft;
- the closed parcels lose their area; a boundary current before the document that no current parcel uses after it is
  closed by the document;
- reading: a document's approval as written; a draft's identities, closings and number of lengths; each boundary's
  stated lengths with the boundary's length less the stated one and BÖHHBÜY m. 48(1)'s limit 0.05 + 0.001·S m (S the
  stated length), within when the difference's size is no more than the limit; `overLimit` the current boundaries with
  one past it; a parcel's recorded area is its rings' (shoelace plus each arc's circular segment, holes less), its area now
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
LENGTHS = "Kenar ölçüleri (m)"


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


def made_from_area(name, stated, ents):
    """A parcel the document makes from an area: (identity, stated area, uid, rings, lengths as (ring, edge, text))."""
    e = ents[SLOT[name] - 1]
    attrs = e["attrs"]
    ident = {}
    for key, attr in (("il", "İl"), ("ilce", "İlçe"), ("mahalle", "Mahalle"), ("ada", "Ada"), ("parsel", "Parsel")):
        if attrs.get(attr, "").strip():
            ident[key] = attrs[attr].strip()
    text = stated if stated is not None else attrs.get(FIELD, "")
    return {"ident": ident, "stated": text, "uid": U(name), "rings": rings_of(e)}


def lengths_from(given, attrs):
    """The lengths an area states: given as (ring, edge, text), else its `Kenar ölçüleri (m)` for the outline."""
    if given is not None:
        return [(r, k, t) for r, k, t in given]
    out = []
    for k, t in enumerate(x.strip() for x in attrs.get(LENGTHS, "").split(";")):
        if t and t not in ("-", "—"):
            out.append((0, k, t))
    return out


def apply(reg, number, doc_row, made, close):
    """`reg` after a document numbered `number` (its row `doc_row`) makes `made` and closes `close`; the answer."""
    reg["documents"].append(doc_row)
    before = [b for b in reg["boundaries"] if "closedBy" not in b]
    earlier = len(reg["boundaries"])
    fresh = []
    shared = set()
    next_b = max((b["number"] for b in reg["boundaries"]), default=0) + 1
    next_p = max((p["number"] for p in reg["parcels"]), default=0) + 1
    created = []
    stated = []
    for m in made:
        rings = []
        ring_edges = []
        for part, hole, places, bulges in m["rings"]:
            edges = []
            for k in range(len(places)):
                p, q, bulge = places[k], places[(k + 1) % len(places)], bulges[k]
                frm, to, bulge, back = (p, q, bulge, False) if p <= q else (q, p, -bulge, True)
                hit = next((b for b in before + fresh if (b["from"]["x"], b["from"]["y"]) == frm
                            and (b["to"]["x"], b["to"]["y"]) == to and abs(b.get("bulge", 0.0) - bulge) <= TOLERANCE),
                           None)
                if hit is None:
                    hit = {"number": next_b, "from": {"x": frm[0], "y": frm[1]}, "to": {"x": to[0], "y": to[1]},
                           "createdBy": number}
                    if bulge != 0:
                        hit["bulge"] = bulge
                    next_b += 1
                    fresh.append(hit)
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
            ring_edges.append(edges)
        for r, k, t in m["lengths"]:
            b = abs(ring_edges[r][k])
            other = next((x for x in stated if x[0] == b), None)
            if other is None:
                stated.append((b, t))
            elif other[1] != t:
                raise ValueError("two lengths for one boundary")
        parcel = {"number": next_p, **m["ident"], "createdBy": number}
        if m["stated"]:
            parcel["statedArea"] = m["stated"]
        parcel["uid"] = m["uid"]
        parcel["rings"] = rings
        created.append(next_p)
        next_p += 1
        reg["parcels"].append(parcel)
    if stated:
        reg.setdefault("lengths", []).extend({"boundary": b, "document": number, "length": t} for b, t in stated)
    split, neighbours = split_neighbours(reg, earlier, close, created)
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
           "boundariesClosed": gone}
    if split:
        out["boundariesSplit"] = split
    if neighbours:
        out["neighbours"] = neighbours
    if stated:
        out["lengths"] = len(stated)
    out["revision"] = "$current"
    return out


def doc_row(number, document):
    d = {"number": number, "name": document["name"]}
    d.update({k: document[k] for k in ("kind", "date", "note") if document.get(k)})
    return d


def record(reg, document, create=(), close=(), ents=None, draft=False):
    """The register after a document: `create` (object name, stated area or None[, lengths as (ring, edge, text) or
    None for the area's attribute]), `close` (parcel numbers). With `draft` the register keeps the draft and the answer
    is what recording would do. The answer as `cad.parcels.record` gives it."""
    ents = ents if ents is not None else ENTS
    reg = copy.deepcopy(reg) if reg else empty()
    number = max((d["number"] for d in reg["documents"]), default=0) + 1
    made = []
    for item in create:
        name, stated = item[0], item[1]
        given = item[2] if len(item) > 2 else None
        m = made_from_area(name, stated, ents)
        m["lengths"] = lengths_from(given, ents[SLOT[name] - 1]["attrs"])
        made.append(m)
    if not draft:
        out = apply(reg, number, doc_row(number, document), made, close)
        return reg, out
    preview = copy.deepcopy(reg)
    out = apply(preview, number, doc_row(number, document), made, close)
    drafts = reg.setdefault("drafts", [])
    row = {"number": max((d["number"] for d in drafts), default=0) + 1, "name": document["name"]}
    row.update({k: document[k] for k in ("kind", "date", "note") if document.get(k)})
    if made:
        row["create"] = [draft_parcel(m) for m in made]
    if close:
        row["close"] = list(close)
    drafts.append(row)
    out["document"] = row["number"]
    del out["revision"]
    out["draft"] = True
    out["revision"] = "$current"
    return reg, out


def draft_parcel(m):
    """A parcel as a draft keeps it."""
    q = {"uid": m["uid"], **m["ident"]}
    if m["stated"]:
        q["statedArea"] = m["stated"]
    rings = []
    for part, hole, places, bulges in m["rings"]:
        r = {}
        if part:
            r["part"] = part
        if hole:
            r["hole"] = True
        r["pts"] = [{"x": x, "y": y} for x, y in places]
        if any(bulges):
            r["bulges"] = bulges
        rings.append(r)
    q["rings"] = rings
    if m["lengths"]:
        q["lengths"] = [dict(({"ring": r} if r else {}), edge=k, length=t) for r, k, t in m["lengths"]]
    return q


def approve(reg, approval, draft=None, document=None):
    """The register after an approval and the answer as `cad.parcels.approve` gives it."""
    reg = copy.deepcopy(reg)
    if document is not None:
        d = next(x for x in reg["documents"] if x["number"] == document)
        d["approval"] = dict(approval)
        return reg, {"document": document, "revision": "$current"}
    at = next(i for i, x in enumerate(reg["drafts"]) if x["number"] == draft)
    row = reg["drafts"].pop(at)
    if not reg["drafts"]:
        del reg["drafts"]
    made = []
    for q in row.get("create", []):
        ident = {k: q[k] for k in ("il", "ilce", "mahalle", "ada", "parsel") if q.get(k)}
        rings = [(r.get("part", 0), r.get("hole", False), [(t["x"], t["y"]) for t in r["pts"]],
                  r.get("bulges") or [0.0] * len(r["pts"])) for r in q["rings"]]
        made.append({"ident": ident, "stated": q.get("statedArea", ""), "uid": q["uid"], "rings": rings,
                     "lengths": [(x.get("ring", 0), x["edge"], x["length"]) for x in q.get("lengths", [])]})
    number = max((d["number"] for d in reg["documents"]), default=0) + 1
    d = doc_row(number, row)
    d["approval"] = dict(approval)
    out = apply(reg, number, d, made, row.get("close", []))
    return reg, {"document": number, "recorded": out, "revision": "$current"}


INSIDE = 1e-6


def circle(b):
    """An arc boundary's centre, radius, start angle and signed sweep: the centre reached from `from` at the chord's
    angle turned by ±(π/2 − |θ|/2), θ = 4·atan(bulge)."""
    a, z = (b["from"]["x"], b["from"]["y"]), (b["to"]["x"], b["to"]["y"])
    theta = 4 * math.atan(b["bulge"])
    c = math.hypot(z[0] - a[0], z[1] - a[1])
    r = c / (2 * math.sin(abs(theta) / 2))
    towards = math.atan2(z[1] - a[1], z[0] - a[0]) + math.copysign(math.pi / 2 - abs(theta) / 2, theta)
    centre = (a[0] + r * math.cos(towards), a[1] + r * math.sin(towards))
    return centre, r, math.atan2(a[1] - centre[1], a[0] - centre[0]), theta


def round_of(centre, start, theta, q):
    return (math.copysign(1, theta) * (math.atan2(q[1] - centre[1], q[0] - centre[0]) - start)) % (2 * math.pi)


def split_neighbours(reg, earlier, close, created):
    """ADR 4620's and 5020's split, on `reg` in place: how many boundaries were split and the parcels whose rings took
    pieces."""
    by_number = {b["number"]: b for b in reg["boundaries"]}
    corners = []
    for p in reg["parcels"]:
        if p["number"] in created:
            for r in p["rings"]:
                for e in r["edges"]:
                    b = by_number[abs(e)]
                    for c in ((b["from"]["x"], b["from"]["y"]), (b["to"]["x"], b["to"]["y"])):
                        if c not in corners:
                            corners.append(c)
    holders_of = {}
    for p in reg["parcels"]:
        if "closedBy" in p or p["number"] in close or p["number"] in created:
            continue
        for r in p["rings"]:
            for e in r["edges"]:
                holders_of.setdefault(abs(e), [])
                if p not in holders_of[abs(e)]:
                    holders_of[abs(e)].append(p)
    current = [b for b in reg["boundaries"] if "closedBy" not in b]
    splits = []
    for b in reg["boundaries"][:earlier]:
        if "closedBy" in b or b["number"] not in holders_of:
            continue
        a, z = (b["from"]["x"], b["from"]["y"]), (b["to"]["x"], b["to"]["y"])
        inside = []
        if not b.get("bulge"):
            dx, dy = z[0] - a[0], z[1] - a[1]
            length_ = math.hypot(dx, dy)
            for c in corners:
                along = ((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / length_
                off = abs((c[0] - a[0]) * dy - (c[1] - a[1]) * dx) / length_
                if off <= INSIDE and along > INSIDE and length_ - along > INSIDE:
                    inside.append((along, c))
        else:
            centre, r, start, theta = circle(b)
            for c in corners:
                off = abs(math.hypot(c[0] - centre[0], c[1] - centre[1]) - r)
                t = round_of(centre, start, theta, c)
                if off <= INSIDE and t * r > INSIDE and (abs(theta) - t) * r > INSIDE:
                    inside.append((t, c))
        if not inside:
            continue
        inside.sort()
        chain = [(0.0, a)] + inside + [(abs(theta) if b.get("bulge") else math.inf, z)]
        pieces = []
        for (t0, p), (t1, q) in zip(chain, chain[1:]):
            frm, to, back = (p, q, False) if p <= q else (q, p, True)
            def fits(x):
                if (x["from"]["x"], x["from"]["y"]) != frm or (x["to"]["x"], x["to"]["y"]) != to:
                    return False
                if not b.get("bulge"):
                    return not x.get("bulge")
                if not x.get("bulge"):
                    return False
                # The piece's arc middle: its own circle's point midway round it.
                cx, rx, sx, tx = circle(x)
                mid = (cx[0] + rx * math.cos(sx + tx / 2), cx[1] + rx * math.sin(sx + tx / 2))
                want_angle = start + math.copysign(1, theta) * (t0 + t1) / 2
                want = (centre[0] + r * math.cos(want_angle), centre[1] + r * math.sin(want_angle))
                return (abs(math.hypot(mid[0] - centre[0], mid[1] - centre[1]) - r) <= INSIDE
                        and math.hypot(mid[0] - want[0], mid[1] - want[1]) < r)
            hit = next((x for x in current if fits(x)), None)
            if hit is None:
                pieces = None
                break
            pieces.append(-hit["number"] if back else hit["number"])
        if pieces is not None:
            splits.append((b["number"], pieces, holders_of[b["number"]]))
    neighbours = set()
    for old, pieces, holders in splits:
        for p in holders:
            for r in p["rings"]:
                edges = []
                for e in r["edges"]:
                    if e == old:
                        edges += pieces
                    elif e == -old:
                        edges += [-x for x in reversed(pieces)]
                    else:
                        edges.append(e)
                r["edges"] = edges
            neighbours.add(p["number"])
    return len(splits), sorted(neighbours)


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
        rows_ = []
        for x in reg.get("lengths", []):
            if x["boundary"] != b["number"]:
                continue
            given = float(x["length"].replace(",", "."))
            difference = length(b) - given
            limit = 0.05 + 0.001 * given
            rows_.append({"document": x["document"], "length": x["length"], "difference": difference, "limit": limit,
                          "within": abs(difference) <= limit})
        if rows_:
            line["lengths"] = rows_
        lines.append(line)
    documents = []
    for d in reg["documents"]:
        row = dict(d)
        row["created"] = [p["number"] for p in reg["parcels"] if p["createdBy"] == d["number"]]
        row["closed"] = [p["number"] for p in reg["parcels"] if p.get("closedBy") == d["number"]]
        documents.append(row)
    drafts = []
    for d in reg.get("drafts", []):
        row = {"number": d["number"], "name": d["name"]}
        row.update({k: d[k] for k in ("kind", "date") if d.get(k)})
        row["create"] = [f"{q['ada']}/{q['parsel']}" for q in d.get("create", [])]
        row["close"] = list(d.get("close", []))
        row["lengths"] = sum(len(q.get("lengths", [])) for q in d.get("create", []))
        drafts.append(row)
    over = sum(1 for b in lines if "closedBy" not in b and any(not x["within"] for x in b.get("lengths", [])))
    extra = {}
    if drafts:
        extra["drafts"] = drafts
    if over:
        extra["overLimit"] = over
    return {**extra, 
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
    for item in create:
        name, stated = item[0], item[1]
        entry = {"uid": U(name)}
        if stated is not None:
            entry["statedArea"] = stated
        if len(item) > 2 and item[2] is not None:
            entry["lengths"] = [dict(({"ring": r} if r else {}), edge=k, length=t) for r, k, t in item[2]]
        out.append(entry)
    return out


def record_case(name, document, create=(), close=(), reg=None, op="execute", note=None, ents=None, draft=False):
    after, out = record(reg, document, create, close, ents=ents, draft=draft)
    input_ = {"document": doc_input(document)}
    if create:
        input_["create"] = wanted(create)
    if close:
        input_["close"] = list(close)
    if draft:
        input_["draft"] = True
    case = {"name": name}
    if note:
        case["note"] = note
    if reg is not None or ents is not None:
        case["setup"] = setup(reg, ents)
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


def middle():
    """The number 2026/145 ifraz gives the boundary between 101/13 and 101/14."""
    after, _ = record(first(), IFRAZ, [("A11", None), ("A12", None)], [1])
    return next(b["number"] for b in after["boundaries"]
                if (b["from"]["x"], b["from"]["y"], b["to"]["x"], b["to"]["y"]) == (X0 + 10, Y0, X0 + 10, Y0 + 10))


def refused_in(name, input_, code, message, path_, ents, reg=None):
    """`refused` over another drawing."""
    case = refused(name, input_, code, message, path_, reg=reg)
    case["setup"] = setup(reg, ents)
    return case


def refused(name, input_, code, message, path_, reg=None):
    case = {"name": name}
    if reg is not None:
        case["setup"] = setup(reg)
    case["steps"] = [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]
    return case


# ADR 4620's own areas, apart from the shared drawing: 101/9 divided at x = 10 (straight north edges), and an area whose
# south edge lies inside 101/8's north boundary.
SPLIT_ENTS = copy.deepcopy(ENTS) + [
    dict(area(who(parsel="16", tapu="100"), [(0, 10), (10, 10), (10, 20), (0, 20)]), id=len(ENTS) + 1),
    dict(area(who(parsel="17", tapu="100"), [(10, 10), (20, 10), (20, 20), (10, 20)]), id=len(ENTS) + 2),
    dict(area(who(parsel="18", tapu="50"), [(25, 10), (35, 10), (35, 15), (25, 15)]), id=len(ENTS) + 3),
]
SLOT.update({"A15": len(ENTS) + 1, "A16": len(ENTS) + 2, "A17": len(ENTS) + 3})


# ADR 5020's own areas: 101/19 north of 101/9 shares its arc; 101/20 and 101/21 divide it at the arc's middle (10, 22),
# each taking half the arc; 101/22 and 101/23 the same with the corner 2 µm off the circle; 104/1–3 state their edge
# lengths in their attribute (well, with a 0, with five lengths for four edges).
HALF = math.tan(math.atan(0.2) / 2)
MORE_ENTS = copy.deepcopy(SPLIT_ENTS) + [
    dict(area(who(parsel="19", tapu="219,7"), [(0, 20), (20, 20), (20, 30), (0, 30)], bulges=[-0.2, 0, 0, 0]),
         id=len(SPLIT_ENTS) + 1),
    dict(area(who(parsel="20", tapu="109,9"), [(0, 20), (10, 22), (10, 30), (0, 30)], bulges=[-HALF, 0, 0, 0]),
         id=len(SPLIT_ENTS) + 2),
    dict(area(who(parsel="21", tapu="109,8"), [(10, 22), (20, 20), (20, 30), (10, 30)], bulges=[-HALF, 0, 0, 0]),
         id=len(SPLIT_ENTS) + 3),
    dict(area(who(parsel="22"), [(0, 20), (10, 22.000002), (10, 30), (0, 30)], bulges=[-HALF, 0, 0, 0]),
         id=len(SPLIT_ENTS) + 4),
    dict(area(who(parsel="23"), [(10, 22.000002), (20, 20), (20, 30), (10, 30)], bulges=[-HALF, 0, 0, 0]),
         id=len(SPLIT_ENTS) + 5),
    dict(area(who(ada="104", parsel="1", **{LENGTHS: "20,00; 10,01; ; 10,00"}), [(160, 0), (180, 0), (180, 10), (160, 10)]),
         id=len(SPLIT_ENTS) + 6),
    dict(area(who(ada="104", parsel="2", **{LENGTHS: "20,00; 0; 20,00"}), [(190, 0), (210, 0), (210, 10), (190, 10)]),
         id=len(SPLIT_ENTS) + 7),
    dict(area(who(ada="104", parsel="3", **{LENGTHS: "1;2;3;4;5"}), [(220, 0), (240, 0), (240, 10), (220, 10)]),
         id=len(SPLIT_ENTS) + 8),
    # Another area that says it is 101/13.
    dict(area(who(parsel="13"), [(250, 0), (260, 0), (260, 10), (250, 10)]), id=len(SPLIT_ENTS) + 9),
]
SLOT.update({f"A{18 + k}": len(SPLIT_ENTS) + 1 + k for k in range(9)})
APPROVAL = {"reference": "2026/145-K", "date": "2026-03-20", "by": "Kadastro Müdürlüğü"}


def arced():
    """Kadastro 1985 with 101/19 north of 101/9 (parcels 1–4), the arc between them one boundary."""
    return record(None, KADASTRO, [("A1", None), ("A2", None), ("A3", None), ("A18", None)], ents=MORE_ENTS)[0]


def drafted():
    """Kadastro 1985, then 2026/145 ifraz kept as a draft with the middle boundary's length."""
    return record(first(), IFRAZ, [("A11", None, [(0, 1, "10,00")]), ("A12", None)], [1], draft=True)[0]


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
        record_case("İfraz: 101/7 kapanır, iki yeni parsel; ortadaki sınır ikisinin, doğu ve batı sınırları sürer; 101/9'la "
                    "ortak kuzey sınırı yeni köşede bölünür, 101/9 parçaları alır",
                    IFRAZ, [("A11", None), ("A12", None)], [1], reg=three,
                    note="ADR 4620: (10, 10) köşesi 101/9'un da kullandığı (0, 10)–(20, 10) sınırının içinde; parçalar "
                         "101/13 ile 101/14'ün kuzey sınırları."),
        record_case("İfraz, komşu sınırı ters yürüyor: 101/9 kapanır, iki yeni parsel; 101/7 parçaları ters sırayla, eksi "
                    "işaretle alır", IFRAZ, [("A15", None), ("A16", None)], [3], reg=three, ents=SPLIT_ENTS),
        record_case("Yeni alanın köşeleri komşunun sınırının içinde ama parçalar belgenin sınırı değil: bölünmez",
                    {"name": "2026/310 tescil"}, [("A17", None)], reg=three, ents=SPLIT_ENTS),
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
                "document_name_taken", "Parsel kayıtlarında “Kadastro 1985” adlı bir belge ya da taslak var; belgenin "
                "adı çizimde tektir, başka bir ad verin.", "document.name", reg=one),
        refused("Belgenin adı bir taslağın", {"document": {"name": "2026/145 ifraz"}, "create": [{"uid": U("A4")}]},
                "document_name_taken", "Parsel kayıtlarında “2026/145 ifraz” adlı bir belge ya da taslak var; belgenin "
                "adı çizimde tektir, başka bir ad verin.", "document.name", reg=drafted()),
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
        # ADR 5020.
        record_case("Yaylı komşu sınırı yeni köşede bölünür: 101/19 kapanır, 101/20 ile 101/21 yayın yarılarını alır; "
                    "101/9 parçaları alır", {"name": "2026/150 ifraz"}, [("A19", None), ("A20", None)], [4],
                    reg=arced(), ents=MORE_ENTS,
                    note="ADR 5020: (10, 22) köşesi 101/9 ile 101/19'un ortak yayının ortası; yarı yayların kabarıklığı "
                         "tan(atan(0,2)/2)."),
        record_case("Köşe yayın 2 µm dışında: yay bölünmez, yeni yaylar ayrı sınır", {"name": "2026/151 ifraz"},
                    [("A21", None), ("A22", None)], [4], reg=arced(), ents=MORE_ENTS),
        record_case("Kenar ölçüleri verilir: belgenindir, sınıra bir kez", brief,
                    [("A1", None, [(0, 0, "20,00"), (0, 1, "10.00")])]),
        record_case("Kenar ölçüleri alanın Kenar ölçüleri özniteliğinden; boş olan yazılmaz", brief, [("A23", None)],
                    ents=MORE_ENTS),
        record_case("İfrazda iki parsel ortak sınıra aynı ölçüyü yazar: bir kez", IFRAZ,
                    [("A11", None, [(0, 1, "10,00")]), ("A12", None, [(0, 3, "10,00")])], [1], reg=first()),
        record_case("Taslak: güncel hiçbir şey değişmez; cevap onayın bugün yazacağı, numara taslağın", IFRAZ,
                    [("A11", None, [(0, 1, "10,00")]), ("A12", None)], [1], reg=first(), draft=True),
        record_case("Taslak, kenar ölçüsüz: pencereden yazılan", IFRAZ, [("A11", None), ("A12", None)], [1],
                    reg=first(), draft=True),
        record_case("Taslağın planı yazmaz", IFRAZ, [("A11", None), ("A12", None)], [1], reg=first(), draft=True,
                    op="plan"),
        refused("Kenar ölçüsünün halkası yok", {"document": brief, "create": [
            {"uid": U("A1"), "lengths": [{"ring": 1, "edge": 0, "length": "5"}]}]}, "invalid_input",
                "Alanın 1 numaralı halkası yok; alanın 1 halkası var, 0'dan numaralıdır (0 dış halka).",
                "create[0].lengths[0]"),
        refused("Kenar ölçüsünün kenarı yok", {"document": brief, "create": [
            {"uid": U("A1"), "lengths": [{"edge": 4, "length": "5"}]}]}, "invalid_input",
                "0 numaralı halkanın 4 numaralı kenarı yok; halkanın 4 kenarı var, 0'dan numaralıdır.",
                "create[0].lengths[0]"),
        refused("Aynı kenarın ölçüsü iki kez", {"document": brief, "create": [
            {"uid": U("A1"), "lengths": [{"edge": 0, "length": "5"}, {"edge": 0, "length": "6"}]}]}, "invalid_input",
                "0 numaralı halkanın 0 numaralı kenarının ölçüsü iki kez verildi.", "create[0].lengths[1]"),
        refused("Kenar ölçüsü sıfır", {"document": brief, "create": [
            {"uid": U("A1"), "lengths": [{"edge": 0, "length": "0,00"}]}]}, "invalid_input",
                "Kenar ölçüsü “0,00” sıfırdan büyük düz bir ondalık sayı değil: rakamlar, en çok bir nokta ya da virgül "
                "(20,00 gibi).", "create[0].lengths[0]"),
        refused_in("Kenar ölçüleri özniteliğinde sıfır", {"document": brief, "create": [{"uid": U("A24")}]},
                   "invalid_input", "Alanın “Kenar ölçüleri (m)” özniteliğinin 2. ölçüsü “0” sıfırdan büyük düz bir "
                   "ondalık sayı değil; ölçüleri 20,00 gibi yazın, yazılmayanı boş bırakın.", "create[0].lengths",
                   MORE_ENTS),
        refused_in("Kenar ölçüleri özniteliği kenarlardan çok", {"document": brief, "create": [{"uid": U("A25")}]},
                   "invalid_input", "Alanın “Kenar ölçüleri (m)” özniteliği 5 ölçü yazıyor; dış halkanın 4 kenarı var. "
                   "Ölçüleri dış halkanın kenarları sırasıyla ; ile ayırarak yazın.", "create[0].lengths", MORE_ENTS),
        refused("İki parsel ortak sınıra iki ayrı ölçü yazıyor", {"document": IFRAZ, "create": [
            {"uid": U("A11"), "lengths": [{"edge": 1, "length": "10,00"}]},
            {"uid": U("A12"), "lengths": [{"edge": 3, "length": "10,02"}]}], "close": [1]}, "invalid_input",
                f"İki kenar aynı sınıra ({middle()}) iki ayrı ölçü yazıyor: “10,00” ve “10,02”; bir belge bir sınıra bir "
                "ölçü yazar.", "create[1].lengths[0]", reg=first()),
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
    # ADR 4620: 101/9's area given the new corner (10, 10) on its south edge.
    cornered = copy.deepcopy(ENTS)
    cornered[SLOT["A3"] - 1]["pts"].insert(1, P(10, 10))
    cornered[SLOT["A3"] - 1]["bulges"] = [0, 0, 0, 0.2, 0]
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
        records_case("İfraz sonrası: 101/7 kapandı, 101/13 ve 101/14 güncel; 101/9'un bölünen sınırının yeni köşesi "
                     "alanında yok: changed", divided),
        records_case("İfraz sonrası 101/9'un alanına yeni köşe eklenmiş: kayıttaki gibi, komşuları 101/13 ve 101/14",
                     divided, cornered),
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
        # ADR 5020.
        records_case("Belgenin kenar ölçüleri: kayıttaki boyla farkı ve cephe sınırı; güney kenarı sınırı aşar",
                     record(None, KADASTRO, [("A1", None, [(0, 0, "20,08"), (0, 1, "10,01")])])[0],
                     note="Güney kenarı 20 m, belgede 20,08: fark −0,08, sınır 0,05 + 0,001·20,08 = 0,07008 m; doğu "
                          "kenarı 10 m, belgede 10,01: fark −0,01, sınır 0,06001 m."),
        records_case("Taslak ve onay listelenir; taslak güncel parselleri değiştirmez",
                     approve(drafted(), APPROVAL, document=1)[0], history=True),
        records_case("Taslak onaylanınca: yeni belge onayıyla, ortak sınırın ölçüsü", approve(drafted(), APPROVAL, draft=1)[0],
                     history=True),
    ]


def step_case(name, command_reg, input_, out, after, step, note=None, op="execute", ents=None):
    """A writing command's case: execute and undo, or plan/validate writing nothing."""
    case = {"name": name}
    if note:
        case["note"] = note
    case["setup"] = setup(command_reg, ents)
    if op != "execute":
        result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
        case["steps"] = [{"op": op, "input": input_, "result": result, "expect": SAME}]
        return case
    case["steps"] = [
        {"op": "execute", "input": input_, "result": {"status": "completed", "output": out, "warnings": []},
         "expect": {"revision": "changed", "canUndo": True, "settings": {"parcelRegister": after}}},
        {"op": "undo", "returns": step, "expect": {"settings": {"parcelRegister": command_reg}}},
    ]
    return case


def conflict(name, input_, reg):
    """A command prepared against an older revision: nothing written."""
    return {"name": name, "setup": setup(reg), "steps": [{"op": "execute", "input": dict(input_, expectedRevision="7"),
            "result": {"status": "conflict", "error": {
                "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                "revision": "$current"}}, "expect": SAME}]}


def plain_draft():
    """Kadastro 1985, then 2026/145 ifraz kept as a draft without lengths (as the window writes it)."""
    return record(first(), IFRAZ, [("A11", None), ("A12", None)], [1], draft=True)[0]


def two_drafts():
    """Kadastro 1985 with two drafts: 2026/145 ifraz (1) and 2026/9 terkin of 101/8 (2)."""
    return record(drafted(), {"name": "2026/9 terkin", "kind": "Terkin"}, (), [2], draft=True)[0]


def approve_cases():
    three = first()
    draft = drafted()
    approved_one, _ = approve(three, {"reference": "1985/12"}, document=1)
    after_draft, out_draft = approve(draft, APPROVAL, draft=1)
    after_doc, out_doc = approve(three, {"reference": "1985/12"}, document=1)
    terminated, _ = record(draft, {"name": "2026/9 terkin"}, (), [1])
    taken, _ = record(draft, {"name": "2026/12 tescil"}, [("A11", None)])
    named, _ = record(draft, {"name": "2026/12 tescil"}, [("A26", None)], ents=MORE_ENTS)
    closing_only, _ = record(first(), {"name": "2026/9 terkin", "kind": "Terkin"}, (), [2], draft=True)
    arc_draft, _ = record(arced(), {"name": "2026/150 ifraz"}, [("A19", None), ("A20", None)], [4], ents=MORE_ENTS,
                          draft=True)
    attr_draft, _ = record(None, KADASTRO, [("A23", None)], ents=MORE_ENTS, draft=True)
    both = two_drafts()
    later, _ = record(three, {"name": "2026/12 tescil"}, [("A4", None)])
    step = "Belge onayı"
    cases = []
    for name_, reg_, input_ in (
            ("Yalnız kapatan taslak onaylanır: 101/8 kapanır, yalnız onun sınırları", closing_only,
             {"draft": 1, "approval": {"reference": "2026/9"}}),
            ("Yaylı taslak onaylanır: komşunun yayı onayda bölünür", arc_draft,
             {"draft": 1, "approval": {"reference": "2026/150", "by": "Kadastro Müdürlüğü"}}),
            ("Alanın özniteliğinden ölçülü taslak onaylanır: ölçüler belgeye", attr_draft,
             {"draft": 1, "approval": {"reference": "1985/1"}}),
            ("İki taslaktan ikincisi onaylanır: birincisi kalır", both, {"draft": 2, "approval": {"reference": "2026/9"}}),
            ("İki taslaktan birincisi onaylanır: ikincisi kalır", both,
             {"draft": 1, "approval": {"reference": "2026/145-K", "date": "2026-03-20"}}),
            ("Sonraki belgenin onayı yazılır", later, {"document": 2, "approval": {"reference": "2026/12"}}),
            ("Kenar ölçüsüz taslak onaylanır: pencereden", plain_draft(), {"draft": 1, "approval": APPROVAL})):
        a = input_["approval"]
        after_, out_ = approve(reg_, a, draft=input_.get("draft"), document=input_.get("document"))
        cases.append(step_case(name_, reg_, input_, out_, after_, step))
    return cases + [
        step_case("Taslak onaylanır: sıradaki numarayla belge, onayıyla; taslak kalkar", draft,
                  {"draft": 1, "approval": APPROVAL}, out_draft, after_draft, step,
                  note="Taslak 2026/145 ifraz'ı taslaktaki gibi yazar: 101/7 kapanır, 101/13 ve 101/14 oluşur; ortak "
                       "sınırın ölçüsü 10,00."),
        step_case("Kayıtlı belgenin onayı yazılır; belge değişmez", three, {"document": 1,
                                                                             "approval": {"reference": "1985/12"}},
                  out_doc, after_doc, step),
        step_case("Onayın planı yazmaz", draft, {"draft": 1, "approval": APPROVAL}, out_draft, after_draft, step,
                  op="plan"),
        step_case("Doğrulama", three, {"document": 1, "approval": {"reference": "1985/12"}}, None, None, step,
                  op="validate"),
        refused("Ne taslak ne belge", {"approval": APPROVAL}, "invalid_input",
                "Onayı yazılacak taslağın (draft) ya da belgenin (document) numarasını verin; ikisinden yalnız biri.",
                "draft", reg=draft),
        refused("Hem taslak hem belge", {"draft": 1, "document": 1, "approval": APPROVAL}, "invalid_input",
                "Onayı yazılacak taslağın (draft) ya da belgenin (document) numarasını verin; ikisinden yalnız biri.",
                "draft", reg=draft),
        refused("Onay sayısı boş", {"document": 1, "approval": {"reference": "  "}}, "invalid_input",
                "Onay sayısı boş olmaz; en çok 64 karakterdir, denetim karakteri taşımaz. Onayı veren yerin yazdığı "
                "sayıyı yazın.", "approval.reference", reg=three),
        refused("Onaylayan 120 karakteri aşıyor", {"document": 1, "approval": {"reference": "1", "by": "K" * 121}},
                "invalid_input", "Onaylayan en çok 120 karakterdir, denetim karakteri taşımaz.", "approval.by",
                reg=three),
        refused("Olmayan taslak", {"draft": 9, "approval": APPROVAL}, "unknown_parcel_draft",
                "Parsel kayıtlarında 9 numaralı belge taslağı yok.", "draft", reg=draft),
        refused("Olmayan belge", {"document": 9, "approval": APPROVAL}, "unknown_parcel_document",
                "Parsel kayıtlarında 9 numaralı belge yok.", "document", reg=three),
        refused("Onayı yazılmış belge", {"document": 1, "approval": APPROVAL}, "document_approved",
                "“Kadastro 1985” belgesinin onayı yazılmış (1985/12); onay bir kez yazılır, değişirse yeni bir belgeyle "
                "kaydedin.", "document", reg=approved_one),
        refused("Taslağın kapatacağı parsel o arada kapanmış", {"draft": 1, "approval": APPROVAL}, "parcel_closed",
                "Taslağın kapatacağı 1 numaralı parsel (101/7) o arada “2026/9 terkin” belgesiyle kapatılmış; taslak "
                "bugün yazılamaz. Taslağı geri çekip bugünkü kayıtlara göre yeniden hazırlayın.", "draft",
                reg=terminated),
        refused("Taslağın alanı o arada başka bir belgeyle kaydedilmiş", {"draft": 1, "approval": APPROVAL},
                "parcel_recorded", f"“{U('A11')}” alanı 4 numaralı güncel parseli (101/13) çiziyor; belge önce o "
                "parseli kapatmalı (close).", "draft", reg=taken),
        refused("Sürüm yazısı geçersiz", {"document": 1, "approval": APPROVAL, "expectedRevision": "abc"},
                "invalid_revision", "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision", reg=three),
        conflict("Beklenen sürüm eski: çatışma", {"draft": 1, "approval": APPROVAL}, draft),
        refused("Taslağın parselinin kimliği o arada başka alanla kaydedilmiş", {"draft": 1, "approval": APPROVAL},
                "parcel_identity_taken", "Ankara/Çankaya/Kızılay/101/13 kimlikli güncel bir parsel var (4 numaralı); "
                "belge önce onu kapatmalı (close).", "draft", reg=named),
        refused("Onay sayısı 64 karakteri aşıyor", {"document": 1, "approval": {"reference": "S" * 65}},
                "invalid_input", "Onay sayısı boş olmaz; en çok 64 karakterdir, denetim karakteri taşımaz. Onayı veren "
                "yerin yazdığı sayıyı yazın.", "approval.reference", reg=three),
        refused("Onay tarihinde denetim karakteri", {"document": 1, "approval": {"reference": "1", "date": "2026\u0007"}},
                "invalid_input", "Onay tarihi en çok 64 karakterdir, denetim karakteri taşımaz.", "approval.date",
                reg=three),
    ]


def withdrawn(reg, number):
    """The register after a draft is withdrawn (none when nothing is left) and the answer."""
    reg = copy.deepcopy(reg)
    at = next(i for i, x in enumerate(reg["drafts"]) if x["number"] == number)
    name = reg["drafts"].pop(at)["name"]
    if not reg["drafts"]:
        del reg["drafts"]
    empty_ = not (reg["documents"] or reg["boundaries"] or reg["parcels"] or reg.get("drafts"))
    return (None if empty_ else reg), {"name": name, "revision": "$current"}


def withdraw_cases():
    draft = drafted()
    alone, _ = record(None, KADASTRO, [("A1", None)], draft=True)
    both = two_drafts()
    closing_only, _ = record(first(), {"name": "2026/9 terkin", "kind": "Terkin"}, (), [2], draft=True)
    joined, _ = record(record(None, KADASTRO, [("A1", None), ("A2", None)])[0], {"name": "2026/200 tevhit"},
                       [("A13", None)], [1, 2], draft=True)
    arc_draft, _ = record(arced(), {"name": "2026/150 ifraz"}, [("A19", None), ("A20", None)], [4], ents=MORE_ENTS,
                          draft=True)
    attr_draft, _ = record(None, KADASTRO, [("A23", None)], ents=MORE_ENTS, draft=True)
    approved_first, _ = approve(both, APPROVAL, draft=1)
    with_approval, _ = record(approve(first(), {"reference": "1985/12"}, document=1)[0], IFRAZ,
                              [("A11", None), ("A12", None)], [1], draft=True)
    with_lengths, _ = record(record(None, KADASTRO, [("A1", None, [(0, 0, "20,00")]), ("A2", None), ("A3", None)])[0],
                             IFRAZ, [("A11", None), ("A12", None)], [1], draft=True)
    overtaken, _ = record(draft, {"name": "2026/9 terkin"}, (), [1])
    step = "Taslağı geri çek"
    cases = []
    for name_, reg_, number in (
            ("Taslak geri çekilir: güncel kayıtlar değişmez", draft, 1),
            ("Tek taslak geri çekilince kayıt kalmaz", alone, 1),
            ("İki taslaktan birincisi geri çekilir", both, 1),
            ("İki taslaktan ikincisi geri çekilir", both, 2),
            ("Yalnız kapatan taslak geri çekilir", closing_only, 1),
            ("Tevhit taslağı geri çekilir: iki parsel güncel kalır", joined, 1),
            ("Yaylı taslak geri çekilir: komşunun yayı bölünmeden kalır", arc_draft, 1),
            ("Öznitelikten ölçülü taslak geri çekilir: kayıt kalmaz", attr_draft, 1),
            ("Biri onaylandıktan sonra öbür taslak geri çekilir: onaylı belge kalır", approved_first, 2),
            ("Belgenin onayı taslak geri çekilince kalır", with_approval, 1),
            ("Belgelerin kenar ölçüleri taslak geri çekilince kalır", with_lengths, 1),
            ("Kapatacağı parsel o arada kapanmış taslak da geri çekilir", overtaken, 1)):
        after_, out_ = withdrawn(reg_, number)
        cases.append(step_case(name_, reg_, {"draft": number}, out_, after_, step))
    after_, out_ = withdrawn(draft, 1)
    twice = step_case("Geri çekilen taslak ikinci kez çekilmez", draft, {"draft": 1}, out_, after_, step)
    twice["steps"].insert(1, {"op": "execute", "input": {"draft": 1}, "result": {"status": "failed", "error": {
        "code": "unknown_parcel_draft", "message": "Parsel kayıtlarında 1 numaralı belge taslağı yok.",
        "path": "draft"}}, "expect": {"revision": "same", "canUndo": True}})
    deleted = step_case("Alanı çizimden silinmiş taslak: kayıt çizimden ayrıdır", draft, {"draft": 1}, out_, after_,
                        step)
    deleted["setup"]["entities"] = [dict(e, id=i + 1) for i, e in enumerate(
        e for e in deleted["setup"]["entities"] if e["id"] != SLOT["A11"])]
    deleted["setup"]["uids"] = [u for i, u in enumerate(deleted["setup"]["uids"]) if i + 1 != SLOT["A11"]]
    return cases + [
        twice,
        deleted,
        step_case("Plan yazmaz", draft, {"draft": 1}, out_, after_, step, op="plan"),
        step_case("Doğrulama", draft, {"draft": 1}, None, None, step, op="validate"),
        refused("Olmayan taslak", {"draft": 2}, "unknown_parcel_draft",
                "Parsel kayıtlarında 2 numaralı belge taslağı yok.", "draft", reg=draft),
        refused("Kaydı olmayan çizim", {"draft": 1}, "unknown_parcel_draft",
                "Parsel kayıtlarında 1 numaralı belge taslağı yok.", "draft"),
        refused("Taslak numarası sıfır", {"draft": 0}, "unknown_parcel_draft",
                "Parsel kayıtlarında 0 numaralı belge taslağı yok.", "draft", reg=draft),
        refused("Sürüm yazısı geçersiz", {"draft": 1, "expectedRevision": "abc"}, "invalid_revision",
                "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                "ya da komutun planından alın.", "expectedRevision", reg=draft),
        conflict("Beklenen sürüm eski: çatışma", {"draft": 1}, draft),
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
    note = ("ADR 4220, 4620 ve 5020. Bağımsız başvuru scripts/fixtures/parcel_register_cases.py: belgeler, sınırların eşlenmesi ve "
            "kapanması, alanlar ve yay boyları betikte; KentOS kodu yok.")
    ok = write("cad.parcels.record", "Parsel kaydı", note, record_cases(), check)
    ok = write("cad.parcels.records", "Parsel kayıtları", note, records_cases(), check) and ok
    note2 = ("ADR 5020. Bağımsız başvuru scripts/fixtures/parcel_register_cases.py: taslak, onay ve kayıt betikte; "
             "KentOS kodu yok.")
    ok = write("cad.parcels.approve", "Belgenin onayı", note2, approve_cases(), check) and ok
    ok = write("cad.parcels.withdraw", "Taslağı geri çek", note2, withdraw_cases(), check) and ok
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
