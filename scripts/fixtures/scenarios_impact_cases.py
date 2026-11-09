#!/usr/bin/env python3
"""The shared cases of `cad.scenarios.impact` (Senaryo ölçüleri; plan/map-cad.md MAP-10, docs/adr/1440), without
KentOS code.

    python3 scripts/fixtures/scenarios_impact_cases.py           # writes fixtures/commands/v1/cad.scenarios.impact.json
    python3 scripts/fixtures/scenarios_impact_cases.py --check   # writes nothing; compares

The rules, by hand:
- the pairs: each layer of the scenario group that stands for (`replaces`) a base layer, in the tree's order;
- each side over its layers, in the drawing's order: objects; areas (rectangles here: exact shoelace); lines' length
  (hypot); capacity Σ area × KAKS over the areas whose use is not park, yeşil, meydan or yol, an area without a KAKS
  counted as a gap; the areas by use (the field's trimmed value, “” for none), sorted by code;
- the changes, matched by the key (first object of a key on its side): the base's keys in order (removed, or geometry
  when the vertices differ, decision when the non-empty trimmed attributes differ, both), then the alternative's new
  keys (added);
- the stamp: SHA-256 over the base layers' objects, “id|layer|kind|k=v;…|xbits,ybits;…”, its first 8 bytes in hex;
  `stale` is whether a given stamp differs from it;
- asked for (docs/adr/2040), the user's values: the impervious area Σ area × the share of its use (a use with area and
  no share a gap), and the area reached, a `from` area whose centroid (a rectangle's middle) lies within the distance
  of a `to` area (the distance to a rectangle: 0 inside, else to its box), with its share of the `from` areas (0 with
  none); a side with `from` areas and no `to` area a gap; and on each side, by persistent id in the drawing's order,
  the `from` areas reached, those not and the `to` areas (`reach`, docs/adr/2260);
- asked for (docs/adr/2660), the area near an infrastructure layer: a measured area (of the `from` uses, or every
  area) whose centroid lies within the distance of an object of the layer (lines here: the distance to their
  segments), with its share; the layer the scenario stands in for is each side's own; a side with measured areas and
  no object on its layer a gap.
"""

import copy
import hashlib
import json
import struct
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.scenarios.impact.json"
X0, Y0 = 487000.0, 4420000.0
OPEN = {"park", "yesil", "meydan", "yol"}


def uid(n):
    return f"0192f5c0-1440-7000-8000-{n:012x}"


def node(id_, name, kind="layer", children=None, **more):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, **more,
            "children": children or []}


def tree(infra=False):
    extra = [node("altyapi", "Kanalizasyon hattı")] if infra else []
    return extra + [node("0", "0"),
            node("imar", "İmar planı", "group", [node("karar", "Karar alanları"), node("yol", "Yollar")]),
            node("alt-a", "Alternatif A", "group", [node("alt-a-karar", "Karar alanları", replaces="karar"),
                                                     node("alt-a-yol", "Yollar", replaces="yol")],
                 scenario={"note": "Okul yanına yeşil alan, ticaret konuta"})]


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def area(layer, r, **attrs):
    x0, y0, x1, y1 = r
    return {"kind": "polygon", "layerId": layer, "attrs": attrs, "pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}


def line(layer, a, b, **attrs):
    return {"kind": "line", "layerId": layer, "attrs": attrs, "a": P(*a), "b": P(*b)}


BASE = [
    area("karar", (0, 0, 40, 50), KararKimlik="K1", Kullanim="konut", KAKS="1.20"),
    area("karar", (40, 0, 70, 50), KararKimlik="K2", Kullanim="ticaret-konut", KAKS="1.75"),
    area("karar", (70, 0, 100, 30), KararKimlik="K3", Kullanim="park"),
    area("karar", (70, 30, 100, 50), KararKimlik="K4", Kullanim="egitim", KAKS="1.00"),
    area("karar", (100, 0, 120, 50), KararKimlik="K6", Kullanim="sanayi", KAKS="1.00"),
    line("yol", (0, -5), (100, -5), YolKimlik="Y1"),
]
ALT = [
    area("alt-a-karar", (0, 0, 40, 50), KararKimlik="K1", Kullanim="konut", KAKS="1.20"),
    area("alt-a-karar", (40, 0, 70, 50), KararKimlik="K2", Kullanim="konut", KAKS="1.50"),
    area("alt-a-karar", (70, 0, 100, 20), KararKimlik="K3", Kullanim="park"),
    area("alt-a-karar", (70, 30, 100, 50), KararKimlik="K4", Kullanim="egitim", KAKS="1.00"),
    area("alt-a-karar", (70, 20, 100, 30), KararKimlik="K5", Kullanim="yesil"),
    area("alt-a-karar", (120, 0, 130, 10), Kullanim="konut"),
    line("alt-a-yol", (0, -5), (100, -5), YolKimlik="Y1"),
    line("alt-a-yol", (100, -5), (100, 60), YolKimlik="Y2"),
]


def setup(base=None, alt=None, layers=None, extra=()):
    ents = [copy.deepcopy(e) for e in (base if base is not None else BASE) + (alt if alt is not None else ALT)
            + list(extra)]
    for i, e in enumerate(ents, start=1):
        e["id"] = i
    return {"format": "kentos.document", "version": 2, "name": "Senaryo ölçüleri",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0}, "layers": layers or tree(), "activeLayer": "karar", "entities": ents,
            "uids": [uid(i + 1) for i in range(len(ents))], "styles": {"items": [], "categories": []}}


# ── The measures ─────────────────────────────────────────────────────────────────────────────────────────────────


def bits(v):
    return format(struct.unpack(">Q", struct.pack(">d", v))[0], "016x")


def verts(e):
    if e["kind"] == "line":
        return [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])]
    return [(p["x"], p["y"]) for p in e["pts"]]


def shoelace(e):
    q = [(Fraction(x), Fraction(y)) for x, y in verts(e)]
    return float(abs(sum(q[i][0] * q[(i + 1) % len(q)][1] - q[(i + 1) % len(q)][0] * q[i][1]
                         for i in range(len(q)))) / 2)


def length(e):
    (ax, ay), (bx, by) = verts(e)
    return ((bx - ax) ** 2 + (by - ay) ** 2) ** 0.5


def pairs(s):
    out = []

    def walk(ns):
        for n in ns:
            if n["type"] == "layer" and n.get("replaces"):
                out.append((n["replaces"], n["id"]))
            walk(n["children"])
    walk(next(n for n in s["layers"] if n["id"] == "alt-a")["children"])
    return out


def kaks(e):
    try:
        v = float(e["attrs"].get("KAKS", "").strip().replace(",", "."))
        return v
    except ValueError:
        return None


def side(s, layers, field, key):
    out = {"objects": 0, "area": 0.0, "length": 0.0, "capacity": 0.0, "noKaks": 0, "noUse": 0, "noKey": 0,
           "uses": {}, "keyed": [], "areas": []}
    for layer in layers:
        for e in [e for e in s["entities"] if e["layerId"] == layer]:
            out["objects"] += 1
            get = lambda k: e["attrs"].get(k, "").strip()  # noqa: E731
            if e["kind"] == "polygon":
                a = shoelace(e)
                out["area"] += a
                u = get(field)
                out["areas"].append((e, u, a))
                if not u:
                    out["noUse"] += 1
                out["uses"][u] = out["uses"].get(u, 0.0) + a
                if u not in OPEN:
                    k = kaks(e)
                    if k is None:
                        out["noKaks"] += 1
                    else:
                        out["capacity"] += a * k
                kk = get(key)
                if not kk:
                    out["noKey"] += 1
                elif kk not in [x[0] for x in out["keyed"]]:
                    out["keyed"].append((kk, e, a))
            elif e["kind"] == "line":
                out["length"] += length(e)
    return out


def capacity(e, a, field):
    if e["attrs"].get(field, "").strip() in OPEN:
        return 0.0
    k = kaks(e)
    return None if k is None else a * k


def stamp(s, layers):
    h = hashlib.sha256()
    for layer in layers:
        for e in [e for e in s["entities"] if e["layerId"] == layer]:
            attrs = ";".join(f"{k}={v}" for k, v in sorted(e["attrs"].items()))
            pts = ";".join(f"{bits(x)},{bits(y)}" for x, y in verts(e))
            h.update(f"{e['id']}|{layer}|{e['kind']}|{attrs}|{pts}\n".encode())
    return h.digest()[:8].hex()


def box(e):
    xs, ys = [p["x"] for p in e["pts"]], [p["y"] for p in e["pts"]]
    return min(xs), min(ys), max(xs), max(ys)


def impervious(sd, shares):
    total, missing = 0.0, []
    for _, u, a in sd["areas"]:
        if u in shares:
            total += a * shares[u]
        elif a > 0 and u not in missing:
            missing.append(u)
    return total, missing


def reached(sd, access):
    """The area reached, the `from` areas' whole, and by persistent id the `from` areas reached, those not and the `to`
    areas (docs/adr/2260), in the drawing's order."""
    to = [e for e, u, _ in sd["areas"] if u in access["to"]]
    targets = [box(e) for e in to]
    got = whole = 0.0
    yes, no = [], []
    for e, u, a in sd["areas"]:
        if u not in access["from"]:
            continue
        whole += a
        x0, y0, x1, y1 = box(e)
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        near = lambda t: (max(t[0] - cx, 0, cx - t[2]) ** 2 + max(t[1] - cy, 0, cy - t[3]) ** 2) ** 0.5  # noqa: E731
        if any(near(t) <= access["distance"] for t in targets):
            got += a
            yes.append(uid(e["id"]))
        else:
            no.append(uid(e["id"]))
    return got, whole, bool(targets), yes, no, [uid(e["id"]) for e in to]


def to_segment(p, a, b):
    (px, py), (ax, ay), (bx, by) = p, a, b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return ((px - ax - t * dx) ** 2 + (py - ay - t * dy) ** 2) ** 0.5


def nearest_on_segment(p, a, b):
    """The segment's nearest place to p and its distance (the foot of the perpendicular, held within the ends)."""
    (px, py), (ax, ay), (bx, by) = p, a, b
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    q = (ax + t * dx, ay + t * dy)
    return q, ((px - q[0]) ** 2 + (py - q[1]) ** 2) ** 0.5


def near(s, sd, layer, infra):
    """The area near the layer's objects (docs/adr/2660), the measured areas' whole, the layer's objects; and the
    measured areas near and beyond and the layer's objects by persistent id (docs/adr/3060); and each measured area
    joined to its nearest object (docs/adr/3460): the first of equals in the drawing's order, its nearest place."""
    objs = [e for e in s["entities"] if e["layerId"] == layer]
    lines = [verts(e) for e in objs]
    uses = [u for u in infra.get("from", []) if u.strip()]
    got = whole = 0.0
    yes, no, links = [], [], []
    for e, u, a in sd["areas"]:
        if uses and u not in uses:
            continue
        whole += a
        x0, y0, x1, y1 = box(e)
        c = ((x0 + x1) / 2, (y0 + y1) / 2)
        if any(to_segment(c, p, q) <= infra["distance"] for pts in lines for p, q in zip(pts, pts[1:])):
            got += a
            yes.append(uid(e["id"]))
        else:
            no.append(uid(e["id"]))
        tried = [(k,) + nearest_on_segment(c, p, q) for k, pts in enumerate(lines) for p, q in zip(pts, pts[1:])]
        if tried:
            best = min(tried, key=lambda x: (x[2], x[0]))
            # Two objects at exactly one distance (axis-parallel roads, whole metres: both sides compute it exactly)
            # take the first in the drawing's order; none at nearly one distance, which would hang on the last digit.
            assert all(x[0] == best[0] or x[2] == best[2] or x[2] - best[2] > 1e-6 for x in tried), (e["id"], tried)
            links.append({"area": uid(e["id"]), "line": uid(objs[best[0]]["id"]), "from": [c[0], c[1]],
                          "to": [best[1][0], best[1][1]], "distance": best[2]})
    return got, whole, len(lines), yes, no, [uid(e["id"]) for e in objs], links


def rs(v):
    """A number as Rust writes an f64 (40.0 as 40)."""
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def report(s, field="Kullanim", key="KararKimlik", since=None, impervious_=None, access=None, infrastructure=None):
    ps = pairs(s)
    bases, alts = [b for b, _ in ps], [l for _, l in ps]
    b, a = side(s, bases, field, key), side(s, alts, field, key)

    def m(id_, label, unit, x, y, method):
        return {"id": id_, "label": label, "unit": unit, "base": x, "alternative": y, "difference": y - x,
                "method": method}
    measures = [
        m("objects", "Nesne sayısı", "count", b["objects"], a["objects"], "Karşılaştırılan katmanlardaki bütün nesneler."),
        m("area", "Alanların toplamı", "m2", b["area"], a["area"],
          "Alanların düzlemsel alanı, koordinatlardan (yay ve delikler dahil)."),
        m("length", "Çizgilerin uzunluğu", "m", b["length"], a["length"], "Çizgi ve çoklu çizgilerin düzlemsel uzunluğu."),
        m("capacity", "İnşaat alanı (alan × KAKS)", "m2", b["capacity"], a["capacity"],
          "Her yapılaşan alanın alanı × KAKS; KAKS'ı olmayan alan sayılmaz (Veri eksikleri)."),
    ]
    asked_gaps = []
    if impervious_ is not None:
        (x, mb), (y, ma) = impervious(b, impervious_), impervious(a, impervious_)
        given = ", ".join(f"{c} {rs(k)}" for c, k in sorted(impervious_.items()))
        measures.append(m("impervious", "Geçirimsiz alan", "m2", x, y,
                          f"Her alanın alanı × kullanımının verilen geçirimsizlik payı ({given}); payı verilmeyen kullanım "
                          "sayılmaz (Veri eksikleri)."))
        missing = sorted(set(mb) | set(ma))
        if missing:
            words = ", ".join(w or "(kullanımı yok)" for w in missing)
            asked_gaps.append(f"Geçirimsizlik payı verilmemiş kullanım: {words}; geçirimsiz alan onlarsız hesaplandı.")
    reach = None
    near_out = None
    if access is not None:
        (xg, xw, xt, xr, xu, xs), (yg, yw, yt, yr, yu, ys) = reached(b, access), reached(a, access)
        reach = {"baseReached": xr, "baseUnreached": xu, "baseTargets": xs,
                 "alternativeReached": yr, "alternativeUnreached": yu, "alternativeTargets": ys}
        fr, to, d = ", ".join(access["from"]), ", ".join(access["to"]), rs(access["distance"])
        measures.append(m("access", f"Erişen alan ({fr} → {to}, {d} m)", "m2", xg, yg,
                          f"Kullanımı {fr} olan alanların ağırlık merkezinden kullanımı {to} olan en yakın alana düz uzaklık "
                          f"en çok {d} m ise alanı sayılır; merkez alanın içindeyse uzaklık 0."))
        share = lambda g_, w_: g_ / w_ * 100 if w_ > 0 else 0.0  # noqa: E731
        measures.append(m("accessShare", "Erişen alanın payı", "%", share(xg, xw), share(yg, yw),
                          f"Erişen alanın {fr} alanlarının toplamına oranı; {fr} alanı olmayan tarafta 0."))
        for has_from, has_to, where in ((xw > 0, xt, "mevcut durumda"), (yw > 0, yt, "senaryoda")):
            if has_from and not has_to:
                asked_gaps.append(f"Erişilecek kullanım ({to}) {where} yok; erişen alan 0.")
    if infrastructure is not None:
        lid = infrastructure["layer"]
        theirs = next((l for b_, l in ps if b_ == lid), lid)
        (xg, xw, xn, xy, xf, xl, xk), (yg, yw, yn, yy, yf, yl, yk) = (near(s, b, lid, infrastructure),
                                                                      near(s, a, theirs, infrastructure))
        near_out = {"baseNear": xy, "baseFar": xf, "baseLines": xl,
                    "alternativeNear": yy, "alternativeFar": yf, "alternativeLines": yl}
        if xk:
            near_out["baseLinks"] = xk
        if yk:
            near_out["alternativeLinks"] = yk
        name = {"altyapi": "Kanalizasyon hattı", "yol": "Yollar", "karar": "Karar alanları"}[lid]
        d = rs(infrastructure["distance"])
        uses_ = [u for u in infrastructure.get("from", []) if u.strip()]
        if uses_:
            label, which = f"Altyapıya yakın alan ({', '.join(uses_)} → {name}, {d} m)", f"Kullanımı {', '.join(uses_)} olan alanların"
        else:
            label, which = f"Altyapıya yakın alan ({name}, {d} m)", "Alanların"
        measures.append(m("infrastructure", label, "m2", xg, yg,
                          f"{which} ağırlık merkezinden {name} katmanındaki en yakın nesneye düz uzaklık en çok {d} m ise "
                          "alanı sayılır; hattın kapasitesi ya da bağlantı hakkı değildir."))
        share = lambda g_, w_: g_ / w_ * 100 if w_ > 0 else 0.0  # noqa: E731
        measures.append(m("infrastructureShare", "Altyapıya yakın alanın payı", "%", share(xg, xw), share(yg, yw),
                          "Altyapıya yakın alanın ölçülen alanların toplamına oranı; ölçülen alanı olmayan tarafta 0."))
        for whole_, objects, where in ((xw, xn, "mevcut durumda"), (yw, yn, "senaryoda")):
            if whole_ > 0 and objects == 0:
                asked_gaps.append(f"Altyapı katmanında ({name}) {where} nesne yok; yakın alan 0.")
    uses = [{"code": c, "base": b["uses"].get(c, 0.0), "alternative": a["uses"].get(c, 0.0),
             "difference": a["uses"].get(c, 0.0) - b["uses"].get(c, 0.0)}
            for c in sorted(set(b["uses"]) | set(a["uses"]))]
    attr = lambda e, k: e["attrs"].get(k, "").strip() or None  # noqa: E731
    decided = lambda e: {k: v.strip() for k, v in e["attrs"].items() if v.strip()}  # noqa: E731

    def change(k, kind, eb=None, ab=None, ea=None, aa=None):
        c = {"key": k, "change": kind}
        for name, v in (("baseUse", attr(eb, field) if eb else None), ("alternativeUse", attr(ea, field) if ea else None),
                        ("baseArea", ab), ("alternativeArea", aa),
                        ("baseCapacity", capacity(eb, ab, field) if eb else None),
                        ("alternativeCapacity", capacity(ea, aa, field) if ea else None)):
            if v is not None:
                c[name] = v
        return c
    changes = []
    for k, eb, ab in b["keyed"]:
        other = [x for x in a["keyed"] if x[0] == k]
        if not other:
            changes.append(change(k, "removed", eb, ab))
            continue
        _, ea, aa = other[0]
        g, d = verts(eb) != verts(ea), decided(eb) != decided(ea)
        if g or d:
            changes.append(change(k, {(True, False): "geometry", (False, True): "decision", (True, True): "both"}[(g, d)],
                                  eb, ab, ea, aa))
    for k, ea, aa in a["keyed"]:
        if k not in [x[0] for x in b["keyed"]]:
            changes.append(change(k, "added", ea=ea, aa=aa))
    gaps = []
    if b["noKaks"] + a["noKaks"]:
        gaps.append(f"KAKS'ı olmayan yapılaşan alan: mevcut durumda {b['noKaks']}, senaryoda {a['noKaks']}; inşaat "
                    "alanı onlarsız hesaplandı.")
    if b["noUse"] + a["noUse"]:
        gaps.append(f"Kullanımı ({field}) olmayan alan: mevcut durumda {b['noUse']}, senaryoda {a['noUse']}.")
    if b["noKey"] + a["noKey"]:
        gaps.append(f"Anahtarı ({key}) olmayan alan: mevcut durumda {b['noKey']}, senaryoda {a['noKey']}; "
                    "değişiklikleri izlenemez.")
    gaps += asked_gaps
    now = stamp(s, bases)
    names = {"karar": "Karar alanları", "yol": "Yollar"}
    out = {"scenario": "alt-a", "name": "Alternatif A",
           "layers": [{"base": x, "layer": y, "name": names[x]} for x, y in ps], "field": field, "key": key,
           "measures": measures, "uses": uses, "changes": changes, "gaps": gaps,
           "assumptions": [f"Alanlar ve uzunluklar düzlemsel, projenin koordinatlarından (EPSG:{s['settings']['srid']}).",
                           "İnşaat alanı = alan × KAKS: brüt alan, terk ve DOP düşülmeden.",
                           "Park, yeşil alan, meydan ve yol inşaat alanı taşımaz.",
                           "Tek puan yok: ölçüler yan yana verilir; senaryoyu kullanıcı seçer."],
           "stamp": now}
    if since is not None:
        out["stale"] = since != now
    if reach is not None:
        out["reach"] = reach
    if near_out is not None:
        out["near"] = near_out
    out["revision"] = "$current"
    return out


SAME = {"revision": "same", "canUndo": False}


def ok(name, s=None, op="plan", **kw):
    s = s or setup()
    input_ = {"scenario": "alt-a", **{k: v for k, v in kw.items() if v is not None}}
    given = {k: v for k, v in kw.items() if v is not None}
    for k, first in (("field", "Kullanim"), ("key", "KararKimlik")):
        if k in given:
            given[k] = given[k].strip() or first
    if "impervious" in given:
        given["impervious_"] = given.pop("impervious")
    out = report(s, **given)
    case = {"name": name, "steps": [{"op": op, "input": input_, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}
    if s is not BASE_SETUP:
        case["setup"] = s
    return case


def refused(name, input_, code, message, s=None, path="scenario"):
    case = {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": SAME}]}
    if s is not None:
        case["setup"] = s
    return case


BASE_SETUP = setup()
# docs/adr/2040: the user's impervious shares by use and an access question (the cases' own values, not KentOS's).
SHARES = {"konut": 0.6, "ticaret-konut": 0.8, "park": 0.1, "egitim": 0.5, "sanayi": 0.9, "yesil": 0.05}
ACCESS = {"distance": 40.0, "from": ["konut"], "to": ["park", "yesil"]}


# docs/adr/2660: a sewer line along y = 60 north of the blocks, on its own layer (not the scenario's).
SEWER = line("altyapi", (0, 60), (130, 60), HatKimlik="K-1")
INFRA_SETUP = setup(layers=tree(infra=True), extra=[SEWER])
EMPTY_INFRA = setup(layers=tree(infra=True))


def moved_base():
    """The field state changed after the comparison: K1's corner moved."""
    base = copy.deepcopy(BASE)
    base[0] = area("karar", (0, 0, 41, 50), KararKimlik="K1", Kullanim="konut", KAKS="1.20")
    return setup(base=base)


def cases():
    first = report(BASE_SETUP)
    old = first["stamp"]
    assert [c["change"] for c in first["changes"]] == ["decision", "geometry", "removed", "added"], first["changes"]
    no_pairs = tree()
    for c in no_pairs[2]["children"]:
        c.pop("replaces")
    return [
        ok("plan: ölçüler, kullanımlar, değişen kararlar, eksikler, varsayımlar", BASE_SETUP),
        ok("uygula: salt okunur", BASE_SETUP, op="execute"),
        ok("doğrulama", BASE_SETUP, op="validate"),
        ok("aynı tabanla damga: güncel", BASE_SETUP, since=old),
        ok("taban değişince eski karşılaştırma eskir (stale)", moved_base(), since=old),
        ok("taban değişince K1 değişikliği de görünür", moved_base()),
        ok("damga başka karşılaştırmanın: eskimiş sayılır", BASE_SETUP, since="0000000000000000"),
        ok("başka kullanım alanı ve anahtar", BASE_SETUP, field="YolKimlik", key="Kullanim"),
        ok("boş alan adı ilk değere döner", BASE_SETUP, field="  ", key=" "),
        ok("aynı senaryo: değişiklik yok", setup(alt=[dict(copy.deepcopy(e), layerId={"karar": "alt-a-karar",
                                                                                          "yol": "alt-a-yol"}[e["layerId"]])
                                                     for e in BASE])),
        ok("boş senaryo: hepsi kaldırılmış", setup(alt=[])),
        ok("boş taban: hepsi eklenmiş", setup(base=[])),
        ok("virgüllü KAKS okunur", setup(alt=[dict(copy.deepcopy(e), attrs=dict(e["attrs"], KAKS="1,50"))
                                              if e["attrs"].get("KararKimlik") == "K2" else e for e in ALT])),
        ok("KAKS yazı değilse eksik sayılır", setup(alt=[dict(copy.deepcopy(e), attrs=dict(e["attrs"], KAKS="yok"))
                                                       if e["attrs"].get("KararKimlik") == "K2" else e for e in ALT])),
        ok("yalnız geometri değişimi", setup(alt=[area("alt-a-karar", (0, 0, 45, 50), KararKimlik="K1", Kullanim="konut",
                                                       KAKS="1.20")], base=[BASE[0]])),
        ok("geometri ve karar birlikte", setup(alt=[area("alt-a-karar", (0, 0, 45, 50), KararKimlik="K1",
                                                         Kullanim="ticaret", KAKS="2.00")], base=[BASE[0]])),
        ok("anahtar yinelenirse ilki eşlenir", setup(alt=ALT + [area("alt-a-karar", (200, 0, 210, 10),
                                                                     KararKimlik="K1", Kullanim="konut", KAKS="1")])),
        ok("park, yeşil, meydan, yol inşaat alanı taşımaz", setup(
            base=[area("karar", (0, 0, 10, 10), KararKimlik="A", Kullanim=u) for u in ("park", "yesil", "meydan", "yol")],
            alt=[])),
        # docs/adr/2040: the impervious area and the access, the user's values.
        ok("geçirimsizlik payları verilmiş: geçirimsiz alan", BASE_SETUP, impervious=SHARES),
        ok("payı verilmeyen kullanım eksik sayılır", BASE_SETUP,
           impervious={k: v for k, v in SHARES.items() if k not in ("sanayi", "yesil")}),
        ok("erişim: konuttan parka ve yeşil alana 40 m", BASE_SETUP, access=ACCESS),
        ok("erişim: 50 m'de mevcut durumdaki konut da erişir", BASE_SETUP, access=dict(ACCESS, distance=50.0)),
        ok("erişilecek kullanım hiçbir tarafta yok: iki eksik", BASE_SETUP,
           access={"distance": 40.0, "from": ["konut"], "to": ["meydan"]}),
        ok("geçirimsizlik ve erişim birlikte", BASE_SETUP, impervious=SHARES, access=ACCESS),
        # docs/adr/2660: the area near an infrastructure layer, the user's distance.
        ok("altyapıya yakınlık: kanalizasyon hattına 40 m", INFRA_SETUP, infrastructure={"layer": "altyapi", "distance": 40.0}),
        ok("altyapıya yakınlık yalnız konut alanlarında", INFRA_SETUP,
           infrastructure={"layer": "altyapi", "distance": 40.0, "from": ["konut"]}),
        ok("senaryonun yerine geçtiği katman altyapı: her taraf kendi yollarıyla", BASE_SETUP,
           infrastructure={"layer": "yol", "distance": 30.0}),
        ok("altyapı katmanı boş: yakın alan 0, iki tarafta da eksik", EMPTY_INFRA,
           infrastructure={"layer": "altyapi", "distance": 40.0}),
        refused("altyapı uzaklığı 0", {"scenario": "alt-a", "infrastructure": {"layer": "altyapi", "distance": 0.0}},
                "invalid_input", "Altyapı uzaklığı 0'dan büyük olmalı (yazılan 0).", s=INFRA_SETUP,
                path="infrastructure.distance"),
        refused("altyapı katmanı çizimde yok", {"scenario": "alt-a", "infrastructure": {"layer": "su", "distance": 40.0}},
                "invalid_input", "“su” kimlikli altyapı katmanı çizimde yok; bir katmanın kimliğini verin.", s=INFRA_SETUP,
                path="infrastructure.layer"),
        refused("altyapı katmanı yerine grup verilmiş", {"scenario": "alt-a", "infrastructure": {"layer": "imar",
                                                                                                "distance": 40.0}},
                "invalid_input", "“imar” kimlikli altyapı katmanı çizimde yok; bir katmanın kimliğini verin.",
                path="infrastructure.layer"),
        refused("geçirimsizlik payı 1'den büyük", {"scenario": "alt-a", "impervious": {"konut": 1.5}}, "invalid_input",
                "“konut” kullanımının geçirimsizlik payı 0 ile 1 arasında olmalı (yazılan 1.5).", path="impervious.konut"),
        refused("erişim uzaklığı 0", {"scenario": "alt-a", "access": {"distance": 0.0, "from": ["konut"], "to": ["park"]}},
                "invalid_input", "Erişim uzaklığı 0'dan büyük olmalı (yazılan 0).", path="access.distance"),
        refused("erişen kullanım yok", {"scenario": "alt-a", "access": {"distance": 40.0, "from": [], "to": ["park"]}},
                "invalid_input", "Erişen kullanım verilmemiş; en az bir kullanım kodu yazın.", path="access.from"),
        refused("senaryo değil", {"scenario": "imar"}, "scenario_not_found",
                "“imar” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin."),
        refused("çizimde olmayan senaryo", {"scenario": "alt-z"}, "scenario_not_found",
                "“alt-z” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin."),
        refused("yerine geçen katmanı olmayan senaryo", {"scenario": "alt-a"}, "nothing_to_compare",
                "“Alternatif A” senaryosunun hiçbir katmanı bir ana katmanın yerine geçmiyor; karşılaştırılacak bir şey "
                "yok.", setup(layers=no_pairs)),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.scenarios.impact", "commandVersion": 1,
           "title": "Senaryo ölçüleri",
           "note": "ADR 1440. Bağımsız başvuru scripts/fixtures/scenarios_impact_cases.py: ölçüler, eşleme ve damga "
                   "elle (rasyonel shoelace, SHA-256); KentOS kodu yok.",
           "setup": BASE_SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Senaryo ölçüleri ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
