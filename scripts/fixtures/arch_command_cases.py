#!/usr/bin/env python3
"""The grid, column, stair and ramp commands' shared cases (docs/adr/0343 §3, §4), from the ADR's rules, without KentOS
code.

Writes fixtures/commands/v1/cad.{grids,columns,stairs,ramps}.create.json and cad.arch.edit.json; `--check` compares and
writes nothing. The grids are straight and along the axes, so their crossings are exact; a stair's risers and a ramp's
run are the drawing reference's (`arch_cases.py`). The rules, as the ADR gives them:

- `cad.grids.create`: no grid (`no_objects`); each grid in order: its points' and bulges' numbers, its fields
  (`invalid_grid`: two points, apart, two finite bulges, a label, a bubble from 0 to 100 mm), its level; then the series:
  its ends, its spacings (finite, above 0), its first grid's length, its level, each grid `spacings` further to its side;
  a label not given is the one after the grid before it in the call, else after the drawing's last grid (by slot),
  else A (letters skip I and O, Z → AA; numbers one more; trailing digits one more); then the line weight, the
  revision, the layer;
- `cad.columns.create`: no column; each in order: its centre's and sizes' numbers, a place (a centre or grids), its
  fields (`invalid_column`), its grids (each an id, there, a grid: `invalid_uid`, `entity_not_found`, `not_a_grid`;
  crossing: `grids_parallel`; the crossing nearest its centre), its level; then the crossings: every pair of the grids
  given in their order, where both run; a column with grids has its place from their labels (a lettered one first);
  untagged columns take `tagStart` and on;
- `cad.stairs.create`, `cad.ramps.create`: their numbers, fields (`invalid_stair`, `invalid_ramp`), level; how they
  fit are warnings after the layer's: the drawing reference's checks and, a level with a height, `stair_rise_level`;
  a stair drawn from its height (docs/adr/0704 §1): riserHeight with height, maxRiser without it, a height or
  maxRiser not above 0 or above 100, more than 400 risers refused (`invalid_stair`); its risers given or
  ⌈height / maxRiser − 10⁻⁹⌉ (maxRiser 0.175 unless given), each height / risers; a cut height with a cut refused,
  else its cut ⌊cutHeight / riser + 10⁻⁹⌋ within 1 and the risers; fitted, its last point along its last segment to
  its last riser of a line long enough for all (the drawing reference's `fit_end`, exact), refused when that riser is
  not past the last turn; given risers over a given maxRiser `stair_riser_high`; railings left, right or both kept
  (docs/adr/0705 §2); a ramp fitted (ADR 0705 §1): without a slope or with no rise refused, else its last point along
  its last segment so that its sloped length (overlapping landings once) is its rise over its slope (the drawing
  reference's `ramp_fit`, exact), refused when that length is used up before its last landing ends;
- `cad.arch.edit`: the ids, these kinds only, the operation's fields (each for its kinds: `field_not_for_kind`), their
  numbers, each object's fields again; Numarala one kind of grids or columns; Akslara bağla and Bağı kopar columns only;
  the revision; a locked layer refusing all;
- a stair's rise bound to its level's height (docs/adr/0724 §1): `heightFromLevel` true draws it as that height given
  would (not with `height`; its level given, the project's, with a height); kept as `heightFromLevel` with its riser
  height the level's height over its risers, again whenever that height or its risers change; a riser height of its
  own, or false, frees it (its risers staying); a bound stair keeps its level; a curved stair's centre post `post`
  (§3): on a curved stair only, its diameter above 0, its radius under the stair's inner radius;
- the document keeps a column with grids at their crossing after every step, its place from their labels; a grid gone
  breaks it (`gridGone`) where it stands.
"""

import argparse
import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_cases as ac  # noqa: E402  (the stair's risers, the ramp's run)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
GRIDS, COLUMNS, STAIRS, RAMPS, EDIT = "cad.grids.create", "cad.columns.create", "cad.stairs.create", "cad.ramps.create", "cad.arch.edit"


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def num(x):
    """A number as Rust's `{}` writes an f64."""
    x = float(x)
    if x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def fixed(v, d):
    t = f"{abs(v):.{d}f}"
    return ("-" + t) if v < 0 and float(t) != 0 else t


def layer(id, name, kind="layer", visible=True, locked=False, children=None):
    return {"id": id, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


def uid_of(i):
    return f"$uidOf:{i}"


MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.06}]
LAYERS = {"aks": "Aks", "kolon": "Kolon", "merdiven": "Merdiven", "cizim": "Çizim", "kilitli": "Kilitli katman",
          "gizli": "Gizli katman", "sinir": "Sınır"}

# The drawing: kind, layer and own fields of each object.
SEED = {
    1: ("grid", "aks", {"pts": [P(0, -2), P(0, 12)], "name": "A"}),
    2: ("grid", "aks", {"pts": [P(6, -2), P(6, 12)], "name": "B"}),
    3: ("grid", "aks", {"pts": [P(-2, 0), P(14, 0)], "name": "1"}),
    4: ("grid", "aks", {"pts": [P(-2, 5), P(14, 5)], "name": "2"}),
    5: ("grid", "kilitli", {"pts": [P(30, 0), P(30, 5)], "name": "K"}),
    6: ("line", "cizim", {"a": P(0, 20), "b": P(5, 20)}),
    7: ("grid", "aks", {"pts": [P(12, -2), P(12, 12)], "name": "C"}),
    8: ("stair", "merdiven", {"pts": [P(20, 0), P(23.75, 0)], "width": 1.2, "risers": 16, "riserHeight": 0.175, "tread": 0.25}),
    9: ("ramp", "merdiven", {"pts": [P(20, 5), P(30, 5)], "width": 1.5, "rise": 0.5, "slope": 5.0}),
}
IDS = sorted(SEED)
NEW = max(IDS) + 1
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Taşıyıcılar",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "arch": {"levels": LEVELS}},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("aks", "Aks"), layer("kolon", "Kolon"), layer("merdiven", "Merdiven"), layer("cizim", "Çizim"),
               layer("kilitli", "Kilitli katman", locked=True), layer("gizli", "Gizli katman", visible=False),
               layer("pafta", "Pafta", kind="group", children=[layer("sinir", "Sınır")])],
    "activeLayer": "aks",
    "styles": {"items": [], "categories": []},
    "entities": [{"kind": k, "id": i, "layerId": ly, "attrs": {}, **f} for i, (k, ly, f) in sorted(SEED.items())],
}

# ── Labels ─────────────────────────────────────────────────────────────────────────────────────────────────────────

LETTERS = "ABCDEFGHJKLMNPQRSTUVWXYZ"


def next_label(t):
    t = t.strip()
    if t and t.isascii() and t.isdigit():
        return str(int(t) + 1)
    if t and all("A" <= ch <= "Z" for ch in t):
        digits = [LETTERS.index(ch) if ch in LETTERS else max(sum(1 for l in LETTERS if l < ch) - 1, 0) for ch in t]
        i = len(digits)
        while True:
            if i == 0:
                digits.insert(0, 0)
                break
            i -= 1
            if digits[i] + 1 < len(LETTERS):
                digits[i] += 1
                break
            digits[i] = 0
        return "".join(LETTERS[d] for d in digits)
    k = len(t)
    while k > 0 and t[k - 1].isdigit():
        k -= 1
    if k < len(t):
        tail = t[k:]
        return t[:k] + str(int(tail) + 1).zfill(len(tail))
    return t


def place_of(a, b):
    lettered = lambda s: s[:1].isalpha()
    return f"{b}-{a}" if not lettered(a) and lettered(b) else f"{a}-{b}"

# ── The drawing as the steps leave it ──────────────────────────────────────────────────────────────────────────


class Drawing:
    def __init__(self):
        self.objs = {i: (k, ly, copy.deepcopy(f)) for i, (k, ly, f) in SEED.items()}
        self.next = NEW

    def ids(self):
        return sorted(self.objs)

    def grids(self):
        return [(i, f) for i, (k, _, f) in sorted(self.objs.items()) if k == "grid"]

    def grid_by_uid(self, u):
        if not u.startswith("$uidOf:"):
            return None
        i = int(u[len("$uidOf:"):])
        o = self.objs.get(i)
        return o[2] if o and o[0] == "grid" else None

    def follow(self):
        """The document's follower (ADR 0343 §3): every column with grids at their crossing; returns those changed."""
        changed = []
        for i, (k, ly, f) in sorted(self.objs.items()):
            if k != "column" or not (f.get("grids") or f.get("place") or f.get("broken")):
                continue
            g = copy.deepcopy(f)
            settle(self, g)
            if g != f:
                self.objs[i] = (k, ly, g)
                changed.append(i)
        return changed


def cross(a, b):
    """Where two straight grids cross, their lines unbounded (exact fractions); None: parallel."""
    (p, q), (r, s) = [[(F(v["x"]), F(v["y"])) for v in g["pts"]] for g in (a, b)]
    d1 = (q[0] - p[0], q[1] - p[1])
    d2 = (s[0] - r[0], s[1] - r[1])
    den = d1[0] * d2[1] - d1[1] * d2[0]
    if den == 0:
        return None
    t = ((r[0] - p[0]) * d2[1] - (r[1] - p[1]) * d2[0]) / den
    return (p[0] + d1[0] * t, p[1] + d1[1] * t)


def within(g, pt):
    (p, q) = [(F(v["x"]), F(v["y"])) for v in g["pts"]]
    return min(p[0], q[0]) <= pt[0] <= max(p[0], q[0]) and min(p[1], q[1]) <= pt[1] <= max(p[1], q[1])


def settle(d, c):
    """A column on its grids as the document keeps it."""
    gs = c.get("grids")
    if not gs or len(gs) != 2:
        c.pop("place", None)
        c.pop("broken", None)
        return
    a, b = d.grid_by_uid(gs[0]), d.grid_by_uid(gs[1])
    p = cross(a, b) if a and b else None
    if p is None:
        c["broken"] = "gridGone"
        return
    c["at"] = P(p[0], p[1])
    c["place"] = place_of(a["name"], b["name"])
    c.pop("broken", None)


def entity(d, i, extra=None):
    k, ly, f = d.objs[i]
    return {"kind": k, "id": i, "layerId": ly, "attrs": {}, **(extra or {}), **f}


def made_entity(kind, layer_id, f, extra=None):
    return {"kind": kind, "id": 0, "layerId": layer_id, "attrs": {}, **(extra or {}), **f}

# ── Shared checks ──────────────────────────────────────────────────────────────────────────────────────────────


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


def points_not_finite(pts, path):
    for k, p in enumerate(pts):
        r = point_not_finite(p, f"{k + 1}. noktanın", f"{path}.pts[{k}]")
        if r:
            return r
    return None


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


def finite_each(values, path):
    for v, field, name in values:
        if v is not None and not math.isfinite(v):
            return not_finite(name, f"{path}.{field}")
    return None


def level_check(arch, path):
    level = (arch or {}).get("level")
    if level is None or any(lv["id"] == level for lv in LEVELS):
        return None
    return refusal("level_not_found", f"Projede “{level}” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.", path)


def positive(v, field, whose, top, code, path):
    if not (math.isfinite(v) and 0 < v <= top):
        return refusal(code, f"{whose} {num(v)}; sıfırdan büyük, en çok {num(top)} olmalı.", f"{path}.{field}")
    return None


def blank(v, field, code, path):
    if v is not None and v.strip() == "":
        return refusal(code, "Boş metin yazılmaz; alanı kaldırın ya da bir değer yazın.", f"{path}.{field}")
    return None


def is_uid(u):
    return u.startswith("$uidOf:") or (len(u) == 36 and all(ch in "0123456789abcdef-" for ch in u))

# ── Each kind's problems, as the contract says them ────────────────────────────────────────────────────────────


def grid_problem(g, path):
    code = "invalid_grid"
    n = len(g["pts"])
    if n != 2:
        return refusal(code, f"Aks iki noktadır (başı ve sonu); {n} verildi.", f"{path}.pts")
    if g["pts"][0] == g["pts"][1]:
        return refusal(code, "Aksın başı ve sonu aynı; aksın bir boyu olmalı.", f"{path}.pts")
    b = g.get("bulges")
    if b is not None and len(b) != 2:
        return refusal(code, "Aksın iki noktası ve iki sonlu bükümü olur.", f"{path}.bulges")
    if g["name"].strip() == "":
        return refusal(code, "Aksın etiketi boş olamaz.", f"{path}.name")
    if g.get("bubbleSize") is not None:
        return positive(g["bubbleSize"], "bubbleSize", "Balonun çapı", 100, code, path)
    return None


def column_problem(c, path):
    code = "invalid_column"
    r = positive(c["width"], "width", "Kolonun eni", 100, code, path)
    if r:
        return r
    if c.get("depth") is not None:
        r = positive(c["depth"], "depth", "Kolonun boyu", 100, code, path)
        if r:
            return r
    g = c.get("grids")
    if g is not None and (len(g) != 2 or g[0] == g[1]):
        return refusal(code, "Kolon iki ayrı aksın kesişimine bağlanır.", f"{path}.grids")
    return blank(c.get("type"), "type", code, path) or blank(c.get("tag"), "tag", code, path)


def flight_problem(pts, what, code, path):
    n = len(pts)
    if not 2 <= n <= 4:
        return refusal(code, f"{what} yolu 2, 3 ya da 4 noktadır (düz, L, U); {n} verildi.", f"{path}.pts")
    for i in range(n - 1):
        if pts[i] == pts[i + 1]:
            return refusal(code, f"{what} yolunun {i + 1}. ve {i + 2}. noktası aynı.", f"{path}.pts[{i + 1}]")
    return None


def arc_problem(s, path):
    """docs/adr/0709: a curved stair's arc: two points, two finite bulges (its arc's, not 0, then 0), its walking line's
    radius more than half its width."""
    b = s.get("bulges")
    if b is None:
        return None
    code, at = "invalid_stair", f"{path}.bulges"
    if len(s["pts"]) != 2:
        return refusal(code, "Döner merdivenin yürüme çizgisi iki noktadır (başı ve sonu) ve bir yaydır.", at)
    if len(b) != 2 or not all(math.isfinite(x) for x in b) or b[1] != 0.0 or b[0] == 0.0:
        return refusal(code, "Döner merdivenin iki bükümü olur: ilki yayınki (sıfır değil, sonlu), ikincisi 0.", at)
    (ax, ay), (bx, by) = [(q["x"], q["y"]) for q in s["pts"]]
    chord = math.sqrt((bx - ax) * (bx - ax) + (by - ay) * (by - ay))
    r = chord * (1.0 + b[0] * b[0]) / (4.0 * abs(b[0]))
    if math.isfinite(s["width"]) and r - s["width"] / 2.0 <= 1e-9:
        return refusal(code, f"Yürüme çizgisinin yarıçapı {r:.3f} m; merdivenin genişliğinin yarısından ({s['width'] / 2.0:.3f} m) büyük olmalı.", at)
    return None


def post_problem(s, path):
    """docs/adr/0724 §3: a centre post on a curved stair only, its diameter above 0 and its radius less than the stair's
    inner radius (the walking line's radius less half its width)."""
    d = s.get("post")
    if d is None:
        return None
    at = f"{path}.post"
    code = "invalid_stair"
    b = (s.get("bulges") or [0.0])[0]
    if len(s["pts"]) != 2 or b == 0.0 or not math.isfinite(b):
        return refusal(code, "Merkez direği yalnız döner merdivende olur.", at)
    if not (math.isfinite(d) and d > 0.0):
        return refusal(code, f"Merkez direğinin çapı {num(d)}; sıfırdan büyük, sonlu olmalı.", at)
    (ax, ay), (bx, by) = [(q["x"], q["y"]) for q in s["pts"]]
    r = math.sqrt((bx - ax) * (bx - ax) + (by - ay) * (by - ay)) * (1.0 + b * b) / (4.0 * abs(b))
    inner = r - s["width"] / 2.0
    if d / 2.0 >= inner - 1e-9:
        return refusal(code, f"Merkez direğinin yarıçapı {d / 2.0:.3f} m; merdivenin iç yarıçapından ({inner:.3f} m) küçük olmalı.", at)
    return None


def stair_problem(s, path):
    code = "invalid_stair"
    r = flight_problem(s["pts"], "Merdivenin", code, path) or arc_problem(s, path)
    if r:
        return r
    for field, whose in (("width", "Merdivenin genişliği"), ("riserHeight", "Rıht yüksekliği"), ("tread", "Basamak derinliği")):
        r = positive(s[field], field, whose, 100, code, path)
        if r:
            return r
    if not 1 <= s["risers"] <= 400:
        return refusal(code, f"Rıht sayısı {s['risers']}; 1 ile 400 arasında olmalı.", f"{path}.risers")
    if s.get("landing") is not None:
        r = positive(s["landing"], "landing", "Sahanlığın boyu", 100, code, path)
        if r:
            return r
    c = s.get("cut")
    if c is not None and not 1 <= c <= s["risers"]:
        return refusal(code, f"Kesme çizgisi {c}. rıhtta; 1 ile {s['risers']} arasında olmalı.", f"{path}.cut")
    r = post_problem(s, path)
    if r:
        return r
    if s.get("heightFromLevel") and (s.get("arch") or {}).get("level") is None:
        return refusal(code, "Merdivenin yüksekliği kata bağlı ama merdivenin katı yok. Merdivene kat verin ya da bağı kaldırın.",
                       f"{path}.heightFromLevel")
    return None


def level_of(id, levels=None):
    return next((lv for lv in levels or LEVELS if lv["id"] == id), None)


def stair_bound(f, levels=None):
    """A stair's rise its level's height (docs/adr/0724 §1): its riser height that height over its risers, as a double
    divided; none to do when it is not bound or its level has no height."""
    if not f.get("heightFromLevel") or not f.get("risers"):
        return
    lv = level_of((f.get("arch") or {}).get("level"), levels)
    h = lv.get("height") if lv else None
    if h is not None and math.isfinite(h) and h > 0:
        f["riserHeight"] = float(h) / float(f["risers"])


def ramp_problem(r, path):
    code = "invalid_ramp"
    p = flight_problem(r["pts"], "Rampanın", code, path)
    if p:
        return p
    p = positive(r["width"], "width", "Rampanın genişliği", 100, code, path)
    if p:
        return p
    if not (math.isfinite(r["rise"]) and abs(r["rise"]) <= 1000):
        return refusal(code, f"Rampanın yüksekliği {num(r['rise'])}; sonlu ve en çok 1000 m olmalı.", f"{path}.rise")
    if r.get("slope") is not None:
        p = positive(r["slope"], "slope", "Rampanın eğimi (%)", 1000, code, path)
        if p:
            return p
    if r.get("baseOffset") is not None:
        # docs/adr/0723 §2: a ramp bound to its level: the offset finite, at most 1000 m, the ramp's level given.
        o = r["baseOffset"]
        if not (math.isfinite(o) and abs(o) <= 1000):
            return refusal(code, f"Rampanın kata göre başlangıç kotu {num(o)}; sonlu ve en çok 1000 m olmalı.", f"{path}.baseOffset")
        if (r.get("arch") or {}).get("level") is None:
            return refusal(code, "Rampanın başlangıç kotu kata göre verildi ama rampanın katı yok. Rampaya kat verin ya da "
                                 "başlangıç kotunu kendisi olarak yazın.", f"{path}.baseOffset")
    if r.get("landing") is not None:
        p = positive(r["landing"], "landing", "Sahanlığın boyu", 100, code, path)
        if p:
            return p
    return None


BOTH_BASES = "Başlangıç kotu ya kendisi (base) ya da kata göre (baseOffset) verilir; ikisi birden verilmez."


def level_bound(f, levels=None):
    """A ramp bound to its level with its base read from it (docs/adr/0723 §2): the level's elevation plus its offset,
    added as doubles in that order."""
    if f.get("baseOffset") is None:
        return
    level = (f.get("arch") or {}).get("level")
    for lv in levels or LEVELS:
        if lv["id"] == level:
            f["base"] = float(lv["elevation"]) + float(f["baseOffset"])


def fpts(pts):
    return [(F(p["x"]), F(p["y"])) for p in pts]


def stair_warnings(s, path):
    """The drawing reference's checks, then the level's height (ADR 0343 §2)."""
    out = []
    w = lambda code, words: out.append({"code": code, "message": words, "path": path})
    if s.get("bulges"):
        # A curved stair (docs/adr/0709): its risers a tread apart along its arc, no landings.
        (a, b) = [(q["x"], q["y"]) for q in s["pts"]]
        _, rad, _, sweep = ac.arc_of(a, b, s["bulges"][0])
        total = rad * abs(sweep)
        st = [k * F(s["tread"]) for k in range(s["risers"]) if float(k * F(s["tread"])) <= total + 1e-9]
        missing = s["risers"] - len(st)
        if missing:
            w("stair_short", f"Yürüme çizgisi ({fixed(total, 2)} m) {s['risers']} rıhta yetmiyor: {missing} rıht sığmıyor. Yolu uzatın, basamağı kısaltın ya da rıht sayısını azaltın.")
        elif st and total - float(st[-1]) > 1e-6:
            w("stair_long", f"Son rıhttan sonra yürüme çizgisinde {fixed(total - float(st[-1]), 2)} m kalıyor; merdiven yoldan kısa.")
        st = None
    else:
        st = True
    pts = fpts(s["pts"])
    width = F(s["width"])
    land = F(s["landing"]) if s.get("landing") is not None else width
    if st:
        st, missing, at = ac.risers(pts, width, s["risers"], F(s["tread"]), land)
        if missing:
            w("stair_short", f"Yürüme çizgisi ({fixed(float(at[-1]), 2)} m) {s['risers']} rıhta yetmiyor: {missing} rıht sığmıyor. Yolu uzatın, basamağı kısaltın ya da rıht sayısını azaltın.")
        elif st and float(at[-1] - st[-1]) > 1e-6:
            w("stair_long", f"Son rıhttan sonra yürüme çizgisinde {fixed(float(at[-1] - st[-1]), 2)} m kalıyor; merdiven yoldan kısa.")
    rule = 2 * s["riserHeight"] + s["tread"]
    if rule < 0.60 - 1e-9 or rule > 0.65 + 1e-9:
        w("stair_step_rule", f"2 × rıht + basamak = {fixed(rule, 3)} m; Blondel'in oranı 0.60–0.65 m. Bu bir bilgidir, yönetmelik denetimi değildir.")
    level = (s.get("arch") or {}).get("level")
    h = next((lv.get("height") for lv in LEVELS if lv["id"] == level), None)
    if h is not None:
        rise = s["risers"] * s["riserHeight"]
        if abs(rise - h) > 1e-3:
            w("stair_rise_level", f"{s['risers']} rıht × {fixed(s['riserHeight'], 3)} m = {fixed(rise, 3)} m; katın yüksekliği {fixed(h, 3)} m. Rıht sayısını ya da yüksekliğini düzeltin.")
    return out


def ramp_warnings(r, path):
    pts = fpts(r["pts"])
    width = F(r["width"])
    land = F(r["landing"]) if r.get("landing") is not None else width
    at = ac.walk(pts)
    run = at[-1] - sum(b - a for a, b in ac.ramp_landings(pts, at, land))
    if run <= F(1, 10**9):
        return [{"code": "ramp_no_run", "message": "Sahanlıklardan sonra rampanın eğimli kolu kalmıyor; yolu uzatın ya da sahanlığı kısaltın.", "path": path}]
    actual = r["rise"] / float(run) * 100
    if r.get("slope") is not None and abs(r["slope"] - actual) > 0.1:
        return [{"code": "ramp_slope_mismatch", "message": f"Rampanın eğimi %{fixed(r['slope'], 2)} verildi; yükseklik ve eğimli uzunlukla %{fixed(actual, 2)}.", "path": path}]
    return []

# ── The creates ─────────────────────────────────────────────────────────────────────────────────────────────────


def finish(inp, conflict, made, warnings):
    lw = inp.get("lineWeight")
    if lw is not None and not 0 <= lw <= 100:
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None
    r = revision(inp, conflict)
    if r:
        return r, None
    r, lw_warn = layer_check(inp["layerId"])
    if r:
        return r, None
    return {"warnings": lw_warn + warnings}, made


def last_label(d):
    gs = d.grids()
    return gs[-1][1]["name"] if gs else None


def after(prev):
    return next_label(prev) if prev is not None else "A"


def grids_create(d, inp, conflict=False):
    series = inp.get("series")
    if not inp.get("grids") and series is None and inp.get("system") is None:
        return refusal("no_objects", "Yazılacak aks verilmedi. Akslar (grids), bir seri (series) ya da bir aks ağı (system) verin.", "grids"), None
    prev = last_label(d)
    made = []
    for i, n in enumerate(inp.get("grids", [])):
        at = f"grids[{i}]"
        r = points_not_finite(n["pts"], at)
        for k, b in enumerate(n.get("bulges") or []):
            r = r or (not_finite("Büküm", f"{at}.bulges[{k}]") if not math.isfinite(b) else None)
        r = r or finite_each([(n.get("bubbleSize"), "bubbleSize", "Balon çapı")], at)
        if r:
            return r, None
        g = {"pts": n["pts"]}
        if n.get("bulges") is not None:
            g["bulges"] = n["bulges"]
        g["name"] = n["label"] if n.get("label") is not None else after(prev)
        if n.get("bubbles", "both") != "both":
            g["bubbles"] = n["bubbles"]
        if n.get("bubbleSize") is not None:
            g["bubbleSize"] = n["bubbleSize"]
        if n.get("arch"):
            g["arch"] = n["arch"]
        r = grid_problem(g, at) or level_check(g.get("arch"), f"{at}.arch.level")
        if r:
            return r, None
        prev = g["name"]
        made.append(g)
    if series is not None:
        r = point_not_finite(series["from"], "Serinin başının", "series.from") or point_not_finite(series["to"], "Serinin sonunun", "series.to")
        if r:
            return r, None
        for k, s in enumerate(series["spacings"]):
            if not math.isfinite(s):
                return not_finite("Aralık", f"series.spacings[{k}]"), None
            if s <= 0:
                return refusal("invalid_grid", f"Akslar arası {num(s)}; sıfırdan büyük olmalı.", f"series.spacings[{k}]"), None
        r = finite_each([(series.get("bubbleSize"), "bubbleSize", "Balon çapı")], "series")
        if r:
            return r, None
        a, b = series["from"], series["to"]
        dx, dy = b["x"] - a["x"], b["y"] - a["y"]
        ln = math.hypot(dx, dy)
        if ln == 0:
            return refusal("invalid_grid", "Serinin ilk aksının başı ve sonu aynı; aksın bir boyu olmalı.", "series.to"), None
        nx, ny = (-dy / ln, dx / ln) if series.get("side", "left") == "left" else (dy / ln, -dx / ln)
        r = level_check(series.get("arch"), "series.arch.level")
        if r:
            return r, None
        offset = 0.0
        label = series["start"] if series.get("start") is not None else after(prev)
        for k in range(len(series["spacings"]) + 1):
            if k:
                offset += series["spacings"][k - 1]
                label = next_label(label)
            g = {"pts": [P(p["x"] + nx * offset, p["y"] + ny * offset) for p in (a, b)], "name": label}
            if series.get("bubbles", "both") != "both":
                g["bubbles"] = series["bubbles"]
            if series.get("bubbleSize") is not None:
                g["bubbleSize"] = series["bubbleSize"]
            if series.get("arch"):
                g["arch"] = series["arch"]
            r = grid_problem(g, "series")
            if r:
                return r, None
            made.append(g)
    if inp.get("system") is not None:
        r = system_made(inp["system"], made)
        if r:
            return r, None
    return finish(inp, conflict, made, [])


def system_made(s, made):
    """A grid system (ADR 0707): its grids across x, then across y, then a column at every crossing row by row; a
    column names its grids as `@k`, the index of the grid among the objects made."""
    r = point_not_finite(s["origin"], "Aks ağının başlangıcının", "system.origin")
    r = r or finite_each([(s.get("angle"), "angle", "Açı"), (s.get("extension"), "extension", "Uzatma"),
                          (s.get("bubbleSize"), "bubbleSize", "Balon çapı")], "system")
    if r:
        return r
    for axis in ("x", "y"):
        for k, gap in enumerate(s.get(axis, [])):
            if not math.isfinite(gap):
                return not_finite("Aralık", f"system.{axis}[{k}]")
            if gap <= 0:
                return refusal("invalid_grid", f"Akslar arası {num(gap)}; sıfırdan büyük olmalı.", f"system.{axis}[{k}]")
    cols = s.get("columns")
    if cols is not None:
        r = finite_each([(cols["width"], "width", "En"), (cols.get("depth"), "depth", "Boy"),
                         (cols.get("rotation"), "rotation", "Dönüş")], "system.columns")
        if r:
            return r
    e = s.get("extension", 1.5)
    if e < 0:
        return refusal("invalid_grid", f"Uzatma {num(e)}; sıfır ya da daha büyük olmalı.", "system.extension")
    if e == 0 and (not s.get("x") or not s.get("y")):
        return refusal("invalid_grid", "Bir yönde tek aks varken uzatma sıfır olursa öbür yönün aksları nokta olur; uzatma verin.", "system.extension")
    for key in ("xStart", "yStart"):
        if s.get(key) is not None and s[key].strip() == "":
            return refusal("invalid_grid", "Aksın ilk adı boş; 1, A gibi bir ad verin.", f"system.{key}")
    r = level_check(s.get("arch"), "system.arch.level")
    if r:
        return r

    def along(gaps):
        at, out = 0.0, [0.0]
        for g in gaps:
            at += g
            out.append(at)
        return out

    xs, ys = along(s.get("x", [])), along(s.get("y", []))
    angle = s.get("angle", 0.0)
    c, sn = (1.0, 0.0) if angle == 0 else (math.cos(angle), math.sin(angle))
    o = s["origin"]

    def world(x, y):
        return P(o["x"] + (x * c - y * sn), o["y"] + (x * sn + y * c))

    def family(positions, start, first, across_x):
        label, out = start if start is not None else first, []
        for i, v in enumerate(positions):
            if i:
                label = next_label(label)
            g = {"pts": [world(v, -e), world(v, ys[-1] + e)] if across_x else [world(-e, v), world(xs[-1] + e, v)],
                 "name": label}
            if s.get("bubbles", "both") != "both":
                g["bubbles"] = s["bubbles"]
            if s.get("bubbleSize") is not None:
                g["bubbleSize"] = s["bubbleSize"]
            if s.get("arch"):
                g["arch"] = s["arch"]
            out.append(g)
        return out

    across_x = family(xs, s.get("xStart"), "1", True)
    across_y = family(ys, s.get("yStart"), "A", False)
    first = len(made)
    for g in across_x + across_y:
        r = grid_problem(g, "system")
        if r:
            return r
        made.append(g)
    if cols is None:
        return None
    tag = cols.get("tagStart")
    for j, gy in enumerate(across_y):
        for i, gx in enumerate(across_x):
            pt = cross(gx, gy)
            n = {**cols, "rotation": cols.get("rotation", angle)}
            col = column_fields(n, tag, P(pt[0], pt[1]))
            if tag is not None:
                tag = next_label(tag)
            col["grids"] = [f"@{first + i}", f"@{first + len(across_x) + j}"]
            col["place"] = place_of(gx["name"], gy["name"])
            if s.get("arch"):
                col["arch"] = s["arch"]
            r = column_problem(col, "system.columns")
            if r:
                return r
            made.append(("column", col))
    return None


def grid_named(d, u, path):
    if not is_uid(u):
        return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin.", path), None
    if u == MISSING or not u.startswith("$uidOf:") or int(u[7:]) not in d.objs:
        return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok. Var olan bir aksın kimliğini verin.", path), None
    g = d.grid_by_uid(u)
    if g is None:
        return refusal("not_a_grid", "Nesne aks değil; kolon yalnız iki aksın kesişimine bağlanır.", path), None
    return None, g


def settled_on(d, c, path):
    gs = c.get("grids")
    if gs is None:
        return None
    if len(gs) != 2 or gs[0] == gs[1]:
        return refusal("invalid_column", "Kolon iki ayrı aksın kesişimine bağlanır; iki aks kimliği verin.", f"{path}.grids")
    r, a = grid_named(d, gs[0], f"{path}.grids[0]")
    if r:
        return r
    r, _ = grid_named(d, gs[1], f"{path}.grids[1]")
    if r:
        return r
    settle(d, c)
    if c.get("broken"):
        return refusal("grids_parallel", "İki aks kesişmiyor (paralel ya da yayları ayrık); kolon onların kesişimine konamaz.", f"{path}.grids")
    return None


def column_fields(n, tag, at):
    c = {"at": at}
    if n.get("shape", "rect") != "rect":
        c["shape"] = n["shape"]
    c["width"] = n["width"]
    if n.get("depth") is not None:
        c["depth"] = n["depth"]
    if n.get("rotation"):
        c["rotation"] = n["rotation"]
    if n.get("type") is not None:
        c["type"] = n["type"]
    if tag is not None:
        c["tag"] = tag
    if n.get("fill"):
        c["fill"] = True
    if n.get("grids") is not None:
        c["grids"] = n["grids"]
    if n.get("arch"):
        c["arch"] = n["arch"]
    return c


def columns_create(d, inp, conflict=False):
    if not inp.get("columns") and inp.get("crossings") is None:
        return refusal("no_objects", "Yazılacak kolon verilmedi. Kolonlar (columns) ya da aks kesişimleri (crossings) verin.", "columns"), None
    tag = [inp.get("tagStart")]

    def next_tag(own):
        if own is not None:
            return own
        if tag[0] is None:
            return None
        t = tag[0]
        tag[0] = next_label(t)
        return t
    made = []
    for i, n in enumerate(inp.get("columns", [])):
        at = f"columns[{i}]"
        r = (point_not_finite(n["at"], "Kolonun merkezinin", f"{at}.at") if n.get("at") else None) or \
            finite_each([(n["width"], "width", "En"), (n.get("depth"), "depth", "Boy"), (n.get("rotation"), "rotation", "Dönüş")], at)
        if r:
            return r, None
        if n.get("at") is None and n.get("grids") is None:
            return refusal("invalid_column", "Kolonun yeri verilmedi: merkezini (at) ya da iki aksını (grids) verin.", at), None
        c = column_fields(n, next_tag(n.get("tag")), n.get("at") or P(0, 0))
        r = column_problem(c, at)
        if r:
            return r, None
        if n.get("at") is None and c.get("grids"):
            g = d.grid_by_uid(c["grids"][0])
            if g:
                c["at"] = g["pts"][0]
        r = settled_on(d, c, at) or level_check(c.get("arch"), f"{at}.arch.level")
        if r:
            return r, None
        made.append(c)
    x = inp.get("crossings")
    if x is not None:
        r = finite_each([(x["width"], "width", "En"), (x.get("depth"), "depth", "Boy"), (x.get("rotation"), "rotation", "Dönüş")], "crossings")
        if r:
            return r, None
        if len(x["grids"]) < 2:
            return refusal("invalid_column", "Aks kesişimleri için en az iki aks verin.", "crossings.grids"), None
        grids = []
        for k, u in enumerate(x["grids"]):
            r, g = grid_named(d, u, f"crossings.grids[{k}]")
            if r:
                return r, None
            grids.append((u, g))
        r = level_check(x.get("arch"), "crossings.arch.level")
        if r:
            return r, None
        for i, (ua, ga) in enumerate(grids):
            for ub, gb in grids[i + 1:]:
                p = cross(ga, gb)
                if p is None or not (within(ga, p) and within(gb, p)):
                    continue
                c = column_fields({**x, "grids": [ua, ub]}, next_tag(None), P(p[0], p[1]))
                c["place"] = place_of(ga["name"], gb["name"])
                made.append(c)
    return finish(inp, conflict, made, [])


def stair_fields(n, risers=None, riser=None):
    s = {"pts": n["pts"], "width": n["width"], "risers": n.get("risers", 0) if risers is None else risers,
         "riserHeight": n.get("riserHeight", 0.0) if riser is None else riser, "tread": n["tread"]}
    if n.get("bulges") is not None:
        s["bulges"] = n["bulges"]
    for k in ("landing", "cut", "arch"):
        if n.get(k) is not None:
            s[k] = n[k]
    if n.get("numbering"):
        s["numbering"] = True
    if n.get("railings") is not None:
        s["railings"] = n["railings"]
    # docs/adr/0724 §1, §3: bound to its level's height (only when true), its centre post.
    if n.get("heightFromLevel") is True:
        s["heightFromLevel"] = True
    if n.get("post") is not None:
        s["post"] = n["post"]
    return s


def level_height(n, at):
    """The height of a stair's level for `heightFromLevel` (docs/adr/0724 §1): not with `height`, its level given and the
    project's (`level_not_found`), the level with a height."""
    code = "invalid_stair"
    field = f"{at}.heightFromLevel"
    if n.get("height") is not None:
        return refusal(code, "Yükseklik ya kendisi (height) ya katından (heightFromLevel) verilir; ikisi birden verilmez.", field)
    level = (n.get("arch") or {}).get("level")
    if level is None:
        return refusal(code, "Merdivenin yüksekliği kata bağlı ama merdivenin katı yok. Merdivene kat verin ya da yüksekliğini yazın.", field)
    r = level_check(n.get("arch"), f"{at}.arch.level")
    if r:
        return r
    lv = level_of(level)
    h = lv.get("height")
    if h is None or not (math.isfinite(h) and h > 0):
        return refusal(code, f"“{lv['name']}” katının yüksekliği yok; merdiven onu izleyemez. Proje ayarları › Mimari'de katın "
                             "yüksekliğini verin ya da merdivenin yüksekliğini yazın.", field)
    return float(h)


DEFAULT_MAX_RISER = 0.175


def designed(n, at, height=None):
    """docs/adr/0704 §1: the risers and their height, as given or from the height (`height`, or the level's for
    `heightFromLevel`, docs/adr/0724 §1); then the highest riser a given count exceeds. A refusal instead when the fields
    do not hold."""
    code = "invalid_stair"
    h = n.get("height") if height is None else height
    if h is None:
        if n.get("maxRiser") is not None:
            return refusal(code, "En fazla rıht yalnız yükseklikle kullanılır; merdivenin yüksekliğini verin.", f"{at}.maxRiser")
        return (n.get("risers", 0), n.get("riserHeight", 0.0), None)
    if n.get("riserHeight", 0.0) != 0.0:
        return refusal(code, "Yükseklik verildi; rıht yüksekliği ondan hesaplanır. Rıht yüksekliğini vermeyin ya da yüksekliği kaldırın.", f"{at}.riserHeight")
    r = positive(h, "height", "Merdivenin yüksekliği", 100, code, at)
    if r:
        return r
    top = n.get("maxRiser", DEFAULT_MAX_RISER)
    r = positive(top, "maxRiser", "En fazla rıht", 100, code, at)
    if r:
        return r
    count = n["risers"] if n.get("risers", 0) > 0 else math.ceil(h / top - 1e-9)
    if n.get("risers", 0) == 0 and count > 400:
        return refusal(code, f"{fixed(h, 3)} m yükseklik en fazla {fixed(top, 3)} m rıhtla {count} rıht ister; en çok 400 olur. Rıhtı yükseltin ya da yüksekliği bölün.", f"{at}.height")
    riser = h / count
    high = top if n.get("risers", 0) > 0 and n.get("maxRiser") is not None and riser > top + 1e-9 else None
    return (count, riser, high)


def js_hypot(a, b):
    """`Math.hypot(a, b)` as the core computes it (crates/shared/geometry-core/src/jsmath.rs): scaled, compensated."""
    xs = [abs(a), abs(b)]
    m = max(xs)
    if m == 0.0:
        return 0.0
    total, comp = 0.0, 0.0
    for x in xs:
        q = x / m
        summand = q * q - comp
        pre = total + summand
        comp = (pre - total) - summand
        total = pre
    return math.sqrt(total) * m


def float_fit_end(pts, width, count, tread, land):
    """The fitted end in the core's floating-point steps (geom::arch::stair_fit_end), so that the stored points are the
    document's bit for bit; `fit_end` in the drawing reference gives the exact point it must lie within 10⁻⁹ of."""
    eps = 1e-9
    n = len(pts)
    (px, py), (lx, ly) = pts[-2], pts[-1]
    dx, dy = lx - px, ly - py
    length = js_hypot(dx, dy)
    ux, uy = dx / length, dy / length
    reach = (float(count) + 1.0) * tread + float(n) * land
    far = pts[:-1] + [(px + ux * reach, py + uy * reach)]
    at = [0.0]
    for (ax, ay), (bx, by) in zip(far, far[1:]):
        at.append(at[-1] + js_hypot(bx - ax, by - ay))
    start, total = at[n - 2], at[-1]
    lands = []
    for k in range(1, n - 1):
        a, b = max(at[k] - land / 2.0, 0.0), min(at[k] + land / 2.0, total)
        if lands and a - lands[-1][1] < tread - eps:
            lands[-1] = (lands[-1][0], max(lands[-1][1], b))
        else:
            lands.append((a, b))
    stations, missing, s = [], 0, 0.0
    for _ in range(count):
        for a, b in lands:
            if a + eps < s < b - eps:
                s = b
                break
        if s > total + eps:
            missing += 1
        else:
            stations.append(s)
        s += tread
    if missing or not stations or stations[-1] - start <= eps:
        return None
    t = stations[-1] - start
    return (px + ux * t, py + uy * t)


def core_arc(a, b, bulge):
    """geom::bulge::bulge_arc's floating-point steps: centre, radius, start angle, sweep."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = js_hypot(dx, dy)
    k = (1.0 - bulge * bulge) / (4.0 * bulge)
    c = ((a[0] + b[0]) / 2.0 - dy * k, (a[1] + b[1]) / 2.0 + dx * k)
    r = (chord * (1.0 + bulge * bulge)) / (4.0 * abs(bulge))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), 4.0 * math.atan(bulge)


def fitted_arc(s, at):
    """docs/adr/0709: a curved stair's end round its centre from its start by (risers − 1) × tread, in the core's steps
    (geom::arch::stair_fit_arc); the independent `curved_end` must agree within 10⁻⁹."""
    a, b = [(p["x"], p["y"]) for p in s["pts"]]
    c, r, a0, sweep = core_arc(a, b, s["bulges"][0])
    turn = -1.0 if sweep < 0 else 1.0
    sw = turn * ((s["risers"] - 1.0) * s["tread"]) / r
    exact = ac.curved_end(a, ac.arc_of(a, b, s["bulges"][0])[0], s["risers"], s["tread"], turn)
    if not (abs(sw) > 1e-9 and abs(sw) < 2 * math.pi - 1e-9):
        assert exact is None
        return refusal("invalid_stair", "Döner merdiven rıhtlara göre bitirilemiyor: rıhtlar yürüme çizgisinde tam bir turdan uzun. Yarıçapı büyütün, basamağı kısaltın ya da yüksekliği bölün.", f"{at}.fit")
    t = a0 + sw
    end, bulge = (c[0] + r * math.cos(t), c[1] + r * math.sin(t)), math.tan(sw / 4.0)
    assert exact and abs(end[0] - exact[0][0]) <= 1e-9 and abs(end[1] - exact[0][1]) <= 1e-9 and abs(bulge - exact[1]) <= 1e-9
    pts = [s["pts"][0], {"x": end[0], "y": end[1]}]
    return (pts, [bulge, 0.0]), (pts, [bulge, 0.0])


def fitted(s, at):
    if s.get("bulges"):
        return fitted_arc(s, at)
    pts = [(p["x"], p["y"]) for p in s["pts"]]
    land = s.get("landing", s["width"])
    got = float_fit_end(pts, s["width"], s["risers"], s["tread"], land)
    exact = ac.fit_end(fpts(s["pts"]), F(s["width"]), s["risers"], F(s["tread"]), F(land))
    assert (got is None) == (exact is None), (got, exact)
    if got is None:
        return refusal("invalid_stair", "Son kol rıhtlara göre bitirilemiyor: rıhtlar son kola varmadan bitiyor. Son dönüşü başa yaklaştırın, rıht ekleyin ya da son kolu tıklanan yerde bitirin.", f"{at}.fit")
    assert abs(got[0] - float(exact[0])) <= 1e-9 and abs(got[1] - float(exact[1])) <= 1e-9, (got, exact)
    # The stored points (the core's floats) and the exact ones the warnings are reckoned on.
    return s["pts"][:-1] + [{"x": got[0], "y": got[1]}], s["pts"][:-1] + [{"x": exact[0], "y": exact[1]}]


def stairs_create(d, inp, conflict=False):
    if not inp["stairs"]:
        return refusal("no_objects", "Yazılacak merdiven verilmedi. En az bir merdivenin yürüme çizgisini ve ölçülerini verin.", "stairs"), None
    made, warnings = [], []
    for i, n in enumerate(inp["stairs"]):
        at = f"stairs[{i}]"
        r = points_not_finite(n["pts"], at) or next((not_finite("Büküm", f"{at}.bulges[{k}]") for k, b in enumerate(n.get("bulges") or []) if not math.isfinite(b)), None) or finite_each(
            [(n["width"], "width", "Genişlik"), (n.get("riserHeight", 0.0), "riserHeight", "Rıht yüksekliği"),
             (n["tread"], "tread", "Basamak derinliği"), (n.get("landing"), "landing", "Sahanlık"),
             (n.get("height"), "height", "Yükseklik"), (n.get("maxRiser"), "maxRiser", "En fazla rıht"),
             (n.get("cutHeight"), "cutHeight", "Kesme yüksekliği"), (n.get("post"), "post", "Merkez direği")], at)
        if r:
            return r, None
        height = None
        if n.get("heightFromLevel") is True:
            height = level_height(n, at)
            if isinstance(height, dict):
                return height, None
        design = designed(n, at, height)
        if "status" in design:
            return design, None
        risers, riser, high = design
        if n.get("cutHeight") is not None and n.get("cut") is not None:
            return refusal("invalid_stair", "Kesme çizgisi ya rıhtla (cut) ya yükseklikle (cutHeight) verilir; ikisini birden vermeyin.", f"{at}.cutHeight"), None
        s = stair_fields(n, risers, riser)
        r = stair_problem(s, at)
        if r:
            return r, None
        if n.get("cutHeight") is not None:
            r = positive(n["cutHeight"], "cutHeight", "Kesme yüksekliği", 100, "invalid_stair", at)
            if r:
                return r, None
            s["cut"] = int(min(max(math.floor(n["cutHeight"] / s["riserHeight"] + 1e-9), 1), s["risers"]))
        exact = s
        if n.get("fit") is True:
            pts = fitted(s, at)
            if isinstance(pts, dict):
                return pts, None
            if s.get("bulges"):
                (s["pts"], s["bulges"]), _ = pts
                exact = s
            else:
                s["pts"] = pts[0]
                exact = {**s, "pts": pts[1]}
            r = stair_problem(s, at)
            if r:
                return r, None
        r = level_check(s.get("arch"), f"{at}.arch.level")
        if r:
            return r, None
        warnings += stair_warnings(exact, at)
        if high is not None:
            warnings.append({"code": "stair_riser_high", "message": f"{s['risers']} rıhtla rıht {fixed(s['riserHeight'], 3)} m; en fazla {fixed(high, 3)} m verildi. Rıht sayısını artırın ya da boş bırakın (yükseklikten hesaplanır).", "path": at})
        made.append(s)
    return finish(inp, conflict, made, warnings)


def ramp_fields(n):
    r = {"pts": n["pts"], "width": n["width"], "rise": n["rise"]}
    for k in ("slope", "base", "baseOffset", "landing", "railings", "arch"):
        if n.get(k) is not None:
            r[k] = n[k]
    return r


def float_ramp_fit(pts, rise, slope, land):
    """The fitted ramp end in the core's floating-point steps (geom::arch::ramp_fit_end), so that the stored points
    are the document's bit for bit; `ramp_fit` in the drawing reference gives the exact point it lies within 1e-9 of."""
    eps = 1e-9
    n = len(pts)
    run = abs(rise) / (slope / 100.0)
    (px, py), (lx, ly) = pts[-2], pts[-1]
    length = js_hypot(lx - px, ly - py)
    ux, uy = (lx - px) / length, (ly - py) / length
    start = 0.0
    for (ax, ay), (bx, by) in zip(pts[:-2], pts[1:-1]):
        start += js_hypot(bx - ax, by - ay)
    reach = run + float(n) * land + start
    far = pts[:-1] + [(px + ux * reach, py + uy * reach)]
    at = [0.0]
    for (ax, ay), (bx, by) in zip(far, far[1:]):
        at.append(at[-1] + js_hypot(bx - ax, by - ay))
    total_far = at[-1]
    lands = []
    for k in range(1, n - 1):
        a, b = max(at[k] - land / 2.0, 0.0), min(at[k] + land / 2.0, total_far)
        if lands and a < lands[-1][1] + eps:
            lands[-1] = (lands[-1][0], max(lands[-1][1], b))
        else:
            lands.append((a, b))
    landed = 0.0
    for a, b in lands:
        landed += b - a
    total = run + landed
    past = lands[-1][1] if lands else 0.0
    if total - start <= eps or total < past - eps:
        return None
    t = total - start
    return (px + ux * t, py + uy * t)


def ramp_fitted(r, at):
    code, path = "invalid_ramp", f"{at}.fit"
    if r.get("slope") is None:
        return refusal(code, "Eğim verilmedi; son kol eğime göre bitirilemez. Eğimi verin ya da son kolu tıklanan yerde bitirin.", path)
    if r["rise"] == 0.0:
        return refusal(code, "Kot farkı sıfır; eğimli uzunluk yok, son kol eğime göre bitirilemez. Kot farkını verin ya da son kolu tıklanan yerde bitirin.", path)
    pts = [(p["x"], p["y"]) for p in r["pts"]]
    land = r.get("landing", r["width"])
    got = float_ramp_fit(pts, r["rise"], r["slope"], land)
    exact = ac.ramp_fit(fpts(r["pts"]), F(r["rise"]), F(r["slope"]), F(land))
    assert (got is None) == (exact is None), (got, exact)
    if got is None:
        return refusal(code, "Son kol eğime göre bitirilemiyor: eğimli uzunluk son kolun sahanlığı bitmeden doluyor. Son dönüşü başa yaklaştırın ya da son kolu tıklanan yerde bitirin.", path)
    assert abs(got[0] - float(exact[0])) <= 1e-9 and abs(got[1] - float(exact[1])) <= 1e-9, (got, exact)
    return r["pts"][:-1] + [{"x": got[0], "y": got[1]}], r["pts"][:-1] + [{"x": exact[0], "y": exact[1]}]


def float_ramp_landings(pts, rise, slope, land, max_run, fit):
    """docs/adr/0723 §1, written from the ADR: a straight corner `max_run` and half a landing past the start of each
    sloped run that is longer than `max_run` (landings at every inner corner, those that touch made one); with `fit`
    the last point is moved again first, as `float_ramp_fit` does."""
    import math
    eps = 1e-9
    pts = [tuple(p) for p in pts]
    for _ in range(1000):
        if fit:
            end = float_ramp_fit(pts, rise, slope, land)
            if end is not None:
                pts[-1] = end
        at = [0.0]
        for a, b in zip(pts, pts[1:]):
            at.append(at[-1] + math.hypot(b[0] - a[0], b[1] - a[1]))
        length = at[-1]
        lands = []
        for c in at[1:-1]:
            a, b = max(c - land / 2.0, 0.0), min(c + land / 2.0, length)
            if lands and a < lands[-1][1] + eps:
                lands[-1][1] = max(lands[-1][1], b)
            else:
                lands.append([a, b])
        start, cut = 0.0, None
        for a, b in lands + [[length, length]]:
            if a - start > max_run + eps and start + max_run + land / 2.0 < a - eps:
                cut = start + max_run + land / 2.0
                break
            start = b
        if cut is None:
            return pts
        k = next((i for i, c in enumerate(at) if c > cut), len(pts) - 1)
        t = (cut - at[k - 1]) / (at[k] - at[k - 1])
        a, b = pts[k - 1], pts[k]
        pts.insert(max(k, 1), (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t))
    return pts


def ramps_create(d, inp, conflict=False):
    if not inp["ramps"]:
        return refusal("no_objects", "Yazılacak rampa verilmedi. En az bir rampanın orta çizgisini ve ölçülerini verin.", "ramps"), None
    made, warnings = [], []
    for i, n in enumerate(inp["ramps"]):
        at = f"ramps[{i}]"
        r = points_not_finite(n["pts"], at) or finite_each(
            [(n["width"], "width", "Genişlik"), (n["rise"], "rise", "Yükseklik"), (n.get("slope"), "slope", "Eğim"),
             (n.get("base"), "base", "Başlangıç kotu"), (n.get("baseOffset"), "baseOffset", "Kata göre başlangıç kotu"),
             (n.get("landing"), "landing", "Sahanlık")], at)
        if r:
            return r, None
        if n.get("base") is not None and n.get("baseOffset") is not None:
            return refusal("invalid_ramp", BOTH_BASES, f"{at}.baseOffset"), None
        f = ramp_fields(n)
        r = ramp_problem(f, at)
        if r:
            return r, None
        exact = f
        if n.get("fit") is True:
            pts = ramp_fitted(f, at)
            if isinstance(pts, dict):
                return pts, None
            f["pts"] = pts[0]
            exact = {**f, "pts": pts[1]}
            r = ramp_problem(f, at)
            if r:
                return r, None
        if n.get("maxRun") is not None:
            m = n["maxRun"]
            if not (m > 0.0 and m <= 100.0):
                return refusal("invalid_ramp", f"En uzun kol 0'dan büyük ve en çok 100 m olmalı; {str(int(m)) if float(m).is_integer() else repr(float(m))} verildi.", f"{at}.maxRun"), None
            got = float_ramp_landings([(p["x"], p["y"]) for p in f["pts"]], f["rise"], f.get("slope"), f.get("landing", f["width"]), m, n.get("fit") is True)
            f["pts"] = [{"x": x, "y": y} for x, y in got]
            exact = {**exact, "pts": f["pts"]}
            r = ramp_problem(f, at)
            if r:
                return r, None
        r = level_check(f.get("arch"), f"{at}.arch.level")
        if r:
            return r, None
        level_bound(f)
        warnings += ramp_warnings(exact, at)
        made.append(f)
    return finish(inp, conflict, made, warnings)


CREATES = {GRIDS: ("grid", "grids", grids_create, "Aks"), COLUMNS: ("column", "columns", columns_create, "Kolon"),
           STAIRS: ("stair", "stairs", stairs_create, "Merdiven"), RAMPS: ("ramp", "ramps", ramps_create, "Rampa")}


def extra_of(inp):
    return {k: inp[k] for k in ("color", "lineWeight") if k in inp}


def create_step(d, command, op, inp, non_finite=None, conflict=False, with_command=False):
    kind, plural, fn, _ = CREATES[command]
    result, made = fn(d, with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op}
    if with_command:
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
    # A grid system's column is (kind, fields), its grids `@k`: the k-th object made (ADR 0707).
    made = [m if isinstance(m, tuple) else (kind, m) for m in made]

    def named(f, ids):
        f = copy.deepcopy(f)
        if any(isinstance(g, str) and g.startswith("@") for g in f.get("grids") or []):
            f["grids"] = ["$any" if ids is None else uid_of(ids[int(g[1:])]) for g in f["grids"]]
        return f

    ents = [made_entity(k, inp["layerId"], named(f, None), extra_of(inp)) for k, f in made]
    if op == "plan":
        s["result"] = {"status": "completed", "output": {plural: ents, "revision": "$current"}, "warnings": result["warnings"]}
        s["within"] = 1e-9
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ids = list(range(d.next, d.next + len(made)))
    d.next += len(made)
    for i, (k, f) in zip(ids, made):
        d.objs[i] = (k, inp["layerId"], named(f, ids))
    s["result"] = {"status": "completed", "output": {"uids": ["$any"] * len(made), "ids": ids, "revision": "$current"},
                   "warnings": result["warnings"]}
    s["within"] = 1e-9
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(d, i, extra_of(inp)) for i in ids},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps):
        assert len(name) > 10, name
        assert name not in [c["name"] for c in self.cases], name
        self.cases.append({"name": name, "steps": steps})


def refusals(c, command, rows):
    for name, inp, kw in rows:
        c.add(name, [create_step(Drawing(), command, "execute", inp, **kw)])


def common_refusals(c, command, inp):
    refusals(c, command, [
        ("olmayan katman reddedilir", {**inp, "layerId": "yok"}, {}),
        ("katman grubu reddedilir", {**inp, "layerId": "pafta"}, {}),
        ("kilitli katman reddedilir", {**inp, "layerId": "kilitli"}, {}),
        ("geçersiz çizgi kalınlığı reddedilir", {**inp, "lineWeight": 120.0}, {}),
        ("geçersiz beklenen sürüm reddedilir", {**inp, "expectedRevision": "07"}, {}),
    ])
    d = Drawing()
    made = create_step(d, command, "execute", inp)
    stale = create_step(d, command, "execute", {**inp, "expectedRevision": "$prepared"}, conflict=True)
    c.add("sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])


def life(c, command, inp, name):
    """Doğrula, plan, yaz, geri al and the hidden layer for one input."""
    d = Drawing()
    c.add(f"{name}: doğrula yazmaz", [create_step(d, command, "validate", inp)])
    c.add(f"{name}: plan yazmadan gösterir", [create_step(Drawing(), command, "plan", inp)])
    d = Drawing()
    step = create_step(d, command, "execute", inp)
    c.add(f"{name}: yazılır ve geri alınır", [step, {"op": "undo", "returns": CREATES[command][3],
                                                    "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}}])
    c.add(f"{name}: gizli katmana uyarıyla yazılır", [create_step(Drawing(), command, "execute", {**inp, "layerId": "gizli"})])

# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────


def grid_cases():
    c = Cases()
    one = {"layerId": "aks", "grids": [{"pts": [P(-2, 9), P(14, 9)], "label": "3"}]}
    life(c, GRIDS, one, "Etiketli tek aks")
    for name, inp in [
        ("etiketsiz aks çizimin son aksından sonrakini alır (C → D)", {"layerId": "aks", "grids": [{"pts": [P(18, -2), P(18, 12)]}]}),
        ("etiketsiz akslar birbirini izler (D, E)", {"layerId": "aks", "grids": [{"pts": [P(18, -2), P(18, 12)]}, {"pts": [P(24, -2), P(24, 12)]}]}),
        ("sayılı aksın ardındaki etiketsiz aks bir sonraki sayı", {"layerId": "aks", "grids": [{"pts": [P(-2, 9), P(14, 9)], "label": "9"}, {"pts": [P(-2, 14), P(14, 14)]}]}),
        ("seri: sağa 6, 6, 4.5 arayla A'dan", {"layerId": "aks", "series": {"from": P(0, -2), "to": P(0, 12), "spacings": [6.0, 6.0, 4.5], "side": "right", "start": "A"}}),
        ("seri: sola, etiketsiz, çizimin son aksından sonra", {"layerId": "aks", "series": {"from": P(-2, 8), "to": P(14, 8), "spacings": [3.0, 3.0]}}),
        ("seri I harfini atlar (H, J)", {"layerId": "aks", "series": {"from": P(40, 0), "to": P(40, 10), "spacings": [5.0], "start": "H"}}),
        ("seri Z'den AA'ya geçer", {"layerId": "aks", "series": {"from": P(40, 0), "to": P(40, 10), "spacings": [5.0], "start": "Z"}}),
        ("başında balonlu, 10 mm balonlu aks", {"layerId": "aks", "grids": [{"pts": [P(0, 30), P(10, 30)], "label": "X1", "bubbles": "start", "bubbleSize": 10.0}]}),
        ("yaylı aks (büküm 0.5)", {"layerId": "aks", "grids": [{"pts": [P(-5, 40), P(5, 40)], "bulges": [0.5, 0.0], "label": "R"}]}),
        ("katı ve evresiyle aks, rengi ve kalınlığıyla", {"layerId": "aks", "color": "#c03030", "lineWeight": 0.18, "grids": [{"pts": [P(0, 50), P(10, 50)], "label": "Z1", "arch": {"level": "zemin", "phase": "new"}}]}),
        ("aksla seri birlikte: seri aksın ardından sürer", {"layerId": "aks", "grids": [{"pts": [P(50, 0), P(50, 10)], "label": "M"}], "series": {"from": P(52, 0), "to": P(52, 10), "spacings": [2.0]}}),
    ]:
        c.add(name, [create_step(Drawing(), GRIDS, "execute", inp)])
    refusals(c, GRIDS, [
        ("aks ve seri verilmezse reddedilir", {"layerId": "aks"}, {}),
        ("sonsuz koordinat reddedilir", {"layerId": "aks", "grids": [{"pts": [P(0, 0), P(5, 0)], "label": "A"}]}, {"non_finite": {"grids[0].pts[1].x": "Infinity"}}),
        ("tek noktalı aks reddedilir", {"layerId": "aks", "grids": [{"pts": [P(0, 0)], "label": "A"}]}, {}),
        ("başı ve sonu aynı aks reddedilir", {"layerId": "aks", "grids": [{"pts": [P(1, 1), P(1, 1)], "label": "A"}]}, {}),
        ("boş etiket reddedilir", {"layerId": "aks", "grids": [{"pts": [P(0, 0), P(5, 0)], "label": "  "}]}, {}),
        ("sıfır balon çapı reddedilir", {"layerId": "aks", "grids": [{"pts": [P(0, 0), P(5, 0)], "label": "A", "bubbleSize": 0.0}]}, {}),
        ("tek bükümlü aks reddedilir", {"layerId": "aks", "grids": [{"pts": [P(0, 0), P(5, 0)], "label": "A", "bulges": [0.2]}]}, {}),
        ("sıfır aralıklı seri reddedilir", {"layerId": "aks", "series": {"from": P(0, 0), "to": P(0, 5), "spacings": [3.0, 0.0]}}, {}),
        ("eksi aralıklı seri reddedilir", {"layerId": "aks", "series": {"from": P(0, 0), "to": P(0, 5), "spacings": [-1.5]}}, {}),
        ("başı ve sonu aynı seri reddedilir", {"layerId": "aks", "series": {"from": P(2, 2), "to": P(2, 2), "spacings": [3.0]}}, {}),
        ("sonsuz aralık reddedilir", {"layerId": "aks", "series": {"from": P(0, 0), "to": P(0, 5), "spacings": [3.0]}}, {"non_finite": {"series.spacings[0]": "NaN"}}),
        ("olmayan kat reddedilir", {"layerId": "aks", "grids": [{"pts": [P(0, 0), P(5, 0)], "label": "A", "arch": {"level": "yok"}}]}, {}),
    ])
    common_refusals(c, GRIDS, one)
    return c.cases


def system_cases():
    """Aks ağı (ADR 0707): two families of grids at right angles, turned, with columns at their crossings."""
    c = Cases()
    base = {"origin": P(0, 20), "x": [6.0, 6.0, 4.5], "y": [5.0, 5.0]}
    turn = math.pi / 6
    cols = {"width": 0.3, "depth": 0.5, "fill": True, "tagStart": "S1"}
    life(c, GRIDS, {"layerId": "aks", "system": {**base, "columns": cols}}, "Kolonlu aks ağı")
    for name, inp in [
        ("aks ağı: 1–4 ve A–C, öntanımlı 1.5 m uzatma", {"layerId": "aks", "system": base}),
        ("30° döndürülmüş aks ağı", {"layerId": "aks", "system": {**base, "angle": turn}}),
        ("döndürülmüş ağın kolonları ağın açısıyla döner", {"layerId": "kolon", "system": {**base, "angle": turn, "columns": {"width": 0.4}}}),
        ("yuvarlak kolonlar kendi dönüşüyle, türüyle", {"layerId": "kolon", "system": {**base, "columns": {"shape": "round", "width": 0.5, "rotation": 0.25, "type": "C1"}}}),
        ("ilk adlar verilir: harfler x'te, sayılar y'de", {"layerId": "aks", "system": {**base, "xStart": "A", "yStart": "1", "columns": {"width": 0.3}}}),
        ("yalnız y aralıkları: x'te tek aks", {"layerId": "aks", "system": {"origin": P(40, 0), "y": [4.0, 4.0]}}),
        ("tek akslı iki aile: iki uzatma boyu aks", {"layerId": "aks", "system": {"origin": P(40, 30), "extension": 2.0}}),
        ("uzatma 0.5, başında balon, 10 mm balon", {"layerId": "aks", "system": {**base, "extension": 0.5, "bubbles": "start", "bubbleSize": 10.0}}),
        ("uzatma sıfırken akslar en dış akslarda biter", {"layerId": "aks", "system": {**base, "extension": 0.0}}),
        ("katı ve evresiyle ağ, renk ve kalınlıkla; kolonlar katla", {"layerId": "aks", "color": "#c03030", "lineWeight": 0.18, "system": {**base, "arch": {"level": "zemin", "phase": "new"}, "columns": {"width": 0.3}}}),
        ("aks ve seriyle birlikte: ağ en sonda", {"layerId": "aks", "grids": [{"pts": [P(50, 0), P(50, 10)], "label": "M"}], "series": {"from": P(52, 0), "to": P(52, 10), "spacings": [2.0]}, "system": {"origin": P(60, 0), "x": [3.0], "y": [3.0]}}),
        ("y ailesi H'den: I harfi atlanır", {"layerId": "aks", "system": {"origin": P(0, 60), "x": [5.0], "y": [3.0, 3.0, 3.0], "yStart": "H"}}),
        ("eğik başlangıç ve -90° dönüş", {"layerId": "aks", "system": {"origin": P(7.25, -3.5), "angle": -math.pi / 2, "x": [4.0], "y": [2.5, 2.5]}}),
    ]:
        c.add(name, [create_step(Drawing(), GRIDS, "execute", inp)])
    refusals(c, GRIDS, [
        ("ağın eksi aralığı reddedilir", {"layerId": "aks", "system": {**base, "x": [6.0, -1.0]}}, {}),
        ("ağın sıfır aralığı reddedilir", {"layerId": "aks", "system": {**base, "y": [0.0]}}, {}),
        ("ağın sonsuz aralığı reddedilir", {"layerId": "aks", "system": base}, {"non_finite": {"system.y[1]": "Infinity"}}),
        ("ağın sonsuz açısı reddedilir", {"layerId": "aks", "system": {**base, "angle": 0.5}}, {"non_finite": {"system.angle": "NaN"}}),
        ("ağın sonsuz başlangıcı reddedilir", {"layerId": "aks", "system": base}, {"non_finite": {"system.origin.x": "Infinity"}}),
        ("eksi uzatma reddedilir", {"layerId": "aks", "system": {**base, "extension": -1.0}}, {}),
        ("tek akslı yönde sıfır uzatma reddedilir", {"layerId": "aks", "system": {"origin": P(0, 0), "x": [3.0], "extension": 0.0}}, {}),
        ("boş ilk ad reddedilir", {"layerId": "aks", "system": {**base, "xStart": "  "}}, {}),
        ("sıfır balon çapı reddedilir", {"layerId": "aks", "system": {**base, "bubbleSize": 0.0}}, {}),
        ("sıfır kolon eni reddedilir", {"layerId": "aks", "system": {**base, "columns": {"width": 0.0}}}, {}),
        ("sonsuz kolon eni reddedilir", {"layerId": "aks", "system": {**base, "columns": {"width": 0.3}}}, {"non_finite": {"system.columns.width": "NaN"}}),
        ("boş kolon türü reddedilir", {"layerId": "aks", "system": {**base, "columns": {"width": 0.3, "type": " "}}}, {}),
        ("ağın olmayan katı reddedilir", {"layerId": "aks", "system": {**base, "arch": {"level": "yok"}}}, {}),
    ])
    common_refusals(c, GRIDS, {"layerId": "aks", "system": base})
    return c.cases


def col(n, layer="kolon", **kw):
    return {"layerId": layer, "columns": n, **kw}


def column_cases():
    c = Cases()
    one = col([{"at": P(3, 3), "width": 0.3, "depth": 0.5}])
    life(c, COLUMNS, one, "Dikdörtgen kolon")
    for name, inp in [
        ("yuvarlak, dolu, etiketli ve türlü kolon", col([{"at": P(4, 4), "shape": "round", "width": 0.4, "fill": True, "tag": "S9", "type": "Betonarme"}])),
        ("döndürülmüş kolon", col([{"at": P(4, 4), "width": 0.3, "depth": 0.6, "rotation": 0.5}])),
        ("akslara bağlı kolon kesişimde durur, konumu harfli aks önce (A-1)", col([{"grids": [uid_of(3), uid_of(1)], "width": 0.4}])),
        ("akslara bağlı kolonun verilen merkezi kesişime gider (B-2)", col([{"at": P(6.1, 5.1), "grids": [uid_of(2), uid_of(4)], "width": 0.4}])),
        ("etiketsiz kolonlar başlangıç etiketinden artar, kendi etiketi olan atlanmaz", col([{"at": P(0, 0), "width": 0.3}, {"at": P(1, 0), "width": 0.3, "tag": "Ö"}, {"at": P(2, 0), "width": 0.3}], tagStart="S1")),
        ("aks kesişimlerinin hepsine kolon (paralel akslar atlanır)", {"layerId": "kolon", "crossings": {"grids": [uid_of(1), uid_of(2), uid_of(3), uid_of(4)], "width": 0.3, "depth": 0.5, "fill": True}, "tagStart": "S1"}),
        ("kesişimlerde yuvarlak kolon, aksın dışındaki kesişim atlanır", {"layerId": "kolon", "crossings": {"grids": [uid_of(7), uid_of(3), uid_of(5)], "shape": "round", "width": 0.5}}),
        ("tek kolon ve kesişimler birlikte", {"layerId": "kolon", "columns": [{"at": P(9, 9), "width": 0.25}], "crossings": {"grids": [uid_of(2), uid_of(4)], "width": 0.3}}),
    ]:
        c.add(name, [create_step(Drawing(), COLUMNS, "execute", inp)])
    refusals(c, COLUMNS, [
        ("kolon ve kesişim verilmezse reddedilir", {"layerId": "kolon"}, {}),
        ("sonsuz en reddedilir", col([{"at": P(0, 0), "width": 0.3}]), {"non_finite": {"columns[0].width": "NaN"}}),
        ("sonsuz merkez reddedilir", col([{"at": P(0, 0), "width": 0.3}]), {"non_finite": {"columns[0].at.y": "-Infinity"}}),
        ("yeri verilmeyen kolon reddedilir", col([{"width": 0.3}]), {}),
        ("sıfır enli kolon reddedilir", col([{"at": P(0, 0), "width": 0.0}]), {}),
        ("tek aksa bağlanan kolon reddedilir", col([{"grids": [uid_of(1)], "width": 0.3}]), {}),
        ("aynı aksa iki kez bağlanan kolon reddedilir", col([{"grids": [uid_of(1), uid_of(1)], "width": 0.3}]), {}),
        ("boş etiketli kolon reddedilir", col([{"at": P(0, 0), "width": 0.3, "tag": " "}]), {}),
        ("kimlik olmayan aks reddedilir", col([{"grids": ["aks-a", uid_of(3)], "width": 0.3}]), {}),
        ("olmayan aks reddedilir", col([{"grids": [MISSING, uid_of(3)], "width": 0.3}]), {}),
        ("aks olmayan nesne reddedilir", col([{"grids": [uid_of(1), uid_of(6)], "width": 0.3}]), {}),
        ("paralel akslar reddedilir", col([{"grids": [uid_of(1), uid_of(2)], "width": 0.3}]), {}),
        ("tek akslı kesişim isteği reddedilir", {"layerId": "kolon", "crossings": {"grids": [uid_of(1)], "width": 0.3}}, {}),
        ("kesişimde aks olmayan nesne reddedilir", {"layerId": "kolon", "crossings": {"grids": [uid_of(1), uid_of(8)], "width": 0.3}}, {}),
        ("olmayan kat reddedilir", col([{"at": P(0, 0), "width": 0.3, "arch": {"level": "yok"}}]), {}),
    ])
    common_refusals(c, COLUMNS, one)
    # The document keeps a column with its grids (ADR 0343 §3).
    d = Drawing()
    made = create_step(d, COLUMNS, "execute", col([{"grids": [uid_of(1), uid_of(3)], "width": 0.4}]))
    col_id = d.next - 1

    def moved(d):
        k, ly, f = d.objs[1]
        d.objs[1] = (k, ly, dict(f, pts=[P(1, -2), P(1, 12)]))
        return d.follow()
    c.add("aks taşınınca bağlı kolon kesişimde kalır", [made, other_step(d, "cad.entities.transform",
                                                                       {"uids": [uid_of(1)], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}, moved, [1])])
    d = Drawing()
    made = create_step(d, COLUMNS, "execute", col([{"grids": [uid_of(1), uid_of(3)], "width": 0.4}]))

    def gone(d):
        del d.objs[1]
        return d.follow()
    step = other_step(d, "cad.entities.delete", {"uids": [uid_of(1)]}, gone, [])
    # The grid's id names an object no longer there: the column keeps it, said as any.
    step["expect"]["entities"][str(col_id)]["grids"] = "$any"
    c.add("aks silinince kolon yerinde kalır, bağı kopar (gridGone)", [made, step])
    assert d.objs[col_id][2]["broken"] == "gridGone"
    return c.cases


def other_step(d, command, inp, change, also):
    ids = change(d)
    return {"op": "execute", "command": command, "input": inp, "result": {"status": "completed", "output": "$any", "warnings": "$any"},
            "within": 1e-9, "expect": {"ids": d.ids(), "entities": {str(i): entity(d, i) for i in list(also) + ids}, "revision": "changed"}}


def stair(**kw):
    s = {"pts": [P(0, 30), P(F(15, 4), 30)], "width": 1.2, "risers": 16, "riserHeight": 0.175, "tread": 0.25}
    s.update(kw)
    return {"layerId": "merdiven", "stairs": [s]}


def stair_cases():
    c = Cases()
    life(c, STAIRS, stair(), "Düz merdiven")
    for name, inp in [
        ("yol uzunsa stair_long uyarısı", stair(pts=[P(0, 30), P(4, 30)])),
        ("yol kısaysa stair_short uyarısı", stair(pts=[P(0, 30), P(3, 30)])),
        ("Blondel oranı dışında stair_step_rule uyarısı", stair(riserHeight=0.20, tread=0.30, pts=[P(0, 30), P(4.5, 30)])),
        ("L merdiven, sahanlık genişlik kadar", stair(pts=[P(0, 30), P(3, 30), P(3, 33)], width=1.0, risers=10, riserHeight=0.18, tread=0.3)),
        ("U merdiven, kesme çizgisi ve numaralar", stair(pts=[P(0, 30), P(3, 30), P(3, 32), P(0, 32)], width=1.0, risers=18, riserHeight=0.17, tread=0.28, cut=9, numbering=True)),
        ("L merdiven, kısa sahanlık", stair(pts=[P(0, 30), P(2, 30), P(2, 32)], width=1.0, risers=12, riserHeight=0.16, tread=0.3, landing=0.5)),
        ("katın yüksekliğini tutmayan rıhtlar stair_rise_level uyarısı", stair(arch={"level": "zemin"})),
        ("katın yüksekliğini tutan rıhtlarda uyarı yok", stair(risers=18, riserHeight=0.17, tread=0.28, pts=[P(0, 30), P(5.04, 30)], arch={"level": "zemin"})),
        ("yüksekliği bilinmeyen katta rıht denetlenmez", stair(arch={"level": "kat-1"})),
    ]:
        c.add(name, [create_step(Drawing(), STAIRS, "execute", inp)])
    refusals(c, STAIRS, [
        ("merdiven verilmezse reddedilir", {"layerId": "merdiven", "stairs": []}, {}),
        ("sonsuz basamak reddedilir", stair(), {"non_finite": {"stairs[0].tread": "Infinity"}}),
        ("sonsuz köşe reddedilir", stair(), {"non_finite": {"stairs[0].pts[0].x": "NaN"}}),
        ("beş noktalı yol reddedilir", stair(pts=[P(0, 0), P(1, 0), P(1, 1), P(2, 1), P(2, 2)]), {}),
        ("tek noktalı yol reddedilir", stair(pts=[P(0, 0)]), {}),
        ("ard arda aynı nokta reddedilir", stair(pts=[P(0, 0), P(2, 0), P(2, 0)]), {}),
        ("sıfır genişlik reddedilir", stair(width=0.0), {}),
        ("eksi rıht yüksekliği reddedilir", stair(riserHeight=-0.17), {}),
        ("sıfır rıht sayısı reddedilir", stair(risers=0), {}),
        ("rıht sayısından büyük kesme reddedilir", stair(cut=20), {}),
        ("olmayan kat reddedilir", stair(arch={"level": "yok"}), {}),
    ])
    common_refusals(c, STAIRS, stair())
    return c.cases


def design(**kw):
    """docs/adr/0704: a stair drawn from its height, fitted to its risers."""
    s = {"pts": [P(0, 30), P(5, 30)], "width": 1.2, "height": 3.0, "tread": 0.28, "fit": True}
    s.update(kw)
    for k in [k for k, v in s.items() if v is None]:
        del s[k]
    return {"layerId": "merdiven", "stairs": [s]}


def design_cases():
    c = Cases()
    life(c, STAIRS, design(), "Yükseklikten düz merdiven, yola sığar")
    for name, inp in [
        ("3,00 m ve en fazla 18 cm rıht: 17 rıht", design(maxRiser=0.18)),
        ("verilen 20 rıht: rıht yüksekliği yükseklikten", design(risers=20)),
        ("verilen rıht en fazla rıhtı aşarsa stair_riser_high uyarısı", design(risers=15, maxRiser=0.18)),
        ("tam bölünen 2,80 m / 17,5 cm: 16 rıht, fazladan rıht yok", design(height=2.8)),
        ("kesme yüksekliği 1,10 m: kesme 6. rıhtta", design(cutHeight=1.1, numbering=True)),
        ("ilk rıhttan alçak kesme yüksekliği: kesme 1. rıhtta", design(cutHeight=0.1)),
        ("merdivenden yüksek kesme yüksekliği: kesme son rıhtta", design(cutHeight=5.0)),
        ("L merdiven: son kol rıhtlara göre biter", design(pts=[P(0, 30), P(3, 30), P(3, 33)], width=1.0)),
        ("U merdiven, kollar arası dar: tek sahanlık, son kol rıhtlara göre", design(pts=[P(0, 30), P(3, 30), P(3, 31.3), P(0, 31.3)])),
        ("kısa yol yola sığdırılınca uzar", design(pts=[P(0, 30), P(2, 30)])),
        ("ters yöne çizilen merdiven yola sığar", design(pts=[P(10, 30), P(9, 30)])),
        ("eğik (3-4-5) merdiven yola sığar", design(pts=[P(0, 30), P(0.75, 31)])),
        ("sığdırmasız kısa yol stair_short uyarısıyla yazılır", design(pts=[P(0, 30), P(2, 30)], fit=None)),
        ("zemin kattan 3,06 m: 18 × 17 cm, stair_rise_level yok", design(height=3.06, arch={"level": "zemin"})),
        ("iki yan korkuluklu U merdiven (ADR 0705)", design(pts=[P(0, 30), P(3, 30), P(3, 31.3), P(0, 31.3)], railings="both", cutHeight=1.1)),
        ("sol korkuluklu düz merdiven", design(railings="left")),
        ("verilen rıhtlarla sağ korkuluk (eski girdi)", stair(railings="right")),
    ]:
        c.add(name, [create_step(Drawing(), STAIRS, "execute", inp)])
    c.add("plan yükseklikten hesaplanan merdiveni gösterir", [create_step(Drawing(), STAIRS, "plan", design(cutHeight=1.1))])
    refusals(c, STAIRS, [
        ("yükseklikle rıht yüksekliği birlikte reddedilir", design(riserHeight=0.17), {}),
        ("yüksekliksiz en fazla rıht reddedilir", design(height=None, risers=18, riserHeight=0.17, maxRiser=0.18), {}),
        ("sıfır yükseklik reddedilir", design(height=0.0), {}),
        ("sıfır en fazla rıht reddedilir", design(maxRiser=0.0), {}),
        ("400'den çok rıht isteyen yükseklik reddedilir", design(height=80.0, maxRiser=0.1), {}),
        ("kesme rıhtı ve kesme yüksekliği birlikte reddedilir", design(cut=6, cutHeight=1.1), {}),
        ("eksi kesme yüksekliği reddedilir", design(cutHeight=-1.0), {}),
        ("rıhtlar son kola varmadan biterse sığdırma reddedilir", design(pts=[P(0, 30), P(8, 30), P(8, 32)]), {}),
        ("sonsuz yükseklik reddedilir", design(), {"non_finite": {"stairs[0].height": "NaN"}}),
        ("sonsuz kesme yüksekliği reddedilir", design(cutHeight=1.1), {"non_finite": {"stairs[0].cutHeight": "Infinity"}}),
        ("rıhtı da yüksekliği de olmayan merdiven reddedilir", design(height=None), {}),
    ])
    return c.cases


def curved(**kw):
    """docs/adr/0709: a curved stair round (0, 30), 1.5 m on its walking line, from its height, fitted to its risers."""
    s = {"pts": [P(1.5, 30), P(-1.5, 30)], "bulges": [1.0, 0.0], "width": 1.0, "height": 3.0, "tread": 0.28, "fit": True}
    s.update(kw)
    for k in [k for k, v in s.items() if v is None]:
        del s[k]
    return {"layerId": "merdiven", "stairs": [s]}


def curved_cases():
    c = Cases()
    life(c, STAIRS, curved(), "Yükseklikten döner merdiven, rıhtlara göre biter")
    quarter = math.tan(math.pi / 8)
    for name, inp in [
        ("saat yönünde döner merdiven, iki yan korkuluk ve numaralar", curved(bulges=[-1.0, 0.0], railings="both", numbering=True)),
        ("kesme yüksekliği 1,10 m: kesme 6. rıhtta, korkuluğun üstü ayrı", curved(cutHeight=1.1, railings="left")),
        ("270° döner merdiven, 4,20 m, 1,20 m yarıçap", curved(pts=[P(1.2, 30), P(0, 31.2)], bulges=[quarter, 0.0], width=1.0, height=4.2, tread=0.25)),
        ("verilen rıhtlarla döner merdiven, yol uzun: stair_long", curved(height=None, fit=None, risers=12, riserHeight=0.175)),
        ("çeyrek dönüş yetmez: stair_short (sığdırmasız)", curved(pts=[P(1.5, 30), P(0, 31.5)], bulges=[quarter, 0.0], fit=None)),
        ("Blondel dışı döner merdiven: stair_step_rule", curved(height=None, fit=None, risers=10, riserHeight=0.2, tread=0.32)),
        ("kattan döner merdiven: stair_rise_level yok", curved(height=3.06, arch={"level": "zemin"})),
        ("kattan döner merdiven: rıhtlar katı tutmuyor", curved(height=None, fit=None, risers=12, riserHeight=0.175, arch={"level": "zemin"})),
        ("düz ve döner merdiven birlikte", {"layerId": "merdiven", "stairs": [curved()["stairs"][0], design()["stairs"][0]]}),
        ("eğik yaylı yürüme çizgisi (3-4-5 kiriş)", curved(pts=[P(0, 30), P(2.4, 33.2)], bulges=[0.5, 0.0], width=1.2, height=2.8)),
        ("saat yönünde 1,80 m yarıçap, sağ korkuluk, sığdırmasız", curved(pts=[P(6.8, 35), P(5, 33.2)], bulges=[-quarter, 0.0], width=1.1, fit=None, height=None, risers=8, riserHeight=0.175, tread=0.27, railings="right")),
    ]:
        c.add(name, [create_step(Drawing(), STAIRS, "execute", inp)])
    c.add("plan döner merdiveni rıhtlarıyla gösterir", [create_step(Drawing(), STAIRS, "plan", curved(cutHeight=1.1))])
    # A mirror turns the stair's arc over (docs/adr/0709): the bulge's sign changes with the points.
    d = Drawing()
    made = create_step(d, STAIRS, "execute", curved(height=None, fit=None, risers=17, riserHeight=0.175))

    def mirrored(d):
        k, ly, f = d.objs[d.next - 1]
        f = dict(f, pts=[{"x": -q["x"], "y": q["y"]} for q in f["pts"]], bulges=[-f["bulges"][0], 0.0])
        d.objs[d.next - 1] = (k, ly, f)
        return []
    c.add("aynalanan döner merdivenin yayı ters döner", [made, other_step(
        d, "cad.entities.transform", {"uids": [uid_of(d.next - 1)], "transform": {"kind": "mirror", "a": {"x": 0, "y": 0}, "b": {"x": 0, "y": 10}}},
        mirrored, [d.next - 1])])
    refusals(c, STAIRS, [
        ("üç noktalı yolda büküm reddedilir", curved(pts=[P(0, 30), P(3, 30), P(3, 33)], bulges=[0.5, 0.0]), {}),
        ("tek büküm reddedilir", curved(bulges=[1.0]), {}),
        ("ikinci büküm sıfır değilse reddedilir", curved(bulges=[1.0, 0.5]), {}),
        ("sıfır büküm (düz) reddedilir", curved(bulges=[0.0, 0.0]), {}),
        ("yarıçap genişliğin yarısından küçükse reddedilir", curved(pts=[P(0.5, 30), P(-0.5, 30)], width=1.2), {}),
        ("yarıçap tam yarı genişlikse reddedilir", curved(pts=[P(0.5, 30), P(-0.5, 30)], width=1.0), {}),
        ("sonsuz büküm reddedilir", curved(), {"non_finite": {"stairs[0].bulges[0]": "NaN"}}),
        ("sonsuz ikinci büküm reddedilir", curved(), {"non_finite": {"stairs[0].bulges[1]": "Infinity"}}),
        ("rıhtlar bir turdan uzunsa sığdırma reddedilir", curved(pts=[P(0.4, 30), P(-0.4, 30)], width=0.6), {}),
    ])
    return c.cases


def rdesign(**kw):
    """docs/adr/0705: a ramp fitted to its rise over its slope."""
    r = {"pts": [P(0, 40), P(5, 40)], "width": 1.5, "rise": 0.5, "slope": 6.25, "fit": True}
    r.update(kw)
    for k in [k for k, v in r.items() if v is None]:
        del r[k]
    return {"layerId": "merdiven", "ramps": [r]}


def ramp_design_cases():
    c = Cases()
    life(c, RAMPS, rdesign(), "Eğimden düz rampa, son kol eğime göre")
    for name, inp in [
        ("L rampa: son kol eğime göre biter", rdesign(pts=[P(0, 40), P(4, 40), P(4, 42)])),
        ("U rampa, 2 m sahanlıklar üst üste: bir kez sayılır", rdesign(pts=[P(0, 40), P(6, 40), P(6, 41.6), P(0, 41.6)], landing=2.0)),
        ("inen rampa: eksi kot farkı da eğime göre biter", rdesign(pts=[P(0, 40), P(0, 37)], rise=-0.3, slope=5.0, base=0.3)),
        ("eğik (3-4-5) rampa eğime göre biter", rdesign(pts=[P(0, 40), P(0.75, 41)], slope=8.0)),
        ("ters yöne çizilen rampa", rdesign(pts=[P(10, 40), P(9, 40)])),
        ("en uzun kol 9 m: 24 m eğimli kola iki ara sahanlık", rdesign(pts=[P(0, 40), P(30, 40)], rise=1.2, slope=5.0, landing=1.5, maxRun=9.0)),
        ("kolları kısa rampa en uzun kolla değişmez", rdesign(maxRun=9.0)),
        ("eğime göre bitmeyen uzun rampa da bölünür", rdesign(pts=[P(0, 40), P(20, 40)], rise=0.8, slope=None, fit=None, maxRun=6.0)),
        ("L rampada dönüşten sonra kalan uzun kol bölünür", rdesign(pts=[P(0, 40), P(6, 40), P(6, 60)], rise=1.0, slope=5.0, maxRun=8.0)),
        ("inen uzun rampa ara sahanlıkla", rdesign(pts=[P(0, 40), P(-30, 40)], rise=-1.2, slope=5.0, base=1.2, maxRun=9.0)),
        ("tam en uzun kol kadar olan kol bölünmez", rdesign(pts=[P(0, 40), P(10, 40)], rise=0.5, slope=5.0, maxRun=10.0)),
        ("sıfır en uzun kol reddedilir", rdesign(maxRun=0.0)),
        ("eksi en uzun kol reddedilir", rdesign(maxRun=-1.0)),
        ("100 m'yi aşan en uzun kol reddedilir", rdesign(maxRun=101.0)),
        ("kısa yol eğime göre uzar", rdesign(pts=[P(0, 40), P(2, 40)])),
        ("uzun yol eğime göre kısalır", rdesign(pts=[P(0, 40), P(20, 40)])),
        ("başlangıç kotlu rampa: +0.30'dan +0.80'e", rdesign(base=0.3)),
        ("sığdırmasız uzun yol ramp_slope_mismatch uyarısıyla", rdesign(pts=[P(0, 40), P(20, 40)], fit=None)),
        ("iki yan korkuluklu rampa", rdesign(railings="both")),
        ("sol korkuluklu L rampa", rdesign(pts=[P(0, 40), P(4, 40), P(4, 42)], railings="left")),
        ("katlı rampa eğime göre biter", rdesign(arch={"level": "zemin"})),
    ]:
        c.add(name, [create_step(Drawing(), RAMPS, "execute", inp)])
    c.add("plan eğime göre biten rampayı gösterir", [create_step(Drawing(), RAMPS, "plan", rdesign(railings="right"))])
    refusals(c, RAMPS, [
        ("eğimsiz rampa eğime göre bitirilemez", rdesign(slope=None), {}),
        ("kot farkı sıfırken eğime göre bitirilemez", rdesign(rise=0.0), {}),
        ("eğimli uzunluk dönüşten önce biterse reddedilir", rdesign(pts=[P(0, 40), P(12, 40), P(12, 43)]), {}),
        ("sonsuz kot farkı önce reddedilir", rdesign(), {"non_finite": {"ramps[0].rise": "NaN"}}),
        ("sıfır genişlik sığdırmadan önce reddedilir", rdesign(width=0.0), {}),
    ])
    return c.cases


def ramp(**kw):
    r = {"pts": [P(0, 40), P(10, 40)], "width": 1.5, "rise": 0.5, "slope": 5.0}
    r.update(kw)
    return {"layerId": "merdiven", "ramps": [r]}


def ramp_cases():
    c = Cases()
    life(c, RAMPS, ramp(), "Eğimi uyan rampa")
    plain = ramp()
    del plain["ramps"][0]["slope"]
    for name, inp in [
        ("eğimsiz rampa", plain),
        ("eğimi uymayan L rampa ramp_slope_mismatch uyarısı", ramp(pts=[P(0, 40), P(6, 40), P(6, 46)], rise=0.6, slope=6.0)),
        ("sahanlığı bütün yolu alan rampa ramp_no_run uyarısı", ramp(pts=[P(0, 40), P(1, 40), P(1, 41)], landing=5.0, slope=None)),
        ("eksi başlangıç kotuyla inen rampa", ramp(pts=[P(0, 40), P(0, 50)], width=1.2, rise=-0.5, base=-0.3, slope=None)),
        ("U rampa, kısa sahanlık", ramp(pts=[P(0, 40), P(5, 40), P(5, 43), P(0, 43)], landing=1.0, slope=None)),
        ("katlı rampa", ramp(arch={"level": "zemin", "phase": "existing"})),
    ]:
        for r in inp["ramps"]:
            if r.get("slope") is None:
                r.pop("slope", None)
        c.add(name, [create_step(Drawing(), RAMPS, "execute", inp)])
    refusals(c, RAMPS, [
        ("rampa verilmezse reddedilir", {"layerId": "merdiven", "ramps": []}, {}),
        ("sonsuz yükseklik reddedilir", ramp(), {"non_finite": {"ramps[0].rise": "NaN"}}),
        ("sonsuz köşe reddedilir", ramp(), {"non_finite": {"ramps[0].pts[1].y": "Infinity"}}),
        ("tek noktalı rampa reddedilir", ramp(pts=[P(0, 0)]), {}),
        ("sıfır genişlik reddedilir", ramp(width=0.0), {}),
        ("çok yüksek rampa reddedilir", ramp(rise=2000.0), {}),
        ("sıfır eğim reddedilir", ramp(slope=0.0), {}),
        ("sıfır sahanlık reddedilir", ramp(landing=0.0), {}),
        ("olmayan kat reddedilir", ramp(arch={"level": "yok"}), {}),
    ])
    common_refusals(c, RAMPS, ramp())
    return c.cases

def ramp_level_cases():
    """docs/adr/0723 §2: a ramp's start bound to its level: written, followed when the level rises, set and freed."""
    c = Cases()

    def bound(level="zemin", offset=0.0, **kw):
        return ramp(arch={"level": level}, baseOffset=offset, **kw)

    life(c, RAMPS, bound("kat-1", -0.15), "1. katın 15 cm altından çıkan kata bağlı rampa")
    for name, inp in [
        ("zemin kata bağlı rampa: başlangıcı katın kotu", bound()),
        ("1. kata bağlı rampa: başlangıcı 1. katın kotu", bound("kat-1")),
        ("1. kattan inen kata bağlı rampa", bound("kat-1", 0.0, pts=[P(0, 40), P(-10, 40)], rise=-0.5)),
        ("kata bağlı, eğime göre biten ve en uzun kollu rampa", bound("kat-1", 0.3, pts=[P(0, 40), P(30, 40)], rise=1.2,
                                                                       landing=1.5, fit=True, maxRun=9.0)),
        ("kata bağlı, kotu katın 2 cm üstünde başlayan rampa", bound("zemin", 0.02)),
    ]:
        c.add(name, [create_step(Drawing(), RAMPS, "execute", inp)])
    nan = bound("kat-1", 0.0)
    refusals(c, RAMPS, [
        ("başlangıç kotu ve kata göre fark birlikte reddedilir", bound("kat-1", 0.1, base=3.0), {}),
        ("katsız rampa kata bağlanamaz", ramp(baseOffset=0.0), {}),
        ("olmayan kata bağlı rampa reddedilir", bound("yok"), {}),
        ("kata göre fark 1000 m'den çok reddedilir", bound("kat-1", -1000.5), {}),
        ("sonsuz kata göre fark reddedilir", nan, {"non_finite": {"ramps[0].baseOffset": "Infinity"}}),
    ])
    # The level raised (a project setting, no undo step): a bound ramp's start follows it in place; one with a base of
    # its own stays.
    raised = [LEVELS[0], {**LEVELS[1], "elevation": 6.5}]
    for name, inp in [
        ("kat yükselince bağlı rampanın başlangıcı da yükselir, geri alma adımı olmadan", bound("kat-1", -0.15)),
        ("kat yükselince kendi kotlu rampa yerinde kalır", ramp(arch={"level": "kat-1"}, base=0.45)),
    ]:
        d = Drawing()
        made = create_step(d, RAMPS, "execute", inp)
        f = copy.deepcopy(d.objs[NEW][2])
        level_bound(f, raised)
        after = {"kind": "ramp", "id": NEW, "layerId": "merdiven", "attrs": {}, **f}
        c.add(name, [made,
                     {"op": "execute", "command": "cad.settings.set", "input": {"arch": {"levels": raised}}, "result": "$any",
                      "within": 1e-9,
                      "expect": {"ids": IDS + [NEW], "entities": {str(NEW): after}, "settings": {"arch": {"levels": raised}},
                                 "canUndo": True, "canRedo": False}},
                     {"op": "undo", "returns": "Rampa", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
                     {"op": "redo", "returns": "Rampa", "within": 1e-9,
                      "expect": {"ids": IDS + [NEW], "entities": {str(NEW): after}}}])

    def one(d, uids, op, **kw):
        return edit_step(d, "execute", {"uids": uids, "operation": op}, **kw)

    def with_bound(level="zemin", offset=0.1):
        d = Drawing()
        return d, create_step(d, RAMPS, "execute", bound(level, offset), with_command=True)

    d = Drawing()
    c.add("Değiştir: rampa kata bağlanır, başlangıcı katın kotu artı fark",
          [one(d, [uid_of(9)], {"kind": "set", "level": "kat-1", "baseOffset": 0.2})])
    d = Drawing()
    c.add("Değiştir: kata göre fark katsız rampada reddedilir", [one(d, [uid_of(9)], {"kind": "set", "baseOffset": 0.2})])
    d, made = with_bound()
    c.add("Değiştir: null kat bağını kaldırır, başlangıç kotu yerinde kalır",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "baseOffset": None})])
    d, made = with_bound()
    c.add("Değiştir: kendi başlangıç kotu yazılınca kat bağı kalkar", [made, one(d, [uid_of(NEW)], {"kind": "set", "base": 0.8})])
    d, made = with_bound()
    c.add("Değiştir: bağlı rampanın katı değişince başlangıcı yeni katı izler",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "level": "kat-1"})])
    d, made = with_bound()
    c.add("Değiştir: bağlı rampanın farkı değişince başlangıcı da değişir",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "baseOffset": -0.3})])
    d, made = with_bound()
    c.add("Değiştir: bağlı rampanın katı kaldırılamaz", [made, one(d, [uid_of(NEW)], {"kind": "set", "level": None})])
    d, made = with_bound()
    c.add("Değiştir: başlangıç kotu ve fark birlikte reddedilir",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "base": 1.0, "baseOffset": 0.0})])
    d = Drawing()
    c.add("Değiştir: merdivene kata göre fark reddedilir", [one(d, [uid_of(8)], {"kind": "set", "baseOffset": 0.0})])
    d, made = with_bound()
    c.add("Değiştir: sonsuz kata göre fark reddedilir",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "baseOffset": 0.0}, non_finite={"operation.baseOffset": "NaN"})])
    d, made = with_bound()
    c.add("Değiştir: bağlı rampanın planı yeni başlangıcı gösterir",
          [made, edit_step(d, "plan", {"uids": [uid_of(NEW)], "operation": {"kind": "set", "level": "kat-1", "baseOffset": 0.5}})])
    return c.cases

def stair_level_cases():
    """docs/adr/0724 §1, §3: a stair's rise bound to its level's height: drawn from it, followed when it changes, set and
    freed; a curved stair's centre post."""
    c = Cases()

    def bound(level="zemin", **kw):
        return design(height=None, heightFromLevel=True, arch={"level": level}, **kw)

    life(c, STAIRS, bound(), "Zemin katın yüksekliğine bağlı merdiven")
    for name, inp in [
        ("kata bağlı, en fazla 18 cm rıhtla 17 rıht", bound(maxRiser=0.18)),
        ("kata bağlı, verilen 20 rıht: rıht yüksekliği katın yüksekliğinden", bound(risers=20)),
        ("kata bağlı U merdiven, kesme yüksekliği 1,10 m", bound(pts=[P(0, 30), P(3, 30), P(3, 32), P(0, 32)], width=1.0,
                                                                 cutHeight=1.1)),
        ("kata bağlı döner merdiven, merkez direğiyle", curved(height=None, heightFromLevel=True, arch={"level": "zemin"}, post=0.2)),
        ("döner merdivenin merkez direği, iki yan korkuluk", curved(post=0.3, railings="both")),
        ("heightFromLevel false: yükseklik kendisi", design(heightFromLevel=False, arch={"level": "zemin"})),
    ]:
        c.add(name, [create_step(Drawing(), STAIRS, "execute", inp)])
    refusals(c, STAIRS, [
        ("yükseklik ve kat bağı birlikte reddedilir", design(heightFromLevel=True, arch={"level": "zemin"}), {}),
        ("katsız merdiven kata bağlanamaz", design(height=None, heightFromLevel=True), {}),
        ("olmayan kata bağlı merdiven reddedilir", bound("yok"), {}),
        ("yüksekliği olmayan kata bağlanamaz", bound("kat-1"), {}),
        ("düz merdivende merkez direği reddedilir", design(post=0.2), {}),
        ("sıfır merkez direği reddedilir", curved(post=0.0), {}),
        ("iç yarıçaptan büyük merkez direği reddedilir", curved(post=2.0), {}),
        ("sonsuz merkez direği reddedilir", curved(post=0.2), {"non_finite": {"stairs[0].post": "NaN"}}),
    ])
    # The level's height raised (a project setting, no undo step): a bound stair's risers follow it, their count kept;
    # one with a height of its own stays.
    raised = [{**LEVELS[0], "height": 3.4}, LEVELS[1]]
    for name, inp in [
        ("kat yükselince bağlı merdivenin rıhtları da yükselir, geri alma adımı olmadan", bound()),
        ("kat yükselince kendi yükseklikli merdiven yerinde kalır", design(arch={"level": "zemin"})),
    ]:
        d = Drawing()
        made = create_step(d, STAIRS, "execute", inp)
        f = copy.deepcopy(d.objs[NEW][2])
        stair_bound(f, raised)
        after = {"kind": "stair", "id": NEW, "layerId": "merdiven", "attrs": {}, **f}
        c.add(name, [made,
                     {"op": "execute", "command": "cad.settings.set", "input": {"arch": {"levels": raised}}, "result": "$any",
                      "within": 1e-9,
                      "expect": {"ids": IDS + [NEW], "entities": {str(NEW): after}, "settings": {"arch": {"levels": raised}},
                                 "canUndo": True, "canRedo": False}},
                     {"op": "undo", "returns": "Merdiven", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
                     {"op": "redo", "returns": "Merdiven", "within": 1e-9,
                      "expect": {"ids": IDS + [NEW], "entities": {str(NEW): after}}}])

    def one(d, uids, op, **kw):
        return edit_step(d, "execute", {"uids": uids, "operation": op}, **kw)

    def made_with(inp):
        d = Drawing()
        return d, create_step(d, STAIRS, "execute", inp, with_command=True)

    d = Drawing()
    c.add("Değiştir: merdivene kat verilip yüksekliği kata bağlanır, rıhtı katın yüksekliğinden",
          [one(d, [uid_of(8)], {"kind": "set", "level": "zemin", "heightFromLevel": True})])
    d, made = made_with(bound())
    c.add("Değiştir: rıht sayısı değişince bağlı merdivenin rıhtı katın yüksekliğinden",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "risers": 20})])
    d, made = made_with(bound())
    c.add("Değiştir: kendi rıht yüksekliği kat bağını kaldırır", [made, one(d, [uid_of(NEW)], {"kind": "set", "riserHeight": 0.16})])
    d, made = made_with(bound())
    c.add("Değiştir: false kat bağını kaldırır, rıhtlar yerinde kalır",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "heightFromLevel": False})])
    d, made = made_with(bound())
    c.add("Değiştir: rıht yüksekliği ve kat bağı birlikte reddedilir",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "riserHeight": 0.16, "heightFromLevel": True})])
    d = Drawing()
    c.add("Değiştir: yüksekliği olmayan kata bağlanamaz",
          [one(d, [uid_of(8)], {"kind": "set", "level": "kat-1", "heightFromLevel": True})])
    d = Drawing()
    c.add("Değiştir: katsız merdiven kata bağlanamaz", [one(d, [uid_of(8)], {"kind": "set", "heightFromLevel": True})])
    d, made = made_with(bound())
    c.add("Değiştir: bağlı merdivenin katı kaldırılamaz", [made, one(d, [uid_of(NEW)], {"kind": "set", "level": None})])
    d = Drawing()
    c.add("Değiştir: rampaya kat yüksekliği bağı reddedilir", [one(d, [uid_of(9)], {"kind": "set", "heightFromLevel": True})])
    d, made = made_with(curved())
    c.add("Değiştir: döner merdivene merkez direği, sonra null kaldırır",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "post": 0.25}), one(d, [uid_of(NEW)], {"kind": "set", "post": None})])
    d = Drawing()
    c.add("Değiştir: düz merdivene merkez direği reddedilir", [one(d, [uid_of(8)], {"kind": "set", "post": 0.2})])
    d, made = made_with(curved(post=0.2))
    c.add("Değiştir: merdiven genişleyince sığmayan merkez direği reddedilir",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "width": 2.9})])
    d, made = made_with(curved())
    c.add("Değiştir: sonsuz merkez direği reddedilir",
          [made, one(d, [uid_of(NEW)], {"kind": "set", "post": 0.2}, non_finite={"operation.post": "NaN"})])
    d, made = made_with(bound())
    c.add("Değiştir: bağlı merdivenin planı katın yüksekliğinden rıhtı gösterir",
          [made, edit_step(d, "plan", {"uids": [uid_of(NEW)], "operation": {"kind": "set", "risers": 17}})])
    return c.cases

# ── cad.arch.edit ──────────────────────────────────────────────────────────────────────────────────────────────


FIELDS = [  # wire name, kinds, nullable
    ("label", ("grid",), False), ("bubbles", ("grid",), False), ("bubbleSize", ("grid",), True),
    ("shape", ("column",), False), ("width", ("column", "stair", "ramp"), False), ("depth", ("column",), True),
    ("rotation", ("column",), False), ("type", ("column",), True), ("tag", ("column",), True), ("fill", ("column",), False),
    ("risers", ("stair",), False), ("riserHeight", ("stair",), False), ("heightFromLevel", ("stair",), False),
    ("post", ("stair",), True), ("tread", ("stair",), False),
    ("landing", ("stair", "ramp"), True), ("cut", ("stair",), True), ("numbering", ("stair",), False),
    ("railings", ("stair", "ramp"), True),
    ("rise", ("ramp",), False), ("slope", ("ramp",), True), ("base", ("ramp",), True), ("baseOffset", ("ramp",), True),
    ("level", ("grid", "column", "stair", "ramp"), True), ("phase", ("grid", "column", "stair", "ramp"), True),
]
KIND_NAME = {"grid": "aks", "column": "kolon", "stair": "merdiven", "ramp": "rampa"}
DEFAULTS = {"bubbles": "both", "shape": "rect", "rotation": 0.0, "fill": False, "numbering": False, "heightFromLevel": False}
NUMBERS = [("bubbleSize", "Balon çapı"), ("width", "Genişlik"), ("depth", "Boy"), ("rotation", "Dönüş"),
           ("riserHeight", "Rıht yüksekliği"), ("post", "Merkez direği"), ("tread", "Basamak derinliği"), ("landing", "Sahanlık"),
           ("rise", "Yükseklik"), ("slope", "Eğim"), ("base", "Başlangıç kotu"), ("baseOffset", "Kata göre başlangıç kotu")]


def set_field(f, name, value):
    if name == "label":
        name = "name"
    if name in ("level", "phase"):
        arch = dict(f.get("arch", {}))
        if value is None:
            arch.pop(name, None)
        else:
            arch[name] = value
        if arch:
            f["arch"] = arch
        else:
            f.pop("arch", None)
        return
    if name == "base":
        # A base of its own frees a ramp from its level (docs/adr/0723 §2).
        f.pop("baseOffset", None)
    if name == "riserHeight":
        # A riser height of its own frees a stair from its level's height (docs/adr/0724 §1).
        f.pop("heightFromLevel", None)
    if value is None or DEFAULTS.get(name, object()) == value:
        f.pop(name, None)
    else:
        f[name] = value


def problem_of(kind, f, path):
    return {"grid": grid_problem, "column": column_problem, "stair": stair_problem, "ramp": ramp_problem}[kind](f, path)


def edit(d, inp, conflict=False):
    uids = inp["uids"]
    if not uids:
        return refusal("no_entities", "Düzenlenecek aks, kolon, merdiven ya da rampa verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"), None
    for i, u in enumerate(uids):
        if not is_uid(u):
            return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", f"uids[{i}]"), None
    found, seen = [], set()
    for i, u in enumerate(uids):
        if u in seen:
            continue
        seen.add(u)
        oid = int(u[7:]) if u.startswith("$uidOf:") else None
        if oid not in d.objs:
            return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", f"uids[{i}]"), None
        found.append((i, oid))
    for i, oid in found:
        if d.objs[oid][0] not in KIND_NAME:
            return refusal("not_arch_element", "Nesne aks, kolon, merdiven ya da rampa değil; Mimari nesneyi düzenle yalnız bunları değiştirir.", f"uids[{i}]"), None
    op = inp["operation"]
    out, warnings = [], []
    if op["kind"] == "set":
        given = [(n, kinds, nul) for n, kinds, nul in FIELDS if n in op]
        if not given:
            return refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi.", "operation"), None
        for _, oid in found:
            kind = d.objs[oid][0]
            bad = next((n for n, kinds, _ in given if kind not in kinds), None)
            if bad:
                return refusal("field_not_for_kind", f"“{bad}” alanı {KIND_NAME[kind]} nesnesinde yok.", f"operation.{bad}"), None
        for n, name in NUMBERS:
            v = op.get(n)
            if v is not None and not math.isfinite(v):
                return not_finite(name, f"operation.{n}"), None
        if "riserHeight" in op and op.get("heightFromLevel") is True:
            return refusal("invalid_stair", "Rıht yüksekliği ya kendisi (riserHeight) ya katın yüksekliğinden (heightFromLevel) "
                                            "verilir; ikisi birden verilmez.", "operation.heightFromLevel"), None
        if "base" in op and "baseOffset" in op:
            return refusal("invalid_ramp", BOTH_BASES, "operation.baseOffset"), None
        for i, oid in found:
            kind, ly, f = d.objs[oid]
            f = copy.deepcopy(f)
            for n, _, _ in given:
                set_field(f, n, op[n])
            path = f"uids[{i}]"
            r = problem_of(kind, f, path)
            if r:
                return r, None
            if kind == "column":
                r = settled_on(d, f, path)
                if r:
                    return r, None
            r = level_check(f.get("arch"), "operation.level")
            if r:
                return r, None
            if kind == "ramp":
                level_bound(f)
                warnings += ramp_warnings(f, path)
            if kind == "stair":
                # Bound now (docs/adr/0724 §1): its level must have a height; its riser height read from it.
                if op.get("heightFromLevel") is True and f.get("heightFromLevel"):
                    lv = level_of((f.get("arch") or {}).get("level"))
                    if lv is None or lv.get("height") is None or not f.get("risers"):
                        return refusal("invalid_stair", "Merdivenin katının yüksekliği yok; merdiven onu izleyemez. Proje "
                                                        "ayarları › Mimari'de katın yüksekliğini verin.", "operation.heightFromLevel"), None
                stair_bound(f)
                warnings += stair_warnings(f, path)
            out.append((oid, f))
    elif op["kind"] == "renumber":
        kinds = [d.objs[oid][0] for _, oid in found]
        if kinds[0] not in ("grid", "column") or any(k != kinds[0] for k in kinds):
            return refusal("mixed_kinds", "Numarala yalnız aksları ya da yalnız kolonları numaralar; ikisi birlikte ya da başka tür verilmez.", "uids"), None
        label = op["start"]
        for k, (i, oid) in enumerate(found):
            if k:
                label = next_label(label)
            kind, ly, f = d.objs[oid]
            f = copy.deepcopy(f)
            f["name" if kind == "grid" else "tag"] = label
            r = problem_of(kind, f, f"uids[{i}]")
            if r:
                return r, None
            out.append((oid, f))
    elif op["kind"] == "attach":
        for _, oid in found:
            kind = d.objs[oid][0]
            if kind != "column":
                return refusal("field_not_for_kind", f"Akslara yalnız kolon bağlanır; {KIND_NAME[kind]} bağlanmaz.", "operation.grids"), None
        for i, oid in found:
            f = copy.deepcopy(d.objs[oid][2])
            f["grids"] = op["grids"]
            f.pop("place", None)
            f.pop("broken", None)
            r = settled_on(d, f, "operation")
            if r:
                return r, None
            out.append((oid, f))
    elif op["kind"] == "detach":
        for i, oid in found:
            kind = d.objs[oid][0]
            if kind != "column":
                return refusal("field_not_for_kind", f"Bağı yalnız kolonun kopar; {KIND_NAME[kind]} bağlı değildir.", "operation"), None
            f = copy.deepcopy(d.objs[oid][2])
            for k in ("grids", "place", "broken"):
                f.pop(k, None)
            out.append((oid, f))
    r = revision(inp, conflict)
    if r:
        return r, None
    for i, oid in found:
        if d.objs[oid][1] == "kilitli":
            return refusal("layer_locked", f"“Kilitli katman” katmanı kilitli; üzerindeki {KIND_NAME[d.objs[oid][0]]} düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{i}]"), None
    return {"warnings": warnings}, out


def edit_step(d, op, inp, non_finite=None, conflict=False):
    result, work = edit(d, with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op}
    if op == "execute" or op == "plan" or op == "validate":
        s["command"] = EDIT
    s["input"] = inp
    if non_finite:
        s["nonFinite"] = non_finite
    if work is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": result["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "plan":
        out = [{"kind": d.objs[oid][0], "id": oid, "layerId": d.objs[oid][1], "attrs": {}, **f} for oid, f in work]
        s["result"] = {"status": "completed", "output": {"elements": out, "revision": "$current"}, "warnings": result["warnings"]}
        s["within"] = 1e-9
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    for oid, f in work:
        k, ly, _ = d.objs[oid]
        d.objs[oid] = (k, ly, f)
    followed = d.follow()
    changed = [oid for oid, _ in work]
    s["result"] = {"status": "completed", "output": {"elements": [entity(d, oid) for oid in changed], "revision": "$current"},
                   "warnings": result["warnings"]}
    s["within"] = 1e-9
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(d, i) for i in changed + [x for x in followed if x not in changed]},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def edit_cases():
    c = Cases()

    def with_column(**kw):
        d = Drawing()
        made = create_step(d, COLUMNS, "execute", col([{"at": P(3, 3), "width": 0.3, **kw}]), with_command=True)
        return d, made, d.next - 1

    def one(d, uids, op, **kw):
        return edit_step(d, "execute", {"uids": uids, "operation": op}, **kw)

    d = Drawing()
    c.add("Değiştir: aksın etiketi, balonları ve balon çapı", [one(d, [uid_of(1)], {"kind": "set", "label": "A1", "bubbles": "end", "bubbleSize": 12.0})])
    d = Drawing()
    c.add("Değiştir: null balon çapını kaldırır", [one(d, [uid_of(2)], {"kind": "set", "bubbleSize": 12.0}),
                                                   one(d, [uid_of(2)], {"kind": "set", "bubbleSize": None})])
    d, made, k = with_column()
    c.add("Değiştir: kolonun biçimi, ölçüleri, dönüşü, türü, etiketi ve dolgusu", [made, one(d, [uid_of(k)], {"kind": "set", "shape": "round", "width": 0.45, "rotation": 0.3, "type": "Çelik", "tag": "S4", "fill": True})])
    d, made, k = with_column(depth=0.6, type="Betonarme")
    c.add("Değiştir: null kolonun boyunu ve türünü kaldırır", [made, one(d, [uid_of(k)], {"kind": "set", "depth": None, "type": None})])
    d = Drawing()
    c.add("Değiştir: merdivenin rıhtları uyarılarıyla", [one(d, [uid_of(8)], {"kind": "set", "risers": 17, "riserHeight": 0.18})])
    d = Drawing()
    c.add("Değiştir: merdivenin sahanlığı, kesmesi ve numaraları", [one(d, [uid_of(8)], {"kind": "set", "landing": 1.0, "cut": 8, "numbering": True})])
    d = Drawing()
    c.add("Değiştir: rampanın eğimi uymayınca uyarı", [one(d, [uid_of(9)], {"kind": "set", "slope": 8.0})])
    d = Drawing()
    c.add("Değiştir: null rampanın eğimini kaldırır, başlangıç kotu", [one(d, [uid_of(9)], {"kind": "set", "slope": None, "base": 0.45})])
    d = Drawing()
    c.add("Değiştir: genişlik merdivene ve rampaya birlikte", [one(d, [uid_of(8), uid_of(9)], {"kind": "set", "width": 1.4})])
    d = Drawing()
    c.add("Değiştir: merdivene ve rampaya iki yan korkuluk (ADR 0705)", [one(d, [uid_of(8), uid_of(9)], {"kind": "set", "railings": "both"})])
    d = Drawing()
    c.add("Değiştir: korkuluk yanı sola, sonra null kaldırır", [one(d, [uid_of(8)], {"kind": "set", "railings": "left"}),
                                                              one(d, [uid_of(8)], {"kind": "set", "railings": None})])
    d = Drawing()
    c.add("Değiştir: aksa korkuluk reddedilir", [one(d, [uid_of(1)], {"kind": "set", "railings": "right"})])
    d = Drawing()
    c.add("Değiştir: kat ve evre her türde, null kaldırır", [one(d, [uid_of(1), uid_of(8)], {"kind": "set", "level": "kat-1", "phase": "new"}),
                                                             one(d, [uid_of(1)], {"kind": "set", "level": None})])
    d = Drawing()
    c.add("Numarala: akslar verilen sırayla", [one(d, [uid_of(2), uid_of(1), uid_of(7)], {"kind": "renumber", "start": "1"})])
    d, made, k = with_column()
    d2 = d
    made2 = create_step(d2, COLUMNS, "execute", col([{"at": P(4, 4), "width": 0.3}]), with_command=True)
    c.add("Numarala: kolonların etiketleri", [made, made2, one(d2, [uid_of(k), uid_of(k + 1)], {"kind": "renumber", "start": "K1"})])
    d, made, k = with_column()
    c.add("Akslara bağla: kolon kesişime gider (B-2)", [made, one(d, [uid_of(k)], {"kind": "attach", "grids": [uid_of(2), uid_of(4)]})])
    d, made, k = with_column(grids=[uid_of(1), uid_of(3)])
    c.add("Bağı kopar: kolon yerinde kalır", [made, one(d, [uid_of(k)], {"kind": "detach"})])
    d, made, k = with_column(grids=[uid_of(1), uid_of(3)])
    c.add("Aksın etiketi değişince bağlı kolonun konumu da değişir", [made, one(d, [uid_of(1)], {"kind": "set", "label": "D"})])
    d, made, k = with_column(grids=[uid_of(1), uid_of(3)])
    c.add("Akslar numaralanınca bağlı kolonun konumu izler", [made, one(d, [uid_of(1), uid_of(3)], {"kind": "renumber", "start": "E"})])
    d = Drawing()
    c.add("Değiştir: kısa basamak yolu uzun bırakır, Blondel dışı", [one(d, [uid_of(8)], {"kind": "set", "tread": 0.2})])
    d = Drawing()
    c.add("Değiştir: kat verilince rıhtlar katın yüksekliğiyle denetlenir", [one(d, [uid_of(8)], {"kind": "set", "level": "zemin"})])
    d = Drawing()
    inp = ramp(pts=[P(0, 40), P(4, 40), P(4, 44)])
    inp["ramps"][0].pop("slope")
    made = create_step(d, RAMPS, "execute", inp, with_command=True)
    c.add("Değiştir: sahanlık bütün yolu alınca ramp_no_run", [made, one(d, [uid_of(d.next - 1)], {"kind": "set", "landing": 20.0})])
    d = Drawing()
    c.add("Doğrula yazmaz", [edit_step(d, "validate", {"uids": [uid_of(8)], "operation": {"kind": "set", "tread": 0.3}})])
    d = Drawing()
    c.add("Plan yazmadan gösterir", [edit_step(d, "plan", {"uids": [uid_of(8)], "operation": {"kind": "set", "tread": 0.3}})])
    d = Drawing()
    c.add("Yazılır ve geri alınır", [one(d, [uid_of(9)], {"kind": "set", "rise": 0.6}),
                                     {"op": "undo", "returns": "Mimari nesneyi düzenle", "expect": {"canRedo": True, "revision": "changed"}}])
    for name, inp, kw in [
        ("kimlik verilmezse reddedilir", {"uids": [], "operation": {"kind": "detach"}}, {}),
        ("geçersiz kimlik reddedilir", {"uids": ["aks"], "operation": {"kind": "detach"}}, {}),
        ("olmayan nesne reddedilir", {"uids": [MISSING], "operation": {"kind": "detach"}}, {}),
        ("mimari olmayan nesne reddedilir", {"uids": [uid_of(6)], "operation": {"kind": "set", "level": "zemin"}}, {}),
        ("boş değişiklik reddedilir", {"uids": [uid_of(1)], "operation": {"kind": "set"}}, {}),
        ("aksın genişliği yok", {"uids": [uid_of(1)], "operation": {"kind": "set", "width": 1.0}}, {}),
        ("merdivenin etiketi yok", {"uids": [uid_of(8)], "operation": {"kind": "set", "label": "M"}}, {}),
        ("rampanın rıhtı yok, merdiven ve rampa birlikte", {"uids": [uid_of(8), uid_of(9)], "operation": {"kind": "set", "risers": 12}}, {}),
        ("sonsuz genişlik reddedilir", {"uids": [uid_of(8)], "operation": {"kind": "set", "width": 1.0}}, {"non_finite": {"operation.width": "NaN"}}),
        ("sıfır enli rampa reddedilir", {"uids": [uid_of(9)], "operation": {"kind": "set", "width": 0.0}}, {}),
        ("sıfır rıht reddedilir", {"uids": [uid_of(8)], "operation": {"kind": "set", "risers": 0}}, {}),
        ("boş aks etiketi reddedilir", {"uids": [uid_of(1)], "operation": {"kind": "set", "label": " "}}, {}),
        ("olmayan kat reddedilir", {"uids": [uid_of(9)], "operation": {"kind": "set", "level": "yok"}}, {}),
        ("aks ve merdiven birlikte numaralanmaz", {"uids": [uid_of(1), uid_of(8)], "operation": {"kind": "renumber", "start": "A"}}, {}),
        ("merdiven numaralanmaz", {"uids": [uid_of(8)], "operation": {"kind": "renumber", "start": "1"}}, {}),
        ("aks akslara bağlanmaz", {"uids": [uid_of(1)], "operation": {"kind": "attach", "grids": [uid_of(2), uid_of(3)]}}, {}),
        ("rampanın bağı koparılmaz", {"uids": [uid_of(9)], "operation": {"kind": "detach"}}, {}),
        ("kilitli katmandaki aks düzenlenmez", {"uids": [uid_of(5)], "operation": {"kind": "set", "label": "L"}}, {}),
        ("geçersiz beklenen sürüm reddedilir", {"uids": [uid_of(1)], "operation": {"kind": "set", "label": "Q"}, "expectedRevision": "07"}, {}),
    ]:
        c.add(name, [edit_step(Drawing(), "execute", inp, **kw)])
    d, made, k = with_column()
    c.add("sıfır enli kolon reddedilir", [made, edit_step(d, "execute", {"uids": [uid_of(k)], "operation": {"kind": "set", "width": 0.0}})])
    d, made, k = with_column()
    c.add("paralel akslara bağlanmaz", [made, edit_step(d, "execute", {"uids": [uid_of(k)], "operation": {"kind": "attach", "grids": [uid_of(1), uid_of(2)]}})])
    d, made, k = with_column()
    c.add("aks olmayana bağlanmaz", [made, edit_step(d, "execute", {"uids": [uid_of(k)], "operation": {"kind": "attach", "grids": [uid_of(6), uid_of(1)]}})])
    d = Drawing()
    made = edit_step(d, "execute", {"uids": [uid_of(1)], "operation": {"kind": "set", "label": "Q"}})
    stale = edit_step(d, "execute", {"uids": [uid_of(1)], "operation": {"kind": "set", "label": "R"}, "expectedRevision": "$prepared"}, conflict=True)
    c.add("sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c.cases


def document(command, title, cases):
    note = ("ADR 0343 §1–§4. Beklenenler scripts/fixtures/arch_command_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "akslar düz ve eksenlere paralel, kesişimleri kesirlerle. Kurulumdaki çizimde 9 nesne var; yeni nesne 10'dan başlar.")
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note,
            "setup": SETUP, "cases": cases}


def files():
    return {
        "cad.grids.create.json": document(GRIDS, "Aks: tek aks, seri, etiketlerin sırası, balonlar, yaylı aks; doğrulama, plan, yazma, geri alma, retler", grid_cases()),
        "cad.columns.create.json": document(COLUMNS, "Kolon: merkezden, akslara bağlı, aks kesişimleri, etiketler; aksla gitmek; retler", column_cases()),
        "cad.stairs.create.json": document(STAIRS, "Merdiven: düz, L, U; sahanlık, kesme, numaralar; denetimlerin uyarıları; retler", stair_cases()),
        "cad.stairs.create.design.json": document(STAIRS, "Merdiven yükseklikten: rıht sayısı ve yüksekliği, kesme yüksekliği, yola sığdırma, tek sahanlıklı U; retler (ADR 0704)", design_cases()),
        "cad.stairs.create.curved.json": document(STAIRS, "Döner merdiven: yaylı yürüme çizgisi, yükseklikten rıhtlara göre biten yay, iki yön, kesme ve korkuluk, ayna; retler (ADR 0709)", curved_cases()),
        "cad.ramps.create.json": document(RAMPS, "Rampa: düz, L, U; eğim, başlangıç kotu, sahanlık; denetimlerin uyarıları; retler", ramp_cases()),
        "cad.ramps.create.design.json": document(RAMPS, "Rampa eğimden: son kol eğime göre, üst üste sahanlık bir kez, korkuluk; retler (ADR 0705)", ramp_design_cases()),
        "cad.ramps.create.levels.json": document(RAMPS, "Rampa katına bağlı: başlangıcı katın kotu artı fark, kat yükselince izler; değiştir, bağı kaldır; retler (ADR 0723 §2)", ramp_level_cases()),
        "cad.stairs.create.levels.json": document(STAIRS, "Merdiven katın yüksekliğine bağlı: rıhtları katın yüksekliğinden, kat değişince izler; değiştir, bağı kaldır; döner merdivenin merkez direği; retler (ADR 0724 §1, §3)", stair_level_cases()),
        "cad.grids.create.system.json": document(GRIDS, "Aks ağı: iki yönde aralık listeleri, açı, uzatma, ilk adlar, kesişimlerde akslara bağlı kolonlar; retler (ADR 0707)", system_cases()),
        "cad.arch.edit.json": document(EDIT, "Mimari nesneyi düzenle: değiştir, numarala, akslara bağla, bağı kopar; kolonun akslarla gitmesi; retler", edit_cases()),
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
