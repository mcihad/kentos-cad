#!/usr/bin/env python3
"""The tag and schedule commands' shared cases (docs/adr/0344 §1–§4), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.tags.{create,check}.json and cad.schedules.{create,edit}.json; `--check` compares and
writes nothing. The walls are straight and along the axes, the spaces rectangles, so every point and value is exact.
The rules, as the ADR gives them:

- an object's values: an opening's number, type (KentOS's type's name, else its style), form, width, height (head −
  sill, with a head), sill (absent 0), head, area (width × height), its wall's style, level and phase; a space's number,
  name, usage, area, perimeter, height (its own, else its level's), volume; a column's tag, place, type, width; a wall's
  style (as type), width (as thickness), length, height, net face area (length × height less its openings with a head),
  volume; a grid's label; a stair's risers; a ramp's slope (given, else rise over run); an attribute by name;
- written: a length to the project's 3 decimals, an area to its 2 (a tag writes “ m²” after it), a volume to 2 (a tag
  “ m³”), a count whole, a slope “%” and its value to 2 decimals without trailing zeros;
- a tag: its template (none: its kind's) filled, each line trimmed, the empty lines at its end dropped; the point it
  ties to an opening's middle, a space's label place (a rectangle's middle), a column's centre, another's middle edge's
  middle; its place, not given, that point for a space, else 4 words' heights (2.5 mm × 1:50) above it; its frame, not
  given, an opening's hexagon, a space's rectangle, else a circle; the document writes its words and point again at
  every step, moving it as far as its object's point went when the step did not move it, breaking it (`targetGone`)
  when its object is gone;
- a schedule: its kind's objects in the drawing's order that its filter takes (an opening's level its wall's), a row
  each, cells without units; sorted (numbers by value, words naturally, empty last), grouped (alike in every column not
  summed: count, area, perimeter, volume, length added up), its heading (a field's title and unit), its total
  (“Toplam” first unless that column is summed; a count's empty sum 0); its look: the heading, numbers right, worked
  out columns' data cells grey (#7f8790), the total bold;
- `cad.schedules.edit`: a data row (1 … rows) and column; its objects' own field (number, type, width, sill, head, name,
  usage, tag, thickness, attribute; a wall's type is its style, not edited), else `cell_computed`; a number on a
  grouped row of more than one object `schedule_group_number`; a length typed in the unit, a point or a comma;
  `set` changes the spec; a locked layer refuses all.
"""

import argparse
import copy
import json
import math
import sys
from functools import cmp_to_key
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_command_cases as acc  # noqa: E402  (refusals, revisions, numbers as Rust writes them)
import point_editor_cases as pe  # noqa: E402  (the natural order)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
CREATE, CHECK, TEDIT, SCREATE, SEDIT = "cad.tags.create", "cad.tags.check", "cad.tags.edit", "cad.schedules.create", "cad.schedules.edit"
P, refusal, num, fixed, layer, uid_of = acc.P, acc.refusal, acc.num, acc.fixed, acc.layer, acc.uid_of
MISSING = acc.MISSING
SCALE = 50
GREY = "#7f8790"
# ADR 0341's words for an opening whose wall is gone (the contract's `OpeningBreak`).
HOST_GONE = "Açıklığın duvarı yok. Açıklığı bir duvara taşıyın (Açıklığı taşı) ya da silin."

LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0}]
LAYERS = {"duvar": "Duvar", "kapi": "Kapı", "mahal": "Mahal", "kolon": "Kolon", "etiket": "Etiket", "cizelge": "Çizelge",
          "cizim": "Çizim", "kilitli": "Kilitli katman", "gizli": "Gizli katman", "sinir": "Sınır"}


def rect(x0, y0, x1, y1):
    return [{"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}]


def opening(host, at, width, y, **kw):
    """An opening on a wall along +x at height `y`, 0.2 wide: its jambs from the start's, the left face's first."""
    a, b = at - width / 2, at + width / 2
    return {"host": uid_of(host), "at": at, "width": width, **kw,
            "place": [P(a, y + 0.1), P(a, y - 0.1), P(b, y + 0.1), P(b, y - 0.1)]}


# The drawing: kind, layer, own fields and attributes of each object. The openings and the first tag name their wall and
# opening by persistent id, which a setup cannot: every case writes them first (`PRELUDE`).
SEED = {
    1: ("wall", "duvar", {"pts": [P(0, 0), P(10, 0)], "width": 0.2, "justify": "center", "style": "D20", "arch": {"level": "zemin"}}, {}),
    2: ("wall", "duvar", {"pts": [P(0, 5), P(10, 5)], "width": 0.2, "justify": "center", "style": "D20", "arch": {"phase": "demolish"}}, {}),
    3: ("space", "mahal", {"parts": rect(20, 0, 24, 3), "number": "101", "name": "Salon", "usage": "Oturma", "arch": {"level": "zemin"}}, {}),
    4: ("space", "mahal", {"parts": rect(25, 0, 28, 2), "number": "102", "name": "Mutfak", "usage": "Islak", "height": 2.4, "arch": {"phase": "new"}}, {}),
    5: ("column", "kolon", {"at": P(40, 0), "width": 0.4, "depth": 0.5, "type": "C30", "tag": "S1"}, {}),
    6: ("column", "kilitli", {"at": P(45, 0), "width": 0.3, "tag": "S2"}, {}),
    7: ("line", "cizim", {"a": P(0, 20), "b": P(5, 20)}, {}),
    8: ("grid", "kolon", {"pts": [P(50, -2), P(50, 8)], "name": "A"}, {}),
    9: ("ramp", "kolon", {"pts": [P(60, 0), P(70, 0)], "width": 1.5, "rise": 0.5}, {}),
    10: ("opening", "kapi", {**opening(1, 9.0, 0.6, 0, form="opening", head=2.1, number="B1"), "host": MISSING, "broken": "hostGone"}, {}),
    11: ("tag", "kilitli", {"of": MISSING, "at": P(80, 0), "text": "X1", "broken": "targetGone"}, {}),
}
SETUP_IDS = sorted(SEED)
KAPI90 = {"form": "door", "frame": 0.05, "head": 2.1, "style": "kentos-kapi-90"}
OPENINGS = [  # (wall, at, width, the wall's y, the type's fields, number, attributes), as Açıklık ekle writes them
    (1, 2.0, 0.9, 0, KAPI90, "K01", {"Malzeme": "Ahşap"}),
    (1, 6.0, 1.2, 0, {"form": "window", "leaves": 2, "frame": 0.06, "sill": 0.9, "head": 2.2, "style": "kentos-pencere-120"}, "P01", {}),
    (2, 3.0, 0.9, 5, KAPI90, "K01", {}),
    (2, 8.0, 0.8, 5, {"form": "door", "frame": 0.05, "head": 2.1, "style": "kentos-kapi-80"}, None, {}),
]
TYPES_OF = {"kentos-kapi-90": "Kapı 90", "kentos-pencere-120": "Pencere 120", "kentos-kapi-80": "Kapı 80"}
FIRST_TAG = {"of": uid_of(12), "at": P(2, 1)}
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
PRELUDE = [
    {"op": "execute", "command": "cad.openings.create", "note": "açıklıklar: kurulum kalıcı kimlik veremez",
     "input": {"layerId": "kapi", "openings": [
         {"host": uid_of(w), "at": at, "type": TYPES_OF[f["style"]], **({"number": n} if n else {}), **({"attrs": a} if a else {})}
         for w, at, _, _, f, n, a in OPENINGS]},
     "result": ANY, "expect": {"revision": "changed"}},
    {"op": "execute", "command": "cad.tags.create", "note": "ilk açıklığın etiketi",
     "input": {"layerId": "etiket", "tags": [FIRST_TAG]}, "result": ANY, "expect": {"revision": "changed"}},
]
NEW = 17
IDS = list(range(1, NEW))
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Etiketler ve çizelgeler",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": SCALE,
                 "workspace": "cad", "arch": {"levels": LEVELS}},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mahal", "Mahal"), layer("kolon", "Kolon"),
               layer("etiket", "Etiket"), layer("cizelge", "Çizelge"), layer("cizim", "Çizim"),
               layer("kilitli", "Kilitli katman", locked=True), layer("gizli", "Gizli katman", visible=False),
               layer("pafta", "Pafta", kind="group", children=[layer("sinir", "Sınır")])],
    "activeLayer": "etiket",
    "styles": {"items": [], "categories": []},
    "entities": [{"kind": k, "id": i, "layerId": ly, "attrs": at, **f} for i, (k, ly, f, at) in sorted(SEED.items())],
}
OPENING_TYPES = TYPES_OF
FORMS = {"door": "Kapı", "window": "Pencere", "opening": "Boşluk"}
PHASES = {"existing": "Mevcut", "demolish": "Yıkılacak", "new": "Yeni", "temporary": "Geçici"}

# ── The drawing as the steps leave it ──────────────────────────────────────────────────────────────────────────


class Drawing:
    def __init__(self):
        self.objs = {i: (k, ly, copy.deepcopy(f), copy.deepcopy(at)) for i, (k, ly, f, at) in SEED.items()}
        for j, (w, at, width, y, f, n, a) in enumerate(OPENINGS):
            g = opening(w, at, width, y, **f)
            if n:
                g["number"] = n
            self.objs[12 + j] = ("opening", "kapi", g, dict(a))
        self.objs[16] = ("tag", "etiket", tag_fields(self, FIRST_TAG["of"], FIRST_TAG), {})
        self.next = NEW
        self.levels = copy.deepcopy(LEVELS)
        self.drawing_unit = "m"
        self.net_areas = {}

    def ids(self):
        return sorted(self.objs)

    def get(self, u):
        """The object a persistent id names: `$uidOf:N` here."""
        if not u.startswith("$uidOf:"):
            return None
        i = int(u[len("$uidOf:"):])
        return (i, self.objs[i]) if i in self.objs else None

    def entity(self, i, extra=None):
        k, ly, f, at = self.objs[i]
        return {"kind": k, "id": i, "layerId": ly, "attrs": at, **(extra or {}), **f}

    def tagged(self):
        return {f["of"] for k, _, f, _ in self.objs.values() if k == "tag"}

    def follow(self, own=(), touched=()):
        """The tags' follower (ADR 0344 §1) after a step that wrote `own` tags and `touched` other objects; then every
        schedule written again. Returns the ids written."""
        out = []
        for i, (k, ly, f, at) in sorted(self.objs.items()):
            if k != "tag":
                continue
            if i not in own and f["of"] not in [uid_of(t) for t in touched]:
                continue
            g = settle_tag(self, f, follow=i not in own)
            if g != f:
                self.objs[i] = (k, ly, g, at)
                out.append(i)
        for i, (k, ly, f, at) in sorted(self.objs.items()):
            if k == "table" and f.get("source", {}).get("kind") == "schedule":
                g = worked_out(self, f)
                if g != f:
                    self.objs[i] = (k, ly, g, at)
                    out.append(i)
        return out


# ── Values (ADR 0344 §1, §3) ───────────────────────────────────────────────────────────────────────────────────


def trim_pct(v):
    t = fixed(v, 2)
    if "." in t:
        t = t.rstrip("0").rstrip(".")
    return "%" + t


def level_name(arch):
    lv = (arch or {}).get("level")
    if lv is None:
        return None
    return next((x["name"] for x in LEVELS if x["id"] == lv), lv)


def level_height(arch):
    lv = (arch or {}).get("level")
    return next((x.get("height") for x in LEVELS if x["id"] == lv), None)


def area_of(parts):
    a = 0.0
    for p in parts:
        pts = p["outer"]["pts"]
        s = 0.0
        for j in range(len(pts)):
            q, r = pts[j], pts[(j + 1) % len(pts)]
            s += q["x"] * r["y"] - r["x"] * q["y"]
        a += abs(s) / 2
    return a


def perimeter_of(parts):
    t = 0.0
    for p in parts:
        pts = p["outer"]["pts"]
        for j in range(len(pts)):
            q, r = pts[j], pts[(j + 1) % len(pts)]
            t += math.hypot(r["x"] - q["x"], r["y"] - q["y"])
    return t


def value(d, i, place, attr=None):
    """(kind, value): `text`, `length`, `area`, `volume`, `count`, `percent`; None: none."""
    k, ly, f, at = d.objs[i]
    if place == "attr":
        v = at.get(attr)
        return ("text", v) if v else None
    if place == "count":
        return ("count", 1)
    text = lambda v: ("text", v) if v else None
    if k == "opening":
        wall = d.get(f["host"])
        wall = wall[1][2] if wall and wall[1][0] == "wall" else None
        h = f["head"] - f.get("sill", 0.0) if f.get("head") is not None else None
        if place in ("number",):
            return text(f.get("number"))
        if place in ("type", "style"):
            return ("text", OPENING_TYPES.get(f["style"], f["style"])) if f.get("style") else None
        if place == "form":
            return ("text", FORMS[f.get("form", "door")])
        if place == "width":
            return ("length", f["width"])
        if place == "height":
            return ("length", h) if h is not None else None
        if place == "sill":
            return ("length", f.get("sill", 0.0))
        if place == "head":
            return ("length", f["head"]) if f.get("head") is not None else None
        if place == "area":
            return ("area", f["width"] * h) if h is not None else None
        if place == "wall":
            return text(wall.get("style")) if wall else None
        if place == "level":
            return text(level_name(wall.get("arch"))) if wall else None
        if place == "phase":
            ph = (wall or {}).get("arch", {}).get("phase")
            return ("text", PHASES[ph]) if ph else None
        return None
    if k == "space":
        if place == "netArea":
            if f.get("broken"):
                return None
            if f.get("method", "net") == "net":
                # Drawn net rings state their own inner boundary; holes are deducted.
                area = area_of(f["parts"]) - sum(area_of([dict(outer=h)]) for p in f["parts"] for h in p.get("holes", []))
                return ("area", area)
            area = d.net_areas.get(i) if "seed" in f else None
            return ("area", float(area)) if area is not None else None
        if place in ("floorElevation", "ceilingElevation"):
            # Exact independent decimal sum and unit ratio. Missing room height
            # must not substitute the level's floor-to-floor height.
            lev = next((v for v in d.levels if v["id"] == f.get("arch", {}).get("level")), None)
            if lev is None or (place == "ceilingElevation" and "height" not in f):
                return None
            metres = Decimal(str(lev["elevation"]))
            if place == "ceilingElevation":
                metres += Decimal(str(f["height"]))
            ratio = dict(m=Decimal(1), cm=Decimal(100), mm=Decimal(1000),
                         **{"in": Decimal(1)/Decimal(".0254"), "ft": Decimal(1)/Decimal(".3048")})[d.drawing_unit]
            return ("text", str((metres*ratio).quantize(Decimal(".001"), rounding=ROUND_HALF_UP)))
        a = area_of(f["parts"]) - sum(area_of([dict(outer=h)]) for p in f["parts"] for h in p.get("holes", []))
        h = f.get("height", level_height(f.get("arch")))
        m = {"number": text(f.get("number")), "name": text(f.get("name")), "usage": text(f.get("usage")),
             "area": ("area", a), "perimeter": ("length", perimeter_of(f["parts"])),
             "height": ("length", h) if h is not None else None,
             "volume": ("volume", a * h) if h is not None else None,
             "level": text(level_name(f.get("arch")))}
        return m.get(place)
    if k == "column":
        m = {"tag": text(f.get("tag")), "place": text(f.get("place")), "type": text(f.get("type")),
             "width": ("length", f["width"]), "level": text(level_name(f.get("arch")))}
        return m.get(place)
    if k == "wall":
        (p, q) = f["pts"][0], f["pts"][-1]
        length = sum(math.hypot(b["x"] - a["x"], b["y"] - a["y"]) for a, b in zip(f["pts"], f["pts"][1:]))
        h = f.get("height", level_height(f.get("arch")))
        me = uid_of(i)
        holes = sum(o[2]["width"] * (o[2]["head"] - o[2].get("sill", 0.0)) for o in d.objs.values()
                    if o[0] == "opening" and o[2]["host"] == me and o[2].get("head") is not None)
        net = length * h - holes if h is not None else None
        m = {"type": text(f.get("style")), "style": text(f.get("style")), "thickness": ("length", f["width"]),
             "width": ("length", f["width"]), "length": ("length", length),
             "height": ("length", h) if h is not None else None,
             "area": ("area", net) if net is not None else None,
             "volume": ("volume", net * f["width"]) if net is not None else None,
             "level": text(level_name(f.get("arch")))}
        return m.get(place)
    if k == "grid":
        return ("text", f["name"]) if place == "label" else (text(level_name(f.get("arch"))) if place == "level" else None)
    if k == "ramp":
        if place == "slope":
            s = f.get("slope")
            if s is None:
                run = math.hypot(f["pts"][1]["x"] - f["pts"][0]["x"], f["pts"][1]["y"] - f["pts"][0]["y"])
                s = f["rise"] / run * 100.0
            return ("percent", s)
        if place == "width":
            return ("length", f["width"])
        return None
    return None


def value_text(v, unit):
    kind, x = v
    if kind == "text":
        return x
    if kind == "length":
        return fixed(x, 3)
    if kind == "area":
        return fixed(x, 2) + (" m²" if unit else "")
    if kind == "volume":
        return fixed(x, 2) + (" m³" if unit else "")
    if kind == "count":
        return str(x)
    return trim_pct(x)


def number_of(v):
    return None if v is None or v[0] == "text" else float(v[1])

# ── Tags (ADR 0344 §1) ─────────────────────────────────────────────────────────────────────────────────────────


PLACES = ["number", "type", "form", "width", "height", "sill", "head", "name", "usage", "area", "perimeter", "level",
          "phase", "tag", "place", "label", "style", "risers", "slope", "floorElevation", "ceilingElevation", "netArea"]
TEMPLATES = {"opening": "{number}", "space": "{number}\n{name}\n{area}", "column": "{tag}", "grid": "{label}",
             "wall": "{style}", "stair": "{risers}", "ramp": "{slope}", "insert": "{attr:NO}"}
SHAPES = {"opening": "hexagon", "space": "rectangle"}


def template_problem(t):
    if len(t) > 200:
        return f"Şablon {len(t)} harf; en çok 200 olur."
    if any(ord(c) < 32 and c != "\n" or ord(c) == 127 for c in t):
        return "Şablonda satır sonundan başka denetim karakteri olmaz."
    if t.count("\n") + 1 > 6:
        return f"Şablon {t.count(chr(10)) + 1} satır; en çok 6 olur."
    rest = t
    while True:
        i = min([j for j in (rest.find("{"), rest.find("}")) if j >= 0], default=-1)
        if i < 0:
            return None
        if rest[i] == "}":
            return "Şablonda açılmamış “}” var; yer adları süslü ayraç içinde yazılır."
        j = rest.find("}", i + 1)
        if j < 0:
            return "Şablonda kapanmamış “{” var; yer adını “}” ile kapatın."
        name = rest[i + 1:j]
        if "{" in name:
            return "Şablonda iç içe süslü ayraç olmaz."
        if name.startswith("attr:"):
            if not name[5:].strip():
                return "“{attr:…}” bir özniteliğin adını ister."
        elif name not in PLACES:
            return f"Şablonda bilinmeyen yer “{{{name}}}”; yerler: {', '.join(PLACES)}, attr:Ad."
        rest = rest[j + 1:]


def fill(d, i, template):
    out, rest = "", template
    while "{" in rest:
        a = rest.index("{")
        b = rest.index("}", a)
        out += rest[:a]
        name = rest[a + 1:b]
        v = value(d, i, "attr", name[5:]) if name.startswith("attr:") else value(d, i, name)
        out += value_text(v, True) if v else ""
        rest = rest[b + 1:]
    out += rest
    # Every empty line dropped (docs/adr/0344's 27 October addendum).
    return "\n".join(ln.strip() for ln in out.split("\n") if ln.strip())


def anchor(d, i):
    k, _, f, _ = d.objs[i]
    if k == "opening":
        p = f["place"]
        return P(sum(q["x"] for q in p) / 4, sum(q["y"] for q in p) / 4)
    if k == "space":
        pts = f["parts"][0]["outer"]["pts"]
        return P((pts[0]["x"] + pts[2]["x"]) / 2, (pts[0]["y"] + pts[2]["y"]) / 2)
    if k == "column":
        return dict(f["at"])
    pts = f["pts"]
    j = (len(pts) - 1) // 2
    return P((pts[j]["x"] + pts[j + 1]["x"]) / 2, (pts[j]["y"] + pts[j + 1]["y"]) / 2)


def settle_tag(d, f, follow):
    g = copy.deepcopy(f)
    found = d.get(f["of"])
    if not found:
        g["broken"] = "targetGone"
        return g
    i = found[0]
    g["text"] = fill(d, i, f.get("content") or TEMPLATES[d.objs[i][0]])
    if not g["text"]:
        g.pop("text")
    a = anchor(d, i)
    if follow and f.get("anchor"):
        g["at"] = P(f["at"]["x"] + (a["x"] - f["anchor"]["x"]), f["at"]["y"] + (a["y"] - f["anchor"]["y"]))
    g["anchor"] = a
    g.pop("broken", None)
    return g


def tag_fields(d, of, n):
    i = d.get(of)[0]
    k = d.objs[i][0]
    a = anchor(d, i)
    size = n.get("size")
    h = (size if size is not None else 2.5) * SCALE / 1000
    at = n.get("at") or (a if k == "space" else P(a["x"], a["y"] + 4 * h))
    t = {"of": of, "at": at}
    if n.get("content") is not None:
        t["content"] = n["content"]
    shape = n.get("shape") or SHAPES.get(k, "circle")
    if shape != "circle":
        t["shape"] = shape
    if size is not None:
        t["size"] = size
    if n.get("leader"):
        t["leader"] = True
    text = fill(d, i, n.get("content") or TEMPLATES[k])
    if text:
        t["text"] = text
    t["anchor"] = a
    return t


def tag_problem(t, path):
    c = t.get("content")
    if c is not None:
        if not c.strip():
            return refusal("invalid_tag", "Boş şablon yazılmaz; alanı kaldırın ya da bir şablon yazın.", f"{path}.content")
        why = template_problem(c)
        if why:
            return refusal("invalid_tag", why, f"{path}.content")
    s = t.get("size")
    if s is not None and not (math.isfinite(s) and 0 < s <= 50):
        return refusal("invalid_tag", f"Etiketin yazı yüksekliği {num(s)}; sıfırdan büyük, en çok 50 mm olmalı.", f"{path}.size")
    return None


def is_visible(ly):
    return ly != "gizli"


def tags_create(d, inp, conflict=False):
    if not inp.get("tags") and inp.get("all") is None:
        return refusal("no_objects", "Etiketlenecek nesne verilmedi. Etiketler (tags) ya da türler (all) verin.", "tags"), None
    made, refreshed = [], []
    done = set(d.tagged())
    explicit = {n["of"] for n in inp.get("tags", [])}
    for i, n in enumerate(inp.get("tags", [])):
        at = f"tags[{i}]"
        if not acc.is_uid(n["of"]):
            return refusal("invalid_uid", f"“{n['of']}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin.", f"{at}.of"), None
        if n.get("at") is not None:
            r = acc.point_not_finite(n["at"], "Etiketin yerinin", f"{at}.at")
            if r:
                return r, None
        found = d.get(n["of"])
        if not found:
            return refusal("entity_not_found", f"“{n['of']}” kimlikli nesne çizimde yok. Var olan bir nesnenin kimliğini verin.", f"{at}.of"), None
        if found[1][0] not in TEMPLATES:
            return refusal("not_taggable", "Bu nesne etiketlenmez: açıklık, mahal, kolon, aks, duvar, merdiven, rampa ya da blok etiketlenir.", f"{at}.of"), None
        r = tag_problem(n, at)
        if r:
            return r, None
        done.add(n["of"])
        made.append(tag_fields(d, n["of"], n))
    al = inp.get("all")
    if al is not None:
        if not al["kinds"]:
            return refusal("no_objects", "Etiketlenecek tür verilmedi; en az bir tür verin.", "all.kinds"), None
        for k, ly in enumerate(al.get("layers") or []):
            if ly not in LAYERS:
                return refusal("layer_not_found", f"“{ly}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", f"all.layers[{k}]"), None
        for index, level in enumerate(al.get("levels") or []):
            if level is not None and not any(row["id"] == level for row in d.levels):
                return refusal("level_not_found", f"Projede “{level}” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.", f"all.levels[{index}]"), None
        r = tag_problem(al, "all")
        if r:
            return r, None
        found = 0
        for i, (k, ly, f, _) in sorted(d.objs.items()):
            if k not in al["kinds"] or not is_visible(ly) or (al.get("layers") is not None and ly not in al["layers"]):
                continue
            # Only the objects of these phases (docs/adr/0346 §2): an opening's its wall's.
            if al.get("phases") is not None and phase_of(d, i) not in al["phases"]:
                continue
            if al.get("levels") is not None:
                source = d.get(f["host"]) if k == "opening" else (i, (k, ly, f, {}))
                # A missing/non-wall host is unknown, not an unassigned floor.
                if source is None or (k == "opening" and source[1][0] != "wall"):
                    continue
                if source[1][2].get("arch", {}).get("level") not in al["levels"]:
                    continue
            if uid_of(i) in done:
                if al.get("refresh") and uid_of(i) not in explicit:
                    for tid, (tk, tl, tf, _) in sorted(d.objs.items()):
                        if tk == "tag" and tl == inp["layerId"] and tf["of"] == uid_of(i):
                            nf = copy.deepcopy(tf)
                            nf.update({key: al[key] for key in ("content", "shape", "size") if al.get(key) is not None})
                            nf = settle_tag(d, nf, follow=False)
                            if nf.get("shape") == "circle": nf.pop("shape")
                            if nf != tf:
                                refreshed.append((tid, nf)); found += 1
                continue
            done.add(uid_of(i))
            made.append(tag_fields(d, uid_of(i), {kk: al[kk] for kk in ("content", "shape", "size") if kk in al}))
            found += 1
        if found == 0 and not inp.get("tags"):
            message = "Verilen kapsamda yeni ya da hedef katmanda değişecek etiket yok." if al.get("refresh") else "Verilen türlerde etiketsiz ve görünen nesne yok."
            return refusal("no_objects", message, "all.kinds"), None
    result, made = finish(inp, conflict, made, refreshed)
    if made is not None and refreshed: result["refreshed"] = refreshed
    return result, made


def phase_of(d, i):
    """An object's phase: its own; an opening's its wall's (docs/adr/0346 §2)."""
    k, _, f, _ = d.objs[i]
    if k == "opening":
        host = f.get("host", "")
        if host.startswith("$uidOf:") and int(host.split(":")[1]) in d.objs:
            return phase_of(d, int(host.split(":")[1]))
        return None
    return (f.get("arch") or {}).get("phase")


def finish(inp, conflict, made, refreshed=()):
    lw = inp.get("lineWeight")
    if lw is not None and not 0 <= lw <= 100:
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None
    r = acc.revision(inp, conflict)
    if r:
        return r, None
    r, warn = layer_check(inp["layerId"])
    if r:
        return r, None
    empty = sum(1 for t in made if t is not None and not t.get("text")) + sum(1 for _, nf in refreshed if not nf.get("text"))
    if empty:
        warn = warn + [{"code": "tag_empty", "message": f"{empty} etiketin yazısı şimdilik boş: nesnelerinin şablondaki değerleri yok (numarasız açıklık gibi). Değerler verilince etiketler kendiliğinden dolar; açıklıklara Açıklıkları numarala ile numara verin."}]
    return {"warnings": warn}, made


def layer_check(layer_id, path="layerId"):
    if layer_id == "pafta":
        return refusal("not_a_layer", "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", path), None
    if layer_id not in LAYERS:
        return refusal("layer_not_found", f"“{layer_id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", path), None
    if layer_id == "kilitli":
        return refusal("layer_locked", "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", path), None
    if layer_id == "gizli":
        return None, [{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": path}]
    return None, []


def extra_of(inp):
    return {k: inp[k] for k in ("color", "lineWeight") if k in inp}


def unchanged(s, d):
    s["expect"] = {"ids": d.ids(), "revision": "same"}
    return s


def create_step(d, op, inp, non_finite=None, conflict=False):
    result, made = tags_create(d, acc.with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "command": CREATE, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if made is None:
        s["result"] = result
        return unchanged(s, d)
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": result["warnings"]}
        return unchanged(s, d)
    ents = [{"kind": "tag", "id": 0, "layerId": inp["layerId"], "attrs": {}, **extra_of(inp), **f} for f in made]
    refreshed = result.get("refreshed", [])
    refresh_entities = [{**{key: value for key, value in d.entity(i).items() if key not in d.objs[i][2]}, **f} for i, f in refreshed]
    if op == "plan":
        output = {"tags": ents, "revision": "$current"}
        if refreshed: output["refreshed"] = refresh_entities
        s["result"] = {"status": "completed", "output": output, "warnings": result["warnings"]}
        s["within"] = 1e-9
        return unchanged(s, d)
    ids = list(range(d.next, d.next + len(made)))
    d.next += len(made)
    for i, f in zip(ids, made):
        d.objs[i] = ("tag", inp["layerId"], f, {})
    for i, f in refreshed:
        k, ly, _, attrs = d.objs[i]
        d.objs[i] = (k, ly, f, attrs)
    d.follow(own=ids + [i for i, _ in refreshed])
    output = {"uids": ["$any"] * len(made), "ids": ids, "revision": "$current"}
    if refreshed: output["refreshed"] = [uid_of(i) for i, _ in refreshed]
    s["result"] = {"status": "completed", "output": output,
                   "warnings": result["warnings"]}
    s["within"] = 1e-9
    s["expect"] = {"ids": d.ids(), "entities": {**{str(i): d.entity(i, extra_of(inp)) for i in ids}, **{str(i): d.entity(i) for i, _ in refreshed}},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def foreign(d, command, inp, writes, moves=(), touched=()):
    """Another command's step (its result not this file's to say); `writes` its changes to the reference drawing."""
    for i, f in writes.items():
        k, ly, _, at = d.objs[i]
        d.objs[i] = (k, ly, f, at)
    for i in moves:
        d.objs.pop(i, None)
    written = d.follow(own=[i for i in writes if d.objs.get(i, ("",))[0] == "tag"], touched=list(writes) + list(moves) + list(touched))
    ents = {str(i): d.entity(i) for i in sorted(set(written) | set(writes)) if i in d.objs}
    # A deleted object's id cannot be looked up after the step: taken before it.
    text = json.dumps(ents, ensure_ascii=False)
    for i in moves:
        text = text.replace(f'"$uidOf:{i}"', f'"$uid:gone{i}"')
    steps = [{"op": "captureUid", "as": f"gone{i}", "id": i} for i in moves]
    return steps + [{"op": "execute", "command": command, "input": inp, "result": {"status": "completed", "output": "$any", "warnings": "$any"},
                     "within": 1e-9, "expect": {"ids": d.ids(), "entities": json.loads(text), "revision": "changed"}}]


def moved(d, i, dx, dy):
    k, ly, f, at = d.objs[i]
    g = copy.deepcopy(f)
    for key in ("at",):
        if key in g:
            g[key] = P(g[key]["x"] + dx, g[key]["y"] + dy)
    if "parts" in g:
        for part in g["parts"]:
            part["outer"]["pts"] = [P(p["x"] + dx, p["y"] + dy) for p in part["outer"]["pts"]]
    if "pts" in g:
        g["pts"] = [P(p["x"] + dx, p["y"] + dy) for p in g["pts"]]
    return g


def move(d, ids, dx, dy):
    inp = {"uids": [uid_of(i) for i in ids], "transform": {"kind": "move", "dx": dx, "dy": dy}}
    return foreign(d, "cad.entities.transform", inp, {i: moved(d, i, dx, dy) for i in ids})


class Cases(acc.Cases):
    def add(self, name, steps):
        super().add(name, copy.deepcopy(PRELUDE) + steps)


def tag_create_cases():
    c = Cases()
    one = {"layerId": "etiket", "tags": [{"of": uid_of(13)}]}
    c.add("Pencereye etiket: doğrula yazmaz", [create_step(Drawing(), "validate", one)])
    c.add("Pencereye etiket: plan yazmadan gösterir", [create_step(Drawing(), "plan", one)])
    d = Drawing()
    c.add("Pencereye etiket: yazılır ve geri alınır", [create_step(d, "execute", one),
                                                      {"op": "undo", "returns": "Etiket", "expect": {"ids": IDS, "canUndo": True, "canRedo": True, "revision": "changed"}}])
    c.add("Pencereye etiket: gizli katmana uyarıyla yazılır", [create_step(Drawing(), "execute", {**one, "layerId": "gizli"})])
    for name, inp in [
        ("mahal etiketi: numara, ad ve alan üç satır, mahalin ortasında, dikdörtgen", {"layerId": "etiket", "tags": [{"of": uid_of(3)}]}),
        ("kolon etiketi: etiketi, dairede, 4 yazı yüksekliği üstte", {"layerId": "etiket", "tags": [{"of": uid_of(5)}]}),
        ("aks etiketi: etiketi, aksın ortasının üstünde", {"layerId": "etiket", "tags": [{"of": uid_of(8)}]}),
        ("rampa etiketi: yükseklik bölü uzunluktan eğim (%5)", {"layerId": "etiket", "tags": [{"of": uid_of(9)}]}),
        ("duvar etiketi: stili", {"layerId": "etiket", "tags": [{"of": uid_of(1)}]}),
        ("şablonla: tür ve genişlik", {"layerId": "etiket", "tags": [{"of": uid_of(13), "content": "{type} {width}"}]}),
        ("şablonla: numara ve öznitelik iki satır", {"layerId": "etiket", "tags": [{"of": uid_of(12), "content": "{number}\n{attr:Malzeme}"}]}),
        ("türünde olmayan yer boş yazılır, çerçeve kalır", {"layerId": "etiket", "tags": [{"of": uid_of(13), "content": "{name}"}]}),
        ("verilen yer, kol, yuvarlak köşeli çerçeve ve 3.5 mm yazı", {"layerId": "etiket", "tags": [{"of": uid_of(14), "at": P(3, 7), "leader": True, "shape": "rounded", "size": 3.5}]}),
        ("mahalin alanı, hacmi ve kat yüksekliği", {"layerId": "etiket", "tags": [{"of": uid_of(3), "content": "{area}\n{volume}\n{height}\n{level}"}]}),
        ("mahalin kendi yüksekliği katın yerine", {"layerId": "etiket", "tags": [{"of": uid_of(4), "content": "{height} {volume}"}]}),
        ("pencerenin denizliği, lentosu, yüksekliği ve alanı", {"layerId": "etiket", "tags": [{"of": uid_of(13), "content": "{sill}/{head}\n{height} {area}\n{form} {level}"}]}),
        ("bütün mahaller etiketlenir", {"layerId": "etiket", "all": {"kinds": ["space"]}}),
        ("bütün açıklıklar: etiketli olan atlanır", {"layerId": "etiket", "all": {"kinds": ["opening"]}}),
        ("bütün kolonlar, yalnız Kolon katmanında", {"layerId": "etiket", "all": {"kinds": ["column"], "layers": ["kolon"]}}),
        ("yalnız yıkılacak duvarın açıklıkları (evre süzgeci)", {"layerId": "etiket", "all": {"kinds": ["opening"], "phases": ["demolish"]}}),
        ("yalnız yeni mahaller (evre süzgeci)", {"layerId": "etiket", "all": {"kinds": ["space"], "phases": ["new"]}}),
        ("etiketler ve türler birlikte: aynı nesne iki kez etiketlenmez", {"layerId": "etiket", "tags": [{"of": uid_of(13)}], "all": {"kinds": ["opening"], "shape": "circle"}}),
        ("renkli ve kalın etiket", {"layerId": "etiket", "tags": [{"of": uid_of(5)}], "color": "#cc0000", "lineWeight": 0.35}),
    ]:
        c.add(name, [create_step(Drawing(), "execute", inp)])
    for name, inp, kw in [
        ("etiket verilmezse reddedilir", {"layerId": "etiket", "tags": []}, {}),
        ("geçersiz kimlik reddedilir", {"layerId": "etiket", "tags": [{"of": "kapi"}]}, {}),
        ("olmayan nesne reddedilir", {"layerId": "etiket", "tags": [{"of": MISSING}]}, {}),
        ("çizgi etiketlenmez", {"layerId": "etiket", "tags": [{"of": uid_of(7)}]}, {}),
        ("bilinmeyen yerli şablon reddedilir", {"layerId": "etiket", "tags": [{"of": uid_of(13), "content": "{numara}"}]}, {}),
        ("kapanmamış süslü ayraç reddedilir", {"layerId": "etiket", "tags": [{"of": uid_of(13), "content": "{number"}]}, {}),
        ("yedi satırlı şablon reddedilir", {"layerId": "etiket", "tags": [{"of": uid_of(13), "content": "a\nb\nc\nd\ne\nf\ng"}]}, {}),
        ("60 mm yazı reddedilir", {"layerId": "etiket", "tags": [{"of": uid_of(13), "size": 60.0}]}, {}),
        ("sonsuz yer reddedilir", {"layerId": "etiket", "tags": [{"of": uid_of(13), "at": P(1, 1)}]}, {"non_finite": {"tags[0].at.x": "NaN"}}),
        ("türsüz toplu etiket reddedilir", {"layerId": "etiket", "all": {"kinds": []}}, {}),
        ("etiketsiz nesnesi olmayan tür reddedilir", {"layerId": "etiket", "all": {"kinds": ["stair"]}}, {}),
        ("toplu etiketin olmayan katmanı reddedilir", {"layerId": "etiket", "all": {"kinds": ["space"], "layers": ["yok"]}}, {}),
        ("evresi geçici olan mahal yoksa reddedilir", {"layerId": "etiket", "all": {"kinds": ["space"], "phases": ["temporary"]}}, {}),
        ("olmayan katman reddedilir", {**one, "layerId": "yok"}, {}),
        ("katman grubu reddedilir", {**one, "layerId": "pafta"}, {}),
        ("kilitli katman reddedilir", {**one, "layerId": "kilitli"}, {}),
        ("geçersiz çizgi kalınlığı reddedilir", {**one, "lineWeight": 120.0}, {}),
        ("geçersiz beklenen sürüm reddedilir", {**one, "expectedRevision": "07"}, {}),
    ]:
        c.add(name, [create_step(Drawing(), "execute", inp, **kw)])
    d = Drawing()
    made = create_step(d, "execute", one)
    stale = create_step(d, "execute", {**one, "expectedRevision": "$prepared"}, conflict=True)
    c.add("sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    # The document's follower.
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(5)}]})
    c.add("kolon taşınınca etiketi de aynı kadar taşınır", [made, *move(d, [5], 2.0, 1.0)])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(5)}]})
    k = d.next - 1
    c.add("kolon ve etiketi birlikte taşınınca etiket bir kez taşınır", [made, *move(d, [5, k], 2.0, 0.0)])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(5)}]})
    k = d.next - 1
    c.add("etiket yalnız taşınınca yeri kalır, bağ noktası nesnede", [made, *move(d, [k], 1.0, 1.0)])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(3)}]})
    c.add("mahal taşınınca etiketi izler", [made, *move(d, [3], 0.0, 4.0)])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(5)}]})
    gone = foreign(d, "cad.entities.delete", {"uids": [uid_of(5)]}, {}, moves=[5])
    c.add("kolon silinince etiketi son yazısıyla kopar, geri alınca bağı döner",
          [made, *gone, {"op": "undo", "returns": "Sil", "expect": {"ids": [*IDS, NEW], "entities": {str(NEW): Drawing_after(made)}, "revision": "changed"}}])
    return c.cases



def refresh_tag_cases():
    """Independent keep/refresh matrix, preserving placement and separate tag layers."""
    c = Cases()
    def seed(d):
        # Two deliberately different labels of one room on the target layer,
        # and a separate label of the other room on another layer.
        return create_step(d, "execute", {"layerId":"etiket", "tags":[
            {"of":uid_of(3), "at":P(-4,8), "content":"{name}", "size":4, "leader":True, "shape":"rounded"},
            {"of":uid_of(3), "at":P(-4,10), "content":"{area}", "size":2, "shape":"none"}]})
    for policy in [MISSING, None, False, True]:
        for fields in [{}, {"content":"{number} / {name}"}, {"shape":"hexagon"}, {"size":3.5},
                       {"content":"{number}\n{usage}", "size":3.5, "shape":"circle"}]:
            d=Drawing(); pre=seed(d); before={str(i):d.entity(i) for i in d.ids()}; before_ids=d.ids()
            al={"kinds":["space"], **fields}
            if policy is not MISSING: al["refresh"]=policy
            inp={"layerId":"etiket", "all":al}
            steps=[pre, create_step(d,"validate",inp), create_step(d,"plan",inp),create_step(d,"execute",inp)]
            after={str(i):d.entity(i) for i in d.ids()}; after_ids=d.ids()
            steps += [{"op":"undo","returns":"Etiket","expect":{"ids":before_ids,"entities":before,"revision":"changed","canRedo":True}},
                      {"op":"redo","returns":"Etiket","expect":{"ids":after_ids,"entities":after,"revision":"changed","canRedo":False}},
                      create_step(d,"execute",inp)]
            c.add(f"Koru/Yenile {policy}/{fields}: two moved labels, new room, one undo and empty repeat",steps)
    for extra in [{"layers":[]},{"layers":["kapi"]},{"phases":["new"]},{"phases":["demolish"]}]:
        d=Drawing(); pre=seed(d)
        c.add(f"Yenile kaynak kapsamı {extra}",[pre,create_step(d,"plan",{"layerId":"etiket","all":{"kinds":["space"],"refresh":True,"content":"{usage}",**extra}})])
    d=Drawing(); pre=seed(d)
    c.add("Yenile yalnız hedef etiket katmanı; başka katmandaki iki etiket korunur",[pre,
        create_step(d,"plan",{"layerId":"cizim","all":{"kinds":["space"],"refresh":True,"content":"{usage}"}}),
        create_step(d,"execute",{"layerId":"cizim","all":{"kinds":["space"],"refresh":True,"content":"{usage}"}})])
    for fields in [{"content":"{bad}"},{"size":0},{"size":51}]:
        d=Drawing();pre=seed(d)
        c.add(f"Yenile alan atomik ret {fields}",[pre,create_step(d,"execute",{"layerId":"etiket","all":{"kinds":["space"],"refresh":True,**fields}})])
    d=Drawing();pre=seed(d)
    c.add("Yenile kilitli hedefte yeni etiket de yazmaz",[pre,create_step(d,"execute",{"layerId":"kilitli","all":{"kinds":["space"],"refresh":True,"content":"{usage}"}})])
    d=Drawing();pre=seed(d)
    c.add("Açık tags kaynağı aynı çağrıda yenilenmez",[pre,create_step(d,"execute",{"layerId":"etiket","tags":[{"of":uid_of(3),"content":"{name}"}],"all":{"kinds":["space"],"refresh":True,"content":"{usage}"}})])
    d=Drawing();pre=seed(d)
    c.add("Yenile eski revision tüm değişikliği reddeder",[{"op":"captureRevision","as":"old"},pre,
        create_step(d,"execute",{"layerId":"etiket","expectedRevision":"$old","all":{"kinds":["space"],"refresh":True,"content":"{usage}"}},conflict=True)])
    return c.cases


def level_tag_cases():
    """Independent source-floor matrix; legacy all-floor cases remain byte-for-byte."""
    c = Cases()
    for refresh in [False, True]:
        for levels in [MISSING, None, [], [None], ["zemin"], ["kat-1"], ["zemin", None], ["zemin", "zemin", None]]:
            d = Drawing()
            pre = create_step(d, "execute", {"layerId":"etiket", "tags":[
                {"of":uid_of(3), "at":P(-4,8), "content":"{name}", "size":4, "leader":True}]})
            before = {str(i):d.entity(i) for i in d.ids()}; ids = d.ids()
            al = {"kinds":list(TEMPLATES), "refresh":refresh, "content":"{level}"}
            if levels is not MISSING: al["levels"] = levels
            inp = {"layerId":"etiket", "all":al}
            steps = [pre, create_step(d,"validate",inp), create_step(d,"plan",inp), create_step(d,"execute",inp)]
            if len(d.ids()) != len(ids) or before != {str(i):d.entity(i) for i in d.ids()}:
                after = {str(i):d.entity(i) for i in d.ids()}; after_ids = d.ids()
                steps += [{"op":"undo","returns":"Etiket","expect":{"ids":ids,"entities":before,"revision":"changed","canRedo":True}},
                          {"op":"redo","returns":"Etiket","expect":{"ids":after_ids,"entities":after,"revision":"changed","canRedo":False}}]
            c.add(f"Kat kapsamı {levels}, yenile={refresh}: miras/önizleme/tek undo",steps)
    for levels in [["missing"], [None,"missing"], ["zemin","missing"], [""]]:
        d=Drawing();inp={"layerId":"etiket","tags":[{"of":uid_of(4)}],"all":{"kinds":["space"],"levels":levels}}
        c.add(f"Yanlış kat açık tags ile de atomik ret {levels}",[create_step(d,op,inp) for op in ["validate","plan","execute"]])
    for extra in [{"layers":["kapi"]},{"layers":["mahal"]},{"phases":["new"]},{"phases":["demolish"]}]:
        d=Drawing();inp={"layerId":"etiket","all":{"kinds":list(TEMPLATES),"levels":["zemin"],**extra}}
        c.add(f"Kat diğer kaynak süzgeçleriyle kesişir {extra}",[create_step(d,"plan",inp),create_step(d,"execute",inp)])
    for levels in [[],["kat-1"]]:
        d=Drawing();inp={"layerId":"etiket","tags":[{"of":uid_of(4)}],"all":{"kinds":["space"],"levels":levels}}
        c.add(f"Açık tags kat süzgecinden bağımsız {levels}",[create_step(d,"plan",inp),create_step(d,"execute",inp)])
    return c.cases


def elevation_tag_cases():
    """Thirty independent cases: five units × six known/unknown elevation states."""
    cases = []
    for unit in ["m", "mm", "cm", "in", "ft"]:
        for state, elevation, height, level in [
            ("zero", "0", "2.75", "zemin"),
            ("basement", "-3.125", "2.625", "zemin"),
            ("upper", "12.375", "3.125", "zemin"),
            ("no-room-height", "3", None, "zemin"),
            ("unassigned", "0", "2.75", None),
            ("unknown-level", "0", "2.75", "missing"),
        ]:
            d = Drawing(); setup = copy.deepcopy(SETUP)
            d.drawing_unit = unit; d.levels[0]["elevation"] = float(elevation)
            setup["settings"]["drawingUnit"] = unit
            setup["settings"]["arch"]["levels"] = copy.deepcopy(d.levels)
            room = d.objs[3][2]
            if level: room["arch"] = {"level": level}
            else: room.pop("arch", None)
            if height is not None: room["height"] = float(height)
            else: room.pop("height", None)
            setup["entities"][2] = d.entity(3)
            inp = {"layerId": "etiket", "tags": [{"of": uid_of(3), "at": P(-5, 8), "leader": True,
                "content": "{number} {name}\n{area}\nZ {floorElevation}\nT {ceilingElevation}"}]}
            before = {str(i): d.entity(i) for i in d.ids()}
            steps = [create_step(d, "validate", inp), create_step(d, "plan", inp), create_step(d, "execute", inp)]
            after = {str(i): d.entity(i) for i in d.ids()}
            steps += [{"op": "undo", "returns": "Etiket", "expect": {"ids": IDS, "entities": before, "revision": "changed", "canRedo": True}},
                      {"op": "redo", "returns": "Etiket", "expect": {"ids": [*IDS, NEW], "entities": after, "revision": "changed", "canRedo": False}}]
            cases.append({"name": f"Mahal kotu {unit}/{state}: exact values, preview and one undo", "setup": setup,
                          "steps": copy.deepcopy(PRELUDE) + steps})
    return cases


def net_area_tag_cases():
    """24 independent closed-rectangle calculations + six drawn/unknown states."""
    cases = []
    def record(name, d, setup):
        inp = {"layerId": "etiket", "tags": [{"of": uid_of(18), "at": P(-5, 8), "leader": True,
               "content": "{number} {name}\nAlan {area}\nNet {netArea} / {netArea}"}]}
        ids = d.ids(); before = {str(i):d.entity(i) for i in ids}
        steps = [create_step(d, "validate", inp), create_step(d, "plan", inp), create_step(d, "execute", inp)]
        after = {str(i):d.entity(i) for i in d.ids()}
        steps += [{"op":"undo","returns":"Etiket","within":1e-9,"expect":{"ids":ids,"entities":before,"revision":"changed","canRedo":True}},
                  {"op":"redo","returns":"Etiket","within":1e-9,"expect":{"ids":d.ids(),"entities":after,"revision":"changed","canRedo":False}}]
        cases.append({"name":name,"setup":setup,"steps":steps})
    for method in ["net","centerline","gross"]:
        for width in [".1",".2",".4",".8"]:
            for reverse in [False,True]:
                d=Drawing();setup=copy.deepcopy(SETUP)
                ox,oy=(Decimal("1000030"),Decimal("2000000")) if reverse else (Decimal(30),Decimal(0))
                def rectangle(inset):
                    return rect(float(ox+inset),float(oy+inset),float(ox+6-inset),float(oy+4-inset))
                points=rectangle(Decimal(0))[0]["outer"]["pts"]
                if reverse: points.reverse()
                d.objs[17]=("wall","duvar",dict(pts=points,closed=True,width=float(width),justify="center",arch=dict(level="zemin")),{})
                inset={"net":Decimal(width)/2,"centerline":Decimal(0),"gross":-Decimal(width)/2}[method]
                room=dict(parts=rectangle(inset),seed=P(float(ox+3),float(oy+2)),number="N-001",name="Net test",arch=dict(level="zemin"))
                if method!="net":room["method"]=method
                d.objs[18]=("space","mahal",room,{})
                d.net_areas[18]=(6-Decimal(width))*(4-Decimal(width));d.next=19
                d.objs={i:d.objs[i] for i in [17,18]}
                setup["entities"] = [d.entity(17),d.entity(18)]
                record(f"Net alan {method}/{width}/reverse={reverse}: iç yüz, preview, tek undo",d,setup)
    for state in ["manual-net","manual-centerline","manual-gross","broken-net","broken-centerline","broken-gross"]:
        d=Drawing();setup=copy.deepcopy(SETUP)
        method=state.split('-',1)[1]
        parts=rect(30,0,36,4)+rect(40,0,42,2)
        parts[1]["holes"]=[rect(40.25,.25,41.25,1.25)[0]["outer"]]
        room=dict(parts=parts,number="N-001",name="Çizilmiş sınır")
        if method!="net":room['method']=method
        if state.startswith('broken'):room['broken']='lostEnclosure'
        d.objs[17]=("line","cizim",dict(a=P(30,0),b=P(31,0)),{})
        d.objs[18]=("space","mahal",room,{});d.next=19
        d.objs={i:d.objs[i] for i in [17,18]}
        setup["entities"] = [d.entity(17),d.entity(18)]
        record(f"Net alan {state}: delik, çok parça ve bilinmeyen net ayrımı",d,setup)
    return cases


def Drawing_after(step):
    return step["expect"]["entities"][str(step["result"]["output"]["ids"][0])]

# ── Etiketleri denetle (ADR 0344 §2) ───────────────────────────────────────────────────────────────────────────


def findings(d, layers=None, missing_space_numbers=False):
    takes = lambda ly: layers is None or ly in layers
    tags = d.tagged()
    dup, order = {}, []
    no_type, off, gone, untagged = [], [], [], []
    for i, (k, ly, f, _) in sorted(d.objs.items()):
        if not takes(ly):
            continue
        if k in ("opening", "space") and f.get("number"):
            key = (k, f["number"])
            if key not in dup:
                dup[key] = []
                order.append(key)
            dup[key].append(i)
        if k == "opening":
            if not f.get("style"):
                no_type.append(i)
            if f.get("broken"):
                off.append(i)
        if k == "tag" and not d.get(f["of"]):
            gone.append(i)
        if k in ("opening", "space") and uid_of(i) not in tags:
            untagged.append(i)
    out = []

    def at(i):
        k, _, f, _ = d.objs[i]
        return dict(f["at"]) if k == "tag" else anchor(d, i)

    def push(code, message, ids):
        out.append({"code": code, "message": message, "uids": [uid_of(i) for i in ids], "at": at(ids[0])})
    for key in order:
        ids = dup[key]
        if len(ids) > 1:
            push("tag_duplicate_number", f"“{key[1]}” numarası {len(ids)} {'açıklıkta' if key[0] == 'opening' else 'mahalde'}.", ids)
    if missing_space_numbers:
        for i,(kind,ly,f,_) in sorted(d.objs.items()):
            if kind != 'space' or not takes(ly) or (f.get('number')or'').strip():continue
            level=next((v['name'] for v in d.levels if v['id']==f.get('arch',{}).get('level')),f.get('arch',{}).get('level','Kat atanmamış'))
            push('tag_missing_space_number',f"“{f.get('name','Adsız')}” mahalının numarası yok ({level}).",[i])
    for i in no_type:
        n = d.objs[i][2].get("number")
        push("tag_missing_type", f"“{n}” açıklığının türü yok." if n else "Numarasız açıklığın türü yok.", [i])
    for i in off:
        push("tag_host_broken", HOST_GONE, [i])
    for i in gone:
        t = d.objs[i][2].get("text")
        push("tag_target_gone", f"“{t.replace(chr(10), ' ')}” etiketinin nesnesi çizimde yok." if t else "Etiketin nesnesi çizimde yok.", [i])
    for i in untagged:
        k, _, f, _ = d.objs[i]
        what = "açıklık" if k == "opening" else "mahal"
        n = f.get("number")
        push("tag_untagged", f"“{n}” {what} etiketsiz." if n else f"Numarasız {what} etiketsiz.", [i])
    return out


def check_step(d, op, inp):
    s = {"op": op, "command": CHECK, "input": inp}
    for k, ly in enumerate(inp.get("layers") or []):
        if ly not in LAYERS:
            s["result"] = refusal("layer_not_found", f"“{ly}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", f"layers[{k}]")
            return unchanged(s, d)
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
    else:
        s["result"] = {"status": "completed", "output": {"findings": findings(d, inp.get("layers"),inp.get('missingSpaceNumbers') is True), "revision": "$current"}, "warnings": []}
    return unchanged(s, d)


def check_cases():
    c = Cases()
    c.add("bütün çizim: yinelenen numara, türsüz, duvarsız, nesnesiz etiket, etiketsizler", [check_step(Drawing(), "execute", {})])
    c.add("doğrula yazmaz ve bulgu vermez", [check_step(Drawing(), "validate", {})])
    c.add("plan aynı bulguları verir, yazmaz", [check_step(Drawing(), "plan", {})])
    for name, layers in [
        ("yalnız Mahal katmanı: iki etiketsiz mahal", ["mahal"]),
        ("yalnız Kapı katmanı: açıklıkların bulguları", ["kapi"]),
        ("yalnız Kilitli katman: nesnesiz etiket", ["kilitli"]),
        ("Kapı ve Mahal katmanları birlikte", ["kapi", "mahal"]),
        ("bulgusu olmayan katman boş liste verir", ["cizim"]),
    ]:
        c.add(name, [check_step(Drawing(), "execute", {"layers": layers})])
    c.add("olmayan katman reddedilir", [check_step(Drawing(), "execute", {"layers": ["yok"]})])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "all": {"kinds": ["space"]}})
    c.add("mahaller etiketlenince etiketsiz mahal kalmaz", [made, check_step(d, "execute", {})])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "all": {"kinds": ["opening", "space"]}})
    c.add("hepsi etiketlenince yalnız yinelenen, türsüz ve kopuklar kalır", [made, check_step(d, "execute", {})])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(5)}]})
    gone = foreign(d, "cad.entities.delete", {"uids": [uid_of(5)]}, {}, moves=[5])
    c.add("kolon silinince etiketi nesnesiz bulunur", [made, *gone, check_step(d, "execute", {"layers": ["etiket"]})])
    d = Drawing()
    gone = foreign(d, "cad.entities.delete", {"uids": [uid_of(14)]}, {}, moves=[14])
    c.add("yinelenen açıklık silinince numara bulgusu kalkar", [*gone, check_step(d, "execute", {"layers": ["kapi"]})])
    d = Drawing()
    gone = foreign(d, "cad.entities.delete", {"uids": [uid_of(12)]}, {}, moves=[12])
    c.add("etiketli açıklık silinince etiketi nesnesiz kalır", [*gone, check_step(d, "execute", {"layers": ["etiket", "kapi"]})])
    d = Drawing()
    c.add("Etiket katmanında yalnız ilk etiket: bulgusu yok", [check_step(d, "execute", {"layers": ["etiket"]})])
    d = Drawing()
    ed = sedit_cell_step(d, "execute", created(d), row=4, column=0, value="K02")
    c.add("çizelgede numara düzeltilince yinelenen kalkar", [ed["made"], ed["step"], check_step(d, "execute", {"layers": ["kapi"]})])
    d = Drawing()
    ed = sedit_cell_step(d, "execute", created(d), row=1, column=2, value="Kapı 80")
    c.add("çizelgede tür yazılınca türsüz bulgusu kalkar", [ed["made"], ed["step"], check_step(d, "execute", {"layers": ["kapi"]})])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(13)}]})
    c.add("tek pencere etiketlenince etiketsizlerden çıkar", [made, check_step(d, "execute", {"layers": ["kapi", "etiket"]})])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(3)}]})
    c.add("tek mahal etiketlenince öbürü etiketsiz kalır", [made, check_step(d, "execute", {"layers": ["mahal"]})])
    d = Drawing()
    c.add("iki kez sorulunca aynı cevap", [check_step(d, "execute", {}), check_step(d, "execute", {})])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(5)}]})
    c.add("kolon etiketi bulgu değiştirmez", [made, check_step(d, "execute", {})])
    return c.cases

# ── Etiketi düzenle ────────────────────────────────────────────────────────────────────────────────────────────


def missing_space_number_cases():
    """An explicit opt-in extends the old report without changing its default output."""
    cases=[]
    for layers in [None,[],['mahal'],['gizli'],['kilitli'],['mahal','gizli','kilitli'],['etiket'],['yok']]:
        d=Drawing();d.objs={};d.next=10;setup=copy.deepcopy(SETUP)
        rows=[(None,'mahal','zemin','Ana giriş'),(None,'mahal',None,'Çalışma'),
              (None,'gizli','kat-1','Depo'),('Z-001','mahal','zemin','Yatak'),
              ('Z-001','kilitli','kat-1','Arşiv'),('Z-1','mahal','zemin','Mutfak'),
              ('Z-001 ','mahal','zemin','Balkon'),('z-001','mahal','zemin','Teras'),
              ('000000000000000000000000000001','mahal','zemin','Uzun numara')]
        for i,(number,ly,level,name) in enumerate(rows,1):
            f=dict(parts=rect(5*i,0,5*i+4,3),name=name)
            if number is not None:f['number']=number
            if level is not None:f['arch']=dict(level=level)
            d.objs[i]=('space',ly,f,{})
        setup['entities']=[d.entity(i)for i in d.ids()]
        made=create_step(d,'execute',dict(layerId='etiket',tags=[dict(of=uid_of(i))for i in range(1,10)]))
        base={}if layers is None else dict(layers=layers)
        steps=[made]
        for enabled in ['absent',None,False,True]:
            inp=dict(base)
            if enabled!='absent':inp['missingSpaceNumbers']=enabled
            steps.extend(check_step(d,op,inp)for op in ['validate','plan','execute'])
        cases.append(dict(name=f'Numarasız mahal opt-in, etiketli/gizli/kilitli/tam metin; layers={layers}',setup=setup,steps=steps))
    return cases


def tedit(d, inp, conflict=False):
    uids = inp["uids"]
    if not uids:
        return refusal("no_entities", "Düzenlenecek etiket verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"), None
    for i, u in enumerate(uids):
        if not acc.is_uid(u):
            return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", f"uids[{i}]"), None
    found = []
    for i, u in enumerate(uids):
        f = d.get(u)
        if not f:
            return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", f"uids[{i}]"), None
        found.append((i, f[0]))
    for at, j in found:
        if d.objs[j][0] != "tag":
            return refusal("not_a_tag", "Nesne etiket değil; Etiketi düzenle yalnız etiketleri değiştirir.", f"uids[{at}]"), None
    c = inp["change"]
    if not c:
        return refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi.", "change"), None
    if c.get("size") is not None and not math.isfinite(c["size"]):
        return acc.not_finite("Yazı yüksekliği", "change.size"), None
    work = {}
    for at, j in found:
        f = copy.deepcopy(d.objs[j][2])
        for key in ("content", "size"):
            if key in c:
                if c[key] is None:
                    f.pop(key, None)
                else:
                    f[key] = c[key]
        if "shape" in c:
            if c["shape"] == "circle":
                f.pop("shape", None)
            else:
                f["shape"] = c["shape"]
        if "leader" in c:
            if c["leader"]:
                f["leader"] = True
            else:
                f.pop("leader", None)
        r = tag_problem(f, "change")
        if r:
            return r, None
        work[j] = settle_tag(d, f, follow=False)
    r = acc.revision(inp, conflict)
    if r:
        return r, None
    for at, j in found:
        if d.objs[j][1] == "kilitli":
            return refusal("layer_locked", "“Kilitli katman” katmanı kilitli; üzerindeki etiket düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{at}]"), None
    return {"warnings": []}, work


def tedit_step(d, op, inp, non_finite=None, conflict=False):
    result, work = tedit(d, acc.with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "command": TEDIT, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if work is None:
        s["result"] = result
        return unchanged(s, d)
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        return unchanged(s, d)
    ents = [{"kind": "tag", "id": j, "layerId": d.objs[j][1], "attrs": {}, **f} for j, f in work.items()]
    s["result"] = {"status": "completed", "output": {"tags": ents, "revision": "$current"}, "warnings": []}
    s["within"] = 1e-9
    if op == "plan":
        return unchanged(s, d)
    if all(f == d.objs[j][2] for j, f in work.items()):
        # Nothing changes: no step is recorded.
        return unchanged(s, d)
    for j, f in work.items():
        k, ly, _, at = d.objs[j]
        d.objs[j] = (k, ly, f, at)
    s["expect"] = {"ids": d.ids(), "entities": {str(j): d.entity(j) for j in work}, "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def tedit_cases():
    c = Cases()
    one = lambda **kw: {"uids": [uid_of(16)], "change": kw}
    c.add("Etiketi düzenle: doğrula yazmaz", [tedit_step(Drawing(), "validate", one(shape="rectangle"))])
    c.add("Etiketi düzenle: plan yazmadan gösterir", [tedit_step(Drawing(), "plan", one(shape="rectangle"))])
    d = Drawing()
    c.add("Etiketi düzenle: yazılır ve geri alınır", [tedit_step(d, "execute", one(shape="rectangle")),
                                                      {"op": "undo", "returns": "Etiketi düzenle", "expect": {"canRedo": True, "revision": "changed"}}])
    for name, ch in [
        ("çerçeve dikdörtgen olur", {"shape": "rectangle"}),
        ("çerçeve daireye döner (varsayılan yazılmaz)", {"shape": "circle"}),
        ("çerçeve kaldırılır (çerçevesiz)", {"shape": "none"}),
        ("şablon: numara ve tür iki satır", {"content": "{number}\n{type}"}),
        ("şablon: numara, biçim ve genişlik", {"content": "{number} {form} {width}"}),
        ("yazı yüksekliği 3.5 mm", {"size": 3.5}),
        ("etiketin kolu açılır", {"leader": True}),
        ("kol kapalıysa kapalı kalır", {"leader": False}),
        ("şablon ve çerçeve birlikte", {"content": "{attr:Malzeme}", "shape": "rounded"}),
    ]:
        c.add(name, [tedit_step(Drawing(), "execute", {"uids": [uid_of(16)], "change": ch})])
    d = Drawing()
    first = tedit_step(d, "execute", one(content="{type}", size=3.0))
    c.add("şablon ve yükseklik kaldırılınca türün şablonuna ve 2.5 mm'ye döner", [first, tedit_step(d, "execute", one(content=None, size=None))])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(13)}]})
    c.add("iki etiket birlikte çerçevesiz olur", [made, tedit_step(d, "execute", {"uids": [uid_of(16), uid_of(17)], "change": {"shape": "none"}})])
    for name, inp, kw in [
        ("etiket verilmezse reddedilir", {"uids": [], "change": {"shape": "none"}}, {}),
        ("geçersiz kimlik reddedilir", {"uids": ["etiket"], "change": {"shape": "none"}}, {}),
        ("olmayan nesne reddedilir", {"uids": [MISSING], "change": {"shape": "none"}}, {}),
        ("çizgi etiket değil", {"uids": [uid_of(7)], "change": {"shape": "none"}}, {}),
        ("boş değişiklik reddedilir", {"uids": [uid_of(16)], "change": {}}, {}),
        ("sonsuz yükseklik reddedilir", {"uids": [uid_of(16)], "change": {"size": 2.0}}, {"non_finite": {"change.size": "NaN"}}),
        ("60 mm yükseklik reddedilir", {"uids": [uid_of(16)], "change": {"size": 60.0}}, {}),
        ("bilinmeyen yerli şablon reddedilir", {"uids": [uid_of(16)], "change": {"content": "{foo}"}}, {}),
        ("boş şablon reddedilir", {"uids": [uid_of(16)], "change": {"content": " "}}, {}),
        ("kilitli katmandaki etiket düzenlenmez", {"uids": [uid_of(11)], "change": {"shape": "none"}}, {}),
        ("geçersiz beklenen sürüm reddedilir", {"uids": [uid_of(16)], "change": {"shape": "none"}, "expectedRevision": "07"}, {}),
    ]:
        c.add(name, [tedit_step(Drawing(), "execute", inp, **kw)])
    d = Drawing()
    ok = tedit_step(d, "execute", one(shape="none"))
    stale = tedit_step(d, "execute", {**one(shape="rectangle"), "expectedRevision": "$prepared"}, conflict=True)
    c.add("sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, ok, stale])
    return c.cases

# ── Schedules (ADR 0344 §3) ────────────────────────────────────────────────────────────────────────────────────


TITLES = {"number": "No", "type": "Tür", "form": "Biçim", "width": "Genişlik", "height": "Yükseklik", "sill": "Denizlik",
          "head": "Lento", "wall": "Duvar", "level": "Kat", "phase": "Evre", "count": "Adet", "area": "Alan",
          "perimeter": "Çevre", "volume": "Hacim", "name": "Ad", "usage": "Kullanım", "tag": "Etiket", "place": "Konum",
          "length": "Uzunluk", "thickness": "Kalınlık", "attr": "Öznitelik"}
LENGTHS = {"width", "height", "sill", "head", "perimeter", "length", "thickness"}
SUMMED = {"count", "area", "perimeter", "volume", "length"}
EDITABLE = {"number", "type", "width", "sill", "head", "name", "usage", "tag", "thickness", "attr"}
NUMERIC = LENGTHS | {"count", "area", "volume"}
KIND = {"openings": "opening", "spaces": "space", "columns": "column", "walls": "wall"}
OF_LABEL = {"openings": "Kapı ve pencere çizelgesi", "spaces": "Mahal çizelgesi", "columns": "Kolon çizelgesi", "walls": "Duvar çizelgesi"}
DEFAULT = {"openings": ["number", "form", "type", "width", "height", "sill", "wall"],
           "spaces": ["number", "name", "usage", "level", "area", "perimeter", "height"],
           "columns": ["tag", "place", "type", "width", "level"],
           "walls": ["type", "thickness", "length", "height", "area", "volume"]}
READS = {
    "count": set(KIND), "level": set(KIND), "phase": set(KIND), "attr": set(KIND),
    "number": {"openings", "spaces"}, "type": {"openings", "columns", "walls"}, "width": {"openings", "columns"},
    "height": {"openings", "spaces", "walls"}, "area": {"openings", "spaces", "walls"}, "volume": {"spaces", "walls"},
    "form": {"openings"}, "sill": {"openings"}, "head": {"openings"}, "wall": {"openings"},
    "name": {"spaces"}, "usage": {"spaces"}, "perimeter": {"spaces"}, "tag": {"columns"}, "place": {"columns"},
    "length": {"walls"}, "thickness": {"walls"},
}


def heading(c):
    if c.get("title"):
        return c["title"]
    f = c["field"]
    if f == "attr":
        return c["attr"]
    if f in LENGTHS:
        return f"{TITLES[f]} (m)"
    if f == "area":
        return "Alan (m²)"
    if f == "volume":
        return "Hacim (m³)"
    return TITLES[f]


def schedule_problem(of, columns, sort, flt, at):
    if not 1 <= len(columns) <= 30:
        return refusal("invalid_schedule", f"Çizelgenin 1 ile 30 arası sütunu olur; {len(columns)} verildi.", f"{at}columns")
    for i, c in enumerate(columns):
        if of not in READS[c["field"]]:
            return refusal("invalid_schedule", f"“{TITLES[c['field']]}” {OF_LABEL[of].lower()}nde okunmaz.", f"{at}columns[{i}].field")
        if c["field"] == "attr" and "attr" not in c:
            return refusal("invalid_schedule", "Öznitelik sütunu okuyacağı özniteliğin adını ister.", f"{at}columns[{i}].attr")
        if c["field"] != "attr" and "attr" in c:
            return refusal("invalid_schedule", "Öznitelik adı yalnız öznitelik sütununda verilir.", f"{at}columns[{i}].attr")
        if c["field"] == "attr" and not c["attr"].strip():
            return refusal("invalid_schedule", "Özniteliğin adı boş olamaz.", f"{at}columns[{i}].attr")
        if "title" in c and not c["title"].strip():
            return refusal("invalid_schedule", "Sütun başlığı boş olamaz, tek satırdır, en çok 100 harftir.", f"{at}columns[{i}].title")
    for i, s in enumerate(sort):
        if s["column"] >= len(columns):
            return refusal("invalid_schedule", f"Sıralama {s['column'] + 1}. sütunu istiyor; çizelgenin {len(columns)} sütunu var.", f"{at}sort[{i}].column")
    if flt and "forms" in flt and of != "openings":
        return refusal("invalid_schedule", "Biçim süzgeci yalnız kapı ve pencere çizelgesindedir.", f"{at}filter.forms")
    for k, lv in enumerate((flt or {}).get("levels", [])):
        if not any(x["id"] == lv for x in LEVELS):
            return refusal("level_not_found", f"Projede “{lv}” kimlikli kat yok. Var olan bir katın kimliğini verin.", f"{at}filter.levels[{k}]")
    for k, ly in enumerate((flt or {}).get("layers", [])):
        if ly not in LAYERS:
            return refusal("layer_not_found", f"“{ly}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", f"{at}filter.layers[{k}]")
    return None


def takes(d, i, flt):
    if not flt:
        return True
    k, ly, f, _ = d.objs[i]
    arch = f.get("arch", {})
    if k == "opening":
        w = d.get(f["host"])
        arch = w[1][2].get("arch", {}) if w and w[1][0] == "wall" else {}
    if "layers" in flt and ly not in flt["layers"]:
        return False
    if "levels" in flt and arch.get("level") not in flt["levels"]:
        return False
    if "phases" in flt and arch.get("phase") not in flt["phases"]:
        return False
    if "forms" in flt and f.get("form", "door") not in flt["forms"]:
        return False
    return True


def add(a, b):
    if a is None or b is None:
        return a or b
    return (a[0], a[1] + b[1])


def schedule_rows(d, src):
    of, columns = src["of"], src["columns"]
    rows = []
    for i, (k, _, _, _) in sorted(d.objs.items()):
        if k != KIND[of] or not takes(d, i, src.get("filter")):
            continue
        vals = [value(d, i, c["field"], c.get("attr")) for c in columns]
        rows.append({"vals": vals, "texts": [value_text(v, False) if v else "" for v in vals], "uids": [uid_of(i)]})

    def cmp(a, b):
        for s in src.get("sort", []):
            j = s["column"]
            ea, eb = a["texts"][j] == "", b["texts"][j] == ""
            if ea and eb:
                o = 0
            elif ea:
                o = 1
            elif eb:
                o = -1
            else:
                x, y = number_of(a["vals"][j]), number_of(b["vals"][j])
                o = (x > y) - (x < y) if x is not None and y is not None else pe.natural_cmp(a["texts"][j], b["texts"][j])
                if s.get("descending"):
                    o = -o
            if o:
                return o
        return 0
    rows.sort(key=cmp_to_key(cmp))
    summed = [c["field"] in SUMMED for c in columns]
    if src.get("group"):
        merged = []
        for r in rows:
            key = [t for t, s in zip(r["texts"], summed) if not s]
            m = next((m for m in merged if [t for t, s in zip(m["texts"], summed) if not s] == key), None)
            if m is None:
                merged.append(r)
                continue
            for j, s in enumerate(summed):
                if s:
                    m["vals"][j] = add(m["vals"][j], r["vals"][j])
            m["uids"] += r["uids"]
        for m in merged:
            for j, s in enumerate(summed):
                if s:
                    m["texts"][j] = value_text(m["vals"][j], False) if m["vals"][j] else ""
        rows = merged
    n = len(rows)
    cells = [[heading(c) for c in columns]] + [r["texts"] for r in rows]
    styles = [{"row": 1, "col": j, "rows": n, "cols": 1, "color": GREY}
              for j, c in enumerate(columns) if c["field"] not in EDITABLE and n > 0]
    if src.get("total"):
        sums = [None] * len(columns)
        for r in rows:
            for j, s in enumerate(summed):
                if s:
                    sums[j] = add(sums[j], r["vals"][j])
        last = [value_text(v, False) if (s and v) else "" for v, s in zip(sums, summed)]
        for j, c in enumerate(columns):
            if c["field"] == "count" and not last[j]:
                last[j] = "0"
        if not summed[0]:
            last[0] = "Toplam"
        cells.append(last)
        styles.append({"row": n + 1, "col": 0, "rows": 1, "cols": len(columns), "bold": True})
    aligns = ["right" if c["field"] in NUMERIC else "left" for c in columns]
    return cells, [r["uids"] for r in rows], aligns, styles


def worked_out(d, t):
    t = copy.deepcopy(t)
    cells, members, aligns, styles = schedule_rows(d, t["source"])
    t["cells"] = cells
    t["rows"] = "$any"
    t["columns"] = "$any"
    t["aligns"] = aligns
    t["header"] = True
    if styles:
        t["styles"] = styles
    else:
        t.pop("styles", None)
    if members:
        t["source"]["members"] = members
    else:
        t["source"].pop("members", None)
    return t


def screate(d, inp, conflict=False):
    r = acc.point_not_finite(inp["at"], "Çizelgenin köşesinin", "at")
    if r:
        return r, None
    columns = inp["columns"] if "columns" in inp else [{"field": f} for f in DEFAULT[inp["of"]]]
    r = schedule_problem(inp["of"], columns, inp.get("sort", []), inp.get("filter"), "")
    if r:
        return r, None
    h = inp.get("height")
    if h is not None:
        if not math.isfinite(h):
            return acc.not_finite("Yazı yüksekliği", "height"), None
        if h <= 0:
            return refusal("invalid_schedule", f"Çizelgenin yazı yüksekliği {num(h)}; sıfırdan büyük olmalı.", "height"), None
    res, made = finish(inp, conflict, [None])
    if made is None:
        return res, None
    src = {"kind": "schedule", "of": inp["of"], "columns": columns}
    if inp.get("filter"):
        src["filter"] = inp["filter"]
    if inp.get("sort"):
        src["sort"] = inp["sort"]
    if inp.get("group"):
        src["group"] = True
    if inp.get("total"):
        src["total"] = True
    t = {"p": inp["at"], "rotation": 0.0, "height": h if h is not None else 2.5 / 1000 * SCALE,
         "rows": [], "columns": [], "cells": [], "header": True, "source": src}
    return res, worked_out(d, t)


def screate_step(d, op, inp, non_finite=None, conflict=False):
    result, t = screate(d, acc.with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "command": SCREATE, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if t is None:
        s["result"] = result
        return unchanged(s, d)
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": result["warnings"]}
        return unchanged(s, d)
    ent = {"kind": "table", "id": 0, "layerId": inp["layerId"], "attrs": {}, **extra_of(inp), **t}
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"table": ent, "revision": "$current"}, "warnings": result["warnings"]}
        s["within"] = 1e-9
        return unchanged(s, d)
    i = d.next
    d.next += 1
    d.objs[i] = ("table", inp["layerId"], t, {})
    s["result"] = {"status": "completed", "output": {"uid": "$uid", "id": i, "rows": len(t["source"].get("members", [])), "revision": "$current"},
                   "warnings": result["warnings"]}
    s["within"] = 1e-9
    s["expect"] = {"ids": d.ids(), "entities": {str(i): d.entity(i, extra_of(inp))}, "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def screate_cases():
    c = Cases()
    one = {"layerId": "cizelge", "at": P(0, -10), "of": "openings"}
    c.add("Kapı ve pencere çizelgesi: doğrula yazmaz", [screate_step(Drawing(), "validate", one)])
    c.add("Kapı ve pencere çizelgesi: plan yazmadan gösterir", [screate_step(Drawing(), "plan", one)])
    d = Drawing()
    c.add("Kapı ve pencere çizelgesi: yazılır ve geri alınır", [screate_step(d, "execute", one),
                                                                {"op": "undo", "returns": "Çizelge", "expect": {"ids": IDS, "canUndo": True, "canRedo": True, "revision": "changed"}}])
    c.add("Kapı ve pencere çizelgesi: gizli katmana uyarıyla yazılır", [screate_step(Drawing(), "execute", {**one, "layerId": "gizli"})])
    for name, inp in [
        ("açıklıklar numaraya göre, toplamıyla", {**one, "sort": [{"column": 0}], "total": True}),
        ("açıklıklar numaraya göre ters", {**one, "sort": [{"column": 0, "descending": True}]}),
        ("açıklıklar genişliğe göre sayıyla sıralanır", {**one, "sort": [{"column": 3}]}),
        ("türe göre grup: adet ve alan toplanır", {**one, "columns": [{"field": "type"}, {"field": "width"}, {"field": "count"}, {"field": "area"}], "group": True, "total": True}),
        ("varsayılan sütunlarla grup: aynı iki kapı tek satır", {**one, "group": True}),
        ("yalnız pencereler", {**one, "filter": {"forms": ["window"]}}),
        ("yalnız Zemin kat: duvarın katı açıklığın katı", {**one, "filter": {"levels": ["zemin"]}}),
        ("başlığı verilmiş ve öznitelik sütunu", {**one, "columns": [{"field": "number", "title": "Kapı no"}, {"field": "attr", "attr": "Malzeme"}, {"field": "head"}, {"field": "level"}]}),
        ("mahal çizelgesi, toplamıyla", {"layerId": "cizelge", "at": P(0, -20), "of": "spaces", "total": True}),
        ("mahaller alana göre büyükten küçüğe, hacimleriyle", {"layerId": "cizelge", "at": P(0, -20), "of": "spaces", "columns": [{"field": "number"}, {"field": "area"}, {"field": "volume"}], "sort": [{"column": 1, "descending": True}], "total": True}),
        ("kolon çizelgesi", {"layerId": "cizelge", "at": P(0, -30), "of": "columns"}),
        ("duvar çizelgesi: net yüzey ve hacim", {"layerId": "cizelge", "at": P(0, -40), "of": "walls", "total": True}),
        ("hiç nesnesi olmayan süzgeç başlık ve toplam verir", {**one, "filter": {"layers": ["cizim"]}, "columns": [{"field": "number"}, {"field": "count"}], "total": True}),
        ("verilen yazı yüksekliği ve renk", {**one, "height": 0.25, "color": "#336699"}),
    ]:
        c.add(name, [screate_step(Drawing(), "execute", inp)])
    for name, inp, kw in [
        ("açıklık çizelgesinde ad okunmaz", {**one, "columns": [{"field": "name"}]}, {}),
        ("sütunsuz çizelge reddedilir", {**one, "columns": []}, {}),
        ("adsız öznitelik sütunu reddedilir", {**one, "columns": [{"field": "attr"}]}, {}),
        ("öznitelik adı başka sütunda verilmez", {**one, "columns": [{"field": "number", "attr": "X"}]}, {}),
        ("olmayan sütuna göre sıralanmaz", {**one, "sort": [{"column": 9}]}, {}),
        ("mahal çizelgesinde biçim süzgeci olmaz", {"layerId": "cizelge", "at": P(0, 0), "of": "spaces", "filter": {"forms": ["door"]}}, {}),
        ("olmayan kat reddedilir", {**one, "filter": {"levels": ["yok"]}}, {}),
        ("süzgecin olmayan katmanı reddedilir", {**one, "filter": {"layers": ["yok"]}}, {}),
        ("sıfır yazı yüksekliği reddedilir", {**one, "height": 0.0}, {}),
        ("sonsuz köşe reddedilir", {**one, "at": P(0, 0)}, {"non_finite": {"at.y": "Infinity"}}),
        ("olmayan katman reddedilir", {**one, "layerId": "yok"}, {}),
        ("katman grubu reddedilir", {**one, "layerId": "pafta"}, {}),
        ("kilitli katman reddedilir", {**one, "layerId": "kilitli"}, {}),
        ("geçersiz çizgi kalınlığı reddedilir", {**one, "lineWeight": 120.0}, {}),
        ("geçersiz beklenen sürüm reddedilir", {**one, "expectedRevision": "07"}, {}),
    ]:
        c.add(name, [screate_step(Drawing(), "execute", inp, **kw)])
    d = Drawing()
    made = screate_step(d, "execute", one)
    stale = screate_step(d, "execute", {**one, "expectedRevision": "$prepared"}, conflict=True)
    c.add("sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    d = Drawing()
    made = screate_step(d, "execute", one)
    gone = foreign(d, "cad.entities.delete", {"uids": [uid_of(15)]}, {}, moves=[15])
    c.add("açıklık silinince çizelgenin satırı da gider", [made, *gone])
    d = Drawing()
    made = screate_step(d, "execute", {"layerId": "cizelge", "at": P(0, -20), "of": "spaces", "total": True})
    c.add("mahal taşınınca çizelgesi aynı kalır", [made, *move(d, [3], 1.0, 0.0)])
    return c.cases

# ── Çizelgeyi düzenle ──────────────────────────────────────────────────────────────────────────────────────────


def created(d, inp=None):
    """A schedule written first: its step and slot."""
    inp = inp or {"layerId": "cizelge", "at": P(0, -10), "of": "openings"}
    step = screate_step(d, "execute", inp)
    return step, d.next - 1


def length_in(v, positive):
    try:
        x = float(v.strip().replace(",", "."))
    except ValueError:
        return None, refusal("invalid_cell_value", f"“{v}” bir uzunluk değil; projenin biriminde bir sayı yazın.", "operation.value")
    if not math.isfinite(x) or (positive and x <= 0) or x < 0:
        return None, refusal("invalid_cell_value", f"“{v}” bu alana yazılmaz: uzunluk {'' if positive else 'eksi olmamalı; '}sıfırdan büyük olmalı.", "operation.value")
    return x, None


def written(d, i, field, attr, v):
    """(fields, attrs) with `v` written, or a refusal."""
    k, ly, f, at = d.objs[i]
    f, at = copy.deepcopy(f), copy.deepcopy(at)
    t = v.strip() or None

    def put(key, x):
        if x is None:
            f.pop(key, None)
        else:
            f[key] = x
    if field == "attr":
        if t is None:
            at.pop(attr, None)
        else:
            at[attr] = t
        return (f, at), None
    pairs = {("opening", "number"): "number", ("space", "number"): "number", ("space", "name"): "name",
             ("space", "usage"): "usage", ("column", "tag"): "tag", ("column", "type"): "type"}
    if (k, field) in pairs:
        put(pairs[(k, field)], t)
        return (f, at), None
    if (k, field) == ("opening", "type"):
        ids = {name: id for id, name in OPENING_TYPES.items()}
        put("style", ids.get(t, t) if t else None)
        return (f, at), None
    if (k, field) in (("opening", "width"), ("column", "width"), ("wall", "thickness")):
        x, r = length_in(v, True)
        if r:
            return None, r
        f["width"] = x
        if k == "opening":
            # Its jambs written from its wall again (ADR 0341 §3): its middle stays, its width grows both ways.
            y = (f["place"][0]["y"] + f["place"][1]["y"]) / 2
            f["place"] = opening(0, f["at"], x, y)["place"]
        return (f, at), None
    if (k, field) in (("opening", "sill"), ("opening", "head")):
        if t is None:
            put(field, None)
            return (f, at), None
        x, r = length_in(t, field == "head")
        if r:
            return None, r
        f[field] = x
        return (f, at), None
    return None, refusal("cell_computed", "Bu hücre bu nesnenin kendi alanı değil; düzenlenmez.", "operation.column")


def sedit(d, table, inp, conflict=False):
    """(result, work, table): `work` the objects' new (fields, attrs) by id, `table` the schedule's source after."""
    u = inp["uid"]
    if not acc.is_uid(u):
        return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin.", "uid"), None, None
    found = d.get(u)
    if not found:
        return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok. Var olan bir çizelgenin kimliğini verin.", "uid"), None, None
    i, (k, ly, t, _) = found
    if k != "table" or t.get("source", {}).get("kind") != "schedule":
        return refusal("not_schedule", "Nesne bir mimari çizelge değil; Çizelge ile yerleştirilen bir tablonun kimliğini verin.", "uid"), None, None
    src = t["source"]
    members = src.get("members", [])
    op = inp["operation"]
    work, new_src = {}, None
    if op["kind"] == "cell":
        r, col = op["row"], op["column"]
        if r == 0 or r > len(members):
            return refusal("cell_out_of_range", f"Çizelgenin {r}. satırı veri satırı değil; veri satırları 1–{len(members)} (0 başlık{', sonuncusu toplam' if src.get('total') else ''}).", "operation.row"), None, None
        if col >= len(src["columns"]):
            return refusal("cell_out_of_range", f"Çizelgenin {len(src['columns'])} sütunu var; {col + 1}. sütun yok.", "operation.column"), None, None
        c = src["columns"][col]
        if c["field"] not in EDITABLE or (c["field"] == "type" and src["of"] == "walls"):
            return refusal("cell_computed", f"“{heading(c)}” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.", "operation.column"), None, None
        row = members[r - 1]
        if c["field"] == "number" and len(row) > 1:
            return refusal("schedule_group_number", f"Bu satır {len(row)} nesnedir; aynı numara hepsine yazılmaz. Grubu kaldırıp tek tek numaralayın.", "operation.row"), None, None
        v = op["value"]
        if any(ord(ch) < 32 or ord(ch) == 127 for ch in v) or len(v) > 200:
            return refusal("invalid_cell_value", "Hücrenin yazısı tek satırdır, en çok 200 harftir.", "operation.value"), None, None
        for m in row:
            j = d.get(m)[0]
            w, rr = written(d, j, c["field"], c.get("attr"), v)
            if rr:
                return rr, None, None
            work[j] = w
    else:
        new_src = copy.deepcopy(src)
        for key in ("columns", "sort"):
            if key in op:
                new_src[key] = op[key]
        if "filter" in op:
            if op["filter"]:
                new_src["filter"] = op["filter"]
            else:
                new_src.pop("filter", None)
        for key in ("group", "total"):
            if key in op:
                if op[key]:
                    new_src[key] = True
                else:
                    new_src.pop(key, None)
        rr = schedule_problem(new_src["of"], new_src["columns"], new_src.get("sort", []), new_src.get("filter"), "operation.")
        if rr:
            return rr, None, None
    r = acc.revision(inp, conflict)
    if r:
        return r, None, None
    for j in work:
        if d.objs[j][1] == "kilitli":
            return refusal("layer_locked", "“Kilitli katman” katmanı kilitli; çizelgenin satırı ya da kendisi düzenlenemez. Kilidi Katmanlar panelinden açın.", "uid"), None, None
    if new_src is not None and ly == "kilitli":
        return refusal("layer_locked", "“Kilitli katman” katmanı kilitli; çizelgenin satırı ya da kendisi düzenlenemez. Kilidi Katmanlar panelinden açın.", "uid"), None, None
    return {"warnings": []}, work, (i, new_src)


def sedit_step(d, op, inp, conflict=False):
    result, work, after = sedit(d, None, inp, conflict)
    s = {"op": op, "command": SEDIT, "input": inp}
    if work is None:
        s["result"] = result
        return unchanged(s, d)
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        return unchanged(s, d)
    i, new_src = after
    target = d if op == "execute" else copy.deepcopy(d)
    for j, (f, at) in work.items():
        k, ly, _, _ = target.objs[j]
        target.objs[j] = (k, ly, f, at)
    if new_src is not None:
        k, ly, t, at = target.objs[i]
        t = copy.deepcopy(t)
        t["source"] = new_src
        target.objs[i] = (k, ly, t, at)
    target.follow(touched=list(work))
    edited = [uid_of(j) for j in sorted(work)]
    s["result"] = {"status": "completed", "output": {"table": target.entity(i), "edited": edited, "revision": "$current"}, "warnings": []}
    s["within"] = 1e-9
    if op == "plan":
        return unchanged(s, d)
    s["expect"] = {"ids": d.ids(), "entities": {str(j): d.entity(j) for j in sorted(set(work) | {i})},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def sedit_cell_step(d, op, made, row, column, value):
    step, i = made
    return {"made": step, "step": sedit_step(d, op, {"uid": uid_of(i), "operation": {"kind": "cell", "row": row, "column": column, "value": value}})}


def sedit_cases():
    c = Cases()
    cell = lambda i, row, col, v: {"uid": uid_of(i), "operation": {"kind": "cell", "row": row, "column": col, "value": v}}
    setop = lambda i, **kw: {"uid": uid_of(i), "operation": {"kind": "set", **kw}}
    d = Drawing()
    made, i = created(d)
    c.add("hücre: kapının numarası yazılır, satır yeniden yazılır", [made, sedit_step(d, "execute", cell(i, 2, 0, "K11"))])
    d = Drawing()
    made, i = created(d)
    c.add("hücre: doğrula yazmaz", [made, sedit_step(d, "validate", cell(i, 2, 0, "K11"))])
    d = Drawing()
    made, i = created(d)
    c.add("hücre: plan çizelgeyi olacağı gibi gösterir, yazmaz", [made, sedit_step(d, "plan", cell(i, 2, 0, "K11"))])
    d = Drawing()
    made, i = created(d)
    c.add("hücre: yazılır ve geri alınır", [made, sedit_step(d, "execute", cell(i, 3, 0, "P02")),
                                             {"op": "undo", "returns": "Çizelgeyi düzenle", "expect": {"canRedo": True, "revision": "changed"}}])
    for name, row, col, v in [
        ("hücre: türün adı yazılır, türün kimliği saklanır", 5, 2, "Kapı 90"),
        ("hücre: genişlik projenin biriminde, virgüllü", 3, 3, "1,4"),
        ("hücre: denizlik yazılır, yükseklik izler", 3, 5, "1.0"),
        ("hücre: boş numara alanı kaldırır", 3, 0, ""),
    ]:
        d = Drawing()
        made, i = created(d)
        c.add(name, [made, sedit_step(d, "execute", cell(i, row, col, v))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -20), "of": "spaces"})
    c.add("mahal çizelgesinde ad yazılır", [made, sedit_step(d, "execute", cell(i, 1, 1, "Oturma odası"))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -20), "of": "spaces", "columns": [{"field": "number"}, {"field": "attr", "attr": "Döşeme"}]})
    c.add("öznitelik sütunu nesnenin özniteliğine yazılır", [made, sedit_step(d, "execute", cell(i, 2, 1, "Seramik"))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -40), "of": "walls"})
    c.add("duvarın kalınlığı yazılır, net yüzeyi ve hacmi izler", [made, sedit_step(d, "execute", cell(i, 2, 1, "0.25"))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -10), "of": "openings", "columns": [{"field": "type"}, {"field": "count"}], "group": True})
    c.add("gruplu satırın türü bütün nesnelerine yazılır", [made, sedit_step(d, "execute", cell(i, 2, 0, "Kapı 80"))])
    d = Drawing()
    made, i = created(d)
    c.add("set: sütunlar, sıralama ve toplam değişir", [made, sedit_step(d, "execute", setop(i, columns=[{"field": "number"}, {"field": "area"}, {"field": "count"}], sort=[{"column": 0, "descending": True}], total=True))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -10), "of": "openings", "filter": {"forms": ["window"]}})
    c.add("set: süzgeç kaldırılır (null)", [made, sedit_step(d, "execute", setop(i, filter=None))])
    d = Drawing()
    made, i = created(d)
    c.add("set: grup açılır", [made, sedit_step(d, "execute", setop(i, group=True))])
    for name, inp_of, kw in [
        ("hesaplanan yükseklik düzenlenmez", lambda i: cell(i, 1, 4, "2.0"), {}),
        ("başlık satırı veri satırı değil", lambda i: cell(i, 0, 0, "X"), {}),
        ("olmayan satır reddedilir", lambda i: cell(i, 9, 0, "X"), {}),
        ("olmayan sütun reddedilir", lambda i: cell(i, 1, 9, "X"), {}),
        ("sayı olmayan genişlik reddedilir", lambda i: cell(i, 1, 3, "geniş"), {}),
        ("sıfır genişlik reddedilir", lambda i: cell(i, 1, 3, "0"), {}),
        ("eksi denizlik reddedilir", lambda i: cell(i, 2, 5, "-0.5"), {}),
        ("iki satırlı değer reddedilir", lambda i: cell(i, 1, 0, "K\n1"), {}),
        ("set: okunmayan sütun reddedilir", lambda i: setop(i, columns=[{"field": "usage"}]), {}),
        ("set: olmayan kat reddedilir", lambda i: setop(i, filter={"levels": ["yok"]}), {}),
        ("geçersiz beklenen sürüm reddedilir", lambda i: {**cell(i, 1, 0, "K9"), "expectedRevision": "07"}, {}),
    ]:
        d = Drawing()
        made, i = created(d)
        c.add(name, [made, sedit_step(d, "execute", inp_of(i))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -10), "of": "openings", "group": True})
    c.add("gruplu satırın numarası yazılmaz", [made, sedit_step(d, "execute", cell(i, 2, 0, "K05"))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -40), "of": "walls"})
    c.add("duvarın türü (stili) çizelgede düzenlenmez", [made, sedit_step(d, "execute", cell(i, 1, 0, "D30"))])
    d = Drawing()
    made, i = created(d, {"layerId": "cizelge", "at": P(0, -30), "of": "columns"})
    c.add("kilitli katmandaki kolonun etiketi yazılmaz", [made, sedit_step(d, "execute", cell(i, 2, 0, "S9"))])
    for name, u in [("geçersiz kimlik reddedilir", "tablo"), ("olmayan nesne reddedilir", MISSING), ("çizgi çizelge değil", uid_of(7))]:
        c.add(name, [sedit_step(Drawing(), "execute", {"uid": u, "operation": {"kind": "cell", "row": 1, "column": 0, "value": "X"}})])
    d = Drawing()
    made, i = created(d)
    ok = sedit_step(d, "execute", cell(i, 2, 0, "K11"))
    stale = sedit_step(d, "execute", {**cell(i, 2, 0, "K12"), "expectedRevision": "$prepared"}, conflict=True)
    c.add("sürüm değişince yazılmaz (çakışma)", [made, {"op": "captureRevision", "as": "prepared"}, ok, stale])
    d = Drawing()
    made, i = created(d)
    refused = {"op": "execute", "command": "cad.tables.edit",
               "input": {"operation": {"kind": "edit", "uid": uid_of(i), "edits": [{"kind": "setCell", "row": 1, "col": 0, "words": "X"}]}},
               "result": refusal("schedule_cell", "Çizelgenin hücreleri tablo olarak düzenlenmez; Çizelgeyi düzenle onları nesnelere yazar.", "operation.edits[0]"),
               "expect": {"ids": d.ids(), "revision": "same"}}
    c.add("Tabloyu düzenle çizelgenin hücresini yazmaz", [made, refused])
    d = Drawing()
    made, i = created(d)
    k = d.next
    tag = create_step(d, "execute", {"layerId": "etiket", "tags": [{"of": uid_of(13)}]})
    ed = sedit_step(d, "execute", cell(i, 3, 0, "P07"))
    ed["expect"]["entities"][str(k)] = d.entity(k)
    c.add("çizelgede numara değişince açıklığın etiketi de değişir", [made, tag, ed])
    return c.cases


def document(command, title, cases):
    note = ("ADR 0344 §1–§4. Beklenenler scripts/fixtures/tag_command_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "duvarlar düz ve eksenlere paralel, mahaller dikdörtgen. Kurulumdaki çizimde 16 nesne var; yeni nesne 17'den başlar. "
            "Çizelgenin satır ve sütun boyları yazı tiplerinin ölçüsündendir, burada denetlenmez ($any).")
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note,
            "setup": SETUP, "cases": cases}


def files():
    return {
        "cad.tags.create.json": document(CREATE, "Etiket: türlerin şablonları ve çerçeveleri, şablon, toplu etiket; nesnesiyle gitmek; retler", tag_create_cases() + refresh_tag_cases() + level_tag_cases() + elevation_tag_cases() + net_area_tag_cases()),
        "cad.tags.edit.json": document(TEDIT, "Etiketi düzenle: çerçeve, şablon, yükseklik, kol; birden çok etiket; retler", tedit_cases()),
        "cad.tags.check.json": document(CHECK, "Etiketleri denetle: yinelenen numara, türsüz, duvarsız, nesnesiz etiket, etiketsizler; katmanlar", check_cases()+missing_space_number_cases()),
        "cad.schedules.create.json": document(SCREATE, "Çizelge: açıklık, mahal, kolon ve duvar; sıralama, grup, toplam, süzgeç, başlıklar; retler", screate_cases()),
        "cad.schedules.edit.json": document(SEDIT, "Çizelgeyi düzenle: hücre nesneye yazılır, hesaplanan hücre, grup, set; retler", sedit_cases()),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, doc in files().items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        assert len(doc["cases"]) >= 20, (name, len(doc["cases"]))
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{doc['command']}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
