#!/usr/bin/env python3
"""The opening commands' shared cases (docs/adr/0341 §3, §4), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.openings.{create,edit}.json; `--check` compares and writes nothing. The walls are
straight, so an opening's jambs are its wall's base line's point and left normal at its interval's ends, out to the
wall's faces (the left face `offset + {0, w/2, w}[justify]`, the right one `width` less). The rules, as the ADR gives
them:

- an opening lies on `[s0, s1]` of its wall's base line (centred at `at`, or from it or to it by its anchor), on one
  edge within the line's ends (1e-9 · max(L, 1) m of slack), clear of its wall's other openings (touching is not
  running into);
- `cad.openings.create`: no opening (`no_objects`); each in order: its numbers (`at`, `distance`, `width`, `swing`,
  then the point's), its wall, its type (KentOS's, by id then name, case aside; none: the form's first), the values it
  gives over the type's, its place (`at`; or the point's nearest place on the base line, with `distance` its near
  jamb that far from the wall's end `from`, or with `middle` the middle of the edge it is by), its side (given, else
  the point's, else left), its fields, its jambs; then the overlaps, the line weight, the revision, the layer;
- `cad.openings.edit`: the ids (none: Türü değiştir's `from` or Numarala's form names every one), openings only, the
  operation's values, each opening's fields and jambs on its wall (a wall gone keeps it where it is; moving or copying
  it then is refused), the overlaps, the revision, a locked layer refusing all;
- the document keeps every opening with its wall at a step's end: read back from jambs the step moved by themselves
  (their centre's nearest place on the base line; hand and side turned when the jambs run against the line or their
  left points fell on its right), else jambs written from the wall and `at`; a wall gone breaks it (`hostGone`), its
  place kept; a wall's split, flip and other path edits read its openings back onto the new line.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"


def num(x):
    """A number as Rust's `{}` writes an f64."""
    if isinstance(x, float) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, kind="layer", visible=True, locked=False, children=None):
    return {"id": id, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


def base(id, layer_id, **extra):
    return {"id": id, "layerId": layer_id, "attrs": {}, **extra}


def wall(id, layer_id, pts, width, justify="center", **extra):
    return {"kind": "wall", **base(id, layer_id), "pts": pts, "width": width, "justify": justify, **extra}


ENTITIES = [
    wall(1, "duvar", [P(0, 0), P(10, 0)], 0.2),
    wall(2, "duvar", [P(0, 5), P(6, 5), P(6, 9)], 0.2, justify="left"),
    wall(3, "duvar", [P(20, 0), P(20, 6)], 0.3),
    wall(4, "kilitli", [P(30, 0), P(34, 0)], 0.2),
    {"kind": "line", **base(5, "cizim"), "a": P(0, 20), "b": P(5, 20)},
    wall(6, "duvar", [P(40, 0), P(40.5, 0)], 0.2),
    wall(7, "duvar", [P(50, 0), P(56, 0)], 0.2, justify="right", offset=0.05),
    wall(8, "duvar", [P(10, 2), P(0, 2)], 0.2),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEW = max(IDS) + 1
LAYERS = {"duvar": "Duvar", "kapi": "Kapı", "cizim": "Çizim", "kilitli": "Kilitli katman", "gizli": "Gizli katman",
          "sinir": "Sınır"}
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Açıklıklar",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad"},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("cizim", "Çizim"),
               layer("kilitli", "Kilitli katman", locked=True), layer("gizli", "Gizli katman", visible=False),
               layer("pafta", "Pafta", kind="group", children=[layer("sinir", "Sınır")])],
    "activeLayer": "duvar",
    "styles": {"items": [], "categories": []},
    "entities": ENTITIES,
}

# ── KentOS's opening types (ADR 0341 §1's table), metres ───────────────────────────────────────────────────────

TYPES = [
    {"id": "kentos-kapi-90", "name": "Kapı 90", "form": "door", "width": 0.9, "leaves": 1, "operation": "swing", "frame": 0.05, "head": 2.1},
    {"id": "kentos-kapi-80", "name": "Kapı 80", "form": "door", "width": 0.8, "leaves": 1, "operation": "swing", "frame": 0.05, "head": 2.1},
    {"id": "kentos-iki-kanatli-kapi-140", "name": "İki kanatlı kapı 140", "form": "door", "width": 1.4, "leaves": 2, "operation": "swing", "frame": 0.05, "head": 2.1},
    {"id": "kentos-surme-kapi-160", "name": "Sürme kapı 160", "form": "door", "width": 1.6, "leaves": 2, "operation": "slide", "frame": 0.05, "head": 2.1},
    {"id": "kentos-pencere-120", "name": "Pencere 120", "form": "window", "width": 1.2, "leaves": 2, "operation": "swing", "frame": 0.06, "sill": 0.9, "head": 2.2},
    {"id": "kentos-sabit-pencere-100", "name": "Sabit pencere 100", "form": "window", "width": 1.0, "leaves": 1, "operation": "fixed", "frame": 0.06, "sill": 0.9, "head": 2.2},
    {"id": "kentos-bosluk-100", "name": "Boşluk 100", "form": "opening", "width": 1.0, "leaves": 1, "operation": "fixed", "head": 2.1},
]
STEPS = {"door": "Kapı", "window": "Pencere", "opening": "Boşluk"}


def find_type(key):
    folded = key.strip().lower()
    for t in TYPES:
        if t["id"] == key:
            return t
    for t in TYPES:
        if t["name"].lower() == folded:
            return t
    return None


def no_type(key, path):
    return refusal("style_not_found", f"“{key}” adında ya da kimliğinde açıklık türü yok. Var olanlar: {', '.join(t['name'] for t in TYPES)}.", path)


def first_type(form):
    return next(t for t in TYPES if t["form"] == form)


def apply_type(f, t):
    f["form"], f["width"], f["leaves"], f["operation"] = t["form"], t["width"], t["leaves"], t["operation"]
    f["swing"] = None
    for k in ("frame", "sill", "head"):
        f[k] = t.get(k)
    f["style"] = t["id"]


# ── The wall's base line ────────────────────────────────────────────────────────────────────────────────────────


class Line:
    def __init__(self, w):
        pts = [(p["x"], p["y"]) for p in w["pts"]]
        self.edges = list(zip(pts, pts[1:]))
        self.cum, total = [], 0.0
        for a, b in self.edges:
            self.cum.append(total)
            total += math.hypot(b[0] - a[0], b[1] - a[1])
        self.length = total
        just = {"left": 0.0, "center": w["width"] / 2, "right": w["width"]}[w["justify"]]
        self.left = w.get("offset", 0.0) + just
        self.right = self.left - w["width"]

    def edge_len(self, i):
        a, b = self.edges[i]
        return math.hypot(b[0] - a[0], b[1] - a[1])

    def at(self, s):
        """The point at `s` and the unit tangent there."""
        for i, (a, b) in enumerate(self.edges):
            ln = self.edge_len(i)
            if s <= self.cum[i] + ln or i == len(self.edges) - 1:
                t = (s - self.cum[i]) / ln
                d = ((b[0] - a[0]) / ln, (b[1] - a[1]) / ln)
                return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t), d
        raise AssertionError

    def nearest(self, q):
        best_d, best_s = math.inf, 0.0
        for i, (a, b) in enumerate(self.edges):
            dx, dy = b[0] - a[0], b[1] - a[1]
            t = ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / (dx * dx + dy * dy)
            t = min(max(t, 0.0), 1.0)
            p = (a[0] + dx * t, a[1] + dy * t)
            d = math.hypot(q[0] - p[0], q[1] - p[1])
            if d < best_d:
                best_d, best_s = d, self.cum[i] + t * self.edge_len(i)
        return best_s

    def beside(self, s, d):
        p, t = self.at(s)
        return P(p[0] - t[1] * d, p[1] + t[0] * d)

    def jambs(self, s0, s1):
        tol = 1e-9 * max(self.length, 1.0)
        if not s1 > s0 or s0 < -tol or s1 > self.length + tol:
            return None
        if not any(s0 >= self.cum[i] - tol and s1 <= self.cum[i] + self.edge_len(i) + tol for i in range(len(self.edges))):
            return None
        return [self.beside(s0, self.left), self.beside(s0, self.right), self.beside(s1, self.left), self.beside(s1, self.right)]


def span(f):
    at, w = f["at"], f["width"]
    return {"start": (at, at + w), "center": (at - w / 2, at + w / 2), "end": (at - w, at)}[f.get("anchor", "center")]


def at_of_centre(s, anchor, w):
    return {"start": s - w / 2, "center": s, "end": s + w / 2}[anchor]


def overlap(a, b):
    tol = 1e-9 * max(abs(a[1]), abs(b[1]), 1.0)
    return a[0] < b[1] - tol and b[0] < a[1] - tol


def read_back(line, place, anchor, width):
    """The `at` of jambs on `line`, and whether the hand and the side turned."""
    m = (sum(p["x"] for p in place) / 4, sum(p["y"] for p in place) / 4)
    s = line.nearest(m)
    p, t = line.at(s)
    n = (-t[1], t[0])
    mid = lambda a, b: ((a["x"] + b["x"]) / 2, (a["y"] + b["y"]) / 2)  # noqa: E731
    m0, m1 = mid(place[0], place[1]), mid(place[2], place[3])
    run = (m1[0] - m0[0], m1[1] - m0[1])
    lm, rm = mid(place[0], place[2]), mid(place[1], place[3])
    dot = lambda a, b: a[0] * b[0] + a[1] * b[1]  # noqa: E731
    left = dot((lm[0] - p[0], lm[1] - p[1]), n)
    right = dot((rm[0] - p[0], rm[1] - p[1]), n)
    return at_of_centre(s, anchor, width), dot(run, t) < 0, left < right


OTHER = {"start": "end", "end": "start", "left": "right", "right": "left"}

# ── An opening's fields and their rule ──────────────────────────────────────────────────────────────────────────


def problem(f):
    w = f["width"]
    if not (math.isfinite(w) and 0 < w <= 100):
        return "width", f"Açıklığın genişliği {num(w)}; sıfırdan büyük, en çok 100 m olmalı."
    if f["leaves"] not in (1, 2):
        return "leaves", f"Açıklığın kanat sayısı {f['leaves']}; 1 ya da 2 olmalı."
    s = f.get("swing")
    if s is not None and not (0 < s <= 180):
        return "swing", f"Kanadın açılma açısı {num(s)}°; sıfırdan büyük, en çok 180° olmalı."
    fr = f.get("frame")
    if fr is not None and not (fr >= 0 and 2 * fr < w):
        return "frame", f"Kasa genişliği {num(fr)} m; sıfır ya da daha büyük, açıklığın yarısından ({num(w / 2)} m) küçük olmalı."
    if f.get("sill") is not None and f.get("head") is not None and f["head"] <= f["sill"]:
        return "head", f"Lento ({num(f['head'])} m) denizlikten ({num(f['sill'])} m) yüksek olmalı."
    if f.get("number") is not None and f["number"].strip() == "":
        return "number", "Açıklığın numarası boş olamaz; numarayı kaldırın ya da bir numara yazın."
    return None


def off_wall(f, line, path):
    s0, s1 = span(f)
    return refusal("opening_off_wall", f"Açıklık duvarın ucundan ya da köşesinden taşıyor: duvar boyunca {s0:.3f}–{s1:.3f} m'de, duvarın "
                   f"uzunluğu {line.length:.3f} m; açıklık duvarın tek bir kenarında, uçlarının arasında kalmalı. Açıklığı kaydırın ya da daraltın.", path)


def placed(f, line, path):
    j = line.jambs(*span(f))
    if j is None:
        return off_wall(f, line, path)
    out = dict(f, place=j)
    out.pop("broken", None)
    return out


def stored(f):
    """The fields as the document keeps them: defaults left out."""
    out = {"host": f["host"], "at": float(f["at"])}
    if f.get("anchor", "center") != "center":
        out["anchor"] = f["anchor"]
    out["width"] = float(f["width"])
    out["form"] = f["form"]
    if f["leaves"] != 1:
        out["leaves"] = f["leaves"]
    if f["operation"] != "swing":
        out["operation"] = f["operation"]
    if f.get("hand", "start") != "start":
        out["hand"] = f["hand"]
    if f.get("side", "left") != "left":
        out["side"] = f["side"]
    for k in ("swing",):
        if f.get(k) is not None:
            out[k] = float(f[k])
    if f.get("showSwing", True) is False:
        out["showSwing"] = False
    for k in ("frame", "sill", "head"):
        if f.get(k) is not None:
            out[k] = float(f[k])
    for k in ("style", "number"):
        if f.get(k) is not None:
            out[k] = f[k]
    out["place"] = f["place"]
    if f.get("broken"):
        out["broken"] = f["broken"]
    return out


def entity(id, layer_id, f, extra=None):
    return {"kind": "opening", "id": id, "layerId": layer_id, "attrs": {}, **(extra or {}), **stored(f)}


# ── Checks the commands share ───────────────────────────────────────────────────────────────────────────────────


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


def revision(inp, conflict):
    rev = inp.get("expectedRevision")
    if rev is None:
        return None
    if not rev.startswith("$") and not (rev.isdigit() and (rev == "0" or not rev.startswith("0"))):
        return refusal("invalid_revision", f"Beklenen sürüm “{rev}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
    if conflict:
        out = refusal("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", "conflict")
        out["error"]["revision"] = "$current"
        return out
    return None


def not_finite(name, path):
    return refusal("not_finite", f"{name} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", path)


def point_not_finite(p, whose, path):
    for axis, name in (("x", "doğu (Y)"), ("y", "kuzey (X)")):
        if not math.isfinite(p[axis]):
            return refusal("not_finite", f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", f"{path}.{axis}")
    return None


def uid_of(i):
    return f"$uidOf:{i}"


UID_TEXT = "0123456789abcdef-"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"


class Drawing:
    """The drawing as the cases' steps leave it: its walls by id and its openings by id."""

    def __init__(self):
        self.walls = {i: e for i, e in BY_ID.items() if e["kind"] == "wall"}
        self.openings = {}
        self.layers = {}
        self.next = NEW
        self.locked = set()

    def wall_named(self, uid, path):
        """The wall `uid` names: (id, its line) or a refusal."""
        if not uid.startswith("$uidOf:"):
            if len(uid) != 36 or any(c not in UID_TEXT for c in uid):
                return refusal("invalid_uid", f"“{uid}” bir kalıcı kimlik (UUID) değil; duvarın kimliğini verin.", path)
            return refusal("entity_not_found", f"Çizimde “{uid}” kimlikli nesne yok. Var olan bir duvarın kimliğini verin.", path)
        i = int(uid[len("$uidOf:"):])
        if i not in self.walls:
            return refusal("not_a_wall", "Nesne duvar değil; kapı, pencere ve boşluk yalnız duvara konur.", path)
        return i, Line(self.walls[i])

    def line_of(self, host):
        i = int(host[len("$uidOf:"):]) if host.startswith("$uidOf:") else None
        return Line(self.walls[i]) if i in self.walls else None

    def on(self, host):
        return [(i, f) for i, f in sorted(self.openings.items()) if f["host"] == host]

    def clear(self, changed, new):
        """The overlap refusal, or None: `changed` [(id, fields, path)], `new` [(fields, path)]."""
        by_host = {}
        ids = [i for i, _, _ in changed]
        for _, f, path in changed:
            if not f.get("broken"):
                by_host.setdefault(f["host"], []).append((span(f), path))
        for f, path in new:
            if not f.get("broken"):
                by_host.setdefault(f["host"], []).append((span(f), path))
        for host in sorted(by_host):
            lst = by_host[host]
            for i, o in self.on(host):
                if i not in ids and not o.get("broken"):
                    lst.append((span(o), None))
            for k, (a, path) in enumerate(lst):
                if path is None:
                    continue
                for j, (b, _) in enumerate(lst):
                    if j != k and overlap(a, b):
                        return refusal("openings_overlap", f"Açıklık ({a[0]:.3f}–{a[1]:.3f} m) aynı duvardaki başka bir açıklığa ({b[0]:.3f}–{b[1]:.3f} m) "
                                       "biniyor. Açıklığı kaydırın ya da daraltın.", path)
        return None


# ── cad.openings.create ─────────────────────────────────────────────────────────────────────────────────────────


def create(d, inp, conflict=False):
    """(result, [fields] to write)."""
    if not inp["openings"]:
        return refusal("no_objects", "Yazılacak açıklık verilmedi. En az bir açıklığın duvarını ve yerini verin.", "openings"), None
    made = []
    for i, n in enumerate(inp["openings"]):
        at = f"openings[{i}]"
        for key, name in (("at", "Konum"), ("distance", "Uzaklık"), ("width", "Genişlik"), ("swing", "Açılma açısı")):
            if key in n and not math.isfinite(n[key]):
                return not_finite(name, f"{at}.{key}"), None
        if "point" in n:
            r = point_not_finite(n["point"], "Açıklığın noktası", f"{at}.point")
            if r:
                return r, None
        w = d.wall_named(n["host"], f"{at}.host")
        if isinstance(w, dict):
            return w, None
        _, line = w
        if "type" in n:
            t = find_type(n["type"])
            if t is None:
                return no_type(n["type"], f"{at}.type"), None
        else:
            t = first_type(n.get("form", "door"))
        f = {"host": n["host"], "at": 0.0, "anchor": "center", "hand": "start", "side": "left", "showSwing": True}
        apply_type(f, t)
        for key in ("form", "width", "leaves", "operation", "hand", "swing"):
            if key in n:
                f[key] = n[key]
        f["number"] = n.get("number")
        side = None
        if "at" in n:
            centre = n["at"]
        elif "point" in n:
            q = (n["point"]["x"], n["point"]["y"])
            s = line.nearest(q)
            p, t_ = line.at(s)
            cross = t_[0] * (q[1] - p[1]) - t_[1] * (q[0] - p[0])
            side = "right" if cross < 0 else "left"
            if "distance" in n:
                frm = n.get("from", "near")
                end = frm if frm != "near" else ("start" if s <= line.length / 2 else "end")
                centre = n["distance"] + f["width"] / 2 if end == "start" else line.length - n["distance"] - f["width"] / 2
            elif n.get("middle"):
                tol = 1e-9 * max(line.length, 1.0)
                k = next((k for k in range(len(line.edges)) if s <= line.cum[k] + line.edge_len(k) + tol), len(line.edges) - 1)
                centre = line.cum[k] + line.edge_len(k) / 2
            else:
                centre = s
        else:
            return refusal("invalid_opening", "Açıklığın yeri verilmedi: duvar boyunca uzaklığı (at) ya da duvarın yanında bir nokta (point) verin.", at), None
        f["at"] = centre
        f["side"] = n.get("side", side or "left")
        pr = problem(f)
        if pr:
            return refusal("invalid_opening", pr[1], f"{at}.{pr[0]}"), None
        r = placed(f, line, at)
        if "status" in r:
            return r, None
        made.append(r)
    r = d.clear([], [(f, f"openings[{i}]") for i, f in enumerate(made)])
    if r:
        return r, None
    if "lineWeight" in inp and not (math.isfinite(inp["lineWeight"]) and 0 <= inp["lineWeight"] <= 100):
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None
    r = revision(inp, conflict)
    if r:
        return r, None
    r, warnings = layer_check(inp["layerId"])
    if r:
        return r, None
    return {"status": "completed", "warnings": warnings}, made


def with_non_finite(inp, non_finite):
    inp = copy.deepcopy(inp)
    for path, word in non_finite.items():
        value = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}[word]
        parts = path.replace("]", "").replace("[", ".").split(".")
        node = inp
        for key in parts[:-1]:
            node = node[int(key)] if key.isdigit() else node[key]
        last = parts[-1]
        node[int(last) if last.isdigit() else last] = value
    return inp


def extra_of(inp):
    e = {}
    if "color" in inp:
        e["color"] = inp["color"]
    if "lineWeight" in inp:
        e["lineWeight"] = inp["lineWeight"]
    return e


def create_step(d, op, inp, non_finite=None, conflict=False, command=None):
    """The step, the drawing changed when it writes."""
    result, made = create(d, with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op}
    if command:
        s["command"] = command
    s["input"] = inp
    if non_finite:
        s["nonFinite"] = non_finite
    if made is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": result["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ents = [entity(0, inp["layerId"], f, extra_of(inp)) for f in made]
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"openings": ents, "revision": "$current"}, "warnings": result["warnings"]}
        s["within"] = 1e-9
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ids = list(range(d.next, d.next + len(made)))
    d.next += len(made)
    for i, f in zip(ids, made):
        d.openings[i] = f
        d.layers[i] = inp["layerId"]
    s["result"] = {"status": "completed", "output": {"uids": ["$any"] * len(made), "ids": ids, "revision": "$current"},
                   "warnings": result["warnings"]}
    s["within"] = 1e-9
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(i, inp["layerId"], f, extra_of(inp)) for i, f in zip(ids, made)},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def drawing_ids(d):
    kept = [i for i in IDS if BY_ID[i]["kind"] != "wall" or i in d.walls]
    return kept + sorted(i for i in list(d.openings) + [w for w in d.walls if w not in BY_ID])


Drawing.ids = drawing_ids


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps):
        assert len(name) > 10
        self.cases.append({"name": name, "steps": steps})


def one(host=1, layer="kapi", **kw):
    return {"layerId": layer, "openings": [{"host": uid_of(host), **kw}]}


def create_cases():
    c = Cases()
    d = Drawing()
    first = create_step(d, "execute", one(at=2.0))
    c.add("kapı: türü verilmezse Kapı 90, ortası duvar boyunca 2 m'de; pervazlar duvarın iki yüzünde; tek adım “Kapı”, geri alınır", [
        first,
        {"op": "undo", "returns": "Kapı", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}},
        {"op": "redo", "returns": "Kapı", "expect": {"ids": IDS + [NEW], "entities": first["expect"]["entities"], "canUndo": True,
                                                     "canRedo": False, "revision": "changed"}, "within": 1e-9},
    ])
    for name, inp in [
        ("pencere adıyla: kanatları, kasası, denizliği ve lentosu türünden; tek adım “Pencere”", one(at=5.0, type="Pencere 120")),
        ("sürme kapı kimliğiyle, köşeli duvarın ilk kenarında; sol hizalı duvarın yüzü yolun kendisi", one(host=2, at=3.0, type="kentos-surme-kapi-160")),
        ("duvarın yanındaki nokta: ortası yola en yakın yerde, kanat noktanın yanına (sağa) açılır", one(point=P(7, -1))),
        ("yakın uçtan uzaklık: yakın pervaz duvarın sonundan 0,5 m'de", one(point=P(7, 1), distance=0.5)),
        ("baştan uzaklık: yakın pervaz duvarın başından 1 m'de, kanat sola", one(point=P(7, 1), distance=1.0, **{"from": "start"})),
        ("kenarın ortası: noktanın yanındaki kenarın ortasında", one(host=2, point=P(4, 6), middle=True)),
        ("ikinci kenarın ortası: köşeli duvarın düşey kenarı", one(host=2, point=P(6.5, 8), middle=True)),
        ("düşey duvarda türün üstüne genişlik, menteşe, açı ve numara", one(host=3, at=3.0, width=1.0, hand="end", swing=120.0, number="K7")),
        ("boşluk biçimi türsüz: Boşluk 100; tek adım “Boşluk”", one(at=7.0, form="opening")),
        ("pencere biçimi türsüz: Pencere 120", one(at=7.0, form="window")),
        ("ötelemeli sağ hizalı duvar: yüzler yolun 0,25 ve 0,05 m solunda", one(host=7, at=3.0)),
        ("renk ve kalınlık açıklığın kendisinin", {"layerId": "kapi", "color": "#AA3300", "lineWeight": 0.35,
                                                   "openings": [{"host": uid_of(1), "at": 2.0}]}),
        ("gizli katmana uyarıyla yazılır", {"layerId": "gizli", "openings": [{"host": uid_of(1), "at": 2.0}]}),
    ]:
        c.add(name, [create_step(Drawing(), "execute", inp)])
    c.add("iki açıklık tek çağrıda, tek adım ilkinin biçimiyle “Kapı”", [create_step(Drawing(), "execute", {
        "layerId": "kapi", "openings": [{"host": uid_of(1), "at": 2.0}, {"host": uid_of(1), "at": 6.0, "type": "Pencere 120"}]})])
    c.add("plan yazmadan açıklığı pervazlarıyla gösterir", [create_step(Drawing(), "plan", one(at=2.0))])
    c.add("doğrulama yazmaz", [create_step(Drawing(), "validate", one(at=2.0))])
    for name, inp, kw in [
        ("açıklık verilmezse ret", {"layerId": "kapi", "openings": []}, {}),
        ("sonsuz konum reddedilir, yeriyle", one(at=2.0), {"non_finite": {"openings[0].at": "NaN"}}),
        ("sonsuz genişlik reddedilir", one(at=2.0, width=1.0), {"non_finite": {"openings[0].width": "Infinity"}}),
        ("sonsuz nokta reddedilir", one(point=P(2, 1)), {"non_finite": {"openings[0].point.x": "NaN"}}),
        ("geçersiz duvar kimliği reddedilir", {"layerId": "kapi", "openings": [{"host": "D-1", "at": 2.0}]}, {}),
        ("çizimde olmayan duvar reddedilir", {"layerId": "kapi", "openings": [{"host": MISSING, "at": 2.0}]}, {}),
        ("duvar olmayan nesneye açıklık konmaz", one(host=5, at=2.0), {}),
        ("olmayan tür reddedilir; var olanlar söylenir", one(at=2.0, type="Kapı 70"), {}),
        ("sıfır genişlik reddedilir", one(at=2.0, width=0.0), {}),
        ("üç kanat reddedilir", one(at=2.0, leaves=3), {}),
        ("180°'den büyük açı reddedilir", one(at=2.0, swing=200.0), {}),
        ("boş numara reddedilir", one(at=2.0, number=" "), {}),
        ("yeri verilmeyen açıklık reddedilir", one(), {}),
        ("duvarın ucundan taşan açıklık reddedilir", one(at=9.8), {}),
        ("köşeden dönen açıklık reddedilir", one(host=2, at=6.0), {}),
        ("kendisinden kısa duvara konmaz", one(host=6, at=0.25), {}),
        ("aynı çağrıda üst üste binen iki açıklık reddedilir", {"layerId": "kapi", "openings": [{"host": uid_of(1), "at": 2.0},
                                                                                                {"host": uid_of(1), "at": 2.5}]}, {}),
        ("kalınlık 100 mm'yi aşarsa ret", {"layerId": "kapi", "lineWeight": 120.0, "openings": [{"host": uid_of(1), "at": 2.0}]}, {}),
        ("kilitli katmana yazılmaz", {"layerId": "kilitli", "openings": [{"host": uid_of(1), "at": 2.0}]}, {}),
        ("katman grubuna yazılmaz", {"layerId": "pafta", "openings": [{"host": uid_of(1), "at": 2.0}]}, {}),
        ("olmayan katman reddedilir", {"layerId": "yok", "openings": [{"host": uid_of(1), "at": 2.0}]}, {}),
        ("geçersiz beklenen sürüm reddedilir", {"layerId": "kapi", "openings": [{"host": uid_of(1), "at": 2.0}], "expectedRevision": "07"}, {}),
    ]:
        c.add(name, [create_step(Drawing(), "execute", inp, **kw)])
    d = Drawing()
    made = create_step(d, "execute", one(at=2.0))
    c.add("duvarda var olan açıklığa binen açıklık reddedilir", [made, create_step(d, "execute", one(at=2.6))])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "kapi", "openings": [{"host": uid_of(1), "at": 5.0}]})
    stale = create_step(d, "execute", {"layerId": "kapi", "openings": [{"host": uid_of(1), "at": 2.0}], "expectedRevision": "$prepared"}, conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c.cases


# ── cad.openings.edit ───────────────────────────────────────────────────────────────────────────────────────────

CREATE = "cad.openings.create"


def made_with(d, openings, layer="kapi"):
    """A step that puts openings in with the create command, the drawing changed."""
    return create_step(d, "execute", {"layerId": layer, "openings": openings}, command=CREATE)


def named(d, uids):
    if not uids:
        return refusal("no_entities", "Düzenlenecek açıklık verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids")
    out, seen = [], set()
    for at, u in enumerate(uids):
        if not u.startswith("$uidOf:"):
            if len(u) != 36 or any(ch not in UID_TEXT for ch in u):
                return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", f"uids[{at}]")
            return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", f"uids[{at}]")
        i = int(u[len("$uidOf:"):])
        if i in seen:
            continue
        seen.add(i)
        out.append((at, i))
    return out


def type_matches(f, key):
    t = find_type(key)
    return f.get("style") is not None and (f["style"] == key or (t is not None and t["id"] == f["style"]))


def edit(d, inp, conflict=False):
    """(result, {id: fields} changed, [fields] added)."""
    op = inp["operation"]
    kind = op["kind"]
    uids = inp.get("uids", [])
    if not uids and (kind == "replaceType" and op.get("from") is not None or kind == "renumber"):
        if kind == "replaceType":
            pick = [i for i, f in sorted(d.openings.items()) if type_matches(f, op["from"])]
            if not pick:
                return refusal("style_not_used", f"Çizimde “{op['from']}” türünde açıklık yok.", "operation"), None, None
        else:
            pick = [i for i, f in sorted(d.openings.items()) if op.get("form") in (None, f["form"])]
            if not pick:
                return refusal("no_entities", "Çizimde numaralanacak açıklık yok.", "operation"), None, None
        uids = [uid_of(i) for i in pick]
    found = named(d, uids)
    if isinstance(found, dict):
        return found, None, None
    for at, i in found:
        if i not in d.openings:
            return refusal("not_an_opening", "Nesne kapı, pencere ya da boşluk değil; Açıklığı düzenle yalnız açıklıkları değiştirir.", f"uids[{at}]"), None, None
    nxt, added = [], []
    if kind == "set":
        ch = {k: v for k, v in op.items() if k != "kind"}
        if not ch:
            return refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi: genişlik, biçim, kanat, açılış, menteşe, yan, açı, açılım yayı, kasa, denizlik, lento ya da numara verin.", "operation"), None, None
        for key, name in (("width", "Genişlik"), ("swing", "Açılma açısı"), ("frame", "Kasa"), ("sill", "Denizlik"), ("head", "Lento")):
            if ch.get(key) is not None and not math.isfinite(ch[key]):
                return not_finite(name, f"operation.{key}"), None, None
        for at, i in found:
            f = dict(d.openings[i])
            f.update(ch)
            nxt.append((at, i, f, False))
    elif kind == "move":
        if "by" not in op and "at" not in op:
            return refusal("nothing_to_set", "Taşımanın uzaklığı verilmedi: duvar boyunca kaydırma (by) ya da yeni konum (at) verin.", "operation"), None, None
        for key in ("by", "at"):
            if key in op and not math.isfinite(op[key]):
                return not_finite("Uzaklık", f"operation.{key}"), None, None
        for at, i in found:
            f = dict(d.openings[i])
            f["at"] = op.get("at", f["at"]) + op.get("by", 0.0)
            nxt.append((at, i, f, True))
    elif kind == "flip":
        what = op.get("what", "side")
        for at, i in found:
            f = dict(d.openings[i])
            if what in ("side", "both"):
                f["side"] = OTHER[f.get("side", "left")]
            if what in ("hand", "both"):
                f["hand"] = OTHER[f.get("hand", "start")]
            nxt.append((at, i, f, False))
    elif kind == "rehost":
        if "point" in op:
            r = point_not_finite(op["point"], "Açıklığın noktası", "operation.point")
            if r:
                return r, None, None
        w = d.wall_named(op["to"], "operation.to")
        if isinstance(w, dict):
            return w, None, None
        _, line = w
        for at, i in found:
            f = dict(d.openings[i])
            f["host"] = op["to"]
            if "point" in op:
                f["at"] = at_of_centre(line.nearest((op["point"]["x"], op["point"]["y"])), f.get("anchor", "center"), f["width"])
            else:
                a, hand, side = read_back(line, f["place"], f.get("anchor", "center"), f["width"])
                f["at"] = a
                if hand:
                    f["hand"] = OTHER[f.get("hand", "start")]
                if side:
                    f["side"] = OTHER[f.get("side", "left")]
            f.pop("broken", None)
            nxt.append((at, i, f, True))
    elif kind == "replaceType":
        t = find_type(op["to"])
        if t is None:
            return no_type(op["to"], "operation.to"), None, None
        for at, i in found:
            if op.get("from") is not None and not type_matches(d.openings[i], op["from"]):
                continue
            f = dict(d.openings[i])
            apply_type(f, t)
            nxt.append((at, i, f, False))
        if not nxt:
            return refusal("style_not_used", f"Verilen açıklıkların hiçbiri “{op.get('from', '')}” türünde değil.", "operation.from"), None, None
    elif kind == "renumber":
        order = sorted(range(len(found)), key=lambda k: (
            int(d.openings[found[k][1]]["host"][len("$uidOf:"):]) if d.openings[found[k][1]]["host"].startswith("$uidOf:")
            and int(d.openings[found[k][1]]["host"][len("$uidOf:"):]) in d.walls else 10 ** 9,
            span(d.openings[found[k][1]])[0], k))
        for n, k in enumerate(order):
            at, i = found[k]
            f = dict(d.openings[i])
            f["number"] = f"{op.get('prefix', '')}{op.get('start', 1) + n}"
            nxt.append((at, i, f, False))
        nxt.sort(key=lambda x: x[0])
    elif kind == "copy":
        if not op["offsets"]:
            return refusal("nothing_to_set", "Kopyaların uzaklıkları verilmedi: duvar boyunca en az bir uzaklık verin.", "operation.offsets"), None, None
        for at, i in found:
            line = d.line_of(d.openings[i]["host"])
            if line is None:
                return refusal("opening_host_gone", "Açıklığın duvarı yok; duvar boyunca kopyalanamaz. Önce bir duvara taşıyın (Başka duvara).", f"uids[{at}]"), None, None
            for k, off in enumerate(op["offsets"]):
                f = dict(d.openings[i])
                f["at"] = f["at"] + off
                f["number"] = None
                path = f"operation.offsets[{k}]"
                pr = problem(f)
                if pr:
                    return refusal("invalid_opening", pr[1], f"{path}.{pr[0]}"), None, None
                r = placed(f, line, path)
                if "status" in r:
                    return r, None, None
                added.append((i, r, path))
    changed = {}
    for at, i, f, must in nxt:
        path = f"uids[{at}]"
        pr = problem(f)
        if pr:
            return refusal("invalid_opening", pr[1], f"{path}.{pr[0]}"), None, None
        line = d.line_of(f["host"])
        if line is not None:
            r = placed(f, line, path)
            if "status" in r:
                return r, None, None
            f = r
        elif must:
            return refusal("opening_host_gone", "Açıklığın duvarı yok; duvar boyunca taşınamaz. Önce bir duvara taşıyın (Başka duvara).", path), None, None
        else:
            f = dict(f, broken="hostGone")
        changed[i] = (f, path)
    r = d.clear([(i, f, p) for i, (f, p) in changed.items()], [(f, p) for _, f, p in added])
    if r:
        return r, None, None
    r = revision(inp, conflict)
    if r:
        return r, None, None
    for at, i in found:
        if d.layers[i] in d.locked:
            return refusal("layer_locked", f"“{LAYERS[d.layers[i]]}” katmanı kilitli; üzerindeki açıklık düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{at}]"), None, None
    return {"status": "completed"}, {i: f for i, (f, _) in changed.items()}, added


def edit_step(d, op, inp, non_finite=None, conflict=False):
    result, changed, added = edit(d, with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if changed is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ents = [entity(i, d.layers[i], f) for i, f in changed.items()]
    if op == "plan":
        out = {"openings": ents}
        if added:
            out["added"] = [entity(0, d.layers[i], f) for i, f, _ in added]
        out["revision"] = "$current"
        s["result"] = {"status": "completed", "output": out, "warnings": []}
        s["within"] = 1e-9
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    new = {}
    for i, f, _ in added:
        new[d.next] = (f, d.layers[i])
        d.next += 1
    for i, f in changed.items():
        d.openings[i] = f
    for j, (f, lay) in new.items():
        d.openings[j] = f
        d.layers[j] = lay
    out = {"openings": ents}
    if new:
        out["added"] = [entity(j, lay, f) for j, (f, lay) in new.items()]
    out["revision"] = "$current"
    s["result"] = {"status": "completed", "output": out, "warnings": []}
    s["within"] = 1e-9
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(i, d.layers[i], d.openings[i]) for i in list(changed) + list(new)},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def other_step(d, command, inp, change):
    """A step of another command whose openings follow: `change(d)` gives the openings it changes, the walls changed."""
    ids = change(d)
    return {"op": "execute", "command": command, "input": inp, "result": {"status": "completed", "output": "$any", "warnings": "$any"},
            "within": 1e-9, "expect": {"entities": {str(i): entity(i, d.layers[i], d.openings[i]) for i in ids}, "revision": "changed"}}


def follow(d, ids):
    """Each opening's jambs written again from its wall."""
    for i in ids:
        f = d.openings[i]
        line = d.line_of(f["host"])
        if line is None:
            d.openings[i] = dict(f, broken="hostGone")
            continue
        r = placed(f, line, "")
        d.openings[i] = dict(f, broken="offWall") if "status" in r else r
    return ids


def edit_cases():
    c = Cases()

    def fresh(*openings):
        d = Drawing()
        return d, made_with(d, list(openings))

    door = {"host": uid_of(1), "at": 2.0}
    first = NEW
    for name, op in [
        ("Değiştir: genişlik ortası yerinde kalarak büyür, numara yazılır", {"kind": "set", "width": 1.0, "number": "K2"}),
        ("Değiştir: açı, açılım yayı kapalı, kasa kaldırılır", {"kind": "set", "swing": 110.0, "showSwing": False, "frame": None}),
        ("Değiştir: biçim pencere, iki kanat, denizlik ve lento", {"kind": "set", "form": "window", "leaves": 2, "sill": 1.0, "head": 2.3}),
        ("Taşı: duvar boyunca 1,5 m", {"kind": "move", "by": 1.5}),
        ("Taşı: yeni konumu 6 m", {"kind": "move", "at": 6.0}),
        ("Çevir: açılış yanı", {"kind": "flip", "what": "side"}),
        ("Çevir: menteşe", {"kind": "flip", "what": "hand"}),
        ("Çevir: ikisi", {"kind": "flip", "what": "both"}),
        ("Başka duvara: düşey duvarda noktanın yanına", {"kind": "rehost", "to": uid_of(3), "point": P(20.5, 3)}),
        ("Başka duvara noktasız: ters yönlü duvarda pervazlarının ortasına en yakın yer; menteşe ve yan çevrilir", {"kind": "rehost", "to": uid_of(8)}),
        ("Türü değiştir: Kapı 80'in değerleri", {"kind": "replaceType", "to": "Kapı 80"}),
        ("Kopyala: duvar boyunca iki yana, numarasız", {"kind": "copy", "offsets": [1.5, 4.0]}),
    ]:
        d, made = fresh(door)
        c.add(name, [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": op})])
    d, made = fresh(door)
    step = edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "move", "by": 1.0}})
    c.add("tek adım “Açıklığı düzenle”, geri alınır", [made, step,
                                                       {"op": "undo", "returns": "Açıklığı düzenle", "expect": {"canRedo": True, "revision": "changed"}}])
    d, made = fresh(door)
    c.add("plan yazmadan açıklığı olacağı gibi gösterir", [made, edit_step(d, "plan", {"uids": [uid_of(first)], "operation": {"kind": "move", "by": 1.0}})])
    d, made = fresh(door)
    c.add("doğrulama yazmaz", [made, edit_step(d, "validate", {"uids": [uid_of(first)], "operation": {"kind": "flip"}})])
    d, made = fresh({"host": uid_of(1), "at": 5.0}, {"host": uid_of(1), "at": 2.0}, {"host": uid_of(1), "at": 8.0, "type": "Pencere 120"},
                    {"host": uid_of(3), "at": 3.0})
    c.add("Numarala kimliksiz: bütün kapılar duvarların sırasıyla ve duvar boyunca K1, K2 …; pencere değişmez",
          [made, edit_step(d, "execute", {"operation": {"kind": "renumber", "prefix": "K", "start": 1, "form": "door"}})])
    d, made = fresh({"host": uid_of(1), "at": 2.0}, {"host": uid_of(1), "at": 5.0, "type": "Kapı 80"}, {"host": uid_of(3), "at": 3.0})
    c.add("Türü değiştir kimliksiz: Kapı 90 türündeki bütün açıklıklar",
          [made, edit_step(d, "execute", {"operation": {"kind": "replaceType", "from": "Kapı 90", "to": "İki kanatlı kapı 140"}})])
    # The document keeps them with their walls (ADR 0341 §3).
    d, made = fresh(door, {"host": uid_of(1), "at": 7.0})

    def moved(d):
        d.walls[1] = dict(d.walls[1], pts=[P(0, 1), P(10, 1)])
        return follow(d, [first, first + 1])
    c.add("duvar taşınınca açıklıkları onunla gider", [made, other_step(d, "cad.entities.transform",
                                                                        {"uids": [uid_of(1)], "transform": {"kind": "move", "dx": 0.0, "dy": 1.0}}, moved)])
    d, made = fresh(door)

    def widened(d):
        d.walls[1] = dict(d.walls[1], width=0.3)
        return follow(d, [first])
    c.add("duvar kalınlaşınca pervazlar yeni yüzlerinde", [made, other_step(d, "cad.walls.edit",
                                                                           {"uids": [uid_of(1)], "operation": {"kind": "set", "width": 0.3}}, widened)])
    d, made = fresh(door)

    def flipped(d):
        d.walls[1] = dict(d.walls[1], pts=[P(10, 0), P(0, 0)])
        f = d.openings[first]
        d.openings[first] = dict(f, at=8.0, hand="end", side="right")
        return follow(d, [first])
    c.add("duvar çevrilince kapı yerinde kalır: konumu tersten, menteşe ve yan çevrilir", [made, other_step(
        d, "cad.walls.edit", {"uids": [uid_of(1)], "operation": {"kind": "flip"}}, flipped)])
    d, made = fresh({"host": uid_of(1), "at": 7.0}, {"host": uid_of(1), "at": 2.0})

    def split(d):
        d.walls[1] = dict(d.walls[1], pts=[P(0, 0), P(5, 0)])
        d.walls[d.next] = dict(BY_ID[1], id=d.next, pts=[P(5, 0), P(10, 0)])
        d.openings[first] = dict(d.openings[first], host=uid_of(d.next), at=2.0)
        d.next += 1
        return follow(d, [first, first + 1])
    c.add("duvar bölününce ikinci parçadaki açıklık yeni duvara geçer, konumu parçanın başından", [made, other_step(
        d, "cad.walls.edit", {"uids": [uid_of(1)], "operation": {"kind": "split", "at": P(5, 0)}}, split)])
    d, made = fresh(door)

    def slid(d):
        d.openings[first] = dict(d.openings[first], at=3.0)
        return follow(d, [first])
    c.add("açıklığın kendisi taşınınca duvarına okunur: yeni konumu", [made, other_step(
        d, "cad.entities.transform", {"uids": [uid_of(first)], "transform": {"kind": "move", "dx": 1.0, "dy": 0.3}}, slid)])
    d, made = fresh(door)

    def mirrored(d):
        d.openings[first] = dict(d.openings[first], side="right")
        return follow(d, [first])
    c.add("açıklık duvarın ekseninde aynalanınca açılış yanı çevrilir", [made, other_step(
        d, "cad.entities.transform", {"uids": [uid_of(first)], "transform": {"kind": "mirror", "a": P(0, 0), "b": P(10, 0)}}, mirrored)])
    d, made = fresh(door)

    def gone(d):
        del d.walls[1]
        return follow(d, [first])
    host_gone = other_step(d, "cad.entities.delete", {"uids": [uid_of(1)]}, gone)
    # The wall's id after it is gone: the one captured before.
    for f in host_gone["expect"]["entities"].values():
        f["host"] = "$uid:duvar"
    d.openings[first]["host"] = "$uid:duvar"
    c.add("duvar silinince açıklık yerinde kalır, nedeni hostGone; duvar boyunca taşınmaz ama numarası değişir", [
        made, {"op": "captureUid", "as": "duvar", "id": 1}, host_gone,
        edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "move", "by": 1.0}}),
        edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "set", "number": "K9"}}),
    ])
    for name, inp, kw in [
        ("açıklık verilmezse ret", {"operation": {"kind": "flip"}}, {}),
        ("geçersiz kimlik reddedilir", {"uids": ["D-1"], "operation": {"kind": "flip"}}, {}),
        ("çizimde olmayan kimlik reddedilir", {"uids": [MISSING], "operation": {"kind": "flip"}}, {}),
        ("açıklık olmayan nesne reddedilir", {"uids": [uid_of(1)], "operation": {"kind": "flip"}}, {}),
        ("boş Değiştir reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set"}}, {}),
        ("uzaklıksız Taşı reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "move"}}, {}),
        ("sonsuz genişlik reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set", "width": 1.0}}, {"non_finite": {"operation.width": "NaN"}}),
        ("sonsuz kaydırma reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "move", "by": 1.0}}, {"non_finite": {"operation.by": "Infinity"}}),
        ("üç kanat reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set", "leaves": 3}}, {}),
        ("denizlikten alçak lento reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set", "sill": 1.0, "head": 0.8}}, {}),
        ("olmayan tür reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "replaceType", "to": "Kapı 70"}}, {}),
        ("verilenlerin hiçbiri o türde değilse ret", {"uids": [uid_of(first)], "operation": {"kind": "replaceType", "from": "Pencere 120", "to": "Kapı 80"}}, {}),
        ("duvar olmayan nesneye taşınmaz", {"uids": [uid_of(first)], "operation": {"kind": "rehost", "to": uid_of(5)}}, {}),
        ("duvardan taşan Taşı reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "move", "by": 10.0}}, {}),
        ("duvardan taşan kopya reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "copy", "offsets": [9.0]}}, {}),
        ("uzaklıksız Kopyala reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "copy", "offsets": []}}, {}),
        ("kendisine binen kopya reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "copy", "offsets": [0.5]}}, {}),
        ("geçersiz beklenen sürüm reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "flip"}, "expectedRevision": "07"}, {}),
    ]:
        d, made = fresh(door)
        c.add(name, [made, edit_step(d, "execute", inp, **kw)])
    d, made = fresh(door, {"host": uid_of(1), "at": 4.0})
    c.add("aynı duvardaki başka açıklığa binen Taşı reddedilir", [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "move", "at": 3.5}})])
    d, made = fresh(door)
    d.locked.add("kapi")
    c.add("kilitli katmandaki açıklık düzenlenmez", [
        made, {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "kapi", "value": True},
               "result": {"status": "completed", "output": "$any", "warnings": "$any"}},
        edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "flip"}})])
    d, made = fresh(door)
    stale = edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "flip"}, "expectedRevision": "$prepared"}, conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [
        {"op": "captureRevision", "as": "prepared"}, made, stale])
    return c.cases


def document(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note,
            "setup": SETUP, "cases": cases}


def files():
    note = ("ADR 0341 §1–§4. Beklenenler scripts/fixtures/opening_command_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "duvarlar düz, pervazlar taban çizgisinin noktası ve sol normali üstünde. Kurulumdaki çizimde 8 nesne var; yeni nesne 9'dan başlar.")
    return {
        "cad.openings.create.json": document(
            "cad.openings.create", "Açıklık ekle: kapı, pencere ve boşluk; tür, yer, yan, pervazlar; doğrulama, plan, yazma, geri alma, retler",
            note, create_cases()),
        "cad.openings.edit.json": document(
            "cad.openings.edit", "Açıklığı düzenle ve duvarla gitmek: değiştir, taşı, çevir, başka duvara, tür, numara, kopya; duvarın düzenlemeleri; retler",
            note, edit_cases()),
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
                print(f"{doc['command']}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
