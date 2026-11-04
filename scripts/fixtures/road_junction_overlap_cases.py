#!/usr/bin/env python3
"""The shared cases of `road.junctions.overlap` (plan/road.md Y17.6, docs/adr/0919; ROAD-JUNC-05).

The drawing is road.junctions.query.json's with one more corridor across the junction. A corridor's top surface is
road_corridor_command_cases.py's reference (its triangles), the terrain road_profile_command_cases.py's TIN, a place's
station the alignment reference's foot. The overlap is ADR 0919 §2's: the two surfaces sampled at the centres of a
`cell` grid in world coordinates over their common box, rows by y then x; a centre on both is overlap; the secondary
corridor's cut and fill against its terrain there, cell by cell. No KentOS code. The cells are the cases' own.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj python3 \\
        scripts/fixtures/road_junction_overlap_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_command_cases as acmd  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
JUNCTIONS = json.loads((OUT / "road.junctions.query.json").read_text())
W = 1e-6
ANY = "$any"
MOST = 1_000_000


def completed(output, warns=()):
    return {"status": "completed", "output": output, "warnings": list(warns)}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path:
        e["path"] = path
    return {"status": "failed", "error": e}


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
NOT_ROAD = failed("not_a_road_project", "Kavşak yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")

C5 = {**copy.deepcopy(ccmd.C2), "id": "c-5", "name": "Kavşak koridoru", "alignmentRevision": 3,
      "regions": [{"from": 350.0, "to": 520.0, "template": "tk-2"}]}
C5.pop("superelevation")
C6 = {**copy.deepcopy(C5), "id": "c-6", "name": "Uzak koridor", "regions": [{"from": 30.0, "to": 120.0, "template": "tk-2"}]}


def setup(work="road"):
    s = copy.deepcopy(JUNCTIONS["setup"])
    s["settings"]["cadWork"] = work
    if work != "road":
        del s["settings"]["road"]
        return s
    s["settings"]["road"]["corridors"] += [copy.deepcopy(C5), copy.deepcopy(C6)]
    return s


ROAD = setup()["settings"]["road"]
ALIGNMENTS = ROAD["alignments"]
CORRIDORS = ROAD["corridors"]
JUNCTION_LIST = ROAD["junctions"]
REPORTS = {j["id"]: j for j in JUNCTIONS["cases"][0]["steps"][0]["result"]["output"]["junctions"]}


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(words, path):
    return Refused("invalid_overlap", words, path)


def find(list_, name, path, code, what):
    for x in list_:
        if x["id"] == name:
            return x
    for x in list_:
        if x["name"].lower() == name.lower():
            return x
    raise Refused(code, f"“{name}” kimlikli ya da adlı {what} projede yok.", path)


class Located:
    """Triangles found by plan place; the first (in their order) holding a place."""

    def __init__(self, tris):
        self.tris = tris
        self.cell = 5.0
        self.grid = {}
        for i, t in enumerate(tris):
            xs, ys = [p[0] for p in t], [p[1] for p in t]
            for gx in range(math.floor(min(xs) / self.cell), math.floor(max(xs) / self.cell) + 1):
                for gy in range(math.floor(min(ys) / self.cell), math.floor(max(ys) / self.cell) + 1):
                    self.grid.setdefault((gx, gy), []).append(i)
        xs = [p[0] for t in tris for p in t]
        ys = [p[1] for t in tris for p in t]
        self.bounds = ((min(xs), min(ys)), (max(xs), max(ys))) if tris else None

    def z(self, x, y):
        for i in self.grid.get((math.floor(x / self.cell), math.floor(y / self.cell)), []):
            a, b, q = self.tris[i]
            d = (b[0] - a[0]) * (q[1] - a[1]) - (q[0] - a[0]) * (b[1] - a[1])
            if abs(d) < 1e-18:
                continue
            u = ((b[0] - x) * (q[1] - y) - (q[0] - x) * (b[1] - y)) / d
            v = ((q[0] - x) * (a[1] - y) - (a[0] - x) * (q[1] - y)) / d
            w = 1 - u - v
            if u >= -1e-12 and v >= -1e-12 and w >= -1e-12:
                return u * a[2] + v * b[2] + w * q[2]
        return None


def grid(a, b, cell):
    lo = (max(a.bounds[0][0], b.bounds[0][0]), max(a.bounds[0][1], b.bounds[0][1]))
    hi = (min(a.bounds[1][0], b.bounds[1][0]), min(a.bounds[1][1], b.bounds[1][1]))
    if lo[0] > hi[0] or lo[1] > hi[1]:
        return None
    first = (math.ceil(lo[0] / cell - 0.5), math.ceil(lo[1] / cell - 0.5))
    last = (math.floor(hi[0] / cell - 0.5), math.floor(hi[1] / cell - 0.5))
    if last[0] < first[0] or last[1] < first[1]:
        return None
    return first, (last[0] - first[0] + 1, last[1] - first[1] + 1)


TRIANGLES = {}


def triangles(c):
    if c["id"] not in TRIANGLES:
        try:
            TRIANGLES[c["id"]] = ccmd.make(c)[2]
        except ccmd.Refused as r:
            TRIANGLES[c["id"]] = r
    got = TRIANGLES[c["id"]]
    if isinstance(got, ccmd.Refused):
        raise Refused(got.result["error"]["code"], got.result["error"]["message"], got.result["error"].get("path"))
    return got


def answer(inp, road=True):
    if not road:
        raise Refused(NOT_ROAD["error"]["code"], NOT_ROAD["error"]["message"])
    j = find(JUNCTION_LIST, inp["junction"], "junction", "unknown_junction", "kavşak")
    rep = REPORTS[j["id"]]
    warns = []
    a_ids = [lg["alignment"] for lg in j["legs"]]
    for lg in j["legs"]:
        a = next(x for x in ALIGNMENTS if x["id"] == lg["alignment"])
        if a["revision"] != lg["alignmentRevision"]:
            warns.append({"code": "junction_stale", "message": f"“{j['name']}” kavşağının “{a['name']}” aksı değişti; "
                          "kavşağı denetleyip yeniden bağlayın.", "path": "junction"})
    if "corridors" in inp:
        chosen = [find(CORRIDORS, n, f"corridors[{k}]", "unknown_corridor", "koridor") for k, n in enumerate(inp["corridors"])]
    else:
        chosen = []
        for aid in a_ids:
            on = [c for c in CORRIDORS if c["alignment"] == aid]
            name = next(a["name"] for a in ALIGNMENTS if a["id"] == aid)
            if not on:
                raise bad(f"“{name}” aksının koridoru yok; corridors ile verin.", "corridors")
            if len(on) > 1:
                raise bad(f"“{name}” aksının {len(on)} koridoru var; hangisi, corridors ile verin.", "corridors")
            chosen.append(on[0])
    tris = [triangles(c) for c in chosen]
    if len(chosen) != 2:
        raise bad(f"Kolların sırasıyla iki koridor verin; {len(chosen)} verildi.", "corridors")
    for k, (c, aid) in enumerate(zip(chosen, a_ids)):
        if c["alignment"] != aid:
            raise bad(f"“{c['name']}” koridoru kavşağın {k + 1}. kolunun aksında değil.", f"corridors[{k}]")
    pri = inp["primary"]
    if pri > 1:
        raise bad("Kavşağın alanının sayıldığı kol (primary) 0 ya da 1 olmalı.", "primary")
    cell = inp["cell"]
    if not cell > 0:
        raise bad("Izgara aralığı (cell) 0'dan büyük olmalı.", "cell")
    sec = 1 - pri
    if chosen[sec]["surface"] != "t-1":
        raise bad(f"“{chosen[sec]['name']}” koridorunun arazi yüzeyi kurulamıyor.", "corridors")
    for c in chosen:
        now = next(a["revision"] for a in ALIGNMENTS if a["id"] == c["alignment"])
        if now != c.get("alignmentRevision", 0):
            warns.append({"code": "corridor_stale",
                          "message": f"“{c['name']}” koridorunun aksı değişti (koridor {c.get('alignmentRevision', 0)}. "
                                     f"revizyona göre kurulu, aks şimdi {now}.); bölgeleri denetleyip aksa yeniden bağlayın.",
                          "path": "corridors"})
    A, B = Located(tris[pri]), Located(tris[sec])
    g = grid(A, B, cell) if A.bounds and B.bounds else None
    if g and g[1][0] * g[1][1] > MOST:
        raise bad(f"Aralık çok küçük: {g[1][0]} × {g[1][1]} hücre; en çok {MOST}.", "cell")
    cells = []
    if g:
        (fx, fy), (nx, ny) = g
        for r in range(ny):
            y = (fy + r) * cell + 0.5 * cell
            for c in range(nx):
                x = (fx + c) * cell + 0.5 * cell
                za, zb = A.z(x, y), B.z(x, y)
                if za is not None and zb is not None:
                    cells.append((x, y, za, zb))
    area_of = cell * cell
    cut = fill = total = 0.0
    uncovered = 0
    largest = None
    ranges = [[math.inf, -math.inf], [math.inf, -math.inf]]
    built = [pcmd.ALIGNED[a_ids[pri]], pcmd.ALIGNED[a_ids[sec]]]
    for x, y, za, zb in cells:
        z = pcmd.TIN.height(x, y)
        if z is None:
            uncovered += 1
        elif z > zb:
            cut += area_of * (z - zb)
        else:
            fill += area_of * (zb - z)
        d = za - zb
        total += abs(d)
        if largest is None or abs(d) > abs(largest["difference"]):
            largest = {"x": x, "y": y, "primary": za, "secondary": zb, "difference": d}
        for k in (0, 1):
            f = acmd.locate(built[k], (x, y))
            if "nearest" in f:
                s = f["nearest"]["at"]["station"]
                ranges[k] = [min(ranges[k][0], s), max(ranges[k][1], s)]
    n = len(cells)
    area = n * area_of
    if n:
        warns.append({"code": "junction_corridors_overlap",
                      "message": f"“{chosen[pri]['name']}” ve “{chosen[sec]['name']}” koridorları “{j['name']}” kavşağında "
                                 f"{area:.1f} m² örtüşüyor; tali koridorun buradaki {cut:.1f} m³ yarması ve {fill:.1f} m³ "
                                 "dolgusu iki kez sayılır.", "path": "corridors"})
    out = {"junction": j["id"], "primary": pri, "primaryCorridor": chosen[pri]["id"],
           "secondaryCorridor": chosen[sec]["id"], "cell": cell, "cells": n, "area": area, "cut": cut, "fill": fill,
           "uncovered": uncovered}
    if n:
        out["meanDifference"] = total / n
        ties = sum(1 for _, _, za, zb in cells if abs(abs(za - zb) - abs(largest["difference"])) < 1e-9)
        if ties > 1:
            largest = {k: (v if k == "difference" else ANY) for k, v in largest.items()}
            largest["difference"] = ANY
        out["largest"] = largest
        out["primaryRange"] = ranges[0]
        out["secondaryRange"] = ranges[1]
    return out, warns


def cases():
    out = []

    def q(name, inp, map_=False):
        try:
            o, w = answer(inp, road=not map_)
            result = completed(o, w)
        except Refused as r:
            result = r.result
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED,
                                       "within": W}]}
        if map_:
            c["setup"] = "$map"
        out.append(c)

    q("Ana yol koridoru sahibi", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 0, "cell": 1.0})
    q("Yan yol koridoru sahibi", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 1, "cell": 1.0})
    q("İri ızgara", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 0, "cell": 4.0})
    q("İnce ızgara", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 0, "cell": 0.5})
    q("Adlarıyla", {"junction": "ana kavşak", "corridors": ["Ana koridor", "Kavşak koridoru"], "primary": 0,
                    "cell": 2.0})
    q("Kavşağa varmayan yan koridor", {"junction": "j-1", "corridors": ["c-1", "c-2"], "primary": 0, "cell": 1.5})
    q("Örtüşmeyen koridorlar", {"junction": "j-1", "corridors": ["c-1", "c-6"], "primary": 0, "cell": 2.0})
    q("Katlı kavşakta da", {"junction": "j-2", "corridors": ["c-1", "c-5"], "primary": 1, "cell": 2.0})
    # Refusals.
    q("Olmayan kavşak", {"junction": "j-9", "primary": 0, "cell": 1.0})
    q("Olmayan koridor", {"junction": "j-1", "corridors": ["yok", "c-5"], "primary": 0, "cell": 1.0})
    q("Kurulamayan koridor", {"junction": "j-1", "corridors": ["c-4", "c-5"], "primary": 0, "cell": 1.0})
    q("Aksın birden çok koridoru", {"junction": "j-1", "primary": 0, "cell": 1.0})
    q("Tek koridor", {"junction": "j-1", "corridors": ["c-1"], "primary": 0, "cell": 1.0})
    q("Koridor kolun aksında değil", {"junction": "j-1", "corridors": ["c-5", "c-1"], "primary": 0, "cell": 1.0})
    q("Üçüncü kol", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 2, "cell": 1.0})
    q("Izgara sıfır", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 0, "cell": 0.0})
    q("Izgara eksi", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 0, "cell": -1.0})
    q("Çok küçük ızgara", {"junction": "j-1", "corridors": ["c-1", "c-5"], "primary": 0, "cell": 0.0625})
    q("Tali koridorun arazisi yok", {"junction": "j-1", "corridors": ["c-3", "c-5"], "primary": 1, "cell": 1.0})
    q("Yol projesi değil", {"junction": "j-1", "primary": 0, "cell": 1.0}, map_=True)
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="map")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.junctions.overlap", "commandVersion": 1,
           "title": "Kavşakta koridor örtüşmesi: alan, iki kez sayılan hacim, kırpılacak km",
           "note": "scripts/fixtures/road_junction_overlap_cases.py'den; koridorların üst yüzeyi "
                   "road_corridor_command_cases.py'nin, arazi road_profile_command_cases.py'nin TIN'i, km aks "
                   "başvurusunun ayağı, ızgara ADR 0919 §2'nin kuralıyla; KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.junctions.overlap.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.junctions.overlap.json")
    else:
        path.write_text(text)
    print(f"road.junctions.overlap.json: {len(found)} durum")


if __name__ == "__main__":
    main()
