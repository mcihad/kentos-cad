#!/usr/bin/env python3
"""The shared cases of `road.terrains.repair` (plan/road.md Y18.6, docs/adr/0916 §6; leaving point records out).

The drawing is road_terrain_quality_cases.py's with one more surface of this script: a plane lattice every 2 m with two
spikes, each inside one square away from its corners, and a breakline on the plane; a second surface of the same
points has the first spike already left out.

A spike inside a square of a plane: the Delaunay triangles round it are those triples of the square's corners and the
spike whose circumcircle holds none of the other four strictly (worked out here, no triangulation library); its tent
over the plane is h·A/3 for each such triangle with the spike as a corner (A its plan area, h the spike's height off
the plane). Leaving it out makes the plane: the difference (after − before) is −tent; the area that changed is the
tent's; the largest change −h at the spike. A plane lattice corner left out changes nothing. A triangulation of n
corners with h on its hull has 2n − 2 − h triangles (a breakline adds no corner of its own off the hull here).

No KentOS code.

    python3 scripts/fixtures/road_terrain_repair_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_drainage_cases as drainage  # noqa: E402
import road_terrain_analysis_cases as analysis  # noqa: E402
import road_terrain_quality_cases as quality  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f610-7c3e-7d4a-9b1e-{:012d}"
UNCHANGED = analysis.UNCHANGED
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
OX = 12000.0


def plane(x, y):
    return 80.0 + 0.05 * (x - OX) - 0.02 * y


LATTICE = [(OX + 2.0 * i, 2.0 * j) for j in range(6) for i in range(6)]
SPIKES = {"s1": (OX + 0.7, 1.3, 1.5), "s2": (OX + 4.6, 6.9, -0.8)}
BREAK = [(OX + 6.3, 0.5), (OX + 9.1, 4.2)]


def setup(work="road"):
    s = quality.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for lid, name in (("onarim", "Onarım"), ("onarim-kirik", "Onarım kırık hattı")):
        s["layers"].append({"id": lid, "name": name, **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])

    def add(e):
        nonlocal k
        k += 1
        s["entities"].append({**e, "id": k})
        s["uids"].append(UID.format(k))

    for x, y in LATTICE:
        add({"kind": "point", "layerId": "onarim", "attrs": {}, "p": {"x": x, "y": y}, "z": plane(x, y)})
    for x, y, h in SPIKES.values():
        add({"kind": "point", "layerId": "onarim", "attrs": {}, "p": {"x": x, "y": y}, "z": plane(x, y) + h})
    add({"kind": "polyline", "layerId": "onarim-kirik", "attrs": {}, "pts": [{"x": x, "y": y} for x, y in BREAK],
         "bulges": [0.0, 0.0], "zs": [plane(x, y) for x, y in BREAK]})
    if work == "road":
        x1, y1, _ = SPIKES["s1"]
        s["settings"]["road"]["surfaces"] += [
            {"id": "t-12", "name": "Onarım", "points": {"layers": ["onarim"]}, "breaklines": {"layers": ["onarim-kirik"]}},
            {"id": "t-13", "name": "Onarılmış", "points": {"layers": ["onarim"]},
             "breaklines": {"layers": ["onarim-kirik"]}, "excluded": [{"x": x1, "y": y1}]}]
    return s


def circle_holds(a, b, c, p):
    """Whether p is strictly inside the circumcircle of the counter-clockwise or clockwise triangle abc."""
    m = [[a[0] - p[0], a[1] - p[1], (a[0] - p[0]) ** 2 + (a[1] - p[1]) ** 2],
         [b[0] - p[0], b[1] - p[1], (b[0] - p[0]) ** 2 + (b[1] - p[1]) ** 2],
         [c[0] - p[0], c[1] - p[1], (c[0] - p[0]) ** 2 + (c[1] - p[1]) ** 2]]
    det = (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
           + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))
    orient = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    return det * orient > 0


def tent(name):
    """The plan area of the spike's triangles and the tent's volume."""
    x, y, h = SPIKES[name]
    i, j = math.floor((x - OX) / 2.0), math.floor(y / 2.0)
    corners = [(OX + 2 * i, 2 * j), (OX + 2 * i + 2, 2 * j), (OX + 2 * i + 2, 2 * j + 2), (OX + 2 * i, 2 * j + 2)]
    pts = corners + [(x, y)]
    star = 0.0
    for a in range(4):
        for b in range(a + 1, 4):
            t = (corners[a], corners[b], (x, y))
            others = [p for p in pts if p not in t]
            area = abs((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0])) / 2
            if area > 0 and not any(circle_holds(*t, p) for p in others):
                star += area
    return star, h * star / 3.0


def counts(excluded):
    corners = len(LATTICE) + len([n for n in SPIKES if n not in excluded]) + len(BREAK)
    excluded_nodes = [n for n in excluded if n not in SPIKES]
    corners -= len(excluded_nodes)
    hull = 20 - len([n for n in excluded_nodes if n in HULL])
    return corners, 2 * corners - 2 - hull


HULL: set = set()  # no hull corner is left out here


def answer(names, restore=(), already=()):
    """The output of leaving `names` out (and taking `restore` back) of a surface with `already` left out."""
    before = set(already)
    after = (before | set(names)) - set(restore)
    cut = fill = area = 0.0
    largest = None
    for n in sorted(after ^ before):
        if n not in SPIKES:
            continue
        star, vol = tent(n)
        x, y, h = SPIKES[n]
        sign = -1.0 if n in after else 1.0  # left out: after − before = −tent; taken back: +tent
        signed = sign * vol
        cut += max(-signed, 0.0)
        fill += max(signed, 0.0)
        area += star
        if largest is None or abs(sign * h) > abs(largest["difference"]):
            largest = {"difference": sign * h, "at": [x, y]}
    cb, tb = counts(before)
    ca, ta = counts(after)
    places = [PLACES[n] for n in list(already) + list(names) if n in after]
    out = {"surface": ANY, "excluded": places, "cornersBefore": cb, "cornersAfter": ca, "trianglesBefore": tb,
           "trianglesAfter": ta, "cut": cut, "fill": fill, "net": fill - cut, "changedArea": area, "revision": ANY}
    if largest:
        out["largest"] = largest
    if not places:
        del out["excluded"]
    return out


PLACES = {**{n: [x, y] for n, (x, y, _) in SPIKES.items()}, "node": [OX + 4.0, 4.0]}


def rs(v):
    """A number as Rust's Display writes it: the shortest form, no “.0” on a whole one."""
    t = repr(float(v))
    return t[:-2] if t.endswith(".0") else t


def at(p):
    return f"({rs(p[0])}, {rs(p[1])})"


def cases():
    out = []

    def step(op, inp, result, expect):
        return {"op": op, "input": inp, "result": result, "expect": expect, "within": W}

    def c(name, steps, setup_=None):
        case = {"name": name, "steps": steps}
        if setup_:
            case["setup"] = setup_
        out.append(case)

    def ok(names, restore=(), already=(), surface="Onarım"):
        o = answer(names, restore, already)
        o["surface"] = "t-12" if surface in ("Onarım", "t-12") else "t-13"
        return {"status": "completed", "output": o, "warnings": []}

    fail = analysis.failed
    s1, s2, node = PLACES["s1"], PLACES["s2"], PLACES["node"]
    c("Sivri noktayı dışarıda bırak", [step("execute", {"surface": "Onarım", "exclude": [s1]}, ok(["s1"]), STEPPED)])
    c("Çukur noktayı dışarıda bırak", [step("execute", {"surface": "Onarım", "exclude": [s2]}, ok(["s2"]), STEPPED)])
    c("İkisi birden", [step("execute", {"surface": "t-12", "exclude": [s1, s2]}, ok(["s1", "s2"]), STEPPED)])
    plan_ok = ok(["s1"])
    c("Plan yazmaz", [step("plan", {"surface": "Onarım", "exclude": [s1]}, plan_ok, UNCHANGED)])
    c("Düzlemdeki bir köşe hiçbir şeyi değiştirmez",
      [step("execute", {"surface": "Onarım", "exclude": [node]}, ok(["node"]), STEPPED)])
    c("Bırakılanı geri al", [step("execute", {"surface": "Onarılmış", "restore": [s1]},
                                  ok([], ["s1"], ["s1"], "Onarılmış"), STEPPED)])
    c("Biri bırakılmışken öbürünü de bırak", [step("execute", {"surface": "Onarılmış", "exclude": [s2]},
                                                   ok(["s2"], (), ["s1"], "Onarılmış"), STEPPED)])
    c("Bırak ve geri al birlikte", [step("execute", {"surface": "Onarılmış", "exclude": [s2], "restore": [s1]},
                                         ok(["s2"], ["s1"], ["s1"], "Onarılmış"), STEPPED)])
    c("Bırakıp geri adım", [step("execute", {"surface": "Onarım", "exclude": [s1]}, ok(["s1"]), STEPPED),
                            {"op": "undo", "returns": "Yüzeyi onar", "expect": {"canUndo": False, "canRedo": True}},
                            step("plan", {"surface": "Onarım", "exclude": [s1]}, plan_ok,
                                 {"canUndo": False, "canRedo": True, "revision": "same"})])
    c("Bırak, sonra geri al", [step("execute", {"surface": "Onarım", "exclude": [s2]}, ok(["s2"]), STEPPED),
                               step("execute", {"surface": "Onarım", "restore": [s2]}, ok([], ["s2"], ["s2"]),
                                    {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"})])
    # Refusals.
    c("Bir şey sorulmadı", [step("execute", {"surface": "Onarım"}, fail(
        "invalid_repair", "Bir şey sorun: dışarıda bırakılacak (exclude) ya da geri alınacak (restore) yerler."),
        UNCHANGED)])
    c("Aynı yer iki kez", [step("execute", {"surface": "Onarım", "exclude": [s1, s1]}, fail(
        "invalid_repair", f"{at(s1)} iki kez verildi.", "exclude[1]"), UNCHANGED)])
    c("Aynı yer iki listede", [step("execute", {"surface": "Onarılmış", "exclude": [s2], "restore": [s2]}, fail(
        "invalid_repair", f"{at(s2)} iki kez verildi.", "restore[0]"), UNCHANGED)])
    c("Bırakılmamışı geri alma", [step("execute", {"surface": "Onarım", "restore": [s1]}, fail(
        "invalid_repair", f"{at(s1)} dışarıda bırakılmamış; geri alınacak bir şey yok.", "restore[0]"),
        UNCHANGED)])
    c("Bırakılmışı yeniden bırakma", [step("execute", {"surface": "Onarılmış", "exclude": [s1]}, fail(
        "invalid_repair", f"{at(s1)} zaten dışarıda bırakılmış.", "exclude[0]"), UNCHANGED)])
    b = [BREAK[0][0], BREAK[0][1]]
    c("Kırık hattın köşesi", [step("execute", {"surface": "Onarım", "exclude": [b]}, fail(
        "repair_breakline", f"{at(b)} bir kırık hattın köşesi; onarım kırık hattı ve tasarım noktasını "
        "değiştirmez.", "exclude[0]"), UNCHANGED)])
    nowhere = [OX + 1.0, 1.0]
    c("Kayıt olmayan yer", [step("execute", {"surface": "Onarım", "exclude": [nowhere]}, fail(
        "repair_not_a_point", f"{at(nowhere)} yerinde yüzeyin bir nokta kaydı yok.", "exclude[0]"),
        UNCHANGED)])
    c("Olmayan yüzey", [step("execute", {"surface": "Tasarım", "exclude": [s1]}, fail(
        "unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "surface"), UNCHANGED)])
    c("Kurulamayan yüzey", [step("execute", {"surface": "Kurulamaz", "exclude": [s1]}, fail(
        "repair_not_a_point", f"{at(s1)} yerinde yüzeyin bir nokta kaydı yok.", "exclude[0]"), UNCHANGED)])
    c("Geçersiz beklenen sürüm", [step("execute", {"surface": "Onarım", "exclude": [s1], "expectedRevision": "on"},
                                       drainage.BAD_REVISION, UNCHANGED)])
    c("Eski sürüme göre hazırlanmış", [step("execute", {"surface": "Onarım", "exclude": [s1], "expectedRevision": "41"},
                                            drainage.CONFLICT, UNCHANGED)])
    c("Yol projesi değil", [step("execute", {"surface": "Onarım", "exclude": [s1]}, fail(
        "not_a_road_project", "Arazi yüzeyi yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
        UNCHANGED)], setup_="$map")
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="map")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.repair", "commandVersion": 1,
           "title": "Yüzeyi onar: nokta kayıtlarını dışarıda bırak ve geri al",
           "note": "scripts/fixtures/road_terrain_repair_cases.py'den; düzlemde kare içindeki sivri noktanın çadırı, "
                   "Delaunay üçgenleri boş çember sınamasıyla, ADR 0916'nın kuralıyla, KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.repair.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.repair.json")
    else:
        path.write_text(text)
    print(f"road.terrains.repair.json: {len(found)} durum")


if __name__ == "__main__":
    main()
