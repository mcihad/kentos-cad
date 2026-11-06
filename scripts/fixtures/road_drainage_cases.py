#!/usr/bin/env python3
"""The shared cases of `road.drainage.query` and `.draw` (plan/road.md Y16, docs/adr/0929; surface drainage).

The drawing is road_culvert_cases.py's (its corridors and culverts; a culvert's inlet is that reference's own answer in
road.culverts.query.json) with three lattice terrains of this script, each square of a lattice planar so the answers
do not depend on how a square is cut into triangles:

- a V valley z = 100 + 0.05 x′ + 0.2 |y − 500| over [2000, 3000] × [0, 1000] (x′ = x − 2000, every 100 m): a drop from
  (x₀, y₀) heads (−0.05, ∓0.2), so it reaches the valley y = 500 after |y₀ − 500| / 4 in x when 4 x′₀ ≥ |y₀ − 500| and runs
  along it to (2000, 500), else it leaves by the west edge; its length is the straight part, √17 / 4 · |y₀ − 500|, plus
  what is left of x′ (or x′₀ √17 to the edge). A catchment's cells are those whose drop passes within the radius of the
  discharge point; its flow path is the longest such drop, its fall the heights' difference.
- a diamond pit z = 50 + |x′| + |y| over [3995, 4005] × [−5, 5] (x′ = x − 4000, every metre): the water stands to the
  rim's lowest corners at 55, over the diamond |x′| + |y| < 5: depth 5, area 50 m², volume 50 · 5 / 3 m³, 41 corners
  under it; a drop from a quadrant runs down its diagonal to an axis and along it to the bottom.
- a plane z = 120 + 0.1 x′ + 0.02 y over [6000, 6010] × [0, 10] (x′ = x − 6000): a drop leaves by the west edge.

Manning's full flow and the Rational method are the ADR's formulas; the culvert's sizes, slope and roughness are
road_culvert_cases.py's. No KentOS code; C, i, the cells and the radii are the cases' own.

    python3 scripts/fixtures/road_drainage_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f5c0-7c3e-7d4a-9b1e-{:012d}"


def completed(output, warns=()):
    return {"status": "completed", "output": output, "warnings": warns if warns == ANY else list(warns)}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path:
        e["path"] = path
    return {"status": "failed", "error": e}


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
    "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}
BAD_REVISION = failed("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
NOT_ROAD = failed("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu "
                  "çizim ikisinden biri değil.")


def step(op, inp, result, expect):
    return {"op": op, "input": inp, "result": result, "expect": expect, "within": W}


# ── The terrains ─────────────────────────────────────────────────────────────────────────────────────────────────
VX = 2000.0


def valley_z(x, y):
    return 100.0 + 0.05 * (x - VX) + 0.2 * abs(y - 500.0)


PX = 4000.0


def pit_z(x, y):
    return 50.0 + abs(x - PX) + abs(y)


FX = 6000.0


def plane_z(x, y):
    return 120.0 + 0.1 * (x - FX) + 0.02 * y


LATTICES = [("vadi", valley_z, [VX + 100.0 * i for i in range(11)], [100.0 * j for j in range(11)]),
            ("cukur", pit_z, [PX + i for i in range(-5, 6)], [float(j) for j in range(-5, 6)]),
            ("duz", plane_z, [FX + i for i in range(11)], [float(j) for j in range(11)])]


def setup(work="road"):
    cases = json.loads((OUT / "road.culverts.edit.json").read_text())
    s = copy.deepcopy(cases["setup"])
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for lid, name in (("vadi", "Vadi"), ("cukur", "Çukur"), ("duz", "Düz"), ("drenaj", "Drenaj")):
        s["layers"].append({"id": lid, "name": name, **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])
    for layer, f, xs, ys in LATTICES:
        for y in ys:
            for x in xs:
                k += 1
                s["entities"].append({"kind": "point", "id": k, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y},
                                      "z": f(x, y)})
                s["uids"].append(UID.format(k))
    s["settings"]["cadWork"] = work
    if work in ("road", "map"):
        s["settings"]["road"]["surfaces"] += [{"id": "t-3", "name": "Vadi", "points": {"layers": ["vadi"]}},
                                              {"id": "t-4", "name": "Çukur", "points": {"layers": ["cukur"]}},
                                              {"id": "t-5", "name": "Düz", "points": {"layers": ["duz"]}}]
        if work == "map":
            # A map project keeps its surfaces (and pads) only, nothing else of a road (docs/adr/1520, 1580).
            s["settings"]["road"] = {"surfaces": s["settings"]["road"]["surfaces"]}
    else:
        s["settings"].pop("road", None)
    return s


# ── The answers in closed form ───────────────────────────────────────────────────────────────────────────────────
def valley_drop(x, y):
    """(the path's corners in plan, its end point, length, end) of a drop on the valley."""
    xr, dy = x - VX, abs(y - 500.0)
    if 4.0 * xr >= dy:
        hit = (x - dy / 4.0, 500.0)
        return [(x, y), hit, (VX, 500.0)], (VX, 500.0), dy * math.sqrt(17.0) / 4.0 + (xr - dy / 4.0), "edge"
    end = (VX, y - 4.0 * xr * math.copysign(1.0, y - 500.0))
    return [(x, y), end], end, xr * math.sqrt(17.0), "edge"


def pit_drop(x, y):
    """A drop's path corners on the pit: down a quadrant's diagonal to an axis, along it to the bottom."""
    xr, yr = x - PX, y
    if xr == 0.0 or yr == 0.0:
        return [(x, y), (PX, 0.0)]
    if abs(xr) < abs(yr):
        hit = (PX, yr - math.copysign(abs(xr), yr))
    else:
        hit = (PX + xr - math.copysign(abs(yr), xr), 0.0)
    return [(x, y), hit, (PX, 0.0)]


def pit_catchment_empty(at, cell, radius):
    for i in range(math.floor((PX - 5.0) / cell), math.floor((PX + 5.0) / cell) + 1):
        for j in range(math.floor(-5.0 / cell), math.floor(5.0 / cell) + 1):
            x, y = (i + 0.5) * cell, (j + 0.5) * cell
            if PX - 5.0 <= x <= PX + 5.0 and -5.0 <= y <= 5.0 and dist_to_path(pit_drop(x, y), at) <= radius:
                return False
    return True


def dist_to_path(path, p):
    best = math.inf
    for a, b in zip(path, path[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        l2 = dx * dx + dy * dy
        t = 0.0 if l2 == 0 else min(max(((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2, 0.0), 1.0)
        best = min(best, math.hypot(a[0] + t * dx - p[0], a[1] + t * dy - p[1]))
    return best


def valley_catchment(at, cell, radius):
    """(cells, the flow path's length, fall) of a valley catchment; None when no cell's drop passes."""
    cells, longest = 0, None
    for i in range(math.floor(VX / cell), math.floor((VX + 1000.0) / cell) + 1):
        for j in range(0, math.floor(1000.0 / cell) + 1):
            x, y = (i + 0.5) * cell, (j + 0.5) * cell
            if not (VX <= x <= VX + 1000.0 and 0.0 <= y <= 1000.0):
                continue
            path, end, length, _ = valley_drop(x, y)
            if dist_to_path(path, at) > radius:
                continue
            cells += 1
            # The drop cut where it comes nearest the point: the point itself on the valley, the end at the edge.
            if longest is None or length > longest[0]:
                longest = (length, valley_z(x, y) - valley_z(*end))
    if cells == 0:
        return None
    return cells, longest[0], longest[1]


def manning(area, perimeter, slope, n):
    v = (area / perimeter) ** (2.0 / 3.0) * math.sqrt(slope) / n
    return v * area, v


# m-1: a box of two 2 × 1.5 m cells at 1 % with n = 0.013 (road_culvert_cases.py's M1).
M1_Q, M1_V = manning(3.0, 7.0, 0.01, 0.013)
M1_Q *= 2


def catchment_out(at, cell, radius, c=None, i=None, culvert=None):
    got = valley_catchment(at, cell, radius)
    cells, length, fall = got
    area = cells * cell * cell
    out = {"at": at, "cells": cells, "cell": cell, "area": area, "outline": ANY, "path": ANY, "length": length,
           "fall": fall, "slope": fall / length}
    warns = []
    if c is not None:
        out["peak"] = c * i * (area / 1e6) / 3.6
    if culvert == "m-1":
        cap = {"culvert": "m-1", "discharge": M1_Q, "velocity": M1_V}
        if "peak" in out:
            cap["ratio"] = out["peak"] / M1_Q
            if cap["ratio"] > 1.0:
                warns.append({"code": "culvert_over_capacity", "message": f"Havzanın en yüksek debisi "
                              f"{out['peak']:.3f} m³/s, “Km 1+380 menfezi” menfezinin dolu akış kapasitesi "
                              f"{M1_Q:.3f} m³/s.", "path": "catchment.culvert"})
        out["capacity"] = cap
    if culvert == "m-2":
        warns.append({"code": "culvert_no_roughness", "message": "“Yan menfez” menfezinin pürüzlülüğü yok; kapasitesi "
                      "verilmedi.", "path": "catchment.culvert"})
    return out, warns


def drop_out(frm, path, to, length, end):
    return {"from": frm, "path": path, "to": to, "length": length, "end": end}


def valley_drops(points):
    out = []
    for p in points:
        _, end, length, kind = valley_drop(*p)
        out.append(drop_out(list(p), ANY, [end[0], end[1], valley_z(*end)], length, kind))
    return out


PIT_POND = {"bottom": [PX, 0.0, 50.0], "level": 55.0, "outlet": ANY, "depth": 5.0, "area": 50.0, "volume": 250.0 / 3.0,
            "corners": 41}


def cases():
    out = []
    q = lambda name, inp, result, setup_=None: out.append(  # noqa: E731
        {"name": name, "steps": [step("execute", inp, result, UNCHANGED)], **({"setup": setup_} if setup_ else {})})
    starts = [(2730.0, 820.0), (2130.0, 180.0), (2025.0, 975.0)]
    q("Vadide üç su damlası", {"surface": "Vadi", "drops": [list(p) for p in starts]},
      completed({"surface": "t-3", "drops": valley_drops(starts)}))
    pl = (6005.3, 5.1)
    yend = 5.1 - 5.3 * 0.2
    q("Düzlemde damla kenara, dışarıdaki damla yok", {"surface": "t-5", "drops": [list(pl), [0.0, -5000.0]]},
      completed({"surface": "t-5", "drops": [
          drop_out(list(pl), ANY, [FX, yend, plane_z(FX, yend)], 5.3 * math.sqrt(1.04), "edge"),
          drop_out([0.0, -5000.0], [], None, 0.0, "outside")]}))
    out[-1]["steps"][0]["result"]["output"]["drops"][1].pop("to")
    q("Çukurda damla dibe", {"surface": "Çukur", "drops": [[PX + 2.3, 1.4]]},
      completed({"surface": "t-4", "drops": [drop_out([PX + 2.3, 1.4], ANY, [PX, 0.0, 50.0], 1.4 * math.sqrt(2.0) + 0.9,
                                                    "lowPoint")]}))
    q("Çukurun göleti", {"surface": "Çukur", "ponds": {"minDepth": 0.0}},
      completed({"surface": "t-4", "ponds": [PIT_POND]}))
    q("Göletten derin istenince yok", {"surface": "Çukur", "ponds": {"minDepth": 6.0}}, completed({"surface": "t-4"}))
    q("Vadide gölet yok", {"surface": "Vadi", "ponds": {"minDepth": 0.0}}, completed({"surface": "t-3"}))
    at = [VX, 500.0]
    cat, w = catchment_out(at, 50.0, 30.0)
    q("Vadinin çıkışının havzası", {"surface": "Vadi", "catchment": {"at": at, "cell": 50.0, "radius": 30.0}},
      completed({"surface": "t-3", "catchment": cat}, w))
    cat, w = catchment_out(at, 50.0, 30.0, 0.9, 200.0)
    q("Rasyonel debiyle", {"surface": "Vadi", "catchment": {"at": at, "cell": 50.0, "radius": 30.0, "runoff": 0.9,
                                                           "intensity": 200.0}},
      completed({"surface": "t-3", "catchment": cat}, w))
    cat, w = catchment_out(at, 50.0, 30.0, 0.9, 200.0, "m-1")
    q("Menfezin kapasitesi aşılıyor", {"surface": "Vadi", "catchment": {
        "at": at, "cell": 50.0, "radius": 30.0, "runoff": 0.9, "intensity": 200.0, "culvert": "m-1"}},
      completed({"surface": "t-3", "catchment": cat}, w))
    cat, w = catchment_out(at, 50.0, 30.0, 0.5, 50.0, "m-1")
    q("Menfez yetiyor", {"surface": "Vadi", "catchment": {
        "at": at, "cell": 50.0, "radius": 30.0, "runoff": 0.5, "intensity": 50.0, "culvert": "Km 1+380 menfezi"}},
      completed({"surface": "t-3", "catchment": cat}, w))
    cat, w = catchment_out(at, 50.0, 30.0, 0.5, 50.0, "m-2")
    q("Pürüzlülüğü olmayan menfez", {"surface": "Vadi", "catchment": {
        "at": at, "cell": 50.0, "radius": 30.0, "runoff": 0.5, "intensity": 50.0, "culvert": "m-2"}},
      completed({"surface": "t-3", "catchment": cat}, w))
    cat, w = catchment_out(at, 100.0, 60.0)
    q("İri ızgara", {"surface": "Vadi", "catchment": {"at": at, "cell": 100.0, "radius": 60.0}},
      completed({"surface": "t-3", "catchment": cat}, w))
    empty_at = [2500.0, 950.0]
    assert valley_catchment(empty_at, 50.0, 1.0) is None
    q("Havzası olmayan nokta", {"surface": "Vadi", "catchment": {"at": empty_at, "cell": 50.0, "radius": 1.0}},
      completed({"surface": "t-3"}, [{"code": "drainage_catchment_empty", "message": "Hiçbir hücrenin suyu (2500, "
                                      "950) noktasının 1 m eriminden geçmiyor; havza yok. Noktayı bir su damlasının "
                                      "yolu üstünde seçin ya da erimi büyütün.", "path": "catchment.at"}]))
    q("Menfezin girişinin havzası", {"surface": "Mevcut zemin", "catchment": {
        "culvert": "m-1", "cell": 20.0, "radius": 30.0, "runoff": 0.3, "intensity": 10.0}},
      completed({"surface": "t-1", "catchment": ANY}, ANY))
    bad = lambda name, inp, msg, path: q(name, inp, failed("invalid_drainage", msg, path))  # noqa: E731
    bad("Hiçbir şey sorulmamış", {"surface": "Vadi"}, "Bir şey sorun: ponds, drops ya da catchment.", None)
    bad("Çok damla", {"surface": "Vadi", "drops": [[2500.0, 500.0]] * 1001},
        "Bir soruda en çok 1000 su damlası okunur.", "drops")
    bad("Eksi derinlik", {"surface": "Vadi", "ponds": {"minDepth": -1.0}},
        "Göletin en az derinliği eksi olmayan bir sayı olmalı.", "ponds.minDepth")
    bad("Sıfır aralık", {"surface": "Vadi", "catchment": {"at": at, "cell": 0.0, "radius": 30.0}},
        "Havzanın ızgara aralığı sıfırdan büyük olmalı.", "catchment.cell")
    bad("Sıfır erim", {"surface": "Vadi", "catchment": {"at": at, "cell": 50.0, "radius": 0.0}},
        "Boşaltım noktasının erimi sıfırdan büyük olmalı.", "catchment.radius")
    bad("Boşaltım noktası yok", {"surface": "Vadi", "catchment": {"cell": 50.0, "radius": 30.0}},
        "Boşaltım noktasını verin: at ya da bir menfez (culvert).", "catchment.at")
    bad("Katsayısız şiddet", {"surface": "Vadi", "catchment": {"at": at, "cell": 50.0, "radius": 30.0,
                                                             "intensity": 80.0}},
        "Akış katsayısı (runoff) ile yağış şiddeti (intensity) birlikte verilir.", "catchment.runoff")
    bad("Birden büyük katsayı", {"surface": "Vadi", "catchment": {"at": at, "cell": 50.0, "radius": 30.0,
                                                                "runoff": 1.5, "intensity": 80.0}},
        "Akış katsayısı 0 ile 1 arası olmalı.", "catchment.runoff")
    bad("Sıfır şiddet", {"surface": "Vadi", "catchment": {"at": at, "cell": 50.0, "radius": 30.0, "runoff": 0.5,
                                                        "intensity": 0.0}},
        "Yağış şiddeti sıfırdan büyük olmalı.", "catchment.intensity")
    bad("Izgara çok büyük", {"surface": "Vadi", "catchment": {"at": at, "cell": 0.5, "radius": 30.0}},
        "Havzanın ızgarası 1000000 hücreyi aşar; aralığı büyütün.", "catchment.cell")
    q("Yüzey yok", {"surface": "Tasarım", "drops": [[0.0, 0.0]]},
      failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "surface"))
    q("Menfez yok", {"surface": "Vadi", "catchment": {"culvert": "m-9", "cell": 50.0, "radius": 30.0}},
      failed("unknown_culvert", "“m-9” kimlikli ya da adlı menfez projede yok.", "catchment.culvert"))
    q("Kurulamayan yüzey", {"surface": "Kurulamaz", "drops": [[0.0, 0.0]]},
      failed("terrain_unbuildable", ANY))
    out[-1]["steps"][0]["result"]["error"] = {"code": "terrain_unbuildable", "message": ANY}
    q("Yol projesi değil", {"surface": "Vadi", "drops": [[0.0, 0.0]]}, NOT_ROAD, setup_="$map")
    # A map project's surface drains as a road project's (docs/adr/1580).
    out.append({**copy.deepcopy(out[0]), "name": "Harita projesinde de su damlası", "setup": "$harita"})
    return out


def draw_cases():
    out = []
    at = [VX, 500.0]
    vq = {"surface": "Vadi", "catchment": {"at": at, "cell": 50.0, "radius": 30.0, "runoff": 0.9, "intensity": 200.0}}
    pq = {"surface": "Çukur", "ponds": {"minDepth": 0.0}}

    def ok(name, inp, n, warns=(), plan=False):
        result = completed({"uids": [ANY] * n, "revision": "$current"}, warns)
        out.append({"name": name, "steps": [step("plan" if plan else "execute", inp, result,
                                                 UNCHANGED if plan else STEPPED)]})

    def no(name, inp, result, setup_=None):
        c = {"name": name, "steps": [step("execute", inp, result, UNCHANGED)]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    ok("Havza yazısıyla", {"query": vq, "layerId": "drenaj", "labels": True}, 3)
    ok("Havza yazısız", {"query": vq, "layerId": "drenaj"}, 2)
    ok("Göletler yazılarıyla", {"query": pq, "layerId": "drenaj", "labels": True}, 2)
    ok("Göletler yazısız", {"query": pq, "layerId": "drenaj"}, 1)
    ok("Planı yazmaz", {"query": vq, "layerId": "drenaj", "labels": True}, 3, plan=True)
    _, w = catchment_out(at, 50.0, 30.0, 0.9, 200.0, "m-1")
    ok("Kapasite uyarısıyla", {"query": {**vq, "catchment": {**vq["catchment"], "culvert": "m-1"}},
                               "layerId": "drenaj", "labels": True}, 3, w)
    _, w = catchment_out(at, 50.0, 30.0, 0.9, 200.0, "m-2")
    ok("Pürüzlülük uyarısıyla", {"query": {**vq, "catchment": {**vq["catchment"], "culvert": "m-2"}},
                                 "layerId": "drenaj"}, 2, w)
    ok("Damlalar çizilmez, göletler çizilir", {"query": {**pq, "drops": [[PX + 2.3, 1.4]]}, "layerId": "drenaj"}, 1)
    ok("Başka katmana", {"query": pq, "layerId": "korkuluk", "labels": True}, 2)
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    ok("Gizli katmana uyarıyla", {"query": pq, "layerId": "gizli"}, 1, [hidden])
    far = [PX + 4.0, 0.3]
    assert pit_catchment_empty(far, 0.5, 0.01)
    ok("Göletler çizilir, havzası olmayan nokta uyarılır", {"query": {**pq, "catchment": {"at": far, "cell": 0.5,
                                                                                          "radius": 0.01}},
                                                           "layerId": "drenaj"}, 1,
       [{"code": "drainage_catchment_empty", "message": "Hiçbir hücrenin suyu (4004, 0.3) noktasının 0.01 m eriminden "
         "geçmiyor; havza yok. Noktayı bir su damlasının yolu üstünde seçin ya da erimi büyütün.",
         "path": "catchment.at"}])
    nothing = failed("drainage_nothing", "Çizilecek havza ya da gölet yok: soruya bir havza (catchment) ya da göletler "
                     "(ponds) katın.", "query")
    no("Çizilecek bir şey yok", {"query": {"surface": "Çukur", "ponds": {"minDepth": 6.0}}, "layerId": "drenaj"}, nothing)
    no("Yalnız damlalar çizilmez", {"query": {"surface": "Vadi", "drops": [[2500.0, 800.0]]}, "layerId": "drenaj"},
       nothing)
    no("Havzası olmayan nokta", {"query": {"surface": "Vadi", "catchment": {"at": [2500.0, 950.0], "cell": 50.0,
                                                                            "radius": 1.0}}, "layerId": "drenaj"},
       nothing)
    no("Kilitli katman", {"query": pq, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Katman grubu", {"query": pq, "layerId": "grup"}, ANY)
    out[-1]["steps"][0]["result"] = {"status": "failed", "error": {"code": "not_a_layer", "message": ANY,
                                                                   "path": "layerId"}}
    no("Katman yok", {"query": pq, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
              "layerId"))
    no("Sorunun reddi", {"query": {"surface": "Tasarım", "ponds": {"minDepth": 0.0}}, "layerId": "drenaj"},
       failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Kurulamayan yüzey", {"query": {"surface": "Kurulamaz", "ponds": {"minDepth": 0.0}}, "layerId": "drenaj"},
       {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    no("Menfez yok", {"query": {**vq, "catchment": {**vq["catchment"], "culvert": "m-9"}}, "layerId": "drenaj"},
       failed("unknown_culvert", "“m-9” kimlikli ya da adlı menfez projede yok.", "catchment.culvert"))
    no("Geçersiz soru", {"query": {"surface": "Vadi"}, "layerId": "drenaj"},
       failed("invalid_drainage", "Bir şey sorun: ponds, drops ya da catchment."))
    no("Geçersiz beklenen sürüm", {"query": pq, "layerId": "drenaj", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"query": pq, "layerId": "drenaj", "expectedRevision": "41"}, CONFLICT)
    no("Yol projesi değil", {"query": pq, "layerId": "drenaj"}, NOT_ROAD, setup_="$map")
    return out


def document(command, title, found):
    road = setup()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="architecture")
        elif c.get("setup") == "$harita":
            c["setup"] = setup(work="map")
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_drainage_cases.py'den; kafes arazilerde kapalı biçimle (vadi, elmas çukur, "
                    "düzlem), Manning ve Rasyonel ADR 0929'ün formülleriyle, KentOS kodu olmadan.",
            "setup": road, "cases": found}


def main():
    docs = {"road.drainage.query.json": document("road.drainage.query", "Drenajı sorgula: damla, gölet, havza, debi",
                                                 cases()),
            "road.drainage.draw.json": document("road.drainage.draw", "Drenajı çiz: havza ve göletler", draw_cases())}
    stale = []
    for name, doc in docs.items():
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if path.read_text() != text:
                stale.append(name)
        else:
            path.write_text(text)
        print(f"{name}: {len(doc['cases'])} durum")
    if stale:
        sys.exit(f"güncel değil: {stale}")


if __name__ == "__main__":
    main()
