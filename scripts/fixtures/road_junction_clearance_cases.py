#!/usr/bin/env python3
"""The shared cases of `road.junctions.clearance` (plan/road.md Y17.4, docs/adr/0917; ROAD-JUNC-09).

The roads here are straight (two PIs each), so a place's station and offset on a road are its projection on the line;
the profiles are road_vertical_cases.py's reference (exact fractions, a circle in 40-digit mpmath). The overlap is
sampled by ADR 0917 §2's rule: the lower road's width and its stations each side of the crossing cut into equal steps
no longer than `step`, a grid place kept when its offset on the upper road lies within that road's edges; on every
grid line the upper road's edge solved where the line crosses it (here in closed form, the lines being straight). The
clearance is the upper profile less the deck's depth less the lower profile. No KentOS code. The edges, depths, steps
and minimums are the cases' own, not a standard's.

    uv run --no-project --with mpmath python3 scripts/fixtures/road_junction_clearance_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_vertical_cases as vref  # noqa: E402

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


def step(inp, result):
    return {"op": "execute", "input": inp, "result": result, "expect": UNCHANGED, "within": W}


# ── The project: straight roads, their profiles and junctions ─────────────────────────────────────────────────────
def al(id_, name, rev, a, b):
    return {"id": id_, "name": name, "revision": rev,
            "pis": [{"id": "a", "x": float(a[0]), "y": float(a[1])}, {"id": "b", "x": float(b[0]), "y": float(b[1])}]}


ALIGNMENTS = [
    al("g-1", "Otoyol", 1, (0, 0), (500, 0)),
    al("g-2", "Köprü yolu", 2, (250, -200), (250, 200)),
    al("g-3", "Eğik geçit", 1, (150, -100), (350, 100)),
    al("g-4", "Alt yol", 3, (420, 150), (380, -150)),
    al("g-5", "Kısa yol", 1, (100, -30), (100, 30)),
    al("g-6", "Profilsiz", 1, (50, -50), (50, 50)),
]


def pv(id_, s, z, **curve):
    return {"id": id_, "station": float(s), "elevation": float(z), **{k: float(v) for k, v in curve.items()}}


def prof(id_, name, alignment, rev, pvis):
    return {"id": id_, "name": name, "alignment": alignment, "alignmentRevision": rev, "revision": 1, "pvis": pvis}


PROFILES = [
    prof("b-1", "Otoyol kotu", "g-1", 1, [pv("a", 0, 100), pv("v1", 250, 101, length=200), pv("b", 500, 99)]),
    prof("b-2", "Köprü kotu", "g-2", 2, [pv("a", 0, 105), pv("v1", 200, 108.5, length=160), pv("b", 400, 106)]),
    prof("b-3", "Eğik kot", "g-3", 1, [pv("a", 0, 104), pv("b", 280, 110)]),
    prof("b-9", "Eş kot", "g-3", 1, [pv("a", 0, 100.7), pv("b", 280, 100.7)]),
    prof("b-4", "Alt kot", "g-4", 3, [pv("a", 0, 92), pv("v1", 150, 90, radius=2000), pv("b", 300, 93)]),
    prof("b-7", "Eski alt kot", "g-4", 2, [pv("a", 0, 92.5), pv("v1", 150, 90.5, radius=2000), pv("b", 300, 93.5)]),
    prof("b-8", "Taşan kot", "g-4", 3, [pv("a", 0, 92), pv("b", 400, 93)]),
    prof("b-5", "Kısa kot", "g-5", 1, [pv("a", 0, 100.5), pv("b", 60, 100.5)]),
    prof("b-6", "Yarım kot", "g-5", 1, [pv("a", 28, 104), pv("b", 60, 104)]),
]


def leg(alignment, rev, left, right):
    return {"alignment": alignment, "alignmentRevision": rev, "left": float(left), "right": float(right)}


def jn(id_, name, kind, legs, **extra):
    return {"id": id_, "name": name, "revision": 1, "kind": kind, **extra, "legs": legs}


JUNCTION_LIST = [
    jn("j-1", "Üst geçit", "separated", [leg("g-2", 2, 3.5, 3.5), leg("g-1", 1, 5, 5)]),
    jn("j-2", "Eğik kavşak", "separated", [leg("g-1", 1, 5, 5), leg("g-3", 1, 4, 4)]),
    jn("j-3", "Alt geçit", "separated", [leg("g-4", 3, 4, 4), leg("g-1", 1, 5, 5)]),
    jn("j-4", "Hemzemin", "atGrade", [leg("g-1", 1, 5, 5), leg("g-5", 1, 3, 3)], radius=10.0),
    jn("j-5", "Kısa geçit", "separated", [leg("g-5", 1, 3, 3), leg("g-1", 1, 5, 5)]),
    jn("j-6", "Eski geçit", "separated", [leg("g-1", 1, 5, 5), leg("g-3", 5, 4, 4)]),
    jn("j-8", "Profilsiz geçit", "separated", [leg("g-6", 1, 3, 3), leg("g-1", 1, 5, 5)]),
]


def setup(work="road"):
    s = copy.deepcopy(JUNCTIONS["setup"])
    s["settings"]["cadWork"] = work
    if work != "road":
        del s["settings"]["road"]
        return s
    s["settings"]["road"] = {"surfaces": s["settings"]["road"]["surfaces"], "alignments": copy.deepcopy(ALIGNMENTS),
                             "profiles": copy.deepcopy(PROFILES), "junctions": copy.deepcopy(JUNCTION_LIST)}
    return s


# ── Geometry ──────────────────────────────────────────────────────────────────────────────────────────────────────
class Line:
    def __init__(self, d):
        a, b = d["pis"][0], d["pis"][1]
        self.p = (a["x"], a["y"])
        self.dir = math.atan2(b["y"] - a["y"], b["x"] - a["x"])
        self.t = (math.cos(self.dir), math.sin(self.dir))
        self.length = math.hypot(b["x"] - a["x"], b["y"] - a["y"])

    def place(self, s, o):
        return (self.p[0] + s * self.t[0] + o * self.t[1], self.p[1] + s * self.t[1] - o * self.t[0])

    def locate(self, q):
        """Station and offset (right positive); None beyond the ends."""
        dx, dy = q[0] - self.p[0], q[1] - self.p[1]
        s = dx * self.t[0] + dy * self.t[1]
        if s < -1e-9 or s > self.length + 1e-9:
            return None
        return s, dx * self.t[1] - dy * self.t[0]


LINES = {a["id"]: Line(a) for a in ALIGNMENTS}


def crossing(a, b):
    """The stations of two lines where they cross."""
    la, lb = LINES[a], LINES[b]
    w = (lb.p[0] - la.p[0], lb.p[1] - la.p[1])
    den = la.t[0] * -lb.t[1] - la.t[1] * -lb.t[0]
    sa = (w[0] * -lb.t[1] - w[1] * -lb.t[0]) / den
    sb = (la.t[0] * w[1] - la.t[1] * w[0]) / den
    return sa, sb


def solved(p):
    return vref.solve([vref.pvi(v["id"], v["station"], v["elevation"],
                                vref.circ(v["radius"]) if "radius" in v else vref.par(v["length"]) if "length" in v
                                else None) for v in p["pvis"]])


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(words, path):
    return Refused("invalid_clearance", words, path)


def find(list_, name, path, what):
    for x in list_:
        if x["id"] == name:
            return x
    for x in list_:
        if x["name"].lower() == name.lower():
            return x
    raise Refused(f"unknown_{what}", f"“{name}” kimlikli ya da adlı {'kavşak' if what == 'junction' else 'boykesit'} "
                                     "projede yok.", path)


def fmt3(v):
    return f"{v:.3f}"


def answer(inp, road=True):
    if not road:
        raise Refused(NOT_ROAD["error"]["code"], NOT_ROAD["error"]["message"])
    j = find(JUNCTION_LIST, inp["junction"], "junction", "junction")
    warns = []
    a_ids = [lg["alignment"] for lg in j["legs"]]
    for lg in j["legs"]:
        al_ = next(a for a in ALIGNMENTS if a["id"] == lg["alignment"])
        if al_["revision"] != lg["alignmentRevision"]:
            warns.append({"code": "junction_stale",
                          "message": f"“{j['name']}” kavşağının “{al_['name']}” aksı değişti; kavşağı denetleyip yeniden "
                                     "bağlayın.", "path": "junction"})
    if j["kind"] == "atGrade":
        raise Refused("invalid_junction", f"“{j['name']}” hemzemin kavşak; düşey açıklık yalnız ayrı seviyeli kavşakta "
                                          "olur.", "junction")
    if "profiles" in inp:
        chosen = [find(PROFILES, n, f"profiles[{k}]", "road_profile") for k, n in enumerate(inp["profiles"])]
    else:
        chosen = []
        for aid in a_ids:
            on = [p for p in PROFILES if p["alignment"] == aid]
            name = next(a["name"] for a in ALIGNMENTS if a["id"] == aid)
            if not on:
                raise bad(f"“{name}” aksının boykesiti yok; profiles ile verin.", "profiles")
            if len(on) > 1:
                raise bad(f"“{name}” aksının {len(on)} boykesiti var; hangisi, profiles ile verin.", "profiles")
            chosen.append(on[0])
    solvers = []
    for p in chosen:
        length = LINES[p["alignment"]].length
        for i, v in enumerate(p["pvis"]):
            if v["station"] < -1e-9 or v["station"] > length + 1e-9:
                raise Refused("invalid_road_profile",
                              f"{v['id']} PVI'si aksın dışında: aksın km'si 0 ile {length:.3f} m arası.",
                              f"pvis[{i}].station")
        solvers.append(solved(p)[1])
    if len(chosen) != 2:
        raise bad(f"Kolların sırasıyla iki boykesit verin; {len(chosen)} verildi.", "profiles")
    for k, (p, aid) in enumerate(zip(chosen, a_ids)):
        if p["alignment"] != aid:
            raise bad(f"“{p['name']}” boykesiti kavşağın {k + 1}. kolunun aksında değil.", f"profiles[{k}]")
    depth, step_, minimum = inp["depth"], inp["step"], inp.get("minimum")
    if not depth >= 0:
        raise bad("Tabliye derinliği (depth) 0 ya da daha büyük olmalı.", "depth")
    if not step_ > 0:
        raise bad("Örnekleme aralığı (step) 0'dan büyük olmalı.", "step")
    if minimum is not None and not minimum >= 0:
        raise bad("Gerekli açıklık (minimum) 0 ya da daha büyük olmalı.", "minimum")
    for p in chosen:
        now = next(a["revision"] for a in ALIGNMENTS if a["id"] == p["alignment"])
        if now != p["alignmentRevision"]:
            warns.append({"code": "profile_stale",
                          "message": f"“{p['name']}” boykesitinin aksı değişti (boykesit {p['alignmentRevision']}. "
                                     f"revizyona göre kurulu, aks şimdi {now}.); PVI'ları denetleyip aksa yeniden "
                                     "bağlayın.", "path": "profiles"})
    s0, s1 = crossing(a_ids[0], a_ids[1])
    stations = [s0, s1]

    def height(k, s):
        r = solvers[k](s)
        return None if r is None else r[0]

    h0, h1 = height(0, s0), height(1, s1)
    if h0 is None or h1 is None:
        raise bad("Bir boykesit aksların kesişimini örtmüyor.", "profiles")
    if abs(h0 - h1) < 1e-9:
        raise bad(f"İki kol kesişimde aynı kotta ({h0:.3f} m); biri üstten geçmeli.", "profiles")
    up = 1 if h1 > h0 else 0
    lo = 1 - up
    U, L = LINES[a_ids[up]], LINES[a_ids[lo]]
    lu, ru = j["legs"][up]["left"], j["legs"][up]["right"]
    ll, rl = j["legs"][lo]["left"], j["legs"][lo]["right"]
    # The angle between the legs as the reading takes it: acos of the tangents' dot product, in degrees and back.
    ta, tb = LINES[a_ids[0]].t, LINES[a_ids[1]].t
    angle = math.degrees(math.acos(max(-1.0, min(1.0, ta[0] * tb[0] + ta[1] * tb[1]))))
    sine = math.sin(math.radians(angle))
    reach = (lu + ru + ll + rl) / abs(sine) + step_
    c = stations[lo]
    g0, g1 = max(c - reach, 0.0), min(c + reach, L.length)
    n = int(max(math.ceil((g1 - g0) / step_), 1.0)) + 1
    m = int(max(math.ceil((ll + rl) / step_), 1.0)) + 1
    if n * m > MOST:
        raise bad(f"Aralık çok küçük: {n} × {m} ızgara noktası; en çok {MOST}.", "step")

    def st(i):
        return g1 if i + 1 == n else g0 + (g1 - g0) * i / (n - 1)

    def of(jj):
        return rl if jj + 1 == m else -ll + (ll + rl) * jj / (m - 1)

    def on_upper(s, o):
        q = L.place(s, o)
        f = U.locate(q)
        return None if f is None else (q, f[0], f[1], s, o)

    def inside(p):
        return -lu - 1e-9 <= p[2] <= ru + 1e-9

    grid = [[on_upper(st(i), of(jj)) for jj in range(m)] for i in range(n)]
    places = [p for row in grid for p in row if p is not None and inside(p)]

    def cross(a, b):
        pa, pb = a[2], b[2]
        if pa is None or pb is None:
            return
        for e in (-lu, ru):
            if (pa[2] - e) * (pb[2] - e) >= 0:
                continue
            # The upper offset is linear along a straight grid line.
            t = (e - pa[2]) / (pb[2] - pa[2])
            q = on_upper(a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]))
            places.append(q)

    for jj in range(m):
        for i in range(n - 1):
            cross((st(i), of(jj), grid[i][jj]), (st(i + 1), of(jj), grid[i + 1][jj]))
    for i in range(n):
        for jj in range(m - 1):
            cross((st(i), of(jj), grid[i][jj]), (st(i), of(jj + 1), grid[i][jj + 1]))
    if not places:
        raise bad("Yollar planda örtüşmüyor.", "junction")

    def at(q, su, ou, sl, ol):
        hu, hl = height(up, su), height(lo, sl)
        if hu is None or hl is None:
            return None
        return {"x": q[0], "y": q[1], "upperStation": su, "upperOffset": ou, "lowerStation": sl, "lowerOffset": ol,
                "upperElevation": hu, "lowerElevation": hl, "clearance": hu - depth - hl}

    read = []
    for p in places:
        r = at(*p)
        if r is None:
            raise bad(f"Boykesitler örtüşme bölgesini örtmüyor ({p[3]:.3f} m km'sinde).", "profiles")
        read.append(r)
    least = min(read, key=lambda r: r["clearance"])
    largest = max(read, key=lambda r: r["clearance"])
    for best in (least, largest):
        if sum(1 for r in read if abs(r["clearance"] - best["clearance"]) < 1e-9) > 1:
            for k in ("x", "y", "upperStation", "upperOffset", "lowerStation", "lowerOffset", "upperElevation",
                      "lowerElevation"):
                best[k] = ANY
    xp = LINES[a_ids[0]].place(s0, 0.0)
    cross_at = at(xp, stations[up], 0.0, stations[lo], 0.0)
    short = minimum is not None and least["clearance"] < minimum
    if short:
        warns.append({"code": "junction_clearance_short",
                      "message": f"“{j['name']}” kavşağında en küçük düşey açıklık {least['clearance']:.3f} m, istenen "
                                 f"{minimum:.3f} m'den az.", "path": "minimum"})
    out = {"junction": j["id"], "upper": up, "upperProfile": chosen[up]["id"], "lowerProfile": chosen[lo]["id"],
           "depth": depth, "step": step_, "samples": len(read), "least": least, "largest": largest,
           "crossing": cross_at}
    if minimum is not None:
        out["minimum"] = minimum
    out["short"] = short
    return out, warns


def cases():
    out = []

    def q(name, inp, map_=False):
        try:
            o, w = answer(inp, road=not map_)
            result = completed(o, w)
        except Refused as r:
            result = r.result
        c = {"name": name, "steps": [step(inp, result)]}
        if map_:
            c["setup"] = "$map"
        out.append(c)

    q("Üst geçit, aksların tek boykesitleri", {"junction": "j-1", "depth": 1.2, "step": 1.0})
    q("Adıyla, boykesitler verilerek", {"junction": "üst geçit", "profiles": ["b-2", "Otoyol kotu"], "depth": 1.2,
                                        "step": 0.8, "minimum": 5.0})
    q("Gerekli açıklıktan az", {"junction": "j-1", "depth": 1.2, "step": 1.0, "minimum": 6.0})
    q("Tabliyesiz, iri aralık", {"junction": "j-1", "depth": 0.0, "step": 2.0})
    q("Eğik geçit 45°", {"junction": "j-2", "profiles": ["b-1", "b-3"], "depth": 1.5, "step": 0.7})
    q("Eğik geçit ince aralık", {"junction": "j-2", "profiles": ["b-1", "b-3"], "depth": 1.5, "step": 0.3,
                                 "minimum": 4.0})
    q("Alt yol dairesel kurpta", {"junction": "j-3", "profiles": ["b-4", "b-1"], "depth": 1.0, "step": 0.9})
    q("Eski boykesitle", {"junction": "j-3", "profiles": ["b-7", "b-1"], "depth": 1.0, "step": 1.3})
    q("Tabliyeler çakışıyor", {"junction": "j-5", "profiles": ["b-5", "b-1"], "depth": 0.6, "step": 0.9,
                               "minimum": 0.0})
    q("Kolu değişmiş kavşak", {"junction": "j-6", "profiles": ["b-1", "b-3"], "depth": 1.0, "step": 1.1})
    # Refusals.
    q("Olmayan kavşak", {"junction": "j-99", "depth": 1.0, "step": 1.0})
    q("Hemzemin kavşakta açıklık yok", {"junction": "j-4", "depth": 1.0, "step": 1.0})
    q("Olmayan boykesit", {"junction": "j-1", "profiles": ["yok", "b-1"], "depth": 1.0, "step": 1.0})
    q("Kurulamayan boykesit", {"junction": "j-3", "profiles": ["b-8", "b-1"], "depth": 1.0, "step": 1.0})
    q("Tek boykesit", {"junction": "j-1", "profiles": ["b-2"], "depth": 1.0, "step": 1.0})
    q("Boykesit kolun aksında değil", {"junction": "j-1", "profiles": ["b-1", "b-2"], "depth": 1.0, "step": 1.0})
    q("Aksın iki boykesiti", {"junction": "j-2", "depth": 1.0, "step": 1.0})
    q("Aksın boykesiti yok", {"junction": "j-8", "depth": 1.0, "step": 1.0})
    q("Eksi derinlik", {"junction": "j-1", "depth": -0.5, "step": 1.0})
    q("Aralık sıfır", {"junction": "j-1", "depth": 1.0, "step": 0.0})
    q("Eksi gerekli açıklık", {"junction": "j-1", "depth": 1.0, "step": 1.0, "minimum": -1.0})
    q("Kollar kesişimde aynı kotta", {"junction": "j-2", "profiles": ["b-1", "b-9"], "depth": 1.0, "step": 1.0})
    q("Çok küçük aralık", {"junction": "j-1", "depth": 1.0, "step": 0.0078125})
    q("Boykesit örtüşmeyi örtmüyor", {"junction": "j-5", "profiles": ["b-6", "b-1"], "depth": 1.0, "step": 0.9})
    q("Yol projesi değil", {"junction": "j-1", "depth": 1.0, "step": 1.0}, map_=True)
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="map")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.junctions.clearance", "commandVersion": 1,
           "title": "Kavşağın düşey açıklığı: üst yolun altındaki en küçük ve en büyük açıklık",
           "note": "scripts/fixtures/road_junction_clearance_cases.py'den; düz aksların doğrularında izdüşüm, boykesitler "
                   "road_vertical_cases.py'nin kesin kesirleri ve dairesi, örnekleme ADR 0917 §2'nin kuralıyla; KentOS "
                   "kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.junctions.clearance.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.junctions.clearance.json")
    else:
        path.write_text(text)
    print(f"road.junctions.clearance.json: {len(found)} durum")


if __name__ == "__main__":
    main()
