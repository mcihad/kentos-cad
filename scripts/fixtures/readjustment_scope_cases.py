#!/usr/bin/env python3
"""The shared cases of `cad.readjustment.scope` (Düzenleme kapsamı; plan/map-cad.md MAP-11, docs/adr/2060), without
KentOS code.

    python3 scripts/fixtures/readjustment_scope_cases.py           # writes fixtures/commands/v1/cad.readjustment.scope.json
    python3 scripts/fixtures/readjustment_scope_cases.py --check   # writes nothing; compares

The rules, by hand (Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13, 31, as the ADR reads them):

- the boundary and the parcels here are convex polygons with straight edges; every coordinate is taken exactly as
  stored (the float's own value, as a fraction);
- the part inside: the parcel clipped by each of the boundary's edges (Sutherland–Hodgman, exact); the part outside:
  the parcel cut by the edges one after another (the part right of edge i, of what was left of the edges before),
  the cells that share a stretch of edge joined into one piece; a piece counts when its mean width 2A/P is over the
  tolerance (A exact, P from the exact corners);
- whole: counted pieces inside and none outside; partial: counted pieces on both sides; outside: none counted inside;
- the computed area: the exact shoelace; the difference: computed − registered; the tolerance (yanılma sınırı): the
  2022 regulation's Article 8(2) table as ADR 1400 records it (from 0, 11, 101, 501, 1001, 5001, 25001 m²: 0.05;
  0.5 + (A − 10)·0.02; 2.3 + (A − 100)·0.01; 6.3 + (A − 500)·0.005; 8.8 + (A − 1000)·0.004; 24.8 + (A − 5000)·0.003;
  84.8 + (A − 25000)·0.0015), from the computed area;
- the status, in this order: outside (nothing counted inside, no narrow piece), sliver (a narrow piece anywhere),
  noRecord, invalidRecord (not plain decimal), needsCorrection (|difference| over the tolerance), then a whole parcel:
  finerThanPolicy (more places than the policy) or registered (basis: the record, its comma a point); a partial one:
  the basis is the exact inside area rounded once to the places by the mode, the outside keeps registered − basis
  (written with the record's places or the policy's, the more): outsideTooSmall when that is below zero, else
  computed;
- duplicate: two parcels in the answer with one Ada/Parsel; the totals: the boundary's exact area, the parts inside,
  what is left, the bases' exact sum written with their most places, the counts.
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.readjustment.scope.json"
X0, Y0 = 487000.0, 4420000.0
W = 1e-6
SAME = {"revision": "same", "canUndo": False}
FIELD = "Tapu alanı (m²)"
RULE = "tkgm-2022-m8-arsiv"


def uid(n):
    return f"0192f800-2060-7000-8000-{n:012x}"


def layer(lid, name):
    return {"id": lid, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.35}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def area_entity(pts, layer_id, attrs=None):
    return {"kind": "polygon", "layerId": layer_id, "attrs": attrs or {}, "pts": [P(*p) for p in pts]}


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def ident(n, recorded=None, ada="101", more=None):
    a = {"İl": "Ankara", "İlçe": "Çankaya", "Mahalle": "Kızılay", "Parsel": str(n)}
    if ada:
        a["Ada"] = ada
    if recorded is not None:
        a[FIELD] = recorded
    a.update(more or {})
    return a


# The boundary: a trapezium whose east edge leans, from (80, −10) to (70, 50); its area is 5100 m².
BOUNDARY = area_entity([(-10, -10), (80, -10), (70, 50), (-10, 50)], "duzenleme", {"Ad": "Düzenleme sınırı"})
PARCELS = [
    area_entity(rect(0, 0, 30, 20), "parsel", ident(1, "600.00")),                       # P1 whole, within
    area_entity(rect(30, 0, 60, 20), "parsel", ident(2, "603,50", more={"Kayıtlı alan": "601.00"})),  # P2 whole, comma
    area_entity(rect(60, 0, 90, 20), "parsel", ident(3, "598.40")),                      # P3 cut by the leaning edge
    area_entity(rect(0, 20, 30, 40), "parsel", ident(4, "590")),                         # P4 beyond the tolerance
    area_entity(rect(30, 20, 60, 40), "parsel", ident(5)),                               # P5 no record
    area_entity(rect(60, 20, 90, 40), "parsel", ident(6, "tapuda yok")),                 # P6 partial, unreadable
    area_entity(rect(100, 0, 130, 20), "parsel", ident(7, "600.00")),                    # P7 outside
    area_entity(rect(0, 49.995, 30, 70), "parsel", ident(8, "600.15")),                  # P8 a 5 mm strip inside
    area_entity(rect(-10.004, 0, -8.5, 20), "parsel", ident(9, "30.08")),                # P9 a 4 mm strip outside
    area_entity(rect(-8, -20, -4, 60), "parsel", ident(10, "321.10")),                   # P10 two pieces outside
    area_entity(rect(-10.05, 22, -9, 28), "parsel", ident(11, "5.99", ada=None)),        # P11 outside < error; no Ada
    area_entity(rect(-30, 30, -10, 40), "parsel", ident(12, "200.00")),                  # P12 touches the west edge
]
LINE = {"kind": "line", "layerId": "yol", "attrs": {}, "a": P(-10, -15), "b": P(80, -15)}


def setup(entities, work="map"):
    ents = []
    for e in entities:
        ents.append({**e, "id": len(ents) + 1})
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work == "gis":
        settings["workspace"] = "gis"
        settings.pop("cadWork")
    return {"format": "kentos.document", "version": 2, "name": "Düzenleme kapsamı", "settings": settings,
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("duzenleme", "Düzenleme sınırı"), layer("parsel", "Parsel"),
                       layer("yol", "Yol")],
            "activeLayer": "parsel", "entities": ents, "uids": [uid(e["id"]) for e in ents],
            "styles": {"items": [], "categories": []}}


# ── Exact plane geometry ───────────────────────────────────────────────────────────────────────────────────────────
def exact(e):
    return [(F(p["x"]), F(p["y"])) for p in e["pts"]]


def side(a, b, p):
    return (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])


def clip(poly, a, b, keep):
    """The part of `poly` left of a→b (keep = 1) or right of it (keep = −1), its edge on the line included."""
    out = []
    n = len(poly)
    for i in range(n):
        cur, nxt = poly[i], poly[(i + 1) % n]
        sc, sn = keep * side(a, b, cur), keep * side(a, b, nxt)
        if sc >= 0:
            out.append(cur)
        if (sc > 0 > sn) or (sc < 0 < sn):
            t = sc / (sc - sn)
            out.append((cur[0] + t * (nxt[0] - cur[0]), cur[1] + t * (nxt[1] - cur[1])))
    dedup = []
    for p in out:
        if not dedup or dedup[-1] != p:
            dedup.append(p)
    if len(dedup) > 1 and dedup[0] == dedup[-1]:
        dedup.pop()
    return dedup


def area(poly):
    n = len(poly)
    if n < 3:
        return F(0)
    s = sum(poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1] for i in range(n))
    return abs(s) / 2


def perimeter(poly):
    n = len(poly)
    return sum(math.hypot(float(poly[(i + 1) % n][0] - poly[i][0]), float(poly[(i + 1) % n][1] - poly[i][1]))
               for i in range(n))


def edges_of(poly):
    return [(poly[i], poly[(i + 1) % len(poly)]) for i in range(len(poly))]


def shared(c1, c2):
    """The length two cells share along their edges (collinear stretches, exact test)."""
    total = 0.0
    for p1, p2 in edges_of(c1):
        d = (p2[0] - p1[0], p2[1] - p1[1])
        dd = d[0] * d[0] + d[1] * d[1]
        if dd == 0:
            continue
        for q1, q2 in edges_of(c2):
            if side(p1, p2, q1) != 0 or side(p1, p2, q2) != 0:
                continue
            t1 = ((q1[0] - p1[0]) * d[0] + (q1[1] - p1[1]) * d[1]) / dd
            t2 = ((q2[0] - p1[0]) * d[0] + (q2[1] - p1[1]) * d[1]) / dd
            lo, hi = max(F(0), min(t1, t2)), min(F(1), max(t1, t2))
            if hi > lo:
                total += float(hi - lo) * math.sqrt(float(dd))
    return total


def inside_pieces(poly, edge):
    rest = poly
    for i in range(len(edge)):
        rest = clip(rest, edge[i], edge[(i + 1) % len(edge)], 1)
        if area(rest) == 0:
            return []
    return [(area(rest), perimeter(rest))]


def outside_pieces(poly, edge):
    cells, rest = [], poly
    for i in range(len(edge)):
        a, b = edge[i], edge[(i + 1) % len(edge)]
        right = clip(rest, a, b, -1)
        if area(right) > 0:
            cells.append(right)
        rest = clip(rest, a, b, 1)
        if area(rest) == 0:
            break
    # Cells that share a stretch of edge are one piece.
    group = list(range(len(cells)))

    def top(i):
        while group[i] != i:
            i = group[i]
        return i

    links = {}
    for i in range(len(cells)):
        for j in range(i + 1, len(cells)):
            s = shared(cells[i], cells[j])
            if s > 0:
                group[top(j)] = top(i)
                links[(i, j)] = s
    out = {}
    for i, c in enumerate(cells):
        a_, p_ = out.get(top(i), (F(0), 0.0))
        out[top(i)] = (a_ + area(c), p_ + perimeter(c))
    for (i, j), s in links.items():
        a_, p_ = out[top(i)]
        out[top(i)] = (a_, p_ - 2 * s)
    return list(out.values())


def width(piece):
    a, p = piece
    return 2 * float(a) / p if p > 0 else 0.0


# ── Numbers ────────────────────────────────────────────────────────────────────────────────────────────────────────
SLICES = [(0, 0, F(0), F("0.05")), (11, 10, F("0.5"), F("0.02")), (101, 100, F("2.3"), F("0.01")),
          (501, 500, F("6.3"), F("0.005")), (1001, 1000, F("8.8"), F("0.004")), (5001, 5000, F("24.8"), F("0.003")),
          (25001, 25000, F("84.8"), F("0.0015"))]


def limit(a):
    for first, start, base, rate in reversed(SLICES):
        if a >= first:
            return base + (a - start) * rate
    return SLICES[0][2]


def plain(text):
    whole, _, frac = text.replace(",", ".").partition(".")
    ok = lambda s: s != "" and all(c in "0123456789" for c in s)
    if "." in text.replace(",", "."):
        return ok(whole) and ok(frac) and text.count(",") + text.count(".") == 1
    return ok(whole)


def places(text):
    return len(text.split(".", 1)[1]) if "." in text else 0


def written(f, s):
    """`f` (a multiple of 10^-s) with exactly `s` places."""
    neg = f < 0
    n = abs(f) * 10 ** s
    assert n.denominator == 1, f
    digits = str(n.numerator).rjust(s + 1, "0")
    body = digits if s == 0 else digits[:-s] + "." + digits[-s:]
    return ("-" if neg and f != 0 else "") + body


def round_to(f, scale, mode):
    n = f * 10 ** scale
    q, r = divmod(n.numerator, n.denominator)
    if r == 0:
        return F(q, 10 ** scale)
    frac_part = F(r, n.denominator)
    neg = n < 0
    toward_zero = q + 1 if neg else q
    away = q if neg else q + 1
    half = frac_part if not neg else 1 - frac_part
    if mode == "away_from_zero":
        v = away
    elif mode == "toward_zero":
        v = toward_zero
    elif mode == "toward_positive":
        v = q + 1
    elif mode == "toward_negative":
        v = q
    elif mode == "half_away_from_zero":
        v = away if half >= F(1, 2) else toward_zero
    elif mode == "half_toward_zero":
        v = away if half > F(1, 2) else toward_zero
    else:
        v = away if half > F(1, 2) else toward_zero if half < F(1, 2) else (q if q % 2 == 0 else q + 1)
    return F(v, 10 ** scale)


# ── The scope by hand ──────────────────────────────────────────────────────────────────────────────────────────────
def scope(entities, boundary_index, chosen, scale=2, mode="half_even", tolerance=0.01, field=FIELD):
    edge = exact(entities[boundary_index])
    given = chosen is not None
    if chosen is None:
        chosen = [i for i, e in enumerate(entities)
                  if e["kind"] == "polygon" and e["layerId"] == "parsel" and i != boundary_index]
    parcels = []
    for i in chosen:
        e = entities[i]
        poly = exact(e)
        ins = inside_pieces(poly, edge)
        if not given and not ins:
            continue
        outs = outside_pieces(poly, edge)
        cin = sum(1 for p in ins if width(p) > tolerance)
        cout = sum(1 for p in outs if width(p) > tolerance)
        slivers = len(ins) + len(outs) - cin - cout
        kind = "outside" if cin == 0 else "whole" if cout == 0 else "partial"
        inside = sum((p[0] for p in ins), F(0))
        outside = sum((p[0] for p in outs), F(0))
        calc = area(poly)
        lim = limit(calc)
        text = (e["attrs"].get(field) or "").strip() or None
        diff = within = None
        if text is not None and plain(text):
            rec = F(text.replace(",", "."))
            diff = calc - rec
            within = abs(diff) <= lim
        basis = rest = None
        if cin == 0 and slivers == 0:
            status = "outside"
        elif slivers > 0:
            status = "sliver"
        elif text is None:
            status = "noRecord"
        elif not plain(text):
            status = "invalidRecord"
        elif within is False:
            status = "needsCorrection"
        elif kind == "whole":
            w = text.replace(",", ".")
            if places(w) > scale:
                status = "finerThanPolicy"
            else:
                status, basis = "registered", w
        else:
            w = text.replace(",", ".")
            b = round_to(inside, scale, mode)
            left = F(w) - b
            rest = written(left, max(places(w), scale))
            if left < 0:
                status = "outsideTooSmall"
            else:
                status, basis = "computed", written(b, scale)
        a = e["attrs"]
        ada, parsel = a.get("Ada", "").strip(), a.get("Parsel", "").strip()
        name = f"{ada}/{parsel}" if ada and parsel else (parsel if not ada else ada)
        p = {"uid": uid(i + 1), "parcel": name, "il": a.get("İl", ""), "ilce": a.get("İlçe", ""),
             "mahalle": a.get("Mahalle", ""), "ada": ada, "parsel": parsel, "kind": kind}
        if text is not None:
            p["recordedText"] = text
        p["calculated"] = float(calc)
        if diff is not None:
            p["difference"] = float(diff)
        p["limit"] = float(lim)
        if within is not None:
            p["within"] = within
        p.update({"inside": float(inside), "outside": float(outside), "pieces": cout, "slivers": slivers})
        if basis is not None:
            p["basis"] = basis
        if rest is not None:
            p["outsideRecorded"] = rest
        p["status"] = status
        p["_noIdentity"] = not ada or not parsel
        parcels.append(p)
    names = [p["parcel"] for p in parcels if not p["_noIdentity"]]
    for p in parcels:
        if not p["_noIdentity"] and names.count(p["parcel"]) > 1:
            p["duplicate"] = True
        if p.pop("_noIdentity"):
            p["noIdentity"] = True
    bases = [p["basis"] for p in parcels if "basis" in p]
    total = sum((F(b) for b in bases), F(0))
    most = max((places(b) for b in bases), default=0)
    b_area = area(edge)
    inside_all = sum(p["inside"] for p in parcels)
    return {"rule": RULE, "field": field, "scale": scale, "rounding": mode, "tolerance": tolerance,
            "boundaryArea": float(b_area), "inside": inside_all, "uncovered": float(b_area) - inside_all,
            "basis": written(total, most), "whole": sum(1 for p in parcels if p["kind"] == "whole"),
            "partial": sum(1 for p in parcels if p["kind"] == "partial"), "ready": len(bases),
            "review": sum(1 for p in parcels if p["status"] not in ("registered", "computed", "outside")),
            "parcels": parcels}


# ── The cases ──────────────────────────────────────────────────────────────────────────────────────────────────────
MAIN = [BOUNDARY] + PARCELS


def P_(n):
    """The entity index of parcel n (the boundary is the first)."""
    return n


def ok(name, entities=MAIN, chosen=None, op="execute", setup_=None, boundary=0, **kw):
    inp = {"boundary": uid(boundary + 1), "scale": kw.get("scale", 2), "rounding": kw.get("mode", "half_even")}
    if chosen is not None:
        inp["uids"] = [uid(i + 1) for i in chosen]
    if "tolerance" in kw:
        inp["tolerance"] = kw["tolerance"]
    if "field" in kw:
        inp["field"] = kw["field"]
    out = None if op == "validate" else scope(entities, boundary, chosen, **kw)
    case = {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": []},
                                     "expect": SAME, "within": W}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, inp, code, message, path, setup_=None):
    case = {"name": name, "steps": [{"op": "execute", "input": inp,
                                     "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                                     "expect": SAME}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def cases():
    with_line = MAIN + [LINE]
    line_uid = uid(len(with_line))
    twins = [BOUNDARY, area_entity(rect(0, 0, 30, 20), "parsel", ident(1, "600.00")),
             area_entity(rect(30, 0, 60, 20), "parsel", ident(1, "600.00"))]
    on_parcels = [dict(BOUNDARY, layerId="parsel")] + PARCELS[:3]
    base = {"boundary": uid(1), "scale": 2, "rounding": "half_even"}
    return [
        ok("karışık ada: tamamı ve kısmen giren, yanılma sınırı dışında, kaydı olmayan, okunmayan, şeritli, iki parçası "
           "dışarıda kalan, girmeyen kısmı hatayı taşımayan parseller"),
        ok("tamamı giren, yanılma sınırı içinde: tescilli yüzölçümü esas (virgüllü kayıt noktayla)", chosen=[P_(2)]),
        ok("kısmen giren: giren kısmın hesaplanan yüzölçümü esas, hata girmeyen kısımda", chosen=[P_(3)]),
        ok("yanılma sınırı dışında: düzeltilmiş yüzölçümü gerekir, esas yok", chosen=[P_(4)]),
        ok("tescilli yüzölçümü yok", chosen=[P_(5)]),
        ok("okunmayan kayıt", chosen=[P_(6)]),
        ok("sınırın dışındaki parsel verilince: dışarıda, esas yok", chosen=[P_(7)]),
        ok("sınır parselin kenarından 5 mm içeri geçiyor: şerit, esas yok", chosen=[P_(8)]),
        ok("tamamı giren parselin 4 mm'lik parçası dışarıda: şerit, esas yok", chosen=[P_(9)]),
        ok("tolerans 3 mm: 4 mm'lik parça sayılır, parsel kısmen giren olur", chosen=[P_(9)], tolerance=0.003),
        ok("iki parçası dışarıda kalan parsel: iki parça, hata ikisinde birlikte", chosen=[P_(10)]),
        ok("girmeyen kısım hatayı taşıyamıyor: esas yok, eksik söylenir", chosen=[P_(11)]),
        ok("sınıra kenarından değen komşu: içeride kısmı yok", chosen=[P_(12)]),
        ok("sıfırdan uzağa: 1000/3 m² → 333,34", chosen=[P_(3)], mode="away_from_zero"),
        ok("sıfır basamak: tescilli 600.00 politikanın basamağından ince, kısmen girenler tam sayıya", scale=0),
        ok("kayıt başka bir öznitelikte", chosen=[P_(2)], field="Kayıtlı alan"),
        ok("seçim sırası korunur", chosen=[P_(3), P_(1)]),
        ok("aynı Ada/Parsel iki parselde: ikisi de işaretlenir", twins, setup_=setup(twins)),
        ok("sınır parsel katmanında: kendisi listelenmez", on_parcels, setup_=setup(on_parcels)),
        ok("CBS projesinde de", setup_=setup(MAIN, "gis")),
        ok("parsel katmanı boş: liste boş, sınırın hepsi kaplanmamış", [BOUNDARY], setup_=setup([BOUNDARY])),
        ok("plan aynı sonucu verir", op="plan"),
        ok("doğrulama", op="validate"),
        refused("basamak 7", {**base, "scale": 7}, "invalid_input", "Basamak 7 en çok 6 olabilir.", "scale"),
        refused("bilinmeyen yuvarlama", {**base, "rounding": "yarim"}, "invalid_input",
                "“yarim” bir yuvarlama yönü değil (half_even, half_away_from_zero …).", "rounding"),
        refused("tolerans 0", {**base, "tolerance": 0.0}, "invalid_input",
                "Tolerans sonlu ve en az 0.000001 m olmalı (yazılan 0).", "tolerance"),
        refused("sınırın kimliği geçersiz", {**base, "boundary": "B1"}, "invalid_uid", "“B1” geçerli bir nesne kimliği değil.",
                "boundary"),
        refused("sınır çizimde yok", {**base, "boundary": uid(99)}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "boundary"),
        refused("sınır alan değil", {**base, "boundary": line_uid}, "not_an_area",
                f"“{line_uid}” bir alan değil; düzenleme sınırı kapalı bir alan olmalı.", "boundary",
                setup_=setup(with_line)),
        refused("parselin kimliği geçersiz", {**base, "uids": ["P1"]}, "invalid_uid", "“P1” geçerli bir nesne kimliği değil.",
                "uids[0]"),
        refused("verilen nesne alan değil", {**base, "uids": [uid(2), line_uid]}, "not_a_parcel",
                f"“{line_uid}” bir alan değil; düzenleme kapsamı yalnız alanları alır.", "uids[1]", setup_=setup(with_line)),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.readjustment.scope", "commandVersion": 1,
           "title": "Düzenleme kapsamı",
           "note": "ADR 2060. Bağımsız başvuru scripts/fixtures/readjustment_scope_cases.py: giren ve girmeyen kısımlar "
                   "kesirle kırpılarak, yanılma sınırı ADR 1400'ün çizelgesiyle, esas alan Yönetmelik m. 31'in kuralıyla, "
                   "yuvarlama elle; KentOS kodu yok.",
           "setup": setup(MAIN), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("Düzenleme kapsamı ortak durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
