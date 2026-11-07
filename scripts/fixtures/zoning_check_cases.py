#!/usr/bin/env python3
"""The shared cases of `cad.zoning.check` (İmar planı denetimi; plan/map-cad.md MAP-09, docs/adr/1420), without
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
         "missingValue", "declaredValued", "kaksReview", "overlap", "uncovered", "outside", "roadGap", "noteMissing",
         "noteUnused"]
OPEN = {"park", "yesil", "meydan", "yol"}
RANK = ["msp", "cdp", "nip", "uip"]
LAYER_IDS = {"plan-siniri": "plan", "karar-alani": "karar", "plan-notu": "notlar", "yol-aksi": "yollar"}
ROADS = ("plan-siniri", "karar-alani", "plan-notu", "yol-aksi")
CLASS_OF = {"plan": "plan-siniri", "karar": "karar-alani", "notlar": "plan-notu", "yollar": "yol-aksi"}


def uid(n):
    return f"0192f5c0-1420-7000-8000-{n:012x}"


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


def road(kimlik, pts):
    return ("yollar", pts, {"YolKimlik": kimlik, "Tur": "tasit"})


def setup(objects, classes=("plan-siniri", "karar-alani", "plan-notu")):
    ents = []
    for i, (layer, geo, attrs) in enumerate(objects, start=1):
        a = {k: v for k, v in attrs.items() if v is not None}
        if layer == "notlar":
            ents.append({"kind": "point", "id": i, "layerId": layer, "attrs": a,
                         "p": {"x": X0 + geo[0], "y": Y0 + geo[1]}})
        elif layer == "yollar":
            ents.append({"kind": "polyline", "id": i, "layerId": layer, "attrs": a,
                         "pts": [{"x": X0 + x, "y": Y0 + y} for x, y in geo]})
        else:
            ents.append({"kind": "polygon", "id": i, "layerId": layer, "attrs": a, "pts": rect_pts(geo)})
    return {"format": "kentos.document", "version": 2, "name": "İmar planı denetimi",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0}, "layers": layers(classes), "activeLayer": "karar" if
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


def counted(ps, t):
    kept = [a for a, p in ps if p > 0 and 2 * a / p > t]
    return sum(kept) if kept else None


def intersect(a, b):
    r = (max(a[0], b[0]), max(a[1], b[1]), min(a[2], b[2]), min(a[3], b[3]))
    return [r] if r[0] < r[2] and r[1] < r[3] else []


# ── The check ────────────────────────────────────────────────────────────────────────────────────────────────────


def check(objects, classes=("plan-siniri", "karar-alani", "plan-notu"), tolerance=0.01):
    found = []

    def say(kind, ats, area, message):
        found.append((KINDS.index(kind), min(ats) if ats else -1,
                      {"kind": kind, "uids": [uid(a + 1) for a in sorted(ats)], **({"area": area} if area is not None
                                                                                   else {}), "message": message}))

    for c in ("plan-siniri", "karar-alani", "plan-notu"):
        if c not in classes:
            say("missingLayer", [], None,
                f"“{' › '.join(cls(c)['layer'])}” katmanı yok; imar-plani profilini uygulayın (cad.profiles.apply).")
    g = lambda o, k: (o[2].get(k) or "").strip()  # noqa: E731
    items = list(enumerate(objects))
    plans = [(i, o) for i, o in items if o[0] == "plan" and "plan-siniri" in classes]
    decisions = [(i, o) for i, o in items if o[0] == "karar" and "karar-alani" in classes]
    notes = [(i, o) for i, o in items if o[0] == "notlar" and "plan-notu" in classes]
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


def case(name, objects, op="plan", classes=("plan-siniri", "karar-alani", "plan-notu"), tolerance=None, expect=None):
    kept = [o for o in objects if CLASS_OF[o[0]] in classes]
    out = check(kept, classes, 0.01 if tolerance is None else tolerance)
    if expect is not None:
        assert [f["kind"] for f in out["findings"]] == expect, (name, [f["kind"] for f in out["findings"]])
    input_ = {} if tolerance is None else {"tolerance": tolerance}
    return {"name": name, "setup": setup(kept, classes), "steps": [{"op": op, "input": input_, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}


def refused(name, tolerance, text):
    return {"name": name, "steps": [{"op": "execute", "input": {"tolerance": tolerance}, "result": {
        "status": "failed", "error": {"code": "invalid_input", "message":
                                      f"Tolerans sonlu ve en az 0.000001 m olmalı (yazılan {text}).",
                                      "path": "tolerance"}}, "expect": SAME}]}


def cases():
    return [
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
        refused("tolerans sıfır", 0.0, "0"),
        refused("tolerans eksi", -1.0, "-1"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.zoning.check", "commandVersion": 1,
           "title": "İmar planı denetimi",
           "note": "ADR 1420. Bağımsız başvuru scripts/fixtures/zoning_check_cases.py: kurallar elle, alanlar eksene "
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
