#!/usr/bin/env python3
"""The shared cases of `road.walls.edit`, `.query` and `.draw` (plan/road.md Y11.3, docs/adr/0909 §3; KentOS Yol's
retaining walls).

The corridor's cross at each station, and the ground along its sample line, are road_section_command_cases.py's (the
corridor's own reference); the wall at a station (its top the higher of the road's edge and the ground under it plus
what stands above, its footing's top the cover below the lower; its stem's trapezoid and its footing) and its totals
(each stretch's plan length times its heights' and its areas' means) are ADR 0909 §3's; the places are
road_alignment_cases.py's 40-digit reference. No KentOS code. The wall's sizes are the cases' own, not a standard's.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_wall_cases.py [--check]
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
import road_stakeout_cases as so  # noqa: E402

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = ccmd.UNCHANGED, ccmd.STEPPED, ccmd.CONFLICT, ccmd.BAD_REVISION
W = 1e-6
NOT_ROAD = failed("not_a_road_project",
                  "İstinat duvarı yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")

SHAPE = {"top": 0.3, "batter": 0.1, "footing": 2.4, "base": 0.5, "toe": 0.6, "cover": 0.6, "above": 0.9}
D1 = {"id": "d-1", "name": "Sağ istinat", "corridor": "c-1", "corridorRevision": 1, "revision": 1, "side": "right",
      "from": 340.0, "to": 420.0, "step": 20.0, "shape": SHAPE, "highest": 5.0, "material": "Betonarme"}
D2 = {"id": "d-2", "name": "Yarma duvarı", "corridor": "c-1", "revision": 1, "side": "left",
      "from": 40.0, "to": 115.0, "step": 25.0, "shape": {**SHAPE, "batter": 0.0, "above": 0.0}}
START = [D1, D2]


ZERO_SHAPE = ("batter", "toe", "cover", "above")


def compact(sh):
    return {k: v for k, v in sh.items() if not (k in ZERO_SHAPE and v == 0.0)}


def road_of(walls):
    out = ccmd.road_of(grc.CORRIDORS)
    if walls:
        out["walls"] = [{**w, "shape": compact(w["shape"])} for w in copy.deepcopy(list(walls))]
    return out


def setup(walls=START, work="road"):
    base = grc.setup([], work)
    if work == "road":
        base["settings"]["road"] = road_of(walls)
    return base


FIRST = len(setup()["entities"]) + 1


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, words):
    return Refused("invalid_retaining_wall", words, path or None)


def num(v):
    return ccmd.num(v)


def find(list_, name, path="wall"):
    for w in list_:
        if w["id"] == name:
            return w
    for w in list_:
        if w["name"].lower() == name.lower():
            return w
    raise Refused("unknown_retaining_wall", f"“{name}” kimlikli ya da adlı istinat duvarı projede yok.", path)


def cross(c, ri, s):
    """The cross at station `s` with region `ri`'s template: the sample line's place and direction, the section and the
    ground along the line (as road_stakeout_cases.cross_at, keeping the ground)."""
    r = c["regions"][ri]
    b = ccmd.pcmd.ALIGNED[c["alignment"]]
    sup = c.get("superelevation")
    if sup:
        d = next(x for x in ccmd.SUPERS if x["id"] == sup)
        if ccmd.supcmd.build(d)[1]:
            sup = None
    fake = {"name": c["name"], "alignment": c["alignment"], "surface": c["surface"], "profile": c["profile"],
            "template": r["template"], "superelevation": sup, "left": c["left"], "right": c["right"]}
    row = ccmd.scmd.sections(fake, [s], ccmd.TEMPLATES)[0][0]
    e, (x, y) = aref.place_at(b.els, mp.mpf(s))
    th = float(e.theta(mp.mpf(s) - e.station))
    return {"at": [float(x), float(y)], "direction": th, "row": row}


def place(cr, offset):
    th = cr["direction"]
    return [cr["at"][0] + math.sin(th) * offset, cr["at"][1] - math.cos(th) * offset]


def ground_at(ground, offset):
    for a, b in zip(ground, ground[1:]):
        if a["offset"] <= offset + 1e-9 and offset <= b["offset"] + 1e-9:
            if b["gap"] or a.get("z") is None or b.get("z") is None:
                return None
            span = b["offset"] - a["offset"]
            if span <= 0:
                return a["z"]
            t = min(1.0, max(0.0, (offset - a["offset"]) / span))
            return a["z"] + (b["z"] - a["z"]) * t
    return None


def sliced(road, ground, sh):
    top = max(road, ground) + sh.get("above", 0.0)
    footing = min(road, ground) - sh.get("cover", 0.0)
    h = top - footing
    area = h * (2 * sh["top"] + sh.get("batter", 0.0) * h) / 2 + sh["footing"] * sh["base"]
    return {"top": top, "footing": footing, "height": h, "area": area, "fill": road > ground}


def read(w):
    """The wall's report, its warnings and, per row, its cross; Refused where it does not read."""
    c = grc.corridor(w["corridor"])
    first = min(r["from"] for r in c["regions"])
    last = max(r["to"] for r in c["regions"])
    for k in ("from", "to"):
        v = w[k]
        if v < first - 1e-9 or v > last + 1e-9:
            raise bad(k, f"“{w['name']}” istinat duvarının km'si ({num(v)} m) koridorun üstünde değil: koridor "
                         f"{num(first)} ile {num(last)} m arası.")
    try:
        _report, warnings, _tri, _regs = ccmd.make(c)
    except ccmd.Refused as r:
        why = f"“{c['name']}” koridoru kurulamıyor: {r.result['error']['message']}"
        raise bad("corridor", f"“{w['name']}” istinat duvarının koridoru okunamıyor: {why}")
    warnings = list(warnings)
    sh = w["shape"]
    rows, crosses = [], []
    for s in so.stepped(w["from"], w["to"], w["step"]):
        km = ccmd.km_text(c["alignment"], s)
        none = bad("from", f"“{w['name']}” istinat duvarının km {km}'sinde koridorun kesiti yok.")
        ri = next((i for i, r in enumerate(c["regions"]) if r["from"] - 1e-9 <= s <= r["to"] + 1e-9), None)
        if ri is None:
            raise none
        cr = cross(c, ri, s)
        side = cr["row"].get(w["side"])
        if side is None:
            raise none
        off, road = side["hinge"]
        g = ground_at(cr["row"]["ground"], off)
        if g is None:
            raise bad("side", f"“{w['name']}” istinat duvarının km {km}'sinde yüzün altında zemin yok.")
        sl = sliced(road, g, sh)
        p = place(cr, off)
        rows.append({"station": s, "kmText": km, "offset": off, "x": p[0], "y": p[1], "road": road, "ground": g,
                     "top": sl["top"], "footing": sl["footing"], "height": sl["height"], "area": sl["area"],
                     "fill": sl["fill"]})
        crosses.append(cr)
    length = face = concrete = 0.0
    for a, b in zip(rows, rows[1:]):
        lng = math.hypot(b["x"] - a["x"], b["y"] - a["y"])
        length += lng
        face += lng * (a["height"] + b["height"]) / 2
        concrete += lng * (a["area"] + b["area"]) / 2
    highest, at = -math.inf, w["from"]
    for r in rows:
        if r["height"] > highest + 1e-12:
            highest, at = r["height"], r["station"]
    if w.get("highest") is not None and highest > w["highest"] + 1e-9:
        warnings.append({"code": "retaining_wall_high", "path": "highest", "message": "$any"})
    stale = w.get("corridorRevision", 0) != c.get("revision", 0)
    if stale:
        warnings.append({"code": "retaining_wall_stale", "path": "wall",
                         "message": f"“{w['name']}” istinat duvarının koridoru değişti; duvarı denetleyip koridora "
                                    "yeniden bağlayın."})
    report = {"id": w["id"], "name": w["name"], "corridor": w["corridor"], "revision": w["revision"], "stale": stale,
              "side": w["side"], "from": w["from"], "to": w["to"], "rows": rows, "length": length, "face": face,
              "concrete": concrete, "highest": highest, "highestAt": at}
    return report, warnings, crosses


def broken(w):
    return {"id": w["id"], "name": w["name"], "corridor": w["corridor"], "revision": w["revision"], "stale": False,
            "side": w["side"], "from": w["from"], "to": w["to"], "rows": [], "length": 0.0, "face": 0.0,
            "concrete": 0.0, "highest": 0.0, "highestAt": w["from"]}


def listed(list_):
    return [{"id": w["id"], "name": w["name"], "corridor": w["corridor"], "side": w["side"], "from": w["from"],
             "to": w["to"], "revision": w["revision"]} for w in list_]


def rule_problem(list_):
    """The definition's rules (contracts' walls_problem), the field of the first broken."""
    names = set()

    def no(field, words):
        raise bad(field, f"İstinat duvarı kurallarına uymuyor: {words}.")
    size = lambda v: 0 < v <= 100  # noqa: E731
    room = lambda v: 0 <= v <= 100  # noqa: E731
    for w in list_:
        name = w["name"]
        if not name.strip() or name.strip() != name or len(name) > 120:
            no("name", "duvarın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter")
        if name.lower() in names:
            no("name", "bu adda başka bir istinat duvarı var")
        names.add(name.lower())
        if not w["from"] < w["to"]:
            no("to", "duvarın bitiş km'si başlangıcından büyük olmalı")
        if not 0 < w["step"] <= 1000:
            no("step", "duvarın okunduğu aralık 0'dan büyük, en çok 1000 m olur")
        if (w["to"] - w["from"]) / w["step"] > 5000:
            no("step", "duvar en çok 5000 km'de okunur; aralığı büyütün")
        sh = w["shape"]
        if not (size(sh["top"]) and size(sh["footing"]) and size(sh["base"])):
            no("shape", "gövdenin üst kalınlığı, temelin genişliği ve kalınlığı 0'dan büyük, en çok 100 m olur")
        if not 0 <= sh.get("batter", 0.0) <= 10:
            no("shape.batter", "gövdenin eğimi 0 ile 10 arası olur (metrede yatay metre)")
        if not all(room(sh.get(k, 0.0)) for k in ("toe", "cover", "above")):
            no("shape", "temelin önü, örtü ve korkuluk payı 0 ile 100 m arası olur")
        if sh.get("toe", 0.0) + sh["top"] > sh["footing"] + 1e-9:
            no("shape.toe", "temelin önü ve gövdenin üst kalınlığı temelin genişliğine sığmalı")
        if w.get("highest") is not None and not size(w["highest"]):
            no("highest", "en yüksek gövde 0'dan büyük, en çok 100 m olur")


FIELDS = ("name", "side", "from", "to", "step", "shape", "highest", "material")
def edit(input_, start=START):
    own = copy.deepcopy(list(start))
    op = input_["operation"]
    found = None
    if input_.get("wall"):
        try:
            found = find(own, input_["wall"])
        except Refused:
            found = None
    if op != "set" and found is None:
        find(own, input_.get("wall", ""))
    cor = None
    if "corridor" in input_:
        cor = next((c for c in grc.CORRIDORS if c["id"] == input_["corridor"]), None) or next(
            (c for c in grc.CORRIDORS if c["name"].lower() == input_["corridor"].lower()), None)
        if cor is None:
            raise Refused("unknown_corridor", f"“{input_['corridor']}” kimlikli ya da adlı koridor projede yok.",
                          "corridor")
    if op == "remove":
        own.remove(found)
        target = None
    elif op == "rebind":
        d = copy.deepcopy(found)
        d["corridorRevision"] = grc.corridor(found["corridor"]).get("revision", 0)
        if d != found:
            d["revision"] = found["revision"] + 1
        own[own.index(found)] = d
        target = d["id"]
    elif found is not None:
        if cor is None and not any(k in input_ for k in FIELDS):
            raise bad("", "Değiştirilecek bir şey yok: duvarın bir alanını verin.")
        d = copy.deepcopy(found)
        for k in FIELDS:
            if k in input_:
                if input_[k] is None:
                    d.pop(k, None)
                else:
                    d[k] = compact(input_[k]) if k == "shape" else input_[k]
        if cor is not None:
            d["corridor"] = cor["id"]
            d["corridorRevision"] = cor.get("revision", 0)
        if d != found:
            d["revision"] = found["revision"] + 1
        own[own.index(found)] = d
        target = d["id"]
    else:
        missing = next((f for f in ("name", "corridor", "side", "from", "to", "step", "shape")
                        if input_.get(f) is None), None)
        if missing:
            raise bad(missing, "Yeni istinat duvarı için name, corridor, side, from, to, step ve shape verilmeli.")
        wid = input_.get("wall") or next(f"d-{n}" for n in range(1, 999) if all(c["id"] != f"d-{n}" for c in own))
        d = {"id": wid, "name": input_["name"], "corridor": cor["id"], "corridorRevision": cor.get("revision", 0),
             "revision": 1}
        for k in FIELDS[1:]:
            if input_.get(k) is not None:
                d[k] = compact(input_[k]) if k == "shape" else input_[k]
        own.append(d)
        target = wid
    rule_problem(own)
    if target is not None:
        read(next(c for c in own if c["id"] == target))
    return own, target


def edit_cases():
    cases = []

    def ok(name, input_, op="execute", undo=False):
        own, target = edit(input_)
        out = {"walls": listed(own), "revision": "$current"}
        if target is not None:
            out["wall"] = read(next(c for c in own if c["id"] == target))[0]
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
            steps.append({"op": "undo", "returns": "İstinat duvarı",
                          "expect": {"road": road_of(START), "canRedo": True}})
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

    new = {"operation": "set", "name": "Yan koridor duvarı", "corridor": "Yan koridor", "side": "right",
           "from": 150.0, "to": 250.0, "step": 50.0, "shape": SHAPE}
    ok("Yeni duvar, koridorun adıyla", new)
    ok("Yeni, kimliğiyle, en yükseği ve malzemesiyle",
       {**new, "wall": "duvar-a", "side": "left", "highest": 3.0, "material": "Taş"})
    ok("Yeni, sıfır eğim ve korkuluk paysız", {**new, "shape": {**SHAPE, "batter": 0.0, "above": 0.0}})
    ok("Yanını çevir", {"operation": "set", "wall": "d-1", "side": "left"})
    ok("Km'lerini kaydır", {"operation": "set", "wall": "Sağ istinat", "from": 360.0, "to": 440.0})
    ok("Sık oku", {"operation": "set", "wall": "d-1", "step": 10.0})
    ok("Temelini genişlet", {"operation": "set", "wall": "d-1", "shape": {**SHAPE, "footing": 3.2, "toe": 0.8}})
    ok("En yükseği ve malzemeyi kaldır", {"operation": "set", "wall": "d-1", "highest": None, "material": None})
    ok("Başka koridora taşı", {"operation": "set", "wall": "d-2", "corridor": "c-2", "from": 100.0, "to": 200.0})
    ok("Eskiyen yarma duvarını yeniden bağla", {"operation": "rebind", "wall": "d-2"})
    ok("Kaldır", {"operation": "remove", "wall": "Yarma duvarı"})
    ok("Planı", new, op="plan")
    ok("Doğrulama", new, op="validate")
    ok("Tek geri alma adımı", new, undo=True)
    no("Yol projesi değil", new, NOT_ROAD, setup(work="map"))
    no("Olmayan duvarı kaldır", {"operation": "remove", "wall": "yok"})
    no("Olmayan koridor", {**new, "corridor": "yok"})
    no("Yansız yeni", {k: v for k, v in new.items() if k != "side"})
    no("Değiştirilecek bir şey yok", {"operation": "set", "wall": "d-1"})
    no("Koridorun dışında", {**new, "to": 900.0})
    no("Ters km'ler", {**new, "from": 250.0, "to": 150.0})
    no("Kurulamayan koridora", {**new, "corridor": "c-4", "from": 10.0, "to": 40.0})
    no("Sıfır aralık", {**new, "step": 0.0})
    no("Çok sık okuma", {**new, "step": 0.01})
    no("Sıfır temel", {**new, "shape": {**SHAPE, "footing": 0.0}})
    no("Temele sığmayan ön", {**new, "shape": {**SHAPE, "toe": 2.3}})
    no("Eksi eğim", {**new, "shape": {**SHAPE, "batter": -0.1}})
    no("Alınmış ad", {**new, "name": "SAĞ istinat"})
    no("Geçersiz beklenen sürüm", {**new, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**new, "expectedRevision": "41"}, CONFLICT)
    return cases


def query_cases():
    cases = []

    def ok(name, input_, walls=START):
        out, warns = [], []
        for w in walls if "wall" not in input_ else [find(walls, input_["wall"])]:
            try:
                r, ws, _ = read(w)
            except Refused:
                r, ws = broken(w), []
            out.append(r)
            warns += ws
        s = step("execute", input_, completed({"walls": out}, warns), {**UNCHANGED, "road": road_of(walls)})
        s["within"] = W
        cases.append(case(name, [s], None if walls is START else setup(walls)))

    def no(name, input_, walls):
        try:
            read(find(walls, input_["wall"]))
        except Refused as r:
            cases.append(case(name, [step("execute", input_, r.result, UNCHANGED)], setup(walls)))
            return
        raise AssertionError(name)

    ok("Bütün duvarlar", {})
    ok("Dolgu duvarı", {"wall": "d-1"})
    ok("Adıyla, yarma duvarı", {"wall": "YARMA duvarı"})
    ok("En yüksek aşılınca uyarı", {"wall": "d-1"}, [{**D1, "highest": 1.0}])
    ok("Sol yanda dolgu", {"wall": "d-1"}, [{**D1, "side": "left"}])
    ok("Sık okuma", {"wall": "d-1"}, [{**D1, "step": 5.0}])
    ok("Aralığa denk gelmeyen uçlar", {"wall": "d-1"}, [{**D1, "from": 333.0, "to": 417.5}])
    ok("Bölge sınırını geçen", {"wall": "d-1"}, [{**D1, "from": 260.0, "to": 340.0}])
    ok("Eğimli gövde, korkuluk paysız", {"wall": "d-1"}, [{**D1, "shape": {**SHAPE, "batter": 0.25, "above": 0.0}}])
    ok("Örtüsüz", {"wall": "d-1"}, [{**D1, "shape": {k: v for k, v in SHAPE.items() if k != "cover"}}])
    ok("Yan koridorda eskiyen", {"wall": "d-1"}, [{**D1, "corridor": "c-2", "from": 100.0, "to": 200.0}])
    ok("Yan koridorda bağlı", {"wall": "d-1"},
       [{**D1, "corridor": "c-2", "corridorRevision": 2, "from": 100.0, "to": 200.0}])
    ok("Tek km'lik aralık", {"wall": "d-1"}, [{**D1, "from": 380.0, "to": 381.0}])
    ok("Okunamayan duvar listede boş", {},
       START + [{**D1, "id": "d-3", "name": "Bozuk", "corridor": "c-4", "from": 10.0, "to": 40.0}])
    ok("Hiç duvar yok", {}, [])
    cases.append(case("Yol projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], setup(work="map")))
    cases.append(case("Olmayan duvar", [step("execute", {"wall": "yok"}, failed(
        "unknown_retaining_wall", "“yok” kimlikli ya da adlı istinat duvarı projede yok.", "wall"), UNCHANGED)]))
    no("Kurulamayan koridordaki duvar", {"wall": "d-3"},
       START + [{**D1, "id": "d-3", "name": "Bozuk", "corridor": "c-4", "from": 10.0, "to": 40.0}])
    no("Yüzeysiz koridordaki duvar", {"wall": "d-3"},
       START + [{**D1, "id": "d-3", "name": "Yüzeysiz", "corridor": "c-3", "from": 120.0, "to": 180.0}])
    no("Koridorun dışına taşan", {"wall": "d-3"},
       START + [{**D1, "id": "d-3", "name": "Taşan", "from": 550.0, "to": 650.0}])
    return cases


def footing_edges(w, offset, fill):
    side = 1.0 if w["side"] == "right" else -1.0
    facing = side if fill else -side
    toe = offset + facing * w["shape"].get("toe", 0.0)
    return [toe, toe - facing * w["shape"]["footing"]]


def drawing(w, labels):
    r, warnings, crosses = read(w)
    pt = lambda p: {"x": p[0], "y": p[1]}  # noqa: E731
    face = [{"x": row["x"], "y": row["y"]} for row in r["rows"]]
    edges = [footing_edges(w, row["offset"], row["fill"]) for row in r["rows"]]
    outline = [pt(place(cr, e[0])) for cr, e in zip(crosses, edges)]
    outline += [pt(place(cr, e[1])) for cr, e in reversed(list(zip(crosses, edges)))]
    objects = [{"pts": face}, {"pts": outline, "closed": True}]
    if labels:
        h = 2.5
        k = len(r["rows"]) // 2
        row, cr, e = r["rows"][k], crosses[k], edges[k]
        out = 1.0 if w["side"] == "right" else -1.0
        reach = max(e[0], e[1], row["offset"]) if out > 0 else min(e[0], e[1], row["offset"])
        a = math.degrees(cr["direction"])
        if not -90 <= a <= 90:
            a += -180 if a > 0 else 180
        objects.append({"p": pt(place(cr, reach + out * 2 * h)), "rotation": a, "height": h,
                        "text": f"İstinat duvarı km {r['rows'][0]['kmText']} – {r['rows'][-1]['kmText']} · "
                                f"H en çok {r['highest']:.2f} m"})
    return {"wall": w["id"], "length": r["length"]}, objects, warnings


def draw_cases():
    cases = []
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}

    def ok(name, input_, op="execute", undo=False, extra=(), walls=START):
        w = find(walls, input_["wall"])
        out, objects, warns = drawing(w, input_.get("labels", False))
        warns = list(extra) + warns
        out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road_of(walls)}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road_of(walls)}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road_of(walls), "ids": list(range(1, FIRST + len(objects))),
                      "fields": {str(FIRST + i): o for i, o in enumerate(objects)}}
        s = step(op, input_, result, expect)
        s["within"] = W
        steps = [s]
        if undo:
            steps.append({"op": "undo", "returns": "İstinat duvarı çiz",
                          "expect": {"ids": list(range(1, FIRST)), "canRedo": True}})
        cases.append(case(name, steps, None if walls is START else setup(walls)))

    def no(name, input_, result, setup_=None, road=START):
        cases.append(case(name, [step("execute", input_, result,
                                      {**UNCHANGED, "road": None if road is None else road_of(road)})], setup_))

    base = {"wall": "d-1", "layerId": "korkuluk"}
    ok("Dolgu duvarı planda", base)
    ok("Yazısıyla", {**base, "labels": True})
    ok("Adıyla, yarma duvarı yazısıyla", {**base, "wall": "YARMA duvarı", "labels": True})
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, extra=[hidden])
    ok("Gruptaki katmana", {**base, "layerId": "yol-korkuluk"})
    ok("Planı", base, op="plan")
    ok("Yazılı planı", {**base, "labels": True}, op="plan")
    ok("Doğrulama", base, op="validate")
    ok("Tek geri alma adımı", base, undo=True)
    ok("Sol yanda", {**base, "labels": True}, walls=[{**D1, "side": "left"}])
    ok("Sık okunan", base, walls=[{**D1, "step": 5.0}])
    ok("En yükseği aşan uyarıyla", base, walls=[{**D1, "highest": 1.0}])
    ok("Yan koridorda eskiyen", {**base, "labels": True},
       walls=[{**D1, "corridor": "c-2", "from": 100.0, "to": 200.0}])
    bozuk = START + [{**D1, "id": "d-3", "name": "Bozuk", "corridor": "c-4", "from": 10.0, "to": 40.0}]
    try:
        read(bozuk[-1])
    except Refused as r:
        no("Kurulamayan koridordaki duvar", {**base, "wall": "d-3"}, r.result, setup(bozuk), bozuk)
    no("Yol projesi değil", base, NOT_ROAD, setup(work="map"), None)
    no("Olmayan duvar", {**base, "wall": "yok"},
       failed("unknown_retaining_wall", "“yok” kimlikli ya da adlı istinat duvarı projede yok.", "wall"))
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
            "note": "scripts/fixtures/road_wall_cases.py yazar; kesit ve kesit çizgisindeki zemin "
                    "road_section_command_cases.py'nin (koridorun başvurusu), duvar ADR 0909 §3'ten, yerler "
                    "road_alignment_cases.py'nin 40 basamaklı başvurusundan; KentOS kodu yok. Ölçüler durumlarındır, "
                    "bir standardın değil.",
            "setup": setup(), "cases": cases}


def main():
    files = {"road.walls.edit": file("road.walls.edit", "İstinat duvarını düzenle", edit_cases()),
             "road.walls.query": file("road.walls.query", "İstinat duvarını sorgula", query_cases()),
             "road.walls.draw": file("road.walls.draw", "İstinat duvarı çiz", draw_cases())}
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
