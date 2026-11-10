#!/usr/bin/env python3
"""The shared cases of `cad.readjustment.sheets` (Ayırma çapı paftaları; plan/map-cad.md MAP-11, docs/adr/2680),
without KentOS code.

    python3 scripts/fixtures/readjustment_sheets_cases.py           # writes fixtures/commands/v1/cad.readjustment.sheets.json
    python3 scripts/fixtures/readjustment_sheets_cases.py --check   # writes nothing; compares

The rules, by hand (Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13, as the ADR reads it):

- the scope, its parts and their numbered points are `readjustment_scope_cases.py`'s exact reference (the same
  drawing); a sheet for every parcel the scope finds partly inside, in its order; none: refused; a partial parcel
  without Ada or Parsel, or with another's: refused (the sheet is named by it);
- a sheet's id is `ayirma-capi-k` (k from 1, the first not taken), its name “Ayırma çapı ‹Ada/Parsel›”; the template's
  map frame (KentOS's separation sheet, read from its file) less its label band holds the parcel's box widened by 5%
  on each side: the scale is the smallest of the paper table's standard scales (`papers.json`) not under the box's
  need (metres over paper metres, the larger of the two sides, rounded up); the centre is the box's middle; a scale
  given is kept and the map centred;
- the table of areas: the record as written (its comma a point), the computed area, the tolerance, the parts inside
  and outside, each value's shortest decimal form rounded once to the places by the mode; the difference of the
  written computed area and the record, exactly (the record's places or the policy's, the more); what the part outside
  keeps (the scope's registered − inside, m. 31/2); “—” for what is not there; the table of parts: Giren / Girmeyen (numbered on a side with more than one), the outer ring's
  numbers closed by its first, each hole's after it, the area as above;
- the book then lists the sheets (A4 portrait, the template's nine items, the map and its scale);
- docs/adr/3480: each outside part given a number, its label: the part's name and the number (trimmed), at the middle
  of the widest stretch inside the part of the horizontal line at the half height of its outer ring's corners (each
  ring's crossings of the line, in order, paired: a straight edge crosses when its ends lie on either side, the upper
  end counted, at the interpolated x; an arc where its circle meets the line within its sweep from its start, its end
  not counted); the first of equal stretches;
- docs/adr/4280: with the user's `numberHeight` (paper mm), the number is a text whose height is that times the
  sheet's scale over 1000, shrunk by tenths (1, 0.9 … 0.1) until its box lies inside the part: tried horizontal at the
  place above, then along the outer ring's longest straight edge (read from the left, in (−90°, 90°]) at the same rule's
  place in the part turned so; the largest share wins, the horizontal among equals; none fits: horizontal at 0.1. The
  box (ADR 0145) is its letters' advances (Barlow 500, read here from the font file without a font library, in
  thousandths of an em) times its height wide, from 0.73 of its height under its middle to 0.65 over it, turned about
  it; inside when the part clipped by the box (Sutherland–Hodgman) has the box's area to 1e-9 of it;
- docs/adr/4680: with `numberLeader` only the full height is tried (horizontal, then along the longest edge); a number
  that does not fit goes outside, horizontal, at the full height, on a leader whose tip is the part's place above and
  whose landing starts on that line one text height east of the line's last crossing of the part (`at`), the tip in
  `leader`.
"""

import copy
import json
import math
import struct
import sys
from decimal import Decimal
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import readjustment_scope_cases as rs  # noqa: E402

ROOT = rs.ROOT
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.readjustment.sheets.json"
TEMPLATE = json.loads((ROOT / "crates" / "shared" / "sheet" / "templates" / "ayirma-capi.json").read_text("utf-8"))
SCALES = json.loads((ROOT / "crates" / "shared" / "sheet" / "data" / "papers.json").read_text("utf-8"))["scales"]
W = 1e-6
STEP = "Ayırma çapı paftaları"
WORD = "Ayırma çapı"
GROUP = "Ayırma çapı noktaları"
MARGIN = 5


def item(template, id_):
    return next(i for i in template["sheet"]["items"] if i["id"] == id_)


MAP_AT = [i["id"] for i in TEMPLATE["sheet"]["items"]].index("harita")
MAP_NAME = item(TEMPLATE, "harita")["name"]


def fit_scale(box):
    """The smallest standard scale the box fits at with its margin (rotation 0: east over the width, north over the
    height), as a host fits a map to objects."""
    m = item(TEMPLATE, "harita")
    band, pad = m["kind"].get("labelBand", 0), m.get("padding", 0)
    width = m["frame"]["width"] - 2 * pad - 2 * band
    height = m["frame"]["height"] - 2 * pad - 2 * band
    w, h = box[2] - box[0], box[3] - box[1]
    need = max((w * 1.0 + h * 0.0) * 1e6 / float(width), (w * 0.0 + h * 1.0) * 1e6 / float(height))
    need *= 1.0 + 2.0 * float(MARGIN) / 100.0
    scale = max(int(math.ceil(need)), 1)
    return next((s for s in SCALES if s >= scale), scale)


def box_of(entity):
    xs = [p["x"] for p in entity["pts"]]
    ys = [p["y"] for p in entity["pts"]]
    return [min(xs), min(ys), max(xs), max(ys)]


def written(v, scale, mode):
    """A computed value at the places by the mode: its shortest decimal form, rounded once."""
    return rs.written(rs.round_to(F(Decimal(repr(v))), scale, mode), scale)


def names(parts):
    on = {s: sum(1 for p in parts if p["side"] == s) for s in ("inside", "outside")}
    seen = {"inside": 0, "outside": 0}
    out = []
    for p in parts:
        seen[p["side"]] += 1
        word = "Giren" if p["side"] == "inside" else "Girmeyen"
        out.append(f"{word} {seen[p['side']]}" if on[p["side"]] > 1 else word)
    return out


def ring_text(r):
    n = [str(x) for x in r["numbers"]]
    return "-".join(n + n[:1])


def boundary(part):
    text = ring_text(part["rings"][0])
    for hole in part["rings"][1:]:
        text += f" (delik {ring_text(hole)})"
    return text


def areas(p, scale, mode):
    rec = p.get("recordedText")
    point = rec.replace(",", ".") if rec is not None else None
    readable = point is not None and rs.plain(point)
    registered = "—" if rec is None else point if readable else rec
    calc = written(p["calculated"], scale, mode)
    diff = rs.written(F(calc) - F(point), max(rs.places(point), scale)) if readable else "—"
    w = lambda v: written(v, scale, mode)  # noqa: E731
    return [["Tescilli yüzölçümü", registered], ["Hesaplanan yüzölçümü", calc], ["Yüzölçümü farkı", diff],
            ["Yanılma sınırı", w(p["limit"])], ["Giren kısım", w(p["inside"])], ["Girmeyen kısım", w(p["outside"])],
            ["Girmeyen kısma kalan (tescilli − giren)", p.get("outsideRecorded", "—")]]


def label_at(part):
    """docs/adr/3480: where an outside part's number stands, by the rule above (the arcs' circles from the bulge)."""
    outer = part["rings"][0]["pts"]
    y = (min(q[1] for q in outer) + max(q[1] for q in outer)) / 2
    xs = crossings(part, y)
    pairs = [(xs[k], xs[k + 1]) for k in range(0, len(xs) - 1, 2)]
    best = max(pairs, key=lambda q: q[1] - q[0]) if pairs else None
    for q in pairs:  # the first of equals
        if q[1] - q[0] == best[1] - best[0]:
            best = q
            break
    return [(best[0] + best[1]) / 2, y]


def crossings(part, y):
    """Where the horizontal line at `y` crosses the part's rings, sorted, by the rule above."""
    xs = []
    for ring in part["rings"]:
        pts, bulges = ring["pts"], ring.get("bulges") or [0.0] * len(ring["pts"])
        for i, a in enumerate(pts):
            b, bulge = pts[(i + 1) % len(pts)], bulges[i]
            if bulge == 0:
                if (a[1] > y) != (b[1] > y):
                    xs.append(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]))
                continue
            theta = 4 * math.atan(bulge)
            chord = math.hypot(b[0] - a[0], b[1] - a[1])
            r = chord / (2 * math.sin(abs(theta) / 2))
            mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
            # The centre on the chord's left normal at r·cos(θ/2), the right one for a clockwise arc.
            d = r * math.cos(theta / 2) * (1 if theta > 0 else -1)
            nx, ny = -(b[1] - a[1]) / chord, (b[0] - a[0]) / chord
            cx, cy = mx + nx * d, my + ny * d
            h = r * r - (y - cy) ** 2
            if h < 0:
                continue
            a0 = math.atan2(a[1] - cy, a[0] - cx)
            for x in (cx - math.sqrt(h), cx + math.sqrt(h)):
                turn = ((math.atan2(y - cy, x - cx) - a0) * (1 if theta > 0 else -1)) % (2 * math.pi)
                if turn < abs(theta):
                    xs.append(x)
    xs.sort()
    return xs


def font_advances(path):
    """A TrueType font's advances by character, thousandths of an em rounded: its head (units per em), hhea (metrics
    count), hmtx and cmap (format 4, Windows Unicode) tables read by hand."""
    data = path.read_bytes()
    count = struct.unpack(">H", data[4:6])[0]
    tables = {}
    for i in range(count):
        tag, _, offset, _ = struct.unpack(">4sIII", data[12 + 16 * i:28 + 16 * i])
        tables[tag.decode("latin-1")] = offset
    upem = struct.unpack(">H", data[tables["head"] + 18:tables["head"] + 20])[0]
    metrics = struct.unpack(">H", data[tables["hhea"] + 34:tables["hhea"] + 36])[0]
    widths = [struct.unpack(">H", data[tables["hmtx"] + 4 * i:tables["hmtx"] + 4 * i + 2])[0] for i in range(metrics)]
    cmap = tables["cmap"]
    for i in range(struct.unpack(">H", data[cmap + 2:cmap + 4])[0]):
        platform, encoding, offset = struct.unpack(">HHI", data[cmap + 4 + 8 * i:cmap + 12 + 8 * i])
        sub_ = cmap + offset
        if (platform, encoding) == (3, 1) and struct.unpack(">H", data[sub_:sub_ + 2])[0] == 4:
            break
    twice = struct.unpack(">H", data[sub_ + 6:sub_ + 8])[0]
    n = twice // 2
    ends = struct.unpack(f">{n}H", data[sub_ + 14:sub_ + 14 + twice])
    starts = struct.unpack(f">{n}H", data[sub_ + 16 + twice:sub_ + 16 + 2 * twice])
    deltas = struct.unpack(f">{n}h", data[sub_ + 16 + 2 * twice:sub_ + 16 + 3 * twice])
    at = sub_ + 16 + 3 * twice
    ranges = struct.unpack(f">{n}H", data[at:at + twice])

    def glyph(c):
        for k in range(n):
            if starts[k] <= c <= ends[k]:
                if ranges[k] == 0:
                    return (c + deltas[k]) & 0xFFFF
                g = struct.unpack(">H", data[at + 2 * k + ranges[k] + 2 * (c - starts[k]):][:2])[0]
                return (g + deltas[k]) & 0xFFFF if g else 0
        return 0

    def advance(ch):
        g = glyph(ord(ch))
        return round(widths[min(g, metrics - 1)] * 1000 / upem)
    return advance


ADVANCE = font_advances(ROOT / "apps/desktop/assets/fonts/drawing/Barlow-500.ttf")
SHARES = [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1]


def text_box(number, at, turn, h):
    """ADR 0145's box of a one-line text centred at `at` (middle centre), turned `turn` degrees."""
    w = sum(ADVANCE(c) for c in number) / 1000.0 * h
    r = math.radians(turn)
    c, s = math.cos(r), math.sin(r)
    return [(at[0] + c * x - s * y, at[1] + s * x + c * y)
            for x, y in ((-w / 2, -0.73 * h), (w / 2, -0.73 * h), (w / 2, 0.65 * h), (-w / 2, 0.65 * h))]


def ring_area(pts):
    n = len(pts)
    ox, oy = pts[0]
    return sum((pts[i][0] - ox) * (pts[(i + 1) % n][1] - oy) - (pts[(i + 1) % n][0] - ox) * (pts[i][1] - oy)
               for i in range(n)) / 2


def clipped(subject, window):
    """Sutherland–Hodgman: the subject ring inside the convex counter-clockwise window."""
    out = list(subject)
    for k in range(len(window)):
        a, b = window[k], window[(k + 1) % len(window)]
        side = lambda q: (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])  # noqa: E731
        given, out = out, []
        for i, q in enumerate(given):
            nxt = given[(i + 1) % len(given)]
            sq, sn = side(q), side(nxt)
            if sq >= 0:
                out.append(q)
            if (sq >= 0) != (sn >= 0):
                f = sq / (sq - sn)
                out.append((q[0] + (nxt[0] - q[0]) * f, q[1] + (nxt[1] - q[1]) * f))
        if not out:
            return []
    return out


def inside(part, box):
    """Whether the box lies inside the part (straight rings here): the part clipped by it keeps its area."""
    assert all(not any(r.get("bulges") or []) for r in part["rings"]), "the fitting cases' parts are straight"
    whole = abs(ring_area(box))
    shared = 0.0
    for k, r in enumerate(part["rings"]):
        piece = clipped([tuple(q) for q in r["pts"]], box)
        a = abs(ring_area(piece)) if len(piece) >= 3 else 0.0
        shared += -a if k else a
    return whole > 0 and shared >= whole * (1 - 1e-9)


def turned(part, about, angle):
    c, s = math.cos(angle), math.sin(angle)
    out = copy.deepcopy(part)
    for r in out["rings"]:
        r["pts"] = [[about[0] + (q[0] - about[0]) * c - (q[1] - about[1]) * s,
                     about[1] + (q[0] - about[0]) * s + (q[1] - about[1]) * c] for q in r["pts"]]
    return out


def turns(part):
    out = [0.0]
    outer = part["rings"][0]
    pts, bulges = outer["pts"], outer.get("bulges") or [0.0] * len(outer["pts"])
    best = None
    for i, a in enumerate(pts):
        b = pts[(i + 1) % len(pts)]
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        if bulges[i] == 0 and (best is None or length > best[0]):
            best = (length, a, b)
    if best is not None:
        _, a, b = best
        angle = math.degrees(math.atan2(b[1] - a[1], b[0] - a[0]))
        if angle > 90:
            angle -= 180
        elif angle <= -90:
            angle += 180
        if angle != 0:
            out.append(angle)
    return out


def fitted(part, number, height, leader=False):
    """docs/adr/4280: the place, turn, height, share and fit of a number's text in its part; docs/adr/4680: with
    `leader`, the full height only, else outside on a leader (its tip last)."""
    origin = part["rings"][0]["pts"][0]
    best = None
    for turn in turns(part):
        if turn == 0:
            at = label_at(part)
        else:
            r = math.radians(turn)
            local = label_at(turned(part, origin, -r))
            c, s = math.cos(r), math.sin(r)
            x, y = local[0] - origin[0], local[1] - origin[1]
            at = [origin[0] + x * c - y * s, origin[1] + x * s + y * c]
        for share in (SHARES[:1] if leader else SHARES):
            if best is not None and best[2] >= share:
                break
            if inside(part, text_box(number, at, turn, height * share)):
                best = (at, turn, share)
                break
    if best is None and leader:
        tip = label_at(part)
        return [crossings(part, tip[1])[-1] + height, tip[1]], 0.0, height, 1.0, False, tip
    if best is None:
        return label_at(part), 0.0, height * SHARES[-1], SHARES[-1], False, None
    at, turn, share = best
    return at, turn, height * share, share, True, None


def labels(p, numbers, height=None, scale=None, leader=False):
    """docs/adr/3480: each numbered outside part's name, number and place; docs/adr/4280: with a height, its fitted
    text."""
    if numbers is None:
        return []
    out, k = [], 0
    for n, part in zip(names(p["parts"]), p["parts"]):
        if part["side"] == "outside":
            number = numbers[k].strip()
            label = {"part": n, "number": number, "at": label_at(part)}
            if height is not None:
                at, turn, h, share, fits, tip = fitted(part, number, height * scale / 1000.0, leader)
                label.update({"at": at, "height": h, "rotation": turn, "share": share, "fits": fits})
                if tip is not None:
                    label["leader"] = tip
            out.append(label)
            k += 1
    return out


def part_rows(p, scale, mode, numbers=None):
    """The table of parts; with the parcel numbers given for its outside parts (m. 13/3, docs/adr/3080), each outside
    part's name says its number."""
    out, k = [], 0
    for n, part in zip(names(p["parts"]), p["parts"]):
        if numbers is not None and part["side"] == "outside":
            n = f"{n} · parsel {numbers[k].strip()}"
            k += 1
        out.append([n, boundary(part), written(part["area"], scale, mode)])
    return out


def sheets(entities, chosen, kw, given_scale=None, taken=(), numbers=None, height=None, leader=False):
    """The sheets' answer rows and the book's listing for the partial parcels of the scope."""
    scope_kw = {k: kw[k] for k in ("scale", "mode", "tolerance", "field") if k in kw}
    scope = rs.scope(entities, 0, chosen, **scope_kw)
    scale, mode = kw.get("scale", 2), kw.get("mode", "half_even")
    rows, listing = [], []
    k = 1
    taken = set(taken)
    by_uid = {rs.uid(i + 1): e for i, e in enumerate(entities)}
    for p in scope["parcels"]:
        if p["kind"] != "partial":
            continue
        while f"ayirma-capi-{k}" in taken:
            k += 1
        sid = f"ayirma-capi-{k}"
        taken.add(sid)
        box = box_of(by_uid[p["uid"]])
        centre = [box[0] + (box[2] - box[0]) / 2.0, box[1] + (box[3] - box[1]) / 2.0]
        s = given_scale if given_scale is not None else fit_scale(box)
        name = f"{WORD} {p['parcel']}"
        rows.append({"sheet": sid, "name": name, "uid": p["uid"], "parcel": p["parcel"], "scale": s, "center": centre,
                     "points": len(p["points"]), "areas": areas(p, scale, mode),
                     "parts": part_rows(p, scale, mode, (numbers or {}).get(p["uid"])),
                     **({"labels": labels(p, (numbers or {}).get(p["uid"]), height, s, leader)}
                        if (numbers or {}).get(p["uid"]) else {})})
        listing.append({"id": sid, "name": name, "width": 210.0, "height": 297.0,
                        "items": len(TEMPLATE["sheet"]["items"]),
                        "maps": [{"item": f"{sid}-item-{MAP_AT}", "name": MAP_NAME, "scale": float(s)}]})
    return rows, listing


# The parcel 101/8, wholly outside the boundary (the scope's kind `outside`; ADR 3880's case).
EIGHT = next(i for i, e in enumerate(rs.MAIN) if e["attrs"].get("Ada") == "101" and e["attrs"].get("Parsel") == "8")


def scope_input(chosen=None, **kw):
    inp = {"boundary": rs.uid(1), "scale": kw.get("scale", 2), "rounding": kw.get("mode", "half_even")}
    if chosen is not None:
        inp["uids"] = [rs.uid(i + 1) for i in chosen]
    if "tolerance" in kw:
        inp["tolerance"] = kw["tolerance"]
    if "field" in kw:
        inp["field"] = kw["field"]
    return inp


def made(rows, plan=False):
    if plan:
        return {"sheets": rows, "uids": [], "revision": "$current"}
    return {"group": "$any", "sheets": [{**r, "layer": "$any"} for r in rows], "uids": "$any", "revision": "$current"}


WRITTEN = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}


def ok(name, chosen, entities=rs.MAIN, setup_=None, op="execute", options=None, numbers=None, height=None,
       leader=False, **kw):
    given = (options or {}).get("scale")
    rows, listing = sheets(entities, chosen, kw, given, numbers=numbers, height=height, leader=leader)
    inp = {"scope": scope_input(chosen, **kw)}
    if options:
        inp["options"] = options
    if numbers is not None:
        inp["numbers"] = numbers
    if height is not None:
        inp["numberHeight"] = height
    if leader:
        inp["numberLeader"] = True
    step = {"op": op, "input": inp, "within": W}
    if op == "validate":
        step["result"] = {"status": "completed", "output": None, "warnings": []}
        step["expect"] = {"revision": "same", "sheets": []}
    else:
        step["result"] = {"status": "completed", "output": made(rows, op == "plan"), "warnings": []}
        step["expect"] = {"revision": "same", "sheets": []} if op == "plan" else {**WRITTEN, "sheets": listing}
    case = {"name": name, "steps": [step]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, inp, code, message, path, setup_=None, status="failed"):
    err = {"code": code, "message": message, "path": path}
    if status == "conflict":
        err["revision"] = "$current"
    case = {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": status, "error": err},
                                     "expect": {"revision": "same", "sheets": []}}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def cases():
    P = rs.P_
    three = [P(3), P(6), P(10)]
    rows, listing = sheets(rs.MAIN, three, {})
    twins = [rs.BOUNDARY, rs.area_entity(rs.rect(60, 0, 90, 20), "parsel", rs.ident(3, "598.40")),
             rs.area_entity(rs.rect(60, 20, 90, 40), "parsel", rs.ident(3, "600.00"))]
    no_map = copy.deepcopy(TEMPLATE)
    no_map["meta"]["id"] = "user:haritasiz-cap"
    item(no_map, "harita")["id"] = "kroki"
    for it in no_map["sheet"]["items"]:
        if it["kind"].get("map") == "harita":
            it["kind"]["map"] = "kroki"
    base = {"scope": scope_input([P(3)])}
    return [
        ok("kısmen giren üç parsel: üç ayırma çapı paftası, noktalar parsellerin katmanlarında", three),
        ok("kısmen giren parsel: yüzölçümleri ve kısımlar politikanın basamağıyla", [P(3)]),
        ok("kaydı okunmayan parsel: tescilli yazıldığı gibi, fark yok", [P(6)]),
        ok("iki parçası dışarıda kalan parsel: Girmeyen 1 ve 2 aynı paftada (m. 13/3)", [P(10)]),
        # docs/adr/3080: the parcel numbers of the parts outside, as the user writes them (m. 13/3).
        ok("dışarıda kalan iki parçanın parsel numaraları yazılınca Kısımlar'da adlarıyla (m. 13/3)", [P(10)],
           numbers={rs.uid(P(10) + 1): ["101/24", " 101/25 "]}),
        ok("üç parselin dış parçaları numaralı: her parça ve numarası parselin katmanında, numara parçanın içinde",
           three, numbers={rs.uid(P(3) + 1): ["101/30"], rs.uid(P(6) + 1): ["101/31"],
                           rs.uid(P(10) + 1): ["101/24", "101/25"]}),
        # docs/adr/4280: the numbers as texts fitted inside their parts, at the user's height on paper.
        ok("numara yazısı 2 mm: dış parçalara sığar; ince uzun parçada en uzun kenarı boyunca döner", three,
           numbers={rs.uid(P(3) + 1): ["101/30"], rs.uid(P(6) + 1): ["101/31"],
                    rs.uid(P(10) + 1): ["101/24", "101/25"]}, height=2.0),
        ok("numara yazısı 6 mm: sığmayan onda bir adımlarla küçülür", three,
           numbers={rs.uid(P(3) + 1): ["101/30"], rs.uid(P(6) + 1): ["101/31"],
                    rs.uid(P(10) + 1): ["101/24", "101/25"]}, height=6.0),
        # docs/adr/4680: a number too big for its part outside on a leader, at the full height.
        ok("numara yazısı 6 mm, kılavuzla: tam boyda sığmayan numara küçülmez, dışarıda kılavuzla", three,
           numbers={rs.uid(P(3) + 1): ["101/30"], rs.uid(P(6) + 1): ["101/31"],
                    rs.uid(P(10) + 1): ["101/24", "101/25"]}, height=6.0, leader=True),
        ok("numara yazısı 2 mm, kılavuzla: tam boyda sığanlar içeride, kılavuz yok", three,
           numbers={rs.uid(P(3) + 1): ["101/30"], rs.uid(P(6) + 1): ["101/31"],
                    rs.uid(P(10) + 1): ["101/24", "101/25"]}, height=2.0, leader=True),
        refused("kılavuz, numaranın yazı boyu olmadan", {"scope": scope_input([P(10)]),
                "numbers": {rs.uid(P(10) + 1): ["101/24", "101/25"]}, "numberLeader": True}, "invalid_input",
                "Kılavuz, numaranın yazısıyla olur: numaranın yazı boyunu (numberHeight) da verin.", "numberLeader"),
        ok("numara yazısı 400 mm: hiçbir boyda sığmaz, yatay ve en küçük payda", [P(10)],
           numbers={rs.uid(P(10) + 1): ["101/24", "101/25"]}, height=400.0),
        ok("numara yazısının planı: yazmaz", [P(10)], op="plan", numbers={rs.uid(P(10) + 1): ["101/24", "101/25"]},
           height=2.0),
        refused("numara yazısının boyu 0", {"scope": scope_input([P(10)]), "numbers": {rs.uid(P(10) + 1): ["101/24", "101/25"]},
                "numberHeight": 0.0}, "invalid_input",
                "Parsel numarasının yazı boyu 0'dan büyük bir milimetre olmalı (yazılan 0).", "numberHeight"),
        refused("parsel numaraları: parça sayısı tutmuyor", {"scope": scope_input([P(10)]),
                "numbers": {rs.uid(P(10) + 1): ["101/24"]}}, "invalid_input",
                "“101/10” parselinin düzenleme dışında 2 parçası var; 1 parsel numarası verildi.",
                f"numbers.{rs.uid(P(10) + 1)}"),
        # ADR 3880: the parcel named by its Ada/Parsel, and why it takes no number.
        refused("parsel numaraları: kısmen giren parsel değil", {"scope": scope_input([P(3)]),
                "numbers": {rs.uid(P(1) + 1): ["5"]}}, "invalid_input",
                "“101/1” parseli kapsamda kısmen giren bir parsel değil; numaralar kısmen giren parsellerin "
                "dışarıda kalan parçalarına verilir.", f"numbers.{rs.uid(P(1) + 1)}"),
        refused("parsel numaraları: kapsamda tamamen içerideki parsel", {"scope": scope_input([P(1), P(3)]),
                "numbers": {rs.uid(P(1) + 1): ["5"]}}, "invalid_input",
                f"“101/1” parseli düzenleme sınırının tamamen içinde, dışarıda parçası yok; numaralar kısmen giren "
                "parsellerin dışarıda kalan parçalarına verilir.", f"numbers.{rs.uid(P(1) + 1)}"),
        refused("parsel numaraları: kapsamda düzenleme dışındaki parsel", {"scope": scope_input([EIGHT, P(3)]),
                "numbers": {rs.uid(EIGHT + 1): ["5"]}}, "invalid_input",
                "“101/8” parseli düzenleme sınırının tamamen dışında, düzenlemeye girmiyor; numaralar kısmen giren "
                "parsellerin dışarıda kalan parçalarına verilir.", f"numbers.{rs.uid(EIGHT + 1)}"),
        refused("parsel numaraları: adı olmayan nesne kimliğiyle söylenir", {"scope": scope_input([P(3)]),
                "numbers": {rs.uid(1): ["5"]}}, "invalid_input",
                f"“{rs.uid(1)}” kapsamda kısmen giren bir parsel değil; numaralar kısmen giren parsellerin "
                "dışarıda kalan parçalarına verilir.", f"numbers.{rs.uid(1)}"),
        refused("parsel numaraları: aynı numara iki parçada", {"scope": scope_input([P(10)]),
                "numbers": {rs.uid(P(10) + 1): ["101/24", "101/24"]}}, "invalid_input",
                "“101/10” parselinin iki parçasına aynı parsel numarası (“101/24”) verilmiş.",
                f"numbers.{rs.uid(P(10) + 1)}"),
        refused("parsel numaraları: boş numara", {"scope": scope_input([P(10)]),
                "numbers": {rs.uid(P(10) + 1): ["101/24", "  "]}}, "invalid_input",
                "Parsel numarası boş olamaz ve en çok 20 karakterdir.", f"numbers.{rs.uid(P(10) + 1)}[1]"),
        ok("köşesi sınırın dışında: girmeyen kısım L biçiminde, sınırı ortak noktalarla", None, rs.CORNER,
           rs.setup(rs.CORNER)),
        ok("sınır parselin içinde: girmeyen kısım delikli, deliği giren kısmın noktaları", None, rs.HOLED,
           rs.setup(rs.HOLED)),
        ok("ölçek verilince harita parseli o ölçekte ortalar", [P(3)], options={"scale": 1000}),
        ok("şablonun öbür soruları cevaplanır", [P(3)],
           options={"values": [{"name": "pafta_no", "value": "G23-c-11-b"}, {"name": "kontrol_eden", "value": "Ayşe Kaya"}]}),
        ok("sıfır basamak: tablolar tam sayıyla", [P(3), P(10)], scale=0),
        ok("sıfırdan uzağa yuvarlama: 1000/3 m² → 333.34", [P(3)], mode="away_from_zero"),
        ok("tolerans 3 mm: 4 mm'lik parça sayılır, parselin çapı olur", [P(9)], tolerance=0.003),
        ok("tamamı giren ve dışarıdaki parseller verilince yalnız kısmen girenin paftası", [P(1), P(3), P(7)]),
        ok("CBS projesinde de", [P(3)], setup_=rs.setup(rs.MAIN, "gis")),
        ok("plan aynı paftaları verir, yazmaz", three, op="plan"),
        ok("doğrulama", three, op="validate"),
        {"name": "geri alma: noktalar, katmanlar ve paftalar tek adımda gider",
         "steps": [{"op": "execute", "input": {"scope": scope_input(three)}, "within": W,
                    "result": {"status": "completed", "output": made(rows), "warnings": []},
                    "expect": {**WRITTEN, "sheets": listing}},
                   {"op": "undo", "returns": STEP, "expect": {"canUndo": False, "canRedo": True, "sheets": []}},
                   {"op": "redo", "returns": STEP, "expect": {"canUndo": True, "canRedo": False, "sheets": listing}}]},
        {"name": "yeniden yapınca var olan pafta söylenir, hiçbir şey yazılmaz",
         "steps": [{"op": "execute", "input": {"scope": scope_input([P(3)])}, "within": W,
                    "result": {"status": "completed", "output": made(sheets(rs.MAIN, [P(3)], {})[0]), "warnings": []},
                    "expect": {**WRITTEN, "sheets": sheets(rs.MAIN, [P(3)], {})[1]}},
                   {"op": "execute", "input": {"scope": scope_input([P(3)])},
                    "result": {"status": "failed", "error": {
                        "code": "separation_sheet_exists",
                        "message": f"“{WORD} 101/3” paftası zaten var; yeniden yapmak için önce o paftayı ve "
                                   f"“{GROUP} › 101/3” katmanını silin.", "path": "scope"}},
                    "expect": {"revision": "same", "sheets": sheets(rs.MAIN, [P(3)], {})[1]}}]},
        refused("kısmen giren parsel yok", {"scope": scope_input([P(1), P(2)])}, "no_partial_parcels",
                "Düzenleme sınırına kısmen giren parsel yok; ayırma çapı yalnız bir kısmı düzenleme sahasında kalan "
                "parsel için düzenlenir (Yönetmelik m. 13/1).", "scope"),
        refused("Ada'sı olmayan kısmen giren parsel", {"scope": scope_input([P(11)])}, "no_identity",
                f"“{rs.uid(P(11) + 1)}” parselinin Ada'sı ya da Parsel'i yok; ayırma çapı paftası parselin kimliğiyle "
                "adlandırılır.", "scope"),
        refused("aynı Ada/Parsel iki kısmen giren parselde", {"scope": scope_input()}, "duplicate_parcel",
                "“101/3” iki parselin kimliği; ayırma çapı paftası parselin kimliğiyle adlandırılır, önce "
                "kimliklerden birini düzeltin.", "scope", rs.setup(twins)),
        refused("bilinmeyen şablon", {**base, "source": {"type": "system", "id": "sys:yok"}}, "unknown_template",
                "“sys:yok” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin.", "source.id"),
        refused("haritası olmayan şablon", {**base, "source": {"type": "custom", "template": no_map}}, "not_fixed_map",
                "Şablonda “harita” kimlikli sabit görünüm penceresi yok; ayırma çapının haritası ona yerleşir.",
                "source"),
        refused("pafta adı verilmez", {**base, "options": {"name": "Çap"}}, "invalid_options",
                f"Ayırma çapı paftalarının adları parsellerinden gelir (“{WORD} 101/10”); options.name verilmez.",
                "options.name"),
        refused("merkez verilmez", {**base, "options": {"center": {"x": 487075.0, "y": 4420010.0}}}, "invalid_options",
                "Her paftanın haritası kendi parselini ortalar; options.center verilmez.", "options.center"),
        refused("parselin özniteliği cevap olarak verilmez", {**base, "options": {"values": [{"name": "ada", "value": "5"}]}},
                "invalid_options", "“ada” her paftada parselin kendi özniteliğinden gelir; cevap olarak verilmez.",
                "options.values[0].name"),
        refused("kapsamın reddi yeriyle: basamak 7", {"scope": {**scope_input([P(3)]), "scale": 7}}, "invalid_input",
                "Basamak 7 en çok 6 olabilir.", "scope.scale"),
        refused("kapsamın reddi yeriyle: sınır çizimde yok", {"scope": {**scope_input([P(3)]), "boundary": rs.uid(99)}},
                "entity_not_found", f"“{rs.uid(99)}” kimlikli nesne çizimde yok.", "scope.boundary"),
        refused("geçersiz sürüm", {**base, "expectedRevision": "01"}, "invalid_revision", "$any", "expectedRevision"),
        refused("eski sürüm: hiçbir şey yazılmaz", {**base, "expectedRevision": "9999"}, "revision_conflict", "$any",
                "expectedRevision", status="conflict"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.readjustment.sheets", "commandVersion": 1,
           "title": "Ayırma çapı paftaları",
           "note": "ADR 2680. Bağımsız başvuru scripts/fixtures/readjustment_sheets_cases.py: kapsam, parçalar ve "
                   "numaralı noktalar readjustment_scope_cases.py'nin kesirli başvurusundan, haritanın ölçeği şablonun "
                   "dosyasındaki çerçeveden ve kâğıt çizelgesinin standart ölçeklerinden, tabloların sayıları elle "
                   "yuvarlanarak; KentOS kodu yok.",
           "setup": rs.setup(rs.MAIN), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("Ayırma çapı paftaları ortak durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
