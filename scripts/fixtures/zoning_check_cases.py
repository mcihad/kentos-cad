#!/usr/bin/env python3
"""The shared cases of `cad.zoning.check` (İmar planı denetimi; plan/map-cad.md MAP-09, docs/adr/1420, 2020, 2240), without
KentOS code.

    python3 scripts/fixtures/zoning_check_cases.py           # writes fixtures/commands/v1/cad.zoning.check.json
    python3 scripts/fixtures/zoning_check_cases.py --check   # writes nothing; compares

The rules, by hand (docs/adr/1420 §2), over the profile `catalog/v1/profiles/imar-plani.json` (its layers and value
lists are read from it, as the command reads them):
- a plan boundary's level empty or off `plan-seviye`: noLevel; its upper plan not among the plans: upperMissing; an
  upper plan not of a higher level (msp > cdp > nip > uip): levelOrder, both named;
- a decision's plan neither given with one plan nor among the plans: noPlan; its use empty: noUse, off the list:
  unknownUse; a use on which something is built (not park, yeşil, meydan, yol) without TAKS, KAKS or a height (kat
  adedi or Hmax), each neither given nor declared “planda verilmeyen”: missingValue (0 is a value); a value declared
  and given: declaredValued; KAKS > TAKS × kat adedi: kaksReview; a note number named but absent: noteMissing;
- a note for areas (Kapsam alan) no decision names: noteUnused;
- a plan whose upper plan is among the plans, its part outside the upper plan's boundary, counted as below:
  outsideUpper, both named (docs/adr/2020);
- with the road layer (Yollar), each road axis's end farther than the tolerance from every other road and from every
  plan boundary's edges: roadGap, its road named and the end said (Y, X to the millimetre), the first end before the
  last (docs/adr/2020); without the layer no road is checked;
- with the road layer, each road with a width (Genislik, a number above 0): its plan the lowest-level plan whose
  boundary holds the middle of its longest segment (the first of equals); its corridor half the width each side of
  each segment, ends flat (the roads here are axis-parallel; no decision comes near a turn's outer corner); each
  decision of that plan whose use is not yol, the pieces of its common part with the corridor counted as below:
  roadWidth, the road and the decision named (docs/adr/2240); each parcel (an area on the parcel layer `parsel`, no
  plan's), the same: roadParcel, the road and the parcel (Ada/Parsel) named (docs/adr/2640);
- plan by plan, over its boundary and the decisions that name it (a plan without decisions skipped): two decisions'
  common part, a boundary's part no decision covers, a decision's part outside the boundary, each piece counted when
  its mean width 2A/P passes the tolerance (docs/adr/0202 §2), the area the sum of the counted pieces. The areas here
  are axis-aligned rectangles, so the pieces are exact by coordinate compression.
Findings by kind in the list's order, then by their first object's place in the drawing.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.zoning.check.json"
PROFILE = json.loads((ROOT / "catalog/v1/profiles/imar-plani.json").read_text("utf-8"))
X0, Y0 = 487000.0, 4420000.0
KINDS = ["missingLayer", "noLevel", "upperMissing", "levelOrder", "outsideUpper", "noPlan", "noUse", "unknownUse",
         "missingValue", "declaredValued", "kaksReview", "overlap", "uncovered", "outside", "roadGap", "roadWidth",
         "roadParcel", "noteMissing", "noteUnused", "symbolMismatch", "draftStale"]
OPEN = {"park", "yesil", "meydan", "yol"}
RANK = ["msp", "cdp", "nip", "uip"]
LAYER_IDS = {"plan-siniri": "plan", "karar-alani": "karar", "plan-notu": "notlar", "yol-aksi": "yollar"}
ROADS = ("plan-siniri", "karar-alani", "plan-notu", "yol-aksi")
CLASS_OF = {"plan": "plan-siniri", "karar": "karar-alani", "notlar": "plan-notu", "yollar": "yol-aksi"}


def uid(n):
    return f"0192f5c0-1420-7000-8000-{n:012x}"


# ADR 4640: the symbols by ADR 4240's rule, from the library on its own (scripts/fixtures/zoning_symbols_cases.py
# derives them by the same written name; KentOS code none), and the library's names.
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import zoning_symbols_cases as zs  # noqa: E402
SYMBOLS = zs.KNOWN
# ADR 5040: a level's area symbols, among which a kept choice must be to be read.
CHOICES = {level: {i["id"] for i in zs.fills(level)} for level in zs.BRANCHES}
NAMES = {i["id"]: i["name"] for i in zs.LIBRARY if i.get("kind") == "symbol"}


def domain(d):
    return {v["code"]: v["label"] for v in next(x for x in PROFILE["domains"] if x["id"] == d)["values"]}


def label(d, code):
    return domain(d).get(code, code)


def cls(id_):
    return next(c for c in PROFILE["classes"] if c["id"] == id_)


def node(id_, name, children=None):
    return {"id": id_, "name": name, "type": "group" if children is not None else "layer", "visible": True,
            "locked": False, "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
            "children": children or []}


def layers(classes=("plan-siniri", "karar-alani", "plan-notu")):
    kids = [node(LAYER_IDS[c], cls(c)["layer"][1]) for c in classes]
    return [node("0", "0"), node("imar", cls("plan-siniri")["layer"][0], kids)]


def rect_pts(r):
    x0, y0, x1, y1 = r
    return [{"x": X0 + x0, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y1},
            {"x": X0 + x0, "y": Y0 + y1}]


def plan(kimlik, r, seviye="uip", ust=None, **more):
    attrs = {"PlanKimlik": kimlik, "Seviye": seviye, "Durum": "yururlukte", **more}
    if ust:
        attrs["UstPlan"] = ust
    return ("plan", r, attrs)


def karar(kimlik, r, kullanim, **attrs):
    return ("karar", r, {"KararKimlik": kimlik, "Kullanim": kullanim, **attrs})


def note(no, at, kapsam="alan", metin="Plan notu metni."):
    return ("notlar", at, {"NotNo": no, "Metin": metin, "Kapsam": kapsam})


def road(kimlik, pts, genislik=None):
    attrs = {"YolKimlik": kimlik, "Tur": "tasit"}
    if genislik is not None:
        attrs["Genislik"] = genislik
    return ("yollar", pts, attrs)


def parsel(ada, no, r):
    return ("parsel", r, {"Ada": ada, "Parsel": no})


def setup(objects, classes=("plan-siniri", "karar-alani", "plan-notu"), zoning=None):
    ents = []
    for i, (layer, geo, attrs) in enumerate(objects, start=1):
        a = {k: v for k, v in attrs.items() if v is not None and k != "@symbol"}
        if layer == "notlar":
            ents.append({"kind": "point", "id": i, "layerId": layer, "attrs": a,
                         "p": {"x": X0 + geo[0], "y": Y0 + geo[1]}})
        elif layer == "yollar":
            ents.append({"kind": "polyline", "id": i, "layerId": layer, "attrs": a,
                         "pts": [{"x": X0 + x, "y": Y0 + y} for x, y in geo]})
        else:
            ents.append({"kind": "polygon", "id": i, "layerId": layer, "attrs": a, "pts": rect_pts(geo)})
            if attrs.get("@symbol"):
                ents[-1]["symbol"] = attrs["@symbol"]
    return {"format": "kentos.document", "version": 2, "name": "İmar planı denetimi",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map",
                         **({"zoningPlan": zoning} if zoning else {})},
            "origin": {"x": X0, "y": Y0}, "layers": layers(classes) + ([node("parsel", "Parsel")] if any(
                o[0] == "parsel" for o in objects) else []), "activeLayer": "karar" if
            "karar-alani" in classes else "plan", "entities": ents,
            "uids": [uid(i + 1) for i in range(len(ents))], "styles": {"items": [], "categories": []}}


# ── Pieces of axis-aligned rectangles ───────────────────────────────────────────────────────────────────────────


def pieces(region_in, region_out):
    """The connected pieces of (union of `region_in`) minus (union of `region_out`): each its area and perimeter."""
    xs = sorted({v for r in region_in + region_out for v in (r[0], r[2])})
    ys = sorted({v for r in region_in + region_out for v in (r[1], r[3])})
    inside = lambda rs, x, y: any(r[0] <= x <= r[2] and r[1] <= y <= r[3] for r in rs)  # noqa: E731
    cells = {}
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            cx, cy = (xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2
            cells[(i, j)] = inside(region_in, cx, cy) and not inside(region_out, cx, cy)
    seen, out = set(), []
    for start, on in cells.items():
        if not on or start in seen:
            continue
        stack, comp = [start], []
        seen.add(start)
        while stack:
            c = stack.pop()
            comp.append(c)
            for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                n = (c[0] + d[0], c[1] + d[1])
                if cells.get(n) and n not in seen:
                    seen.add(n)
                    stack.append(n)
        a = sum((xs[i + 1] - xs[i]) * (ys[j + 1] - ys[j]) for i, j in comp)
        p = 0.0
        members = set(comp)
        for i, j in comp:
            w, h = xs[i + 1] - xs[i], ys[j + 1] - ys[j]
            p += (h if (i - 1, j) not in members else 0) + (h if (i + 1, j) not in members else 0)
            p += (w if (i, j - 1) not in members else 0) + (w if (i, j + 1) not in members else 0)
        out.append((a, p))
    return out


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def outlines(region_in, t):
    """docs/adr/3440: the counted pieces of the union of `region_in` (as `pieces` and `counted` find them) as regions:
    each piece's boundary traced over its cells (each cell side between a piece's cell and another, its interior on the
    left), vertices only where the way turns; the outer ring counter-clockwise, the holes clockwise, each from its
    lowest vertex (the westmost of the lowest); the regions by their first vertices (north, then east)."""
    xs = sorted({v for r in region_in for v in (r[0], r[2])})
    ys = sorted({v for r in region_in for v in (r[1], r[3])})
    inside = lambda x, y: any(r[0] <= x <= r[2] and r[1] <= y <= r[3] for r in region_in)  # noqa: E731
    on = {(i, j) for i in range(len(xs) - 1) for j in range(len(ys) - 1)
          if inside((xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2)}
    seen, out = set(), []
    for start in sorted(on):
        if start in seen:
            continue
        stack, comp = [start], set()
        seen.add(start)
        while stack:
            c = stack.pop()
            comp.add(c)
            for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                n = (c[0] + d[0], c[1] + d[1])
                if n in on and n not in seen:
                    seen.add(n)
                    stack.append(n)
        a = sum((xs[i + 1] - xs[i]) * (ys[j + 1] - ys[j]) for i, j in comp)
        per = 0.0
        sides = {}
        for i, j in comp:
            w, h = xs[i + 1] - xs[i], ys[j + 1] - ys[j]
            if (i, j - 1) not in comp:
                per += w
                sides[(i, j)] = (i + 1, j)
            if (i + 1, j) not in comp:
                per += h
                sides[(i + 1, j)] = (i + 1, j + 1)
            if (i, j + 1) not in comp:
                per += w
                sides[(i + 1, j + 1)] = (i, j + 1)
            if (i - 1, j) not in comp:
                per += h
                sides[(i, j + 1)] = (i, j)
        if not (per > 0 and 2 * a / per > t):
            continue
        rings = []
        while sides:
            first = min(sides)
            loop, at = [first], sides.pop(first)
            while at != first:
                loop.append(at)
                at = sides.pop(at)
            # Only the turns: a vertex between two sides of one direction goes.
            n = len(loop)
            turns = [loop[k] for k in range(n)
                     if (loop[k][0] - loop[k - 1][0], loop[k][1] - loop[k - 1][1]) != (0, 0) and
                     ((loop[k][0] - loop[k - 1][0]) * (loop[(k + 1) % n][1] - loop[k][1]) -
                      (loop[k][1] - loop[k - 1][1]) * (loop[(k + 1) % n][0] - loop[k][0])) != 0]
            pts = [(xs[i], ys[j]) for i, j in turns]
            m = len(pts)
            area2 = sum(pts[k][0] * pts[(k + 1) % m][1] - pts[(k + 1) % m][0] * pts[k][1] for k in range(m))
            low = min(range(m), key=lambda k: (pts[k][1], pts[k][0]))
            rings.append((area2 > 0, pts[low:] + pts[:low]))
        outer = [r for ccw, r in rings if ccw]
        holes = sorted((r for ccw, r in rings if not ccw), key=lambda r: (r[0][1], r[0][0]))
        assert len(outer) == 1, "one outer ring a piece"
        region = {"outer": {"pts": [P(x, y) for x, y in outer[0]]}}
        if holes:
            region["holes"] = [{"pts": [P(x, y) for x, y in h]} for h in holes]
        out.append(((outer[0][0][1], outer[0][0][0]), region))
    out.sort(key=lambda r: r[0])
    return [r for _, r in out]


def counted(ps, t):
    kept = [a for a, p in ps if p > 0 and 2 * a / p > t]
    return sum(kept) if kept else None


def intersect(a, b):
    r = (max(a[0], b[0]), max(a[1], b[1]), min(a[2], b[2]), min(a[3], b[3]))
    return [r] if r[0] < r[2] and r[1] < r[3] else []


# ── The check ────────────────────────────────────────────────────────────────────────────────────────────────────


def check(objects, classes=("plan-siniri", "karar-alani", "plan-notu"), tolerance=0.01, zoning=None):
    found = []
    zoning = zoning or {}

    def say(kind, ats, area, message, regions=()):
        found.append((KINDS.index(kind), min(ats) if ats else -1,
                      {"kind": kind, "uids": [uid(a + 1) for a in sorted(ats)], **({"area": area} if area is not None
                                                                                   else {}), "message": message,
                       **({"regions": list(regions)} if regions else {})}))

    for c in ("plan-siniri", "karar-alani", "plan-notu"):
        if c not in classes:
            say("missingLayer", [], None,
                f"“{' › '.join(cls(c)['layer'])}” katmanı yok; imar-plani profilini uygulayın (cad.profiles.apply).")
    g = lambda o, k: (o[2].get(k) or "").strip()  # noqa: E731
    items = list(enumerate(objects))
    plans = [(i, o) for i, o in items if o[0] == "plan" and "plan-siniri" in classes]
    decisions = [(i, o) for i, o in items if o[0] == "karar" and "karar-alani" in classes]
    notes = [(i, o) for i, o in items if o[0] == "notlar" and "plan-notu" in classes]
    parcels = [(i, o) for i, o in items if o[0] == "parsel"]
    ids = [g(o, "PlanKimlik") for _, o in plans]
    for i, o in plans:
        lv = g(o, "Seviye")
        if lv not in RANK:
            say("noLevel", [i], None, "Plan sınırının kademesi yok ya da listede değil (plan-seviye).")
        up = g(o, "UstPlan")
        if not up:
            continue
        upper = [(j, q) for j, q in plans if g(q, "PlanKimlik") == up]
        if not upper:
            say("upperMissing", [i], None, f"Üst plan “{up}” plan sınırlarında yok.")
            continue
        j, q = upper[0]
        ul = g(q, "Seviye")
        if ul in RANK and lv in RANK and RANK.index(ul) >= RANK.index(lv):
            say("levelOrder", [i, j], None, f"Üst plan “{up}” ({label('plan-seviye', ul)}) bu planın "
                                            f"({label('plan-seviye', lv)}) üstünde bir kademe değil.")
        area = counted(pieces([o[1]], [q[1]]), tolerance)
        if area is not None:
            say("outsideUpper", [i, j], area, f"Plan sınırı, üst plan “{up}” sınırının dışına taşıyor.")
    numbers = {g(o, "NotNo") for _, o in notes if g(o, "NotNo")}
    named = set()
    uses = domain("plan-kullanim")
    for i, o in decisions:
        pl = g(o, "Plan")
        if not pl and len(plans) != 1:
            say("noPlan", [i], None, f"Karar alanının planı yazılmamış; çizimde {len(plans)} plan sınırı var.")
        elif pl and pl not in ids:
            say("noPlan", [i], None, f"Karar alanının planı “{pl}” plan sınırlarında yok.")
        u = g(o, "Kullanim")
        if not u:
            say("noUse", [i], None, "Karar alanının kullanımı yok; plan kararı yazılmadan alan bir karar değildir.")
        elif u not in uses:
            say("unknownUse", [i], None, f"“{u}” kullanım listesinde yok (plan-kullanim).")
        declared = [s.strip() for s in g(o, "Belirtilmeyen").split(";") if s.strip()]
        has = lambda k: bool(g(o, k))  # noqa: E731
        if u and u not in OPEN:
            miss = [label("plan-deger", k) for k in ("TAKS", "KAKS") if not has(k) and k not in declared]
            if not has("KatAdedi") and not has("Hmax") and "KatAdedi" not in declared and "Hmax" not in declared:
                miss.append("Kat adedi / Hmax")
            if miss:
                say("missingValue", [i], None, f"{label('plan-kullanim', u)}: {', '.join(miss)} verilmemiş; değeri "
                                               "yazın ya da Planda verilmeyen'e ekleyin.")
        both = [label("plan-deger", d) for d in declared
                if (has("OnCekme") or has("YanCekme") or has("ArkaCekme") if d == "Cekme" else has(d))]
        if both:
            say("declaredValued", [i], None, f"{', '.join(both)} hem verilmiş hem Planda verilmeyen'de.")
        tk, kk, fl = g(o, "TAKS"), g(o, "KAKS"), g(o, "KatAdedi")
        try:
            t_, k_, n_ = (float(v.replace(",", ".")) for v in (tk, kk, fl))
            prod = t_ * n_
            if k_ > prod + 1e-9:
                say("kaksReview", [i], None, f"KAKS {kk} > TAKS × kat adedi ({tk} × {fl} = {prod:.2f}); kapalı "
                                             "çıkmalar emsale girebilir, gözden geçirin.")
        except ValueError:
            pass
        wanted = [w for w in g(o, "PlanNotlari").replace(",", ";").replace(" ", ";").split(";") if w]
        absent = [w for w in wanted if w not in numbers]
        if absent:
            say("noteMissing", [i], None, f"Plan notu {', '.join(absent)} yok.")
        named.update(wanted)
    for i, o in notes:
        no = g(o, "NotNo")
        if g(o, "Kapsam") == "alan" and no and no not in named:
            say("noteUnused", [i], None, f"Plan notu {no} karar alanları içindir ama hiçbir karar alanı onu anmıyor.")
    # Plan by plan.
    for (pi, po), pid in zip(plans, ids):
        own = []
        for i, o in decisions:
            pl = g(o, "Plan")
            r = (ids[0] if len(plans) == 1 else None) if not pl else (pl if pl in ids else None)
            if r == pid and pid:
                own.append((i, o))
        if not own:
            continue
        for a in range(len(own)):
            for b in range(a + 1, len(own)):
                (i, o), (j, q) = own[a], own[b]
                area = counted(pieces(intersect(o[1], q[1]), []), tolerance)
                if area is not None:
                    words = [label("plan-kullanim", g(x, "Kullanim")) for _, x in sorted([(i, o), (j, q)])]
                    say("overlap", [i, j], area, f"İki karar alanı aynı yerde: {' ve '.join(words)} (karar çelişkisi).")
        area = counted(pieces([po[1]], [o[1] for _, o in own]), tolerance)
        if area is not None:
            say("uncovered", [pi], area, "Plan sınırının bir kısmında karar yok.")
        for i, o in own:
            area = counted(pieces([o[1]], [po[1]]), tolerance)
            if area is not None:
                say("outside", [i], area, "Karar alanı plan sınırının dışına taşıyor.")
    # The roads: each end against the other roads' segments and the plan boundaries' edges.
    roads = [(i, o) for i, o in items if o[0] == "yollar" and "yol-aksi" in classes]

    def to_segment(p, a, b):
        dx, dy = b[0] - a[0], b[1] - a[1]
        l2 = dx * dx + dy * dy
        t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
        return ((p[0] - a[0] - t * dx) ** 2 + (p[1] - a[1] - t * dy) ** 2) ** 0.5

    def rect_edges(r):
        x0, y0, x1, y1 = r
        c = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        return [(c[k], c[(k + 1) % 4]) for k in range(4)]

    for i, o in roads:
        for end in (o[1][0], o[1][-1]):
            near = any(to_segment(end, a, b) <= tolerance for j, q in roads if j != i for a, b in zip(q[1], q[1][1:]))
            near = near or any(to_segment(end, a, b) <= tolerance for _, q in plans for a, b in rect_edges(q[1]))
            if not near:
                say("roadGap", [i], None, f"“{g(o, 'YolKimlik')}” yol aksının ucu (Y {X0 + end[0]:.3f}, X {Y0 + end[1]:.3f}) "
                                          "başka bir yola ya da plan sınırına bağlanmıyor (kopuk yol).")
    # The roads' widths (docs/adr/2240).
    for i, o in roads:
        written = g(o, "Genislik")
        try:
            width = float(written.replace(",", "."))
        except ValueError:
            continue
        if not width > 0:
            continue
        segs = list(zip(o[1], o[1][1:]))
        chord = lambda s: ((s[1][0] - s[0][0]) ** 2 + (s[1][1] - s[0][1]) ** 2) ** 0.5  # noqa: E731
        longest = segs[0]
        for s in segs[1:]:
            if chord(s) > chord(longest):
                longest = s
        mx, my = (longest[0][0] + longest[1][0]) / 2, (longest[0][1] + longest[1][1]) / 2
        home = None
        for (pi, po), pid in zip(plans, ids):
            r = po[1]
            if not (r[0] <= mx <= r[2] and r[1] <= my <= r[3]):
                continue
            rk = RANK.index(g(po, "Seviye")) if g(po, "Seviye") in RANK else -1
            if home is None or rk > home[0]:
                home = (rk, pid)
        if home is None:
            continue
        h = width / 2
        assert all(ax == bx or ay == by for (ax, ay), (bx, by) in segs), "axis-parallel roads only"
        corridor = [((min(ax, bx), min(ay, by) - h, max(ax, bx), max(ay, by) + h) if ay == by else
                     (min(ax, bx) - h, min(ay, by), max(ax, bx) + h, max(ay, by))) for (ax, ay), (bx, by) in segs]
        turns = [b for (_, b) in segs[:-1]]
        for j, q in decisions:
            pl = g(q, "Plan")
            r = (ids[0] if len(plans) == 1 else None) if not pl else (pl if pl in ids else None)
            if r != home[1] or g(q, "Kullanim") == "yol":
                continue
            for tx, ty in turns:
                x0, y0, x1, y1 = q[1]
                near = ((max(x0 - tx, 0, tx - x1)) ** 2 + (max(y0 - ty, 0, ty - y1)) ** 2) ** 0.5
                assert near >= h, "no decision within the turn's round corner"
            inside = [c for cr in corridor for c in intersect(q[1], cr)]
            area = counted(pieces(inside, []), tolerance)
            if area is not None:
                say("roadWidth", [i, j], area, f"“{g(o, 'YolKimlik')}” yolunun {written} m genişliği içine "
                                               f"{label('plan-kullanim', g(q, 'Kullanim'))} kararı “{g(q, 'KararKimlik')}” giriyor.",
                    outlines(inside, tolerance))
        # docs/adr/2640: the parcels inside the road's width, whatever their plan.
        for j, q in parcels:
            for tx, ty in turns:
                x0, y0, x1, y1 = q[1]
                near = ((max(x0 - tx, 0, tx - x1)) ** 2 + (max(y0 - ty, 0, ty - y1)) ** 2) ** 0.5
                assert near >= h, "no parcel within the turn's round corner"
            inside = [c for cr in corridor for c in intersect(q[1], cr)]
            area = counted(pieces(inside, []), tolerance)
            if area is not None:
                name = "/".join(v for v in (g(q, "Ada"), g(q, "Parsel")) if v) or "kimliksiz"
                say("roadParcel", [i, j], area, f"“{g(o, 'YolKimlik')}” yolunun {written} m genişliği içine “{name}” "
                                                "parseli giriyor.", outlines(inside, tolerance))
    # ADR 4640: a decision area whose library symbol is not the one KentOS's table gives its use (ADR 4240's rule: its
    # Plan names a plan boundary, else the one plan boundary there is; the level that plan's), judged only when the
    # area has a symbol and the table maps its use. ADR 5040: the project's kept choice goes before the table, when it
    # is one of the level's area symbols and its use is on the list.
    kept = {(k["level"], k["use"]): k["symbol"] for k in zoning.get("symbols", [])
            if k["symbol"] in CHOICES.get(k["level"], set()) and k["use"] in domain("plan-kullanim")}
    for i, o in decisions:
        symbol = (o[2].get("@symbol") or "").strip()
        if not symbol:
            continue
        written = g(o, "Plan")
        named = [q for _, q in plans if g(q, "PlanKimlik") == written] if written else (
            [q for _, q in plans] if len(plans) == 1 else [])
        if not named:
            continue
        level, use = g(named[0], "Seviye"), g(o, "Kullanim")
        if not level or not use or use not in domain("plan-kullanim"):
            continue
        want = kept.get((level, use)) or SYMBOLS.get(level, {}).get(use)
        if want is None or want == symbol:
            continue
        say("symbolMismatch", [i], None, f"Karar alanının sembolü “{NAMES.get(symbol, symbol)}”, kararı "
                                         f"“{label('plan-kullanim', use)}” ({label('plan-seviye', level)}) "
                                         f"“{NAMES.get(want, want)}” sembolünü verir: sembol kararı göstermiyor. "
                                         "Sembolleri karardan ver ile düzeltin.")
    # ADR 5040: a plan change draft whose plan is gone, whose decision area is gone, or whose area's field is no longer
    # the value the draft read; each draft in its order, its changes in theirs.
    plan_ids = [g(q, "PlanKimlik") for _, q in plans] if "karar-alani" in classes else []
    by_uid = {uid(i + 1): (i, o) for i, o in items}
    for d in zoning.get("drafts", []):
        if d["plan"] not in plan_ids:
            say("draftStale", [], None, f"“{d['id']}” plan değişikliği taslağının planı “{d['plan']}” çizimde yok; "
                                        "taslağı silin ya da planı geri getirin.")
        for c in d["changes"]:
            if c["area"] not in by_uid:
                say("draftStale", [], None, f"“{d['id']}” plan değişikliği taslağının değiştirdiği “{c['area']}” karar "
                                            "alanı çizimde yok; taslağı yeniden kaydedin ya da silin.")
                continue
            i, o = by_uid[c["area"]]
            now = g(o, c["field"])
            if now != c.get("before", ""):
                say("draftStale", [i] if o[0] == "karar" else [], None,
                    f"“{d['id']}” plan değişikliği taslağı eskidi: karar alanının {c['field']} değeri taslak yazılırken "
                    f"“{c.get('before', '')}”, şimdi “{now}”. Taslağı yeniden kaydedin.")
    found.sort(key=lambda f: (f[0], f[1]))
    findings = [f[2] for f in found]
    counts = {}
    for f in findings:
        counts[f["kind"]] = counts.get(f["kind"], 0) + 1
    return {"profile": "imar-plani", "tolerance": tolerance, "plans": len(plans), "decisions": len(decisions),
            "notes": len(notes), "findings": findings, "counts": dict(sorted(counts.items()))}


# ── The drawings ─────────────────────────────────────────────────────────────────────────────────────────────────

GOOD = [
    plan("UIP-1", (0, 0, 100, 50), ust="NIP-1"),
    plan("NIP-1", (-20, -20, 140, 80), seviye="nip"),
    karar("K1", (0, 0, 40, 50), "konut", Plan="UIP-1", Nizam="A", TAKS="0.40", KAKS="1.20", KatAdedi="3",
          PlanNotlari="2"),
    karar("K2", (40, 0, 70, 50), "ticaret-konut", Plan="UIP-1", Nizam="BL", TAKS="0.35", KAKS="1.75", KatAdedi="5"),
    karar("K3", (70, 0, 100, 30), "park", Plan="UIP-1"),
    karar("K4", (70, 30, 100, 50), "egitim", Plan="UIP-1", Belirtilmeyen="TAKS", KAKS="1.00", Hmax="15.50"),
    note("1", (10, 60), kapsam="genel"),
    note("2", (20, 60)),
]

MESSY = [
    plan("UIP-1", (0, 0, 100, 50), ust="NIP-1"),
    plan("NIP-1", (-20, -20, 140, 80), seviye="nip"),
    plan("UIP-2", (500, 0, 510, 10), seviye="", ust="CDP-1"),
    plan("UIP-3", (600, 0, 610, 10), ust="UIP-1"),
    karar("K1", (0, 0, 40, 50), "konut", Plan="UIP-1", Nizam="A", TAKS="0.40", KAKS="1.20", KatAdedi="3",
          PlanNotlari="1;2"),
    karar("K2", (40, 0, 70, 50), "ticaret-konut", Plan="UIP-1", Nizam="BL", TAKS="0.35", KAKS="2.50", KatAdedi="5",
          PlanNotlari="3"),
    karar("K3", (70, 0, 100, 30), "park", Plan="UIP-1"),
    karar("K4", (70, 30, 95, 50), "egitim", Plan="UIP-1", Belirtilmeyen="TAKS", KAKS="1.00"),
    karar("K5", (35, 40, 45, 60), "konut", Plan="UIP-1", TAKS="0,30", KAKS="0.90", KatAdedi="3",
          Belirtilmeyen="KAKS"),
    karar("K6", (300, 0, 310, 10), "", Plan="UIP-9"),
    karar("K7", (320, 0, 330, 10), "otel", TAKS="0.30", KAKS="0.60", Hmax="9.50"),
    note("1", (10, 60), kapsam="genel"),
    note("2", (20, 60)),
    note("4", (30, 60)),
]


# docs/adr/2240: a plan of four blocks between two roads whose areas are decisions of use yol; its nazım plan's own
# decision covers it all (not the uygulama plan's road's business).
ROADY = [
    plan("UIP-R", (0, 0, 100, 60), ust="NIP-R"),
    plan("NIP-R", (-20, -20, 140, 80), seviye="nip"),
    karar("N1", (-20, -20, 140, 80), "konut", Plan="NIP-R", TAKS="0.40", KAKS="1.20", KatAdedi="3"),
    karar("A1", (0, 0, 45, 25), "konut", Plan="UIP-R", TAKS="0.40", KAKS="1.20", KatAdedi="3"),
    karar("A2", (55, 0, 100, 25), "konut", Plan="UIP-R", TAKS="0.40", KAKS="1.20", KatAdedi="3"),
    karar("A3", (0, 35, 45, 60), "ticaret", Plan="UIP-R", TAKS="0.50", KAKS="2.00", KatAdedi="4"),
    karar("A4", (55, 35, 100, 60), "park", Plan="UIP-R"),
    karar("Y-A", (45, 0, 55, 60), "yol", Plan="UIP-R"),
    karar("Y-B", (0, 25, 45, 35), "yol", Plan="UIP-R"),
    karar("Y-C", (55, 25, 100, 35), "yol", Plan="UIP-R"),
]


def roady(w1="10", w2="10"):
    return ROADY + [road("Y1", [(0, 30), (100, 30)], w1), road("Y2", [(50, 0), (50, 60)], w2)]


# docs/adr/3440: a U-shaped road 12 m wide whose two arms enter the housing decision K1 in two places; its right part
# is the road decision K2, so no decision comes near the turns.
U_ROAD = [
    plan("UIP-U", (0, 0, 120, 100)),
    karar("K1", (0, 0, 50, 100), "konut", TAKS="0.40", KAKS="1.20", KatAdedi="3"),
    karar("K2", (50, 0, 120, 100), "yol"),
    road("Y-U", [(0, 30), (100, 30), (100, 70), (0, 70)], "12"),
]


# docs/adr/2640: three parcels on the parcel layer, two in block A1 and one in A4, up to the planned roads' edges.
PARCELS = [parsel("101", "1", (0, 0, 20, 25)), parsel("101", "2", (20, 0, 45, 25)), parsel("102", "1", (55, 35, 100, 60))]


# ADR 4640: decision areas carrying library symbols: a trade area drawn with the park's symbol, a park with its own, a
# housing area (no symbol in the table) with the park's, a trade area without a symbol.
PARK, TRADE = SYMBOLS["uip"]["park"], SYMBOLS["uip"]["ticaret"]
SYMBOLED = [
    plan("UIP-S", (0, 0, 100, 50)),
    karar("S1", (0, 0, 25, 50), "ticaret", Plan="UIP-S", Nizam="BL", TAKS="0.35", KAKS="1.75", KatAdedi="5",
          **{"@symbol": PARK}),
    karar("S2", (25, 0, 50, 50), "park", Plan="UIP-S", **{"@symbol": PARK}),
    karar("S3", (50, 0, 75, 50), "konut", Plan="UIP-S", Nizam="A", TAKS="0.40", KAKS="1.20", KatAdedi="3",
          **{"@symbol": PARK}),
    karar("S4", (75, 0, 100, 50), "ticaret", Plan="UIP-S", Nizam="BL", TAKS="0.35", KAKS="1.75", KatAdedi="5"),
]


# docs/adr/5040: kept choices and plan change drafts over GOOD (K1 is object 3, K2 4, K3 5).
GELISME, ARBORETUM = "mpyy.uip.konut.gelisme-konut-alani", "mpyy.uip.acik-yesil.arboretum-botanik-parki"
DRAFT_OK = {"id": "T-1", "plan": "UIP-1", "revision": "0", "reason": "Gerekçe.",
            "changes": [{"area": uid(5), "field": "Kullanim", "before": "park", "after": "ticaret"},
                        {"area": uid(3), "field": "KAKS", "before": "1.20", "after": "1.50"}]}
DRAFT_STALE = {"id": "T-1", "plan": "UIP-1", "revision": "0", "reason": "Gerekçe.",
               "changes": [{"area": uid(5), "field": "Kullanim", "before": "park", "after": "ticaret"},
                           {"area": uid(3), "field": "KAKS", "before": "1.00", "after": "1.50"},
                           {"area": uid(99), "field": "Hmax", "before": "", "after": "9.50"}]}
DRAFT_NO_PLAN = {"id": "T-2", "plan": "UIP-9", "revision": "0", "reason": "Gerekçe.",
                 "changes": [{"area": uid(4), "field": "KatAdedi", "before": "5", "after": "6"}]}


def with_(objects, index, **attrs):
    out = copy.deepcopy(objects)
    out[index][2].update(attrs)
    for k in [k for k, v in out[index][2].items() if v is None]:
        del out[index][2][k]
    return out


def geo(objects, index, r):
    out = copy.deepcopy(objects)
    out[index] = (out[index][0], r, out[index][2])
    return out


SAME = {"revision": "same", "canUndo": False}


def case(name, objects, op="plan", classes=("plan-siniri", "karar-alani", "plan-notu"), tolerance=None, expect=None,
         zoning=None):
    kept = [o for o in objects if o[0] == "parsel" or CLASS_OF[o[0]] in classes]
    out = check(kept, classes, 0.01 if tolerance is None else tolerance, zoning)
    if expect is not None:
        assert [f["kind"] for f in out["findings"]] == expect, (name, [f["kind"] for f in out["findings"]])
    input_ = {} if tolerance is None else {"tolerance": tolerance}
    return {"name": name, "setup": setup(kept, classes, zoning), "steps": [{"op": op, "input": input_, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}


def refused(name, tolerance, text):
    return {"name": name, "steps": [{"op": "execute", "input": {"tolerance": tolerance}, "result": {
        "status": "failed", "error": {"code": "invalid_input", "message":
                                      f"Tolerans sonlu ve en az 0.000001 m olmalı (yazılan {text}).",
                                      "path": "tolerance"}}, "expect": SAME}]}


def cases():
    return [
        case("U biçimli yol karara iki yerden girer: tek bulgu, iki bölge, alttaki önce", U_ROAD, classes=ROADS,
             expect=["roadWidth"]),
        case("temiz plan: bulgu yok", GOOD, expect=[]),
        case("karışık plan: her bulgu türü, türün sırasıyla", MESSY,
             expect=["noLevel", "upperMissing", "levelOrder", "outsideUpper", "noPlan", "noPlan", "noUse", "unknownUse",
                     "missingValue", "declaredValued", "kaksReview", "overlap", "overlap", "uncovered", "outside",
                     "noteMissing", "noteUnused"]),
        case("uygula: salt okunur", MESSY, op="execute"),
        case("doğrulama", MESSY, op="validate"),
        case("geniş tolerans dar parçaları saymaz", MESSY, tolerance=6.0,
             expect=["noLevel", "upperMissing", "levelOrder", "noPlan", "noPlan", "noUse", "unknownUse",
                     "missingValue", "declaredValued", "kaksReview", "noteMissing", "noteUnused"]),
        case("profil katmanları yoksa söylenir", GOOD, classes=("plan-siniri",), expect=["missingLayer",
                                                                                     "missingLayer"]),
        case("kullanımı olmayan karar", with_(GOOD, 4, Kullanim=None), expect=["noUse"]),
        case("listede olmayan kullanım: üstüne yapılaşan sayılır", with_(GOOD, 4, Kullanim="otel"),
             expect=["unknownUse", "missingValue"]),
        case("TAKS ne verilmiş ne planda verilmeyen", with_(GOOD, 2, TAKS=None), expect=["missingValue"]),
        case("TAKS 0 bir değerdir, boş değil", with_(GOOD, 2, TAKS="0", KAKS="0", KatAdedi="0"), expect=[]),
        case("Hmax verilince kat adedi istenmez", with_(GOOD, 2, KatAdedi=None, Hmax="9.50"), expect=[]),
        case("park değer istemez", with_(GOOD, 4, Kullanim="park", KAKS=None, Hmax=None, Belirtilmeyen=None),
             expect=[]),
        case("çekme hem verilmiş hem planda verilmeyen", with_(GOOD, 3, OnCekme="5", Belirtilmeyen="Cekme"),
             expect=["declaredValued"]),
        case("KAKS, TAKS × kat adedini aşıyor: gözden geçir", with_(GOOD, 3, KAKS="2.50"), expect=["kaksReview"]),
        case("KAKS, TAKS × kat adedine eşit: bulgu yok", with_(GOOD, 3, KAKS="1.75"), expect=[]),
        case("iki karar aynı yerde: karar çelişkisi", geo(GOOD, 4, (60, 0, 100, 30)), expect=["overlap"]),
        case("plan sınırında kararsız kısım", geo(GOOD, 5, (70, 30, 90, 50)), expect=["uncovered"]),
        case("karar plan sınırının dışına taşıyor", geo(GOOD, 5, (70, 30, 100, 60)), expect=["outside"]),
        case("iki planda karar alanının planı yazılmamış: planına sayılmaz", with_(GOOD, 2, Plan=None),
             expect=["noPlan", "uncovered"]),
        case("tek planda planı yazılmayan karar o planındır",
             [with_([GOOD[0]], 0, UstPlan=None)[0]] + with_(GOOD, 2, Plan=None)[2:], expect=[]),
        case("planı plan sınırlarında olmayan karar", with_(GOOD, 2, Plan="UIP-9"),
             expect=["noPlan", "uncovered"]),
        case("anılan plan notu yok", with_(GOOD, 2, PlanNotlari="2, 7"), expect=["noteMissing"]),
        case("hiçbir kararın anmadığı alan notu", with_(GOOD, 2, PlanNotlari=None), expect=["noteUnused"]),
        case("üst plan aynı kademede", with_(GOOD, 1, Seviye="uip"), expect=["levelOrder"]),
        case("üst plan yok", with_(GOOD, 0, UstPlan="CDP-1"), expect=["upperMissing"]),
        case("kademesi olmayan plan", with_(GOOD, 1, Seviye=None), expect=["noLevel"]),
        # docs/adr/2020: a plan outside its upper plan, the roads' continuity.
        case("alt plan üst planın sınırının dışına taşıyor", geo(GOOD, 1, (-20, -20, 80, 80), ), expect=["outsideUpper"]),
        case("yollar plan sınırına ve birbirine bağlı: bulgu yok", GOOD + [road("Y1", [(0, 25), (100, 25)]),
                                                                       road("Y2", [(50, 0), (50, 25)])],
             classes=ROADS, expect=[]),
        case("kopuk yol aksı: ucu ne yola ne plan sınırına değiyor", GOOD + [road("Y1", [(0, 25), (100, 25)]),
                                                                          road("Y3", [(50, 25), (50, 45)])],
             classes=ROADS, expect=["roadGap"]),
        case("iki ucu da kopuk yol: iki bulgu, baştaki önce", GOOD + [road("Y4", [(20, 10), (30, 10), (30, 20)])],
             classes=ROADS, expect=["roadGap", "roadGap"]),
        case("plan sınırına toleransta yaklaşan yol bağlı sayılır", GOOD + [road("Y5", [(0, 40), (100.005, 40)])],
             classes=ROADS, expect=[]),
        case("plan sınırına toleranstan uzak kalan yol kopuk", GOOD + [road("Y6", [(0, 40), (99.95, 40)])],
             classes=ROADS, expect=["roadGap"]),
        # docs/adr/2240: a decision inside a road's planned width.
        case("yollar planda yazılan genişlikte: yol kararları ve nazım planın kararı sayılmaz", roady(), classes=ROADS,
             expect=[]),
        case("yol genişliği çizilenden geniş: iki yandaki dört karar yola giriyor", roady(w1="12"), classes=ROADS,
             expect=["roadWidth"] * 4),
        case("genişliği yazılmamış yol denetlenmez", roady(w1=None), classes=ROADS, expect=[]),
        case("dönen yol: iki kolunda da genişlik; dönüşün yuvarlak dış köşesi kararlara değmiyor",
             ROADY + [road("Y3", [(0, 30), (50, 30), (50, 60)], "12")], classes=ROADS, expect=["roadWidth"] * 3),
        case("yola giren 3,125 cm'lik şerit 1 cm toleransla sayılır", roady(w1="10.0625"), classes=ROADS,
             expect=["roadWidth"] * 4),
        case("aynı şerit 5 cm toleransla dar parçadır, sayılmaz", roady(w1="10.0625"), classes=ROADS, tolerance=0.05,
             expect=[]),
        # docs/adr/2640: a parcel inside a road's planned width.
        case("parseller yol genişliğinin kıyısında: bulgu yok", roady() + PARCELS, classes=ROADS, expect=[]),
        case("yol genişliği çizilenden geniş: yola giren kararlar ve parseller", roady(w1="12") + PARCELS,
             classes=ROADS, expect=["roadWidth"] * 4 + ["roadParcel"] * 3),
        case("iki yol da geniş: bir parsel iki yola giriyor, her yolun bulgusu ayrı", roady(w1="12", w2="12") + PARCELS,
             classes=ROADS, expect=["roadWidth"] * 8 + ["roadParcel"] * 5),
        case("parsele giren 3,125 cm'lik şerit 1 cm toleransla sayılır", roady(w1="10.0625") + PARCELS, classes=ROADS,
             expect=["roadWidth"] * 4 + ["roadParcel"] * 3),
        case("aynı parsel şeridi 5 cm toleransla dar parçadır", roady(w1="10.0625") + PARCELS, classes=ROADS,
             tolerance=0.05, expect=[]),
        case("kimliği yazılmamış parsel kimliksiz diye söylenir",
             roady(w1="12") + [("parsel", (0, 0, 20, 25), {})], classes=ROADS, expect=["roadWidth"] * 4 + ["roadParcel"]),
        # docs/adr/4640: the symbol against the decision.
        case("sembol kararı göstermiyor: ticaret alanı park sembolüyle; eşleşen, eşi olmayan kullanım ve sembolsüz alan "
             "denetlenmez", SYMBOLED, expect=["symbolMismatch"]),
        case("sembolü kararınınki: bulgu yok", with_(SYMBOLED, 1, **{"@symbol": TRADE}), expect=[]),
        case("Plan yazılmamış alan tek plan sınırının kademesiyle denetlenir",
             with_(SYMBOLED, 1, Plan=None), expect=["symbolMismatch"]),
        case("nazım planın kararı: sanayi alanı spor sembolüyle",
             [plan("NIP-S", (0, 0, 100, 50), seviye="nip"),
              karar("N1", (0, 0, 100, 50), "sanayi", Plan="NIP-S", TAKS="0.50", KAKS="1.00", KatAdedi="2",
                    **{"@symbol": SYMBOLS["nip"]["spor"]})],
             expect=["symbolMismatch"]),
        # docs/adr/5040: the project's kept symbol choices; plan change drafts gone stale.
        case("projede saklanan konut seçimiyle konut alanı da denetlenir",
             SYMBOLED, zoning={"symbols": [{"level": "uip", "use": "konut", "symbol": GELISME}]},
             expect=["symbolMismatch", "symbolMismatch"]),
        case("saklanan park seçimi tablonun önüne geçer: park sembollü park alanı da uyuşmaz",
             SYMBOLED, zoning={"symbols": [{"level": "uip", "use": "park", "symbol": ARBORETUM}]},
             expect=["symbolMismatch", "symbolMismatch"]),
        case("kademenin sembolü olmayan saklı seçim okunmaz",
             SYMBOLED, zoning={"symbols": [{"level": "uip", "use": "konut", "symbol": SYMBOLS["nip"]["spor"]}]},
             expect=["symbolMismatch"]),
        case("taslağın okuduğu değerler yürürlükteki planınki: bulgu yok", GOOD, zoning={"drafts": [DRAFT_OK]},
             expect=[]),
        case("taslak eskidi: değişen değer, silinen alan, olmayan plan", GOOD,
             zoning={"drafts": [DRAFT_STALE, DRAFT_NO_PLAN]}, expect=["draftStale"] * 3),
        refused("tolerans sıfır", 0.0, "0"),
        refused("tolerans eksi", -1.0, "-1"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.zoning.check", "commandVersion": 1,
           "title": "İmar planı denetimi",
           "note": "ADR 1420, 2020, 2240, 4640, 5040. Bağımsız başvuru scripts/fixtures/zoning_check_cases.py: kurallar elle, alanlar eksene "
                   "paralel dikdörtgen, parçalar koordinat sıkıştırmasıyla tam; KentOS kodu yok.",
           "setup": setup(GOOD), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("İmar planı denetimi ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
