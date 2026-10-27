#!/usr/bin/env python3
"""The slab commands' shared cases (docs/adr/0722 §2–§5), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.slabs.{create,edit}.json; `--check` compares and writes nothing. The drawing is the space
cases' (`space_command_cases.py`: an 8 × 5 building of 20 cm walls on the ground floor, a 15 cm partition at x = 5, a
lone wall far away) with a slab layer. Its Metraj (`cad.spaces.quantities`) is that script's, which measures the slabs as
ADR 0722 §5 says. The rules, as the ADR gives them:

- an outline drawn is kept as given; one found from the walls round a point is, with `outline`, the outer boundary of
  the walls' line work's piece holding it (the largest such boundary round it: the building to its walls' outer faces)
  and, with `room`, the room round it to their inner faces (the space reference's net face); the walls read are those
  of the slab's level, every wall when it has none;
- an edge offset moves every edge of every ring by the same distance, out of the slab when positive: each ring
  oriented (the outer counter-clockwise, the holes clockwise), each edge moved to its right, consecutive moved edges
  meeting where their lines cross; it fails when a ring turns over or goes, an edge turns round, a ring's edges cross,
  or a hole leaves its outer ring or meets another;
- the edge (a railing or a parapet) runs along the free sides: the ring edges whose middle no wall's face (of the walls
  the slab reads) runs through; the document writes them again whenever the walls change;
- `cad.slabs.edit`: set changes fields; offset moves the edges; subtract takes an area out (a hole, a notch) and add adds
  one: the cases' areas are axis-aligned rectangles, measured on their compressed grid (each cell in or out; its area
  the cells', its perimeter the sides between in and out).
"""

import argparse
import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import space_cases as sp  # noqa: E402
import space_command_cases as sc  # noqa: E402
from wall_join_cases import from_json  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
CREATE, EDIT, QUANT = "cad.slabs.create", "cad.slabs.edit", "cad.spaces.quantities"
P, refusal, num, uid_of = sc.P, sc.refusal, sc.num, sc.uid_of

# The space cases' drawing with a slab layer.
sc.LAYERS["doseme"] = "Döşeme"
SETUP = copy.deepcopy(sc.SETUP)
SETUP["name"] = "Döşemeler"
SETUP["layers"].insert(2, sc.layer("doseme", "Döşeme"))
WIDTH = {"railing": 0.05, "parapet": 0.15}
WHOSE = {"railing": "Korkuluğun", "parapet": "Parapetin"}
LEFT, RIGHT = P(2, 2), P(7, 2)
MAX_OFFSET = 10

# ── Outlines ──────────────────────────────────────────────────────────────────────────────────────────────────────


def walls_read(d, level):
    return [from_json(w) for _, w in sorted(d.walls.items()) if level is None or w.get("arch", {}).get("level") == level]


def outline(d, seed, level):
    """The largest outer boundary of the walls' line work's pieces round the seed, counter-clockwise; or None."""
    walls = walls_read(d, level)
    if not walls:
        return None
    pieces = sp.split(sp.faces(walls))
    best = None
    for comp in sp.components(pieces):
        cw = [c for c in sp.traced(comp) if sp.area2(c) < 0]
        if not cw:
            continue
        ring = min(cw, key=sp.area2)[::-1]
        if sp.inside(ring, seed) and (best is None or sp.area2(ring) > sp.area2(best)):
            best = ring
    return None if best is None else [(best, [])]


def found(d, seed, boundary, level):
    q = (F(seed["x"]), F(seed["y"]))
    if boundary == "room":
        return d.found(seed, "net", level)
    return outline(d, q, level)


def rings_of(parts_json):
    return sc.exact_parts(parts_json)

# ── The edge offset ───────────────────────────────────────────────────────────────────────────────────────────────


def oriented(ring, ccw):
    return ring if (sp.area2(ring) > 0) == ccw else ring[::-1]


def moved_ring(ring, d):
    n = len(ring)
    out = []
    for i in range(n):
        p = ring[i]
        tp, ti = sp.sub(p, ring[(i - 1) % n]), sp.sub(ring[(i + 1) % n], p)
        ap, ai = sp.add(p, sp.mul(sp.rightn(tp), d)), sp.add(p, sp.mul(sp.rightn(ti), d))
        q = sp.meet(ap, tp, ai, ti)
        out.append(q if q is not None else ai)
    return out


def seg_meet(a, b, c, e):
    """Whether two segments cross or touch."""
    d1, d2 = sp.sub(b, a), sp.sub(e, c)
    den = sp.cross(d1, d2)
    if den == 0:
        if sp.cross(sp.sub(c, a), d1) != 0:
            return False
        return any(sp.on_segment(q, a, b) for q in (c, e)) or any(sp.on_segment(q, c, e) for q in (a, b))
    t = sp.cross(sp.sub(c, a), d2) / den
    u = sp.cross(sp.sub(c, a), d1) / den
    return 0 <= t <= 1 and 0 <= u <= 1


def self_crossing(r):
    n = len(r)
    for i in range(n):
        for j in range(i + 1, n):
            if j == i + 1 or (i == 0 and j == n - 1):
                continue
            if seg_meet(r[i], r[(i + 1) % n], r[j], r[(j + 1) % n]):
                return True
    return False


def ring_failure(before, after, hole):
    sb, sa = sp.area2(before), sp.area2(after)
    if sa == 0 or (sb > 0) != (sa > 0):
        return ("Kenar ötelemesi deliği kapatıyor ya da ters çeviriyor." if hole
                else "Kenar ötelemesi döşemeyi kapatıyor ya da ters çeviriyor.")
    n = len(before)
    for i in range(n):
        if sp.dot(sp.sub(before[(i + 1) % n], before[i]), sp.sub(after[(i + 1) % n], after[i])) <= 0:
            return "Kenar ötelemesi bir kenarı yok ediyor ya da ters çeviriyor; daha küçük bir uzaklık verin."
    if self_crossing(after):
        return "Kenar ötelemesi halkanın kenarlarını kesiştiriyor; daha küçük bir uzaklık verin."
    return None


def rings_meet(a, b):
    return any(seg_meet(a[i], a[(i + 1) % len(a)], b[j], b[(j + 1) % len(b)]) for i in range(len(a)) for j in range(len(b)))


def offset(parts, d, path):
    """(parts, None) or (None, refusal)."""
    if d == 0:
        return parts, None
    if abs(d) > MAX_OFFSET:
        return None, refusal("invalid_slab", f"Kenar ötelemesi en çok ±{MAX_OFFSET} m olur; {num(d)} verildi.", path)
    d = F(d)
    out = []
    for k, (outer, holes) in enumerate(parts):
        o = oriented(outer, True)
        mo = moved_ring(o, d)
        why = ring_failure(o, mo, False)
        if why:
            return None, refusal("slab_offset_failed", f"{why.rstrip('.')} ({k + 1}. parçanın dış halkası).", path)
        mh = []
        for h_i, h in enumerate(holes):
            hh = oriented(h, False)
            m = moved_ring(hh, d)
            why = ring_failure(hh, m, True)
            if why is None and (rings_meet(m, mo) or not sp.inside(mo, m[0]) or any(rings_meet(m, g) for g in mh)):
                why = "Kenar ötelemesi deliği dış kenara ya da başka bir deliğe taşıyor."
            if why:
                return None, refusal("slab_offset_failed", f"{why.rstrip('.')} ({k + 1}. parçanın {h_i + 1}. deliği).", path)
            mh.append(m)
        out.append((mo, mh))
    return out, None

# ── Rectangles' unions and differences, on their compressed grid ─────────────────────────────────────────────────


def boxes_of(parts):
    """The parts as rectangles (outer, holes) when every ring is an axis-aligned rectangle."""
    def box(r):
        xs, ys = [q[0] for q in r], [q[1] for q in r]
        return (min(xs), min(ys), max(xs), max(ys))
    return [(box(o), [box(h) for h in hs]) for o, hs in parts]


def grid_measure(inside_fn, xs, ys):
    xs, ys = sorted(set(xs)), sorted(set(ys))
    cells = {}
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            cells[(i, j)] = inside_fn(((xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2))
    area, perim = F(0), F(0)
    for (i, j), on in cells.items():
        if not on:
            continue
        area += (xs[i + 1] - xs[i]) * (ys[j + 1] - ys[j])
        for di, dj, side in ((1, 0, ys[j + 1] - ys[j]), (-1, 0, ys[j + 1] - ys[j]), (0, 1, xs[i + 1] - xs[i]), (0, -1, xs[i + 1] - xs[i])):
            if not cells.get((i + di, j + dj), False):
                perim += side
    return area, perim


def in_box(b, q):
    return b[0] < q[0] < b[2] and b[1] < q[1] < b[3]


def in_parts(bparts, q):
    return any(in_box(o, q) and not any(in_box(h, q) for h in hs) for o, hs in bparts)


def combined(parts, other, cut):
    """The measure (area, perimeter) of the parts less or with the other rectangles; and the area before."""
    a, b = boxes_of(parts), boxes_of(other)
    xs = [v for o, hs in a + b for bx in [o] + hs for v in (bx[0], bx[2])]
    ys = [v for o, hs in a + b for bx in [o] + hs for v in (bx[1], bx[3])]
    if cut:
        return grid_measure(lambda q: in_parts(a, q) and not in_parts(b, q), xs, ys)
    return grid_measure(lambda q: in_parts(a, q) or in_parts(b, q), xs, ys)

# ── The slab's fields ─────────────────────────────────────────────────────────────────────────────────────────────


def ring_problem(r, at):
    if len(r) < 3:
        return refusal("invalid_slab", f"Döşemenin halkası en az üç köşedir; {len(r)} verildi.", at)
    return None


def problem(s, prefix):
    if "parts" in s:
        for i, (o, hs) in enumerate(s["parts"]):
            for at, r in [(f"parts[{i}].outer", o)] + [(f"parts[{i}].holes[{k}]", h) for k, h in enumerate(hs)]:
                r = ring_problem(r, f"{prefix}.{at}")
                if r:
                    return r
    t = s["thickness"]
    if not (math.isfinite(t) and 0 < t <= 5):
        return refusal("invalid_slab", f"Döşemenin kalınlığı sıfırdan büyük, en çok 5 m olmalı; {num(t)} verildi.", f"{prefix}.thickness")
    o = s.get("offset", 0.0)
    if not (math.isfinite(o) and abs(o) <= 100):
        return refusal("invalid_slab", f"Döşemenin kata göre kotu sonlu, en çok ±100 m olmalı; {num(o)} verildi.", f"{prefix}.offset")
    m = s.get("material")
    if m is not None and (m.strip() == "" or m.strip() != m or len(m) > 120):
        return refusal("invalid_slab", "Döşemenin malzemesi boş, kenarı boşluklu ya da denetim karakterli olamaz; en çok 120 karakter.", f"{prefix}.material")
    e = s.get("edge")
    if e is not None and not (math.isfinite(e["width"]) and 0 < e["width"] <= 2):
        return refusal("invalid_slab", f"{WHOSE[e['kind']]} eni sıfırdan büyük, en çok 2 m olmalı; {num(e['width'])} verildi.", f"{prefix}.edge.width")
    return None


def free_length(d, s):
    return sc.free_length(d, s["parts"], s.get("arch", {}).get("level")) if "parts" in s else F(0)


def stored(d, s):
    """The slab's fields as the document keeps them; an outline found or moved is `$any` (its order is the core's)."""
    out = {"parts": s["json"] if s.get("json") is not None else "$any"}
    if s.get("role", "floor") != "floor":
        out["role"] = s["role"]
    out["thickness"] = s["thickness"]
    if s.get("offset"):
        out["offset"] = s["offset"]
    if s.get("arch"):
        out["arch"] = s["arch"]
    if s.get("material") is not None:
        out["material"] = s["material"]
    e = s.get("edge")
    if e is not None:
        edge = {"kind": e["kind"], "width": e["width"]}
        if free_length(d, s) > 0:
            edge["runs"] = "$any"
        out["edge"] = edge
    if s.get("excluded"):
        out["excluded"] = True
    return out


def entity(d, id, layer_id, s, extra=None):
    return {"kind": "slab", "id": id, "layerId": layer_id, "attrs": dict(s.get("attrs", {})), **(extra or {}), **stored(d, s)}

# ── cad.slabs.create ──────────────────────────────────────────────────────────────────────────────────────────────


def not_finite(name, path):
    return refusal("not_finite", f"{name} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", path)


def level_check(arch, path):
    level = arch.get("level")
    if level is None or any(lv["id"] == level for lv in sc.LEVELS):
        return None
    return refusal("level_not_found", f"Projede “{level}” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.", path)


def outline_of(d, parts, seed, boundary, level, path):
    """(parts as exact rings, json or None) or a refusal."""
    if parts is not None and seed is None:
        return (rings_of(parts), parts), None
    if seed is not None and parts is None:
        p = found(d, seed, boundary, level)
        if p is None:
            words = ("Duvarlar noktanın çevresinde bir oda kapatmıyor; döşemenin sınırı bulunamadı. Noktayı duvarlarla çevrili bir odanın içine verin ya da sınırı çizin."
                     if boundary == "room" else
                     "Nokta hiçbir duvar konturunun içinde değil; döşemenin sınırı bulunamadı. Noktayı yapının içine verin ya da sınırı çizin.")
            return None, refusal("slab_not_enclosed", words, f"{path}.seed")
        return (p, None), None
    if parts is not None:
        return None, refusal("invalid_slab", "Döşemenin sınırı ya çizilir (parts) ya da duvarlardan bulunur (seed); ikisi birden verilmez.", path)
    return None, refusal("invalid_slab", "Döşemenin yeri verilmedi: sınırını (parts) ya da duvarlarla çevrili bir iç noktasını (seed) verin.", path)


def create(d, inp, conflict=False):
    if not inp["slabs"]:
        return refusal("no_objects", "Yazılacak döşeme verilmedi. En az bir döşemenin sınırını ya da duvarlarla çevrili bir iç noktasını verin.", "slabs"), None
    made = []
    for i, n in enumerate(inp["slabs"]):
        at = f"slabs[{i}]"
        if "seed" in n:
            r = sc.point_not_finite(n["seed"], "Döşemenin iç noktasının", f"{at}.seed")
            if r:
                return r, None
        for k, part in enumerate(n.get("parts", []) or []):
            for h, ring in enumerate([part["outer"]] + part.get("holes", [])):
                rp = f"{at}.parts[{k}].outer" if h == 0 else f"{at}.parts[{k}].holes[{h - 1}]"
                for j, q in enumerate(ring["pts"]):
                    r = sc.point_not_finite(q, f"{j + 1}. köşenin", f"{rp}.pts[{j}]")
                    if r:
                        return r, None
        for key, name in (("thickness", "Kalınlık"), ("offset", "Kot farkı"), ("edgeOffset", "Kenar ötelemesi")):
            if key in n and not math.isfinite(n[key]):
                return not_finite(name, f"{at}.{key}"), None
        if "edge" in n and "width" in n["edge"] and not math.isfinite(n["edge"]["width"]):
            return not_finite(f"{WHOSE[n['edge']['kind']]} eni", f"{at}.edge.width"), None
        arch = n.get("arch", {})
        if ("parts" in n) != ("seed" in n):
            r = level_check(arch, f"{at}.arch.level")
            if r:
                return r, None
        got, r = outline_of(d, n.get("parts"), n.get("seed"), n.get("boundary", "outline"), arch.get("level"), at)
        if r:
            return r, None
        parts, js = got
        if n.get("edgeOffset", 0):
            parts, r = offset(parts, n["edgeOffset"], f"{at}.edgeOffset")
            if r:
                return r, None
            js = None
        s = {"parts": parts, "json": js, "role": n.get("role", "floor"), "thickness": n.get("thickness", 0.15),
             "offset": n.get("offset", 0.0), "arch": arch, "material": n.get("material"), "excluded": n.get("excluded", False),
             "attrs": n.get("attrs", {})}
        if "edge" in n:
            s["edge"] = {"kind": n["edge"]["kind"], "width": n["edge"].get("width", WIDTH[n["edge"]["kind"]])}
        r = problem(s, at)
        if r:
            return r, None
        made.append(s)
    if "lineWeight" in inp and not (math.isfinite(inp["lineWeight"]) and 0 <= inp["lineWeight"] <= 100):
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None
    r = sc.revision(inp, conflict)
    if r:
        return r, None
    r, warnings = sc.layer_check(inp["layerId"])
    if r:
        return r, None
    return {"status": "completed", "warnings": warnings}, made


def extra_of(inp):
    return {k: inp[k] for k in ("color", "lineWeight") if k in inp}


def create_step(d, op, inp, non_finite=None, conflict=False, command=None):
    result, made = create(d, sc.with_non_finite(inp, non_finite or {}), conflict)
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
        s["result"] = {"status": "completed", "output": {"slabs": [entity(d, 0, inp["layerId"], m, extra_of(inp)) for m in made],
                                                          "revision": "$current"}, "warnings": result["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ids = list(range(d.next, d.next + len(made)))
    d.next += len(made)
    for i, m in zip(ids, made):
        d.slabs[i] = (inp["layerId"], m, extra_of(inp))
    s["result"] = {"status": "completed", "output": {"uids": ["$any"] * len(made), "ids": ids, "revision": "$current"},
                   "warnings": result["warnings"]}
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(d, i, inp["layerId"], m, extra_of(inp)) for i, m in zip(ids, made)},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def quant(d, inp=None):
    return sc.quant_step(d, inp, command=QUANT)

# ── cad.slabs.edit ────────────────────────────────────────────────────────────────────────────────────────────────


def named(d, uids):
    if not uids:
        return refusal("no_entities", "Düzenlenecek döşeme verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids")
    return sc.named(d, uids)


def edit(d, inp, conflict=False):
    found_ = named(d, inp["uids"])
    if isinstance(found_, dict):
        return found_, None
    for at, i in found_:
        if i not in d.slabs:
            return refusal("not_a_slab", "Nesne döşeme değil; Döşemeyi düzenle yalnız döşemeleri değiştirir.", f"uids[{at}]"), None
    op = inp["operation"]
    kind = op["kind"]
    updates = []
    if kind == "set":
        ch = {k: v for k, v in op.items() if k != "kind"}
        if not ch:
            return refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi: tür, kalınlık, kot farkı, kat, evre, malzeme, kenar ya da metraj dışı seçimlerinden birini verin.", "operation"), None
        for key, name in (("thickness", "Kalınlık"), ("offset", "Kot farkı")):
            if key in ch and not math.isfinite(ch[key]):
                return not_finite(name, f"operation.{key}"), None
        if ch.get("edge") is not None and "width" in ch["edge"] and not math.isfinite(ch["edge"]["width"]):
            return not_finite(f"{WHOSE[ch['edge']['kind']]} eni", "operation.edge.width"), None
        for at, i in found_:
            s = copy.deepcopy(d.slabs[i][1])
            for k in ("role", "thickness", "offset", "material", "excluded"):
                if k in ch:
                    s[k] = ch[k]
            arch = dict(s.get("arch", {}))
            for k in ("level", "phase"):
                if k in ch:
                    if ch[k] is None:
                        arch.pop(k, None)
                    else:
                        arch[k] = ch[k]
            s["arch"] = arch
            if "edge" in ch:
                e = ch["edge"]
                if e is None:
                    s.pop("edge", None)
                else:
                    s["edge"] = {"kind": e["kind"], "width": e.get("width", WIDTH[e["kind"]])}
            r = problem(s, f"uids[{at}]") or level_check(arch, "operation.level")
            if r:
                return r, None
            updates.append((i, s))
    elif kind == "offset":
        dist = op["distance"]
        if not math.isfinite(dist):
            return not_finite("Kenar ötelemesi", "operation.distance"), None
        if dist == 0:
            return refusal("nothing_to_set", "Kenar ötelemesi sıfır; kenarları taşımak için sıfırdan farklı bir uzaklık verin.", "operation.distance"), None
        for at, i in found_:
            s = copy.deepcopy(d.slabs[i][1])
            parts, r = offset(s["parts"], dist, f"uids[{at}]")
            if r:
                return r, None
            s.update(parts=parts, json=None)
            updates.append((i, s))
    else:
        cut = kind == "subtract"
        if "seed" in op:
            r = sc.point_not_finite(op["seed"], "Alanın iç noktasının", "operation.seed")
            if r:
                return r, None
        for at, i in found_:
            s = copy.deepcopy(d.slabs[i][1])
            got, r = outline_of(d, op.get("parts"), op.get("seed"), op.get("boundary", "outline"), s.get("arch", {}).get("level"), "operation")
            if r:
                return r, None
            other = got[0]
            before = sc.area_of(s["parts"])
            area, perim = combined(s["parts"], other, cut)
            if cut and (area == before or area == 0):
                words = ("Kesilen alan döşemenin hepsini götürüyor; döşemenin içinde kalan bir alan verin ya da döşemeyi silin." if area == 0
                         else "Kesilen alan döşemeye değmiyor; döşemenin üzerinde kalan bir alan verin.")
                return refusal("slab_cut_missed", words, f"uids[{at}]"), None
            s.pop("parts")
            s.update(json=None, measure=(area, perim))
            updates.append((i, s))
    r = sc.revision(inp, conflict)
    if r:
        return r, None
    for at, i in found_:
        if d.slabs[i][0] in d.locked:
            return refusal("layer_locked", f"“{sc.LAYERS[d.slabs[i][0]]}” katmanı kilitli; üzerindeki döşeme düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{at}]"), None
    return {"status": "completed"}, updates


def edit_step(d, op, inp, non_finite=None, conflict=False, command=None):
    result, work = edit(d, sc.with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op}
    if command:
        s["command"] = command
    s["input"] = inp
    if non_finite:
        s["nonFinite"] = non_finite
    if work is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ents = [entity(d, i, d.slabs[i][0], u, d.slabs[i][2]) for i, u in work]
    s["result"] = {"status": "completed", "output": {"slabs": ents, "revision": "$current"}, "warnings": []}
    if op == "plan":
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    for i, u in work:
        d.slabs[i] = (d.slabs[i][0], u, d.slabs[i][2])
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(d, i, d.slabs[i][0], d.slabs[i][1], d.slabs[i][2]) for i, _ in work},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s

# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────


def box_json(x0, y0, x1, y1, holes=()):
    part = {"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}
    if holes:
        part["holes"] = [{"pts": [P(a, b), P(a, d), P(c, d), P(c, b)]} for a, b, c, d in holes]
    return part


def one(layer="doseme", **kw):
    return {"layerId": layer, "slabs": [kw]}


BALCONY = box_json(2, 5.1, 6, 6.6)


def create_cases():
    c = sc.Cases()
    d = sc.Drawing()
    inp = one(parts=[box_json(20, 0, 24, 3)], material="Betonarme")
    c.add("çizilmiş dikdörtgen döşeme yazılır; metrajda alan, çevre ve hacim", [
        create_step(d, "execute", inp), quant(d),
        {"op": "undo", "returns": "Döşeme", "expect": {"ids": sc.IDS, "canUndo": False, "canRedo": True, "revision": "changed"}}])
    d = sc.Drawing()
    c.add("plan yazmadan döşemeyi gösterir", [create_step(d, "plan", one(parts=[box_json(20, 0, 24, 3)]))])
    d = sc.Drawing()
    c.add("doğrulama çizimi değiştirmez", [create_step(d, "validate", one(parts=[box_json(20, 0, 24, 3)]))])
    for name, n in [
        ("duvarların dış yüzlerinden yapının döşemesi (kat verilmedi: bütün duvarlar)", {"seed": LEFT}),
        ("aynı kattaki duvarların dış yüzlerinden döşeme, 20 cm", {"seed": RIGHT, "thickness": 0.2, "arch": {"level": "zemin"}}),
        ("odanın iç yüzlerinden döşeme: ıslak hacmin 5 cm aşağıdaki döşemesi",
         {"seed": RIGHT, "boundary": "room", "offset": -0.05, "arch": {"level": "zemin"}, "material": "Su yalıtımlı şap"}),
        ("dış konturun kenarı 10 cm dışa ötelenir", {"seed": LEFT, "edgeOffset": 0.1}),
        ("çizilmiş döşemenin kenarı 50 cm içe ötelenir", {"parts": [box_json(20, 0, 24, 3)], "edgeOffset": -0.5}),
        ("delikli döşemede ötelenince delik küçülür", {"parts": [box_json(20, 0, 30, 6, holes=[(22, 2, 25, 4)])], "edgeOffset": 0.25}),
        ("L konturlu döşeme", {"parts": [{"outer": {"pts": [P(20, 0), P(26, 0), P(26, 3), P(23, 3), P(23, 6), P(20, 6)]}}]}),
        ("avlulu döşeme: delik alanı düşülür, çevreye katılır", {"parts": [box_json(20, 0, 30, 8, holes=[(23, 3, 27, 5)])]}),
        ("iki parçalı döşeme", {"parts": [box_json(20, 0, 22, 2), box_json(25, 0, 27, 2)]}),
        ("duvara bitişik balkon: korkuluk serbest üç kenar boyunca", {"parts": [BALCONY], "role": "balcony", "edge": {"kind": "railing"}}),
        ("teras: dört yanı serbest, parapet 20 cm", {"parts": [box_json(20, 10, 26, 14)], "role": "terrace", "edge": {"kind": "parapet", "width": 0.2}}),
        ("dış konturdan döşemenin korkuluğunun serbest kenarı yok", {"seed": LEFT, "edge": {"kind": "railing"}}),
        ("metraj dışı döşeme listelenir, toplanmaz", {"parts": [box_json(20, 0, 24, 3)], "excluded": True}),
    ]:
        d = sc.Drawing()
        c.add(name, [create_step(d, "execute", one(**n)), quant(d)])
    d = sc.Drawing()
    shaft = sc.create_step(d, "execute", {"layerId": "mahal", "spaces": [{"parts": [sc.box_json(3, 1, 4, 2)], "void": True}]},
                           command="cad.spaces.create")
    c.add("şaft boşluğu döşemenin net alanından düşülür", [
        shaft, create_step(d, "execute", one(seed=LEFT)), quant(d)])
    d = sc.Drawing()
    c.add("gizli katmana uyarıyla yazılır", [create_step(d, "execute", one(layer="gizli", parts=[box_json(20, 0, 24, 3)]))])
    for name, inp, kw in [
        ("döşeme verilmezse reddedilir", {"layerId": "doseme", "slabs": []}, {}),
        ("ne sınır ne iç nokta: reddedilir", one(thickness=0.2), {}),
        ("sınır ve iç nokta birlikte verilmez", one(parts=[box_json(20, 0, 24, 3)], seed=LEFT), {}),
        ("duvarların dışındaki noktadan döşeme bulunmaz", one(seed=P(40, 40)), {}),
        ("odası olmayan noktadan oda döşemesi bulunmaz", one(seed=P(40, 40), boundary="room"), {}),
        ("katında duvarı olmayan noktadan bulunmaz", one(seed=LEFT, arch={"level": "kat-1"}), {}),
        ("projede olmayan kat reddedilir", one(seed=LEFT, arch={"level": "bodrum"}), {}),
        ("ters çeviren kenar ötelemesi reddedilir", one(parts=[box_json(20, 0, 24, 3)], edgeOffset=-1.6), {}),
        ("deliği dış kenara taşıyan öteleme reddedilir", one(parts=[box_json(20, 0, 30, 6, holes=[(21, 1, 23, 3)])], edgeOffset=-1.5), {}),
        ("deliği kapatan öteleme reddedilir", one(parts=[box_json(20, 0, 30, 6, holes=[(21, 1, 23, 3)])], edgeOffset=1), {}),
        ("10 metreyi aşan öteleme reddedilir", one(parts=[box_json(20, 0, 24, 3)], edgeOffset=12), {}),
        ("sıfır kalınlık reddedilir", one(parts=[box_json(20, 0, 24, 3)], thickness=0), {}),
        ("5 metreyi aşan kalınlık reddedilir", one(parts=[box_json(20, 0, 24, 3)], thickness=6), {}),
        ("100 metreyi aşan kot farkı reddedilir", one(parts=[box_json(20, 0, 24, 3)], offset=200), {}),
        ("boş malzeme reddedilir", one(parts=[box_json(20, 0, 24, 3)], material="  "), {}),
        ("2 metreyi aşan korkuluk eni reddedilir", one(parts=[BALCONY], edge={"kind": "railing", "width": 3}), {}),
        ("iki köşeli halka reddedilir", one(parts=[{"outer": {"pts": [P(20, 0), P(24, 0)]}}]), {}),
        ("sonsuz iç nokta reddedilir", one(seed=P(2, 2)), {"non_finite": {"slabs[0].seed.x": "NaN"}}),
        ("sonsuz köşe reddedilir", one(parts=[box_json(20, 0, 24, 3)]), {"non_finite": {"slabs[0].parts[0].outer.pts[2].y": "Infinity"}}),
        ("sonsuz kalınlık reddedilir", one(parts=[box_json(20, 0, 24, 3)], thickness=0.2), {"non_finite": {"slabs[0].thickness": "NaN"}}),
        ("sonsuz kenar ötelemesi reddedilir", one(parts=[box_json(20, 0, 24, 3)], edgeOffset=0.1), {"non_finite": {"slabs[0].edgeOffset": "-Infinity"}}),
        ("sonsuz parapet eni reddedilir", one(parts=[BALCONY], edge={"kind": "parapet", "width": 0.2}), {"non_finite": {"slabs[0].edge.width": "NaN"}}),
        ("geçersiz çizgi kalınlığı reddedilir", dict(one(parts=[box_json(20, 0, 24, 3)]), lineWeight=150), {}),
        ("geçersiz beklenen sürüm reddedilir", dict(one(parts=[box_json(20, 0, 24, 3)]), expectedRevision="07"), {}),
        ("çizimde olmayan katman reddedilir", one(layer="yok", parts=[box_json(20, 0, 24, 3)]), {}),
        ("katman grubuna yazılmaz", one(layer="pafta", parts=[box_json(20, 0, 24, 3)]), {}),
        ("kilitli katmana yazılmaz", one(layer="kilitli", parts=[box_json(20, 0, 24, 3)]), {}),
    ]:
        d = sc.Drawing()
        c.add(name, [create_step(d, "execute", inp, **kw)])
    d = sc.Drawing()
    room = sc.create_step(d, "execute", {"layerId": "mahal", "spaces": [{"seed": LEFT}]}, command="cad.spaces.create")
    stale = create_step(d, "execute", dict(one(parts=[box_json(20, 0, 24, 3)]), expectedRevision="$prepared"), conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [{"op": "captureRevision", "as": "prepared"}, room, stale])
    # The document writes the edge's runs again when the walls change (ADR 0722 §3).
    d = sc.Drawing()
    made = create_step(d, "execute", one(parts=[BALCONY], role="balcony", edge={"kind": "railing"}, arch={"level": "zemin"}))
    del d.walls[3]
    gone = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [uid_of(3)]},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}
    c.add("duvarı silinen balkonun korkuluğu dört yanı izler", [made, gone, quant(d)])
    return c.cases


def edit_cases():
    c = sc.Cases()
    first = sc.NEW

    def fresh(**n):
        d = sc.Drawing()
        return d, create_step(d, "execute", one(**n), command=CREATE)

    def ed(d, op, uids=None, **kw):
        return edit_step(d, "execute", {"uids": uids or [uid_of(first)], "operation": op}, **kw)
    for name, n, op in [
        ("tür ve kalınlık değişir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set", "role": "terrace", "thickness": 0.25}),
        ("ıslak hacim için kot farkı ve malzeme", {"seed": RIGHT, "boundary": "room", "arch": {"level": "zemin"}},
         {"kind": "set", "offset": -0.05, "material": "Su yalıtımlı şap"}),
        ("malzeme kaldırılır", {"parts": [box_json(20, 0, 24, 3)], "material": "Betonarme"}, {"kind": "set", "material": None}),
        ("metraj dışı yapılır", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set", "excluded": True}),
        ("balkona korkuluk verilir", {"parts": [BALCONY], "role": "balcony"}, {"kind": "set", "edge": {"kind": "railing"}}),
        ("korkuluk parapete döner", {"parts": [BALCONY], "role": "balcony", "edge": {"kind": "railing"}},
         {"kind": "set", "edge": {"kind": "parapet", "width": 0.2}}),
        ("kenar kaldırılır", {"parts": [BALCONY], "role": "balcony", "edge": {"kind": "railing"}}, {"kind": "set", "edge": None}),
        ("başka kata alınan balkonun korkuluğu dört yanı izler", {"parts": [BALCONY], "edge": {"kind": "railing"}, "arch": {"level": "zemin"}},
         {"kind": "set", "level": "kat-1"}),
        ("evre verilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set", "phase": "new"}),
        ("kenar 25 cm dışa ötelenir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "offset", "distance": 0.25}),
        ("delik açılır", {"parts": [box_json(20, 0, 30, 8)]}, {"kind": "subtract", "parts": [box_json(23, 3, 27, 5)]}),
        ("kenardan girinti kesilir", {"parts": [box_json(20, 0, 30, 8)]}, {"kind": "subtract", "parts": [box_json(24, 6, 26, 9)]}),
        ("ıslak hacmin odası döşemeden çıkarılır", {"seed": LEFT, "arch": {"level": "zemin"}},
         {"kind": "subtract", "seed": RIGHT, "boundary": "room"}),
        ("bitişik alan eklenir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "add", "parts": [box_json(24, 0, 26, 3)]}),
        ("ayrık alan eklenir: iki parça", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "add", "parts": [box_json(30, 0, 31, 1)]}),
    ]:
        d, made = fresh(**n)
        c.add(name, [made, ed(d, op), quant(d)])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    c.add("plan yazmadan döşemeyi gösterir", [made, edit_step(d, "plan", {"uids": [uid_of(first)], "operation": {"kind": "set", "thickness": 0.3}})])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    c.add("doğrulama çizimi değiştirmez", [made, edit_step(d, "validate", {"uids": [uid_of(first)], "operation": {"kind": "offset", "distance": 0.1}})])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    c.add("geri alma düzenlemeyi geri verir", [made, ed(d, {"kind": "set", "thickness": 0.3}),
          {"op": "undo", "returns": "Döşemeyi düzenle", "expect": {"canRedo": True, "revision": "changed"}}])
    for name, n, op, kw in [
        ("değiştirilecek alan verilmezse reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set"}, {}),
        ("sıfır öteleme reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "offset", "distance": 0}, {}),
        ("ters çeviren öteleme reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "offset", "distance": -2}, {}),
        ("10 metreyi aşan öteleme reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "offset", "distance": -11}, {}),
        ("sıfır kalınlık reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set", "thickness": 0}, {}),
        ("projede olmayan kat reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set", "level": "bodrum"}, {}),
        ("döşemeye değmeyen kesim reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "subtract", "parts": [box_json(30, 0, 31, 1)]}, {}),
        ("döşemenin hepsini götüren kesim reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "subtract", "parts": [box_json(19, -1, 25, 4)]}, {}),
        ("kesilecek alan verilmezse reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "subtract"}, {}),
        ("duvarların dışındaki noktadan kesilecek oda bulunmaz", {"parts": [box_json(20, 0, 24, 3)]},
         {"kind": "subtract", "seed": P(40, 40), "boundary": "room"}, {}),
        ("sonsuz kalınlık reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set", "thickness": 0.2}, {"non_finite": {"operation.thickness": "NaN"}}),
        ("sonsuz öteleme reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "offset", "distance": 0.2}, {"non_finite": {"operation.distance": "Infinity"}}),
        ("sonsuz iç nokta reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "subtract", "seed": P(2, 2)}, {"non_finite": {"operation.seed.y": "NaN"}}),
        ("geçersiz beklenen sürüm reddedilir", {"parts": [box_json(20, 0, 24, 3)]}, {"kind": "set", "thickness": 0.3}, {"revision": "07"}),
    ]:
        d, made = fresh(**n)
        inp = {"uids": [uid_of(first)], "operation": op}
        if "revision" in kw:
            inp["expectedRevision"] = kw.pop("revision")
        c.add(name, [made, edit_step(d, "execute", inp, **kw)])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    c.add("döşeme olmayan nesne reddedilir", [made, ed(d, {"kind": "set", "thickness": 0.3}, uids=[uid_of(1)])])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    c.add("kimlik verilmezse reddedilir", [made, edit_step(d, "execute", {"uids": [], "operation": {"kind": "set", "thickness": 0.3}})])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    c.add("geçersiz kimlik reddedilir", [made, edit_step(d, "execute", {"uids": ["döşeme"], "operation": {"kind": "set", "thickness": 0.3}})])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    c.add("çizimde olmayan kimlik reddedilir", [made, edit_step(d, "execute", {"uids": [sc.MISSING], "operation": {"kind": "set", "thickness": 0.3}})])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    d.locked.add("doseme")
    c.add("kilitli katmandaki döşeme düzenlenmez", [
        made, {"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "doseme", "value": True},
               "result": {"status": "completed", "output": "$any", "warnings": "$any"}},
        ed(d, {"kind": "set", "thickness": 0.3})])
    d, made = fresh(parts=[box_json(20, 0, 24, 3)])
    stale = edit_step(d, "execute", {"uids": [uid_of(first)], "operation": {"kind": "set", "thickness": 0.3}, "expectedRevision": "$prepared"},
                      conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c.cases


def document(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note,
            "setup": SETUP, "cases": cases}


def files():
    note = ("ADR 0722 §2–§5. Beklenenler scripts/fixtures/slab_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "duvarlardan bulunan sınır space_cases.py'nin kesirli yüz bulmasından, ölçüleri space_command_cases.py'nin Metraj adımıyla "
            "(alan, çevre, hacim, boşluk kesimi, serbest kenar uzunluğu). Kurulumdaki çizimde 7 nesne var; yeni nesne 8'den başlar.")
    return {
        "cad.slabs.create.json": document(CREATE, "Döşeme oluştur: çizilmiş ya da duvarlardan sınır, kenar ötelemesi, korkuluk ve parapet; doğrulama, plan, yazma, geri alma, retler", note, create_cases()),
        "cad.slabs.edit.json": document(EDIT, "Döşemeyi düzenle: değiştir, ötele, kes, ekle; kenarın duvarları izlemesi; retler", note, edit_cases()),
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
