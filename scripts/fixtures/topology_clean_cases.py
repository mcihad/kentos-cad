#!/usr/bin/env python3
"""The shared cases of `cad.topology.clean` (Topolojik temizlik with its moves reported; plan/map-cad.md MAP-07,
docs/adr/1262), without KentOS code.

    python3 scripts/fixtures/topology_clean_cases.py           # writes fixtures/commands/v1/cad.topology.clean.json
    python3 scripts/fixtures/topology_clean_cases.py --check   # writes nothing; compares

The clean-up itself is the independent reference of docs/adr/0148 (`scripts/fixtures/topology_cases.py`'s `clean`);
this script adds the report by hand: each move's distance (hypot of from − to), each changed object's source vertices
kept bit for bit, its moves, their farthest and total, its area (shoelace; the areas here are straight) or length
(a straight edge its chord, an arc edge r·θ with θ = 4·atan|b|, r = chord / (2·sin(θ/2))) before and after, the change
of the areas' sum, and the moves past the quality profile's acceptance threshold.
"""

import copy
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "fixtures"))
from topology_cases import clean  # noqa: E402  (the independent reference of docs/adr/0148)

OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.topology.clean.json"
X0, Y0 = 487000.0, 4420000.0
PROFILE = {"name": "Kadastro güncelleme", "edition": "2026/1", "precision": 0.0001, "acceptance": 0.05}


def uid(n):
    return f"0192f5c0-1262-7000-8000-{n:012x}"


def layer(id_, name, locked=False, visible=True):
    return {"id": id_, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def line(n, a, b, layer_="cizim", za=None, zb=None):
    e = {"kind": "line", "id": n, "layerId": layer_, "attrs": {}, "a": P(*a), "b": P(*b)}
    if za is not None:
        e["za"] = za
    if zb is not None:
        e["zb"] = zb
    return e


def area(n, pts):
    return {"kind": "polygon", "id": n, "layerId": "cizim", "attrs": {}, "pts": [P(*p) for p in pts]}


ENTS = [
    line(1, (0, 0), (10, 0), za=100.0, zb=101.0),           # A
    line(2, (10.03, 0.01), (20, 5)),                          # B: its start joins A's end (Uçlar)
    line(3, (5, 10), (5, 0.03)),                              # C: stops 3 cm short of A (Uzat)
    line(4, (7, 10), (7, -0.04)),                             # D: runs 4 cm past A (Buda)
    area(5, [(0, 20), (10, 20), (10, 30), (0, 30)]),          # two parcels whose shared corners are a little off
    area(6, [(10.02, 20.01), (20, 20), (20, 30), (10.01, 29.98)]),
    {"kind": "point", "id": 7, "layerId": "cizim", "attrs": {}, "p": P(30.02, -0.01), "z": 101.25},
    line(8, (25, 0), (30, 0)),                                # E: its end goes to the point
    line(9, (40, 0), (50, 0), layer_="kilitli"),              # L: locked, a support
    line(10, (50.02, 0), (60, 0)),                            # F: its start goes to L's end
    line(11, (40, 10), (50, 10), layer_="gizli"),             # H: hidden, takes no part
    line(12, (50.02, 10), (60, 10)),                          # G: nothing near it that takes part
    {"kind": "polyline", "id": 13, "layerId": "cizim", "attrs": {}, "pts": [P(0, 45), P(5, 42), P(10.02, 40.01)],
     "bulges": [0.3, 0.0, 0.0]},                              # M: an arc edge, its end joins K's start
    line(14, (10, 40), (20, 40)),                             # K
    {"kind": "text", "id": 15, "layerId": "cizim", "attrs": {}, "text": "Ada 101", "p": P(0, 50), "height": 2.0,
     "rotation": 0.0},
]
LAYERS = {"cizim": (False, True), "kilitli": (True, True), "gizli": (False, False), "0": (False, True)}


def setup(profile=None):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"}
    if profile:
        settings["topology"] = {"tolerance": 0.001, "rules": [], "profile": profile}
    return {"format": "kentos.document", "version": 2, "name": "Topolojik temizlik raporu", "settings": settings,
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True),
                       layer("gizli", "Gizli", visible=False)],
            "activeLayer": "cizim", "entities": copy.deepcopy(ENTS), "uids": [uid(i + 1) for i in range(len(ENTS))],
            "styles": {"items": [], "categories": []}}


CORRECTED = {"line", "polyline", "arc", "polygon"}


def topo(e, fixed):
    """The object as the clean-up takes it (docs/adr/0148 §2)."""
    if e["kind"] == "line":
        return {"kind": "line", "fixed": fixed, "paths": [{"pts": [e["a"], e["b"]], "closed": False,
                                                           "zs": [e.get("za"), e.get("zb")]}]}
    if e["kind"] in ("polyline", "polygon"):
        n = len(e["pts"])
        p = {"pts": list(e["pts"]), "closed": e["kind"] == "polygon" or bool(e.get("closed")),
             "zs": list(e.get("zs") or [None] * n)}
        if any(b != 0.0 for b in e.get("bulges") or []):
            p["bulges"] = list(e["bulges"])
        return {"kind": "area" if e["kind"] == "polygon" else "polyline", "fixed": fixed, "paths": [p]}
    if e["kind"] == "point":
        return {"kind": "point", "fixed": True, "paths": [{"pts": [e["p"]], "closed": False, "zs": [e.get("z")]}]}
    return None


def edge_len(a, b, bulge):
    c = math.hypot(b["x"] - a["x"], b["y"] - a["y"])
    if not bulge:
        return c
    t = 4.0 * math.atan(abs(bulge))
    return c / (2.0 * math.sin(t / 2.0)) * t


def length(p):
    pts, n = p["pts"], len(p["pts"])
    bs = p.get("bulges") or [0.0] * n
    m = n if p["closed"] else n - 1
    return sum(edge_len(pts[i], pts[(i + 1) % n], bs[i]) for i in range(m))


def shoelace(p):
    """Exactly, in rationals of the floats given: at map coordinates a float shoelace loses the digits."""
    pts = [(Fraction(q["x"]), Fraction(q["y"])) for q in p["pts"]]
    s = sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts)))
    return float(abs(s) / 2)


def same_path(a, b):
    def norm(p):
        bs = p.get("bulges")
        return ([(q["x"], q["y"]) for q in p["pts"]], p["closed"], list(p["zs"]),
                None if not bs or all(v == 0.0 for v in bs) else list(bs))
    return norm(a) == norm(b)


def report(profile=None, uids=None, tolerance=0.05, works=None, ents=None):
    ents = ents if ents is not None else ENTS
    works = dict({"ends": True, "vertices": False, "extend": True, "trim": True}, **(works or {}))
    is_open = lambda e: e["kind"] in CORRECTED and LAYERS[e["layerId"]] == (False, True)  # noqa: E731
    skipped = 0
    if uids is None:
        targets = {e["id"] for e in ents if is_open(e)}
    else:
        targets = set()
        for u in uids:
            e = ents[int(u[-12:], 16) - 1]
            if is_open(e):
                targets.add(e["id"])
            else:
                skipped += 1
    objects, taken = [], []
    for e in ents:
        if not LAYERS[e["layerId"]][1]:
            continue
        o = topo(e, e["id"] not in targets)
        if o:
            objects.append(o)
            taken.append(e)
    r = clean(copy.deepcopy(objects), tolerance, works)
    moves = [{"uid": uid(taken[c["object"]]["id"]), "kind": c["kind"], "from": c["from"], "to": c["to"],
              "distance": math.hypot(c["from"]["x"] - c["to"]["x"], c["from"]["y"] - c["to"]["y"])}
             for c in r["changes"]]
    changes, area_change = [], 0.0
    for ch in r["changed"]:
        o = objects[ch["object"]]
        if all(same_path(a, b) for a, b in zip(o["paths"], ch["paths"])):
            continue
        e = taken[ch["object"]]
        before, after = o["paths"][0], ch["paths"][0]
        now = {(q["x"], q["y"]) for q in after["pts"]}
        own = [m["distance"] for m in moves if m["uid"] == uid(e["id"])]
        c = {"uid": uid(e["id"]), "layer": e["layerId"]}
        if e["kind"] == "polygon":
            c["areaBefore"], c["areaAfter"] = shoelace(before), shoelace(after)
            area_change += c["areaAfter"] - c["areaBefore"]
        else:
            c["lengthBefore"], c["lengthAfter"] = length(before), length(after)
        c.update({"kept": sum(1 for q in before["pts"] if (q["x"], q["y"]) in now), "moves": len(own),
                  "maxMoved": max(own, default=0.0), "totalMoved": sum(own)})
        changes.append(c)
    acceptance = profile["acceptance"] if profile else None
    out = {"objects": len(targets), "supports": sum(1 for o in objects if o["fixed"]), "skipped": skipped,
           "counts": r["counts"], "moves": moves, "changes": changes, "moved": len(moves),
           "maxMoved": max((m["distance"] for m in moves), default=0.0),
           "totalMoved": sum(m["distance"] for m in moves), "areaChange": area_change}
    if acceptance is not None:
        out["acceptance"] = acceptance
    out["beyond"] = sum(1 for m in moves if acceptance is not None and m["distance"] > acceptance)
    out["revision"] = "$current"
    return out


SAME = {"revision": "same", "canUndo": False}


def input_of(uids=None, tolerance=0.05, works=None):
    i = {"tolerance": tolerance}
    if uids is not None:
        i["uids"] = uids
    i.update(works or {})
    return i


def planned(name, op="plan", profile=None, **kw):
    out = report(profile=profile, **kw)
    result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
    case = {"name": name, "steps": [{"op": op, "input": input_of(**kw), "result": result, "expect": SAME,
                                     "within": 1e-9}]}
    if profile:
        case["setup"] = setup(profile)
    return case


def beyond_text(out, profile):
    far = max(out["moves"], key=lambda m: m["distance"])
    return (f"Temizlik {out['beyond']} köşeyi kabul eşiğinden uzağa taşıyor; en uzağı “{far['uid']}” nesnesinde "
            f"{far['distance']:.4f} m, kalite profilinin (“{profile['name']}”) kabul eşiği {profile['acceptance']} m. "
            "Hiçbir şey yazılmadı: toleransı kabul eşiğine indirin ya da o köşeleri ölçüyle düzeltin.")


def refused(name, input_, code, message, path_, op="execute", profile=None, status="failed"):
    case = {"name": name, "steps": [{"op": op, "input": input_, "result": {"status": status, "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}
    if profile:
        case["setup"] = setup(profile)
    return case


def cases():
    strict = dict(PROFILE, acceptance=0.03)
    whole = report()
    b_after = whole["moves"][0]["to"]
    assert whole["moves"][0]["uid"] == uid(2) and whole["moves"][0]["kind"] == "end"
    out = []
    out.append(planned("plan: bütün çizim, ilk değerlerle; her taşınma, nesne başına özet, hiçbir şey yazılmaz"))
    out.append(planned("plan: Köşeler açık; parsellerin ortak köşeleri birleşir, alan değişimi söylenir",
                       works={"vertices": True}))
    out.append(planned("plan: yalnız B düzeltilir; öbürleri dayanak", uids=[uid(2)]))
    out.append(planned("plan: kilitli, gizli ve yazı verilince skipped sayılır", uids=[uid(9), uid(11), uid(15), uid(2)]))
    out.append(planned("plan: küçük toleransla düzeltilecek bir şey yok", tolerance=0.001))
    out.append(planned("plan: dört iş kapalı", works={"ends": False, "vertices": False, "extend": False, "trim": False}))
    out.append(planned("plan: kabul eşiğini aşan taşınmalar sayılır, plan reddetmez", profile=strict))
    out.append(planned("plan: kabul eşiği 0,05 m, aşan yok", profile=PROFILE))
    out.append(planned("doğrulama: ilk değerlerle geçer", op="validate"))
    ex = report()
    out.append({"name": "uygula: tek geri alma adımı; B'nin başı A'nın ucunda", "steps": [
        {"op": "execute", "input": input_of(), "result": {"status": "completed", "output": ex, "warnings": []},
         "within": 1e-9, "expect": {"revision": "changed", "canUndo": True, "fields": {"2": {"a": b_after}}}},
        {"op": "undo", "returns": "Topolojik temizlik", "expect": {"fields": {"2": {"a": ENTS[1]["a"]}}}}]})
    nothing = report(tolerance=0.001)
    out.append({"name": "uygula: düzeltilecek bir şey yoksa hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": input_of(tolerance=0.001),
         "result": {"status": "completed", "output": nothing, "warnings": []}, "within": 1e-9, "expect": SAME}]})
    ok_profile = report(profile=PROFILE)
    out.append({"name": "uygula: kabul eşiği 0,05 m, geçer", "setup": setup(PROFILE), "steps": [
        {"op": "execute", "input": input_of(), "result": {"status": "completed", "output": ok_profile, "warnings": []},
         "within": 1e-9, "expect": {"revision": "changed", "canUndo": True}}]})
    out.append(refused("uygula: kabul eşiği 0,03 m, aşan taşınma bütün temizliği reddeder", input_of(),
                       "clean_beyond_acceptance", beyond_text(report(profile=strict), strict), "tolerance",
                       profile=strict))
    out.append(refused("doğrulama: kabul eşiğini aşan reddedilir", input_of(), "clean_beyond_acceptance",
                       beyond_text(report(profile=strict), strict), "tolerance", op="validate", profile=strict))
    out.append(refused("tolerans sıfır", input_of(tolerance=0.0), "invalid_input",
                       "Tolerans sonlu ve en az 0.000001 m olmalı (yazılan 0).", "tolerance"))
    out.append(refused("tolerans eksi", input_of(tolerance=-0.01), "invalid_input",
                       "Tolerans sonlu ve en az 0.000001 m olmalı (yazılan -0.01).", "tolerance"))
    out.append(refused("kimlik UUID değil", input_of(uids=["x"]), "invalid_uid", "“x” geçerli bir nesne kimliği değil.",
                       "uids[0]"))
    out.append(refused("çizimde olmayan nesne", input_of(uids=[uid(2), uid(99)]), "entity_not_found",
                       f"“{uid(99)}” kimlikli nesne çizimde yok.", "uids[1]"))
    out.append(refused("sürüm yazısı geçersiz", dict(input_of(), expectedRevision="abc"), "invalid_revision",
                       "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                       "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    out.append({"name": "beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": dict(
        input_of(), expectedRevision="7"), "result": {"status": "conflict", "error": {
            "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
            "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
            "revision": "$current"}}, "expect": SAME}]})
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.topology.clean", "commandVersion": 1,
           "title": "Topolojik temizlik (taşınma raporu)",
           "note": "ADR 1262. Bağımsız başvuru scripts/fixtures/topology_clean_cases.py: temizlik ADR 0148'in bağımsız "
                   "başvurusuyla (topology_cases.py), rapor elle; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Topolojik temizlik ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
