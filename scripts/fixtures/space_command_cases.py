#!/usr/bin/env python3
"""The space commands' shared cases (docs/adr/0342 §3, §4), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.spaces.{create,edit,quantities}.json; `--check` compares and writes nothing. A space
found from the walls is the space reference's (`space_cases.py`: the face the walls' faces close round the seed, moved
by the method); its rings' order is the implementation's, so a case says its boundary through Metraj: the area, the
perimeter and the area by each method, exact fractions as floats. The rules, as the ADR gives them:

- `cad.spaces.create`: no space (`no_objects`); each in order: its numbers (seed, height, the drawn rings' points),
  its boundary (from the walls round the seed: `space_not_enclosed`; drawn; neither: `invalid_space`), its fields
  (`invalid_space`), its level (`level_not_found`); then the line weight, the revision, the layer;
- `cad.spaces.edit`: the ids, spaces only, the operation's values; Değiştir sets fields (a new method finds a space
  from the walls again), Birleştir makes the spaces one drawn space (the first keeps its id; they must touch), Böl
  cuts each along a line (the largest keeps its id), Duvarlardan bul finds the boundary from the walls round the seed
  given, else its own, else its label's point; the revision; a locked layer refuses all;
- `cad.spaces.quantities`: spaces, walls and openings measured as the ADR says (§4), filtered by levels, phases,
  usages, layers; totals summed in the rows' order; a room's baseboard, the slabs and the floor finishes' changes at
  the passages as ADR 0722 §5 says (`baseboard`, `slab_row`, `transitions` below);
- the document finds a space from its walls again when they change (a wall moved), and breaks it (`lostEnclosure`)
  when they no longer close round it.
"""

import argparse
import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import space_cases as sp  # noqa: E402  (the boundary from the walls)
from wall_join_cases import from_json, path_len  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def num(x):
    if isinstance(x, float) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def layer(id, name, kind="layer", visible=True, locked=False, children=None):
    return {"id": id, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


def base(id, layer_id, **extra):
    return {"id": id, "layerId": layer_id, "attrs": {}, **extra}


def wall(id, layer_id, pts, width, **extra):
    return {"kind": "wall", **base(id, layer_id), "pts": pts, "width": width, "justify": "center", **extra}


ZEMIN = {"level": "zemin"}
ENTITIES = [
    wall(1, "duvar", [P(0, 0), P(8, 0)], 0.2, arch=ZEMIN),
    wall(2, "duvar", [P(8, 0), P(8, 5)], 0.2, arch=ZEMIN),
    wall(3, "duvar", [P(8, 5), P(0, 5)], 0.2, arch=ZEMIN),
    wall(4, "duvar", [P(0, 5), P(0, 0)], 0.2, arch=ZEMIN),
    wall(5, "duvar", [P(5, 0), P(5, 5)], 0.15, arch={"level": "zemin", "phase": "new"}),
    {"kind": "line", **base(6, "cizim"), "a": P(0, 20), "b": P(5, 20)},
    wall(7, "kilitli", [P(30, 0), P(34, 0)], 0.2),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEW = max(IDS) + 1
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]
LAYERS = {"duvar": "Duvar", "mahal": "Mahal", "kapi": "Kapı", "cizim": "Çizim", "kilitli": "Kilitli katman", "gizli": "Gizli katman",
          "sinir": "Sınır"}
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Mahaller",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "arch": {"levels": LEVELS}},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("duvar", "Duvar"), layer("mahal", "Mahal"), layer("kapi", "Kapı"), layer("cizim", "Çizim"),
               layer("kilitli", "Kilitli katman", locked=True), layer("gizli", "Gizli katman", visible=False),
               layer("pafta", "Pafta", kind="group", children=[layer("sinir", "Sınır")])],
    "activeLayer": "duvar",
    "styles": {"items": [], "categories": []},
    "entities": ENTITIES,
}
METHOD_LABEL = {"net": "Net (iç yüzler)", "centerline": "Duvar ekseni", "gross": "Brüt (dış yüzler)"}
CREATE, EDIT, QUANT = "cad.spaces.create", "cad.spaces.edit", "cad.spaces.quantities"


def uid_of(i):
    return f"$uidOf:{i}"


UID_TEXT = "0123456789abcdef-"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"

# ── The drawing as the steps leave it ──────────────────────────────────────────────────────────────────────────


class Drawing:
    def __init__(self):
        self.walls = {i: copy.deepcopy(e) for i, e in BY_ID.items() if e["kind"] == "wall"}
        self.spaces = {}      # id → (layer, fields with "parts" as exact rings, entity extras)
        self.openings = {}    # id → (host wall id, width, head, sill, form, style, depth, at)
        self.slabs = {}       # id → (layer, fields with "parts" as exact rings, entity extras) (ADR 0722)
        self.next = NEW
        self.locked = set()

    def ids(self):
        kept = [i for i in IDS if BY_ID[i]["kind"] != "wall" or i in self.walls]
        return kept + sorted(list(self.spaces) + list(self.openings) + list(self.slabs) + [w for w in self.walls if w not in BY_ID])

    def follow(self):
        """The document's follower (ADR 0342 §3): each space found from the walls is found again."""
        for i, (ly, s, ex) in list(self.spaces.items()):
            if s.get("seed") is None:
                continue
            p = self.found(s["seed"], s.get("method", "net"), s.get("arch", {}).get("level"))
            s = copy.deepcopy(s)
            if p is None:
                s["broken"] = "lostEnclosure"
            else:
                s.update(parts=p, exact=False, broken=None)
            self.spaces[i] = (ly, s, ex)

    def found(self, seed, method, level=None):
        """The exact rings (outer, holes) round the seed by the method, or None."""
        # Explicit level identity isolates boundaries; absent keeps the legacy lookup.
        walls = [from_json(w) for _, w in sorted(self.walls.items())
                 if level is None or w.get("arch", {}).get("level") == level]
        segs = sp.faces(walls)
        f = sp.net_face(segs, (F(seed["x"]), F(seed["y"])))
        if f is None:
            return None
        outer, holes = f
        holes = [h if sp.area2(h) < 0 else h[::-1] for h in holes]
        return [(sp.moved(outer, segs, method), [sp.moved(h, segs, method) for h in holes])]


def exact_parts(parts_json):
    out = []
    for p in parts_json:
        ring = lambda r: [(F(q["x"]), F(q["y"])) for q in r["pts"]]  # noqa: E731
        out.append((ring(p["outer"]), [ring(h) for h in p.get("holes", [])]))
    return out


def area_of(parts):
    return sum(abs(sp.area2(o)) / 2 - sum(abs(sp.area2(h)) / 2 for h in hs) for o, hs in parts)


def perimeter_of(parts):
    return sum(sp.ring_len(o) + sum(sp.ring_len(h) for h in hs) for o, hs in parts)


def json_parts(parts):
    return [dict({"outer": {"pts": [P(*q) for q in o]}}, **({"holes": [{"pts": [P(*q) for q in h]} for h in hs]} if hs else {}))
            for o, hs in parts]

# ── Checks the commands share ──────────────────────────────────────────────────────────────────────────────────


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


def problem(s, prefix):
    """The space's problem as the contract says it, or None."""
    for i, (o, hs) in enumerate(s["parts"]):
        for at, r in [(f"parts[{i}].outer", o)] + [(f"parts[{i}].holes[{k}]", h) for k, h in enumerate(hs)]:
            if len(r) < 3:
                return refusal("invalid_space", f"Mahalin halkası en az üç köşedir; {len(r)} verildi.", f"{prefix}.{at}")
    h = s.get("height")
    if h is not None and not (math.isfinite(h) and 0 < h <= 1000):
        return refusal("invalid_space", f"Mahalin yüksekliği {num(h)}; sıfırdan büyük, en çok 1000 m olmalı.", f"{prefix}.height")
    for key in ("number", "name", "usage"):
        if s.get(key) is not None and s[key].strip() == "":
            return refusal("invalid_space", "Boş metin yazılmaz; alanı kaldırın ya da bir değer yazın.", f"{prefix}.{key}")
    return None


def level_check(s, path):
    level = s.get("arch", {}).get("level")
    if level is None or any(lv["id"] == level for lv in LEVELS):
        return None
    return refusal("level_not_found", f"Projede “{level}” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.", path)


def stored(s, exact):
    """A space's fields as the document keeps them; a boundary from the walls is `$any` (its order is the core's)."""
    out = {"parts": json_parts(s["parts"]) if exact else "$any"}
    if s.get("seed") is not None:
        out["seed"] = s["seed"]
    if s.get("method", "net") != "net":
        out["method"] = s["method"]
    for k in ("number", "name", "usage"):
        if s.get(k) is not None:
            out[k] = s[k]
    if s.get("arch"):
        out["arch"] = s["arch"]
    if s.get("height") is not None:
        out["height"] = s["height"]
    fin = {k: v for k, v in s.get("finishes", {}).items() if v is not None}
    if fin:
        out["finishes"] = fin
    if s.get("void"):
        # A void (docs/adr/0708): written only when true.
        out["void"] = True
    if s.get("broken"):
        out["broken"] = s["broken"]
    return out


def entity(id, layer_id, s, exact, extra=None):
    return {"kind": "space", "id": id, "layerId": layer_id, "attrs": dict(s.get("attrs", {})), **(extra or {}), **stored(s, exact)}

# ── cad.spaces.create ────────────────────────────────────────────────────────────────────────────────────────


def create(d, inp, conflict=False):
    if not inp["spaces"]:
        return refusal("no_objects", "Yazılacak mahal verilmedi. En az bir mahalin iç noktasını ya da sınırını verin.", "spaces"), None
    made = []
    for i, n in enumerate(inp["spaces"]):
        at = f"spaces[{i}]"
        if "seed" in n:
            r = point_not_finite(n["seed"], "Mahalin iç noktasının", f"{at}.seed")
            if r:
                return r, None
        if "height" in n and not math.isfinite(n["height"]):
            return not_finite("Yükseklik", f"{at}.height"), None
        s = {"seed": n.get("seed"), "method": n.get("method", "net"), "number": n.get("number"), "name": n.get("name"),
             "usage": n.get("usage"), "arch": n.get("arch", {}), "height": n.get("height"),
             "finishes": n.get("finishes", {}), "attrs": n.get("attrs", {}), "void": n.get("void", False)}
        if "seed" in n:
            r = level_check(s, f"{at}.arch.level")
            if r:
                return r, None
            parts = d.found(n["seed"], s["method"], s.get("arch", {}).get("level"))
            if parts is None:
                return refusal("space_not_enclosed", "Duvarlar noktanın çevresini kapatmıyor; mahal bulunamadı. Duvarları kapatın ya da mahalin sınırını çizin.", f"{at}.seed"), None
            s["parts"], s["exact"] = parts, False
        elif "parts" in n:
            s["parts"], s["exact"] = exact_parts(n["parts"]), True
            s["json"] = n["parts"]
        else:
            return refusal("invalid_space", "Mahalin yeri verilmedi: duvarlarla çevrili bir iç nokta (seed) ya da sınırını (parts) verin.", at), None
        r = problem(s, at) or level_check(s, f"{at}.arch.level")
        if r:
            return r, None
        made.append(s)
    if "lineWeight" in inp and not (math.isfinite(inp["lineWeight"]) and 0 <= inp["lineWeight"] <= 100):
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None
    r = revision(inp, conflict)
    if r:
        return r, None
    r, warnings = layer_check(inp["layerId"])
    if r:
        return r, None
    return {"status": "completed", "warnings": warnings}, made


def extra_of(inp):
    return {k: inp[k] for k in ("color", "lineWeight") if k in inp}


def ent_of(id, layer_id, s, extra=None):
    e = entity(id, layer_id, s, s.get("exact", False), extra)
    if s.get("exact") and "json" in s:
        e["parts"] = s["json"]
    return e


def create_step(d, op, inp, non_finite=None, conflict=False, command=None):
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
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"spaces": [ent_of(0, inp["layerId"], m, extra_of(inp)) for m in made],
                                                          "revision": "$current"}, "warnings": result["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ids = list(range(d.next, d.next + len(made)))
    d.next += len(made)
    for i, m in zip(ids, made):
        d.spaces[i] = (inp["layerId"], m, extra_of(inp))
    s["result"] = {"status": "completed", "output": {"uids": ["$any"] * len(made), "ids": ids, "revision": "$current"},
                   "warnings": result["warnings"]}
    s["expect"] = {"ids": d.ids(), "entities": {str(i): ent_of(i, inp["layerId"], m, extra_of(inp)) for i, m in zip(ids, made)},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s

# ── cad.spaces.quantities ────────────────────────────────────────────────────────────────────────────────────


def level_height(arch):
    lv = next((x for x in LEVELS if x["id"] == arch.get("level")), None) if arch else None
    return None if lv is None else lv.get("height")


# ── A room's baseboard, the passages and the slabs (ADR 0722 §5) ─────────────────────────────────────────────


def reads(level, other):
    """An object on `level` reads a passage or a void on `other`: the same level, or every one without a level."""
    return level is None or level == other


def passages(d):
    """The doors and the gaps with no sill on a wall, in the drawing's order: (id, number, width, A, B, half, level)
    with A→B the axis between its jambs' middles (the walls are straight, centred; `at` its middle along the wall)."""
    out = []
    for oid, (host, width, head, sill, form, style, depth, at) in sorted(d.openings.items()):
        if not (form == "door" or (form == "opening" and (sill is None or sill <= 0))) or host not in d.walls:
            continue
        w = d.walls[host]
        a, b = (F(w["pts"][0]["x"]), F(w["pts"][0]["y"])), (F(w["pts"][1]["x"]), F(w["pts"][1]["y"]))
        t = sp.sub(b, a)
        ln = sp.exact_len(t)
        u = (t[0] / ln, t[1] / ln)
        at, half_w = F(at), F(width) / 2
        start, end = sp.add(a, sp.mul(u, at - half_w)), sp.add(a, sp.mul(u, at + half_w))
        out.append((oid, None, width, start, end, F(w["width"]) / 2, w.get("arch", {}).get("level")))
    return out


def along(parts, p):
    """How much of the passage's axis the rings' edges run along inside its band, and the side the room lies on by its
    longest such edge (+1 left of the axis, −1 right); None when none does."""
    _, _, _, a, b, half, _ = p
    t = sp.sub(b, a)
    ln = sp.exact_len(t)
    u = (t[0] / ln, t[1] / ln)
    total, best = F(0), None
    for outer, holes in parts:
        for k, ring in enumerate([outer] + list(holes)):
            left = (sp.area2(ring) > 0) == (k == 0)
            n = len(ring)
            for i in range(n):
                s0, s1 = ring[i], ring[(i + 1) % n]
                e = sp.sub(s1, s0)
                if e == (0, 0) or sp.cross(u, e) != 0:
                    continue
                if abs(sp.cross(u, sp.sub(s0, a))) > half:
                    continue
                k0, k1 = sp.dot(u, sp.sub(s0, a)), sp.dot(u, sp.sub(s1, a))
                over = min(max(k0, k1), ln) - max(min(k0, k1), F(0))
                if over <= 0:
                    continue
                total += over
                side = 1 if (sp.dot(u, e) > 0) == left else -1
                if best is None or over > best[0]:
                    best = (over, side)
    return None if best is None else (total, best[1])


def baseboard(parts, level, all_passages):
    off = sum((along(parts, p)[0] for p in all_passages if reads(level, p[6]) and along(parts, p) is not None), F(0))
    return max(perimeter_of(parts) - off, F(0))


def transitions(rooms, all_passages):
    """rooms: (uid, level, parts, floor) of the rows' rooms; the passages with a room on each side whose floors are
    known and differ."""
    out = []
    for p in all_passages:
        sides = [None, None]
        for uid, level, parts, floor in rooms:
            if not reads(level, p[6]):
                continue
            got = along(parts, p)
            if got is None:
                continue
            k = 0 if got[1] > 0 else 1
            if sides[k] is None or got[0] > sides[k][0]:
                sides[k] = (got[0], uid, floor)
        if sides[0] and sides[1] and sides[0][2] and sides[1][2] and sides[0][2] != sides[1][2] and sides[0][1] != sides[1][1]:
            row = {"opening": uid_of(p[0]), "width": p[2], "spaces": [sides[0][1], sides[1][1]],
                   "floors": [sides[0][2], sides[1][2]]}
            if p[1] is not None:
                row["number"] = p[1]
            out.append(row)
    return out


def clip(ring, window):
    """The ring cut to a convex window (Sutherland–Hodgman), both counter-clockwise or the window either way."""
    if sp.area2(window) < 0:
        window = window[::-1]
    out = list(ring)
    n = len(window)
    for i in range(n):
        if not out:
            break
        a, b = window[i], window[(i + 1) % n]
        inside_ = lambda q: sp.cross(sp.sub(b, a), sp.sub(q, a)) >= 0  # noqa: E731
        res = []
        for j in range(len(out)):
            cur, nxt = out[j], out[(j + 1) % len(out)]
            ci, ni = inside_(cur), inside_(nxt)
            if ci:
                res.append(cur)
            if ci != ni:
                d1, d2 = sp.sub(nxt, cur), sp.sub(b, a)
                tt = sp.cross(sp.sub(a, cur), d2) / sp.cross(d1, d2)
                res.append(sp.add(cur, sp.mul(d1, tt)))
        out = res
    return out


def void_cut(parts, voids):
    """The area the voids (convex, apart: the cases' rooms) take out of the parts."""
    total = F(0)
    for v_outer, _ in voids:
        for outer, holes in parts:
            c = clip(outer, v_outer)
            total += abs(sp.area2(c)) / 2 if len(c) >= 3 else 0
            for h in holes:
                ch = clip(h, v_outer)
                total -= abs(sp.area2(ch)) / 2 if len(ch) >= 3 else 0
    return total


ON_FACE = F(1, 10**6)


def near_segment(m, a, b):
    """Whether m lies within ON_FACE of the segment (the walls' widths are binary fractions: 0.2 is not 1/5)."""
    t = sp.sub(b, a)
    tt = sp.dot(t, t)
    u = F(0) if tt == 0 else min(max(sp.dot(sp.sub(m, a), t) / tt, F(0)), F(1))
    q = sp.sub(m, sp.add(a, sp.mul(t, u)))
    return sp.dot(q, q) <= ON_FACE * ON_FACE


def free_length(d, parts, level):
    """The length of the rings' edges whose middle lies on no wall's face (within ON_FACE; of the walls the slab
    reads)."""
    walls = [from_json(w) for _, w in sorted(d.walls.items()) if level is None or w.get("arch", {}).get("level") == level]
    segs = [(a, b) for a, b, w in sp.faces(walls) if w > 0] if walls else []
    total = F(0)
    for outer, holes in parts:
        for ring in [outer] + list(holes):
            n = len(ring)
            for i in range(n):
                s0, s1 = ring[i], ring[(i + 1) % n]
                m = ((s0[0] + s1[0]) / 2, (s0[1] + s1[1]) / 2)
                if not any(near_segment(m, a, b) for a, b in segs):
                    total += sp.exact_len(sp.sub(s1, s0))
    return total


def slab_row(d, i, s):
    """A slab measured; one whose rings a reference measured on its grid (`measure`: area, perimeter; slab_cases.py) has
    no void or free side in the cases."""
    parts = s.get("parts")
    level = s.get("arch", {}).get("level")
    area = area_of(parts) if parts is not None else s["measure"][0]
    voids = [vp for _, (ly, v, _) in sorted(d.spaces.items()) if v.get("void") and reads(level, v.get("arch", {}).get("level"))
             for vp in v["parts"]]
    cut = void_cut(parts, voids) if parts is not None else F(0)
    net = area - cut
    thickness = F(s.get("thickness", 0.15))
    offset = s.get("offset", 0.0)
    lv = next((x for x in LEVELS if x["id"] == level), None)
    edge = s.get("edge")
    edge_len = free_length(d, parts, level) if edge and parts is not None else F(0)
    rule = "alan: halkaların alanı, delikler düşülür; boşluk: aynı kattaki boşluk mahallerinin örttüğü alan düşülür; hacim: net alan × kalınlık"
    if edge:
        rule += "; kenar: serbest kenarlar boyunca"
    if s.get("excluded"):
        rule += "; metraj dışı: toplamlara katılmaz"
    row = {"uid": uid_of(i), "role": s.get("role", "floor"), "level": level, "material": s.get("material"),
           "thickness": float(thickness), "offset": offset or None,
           "top": None if lv is None else lv["elevation"] + offset,
           "area": float(area), "voidArea": float(cut) or None, "netArea": float(net),
           "perimeter": float(perimeter_of(parts) if parts is not None else s["measure"][1]), "volume": float(net * thickness),
           "edge": edge["kind"] if edge else None, "edgeLength": float(edge_len) or None,
           "excluded": True if s.get("excluded") else None, "rule": rule}
    return {k: v for k, v in row.items() if v is not None}


def quantities(d, inp):
    for k, lv in enumerate(inp.get("levels", []) or []):
        if not any(x["id"] == lv for x in LEVELS):
            return refusal("level_not_found", f"Projede “{lv}” kimlikli kat yok. Var olan bir katın kimliğini verin.", f"levels[{k}]")
    for k, ly in enumerate(inp.get("layers", []) or []):
        if ly not in LAYERS and ly != "pafta":
            return refusal("layer_not_found", f"“{ly}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", f"layers[{k}]")

    def takes(layer_id, arch):
        return ((inp.get("layers") is None or layer_id in inp["layers"])
                and (inp.get("levels") is None or arch.get("level") in inp["levels"])
                and (inp.get("phases") is None or arch.get("phase") in inp["phases"]))
    on_wall, openings = {}, []
    recess = ("niche", "flue")
    recessed = set()
    for oid, (host, width, head, sill, form, style, depth, _at) in sorted(d.openings.items()):
        h = None if head is None else head - (sill or 0.0)
        a = None if h is None else width * h
        wall_ok = host in d.walls
        # A niche or flue does not go through: its wall's net face keeps it (ADR 0719 §4).
        if wall_ok and a is not None and form not in recess:
            on_wall.setdefault(host, []).append((oid, a))
        if wall_ok and form in recess:
            recessed.add(host)
        arch = d.walls[host].get("arch", {}) if wall_ok else {}
        if not takes("kapi", arch) or inp.get("usages") is not None:
            continue
        row = {"uid": uid_of(oid), "form": form, "style": style, "wall": uid_of(host) if wall_ok else None,
               "width": width, "height": h, "area": a, "depth": depth if form in recess else None}
        openings.append({k: v for k, v in row.items() if v is not None})
    spaces_rows, walls_rows, slab_rows, rooms = [], [], [], []
    all_passages = passages(d)
    for i in d.ids():
        if i in d.spaces:
            layer_id, s, _ = d.spaces[i]
            arch = s.get("arch", {})
            if not takes(layer_id, arch):
                continue
            if inp.get("usages") is not None and s.get("usage") not in inp["usages"]:
                continue
            area = float(area_of(s["parts"]))
            height = s.get("height") if s.get("height") is not None else level_height(arch)
            row = {"uid": uid_of(i), "number": s.get("number"), "name": s.get("name"), "usage": s.get("usage"),
                   "level": arch.get("level"), "method": s.get("method", "net"), "void": True if s.get("void") else None,
                   "area": area,
                   "perimeter": float(perimeter_of(s["parts"])), "height": height,
                   "volume": None if height is None else area * height}
            if s.get("seed") is not None:
                by = {}
                for m in ("net", "centerline", "gross"):
                    p = d.found(s["seed"], m, s.get("arch", {}).get("level"))
                    if p is not None:
                        by[m] = float(area_of(p))
                row["byMethod"] = by
            row["rule"] = f"alan: halkaların alanı, delikler düşülür ({METHOD_LABEL[s.get('method', 'net')]}); hacim: alan × yükseklik"
            if s.get("void"):
                row["rule"] += "; boşluk: alanı mahal toplamına katılmaz, boşluk toplamındadır"
            else:
                row["rule"] += "; süpürgelik: çevreden kenarları boyunca duran geçişlerin (kapı, döşemeye inen boşluk) genişliği düşülür"
                row["baseboard"] = float(baseboard(s["parts"], arch.get("level"), all_passages))
                rooms.append((uid_of(i), arch.get("level"), s["parts"], s.get("finishes", {}).get("floor")))
            spaces_rows.append({k: v for k, v in row.items() if v is not None})
        elif i in d.slabs and inp.get("usages") is None:
            layer_id, s, _ = d.slabs[i]
            if takes(layer_id, s.get("arch", {})):
                slab_rows.append(slab_row(d, i, s))
        elif i in d.walls and inp.get("usages") is None:
            w = d.walls[i]
            arch = w.get("arch", {})
            if not takes(w["layerId"], arch):
                continue
            length = float(path_len(from_json(w).pts, w.get("closed", False)))
            height = w.get("height", level_height(arch))
            ops = on_wall.get(i, [])
            oa = sum(a for _, a in ops) if ops else 0.0
            gross = None if height is None else length * height
            net = None if gross is None else gross - oa
            row = {"uid": uid_of(i), "level": arch.get("level"), "length": length, "width": w["width"], "height": height,
                   "grossArea": gross, "openingsArea": oa, "netArea": net,
                   "volume": None if net is None else net * w["width"], "openings": [uid_of(o) for o, _ in ops] or None,
                   "rule": ("yüzey: taban çizgisinin uzunluğu × yükseklik, duvarı geçen açıklıkların alanı düşülür (niş ve baca düşülmez); hacim: net yüzey × kalınlık"
                            if i in recessed else "yüzey: taban çizgisinin uzunluğu × yükseklik, açıklıkların alanı düşülür; hacim: net yüzey × kalınlık")}
            walls_rows.append({k: v for k, v in row.items() if v is not None})
    totals = {"spaceArea": sum((r["area"] for r in spaces_rows if not r.get("void")), 0.0),
              "wallLength": sum((r["length"] for r in walls_rows), 0.0),
              "wallNetArea": sum((r["netArea"] for r in walls_rows if "netArea" in r), 0.0),
              "openings": len(openings),
              "openingArea": sum((r["area"] for r in openings if "area" in r and r["form"] not in recess), 0.0)}
    void_area = sum((r["area"] for r in spaces_rows if r.get("void")), 0.0)
    if void_area:
        # The voids' area apart (docs/adr/0708); none: the key is not written.
        totals["voidArea"] = void_area
    # ADR 0722 §5: baseboards, slabs (those left out not summed), their edges; a zero is not written.
    summed = [r for r in slab_rows if not r.get("excluded")]
    for key, value in (("baseboardLength", sum((r.get("baseboard", 0.0) for r in spaces_rows), 0.0)),
                       ("slabArea", sum((r["netArea"] for r in summed), 0.0)),
                       ("slabVolume", sum((r["volume"] for r in summed), 0.0)),
                       ("railingLength", sum((r.get("edgeLength", 0.0) for r in summed if r.get("edge") == "railing"), 0.0)),
                       ("parapetLength", sum((r.get("edgeLength", 0.0) for r in summed if r.get("edge") == "parapet"), 0.0))):
        if value:
            totals[key] = value
    out = {"spaces": spaces_rows, "walls": walls_rows, "openings": openings, "totals": totals}
    if slab_rows:
        out["slabs"] = slab_rows
    moves = transitions(rooms, all_passages)
    if moves:
        out["transitions"] = moves
    return {"status": "completed", "output": out, "warnings": []}


def quant_step(d, inp=None, op="execute", command=None):
    inp = inp or {}
    s = {"op": op}
    if command:
        s["command"] = command
    s["input"] = inp
    s["result"] = quantities(d, inp)
    s["within"] = 1e-9
    s["expect"] = {"revision": "same"}
    return s

# ── cad.spaces.edit ──────────────────────────────────────────────────────────────────────────────────────────


def named(d, uids):
    if not uids:
        return refusal("no_entities", "Düzenlenecek mahal verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids")
    out, seen = [], set()
    for at, u in enumerate(uids):
        if not u.startswith("$uidOf:"):
            if len(u) != 36 or any(ch not in UID_TEXT for ch in u):
                return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", f"uids[{at}]")
            return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", f"uids[{at}]")
        i = int(u[len("$uidOf:"):])
        if i not in seen:
            seen.add(i)
            out.append((at, i))
    return out


def label_of(parts):
    (o, _), = parts
    xs, ys = [q[0] for q in o], [q[1] for q in o]
    return P((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)


def rect(x0, y0, x1, y1):
    return [(F(x0), F(y0)), (F(x1), F(y0)), (F(x1), F(y1)), (F(x0), F(y1))]


def bbox_union(parts_list):
    """The union of rectangles that make one rectangle (the cases' merges), or None."""
    xs = [q[0] for parts in parts_list for o, _ in parts for q in o]
    ys = [q[1] for parts in parts_list for o, _ in parts for q in o]
    box = rect(min(xs), min(ys), max(xs), max(ys))
    whole = abs(sp.area2(box)) / 2
    return [(box, [])] if whole == sum(area_of(p) for p in parts_list) else None


def edit(d, inp, conflict=False):
    found = named(d, inp["uids"])
    if isinstance(found, dict):
        return found, None
    for at, i in found:
        if i not in d.spaces:
            return refusal("not_a_space", "Nesne mahal değil; Mahali düzenle yalnız mahalleri değiştirir.", f"uids[{at}]"), None
    op = inp["operation"]
    kind = op["kind"]
    updates, added, removed = [], [], []
    if kind == "set":
        ch = {k: v for k, v in op.items() if k != "kind"}
        if not ch:
            return refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi: numara, ad, kullanım, yöntem, kat, evre, yükseklik ya da kaplamalardan birini verin.", "operation"), None
        if ch.get("height") is not None and not math.isfinite(ch["height"]):
            return not_finite("Yükseklik", "operation.height"), None
        for at, i in found:
            layer_id, s, extra = d.spaces[i]
            s = copy.deepcopy(s)
            for k in ("number", "name", "usage", "height"):
                if k in ch:
                    s[k] = ch[k]
            if "method" in ch:
                s["method"] = ch["method"]
            arch = dict(s.get("arch", {}))
            for k, key in (("level", "level"), ("phase", "phase")):
                if k in ch:
                    if ch[k] is None:
                        arch.pop(key, None)
                    else:
                        arch[key] = ch[k]
            s["arch"] = arch
            fin = dict(s.get("finishes", {}))
            for k in ("floor", "wall", "ceiling"):
                if k in ch:
                    fin[k] = ch[k]
            s["finishes"] = fin
            if "void" in ch:
                s["void"] = ch["void"]
            if ("method" in ch or "level" in ch) and s.get("seed") is not None:
                p = d.found(s["seed"], s["method"], s.get("arch", {}).get("level"))
                if p is None:
                    s["broken"] = "lostEnclosure"
                else:
                    s["parts"], s["exact"], s["broken"] = p, False, None
                    s.pop("json", None)
            r = problem(s, f"uids[{at}]") or level_check(s, "operation.level")
            if r:
                return r, None
            updates.append((i, s))
    elif kind == "merge":
        if len(found) < 2:
            return refusal("too_few_objects", "Birleştirmek için en az iki mahal verin.", "uids"), None
        one = bbox_union([d.spaces[i][1]["parts"] for _, i in found])
        if one is None:
            return refusal("spaces_not_touching", "Mahaller birleşince 2 ayrı parça oluyor; birleştirilecek mahaller birbirine değmeli.", "uids"), None
        first = found[0][1]
        s = copy.deepcopy(d.spaces[first][1])
        s.update(parts=one, exact=False, seed=None, broken=None)
        s.pop("json", None)
        updates.append((first, s))
        removed = [i for _, i in found[1:]]
    elif kind == "split":
        line = op["line"]
        if len(line) < 2:
            return refusal("split_missed", "Bölme çizgisi en az iki noktadır.", "operation.line"), None
        for k, p in enumerate(line):
            r = point_not_finite(p, f"{k + 1}. noktanın", f"operation.line[{k}]")
            if r:
                return r, None
        # The cases' lines are vertical and cross rectangles whole.
        x = F(line[0]["x"])
        for at, i in found:
            s = d.spaces[i][1]
            (o, _), = s["parts"]
            xs = [q[0] for q in o]
            ys = [q[1] for q in o]
            if not (min(xs) < x < max(xs)) or line[0]["x"] != line[-1]["x"]:
                return refusal("split_missed", "Çizgi mahali bölmüyor: mahalin içinden geçip sınırını iki yerde kesmeli.", f"uids[{at}]"), None
            a, b = rect(min(xs), min(ys), x, max(ys)), rect(x, min(ys), max(xs), max(ys))
            pieces = sorted([a, b], key=lambda r: -abs(sp.area2(r)))
            kept = copy.deepcopy(s)
            kept.update(parts=[(pieces[0], [])], exact=False, seed=None, broken=None)
            kept.pop("json", None)
            updates.append((i, kept))
            new = copy.deepcopy(kept)
            new["parts"] = [(pieces[1], [])]
            added.append((d.spaces[i][0], new, d.spaces[i][2]))
    elif kind == "rebound":
        if op.get("seed") is not None:
            r = point_not_finite(op["seed"], "Mahalin iç noktasının", "operation.seed")
            if r:
                return r, None
        for at, i in found:
            s = copy.deepcopy(d.spaces[i][1])
            # Else its own seed, else its label's point (the pole: the cases' boxes' centres).
            seed = op.get("seed") or s.get("seed") or label_of(s["parts"])
            p = d.found(seed, s.get("method", "net"), s.get("arch", {}).get("level"))
            if p is None:
                return refusal("space_not_enclosed", "Duvarlar noktanın çevresini kapatmıyor; mahal bulunamadı. Duvarları kapatın ya da mahalin sınırını çizin.", f"uids[{at}]"), None
            s.update(seed=seed, parts=p, exact=False, broken=None)
            s.pop("json", None)
            updates.append((i, s))
    r = revision(inp, conflict)
    if r:
        return r, None
    for at, i in found:
        if d.spaces[i][0] in d.locked:
            return refusal("layer_locked", f"“{LAYERS[d.spaces[i][0]]}” katmanı kilitli; üzerindeki mahal düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{at}]"), None
    return {"status": "completed"}, (updates, added, removed)


def edit_step(d, op, inp, non_finite=None, conflict=False):
    result, work = edit(d, with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if work is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    updates, added, removed = work
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ents = [ent_of(i, d.spaces[i][0], u, d.spaces[i][2]) for i, u in updates]
    out = {"spaces": ents}
    if op == "plan":
        if added:
            out["added"] = [ent_of(0, ly, a, ex) for ly, a, ex in added]
        out["kept"] = ["$any"] * len(updates)
        if removed:
            out["removed"] = ["$any"] * len(removed)
        out["revision"] = "$current"
        s["result"] = {"status": "completed", "output": out, "warnings": []}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    new_ids = []
    for ly, a, ex in added:
        d.spaces[d.next] = (ly, a, ex)
        new_ids.append(d.next)
        d.next += 1
    for i, u in updates:
        d.spaces[i] = (d.spaces[i][0], u, d.spaces[i][2])
    for i in removed:
        del d.spaces[i]
    if added:
        out["added"] = [ent_of(j, d.spaces[j][0], d.spaces[j][1], d.spaces[j][2]) for j in new_ids]
    out["kept"] = ["$any"] * len(updates)
    if removed:
        out["removed"] = ["$any"] * len(removed)
    out["revision"] = "$current"
    s["result"] = {"status": "completed", "output": out, "warnings": []}
    s["expect"] = {"ids": d.ids(), "entities": {str(i): ent_of(i, d.spaces[i][0], d.spaces[i][1], d.spaces[i][2])
                                                 for i in [i for i, _ in updates] + new_ids},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s

# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps):
        assert len(name) > 10
        self.cases.append({"name": name, "steps": steps})


LEFT, RIGHT = P(2, 2), P(7, 2)


def one(layer="mahal", **kw):
    return {"layerId": layer, "spaces": [kw]}


def box_json(x0, y0, x1, y1):
    return {"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}


def create_cases():
    c = Cases()
    d = Drawing()
    first = create_step(d, "execute", one(seed=LEFT, number="Z-01", name="Salon"))
    c.add("duvarlarla çevrili sol oda: net sınır, tek adım “Mahal”, geri alınır ve yinelenir; metraj alanını söyler", [
        first, quant_step(d, command=QUANT),
        {"op": "undo", "returns": "Mahal", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}},
        {"op": "redo", "returns": "Mahal", "expect": {"ids": IDS + [NEW], "canUndo": True, "canRedo": False, "revision": "changed"}},
    ])
    for name, inp in [
        ("sağ oda duvar ekseninden", one(seed=RIGHT, method="centerline")),
        ("sağ oda dış yüzlerden (brüt)", one(seed=RIGHT, method="gross")),
        ("sol oda kat, evre, yükseklik, kaplamalar ve kullanımla", one(seed=LEFT, usage="Oturma", arch={"level": "zemin", "phase": "new"},
                                                                     height=2.8, finishes={"floor": "Parke", "ceiling": "Alçı"})),
        ("başka katın duvarları açık kat seçiminin mahali kapatmaz", one(seed=LEFT, arch={"level":"kat-1"})),
        ("çizilmiş sınır olduğu gibi saklanır", one(parts=[box_json(20, 0, 24, 3)], name="Depo")),
        ("çizilmiş sınır deliği ve ikinci parçasıyla", one(parts=[dict(box_json(20, 0, 26, 4), holes=[{"pts": [P(22, 1), P(22, 2), P(23, 2), P(23, 1)]}]),
                                                                box_json(30, 10, 32, 12)], method="gross")),
        ("öznitelik mahalin kendisinin", one(seed=LEFT, attrs={"kod": "M-1"})),
        ("renk ve kalınlık mahalin kendisinin", {"layerId": "mahal", "color": "#3366AA", "lineWeight": 0.13, "spaces": [{"seed": LEFT}]}),
        ("gizli katmana uyarıyla yazılır", one(layer="gizli", seed=LEFT)),
        ("iki mahal tek çağrıda, tek adım", {"layerId": "mahal", "spaces": [{"seed": LEFT, "number": "1"}, {"seed": RIGHT, "number": "2"}]}),
        # A void (docs/adr/0708): its area summed apart in the quantities.
        ("çizilmiş şaft boşluk olarak; metraj alanını boşluk toplamına yazar", one(parts=[box_json(20, 0, 20.8, 0.6)], name="Şaft", usage="Tesisat şaftı", void=True)),
        ("duvarlarla çevrili sağ oda boşluk (asansör); oda ile boşluk ayrı toplanır", {"layerId": "mahal", "spaces": [{"seed": LEFT, "name": "Salon"}, {"seed": RIGHT, "name": "Asansör", "void": True}]}),
        ("boşluk false yazılmaz: oda olarak kalır", one(seed=LEFT, void=False)),
    ]:
        dd = Drawing()
        c.add(name, [create_step(dd, "execute", inp), quant_step(dd, command=QUANT)])
    dd = Drawing()
    c.add("plan yazmadan mahali gösterir", [create_step(dd, "plan", one(parts=[box_json(20, 0, 24, 3)]))])
    dd = Drawing()
    c.add("doğrulama yazmaz", [create_step(dd, "validate", one(seed=LEFT))])
    for name, inp, kw in [
        ("mahal verilmezse ret", {"layerId": "mahal", "spaces": []}, {}),
        ("sonsuz iç nokta reddedilir", one(seed=LEFT), {"non_finite": {"spaces[0].seed.x": "NaN"}}),
        ("sonsuz yükseklik reddedilir", one(seed=LEFT, height=2.0), {"non_finite": {"spaces[0].height": "Infinity"}}),
        ("duvarların kapatmadığı nokta reddedilir", one(seed=P(20, 20)), {}),
        ("yeri verilmeyen mahal reddedilir", one(name="Boş"), {}),
        ("iki köşeli halka reddedilir", one(parts=[{"outer": {"pts": [P(0, 0), P(1, 0)]}}]), {}),
        ("sıfır yükseklik reddedilir", one(seed=LEFT, height=0.0), {}),
        ("boş ad reddedilir", one(seed=LEFT, name=" "), {}),
        ("projede olmayan kat reddedilir", one(seed=LEFT, arch={"level": "bodrum"}), {}),
        ("kalınlık 100 mm'yi aşarsa ret", {"layerId": "mahal", "lineWeight": 120.0, "spaces": [{"seed": LEFT}]}, {}),
        ("kilitli katmana yazılmaz", one(layer="kilitli", seed=LEFT), {}),
        ("katman grubuna yazılmaz", one(layer="pafta", seed=LEFT), {}),
        ("olmayan katman reddedilir", one(layer="yok", seed=LEFT), {}),
        ("geçersiz beklenen sürüm reddedilir", dict(one(seed=LEFT), expectedRevision="07"), {}),
    ]:
        c.add(name, [create_step(Drawing(), "execute", inp, **kw)])
    dd = Drawing()
    made = create_step(dd, "execute", one(seed=RIGHT))
    stale = create_step(dd, "execute", dict(one(seed=LEFT), expectedRevision="$prepared"), conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c.cases


def edit_cases():
    c = Cases()

    def fresh(*spaces):
        d = Drawing()
        return d, create_step(d, "execute", {"layerId": "mahal", "spaces": list(spaces)}, command=CREATE)

    first = NEW
    for name, op in [
        ("Değiştir: numara, ad ve kullanım", {"kind": "set", "number": "Z-02", "name": "Yatak odası", "usage": "Yatak"}),
        ("Değiştir: kat, evre ve yükseklik", {"kind": "set", "level": "zemin", "phase": "existing", "height": 2.7}),
        ("Değiştir: kaplamalar", {"kind": "set", "floor": "Seramik", "wall": "Fayans", "ceiling": "Asma tavan"}),
        ("Değiştir: yöntem duvar ekseni, sınır duvarlardan yeniden", {"kind": "set", "method": "centerline"}),
        ("Değiştir: yöntem brüt", {"kind": "set", "method": "gross"}),
    ]:
        d, made = fresh({"seed": LEFT, "name": "Oda"})
        c.add(name, [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": op}), quant_step(d, command=QUANT)])
    # A room made a void and a room again (docs/adr/0708).
    d, made = fresh({"seed": LEFT, "name": "Oda"}, {"seed": RIGHT, "name": "Şaft"})
    c.add("Değiştir: oda boşluk olur, metraj boşluğu ayrı toplar; sonra yine oda",
          [made, edit_step(d, "execute", {"uids": [uid_of(first + 1)], "operation": {"kind": "set", "void": True}}), quant_step(d, command=QUANT),
           edit_step(d, "execute", {"uids": [uid_of(first + 1)], "operation": {"kind": "set", "void": False}}), quant_step(d, command=QUANT)])
    d, made = fresh({"parts": [box_json(20, 0, 23, 4)], "number": "A", "void": True}, {"parts": [box_json(23, 0, 26, 4)], "number": "B"})
    c.add("Birleştir: ilk mahal boşluksa birleşen de boşluktur",
          [made, edit_step(d, "execute", {"uids": [uid_of(first), uid_of(first + 1)], "operation": {"kind": "merge"}}), quant_step(d, command=QUANT)])
    d, made = fresh({"seed": LEFT, "height": 2.5, "name": "Oda"})
    c.add("Değiştir: yüksekliği kaldırmak katınkini verir", [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "set", "height": None}})])
    d, made = fresh({"parts": [box_json(20, 0, 23, 4)], "number": "A"}, {"parts": [box_json(23, 0, 26, 4)], "number": "B"})
    c.add("Birleştir: değen iki mahal, ilki kimliğini korur, öbürü silinir; sınır çizilmiş olur",
          [made, edit_step(d, "execute", {"uids": [uid_of(first), uid_of(first + 1)], "operation": {"kind": "merge"}}), quant_step(d, command=QUANT)])
    d, made = fresh({"parts": [box_json(20, 0, 22, 4)]}, {"parts": [box_json(22, 0, 24, 4)]}, {"parts": [box_json(24, 0, 26, 4)]})
    c.add("Birleştir: üç mahal sırayla", [made, edit_step(d, "execute", {"uids": [uid_of(first + 2), uid_of(first), uid_of(first + 1)], "operation": {"kind": "merge"}}),
                                         quant_step(d, command=QUANT)])
    d, made = fresh({"parts": [box_json(20, 0, 26, 4)], "number": "C", "name": "Ofis"})
    c.add("Böl: büyük parça kimliğini korur, küçüğü aynı alanlarla yeni mahal",
          [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "split", "line": [P(22, -1), P(22, 5)]}}), quant_step(d, command=QUANT)])
    d, made = fresh({"parts": [box_json(1, 1, 3, 3)], "name": "Taslak"})
    c.add("Duvarlardan bul: çizilmiş mahal verilen noktadan duvarlardan bulunan olur",
          [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "rebound", "seed": LEFT}}), quant_step(d, command=QUANT)])
    d, made = fresh({"parts": [box_json(6, 1, 7, 3)], "name": "Taslak"})
    c.add("Duvarlardan bul: tohumsuz çizilmiş mahal etiketinin noktasından bulunur",
          [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "rebound"}}), quant_step(d, command=QUANT)])
    # The document keeps them with their walls (ADR 0342 §3).
    d, made = fresh({"seed": LEFT, "name": "Sol"}, {"seed": RIGHT, "name": "Sağ"})

    def moved(d):
        d.walls[5] = dict(d.walls[5], pts=[P(6, 0), P(6, 5)])
        d.follow()
    step = {"op": "execute", "command": "cad.entities.transform",
            "input": {"uids": [uid_of(5)], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}
    moved(d)
    c.add("ara duvar taşınınca iki mahalin sınırı ve alanı aynı adımda değişir", [made, step, quant_step(d, command=QUANT)])
    d, made = fresh({"seed": LEFT, "name": "Sol"}, {"seed": RIGHT, "name": "Sağ", "method": "centerline"})
    thick = {"op": "execute", "command": "cad.walls.edit",
             "input": {"uids": [uid_of(5)], "operation": {"kind": "set", "width": 0.25}},
             "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}
    d.walls[5] = dict(d.walls[5], width=0.25)
    d.follow()
    c.add("ara duvar kalınlaşınca net mahal daralır, eksenden ölçülen değişmez; aynı adımda (CAD-ARCH-08)",
          [made, thick, quant_step(d, command=QUANT)])
    d, made = fresh({"seed": LEFT, "name": "Sol"})
    gone = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [uid_of(1)]},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"},
            "expect": {"fields": {str(first): {"broken": "lostEnclosure"}}, "revision": "changed"}}
    c.add("duvar silinince mahal kapanmaz: sınırı son hâliyle kalır, nedeni lostEnclosure", [made, gone])
    d, made = fresh({"seed": LEFT})
    c.add("plan yazmadan mahali olacağı gibi gösterir", [made, edit_step(d, "plan", {"uids": [uid_of(first)], "operation": {"kind": "set", "name": "Oda"}})])
    for op in ("plan", "execute"):
        d, made = fresh({"seed": LEFT})
        c.add(f"kat değişince başka katın sınırı kullanılmaz: {op}", [made,
            edit_step(d,op,{"uids":[uid_of(first)],"operation":{"kind":"set","level":"kat-1"}})])
    d, made = fresh({"seed": LEFT})
    c.add("doğrulama yazmaz", [made, edit_step(d, "validate", {"uids": [uid_of(first)], "operation": {"kind": "set", "name": "Oda"}})])
    d, made = fresh({"seed": LEFT})
    c.add("tek adım “Mahali düzenle”, geri alınır", [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "set", "name": "Oda"}}),
                                                    {"op": "undo", "returns": "Mahali düzenle", "expect": {"canRedo": True, "revision": "changed"}}])
    for name, inp, kw, spaces in [
        ("mahal verilmezse ret", {"uids": [], "operation": {"kind": "merge"}}, {}, [{"seed": LEFT}]),
        ("geçersiz kimlik reddedilir", {"uids": ["D-1"], "operation": {"kind": "merge"}}, {}, [{"seed": LEFT}]),
        ("çizimde olmayan kimlik reddedilir", {"uids": [MISSING], "operation": {"kind": "merge"}}, {}, [{"seed": LEFT}]),
        ("mahal olmayan nesne reddedilir", {"uids": [uid_of(1)], "operation": {"kind": "set", "name": "x"}}, {}, [{"seed": LEFT}]),
        ("boş Değiştir reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set"}}, {}, [{"seed": LEFT}]),
        ("sonsuz yükseklik reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set", "height": 2.0}}, {"non_finite": {"operation.height": "NaN"}}, [{"seed": LEFT}]),
        ("sıfır yükseklik reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set", "height": 0.0}}, {}, [{"seed": LEFT}]),
        ("projede olmayan kat reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set", "level": "bodrum"}}, {}, [{"seed": LEFT}]),
        ("tek mahal birleştirilmez", {"uids": [uid_of(first)], "operation": {"kind": "merge"}}, {}, [{"seed": LEFT}]),
        ("değmeyen mahaller birleştirilmez", {"uids": [uid_of(first), uid_of(first + 1)], "operation": {"kind": "merge"}}, {},
         [{"parts": [box_json(20, 0, 22, 2)]}, {"parts": [box_json(30, 0, 32, 2)]}]),
        ("tek noktalı bölme çizgisi reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "split", "line": [P(21, 1)]}}, {}, [{"parts": [box_json(20, 0, 22, 2)]}]),
        ("mahali kesmeyen çizgi reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "split", "line": [P(25, -1), P(25, 5)]}}, {}, [{"parts": [box_json(20, 0, 22, 2)]}]),
        ("sonsuz bölme noktası reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "split", "line": [P(21, -1), P(21, 5)]}},
         {"non_finite": {"operation.line[1].y": "NaN"}}, [{"parts": [box_json(20, 0, 22, 2)]}]),
        ("duvarların kapatmadığı noktadan bulunmaz", {"uids": [uid_of(first)], "operation": {"kind": "rebound", "seed": P(40, 40)}}, {}, [{"seed": LEFT}]),
        ("geçersiz beklenen sürüm reddedilir", {"uids": [uid_of(first)], "operation": {"kind": "set", "name": "x"}, "expectedRevision": "07"}, {}, [{"seed": LEFT}]),
    ]:
        d, made = fresh(*spaces)
        c.add(name, [made, edit_step(d, "execute", inp, **kw)])
    d, made = fresh({"seed": LEFT})
    d.locked.add("mahal")
    c.add("kilitli katmandaki mahal düzenlenmez", [
        made, {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "mahal", "value": True},
               "result": {"status": "completed", "output": "$any", "warnings": "$any"}},
        edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "set", "name": "x"}})])
    d, made = fresh({"seed": LEFT})
    stale = edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "set", "name": "x"}, "expectedRevision": "$prepared"}, conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c.cases


def quantities_cases():
    c = Cases()

    def drawn():
        d = Drawing()
        made = create_step(d, "execute", {"layerId": "mahal", "spaces": [
            {"seed": LEFT, "number": "1", "usage": "Oturma", "arch": {"level": "zemin", "phase": "new"}},
            {"seed": RIGHT, "number": "2", "usage": "Islak", "height": 2.4},
            {"parts": [box_json(20, 0, 24, 3)], "usage": "Depo", "arch": {"level": "kat-1"}}]}, command=CREATE)
        return d, made
    for name, inp in [
        ("bütün çizim: mahaller, duvarlar, toplamlar", {}),
        ("zemin katı süzgeci", {"levels": ["zemin"]}),
        ("1. kat süzgeci", {"levels": ["kat-1"]}),
        ("evre süzgeci: yeni", {"phases": ["new"]}),
        ("kullanım süzgeci yalnız mahalleri verir", {"usages": ["Islak", "Depo"]}),
        ("katman süzgeci: yalnız duvar katmanı", {"layers": ["duvar"]}),
        ("katman süzgeci: kilitli katmanın duvarı yüksekliksiz", {"layers": ["kilitli"]}),
        ("katman süzgeci: yalnız mahaller", {"layers": ["mahal"]}),
        ("iki kat birden", {"levels": ["zemin", "kat-1"]}),
        ("evre süzgeci: var olan (hiçbiri)", {"phases": ["existing"]}),
        ("kullanım süzgeci: tek kullanım", {"usages": ["Oturma"]}),
        ("kat ve kullanım birlikte", {"levels": ["zemin"], "usages": ["Islak", "Oturma"]}),
        ("boş katman: hiçbir satır, toplamlar sıfır", {"layers": ["sinir"]}),
    ]:
        d, made = drawn()
        c.add(name, [made, quant_step(d, inp)])
    def opening(d, host, at, kind=None):
        o = {"host": uid_of(host), "at": at}
        if kind == "window":
            o["type"] = "Pencere 120"
            d.openings[d.next] = (host, 1.2, 2.2, 0.9, "window", "kentos-pencere-120", None, at)
        elif kind == "niche":
            o["type"] = "Niş 60"
            d.openings[d.next] = (host, 0.6, 1.5, 0.9, "niche", "kentos-nis-60", 0.1, at)
        elif kind == "flue":
            o["form"], o["depth"] = "flue", 0.1
            d.openings[d.next] = (host, 0.2, None, None, "flue", "kentos-baca-20", 0.1, at)
        else:
            d.openings[d.next] = (host, 0.9, 2.1, None, "door", "kentos-kapi-90", None, at)
        d.next += 1
        return {"op": "execute", "command": "cad.openings.create", "input": {"layerId": "kapi", "openings": [o]},
                "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}
    d, made = drawn()
    c.add("kapının alanı duvarının net yüzeyinden düşülür", [made, opening(d, 1, 2.0), quant_step(d)])
    d, made = drawn()
    c.add("pencerenin yüksekliği lento eksi denizlik", [made, opening(d, 3, 6.0, "window"), quant_step(d)])
    d, made = drawn()
    c.add("bir duvarda kapı ve pencere: ikisi de düşülür, toplamda iki açıklık",
          [made, opening(d, 1, 1.5), opening(d, 1, 6.5, "window"), quant_step(d)])
    d, made = drawn()
    c.add("ara duvarın kapısı: evre süzgeciyle açıklık da süzülür",
          [made, opening(d, 5, 2.5), opening(d, 1, 6.5), quant_step(d, {"phases": ["new"]})])
    d, made = drawn()
    c.add("kullanım süzgecinde açıklık yok", [made, opening(d, 1, 2.0), quant_step(d, {"usages": ["Oturma"]})])
    d, made = drawn()
    c.add("niş duvarın net yüzeyinden düşülmez; satırında derinliği, toplam açıklık alanında yok",
          [made, opening(d, 1, 2.0), opening(d, 1, 6.0, "niche"), quant_step(d)])
    d, made = drawn()
    c.add("baca: yüksekliksiz, alansız; derinliği satırında", [made, opening(d, 3, 6.0, "flue"), quant_step(d)])
    d, made = drawn()
    c.add("plan aynı cevabı verir, çizim değişmez", [made, quant_step(d, op="plan")])
    d = Drawing()
    c.add("mahalsiz çizim: yalnız duvarlar", [quant_step(d)])
    d, made = drawn()
    c.add("brüt yöntemli mahal: alanı dış yüzlerden, üç yöntemin alanı satırda",
          [made, dict(edit_step(d, "execute", {"uids": [uid_of(NEW)], "operation": {"kind": "set", "method": "gross"}}), command=EDIT),
           quant_step(d)])
    for name, inp in [
        ("projede olmayan kat reddedilir", {"levels": ["bodrum"]}),
        ("çizimde olmayan katman reddedilir", {"layers": ["yok"]}),
    ]:
        d, made = drawn()
        c.add(name, [made, quant_step(d, inp)])
    return c.cases


def document(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note,
            "setup": SETUP, "cases": cases}


def files():
    note = ("ADR 0342 §1–§4. Beklenenler scripts/fixtures/space_command_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "duvarlardan bulunan sınır space_cases.py'nin kesirli yüz bulmasından, ölçüleri Metraj adımıyla. Kurulumdaki çizimde 7 nesne var; "
            "yeni nesne 8'den başlar.")
    return {
        "cad.spaces.create.json": document(CREATE, "Mahal oluştur: duvarlardan ya da çizilmiş sınır, yöntem, bilgiler; doğrulama, plan, yazma, geri alma, retler", note, create_cases()),
        "cad.spaces.edit.json": document(EDIT, "Mahali düzenle ve duvarla güncellenmek: değiştir, birleştir, böl, duvarlardan bul; retler", note, edit_cases()),
        "cad.spaces.quantities.json": document(QUANT, "Metraj: mahaller, duvarlar, açıklıklar; kat, evre, kullanım ve katman süzgeçleri; retler", note, quantities_cases()),
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
