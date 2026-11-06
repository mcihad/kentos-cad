#!/usr/bin/env python3
"""The shared cases of `road.culverts.edit`, `.query` and `.draw` (plan/road.md Y11.2, docs/adr/0909 §2;
ROAD-DRAIN-01, 02, 11).

The corridor's cross at a culvert's station is road_stakeout_cases.py's (the corridor's own reference); the barrel is
written here from the ADR, no KentOS code: both sides in fill (the toe below the hinge), the barrel between the toes
over the skew's cosine plus what is added beyond, its flow line falling toward the side the water runs to, the least
cover the road's top between the hinges less the barrel's outer top at the top's points, the pipes' length and the
barrel's concrete; its ends placed on the alignment by road_alignment_cases.py's reference at 40 digits. The sizes are
the cases' own, not a standard's.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \
        scripts/fixtures/road_culvert_cases.py [--check]
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
NOT_ROAD = failed("not_a_road_project", "Menfez yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")

BOX = {"kind": "box", "width": 2.0, "height": 1.5, "thickness": 0.25, "slab": 0.3, "cells": 2}
PIPE = {"kind": "pipe", "diameter": 1.0, "thickness": 0.12, "count": 2, "spacing": 0.5}
M1 = {"id": "m-1", "name": "Km 1+380 menfezi", "corridor": "c-1", "corridorRevision": 1, "revision": 1, "station": 380.0,
      "shape": BOX, "invert": 106.5, "slope": 1.0, "flow": "right", "extend": 1.0, "cover": 0.5,
      "material": "Betonarme", "roughness": 0.013}
M2 = {"id": "m-2", "name": "Yan menfez", "corridor": "c-2", "revision": 1, "station": 120.0, "skew": 20.0,
      "shape": PIPE, "invert": 104.0, "slope": 2.0, "flow": "left"}
START = [M1, M2]


def road_of(culverts):
    out = ccmd.road_of(grc.CORRIDORS)
    if culverts:
        out["culverts"] = copy.deepcopy(list(culverts))
    return out


def setup(culverts=START, work="road"):
    base = grc.setup([], work)
    if work == "road":
        base["settings"]["road"] = road_of(culverts)
    return base


FIRST = len(setup()["entities"]) + 1


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, words):
    return Refused("invalid_culvert", words, path or None)


def num(v):
    return ccmd.num(v)


def find(list_, name, path="culvert"):
    for c in list_:
        if c["id"] == name:
            return c
    for c in list_:
        if c["name"].lower() == name.lower():
            return c
    raise Refused("unknown_culvert", f"“{name}” kimlikli ya da adlı menfez projede yok.", path)


def top_of(shape):
    return shape["diameter"] + shape["thickness"] if shape["kind"] == "pipe" else shape["height"] + shape["slab"]


def width_of(shape):
    if shape["kind"] == "pipe":
        n = shape["count"]
        return n * (shape["diameter"] + 2 * shape["thickness"]) + (n - 1) * shape.get("spacing", 0.0)
    n = shape["cells"]
    return n * shape["width"] + (n + 1) * shape["thickness"]


def concrete_of(shape):
    if shape["kind"] == "pipe":
        r, o = shape["diameter"] / 2, shape["diameter"] / 2 + shape["thickness"]
        return shape["count"] * math.pi * (o * o - r * r)
    return width_of(shape) * (shape["height"] + 2 * shape["slab"]) - shape["cells"] * shape["width"] * shape["height"]


def read(m):
    """The culvert's report, warnings, barrel angle and axis place; Refused where it does not read."""
    c = grc.corridor(m["corridor"])
    first = min(r["from"] for r in c["regions"])
    last = max(r["to"] for r in c["regions"])
    s = m["station"]
    if s < first - 1e-9 or s > last + 1e-9:
        raise bad("station", f"“{m['name']}” menfezinin km'si ({num(s)} m) koridorun üstünde değil: koridor "
                             f"{num(first)} ile {num(last)} m arası.")
    try:
        table, warnings = so.table(c, {"corridor": c["id"], "from": s, "to": s, "step": 1.0})
    except ccmd.Refused as r:
        why = f"“{c['name']}” koridoru kurulamıyor: {r.result['error']['message']}"
        raise bad("corridor", f"“{m['name']}” menfezinin koridoru okunamıyor: {why}")
    warnings = [w for w in warnings if w["code"] != "stakeout_unsolved"]
    if not table["rows"]:
        raise bad("station", f"“{m['name']}” menfezinin km'sinde koridorun kesiti yok.")
    region = table["rows"][0]["region"]
    rows = [r for r in table["rows"] if r["region"] == region]
    point = lambda code: next((r for r in rows if r["code"] == code), None)  # noqa: E731
    toes, hinges = [0.0, 0.0], [0.0, 0.0]
    for k, (word, side) in enumerate((("left", "sol"), ("right", "sağ"))):
        h, d = point(f"{word}.hinge"), point(f"{word}.daylight")
        if h is None or d is None or not h["z"] - d["z"] > 1e-9:
            raise bad("station", f"“{m['name']}” menfezinin km'sinde {side} yan dolguda değil; menfez dolgunun "
                                 "altından geçer.")
        toes[k], hinges[k] = d["offset"], h["offset"]
    skew = m.get("skew", 0.0)
    cs = math.cos(math.radians(skew))
    extend = m.get("extend", 0.0)
    ends = [toes[0] / cs - extend, toes[1] / cs + extend]
    fall = m["slope"] / 100 * (1 if m["flow"] == "right" else -1)
    inverts = [m["invert"] - fall * d for d in ends]
    tops = [(r["z"] - (m["invert"] - fall * r["offset"] / cs + top_of(m["shape"])), r["offset"])
            for r in rows if hinges[0] - 1e-9 <= r["offset"] <= hinges[1] + 1e-9]
    least, at = min(tops, key=lambda t: t[0])
    b = ccmd.pcmd.ALIGNED[c["alignment"]]
    e, (x, y) = aref.place_at(b.els, mp.mpf(s))
    th = e.theta(mp.mpf(s) - e.station)
    angle = float(th) - math.pi / 2 + math.radians(skew)
    x, y = float(x), float(y)
    ends_out = [{"side": side, "distance": d, "offset": d * cs, "x": x + math.cos(angle) * d,
                 "y": y + math.sin(angle) * d, "invert": inv}
                for side, d, inv in zip(("left", "right"), ends, inverts)]
    if m.get("cover") is not None and least < m["cover"] - 1e-9:
        warnings.append({"code": "culvert_cover", "path": "cover", "message": "$any"})
    stale = m.get("corridorRevision", 0) != c.get("revision", 0)
    if stale:
        warnings.append({"code": "culvert_stale", "path": "culvert",
                         "message": f"“{m['name']}” menfezinin koridoru değişti; menfezi denetleyip koridora yeniden "
                                    "bağlayın."})
    length = ends[1] - ends[0]
    pipes = m["shape"]["count"] if m["shape"]["kind"] == "pipe" else 0
    report = {"id": m["id"], "name": m["name"], "corridor": m["corridor"], "revision": m["revision"], "stale": stale,
              "station": s, "kmText": ccmd.km_text(c["alignment"], s), "length": length,
              "width": width_of(m["shape"]), "ends": ends_out, "inlet": "left" if m["flow"] == "right" else "right",
              "cover": least, "coverOffset": at, "pipes": pipes * length, "concrete": concrete_of(m["shape"]) * length}
    return report, warnings, angle, [x, y]


def broken(m):
    return {"id": m["id"], "name": m["name"], "corridor": m["corridor"], "revision": m["revision"], "stale": False,
            "station": m["station"], "kmText": "", "length": 0.0, "width": width_of(m["shape"]), "ends": [],
            "inlet": "left" if m["flow"] == "right" else "right", "cover": 0.0, "coverOffset": 0.0, "pipes": 0.0,
            "concrete": 0.0}


def listed(list_):
    return [{"id": m["id"], "name": m["name"], "corridor": m["corridor"], "station": m["station"],
             "revision": m["revision"]} for m in list_]


def rule_problem(list_):
    """The definition's rules (contracts' culverts_problem), the field of the first broken."""
    names = set()

    def no(field, words):
        raise bad(field, f"Menfez kurallarına uymuyor: {words}.")
    size = lambda v: 0 < v <= 100  # noqa: E731
    for m in list_:
        name = m["name"]
        if not name.strip() or name.strip() != name or len(name) > 120:
            no("name", "menfezin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter")
        if name.lower() in names:
            no("name", "bu adda başka bir menfez var")
        names.add(name.lower())
        if not abs(m.get("skew", 0.0)) < 90:
            no("skew", "menfezin açısı -90° ile 90° arası olur (sınırlar dışında)")
        sh = m["shape"]
        if sh["kind"] == "pipe":
            if not size(sh["diameter"]) or not size(sh["thickness"]):
                no("shape", "borunun çapı ve et kalınlığı 0'dan büyük, en çok 100 m olur")
            if not 1 <= sh["count"] <= 20:
                no("shape.count", "boru sayısı 1 ile 20 arası olur")
        else:
            if not all(size(sh[k]) for k in ("width", "height", "thickness", "slab")):
                no("shape", "kutunun genişliği, yüksekliği, duvar ve döşeme kalınlığı 0'dan büyük, en çok 100 m olur")
            if not 1 <= sh["cells"] <= 20:
                no("shape.cells", "göz sayısı 1 ile 20 arası olur")
        if not 0 <= m["slope"] <= 100:
            no("slope", "akış eğimi oranla 0 ile 1 (%0–100) arası olur")
        if not 0 <= m.get("extend", 0.0) <= 100:
            no("extend", "topuğun ötesine boy 0 ile 100 m arası olur")
        if m.get("cover") is not None and not 0 <= m["cover"] <= 100:
            no("cover", "en az örtü 0 ile 100 m arası olur")
        if m.get("roughness") is not None and not 0 < m["roughness"] <= 1:
            no("roughness", "Manning pürüzlülüğü 0'dan büyük, en çok 1 olur")


FIELDS = ("name", "station", "skew", "shape", "invert", "slope", "flow", "extend", "cover", "material", "roughness")


def edit(input_, start=START):
    own = copy.deepcopy(list(start))
    op = input_["operation"]
    found = None
    if input_.get("culvert"):
        try:
            found = find(own, input_["culvert"])
        except Refused:
            found = None
    if op != "set" and found is None:
        find(own, input_.get("culvert", ""))
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
            raise bad("", "Değiştirilecek bir şey yok: menfezin bir alanını verin.")
        d = copy.deepcopy(found)
        for k in FIELDS:
            if k in input_:
                if input_[k] is None:
                    d.pop(k, None)
                else:
                    d[k] = input_[k]
        if cor is not None:
            d["corridor"] = cor["id"]
            d["corridorRevision"] = cor.get("revision", 0)
        for k in ("skew", "extend"):
            if d.get(k) == 0.0:
                d.pop(k)
        if d != found:
            d["revision"] = found["revision"] + 1
        own[own.index(found)] = d
        target = d["id"]
    else:
        missing = next((f for f in ("name", "corridor", "station", "shape", "invert", "slope", "flow")
                        if input_.get(f) is None), None)
        if missing:
            raise bad(missing, "Yeni menfez için name, corridor, station, shape, invert, slope ve flow verilmeli.")
        mid = input_.get("culvert") or next(f"m-{n}" for n in range(1, 999) if all(c["id"] != f"m-{n}" for c in own))
        d = {"id": mid, "name": input_["name"], "corridor": cor["id"], "corridorRevision": cor.get("revision", 0),
             "revision": 1}
        for k in FIELDS[1:]:
            if input_.get(k) is not None and not (k in ("skew", "extend") and input_[k] == 0.0):
                d[k] = input_[k]
        own.append(d)
        target = mid
    rule_problem(own)
    if target is not None:
        read(next(c for c in own if c["id"] == target))
    return own, target


def edit_cases():
    cases = []

    def ok(name, input_, op="execute", undo=False):
        own, target = edit(input_)
        out = {"culverts": listed(own), "revision": "$current"}
        if target is not None:
            out["culvert"] = read(next(c for c in own if c["id"] == target))[0]
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
            steps.append({"op": "undo", "returns": "Menfez", "expect": {"road": road_of(START), "canRedo": True}})
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

    new = {"operation": "set", "name": "Km 0+120 menfezi", "corridor": "Yan koridor", "station": 150.0,
           "shape": {"kind": "pipe", "diameter": 1.2, "thickness": 0.15, "count": 1}, "invert": 104.2, "slope": 1.5,
           "flow": "right"}
    ok("Yeni boru menfez, koridorun adıyla", new)
    ok("Yeni, kimliğiyle, açılı ve uçları uzatılmış", {**new, "culvert": "menfez-a", "skew": -15.0, "extend": 2.0})
    ok("Yeni kutu menfez, örtüsü ve malzemesiyle", {**new, "shape": BOX, "invert": 103.0, "cover": 0.3,
                                                    "material": "Betonarme", "roughness": 0.015})
    ok("Akışı sola çevir", {"operation": "set", "culvert": "m-1", "flow": "left"})
    ok("Km'sini kaydır", {"operation": "set", "culvert": "Km 1+380 menfezi", "station": 400.0})
    ok("Açısını ver", {"operation": "set", "culvert": "m-1", "skew": 30.0})
    ok("Tek gözlü kutuya çevir", {"operation": "set", "culvert": "m-1", "shape": {**BOX, "cells": 1}})
    ok("Örtüyü ve malzemeyi kaldır", {"operation": "set", "culvert": "m-1", "cover": None, "material": None,
                                      "roughness": None})
    ok("Eskiyen yan menfezi yeniden bağla", {"operation": "rebind", "culvert": "m-2"})
    ok("Kaldır", {"operation": "remove", "culvert": "Yan menfez"})
    ok("Planı", new, op="plan")
    ok("Doğrulama", new, op="validate")
    ok("Tek geri alma adımı", new, undo=True)
    no("Yol projesi değil", new, NOT_ROAD, setup(work="map"))
    no("Olmayan menfezi kaldır", {"operation": "remove", "culvert": "yok"})
    no("Olmayan koridor", {**new, "corridor": "yok"})
    no("Akışsız yeni", {k: v for k, v in new.items() if k != "flow"})
    no("Değiştirilecek bir şey yok", {"operation": "set", "culvert": "m-1"})
    no("Yarmada menfez", {**new, "corridor": "c-1", "station": 60.0})
    no("Koridorun dışında", {**new, "station": 900.0})
    no("Kurulamayan koridora", {**new, "corridor": "c-4", "station": 80.0})
    no("Dik açı", {**new, "skew": 90.0})
    no("Sıfır çap", {**new, "shape": {"kind": "pipe", "diameter": 0.0, "thickness": 0.1, "count": 1}})
    no("Göz sayısı sıfır", {**new, "shape": {**BOX, "cells": 0}})
    no("Eksi eğim", {**new, "slope": -1.0})
    no("Pürüzlülük birden büyük", {**new, "roughness": 2.0})
    no("Alınmış ad", {**new, "name": "yan MENFEZ"})
    no("Geçersiz beklenen sürüm", {**new, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**new, "expectedRevision": "41"}, CONFLICT)
    return cases


def query_cases():
    cases = []

    def ok(name, input_, culverts=START):
        out, warns = [], []
        for m in culverts if "culvert" not in input_ else [find(culverts, input_["culvert"])]:
            try:
                r, w, _, _ = read(m)
            except Refused:
                r, w = broken(m), []
            out.append(r)
            warns += w
        s = step("execute", input_, completed({"culverts": out}, warns), {**UNCHANGED, "road": road_of(culverts)})
        s["within"] = W
        cases.append(case(name, [s], None if culverts is START else setup(culverts)))

    def no(name, input_, culverts):
        try:
            read(find(culverts, input_["culvert"]))
        except Refused as r:
            cases.append(case(name, [step("execute", input_, r.result, UNCHANGED)], setup(culverts)))
            return
        raise AssertionError(name)

    ok("Bütün menfezler", {})
    ok("Kutu menfez", {"culvert": "m-1"})
    ok("Adıyla, eskiyen açılı boru menfez", {"culvert": "yan MENFEZ"})
    ok("Az örtü uyarısı", {"culvert": "m-1"}, [{**M1, "cover": 2.0}])
    ok("Düşük dolguda örtü eksi", {"culvert": "m-1"}, [{**M1, "station": 160.0, "invert": 100.5}])
    ok("Sola akış", {"culvert": "m-1"}, [{**M1, "flow": "left"}])
    ok("Sıfır eğim", {"culvert": "m-1"}, [{**M1, "slope": 0.0}])
    ok("Uzatmasız", {"culvert": "m-1"}, [{k: v for k, v in M1.items() if k != "extend"}])
    ok("Sağa 45°", {"culvert": "m-1"}, [{**M1, "skew": 45.0}])
    ok("Sola 30°", {"culvert": "m-1"}, [{**M1, "skew": -30.0}])
    ok("Tek boru", {"culvert": "m-1"}, [{**M1, "shape": {"kind": "pipe", "diameter": 1.5, "thickness": 0.15, "count": 1}}])
    ok("Üç gözlü kutu", {"culvert": "m-1"}, [{**M1, "shape": {**BOX, "cells": 3}}])
    ok("Bölge sınırında: ilk bölgenin kesiti", {"culvert": "m-1"}, [{**M1, "station": 340.0}])
    ok("Bağlı yan menfez eskimez", {"culvert": "m-2"}, [{**M2, "corridorRevision": 2}])
    ok("Okunamayan menfez listede boş", {}, START + [{**M1, "id": "m-3", "name": "Yarma", "station": 60.0}])
    ok("Hiç menfez yok", {}, [])
    cases.append(case("Yol projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], setup(work="map")))
    cases.append(case("Olmayan menfez", [step("execute", {"culvert": "yok"}, failed(
        "unknown_culvert", "“yok” kimlikli ya da adlı menfez projede yok.", "culvert"), UNCHANGED)]))
    no("Yarmadaki menfez sorulunca", {"culvert": "m-3"}, START + [{**M1, "id": "m-3", "name": "Yarma", "station": 60.0}])
    no("Kurulamayan koridordaki menfez", {"culvert": "m-4"},
       START + [{**M1, "id": "m-4", "name": "Bozuk", "corridor": "c-4", "station": 80.0}])
    return cases


def cross_words(shape):
    if shape["kind"] == "pipe":
        d = f"Ø{shape['diameter']:.2f} m"
        return f"{d} × {shape['count']}" if shape["count"] > 1 else d
    one = f"{shape['width']:.2f} × {shape['height']:.2f} m kutu"
    return f"{one} × {shape['cells']}" if shape["cells"] > 1 else one


def drawing(m, labels):
    r, warnings, angle, at = read(m)
    co, si = math.cos(angle), math.sin(angle)
    half = r["width"] / 2

    def along(d, side):
        return {"x": at[0] + co * d - si * side, "y": at[1] + si * d + co * side}
    d0, d1 = r["ends"][0]["distance"], r["ends"][1]["distance"]
    objects = [{"pts": [along(d0, half), along(d1, half), along(d1, -half), along(d0, -half)], "closed": True},
               {"pts": [along(d0, 0), along(d1, 0)]}]
    if labels:
        h = 2.5
        a = math.degrees(angle)
        if not -90 <= a <= 90:
            a += -180 if a > 0 else 180
        objects.append({"p": along((d0 + d1) / 2, half + h), "rotation": a, "height": h,
                        "text": f"Menfez km {r['kmText']} · {cross_words(m['shape'])} · L={r['length']:.2f} m"})
    return {"culvert": m["id"], "length": r["length"]}, objects, warnings


def draw_cases():
    cases = []
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}

    def ok(name, input_, op="execute", undo=False, extra=(), culverts=START):
        m = find(culverts, input_["culvert"])
        out, objects, warns = drawing(m, input_.get("labels", False))
        warns = list(extra) + warns
        out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road_of(culverts)}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road_of(culverts)}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road_of(culverts), "ids": list(range(1, FIRST + len(objects))),
                      "fields": {str(FIRST + i): o for i, o in enumerate(objects)}}
        s = step(op, input_, result, expect)
        s["within"] = W
        steps = [s]
        if undo:
            steps.append({"op": "undo", "returns": "Menfez çiz", "expect": {"ids": list(range(1, FIRST)), "canRedo": True}})
        cases.append(case(name, steps, None if culverts is START else setup(culverts)))

    def no(name, input_, result, setup_=None, road=START):
        cases.append(case(name, [step("execute", input_, result,
                                      {**UNCHANGED, "road": None if road is None else road_of(road)})], setup_))

    base = {"culvert": "m-1", "layerId": "korkuluk"}
    ok("Kutu menfez planda", base)
    ok("Yazısıyla", {**base, "labels": True})
    ok("Adıyla, açılı boru menfez yazısıyla", {**base, "culvert": "yan MENFEZ", "labels": True})
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, extra=[hidden])
    ok("Gruptaki katmana", {**base, "layerId": "yol-korkuluk"})
    ok("Planı", base, op="plan")
    ok("Yazılı planı", {**base, "labels": True}, op="plan")
    ok("Doğrulama", base, op="validate")
    ok("Tek geri alma adımı", base, undo=True)
    ok("Sağa 45°", {**base, "labels": True}, culverts=[{**M1, "skew": 45.0}])
    ok("Sola 60°", base, culverts=[{**M1, "skew": -60.0}])
    ok("Tek boru", {**base, "labels": True},
       culverts=[{**M1, "shape": {"kind": "pipe", "diameter": 1.5, "thickness": 0.15, "count": 1}}])
    ok("Az örtüyle uyarı", base, culverts=[{**M1, "cover": 2.0}])
    yarma = START + [{**M1, "id": "m-3", "name": "Yarma", "station": 60.0}]
    try:
        read(yarma[-1])
    except Refused as r:
        no("Yarmadaki menfez", {**base, "culvert": "m-3"}, r.result, setup(yarma), yarma)
    no("Yol projesi değil", base, NOT_ROAD, setup(work="map"), None)
    no("Olmayan menfez", {**base, "culvert": "yok"},
       failed("unknown_culvert", "“yok” kimlikli ya da adlı menfez projede yok.", "culvert"))
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
            "note": "scripts/fixtures/road_culvert_cases.py yazar; kesit road_stakeout_cases.py'nin (koridorun "
                    "başvurusu), gövde ADR 0909 §2'den, yerler road_alignment_cases.py'nin 40 basamaklı başvurusundan; "
                    "KentOS kodu yok. Ölçüler durumlarındır, bir standardın değil.",
            "setup": setup(), "cases": cases}


def main():
    files = {"road.culverts.edit": file("road.culverts.edit", "Menfezi düzenle", edit_cases()),
             "road.culverts.query": file("road.culverts.query", "Menfezi sorgula", query_cases()),
             "road.culverts.draw": file("road.culverts.draw", "Menfez çiz", draw_cases())}
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
