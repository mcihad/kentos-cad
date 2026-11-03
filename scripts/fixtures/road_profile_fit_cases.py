#!/usr/bin/env python3
"""The shared cases of `road.profiles.fit` (plan/road.md Y10.6b, docs/adr/0908 §13; ROAD-IO-14).

A profile read off a line drawn in a profile view by a reference written here from the ADR's rule and no KentOS code,
at 40 digits (mpmath): the view maps a drawing point (x, y) to station + (x − at.x) and datum + (y − at.y)/exaggeration;
straight segments are grades, a corner where the grade changes a PVI (with a parabola of `length` when given), an arc a
vertical curve between its ends whose PVI is where its end tangents meet (symmetric when midway, asymmetric otherwise).
The profile is built and judged by road_vertical_cases.py and road_profile_command_cases.py's references; the deviation
is each vertex's and arc middle's vertical distance from the profile at its station.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy python3 \
        scripts/fixtures/road_profile_fit_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_profile_command_cases as pcmd  # noqa: E402

mp.mp.dps = 40
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.profiles.fit.json"
completed, failed, step = pcmd.completed, pcmd.failed, pcmd.step
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = pcmd.UNCHANGED, pcmd.STEPPED, pcmd.CONFLICT, pcmd.BAD_REVISION
STEP_NAME = "Çizimden boykesit"
EXISTING = [pcmd.MAIN]
VIEW = {"at": [0.0, -500.0], "datum": 90.0, "exaggeration": 10.0}


def case(name, steps, setup_=None):
    c = {"name": name, "steps": steps}
    if setup_ is not None:
        c["setup"] = setup_
    return c


def U(n):
    return pcmd.U(n)


def draw(points, view=VIEW, station=0.0):
    """(station, elevation) → the drawing."""
    ex = view.get("exaggeration", 1.0)
    return [(view["at"][0] + (s - station), view["at"][1] + (z - view["datum"]) * ex) for s, z in points]


def fillet(p0, v, p2, r):
    """A drawn corner p0–v–p2 rounded by an arc of radius r in the drawing: the polyline's points and bulges."""
    p0, v, p2 = [mp.mpf(c) for c in p0], [mp.mpf(c) for c in v], [mp.mpf(c) for c in p2]
    u1 = [v[0] - p0[0], v[1] - p0[1]]
    u2 = [p2[0] - v[0], p2[1] - v[1]]
    l1, l2 = mp.sqrt(u1[0] ** 2 + u1[1] ** 2), mp.sqrt(u2[0] ** 2 + u2[1] ** 2)
    u1, u2 = [u1[0] / l1, u1[1] / l1], [u2[0] / l2, u2[1] / l2]
    theta = mp.atan2(u1[0] * u2[1] - u1[1] * u2[0], u1[0] * u2[0] + u1[1] * u2[1])
    t = r * mp.tan(abs(theta) / 2)
    t1 = [v[0] - u1[0] * t, v[1] - u1[1] * t]
    t2 = [v[0] + u2[0] * t, v[1] + u2[1] * t]
    pts = [p0, t1, t2, p2]
    return [(float(a), float(b)) for a, b in pts], [0.0, float(mp.tan(theta / 4)), 0.0]


def poly(n, pts, bulges=None, closed=False):
    e = {"kind": "polyline", "id": n, "layerId": "0", "attrs": {}, "pts": [{"x": x, "y": y} for x, y in pts]}
    if bulges and any(bulges):
        e["bulges"] = bulges
    if closed:
        e["closed"] = True
    return e


FIRST = len(pcmd.ENTITIES) + 1
GRADES = [(0, 100), (150, 104), (300, 99), (450, 106)]
CREST, CREST_B = fillet(*draw([(50, 100), (200, 106), (350, 100)]), 400)
SAG, SAG_B = fillet(*draw([(20, 104), (180, 98), (420, 103)]), 300)
LINES = [
    poly(FIRST, draw(GRADES)),
    poly(FIRST + 1, draw(GRADES)[::-1]),
    poly(FIRST + 2, CREST, CREST_B),
    poly(FIRST + 3, SAG, SAG_B),
    {"kind": "line", "id": FIRST + 4, "layerId": "0", "attrs": {}, "a": {"x": 30.0, "y": -400.0},
     "b": {"x": 230.0, "y": -360.0}},
    poly(FIRST + 5, [(0.0, -400.0), (100.0, -380.0), (200.0, -400.0)], [0.1, -0.1]),
    poly(FIRST + 6, [(0.0, -400.0), (100.0, -380.0), (80.0, -390.0), (200.0, -400.0)]),
    poly(FIRST + 7, [(0.0, -400.0), (50.0, -400.0), (50.0, -350.0)], closed=True),
    poly(FIRST + 8, draw([(0, 100), (300, 103), (800, 106)])),
    poly(FIRST + 9, draw([(0, 100), (100, 103), (200, 99)])),
]


def setup(work="road"):
    base = pcmd.setup(EXISTING if work == "road" else (), work)
    base["entities"] += copy.deepcopy(LINES)
    base["uids"] += [U(e["id"]) for e in LINES]
    return base


def bad(path, words):
    return failed("invalid_profile_fit", words, path)


class Refused(Exception):
    def __init__(self, result):
        super().__init__(result["error"]["message"])
        self.result = result


# ── The reference ─────────────────────────────────────────────────────────────────────────────────────────────────

def M(v):
    return mp.mpf(v)


def arc_of(a, b, g):
    a, b = [M(c) for c in a], [M(c) for c in b]
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = mp.sqrt(dx * dx + dy * dy)
    turn = 4 * mp.atan(M(g))
    r = chord / abs(2 * mp.sin(turn / 2))
    h = r * mp.cos(turn / 2) * mp.sign(turn)
    return [(a[0] + b[0]) / 2 - dy / chord * h, (a[1] + b[1]) / 2 + dx / chord * h], turn


def tangent(c, p, turn):
    rx, ry = M(p[0]) - c[0], M(p[1]) - c[1]
    return [-ry, rx] if turn > 0 else [ry, -rx]


def traced(pts, bulges, view, station, length):
    ex = M(view.get("exaggeration", 1.0))
    if pts[-1][0] < pts[0][0]:
        pts, bulges = pts[::-1], [-b for b in bulges[::-1]]
    read = [(M(station) + M(x) - view["at"][0], M(view["datum"]) + (M(y) - view["at"][1]) / ex) for x, y in pts]
    for k in range(1, len(read)):
        if read[k][0] <= read[k - 1][0]:
            raise Refused(bad("uid", "Çizimden boykesit kurulamıyor: " + (
                "çizginin iki ayrı km'si yok." if len(read) == 2
                else f"{k + 1}. köşede km geri gidiyor; boykesit soldan sağa artar.")))
    out = [(read[0][0], read[0][1], {})]
    n = len(bulges)
    for i in range(n):
        if bulges[i]:
            if i > 0 and bulges[i - 1]:
                raise Refused(bad("uid", f"Çizimden boykesit kurulamıyor: {i + 1}. parçada aralarında eğim olmayan "
                                         "iki yay var."))
            c, turn = arc_of(pts[i], pts[i + 1], bulges[i])
            ta, tb = tangent(c, pts[i], turn), tangent(c, pts[i + 1], turn)
            da, db = [ta[0], ta[1] / ex], [tb[0], tb[1] / ex]
            p, q = read[i], read[i + 1]
            cross = da[0] * db[1] - da[1] * db[0]
            t = ((q[0] - p[0]) * db[1] - (q[1] - p[1]) * db[0]) / cross
            v = (p[0] + t * da[0], p[1] + t * da[1])
            l1, l2 = v[0] - p[0], q[0] - v[0]
            curve = {"length": l1 + l2} if abs(l1 - l2) <= M("1e-9") * (l1 + l2) else {"lengthIn": l1, "lengthOut": l2}
            out.append((v[0], v[1], curve))
        elif i + 1 < n and not bulges[i + 1]:
            p, v, q = read[i], read[i + 1], read[i + 2]
            g1, g2 = (v[1] - p[1]) / (v[0] - p[0]), (q[1] - v[1]) / (q[0] - v[0])
            if abs(g2 - g1) > M("1e-12"):
                out.append((v[0], v[1], {"length": M(length)} if length else {}))
    out.append((read[-1][0], read[-1][1], {}))
    pvis = []
    for k, (s, z, c) in enumerate(out):
        pid = "a" if k == 0 else "b" if k == len(out) - 1 else f"v{k}"
        pvis.append(pcmd.pv(pid, float(s), float(z), **{key: float(val) for key, val in c.items()}))
    return pvis, read


def probes(pts, bulges):
    out = [tuple(p) for p in pts]
    for i, b in enumerate(bulges):
        if b:
            (ax, ay), (cx, cy) = pts[i], pts[i + 1]
            dx, dy = M(cx) - ax, M(cy) - ay
            out.append(((M(ax) + cx) / 2 + M(b) / 2 * dy, (M(ay) + cy) / 2 - M(b) / 2 * dx))
    return out


def fitted(input_):
    if input_["alignment"].lower() not in ("g-1", "ana aks", "g-2", "yan aks"):
        raise Refused(failed("unknown_alignment", f"“{input_['alignment']}” kimlikli ya da adlı aks projede yok.",
                             "alignment"))
    alignment = {"ana aks": "g-1", "yan aks": "g-2"}.get(input_["alignment"].lower(), input_["alignment"])
    surface = None
    if input_.get("surface"):
        s = input_["surface"]
        if s.lower() not in ("t-1", "mevcut zemin"):
            raise Refused(failed("unknown_terrain", f"“{s}” kimlikli ya da adlı yüzey projede yok.", "surface"))
        surface = "t-1"
    e = next((x for x in LINES if U(x["id"]) == input_["uid"]), None)
    if e is None:
        raise Refused(failed("entity_not_found", f"“{input_['uid']}” kimlikli nesne çizimde yok.", "uid"))
    if e["kind"] == "line":
        pts, bulges = [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])], [0]
    elif e.get("closed"):
        raise Refused(bad("uid", f"“{input_['uid']}” kapalı bir çoklu çizgi; boykesit açık bir çizgiden kurulur."))
    else:
        pts = [(p["x"], p["y"]) for p in e["pts"]]
        bulges = (e.get("bulges") or [0] * (len(pts) - 1))[:len(pts) - 1]
    for k, what in (("exaggeration", "Abartma (exaggeration)"), ("length", "Düşey kurbun boyu (length)"),
                    ("tolerance", "Tolerans (tolerance)")):
        if k in input_ and not input_[k] > 0:
            raise Refused(bad(k, f"{what} 0'dan büyük olmalı."))
    view = {"at": input_["at"], "datum": input_["datum"], "exaggeration": input_.get("exaggeration", 1.0)}
    station = input_.get("station", 0.0)
    pvis, _ = traced(pts, bulges, view, station, input_.get("length", 0))
    name = (input_.get("name") or "").strip() or "1. boykesit"
    if name.lower() == "proje kotu":
        raise Refused(bad("name", f"Projede “{name}” adlı bir boykesit var."))
    made = {"id": "b-2", "name": name, "alignment": alignment, "alignmentRevision": pcmd.REV[alignment], "revision": 1,
            "pvis": pvis, **({"surface": surface} if surface else {})}
    try:
        _, at, _, _ = pcmd.solve(made)
        length = float(pcmd.ALIGNED[alignment].length)
        if any(p["station"] < -1e-9 or p["station"] > length + 1e-9 for p in pvis):
            raise pcmd.vref.Refused(kind="Off")
    except pcmd.vref.Refused:
        r = pcmd.vertical_refusal(made)
        raise Refused(bad("uid", "Çizimden boykesit kurulamıyor: " + r["error"]["message"]))
    ex = M(view["exaggeration"])
    seen = []
    for p in probes(pts, bulges):
        s = M(station) + M(p[0]) - view["at"][0]
        z = M(view["datum"]) + (M(p[1]) - view["at"][1]) / ex
        zp = at(float(s))
        seen.append((abs(M(zp[0]) - z), (s, z)))
    worst, where = max(seen, key=lambda t: t[0])
    tied = worst < M("1e-6") or sum(1 for d, _ in seen if worst - d < M("1e-6")) > 1
    curves = sum(1 for p in pvis if p.get("length") or p.get("lengthIn"))
    out = {"id": "b-2", "name": name, "alignment": alignment, "source": input_["uid"],
           "pvis": pvis, "grades": len(pvis) - 1, "curves": curves,
           "stations": [pvis[0]["station"], pvis[-1]["station"]], "deviation": float(worst),
           "worst": "$any" if tied else [float(where[0]), float(where[1])], "revision": "$current"}
    warnings = []
    if "tolerance" in input_ and worst > input_["tolerance"]:
        warnings.append({"code": "profile_fit_deviation", "path": "tolerance", "message": "$any"})
    return out, warnings, made


def ok(name, input_, op="execute", undo=False):
    out, warnings, made = fitted(input_)
    before = pcmd.road_of(EXISTING)
    if op == "validate":
        result, expect = completed(None, warnings), {**UNCHANGED, "road": before}
    elif op == "plan":
        result, expect = completed(out, warnings), {**UNCHANGED, "road": before}
    else:
        result, expect = completed(out, warnings), {**STEPPED, "road": pcmd.road_of(EXISTING + [made])}
    steps = [step(op, input_, result, expect)]
    if undo:
        steps.append({"op": "undo", "returns": STEP_NAME, "expect": {"canRedo": True, "road": before}})
    return case(name, steps)


def no(name, input_, result=None, setup_=None):
    if result is None:
        try:
            fitted(input_)
        except Refused as r:
            result = r.result
        else:
            raise AssertionError(f"{name}: refuses")
    road = pcmd.road_of(EXISTING) if setup_ is None else None
    return case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_)


def inp(n, **kw):
    return {"uid": U(FIRST + n), "alignment": "g-1", **VIEW, **kw}


def cases():
    return [
        ok("Eğimler: köşeler kurpsuz PVI", inp(0)),
        ok("Eğimler, köşelere 60 m parabol, yüzeyiyle", inp(0, length=60.0, surface="Mevcut zemin", name="Proje kotu 2")),
        ok("Sağdan sola çizilmiş çizgi soldan sağa okunur", inp(1, length=60.0)),
        ok("Simetrik tepe yayı: simetrik parabol", inp(2, tolerance=0.001)),
        ok("Asimetrik çukur yayı: asimetrik parabol", inp(3, tolerance=0.5)),
        ok("Abartmasız görünümde aynı çizgi", inp(0, exaggeration=1.0, datum=-410.0)),
        ok("Görünüm 100 m'den başlar", inp(0, station=100.0, length=60.0)),
        ok("Tek çizgi: tek eğim, adıyla", {"uid": U(FIRST + 4), "alignment": "Ana aks", "at": [0.0, -500.0], "datum": 90.0,
                                           "exaggeration": 10.0, "name": "Köy yolu kotu"}),
        ok("Yan aksta", inp(9, alignment="Yan aks")),
        ok("Planı", inp(2), op="plan"),
        ok("Doğrulama", inp(0, length=60.0), op="validate"),
        ok("Tek geri alma adımı", inp(3), undo=True),
        no("Yol projesi değil", inp(0), pcmd.NOT_ROAD, setup(work="map")),
        no("Olmayan aks", inp(0, alignment="yok")),
        no("Olmayan yüzey", inp(0, surface="yok")),
        no("Olmayan nesne", {**inp(0), "uid": U(999)}),
        no("Kapalı çoklu çizgi", inp(7)),
        no("Art arda iki yay", inp(5)),
        no("Geri giden km", inp(6)),
        no("Aksın dışında PVI", inp(8)),
        no("Sığmayan düşey kurp", inp(0, length=400.0)),
        no("Sıfır abartma", inp(0, exaggeration=0.0)),
        no("Eksi kurp boyu", inp(0, length=-5.0)),
        no("Sıfır tolerans", inp(0, tolerance=0.0)),
        no("Alınmış ad", inp(0, name="proje KOTU")),
        no("Geçersiz beklenen sürüm", inp(0, expectedRevision="on"), BAD_REVISION),
        no("Eski sürüme göre hazırlanmış", inp(0, expectedRevision="41"), CONFLICT),
    ]


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.profiles.fit", "commandVersion": 1,
            "title": "Çizimden boykesit",
            "note": "scripts/fixtures/road_profile_fit_cases.py yazar; PVI'lar ve kurplar ADR 0908 §13'ün kuralıyla 40 "
                    "basamakta (mpmath), boykesit road_vertical_cases.py'nin başvurusuyla kurulur ve denetlenir; "
                    "KentOS kodu yok. `expect.road` projenin yol ayarlarıdır.",
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
