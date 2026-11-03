#!/usr/bin/env python3
"""The shared cases of `road.alignments.fit` (plan/road.md Y10.6, docs/adr/0908 §12; ROAD-IO-14).

An alignment made of a drawn line by a reference written here from the ADR's rule and no KentOS code, at 40 digits
(mpmath): element by element (a straight segment a tangent, an arc its circular curve; a run of curves between two
tangents one PI where they meet) and by best fit (a vertex on a curve where the circle through it and its neighbours
is smaller than the maximum radius; each run of tangent vertices a line by total least squares; the lines' meetings
the PIs; a curve's radius the one whose arc, tangent to both lines, lies nearest the curve's vertices). The deviation
is each probe's (vertices and arcs' middles) least distance to the alignment's tangents and arcs.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_alignment_fit_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_command_cases as acmd  # noqa: E402

mp.mp.dps = 40
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.alignments.fit.json"
completed, failed, step = acmd.completed, acmd.failed, acmd.step
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = acmd.UNCHANGED, acmd.STEPPED, acmd.CONFLICT, acmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
EXISTING = [{"id": "g-1", "name": "Ana aks", "revision": 1,
             "pis": [acmd.pi("a", 20, 50), acmd.pi("p1", 300, 60, 150), acmd.pi("b", 560, 360)]}]
STEP_NAME = "Çizgiden aks"


def case(name, steps, setup_=None):
    c = {"name": name, "steps": steps}
    if setup_ is not None:
        c["setup"] = setup_
    return c


def U(n):
    return f"0191f0aa-0000-7000-8000-{n:012x}"


def poly(n, pts, bulges=None, closed=False):
    e = {"kind": "polyline", "id": n, "layerId": "0", "attrs": {}, "pts": [{"x": x, "y": y} for x, y in pts]}
    if bulges:
        e["bulges"] = bulges
    if closed:
        e["closed"] = True
    return e


Q = float(mp.tan(mp.pi / 8))  # a quarter turn's bulge
# 1: tangent east, a quarter turn left of radius 80, tangent north.
CURVED = [(100.0, 100.0), (300.0, 100.0), (380.0, 180.0), (380.0, 400.0)]
# 2: three tangents meeting at angles.
ZIGZAG = [(0.0, 0.0), (200.0, 0.0), (300.0, 150.0), (500.0, 150.0)]
# 3: two arcs one after another (a reverse curve).
REVERSE = [(0.0, 0.0), (100.0, 0.0), (150.0, 50.0), (200.0, 100.0), (300.0, 100.0)]
# 4: an arc not tangent to the tangent after it.
KINKED = [(0.0, 0.0), (100.0, 0.0), (150.0, 50.0), (250.0, 50.0)]


def sampled():
    """A tangent, an arc of radius 60 turning a quarter left, a tangent: every 2 m, the arc in 36 pieces."""
    pts = [(1000.0 + 2.0 * i, 500.0) for i in range(31)]
    for i in range(1, 36):
        a = mp.pi / 2 * i / 36
        pts.append((float(1060 + 60 * mp.sin(a)), float(560 - 60 * mp.cos(a))))
    pts += [(1120.0, 560.0 + 2.0 * i) for i in range(31)]
    return pts


SAMPLED = sampled()
ENTITIES = [
    poly(1, CURVED, [0, Q, 0]),
    poly(2, ZIGZAG),
    poly(3, REVERSE, [0, Q / 2, -Q / 2, 0]),
    poly(4, KINKED, [0, float(mp.tan(mp.pi / 16)), 0]),
    {"kind": "line", "id": 5, "layerId": "0", "attrs": {}, "a": {"x": 0.0, "y": -50.0}, "b": {"x": 300.0, "y": -50.0}},
    poly(6, SAMPLED),
    poly(7, [(0.0, 0.0), (50.0, 0.0), (50.0, 50.0)], closed=True),
    {"kind": "point", "id": 8, "layerId": "0", "attrs": {}, "p": {"x": 1.0, "y": 2.0}},
]


def setup(work="road"):
    base = acmd.setup(EXISTING if work == "road" else None, work)
    base["entities"] = copy.deepcopy(ENTITIES)
    base["uids"] = [U(e["id"]) for e in ENTITIES]
    return base


def bad(path, words):
    return failed("invalid_alignment_fit", words, path)


class Refused(Exception):
    def __init__(self, result):
        super().__init__(result["error"]["message"])
        self.result = result


# ── Geometry at 40 digits ─────────────────────────────────────────────────────────────────────────────────────────


def V(p):
    return [mp.mpf(p[0]), mp.mpf(p[1])]


def unit(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l = mp.sqrt(dx * dx + dy * dy)
    return [dx / l, dy / l]


def meet(p, d, q, e):
    cross = d[0] * e[1] - d[1] * e[0]
    if abs(cross) < mp.mpf("1e-12"):
        return None
    t = ((q[0] - p[0]) * e[1] - (q[1] - p[1]) * e[0]) / cross
    return [p[0] + t * d[0], p[1] + t * d[1]]


def arc_of(a, b, bulge):
    """An arc segment's centre, radius and turn."""
    a, b = V(a), V(b)
    chord = mp.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)
    turn = 4 * mp.atan(mp.mpf(bulge))
    r = chord / abs(2 * mp.sin(turn / 2))
    m = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2]
    u = [(b[0] - a[0]) / chord, (b[1] - a[1]) / chord]
    h = r * mp.cos(turn / 2) * mp.sign(turn)
    return [m[0] - u[1] * h, m[1] + u[0] * h], r, turn


def elements_pis(pts, bulges, radius):
    """The PIs element by element: a run of arcs between tangents one PI; refuses two arcs in a row."""
    segs = list(zip(pts, pts[1:], bulges))
    out = [(V(pts[0]), 0)]
    i = 0
    while i < len(segs):
        a, b, g = segs[i]
        if g == 0:
            if i + 1 < len(segs) and segs[i + 1][2] == 0:
                out.append((V(b), 0))
            i += 1
            continue
        if i + 1 < len(segs) and segs[i + 1][2] != 0:
            raise Refused(bad("uid", f"Çizgiden aks kurulamıyor: {i + 2}. parçada aralarında aliyman olmayan iki yay var "
                                     "(bileşik ya da ters kurp)."))
        c, r, turn = arc_of(a, b, g)
        # The tangents at the arc's ends, their meeting the PI.
        def tang(p):
            q = V(p)
            rr = unit(c, q)
            return [-rr[1], rr[0]] if turn > 0 else [rr[1], -rr[0]]
        v = meet(V(a), tang(a), V(b), tang(b))
        out.append((v, r))
        i += 1
    out.append((V(pts[-1]), 0))
    pis = [list(p) + [r] for p, r in out]
    if radius:
        for p in pis[1:-1]:
            if p[2] == 0:
                p[2] = mp.mpf(radius)
    return pis


def circumradius(a, b, c):
    cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    if cross == 0:
        return mp.inf
    ab = mp.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)
    bc = mp.sqrt((c[0] - b[0]) ** 2 + (c[1] - b[1]) ** 2)
    ca = mp.sqrt((a[0] - c[0]) ** 2 + (a[1] - c[1]) ** 2)
    return ab * bc * ca / (2 * abs(cross))


def tls(points):
    n = len(points)
    cx = sum(p[0] for p in points) / n
    cy = sum(p[1] for p in points) / n
    sxx = sum((p[0] - cx) ** 2 for p in points)
    syy = sum((p[1] - cy) ** 2 for p in points)
    sxy = sum((p[0] - cx) * (p[1] - cy) for p in points)
    th = mp.atan2(2 * sxy, sxx - syy) / 2
    d = [mp.cos(th), mp.sin(th)]
    f, l = points[0], points[-1]
    if d[0] * (l[0] - f[0]) + d[1] * (l[1] - f[1]) < 0:
        d = [-d[0], -d[1]]
    return [cx, cy], d


def foot(c, d, p):
    t = (p[0] - c[0]) * d[0] + (p[1] - c[1]) * d[1]
    return [c[0] + t * d[0], c[1] + t * d[1]]


def best_radius(v, d, e, pts, most):
    turn = mp.atan2(d[0] * e[1] - d[1] * e[0], d[0] * e[0] + d[1] * e[1])
    half = abs(turn) / 2
    b = [e[0] - d[0], e[1] - d[1]]
    bl = mp.sqrt(b[0] ** 2 + b[1] ** 2)
    b = [b[0] / bl, b[1] / bl]

    def f(r):
        c = [v[0] + b[0] * r / mp.cos(half), v[1] + b[1] * r / mp.cos(half)]
        return sum((mp.sqrt((p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2) - r) ** 2 for p in pts)
    # The least: where the derivative vanishes, from a scan's best start; held to (0, most].
    grid = [most * mp.mpf(k) / 400 for k in range(1, 401)]
    r0 = min(grid, key=f)
    try:
        r = mp.findroot(lambda r: mp.diff(f, r), r0)
    except (ValueError, ZeroDivisionError):
        r = r0
    return min(max(r, mp.mpf(0)), most)


def bestfit_pis(points, most):
    p = []
    for q in points:
        if not p or p[-1] != list(q):
            p.append(list(q))
    p = [V(q) for q in p]
    n = len(p)
    straight = [i == 0 or i == n - 1 or circumradius(p[i - 1], p[i], p[i + 1]) >= most for i in range(n)]
    runs, i = [], 0
    while i < n:
        if not straight[i]:
            i += 1
            continue
        s = i
        while i + 1 < n and straight[i + 1]:
            i += 1
        if i > s:
            runs.append((s, i))
        i += 1
    lines = [tls(p[a:b + 1]) for a, b in runs]
    out = [foot(*lines[0], p[0]) + [0]]
    for k in range(1, len(runs)):
        (c0, d0), (c1, d1) = lines[k - 1], lines[k]
        v = meet(c0, d0, c1, d1)
        between = p[runs[k - 1][1] + 1:runs[k][0]]
        if not between:
            r = mp.mpf(0)
        else:
            turn = abs(mp.atan2(d0[0] * d1[1] - d0[1] * d1[0], d0[0] * d1[0] + d0[1] * d1[1]))
            reach = lambda q: mp.sqrt((q[0] - v[0]) ** 2 + (q[1] - v[1]) ** 2)  # noqa: E731
            room = min(reach(p[runs[k - 1][0]]), reach(p[runs[k][1]]))
            r = best_radius(v, d0, d1, between, room / mp.tan(turn / 2))
        out.append(v + [r])
    out.append(foot(*lines[-1], p[-1]) + [0])
    return out


def geometry(pis):
    """The alignment's elements from PIs without clothoids: ('line', a, b) and ('arc', centre, r, a, b, turn)."""
    els = []
    start = pis[0][:2]
    for i in range(1, len(pis) - 1):
        a, v, b = pis[i - 1][:2], pis[i][:2], pis[i + 1][:2]
        r = pis[i][2]
        d, e = unit(a, v), unit(v, b)
        turn = mp.atan2(d[0] * e[1] - d[1] * e[0], d[0] * e[0] + d[1] * e[1])
        if r == 0:
            els.append(("line", start, v))
            start = v
            continue
        t = r * mp.tan(abs(turn) / 2)
        tc = [v[0] - d[0] * t, v[1] - d[1] * t]
        ct = [v[0] + e[0] * t, v[1] + e[1] * t]
        n = [-d[1], d[0]] if turn > 0 else [d[1], -d[0]]
        c = [tc[0] + n[0] * r, tc[1] + n[1] * r]
        els.append(("line", start, tc))
        els.append(("arc", c, r, tc, ct, turn))
        start = ct
    els.append(("line", start, pis[-1][:2]))
    length = mp.mpf(0)
    for el in els:
        length += mp.sqrt((el[2][0] - el[1][0]) ** 2 + (el[2][1] - el[1][1]) ** 2) if el[0] == "line" else el[2] * abs(el[5])
    return els, length


def distance(els, q):
    best = mp.inf
    for el in els:
        if el[0] == "line":
            a, b = el[1], el[2]
            dx, dy = b[0] - a[0], b[1] - a[1]
            l2 = dx * dx + dy * dy
            t = 0 if l2 == 0 else max(0, min(1, ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / l2))
            x, y = a[0] + t * dx, a[1] + t * dy
            best = min(best, mp.sqrt((q[0] - x) ** 2 + (q[1] - y) ** 2))
        else:
            c, r, s, e, turn = el[1], el[2], el[3], el[4], el[5]
            ang = lambda p: mp.atan2(p[1] - c[1], p[0] - c[0])  # noqa: E731
            a0, aq = ang(s), ang(q)
            rel = (aq - a0) * mp.sign(turn)
            rel = rel % (2 * mp.pi)
            if rel <= abs(turn):
                best = min(best, abs(mp.sqrt((q[0] - c[0]) ** 2 + (q[1] - c[1]) ** 2) - r))
            for p in (s, e):
                best = min(best, mp.sqrt((q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2))
    return best


def probes(pts, bulges):
    out = [V(p) for p in pts]
    for (a, b), g in zip(zip(pts, pts[1:]), bulges):
        if g:
            s = mp.mpf(g) / 2
            out.append([(mp.mpf(a[0]) + b[0]) / 2 + s * (b[1] - a[1]), (mp.mpf(a[1]) + b[1]) / 2 - s * (b[0] - a[0])])
    return out


def fitted(input_):
    for k in ("radius", "maxRadius", "tolerance"):
        if k in input_ and not input_[k] > 0:
            what = {"radius": "Teğetler arasındaki kurbun yarıçapı (radius)", "maxRadius": "En büyük yarıçap (maxRadius)",
                    "tolerance": "Tolerans (tolerance)"}[k]
            raise Refused(bad(k, f"{what} 0'dan büyük olmalı."))
    e = next((x for x in ENTITIES if U(x["id"]) == input_["uid"]), None)
    if e is None:
        raise Refused(failed("entity_not_found", f"“{input_['uid']}” kimlikli nesne çizimde yok.", "uid"))
    if e["kind"] == "line":
        pts, bulges = [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])], [0]
    elif e["kind"] == "polyline" and e.get("closed"):
        raise Refused(bad("uid", f"“{input_['uid']}” kapalı bir çoklu çizgi; aks açık bir çizgiden kurulur."))
    elif e["kind"] == "polyline":
        pts = [(p["x"], p["y"]) for p in e["pts"]]
        bulges = (e.get("bulges") or [0] * (len(pts) - 1))[:len(pts) - 1]
    else:
        raise Refused(bad("uid", f"“{input_['uid']}” bir çizgi ya da çoklu çizgi değil."))
    if input_.get("reverse"):
        pts, bulges = pts[::-1], [-g for g in bulges[::-1]]
    if input_["method"] == "elements":
        pis = elements_pis(pts, bulges, input_.get("radius", 0))
    else:
        if "maxRadius" not in input_:
            raise Refused(bad("maxRadius", "En iyi uyumda kurpları ayıran en büyük yarıçap (maxRadius) verilmeli."))
        pis = bestfit_pis(pts, mp.mpf(input_["maxRadius"]))
    name = (input_.get("name") or "").strip() or "1. aks"
    if name.lower() == "ana aks":
        raise Refused(bad("name", f"Projede “{name}” adlı bir aks var."))
    els, length = geometry(pis)
    seen = [(distance(els, q), q) for q in probes(pts, bulges)]
    worst, at = max(seen, key=lambda t: t[0])
    # Two probes equally far (a symmetric line): which one the alignment's own rounding names is no rule's.
    tied = sum(1 for d, _ in seen if worst - d < mp.mpf("1e-6")) > 1
    n = len(pis)
    ids = ["a"] + [f"p{i}" for i in range(1, n - 1)] + ["b"]
    out = {"id": "g-2", "name": name, "source": input_["uid"], "method": input_["method"],
           "pis": [{"id": i, "x": float(p[0]), "y": float(p[1]), **({"radius": float(p[2])} if p[2] else {})}
                   for i, p in zip(ids, pis)],
           "tangents": sum(1 for el in els if el[0] == "line" and el[1] != el[2]),
           "curves": sum(1 for el in els if el[0] == "arc"), "length": float(length),
           "deviation": float(worst), "worst": "$any" if tied else [float(at[0]), float(at[1])]}
    warnings = []
    if "tolerance" in input_ and worst > input_["tolerance"]:
        warnings.append({"code": "alignment_fit_deviation", "path": "tolerance", "message": "$any"})
    return out, warnings, {"id": "g-2", "name": name, "revision": 1,
                            "pis": [{"id": i, "x": float(p[0]), "y": float(p[1]), "radius": float(p[2])}
                                    if p[2] else {"id": i, "x": float(p[0]), "y": float(p[1])}
                                    for i, p in zip(ids, pis)],
                            **({"stationing": {"start": input_["start"]}} if input_.get("start") else {})}


def ok(name, input_, op="execute", undo=False, deviation_any=False):
    out, warnings, made = fitted(input_)
    if deviation_any or out["deviation"] < 1e-6:
        # Where the alignment lies on the line the worst probe is noise's choice.
        out["worst"] = "$any"
    out = {**out, "revision": "$current"}
    if op == "validate":
        result, expect = completed(None, warnings), {**UNCHANGED, "road": {"alignments": EXISTING}}
    elif op == "plan":
        result, expect = completed(out, warnings), {**UNCHANGED, "road": {"alignments": EXISTING}}
    else:
        result, expect = completed(out, warnings), {**STEPPED, "road": {"alignments": EXISTING + [made]}}
    s = step(op, input_, result, expect)
    s["within"] = 1e-6
    steps = [s]
    if undo:
        steps.append({"op": "undo", "returns": STEP_NAME, "expect": {"canRedo": True, "road": {"alignments": EXISTING}}})
    return case(name, steps)


def no(name, input_, result=None, setup_=None):
    if result is None:
        try:
            fitted(input_)
        except Refused as r:
            result = r.result
        else:
            raise AssertionError(f"{name}: refuses")
    road = None if setup_ is not None else {"alignments": EXISTING}
    return case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_)


def cases():
    return [
        ok("Aliyman, kurp, aliyman: yaylı çoklu çizgi eleman eleman", {"uid": U(1), "method": "elements"}),
        ok("Aynısı ters yönde, adıyla, km 2+000'dan", {"uid": U(1), "method": "elements", "reverse": True,
                                                      "name": "Bağlantı yolu", "start": 2000.0}),
        ok("Kırık çizgi: kurpsuz PI'lar", {"uid": U(2), "method": "elements"}),
        ok("Kırık çizgi, teğetler arasına 30 m kurp", {"uid": U(2), "method": "elements", "radius": 30.0}),
        ok("Tek çizgi: tek aliyman", {"uid": U(5), "method": "elements"}),
        ok("Teğet olmayan yay: sapma toleransı aşar", {"uid": U(4), "method": "elements", "tolerance": 0.01},
           deviation_any=True),
        ok("Sık köşeli eksen, en iyi uyum 500 m", {"uid": U(6), "method": "bestFit", "maxRadius": 500.0, "tolerance": 0.001}),
        ok("En iyi uyum ters yönde", {"uid": U(6), "method": "bestFit", "maxRadius": 500.0, "reverse": True}),
        ok("Kırık çizgi en iyi uyumla: köşelerde kurpsuz", {"uid": U(2), "method": "bestFit", "maxRadius": 100.0}),
        ok("Planı", {"uid": U(1), "method": "elements"}, op="plan"),
        ok("Doğrulama", {"uid": U(2), "method": "elements", "radius": 30.0}, op="validate"),
        ok("Tek geri alma adımı", {"uid": U(6), "method": "bestFit", "maxRadius": 500.0}, undo=True),
        no("Yol projesi değil", {"uid": U(1), "method": "elements"}, NOT_ROAD, setup(work="map")),
        no("Olmayan nesne", {"uid": U(99), "method": "elements"}),
        no("Nokta", {"uid": U(8), "method": "elements"}),
        no("Kapalı çoklu çizgi", {"uid": U(7), "method": "elements"}),
        no("Ters kurp: iki yay art arda", {"uid": U(3), "method": "elements"}),
        no("En iyi uyumda en büyük yarıçap yok", {"uid": U(6), "method": "bestFit"}),
        no("Sıfır yarıçap", {"uid": U(2), "method": "elements", "radius": 0.0}),
        no("Eksi en büyük yarıçap", {"uid": U(6), "method": "bestFit", "maxRadius": -5.0}),
        no("Sıfır tolerans", {"uid": U(1), "method": "elements", "tolerance": 0.0}),
        no("Alınmış ad", {"uid": U(1), "method": "elements", "name": "ana AKS"}),
        no("Geçersiz beklenen sürüm", {"uid": U(1), "method": "elements", "expectedRevision": "on"}, BAD_REVISION),
        no("Eski sürüme göre hazırlanmış", {"uid": U(1), "method": "elements", "expectedRevision": "41"}, CONFLICT),
    ]


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.alignments.fit", "commandVersion": 1,
            "title": "Çizgiden aks",
            "note": "scripts/fixtures/road_alignment_fit_cases.py yazar; PI'lar, yarıçaplar ve sapma ADR 0908 §12'nin "
                    "kuralıyla 40 basamakta (mpmath), KentOS kodu yok.",
            "setup": setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
