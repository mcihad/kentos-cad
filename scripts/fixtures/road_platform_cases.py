#!/usr/bin/env python3
"""The shared cases of `road.platforms.edit`, `.query` and `.draw` (plan/road.md Y14, docs/adr/0927; KentOS Yol's
graded pads).

The terrains are planes (four points each, two triangles): along a slope line the terrain is linear, so the daylight is
the slope line's and the plane's meeting in closed form, u = (zₜ − z₀) / (m − g); a line whose meeting is past its reach
or off the terrain is open. With a pad all in fill (or all in cut) every piece between two neighbouring slope lines is
plane (an edge's strip: the pad's edge and the slope rising or falling square to it; a corner's fan: a triangle), so the
design surface does not depend on how it is triangulated and the volume is each piece's plan area times the height
difference at its centroid (a linear function's mean). The balance replays ADR 0927's bisection on those volumes. No
KentOS code; the slopes, spacings, fans and reaches are the cases' own, not a standard's.

    python3 scripts/fixtures/road_platform_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_grades  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"


def completed(output, warns=()):
    return {"status": "completed", "output": output, "warnings": list(warns)}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path:
        e["path"] = path
    return {"status": "failed", "error": e}


def step(op, input_, result, expect=None):
    s = {"op": op, "input": input_, "result": result}
    if expect is not None:
        s["expect"] = expect
    s["within"] = W
    return s


def case(name, steps, setup_=None):
    c = {"name": name, "steps": steps}
    if setup_ is not None:
        c["setup"] = setup_
    return c


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
    "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}
BAD_REVISION = failed("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
NOT_ROAD = failed("not_a_road_project", "Platform Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim "
                  "ikisinden biri değil.")


def num(v):
    """A number as Rust's Display writes an f64."""
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def comma(v, digits):
    return f"{v:.{digits}f}".replace(".", ",")


# ── The terrains: planes over [−200, 400]² ────────────────────────────────────────────────────────────────────────
LOW, HIGH = -200.0, 400.0
PLANES = {"t-1": (100.0, 0.05, 0.02), "t-2": (100.0, 0.0, 0.0)}


def terrain_z(sid, x, y):
    z0, gx, gy = PLANES[sid]
    return z0 + gx * x + gy * y


def inside(x, y):
    return LOW - 1e-9 <= x <= HIGH + 1e-9 and LOW - 1e-9 <= y <= HIGH + 1e-9


def node(id_, name, **kw):
    n = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
         "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    n.update(kw)
    return n


SURFACES = [{"id": "t-1", "name": "Eğimli arazi", "points": {"layers": ["arazi"]}},
            {"id": "t-2", "name": "Düz arazi", "points": {"layers": ["duz"]}}]

P1 = {"id": "p-1", "name": "Depo", "revision": 1, "corners": [[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]],
      "base": [0.0, 0.0, 104.0], "cut": 1.5, "fill": 2.0, "step": 10.0, "fan": 15.0, "reach": 100.0, "surface": "t-1"}
P2 = {"id": "p-2", "name": "Yarma", "revision": 1,
      "corners": [[100.0, 100.0], [140.0, 100.0], [140.0, 130.0], [100.0, 130.0]],
      "base": [100.0, 100.0, 104.0], "cut": 1.5, "fill": 2.0, "step": 10.0, "fan": 30.0, "reach": 100.0,
      "surface": "t-1"}
START = [P1, P2]
ZERO = ("grade", "direction")


def compact(p):
    return {k: v for k, v in p.items() if not (k in ZERO and v == 0.0) and not (k == "revision" and v == 0)}


def road_of(platforms):
    out = {"surfaces": copy.deepcopy(SURFACES)}
    if platforms:
        out["platforms"] = [compact(p) for p in copy.deepcopy(list(platforms))]
    return out


def setup(platforms=START, work="road"):
    layers = [node("0", "0"), node("arazi", "Arazi"), node("duz", "Düz arazi"), node("platform", "Platform"),
              node("gizli", "Gizli", visible=False), node("kilitli", "Kilitli", locked=True),
              node("grup", "Yol", type="group", children=[node("yol-platform", "Yol platform")])]
    entities, k = [], 0
    for sid, layer in (("t-1", "arazi"), ("t-2", "duz")):
        for x, y in ((LOW, LOW), (HIGH, LOW), (HIGH, HIGH), (LOW, HIGH)):
            k += 1
            entities.append({"kind": "point", "id": k, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y},
                             "z": terrain_z(sid, x, y)})
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work in ("road", "map"):
        # A map project holds surfaces and pads too (docs/adr/1580).
        settings["road"] = road_of(platforms)
    return {"format": "kentos.document", "version": 2, "name": "Platformlar", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": layers, "activeLayer": "0", "entities": entities,
            "uids": [f"0192f5a0-7c3e-7d4a-9b1e-{i:012d}" for i in range(1, k + 1)],
            "styles": {"items": [], "categories": []}}


FIRST = len(setup()["entities"]) + 1


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def find(list_, name, path="platform"):
    for p in list_:
        if p["id"] == name:
            return p
    for p in list_:
        if p["name"].lower() == name.lower():
            return p
    raise Refused("unknown_platform", f"“{name}” kimlikli ya da adlı platform projede yok.", path)


# ── Grading (ADR 0927 §1) ─────────────────────────────────────────────────────────────────────────────────────────
def plane(p, x, y, offset=0.0):
    d = math.radians(p.get("direction", 0.0))
    b = p["base"]
    return b[2] + offset + p.get("grade", 0.0) / 100.0 * ((x - b[0]) * math.cos(d) + (y - b[1]) * math.sin(d))


def area2(ring):
    n = len(ring)
    return sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))


def crossing(a, b, c, d):
    def o(p, q, r):
        return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])

    d1, d2, d3, d4 = o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d)
    if ((d1 > 0 > d2) or (d1 < 0 < d2)) and ((d3 > 0 > d4) or (d3 < 0 < d4)):
        return True

    def on(p, q, r):
        return o(p, q, r) == 0 and min(p[0], q[0]) <= r[0] <= max(p[0], q[0]) and min(p[1], q[1]) <= r[1] <= max(p[1], q[1])

    return on(c, d, a) or on(c, d, b) or on(a, b, c) or on(a, b, d)


def ring_of(p):
    """The boundary checked (after the contract's rules) and counterclockwise."""
    c = p["corners"]
    n = len(c)
    for i in range(n):
        if c[i] == c[(i + 1) % n]:
            raise Refused("invalid_platform", f"Platform tesviye edilemiyor: Platformun {(i + 1) % n + 1}. köşesi bir "
                                              "öncekiyle aynı yerde.", "corners")
    for i in range(n):
        for j in range(i + 1, n):
            if j == i + 1 or (i == 0 and j == n - 1):
                continue
            if crossing(c[i], c[(i + 1) % n], c[j], c[(j + 1) % n]):
                raise Refused("invalid_platform", f"Platform tesviye edilemiyor: Platformun sınırı kendini kesiyor: "
                                                  f"{i + 1}. ve {j + 1}. kenar.", "corners")
    ring = [list(x) for x in c]
    if area2(ring) < 0:
        ring.reverse()
    return ring


def outward(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l = math.hypot(dx, dy)
    return [dy / l, -dx / l]


def daylight(p, at, d, z0):
    """(side, meeting) of the line from `at` along `d`; the meeting (x, y, z) or None (open)."""
    sid = p["surface"]
    if not inside(*at):
        return "open", None
    zt = terrain_z(sid, *at)
    side, m = ("cut", 1.0 / p["cut"]) if zt > z0 else ("fill", -1.0 / p["fill"])
    z0_, gx, gy = PLANES[sid]
    g = gx * d[0] + gy * d[1]
    if zt == z0:
        return side, [at[0], at[1], z0]
    if m == g:
        return "open", None
    u = (zt - z0) / (m - g)
    if u < 0 or u > p["reach"] + 1e-9:
        return "open", None
    q = [at[0] + u * d[0], at[1] + u * d[1]]
    if not inside(*q):
        return "open", None
    return side, [q[0], q[1], z0 + m * u]


def grade(p, offset=0.0):
    ring = ring_of(p)
    n = len(ring)
    lines = []

    def push(at, d, corner, at_corner):
        z0 = plane(p, at[0], at[1], offset)
        side, hit = daylight(p, at, d, z0)
        line = {"from": [at[0], at[1], z0]}
        if hit is not None:
            line["to"] = hit
        line["side"] = side
        line["corner"] = corner
        if at_corner:
            line["atCorner"] = True
        lines.append(line)

    for i in range(n):
        prev, here, nxt = ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]
        n1, n2 = outward(prev, here), outward(here, nxt)
        turn = (here[0] - prev[0]) * (nxt[1] - here[1]) - (here[1] - prev[1]) * (nxt[0] - here[0])
        if turn > 0:
            a1, a2 = math.atan2(n1[1], n1[0]), math.atan2(n2[1], n2[0])
            while a2 < a1:
                a2 += 2 * math.pi
            k = max(1, math.ceil(math.degrees(a2 - a1) / p["fan"]))
            for j in range(k + 1):
                a = a1 + (a2 - a1) * j / k
                push(here, [math.cos(a), math.sin(a)], i, True)
        else:
            b = [n1[0] + n2[0], n1[1] + n2[1]]
            l = math.hypot(*b)
            push(here, [b[0] / l, b[1] / l] if l > 1e-12 else n1, i, True)
        length = math.hypot(nxt[0] - here[0], nxt[1] - here[1])
        k = math.ceil(length / p["step"])
        for j in range(1, k):
            t = j / k
            push([here[0] + t * (nxt[0] - here[0]), here[1] + t * (nxt[1] - here[1])], n2, i, False)
    top = [[c[0], c[1], plane(p, c[0], c[1], offset)] for c in ring]
    return ring, top, lines


def piece_integral(sid, verts):
    """∫(design − terrain) over a plane piece: its plan area and the difference at its centroid."""
    pts = []
    for v in verts:
        if not pts or (abs(pts[-1][0] - v[0]) > 1e-12 or abs(pts[-1][1] - v[1]) > 1e-12):
            pts.append(v)
    while len(pts) > 1 and abs(pts[0][0] - pts[-1][0]) < 1e-12 and abs(pts[0][1] - pts[-1][1]) < 1e-12:
        pts.pop()
    if len(pts) < 3:
        return 0.0, 0.0
    a2 = area2(pts)
    if a2 == 0:
        return 0.0, 0.0
    cx = sum((pts[i][0] + pts[(i + 1) % len(pts)][0]) * (pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1]) for i in range(len(pts))) / (3 * a2)
    cy = sum((pts[i][1] + pts[(i + 1) % len(pts)][1]) * (pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1]) for i in range(len(pts))) / (3 * a2)
    # The design's plane through three of the piece's corners not on one line.
    best = None
    for i in range(len(pts)):
        for j in range(i + 1, len(pts)):
            for k in range(j + 1, len(pts)):
                a, b, c = pts[i], pts[j], pts[k]
                det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
                if best is None or abs(det) > abs(best[0]):
                    best = (det, a, b, c)
    det, a, b, c = best
    # z = a.z + α (x − a.x) + β (y − a.y)
    alpha = ((b[2] - a[2]) * (c[1] - a[1]) - (c[2] - a[2]) * (b[1] - a[1])) / det
    beta = ((b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2])) / det
    design = a[2] + alpha * (cx - a[0]) + beta * (cy - a[1])
    return abs(a2) / 2.0, abs(a2) / 2.0 * (design - terrain_z(sid, cx, cy))


def flat(verts):
    """Whether the corners lie on one plane (the design is then linear over them, however it is triangulated)."""
    pts = [v for i, v in enumerate(verts) if all(abs(v[0] - w[0]) > 1e-12 or abs(v[1] - w[1]) > 1e-12 for w in verts[:i])]
    if len(pts) <= 3:
        return True
    a, b, c = pts[0], pts[1], pts[2]
    det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
    if abs(det) < 1e-12:
        a, b, c = pts[0], pts[1], pts[3]
        det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
    alpha = ((b[2] - a[2]) * (c[1] - a[1]) - (c[2] - a[2]) * (b[1] - a[1])) / det
    beta = ((b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2])) / det
    return all(abs(a[2] + alpha * (q[0] - a[0]) + beta * (q[1] - a[1]) - q[2]) < 1e-9 for q in pts)


def volumes(p, ring, top, lines):
    """Cut and fill of a pad all in fill or all in cut (and their plan areas); None when it mixes, or a piece between
    two slope lines is not a plane (near a reflex corner): there the volume is the triangulation's."""
    sides = {l["side"] for l in lines}
    if len(sides) != 1:
        return None
    n = len(lines)
    if not all(flat([lines[i]["from"], lines[(i + 1) % n]["from"], lines[(i + 1) % n]["to"], lines[i]["to"]])
               for i in range(n)):
        return None
    sid = p["surface"]
    pieces = [top]
    n = len(lines)
    for i in range(n):
        a, b = lines[i], lines[(i + 1) % n]
        pieces.append([a["from"], b["from"], b["to"], a["to"]])
    area = total = 0.0
    for verts in pieces:
        s, v = piece_integral(sid, verts)
        area += s
        total += v
    if sides == {"fill"}:
        return {"cut": 0.0, "fill": total, "cutArea": 0.0, "fillArea": area}
    return {"cut": -total, "fill": 0.0, "cutArea": area, "fillArea": 0.0}


def read(p, offset=0.0):
    """The pad's report and warnings."""
    ring, top, lines = grade(p, offset)
    open_ = sum(1 for l in lines if l["side"] == "open")
    report = {"id": p["id"], "name": p["name"], "surface": p["surface"], "revision": p.get("revision", 0), "top": top,
              "lines": lines, "daylight": [l["to"] for l in lines] if open_ == 0 else [], "open": open_,
              "topArea": area2(ring) / 2.0}
    warnings = []
    if open_ > 0:
        report["why"] = f"{open_} şev çizgisi araziyle buluşmuyor: erimi büyütün ya da yüzeyi genişletin."
        warnings.append({"code": "platform_open", "path": "platform", "message":
                         f"“{p['name']}” platformunun {open_} şev çizgisi araziyle buluşmuyor (yüzeyin dışında ya da "
                         f"{num(p['reach'])} m eriminden uzakta); hacim ve günışığı çizgisi yok."})
    else:
        report["footprint"] = abs(area2([l["to"] for l in lines])) / 2.0
        v = volumes(p, ring, top, lines)
        if v is None:
            report.update({"cut": ANY, "fill": ANY, "cutArea": ANY, "fillArea": ANY})
        else:
            report.update(v)
    return report, warnings


def net(p, offset):
    r, _ = read(p, offset)
    return r["cut"] - r["fill"]


def balance(p, low, high, tol):
    """ADR 0927 §1's bisection: (offset, net, iterations, converged), or None when the net keeps its sign."""
    lo, hi = low, high
    flo, fhi = net(p, lo), net(p, hi)
    if flo == 0:
        return lo, 0.0, 0, True
    if fhi == 0:
        return hi, 0.0, 0, True
    if (flo > 0) == (fhi > 0):
        return None
    best = (lo, flo)
    for k in range(1, 61):
        mid = 0.5 * (lo + hi)
        f = net(p, mid)
        if abs(f) < abs(best[1]):
            best = (mid, f)
        if abs(f) <= tol:
            return mid, f, k, True
        if (f > 0) == (flo > 0):
            lo, flo = mid, f
        else:
            hi = mid
    return best[0], best[1], 60, False


def listed(list_):
    return [{"id": p["id"], "name": p["name"], "surface": p["surface"], "revision": p.get("revision", 0)} for p in list_]


# ── road.platforms.edit ───────────────────────────────────────────────────────────────────────────────────────────
FIELDS = ("name", "corners", "base", "grade", "direction", "cut", "fill", "step", "fan", "reach", "surface")
NEEDED = ("name", "corners", "base", "cut", "fill", "step", "fan", "reach", "surface")


def surface_id(name):
    for s in SURFACES:
        if s["id"] == name:
            return s["id"]
    for s in SURFACES:
        if s["name"].lower() == name.lower():
            return s["id"]
    raise Refused("unknown_terrain", f"“{name}” kimlikli ya da adlı yüzey projede yok.", "surface")


def rules(p):
    """The contract's rules for a pad (ADR 0927 §2); the first broken: (field, words)."""
    if len(p["corners"]) < 3 or len(p["corners"]) > 1000:
        return "corners", "platformun sınırı 3 ile 1000 arası köşedir"
    if abs(p.get("grade", 0.0)) > 100:
        return "grade", "platformun eğimi oranla yazılır (0,02 %2'dir), en çok 1 (%100) olur"
    for field, top in (("cut", 100.0), ("fill", 100.0), ("step", 1000.0), ("reach", 10000.0)):
        if not (0 < p[field] <= top):
            return field, f"0'dan büyük, en çok {num(top)} olur"
    if not (0 < p["fan"] <= 90):
        return "fan", "köşe yelpazesinin açısı 0'dan büyük, en çok 90 derece olur"
    return None


def edit(input_, start=START):
    """(result, the list after) of an edit; Refused where it is refused."""
    own = copy.deepcopy(list(start))
    op = input_["operation"]
    found = None
    if input_.get("platform") is not None:
        try:
            found = find(own, input_["platform"])
        except Refused:
            found = None
    if op == "remove" and found is None:
        find(own, input_.get("platform", ""))
    surface = surface_id(input_["surface"]) if "surface" in input_ else None
    if op == "rebind":
        raise Refused("invalid_platform", "Platformun yeniden bağlanacak bir kaynağı yok: yüzeyi değişince her okumada "
                                          "yeniden tesviye edilir.", "operation")
    target = None
    if op == "remove":
        own = [p for p in own if p["id"] != found["id"]]
    elif found is not None:
        if not any(f in input_ for f in FIELDS):
            raise Refused("invalid_platform", "Değiştirilecek bir şey yok: platformun bir alanını verin.")
        d = copy.deepcopy(found)
        for f in FIELDS:
            if f in input_:
                d[f] = surface if f == "surface" else input_[f]
        if d != found:
            d["revision"] = found.get("revision", 0) + 1
        own = [d if p["id"] == found["id"] else p for p in own]
        target = d["id"]
    else:
        missing = next((f for f in NEEDED if f not in input_), None)
        if missing:
            raise Refused("invalid_platform", "Yeni platform için name, corners, base, cut, fill, step, fan, reach ve "
                                              "surface verilmeli.", missing)
        ids = {p["id"] for p in own}
        pid = input_.get("platform") or next(f"p-{n}" for n in range(1, 10000) if f"p-{n}" not in ids)
        d = {"id": pid, "name": input_["name"], "revision": 1, "corners": input_["corners"], "base": input_["base"],
             "grade": input_.get("grade", 0.0), "direction": input_.get("direction", 0.0), "cut": input_["cut"],
             "fill": input_["fill"], "step": input_["step"], "fan": input_["fan"], "reach": input_["reach"],
             "surface": surface}
        own.append(d)
        target = pid
    for p in own:
        if True:
            broken = rules(p)
            if broken:
                raise Refused("invalid_platform", f"Platform kurallarına uymuyor: {broken[1]}.", broken[0])
    names = set()
    for p in own:
        if p["name"].lower() in names:
            raise Refused("invalid_platform", "Platform kurallarına uymuyor: bu adda başka bir platform var.", "name")
        names.add(p["name"].lower())
    warnings = []
    report = None
    if target is not None:
        d = next(p for p in own if p["id"] == target)
        ring_of(d)
        report, warnings = read(d)
    out = {"platforms": listed(own), "revision": "$current"}
    if report is not None:
        out = {"platform": report, **out}
    return completed(out, warnings), own


def edit_cases():
    cases = []

    def ok(name, input_, op="execute", undo=False, start=START):
        result, after = edit(input_, start)
        if op == "execute" and after != start:
            expect = {**STEPPED, "road": road_of(after)}
        elif op == "execute":
            expect = {**UNCHANGED, "road": road_of(start)}
        else:
            expect = {**UNCHANGED, "road": road_of(start)}
            if op == "validate":
                result = completed(None, [])
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Platform", "expect": {"canRedo": True, "road": road_of(start)}})
        cases.append(case(name, steps, None if start is START else setup(start)))

    def no(name, input_, start=START, setup_=None):
        try:
            edit(input_, start)
        except Refused as r:
            cases.append(case(name, [step("execute", input_, r.result, UNCHANGED)],
                              setup_ if setup_ is not None else (None if start is START else setup(start))))
            return
        raise AssertionError(name)

    new = {"operation": "set", "name": "Otopark", "corners": [[200.0, 0.0], [260.0, 0.0], [260.0, 40.0], [200.0, 40.0]],
           "base": [200.0, 0.0, 115.0], "cut": 1.5, "fill": 2.0, "step": 10.0, "fan": 15.0, "reach": 100.0,
           "surface": "Eğimli arazi"}
    ok("Yeni platform: kimliği ilk boş p-3, dolguda", new)
    ok("Yeni platform kendi kimliğiyle, eğimli", {**new, "platform": "otopark", "grade": 2.0, "direction": 90.0})
    ok("Yeni platform saat yönünde köşelerle", {**new, "corners": list(reversed(new["corners"]))})
    ok("Yeni platform düz arazide yarmada", {**new, "base": [200.0, 0.0, 97.0], "surface": "t-2"})
    ok("Planı", new, op="plan")
    ok("Doğrulama", new, op="validate")
    ok("Tek geri alma adımı", new, undo=True)
    ok("Adını değiştir", {"operation": "set", "platform": "p-1", "name": "Ana depo"})
    ok("Adıyla bulunur, kotu değişir", {"operation": "set", "platform": "DEPO", "base": [0.0, 0.0, 105.0]})
    ok("Şevleri değişir", {"operation": "set", "platform": "p-2", "cut": 1.0, "fill": 3.0})
    ok("Köşeleri değişir: beşgen", {"operation": "set", "platform": "p-1",
                                     "corners": [[0.0, 0.0], [40.0, 0.0], [50.0, 15.0], [40.0, 30.0], [0.0, 30.0]]})
    ok("Erim kısalınca açık çizgiler uyarısı", {"operation": "set", "platform": "p-1", "reach": 1.0})
    ok("Aynı değerler: sürüm artmaz", {"operation": "set", "platform": "p-1", "cut": 1.5})
    ok("Kaldır", {"operation": "remove", "platform": "p-2"})
    cases.append(case("Yol projesi değil", [step("execute", new, NOT_ROAD, UNCHANGED)], setup(work="architecture")))
    no("Olmayan platformu kaldır", {"operation": "remove", "platform": "yok"})
    no("Olmayan yüzey", {**new, "surface": "Yok"})
    no("Yeniden bağlanmaz", {"operation": "rebind", "platform": "p-1"})
    no("Değiştirilecek bir şey yok", {"operation": "set", "platform": "p-1"})
    no("Yeni platformun şevi yok", {k: v for k, v in new.items() if k != "cut"})
    no("Sıfır dolgu şevi", {**new, "fill": 0.0})
    no("Doksandan geniş yelpaze", {**new, "fan": 120.0})
    no("İki köşeli sınır", {**new, "corners": [[200.0, 0.0], [260.0, 0.0]]})
    no("Kendini kesen sınır", {**new, "corners": [[200.0, 0.0], [260.0, 40.0], [260.0, 0.0], [200.0, 40.0]]})
    no("Aynı yerde iki köşe", {**new, "corners": [[200.0, 0.0], [200.0, 0.0], [260.0, 40.0], [200.0, 40.0]]})
    no("Aynı ad", {**new, "name": "depo"})
    cases.append(case("Geçersiz beklenen sürüm", [step("execute", {**new, "expectedRevision": "on"}, BAD_REVISION, UNCHANGED)]))
    cases.append(case("Eski sürüme göre hazırlanmış", [step("execute", {**new, "expectedRevision": "41"}, CONFLICT, UNCHANGED)]))
    return cases


# ── road.platforms.query ──────────────────────────────────────────────────────────────────────────────────────────
def query_cases():
    cases = []

    def ok(name, input_, start=START, extra=None):
        out, warns = [], []
        for p in start if "platform" not in input_ else [find(start, input_["platform"])]:
            r, ws = read(p)
            out.append(r)
            warns += ws
        output = {"platforms": out}
        if extra is not None:
            found, more = extra
            if found is not None:
                output["balance"] = found
            warns += more
        cases.append(case(name, [step("execute", input_, completed(output, warns), {**UNCHANGED, "road": road_of(start)})],
                          None if start is START else setup(start)))

    def no(name, input_, result, start=START, setup_=None):
        cases.append(case(name, [step("execute", input_, result, UNCHANGED)],
                          setup_ if setup_ is not None else (None if start is START else setup(start))))

    def balanced(p, low, high, tol):
        b = balance(p, low, high, tol)
        if b is None:
            return None, [{"code": "platform_no_balance", "path": "balance", "message": ANY}]
        off, f, k, conv = b
        return {"offset": off, "elevation": p["base"][2] + off, "net": f, "iterations": k, "converged": conv}, []

    ok("Bütün platformlar", {})
    ok("Dolgu platformu", {"platform": "p-1"})
    ok("Adıyla, yarma platformu", {"platform": "YARMA"})
    ok("Eğimli platform", {"platform": "p-1"}, [{**P1, "grade": 2.0, "direction": 90.0}])
    ok("Eğimi doğuya dönük platform", {"platform": "p-1"}, [{**P1, "grade": 1.0, "direction": 0.0}])
    ok("Seyrek şev çizgileri", {"platform": "p-1"}, [{**P1, "step": 40.0}])
    ok("Sık şev çizgileri", {"platform": "p-1"}, [{**P1, "step": 2.5}])
    ok("Geniş yelpaze", {"platform": "p-1"}, [{**P1, "fan": 90.0}])
    ok("Beşgen platform", {"platform": "p-1"},
       [{**P1, "corners": [[0.0, 0.0], [40.0, 0.0], [50.0, 15.0], [40.0, 30.0], [0.0, 30.0]]}])
    ok("İçbükey köşeli platform", {"platform": "p-1"},
       [{**P1, "corners": [[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [20.0, 30.0], [20.0, 15.0], [0.0, 15.0]]}])
    ok("Düz arazide dolgu", {"platform": "p-1"}, [{**P1, "surface": "t-2", "base": [0.0, 0.0, 102.0]}])
    ok("Düz arazide yarma", {"platform": "p-1"}, [{**P1, "surface": "t-2", "base": [0.0, 0.0, 97.0]}])
    ok("Kazı ve dolgu birlikte", {"platform": "p-1"}, [{**P1, "base": [0.0, 0.0, 101.5]}])
    ok("Erimin ötesinde açık", {"platform": "p-1"}, [{**P1, "reach": 1.0}])
    ok("Arazinin dışına taşan açık", {"platform": "p-1"},
       [{**P1, "corners": [[380.0, 0.0], [398.0, 0.0], [398.0, 30.0], [380.0, 30.0]], "base": [380.0, 0.0, 130.0]}])
    ok("Hiç platform yok", {}, [])
    flat = {**P1, "surface": "t-2", "base": [0.0, 0.0, 101.0], "cut": 2.0, "fill": 2.0}
    ok("Düz arazide dengelenir", {"platform": "p-1", "balance": {"low": -3.0, "high": 2.0, "tolerance": 50.0}}, [flat],
       balanced(flat, -3.0, 2.0, 50.0))
    ok("Geniş toleransla ilk adımda", {"platform": "p-1", "balance": {"low": -3.0, "high": 2.0, "tolerance": 1e9}},
       [flat], balanced(flat, -3.0, 2.0, 1e9))
    ok("İşaret değişmeyen aralık: denge yok", {"platform": "p-1", "balance": {"low": 1.0, "high": 3.0, "tolerance": 50.0}},
       [flat], balanced(flat, 1.0, 3.0, 50.0))
    cases.append(case("Yol projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], setup(work="architecture")))
    # A map project grades its pads as a road project does (docs/adr/1580).
    cases.append({**copy.deepcopy(cases[1]), "name": "Harita projesinde de platform", "setup": setup(work="map")})
    no("Olmayan platform", {"platform": "yok"}, failed("unknown_platform", "“yok” kimlikli ya da adlı platform projede yok.", "platform"))
    no("Dengeleme tek platform ister", {"balance": {"low": -1.0, "high": 1.0, "tolerance": 1.0}},
       failed("invalid_platform", "Dengeleme tek platform içindir: platform verin.", "balance"))
    no("Ters aralık", {"platform": "p-1", "balance": {"low": 1.0, "high": -1.0, "tolerance": 1.0}},
       failed("invalid_platform", "Dengeleme aralığının alt ucu üst ucundan küçük, tolerans 0'dan büyük olmalı.", "balance"))
    no("Sıfır tolerans", {"platform": "p-1", "balance": {"low": -1.0, "high": 1.0, "tolerance": 0.0}},
       failed("invalid_platform", "Dengeleme aralığının alt ucu üst ucundan küçük, tolerans 0'dan büyük olmalı.", "balance"))
    return cases


# ── road.platforms.draw ───────────────────────────────────────────────────────────────────────────────────────────
def drawing(p, labels):
    r, warnings = read(p)
    pts = lambda ring: {"pts": [{"x": q[0], "y": q[1]} for q in ring], "zs": [q[2] for q in ring], "closed": True}  # noqa: E731
    objects = [pts(r["top"])]
    if r["daylight"]:
        objects.append(pts(r["daylight"]))
    for l in r["lines"]:
        to = l.get("to")
        if to is not None and math.hypot(to[0] - l["from"][0], to[1] - l["from"][1]) > 1e-9:
            objects.append({"a": {"x": l["from"][0], "y": l["from"][1]}, "b": {"x": to[0], "y": to[1]},
                            "za": l["from"][2], "zb": to[2]})
    if labels:
        n = len(r["top"])
        cx = sum(q[0] / n for q in r["top"])
        cy = sum(q[1] / n for q in r["top"])
        vol = ""
        if "cut" in r and r["cut"] != ANY:
            vol = f" · Kazı {comma(r['cut'], 1)} m³ · Dolgu {comma(r['fill'], 1)} m³"
        objects.append({"p": {"x": cx, "y": cy}, "rotation": 0.0, "height": 2.5,
                        "text": f"{p['name']} · Kot {comma(p['base'][2], 2)}{vol}"})
    return {"platform": p["id"]}, objects, warnings


def draw_cases():
    cases = []
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}

    def ok(name, input_, op="execute", undo=False, extra=(), start=START):
        p = find(start, input_["platform"])
        out, objects, warns = drawing(p, input_.get("labels", False))
        warns = list(extra) + warns
        out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road_of(start)}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road_of(start)}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road_of(start), "ids": list(range(1, FIRST + len(objects))),
                      "fields": {str(FIRST + i): o for i, o in enumerate(objects)}}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Platform çiz",
                          "expect": {"ids": list(range(1, FIRST)), "canRedo": True}})
        cases.append(case(name, steps, None if start is START else setup(start)))

    def no(name, input_, result, setup_=None):
        cases.append(case(name, [step("execute", input_, result, UNCHANGED)], setup_))

    base = {"platform": "p-1", "layerId": "platform"}
    ok("Dolgu platformu planda", base)
    ok("Yazısıyla", {**base, "labels": True})
    ok("Adıyla, yarma platformu yazısıyla", {**base, "platform": "YARMA", "labels": True})
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, extra=[hidden])
    ok("Gruptaki katmana", {**base, "layerId": "yol-platform"})
    ok("Planı", base, op="plan")
    ok("Yazılı planı", {**base, "labels": True}, op="plan")
    ok("Doğrulama", base, op="validate")
    ok("Tek geri alma adımı", base, undo=True)
    ok("Eğimli platform", {**base, "labels": True}, start=[{**P1, "grade": 2.0, "direction": 90.0}])
    ok("Beşgen platform", base, start=[{**P1, "corners": [[0.0, 0.0], [40.0, 0.0], [50.0, 15.0], [40.0, 30.0], [0.0, 30.0]]}])
    ok("İçbükey köşeli platform", base,
       start=[{**P1, "corners": [[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [20.0, 30.0], [20.0, 15.0], [0.0, 15.0]]}])
    ok("Açık çizgilerle: yalnız sınır", {**base, "labels": True}, start=[{**P1, "reach": 1.0}])
    ok("Kazı ve dolgu birlikte: hacimsiz yazı değil", base, start=[{**P1, "base": [0.0, 0.0, 101.5]}])
    no("Yol projesi değil", base, NOT_ROAD, setup(work="architecture"))
    no("Olmayan platform", {**base, "platform": "yok"},
       failed("unknown_platform", "“yok” kimlikli ya da adlı platform projede yok.", "platform"))
    no("Olmayan katman", {**base, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Katman grubu", {**base, "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
              "layerId"))
    no("Kilitli katman", {**base, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Geçersiz beklenen sürüm", {**base, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**base, "expectedRevision": "41"}, CONFLICT)
    return cases


def file(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_platform_cases.py yazar; arazi düzlem, günışığı kapalı biçimle, hacim her düz "
                    "parçanın alanı ile ağırlık merkezindeki kot farkından, denge ADR 0927'nin ikiye bölmesinden; KentOS "
                    "kodu yok. Şevler, aralıklar ve erimler durumlarındır, bir standardın değil.",
            "setup": setup(), "cases": cases}


def main():
    files = {"road.platforms.edit": file("road.platforms.edit", "Platformu düzenle", edit_cases()),
             "road.platforms.query": file("road.platforms.query", "Platformu sorgula", query_cases()),
             "road.platforms.draw": file("road.platforms.draw", "Platform çiz", draw_cases())}
    check = "--check" in sys.argv
    for name, data in files.items():
        path = OUT / f"{name}.json"
        data = road_grades.fixture(data)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if path.read_text(encoding="utf-8") != text:
                raise SystemExit(f"{path} güncel değil: python3 {Path(__file__).name}")
            print(f"{path.name}: {len(data['cases'])} durum güncel")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
