#!/usr/bin/env python3
"""The shared cases of `road.junctions.edit`, `.query` and `.draw` (plan/road.md Y12.2, docs/adr/0910 §2; KentOS Yol's
junctions).

The alignments are road_alignment_cases.py's 40-digit reference; the crossing (both alignments sampled every metre,
crossing segments refined by Newton's method on both stations, here in 40 digits), the legs taken along their tangents
at the crossing, each quadrant's curb return (the fillet tangent to the two legs' edges, outward of both) and the
warnings are ADR 0910 §2's. No KentOS code. The edges and radii are the cases' own, not a standard's.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_junction_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = ccmd.UNCHANGED, ccmd.STEPPED, ccmd.CONFLICT, ccmd.BAD_REVISION
W = 1e-6
ALIGNED = ccmd.pcmd.ALIGNED
NOT_ROAD = failed("not_a_road_project", "Kavşak yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
QUADRANTS = [("right", "right"), ("right", "left"), ("left", "left"), ("left", "right")]
ALIGNMENTS = grc.setup([])["settings"]["road"]["alignments"]

J1 = {"id": "j-1", "name": "Ana kavşak", "revision": 1, "kind": "atGrade", "radius": 12.0,
      "legs": [{"alignment": "g-1", "alignmentRevision": 1, "left": 5.0, "right": 5.0},
               {"alignment": "g-2", "alignmentRevision": 3, "left": 3.5, "right": 3.5}]}
J2 = {"id": "j-2", "name": "Üst geçit", "revision": 1, "kind": "separated",
      "legs": [{"alignment": "g-1", "alignmentRevision": 1, "left": 5.0, "right": 5.0},
               {"alignment": "g-2", "alignmentRevision": 2, "left": 3.5, "right": 3.5}]}
START = [J1, J2]


def road_of(junctions):
    out = ccmd.road_of(grc.CORRIDORS)
    if junctions:
        out["junctions"] = copy.deepcopy(list(junctions))
        for j in out["junctions"]:
            for leg in j["legs"]:
                if leg.get("alignmentRevision") == 0:
                    leg.pop("alignmentRevision")
    return out


def setup(junctions=START, work="road"):
    base = grc.setup([], work)
    if work == "road":
        base["settings"]["road"] = road_of(junctions)
    return base


FIRST = len(setup()["entities"]) + 1


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, words):
    return Refused("invalid_junction", words, path or None)


def find(list_, name, path="junction"):
    for j in list_:
        if j["id"] == name:
            return j
    for j in list_:
        if j["name"].lower() == name.lower():
            return j
    raise Refused("unknown_junction", f"“{name}” kimlikli ya da adlı kavşak projede yok.", path)


def alignment(name, path):
    for a in ALIGNMENTS:
        if a["id"] == name:
            return a
    for a in ALIGNMENTS:
        if a["name"].lower() == name.lower():
            return a
    raise Refused("unknown_alignment", f"“{name}” kimlikli ya da adlı aks projede yok.", path)


def at(aid, s):
    e, (x, y) = aref.place_at(ALIGNED[aid].els, mp.mpf(s))
    return e, mp.mpf(x), mp.mpf(y), e.theta(mp.mpf(s) - e.station)


def crossings(a, b):
    def sample(aid):
        length = ALIGNED[aid].length
        n = max(math.ceil(length / 1.0), 1)
        out = []
        for k in range(n + 1):
            s = min(k * 1.0, length)
            _, x, y, _ = at(aid, s)
            out.append((s, float(x), float(y)))
        return out
    pa, pb = sample(a), sample(b)
    found = []
    for (s0, x0, y0), (s1, x1, y1) in zip(pa, pa[1:]):
        for (u0, X0, Y0), (u1, X1, Y1) in zip(pb, pb[1:]):
            da, db = (x1 - x0, y1 - y0), (X1 - X0, Y1 - Y0)
            den = da[0] * db[1] - da[1] * db[0]
            if abs(den) < 1e-15:
                continue
            w = (X0 - x0, Y0 - y0)
            t = (w[0] * db[1] - w[1] * db[0]) / den
            u = (w[0] * da[1] - w[1] * da[0]) / den
            if not (-1e-9 <= t <= 1 + 1e-9 and -1e-9 <= u <= 1 + 1e-9):
                continue
            sa, sb = mp.mpf(s0 + t * (s1 - s0)), mp.mpf(u0 + u * (u1 - u0))
            for _ in range(60):
                _, xa, ya, tha = at(a, sa)
                _, xb, yb, thb = at(b, sb)
                f = (xa - xb, ya - yb)
                ta, tb = (mp.cos(tha), mp.sin(tha)), (mp.cos(thb), mp.sin(thb))
                d = -ta[0] * tb[1] + ta[1] * tb[0]
                dsa = (-f[0] * -tb[1] - -f[1] * -tb[0]) / d
                dsb = (ta[0] * -f[1] - ta[1] * -f[0]) / d
                sa, sb = sa + dsa, sb + dsb
                if abs(dsa) + abs(dsb) < mp.mpf(10) ** -30:
                    break
            sa, sb = float(sa), float(sb)
            if not any(abs(c[0] - sa) < 1e-6 and abs(c[1] - sb) < 1e-6 for c in found):
                found.append((sa, sb))
    return sorted(found)


def read(j):
    """The junction's report and warnings; Refused where it does not read."""
    legs_al = [alignment(leg["alignment"], "legs") for leg in j["legs"]]
    a, b = legs_al[0]["id"], legs_al[1]["id"]
    found = crossings(a, b)
    if not found:
        raise bad("legs", f"“{j['name']}” kavşağının kolları (“{legs_al[0]['name']}” ve “{legs_al[1]['name']}”) "
                          "kesişmiyor.")
    k = j.get("at", 0)
    if k >= len(found):
        raise bad("at", f"“{j['name']}” kavşağının kolları {len(found)} yerde kesişiyor; {k + 1}. kesişim yok.")
    sa, sb = found[k]
    _, xa, ya, tha = at(a, sa)
    _, _, _, thb = at(b, sb)
    ta, tb = (float(mp.cos(tha)), float(mp.sin(tha))), (float(mp.cos(thb)), float(mp.sin(thb)))
    if abs(ta[0] * tb[1] - ta[1] * tb[0]) < 1e-9:
        raise bad("legs", f"“{j['name']}” kavşağının kolları kesişimde paralel.")
    angle = math.degrees(math.acos(max(-1.0, min(1.0, ta[0] * tb[0] + ta[1] * tb[1]))))
    p = (float(xa), float(ya))
    warnings, stale, legs = [], False, []
    for al, s, th, leg in ((legs_al[0], sa, tha, j["legs"][0]), (legs_al[1], sb, thb, j["legs"][1])):
        changed = al.get("revision", 0) != leg.get("alignmentRevision", 0)
        if changed:
            stale = True
            warnings.append({"code": "junction_stale", "path": "junction",
                             "message": f"“{j['name']}” kavşağının “{al['name']}” aksı değişti; kavşağı denetleyip "
                                        "yeniden bağlayın."})
        legs.append({"alignment": al["id"], "station": s, "kmText": ccmd.km_text(al["id"], s),
                     "direction": math.degrees(float(th)), "stale": changed})
    returns = []
    if j["kind"] == "atGrade":
        reach = [[sa], [sb]]
        for k, (wa, wb) in enumerate(QUADRANTS):
            r = j["radii"][k] if j.get("radii") else j["radius"]
            (ka, oa) = (1.0, j["legs"][0]["right"]) if wa == "right" else (-1.0, -j["legs"][0]["left"])
            (kb, ob) = (1.0, j["legs"][1]["right"]) if wb == "right" else (-1.0, -j["legs"][1]["left"])
            na, nb = (ta[1], -ta[0]), (tb[1], -tb[0])
            ca, cb = oa + ka * r, ob + kb * r
            w = (cb * nb[0] - ca * na[0], cb * nb[1] - ca * na[1])
            den = ta[0] * -tb[1] - ta[1] * -tb[0]
            u = (w[0] * -tb[1] - w[1] * -tb[0]) / den
            c = (p[0] + ca * na[0] + u * ta[0], p[1] + ca * na[1] + u * ta[1])
            on_a = (c[0] - ka * r * na[0], c[1] - ka * r * na[1])
            on_b = (c[0] - kb * r * nb[0], c[1] - kb * r * nb[1])
            va, vb = (on_a[0] - c[0], on_a[1] - c[1]), (on_b[0] - c[0], on_b[1] - c[1])
            sweep = math.atan2(va[0] * vb[1] - va[1] * vb[0], va[0] * vb[0] + va[1] * vb[1])
            qa = sa + (on_a[0] - p[0]) * ta[0] + (on_a[1] - p[1]) * ta[1]
            qb = sb + (on_b[0] - p[0]) * tb[0] + (on_b[1] - p[1]) * tb[1]
            reach[0].append(qa)
            reach[1].append(qb)
            returns.append({"a": wa, "b": wb, "radius": r, "cx": c[0], "cy": c[1],
                            "onA": {"x": on_a[0], "y": on_a[1], "station": qa},
                            "onB": {"x": on_b[0], "y": on_b[1], "station": qb},
                            "sweep": math.degrees(sweep), "length": r * abs(sweep)})
        for al, stations in zip(legs_al, reach):
            lo, hi = min(stations), max(stations)
            curved = any(float(e.station) < hi and float(e.station + e.length) > lo
                         and abs(float(e.curvature(e.length / 2))) > 1e-12 for e in ALIGNED[al["id"]].els)
            if curved:
                warnings.append({"code": "junction_on_curve", "path": "legs",
                                 "message": f"“{j['name']}” kavşağının “{al['name']}” kolu dönüş yaylarının erimi "
                                            "içinde kurpta; yaylar kolu düz kabul eder."})
    report = {"id": j["id"], "name": j["name"], "revision": j["revision"], "kind": j["kind"],
              "connected": j["kind"] == "atGrade", "stale": stale, "x": p[0], "y": p[1], "legs": legs,
              "angle": angle, "returns": returns}
    return report, warnings


def broken(j):
    return {"id": j["id"], "name": j["name"], "revision": j["revision"], "kind": j["kind"],
            "connected": j["kind"] == "atGrade", "stale": False, "x": 0.0, "y": 0.0, "legs": [], "angle": 0.0,
            "returns": []}


def listed(list_):
    return [{"id": j["id"], "name": j["name"], "kind": j["kind"], "revision": j["revision"]} for j in list_]


def rule_problem(list_):
    names = set()

    def no(field, words):
        raise bad(field, f"Kavşak kurallarına uymuyor: {words}.")
    size = lambda v: 0 < v <= 1000  # noqa: E731
    for j in list_:
        name = j["name"]
        if not name.strip() or name.strip() != name or len(name) > 120:
            no("name", "kavşağın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter")
        if name.lower() in names:
            no("name", "bu adda başka bir kavşak var")
        names.add(name.lower())
        if len(j["legs"]) != 2:
            no("legs", "kavşağın iki kolu olur")
        if j["legs"][0]["alignment"] == j["legs"][1]["alignment"]:
            no("legs", "kavşağın iki kolu ayrı akslarda olur")
        for k, leg in enumerate(j["legs"]):
            al = next(a for a in ALIGNMENTS if a["id"] == leg["alignment"])
            if "offsetOf" in al:
                no(f"legs[{k}].alignment", "kolun aksı PI'larıyla kurulan bir aks olur")
            if not (size(leg["left"]) and size(leg["right"])) or leg["left"] > 100 or leg["right"] > 100:
                no(f"legs[{k}].left", "kolun kenarları 0'dan büyük, en çok 100 m olur")
        if j.get("at", 0) > 64:
            no("at", "kesişimin sırası en çok 64 olur")
        if j["kind"] == "atGrade":
            if not size(j.get("radius", 0.0)):
                no("radius", "hemzemin kavşağın dönüş yarıçapı 0'dan büyük, en çok 1000 m olur")
            if j.get("radii") is not None and (len(j["radii"]) != 4 or any(not size(v) for v in j["radii"])):
                no("radii", "çeyreklerin yarıçapı dört olur, her biri 0'dan büyük, en çok 1000 m")


FIELDS = ("name", "kind", "legs", "at", "radius", "radii")


def legs_of(given):
    out = []
    for k, leg in enumerate(given):
        al = alignment(leg["alignment"], f"legs[{k}].alignment")
        out.append({"alignment": al["id"], "alignmentRevision": al.get("revision", 0), "left": leg["left"],
                    "right": leg["right"]})
    return out


def tidy(d):
    if d["kind"] == "separated":
        d.pop("radius", None)
        d.pop("radii", None)
    for k in ("at", "radius"):
        if d.get(k) in (0, 0.0):
            d.pop(k, None)
    return d


def edit(input_, start=START):
    own = copy.deepcopy(list(start))
    op = input_["operation"]
    found = None
    if input_.get("junction"):
        try:
            found = find(own, input_["junction"])
        except Refused:
            found = None
    if op != "set" and found is None:
        find(own, input_.get("junction", ""))
    legs = legs_of(input_["legs"]) if "legs" in input_ else None
    if op == "remove":
        own.remove(found)
        target = None
    elif op == "rebind":
        d = copy.deepcopy(found)
        for leg in d["legs"]:
            leg["alignmentRevision"] = next(a for a in ALIGNMENTS if a["id"] == leg["alignment"]).get("revision", 0)
        if d != found:
            d["revision"] = found["revision"] + 1
        own[own.index(found)] = d
        target = d["id"]
    elif found is not None:
        if not any(k in input_ for k in FIELDS):
            raise bad("", "Değiştirilecek bir şey yok: kavşağın bir alanını verin.")
        d = copy.deepcopy(found)
        for k in FIELDS:
            if k in input_:
                if input_[k] is None:
                    d.pop(k, None)
                else:
                    d[k] = legs if k == "legs" else input_[k]
        d = tidy(d)
        if d != found:
            d["revision"] = found["revision"] + 1
        own[own.index(found)] = d
        target = d["id"]
    else:
        missing = next((f for f in ("name", "kind", "legs") if input_.get(f) is None), None)
        if missing:
            raise bad(missing, "Yeni kavşak için name, kind ve legs verilmeli.")
        jid = input_.get("junction") or next(f"j-{n}" for n in range(1, 999) if all(c["id"] != f"j-{n}" for c in own))
        d = {"id": jid, "name": input_["name"], "revision": 1, "kind": input_["kind"], "legs": legs}
        for k in ("at", "radius", "radii"):
            if input_.get(k) is not None:
                d[k] = input_[k]
        own.append(tidy(d))
        target = jid
    rule_problem(own)
    if target is not None:
        read(next(c for c in own if c["id"] == target))
    return own, target


NEW = {"operation": "set", "name": "Yeni kavşak", "kind": "atGrade", "radius": 15.0,
       "legs": [{"alignment": "Ana aks", "left": 6.0, "right": 6.0}, {"alignment": "Yan aks", "left": 4.0, "right": 4.0}]}


def edit_cases():
    cases = []

    def ok(name, input_, op="execute", undo=False):
        own, target = edit(input_)
        out = {"junctions": listed(own), "revision": "$current"}
        if target is not None:
            out["junction"] = read(next(c for c in own if c["id"] == target))[0]
        if op == "validate":
            result, expect = completed(None), {**UNCHANGED, "road": road_of(START)}
        elif op == "plan":
            result, expect = completed(out), {**UNCHANGED, "road": road_of(START)}
        else:
            result, expect = completed(out), {**STEPPED, "road": road_of(own)}
        s = step(op, input_, result, expect)
        s["within"] = W
        steps = [s]
        if undo:
            steps.append({"op": "undo", "returns": "Kavşak", "expect": {"road": road_of(START), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result=None, setup_=None):
        if result is None:
            try:
                edit(input_)
            except Refused as r:
                result = r.result
            else:
                raise AssertionError(f"{name}: refuses")
        road = None if setup_ is not None else road_of(START)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    ok("Yeni hemzemin kavşak, aksların adıyla", NEW)
    ok("Yeni, kimliğiyle ve çeyreklerin yarıçaplarıyla", {**NEW, "junction": "kavsak-a", "radii": [10.0, 14.0, 18.0, 22.0]})
    ok("Yeni katlı kavşak", {**NEW, "kind": "separated", "radius": None})
    ok("Yarıçapı değiştir", {"operation": "set", "junction": "j-1", "radius": 20.0})
    ok("Katlıya çevir: yaylar kalkar", {"operation": "set", "junction": "Ana kavşak", "kind": "separated"})
    ok("Hemzemine çevir", {"operation": "set", "junction": "j-2", "kind": "atGrade", "radius": 9.0})
    ok("Kenarları değiştir", {"operation": "set", "junction": "j-1",
                              "legs": [{"alignment": "g-1", "left": 7.5, "right": 4.0},
                                       {"alignment": "g-2", "left": 3.0, "right": 6.0}]})
    ok("Kolların sırasını çevir", {"operation": "set", "junction": "j-1",
                                   "legs": [{"alignment": "g-2", "left": 3.5, "right": 3.5},
                                            {"alignment": "g-1", "left": 5.0, "right": 5.0}]})
    ok("Adını değiştir", {"operation": "set", "junction": "j-1", "name": "Merkez kavşak"})
    ok("Eskiyen katlı kavşağı yeniden bağla", {"operation": "rebind", "junction": "j-2"})
    ok("Kaldır", {"operation": "remove", "junction": "Üst geçit"})
    ok("Planı", NEW, op="plan")
    ok("Doğrulama", NEW, op="validate")
    ok("Tek geri alma adımı", NEW, undo=True)
    no("Yol projesi değil", NEW, NOT_ROAD, setup(work="map"))
    no("Olmayan kavşağı kaldır", {"operation": "remove", "junction": "yok"})
    no("Olmayan aks", {**NEW, "legs": [{"alignment": "yok", "left": 5.0, "right": 5.0}, NEW["legs"][1]]})
    no("Kolsuz yeni", {k: v for k, v in NEW.items() if k != "legs"})
    no("Değiştirilecek bir şey yok", {"operation": "set", "junction": "j-1"})
    no("Aynı aks iki kez", {**NEW, "legs": [NEW["legs"][0], NEW["legs"][0]]})
    no("Paralel aks kol olmaz", {**NEW, "legs": [NEW["legs"][0], {"alignment": "Sağ kenar", "left": 3.0, "right": 3.0}]})
    no("Kesişmeyen kollar", {**NEW, "legs": [NEW["legs"][0], {"alignment": "Dar kurp", "left": 3.0, "right": 3.0}]})
    no("İkinci kesişim yok", {**NEW, "at": 1})
    no("Yarıçapsız hemzemin", {**NEW, "radius": None})
    no("Üç çeyrek yarıçapı", {**NEW, "radii": [10.0, 12.0, 14.0]})
    no("Sıfır kenar", {**NEW, "legs": [{**NEW["legs"][0], "left": 0.0}, NEW["legs"][1]]})
    no("Alınmış ad", {**NEW, "name": "ana KAVŞAK"})
    no("Geçersiz beklenen sürüm", {**NEW, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**NEW, "expectedRevision": "41"}, CONFLICT)
    return cases


def query_cases():
    cases = []

    def ok(name, input_, junctions=START):
        out, warns = [], []
        for j in junctions if "junction" not in input_ else [find(junctions, input_["junction"])]:
            try:
                r, w = read(j)
            except Refused:
                r, w = broken(j), []
            out.append(r)
            warns += w
        s = step("execute", input_, completed({"junctions": out}, warns), {**UNCHANGED, "road": road_of(junctions)})
        s["within"] = W
        cases.append(case(name, [s], None if junctions is START else setup(junctions)))

    def no(name, input_, junctions):
        try:
            read(find(junctions, input_["junction"]))
        except Refused as r:
            cases.append(case(name, [step("execute", input_, r.result, UNCHANGED)], setup(junctions)))
            return
        raise AssertionError(name)

    ok("Bütün kavşaklar", {})
    ok("Hemzemin kavşak", {"junction": "j-1"})
    ok("Adıyla, eskiyen katlı kavşak", {"junction": "üst geçit"})
    ok("Büyük yarıçap kurpa uzanır", {"junction": "j-1"}, [{**J1, "radius": 60.0}])
    ok("Çeyreklerin kendi yarıçapları", {"junction": "j-1"}, [{**J1, "radii": [8.0, 16.0, 24.0, 32.0]}])
    ok("Kolların sırası çevrilmiş", {"junction": "j-1"}, [{**J1, "legs": [J1["legs"][1], J1["legs"][0]]}])
    ok("Eşit olmayan kenarlar", {"junction": "j-1"}, [{**J1, "legs": [{**J1["legs"][0], "left": 9.0, "right": 2.5},
                                                                    {**J1["legs"][1], "left": 1.5, "right": 7.0}]}])
    ok("Küçük yarıçap", {"junction": "j-1"}, [{**J1, "radius": 1.0}])
    ok("İki kolu da eskimiş", {"junction": "j-1"}, [{**J1, "legs": [{**J1["legs"][0], "alignmentRevision": 0},
                                                                   {**J1["legs"][1], "alignmentRevision": 1}]}])
    ok("Katlı kavşak bağlı değil", {"junction": "j-2"}, [{**J2, "legs": [J1["legs"][0], J1["legs"][1]]}])
    ok("Okunamayan kavşak listede boş", {}, START + [{**J1, "id": "j-3", "name": "Kesişmeyen",
                                                     "legs": [J1["legs"][0], {"alignment": "g-4", "alignmentRevision": 1,
                                                                              "left": 3.0, "right": 3.0}]}])
    ok("Hiç kavşak yok", {}, [])
    cases.append(case("Yol projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], setup(work="map")))
    cases.append(case("Olmayan kavşak", [step("execute", {"junction": "yok"}, failed(
        "unknown_junction", "“yok” kimlikli ya da adlı kavşak projede yok.", "junction"), UNCHANGED)]))
    no("Kesişmeyen kollar sorulunca", {"junction": "j-3"},
       START + [{**J1, "id": "j-3", "name": "Kesişmeyen",
                 "legs": [J1["legs"][0], {"alignment": "g-4", "alignmentRevision": 1, "left": 3.0, "right": 3.0}]}])
    no("İkinci kesişim sorulunca", {"junction": "j-3"}, START + [{**J1, "id": "j-3", "name": "İkinci", "at": 1}])
    ok("Kenar kenara geniş yollar", {"junction": "j-1"}, [{**J1, "legs": [{**J1["legs"][0], "left": 20.0, "right": 20.0},
                                                                        {**J1["legs"][1], "left": 15.0, "right": 15.0}]}])
    ok("Yalnız çeyrek yarıçapları kurpta", {"junction": "j-1"}, [{**J1, "radii": [60.0, 12.0, 12.0, 12.0]}])
    ok("Yeniden bağlanmış kollar", {"junction": "j-2"}, [{**J2, "legs": [J1["legs"][0], J1["legs"][1]], "name": "Bağlı"}])
    ok("Tek kavşak listede", {}, [J1])
    return cases


def drawing(j, labels):
    r, warnings = read(j)
    objects = [{"pts": [{"x": q["onA"]["x"], "y": q["onA"]["y"]}, {"x": q["onB"]["x"], "y": q["onB"]["y"]}],
                "bulges": [math.tan(math.radians(q["sweep"]) / 4), 0.0]} for q in r["returns"]]
    if labels:
        objects.append({"p": {"x": r["x"], "y": r["y"]}, "rotation": 0.0, "height": 2.5,
                        "text": f"{r['name']} · {r['angle']:.1f}°"})
    return {"junction": j["id"]}, objects, warnings


def draw_cases():
    cases = []
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}

    def ok(name, input_, op="execute", undo=False, extra=(), junctions=START):
        j = find(junctions, input_["junction"])
        out, objects, warns = drawing(j, input_.get("labels", False))
        warns = list(extra) + warns
        out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road_of(junctions)}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road_of(junctions)}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road_of(junctions), "ids": list(range(1, FIRST + len(objects))),
                      "fields": {str(FIRST + i): o for i, o in enumerate(objects)}}
        s = step(op, input_, result, expect)
        s["within"] = W
        steps = [s]
        if undo:
            steps.append({"op": "undo", "returns": "Kavşak çiz", "expect": {"ids": list(range(1, FIRST)), "canRedo": True}})
        cases.append(case(name, steps, None if junctions is START else setup(junctions)))

    def no(name, input_, result, setup_=None, road=START):
        cases.append(case(name, [step("execute", input_, result,
                                      {**UNCHANGED, "road": None if road is None else road_of(road)})], setup_))

    base = {"junction": "j-1", "layerId": "korkuluk"}
    ok("Dönüş yayları planda", base)
    ok("Yazısıyla", {**base, "labels": True})
    ok("Adıyla", {**base, "junction": "ana KAVŞAK"})
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, extra=[hidden])
    ok("Gruptaki katmana", {**base, "layerId": "yol-korkuluk"})
    ok("Planı", base, op="plan")
    ok("Yazılı planı", {**base, "labels": True}, op="plan")
    ok("Doğrulama", base, op="validate")
    ok("Tek geri alma adımı", base, undo=True)
    ok("Çeyreklerin kendi yarıçapları", {**base, "labels": True}, junctions=[{**J1, "radii": [8.0, 16.0, 24.0, 32.0]}])
    ok("Kurpa uzanan yaylar uyarıyla", base, junctions=[{**J1, "radius": 60.0}])
    ok("Kolların sırası çevrilmiş", base, junctions=[{**J1, "legs": [J1["legs"][1], J1["legs"][0]]}])
    ok("Eskiyen kollarıyla uyarı", base, junctions=[{**J1, "legs": [J1["legs"][0], {**J1["legs"][1], "alignmentRevision": 2}]}])
    no("Katlı kavşağın yayı yok", {**base, "junction": "j-2"},
       failed("invalid_junction", "“Üst geçit” katlı bir kavşak; kolları birleşmez, dönüş yayı yok.", "junction"))
    no("Yol projesi değil", base, NOT_ROAD, setup(work="map"), None)
    no("Olmayan kavşak", {**base, "junction": "yok"},
       failed("unknown_junction", "“yok” kimlikli ya da adlı kavşak projede yok.", "junction"))
    no("Olmayan katman", {**base, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Katman grubu", {**base, "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
              "layerId"))
    no("Kilitli katman", {**base, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Geçersiz beklenen sürüm", {**base, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**base, "expectedRevision": "41"}, CONFLICT)
    return cases


def file(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_junction_cases.py yazar; akslar road_alignment_cases.py'nin 40 basamaklı "
                    "başvurusundan, kesişim, kolların teğetleri ve dönüş yayları ADR 0910 §2'den; KentOS kodu yok. "
                    "Kenarlar ve yarıçaplar durumlarındır, bir standardın değil.",
            "setup": setup(), "cases": cases}


def main():
    files = {"road.junctions.edit": file("road.junctions.edit", "Kavşağı düzenle", edit_cases()),
             "road.junctions.query": file("road.junctions.query", "Kavşağı sorgula", query_cases()),
             "road.junctions.draw": file("road.junctions.draw", "Kavşak çiz", draw_cases())}
    check = "--check" in sys.argv
    for name, data in files.items():
        path = OUT / f"{name}.json"
        data = road_grades.fixture(data)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if path.read_text(encoding="utf-8") != text:
                raise SystemExit(f"{path} güncel değil: python3 {Path(__file__).name}")
            print(f"{path.name}: {len(data['cases'])} durum güncel")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
