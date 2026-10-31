#!/usr/bin/env python3
"""Revolving, pocket and folding doors and the sliding door's overlap and track as commands (docs/adr/0777), from the
ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.openings.create.special.json, cad.openings.query.special.json,
cad.openings.edit.special.json and cad.openingtypes.edit.special.json; `--check` compares and writes nothing. The walls
are straight and run along x, so an opening's jambs are `(x0 + s, y ± t/2)`. The rules, as the ADR gives them:

- a revolving door has 3 or 4 wings, a pocket door 1 or 2 leaves, a folding door 2–6 panels; the three are doors only;
- the construction's `drum` is a revolving door's (at most its light), `lip` a pocket door's, `bothSides` a folding door's
  (an even number of panels), `track` and `overlap` a sliding one's (an overlap two leaves', under the light); a switch
  turned off is not written;
- its widths (ADR 0772's, rough → unit → leaf): a revolving door's wings are the drum's radius (the drum, none: the
  light), its clear width the throat D·cos(180°/n); a pocket door's clear width the light less each leaf's lip (none:
  5 cm); a folding door's panels the light / n, its clear width the light less the panels folded flat, n·tl (tl the
  leaf's thickness, none: 4 cm); two sliding leaves with an overlap are (light + overlap)/2 each, its clear width the
  light less one leaf;
- a pocket door's pocket from the jamb at its `hand` (two leaves: each at its own) runs w − lip − fit − frame + 1 cm
  into the wall, tl + 2 cm across, centred; its wall must be that and 2,5 cm each side thick (else
  `opening_pocket_thin`) and its pocket on the door's edge of the wall (else `opening_pocket_short`); no other opening
  may stand in it (`opening_pocket_short`).

ADR 0792 (three- and four-leaf sliding doors and windows) is written to the `.multislide.json` files of the same four
commands: a sliding door or window of 1–4 leaves; `tracks` 2 or 3 only on three or four; `bothSides` on a sliding one
only with four leaves and two tracks; an overlap under the light's n-th part; the leaves, clear width, tracks and layout
(`slide_multi`).
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

PLAY, LIP, LEAF, SKIN, SLIDE_LAP = 0.01, 0.05, 0.04, 0.025, 0.05
FIELDS = ["width", "leaves", "operation", "swing", "showSwing", "frame", "sill", "head", "depth", "sizeBy", "fit",
          "leafThickness", "split", "frameDepth", "frameOffset", "threshold", "casing", "sillBoard", "mullions", "transoms",
          "useZone", "drum", "overlap", "track", "bothSides", "lip", "tracks"]
BUILD = FIELDS[9:]
LABEL = {"swing": "Kanatlı", "slide": "Sürme", "fixed": "Sabit", "revolving": "Döner", "pocket": "Cebe giren",
         "folding": "Katlanır"}


def num(x):
    if isinstance(x, float) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def P(x, y):
    return {"x": float(x), "y": float(y)}


def UID(n):
    return f"0192f5a1-7770-7000-8000-{n:012x}"


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# The walls: (start x, y, end x, thickness), all along x but the fourth, which turns up at its corner.
WALLS = {1: (0.0, 0.0, 12.0, 0.3), 2: (0.0, 5.0, 6.0, 0.1), 3: (0.0, 10.0, 4.0, 0.3), 5: (20.0, 0.0, 32.0, 0.3)}
CORNER = 4  # (0, 15) → (5, 15) → (5, 19), 0.3 thick


def wall(id):
    if id == CORNER:
        pts = [P(0, 15), P(5, 15), P(5, 19)]
        t = 0.3
    else:
        x0, y, x1, t = WALLS[id]
        pts = [P(x0, y), P(x1, y)]
    return {"kind": "wall", "id": id, "layerId": "duvar", "attrs": {}, "pts": pts, "width": t, "justify": "center"}


def wall_length(id):
    return 9.0 if id == CORNER else WALLS[id][2] - WALLS[id][0]


def thickness(id):
    return 0.3 if id == CORNER else WALLS[id][3]


def place(host, at, width):
    s0, s1 = at - width / 2, at + width / 2
    if host == CORNER:
        t = 0.15
        return [P(s0, 15 + t), P(s0, 15 - t), P(s1, 15 + t), P(s1, 15 - t)]
    x0, y, _, t = WALLS[host]
    return [P(x0 + s0, y + t / 2), P(x0 + s0, y - t / 2), P(x0 + s1, y + t / 2), P(x0 + s1, y - t / 2)]


# ── Types ────────────────────────────────────────────────────────────────────────────────────────────────────────

D01 = {"id": "d01", "name": "D01", "version": 1, "form": "door", "width": 2.4, "leaves": 4, "operation": "revolving",
       "frame": 0.1, "head": 2.4, "build": {"drum": 2.0}}
CG1 = {"id": "cg1", "name": "CG1", "version": 1, "form": "door", "width": 0.9, "operation": "pocket", "frame": 0.03,
       "head": 2.1, "build": {"leafThickness": 0.04, "lip": 0.06}}
KT1 = {"id": "kt1", "name": "KT1", "version": 1, "form": "door", "width": 1.6, "leaves": 4, "operation": "folding",
       "frame": 0.04, "head": 2.1}
TYPES = [D01, CG1, KT1]
KAPI90 = {"id": "kentos-kapi-90", "name": "Kapı 90", "form": "door", "width": 0.9, "frame": 0.05, "head": 2.1}


def build_json(b):
    out = {}
    for k in BUILD:
        v = (b or {}).get(k)
        if v is None or v is False:
            continue
        out[k] = v if isinstance(v, (bool, str, int)) and not isinstance(v, float) else float(v)
    return out or None


def type_json(t):
    out = {"id": t["id"], "name": t["name"], "version": t["version"], "form": t["form"], "width": float(t["width"])}
    if t.get("leaves", 1) != 1:
        out["leaves"] = t["leaves"]
    if t.get("operation", "swing") != "swing":
        out["operation"] = t["operation"]
    for k in ("swing", "frame", "sill", "head", "depth"):
        if t.get(k) is not None:
            out[k] = float(t[k])
    b = build_json(t.get("build"))
    if b:
        out["build"] = b
    return out


# ── Openings ─────────────────────────────────────────────────────────────────────────────────────────────────────


def opening(host, at, width, **f):
    o = {"host": host, "at": at, "width": width, "form": "door", "leaves": 1, "operation": "swing", "hand": "start"}
    o.update(f)
    return o


def from_type(t, host, at, number=None, **own):
    o = opening(host, at, t["width"], leaves=t.get("leaves", 1), operation=t.get("operation", "swing"))
    for k in ("swing", "frame", "sill", "head"):
        if t.get(k) is not None:
            o[k] = t[k]
    o["build"] = copy.deepcopy(t.get("build"))
    o["style"], o["typeVersion"] = t["id"], t["version"]
    if number:
        o["number"] = number
    over = []
    for k, v in own.items():
        if k in BUILD:
            b = dict(o.get("build") or {})
            b[k] = v
            o["build"] = b
        else:
            o[k] = v
        over.append(k)
    if over:
        o["overrides"] = [k for k in FIELDS if k in over]
    return o


def from_kapi(host, at, number=None, **own):
    """An opening made without a type: KentOS's first door (Kapı 90), the values given over it."""
    o = opening(host, at, KAPI90["width"], frame=0.05, head=2.1, style="kentos-kapi-90")
    if number:
        o["number"] = number
    for k, v in own.items():
        if k == "build":
            o["build"] = {x: y for x, y in v.items() if y is not False} or None
        else:
            o[k] = v
    return o


def stored(o, id, layer_id="kapi"):
    out = {"kind": "opening", "id": id, "layerId": layer_id, "attrs": {}, "host": f"$uidOf:{o['host']}",
           "at": float(o["at"]), "width": float(o["width"]), "form": o["form"]}
    if o.get("leaves", 1) != 1:
        out["leaves"] = o["leaves"]
    if o.get("operation", "swing") != "swing":
        out["operation"] = o["operation"]
    if o.get("hand", "start") != "start":
        out["hand"] = o["hand"]
    if o.get("side", "left") != "left":
        out["side"] = o["side"]
    for k in ("swing", "frame", "sill", "head"):
        if o.get(k) is not None:
            out[k] = float(o[k])
    for k in ("style", "number"):
        if o.get(k) is not None:
            out[k] = o[k]
    if o.get("typeVersion") is not None:
        out["typeVersion"] = o["typeVersion"]
    if o.get("overrides"):
        out["overrides"] = o["overrides"]
    b = build_json(o.get("build"))
    if b:
        out["build"] = b
    out["place"] = place(o["host"], o["at"], o["width"])
    return out


def sizes(o):
    """Its widths (ADR 0772 §2 with ADR 0777)."""
    b = o.get("build") or {}
    fit = b.get("fit") or 0.0
    frame = o.get("frame") or 0.0
    rough = o["width"]
    unit = rough - 2.0 * fit
    leaf = unit - 2.0 * frame
    t = b.get("leafThickness") if b.get("leafThickness") is not None else LEAF
    op, n = o.get("operation", "swing"), o.get("leaves", 1)
    angle = o.get("swing") if o.get("swing") is not None else 90.0
    lip = b.get("lip") if b.get("lip") is not None else LIP
    drum = (b.get("drum") if b.get("drum") is not None else leaf) if op == "revolving" else None
    multi = slide_multi(o, leaf) if op == "slide" and o["form"] in ("door", "window") and n in (3, 4) else None
    if op == "slide" and n == 2:
        w = (leaf + b["overlap"]) / 2.0 if b.get("overlap") is not None else leaf * (1.0 + SLIDE_LAP) / 2.0
        leaves = [w, w]
    elif multi:
        leaves = [multi["width"]] * n
    elif op == "revolving":
        leaves = [drum / 2.0] * n
    elif op == "folding":
        leaves = [leaf / n] * n
    elif n == 2:
        share = b.get("split") if b.get("split") is not None else 0.5
        leaves = [leaf * share, leaf * (1.0 - share)]
    else:
        leaves = [leaf]
    pockets = []
    if op == "pocket":
        def pocket(end, w):
            return {"atEnd": end, "length": max(w - lip - fit - frame + PLAY, 0.0), "width": t + 2.0 * PLAY}
        pockets = [pocket(o.get("hand") == "end", leaves[0])] if n == 1 else [pocket(False, leaves[0]),
                                                                               pocket(True, leaves[1])]
    r = math.radians(angle)
    if op == "fixed":
        clear = 0.0
    elif multi:
        clear = multi["clear"]
    elif op == "slide":
        clear = leaf if n == 1 else leaf - leaves[0]
    elif op == "revolving":
        clear = drum * math.cos(math.pi / n)
    elif op == "pocket":
        clear = leaf - lip * n
    elif op == "folding":
        clear = leaf - t * n
    else:
        clear = leaf - sum(max(w * math.cos(r), 0.0) + t * max(math.sin(r), 0.0) for w in leaves)
    out = {"rough": rough, "unit": unit, "leaf": leaf, "leaves": leaves}
    if o["form"] == "door":
        out["clear"] = max(clear, 0.0)
    if drum is not None:
        out["drum"] = drum
    if pockets:
        out["pockets"] = pockets
    if multi:
        out["tracks"] = multi["tracks"]
        out["panels"] = multi["panels"]
    out["sizeBy"] = b.get("sizeBy") or "rough"
    out["nominal"] = {"rough": rough, "unit": unit, "leaf": leaf}[out["sizeBy"]]
    return out


def slide_multi(o, leaf):
    """ADR 0792 §1, from its words: n = 3 or 4 leaves, t tracks (none: 2), both jambs only with four (two from each);
    from the jamb they stack at the k-th leaf on track k slides while k < t, the rest stay on track 0; neighbours on
    different tracks overlap by o (none: 5 % of the light over the n − 1 meetings), on one track they butt; each leaf
    (light + laps · o)/n wide. Open, the sliding leaves lie over the first: a single jamb's stack frees the light less a
    leaf, or up to the first leaf that stays, t·(w − o) from the jamb, less a leaf; both jambs free the light less two."""
    b = o.get("build") or {}
    n = o["leaves"]
    t = b.get("tracks") or 2
    both = bool(b.get("bothSides")) and n == 4
    lap = b["overlap"] if b.get("overlap") is not None else leaf * SLIDE_LAP / (n - 1)
    if both:
        order = [0, 1, 1, 0]
    else:
        order = [k if k < t else 0 for k in range(n)]
    laps = sum(1 for a, z in zip(order, order[1:]) if a != z)
    w = (leaf + laps * lap) / n
    if both:
        clear = leaf - 2 * w
        letters = "OXXO"
    else:
        clear = t * (w - lap) - w if t < n else leaf - w
        letters = "".join("X" if 0 < k < t else "O" for k in range(n))
        if o.get("hand") == "end":
            letters = letters[::-1]
    return {"tracks": t, "width": w, "clear": clear, "panels": letters}


def reach(o):
    s0, s1 = o["at"] - o["width"] / 2, o["at"] + o["width"] / 2
    for p in sizes(o).get("pockets", []):
        if p["atEnd"]:
            s1 += p["length"]
        else:
            s0 -= p["length"]
    return s0, s1


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────

BASE = {
    6: from_type(D01, 5, 2.0, number="D1"),
    7: from_type(CG1, 5, 6.0, number="CG1"),
    8: from_type(KT1, 5, 9.0, number="KT1"),
    9: opening(5, 11.2, 1.2, leaves=2, operation="slide", frame=0.04, head=2.1, number="S1",
               build={"overlap": 0.1, "track": True}),
    10: from_kapi(1, 10.5, number="K1"),
}
IDS = [1, 2, 3, 4, 5] + sorted(BASE)
NEW = max(IDS) + 1


def setup():
    ents = [wall(i) for i in (1, 2, 3, 4, 5)]
    for id in sorted(BASE):
        e = stored(BASE[id], id)
        e["host"] = UID(BASE[id]["host"])
        ents.append(e)
    return {
        "format": "kentos.document", "version": 2, "name": "Özel kapılar",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad", "arch": {"openingTypes": [type_json(t) for t in TYPES]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı")],
        "activeLayer": "kapi",
        "styles": {"items": [], "categories": []},
        "entities": ents,
    }


def problem(o, path):
    """An opening's refusal as the contract orders ADR 0777's rules: the operation's form, its leaves, then the
    construction's fields (drum, lip, both sides, track, overlap); none."""
    op, n = o.get("operation", "swing"), o.get("leaves", 1)
    special = op in ("revolving", "pocket", "folding")
    if special and o["form"] != "door":
        form = {"window": "pencere", "opening": "boşluk"}[o["form"]]
        return failed("invalid_opening", f"{LABEL[op]} açılış yalnız kapınındır; {form} için Kanatlı, Sürme ya da Sabit seçin.",
                      f"{path}.operation")
    lo, hi = {"revolving": (3, 4), "folding": (2, 6), "slide": (1, 4)}.get(op, (1, 2))
    if not lo <= n <= hi:
        word = "panel" if op == "folding" else "kanat"
        if op == "slide":
            words = f"Sürme açıklığın kanat sayısı {n}; 1 ile 4 arasında olmalı."
        elif not special:
            words = f"Açıklığın kanat sayısı {n}; 1 ya da 2 olmalı."
        elif lo + 1 == hi:
            words = f"{LABEL[op]} kapının {word} sayısı {n}; {lo} ya da {hi} olmalı."
        else:
            words = f"{LABEL[op]} kapının {word} sayısı {n}; {lo} ile {hi} arasında olmalı."
        return failed("invalid_opening", words, f"{path}.leaves")
    b = o.get("build") or {}
    s = sizes(o)

    def only(name, what, of):
        if op != of:
            return failed("invalid_opening", f"{what} yalnız {LABEL[of].lower()} açıklığındır; kaldırın ya da açılışı {LABEL[of]} yapın.",
                          f"{path}.build.{name}")
    if b.get("drum") is not None:
        why = only("drum", "Tambur çapı", "revolving")
        if why:
            return why
        if b["drum"] > s["leaf"] + 1e-9:
            return failed("invalid_opening",
                          f"Döner kapının tamburu ({num(b['drum'])} m) ışıktan ({s['leaf']:.3f} m) geniş; tambur kasanın içine sığmalı. Çapı küçültün ya da açıklığı genişletin.",
                          f"{path}.build.drum")
    if b.get("lip") is not None:
        why = only("lip", "Cepten kalan pay", "pocket")
        if why:
            return why
    if b.get("bothSides"):
        if op == "slide":
            if n != 4:
                return failed("invalid_opening",
                              f"İki yana açılma dört kanatlı sürmenindir ({n} kanat); kanat sayısını 4 yapın ya da tek yana açın.",
                              f"{path}.build.bothSides")
        elif only("bothSides", "İki yana toplanma", "folding"):
            return only("bothSides", "İki yana toplanma", "folding")
        elif n % 2:
            return failed("invalid_opening", f"{n} panel iki yana eşit toplanmaz; panel sayısını çift yapın ya da tek yana toplayın.",
                          f"{path}.build.bothSides")
    if b.get("track"):
        why = only("track", "Ray", "slide")
        if why:
            return why
    if b.get("overlap") is not None:
        why = only("overlap", "Kanat bindirmesi", "slide")
        if why:
            return why
        if n < 2:
            return failed("invalid_opening", "Kanat bindirmesi çok kanatlı sürmenindir; tek kanatlıda kaldırın.",
                          f"{path}.build.overlap")
        if n == 2 and b["overlap"] >= s["leaf"]:
            return failed("invalid_opening",
                          f"Kanat bindirmesi ({num(b['overlap'])} m) ışıktan ({s['leaf']:.3f} m) küçük olmalı.",
                          f"{path}.build.overlap")
        if n > 2 and b["overlap"] >= s["leaf"] / n:
            part = "üçte" if n == 3 else "dörtte"
            return failed("invalid_opening",
                          f"Kanat bindirmesi ({num(b['overlap'])} m) ışığın {part} birinden ({s['leaf'] / n:.3f} m) küçük olmalı.",
                          f"{path}.build.overlap")
    if b.get("tracks") is not None:
        why = only("tracks", "Ray sayısı", "slide")
        if why:
            return why
        if n < 3:
            return failed("invalid_opening",
                          "Ray sayısı üç ya da dört kanatlı sürmenindir; kaldırın ya da kanat sayısını artırın.",
                          f"{path}.build.tracks")
        if b["tracks"] not in (2, 3):
            return failed("invalid_opening", f"Ray sayısı {b['tracks']}; 2 ya da 3 olmalı.", f"{path}.build.tracks")
        if b["tracks"] == 3 and b.get("bothSides"):
            return failed("invalid_opening",
                          "İki yana açılan dört kanat iki raydadır; ray sayısını 2 yapın ya da tek yana açın.",
                          f"{path}.build.tracks")
    return None


def on_wall(o, path):
    """A pocket door its wall cannot take (ADR 0777 §2): too thin, or its pocket off the door's edge."""
    if o.get("operation") != "pocket":
        return None
    t = thickness(o["host"])
    need = max(p["width"] for p in sizes(o)["pockets"]) + 2 * SKIN
    if need > t + 1e-9 * max(t, 1.0):
        return failed("opening_pocket_thin",
                      f"Cebe giren kapının cebi için duvar en az {need:.3f} m kalınlığında olmalı (cep ve iki yanında 2,5 cm duvar); bu duvar {t:.3f} m. Daha kalın bir duvar seçin, kanadı inceltin ya da kayar kapı kullanın.",
                      f"{path}.operation")
    r0, r1 = reach(o)
    edges = [(0.0, 5.0), (5.0, 9.0)] if o["host"] == CORNER else [(0.0, wall_length(o["host"]))]
    if not any(r0 >= a - 1e-12 and r1 <= z + 1e-12 for a, z in edges):
        return failed("opening_pocket_short",
                      f"Cebe giren kapının cebi duvarın ucundan ya da köşesinden taşıyor: kapı ve cebi duvar boyunca {r0:.3f}–{r1:.3f} m, duvarın uzunluğu {wall_length(o['host']):.3f} m; cep kapıyla aynı kenarda kalmalı. Kapıyı kaydırın, menteşe yanını değiştirin ya da daraltın.",
                      f"{path}.at")
    return None


def into_others(o, others, path):
    """Its pocket into another opening of its wall (ADR 0777 §2)."""
    a = reach(o)
    span = (o["at"] - o["width"] / 2, o["at"] + o["width"] / 2)
    for x in others:
        if x["host"] != o["host"]:
            continue
        b = reach(x)
        if a[0] < b[1] - 1e-9 and b[0] < a[1] - 1e-9:
            xs = (x["at"] - x["width"] / 2, x["at"] + x["width"] / 2)
            if not (span[0] < xs[1] - 1e-9 and xs[0] < span[1] - 1e-9):
                return failed("opening_pocket_short",
                              f"Cebe giren kapının cebi aynı duvardaki başka bir açıklığa ({b[0]:.3f}–{b[1]:.3f} m) giriyor: kapı ve cebi duvar boyunca {a[0]:.3f}–{a[1]:.3f} m. Cebin uzunluğu duvarda boş kalmalı; kapıyı kaydırın, menteşe yanını değiştirin ya da öbür açıklığı taşıyın.",
                              path)
    return None


def doc(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": setup(), "cases": cases}


NOTE = ("ADR 0777. Beklenenler scripts/fixtures/special_doors_command_cases.py'nin ADR'den yazılmış kurallarından, "
        "KentOS kodu olmadan. Kurulumda beş duvar (12 m, 0,10 m'lik ince, 4 m'lik kısa, köşeli, özel kapıların) ve beş "
        "açıklık (D01 döner, CG1 cebe giren, KT1 katlanır, raylı iki kanatlı sürme, Kapı 90) var; yeni nesne 11'den başlar.")


# ── cad.openings.create ──────────────────────────────────────────────────────────────────────────────────────────


def create_cases():
    out = []

    def case(name, given, made=None, why=None):
        """`given` the input's opening, `made` the opening the reference says it is (None: refused with `why`)."""
        step = {"op": "execute", "input": {"layerId": "kapi", "openings": [given]}, "within": 1e-9}
        if made is None:
            step["result"] = why
            step["expect"] = {"ids": IDS, "revision": "same"}
        else:
            step["result"] = {"status": "completed", "output": {"uids": ["$any"], "ids": [NEW], "revision": "$current"},
                              "warnings": []}
            step["expect"] = {"ids": IDS + [NEW], "entities": {str(NEW): stored(made, NEW)}, "canUndo": True,
                              "dirty": True, "revision": "changed"}
        out.append({"name": name, "steps": [step]})

    def try_make(given, made, path="openings[0]"):
        why = problem(made, path) or on_wall(made, path) or into_others(made, BASE.values(), path)
        return (None, why) if why else (made, None)

    def add(name, given, made):
        m, why = try_make(given, made)
        case(name, given, m, why)

    H = lambda id: f"$uidOf:{id}"  # noqa: E731
    # Döner.
    add("Döner: D01 tipiyle, tamburu 2 m, dört kanat", {"host": H(1), "at": 3.0, "type": "D01"}, from_type(D01, 1, 3.0))
    add("Döner: tipsiz, üç kanat, tambur 1,8 m",
        {"host": H(1), "at": 3.0, "width": 2.2, "operation": "revolving", "leaves": 3, "build": {"drum": 1.8}},
        from_kapi(1, 3.0, width=2.2, operation="revolving", leaves=3, build={"drum": 1.8}))
    add("Döner: kanat sayısı verilmezse ret (bir kanat)", {"host": H(1), "at": 3.0, "width": 2.2, "operation": "revolving"},
        from_kapi(1, 3.0, width=2.2, operation="revolving"))
    add("Döner: beş kanat reddedilir",
        {"host": H(1), "at": 3.0, "width": 2.2, "operation": "revolving", "leaves": 5},
        from_kapi(1, 3.0, width=2.2, operation="revolving", leaves=5))
    add("Döner: tambur ışıktan geniş reddedilir",
        {"host": H(1), "at": 3.0, "width": 2.0, "operation": "revolving", "leaves": 4, "build": {"drum": 2.5}},
        from_kapi(1, 3.0, width=2.0, operation="revolving", leaves=4, build={"drum": 2.5}))
    add("Döner pencere olmaz", {"host": H(1), "at": 3.0, "form": "window", "operation": "revolving", "leaves": 4},
        {**opening(1, 3.0, 1.2, form="window", leaves=4, operation="revolving", frame=0.06, sill=0.9, head=2.2),
         "style": "kentos-pencere-120"})
    add("Kanatlı kapıya tambur verilmez", {"host": H(1), "at": 3.0, "build": {"drum": 0.8}},
        from_kapi(1, 3.0, build={"drum": 0.8}))
    # Cebe giren.
    add("Cebe giren: CG1 tipiyle, cebi başta duvarın içinde", {"host": H(1), "at": 3.0, "type": "CG1"},
        from_type(CG1, 1, 3.0))
    add("Cebe giren: kendi payıyla, istisnası söylenir", {"host": H(1), "at": 3.0, "type": "CG1", "build": {"lip": 0.1}},
        from_type(CG1, 1, 3.0, lip=0.1))
    add("Cebe giren: duvarın başında cebi taşar", {"host": H(1), "at": 0.6, "type": "CG1"}, from_type(CG1, 1, 0.6))
    add("Cebe giren: duvarın başında, menteşe sonda: cep öbür yana", {"host": H(1), "at": 0.6, "type": "CG1", "hand": "end"},
        {**from_type(CG1, 1, 0.6), "hand": "end"})
    add("Cebe giren: 10 cm'lik duvara sığmaz", {"host": H(2), "at": 3.0, "type": "CG1"}, from_type(CG1, 2, 3.0))
    add("Cebe giren: 3 cm kanatla 10 cm'lik duvara sığar",
        {"host": H(2), "at": 3.0, "type": "CG1", "build": {"leafThickness": 0.03}},
        from_type(CG1, 2, 3.0, leafThickness=0.03))
    add("Cebe giren: cebi köşeyi döner, reddedilir", {"host": H(CORNER), "at": 4.3, "type": "CG1", "hand": "end"},
        {**from_type(CG1, CORNER, 4.3), "hand": "end"})
    add("Cebe giren: cebi duvardaki kapıya girer", {"host": H(1), "at": 9.0, "type": "CG1", "hand": "end"},
        {**from_type(CG1, 1, 9.0), "hand": "end"})
    add("Cebe giren: iki kanat iki cebe", {"host": H(1), "at": 5.0, "width": 1.6, "operation": "pocket", "leaves": 2},
        from_kapi(1, 5.0, width=1.6, operation="pocket", leaves=2))
    add("Cebe giren: üç kanat reddedilir", {"host": H(1), "at": 5.0, "width": 1.6, "operation": "pocket", "leaves": 3},
        from_kapi(1, 5.0, width=1.6, operation="pocket", leaves=3))
    add("Katlanır kapıya cep payı verilmez",
        {"host": H(1), "at": 5.0, "width": 1.6, "operation": "folding", "leaves": 4, "build": {"lip": 0.05}},
        from_kapi(1, 5.0, width=1.6, operation="folding", leaves=4, build={"lip": 0.05}))
    # Katlanır.
    add("Katlanır: KT1 tipiyle, dört panel", {"host": H(1), "at": 5.0, "type": "KT1"}, from_type(KT1, 1, 5.0))
    add("Katlanır: altı panel iki yana, 45°",
        {"host": H(1), "at": 5.0, "width": 2.4, "operation": "folding", "leaves": 6, "swing": 45.0,
         "build": {"bothSides": True}},
        from_kapi(1, 5.0, width=2.4, operation="folding", leaves=6, swing=45.0, build={"bothSides": True}))
    add("Katlanır: beş panel iki yana toplanmaz",
        {"host": H(1), "at": 5.0, "width": 2.0, "operation": "folding", "leaves": 5, "build": {"bothSides": True}},
        from_kapi(1, 5.0, width=2.0, operation="folding", leaves=5, build={"bothSides": True}))
    add("Katlanır: yedi panel reddedilir", {"host": H(1), "at": 5.0, "width": 2.4, "operation": "folding", "leaves": 7},
        from_kapi(1, 5.0, width=2.4, operation="folding", leaves=7))
    # Sürme.
    add("Sürme: iki kanat, 10 cm bindirme, raylı",
        {"host": H(1), "at": 5.0, "width": 1.6, "operation": "slide", "leaves": 2, "build": {"overlap": 0.1, "track": True}},
        from_kapi(1, 5.0, width=1.6, operation="slide", leaves=2, build={"overlap": 0.1, "track": True}))
    add("Sürme: kapalı ray yazılmaz",
        {"host": H(1), "at": 5.0, "operation": "slide", "build": {"track": False}},
        from_kapi(1, 5.0, operation="slide", build={"track": False}))
    add("Sürme: tek kanada bindirme verilmez", {"host": H(1), "at": 5.0, "operation": "slide", "build": {"overlap": 0.05}},
        from_kapi(1, 5.0, operation="slide", build={"overlap": 0.05}))
    add("Sürme: ışıktan uzun bindirme reddedilir",
        {"host": H(1), "at": 5.0, "width": 1.0, "operation": "slide", "leaves": 2, "build": {"overlap": 0.95}},
        from_kapi(1, 5.0, width=1.0, operation="slide", leaves=2, build={"overlap": 0.95}))
    add("Kanatlı kapıya ray verilmez", {"host": H(1), "at": 5.0, "build": {"track": True}},
        from_kapi(1, 5.0, build={"track": True}))
    return out


# ── cad.openings.query ───────────────────────────────────────────────────────────────────────────────────────────


def row(id):
    o = BASE[id]
    out = {"uid": f"$uidOf:{id}", "form": o["form"]}
    if o.get("number"):
        out["number"] = o["number"]
    if o.get("style"):
        out["type"] = o["style"]
        t = next((t for t in TYPES if t["id"] == o["style"]), None)
        out["typeName"] = t["name"] if t else KAPI90["name"]
        if t:
            out["typeVersion"] = o["typeVersion"]
    out["host"] = f"$uidOf:{o['host']}"
    out["sizes"] = sizes(o)
    return out


def uses(openings):
    """Each project type, how many openings follow it, how many with an older version, how many with exceptions."""
    out = []
    for t in TYPES:
        fs = [f for f in openings if f.get("style") == t["id"] and f.get("typeVersion") is not None]
        out.append({"type": type_json(t), "count": len(fs), "stale": sum(1 for f in fs if f["typeVersion"] < t["version"]),
                    "exceptions": sum(1 for f in fs if f.get("overrides"))})
    return out


def query_cases():
    out = []

    def case(name, input, ids):
        out.append({"name": name, "steps": [{"op": "plan", "input": input, "within": 1e-9, "result": {
            "status": "completed", "output": {"openings": [row(i) for i in ids], "types": uses(BASE.values()),
                                              "revision": "$current"}, "warnings": []}}]})
    case("Bütün özel kapılar ve genişlikleri", {}, [6, 7, 8, 9, 10])
    case("Döner: tamburun boğazı, dört kanat", {"uids": ["$uidOf:6"]}, [6])
    case("Cebe giren: payı ve cebi", {"uids": ["$uidOf:7"]}, [7])
    case("Katlanır: paneller düz toplanır", {"uids": ["$uidOf:8"]}, [8])
    case("Sürme: bindirmeli iki kanat", {"uids": ["$uidOf:9"]}, [9])
    case("Kanatlı kapı yanında aynı kalır", {"uids": ["$uidOf:10"]}, [10])
    case("D01 tipinin kapıları", {"type": "D01"}, [6])
    case("CG1 tipinin kapıları adıyla", {"type": "cg1"}, [7])

    def made(name, given, o):
        """Made first with cad.openings.create, then read."""
        r = {"uid": f"$uidOf:{NEW}", "form": o["form"]}
        if o.get("style"):
            r["type"] = o["style"]
            t = next((t for t in TYPES if t["id"] == o["style"]), None)
            r["typeName"] = t["name"] if t else {"kentos-kapi-90": "Kapı 90", "kentos-pencere-120": "Pencere 120"}[o["style"]]
            if t:
                r["typeVersion"] = o["typeVersion"]
                if o.get("overrides"):
                    r["overrides"] = o["overrides"]
        r["host"] = f"$uidOf:{o['host']}"
        r["sizes"] = sizes(o)
        out.append({"name": name, "steps": [
            {"op": "execute", "command": "cad.openings.create", "input": {"layerId": "kapi", "openings": [given]},
             "within": 1e-9, "result": {"status": "completed", "output": {"uids": ["$any"], "ids": [NEW], "revision": "$current"},
                                        "warnings": []}},
            {"op": "plan", "input": {"uids": [f"$uidOf:{NEW}"]}, "within": 1e-9,
             "result": {"status": "completed", "output": {"openings": [r], "types": uses(list(BASE.values()) + [o]),
                                                          "revision": "$current"}, "warnings": []}}]})
    H = lambda id: f"$uidOf:{id}"  # noqa: E731
    made("Döner: üç kanat, tambur 1,8 m: boğazı 0,9 m",
         {"host": H(1), "at": 5.0, "width": 2.2, "operation": "revolving", "leaves": 3, "build": {"drum": 1.8}},
         from_kapi(1, 5.0, width=2.2, operation="revolving", leaves=3, build={"drum": 1.8}))
    made("Döner: tamburu verilmezse ışık kadar",
         {"host": H(1), "at": 5.0, "width": 2.0, "operation": "revolving", "leaves": 4},
         from_kapi(1, 5.0, width=2.0, operation="revolving", leaves=4))
    made("Döner: D01'in kendi tamburuyla, istisnası",
         {"host": H(1), "at": 5.0, "type": "D01", "build": {"drum": 1.6}}, from_type(D01, 1, 5.0, drum=1.6))
    made("Cebe giren: iki kanat iki cebe",
         {"host": H(1), "at": 5.0, "width": 1.6, "operation": "pocket", "leaves": 2},
         from_kapi(1, 5.0, width=1.6, operation="pocket", leaves=2))
    made("Cebe giren: iki kanat, pay 0,6",
         {"host": H(1), "at": 5.0, "width": 1.6, "operation": "pocket", "leaves": 2, "build": {"split": 0.6}},
         from_kapi(1, 5.0, width=1.6, operation="pocket", leaves=2, build={"split": 0.6}))
    made("Cebe giren: sonda, montaj boşluklu",
         {"host": H(1), "at": 5.0, "operation": "pocket", "hand": "end", "build": {"fit": 0.01}},
         from_kapi(1, 5.0, operation="pocket", hand="end", build={"fit": 0.01}))
    made("Cebe giren: kalın kanat, kısa pay",
         {"host": H(1), "at": 5.0, "operation": "pocket", "build": {"leafThickness": 0.05, "lip": 0.02}},
         from_kapi(1, 5.0, operation="pocket", build={"leafThickness": 0.05, "lip": 0.02}))
    made("Cebe giren: CG1, 3 cm kanat", {"host": H(1), "at": 5.0, "type": "CG1", "build": {"leafThickness": 0.03}},
         from_type(CG1, 1, 5.0, leafThickness=0.03))
    made("Katlanır: altı ince panel",
         {"host": H(1), "at": 5.0, "width": 2.4, "operation": "folding", "leaves": 6, "build": {"leafThickness": 0.03}},
         from_kapi(1, 5.0, width=2.4, operation="folding", leaves=6, build={"leafThickness": 0.03}))
    made("Katlanır: iki panel 60°, geçişi açıya bağlı değil",
         {"host": H(1), "at": 5.0, "width": 1.0, "operation": "folding", "leaves": 2, "swing": 60.0},
         from_kapi(1, 5.0, width=1.0, operation="folding", leaves=2, swing=60.0))
    made("Sürme: bindirmesiz iki kanat %5",
         {"host": H(1), "at": 5.0, "width": 1.6, "operation": "slide", "leaves": 2},
         from_kapi(1, 5.0, width=1.6, operation="slide", leaves=2))
    made("Sürme: tek kanat yüzde rayda",
         {"host": H(1), "at": 5.0, "operation": "slide", "build": {"track": True}},
         from_kapi(1, 5.0, operation="slide", build={"track": True}))
    made("Sürme pencere: 8 cm bindirme, net geçişi yok",
         {"host": H(1), "at": 5.0, "form": "window", "operation": "slide", "build": {"overlap": 0.08, "track": True}},
         {**opening(1, 5.0, 1.2, form="window", leaves=2, operation="slide", frame=0.06, sill=0.9, head=2.2,
                    build={"overlap": 0.08, "track": True}), "style": "kentos-pencere-120"})
    return out


# ── cad.openings.edit ────────────────────────────────────────────────────────────────────────────────────────────


def edit_cases():
    out = []

    def case(name, id, op, change=None, unset=()):
        """`change` the fields given (build's under `build`), `unset` the construction's fields taken away."""
        o = copy.deepcopy(BASE[id])
        for k, v in (change or {}).items():
            if k == "build":
                b = dict(o.get("build") or {})
                for x, y in v.items():
                    if y is False:
                        b.pop(x, None)
                    else:
                        b[x] = y
                o["build"] = b or None
            else:
                o[k] = v
        b = dict(o.get("build") or {})
        for k in unset:
            b.pop(k, None)
        o["build"] = b or None
        operation = {"kind": "set", **op}
        if unset:
            operation["unset"] = list(unset)
        others = [x for i, x in BASE.items() if i != id]
        why = problem(o, "uids[0]") or on_wall(o, "uids[0]") or into_others(o, others, "uids[0]")
        step = {"op": "execute", "input": {"uids": [f"$uidOf:{id}"], "operation": operation}, "within": 1e-9}
        if why:
            step["result"] = why
            step["expect"] = {"ids": IDS, "revision": "same"}
        else:
            step["result"] = {"status": "completed", "output": {"openings": [stored(o, id)], "revision": "$current"},
                              "warnings": []}
            step["expect"] = {"ids": IDS, "entities": {str(id): stored(o, id)}, "revision": "changed"}
        out.append({"name": name, "steps": [step]})
    case("Kanatlı kapı cebe giren olur, cebi başta", 10, {"operation": "pocket"}, {"operation": "pocket"})
    case("Cebe giren, menteşe sonda: cebi sona", 10, {"operation": "pocket", "hand": "end"},
         {"operation": "pocket", "hand": "end"})
    case("Geniş cebe giren duvarın sonunu aşar", 10, {"operation": "pocket", "hand": "end", "width": 1.6},
         {"operation": "pocket", "hand": "end", "width": 1.6})
    case("Kanatlı kapı üç panelli katlanır olur", 10, {"operation": "folding", "leaves": 3},
         {"operation": "folding", "leaves": 3})
    case("Döner olur ama tek kanatla reddedilir", 10, {"operation": "revolving"}, {"operation": "revolving"})
    case("Döner, dört kanat, 80 cm tambur", 10, {"operation": "revolving", "leaves": 4, "build": {"drum": 0.8}},
         {"operation": "revolving", "leaves": 4, "build": {"drum": 0.8}})
    case("Sürmenin bindirmesi 20 cm", 9, {"build": {"overlap": 0.2}}, {"build": {"overlap": 0.2}})
    case("Sürmenin rayı kapatılır, yazılmaz", 9, {"build": {"track": False}}, {"build": {"track": False}})
    case("Sürme tek kanada inerse bindirmesi reddedilir", 9, {"leaves": 1}, {"leaves": 1})
    case("Sürmenin bindirmesi ve rayı kaldırılır", 9, {}, None, unset=("overlap", "track"))
    case("Sürme kapı katlanır olunca rayı reddedilir", 9, {"operation": "folding", "leaves": 4},
         {"operation": "folding", "leaves": 4})
    case("Kapı iki kanatlı cebe giren olur, iki cep", 10, {"operation": "pocket", "leaves": 2, "width": 1.6},
         {"operation": "pocket", "leaves": 2, "width": 1.6})
    case("Cebe giren, kendi payıyla", 10, {"operation": "pocket", "build": {"lip": 0.1}},
         {"operation": "pocket", "build": {"lip": 0.1}})
    case("Cebe giren, 30 cm kanat duvara sığmaz", 10, {"operation": "pocket", "build": {"leafThickness": 0.3}},
         {"operation": "pocket", "build": {"leafThickness": 0.3}})
    case("Sürme cebe giren olunca rayı reddedilir", 9, {"operation": "pocket"}, {"operation": "pocket"})
    case("CG1'in kapısının cebi başka yana", 7, {"hand": "end"}, {"hand": "end"})
    case("CG1'in kapısı genişleyince cebi döner kapıya girer", 7, {"width": 2.0}, {"width": 2.0})
    case("KT1'in kapısı yedi panelli olmaz", 8, {"leaves": 7}, {"leaves": 7})
    case("D01'in kapısının tamburu ışıktan geniş olmaz", 6, {"build": {"drum": 2.5}}, {"build": {"drum": 2.5}})
    case("Kapı raylı iki kanatlı sürme olur", 10,
         {"operation": "slide", "leaves": 2, "build": {"overlap": 0.05, "track": True}},
         {"operation": "slide", "leaves": 2, "build": {"overlap": 0.05, "track": True}})
    case("Kapı iki yana katlanır olur", 10, {"operation": "folding", "leaves": 4, "build": {"bothSides": True}},
         {"operation": "folding", "leaves": 4, "build": {"bothSides": True}})
    case("Pencere cebe giren olmaz", 10, {"form": "window", "operation": "pocket"}, {"form": "window", "operation": "pocket"})
    return out


# ── cad.openingtypes.edit ────────────────────────────────────────────────────────────────────────────────────────


def types_cases():
    out = []

    def done(types, report, warnings=(), entities=None):
        r = {"status": "completed", "output": {"types": [type_json(t) for t in types], "report": report,
                                                "revision": "$current"}, "warnings": list(warnings)}
        e = {"ids": IDS, "revision": "changed"}
        if entities:
            e["entities"] = entities
        return r, e

    def report(applied=(), fields=(), refused=()):
        return {"applied": [f"$uidOf:{i}" for i in applied], "kept": [], "skipped": [], "refused": list(refused),
                "fields": list(fields)}

    def case(name, op, result, expect):
        out.append({"name": name, "steps": [{"op": "execute", "input": {"operation": op}, "within": 1e-9,
                                              "result": result, "expect": expect}]})

    # A new type.
    d02 = {"id": "d02", "name": "D02", "version": 1, "form": "door", "width": 2.0, "leaves": 3, "operation": "revolving",
           "frame": 0.05, "head": 2.1, "build": {"drum": 1.8}}
    r, e = done(TYPES + [d02], report())
    case("Yeni tip: üç kanatlı döner, tambur 1,8 m",
         {"kind": "define", "name": "D02", "copyOf": "Kapı 90",
          "change": {"width": 2.0, "operation": "revolving", "leaves": 3, "build": {"drum": 1.8}}}, r, e)
    kt2 = {"id": "kt2", "name": "KT2", "version": 1, "form": "door", "width": 2.4, "leaves": 6, "operation": "folding",
           "frame": 0.04, "head": 2.1, "build": {"bothSides": True}}
    r, e = done(TYPES + [kt2], report())
    case("Yeni tip: KT1'in kopyası, altı panel iki yana",
         {"kind": "define", "name": "KT2", "copyOf": "KT1", "change": {"width": 2.4, "leaves": 6, "build": {"bothSides": True}}},
         r, e)
    case("Yeni tip: iki kanatlı döner reddedilir",
         {"kind": "define", "name": "D03", "copyOf": "D01", "change": {"leaves": 2}},
         failed("invalid_opening_type", "Döner kapının kanat sayısı 2; 3 ya da 4 olmalı.", "operation.change.leaves"),
         {"ids": IDS, "revision": "same"})
    # A type's change: its door follows.
    cg = copy.deepcopy(CG1)
    cg["version"] = 2
    cg["build"] = {"leafThickness": 0.04, "lip": 0.1}
    door = copy.deepcopy(BASE[7])
    door["build"] = dict(cg["build"])
    door["typeVersion"] = 2
    r, e = done([D01, cg, KT1], report([7], ["lip"]), entities={"7": stored(door, 7)})
    case("CG1'in payı 10 cm: kapısı izler, cebi kısalır", {"kind": "update", "type": "CG1", "change": {"build": {"lip": 0.1}}},
         r, e)
    cg = copy.deepcopy(CG1)
    cg["version"] = 2
    cg["build"] = {"leafThickness": 0.25, "lip": 0.06}
    door = copy.deepcopy(BASE[7])
    door["build"] = dict(cg["build"])
    why = on_wall(door, "")["error"]["message"]
    case("CG1'in kanadı 25 cm: cebi 30 cm'lik duvara sığmaz, hiçbir şey yazılmaz",
         {"kind": "update", "type": "CG1", "change": {"build": {"leafThickness": 0.25}}},
         failed("opening_type_refused",
                f"Tipi izleyen “$uidOf:7” açıklığı değişikliği alamıyor: {why} Hiçbir şey yazılmadı; açıklığı düzeltin, tipten ayırın ya da sığmayanları atlamayı (onRefused: skip) seçin.",
                "operation"),
         {"ids": IDS, "revision": "same"})
    d = copy.deepcopy(D01)
    d["version"] = 2
    d["leaves"] = 3
    door = copy.deepcopy(BASE[6])
    door["leaves"] = 3
    door["typeVersion"] = 2
    r, e = done([d, CG1, KT1], report([6], ["leaves"]), entities={"6": stored(door, 6)})
    case("D01 üç kanatlı olur: kapısı izler", {"kind": "update", "type": "D01", "change": {"leaves": 3}}, r, e)
    k = copy.deepcopy(KT1)
    k["version"] = 2
    k["build"] = {"bothSides": True}
    door = copy.deepcopy(BASE[8])
    door["build"] = {"bothSides": True}
    door["typeVersion"] = 2
    r, e = done([D01, CG1, k], report([8], ["bothSides"]), entities={"8": stored(door, 8)})
    case("KT1 iki yana toplanır: kapısı izler", {"kind": "update", "type": "KT1", "change": {"build": {"bothSides": True}}},
         r, e)
    case("KT1'e ray verilmez", {"kind": "update", "type": "KT1", "change": {"build": {"track": True}}},
         failed("invalid_opening_type", "Ray yalnız sürme açıklığındır; kaldırın ya da açılışı Sürme yapın.",
                "operation.change.build.track"),
         {"ids": IDS, "revision": "same"})
    same = {"ids": IDS, "revision": "same"}

    def refused(name, op, words, field):
        case(name, op, failed("invalid_opening_type", words, f"operation.change.{field}"), same)
    cg2 = {"id": "cg2", "name": "CG2", "version": 1, "form": "door", "width": 1.6, "leaves": 2, "operation": "pocket",
           "frame": 0.03, "head": 2.1, "build": {"leafThickness": 0.04, "lip": 0.06}}
    r, e = done(TYPES + [cg2], report())
    case("Yeni tip: CG1'in iki kanatlı kopyası", {"kind": "define", "name": "CG2", "copyOf": "CG1",
                                                    "change": {"width": 1.6, "leaves": 2}}, r, e)
    s01 = {"id": "s01", "name": "S01", "version": 1, "form": "door", "width": 0.9, "operation": "slide", "frame": 0.05,
           "head": 2.1}
    r, e = done(TYPES + [s01], report())
    case("Yeni tip: kapalı rayı yazılmayan sürme", {"kind": "define", "name": "S01", "copyOf": "Kapı 90",
                                                     "change": {"operation": "slide", "build": {"track": False}}}, r, e)
    refused("Yeni tip: tek panelli katlanır reddedilir",
            {"kind": "define", "name": "KT9", "copyOf": "KT1", "change": {"leaves": 1}},
            "Katlanır kapının panel sayısı 1; 2 ile 6 arasında olmalı.", "leaves")
    refused("Yeni tip: katlanır kapıya tambur verilmez",
            {"kind": "define", "name": "KT9", "copyOf": "KT1", "change": {"build": {"drum": 1.0}}},
            "Tambur çapı yalnız döner açıklığındır; kaldırın ya da açılışı Döner yapın.", "build.drum")
    refused("Yeni tip: tek kanatlı sürmeye bindirme verilmez",
            {"kind": "define", "name": "S09", "copyOf": "Kapı 90", "change": {"operation": "slide", "build": {"overlap": 0.05}}},
            "Kanat bindirmesi çok kanatlı sürmenindir; tek kanatlıda kaldırın.", "build.overlap")
    refused("Yeni tip: döner pencere olmaz",
            {"kind": "define", "name": "P09", "copyOf": "Pencere 120", "change": {"operation": "revolving", "leaves": 4}},
            "Döner açılış yalnız kapınındır; pencere için Kanatlı, Sürme ya da Sabit seçin.", "operation")
    refused("D01'in tamburu ışıktan geniş olmaz",
            {"kind": "update", "type": "D01", "change": {"build": {"drum": 2.4}}},
            "Döner kapının tamburu (2.4 m) ışıktan (2.200 m) geniş; tambur kasanın içine sığmalı. Çapı küçültün ya da açıklığı genişletin.",
            "build.drum")
    d = copy.deepcopy(D01)
    d["version"], d["build"] = 2, {"drum": 1.8}
    door = copy.deepcopy(BASE[6])
    door["build"], door["typeVersion"] = {"drum": 1.8}, 2
    r, e = done([d, CG1, KT1], report([6], ["drum"]), entities={"6": stored(door, 6)})
    case("D01'in tamburu 1,8 m: kapısı izler", {"kind": "update", "type": "D01", "change": {"build": {"drum": 1.8}}}, r, e)
    d = copy.deepcopy(D01)
    d["version"] = 2
    d.pop("build")
    door = copy.deepcopy(BASE[6])
    door["build"], door["typeVersion"] = None, 2
    r, e = done([d, CG1, KT1], report([6], ["drum"]), entities={"6": stored(door, 6)})
    case("D01'in tamburu kaldırılır: ışık kadar", {"kind": "update", "type": "D01", "change": {"unset": ["drum"]}}, r, e)
    cg = copy.deepcopy(CG1)
    cg["version"], cg["leaves"] = 2, 2
    door = copy.deepcopy(BASE[7])
    door["leaves"], door["typeVersion"] = 2, 2
    r, e = done([D01, cg, KT1], report([7], ["leaves"]), entities={"7": stored(door, 7)})
    case("CG1 iki kanatlı olur: kapısının iki cebi", {"kind": "update", "type": "CG1", "change": {"leaves": 2}}, r, e)
    door = copy.deepcopy(BASE[7])
    door["width"] = 2.0
    why = into_others(door, [x for i, x in BASE.items() if i != 7], "")["error"]["message"]
    case("CG1 iki metre olunca cebi döner kapıya girer, hiçbir şey yazılmaz",
         {"kind": "update", "type": "CG1", "change": {"width": 2.0}},
         failed("opening_type_refused",
                f"Tipi izleyen “$uidOf:7” açıklığı değişikliği alamıyor: {why} Hiçbir şey yazılmadı; açıklığı düzeltin, tipten ayırın ya da sığmayanları atlamayı (onRefused: skip) seçin.",
                "operation"), same)
    k = copy.deepcopy(KT1)
    k["version"], k["leaves"] = 2, 6
    door = copy.deepcopy(BASE[8])
    door["leaves"], door["typeVersion"] = 6, 2
    r, e = done([D01, CG1, k], report([8], ["leaves"]), entities={"8": stored(door, 8)})
    case("KT1 altı panelli olur: kapısı izler", {"kind": "update", "type": "KT1", "change": {"leaves": 6}}, r, e)
    refused("KT1 sabit olunca dört kanadı reddedilir",
            {"kind": "update", "type": "KT1", "change": {"operation": "fixed"}},
            "Açıklığın kanat sayısı 4; 1 ya da 2 olmalı.", "leaves")
    return out


# ── Three- and four-leaf sliding doors and windows (docs/adr/0792 §1) ────────────────────────────────────────────

NOTE_MULTI = ("ADR 0792. Beklenenler scripts/fixtures/special_doors_command_cases.py'nin ADR'den yazılmış kurallarından, "
              "KentOS kodu olmadan: üç ve dört kanatlı sürmenin rayları, sabit ve sürme kanatları, toplandığı yan, "
              "bindirmesi, net geçişi ve kanat düzeni. Kurulum ADR 0777'nin çizimi; yeni nesne 11'den başlar.")
PENCERE120 = {"form": "window", "width": 1.2, "leaves": 2, "frame": 0.06, "sill": 0.9, "head": 2.2}


def window(host, at, **own):
    """A window made without a type: KentOS's first window (Pencere 120), the values given over it."""
    o = opening(host, at, 1.2, form="window", leaves=2, frame=0.06, sill=0.9, head=2.2)
    o["style"] = "kentos-pencere-120"
    for k, v in own.items():
        o[k] = {x: y for x, y in v.items() if y is not False} or None if k == "build" else v
    return o


def multi_create_cases():
    out = []

    def add(name, given, made):
        why = problem(made, "openings[0]")
        step = {"op": "execute", "input": {"layerId": "kapi", "openings": [given]}, "within": 1e-9}
        if why:
            step["result"] = why
            step["expect"] = {"ids": IDS, "revision": "same"}
        else:
            step["result"] = {"status": "completed", "output": {"uids": ["$any"], "ids": [NEW], "revision": "$current"},
                              "warnings": []}
            step["expect"] = {"ids": IDS + [NEW], "entities": {str(NEW): stored(made, NEW)}, "canUndo": True,
                              "dirty": True, "revision": "changed"}
        out.append({"name": name, "steps": [step]})
    H = "$uidOf:1"

    def door(width, leaves, build=None, **own):
        given = {"host": H, "at": 5.0, "width": width, "operation": "slide", "leaves": leaves, **own}
        if build:
            given["build"] = build
        return given, from_kapi(1, 5.0, width=width, operation="slide", leaves=leaves, **own,
                                **({"build": build} if build else {}))
    add("Üç kanat iki rayda: ortadaki başa kayar (OXO)", *door(2.4, 3))
    add("Üç kanat üç rayda: teleskop başa (OXX)", *door(2.4, 3, {"tracks": 3}))
    add("Üç kanat üç rayda sona toplanır (XXO)", *door(2.4, 3, {"tracks": 3}, hand="end"))
    add("Dört kanat iki yana açılır (OXXO)", *door(3.2, 4, {"bothSides": True}))
    add("Dört kanat üç rayda tek yana (OXXO), sonuncusu sabit", *door(3.2, 4, {"tracks": 3}))
    add("Dört kanat iki rayda tek yana (OXOO)", *door(3.2, 4))
    add("Üç kanat, 6 cm bindirme, raylı", *door(2.4, 3, {"overlap": 0.06, "track": True}))
    add("Dört kanat iki yana, raylı, 5 cm bindirme", *door(3.2, 4, {"bothSides": True, "track": True, "overlap": 0.05}))
    add("Dört kanat bindirmesiz", *door(3.2, 4, {"overlap": 0.0, "tracks": 3}))
    add("Beş kanatlı sürme reddedilir", *door(3.2, 5))
    add("Ray sayısı 4 reddedilir", *door(3.2, 4, {"tracks": 4}))
    add("İki kanatlı sürmede ray sayısı reddedilir", *door(1.6, 2, {"tracks": 2}))
    add("Tek kanatlı sürmede ray sayısı reddedilir", *door(0.9, 1, {"tracks": 3}))
    add("Kanatlı kapıya ray sayısı verilmez", {"host": H, "at": 5.0, "build": {"tracks": 2}},
        from_kapi(1, 5.0, build={"tracks": 2}))
    add("Üç kanat iki yana açılmaz", *door(2.4, 3, {"bothSides": True}))
    add("İki yana açılan dört kanat üç rayda olmaz", *door(3.2, 4, {"bothSides": True, "tracks": 3}))
    add("Dört kanadın bindirmesi ışığın dörtte birini aşamaz", *door(2.0, 4, {"overlap": 0.5}))
    add("Üç kanadın bindirmesi ışığın üçte birini aşamaz", *door(2.4, 3, {"overlap": 0.8}))
    add("Kanatlı kapıya iki yana açılma verilmez", {"host": H, "at": 5.0, "build": {"bothSides": True}},
        from_kapi(1, 5.0, build={"bothSides": True}))
    add("Sabit kapıya üç kanat verilmez", {"host": H, "at": 5.0, "operation": "fixed", "leaves": 3},
        from_kapi(1, 5.0, operation="fixed", leaves=3))
    add("Üç kanatlı sürme pencere, iki ray",
        {"host": H, "at": 5.0, "form": "window", "width": 2.4, "operation": "slide", "leaves": 3},
        window(1, 5.0, width=2.4, operation="slide", leaves=3))
    add("Dört kanatlı sürme pencere iki yana, raylı",
        {"host": H, "at": 5.0, "form": "window", "width": 3.2, "operation": "slide", "leaves": 4,
         "build": {"bothSides": True, "track": True}},
        window(1, 5.0, width=3.2, operation="slide", leaves=4, build={"bothSides": True, "track": True}))
    add("Üç kanatlı pencere üç rayda, sona",
        {"host": H, "at": 5.0, "form": "window", "width": 2.4, "operation": "slide", "leaves": 3, "hand": "end",
         "build": {"tracks": 3}},
        window(1, 5.0, width=2.4, operation="slide", leaves=3, hand="end", build={"tracks": 3}))
    return out


def multi_query_cases():
    out = []

    def made(name, given, o):
        r = {"uid": f"$uidOf:{NEW}", "form": o["form"], "type": o["style"],
             "typeName": {"kentos-kapi-90": "Kapı 90", "kentos-pencere-120": "Pencere 120"}[o["style"]],
             "host": f"$uidOf:{o['host']}", "sizes": sizes(o)}
        out.append({"name": name, "steps": [
            {"op": "execute", "command": "cad.openings.create", "input": {"layerId": "kapi", "openings": [given]},
             "within": 1e-9, "result": {"status": "completed", "output": {"uids": ["$any"], "ids": [NEW], "revision": "$current"},
                                        "warnings": []}},
            {"op": "plan", "input": {"uids": [f"$uidOf:{NEW}"]}, "within": 1e-9,
             "result": {"status": "completed", "output": {"openings": [r], "types": uses(list(BASE.values()) + [o]),
                                                          "revision": "$current"}, "warnings": []}}]})
    H = "$uidOf:1"

    def door(name, width, leaves, build=None, **own):
        given = {"host": H, "at": 5.0, "width": width, "operation": "slide", "leaves": leaves, **own}
        if build:
            given["build"] = build
        made(name, given, from_kapi(1, 5.0, width=width, operation="slide", leaves=leaves, **own,
                                    **({"build": build} if build else {})))
    door("OXO: ortadaki kanat açılır, net geçiş bir kanat eksi iki bindirme", 2.4, 3)
    door("OXX: teleskop, net geçiş ışık eksi bir kanat", 2.4, 3, {"tracks": 3})
    door("XXO: sona toplanır, aynı net geçiş", 2.4, 3, {"tracks": 3}, hand="end")
    door("OXO sona: düzen aynı", 2.4, 3, hand="end")
    door("OXXO iki yana: ışık eksi iki kanat", 3.2, 4, {"bothSides": True})
    door("OXXO tek yana üç rayda: sonuncu sabit", 3.2, 4, {"tracks": 3})
    door("OXXO tek yana sona", 3.2, 4, {"tracks": 3}, hand="end")
    door("OXOO: iki rayda dört kanat", 3.2, 4)
    door("OOXO: sona iki rayda", 3.2, 4, hand="end")
    door("Üç kanat, 8 cm bindirme", 2.4, 3, {"overlap": 0.08})
    door("Dört kanat iki yana, bindirmesiz", 3.2, 4, {"bothSides": True, "overlap": 0.0})
    door("Üç kanat, montaj boşluğu 1 cm, üç ray", 2.4, 3, {"fit": 0.01, "tracks": 3})
    door("Dört kanat, kanat eni ölçü referansı", 3.2, 4, {"sizeBy": "leaf", "bothSides": True})
    door("Üç kanat dar açıklıkta", 1.2, 3)
    door("Dört kanat geniş açıklıkta, üç ray", 4.8, 4, {"tracks": 3, "overlap": 0.1})
    door("Üç kanat, kalın kanat net geçişi değiştirmez", 2.4, 3, {"leafThickness": 0.06, "tracks": 3})
    door("Raylı üç kanat aynı ölçülerle", 2.4, 3, {"track": True, "tracks": 2})
    made("Üç kanatlı pencere: net geçişi yok, düzeni var",
         {"host": H, "at": 5.0, "form": "window", "width": 2.4, "operation": "slide", "leaves": 3},
         window(1, 5.0, width=2.4, operation="slide", leaves=3))
    made("Dört kanatlı pencere iki yana",
         {"host": H, "at": 5.0, "form": "window", "width": 3.2, "operation": "slide", "leaves": 4,
          "build": {"bothSides": True}},
         window(1, 5.0, width=3.2, operation="slide", leaves=4, build={"bothSides": True}))
    made("Üç kanatlı pencere üç rayda",
         {"host": H, "at": 5.0, "form": "window", "width": 2.4, "operation": "slide", "leaves": 3,
          "build": {"tracks": 3}},
         window(1, 5.0, width=2.4, operation="slide", leaves=3, build={"tracks": 3}))
    return out


def multi_edit_cases():
    out = []

    def case(name, id, op, change=None, unset=()):
        o = copy.deepcopy(BASE[id])
        for k, v in (change or {}).items():
            if k == "build":
                b = dict(o.get("build") or {})
                b.update(v)
                o["build"] = b or None
            else:
                o[k] = v
        b = dict(o.get("build") or {})
        for k in unset:
            b.pop(k, None)
        o["build"] = b or None
        operation = {"kind": "set", **op}
        if unset:
            operation["unset"] = list(unset)
        others = [x for i, x in BASE.items() if i != id]
        why = problem(o, "uids[0]") or on_wall(o, "uids[0]") or into_others(o, others, "uids[0]")
        step = {"op": "execute", "input": {"uids": [f"$uidOf:{id}"], "operation": operation}, "within": 1e-9}
        if why:
            step["result"] = why
            step["expect"] = {"ids": IDS, "revision": "same"}
        else:
            step["result"] = {"status": "completed", "output": {"openings": [stored(o, id)], "revision": "$current"},
                              "warnings": []}
            step["expect"] = {"ids": IDS, "entities": {str(id): stored(o, id)}, "revision": "changed"}
        out.append({"name": name, "steps": [step]})
    same = lambda c: (c, c)  # noqa: E731
    case("S1 üç kanatlı olur, bindirmesi kalır", 9, *same({"leaves": 3}))
    case("S1 dört kanatlı, iki yana", 9, *same({"leaves": 4, "build": {"bothSides": True}}))
    case("S1 üç kanat üç rayda", 9, *same({"leaves": 3, "build": {"tracks": 3}}))
    case("S1 dört kanat üç rayda, sona", 9, *same({"leaves": 4, "hand": "end", "build": {"tracks": 3}}))
    case("S1 iki kanatken ray sayısı reddedilir", 9, *same({"build": {"tracks": 2}}))
    case("S1 beş kanatlı olmaz", 9, *same({"leaves": 5}))
    case("S1 dört kanatta 30 cm bindirme ışığın dörtte birini aşar", 9, *same({"leaves": 4, "build": {"overlap": 0.3}}))
    case("S1 üç kanatta iki yana açılmaz", 9, *same({"leaves": 3, "build": {"bothSides": True}}))
    case("S1 dört kanat iki yana üç rayda olmaz", 9, *same({"leaves": 4, "build": {"bothSides": True, "tracks": 3}}))
    case("S1 üç kanat, bindirmesi ve rayı kaldırılır", 9, {"leaves": 3}, {"leaves": 3}, unset=("overlap", "track"))
    case("S1 üç kanatlı pencere olur", 9, *same({"form": "window", "leaves": 3}))
    case("S1 dört kanatlı sağa açılan pencere", 9, *same({"form": "window", "leaves": 4, "side": "right"}))
    case("S1 dört kanat, 4 cm bindirme", 9, *same({"leaves": 4, "build": {"overlap": 0.04}}))
    case("S1 katlanır olunca ray sayısı reddedilir", 9,
         *same({"operation": "folding", "leaves": 4, "build": {"tracks": 2}}))
    case("Kapı üç kanatlı sürme olur, 2,4 m", 10, *same({"operation": "slide", "leaves": 3, "width": 2.4}))
    case("Kapı dört kanatlı sürme iki yana, 2,8 m", 10,
         *same({"operation": "slide", "leaves": 4, "width": 2.8, "build": {"bothSides": True}}))
    case("Kapı üç rayda teleskop, raylı", 10,
         *same({"operation": "slide", "leaves": 3, "width": 2.4, "build": {"tracks": 3, "track": True}}))
    case("Kanatlı kapıya üç kanat verilmez", 10, *same({"leaves": 3}))
    case("Kanatlı kapıya ray sayısı verilmez", 10, *same({"build": {"tracks": 3}}))
    case("D01 sürme olunca tamburu reddedilir", 6, *same({"operation": "slide"}))
    case("S1 iki kanatta iki yana açılmaz", 9, *same({"build": {"bothSides": True}}))
    case("Sabit kapı üç kanatlı olmaz", 10, *same({"operation": "fixed", "leaves": 3}))
    return out


def multi_types_cases():
    out = []
    same = {"ids": IDS, "revision": "same"}

    def done(types, report, entities=None):
        r = {"status": "completed", "output": {"types": [type_json(t) for t in types], "report": report,
                                                "revision": "$current"}, "warnings": []}
        e = {"ids": IDS, "revision": "changed"}
        if entities:
            e["entities"] = entities
        return r, e

    def report(applied=(), fields=()):
        return {"applied": [f"$uidOf:{i}" for i in applied], "kept": [], "skipped": [], "refused": [],
                "fields": list(fields)}

    def case(name, op, result, expect):
        out.append({"name": name, "steps": [{"op": "execute", "input": {"operation": op}, "within": 1e-9,
                                              "result": result, "expect": expect}]})

    def refused(name, op, why):
        case(name, op, failed("invalid_opening_type", why["error"]["message"],
                              why["error"]["path"].replace("x.", "operation.change.", 1)), same)

    def define(name, label, copy_of, base, change):
        t = {"id": label.lower(), "name": label, "version": 1, **copy.deepcopy(base)}
        for k, v in change.items():
            if k == "build":
                t["build"] = {**(t.get("build") or {}), **v}
            else:
                t[k] = v
        o = {**t, "host": 1, "at": 5.0, "hand": "start"}
        o.setdefault("leaves", 1)
        o.setdefault("operation", "swing")
        why = problem(o, "x")
        op = {"kind": "define", "name": label, "copyOf": copy_of, "change": change}
        if why:
            refused(name, op, why)
        else:
            case(name, op, *done(TYPES + [t], report()))
    K90 = {"form": "door", "width": 0.9, "frame": 0.05, "head": 2.1}
    define("Yeni tip: üç kanatlı sürme, iki ray", "S3", "Kapı 90", K90,
           {"width": 2.4, "operation": "slide", "leaves": 3})
    define("Yeni tip: üç kanat üç rayda", "S3T", "Kapı 90", K90,
           {"width": 2.4, "operation": "slide", "leaves": 3, "build": {"tracks": 3}})
    define("Yeni tip: dört kanat iki yana", "S4", "Kapı 90", K90,
           {"width": 3.2, "operation": "slide", "leaves": 4, "build": {"bothSides": True}})
    define("Yeni tip: dört kanat iki yana, raylı, bindirmeli", "S4R", "Kapı 90", K90,
           {"width": 3.2, "operation": "slide", "leaves": 4, "build": {"bothSides": True, "track": True, "overlap": 0.06}})
    define("Yeni tip: dört kanat üç rayda tek yana", "S4T", "Kapı 90", K90,
           {"width": 3.2, "operation": "slide", "leaves": 4, "build": {"tracks": 3}})
    define("Yeni tip: üç kanatlı sürme pencere", "W3", "Pencere 120", PENCERE120,
           {"width": 2.4, "operation": "slide", "leaves": 3})
    define("Yeni tip: beş kanatlı sürme reddedilir", "S5", "Kapı 90", K90,
           {"width": 3.2, "operation": "slide", "leaves": 5})
    define("Yeni tip: iki kanatta ray sayısı reddedilir", "S2T", "Kapı 90", K90,
           {"width": 1.6, "operation": "slide", "leaves": 2, "build": {"tracks": 2}})
    define("Yeni tip: tek ray reddedilir", "S3R", "Kapı 90", K90,
           {"width": 2.4, "operation": "slide", "leaves": 3, "build": {"tracks": 1}})
    define("Yeni tip: iki yana dört kanat üç rayda olmaz", "S4X", "Kapı 90", K90,
           {"width": 3.2, "operation": "slide", "leaves": 4, "build": {"bothSides": True, "tracks": 3}})
    define("Yeni tip: üç kanat iki yana açılmaz", "S3B", "Kapı 90", K90,
           {"width": 2.4, "operation": "slide", "leaves": 3, "build": {"bothSides": True}})
    define("Yeni tip: katlanır kapıya ray sayısı verilmez", "KT9", "KT1",
           {k: v for k, v in KT1.items() if k not in ("id", "name", "version")}, {"build": {"tracks": 2}})
    define("Yeni tip: üç kanadın bindirmesi ışığın üçte birinden büyük", "S3L", "Kapı 90", K90,
           {"width": 2.4, "operation": "slide", "leaves": 3, "build": {"overlap": 0.8}})

    def follows(name, change, fields):
        k = copy.deepcopy(KT1)
        k["version"] = 2
        door = copy.deepcopy(BASE[8])
        door["typeVersion"] = 2
        for key, v in change.items():
            if key == "build":
                k["build"] = {**(k.get("build") or {}), **v}
                door["build"] = {**(door.get("build") or {}), **v}
            else:
                k[key] = v
                door[key] = v
        why = problem(door, "x")
        op = {"kind": "update", "type": "KT1", "change": change}
        if why:
            refused(name, op, why)
        else:
            case(name, op, *done([D01, CG1, k], report([8], fields), entities={"8": stored(door, 8)}))
    follows("KT1 dört kanatlı sürme olur: kapısı izler", {"operation": "slide"}, ["operation"])
    follows("KT1 iki yana açılan sürme olur", {"operation": "slide", "build": {"bothSides": True}},
            ["operation", "bothSides"])
    follows("KT1 sürme, üç rayda", {"operation": "slide", "build": {"tracks": 3}}, ["operation", "tracks"])
    follows("KT1 sürme, üç kanat üç rayda", {"operation": "slide", "leaves": 3, "build": {"tracks": 3}},
            ["leaves", "operation", "tracks"])
    follows("KT1 sürme, 10 cm bindirme", {"operation": "slide", "build": {"overlap": 0.1}}, ["operation", "overlap"])
    follows("KT1 sürme, iki yana ve üç ray reddedilir",
            {"operation": "slide", "build": {"bothSides": True, "tracks": 3}}, [])
    follows("KT1 sürme, beş kanat reddedilir", {"operation": "slide", "leaves": 5}, [])
    refused("D01 sürme olunca tamburu reddedilir", {"kind": "update", "type": "D01", "change": {"operation": "slide"}},
            problem({**BASE[6], "operation": "slide"}, "x"))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    files = {
        "cad.openings.create.special.json": doc(
            "cad.openings.create", "Açıklık ekle: döner, cebe giren, katlanır kapı, sürmenin bindirmesi ve rayı; retler",
            NOTE, create_cases()),
        "cad.openings.query.special.json": doc(
            "cad.openings.query", "Açıklıkları oku: döner, cebe giren ve katlanır kapının genişlikleri, tamburu ve cebi",
            NOTE, query_cases()),
        "cad.openings.edit.special.json": doc(
            "cad.openings.edit", "Açıklığı değiştir: açılışı döner, cebe giren, katlanır; sürmenin bindirmesi ve rayı; retler",
            NOTE, edit_cases()),
        "cad.openingtypes.edit.special.json": doc(
            "cad.openingtypes.edit", "Kapı tipleri: döner, cebe giren ve katlanır tipler, izleyen kapılar, cebe sığmayan",
            NOTE, types_cases()),
        "cad.openings.create.multislide.json": doc(
            "cad.openings.create", "Açıklık ekle: üç ve dört kanatlı sürme, raylar, iki yana açılma, bindirme; retler",
            NOTE_MULTI, multi_create_cases()),
        "cad.openings.query.multislide.json": doc(
            "cad.openings.query", "Açıklıkları oku: üç ve dört kanatlı sürmenin kanatları, net geçişi, rayları ve düzeni",
            NOTE_MULTI, multi_query_cases()),
        "cad.openings.edit.multislide.json": doc(
            "cad.openings.edit", "Açıklığı değiştir: sürme üç ve dört kanatlı olur, raylar, iki yana; retler",
            NOTE_MULTI, multi_edit_cases()),
        "cad.openingtypes.edit.multislide.json": doc(
            "cad.openingtypes.edit", "Kapı tipleri: üç ve dört kanatlı sürme tipleri, izleyen kapı, retler",
            NOTE_MULTI, multi_types_cases()),
    }
    bad = 0
    for name, d in files.items():
        text = json.dumps(d, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın", file=sys.stderr)
                bad += 1
        else:
            path.write_text(text)
            print(f"{path.relative_to(ROOT)}: {len(d['cases'])} durum")
    if args.check and not bad:
        print("özel kapıların komut durumları güncel")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
