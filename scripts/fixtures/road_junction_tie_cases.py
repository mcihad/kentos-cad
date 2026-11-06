#!/usr/bin/env python3
"""The shared cases of `road.junctions.tie` (plan/road.md Y17.5, docs/adr/0918; ROAD-JUNC-03).

The roads here are straight (two PIs each): where the secondary centreline crosses the primary road's edges is the
line's intersection with the edge line, a place's station and offset its projection; the profiles are
road_vertical_cases.py's reference (exact fractions, a circle in 40-digit mpmath); a superelevation's slope between
rows piecewise linear, its end values beyond them (docs/adr/0904). The tie is ADR 0918 §2's: three PVIs without
curves at the crossing and the two edges, the primary elevation there plus the slope times the edge's width, the
secondary PVIs between the edges taken off. No KentOS code. Slopes, widths and limits are the cases' own.

    uv run --no-project --with mpmath python3 scripts/fixtures/road_junction_tie_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_grades  # noqa: E402
import road_junction_clearance_cases as cref  # noqa: E402
import road_vertical_cases as vref  # noqa: E402

OUT = cref.OUT
W = 1e-6
completed, failed, Refused, Line, solved = cref.completed, cref.failed, cref.Refused, cref.Line, cref.solved
UNCHANGED = cref.UNCHANGED
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
    "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}
BAD_REVISION = failed("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
NOT_ROAD = cref.NOT_ROAD
al, pv, prof, leg, jn = cref.al, cref.pv, cref.prof, cref.leg, cref.jn

ALIGNMENTS = [
    al("g-1", "Ana yol", 1, (0, 0), (500, 0)),
    al("g-2", "Tali yol", 1, (250, -200), (250, 200)),
    al("g-3", "Eğik tali", 2, (150, -100), (350, 100)),
    al("g-5", "Kısa tali", 1, (100, -30), (100, 40)),
    al("g-6", "Kenarda başlayan", 1, (300, -3), (300, 60)),
    al("g-7", "Ters tali", 1, (400, 120), (400, -120)),
]
PROFILES = [
    prof("b-1", "Ana kot", "g-1", 1, [pv("a", 0, 100), pv("v1", 250, 102, length=100), pv("b", 500, 101)]),
    prof("b-2", "Tali kot", "g-2", 1, [pv("a", 0, 98), pv("v1", 120, 99, radius=3000), pv("v2", 200, 100.5, length=20),
                                       pv("v3", 280, 99.5, length=40), pv("b", 400, 97)]),
    prof("b-8", "Çakışan kot", "g-2", 1, [pv("a", 0, 98), pv("j-1.a", 50, 98.5), pv("b", 400, 97)]),
    prof("b-9", "Bozuk kot", "g-2", 1, [pv("a", 0, 98), pv("v1", 185, 99, length=36), pv("v3", 215, 99.5, length=36),
                                        pv("b", 400, 97)]),
    prof("b-3", "Eğik kot", "g-3", 1, [pv("a", 0, 99), pv("v1", 141, 101, length=30), pv("b", 282, 100)]),
    prof("b-5", "Kısa kot", "g-5", 1, [pv("a", 0, 100.2), pv("b", 70, 101.5)]),
    prof("b-6", "Geç başlayan", "g-5", 1, [pv("a", 27, 100.2), pv("b", 70, 101.5)]),
    prof("b-10", "Kenar kot", "g-6", 1, [pv("a", 0, 101), pv("b", 63, 103)]),
    prof("b-11", "Ters kot", "g-7", 1, [pv("a", 0, 103), pv("v1", 120, 100.9, length=10), pv("b", 240, 102)]),
    prof("b-12", "Taşan kot", "g-7", 1, [pv("a", 0, 103), pv("b", 300, 102)]),
]


def sup(id_, name, alignment, rev, nl, nr, rows):
    return {"id": id_, "name": name, "alignment": alignment, **({"alignmentRevision": rev} if rev else {}), "revision": 1,
            "normalLeft": nl, "normalRight": nr,
            "method": {"method": "stations", "rows": [{"station": float(s), "left": float(l), "right": float(r)}
                                                      for s, l, r in rows]}}


SUPERS = [
    sup("d-1", "Ana dever", "g-1", 1, -2.5, -2.5, [(150, -2.5, -2.5), (240, 3.0, -3.0), (420, 3.0, -3.0)]),
    sup("d-2", "Tali dever", "g-2", 1, -2.0, -2.0, [(100, -2.0, -2.0)]),
    sup("d-3", "Eski dever", "g-1", 0, -2.0, -2.0, [(0, -2.0, -2.0)]),
    sup("d-4", "Bozuk dever", "g-1", 1, -2.0, -2.0, [(100, -2.0, -2.0), (900, -2.0, -2.0)]),
]

JUNCTION_LIST = [
    jn("j-1", "Dörtyol", "atGrade", [leg("g-1", 1, 5, 5), leg("g-2", 1, 3.5, 3.5)], radius=10.0),
    jn("j-2", "Eğik kavşak", "atGrade", [leg("g-3", 1, 3, 4), leg("g-1", 1, 5, 6)], radius=12.0),
    jn("j-3", "Kısa kavşak", "atGrade", [leg("g-1", 1, 5, 5), leg("g-5", 1, 3, 3)], radius=8.0),
    jn("j-4", "Kenar kavşağı", "atGrade", [leg("g-1", 1, 5, 5), leg("g-6", 1, 3, 3)], radius=8.0),
    jn("j-5", "Üst geçit", "separated", [leg("g-1", 1, 5, 5), leg("g-2", 1, 3.5, 3.5)]),
    jn("j-6", "Ters kavşak", "atGrade", [leg("g-7", 1, 3, 3), leg("g-1", 1, 5, 5)], radius=10.0),
]


def setup(work="road"):
    s = cref.setup(work)
    if work == "road":
        s["settings"]["road"].update(alignments=copy.deepcopy(ALIGNMENTS), profiles=copy.deepcopy(PROFILES),
                                     superelevations=copy.deepcopy(SUPERS), junctions=copy.deepcopy(JUNCTION_LIST))
    return s


LINES = {a["id"]: Line(a) for a in ALIGNMENTS}


def crossing(a, b):
    """The stations of two lines where they cross."""
    la, lb = LINES[a], LINES[b]
    w = (lb.p[0] - la.p[0], lb.p[1] - la.p[1])
    den = la.t[0] * -lb.t[1] - la.t[1] * -lb.t[0]
    return (w[0] * -lb.t[1] - w[1] * -lb.t[0]) / den, (la.t[0] * w[1] - la.t[1] * w[0]) / den


def along(points, s, none):
    """A superelevation's slope at s (docs/adr/0904): linear between rows, the end values beyond."""
    if not points:
        return none
    if s <= points[0][0]:
        return points[0][1]
    if s >= points[-1][0]:
        return points[-1][1]
    for (a, va), (b, vb) in zip(points, points[1:]):
        if a <= s < b:
            return vb if b == a else va + (vb - va) * (s - a) / (b - a)
    return points[-1][1]


def bad(words, path):
    return Refused("invalid_tie", words, path)


def find(list_, name, path, code, what):
    for x in list_:
        if x["id"] == name:
            return x
    for x in list_:
        if x["name"].lower() == name.lower():
            return x
    raise Refused(code, f"“{name}” kimlikli ya da adlı {what} projede yok.", path)


def vstop(p):
    """The profile built, or its refusal (docs/adr/0902's words)."""
    length = LINES[p["alignment"]].length
    try:
        at = solved(p)[1]
    except vref.Refused as r:
        kw = r.kw
        if kw["kind"] == "Overlap":
            ids = [v["id"] for v in p["pvis"]]
            carries = any(k in p["pvis"][ids.index(kw["b"])] for k in ("length", "radius"))
            who = kw["b"] if carries else kw["a"]
            raise Refused("profile_curves_overlap",
                          f"{kw['a']}–{kw['b']} arasına düşey kurplar sığmıyor: kolları {kw['excess']:.3f} m uzun. Kurp "
                          "boyunu ya da yarıçapı küçültün, PVI'ları uzaklaştırın.", f"pvis[{ids.index(who)}]")
        raise
    for i, v in enumerate(p["pvis"]):
        if v["station"] < -1e-9 or v["station"] > length + 1e-9:
            raise Refused("invalid_road_profile", f"{v['id']} PVI'si aksın dışında: aksın km'si 0 ile {length:.3f} m arası.",
                          f"pvis[{i}].station")
    return at


def answer(inp, road=True):
    if not road:
        raise Refused(NOT_ROAD["error"]["code"], NOT_ROAD["error"]["message"])
    j = find(JUNCTION_LIST, inp["junction"], "junction", "unknown_junction", "kavşak")
    warns = []
    a_ids = [lg["alignment"] for lg in j["legs"]]
    for lg in j["legs"]:
        a = next(x for x in ALIGNMENTS if x["id"] == lg["alignment"])
        if a["revision"] != lg["alignmentRevision"]:
            warns.append({"code": "junction_stale", "message": f"“{j['name']}” kavşağının “{a['name']}” aksı değişti; "
                          "kavşağı denetleyip yeniden bağlayın.", "path": "junction"})
    if j["kind"] == "separated":
        raise Refused("invalid_junction", f"“{j['name']}” ayrı seviyeli kavşak; boykesit bağı yalnız hemzemin kavşakta "
                                          "olur.", "junction")
    if "profiles" in inp:
        chosen = [find(PROFILES, n, f"profiles[{k}]", "unknown_road_profile", "boykesit")
                  for k, n in enumerate(inp["profiles"])]
    else:
        chosen = []
        for aid in a_ids:
            on = [p for p in PROFILES if p["alignment"] == aid]
            name = next(a["name"] for a in ALIGNMENTS if a["id"] == aid)
            if not on:
                raise bad(f"“{name}” aksının boykesiti yok; profiles ile verin.", "profiles")
            if len(on) > 1:
                raise bad(f"“{name}” aksının {len(on)} boykesiti var; hangisi, profiles ile verin.", "profiles")
            chosen.append(on[0])
    built = [vstop(p) for p in chosen]
    if len(chosen) != 2:
        raise bad(f"Kolların sırasıyla iki boykesit verin; {len(chosen)} verildi.", "profiles")
    for k, (p, aid) in enumerate(zip(chosen, a_ids)):
        if p["alignment"] != aid:
            raise bad(f"“{p['name']}” boykesiti kavşağın {k + 1}. kolunun aksında değil.", f"profiles[{k}]")
    pri = inp["primary"]
    if pri > 1:
        raise bad("Sırtı korunan kol (primary) 0 ya da 1 olmalı.", "primary")
    sec = 1 - pri
    cf, sname = inp.get("crossfall"), inp.get("superelevation")
    if (cf is None) == (sname is None):
        raise bad("Ana yolun enine eğimini ya crossfall ya da superelevation ile verin; biri verilir.", "crossfall")
    if cf is not None:
        def slopes(_s, cf=cf):
            return cf[0], cf[1]
    else:
        d = find(SUPERS, sname, "superelevation", "unknown_superelevation", "dever çizelgesi")
        length = LINES[d["alignment"]].length
        for i, r in enumerate(d["method"]["rows"]):
            if r["station"] < -1e-9 or r["station"] > length + 1e-9:
                raise Refused("invalid_superelevation", f"{i + 1}. satır aksın dışında: aksın km'si 0 ile {length:.3f} m "
                                                        "arası.", f"method.rows[{i}].station")
        if d["alignment"] != a_ids[pri]:
            raise bad(f"“{d['name']}” dever çizelgesi ana yolun aksında değil.", "superelevation")
        now = next(a["revision"] for a in ALIGNMENTS if a["id"] == d["alignment"])
        if now != d.get("alignmentRevision", 0):
            warns.append({"code": "superelevation_stale",
                          "message": f"“{d['name']}” dever çizelgesinin aksı değişti (çizelge {d.get('alignmentRevision', 0)}. "
                                     f"revizyona göre kurulu, aks şimdi {now}.); km'leri denetleyip aksa yeniden "
                                     "bağlayın.", "path": "superelevation"})
        L = [(r["station"], r["left"]) for r in d["method"]["rows"]]
        R = [(r["station"], r["right"]) for r in d["method"]["rows"]]

        def slopes(s, L=L, R=R, d=d):
            return along(L, s, d["normalLeft"]), along(R, s, d["normalRight"])
    most = inp.get("maxGradeChange")
    if most is not None and not most >= 0:
        raise bad("En büyük eğim değişimi (maxGradeChange) 0 ya da daha büyük olmalı.", "maxGradeChange")
    for p in chosen:
        now = next(a["revision"] for a in ALIGNMENTS if a["id"] == p["alignment"])
        if now != p["alignmentRevision"]:
            warns.append({"code": "profile_stale",
                          "message": f"“{p['name']}” boykesitinin aksı değişti (boykesit {p['alignmentRevision']}. "
                                     f"revizyona göre kurulu, aks şimdi {now}.); PVI'ları denetleyip aksa yeniden "
                                     "bağlayın.", "path": "profiles"})
    P, Q = LINES[a_ids[pri]], LINES[a_ids[sec]]
    redp, redq = built[pri], built[sec]
    sa, sb = crossing(a_ids[0], a_ids[1])
    sp, sq = (sa, sb) if pri == 0 else (sb, sa)
    left, right = j["legs"][pri]["left"], j["legs"][pri]["right"]
    pname, qname = chosen[pri]["name"], chosen[sec]["name"]

    def uncovered(name):
        return bad(f"“{name}” boykesiti bağı örtmüyor.", "profiles")

    def red(f, s):
        r = f(s)
        return None if r is None else r[0]

    z0 = red(redp, sp)
    if z0 is None:
        raise uncovered(pname)
    ties = [{"station": sq, "primaryStation": sp, "primaryOffset": 0.0, "crossfall": 0.0, "elevation": z0}]
    for k, edge in enumerate((-left, right)):
        # The secondary line's station where its offset on the primary line is the edge's: offset(s) is linear.
        o0 = P.locate(Q.place(0.0, 0.0))
        o1 = P.locate(Q.place(1.0, 0.0))
        rate = o1[1] - o0[1]
        s = (edge - o0[1]) / rate
        if not -1e-9 <= s <= Q.length + 1e-9:
            raise bad(f"Tali yolun ekseni ana yolun {'sol' if k == 0 else 'sağ'} kenarını kesmiyor.", "junction")
        foot = P.locate(Q.place(s, 0.0))
        slope = slopes(foot[0])[k]
        width = left if k == 0 else right
        zp = red(redp, foot[0])
        if zp is None:
            raise uncovered(pname)
        ties.append({"station": s, "primaryStation": foot[0], "primaryOffset": foot[1], "crossfall": slope,
                     "elevation": zp + slope / 100 * width})
    ties.sort(key=lambda t: t["station"])
    sdef = chosen[sec]
    lo, hi = ties[0]["station"] - 1e-6, ties[2]["station"] + 1e-6
    if sdef["pvis"][0]["station"] >= lo or sdef["pvis"][-1]["station"] <= hi:
        raise bad(f"“{qname}” boykesiti bağı örtmüyor: ilk ve son PVI'sı ana yolun iki kenarının dışında olmalı.",
                  "profiles")
    out_ties = []
    for t, suffix in zip(ties, "axb"):
        before = red(redq, t["station"])
        if before is None:
            raise uncovered(qname)
        out_ties.append({"pvi": f"{j['id']}.{suffix}", **t, "before": before, "difference": t["elevation"] - before})
    inside = [v for v in sdef["pvis"] if lo <= v["station"] <= hi]
    for t in out_ties:
        if any(v["id"] == t["pvi"] for v in sdef["pvis"] if not lo <= v["station"] <= hi):
            raise bad(f"“{t['pvi']}” kimlikli PVI boykesitte zaten var.", "profiles")
    kept_before = [v for v in sdef["pvis"] if v["station"] < lo]
    kept_after = [v for v in sdef["pvis"] if v["station"] > hi]
    new = [{"id": t["pvi"], "station": t["station"], "elevation": t["elevation"]} for t in out_ties]
    tied = {**sdef, "pvis": kept_before + new + kept_after}
    vstop(tied)
    pts = [(kept_before[-1]["station"], kept_before[-1]["elevation"])] + \
          [(t["station"], t["elevation"]) for t in out_ties] + [(kept_after[0]["station"], kept_after[0]["elevation"])]
    grades = [(b[1] - a[1]) / (b[0] - a[0]) * 100 for a, b in zip(pts, pts[1:])]
    changes = [abs(b - a) for a, b in zip(grades, grades[1:])]
    if most is not None:
        for t, c in zip(out_ties, changes):
            if c > most:
                warns.append({"code": "junction_grade_change",
                              "message": f"“{t['pvi']}” PVI'sinde eğim değişimi %{c:.3f}, en çok %{most:.3f} istendi.",
                              "path": "maxGradeChange"})
    return {"junction": j["id"], "primary": pri, "profile": sdef["id"], "ties": out_ties,
            "removed": [v["id"] for v in inside], "grades": grades, "changes": changes, "revision": "$any"}, warns, tied


def cases():
    out = []

    def q(name, inp, op="execute", map_=False, after=None):
        try:
            o, w, tied = answer(inp, road=not map_)
            result = completed(o, w)
            expect = STEPPED if op == "execute" else UNCHANGED
        except Refused as r:
            result, expect, tied = r.result, UNCHANGED, None
        steps = [{"op": op, "input": inp, "result": result, "expect": expect, "within": W}]
        if after == "undo" and tied is not None:
            road = setup()["settings"]["road"]
            written = copy.deepcopy(road)
            written["profiles"] = [{**tied, "revision": tied["revision"] + 1} if p["id"] == tied["id"] else p
                                   for p in written["profiles"]]
            steps[0]["expect"] = {**expect, "road": written}
            steps.append({"op": "undo", "returns": "Kavşağa bağla", "expect": {"road": road, "canRedo": True},
                          "within": W})
        c = {"name": name, "steps": steps}
        if map_:
            c["setup"] = "$map"
        out.append(c)

    q("Dörtyol, ana yolun sırtı korunur", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5]}, after="undo")
    q("Adıyla, eğim değişimi sınırıyla", {"junction": "dörtyol", "primary": 0, "profiles": ["b-1", "Tali kot"],
                                          "crossfall": [-2.0, -3.0], "maxGradeChange": 30.0})
    q("Eğim değişimi aşılıyor", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5], "maxGradeChange": 1.0})
    q("Planı yazmaz", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5]}, op="plan")
    q("Deverle", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "superelevation": "d-1"})
    q("Eski deverle", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "superelevation": "Eski dever"})
    q("Eğik kavşak, ana yol ikinci kol", {"junction": "j-2", "primary": 1, "crossfall": [-2.0, -2.5]})
    q("Kısa tali yol", {"junction": "j-3", "primary": 0, "profiles": ["b-1", "b-5"], "crossfall": [-2.5, -2.5]})
    q("Ters yönlü tali yol", {"junction": "j-6", "primary": 1, "profiles": ["b-11", "b-1"], "crossfall": [-2.5, -2.5]})
    q("Tali yolun sırtı korunur", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 1, "crossfall": [-2.0, -2.0]})
    # Refusals.
    q("Olmayan kavşak", {"junction": "j-9", "primary": 0, "crossfall": [-2.5, -2.5]})
    q("Ayrı seviyeli kavşak", {"junction": "j-5", "primary": 0, "crossfall": [-2.5, -2.5]})
    q("Olmayan boykesit", {"junction": "j-1", "primary": 0, "profiles": ["yok", "b-2"], "crossfall": [-2.5, -2.5]})
    q("Aksın iki boykesiti", {"junction": "j-3", "primary": 0, "crossfall": [-2.5, -2.5]})
    q("Kurulamayan boykesit", {"junction": "j-6", "primary": 1, "profiles": ["b-12", "b-1"], "crossfall": [-2.5, -2.5]})
    q("Kurpları çakışan boykesit", {"junction": "j-1", "primary": 0, "profiles": ["b-1", "b-9"],
                                    "crossfall": [-2.5, -2.5]})
    q("Boykesit kolun aksında değil", {"junction": "j-1", "primary": 0, "profiles": ["b-2", "b-1"],
                                       "crossfall": [-2.5, -2.5]})
    q("Üçüncü kol", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 2, "crossfall": [-2.5, -2.5]})
    q("Eğim iki kez", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5], "superelevation": "d-1"})
    q("Eğim yok", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0})
    q("Olmayan dever", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "superelevation": "yok"})
    q("Kurulamayan dever", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "superelevation": "d-4"})
    q("Dever tali yolun", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "superelevation": "Tali dever"})
    q("Eksi eğim değişimi", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5], "maxGradeChange": -1.0})
    q("Kenarı kesmiyor", {"junction": "j-4", "primary": 0, "crossfall": [-2.5, -2.5]})
    q("Boykesit bağı örtmüyor", {"junction": "j-3", "primary": 0, "profiles": ["b-1", "b-6"],
                                 "crossfall": [-2.5, -2.5]})
    q("PVI kimliği alınmış", {"junction": "j-1", "primary": 0, "profiles": ["b-1", "b-8"], "crossfall": [-2.5, -2.5]})
    q("Geçersiz beklenen sürüm", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5], "expectedRevision": "on"})
    q("Eski sürüme göre", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5], "expectedRevision": "41"})
    q("Yol projesi değil", {"junction": "j-1", "profiles": ["b-1", "b-2"], "primary": 0, "crossfall": [-2.5, -2.5]}, map_=True)
    for c in out:
        r = c["steps"][0]["result"]
        if c["name"] == "Geçersiz beklenen sürüm":
            c["steps"][0]["result"], c["steps"][0]["expect"] = BAD_REVISION, UNCHANGED
        if c["name"] == "Eski sürüme göre":
            c["steps"][0]["result"], c["steps"][0]["expect"] = CONFLICT, UNCHANGED
        if c["steps"][0]["op"] == "plan" and r["status"] == "completed":
            r["output"]["revision"] = "$any"
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="map")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.junctions.tie", "commandVersion": 1,
           "title": "Kavşağa bağla: tali yolun boykesiti ana yolun sırtına",
           "note": "scripts/fixtures/road_junction_tie_cases.py'den; düz aksların doğrularında kenarların kesişimi, "
                   "boykesitler road_vertical_cases.py'nin kesin kesirleri ve dairesi, dever km satırlarından doğrusal, "
                   "bağ ADR 0918 §2'nin kuralıyla; KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    doc = road_grades.fixture(doc)
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.junctions.tie.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.junctions.tie.json")
    else:
        path.write_text(text)
    print(f"road.junctions.tie.json: {len(found)} durum")


if __name__ == "__main__":
    main()
