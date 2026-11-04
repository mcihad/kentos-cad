#!/usr/bin/env python3
"""The shared cases of `road.terrains.density` and `road.terrains.sparse` (plan/road.md Y19.3, docs/adr/0920 §6, §7;
a surface's point density grid and its sparse cells drawn).

The drawing is road_terrain_coverage_cases.py's. The surfaces are lattices: their corners are the lattice's points,
their triangles cover the lattice's rectangle, so a cell centre is on the surface when it is inside the rectangle or on its edge. A
cell is the k-th multiple of the size from the world's origin, a point is the cell floor(x / size), floor(y / size)
(the cell above and to the right of an edge); a cell counts when its centre is inside the boundary's outer ring and in
no hole (even-odd), or, without a boundary, on the surface. The sparse cells' outlines are their squares' union
(road_terrain_coverage_cases.py's `outlines`). No KentOS code; the cell sizes, leasts and shares are the cases' own.

    python3 scripts/fixtures/road_terrain_density_cases.py [--check]
"""

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_coverage_cases as cov  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
MOST = 1_000_000
UNCHANGED = cov.UNCHANGED
refused = cov.quality.analysis.failed


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = refused(code, message, path)


def bad(message, path):
    return Refused("invalid_point_density", message, path)


def numbers(cell, least, share):
    if not (math.isfinite(cell) and cell > 0):
        raise bad("Hücre boyu (cell) sıfırdan büyük olmalı.", "cell")
    if least == 0:
        raise bad("En az nokta (least) 1 ya da daha çok olmalı.", "least")
    if share is not None and not (0 < share <= 1):
        raise bad("İstenen pay (share) 0'dan büyük, 1'den büyük olmayan bir oran olmalı.", "share")


def sid_of(name):
    sid = next((k for k, v in cov.NAMES.items() if name in (k, v)), None)
    if sid is None or sid not in cov.CORNERS:
        raise Refused("unknown_terrain", f"“{name}” kimlikli ya da adlı yüzey projede yok.", "surface")
    return sid


def region_of(uid):
    for key, u in cov.UIDS.items():
        if u == uid:
            if key in cov.RINGS:
                return cov.RINGS[key]
            raise Refused("invalid_coverage", f"“{uid}” sınır olamaz: düz kenarlı, tek parçalı kapalı bir çoklu çizgi ya da "
                                              "alan olmalı.", "boundary")
    raise Refused("entity_not_found", f"“{uid}” kimlikli nesne çizimde yok.", "boundary")


def grid(inp, least):
    sid = sid_of(inp["surface"])
    region = region_of(inp["boundary"]) if "boundary" in inp else None
    corners = cov.CORNERS[sid]
    c = inp["cell"]
    pts_ = region[0] if region else [(p[0], p[1]) for p in corners]
    lo = (min(p[0] for p in pts_), min(p[1] for p in pts_))
    hi = (max(p[0] for p in pts_), max(p[1] for p in pts_))
    fx, fy = math.floor(lo[0] / c), math.floor(lo[1] / c)
    cols, rows = math.floor(hi[0] / c) - fx + 1, math.floor(hi[1] / c) - fy + 1
    if cols * rows > MOST:
        raise bad(f"{fmt(c)} m hücreyle ızgara {cols * rows} hücre olur; en çok {MOST} hücre okunur. Hücreyi büyütün.", "cell")
    counts = [0] * (cols * rows)
    for x, y, _ in corners:
        i, j = math.floor(x / c) - fx, math.floor(y / c) - fy
        if 0 <= i < cols and 0 <= j < rows:
            counts[j * cols + i] += 1
    xs = [p[0] for p in corners]
    ys = [p[1] for p in corners]
    counted = []
    for k, n in enumerate(counts):
        i, j = k % cols, k // cols
        cx, cy = (fx + i) * c + c / 2, (fy + j) * c + c / 2
        if region:
            on = cov.inside(region[0], (cx, cy)) and not any(cov.inside(h, (cx, cy)) for h in region[1])
        else:
            on = min(xs) <= cx <= max(xs) and min(ys) <= cy <= max(ys)
        counted.append(n if on else None)
    return sid, (fx * c, fy * c), cols, rows, counted


def fmt(v):
    """Rust's f64 Display of the cases' sizes."""
    return repr(float(v)).removesuffix(".0") if float(v).is_integer() else repr(float(v))


def answer(inp):
    numbers(inp["cell"], inp.get("least"), inp.get("share"))
    sid, origin, cols, rows, counted = grid(inp, inp.get("least"))
    live = [n for n in counted if n is not None]
    cells, filled = len(live), sum(1 for n in live if n > 0)
    c = inp["cell"]
    share = filled / cells if cells else None
    area = cells * c * c
    points = sum(live)
    out = {"surface": sid, "cell": c, "origin": {"x": origin[0], "y": origin[1]}, "columns": cols, "rows": rows,
           "cells": cells, "filled": filled, "empty": cells - filled}
    if share is not None:
        out["share"] = share
    if "least" in inp:
        out["sparse"] = sum(1 for n in live if n < inp["least"])
    out.update(points=points, area=area)
    if area > 0:
        out["mean"] = points / area
    out["most"] = max(live, default=0)
    warns = []
    if "share" in inp:
        meets = share is not None and share >= inp["share"]
        out["meets"] = meets
        if not meets:
            warns.append({"code": "terrain_density_short",
                          "message": f"“{cov.NAMES[sid]}” yüzeyinin {cells} hücresinin {filled}'inde nokta var "
                                     f"(%{100 * (share or 0.0):.1f}); istenen %{100 * inp['share']:.1f}.", "path": "share"})
    return out, warns


def density_cases():
    out = []

    def q(name, inp, result=None, setup_=None):
        if result is None:
            try:
                o, w = answer(inp)
                result = {"status": "completed", "output": o, "warnings": w}
            except Refused as r:
                result = r.result
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    b = cov.UIDS
    q("Seyrek yüzeyin metrelik ızgarası", {"surface": "t-10", "cell": 1.0})
    q("İki metrelik hücre, ikiden az noktalı seyrek", {"surface": "t-10", "cell": 2.0, "least": 2})
    q("Payı tutmayan yüzey uyarıyla", {"surface": "t-10", "cell": 1.0, "least": 1, "share": 0.9})
    q("Yarım metrelik hücre", {"surface": "t-10", "cell": 0.5})
    q("Dört metrelik hücre payı tutar", {"surface": "t-10", "cell": 4.0, "share": 0.5})
    q("Proje sınırında", {"surface": "t-10", "cell": 1.5, "boundary": b["rect"], "least": 1})
    q("Yüzeyi aşan sınır", {"surface": "t-10", "cell": 2.5, "boundary": b["wide"], "share": 0.9})
    q("Terasın sık ızgarası payı tutar", {"surface": "t-6", "cell": 1.0, "share": 0.9})
    q("Terasın üç metrelik hücreleri", {"surface": "t-6", "cell": 3.0, "least": 9})
    q("L biçimli sınır", {"surface": "t-6", "cell": 2.0, "boundary": b["ell"], "least": 4})
    q("Delikli alan sınırı", {"surface": "t-5", "cell": 2.0, "boundary": b["holed"], "least": 4, "share": 1.0})
    q("Delikli yüzey", {"surface": "t-12", "cell": 1.0, "least": 1, "share": 0.5})
    q("Delikli yüzey, iri hücre", {"surface": "t-12", "cell": 4.5, "least": 2})
    q("Yüzeye değmeyen sınır", {"surface": "t-6", "cell": 3.0, "boundary": b["far"], "share": 0.9})
    q("Yüzey adıyla", {"surface": "Seyrek", "cell": 3.0, "least": 1})
    q("Düzlem yüzey, üç metre", {"surface": "t-5", "cell": 3.0, "least": 9, "share": 0.25})
    # Refusals.
    q("Hücre sıfır", {"surface": "t-10", "cell": 0.0})
    q("Hücre eksi", {"surface": "t-10", "cell": -1.0})
    q("En az nokta sıfır", {"surface": "t-10", "cell": 1.0, "least": 0})
    q("Pay sıfır", {"surface": "t-10", "cell": 1.0, "share": 0.0})
    q("Pay birden büyük", {"surface": "t-10", "cell": 1.0, "share": 1.5})
    q("Olmayan yüzey", {"surface": "t-99", "cell": 1.0})
    q("Olmayan sınır", {"surface": "t-10", "cell": 1.0, "boundary": "0192f600-7c3e-7d4a-9b1e-999999999999"})
    q("Açık çoklu çizgi sınır olamaz", {"surface": "t-10", "cell": 1.0, "boundary": b["open"]})
    q("Çok hücre", {"surface": "t-6", "cell": 0.001})
    q("Yol projesi değil", {"surface": "t-10", "cell": 1.0},
      refused("not_a_road_project", "Arazi yüzeyi yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
      setup_="$map")
    return out


def drawing(inp):
    numbers(inp["cell"], inp["least"], None)
    sid, origin, cols, rows, counted = grid(inp, inp["least"])
    c = inp["cell"]
    squares = []
    for k, n in enumerate(counted):
        if n is not None and n < inp["least"]:
            i, j = k % cols, k // cols
            squares.append((origin[0] + i * c, origin[0] + (i + 1) * c, origin[1] + j * c, origin[1] + (j + 1) * c))
    loops = cov.outlines(squares)
    outers = [r for r in loops if cov.shoelace(r) > 0]
    holes = [r for r in loops if cov.shoelace(r) < 0]
    objects = [{"kind": "polyline", "pts": cov.pts(r), "closed": True} for r in loops]
    total = 0.0
    for o in outers:
        own = sum(-cov.shoelace(h) for h in holes
                  if cov.inside(o, h[0]) and not any(x is not o and cov.inside(x, h[0]) and cov.shoelace(x) < cov.shoelace(o)
                                                     for x in outers))
        a = cov.shoelace(o) - own
        total += a
        if inp.get("labels"):
            m = cov.centre(o)
            objects.append({"kind": "text", "p": {"x": m[0], "y": m[1]}, "text": f"Seyrek · {a:.1f} m²".replace(".", ","),
                            "height": 2.5})
    return sid, objects, len(outers), len(holes), total


def sparse_cases():
    out = []
    base = cov.setup()
    first = max(e["id"] for e in base["entities"]) + 1
    stepped = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}

    def ok(name, inp, plan=False, undo=False):
        sid, objects, areas, holes, area = drawing(inp)
        result = {"status": "completed", "output": {"surface": sid, "areas": areas, "holes": holes, "area": area,
                                                    "uids": ["$any"] * len(objects), "revision": "$any" if plan else "$current"},
                  "warnings": []}
        if inp["layerId"] == "gizli":
            result["warnings"] = [{"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.",
                                   "path": "layerId"}]
        expect = dict(UNCHANGED) if plan or not objects else dict(stepped)
        if not plan and objects:
            expect["fields"] = {str(first + i): o for i, o in enumerate(objects)}
        steps = [{"op": "plan" if plan else "execute", "input": inp, "result": result, "expect": expect, "within": W}]
        if undo:
            steps.append({"op": "undo", "returns": "Seyrek alanları çiz",
                          "expect": {"canUndo": False, "canRedo": True, "ids": [e["id"] for e in base["entities"]]}})
        out.append({"name": name, "steps": steps})

    def no(name, inp, result=None, setup_=None):
        if result is None:
            try:
                drawing(inp)
                raise AssertionError(name)
            except Refused as r:
                result = r.result
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    b = cov.UIDS
    k = "kapsam"
    ok("Seyrek yüzeyin noktasız hücreleri", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": k}, undo=True)
    ok("Alanıyla", {"surface": "Seyrek", "cell": 1.0, "least": 1, "layerId": k, "labels": True})
    ok("İki metre, ikiden az", {"surface": "t-10", "cell": 2.0, "least": 2, "layerId": k, "labels": True})
    ok("Proje sınırında", {"surface": "t-10", "cell": 1.5, "least": 1, "boundary": b["rect"], "layerId": k, "labels": True})
    ok("Yüzeyi aşan sınırda verisiz yer de seyrek", {"surface": "t-10", "cell": 2.5, "least": 1, "boundary": b["wide"],
                                                     "layerId": k, "labels": True})
    ok("Delikli yüzey, ortası dolu", {"surface": "t-12", "cell": 1.0, "least": 1, "layerId": k, "labels": True})
    ok("Delikli yüzey, iri hücre", {"surface": "t-12", "cell": 4.5, "least": 2, "layerId": k, "labels": True})
    ok("Teras, dokuzdan az noktalı", {"surface": "t-6", "cell": 3.0, "least": 9, "layerId": k, "labels": True})
    ok("Delikli alan sınırında", {"surface": "t-5", "cell": 2.0, "least": 4, "boundary": b["holed"], "layerId": k,
                                  "labels": True})
    ok("L biçimli sınır", {"surface": "t-6", "cell": 2.0, "least": 4, "boundary": b["ell"], "layerId": k})
    ok("Planı yazmaz", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": k, "labels": True}, plan=True)
    ok("Gizli katmana uyarıyla", {"surface": "t-10", "cell": 2.0, "least": 2, "layerId": "gizli"})
    ok("Seyrek yok, hiçbir şey çizilmez", {"surface": "t-6", "cell": 1.0, "least": 1, "layerId": k})
    # Refusals.
    no("Hücre sıfır", {"surface": "t-10", "cell": 0.0, "least": 1, "layerId": k})
    no("En az nokta sıfır", {"surface": "t-10", "cell": 1.0, "least": 0, "layerId": k})
    no("Olmayan yüzey", {"surface": "yok", "cell": 1.0, "least": 1, "layerId": k})
    no("Olmayan sınır", {"surface": "t-10", "cell": 1.0, "least": 1, "boundary": "0192f600-7c3e-7d4a-9b1e-999999999999",
                         "layerId": k})
    no("Çok hücre", {"surface": "t-6", "cell": 0.001, "least": 1, "layerId": k})
    no("Açık çoklu çizgi sınır olamaz", {"surface": "t-10", "cell": 1.0, "least": 1, "boundary": b["open"], "layerId": k})
    no("Olmayan katman", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": "yok"},
       refused("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Kilitli katman", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": "kilitli"},
       refused("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
               "etkinleştirin.", "layerId"))
    no("Katman grubu", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": "grup"},
       {"status": "failed", "error": {"code": "not_a_layer", "message": "$any", "path": "layerId"}})
    no("Geçersiz beklenen sürüm", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": k, "expectedRevision": "on"},
       refused("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
               "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    no("Eski sürüme göre", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": k, "expectedRevision": "41"},
       {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra "
        "değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
        "revision": "$current"}})
    no("Yol projesi değil", {"surface": "t-10", "cell": 1.0, "least": 1, "layerId": k},
       refused("not_a_road_project", "Arazi yüzeyi yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
       setup_="$map")
    return out


def main():
    docs = {
        "road.terrains.density.json": ("road.terrains.density", "Nokta yoğunluğu: yüzeyin köşeleri kare hücrelerde",
                                       density_cases()),
        "road.terrains.sparse.json": ("road.terrains.sparse", "Seyrek alanları çiz: az noktalı hücrelerin dış çizgileri",
                                      sparse_cases()),
    }
    stale = []
    for name, (command, title, found) in docs.items():
        for c in found:
            if c.get("setup") == "$map":
                c["setup"] = cov.setup(work="map")
        doc = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
               "note": "scripts/fixtures/road_terrain_density_cases.py'den; kafes yüzeylerin köşeleri dünyanın başlangıcından "
                       "hücrenin katlarıyla sayılarak, sınırın halkalarında hücre merkezi tek-çift kuralıyla, seyrek "
                       "hücrelerin birleşiminin dış çizgileri ADR 0920'nin kuralıyla; KentOS kodu olmadan.",
               "setup": cov.setup(), "cases": found}
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if not path.exists() or path.read_text() != text:
                stale.append(name)
        else:
            path.write_text(text)
        print(f"{name}: {len(found)} durum")
    if stale:
        sys.exit(f"güncel değil: {stale}")


if __name__ == "__main__":
    main()
