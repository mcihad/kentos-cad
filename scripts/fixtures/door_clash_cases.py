#!/usr/bin/env python3
"""The doors' audit for revolving, folding and sliding doors and its fixes (docs/adr/0787 §1), from the ADR's rules,
without KentOS code.

Writes fixtures/commands/v1/cad.arch.audit.doors.special.json; `--check` compares and writes nothing. The walls run
along x or y, so a door's jambs and zones are exact sums. The rules, as the ADR gives them (in the frame of the door's
jambs: `u` along its wall from its start jamb, `n` its wall's left, the light c = w − 2·fit − 2·frame from fit + frame):

- A door's zones as it opens: a swinging leaf's sweep (its hinge and 17 points of its arc, ADR 0772 §5); a revolving
  door's drum, a circle of radius D/2 (D its drum, none: the light) about the light's middle on the wall's middle, 32
  points from 0°; a folding door's panels gathered at the jamb at its `hand` (both sides: half each, the start the
  fewer), each group's reach a rectangle along the light as far as its panels go and a panel (c/n) deep out of the face
  it opens to; a one-leaf sliding door on its track the leaf's travel, beside the face it opens to from t/4 out, the
  leaf's thickness (none: 4 cm) deep, from the light's far side to a light past the jamb at its `hand` (ADR 0777 §4).
- A clash (`arch_door_swing_clash`, physical): a zone and another wall (not its own), a column or a fixture on its
  level, or another door's zone, overlap by more than a millimetre (convex: no separating axis). Each says what of the
  door strikes: the leaf, the drum, the panels, the sliding leaf.
- Its fix: the door turned (`cad.openings.edit`'s flip) to the other face, then its other hinge side, then both, the
  first whose zones change and clear every obstacle and every other door's zones; none on a locked layer.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import special_doors_command_cases as sd  # noqa: E402  (ADR 0777's widths, written without KentOS code)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.audit.doors.special.json"
LEAF = 0.04
BLOCK = "0192f5a1-7870-7000-8000-0000000000b1"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def UID(n):
    return f"0192f5a1-7871-7000-8000-{n:012x}"


def layer(id, name, locked=False, visible=True):
    return {"id": id, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────

# Walls: id → (start, end, level). All 0.2 m thick, centred.
WALLS = {
    1: ((0.0, 0.0), (14.0, 0.0), "zemin"),
    2: ((3.0, 0.1), (3.0, 3.0), "zemin"),
    3: ((0.0, -1.8), (14.0, -1.8), "zemin"),
    4: ((10.4, -0.1), (10.4, -1.7), "zemin"),
}
T = 0.2
# Columns: id → (centre, size); fixtures: id → corner (the block is 0.6 × 0.6).
COLUMNS = {5: ((6.0, 1.1), 0.4), 6: ((4.4, 0.35), 0.3)}
FIXTURES = {7: (8.2, 0.3)}
# Doors: id → its fields (`host`, `at`, `width`, `operation`, `leaves`, `hand`, `side`, `frame`, `build`, `number`,
# `layer`).
DOORS = {
    8: {"host": 1, "at": 6.0, "width": 2.4, "operation": "revolving", "leaves": 4, "frame": 0.1, "build": {"drum": 2.0},
        "number": "D1"},
    9: {"host": 1, "at": 3.9, "width": 1.6, "operation": "folding", "leaves": 2, "frame": 0.04, "number": "KT1"},
    10: {"host": 1, "at": 11.5, "width": 1.0, "operation": "slide", "frame": 0.05, "side": "right",
         "build": {"track": True}, "number": "S1"},
    11: {"host": 1, "at": 8.6, "width": 2.0, "operation": "folding", "leaves": 4, "frame": 0.04,
         "build": {"bothSides": True}, "number": "KT2", "layer": "kilitli"},
    12: {"host": 2, "at": 0.8, "width": 0.9, "frame": 0.05, "side": "right", "number": "K3"},
    13: {"host": 3, "at": 6.0, "width": 0.9, "frame": 0.05, "number": "K4"},
}
IDS = list(range(1, 14))


def ids_of(scene):
    """Its objects' ids: 1 to the largest, gaps kept by fillers."""
    top = max(set(scene["walls"]) | set(scene["columns"]) | set(scene["fixtures"]) | set(scene["doors"]))
    return list(range(1, top + 1))


def frame(host):
    (ax, ay), (bx, by), _ = WALLS[host]
    L = math.hypot(bx - ax, by - ay)
    u = ((bx - ax) / L, (by - ay) / L)
    return (ax, ay), u, (-u[1], u[0])


def pt(a, u, n, s, d):
    return (a[0] + u[0] * s + n[0] * d, a[1] + u[1] * s + n[1] * d)


def jambs(o):
    a, u, n = frame(o["host"])
    s0, s1 = o["at"] - o["width"] / 2, o["at"] + o["width"] / 2
    return [pt(a, u, n, s0, T / 2), pt(a, u, n, s0, -T / 2), pt(a, u, n, s1, T / 2), pt(a, u, n, s1, -T / 2)]


def rotate(v, deg):
    r = math.radians(deg)
    return (v[0] * math.cos(r) - v[1] * math.sin(r), v[0] * math.sin(r) + v[1] * math.cos(r))


def zones(o):
    """The door's zones as it opens, in the core's order: its own (leaves, drum, groups), then a sliding leaf's travel."""
    a, u, n = frame(o["host"])
    b = o.get("build") or {}
    fit, fr = b.get("fit", 0.0), o.get("frame", 0.0)
    s0 = o["at"] - o["width"] / 2
    s1 = o["at"] + o["width"] / 2
    clear = o["width"] - 2 * fr - 2 * fit
    left = o.get("side", "left") == "left"
    face = T / 2 if left else -T / 2
    out = n if left else (-n[0], -n[1])
    op, k = o.get("operation", "swing"), o.get("leaves", 1)
    hand_end = o.get("hand", "start") == "end"
    at0, at1 = pt(a, u, n, s0 + fit + fr, face), pt(a, u, n, s1 - fit - fr, face)
    mu = (-u[0], -u[1])
    if op == "swing":
        if k == 2:
            leaves = [(at0, u, clear / 2), (at1, mu, clear / 2)]
        elif hand_end:
            leaves = [(at1, mu, clear)]
        else:
            leaves = [(at0, u, clear)]
        rings = []
        for hinge, closed, ln in leaves:
            turn = 90.0 if closed[0] * out[1] - closed[1] * out[0] >= 0 else -90.0
            ring = [hinge]
            for i in range(17):
                d = rotate(closed, turn * i / 16)
                ring.append((hinge[0] + d[0] * ln, hinge[1] + d[1] * ln))
            rings.append(ring)
        return rings
    if op == "revolving":
        r = min(b.get("drum", clear), clear) / 2
        c = pt(a, u, n, s0 + fit + fr + clear / 2, 0.0)
        return [[(c[0] + r * math.cos(2 * math.pi * i / 32), c[1] + r * math.sin(2 * math.pi * i / 32))
                 for i in range(32)]]
    if op == "folding":
        w = clear / k
        if b.get("bothSides"):
            groups = [(at0, u, k // 2), (at1, mu, k - k // 2)]
        elif hand_end:
            groups = [(at1, mu, k)]
        else:
            groups = [(at0, u, k)]
        rings = []
        for h, d, p in groups:
            along = (d[0] * w * p, d[1] * w * p)
            deep = (out[0] * w, out[1] * w)
            rings.append([h, (h[0] + along[0], h[1] + along[1]), (h[0] + along[0] + deep[0], h[1] + along[1] + deep[1]),
                          (h[0] + deep[0], h[1] + deep[1])])
        return rings
    if op == "slide" and k == 1 and b.get("track"):
        tl = b.get("leafThickness", LEAF)
        kk = T / 4
        start = fit + fr

        def on(s, off):
            p = pt(a, u, n, s0 + s, face)
            return (p[0] + out[0] * off, p[1] + out[1] * off)
        lo, hi = (start, start + 2 * clear) if hand_end else (start - clear, start + clear)
        return [[on(lo, kk), on(hi, kk), on(hi, kk + tl), on(lo, kk + tl)]]
    return []


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def wall_rect(i):
    (ax, ay), (bx, by), _ = WALLS[i]
    if ay == by:
        return rect(min(ax, bx), ay - T / 2, max(ax, bx), ay + T / 2)
    return rect(ax - T / 2, min(ay, by), ax + T / 2, max(ay, by))


def obstacles(scene):
    """The obstacles in the drawing's order: (id, what, level, ring)."""
    out = []
    for i in sorted(set(scene["walls"]) | set(scene["columns"]) | set(scene["fixtures"])):
        if i in scene["walls"]:
            out.append((i, "duvar", WALLS[i][2], wall_rect(i)))
        elif i in scene["columns"]:
            (cx, cy), s = scene["columns"][i]
            out.append((i, "kolon", "zemin", rect(cx - s / 2, cy - s / 2, cx + s / 2, cy + s / 2)))
        else:
            x, y = scene["fixtures"][i]
            out.append((i, "donatı", "zemin", rect(x, y, x + 0.6, y + 0.6)))
    return out


def overlap(a, b):
    """Two convex rings overlap by more than a millimetre on every separating axis."""
    for poly in (a, b):
        for i in range(len(poly)):
            p, q = poly[i], poly[(i + 1) % len(poly)]
            ax = (-(q[1] - p[1]), q[0] - p[0])
            m = math.hypot(*ax)
            if m == 0:
                continue
            ax = (ax[0] / m, ax[1] / m)
            pa = [x * ax[0] + y * ax[1] for x, y in a]
            pb = [x * ax[0] + y * ax[1] for x, y in b]
            if min(max(pa), max(pb)) - max(min(pa), min(pb)) <= 0.001:
                return False
    return True


def middle(ring):
    return (sum(p[0] for p in ring) / len(ring), sum(p[1] for p in ring) / len(ring))


DATIVE = {"duvar": "duvara", "kolon": "kolona", "donatı": "donatıya"}


def clash_words(o, what):
    m, op = o["number"], o.get("operation", "swing")
    if op == "revolving":
        return (f"{m} döner kapısının tamburu bir {DATIVE[what]} çarpıyor (fiziksel çarpışma).",
                "Döner kapıyı kaydırın, tamburunu küçültün ya da engeli taşıyın.")
    if op == "folding":
        return (f"{m} katlanır kapısının panelleri toplanırken bir {DATIVE[what]} çarpıyor (fiziksel çarpışma).",
                "Kapının açılış yönünü ya da toplandığı yanı çevirin, panel sayısını artırın ya da engeli taşıyın.")
    if op == "slide":
        return (f"{m} sürme kapısının kanadı kayarken bir {DATIVE[what]} çarpıyor (fiziksel çarpışma).",
                "Kapının kaydığı yüzü ya da yönü çevirin ya da engeli taşıyın.")
    return (f"{m} kapısının kanadı açılırken bir {DATIVE[what]} çarpıyor (fiziksel çarpışma).",
            "Kapının açılış yönünü ya da menteşesini çevirin, kanadı daraltın ya da engeli taşıyın.")


TURNS = [("Açılışı öbür yüze çevir", "side"), ("Menteşe yanını çevir", "hand"), ("Açılışı ve menteşe yanını çevir", "both")]


def turned(o, what):
    t = dict(o)
    if what != "hand":
        t["side"] = "right" if o.get("side", "left") == "left" else "left"
    if what != "side":
        t["hand"] = "end" if o.get("hand", "start") == "start" else "start"
    return t


def level(scene, o):
    return WALLS[o["host"]][2] if o["host"] not in scene.get("levels", {}) else scene["levels"][o["host"]]


def door_fix(scene, i):
    o = scene["doors"][i]
    if o.get("layer", "kapi") in scene.get("locked", ("kilitli",)):
        return None
    now = zones(o)
    lv = level(scene, o)
    for label, what in TURNS:
        zs = zones(turned(o, what))
        if zs == now:
            continue
        hit = any(oid != o["host"] and olv == lv and overlap(z, ring)
                  for z in zs for oid, _, olv, ring in obstacles_of(scene))
        hit = hit or any(j != i and level(scene, x) == lv and overlap(z, s)
                         for z in zs for j, x in scene["doors"].items() for s in zones(x))
        if not hit:
            return {"label": label, "step": {"command": "cad.openings.edit",
                                             "input": {"uids": [f"$uidOf:{i}"],
                                                       "operation": {"kind": "flip", "what": what}}}}
    return None


def obstacles_of(scene):
    out = []
    for oid, what, lv, ring in obstacles(scene):
        if oid in WALLS and oid in scene.get("levels", {}):
            lv = scene["levels"][oid]
        out.append((oid, what, lv, ring))
    return out


def finding(code, message, repair, uids, at, nature, fix=None):
    out = {"key": f"{code}:" + ",".join(f"$uidOf:{u}" for u in uids), "check": "doors", "code": code,
           "message": message, "repair": repair, "uids": [f"$uidOf:{u}" for u in uids], "at": P(*at)}
    if fix:
        out["fix"] = fix
    out["nature"] = nature
    return out


def findings(scene, min_clear=None):
    out = []
    ids = sorted(scene["doors"])
    for n, i in enumerate(ids):
        o = scene["doors"][i]
        lv = level(scene, o)
        fix = None
        tried = False
        for z in zones(o):
            for oid, what, olv, ring in obstacles_of(scene):
                if oid == o["host"] or olv != lv or not overlap(z, ring):
                    continue
                if not tried:
                    fix, tried = door_fix(scene, i), True
                message, repair = clash_words(o, what)
                out.append(finding("arch_door_swing_clash", message, repair, [i, oid], middle(z), "physical", fix))
            for j in ids[n + 1:]:
                x = scene["doors"][j]
                if level(scene, x) != lv or not any(overlap(z, s) for s in zones(x)):
                    continue
                if not tried:
                    fix, tried = door_fix(scene, i), True
                if o.get("operation", "swing") == "swing" and x.get("operation", "swing") == "swing":
                    message = f"{o['number']} ve {x['number']} kapılarının kanatları açılırken birbirine çarpıyor (fiziksel çarpışma)."
                else:
                    message = f"{o['number']} ve {x['number']} kapıları açılırken birbirinin alanına giriyor (fiziksel çarpışma)."
                out.append(finding("arch_door_swing_clash", message,
                                   "Kapılardan birinin açılış yönünü ya da menteşesini çevirin ya da kapıyı kaydırın.",
                                   [i, j], middle(z), "physical", fix))
        if min_clear is not None:
            clear = sd.sizes(entity_fields(o))["clear"]
            if clear < min_clear - 1e-9:
                out.append(finding("arch_door_clear_narrow",
                                   f"{o['number']} kapısının net geçişi {clear:.3f} m; proje kuralının en az {min_clear:.3f} m'sinden dar (proje kuralı).",
                                   "Kapıyı genişletin, kasayı inceltin ya da kanadın açılma açısını büyütün.",
                                   [i], middle(jambs(o)), "rule"))
    rank = {"arch_door_swing_clash": 0, "arch_door_front_blocked": 1, "arch_door_clear_narrow": 2}
    out.sort(key=lambda f: (rank[f["code"]], f["at"]["x"], f["at"]["y"]))
    seen = {}
    for f in out:
        seen[f["key"]] = seen.get(f["key"], 0) + 1
        if seen[f["key"]] > 1:
            f["key"] += f"#{seen[f['key']]}"
    return out


def entity_fields(o):
    """The door as ADR 0777's sizes read it."""
    out = {"form": "door", "width": o["width"], "operation": o.get("operation", "swing"), "leaves": o.get("leaves", 1),
           "hand": o.get("hand", "start"), "frame": o.get("frame"), "build": o.get("build")}
    return out


def door_entity(i, o):
    e = {"kind": "opening", "id": i, "layerId": o.get("layer", "kapi"), "attrs": {}, "host": UID(o["host"]),
         "at": o["at"], "width": o["width"], "form": "door"}
    if o.get("leaves", 1) != 1:
        e["leaves"] = o["leaves"]
    if o.get("operation", "swing") != "swing":
        e["operation"] = o["operation"]
    if o.get("hand", "start") != "start":
        e["hand"] = o["hand"]
    if o.get("side", "left") != "left":
        e["side"] = o["side"]
    e["frame"] = o["frame"]
    e["head"] = 2.1
    e["number"] = o["number"]
    if o.get("build"):
        e["build"] = {k: v for k, v in o["build"].items()}
    e["place"] = [P(*p) for p in jambs(o)]
    return e


def filler(i):
    """An id kept: a short line on a hidden layer (the KCAD v2 reference writer gives each object its slot)."""
    return {"kind": "line", "id": i, "layerId": "bos", "attrs": {}, "a": P(-1.0, -5.0 - i), "b": P(-0.9, -5.0 - i)}


def setup(scene):
    ents = []
    levels = scene.get("levels", {})
    for i in ids_of(scene):
        if i in scene["walls"]:
            (a, b, lv) = WALLS[i]
            ents.append({"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)],
                         "width": T, "justify": "center", "arch": {"level": levels.get(i, lv)}})
        elif i in scene["columns"]:
            c, s = scene["columns"][i]
            ents.append({"kind": "column", "id": i, "layerId": "mimari", "attrs": {}, "at": P(*c), "width": s,
                         "depth": s, "arch": {"level": "zemin"}})
        elif i in scene["fixtures"]:
            ents.append({"kind": "insert", "id": i, "layerId": "mimari", "attrs": {}, "block": BLOCK,
                         "p": P(*scene["fixtures"][i]), "scale": 1.0, "rotation": 0.0, "arch": {"level": "zemin"}})
        elif i in scene["doors"]:
            ents.append(door_entity(i, scene["doors"][i]))
        else:
            ents.append(filler(i))
    locked = scene.get("locked", ("kilitli",))
    return {
        "format": "kentos.document", "version": 2, "name": "Özel kapıların çarpışması",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0},
                                         {"id": "kat-1", "name": "1. kat", "elevation": 3.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("kilitli", "Kilitli kapılar",
                                                                        locked="kilitli" in locked),
                   layer("mimari", "Mimari"), layer("bos", "Boş", visible=False)],
        "activeLayer": "kapi",
        "styles": {"items": [], "categories": []},
        "blocks": [{"id": BLOCK, "name": "Dolap", "base": P(0, 0),
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0),
                                  "b": P(0.6, 0.6)}]}],
        "entities": ents,
    }


def base():
    return {"walls": dict.fromkeys(WALLS), "columns": dict(COLUMNS), "fixtures": dict(FIXTURES),
            "doors": copy.deepcopy(DOORS)}


def audit(scene, inp=None, min_clear=None):
    inp = inp or {"checks": ["doors"]}
    return {"op": "execute", "input": inp, "within": 1e-9,
            "result": {"status": "completed",
                       "output": {"findings": findings(scene, min_clear), "outside": 0, "excepted": 0,
                                  "revision": "$current"}, "warnings": []},
            "expect": {"revision": "same"}}


def flip_step(i, what):
    return {"op": "execute", "command": "cad.openings.edit",
            "input": {"uids": [f"$uidOf:{i}"], "operation": {"kind": "flip", "what": what}}, "result": "$any",
            "expect": {"revision": "changed"}}


def cases():
    out = []

    def case(name, scene, steps=None):
        out.append({"name": name, "setup": setup(scene), "steps": steps or [audit(scene)]})

    def variant(change):
        s = base()
        change(s)
        return s

    case("Döner tambur kolona, katlanır paneller kolona ve kapıya, raylı sürme duvara çarpar; düzeltmeleriyle", base())
    case("Üç kanatlı döner kapı: tambur aynı", variant(lambda s: s["doors"][8].update(leaves=3)))
    case("Tamburu 1,6 m: kolona yetişmez", variant(lambda s: s["doors"][8]["build"].update(drum=1.6)))
    case("Kolon kalkınca tamburun çarpışması kalkar", variant(lambda s: s["columns"].pop(5)))
    case("Öbür yüze açılan katlanır kapı kimseye çarpmaz", variant(lambda s: s["doors"][9].update(side="right")))
    case("Dört panelli katlanır kapı: panel sığlaşır, kapı kalkar, kolon kalır",
         variant(lambda s: s["doors"][9].update(leaves=4)))
    case("Menteşesi sonda katlanır kapı: alanı aynı", variant(lambda s: s["doors"][9].update(hand="end")))
    case("Katlanır kapının iki yüzü de dolu: düzeltme yok",
         variant(lambda s: s["columns"].update({14: ((3.9, -0.4), 0.3)})))
    case("Raylı sürme öbür yüzde kayar: duvara çarpmaz", variant(lambda s: s["doors"][10].update(side="left")))
    case("Raysız sürme kapı duvarın içinde kayar", variant(lambda s: s["doors"][10]["build"].pop("track")))
    case("İki kanatlı raylı sürme: kanatlar duvarın içinde",
         variant(lambda s: s["doors"][10].update(leaves=2, width=1.4)))
    case("Kilitsiz katlanır kapı düzeltme alır", variant(lambda s: s.update(locked=())))
    case("Tek yana toplanan dört panel", variant(lambda s: s["doors"][11]["build"].pop("bothSides")))
    case("Donatı duvarın öbür yanına taşınınca katlanır kapı serbest",
         variant(lambda s: s["fixtures"].update({7: (8.2, -1.4)})))
    case("K3'ün menteşesi sonda", variant(lambda s: s["doors"][12].update(hand="end")))
    case("K4 öbür yüze açılınca tamburla çarpışma kalkar", variant(lambda s: s["doors"][13].update(side="right")))
    case("K4'ün duvarı başka katta: tamburla çarpışmaz", variant(lambda s: s.update(levels={3: "kat-1", 4: "kat-1"})))
    case("Proje kuralı: 1 m net geçişten dar kapılar da",
         base(), [audit(base(), {"checks": ["doors"], "minClearWidth": 1.0}, 1.0)])

    # The fixes applied: the door turned, the audit again.
    s = base()
    after = copy.deepcopy(s)
    after["doors"][9]["side"] = "right"
    case("Düzelt: katlanır kapı öbür yüze çevrilir, çarpışması kalkar", s, [audit(s), flip_step(9, "side"),
                                                                          audit(after)])
    after = copy.deepcopy(s)
    after["doors"][10]["side"] = "left"
    case("Düzelt: raylı sürme öbür yüze çevrilir", s, [flip_step(10, "side"), audit(after)])

    # A column in front of the sliding door's other face: its fix turns it to slide the other way.
    s = variant(lambda sc: sc["fixtures"].update({7: (11.2, 0.12)}))
    after = copy.deepcopy(s)
    after["doors"][10]["hand"] = "end"
    case("Öbür yüzde donatı: sürmenin düzeltmesi menteşe yanı", s, [audit(s), flip_step(10, "hand"), audit(after)])
    case("Döner kapı çevrilse de tamburu aynı yerde", base(), [flip_step(8, "side"), audit(base())])
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    value = {"format": "kentos.command-cases", "version": 1, "command": "cad.arch.audit", "commandVersion": 1,
             "title": "Döner, katlanır ve raylı sürme kapının çarpışması",
             "note": "ADR 0787 §1. Beklenenler scripts/fixtures/door_clash_cases.py'nin ADR'den yazılmış kurallarından, "
                     "KentOS kodu olmadan. Her durumun kendi çizimi var: dört duvar, iki kolon, bir donatı, D1 döner, "
                     "KT1 ve KT2 katlanır (KT2 kilitli katmanda), S1 raylı sürme, K3 ve K4 kanatlı kapı.",
             "setup": setup(base()), "cases": cases()}
    text = json.dumps(value, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"farklı: {OUT.relative_to(ROOT)}")
            sys.exit(1)
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(value['cases'])} durum)")


if __name__ == "__main__":
    main()
