#!/usr/bin/env python3
"""A column's temporary distances and Mesafe (docs/adr/0712), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.columns.distances.json and cad.arch.edit.distance.json; `--check` compares and writes
nothing.

The rules, as ADR 0712 gives them:
- a column's sides run from corner k to the next, the corners (−w/2, −d/2), (w/2, −d/2), (w/2, d/2), (−w/2, d/2) turned
  by its rotation about its centre (a round column's: its bounding square's, w × w); side 0 is below the centre, 1 right,
  2 above, 3 left; outward is to the right of a side;
- on side k, outward: the nearest straight edge running parallel to it (within 0.001°) and overlapping it along its line
  by more than 1e-9 m. A wall (on the column's level, or both on none): its path edge, its faces the left face
  (`offset + w/2` centred, `offset` left, `offset + w` right) and the right face (that less the width) to its own
  left; the nearer of them not more than 1e-9 m behind the side. Another column (same level): its side, not behind.
  The distance square to the side (0 when touching), measured at the middle of the overlap. Walls before columns, then
  the drawing's order (the column measured first among the columns), then the lower edge; a later one only when nearer
  by more than 1e-9 m;
- Mesafe moves the column's centre along side k's outward normal by the gap less the distance typed; one column, not
  tied to grids.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wall_command_cases as wc  # noqa: E402  (layers, ids, non-finite numbers)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
TIE = 1e-9
PARALLEL = 1.745_329_251_994_33e-5
STEP = "Mimari nesneyi düzenle"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def uid(i):
    return f"0192f5a1-7112-7000-8000-{i:012x}"


def wall(id, pts, width=0.2, layer="duvar", **extra):
    return {"kind": "wall", "id": id, "layerId": layer, "attrs": {}, "pts": [P(*p) for p in pts], "width": width,
            "justify": extra.pop("justify", "center"), **extra}


def column(id, at, width, layer="kolon", **extra):
    return {"kind": "column", "id": id, "layerId": layer, "attrs": {}, "at": P(*at), "width": width, **extra}


def grid(id, pts, name):
    return {"kind": "grid", "id": id, "layerId": "aks", "attrs": {}, "pts": [P(*p) for p in pts], "name": name}


ENTITIES = [
    # A: a 6 × 5 room of four walls; a square column, a deep one, a round one and one against the east wall.
    wall(1, [(0, 0), (6, 0)]),
    wall(2, [(6, 0), (6, 5)]),
    wall(3, [(6, 5), (0, 5)]),
    wall(4, [(0, 5), (0, 0)]),
    column(5, (2, 2), 0.4),
    column(6, (4.5, 3.5), 0.3, depth=0.5),
    column(7, (4.5, 1.2), 0.4, shape="round"),
    column(8, (5.75, 4.4), 0.3),
    # B: a closed room with two columns turned 30° side by side, and one turned 45° alone.
    wall(9, [(0, 10), (6, 10), (6, 15), (0, 15)], closed=True),
    column(10, (2, 12.5), 0.4, rotation=math.pi / 6),
    column(11, (2 + 1.5 * math.cos(math.pi / 6), 12.5 + 1.5 * math.sin(math.pi / 6)), 0.4, rotation=math.pi / 6),
    column(12, (4.6, 11.0), 0.3, rotation=math.pi / 4),
    # C: grids A and 1, a column tied where they cross, a wall south of it.
    grid(13, [(-1, 20), (10, 20)], "1"),
    grid(14, [(2, 19), (2, 25)], "A"),
    column(15, (2, 20), 0.4, grids=[uid(14), uid(13)], place="A-1"),
    wall(16, [(0, 18), (6, 18)]),
    # D: a column on the first floor, a wall of the floor west of it and one of the ground floor east.
    wall(17, [(0, 30), (0, 34)], arch={"level": "kat-1"}),
    wall(18, [(4, 30), (4, 34)], arch={"level": "zemin"}),
    column(19, (2, 32), 0.4, arch={"level": "kat-1"}),
    # E: a column on a locked layer between two walls; a left-justified wall beside a column.
    wall(20, [(0, 40), (0, 44)]),
    wall(21, [(4, 40), (4, 44)]),
    column(22, (2, 42), 0.4, layer="kilitli"),
    wall(23, [(0, 50), (6, 50)], justify="left"),
    column(24, (3, 52), 0.4),
    {"kind": "line", "id": 25, "layerId": "cizim", "attrs": {}, "a": P(0, 60), "b": P(4, 60)},
]
SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad", "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                                                    {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]}}
SETUP = {
    "format": "kentos.document", "version": 2, "name": "Kolon mesafeleri", "settings": SETTINGS, "origin": P(0, 0),
    "layers": [wc.layer("duvar", "Duvar"), wc.layer("kolon", "Kolon"), wc.layer("aks", "Aks"), wc.layer("cizim", "Çizim"),
               wc.layer("kilitli", "Kilitli katman", locked=True)],
    "activeLayer": "duvar", "styles": {"items": [], "categories": []}, "entities": ENTITIES,
    "uids": [uid(e["id"]) for e in ENTITIES],
}
IDS = [e["id"] for e in ENTITIES]
BY_ID = {e["id"]: e for e in ENTITIES}


# ── Geometry ─────────────────────────────────────────────────────────────────────────────────────────────────────

def xy(p):
    return (p["x"], p["y"])


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def along(p, d, t):
    return (p[0] + d[0] * t, p[1] + d[1] * t)


def dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def unit(a, b):
    l = dist(a, b)
    return ((b[0] - a[0]) / l, (b[1] - a[1]) / l)


def level(e):
    return (e.get("arch") or {}).get("level")


def left_face(w):
    return w.get("offset", 0.0) + {"left": 0.0, "center": w["width"] / 2.0, "right": w["width"]}[w.get("justify", "center")]


def wall_edges(w):
    pts = [xy(p) for p in w["pts"]]
    n = len(pts)
    count = n if w.get("closed") else n - 1
    out = []
    for k in range(count):
        a, b = pts[k], pts[(k + 1) % n]
        if dist(a, b) > 0 and ((w.get("bulges") or [0.0] * n)[k] == 0.0):
            out.append((k, a, b))
    return out


def corners(c):
    hw = c["width"] / 2.0
    hd = hw if c.get("shape") == "round" else c.get("depth", c["width"]) / 2.0
    rot = c.get("rotation", 0.0)
    s, k = math.sin(rot), math.cos(rot)
    at = xy(c["at"])
    return [(at[0] + x * k - y * s, at[1] + x * s + y * k) for x, y in [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)]]


def sides(c):
    k = corners(c)
    return [(k[i], k[(i + 1) % 4]) for i in range(4)]


def overlap(a, u, l, c, d):
    tc, td = dot(sub(c, a), u), dot(sub(d, a), u)
    lo, hi = max(0.0, min(tc, td)), min(l, max(tc, td))
    return (lo + hi) / 2.0 if hi - lo > TIE else None


def column_order(state, first):
    return [first] + [i for i in sorted(state) if state[i]["kind"] == "column" and i != first]


def gaps(state, k):
    me = state[k]
    walls = [i for i in sorted(state) if state[i]["kind"] == "wall"]
    cols = column_order(state, k)
    best = [None] * 4

    def take(side, g):
        if best[side] is None or g["distance"] < best[side]["distance"] - TIE:
            best[side] = g
    for side, (a, b) in enumerate(sides(me)):
        u = unit(a, b)
        l = dist(a, b)
        n = (u[1], -u[0])
        for j in walls:
            w = state[j]
            if level(w) != level(me):
                continue
            for e, c, d in wall_edges(w):
                v = unit(c, d)
                if abs(cross(u, v)) > PARALLEL:
                    continue
                mid = overlap(a, u, l, c, d)
                if mid is None:
                    continue
                sgn = 1.0 if dot((-v[1], v[0]), n) > 0 else -1.0
                sd = dot(sub(c, a), n)
                lf = left_face(w)
                faces = [(sd + sgn * lf, True), (sd + sgn * (lf - w["width"]), False)]
                ahead = [f for f in faces if f[0] >= -TIE]
                if not ahead:
                    continue
                near, left = min(ahead, key=lambda f: f[0])
                base = along(a, u, mid)
                take(side, {"side": side, "target": "wall", "id": j, "edge": e, "face": "left" if left else "right",
                            "distance": max(0.0, near), "from": base, "to": along(base, n, near)})
        for j in cols[1:]:
            o = state[j]
            if level(o) != level(me):
                continue
            for s, (c, d) in enumerate(sides(o)):
                v = unit(c, d)
                if abs(cross(u, v)) > PARALLEL:
                    continue
                mid = overlap(a, u, l, c, d)
                if mid is None:
                    continue
                f = dot(sub(c, a), n)
                if f < -TIE:
                    continue
                base = along(a, u, mid)
                take(side, {"side": side, "target": "column", "id": j, "edge": s, "distance": max(0.0, f),
                            "from": base, "to": along(base, n, f)})
    return best


def u(i):
    return f"$uidOf:{i}"


def answer(state, ids):
    out = []
    for i in ids:
        c = state[i]
        item = {"uid": u(i)}
        if "grids" in c:
            item["tied"] = True
        item["sides"] = []
        for g in gaps(state, i):
            if g is None:
                continue
            s = {"side": g["side"], "distance": g["distance"], "uid": u(g["id"]), "target": g["target"], "edge": g["edge"]}
            if g["target"] == "wall":
                s["face"] = g["face"]
            s.update({"from": P(*g["from"]), "to": P(*g["to"])})
            item["sides"].append(s)
        out.append(item)
    return {"columns": out, "revision": "$current"}


# ── The steps ────────────────────────────────────────────────────────────────────────────────────────────────────

def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def before():
    return {"ids": IDS, "revision": "same"}


def named(inp, empty):
    return wc.named(inp, empty)


def distances_cases():
    state = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
    c = []

    def ask(name, inp, op="execute"):
        got = named(inp, "Ölçülecek kolon verilmedi.")
        if isinstance(got, dict):
            result = got
        else:
            result = None
            for at, i in got:
                if state[i]["kind"] != "column":
                    result = refusal("not_a_column", "Nesne kolon değil; kolonun mesafeleri yalnız kolonlarda ölçülür.",
                                     f"uids[{at}]")
                    break
            if result is None:
                out = answer(state, [i for _, i in got])
                result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
        c.append({"name": name, "steps": [{"op": op, "input": inp, "result": result, "within": 1e-9, "expect": before()}]})
    ask("kare kolon: dört yanda odanın duvarlarına net mesafeler", {"uids": [u(5)]})
    ask("derin kolon: üstte kuzey duvarı, solda kare kolon değil (örtüşmüyor), sağda doğu duvarı", {"uids": [u(6)]})
    ask("yuvarlak kolon: çevreleyen karenin yanlarından", {"uids": [u(7)]})
    ask("doğu duvarına dayalı kolon: o yanda 0", {"uids": [u(8)]})
    ask("30° dönük kolon: yalnız yanındaki aynı dönüklükte kolona", {"uids": [u(10)]})
    ask("öbür 30° dönük kolon: karşı yandan aynı aralık", {"uids": [u(11)]})
    ask("45° dönük tek kolon: paralel yüz yok", {"uids": [u(12)]})
    ask("akslara bağlı kolon: tied, güneyindeki duvara mesafe", {"uids": [u(15)]})
    ask("1. kattaki kolon: kendi katındaki duvar ölçülür, zemin kattaki atlanır", {"uids": [u(19)]})
    ask("kilitli katmandaki kolon da ölçülür", {"uids": [u(22)]})
    ask("sola hizalı duvarın yüzleri yolunun solunda", {"uids": [u(24)]})
    ask("iki 30° dönük kolon birlikte", {"uids": [u(10), u(11)]})
    ask("iki kolon birden, verildiği sırayla", {"uids": [u(6), u(5)]})
    ask("plan aynı cevabı verir", {"uids": [u(5)]}, op="plan")
    ask("doğrula cevap vermez, yazmaz", {"uids": [u(5)]}, op="validate")
    ask("duvar kolon değil: reddedilir", {"uids": [u(1)]})
    ask("aks kolon değil: reddedilir", {"uids": [u(13)]})
    ask("çizgi kolon değil: reddedilir", {"uids": [u(25)]})
    ask("kimliksiz istek reddedilir", {"uids": []})
    ask("geçersiz kimlik reddedilir", {"uids": ["K-1"]})
    ask("çizimde olmayan kimlik reddedilir", {"uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]})
    ask("ikinci kimlik kolon değilse reddedilir", {"uids": [u(5), u(2)]})
    return c


def edit(state, inp):
    got = named(inp, "Düzenlenecek aks, kolon, merdiven ya da rampa verilmedi.")
    if isinstance(got, dict):
        return got, None
    for at, i in got:
        if state[i]["kind"] not in ("grid", "column", "stair", "ramp"):
            return refusal("not_arch_element", "Nesne aks, kolon, merdiven ya da rampa değil; Mimari nesneyi düzenle yalnız bunları değiştirir.",
                           f"uids[{at}]"), None
    op = inp["operation"]
    if len(got) != 1:
        return refusal("invalid_column", "Mesafe tek kolonla çalışır; bir kolon verin.", "uids"), None
    at, i = got[0]
    c = state[i]
    if c["kind"] != "column":
        name = {"grid": "aks", "stair": "merdiven", "ramp": "rampa"}[c["kind"]]
        return refusal("not_a_column", f"Mesafe yalnız kolonu kaydırır; {name} kaydırılmaz.", f"uids[{at}]"), None
    d = op["distance"]
    if not math.isfinite(d):
        return refusal("not_finite", "Mesafe sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir uzunluk verin.", "operation.distance"), None
    if d < 0:
        return refusal("invalid_column", f"Mesafe {wc.num(d)}; sıfır ya da daha büyük olmalı.", "operation.distance"), None
    if "grids" in c:
        place = f" ({c['place']})" if "place" in c else ""
        return refusal("column_on_grids", f"Kolon akslara bağlı{place}; yeri aksların kesişimidir. Kaydırmak için önce Bağı kopar'ı seçin ya da aksı taşıyın.",
                       f"uids[{at}]"), None
    side = op["side"]
    if side > 3:
        return refusal("invalid_column", f"Kolonun {side}. yanı yok; yanları 0 ile 3 arasında.", "operation.side"), None
    g = gaps(state, i)[side]
    if g is None:
        return refusal("invalid_column", "Kolonun bu yanında ona paralel bir duvar ya da kolon yok; mesafe ölçülemiyor.", "operation.side"), None
    a, b = sides(c)[side]
    uu = unit(a, b)
    n = (uu[1], -uu[0])
    to = along(xy(c["at"]), n, g["distance"] - d)
    if dist(to, xy(c["at"])) <= TIE:
        return refusal("nothing_to_set", "Kolon zaten bu konumda; kaydırılacak bir şey yok.", "operation"), None
    if c["layerId"] == "kilitli":
        return refusal("layer_locked", "“Kilitli katman” katmanı kilitli; üzerindeki kolon düzenlenemez. Kilidi Katmanlar panelinden açın.",
                       f"uids[{at}]"), None
    moved = dict(c, at=P(*to))
    return {"status": "completed", "output": {"elements": [moved], "revision": "$current"}, "warnings": []}, {i: moved}


def step(op, inp, non_finite=None):
    state = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
    result, changed = edit(state, wc.with_non_finite(inp, non_finite or {}))
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    s["within"] = 1e-9
    if changed is None or op in ("validate", "plan"):
        if changed is not None and op == "validate":
            result = {"status": "completed", "output": None, "warnings": []}
        s["result"] = result
        s["expect"] = before()
        return s
    s["result"] = result
    s["expect"] = {"ids": IDS, "entities": {str(i): e for i, e in changed.items()}, "canUndo": True, "dirty": True,
                   "revision": "changed"}
    return s


def distance_in(i, side, d):
    return {"uids": [u(i)], "operation": {"kind": "distance", "side": side, "distance": d}}


def edit_cases():
    c = []

    def add(name, steps):
        assert name not in [x["name"] for x in c], name
        c.append({"name": name, "steps": steps})
    first = step("execute", distance_in(5, 3, 1.5))
    add("kare kolonun solu batı duvarına 1,50 m: kolon kayar, duvar yerinde; tek adım", [
        first, {"op": "undo", "returns": STEP, "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed",
                                                          "entities": {"5": BY_ID[5]}}}])
    for name, inp in [
        ("kare kolonun altı güney duvarına 1,00 m", distance_in(5, 0, 1.0)),
        ("kare kolonun sağı doğu duvarına 2,00 m (sola kayar)", distance_in(5, 1, 2.0)),
        ("kare kolonun üstü kuzey duvarına 0,50 m", distance_in(5, 2, 0.5)),
        ("derin kolonun üstü kuzey duvarına 0,60 m", distance_in(6, 2, 0.6)),
        ("yuvarlak kolonun altı güney duvarına 0,80 m", distance_in(7, 0, 0.8)),
        ("duvara dayalı kolonu doğu duvarından 0,25 m ayır", distance_in(8, 1, 0.25)),
        ("30° dönük kolon yanındakine 0,80 m: kendi dönüklüğünde kayar", distance_in(10, 1, 0.8)),
        ("1. kattaki kolon kendi katının duvarına 1,00 m", distance_in(19, 3, 1.0)),
        ("sola hizalı duvara 0,50 m", distance_in(24, 0, 0.5)),
        ("mesafe sıfır: kolon duvara dayanır", distance_in(5, 3, 0.0)),
    ]:
        add(name, [step("execute", inp)])
    add("plan kolonu yazmadan gösterir", [step("plan", distance_in(5, 3, 1.5))])
    add("doğrula yazmaz", [step("validate", distance_in(5, 3, 1.5))])
    for name, inp, nf in [
        ("akslara bağlı kolon reddedilir", distance_in(15, 0, 1.0), None),
        ("iki kolon birden reddedilir", {"uids": [u(5), u(6)], "operation": {"kind": "distance", "side": 0, "distance": 1.0}}, None),
        ("aks kaydırılmaz", distance_in(14, 0, 1.0), None),
        ("duvar mimari nesne değil: reddedilir", distance_in(1, 0, 1.0), None),
        ("olmayan yan reddedilir", distance_in(5, 4, 1.0), None),
        ("paralel yüzü olmayan yan reddedilir", distance_in(12, 0, 1.0), None),
        ("eksi mesafe reddedilir", distance_in(5, 0, -1.0), None),
        ("sonsuz mesafe reddedilir", distance_in(5, 0, 1.0), {"operation.distance": "NaN"}),
        ("mesafe zaten bu: yazılmaz", distance_in(5, 0, 1.7), None),
        ("kilitli katmandaki kolon kaydırılmaz", distance_in(22, 1, 1.0), None),
    ]:
        add(name, [step("execute", inp, nf)])
    return c


NOTE = "ADR 0712. Beklenenler scripts/fixtures/column_shift_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan."


def document(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": SETUP, "cases": cases}


def files():
    return {
        "cad.columns.distances.json": document(
            "cad.columns.distances", "Kolonun mesafeleri: dört yanda en yakın paralel duvar ya da kolon yüzüne net mesafe; dönük ve yuvarlak kolon, kat, akslar; retler",
            distances_cases()),
        "cad.arch.edit.distance.json": document(
            "cad.arch.edit", "Kolonu mesafeyle kaydır: yanına dik, karşı yüz sabit; akslara bağlı, kilitli ve paralel yüzsüz retler", edit_cases()),
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
                print(f"{name}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
