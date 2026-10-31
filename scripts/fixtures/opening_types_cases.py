#!/usr/bin/env python3
"""Door and window types, their openings and the doors' audit (docs/adr/0772), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.openingtypes.edit.json, cad.openings.query.json, cad.openings.edit.types.json and
cad.arch.audit.doors.json; `--check` compares and writes nothing. The walls are straight and run along x, so an
opening's jambs are `(x0 + s, ±w/2)`. The rules, as the ADR gives them:

- a type is a form, a rough opening (`width`), leaves, an operation, a swing, its arc shown, a frame, a sill, a head, a
  depth and a construction (`build`), with an id, a name, a version and a source; an opening following it has its id as
  `style` and the version it took as `typeVersion`, and keeps its own value of the fields in `overrides`;
- an opening's widths: rough = width; unit = rough − 2·fit; leaf = unit − 2·frame; a door's leaves split the leaf as
  `split` says (two sliding leaves (leaf + 5 %)/2 each); its net clear width open at θ (none: 90°) is the leaf less each
  open leaf's `max(l·cos θ, 0) + t·max(sin θ, 0)` (t the leaf's thickness, none: 0.04 m), a sliding door the leaf less one
  leaf (one leaf: all of it), a fixed door 0, a bare opening its leaf; none for a window;
- a type's change: the fields given set, a nominal size giving the rough opening (rough = value + 2·fit [+ 2·frame]); a
  type sized by its leaf or its unit keeps that size when its frame or fit change (its rough opening follows); the version
  goes up by one; each opening following it takes every field it does not keep, those on a locked layer skipped, those it
  does not fit refused (all or none, or with `skip` left at the older version), checked one at a time along each wall.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

FIELDS = ["width", "leaves", "operation", "swing", "showSwing", "frame", "sill", "head", "depth", "sizeBy", "fit",
          "leafThickness", "split", "frameDepth", "frameOffset", "threshold", "casing", "sillBoard", "mullions", "transoms",
          "useZone"]
BUILD = FIELDS[9:]
LABELS = {"width": "Kaba boşluk", "leaves": "Kanat", "operation": "Açılış", "swing": "Açılma açısı",
          "showSwing": "Açılım yayı", "frame": "Kasa", "sill": "Denizlik yüksekliği", "head": "Lento", "depth": "Derinlik",
          "sizeBy": "Ölçü referansı", "fit": "Montaj boşluğu", "leafThickness": "Kanat kalınlığı", "split": "Kanat payı",
          "frameDepth": "Kasa derinliği", "frameOffset": "Kasanın yüzden uzaklığı", "threshold": "Eşik",
          "casing": "Pervaz", "sillBoard": "Denizlik çıkıntısı", "mullions": "Dikey kayıt", "transoms": "Yatay kayıt",
          "useZone": "Kullanım alanı"}
BUILTIN = [
    {"id": "kentos-kapi-90", "name": "Kapı 90", "form": "door", "width": 0.9, "leaves": 1, "operation": "swing", "frame": 0.05, "head": 2.1},
    {"id": "kentos-kapi-80", "name": "Kapı 80", "form": "door", "width": 0.8, "leaves": 1, "operation": "swing", "frame": 0.05, "head": 2.1},
    {"id": "kentos-iki-kanatli-kapi-140", "name": "İki kanatlı kapı 140", "form": "door", "width": 1.4, "leaves": 2, "operation": "swing", "frame": 0.05, "head": 2.1},
    {"id": "kentos-surme-kapi-160", "name": "Sürme kapı 160", "form": "door", "width": 1.6, "leaves": 2, "operation": "slide", "frame": 0.05, "head": 2.1},
    {"id": "kentos-pencere-120", "name": "Pencere 120", "form": "window", "width": 1.2, "leaves": 2, "operation": "swing", "frame": 0.06, "sill": 0.9, "head": 2.2},
    {"id": "kentos-sabit-pencere-100", "name": "Sabit pencere 100", "form": "window", "width": 1.0, "leaves": 1, "operation": "fixed", "frame": 0.06, "sill": 0.9, "head": 2.2},
    {"id": "kentos-bosluk-100", "name": "Boşluk 100", "form": "opening", "width": 1.0, "leaves": 1, "operation": "fixed", "head": 2.1},
    {"id": "kentos-tesisat-20", "name": "Tesisat boşluğu 20", "form": "opening", "width": 0.2, "leaves": 1, "operation": "fixed", "sill": 2.3, "head": 2.5},
    {"id": "kentos-nis-60", "name": "Niş 60", "form": "niche", "width": 0.6, "leaves": 1, "operation": "fixed", "sill": 0.9, "head": 1.5, "depth": 0.1},
    {"id": "kentos-kalorifer-nisi-100", "name": "Kalorifer nişi 100", "form": "niche", "width": 1.0, "leaves": 1, "operation": "fixed", "sill": 0.1, "head": 0.8, "depth": 0.1},
    {"id": "kentos-baca-20", "name": "Baca 20", "form": "flue", "width": 0.2, "leaves": 1, "operation": "fixed", "depth": 0.2},
]
FORM_LABEL = {"door": "Kapı", "window": "Pencere", "opening": "Boşluk", "niche": "Niş", "flue": "Baca"}


def num(x):
    if isinstance(x, float) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def done(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def wall(id, x0, y, x1, level=None, layer_id="duvar"):
    w = {"kind": "wall", "id": id, "layerId": layer_id, "attrs": {}, "pts": [P(x0, y), P(x1, y)], "width": 0.2,
         "justify": "center"}
    if level:
        w["arch"] = {"level": level}
    return w


# ── Types ────────────────────────────────────────────────────────────────────────────────────────────────────────


def type_json(t):
    """A type as the project keeps it: defaults left out."""
    out = {"id": t["id"], "name": t["name"], "version": t["version"]}
    if t.get("source") is not None:
        out["source"] = t["source"]
    out["form"] = t["form"]
    out["width"] = float(t["width"])
    if t.get("leaves", 1) != 1:
        out["leaves"] = t["leaves"]
    if t.get("operation", "swing") != "swing":
        out["operation"] = t["operation"]
    if t.get("swing") is not None:
        out["swing"] = float(t["swing"])
    if t.get("showSwing", True) is False:
        out["showSwing"] = False
    for k in ("frame", "sill", "head", "depth"):
        if t.get(k) is not None:
            out[k] = float(t[k])
    b = build_json(t.get("build"))
    if b:
        out["build"] = b
    return out


def build_json(b):
    if not b:
        return None
    out = {}
    for k in BUILD:
        if b.get(k) is not None:
            out[k] = b[k] if k in ("sizeBy", "mullions", "transoms") else float(b[k])
    return out or None


def value(o, k):
    """An opening's or a type's value of a field a type governs (None: none)."""
    if k in BUILD:
        return (o.get("build") or {}).get(k)
    if k == "leaves":
        return o.get("leaves", 1)
    if k == "operation":
        return o.get("operation", "swing")
    if k == "showSwing":
        return o.get("showSwing", True)
    return o.get(k)


def set_value(o, k, v):
    if k in BUILD:
        b = dict(o.get("build") or {})
        if v is None:
            b.pop(k, None)
        else:
            b[k] = v
        o["build"] = b or None
    else:
        o[k] = v


def rough_of(by, v, fit, frame):
    return {"rough": v, "unit": v + 2.0 * fit, "leaf": v + 2.0 * fit + 2.0 * frame}[by]


def sizes(f):
    b = f.get("build") or {}
    fit = b.get("fit") or 0.0
    frame = f.get("frame") or 0.0
    rough = f["width"]
    unit = rough - 2.0 * fit
    leaf = unit - 2.0 * frame
    t = b.get("leafThickness") if b.get("leafThickness") is not None else 0.04
    angle = f.get("swing") if f.get("swing") is not None else 90.0
    share = b.get("split") if b.get("split") is not None else 0.5
    op, n = f.get("operation", "swing"), f.get("leaves", 1)
    if f["form"] not in ("door", "window"):
        leaves = []
    elif op == "slide" and n == 2:
        w = leaf * (1.0 + 0.05) / 2.0
        leaves = [w, w]
    elif n == 2:
        leaves = [leaf * share, leaf * (1.0 - share)]
    else:
        leaves = [leaf]
    r = math.radians(angle)

    def takes(length):
        return max(length * math.cos(r), 0.0) + t * max(math.sin(r), 0.0)
    clear = first = None
    if f["form"] == "door":
        if op == "fixed":
            clear = 0.0
        elif op == "slide":
            clear = leaf if len(leaves) == 1 else leaf - leaves[0]
        elif len(leaves) == 1:
            clear = leaf - takes(leaves[0])
        else:
            clear = leaf - takes(leaves[0]) - takes(leaves[1])
            first = leaves[0] - takes(leaves[0])
    elif f["form"] == "opening":
        clear = leaf
    by = b.get("sizeBy") or "rough"
    out = {"rough": rough, "unit": unit, "leaf": leaf, "leaves": leaves}
    if clear is not None:
        out["clear"] = max(clear, 0.0)
    if first is not None:
        out["clearFirst"] = max(first, 0.0)
    out["sizeBy"] = by
    out["nominal"] = {"rough": rough, "unit": unit, "leaf": leaf}[by]
    return out


def type_problem(t):
    """A type's problem as the ADR orders them: (field, words) or None."""
    w = t["width"]
    if not (math.isfinite(w) and 0 < w <= 100):
        return "width", f"Açıklığın genişliği {num(w)}; sıfırdan büyük, en çok 100 m olmalı."
    fr = t.get("frame")
    if fr is not None and not (fr >= 0 and 2 * fr < w):
        return "frame", f"Kasa genişliği {num(fr)} m; sıfır ya da daha büyük, açıklığın yarısından ({num(w / 2)} m) küçük olmalı."
    if t.get("sill") is not None and t.get("head") is not None and t["head"] <= t["sill"]:
        return "head", f"Lento ({num(t['head'])} m) denizlikten ({num(t['sill'])} m) yüksek olmalı."
    b = t.get("build") or {}
    for k, what, zero in (("fit", "Montaj boşluğu", True), ("leafThickness", "Kanat kalınlığı", False),
                          ("frameDepth", "Kasa derinliği", False), ("frameOffset", "Kasanın yüzden uzaklığı", True),
                          ("threshold", "Eşik derinliği", False), ("casing", "Pervaz genişliği", False),
                          ("sillBoard", "Denizlik çıkıntısı", False), ("useZone", "Kullanım alanı derinliği", False)):
        x = b.get(k)
        if x is None:
            continue
        if not (math.isfinite(x) and (x >= 0 if zero else x > 0) and x <= 10):
            lead = "sıfır ya da daha büyük," if zero else "sıfırdan büyük,"
            return f"build.{k}", f"{what} {num(x)} m; {lead} en çok 10 m olmalı."
    s = b.get("split")
    if s is not None and not (0 < s < 1):
        return "build.split", f"Kanatların payı {num(s)}; 0 ile 1 arasında olmalı (ör. 0,6: birinci kanat ışığın %60'ı)."
    for k, what in (("mullions", "Dikey kayıt"), ("transoms", "Yatay kayıt")):
        if b.get(k) is not None and b[k] > 20:
            return f"build.{k}", f"{what} sayısı {b[k]}; en çok 20 olmalı."
    if b.get("fit") is not None:
        fit, frame = b["fit"], (t.get("frame") or 0.0)
        if not (w - 2 * fit - 2 * frame > 0):
            return "build.fit", (f"Montaj boşluğu ({num(fit)} m) ve kasa ({num(frame)} m) iki yandan kaba boşluğu ({num(w)} m) "
                                 "dolduruyor; ışık kalmıyor.")
    return None


K01 = {"id": "k01", "name": "K01", "version": 1, "form": "door", "width": 1.02, "frame": 0.05, "head": 2.1,
       "build": {"sizeBy": "leaf", "fit": 0.01, "leafThickness": 0.04}}
K02 = {"id": "k02", "name": "K02", "version": 1, "form": "door", "width": 0.9, "frame": 0.05, "head": 2.1}
P01 = {"id": "p01", "name": "P01", "version": 1, "source": "Ofis 2026", "form": "window", "width": 1.2, "leaves": 2,
       "frame": 0.06, "sill": 0.9, "head": 2.2, "build": {"mullions": 1, "sillBoard": 0.05}}
TYPES = [K01, K02, P01]
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0}, {"id": "kat1", "name": "1. kat", "elevation": 3.0}]

# ── The drawing: three walls, twenty K01 doors along the first, K02 doors and a P01 window on the second ──────────

WALLS = {1: (0.0, 0.0, 60.0, "zemin"), 2: (0.0, 10.0, 10.0, "kat1"), 3: (0.0, 20.0, 12.0, None)}


def place(host, at, width):
    x0, y, _, _ = WALLS[host]
    s0, s1 = at - width / 2, at + width / 2
    return [P(x0 + s0, y + 0.1), P(x0 + s0, y - 0.1), P(x0 + s1, y + 0.1), P(x0 + s1, y - 0.1)]


def from_type(t, host, at, number=None, **own):
    """An opening made with the project type `t`, its own values over the type's."""
    f = {"host": host, "at": at, "form": t["form"], "leaves": t.get("leaves", 1), "operation": t.get("operation", "swing"),
         "showSwing": t.get("showSwing", True), "width": t["width"]}
    for k in ("swing", "frame", "sill", "head", "depth"):
        f[k] = t.get(k)
    f["build"] = copy.deepcopy(t.get("build"))
    f["style"], f["typeVersion"], f["number"] = t["id"], t["version"], number
    overrides = []
    for k, v in own.items():
        set_value(f, k, v)
        overrides.append(k)
    f["overrides"] = [k for k in FIELDS if k in overrides]
    return f


def from_builtin(key, host, at, number=None, **extra):
    t = next(t for t in BUILTIN if t["id"] == key)
    f = {"host": host, "at": at, "form": t["form"], "leaves": t["leaves"], "operation": t["operation"], "width": t["width"]}
    for k in ("frame", "sill", "head", "depth"):
        f[k] = t.get(k)
    f["style"], f["number"] = t["id"], number
    f.update(extra)
    return f


def stored(f, layer_id, id):
    """An opening as the document keeps it: defaults left out."""
    out = {"kind": "opening", "id": id, "layerId": layer_id, "attrs": {}, "host": f"$uidOf:{f['host']}", "at": float(f["at"]),
           "width": float(f["width"]), "form": f["form"]}
    if f.get("leaves", 1) != 1:
        out["leaves"] = f["leaves"]
    if f.get("operation", "swing") != "swing":
        out["operation"] = f["operation"]
    if f.get("hand", "start") != "start":
        out["hand"] = f["hand"]
    if f.get("side", "left") != "left":
        out["side"] = f["side"]
    if f.get("swing") is not None:
        out["swing"] = float(f["swing"])
    if f.get("showSwing", True) is False:
        out["showSwing"] = False
    for k in ("frame", "sill", "head", "depth"):
        if f.get(k) is not None:
            out[k] = float(f[k])
    for k in ("style", "number"):
        if f.get(k) is not None:
            out[k] = f[k]
    if f.get("typeVersion") is not None:
        out["typeVersion"] = f["typeVersion"]
    if f.get("overrides"):
        out["overrides"] = f["overrides"]
    b = build_json(f.get("build"))
    if b:
        out["build"] = b
    out["place"] = place(f["host"], f["at"], f["width"])
    return out


class Drawing:
    """The openings by slot, with their layers."""

    def __init__(self):
        self.openings = {}
        self.layers = {}
        self.types = copy.deepcopy(TYPES)

    def add(self, id, f, layer_id="kapi"):
        self.openings[id], self.layers[id] = f, layer_id

    def entity(self, id):
        return stored(self.openings[id], self.layers[id], id)

    def settings_arch(self):
        return {"levels": LEVELS, "openingTypes": [type_json(t) for t in self.types]}


def base_drawing():
    d = Drawing()
    for k in range(20):
        own = {"frame": 0.07} if k == 5 else {}
        d.add(5 + k, from_type(K01, 1, 1.5 + 2.5 * k, number=f"K{k + 1}", **own))
    d.add(25, from_type(K02, 2, 2.0, number="K21"))
    d.add(26, from_type(K02, 2, 4.0, number="K22"))
    d.add(27, from_type(K02, 2, 9.4, number="K23"))
    d.add(28, from_type(P01, 2, 6.5, number="P1"))
    # On the third wall, on no level: KentOS types copied, some built (docs/adr/0772 §3).
    d.add(29, from_builtin("kentos-kapi-90", 3, 1.0, number="K24"))
    d.add(30, from_builtin("kentos-iki-kanatli-kapi-140", 3, 3.0, number="K25", build={"split": 0.6, "leafThickness": 0.05}))
    d.add(31, from_builtin("kentos-surme-kapi-160", 3, 5.5, number="K26"))
    d.add(32, from_builtin("kentos-kapi-80", 3, 7.5, number="K27", swing=60.0))
    d.add(33, from_builtin("kentos-bosluk-100", 3, 9.5, number="B1"))
    d.add(34, from_builtin("kentos-sabit-pencere-100", 3, 11.0, number="P2"))
    return d


BASE = base_drawing()
NEW = 35


def filler():
    """Slot 4, kept free by the ids above, holds a short line on a hidden layer: a file's ids run 1, 2, 3 … with no
    gap (the independent KCAD v2 writer gives each object its place as its slot)."""
    return {"kind": "line", "id": 4, "layerId": "bos", "attrs": {}, "a": P(-1.0, -1.0), "b": P(-0.9, -1.0)}


def hidden_layer():
    out = layer("bos", "Boş")
    out["visible"] = False
    return out


def UID(n):
    return f"0192f5a1-7720-7000-8000-{n:012x}"


def real_hosts(e):
    """The setup's file names its objects by their persistent ids (docs/adr/0576 §3)."""
    e = copy.deepcopy(e)
    if isinstance(e.get("host"), str) and e["host"].startswith("$uidOf:"):
        e["host"] = UID(int(e["host"][len("$uidOf:"):]))
    return e


def setup():
    ents = [wall(i, x0, y, x1, level) for i, (x0, y, x1, level) in WALLS.items()]
    ents.append(filler())
    for id in sorted(BASE.openings):
        ents.append(real_hosts(BASE.entity(id)))
    return {
        "format": "kentos.document", "version": 2, "name": "Kapı ve pencere tipleri",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad", "arch": BASE.settings_arch()},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("kapi2", "Kapı 2"), hidden_layer()],
        "activeLayer": "kapi",
        "styles": {"items": [], "categories": []},
        "entities": ents,
    }


IDS = [1, 2, 3, 4] + sorted(BASE.openings)


def host_uid_text(f):
    return f"$uidOf:{f['host']}"


# ── The setup's hosts are given as slots; the document writes ids. Fix them as the runner reads `$uidOf:N`. ────────


def setup_doc():
    s = setup()
    return s


# ── A type's change and its openings ────────────────────────────────────────────────────────────────────────────


def changed(t, c, path):
    """`t` with the change `c`: (type, None) or (None, refusal)."""
    for k, what in (("width", "Kaba boşluk"), ("swing", "Açılma açısı"), ("frame", "Kasa"), ("sill", "Denizlik"),
                    ("head", "Lento"), ("depth", "Derinlik")):
        pass
    if c.get("width") is not None and c.get("nominal") is not None:
        return None, refusal("invalid_opening_type", "Kaba boşluk (width) ile ölçü (nominal) birlikte verilmez; birini verin.",
                             f"{path}.nominal")
    n = copy.deepcopy(t)
    for k in ("width", "leaves", "operation", "swing", "showSwing", "frame", "sill", "head", "depth"):
        if k in c:
            n[k] = c[k]
    for k, v in (c.get("build") or {}).items():
        set_value(n, k, v)
    for i, k in enumerate(c.get("unset") or []):
        if k not in BUILD:
            return None, refusal("invalid_opening_type",
                                 f"“{LABELS[k]}” yapının alanı değil; yalnız yapının alanları kaldırılır.", f"{path}.unset[{i}]")
        set_value(n, k, None)
    if c.get("nominal") is not None:
        by, v = c["nominal"]["by"], c["nominal"]["value"]
        set_value(n, "sizeBy", by)
        n["width"] = rough_of(by, v, (n.get("build") or {}).get("fit") or 0.0, n.get("frame") or 0.0)
    if c.get("width") is None and c.get("nominal") is None:
        keep_nominal(t, n)
    p = type_problem(n)
    if p:
        return None, refusal("invalid_opening_type", p[1], f"{path}.{p[0]}")
    return n, None


def keep_nominal(old, n):
    by = (n.get("build") or {}).get("sizeBy") or "rough"
    if by == "rough" or n["width"] != old["width"]:
        return
    nominal = sizes(old)["nominal"]
    n["width"] = rough_of(by, nominal, (n.get("build") or {}).get("fit") or 0.0, n.get("frame") or 0.0)


def differs(a, b):
    return [k for k in FIELDS if value(a, k) != value(b, k)]


def take_type(f, t):
    out = copy.deepcopy(f)
    out["form"] = t["form"]
    for k in FIELDS:
        if k not in out.get("overrides", []):
            set_value(out, k, copy.deepcopy(value(t, k)))
    out["style"], out["typeVersion"] = t["id"], t["version"]
    return out


def span(f):
    return f["at"] - f["width"] / 2, f["at"] + f["width"] / 2


def length(host):
    x0, _, x1, _ = WALLS[host]
    return x1 - x0


def off_wall_words(f):
    s0, s1 = span(f)
    return (f"Açıklık duvarın ucundan ya da köşesinden taşıyor: duvar boyunca {s0:.3f}–{s1:.3f} m'de, duvarın "
            f"uzunluğu {length(f['host']):.3f} m; açıklık duvarın tek bir kenarında, uçlarının arasında kalmalı. "
            "Açıklığı kaydırın ya da daraltın.")


def overlap(a, b):
    tol = 1e-9 * max(abs(a[1]), abs(b[1]), 1.0)
    return a[0] < b[1] - tol and b[0] < a[1] - tol


def heights(f):
    return (f["sill"] if f.get("sill") is not None else 0.0, f["head"] if f.get("head") is not None else math.inf)


def clashes(a, b):
    (a0, a1), (b0, b1) = heights(a), heights(b)
    cut = lambda f: f["form"] in ("door", "window")  # noqa: E731 (doors and windows only here)
    return (a0 < b1 - 1e-9 and b0 < a1 - 1e-9) or (cut(a) and cut(b))


def first_clash(d, changed):
    """The first opening of `changed` (id → fields, in slot order) running into another on its wall, with the words."""
    by_host = {}
    for id, f in changed.items():
        by_host.setdefault(f["host"], []).append((span(f), id, f))
    for host in sorted(by_host, key=lambda h: f"{h}"):
        lst = by_host[host]
        for id, f in d.openings.items():
            if id not in changed and f["host"] == host:
                lst.append((span(f), None, f))
        for i in range(len(lst)):
            if lst[i][1] is None:
                continue
            for j in range(len(lst)):
                if j != i and overlap(lst[i][0], lst[j][0]) and clashes(lst[i][2], lst[j][2]):
                    a, b = lst[i][0], lst[j][0]
                    return lst[i][1], (f"Açıklık ({a[0]:.3f}–{a[1]:.3f} m) aynı duvardaki başka bir açıklığa "
                                       f"({b[0]:.3f}–{b[1]:.3f} m) biniyor. Açıklığı kaydırın ya da daraltın.")
    return None


def refused_words(id, words):
    return (f"Tipi izleyen “$uidOf:{id}” açıklığı değişikliği alamıyor: {words} Hiçbir şey yazılmadı; açıklığı düzeltin, "
            "tipten ayırın ya da sığmayanları atlamayı (onRefused: skip) seçin.")


def follow(d, new, fields, refused="stop", locked=(), own=None):
    """(updates id → fields, report) or a refusal."""
    report = {"applied": [], "kept": [], "skipped": [], "refused": [], "fields": fields}
    nxt = {}
    for id in sorted(d.openings):
        f = d.openings[id]
        if own and id == own[0]:
            f = own[1]
        if f.get("style") != new["id"] or f.get("typeVersion") is None:
            continue
        if d.layers[id] in locked:
            report["skipped"].append({"uid": f"$uidOf:{id}", "why": "locked",
                                      "message": "Kilitli katmanda; eski sürümde kaldı. Kilidi açıp tipi yeniden uygulayın."})
            continue
        kept = [k for k in fields if k in f.get("overrides", [])]
        g = take_type(f, new)
        s0, s1 = span(g)
        if s0 < -1e-9 * max(length(g["host"]), 1) or s1 > length(g["host"]) * (1 + 1e-9):
            if refused == "stop":
                return refusal("opening_type_refused", refused_words(id, off_wall_words(g)), "operation")
            report["refused"].append({"uid": f"$uidOf:{id}", "why": "offWall", "message": off_wall_words(g)})
            continue
        if kept:
            report["kept"].append({"uid": f"$uidOf:{id}", "fields": kept})
        nxt[id] = g
    while True:
        c = first_clash(d, nxt)
        if c is None:
            break
        id, words = c
        if refused == "stop":
            return refusal("opening_type_refused", refused_words(id, words), "operation")
        report["kept"] = [k for k in report["kept"] if k["uid"] != f"$uidOf:{id}"]
        del nxt[id]
        report["refused"].append({"uid": f"$uidOf:{id}", "why": "overlap", "message": words})
    report["applied"] = [f"$uidOf:{id}" for id in nxt]
    return nxt, report


def warnings_of(report):
    out = []
    if report["kept"]:
        out.append({"code": "opening_type_kept",
                    "message": f"{len(report['kept'])} açıklık değişen alanda kendi değerini (örnek istisnası) korudu."})
    left = len(report["skipped"]) + len(report["refused"])
    if left:
        out.append({"code": "opening_type_skipped",
                    "message": f"{left} açıklık güncellenmedi ({len(report['skipped'])} kilitli katmanda, "
                               f"{len(report['refused'])} sığmadı); eski sürümde kaldı."})
    return out


# ── Steps ────────────────────────────────────────────────────────────────────────────────────────────────────────


def edited(types, report, library=None):
    out = {"types": [type_json(t) for t in types], "report": report, "revision": "$current"}
    if library is not None:
        out["library"] = library
    return out


EMPTY = {"applied": [], "kept": [], "skipped": [], "refused": [], "fields": []}


def types_step(d, op, inp, result, changed_ids=(), command=None):
    s = {"op": op, "input": inp, "result": result, "within": 1e-9}
    if command:
        s["command"] = command
    if result["status"] != "completed" or op != "execute":
        s["expect"] = {"ids": IDS_NOW(d), "revision": "same"}
        return s
    s["expect"] = {"ids": IDS_NOW(d), "settings": {"arch": d.settings_arch()},
                   "entities": {str(i): d.entity(i) for i in changed_ids}, "revision": "changed"}
    return s


def IDS_NOW(d):
    return [1, 2, 3, 4] + sorted(d.openings)


def find_project(d, key, path):
    folded = key.strip().lower()
    for t in d.types:
        if t["id"] == key:
            return t, None
    for t in d.types:
        if t["name"].lower() == folded:
            return t, None
    names = ", ".join(t["name"] for t in d.types)
    words = (f"Projede “{key}” tipi yok; projede hiç kapı ve pencere tipi yok. Önce bir tip tanımlayın." if not d.types
             else f"Projede “{key}” adında ya da kimliğinde tip yok. Var olanlar: {names}.")
    return None, refusal("opening_type_not_found", words, path)


def find_any(d, key, path):
    t, _ = find_project(d, key, path)
    if t:
        return ("project", t), None
    folded = key.strip().lower()
    for b in BUILTIN:
        if b["id"] == key:
            return ("builtin", b), None
    for b in BUILTIN:
        if b["name"].lower() == folded:
            return ("builtin", b), None
    names = ", ".join([t["name"] for t in d.types] + [t["name"] for t in BUILTIN])
    return None, refusal("opening_type_not_found",
                         f"“{key}” adında ya da kimliğinde kapı ve pencere tipi yok. Var olanlar: {names}.", path)


def of_builtin(b, id, name):
    t = {"id": id, "name": name, "version": 1, "form": b["form"], "width": b["width"], "leaves": b["leaves"],
         "operation": b["operation"]}
    for k in ("frame", "sill", "head", "depth"):
        if b.get(k) is not None:
            t[k] = b[k]
    return t


def id_of(name):
    fold = {"ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ı": "i", "I": "i", "İ": "i", "i": "i", "ö": "o", "Ö": "o", "ş": "s",
            "Ş": "s", "ü": "u", "Ü": "u"}
    out = ""
    for ch in name:
        if ch in fold:
            c = fold[ch]
        elif ch.isascii() and ch.isalnum():
            c = ch.lower()
        else:
            c = "-"
        if c == "-" and (not out or out.endswith("-")):
            continue
        out += c
    out = out.rstrip("-") or "tip"
    return out[:60]


def free_id(d, name):
    base = id_of(name)
    i, n = base, 2
    while any(t["id"] == i for t in d.types):
        i, n = f"{base}-{n}", n + 1
    return i


def taken(types, id, name, but=None):
    for k, t in enumerate(types):
        if k == but:
            continue
        if t["id"] == id:
            return f"Projede “{id}” kimlikli bir tip (“{t['name']}”) zaten var."
        if t["name"].lower() == name.lower():
            return f"Projede “{t['name']}” adında bir tip zaten var; başka bir ad verin."
    return None


def define(d, inp):
    op = inp["operation"]
    name = op["name"]
    if not name.strip() or name.strip() != name or len(name) > 128:
        return refusal("invalid_opening_type", f"“{name}” tip adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 128 harf.",
                       "operation.name"), None
    if op.get("id") is not None:
        i = op["id"]
        ok = i and len(i) <= 64 and not i.startswith("kentos-") and all(c.islower() and c.isascii() or c.isdigit() or c == "-" for c in i)
        if not ok:
            return refusal("invalid_opening_type", f"“{i}” tip kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf, “kentos-” ile başlamadan.",
                           "operation.id"), None
    else:
        i = free_id(d, name)
    why = taken(d.types, i, name)
    if why:
        return refusal("opening_type_exists", why, "operation.name"), None
    if op.get("copyOf") is not None:
        found, bad = find_any(d, op["copyOf"], "operation.copyOf")
        if bad:
            return bad, None
        kind, src = found
        base = (dict(copy.deepcopy(src), id=i, name=name, version=1) if kind == "project" else of_builtin(src, i, name))
        base.pop("source", None)
        if op.get("form") and op["form"] != base["form"]:
            return refusal("opening_type_form", f"Kopyalanan tip {FORM_LABEL[base['form']].lower()}; biçimi {FORM_LABEL[op['form']].lower()} olamaz. "
                           "Biçimi verilmeden kopyalayın ya da o biçimde bir tip seçin.", "operation.form"), None
    else:
        form = op.get("form", "door")
        base = of_builtin(next(b for b in BUILTIN if b["form"] == form), i, name)
    n, bad = changed(base, op.get("change") or {}, "operation.change")
    if bad:
        return bad, None
    return None, n


def edit_case(name, steps):
    return {"name": name, "steps": steps}


def update_step(d, key, change, refused="stop", locked=(), plan=False):
    """An Update: its step, and the drawing after it (a copy)."""
    inp = {"operation": {"kind": "update", "type": key, "change": change}}
    if refused != "stop":
        inp["onRefused"] = refused
    old, bad = find_project(d, key, "operation.type")
    if bad:
        return types_step(d, "execute", inp, bad), d
    if not change:
        return types_step(d, "execute", inp, refusal("nothing_to_set", "Tipin değişecek bir alanı verilmedi: kaba boşluk ya da ölçü, kanat, açılış, açı, kasa, denizlik, lento, derinlik ya da yapı verin.",
                                                     "operation.change")), d
    new, bad = changed(old, change, "operation.change")
    if bad:
        return types_step(d, "execute", inp, bad), d
    fields = differs(new, old)
    if not fields:
        return types_step(d, "execute", inp, refusal("nothing_to_set", f"“{old['name']}” tipi zaten bu değerlerde; hiçbir şey değişmiyor.",
                                                     "operation.change")), d
    new["version"] = old["version"] + 1
    r = follow(d, new, fields, refused, locked)
    if isinstance(r, dict):
        return types_step(d, "plan" if plan else "execute", inp, r), d
    nxt, report = r
    after = copy.deepcopy(d)
    after.types = [new if t["id"] == new["id"] else t for t in after.types]
    for i, f in nxt.items():
        after.openings[i] = f
    result = done(edited(after.types, report), warnings_of(report))
    if plan:
        return types_step(d, "plan", inp, result), d
    return types_step(after, "execute", inp, result, nxt.keys()), after


# ── cad.openingtypes.edit ────────────────────────────────────────────────────────────────────────────────────────


def define_case(name, op):
    d = copy.deepcopy(BASE)
    inp = {"operation": op}
    bad, n = define(d, inp)
    if bad:
        return edit_case(name, [types_step(d, "validate", inp, {"status": bad["status"], "error": bad["error"]}),
                                types_step(d, "execute", inp, bad)])
    after = copy.deepcopy(d)
    after.types.append(n)
    result = done(edited(after.types, EMPTY))
    return edit_case(name, [
        types_step(d, "plan", inp, result),
        types_step(after, "execute", inp, result),
        {"op": "undo", "returns": "Açıklık tipi", "expect": {"settings": {"arch": d.settings_arch()}, "canUndo": False,
                                                            "revision": "changed"}},
        {"op": "redo", "returns": "Açıklık tipi", "expect": {"settings": {"arch": after.settings_arch()}, "canUndo": True,
                                                            "revision": "changed"}},
    ])


def types_edit_cases():
    out = [
        define_case("Yeni tip: K01'in kopyası, 80 cm kanatla", {"kind": "define", "name": "K03", "copyOf": "K01",
                                                                 "change": {"nominal": {"by": "leaf", "value": 0.8}}}),
        define_case("Yeni tip: KentOS'un Kapı 90'ı adıyla kopyalanır", {"kind": "define", "name": "K03", "copyOf": "kapı 90"}),
        define_case("Yeni tip: biçimi verilen pencere, ilk KentOS penceresinden", {"kind": "define", "name": "P02", "form": "window"}),
        define_case("Yeni tip: kimlik verilir, kaba boşlukla", {"kind": "define", "name": "Islak hacim kapısı", "id": "islak-kapi",
                                                               "copyOf": "K02", "change": {"width": 0.8}}),
        define_case("Yeni tip: kimlik adından, Türkçe harfler katlanır", {"kind": "define", "name": "Çift kanat Ş 2",
                                                                          "copyOf": "kentos-iki-kanatli-kapi-140",
                                                                          "change": {"build": {"split": 0.6}}}),
        define_case("Yeni tip: kasa dış eniyle ölçülen sürme kapı", {"kind": "define", "name": "S01", "copyOf": "Sürme kapı 160",
                                                                    "change": {"build": {"fit": 0.015},
                                                                               "nominal": {"by": "unit", "value": 1.6}}}),
        define_case("Ret: aynı ad (büyük küçük harf ayırmadan)", {"kind": "define", "name": "k01"}),
        define_case("Ret: KentOS'a ayrılmış kimlik", {"kind": "define", "name": "K09", "id": "kentos-k09"}),
        define_case("Ret: boşluklu ad", {"kind": "define", "name": " K09"}),
        define_case("Ret: kopyalanacak tip yok", {"kind": "define", "name": "K09", "copyOf": "K99"}),
        define_case("Ret: kapıdan pencere kopyalanmaz", {"kind": "define", "name": "K09", "copyOf": "K02", "form": "window"}),
        define_case("Ret: kasa açıklığın yarısını geçer", {"kind": "define", "name": "K09", "copyOf": "K02",
                                                         "change": {"frame": 0.6}}),
        define_case("Ret: kaba boşluk ile ölçü birlikte", {"kind": "define", "name": "K09", "copyOf": "K02",
                                                          "change": {"width": 1.0, "nominal": {"by": "leaf", "value": 0.9}}}),
        define_case("Ret: yapının olmayan alanı kaldırılmaz", {"kind": "define", "name": "K09", "copyOf": "K02",
                                                             "change": {"unset": ["frame"]}}),
        define_case("Ret: kanat payı birden büyük", {"kind": "define", "name": "K09", "copyOf": "K02",
                                                    "change": {"build": {"split": 1.5}}}),
    ]
    # The acceptance (docs/adr/0772): K01's frame changed, its 20 doors follow, the one with its own frame keeps it.
    d = copy.deepcopy(BASE)
    plan, _ = update_step(d, "K01", {"frame": 0.06}, plan=True)
    step, after = update_step(d, "K01", {"frame": 0.06})
    out.append(edit_case("Kabul: K01'in kasası değişir; 19 kapı izler, istisnalı kapı kendi kasasını korur, kanat 90 cm kalır",
                         [plan, step, {"op": "undo", "returns": "Açıklık tipi",
                                       "expect": {"settings": {"arch": d.settings_arch()},
                                                  "entities": {str(i): d.entity(i) for i in range(5, 25)}, "revision": "changed"}}]))
    step, after = update_step(copy.deepcopy(BASE), "k01", {"nominal": {"by": "unit", "value": 0.98}})
    out.append(edit_case("Tipin ölçüsü kasa dış eniyle verilir; kaba boşluk ondan", [step]))
    step, _ = update_step(copy.deepcopy(BASE), "P01", {"unset": ["mullions"], "head": 2.3})
    out.append(edit_case("Pencere tipinin dikey kaydı kaldırılır, lentosu yükselir", [step]))
    step, _ = update_step(copy.deepcopy(BASE), "K02", {"width": 1.4})
    out.append(edit_case("Sığmayan açıklık bütün değişikliği reddeder (dur)", [step]))
    step, _ = update_step(copy.deepcopy(BASE), "K02", {"width": 1.4}, refused="skip")
    out.append(edit_case("Sığmayan açıklık atlanır, eski sürümde kalır (atla)", [step]))
    step, _ = update_step(copy.deepcopy(BASE), "K01", {"width": 3.0}, refused="skip")
    out.append(edit_case("Komşusuna binen kapılar teker teker atlanır", [step]))
    step, _ = update_step(copy.deepcopy(BASE), "K01", {"width": 3.0})
    out.append(edit_case("Komşusuna binen kapı bütün değişikliği reddeder", [step]))
    out.append(edit_case("Ret: değişecek alan yok", [update_step(copy.deepcopy(BASE), "K01", {})[0]]))
    d = copy.deepcopy(BASE)
    inp = {"operation": {"kind": "update", "type": "K01", "change": {"frame": 0.06}}, "expectedRevision": "x1"}
    out.append(edit_case("Ret: geçersiz beklenen sürüm", [types_step(d, "execute", inp, refusal(
        "invalid_revision", "Beklenen sürüm “x1” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
        "expectedRevision"))]))
    conflict = refusal("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                       "expectedRevision")
    conflict["status"] = "conflict"
    conflict["error"]["revision"] = "$current"
    inp = {"operation": {"kind": "update", "type": "K01", "change": {"frame": 0.06}}, "expectedRevision": "999"}
    out.append(edit_case("Ret: çizim değişmiş", [types_step(d, "execute", inp, conflict)]))
    inp = {"operation": {"kind": "update", "type": "K01", "change": {"width": 1.0}}}
    step = types_step(d, "execute", inp, refusal("not_finite", "Kaba boşluk sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.",
                                                 "operation.change.width"))
    step["nonFinite"] = {"operation.change.width": "NaN"}
    out.append(edit_case("Ret: sonlu olmayan kaba boşluk", [step]))
    move = {"op": "execute", "command": "cad.entities.set", "input": {"uids": ["$uidOf:26"], "layerId": "kapi2", "operation": "layer"},
            "result": "$any"}
    lock = {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "kapi2", "value": True}, "result": "$any"}
    inp = {"operation": {"kind": "remove", "type": "K02", "detach": True}}
    out.append(edit_case("Ret: kilitli katmandaki kapı tipten ayrılmaz", [move, lock, types_step(d, "execute", inp, refusal(
        "layer_locked", "Tipi izleyen bir açıklık kilitli katmanda; tipten ayrılamaz. Kilidi açın ya da tipi silmeyin (“$uidOf:26”).",
        "operation.type"))]))
    out.append(edit_case("Ret: tip zaten bu değerlerde", [update_step(copy.deepcopy(BASE), "K01", {"frame": 0.05})[0]]))
    out.append(edit_case("Ret: projede olmayan tip", [update_step(copy.deepcopy(BASE), "K09", {"frame": 0.05})[0]]))
    # A locked layer: the door there stays at the older version.
    d = copy.deepcopy(BASE)
    d.layers[26] = "kapi2"
    lock = {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "kapi2", "value": True},
            "result": "$any"}
    move = {"op": "execute", "command": "cad.entities.set", "input": {"uids": ["$uidOf:26"], "layerId": "kapi2", "operation": "layer"},
            "result": "$any"}
    step, _ = update_step(d, "K02", {"head": 2.2}, locked=("kapi2",))
    out.append(edit_case("Kilitli katmandaki kapı eski sürümde kalır ve söylenir", [move, lock, step]))
    # Rename and remove.
    d = copy.deepcopy(BASE)
    after = copy.deepcopy(d)
    after.types[1]["name"] = "K-02"
    inp = {"operation": {"kind": "rename", "type": "k02", "name": "K-02"}}
    out.append(edit_case("Yeniden adlandır: kimlik ve izleyenler aynı", [types_step(after, "execute", inp, done(edited(after.types, EMPTY)))]))
    inp = {"operation": {"kind": "rename", "type": "K02", "name": "k01"}}
    out.append(edit_case("Ret: başka tipin adı", [types_step(d, "execute", inp, refusal("opening_type_exists",
                                                  "Projede “K01” adında bir tip zaten var; başka bir ad verin.", "operation.name"))]))
    inp = {"operation": {"kind": "rename", "type": "K02", "name": "K02"}}
    out.append(edit_case("Ret: ad aynı", [types_step(d, "execute", inp, refusal("nothing_to_set", "Tipin adı zaten “K02”.",
                                                                               "operation.name"))]))
    inp = {"operation": {"kind": "remove", "type": "K02"}}
    out.append(edit_case("Ret: izlenen tip silinmez", [types_step(d, "execute", inp, refusal(
        "opening_type_in_use", "“K02” tipini 3 açıklık izliyor; silmek için onları tipten ayırın (detach: true) ya da önce başka tip verin.",
        "operation.type"))]))
    after = copy.deepcopy(d)
    after.types = [t for t in after.types if t["id"] != "k02"]
    for i in (25, 26, 27):
        f = after.openings[i]
        f.pop("typeVersion", None)
        f["style"], f["overrides"] = None, []
    rep = dict(EMPTY, applied=[f"$uidOf:{i}" for i in (25, 26, 27)])
    inp = {"operation": {"kind": "remove", "type": "K02", "detach": True}}
    out.append(edit_case("Sil ve ayır: kapılar değerleriyle kalır, tip izlemez",
                         [types_step(after, "execute", inp, done(edited(after.types, rep)), (25, 26, 27))]))
    # The office library.
    lib_type = {"id": "ofis-k10", "name": "Ofis K10", "version": 3, "form": "door", "width": 1.0, "frame": 0.04, "head": 2.2}
    library = {"format": "kentos.opening-types", "version": 1, "library": "Ofis 2026", "types": [type_json(lib_type)]}
    after = copy.deepcopy(d)
    after.types.append(dict(lib_type, source="Ofis 2026"))
    inp = {"operation": {"kind": "import", "library": library}}
    out.append(edit_case("Kitaplıktan al: tip kaynağıyla gelir", [types_step(after, "execute", inp, done(edited(after.types, EMPTY)))]))
    clash = dict(library, types=[type_json(dict(K02, id="ofis-k02"))])
    inp = {"operation": {"kind": "import", "library": clash}}
    out.append(edit_case("Ret: aynı adlı başka kaynaklı tip", [types_step(d, "execute", inp, refusal(
        "opening_type_exists", "Projede “K02” adında bir tip zaten var; başka bir ad verin.", "operation.library.types[0].name"))]))
    inp = {"operation": {"kind": "import", "library": clash, "onConflict": "skip"}}
    step = types_step(d, "execute", inp, done(edited(d.types, EMPTY)))
    step["expect"]["revision"] = "same"  # nothing taken: the drawing is as it was
    out.append(edit_case("Çakışmada atla: projeninki kalır", [step]))
    after = copy.deepcopy(d)
    renamed = dict(K02, id="k02-2", name="K02 (2)", source="Ofis 2026")
    after.types.append(renamed)
    inp = {"operation": {"kind": "import", "library": clash, "onConflict": "rename"}}
    out.append(edit_case("Çakışmada yeniden adlandır: yanına “(2)” ile gelir",
                         [types_step(after, "execute", inp, done(edited(after.types, EMPTY)))]))
    newer = dict(P01, version=1, head=2.4)
    newer.pop("source")
    lib = dict(library, types=[type_json(newer)])
    inp = {"operation": {"kind": "import", "library": lib, "onConflict": "replace"}}
    new = dict(P01, head=2.4, version=2)
    r = follow(d, new, ["head"])
    nxt, report = r
    after = copy.deepcopy(d)
    after.types = [new if t["id"] == "p01" else t for t in after.types]
    for i, f in nxt.items():
        after.openings[i] = f
    out.append(edit_case("Aynı kitaplıktan değiştir: yeni sürüm, pencere izler",
                         [types_step(after, "execute", inp, done(edited(after.types, report), warnings_of(report)), nxt.keys())]))
    inp = {"operation": {"kind": "import", "library": dict(library, format="kentos.style")}}
    out.append(edit_case("Ret: kitaplık değil", [types_step(d, "execute", inp, refusal(
        "import_unreadable", "Dosya bir KentOS kapı ve pencere tipleri kitaplığı değil (“kentos.style” sürüm 1); “kentos.opening-types” sürüm 1 beklenir.",
        "operation.library"))]))
    inp = {"operation": {"kind": "export", "library": "Ofis 2026", "types": ["K01", "p01"]}}
    lib = {"format": "kentos.opening-types", "version": 1, "library": "Ofis 2026", "types": [type_json(K01), type_json(P01)]}
    out.append(edit_case("Kitaplığa yaz: çizim değişmez, dosyanın içeriği gelir",
                         [types_step(d, "execute", inp, done(edited(d.types, EMPTY, lib)))]))
    step = types_step(d, "execute", inp, done(edited(d.types, EMPTY, lib)))
    step["expect"] = {"ids": IDS_NOW(d), "revision": "same"}
    out[-1]["steps"] = [step]
    inp = {"operation": {"kind": "export", "library": "Ofis", "types": ["K99"]}}
    out.append(edit_case("Ret: yazılacak tip yok", [types_step(d, "execute", inp, find_project(d, "K99", "operation.types[0]")[1])]))
    return out


# ── cad.openings.query ───────────────────────────────────────────────────────────────────────────────────────────


def type_name(style):
    for t in BASE.types:
        if t["id"] == style:
            return t["name"]
    for b in BUILTIN:
        if b["id"] == style or b["name"] == style:
            return b["name"]
    return style


def row(d, i):
    f = d.openings[i]
    out = {"uid": f"$uidOf:{i}", "form": f["form"]}
    if f.get("number") is not None:
        out["number"] = f["number"]
    if f.get("style") is not None:
        out["type"] = f["style"]
        out["typeName"] = type_name(f["style"])
    if f.get("typeVersion") is not None:
        out["typeVersion"] = f["typeVersion"]
        t = next((t for t in d.types if t["id"] == f["style"]), None)
        if t and f["typeVersion"] < t["version"]:
            out["stale"] = True
    if f.get("overrides"):
        out["overrides"] = f["overrides"]
    level = WALLS[f["host"]][3]
    if level:
        out["level"] = level
    out["host"] = f"$uidOf:{f['host']}"
    out["sizes"] = sizes(f)
    return out


def uses(d):
    out = []
    for t in d.types:
        fs = [f for f in d.openings.values() if f.get("style") == t["id"] and f.get("typeVersion") is not None]
        out.append({"type": type_json(t), "count": len(fs), "stale": sum(1 for f in fs if f["typeVersion"] < t["version"]),
                     "exceptions": sum(1 for f in fs if f.get("overrides"))})
    return out


def query_case(name, inp, ids=None, bad=None, d=None):
    d = d or BASE
    if bad:
        result = bad
    else:
        result = done({"openings": [row(d, i) for i in ids], "types": uses(d), "revision": "$current"})
    steps = [{"op": op, "input": inp, "result": result if op != "validate" or bad is None else {"status": "completed", "output": None, "warnings": []},
              "within": 1e-9, "expect": {"revision": "same"}} for op in ("plan", "execute")]
    return {"name": name, "steps": steps}


def query_cases():
    all_ids = sorted(BASE.openings)
    k01 = list(range(5, 25))
    out = [
        query_case("Bütün açıklıklar, tipleri ve genişlikleri", {}, all_ids),
        query_case("Aynı tiptekiler: K01'in 20 kapısı", {"type": "K01"}, k01),
        query_case("Aynı tiptekiler kimlikle", {"type": "k02"}, [25, 26, 27]),
        query_case("Aynı tiptekiler katla: K01 zemin katta", {"type": "K01", "level": "zemin"}, k01),
        query_case("Aynı tiptekiler katla: K01 1. katta yok", {"type": "K01", "level": "kat1"}, []),
        query_case("Katsızlar: üçüncü duvarın açıklıkları", {"level": None}, list(range(29, 35))),
        query_case("1. katın açıklıkları", {"level": "kat1"}, [25, 26, 27, 28]),
        query_case("Seçim kapsamı: iki kapı", {"uids": ["$uidOf:6", "$uidOf:30"]}, [6, 30]),
        query_case("Seçim kapsamı ve tip birlikte", {"uids": ["$uidOf:6", "$uidOf:30"], "type": "K01"}, [6]),
        query_case("Biçim: pencereler", {"form": "window"}, [28, 34]),
        query_case("KentOS tipiyle yapılanlar adıyla", {"type": "Kapı 90"}, [29]),
        query_case("Net geçiş: iki kanatlı, eşit olmayan kanatlar", {"uids": ["$uidOf:30"]}, [30]),
        query_case("Net geçiş: iki kanatlı sürme kapı", {"uids": ["$uidOf:31"]}, [31]),
        query_case("Net geçiş: 60° açılan kapı", {"uids": ["$uidOf:32"]}, [32]),
        query_case("Net geçiş: boşluk ışığı kadar", {"uids": ["$uidOf:33"]}, [33]),
        query_case("Net geçiş: 90 cm kanat ile 90 cm kaba boşluk aynı değil", {"uids": ["$uidOf:5", "$uidOf:25"]}, [5, 25]),
        query_case("İstisnalı kapı kendi kasasıyla", {"uids": ["$uidOf:10"]}, [10]),
        query_case("Ret: projede ve KentOS'ta olmayan tip", {"type": "K99"}, bad=find_any(BASE, "K99", "type")[1]),
        query_case("Ret: projede olmayan kat", {"level": "kat9"},
                   bad=refusal("level_not_found", "Projede “kat9” kimlikli kat yok.", "level")),
        query_case("Ret: duvar açıklık değil", {"uids": ["$uidOf:1"]},
                   bad=refusal("not_an_opening", "Nesne kapı, pencere ya da boşluk değil.", "uids[0]")),
        query_case("Ret: geçersiz kimlik", {"uids": ["K1"]}, bad=refusal("invalid_uid",
                   "“K1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.",
                   "uids[0]")),
        query_case("Ret: çizimde olmayan nesne", {"uids": ["0192f5a1-7720-7000-8000-00000000ffff"]}, bad=refusal(
            "entity_not_found", "“0192f5a1-7720-7000-8000-00000000ffff” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
            "uids[0]")),
    ]
    # After a locked door missed K02's change: it is behind, counted.
    d = copy.deepcopy(BASE)
    d.layers[26] = "kapi2"
    step, after = update_step(d, "K02", {"head": 2.2}, locked=("kapi2",))
    c = query_case("Kilitli katmanda kalan kapı eski sürümde görünür", {"type": "K02"}, [25, 26, 27], d=after)
    move = {"op": "execute", "command": "cad.entities.set", "input": {"uids": ["$uidOf:26"], "layerId": "kapi2", "operation": "layer"}, "result": "$any"}
    lock = {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "kapi2", "value": True},
            "result": "$any"}
    upd = {"op": "execute", "command": "cad.openingtypes.edit", "input": {"operation": {"kind": "update", "type": "K02",
                                                                                         "change": {"head": 2.2}}}, "result": "$any"}
    c["steps"] = [move, lock, upd] + c["steps"]
    out.append(c)
    return out


# ── cad.openings.edit with types ─────────────────────────────────────────────────────────────────────────────────


def opening_step(d, op, inp, result, changed_ids=(), settings=False, command=None):
    s = {"op": op, "input": inp, "result": result, "within": 1e-9}
    if command:
        s["command"] = command
    if result["status"] != "completed" or op != "execute":
        s["expect"] = {"ids": IDS_NOW(d), "revision": "same"}
        return s
    s["expect"] = {"ids": IDS_NOW(d), "entities": {str(i): d.entity(i) for i in changed_ids}, "revision": "changed"}
    if settings:
        s["expect"]["settings"] = {"arch": d.settings_arch()}
    return s


def edited_openings(d, ids, report=None):
    out = {"openings": [d.entity(i) for i in ids], "revision": "$current"}
    if report is not None:
        out["report"] = report
    return out


def override_warning(fields, at=0):
    names = ", ".join(f"“{LABELS[k]}”" for k in fields)
    return {"code": "opening_type_override",
            "message": f"Açıklık tipinin {names} alanında artık kendi değerini tutuyor (örnek istisnası); tipe döndürmek için Tipe döndür.",
            "path": f"uids[{at}]"}


def set_case(name, i, change):
    d = copy.deepcopy(BASE)
    f = copy.deepcopy(d.openings[i])
    if "form" in change and f.get("typeVersion") is not None and change["form"] != f["form"]:
        inp = {"uids": [f"$uidOf:{i}"], "operation": dict({"kind": "set"}, **change)}
        return edit_case(name, [opening_step(d, "execute", inp, refusal(
            "opening_type_form", "Açıklık bir tipi izliyor; biçim tipindir. Biçimi değiştirmek için önce tipten ayırın ya da başka tip verin.",
            "operation.form"))])
    g = copy.deepcopy(f)
    for k, v in change.items():
        if k == "build":
            for bk, bv in v.items():
                set_value(g, bk, bv)
        else:
            g[k] = v
    warnings = []
    if g.get("typeVersion") is not None:
        own = [k for k in FIELDS if value(g, k) != value(f, k) and k not in g.get("overrides", [])]
        if own:
            g["overrides"] = sorted(set(g.get("overrides", []) + own), key=FIELDS.index)
            warnings.append(override_warning(own))
    d.openings[i] = g
    inp = {"uids": [f"$uidOf:{i}"], "operation": dict({"kind": "set"}, **change)}
    return edit_case(name, [opening_step(d, "execute", inp, done(edited_openings(d, [i]), warnings), [i])])


def create_case(name, n, layer="kapi"):
    d = copy.deepcopy(BASE)
    t = next(t for t in TYPES if n.get("type") in (t["name"], t["id"]))
    if n.get("form") and n["form"] != t["form"]:
        inp = {"layerId": layer, "openings": [dict(n, host="$uidOf:2")]}
        return edit_case(name, [opening_step(d, "execute", inp, refusal(
            "opening_type_form", f"“{t['name']}” tipi {FORM_LABEL[t['form']].lower()}; açıklığın biçimi tipindir. Biçimi vermeyin ya da o biçimde bir tip seçin.",
            "openings[0].form"), command="cad.openings.create")])
    own = {k: n[k] for k in ("width", "swing", "frame", "sill", "head") if k in n}
    own.update(n.get("build") or {})
    f = from_type(t, 2, n["at"], number=n.get("number"), **own)
    d.add(NEW, f, layer)
    inp = {"layerId": layer, "openings": [dict(n, host="$uidOf:2")]}
    result = done({"uids": ["$any"], "ids": [NEW], "revision": "$current"})
    return edit_case(name, [opening_step(d, "execute", inp, result, [NEW], command="cad.openings.create"),
                            {"op": "undo", "returns": FORM_LABEL[t["form"]], "expect": {"ids": IDS, "revision": "changed"}}])


def not_typed(at=0):
    return refusal("opening_not_typed", "Açıklık bir proje tipini izlemiyor; önce bir tip verin (Türü değiştir).", f"uids[{at}]")


def opening_edit_cases():
    out = [
        create_case("Ekle: K01 tipinde kapı tipi izler", {"at": 8.0, "type": "K01", "number": "K30"}),
        create_case("Ekle: K01'in kaba boşluğu örneğe özel", {"at": 8.0, "type": "K01", "width": 1.1}),
        create_case("Ekle: kanat kalınlığı örneğe özel", {"at": 8.0, "type": "k01", "build": {"leafThickness": 0.05}}),
        create_case("Ret: tipin biçimi değiştirilmez", {"at": 8.0, "type": "K01", "form": "window"}),
        set_case("Değiştir: izleyen kapının kasası istisna olur ve söylenir", 6, {"frame": 0.06}),
        set_case("Değiştir: kanat payı ve kalınlığı istisna", 7, {"build": {"leafThickness": 0.05}}),
        set_case("Değiştir: numara tipin değil", 6, {"number": "K99"}),
        set_case("Ret: izleyen kapının biçimi değiştirilmez", 6, {"form": "window"}),
        set_case("Değiştir: KentOS tipiyle yapılan kapıya yapı eklenir, istisna yok", 29, {"build": {"leafThickness": 0.05,
                                                                                                         "useZone": 1.2}}),
    ]
    # Türü değiştir to a project type: it follows, its exceptions gone.
    d = copy.deepcopy(BASE)
    d.openings[10] = from_type(K02, 1, d.openings[10]["at"], number=d.openings[10]["number"])
    inp = {"uids": ["$uidOf:10"], "operation": {"kind": "replaceType", "to": "K02"}}
    out.append(edit_case("Türü değiştir: proje tipini izler, istisnası kalkar",
                         [opening_step(d, "execute", inp, done(edited_openings(d, [10])), [10])]))
    d = copy.deepcopy(BASE)
    inp = {"operation": {"kind": "replaceType", "from": "K01", "to": "K02", "level": "kat1"}}
    out.append(edit_case("Aynı tiptekileri değiştir: 1. katta K01 yok", [opening_step(d, "execute", inp, refusal(
        "style_not_used", "Çizimde “K01” türünde açıklık yok.", "operation"))]))
    d = copy.deepcopy(BASE)
    for i in (25, 26, 27):
        f = d.openings[i]
        d.openings[i] = from_builtin("kentos-kapi-80", 2, f["at"], number=f["number"])
    inp = {"operation": {"kind": "replaceType", "from": "K02", "to": "Kapı 80", "level": "kat1"}}
    out.append(edit_case("Aynı tiptekileri değiştir: 1. kattaki K02'ler KentOS'un Kapı 80'i olur, tipi izlemez",
                         [opening_step(d, "execute", inp, done(edited_openings(d, [25, 26, 27])), [25, 26, 27])]))
    # Tipten ayır.
    d = copy.deepcopy(BASE)
    f = d.openings[10]
    f["style"], f["overrides"] = None, []
    f.pop("typeVersion")
    inp = {"uids": ["$uidOf:10"], "operation": {"kind": "detach"}}
    out.append(edit_case("Tipten ayır: değerleri kalır, tip izlemez", [opening_step(d, "execute", inp, done(edited_openings(d, [10])), [10])]))
    out.append(edit_case("Ret: tipi izlemeyen kapı ayrılmaz", [opening_step(BASE, "execute",
                         {"uids": ["$uidOf:29"], "operation": {"kind": "detach"}}, not_typed())]))
    # Tipe uygula: the exception's frame becomes K01's; the leaf stays 90 cm, the rough opening follows.
    d = copy.deepcopy(BASE)
    old = d.types[0]
    new = copy.deepcopy(old)
    new["frame"] = 0.07
    keep_nominal(old, new)
    new["version"] = 2
    own = copy.deepcopy(d.openings[10])
    own["overrides"] = []
    r = follow(d, new, differs(new, old), own=(10, own))
    nxt, report = r
    after = copy.deepcopy(d)
    after.types[0] = new
    for i, f in nxt.items():
        after.openings[i] = f
    inp = {"uids": ["$uidOf:10"], "operation": {"kind": "push"}}
    result = done(edited_openings(after, sorted(nxt), report), warnings_of(report))
    out.append(edit_case("Tipe uygula: istisnalı kasa K01'in olur, 20 kapı izler, kanat 90 cm kalır", [
        opening_step(d, "plan", inp, result),
        opening_step(after, "execute", inp, result, sorted(nxt), settings=True),
        {"op": "undo", "returns": "Açıklığı düzenle", "expect": {"settings": {"arch": d.settings_arch()},
                                                                 "entities": {"10": d.entity(10)}, "revision": "changed"}},
    ]))
    out.append(edit_case("Ret: istisnası olmayan kapı tipe uygulanmaz", [opening_step(BASE, "execute",
                         {"uids": ["$uidOf:6"], "operation": {"kind": "push"}}, refusal(
                             "nothing_to_set", "Açıklığın tipe uygulanacak kendi değeri yok: alanları verin ya da önce bir istisna yapın.",
                             "operation.fields"))]))
    # Tipe uygula with another door keeping its own frame: it keeps it, said.
    d = copy.deepcopy(BASE)
    six = d.openings[6]
    six["frame"], six["overrides"] = 0.06, ["frame"]
    old = d.types[0]
    new = dict(copy.deepcopy(old), frame=0.07)
    keep_nominal(old, new)
    new["version"] = 2
    own = dict(copy.deepcopy(d.openings[10]), overrides=[])
    nxt, report = follow(d, new, differs(new, old), own=(10, own))
    after = copy.deepcopy(d)
    after.types[0] = new
    for i, f in nxt.items():
        after.openings[i] = f
    set6 = {"op": "execute", "input": {"uids": ["$uidOf:6"], "operation": {"kind": "set", "frame": 0.06}}, "result": "$any"}
    inp = {"uids": ["$uidOf:10"], "operation": {"kind": "push", "fields": ["frame"]}}
    out.append(edit_case("Tipe uygula: kendi kasası olan başka kapı onu korur ve söylenir", [
        set6, opening_step(after, "execute", inp, done(edited_openings(after, sorted(nxt), report), warnings_of(report)),
                           sorted(nxt), settings=True)]))
    # Tipe uygula with a follower on a locked layer: it stays behind.
    d = copy.deepcopy(BASE)
    d.layers[26] = "kapi2"
    d.openings[25]["head"], d.openings[25]["overrides"] = 2.3, ["head"]
    old = d.types[1]
    new = dict(copy.deepcopy(old), head=2.3, version=2)
    own = dict(copy.deepcopy(d.openings[25]), overrides=[])
    nxt, report = follow(d, new, ["head"], locked=("kapi2",), own=(25, own))
    after = copy.deepcopy(d)
    after.types[1] = new
    for i, f in nxt.items():
        after.openings[i] = f
    move = {"op": "execute", "command": "cad.entities.set", "input": {"uids": ["$uidOf:26"], "layerId": "kapi2", "operation": "layer"},
            "result": "$any"}
    lock = {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "kapi2", "value": True}, "result": "$any"}
    set25 = {"op": "execute", "input": {"uids": ["$uidOf:25"], "operation": {"kind": "set", "head": 2.3}}, "result": "$any"}
    inp = {"uids": ["$uidOf:25"], "operation": {"kind": "push"}}
    out.append(edit_case("Tipe uygula: kilitli katmandaki kapı eski sürümde kalır", [
        move, lock, set25,
        opening_step(after, "execute", inp, done(edited_openings(after, sorted(nxt), report), warnings_of(report)),
                     sorted(nxt), settings=True)]))
    # Tipe uygula a width the wall's end cannot take: nothing written.
    d = copy.deepcopy(BASE)
    d.openings[25]["width"], d.openings[25]["overrides"] = 1.4, ["width"]
    old = d.types[1]
    new = dict(copy.deepcopy(old), width=1.4, version=2)
    own = dict(copy.deepcopy(d.openings[25]), overrides=[])
    bad = follow(d, new, ["width"], own=(25, own))
    set25 = {"op": "execute", "input": {"uids": ["$uidOf:25"], "operation": {"kind": "set", "width": 1.4}}, "result": "$any"}
    inp = {"uids": ["$uidOf:25"], "operation": {"kind": "push"}}
    out.append(edit_case("Ret: tipe uygulanan genişlik duvarın ucundaki kapıya sığmaz", [set25, opening_step(d, "execute", inp, bad)]))
    out.append(edit_case("Ret: tipe tek kapı uygulanır", [opening_step(BASE, "execute",
                         {"uids": ["$uidOf:6", "$uidOf:7"], "operation": {"kind": "push"}}, refusal(
                             "invalid_opening", "Tipe uygulamak için tek bir açıklık verin; onun değerleri tipin olur.", "uids"))]))
    out.append(edit_case("Ret: tip zaten kapının değerlerinde", [opening_step(BASE, "execute",
                         {"uids": ["$uidOf:6"], "operation": {"kind": "push", "fields": ["head"]}}, refusal(
                             "nothing_to_set", "“K01” tipi zaten açıklığın bu değerlerinde.", "operation.fields"))]))
    # Tipe döndür.
    d = copy.deepcopy(BASE)
    d.openings[10] = from_type(K01, 1, d.openings[10]["at"], number=d.openings[10]["number"])
    inp = {"uids": ["$uidOf:10"], "operation": {"kind": "reset"}}
    out.append(edit_case("Tipe döndür: istisna kalkar, tipin kasası gelir", [opening_step(d, "execute", inp, done(edited_openings(d, [10])), [10])]))
    d = copy.deepcopy(BASE)
    inp = {"uids": ["$uidOf:10"], "operation": {"kind": "reset", "fields": ["width"]}}
    step = opening_step(d, "execute", inp, done(edited_openings(d, [10])), [10])
    step["expect"]["revision"] = "same"  # the door keeps its frame and takes nothing new
    out.append(edit_case("Tipe döndür: yalnız verilen alan; kasa istisnası kalır", [step]))
    out.append(edit_case("Ret: tipi izlemeyen kapı tipe döndürülmez", [opening_step(BASE, "execute",
                         {"uids": ["$uidOf:30"], "operation": {"kind": "reset"}}, not_typed())]))
    # Tipi yeniden uygula after a locked door missed the change.
    d = copy.deepcopy(BASE)
    d.layers[26] = "kapi2"
    _, after = update_step(d, "K02", {"head": 2.2}, locked=("kapi2",))
    after.openings[26] = take_type(after.openings[26], after.types[1])
    move = {"op": "execute", "command": "cad.entities.set", "input": {"uids": ["$uidOf:26"], "layerId": "kapi2", "operation": "layer"},
            "result": "$any"}
    lock = {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "kapi2", "value": True}, "result": "$any"}
    unlock = {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "kapi2", "value": False}, "result": "$any"}
    upd = {"op": "execute", "command": "cad.openingtypes.edit", "input": {"operation": {"kind": "update", "type": "K02", "change": {"head": 2.2}}},
           "result": "$any"}
    inp = {"uids": ["$uidOf:26"], "operation": {"kind": "refresh"}}
    out.append(edit_case("Tipi yeniden uygula: kilitte kalan kapı son sürümü alır",
                         [move, lock, upd, unlock, opening_step(after, "execute", inp, done(edited_openings(after, [26])), [26])]))
    out.append(edit_case("Ret: tipi izlemeyen kapı yeniden uygulanmaz", [opening_step(BASE, "execute",
                         {"uids": ["$uidOf:31"], "operation": {"kind": "refresh"}}, not_typed())]))
    return out


# ── cad.arch.audit's doors ───────────────────────────────────────────────────────────────────────────────────────


def AUID(n):
    return f"0192f5a1-7721-7000-8000-{n:012x}"


DWALLS = {1: ((0.0, 0.0), (8.0, 0.0)), 2: ((2.5, 0.1), (2.5, 4.0)), 3: ((0.0, -1.8), (8.0, -1.8))}
# The doors: wall, centre, width, side, hand, number, build.
DOORS = {
    5: (1, 2.0, 1.0, "left", "start", "K1", None),
    6: (1, 5.0, 1.0, "left", "start", "K2", {"useZone": 1.2}),
    7: (1, 6.5, 1.0, "left", "end", "K3", None),
    8: (1, 3.5, 1.0, "right", "start", "K4", None),
    9: (3, 3.6, 1.0, "left", "start", "K5", None),
    10: (3, 6.0, 0.8, "left", "start", "K6", None),
}
COLUMN = 11
FIXTURE = 12
BLOCK = "0192f5a1-7721-7000-8000-0000000000b1"


def door_place(w, at, width):
    (x0, y), _ = DWALLS[w]
    s0, s1 = at - width / 2, at + width / 2
    return [P(x0 + s0, y + 0.1), P(x0 + s0, y - 0.1), P(x0 + s1, y + 0.1), P(x0 + s1, y - 0.1)]


def door_entity(id, spec):
    w, at, width, side, hand, number, build = spec
    e = {"kind": "opening", "id": id, "layerId": "kapi", "attrs": {}, "host": AUID(w), "at": at, "width": width,
         "form": "door"}
    if hand != "start":
        e["hand"] = hand
    if side != "left":
        e["side"] = side
    e.update({"frame": 0.05, "head": 2.1, "style": "kentos-kapi-90", "number": number})
    if build:
        e["build"] = build
    e["place"] = door_place(w, at, width)
    return e


def audit_setup():
    ents = []
    for i, (a, b) in DWALLS.items():
        ents.append({"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": 0.2,
                     "justify": "center", "arch": {"level": "zemin"}})
    ents.append(filler())
    for i in sorted(DOORS):
        ents.append(door_entity(i, DOORS[i]))
    ents.append({"kind": "column", "id": COLUMN, "layerId": "mimari", "attrs": {}, "at": P(7.0, 1.0), "width": 0.4,
                 "depth": 0.4, "arch": {"level": "zemin"}})
    ents.append({"kind": "insert", "id": FIXTURE, "layerId": "mimari", "attrs": {}, "block": BLOCK, "p": P(4.8, 0.8),
                 "scale": 1.0, "rotation": 0.0, "arch": {"level": "zemin"}})
    return {
        "format": "kentos.document", "version": 2, "name": "Kapıların denetimi",
        "uids": [AUID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mimari", "Mimari"), hidden_layer()],
        "activeLayer": "kapi",
        "styles": {"items": [], "categories": []},
        "blocks": [{"id": BLOCK, "name": "Dolap", "base": P(0, 0),
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(0.6, 0.6)}]}],
        "entities": ents,
    }


def rotate(v, deg):
    r = deg * math.pi / 180.0
    return (v[0] * math.cos(r) - v[1] * math.sin(r), v[0] * math.sin(r) + v[1] * math.cos(r))


def door_frame(spec):
    """Its hinge, where it shuts, its length, its use zone's corners, its side's outward unit."""
    w, at, width, side, hand, _, build = spec
    (x0, y), _ = DWALLS[w]
    s0, s1 = at - width / 2, at + width / 2
    fit, fr = 0.0, 0.05
    clear = width - 2 * fr - 2 * fit
    face = y + 0.1 if side == "left" else y - 0.1
    out = (0.0, 1.0) if side == "left" else (0.0, -1.0)
    if hand == "start":
        hinge, closed = (x0 + s0 + fit + fr, face), (1.0, 0.0)
    else:
        hinge, closed = (x0 + s1 - fit - fr, face), (-1.0, 0.0)
    zone = None
    if build and build.get("useZone"):
        a, c = (x0 + s0 + fit + fr, face), (x0 + s1 - fit - fr, face)
        dz = build["useZone"]
        zone = [a, c, (c[0] + out[0] * dz, c[1] + out[1] * dz), (a[0] + out[0] * dz, a[1] + out[1] * dz)]
    return hinge, closed, clear, zone, out


def sweep(spec):
    hinge, closed, length, _, out = door_frame(spec)
    turn = 90.0 if closed[0] * out[1] - closed[1] * out[0] >= 0 else -90.0
    ring = [hinge]
    for k in range(17):
        d = rotate(closed, turn * k / 16)
        ring.append((hinge[0] + d[0] * length, hinge[1] + d[1] * length))
    return ring


def middle(ring):
    n = len(ring)
    return (sum(p[0] for p in ring) / n, sum(p[1] for p in ring) / n)


def convex_overlap(a, b):
    """Two convex rings overlap by more than a millimetre on every separating axis."""
    for poly in (a, b):
        for i in range(len(poly)):
            p, q = poly[i], poly[(i + 1) % len(poly)]
            ax = (-(q[1] - p[1]), q[0] - p[0])
            n = math.hypot(*ax)
            if n == 0:
                continue
            ax = (ax[0] / n, ax[1] / n)
            pa = [x * ax[0] + y * ax[1] for x, y in a]
            pb = [x * ax[0] + y * ax[1] for x, y in b]
            if min(max(pa), max(pb)) - max(min(pa), min(pb)) <= 0.001:
                return False
    return True


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def wall_rect(i):
    (ax, ay), (bx, by) = DWALLS[i]
    if ay == by:
        return rect(ax, ay - 0.1, bx, ay + 0.1)
    return rect(ax - 0.1, ay, ax + 0.1, by)


def obstacles(removed=()):
    out = [(i, "duvar", wall_rect(i)) for i in DWALLS]
    if COLUMN not in removed:
        out.append((COLUMN, "kolon", rect(6.8, 0.8, 7.2, 1.2)))
    out.append((FIXTURE, "donatı", rect(4.8, 0.8, 5.4, 1.4)))
    return out


DATIVE = {"duvar": "duvara", "kolon": "kolona", "donatı": "donatıya"}


def door_finding(code, message, repair, uids, at, nature):
    return {"key": f"{code}:" + ",".join(f"$uidOf:{u}" for u in uids), "check": "doors", "code": code, "message": message,
            "repair": repair, "uids": [f"$uidOf:{u}" for u in uids], "at": P(*at), "nature": nature}


def door_findings(doors, min_clear=None, removed=()):
    out = []
    ids = sorted(doors)
    for n, i in enumerate(ids):
        spec = doors[i]
        mark = spec[5]
        ring = sweep(spec)
        for oid, what, poly in obstacles(removed):
            if oid == spec[0] or not convex_overlap(ring, poly):
                continue
            out.append(door_finding("arch_door_swing_clash",
                                    f"{mark} kapısının kanadı açılırken bir {DATIVE[what]} çarpıyor (fiziksel çarpışma).",
                                    "Kapının açılış yönünü ya da menteşesini çevirin, kanadı daraltın ya da engeli taşıyın.",
                                    [i, oid], middle(ring), "physical"))
        for j in ids[n + 1:]:
            if convex_overlap(ring, sweep(doors[j])):
                out.append(door_finding("arch_door_swing_clash",
                                        f"{mark} ve {doors[j][5]} kapılarının kanatları açılırken birbirine çarpıyor (fiziksel çarpışma).",
                                        "Kapılardan birinin açılış yönünü ya da menteşesini çevirin ya da kapıyı kaydırın.",
                                        [i, j], middle(ring), "physical"))
        zone = door_frame(spec)[3]
        if zone:
            for oid, what, poly in obstacles(removed):
                if oid == spec[0] or not convex_overlap(zone, poly):
                    continue
                out.append(door_finding("arch_door_front_blocked",
                                        f"{mark} kapısının önündeki kullanım alanına bir {what} giriyor (net geçiş).",
                                        "Engeli kullanım alanının dışına alın ya da kapıyı başka yere taşıyın.",
                                        [i, oid], middle(zone), "passage"))
        if min_clear is not None:
            clear = (spec[2] - 0.1) - 0.04
            if clear < min_clear - 1e-9:
                pl = door_place(spec[0], spec[1], spec[2])
                out.append(door_finding("arch_door_clear_narrow",
                                        f"{mark} kapısının net geçişi {clear:.3f} m; proje kuralının en az {min_clear:.3f} m'sinden dar (proje kuralı).",
                                        "Kapıyı genişletin, kasayı inceltin ya da kanadın açılma açısını büyütün.",
                                        [i], middle([(p["x"], p["y"]) for p in pl]), "rule"))
    rank = {"arch_door_swing_clash": 0, "arch_door_front_blocked": 1, "arch_door_clear_narrow": 2}
    out.sort(key=lambda f: (rank[f["code"]], f["at"]["x"], f["at"]["y"]))
    seen = {}
    for f in out:
        seen[f["key"]] = seen.get(f["key"], 0) + 1
        if seen[f["key"]] > 1:
            f["key"] += f"#{seen[f['key']]}"
    return out


def audit_step(inp, findings):
    return {"op": "execute", "input": inp, "within": 1e-9,
            "result": done({"findings": findings, "outside": 0, "excepted": 0, "revision": "$current"}),
            "expect": {"revision": "same"}}


def audit_cases():
    out = []
    doors = dict(DOORS)
    out.append(edit_case("Kapılar: çarpışma, kapı önü ve iki kanadın çarpışması ayrı bulgular",
                         [audit_step({"checks": ["doors"]}, door_findings(doors))]))
    out.append(edit_case("Kapılar yalnız adıyla istenince: bütün gruplar onları saymaz",
                         [audit_step({"checks": ["openings"]}, [])]))
    out.append(edit_case("Proje kuralı: 80 cm net geçişten dar kapı", [audit_step({"checks": ["doors"], "minClearWidth": 0.8},
                                                                               door_findings(doors, 0.8))]))
    out.append(edit_case("Proje kuralı: 90 cm'den bütün kapılar dar", [audit_step({"checks": ["doors"], "minClearWidth": 0.9},
                                                                              door_findings(doors, 0.9))]))
    out.append(edit_case("Ret: eksi net geçiş kuralı", [{"op": "execute", "input": {"checks": ["doors"], "minClearWidth": -1.0},
                         "result": refusal("invalid_arch_audit", "En az net geçiş -1 m; sıfır ya da daha büyük olmalı.", "minClearWidth"),
                         "expect": {"revision": "same"}}]))
    # Açılışı çevir: K1 opens to the other side, clear of the wall.
    flipped = dict(doors)
    flipped[5] = doors[5][:3] + ("right",) + doors[5][4:]
    flip = {"op": "execute", "command": "cad.openings.edit", "input": {"uids": ["$uidOf:5"], "operation": {"kind": "flip", "what": "side"}},
            "result": "$any"}
    out.append(edit_case("Açılışı çevrilen kapı duvara çarpmaz", [flip, audit_step({"checks": ["doors"]}, door_findings(flipped))]))
    # K5 opens away from K4: the two leaves part.
    apart = dict(doors)
    apart[9] = doors[9][:3] + ("right",) + doors[9][4:]
    flip = {"op": "execute", "command": "cad.openings.edit", "input": {"uids": ["$uidOf:9"], "operation": {"kind": "flip", "what": "side"}},
            "result": "$any"}
    out.append(edit_case("Karşılıklı kanatlar ayrılınca çarpışma kalkar", [flip, audit_step({"checks": ["doors"]}, door_findings(apart))]))
    # A use zone given to K3: the column stands in it.
    zoned = dict(doors)
    zoned[7] = doors[7][:6] + ({"useZone": 1.2},)
    setz = {"op": "execute", "command": "cad.openings.edit", "input": {"uids": ["$uidOf:7"], "operation": {"kind": "set", "build": {"useZone": 1.2}}},
            "result": "$any"}
    out.append(edit_case("Kullanım alanı verilen kapının önünde kolon", [setz, audit_step({"checks": ["doors"]}, door_findings(zoned))]))
    # The column gone.
    gone = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [f"$uidOf:{COLUMN}"]}, "result": "$any"}
    out.append(edit_case("Kolon silinince kapının çarpışması kalkar", [gone, audit_step({"checks": ["doors"]},
                                                                                       door_findings(doors, removed=(COLUMN,)))]))
    # The use zone taken away: the fixture is in the leaf's way still, no longer in its use zone.
    bare = dict(doors)
    bare[6] = doors[6][:6] + (None,)
    unz = {"op": "execute", "command": "cad.openings.edit", "input": {"uids": ["$uidOf:6"], "operation": {"kind": "set", "unset": ["useZone"]}},
           "result": "$any"}
    out.append(edit_case("Kullanım alanı kaldırılınca yalnız fiziksel çarpışma kalır", [unz, audit_step({"checks": ["doors"]}, door_findings(bare))]))
    out.append(edit_case("Kapılar ve açıklıklar birlikte, kural verilmeden", [audit_step({"checks": ["openings", "doors"]}, door_findings(doors))]))
    # One door's findings at a time (`within`): those naming it are kept, the others counted outside.
    every = door_findings(doors, 0.8)
    for i in sorted(DOORS):
        kept = [f for f in every if f"$uidOf:{i}" in f["uids"]]
        step = audit_step({"checks": ["doors"], "minClearWidth": 0.8, "within": {"uids": [f"$uidOf:{i}"]}}, kept)
        step["result"]["output"]["outside"] = len(every) - len(kept)
        out.append(edit_case(f"Yalnız {DOORS[i][5]} kapısının bulguları", [step]))
    for k, rule in enumerate((0.5, 0.7, 0.85, 1.0)):
        out.append(edit_case(f"Proje kuralı {rule:.2f} m", [audit_step({"checks": ["doors"], "minClearWidth": rule},
                                                                      door_findings(doors, rule))]))
    step = audit_step({"checks": ["doors"], "within": {"levels": ["zemin"]}}, door_findings(doors))
    out.append(edit_case("Zemin kattaki kapılar", [step]))
    return out


def document(command, title, note, cases, s):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note,
            "setup": s, "cases": cases}


def files():
    note = ("ADR 0772. Beklenenler scripts/fixtures/opening_types_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "duvarlar x boyunca düz, pervazlar (x0 + s, ±0,1). Kurulumda üç duvar, yirmi K01 kapısı (birinin kendi kasası), üç K02 kapısı, "
            "bir P01 penceresi ve KentOS tipleriyle yapılmış altı açıklık var; yeni nesne 35'ten başlar.")
    s = setup()
    return {
        "cad.openingtypes.edit.json": document("cad.openingtypes.edit", "Kapı ve pencere tipleri: tanım, değişiklik ve izleyen açıklıklar, ad, silme, kitaplık; retler",
                                               note, types_edit_cases(), s),
        "cad.openings.query.json": document("cad.openings.query", "Açıklıkları oku: tipleri, sürümleri, istisnaları, katları ve dört genişliği; süzgeçler; retler",
                                            note, query_cases(), s),
        "cad.openings.edit.types.json": document("cad.openings.edit", "Açıklığı düzenle ve tipler: istisna, Türü değiştir, Tipten ayır, Tipe uygula, Tipe döndür, Tipi yeniden uygula",
                                                 note, opening_edit_cases(), s),
        "cad.arch.audit.doors.json": document("cad.arch.audit", "Mimari denetim: kapılar; fiziksel çarpışma, net geçiş ve proje kuralı ayrı",
                                              "ADR 0772 §5. Beklenenler scripts/fixtures/opening_types_cases.py'den: kanadın süpürdüğü alan menteşe ve 17 yay noktası, "
                                              "engellerle dışbükey ayırma ekseni sınaması (1 mm'den derin), KentOS kodu olmadan.", audit_cases(), audit_setup()),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, doc in files().items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{doc['command']} ({name}): {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
