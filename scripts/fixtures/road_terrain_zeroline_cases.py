#!/usr/bin/env python3
"""The shared cases of `road.terrains.zeroline` (docs/adr/1620; plan/map-cad.md MAP-15): where two surfaces meet.

The drawing is road_terrain_compare_cases.py's with one more lattice, a mound on the plane (z = plane + 0.4 −
0.1·(|x − 6005| + |y − 5|), corners every metre): its zero line over the plane is a closed diamond of radius 4. Every
lattice is z = g(x) + h(y), so over the rectangles between the union of two lattices' lines each is one plane and the
difference d is linear: its zero set in a rectangle is the segment between the edges it crosses (a corner at zero
counted once), a rectangle all zero is coincident area. Segments are joined at their ends (places to 10⁻⁹ m; a place
where three or more meet ends the lines), a vertex in line with its neighbours is dropped, each line starts at its
smallest place (x, then y) and the lines are ordered by it. The places themselves are the core's to compute; the
cases hold each line's closure and plan length, the total and the coincident area.

No KentOS code.

    python3 scripts/fixtures/road_terrain_zeroline_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_drainage_cases as drainage  # noqa: E402
import road_terrain_analysis_cases as analysis  # noqa: E402
import road_terrain_compare_cases as compare  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f620-7c3e-7d4a-9b1e-{:012d}"
FX = drainage.FX
NOT_SURFACE = analysis.failed("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS "
                              "Yol); bu çizim ikisinden biri değil.")
CONFLICT = drainage.CONFLICT
BAD_REVISION = drainage.BAD_REVISION
UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}


def mound_z(x, y):
    return drainage.plane_z(x, y) + 0.4 - 0.1 * (abs(x - FX - 5.0) + abs(y - 5.0))


MOUND = ("tumsek", mound_z, [FX + i for i in range(11)], [float(j) for j in range(11)])
LATTICES = {**compare.LATTICES, "t-10": MOUND}
NAMES = {**compare.NAMES, "t-10": "Tümsek"}


def setup(work="road"):
    s = compare.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for lid, name in (("tumsek", "Tümsek"), ("sifir", "Sıfır hattı")):
        s["layers"].append({"id": lid, "name": name, **copy.deepcopy(base)})
    # The zero line's layer in a colour of its own, as a drawing standard would give it.
    s["layers"][-1]["style"]["color"] = "#E5484D"
    s["layers"][-1]["style"]["lineWeight"] = 0.5
    k = max(e["id"] for e in s["entities"])
    layer, f, xs, ys = MOUND
    for y in ys:
        for x in xs:
            k += 1
            s["entities"].append({"kind": "point", "id": k, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y},
                                  "z": f(x, y)})
            s["uids"].append(UID.format(k))
    if work in ("road", "map"):
        s["settings"]["road"]["surfaces"].append({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]}})
        if work == "map":
            s["settings"]["road"] = {"surfaces": s["settings"]["road"]["surfaces"]}
    return s


def joint(p):
    return (round(p[0] * 1e9), round(p[1] * 1e9))


def zero_lines(bid, cid):
    """(lines [(points, closed)], coincident area) where the comparison meets the base."""
    _, fb, bxs, bys = LATTICES[bid]
    _, fc, cxs, cys = LATTICES[cid]
    xs = sorted({x for x in bxs + cxs if cxs[0] <= x <= cxs[-1]})
    ys = sorted({y for y in bys + cys if cys[0] <= y <= cys[-1]})
    segments, coincident = [], 0.0
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            if not (bxs[0] <= x0 and x1 <= bxs[-1] and bys[0] <= y0 and y1 <= bys[-1]):
                continue
            poly = [((x, y), fc(x, y) - fb(x, y)) for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
            # Exact zeros: the lattices' heights are sums of terms that cancel only where they should.
            poly = [(p, 0.0 if abs(d) < 1e-12 else d) for p, d in poly]
            if all(d == 0.0 for _, d in poly):
                coincident += (x1 - x0) * (y1 - y0)
                continue
            at = []
            for k in range(4):
                (p, dp), (q, dq) = poly[k], poly[(k + 1) % 4]
                if dp == 0.0:
                    at.append(p)
                if (dp < 0 < dq) or (dp > 0 > dq):
                    t = dp / (dp - dq)
                    at.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
            out = []
            for p in at:
                if not out or joint(out[-1]) != joint(p):
                    out.append(p)
            if len(out) > 1 and joint(out[0]) == joint(out[-1]):
                out.pop()
            if len(out) == 2:
                segments.append((out[0], out[1]))
    # Each segment once.
    seen, segs = set(), []
    for a, b in segments:
        ka, kb = joint(a), joint(b)
        if ka == kb or (min(ka, kb), max(ka, kb)) in seen:
            continue
        seen.add((min(ka, kb), max(ka, kb)))
        segs.append((a, b))
    at = {}
    for i, (a, b) in enumerate(segs):
        at.setdefault(joint(a), []).append(i)
        at.setdefault(joint(b), []).append(i)
    used = [False] * len(segs)

    def walk(i, here):
        pts = [here]
        while i is not None:
            used[i] = True
            a, b = segs[i]
            here = b if joint(a) == joint(here) else a
            pts.append(here)
            nxt = at[joint(here)]
            i = next((j for j in nxt if not used[j]), None) if len(nxt) == 2 else None
        return pts

    lines = []
    for e in sorted(k for k, v in at.items() if len(v) != 2):
        for i in at[e]:
            if not used[i]:
                a, b = segs[i]
                lines.append((walk(i, a if joint(a) == e else b), False))
    for i in range(len(segs)):
        if not used[i]:
            pts = walk(i, segs[i][0])
            pts.pop()
            lines.append((pts, True))
    return lines, coincident


def length(points, closed):
    n = len(points)
    run = sum(math.dist(points[k], points[k + 1]) for k in range(n - 1))
    if closed and n > 2:
        run += math.dist(points[0], points[-1])
    return run


def answer(inp):
    def find(name):
        # By id, else by name with case aside (Turkish İ/ı folded as the core folds them).
        return next(i for i, n in NAMES.items() if name == i or name.lower() == n.lower())
    bid, cid = find(inp["base"]), find(inp["comparison"])
    lines, coincident = zero_lines(bid, cid)
    # Ordered by each line's smallest place (an open line's smaller end, a closed line's least corner).
    keyed = []
    for pts, closed in lines:
        first = min(pts) if closed else min(pts[0], pts[-1])
        keyed.append((first, {"closed": closed, "length": length(pts, closed), "points": ANY}))
    keyed.sort(key=lambda kv: kv[0])
    out = [v for _, v in keyed]
    return {"base": bid, "comparison": cid, "lines": out, "length": sum(l["length"] for l in out),
            "coincident": coincident}


def cases():
    out = []
    n = len(setup()["entities"])

    def ok(name, inp, op="execute", warns=(), note=None, setup_=None):
        o = answer(inp)
        written = op == "execute"
        o["revision"] = "$current"
        if written:
            o["uids"] = [ANY] * len(o["lines"])
        result = ({"status": "completed", "output": o, "warnings": list(warns)} if op != "validate"
                  else {"status": "completed", "output": None, "warnings": list(warns)})
        expect = ({**STEPPED, "ids": list(range(1, n + len(o["lines"]) + 1))} if written
                  else {**UNCHANGED, "ids": list(range(1, n + 1))})
        c = {"name": name, **({"note": note} if note else {}),
             "steps": [{"op": op, "input": {"layerId": "sifir", **inp} if "layerId" not in inp else inp,
                        "result": result, "expect": expect, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    def no(name, inp, result, setup_=None):
        c = {"name": name, "steps": [{"op": "execute", "input": {"layerId": "sifir", **inp} if "layerId" not in inp
                                      else inp, "result": result, "expect": {**UNCHANGED}}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    ok("Tümsek düzün üstünde: kapalı çizgi", {"base": "Düz", "comparison": "Tümsek"},
       note="eşkenar dörtgen, yarıçapı 4 m: 4·4·√2")
    ok("Düz tümseğin altında: aynı çizgi, karşılaştırma ters", {"base": "Tümsek", "comparison": "Düz"})
    ok("Set terasın üstünde: iki açık çizgi", {"base": "Teras", "comparison": "Set"},
       note="fark 0,2 − 0,05·|x − 8005|: x = 8001 ve 8009 kafes çizgilerinde, iki yanın parçası da bulur")
    ok("Teras setin altında", {"base": "Set", "comparison": "Teras"})
    ok("Kimliklerle", {"base": "t-5", "comparison": "t-10"})
    ok("Adlar küçük harfle", {"base": "düz", "comparison": "tümsek"})
    ok("Plan yazmaz", {"base": "Düz", "comparison": "Tümsek"}, op="plan")
    ok("Doğrulama yazmaz", {"base": "Teras", "comparison": "Set"}, op="validate")
    ok("Başka katmana", {"base": "Düz", "comparison": "Tümsek", "layerId": "drenaj"})
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    ok("Gizli katmana uyarıyla", {"base": "Teras", "comparison": "Set", "layerId": "gizli"}, warns=[hidden])
    ok("Harita projesinde de", {"base": "Düz", "comparison": "Tümsek"}, setup_="$harita")
    # Refusals.
    none = analysis.failed("no_zero_line", "İki yüzeyin farkı hiçbir yerde işaret değiştirmiyor: sıfır hattı yok "
                           "(karşılaştırma tabanın hep altında ya da hep üstünde, ya da örtüşmüyorlar).", "comparison")
    no("Kazı düzün hep altında: sıfır hattı yok", {"base": "Düz", "comparison": "Kazı"}, none)
    no("Örtüşmeyen yüzeyler", {"base": "Vadi", "comparison": "Düz"}, none)
    no("Aynı yüzey", {"base": "Düz", "comparison": "t-5"},
       analysis.failed("invalid_analysis", "Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.", "comparison"))
    no("Olmayan taban", {"base": "Tasarım", "comparison": "Düz"},
       analysis.failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "base"))
    no("Olmayan karşılaştırma", {"base": "Düz", "comparison": "Tasarım"},
       analysis.failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "comparison"))
    no("Kurulamayan yüzey", {"base": "Düz", "comparison": "Kurulamaz"},
       {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    no("Kilitli katman", {"base": "Düz", "comparison": "Tümsek", "layerId": "kilitli"},
       analysis.failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir "
                       "katmanı etkinleştirin.", "layerId"))
    no("Katman grubu", {"base": "Düz", "comparison": "Tümsek", "layerId": "grup"},
       {"status": "failed", "error": {"code": "not_a_layer", "message": ANY, "path": "layerId"}})
    no("Katman yok", {"base": "Düz", "comparison": "Tümsek", "layerId": "yok"},
       analysis.failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                       "layerId"))
    no("Geçersiz beklenen sürüm", {"base": "Düz", "comparison": "Tümsek", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"base": "Düz", "comparison": "Tümsek", "expectedRevision": "41"}, CONFLICT)
    no("Mimari proje", {"base": "Düz", "comparison": "Tümsek"}, NOT_SURFACE, setup_="$mimari")
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$mimari":
            c["setup"] = setup(work="architecture")
        elif c.get("setup") == "$harita":
            c["setup"] = setup(work="map")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.zeroline", "commandVersion": 1,
           "title": "Sıfır hattı: iki yüzeyin kazı-dolgu sınırı",
           "note": "scripts/fixtures/road_terrain_zeroline_cases.py'den; kafeslerin dikdörtgenlerinde fark doğrusal, sıfır "
                   "kümesi kenarlar arasında parça, ADR 1620'nin birleştirme kuralıyla, KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.zeroline.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.zeroline.json")
    else:
        path.write_text(text)
    print(f"road.terrains.zeroline.json: {len(found)} durum")


if __name__ == "__main__":
    main()
