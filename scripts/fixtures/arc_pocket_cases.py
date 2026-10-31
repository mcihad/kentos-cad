#!/usr/bin/env python3
"""Pocket doors in arc walls and the pocket's void in the quantities (docs/adr/0787 §2, §3), from the ADR's rules,
without KentOS code.

Writes fixtures/walls/v1/arc-pockets.json (the shared core's drawing: `kentos-geometry-core`, tests/all/openings.rs),
fixtures/commands/v1/cad.openings.create.arcpocket.json and cad.spaces.quantities.pockets.json; `--check` compares and
writes nothing.

An arc wall's base line is an arc of radius R about C from angle α0, sweeping Φ (counter-clockwise positive, its sign
σ); the point s along it is at θ(s) = α0 + σ·s/R, and the point d to its left at radius R − σ·d (the left of a
counter-clockwise arc is its inside). The wall's left face is d = lf (its offset, plus 0, t/2 or t as it is justified
left, centre or right), its right face lf − t. The rules, as ADR 0787 gives them (ADR 0777 §2's, along the arc):

- An opening's place along its wall is its base line's arc length, as every opening's (ADR 0341): [at − w/2, at + w/2].
  Its jambs are radial: the start's left and right face points, then the end's.
- A pocket door's pocket runs from the jamb at its `hand` (two leaves: each at its own) w − lip − fit − frame + 1 cm
  along the base line (w its leaf: the rough width less the fitting gaps and frames, two split as given), tl + 2 cm
  across, centred in the wall; it follows the arc. Its wall must be the pocket and 2,5 cm each side thick
  (`opening_pocket_thin`) and the door and its pocket must lie within the wall's arc length (`opening_pocket_short`).
- The wall draws as a straight wall's would (ADR 0777 §2): its faces broken over the door, the jamb a pocket opens at
  keeping only the wall each side of the cavity, the cavity's far end its own cap, its sides its inner lines; on an arc
  all of these are arcs about C.
- Its leaves shut: the frame stands as on any arc wall (on the chord between the jambs' middles), the leaves are bands
  about C, tl wide about the band's middle radius (the start jamb's middle's distance from C), from the radial line
  through the light's start to the one through its end (the light's ends on the band's middle line), two split by
  angle; each a closed ring: inner arc, end radial, outer arc back, start radial.
- Its void (§3): each pocket's length × its width × (its door's head − sill), on an arc scaled to the wall's middle
  ((R − σ·m)/R, m the wall's middle's distance left of its base line), is taken off its wall's volume; the wall's faces
  keep it (the gross and net faces are as before) and the wall's rule says so.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import special_doors_command_cases as sd  # noqa: E402  (ADR 0777's sizes and refusals, written without KentOS code)

ROOT = Path(__file__).resolve().parents[2]
GEOMETRY = ROOT / "fixtures/walls/v1/arc-pockets.json"
COMMANDS = ROOT / "fixtures/commands/v1"
PLAY, LIP, LEAF, SKIN = 0.01, 0.05, 0.04, 0.025


def P(x, y):
    return {"x": float(x), "y": float(y)}


# ── An arc wall ──────────────────────────────────────────────────────────────────────────────────────────────────


class Arc:
    def __init__(self, c, r, a0, sweep, t, justify="center"):
        self.c, self.r, self.a0, self.sweep, self.t, self.justify = c, r, a0, sweep, t, justify
        self.sg = 1.0 if sweep > 0 else -1.0
        self.length = r * abs(sweep)
        self.lf = {"left": 0.0, "center": t / 2.0, "right": t}[justify]

    def at(self, s, d):
        th = self.a0 + self.sg * s / self.r
        rr = self.r - self.sg * d
        return (self.c[0] + rr * math.cos(th), self.c[1] + rr * math.sin(th))

    def pts(self):
        return [self.at(0.0, 0.0), self.at(self.length, 0.0)]

    def bulge(self):
        return math.tan(self.sweep / 4.0)

    def json(self):
        a, b = self.pts()
        out = {"pts": [list(a), list(b)], "bulges": [self.bulge(), 0.0], "width": self.t}
        if self.justify != "center":
            out["justify"] = self.justify
        return out

    def stroke(self, s0, s1, d):
        """The wall's line `d` left of its base line from s0 to s1: an arc about C."""
        return {"pts": [list(self.at(s0, d)), list(self.at(s1, d))],
                "bulges": [math.tan(self.sg * (s1 - s0) / self.r / 4.0), 0.0]}

    def sector(self, s0, s1, d0, d1):
        """The area between the lines d0 and d1 from s0 to s1."""
        ra, rb = self.r - self.sg * d0, self.r - self.sg * d1
        return (s1 - s0) / self.r / 2.0 * abs(ra * ra - rb * rb)


# ── A pocket door ────────────────────────────────────────────────────────────────────────────────────────────────


class Pocket:
    def __init__(self, at, width, leaves=1, hand="start", side="left", frame=0.0, fit=0.0, lip=None, leaf=None,
                 split=None):
        self.at, self.width, self.leaves, self.hand, self.side = at, width, leaves, hand, side
        self.frame, self.fit, self.lip, self.leaf, self.split = frame, fit, lip, leaf, split

    def json(self):
        out = {"wall": 0, "at": self.at, "width": self.width, "form": "door", "operation": "pocket"}
        if self.leaves != 1:
            out["leaves"] = self.leaves
        if self.hand != "start":
            out["hand"] = self.hand
        if self.side != "left":
            out["side"] = self.side
        if self.frame:
            out["frame"] = self.frame
        b = {}
        for k, v in (("fit", self.fit or None), ("lip", self.lip), ("leafThickness", self.leaf), ("split", self.split)):
            if v is not None:
                b[k] = v
        if b:
            out["build"] = b
        return out

    def span(self):
        return self.at - self.width / 2.0, self.at + self.width / 2.0

    def pockets(self):
        """(at its end, length, width) of each pocket."""
        light = self.width - 2 * self.fit - 2 * self.frame
        t = self.leaf if self.leaf is not None else LEAF
        lip = self.lip if self.lip is not None else LIP

        def one(end, w):
            return end, max(w - lip - self.fit - self.frame + PLAY, 0.0), t + 2 * PLAY
        if self.leaves == 2:
            share = self.split if self.split is not None else 0.5
            return [one(False, light * share), one(True, light * (1 - share))]
        return [one(self.hand == "end", light)]


def mid(a, b):
    return ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0)


def drawing(w, o):
    """The door's jambs, its symbol, its wall's lines, caps, inner lines and body's area."""
    s0, s1 = o.span()
    t, lf = w.t, w.lf
    jambs = [w.at(s0, lf), w.at(s0, lf - t), w.at(s1, lf), w.at(s1, lf - t)]
    # The cavity across the wall, as distances left of the base line.
    pk = o.pockets()
    dw = pk[0][2]
    near, far = (t - dw) / 2.0, (t + dw) / 2.0
    lo, hi = lf - far, lf - near
    holes = []  # (s0, s1, through, open0, open1)
    main = [s0, s1, True, False, False]
    for end, length, _ in pk:
        if length <= 0:
            continue
        if end:
            main[4] = True
            holes.append([s1, s1 + length, False, True, False])
        else:
            main[3] = True
            holes.append([s0 - length, s0, False, False, True])
    holes.insert(0, main)
    caps = [[list(w.at(0, lf)), list(w.at(0, lf - t))], [list(w.at(w.length, lf)), list(w.at(w.length, lf - t))]]
    recesses = []
    for a, z, through, o0, o1 in holes:
        for s, opened in ((a, o0), (z, o1)):
            if through:
                if opened:
                    caps.append([list(w.at(s, lf)), list(w.at(s, hi))])
                    caps.append([list(w.at(s, lo)), list(w.at(s, lf - t))])
                else:
                    caps.append([list(w.at(s, lf)), list(w.at(s, lf - t))])
            elif not opened:
                caps.append([list(w.at(s, hi)), list(w.at(s, lo))])
        if not through:
            recesses.append(w.stroke(a, z, hi))
            recesses.append(w.stroke(a, z, lo))
    lines = [[w.stroke(0.0, s0, d), w.stroke(s1, w.length, d)] for d in (lf, lf - t)]
    body = w.sector(0.0, w.length, lf, lf - t) - w.sector(s0, s1, lf, lf - t)
    for a, z, through, _, _ in holes[1:]:
        body -= w.sector(a, z, hi, lo)
    # Its symbol: the frame on the chord, the leaves about C.
    m0, m1 = mid(jambs[0], jambs[1]), mid(jambs[2], jambs[3])
    L = math.hypot(m1[0] - m0[0], m1[1] - m0[1])
    u = ((m1[0] - m0[0]) / L, (m1[1] - m0[1]) / L)
    tt = math.hypot(jambs[0][0] - jambs[1][0], jambs[0][1] - jambs[1][1])
    half = L / 2.0 * 0.999
    fit = min(max(o.fit, 0.0), half)
    fr = min(max(o.frame, 0.0), max(half - fit, 0.0))
    clear = L - 2 * fr - 2 * fit

    def across(l, r, d):
        if d == 0.0:
            return l
        if d == tt:
            return r
        if d == tt / 2.0:
            return mid(l, r)
        k = d / tt if tt > 0 else 0.0
        return (l[0] + (r[0] - l[0]) * k, l[1] + (r[1] - l[1]) * k)

    def start(s, d):
        p = across(jambs[0], jambs[1], d)
        return (p[0] + u[0] * s, p[1] + u[1] * s)

    def end(s, d):
        p = across(jambs[2], jambs[3], d)
        return (p[0] - u[0] * s, p[1] - u[1] * s)
    symbol = []
    if fr > 0:
        symbol.append({"kind": "ring", "pts": [list(start(fit, 0.0)), list(start(fit + fr, 0.0)),
                                                 list(start(fit + fr, tt)), list(start(fit, tt))]})
        symbol.append({"kind": "ring", "pts": [list(end(fit, 0.0)), list(end(fit + fr, 0.0)),
                                                 list(end(fit + fr, tt)), list(end(fit, tt))]})
    c = w.c
    mm = across(jambs[0], jambs[1], tt / 2.0)
    rmid = math.hypot(mm[0] - c[0], mm[1] - c[1])
    e0, e1 = start(fit + fr, tt / 2.0), start(fit + fr + clear, tt / 2.0)
    th0 = math.atan2(e0[1] - c[1], e0[0] - c[0])
    d = math.atan2(e1[1] - c[1], e1[0] - c[0]) - th0
    if d > math.pi:
        d -= 2 * math.pi
    elif d <= -math.pi:
        d += 2 * math.pi
    tl = o.leaf if o.leaf is not None else LEAF
    r1, r2 = rmid - tl / 2.0, rmid + tl / 2.0

    def pt(rr, th):
        return [c[0] + rr * math.cos(th), c[1] + rr * math.sin(th)]

    def band(a, z):
        q = math.tan((z - a) / 4.0)
        return {"kind": "ring", "pts": [pt(r1, a), pt(r1, z), pt(r2, z), pt(r2, a)], "bulges": [q, 0.0, -q, 0.0]}
    if o.leaves == 2:
        share = o.split if o.split is not None else 0.5
        m = th0 + d * share
        symbol += [band(th0, m), band(m, th0 + d)]
    else:
        symbol.append(band(th0, th0 + d))
    return {"jambs": [list(p) for p in jambs], "symbol": symbol, "lines": lines, "caps": caps, "recesses": recesses,
            "body": body}


GEOMETRY_CASES = [
    ("Yaylı duvar (R 5 m, saat yönü tersine), cep başta", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3),
     Pocket(3.0, 0.9)),
    ("Cep sonda (menteşe sonda)", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3), Pocket(3.0, 0.9, hand="end")),
    ("Kasalı ve montaj boşluklu cep", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3),
     Pocket(3.5, 1.0, frame=0.04, fit=0.01)),
    ("Kalın kanat ve büyük pay", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3),
     Pocket(3.0, 0.9, frame=0.03, lip=0.08, leaf=0.05)),
    ("İki kanat: iki cep, iki yana", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3),
     Pocket(3.9, 1.6, leaves=2, frame=0.04)),
    ("İki eşit olmayan kanat", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3),
     Pocket(3.9, 1.6, leaves=2, frame=0.04, split=0.6)),
    ("Saat yönündeki yay: duvarın solu dışı", Arc((10.0, 2.0), 4.0, math.pi / 2, -math.pi / 2, 0.25),
     Pocket(3.0, 0.8, frame=0.03)),
    ("Saat yönünde, cep sonda", Arc((10.0, 2.0), 4.0, math.pi / 2, -math.pi / 2, 0.25),
     Pocket(2.6, 0.8, frame=0.03, hand="end")),
    ("Sola yaslı duvar", Arc((0.0, 0.0), 6.0, 0.2, 1.2, 0.3, "left"), Pocket(3.0, 0.9, frame=0.04)),
    ("Sağa yaslı duvar", Arc((0.0, 0.0), 6.0, 0.2, 1.2, 0.3, "right"), Pocket(3.0, 0.9, frame=0.04)),
    ("Dar yay (R 2,5 m)", Arc((-3.0, 1.0), 2.5, -0.4, 2.4, 0.3), Pocket(3.0, 0.8, frame=0.03)),
    ("Geniş yay (R 40 m)", Arc((0.0, -40.0), 40.0, 1.45, 0.25, 0.2), Pocket(5.0, 0.9, frame=0.04)),
    ("Büyük koordinatta yay", Arc((500000.0, 4400000.0), 7.0, 0.3, 1.0, 0.3), Pocket(3.0, 0.9, frame=0.04)),
    ("Öbür yüze açılan cep (yan sağ)", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3), Pocket(3.0, 0.9, side="right")),
    ("Yarım daire yay, kapı tepede", Arc((0.0, 0.0), 4.5, 0.0, math.pi, 0.3), Pocket(7.6, 0.9, frame=0.04)),
    ("Yarım daire, iki kanat", Arc((0.0, 0.0), 4.5, 0.0, math.pi, 0.3), Pocket(7.0, 1.4, leaves=2, frame=0.04)),
    ("Saat yönü, iki kanat, sola yaslı", Arc((2.0, 2.0), 6.0, 2.0, -1.5, 0.3, "left"),
     Pocket(4.5, 1.6, leaves=2, frame=0.04, split=0.4)),
    ("İnce kanat, 0,15 m duvar", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.15), Pocket(3.0, 0.8, leaf=0.03)),
    ("Kasasız cep", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3), Pocket(4.0, 0.7)),
    ("Duvarın başına yakın cep", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3), Pocket(1.4, 0.9, hand="end")),
    ("Duvarın sonuna yakın cep", Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3), Pocket(6.4, 0.9)),
]


def geometry():
    cases = []
    for name, w, o in GEOMETRY_CASES:
        c = {"name": name, "wall": w.json(), "opening": o.json()}
        c.update(drawing(w, o))
        cases.append(c)
    return {"format": "kentos.arc-pockets", "version": 1,
            "note": "ADR 0787 §2. Beklenenler scripts/fixtures/arc_pocket_cases.py'nin ADR'den yazılmış kurallarından, "
                    "KentOS kodu olmadan.",
            "cases": cases}


# ── Commands ─────────────────────────────────────────────────────────────────────────────────────────────────────


def UID(n):
    return f"0192f5a1-7870-7000-8000-{n:012x}"


# The walls: an arc wall (R 5 m, a quarter, 0,3 m), a thin arc wall (0,1 m), a clockwise left-justified one, a
# straight wall.
ARCS = {
    1: Arc((0.0, 0.0), 5.0, 0.0, math.pi / 2, 0.3),
    2: Arc((20.0, 0.0), 5.0, 0.0, math.pi / 2, 0.1),
    3: Arc((40.0, 2.0), 6.0, 2.0, -1.5, 0.3, "left"),
}
STRAIGHT = 4  # (0, -10) → (12, -10), 0.3 m
HEIGHTS = {1: 3.0, 2: 2.8, 3: 3.0, 4: 3.0}


def wall_entity(i):
    if i == STRAIGHT:
        e = {"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(0, -10), P(12, -10)], "width": 0.3,
             "justify": "center"}
    else:
        w = ARCS[i]
        a, b = w.pts()
        e = {"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)],
             "bulges": [w.bulge(), 0.0], "width": w.t, "justify": w.justify}
    e["height"] = HEIGHTS[i]
    return e


def wall_length(i):
    return 12.0 if i == STRAIGHT else ARCS[i].length


def thickness(i):
    return 0.3 if i == STRAIGHT else ARCS[i].t


def place(host, at, width):
    s0, s1 = at - width / 2, at + width / 2
    if host == STRAIGHT:
        return [P(s0, -9.85), P(s0, -10.15), P(s1, -9.85), P(s1, -10.15)]
    w = ARCS[host]
    return [P(*w.at(s0, w.lf)), P(*w.at(s0, w.lf - w.t)), P(*w.at(s1, w.lf)), P(*w.at(s1, w.lf - w.t))]


def stored(o, id):
    e = sd.stored(o, id)
    e["place"] = place(o["host"], o["at"], o["width"])
    return e


def on_wall(o, path):
    """A pocket door its arc wall cannot take: too thin, or its pocket past its wall's arc length."""
    if o.get("operation") != "pocket":
        return None
    t = thickness(o["host"])
    need = max(p["width"] for p in sd.sizes(o)["pockets"]) + 2 * SKIN
    if need > t + 1e-9 * max(t, 1.0):
        return sd.failed("opening_pocket_thin",
                         f"Cebe giren kapının cebi için duvar en az {need:.3f} m kalınlığında olmalı (cep ve iki yanında 2,5 cm duvar); bu duvar {t:.3f} m. Daha kalın bir duvar seçin, kanadı inceltin ya da kayar kapı kullanın.",
                         f"{path}.operation")
    r0, r1 = sd.reach(o)
    n = wall_length(o["host"])
    tol = 1e-9 * max(n, 1.0)
    if r0 < -tol or r1 > n + tol:
        return sd.failed("opening_pocket_short",
                         f"Cebe giren kapının cebi duvarın ucundan ya da köşesinden taşıyor: kapı ve cebi duvar boyunca {r0:.3f}–{r1:.3f} m, duvarın uzunluğu {n:.3f} m; cep kapıyla aynı kenarda kalmalı. Kapıyı kaydırın, menteşe yanını değiştirin ya da daraltın.",
                         f"{path}.at")
    return None


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


CG1 = {"id": "cg1", "name": "CG1", "version": 1, "form": "door", "width": 0.9, "operation": "pocket", "frame": 0.03,
       "head": 2.1, "build": {"leafThickness": 0.04, "lip": 0.06}}
# The doors already there: a pocket door on the arc, a swing door on the arc, a pocket door on the straight wall.
BASE = {
    5: sd.from_type(CG1, 1, 5.5, number="CG1"),
    6: sd.from_kapi(1, 1.2, number="K1"),
    7: sd.from_type(CG1, STRAIGHT, 6.0, number="CG2"),
}
IDS = [1, 2, 3, 4] + sorted(BASE)
NEW = max(IDS) + 1


def setup():
    ents = [wall_entity(i) for i in (1, 2, 3, 4)]
    for id in sorted(BASE):
        e = stored(BASE[id], id)
        e["host"] = UID(BASE[id]["host"])
        ents.append(e)
    return {
        "format": "kentos.document", "version": 2, "name": "Yaylı duvarda cep",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad", "arch": {"openingTypes": [sd.type_json(CG1)]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı")],
        "activeLayer": "kapi",
        "styles": {"items": [], "categories": []},
        "entities": ents,
    }


def doc(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": setup(), "cases": cases}


NOTE = ("ADR 0787. Beklenenler scripts/fixtures/arc_pocket_cases.py'nin ADR'den yazılmış kurallarından (ölçüler ve "
        "retler ADR 0777'nin special_doors_command_cases.py'sinden), KentOS kodu olmadan. Kurulumda üç yaylı duvar "
        "(R 5 m çeyrek 0,30 m; R 5 m 0,10 m; saat yönünde sola yaslı R 6 m) ve bir düz duvar, yaylı duvarda CG1 cebe "
        "giren ve Kapı 90, düz duvarda CG1; yeni nesne 8'den başlar.")


def create_cases():
    out = []
    H = lambda id: f"$uidOf:{id}"  # noqa: E731

    def add(name, given, made):
        path = "openings[0]"
        why = sd.problem(made, path) or on_wall(made, path) or sd.into_others(made, BASE.values(), path)
        step = {"op": "execute", "input": {"layerId": "kapi", "openings": [given]}, "within": 1e-9}
        if why:
            step["result"] = why
            step["expect"] = {"ids": IDS, "revision": "same"}
        else:
            step["result"] = {"status": "completed", "output": {"uids": ["$any"], "ids": [NEW], "revision": "$current"},
                              "warnings": []}
            step["expect"] = {"ids": IDS + [NEW], "entities": {str(NEW): stored(made, NEW)}, "canUndo": True,
                              "dirty": True, "revision": "changed"}
        out.append({"name": name, "steps": [step]})

    add("Yaylı duvarda CG1: cep yay boyunca, başta", {"host": H(1), "at": 3.4, "type": "CG1"}, sd.from_type(CG1, 1, 3.4))
    add("Yaylı duvarda CG1, menteşe sonda", {"host": H(1), "at": 3.0, "type": "CG1", "hand": "end"},
        sd.from_type(CG1, 1, 3.0, hand="end"))
    add("Menteşe sonda: cebi öbür CG1'in cebine giriyor", {"host": H(1), "at": 3.4, "type": "CG1", "hand": "end"},
        sd.from_type(CG1, 1, 3.4, hand="end"))
    add("Tipsiz cebe giren kapı yaylı duvarda", {"host": H(1), "at": 3.4, "operation": "pocket"},
        sd.from_kapi(1, 3.4, operation="pocket"))
    add("İki kanatlı cep yaylı duvarda", {"host": H(1), "at": 2.95, "width": 1.4, "operation": "pocket", "leaves": 2},
        sd.from_kapi(1, 2.95, width=1.4, operation="pocket", leaves=2))
    add("İki kanatlı geniş cep iki yandaki kapılara giriyor",
        {"host": H(1), "at": 3.6, "width": 1.6, "operation": "pocket", "leaves": 2},
        sd.from_kapi(1, 3.6, width=1.6, operation="pocket", leaves=2))
    add("Saat yönündeki sola yaslı yayda cep", {"host": H(3), "at": 4.0, "type": "CG1"}, sd.from_type(CG1, 3, 4.0))
    add("Saat yönündeki yayda iki kanat, eşit değil",
        {"host": H(3), "at": 4.5, "width": 1.6, "operation": "pocket", "leaves": 2, "build": {"split": 0.4}},
        sd.from_kapi(3, 4.5, width=1.6, operation="pocket", leaves=2, build={"split": 0.4}))
    add("Cebi yayın başından taşar (yay boyu kuralı)", {"host": H(1), "at": 0.9, "type": "CG1"},
        sd.from_type(CG1, 1, 0.9))
    add("Menteşe sonda: cebi yayın sonundan taşar", {"host": H(1), "at": 7.2, "type": "CG1", "hand": "end"},
        sd.from_type(CG1, 1, 7.2, hand="end"))
    add("Cebi yayın sonuna tam değer", {"host": H(1), "at": 2.5 * math.pi - 0.45 - 0.76, "type": "CG1",
                                         "hand": "end"},
        sd.from_type(CG1, 1, 2.5 * math.pi - 0.45 - 0.76, hand="end"))
    add("İnce yaylı duvar cebi almaz", {"host": H(2), "at": 3.0, "type": "CG1"}, sd.from_type(CG1, 2, 3.0))
    add("İnce yaylı duvarda kanatlı kapı olur", {"host": H(2), "at": 3.0}, sd.from_kapi(2, 3.0))
    add("Cebi Kapı 90'a giriyor", {"host": H(1), "at": 2.6, "type": "CG1"}, sd.from_type(CG1, 1, 2.6))
    add("Cebi öbür cebe giriyor", {"host": H(1), "at": 7.0, "type": "CG1", "hand": "end"},
        sd.from_type(CG1, 1, 7.0, hand="end"))
    add("Kalın kanat, büyük pay yaylı duvarda",
        {"host": H(1), "at": 3.4, "operation": "pocket", "build": {"leafThickness": 0.06, "lip": 0.1}},
        sd.from_kapi(1, 3.4, operation="pocket", build={"leafThickness": 0.06, "lip": 0.1}))
    add("Kanadı duvara sığmayacak kadar kalın", {"host": H(1), "at": 3.4, "operation": "pocket",
                                                  "build": {"leafThickness": 0.24}},
        sd.from_kapi(1, 3.4, operation="pocket", build={"leafThickness": 0.24}))
    add("Yaylı duvarda öbür yüze açılan cep", {"host": H(1), "at": 3.4, "type": "CG1", "side": "right"},
        sd.from_type(CG1, 1, 3.4, side="right"))
    add("Saat yönündeki yayda cebi başından taşar", {"host": H(3), "at": 1.0, "type": "CG1"}, sd.from_type(CG1, 3, 1.0))
    add("Saat yönündeki yayda menteşe sonda, sona yakın", {"host": H(3), "at": 7.0, "type": "CG1", "hand": "end"},
        sd.from_type(CG1, 3, 7.0, hand="end"))
    add("Düz duvardaki eski kural aynen", {"host": H(STRAIGHT), "at": 2.0, "type": "CG1"},
        sd.from_type(CG1, STRAIGHT, 2.0))
    add("Düz duvarda cebi CG2'nin cebine giriyor", {"host": H(STRAIGHT), "at": 4.0, "type": "CG1", "hand": "end"},
        sd.from_type(CG1, STRAIGHT, 4.0, hand="end"))
    add("Geniş tek kanat yaylı duvarda", {"host": H(1), "at": 3.6, "width": 1.2, "operation": "pocket"},
        sd.from_kapi(1, 3.6, width=1.2, operation="pocket"))
    return out


# ── cad.spaces.quantities ────────────────────────────────────────────────────────────────────────────────────────


def wall_row(i, doors):
    """Its row: its openings (`doors` the drawing's openings by id), their area off its faces, its pockets off its
    volume."""
    length = wall_length(i)
    h = HEIGHTS[i]
    gross = length * h
    mine = [(id, o) for id, o in sorted(doors.items()) if o["host"] == i]
    areas = [(id, o["width"] * (o["head"] - (o.get("sill") or 0.0))) for id, o in mine if o.get("head") is not None]
    openings_area = sum(a for _, a in areas) + 0.0
    net = gross - openings_area
    pockets = 0.0
    any_pocket = False
    for _, o in mine:
        if o.get("operation") != "pocket" or o.get("head") is None:
            continue
        height = o["head"] - (o.get("sill") or 0.0)
        area = sum(p["length"] * p["width"] for p in sd.sizes(o)["pockets"])
        if i == STRAIGHT:
            scale = 1.0
        else:
            w = ARCS[i]
            m = w.lf - w.t / 2.0
            scale = (w.r - w.sg * m) / w.r
        if area > 0 and height > 0:
            pockets += area * scale * height
            any_pocket = True
    rule = "yüzey: taban çizgisinin uzunluğu × yükseklik, açıklıkların alanı düşülür; hacim: net yüzey × kalınlık"
    if any_pocket:
        rule += ", cebe giren kapıların cebi düşülür (cebin boyu × eni × kapının yüksekliği; yüzeyler cebi düşmez)"
    row = {"uid": f"$uidOf:{i}", "length": length, "width": thickness(i), "height": h, "grossArea": gross,
           "openingsArea": openings_area, "netArea": net,
           "volume": net * thickness(i) - (pockets if any_pocket else 0.0)}
    if any_pocket:
        row["pocketsVolume"] = pockets
    if areas:
        row["openings"] = [f"$uidOf:{id}" for id, _ in areas]
    row["rule"] = rule
    return row


def opening_row(id, o):
    row = {"uid": f"$uidOf:{id}", "form": o["form"]}
    if o.get("style"):
        row["style"] = o["style"]
    if o.get("number"):
        row["number"] = o["number"]
    row["wall"] = f"$uidOf:{o['host']}"
    row["width"] = o["width"]
    if o.get("head") is not None:
        h = o["head"] - (o.get("sill") or 0.0)
        row["height"] = h
        row["area"] = o["width"] * h
    return row


def quantities(doors, walls=(1, 2, 3, 4), openings_shown=True):
    """The report: the walls given, the openings when their layer is asked for (each wall's openings and pockets
    whatever its rows show)."""
    rows = [wall_row(i, doors) for i in walls]
    openings = [opening_row(id, o) for id, o in sorted(doors.items())] if openings_shown else []
    totals = {"spaceArea": 0.0, "wallLength": sum(r["length"] for r in rows) + 0.0,
              "wallNetArea": sum(r["netArea"] for r in rows) + 0.0, "openings": len(openings),
              "openingArea": sum(r.get("area", 0.0) for r in openings) + 0.0}
    return {"spaces": [], "walls": rows, "openings": openings, "totals": totals}


def q_step(doors, inp=None, walls=(1, 2, 3, 4), openings_shown=True):
    return {"op": "execute", "input": inp or {}, "within": 1e-9,
            "result": {"status": "completed", "output": quantities(doors, walls, openings_shown), "warnings": []},
            "expect": {"revision": "same"}}


def create_step(given, made, id):
    return {"op": "execute", "command": "cad.openings.create", "input": {"layerId": "kapi", "openings": [given]},
            "within": 1e-9, "result": {"status": "completed", "output": {"uids": ["$any"], "ids": [id],
                                                                           "revision": "$current"}, "warnings": []},
            "expect": {"ids": "$any", "entities": {str(id): stored(made, id)}, "revision": "changed"}}


def quantity_cases():
    out = []
    H = lambda id: f"$uidOf:{id}"  # noqa: E731
    base = dict(BASE)

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    case("Cepler duvarların hacminden düşülür, yüzeyler aynı", [q_step(base)])
    case("Yalnız duvar katmanı: duvarlar cepleriyle, kapı satırı yok",
         [q_step(base, {"layers": ["duvar"]}, openings_shown=False)])
    more = [
        ("Yaylı duvara ikinci cep (menteşe sonda)", {"host": H(1), "at": 3.0, "type": "CG1", "hand": "end"},
         sd.from_type(CG1, 1, 3.0, hand="end")),
        ("Saat yönündeki sola yaslı yaya cep: ölçek duvarın ortasına", {"host": H(3), "at": 4.0, "type": "CG1"},
         sd.from_type(CG1, 3, 4.0)),
        ("İki kanatlı cep iki cebiyle düşülür", {"host": H(1), "at": 2.95, "width": 1.4, "operation": "pocket",
                                                 "leaves": 2}, sd.from_kapi(1, 2.95, width=1.4, operation="pocket",
                                                                            leaves=2)),
        ("Eşiği yüksek kapı: yükseklik üst − alt kot", {"host": H(3), "at": 4.0, "operation": "pocket", "sill": 0.3,
                                                         "head": 2.4},
         sd.from_kapi(3, 4.0, operation="pocket", sill=0.3, head=2.4)),
        ("Kalın kanatlı cep daha çok düşer", {"host": H(STRAIGHT), "at": 2.0, "operation": "pocket",
                                              "build": {"leafThickness": 0.06}},
         sd.from_kapi(STRAIGHT, 2.0, operation="pocket", build={"leafThickness": 0.06})),
        ("Büyük pay cebi kısaltır", {"host": H(STRAIGHT), "at": 2.0, "operation": "pocket", "build": {"lip": 0.2}},
         sd.from_kapi(STRAIGHT, 2.0, operation="pocket", build={"lip": 0.2})),
        ("Kanatlı kapı cep değildir", {"host": H(3), "at": 4.0}, sd.from_kapi(3, 4.0)),
        ("Sürme kapı cep değildir", {"host": H(3), "at": 4.0, "operation": "slide"},
         sd.from_kapi(3, 4.0, operation="slide")),
        ("Geniş kanatlı cep", {"host": H(1), "at": 3.6, "width": 1.2, "operation": "pocket"},
         sd.from_kapi(1, 3.6, width=1.2, operation="pocket")),
    ]
    for name, given, made in more:
        doors = dict(base)
        doors[NEW] = made
        case(name, [create_step(given, made, NEW), q_step(doors)])
    # Its own heights.
    doors = dict(base)
    set_head = {"op": "execute", "command": "cad.openings.edit",
                "input": {"uids": ["$uidOf:5"], "operation": {"kind": "set", "head": 2.4}}, "result": "$any"}
    d5 = copy.deepcopy(base[5])
    d5["head"] = 2.4
    doors[5] = d5
    case("Kapının yüksekliği değişince cebin hacmi de", [set_head, q_step(doors)])
    gone = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:5"]}, "result": "$any"}
    doors = {k: v for k, v in base.items() if k != 5}
    case("Cebe giren kapı silinince yaylı duvarın hacmi tam", [gone, q_step(doors)])
    to_swing = {"op": "execute", "command": "cad.openings.edit",
                "input": {"uids": ["$uidOf:7"], "operation": {"kind": "set", "operation": "swing", "unset": ["lip"]}},
                "result": "$any", "expect": {"revision": "changed"}}
    d7 = copy.deepcopy(base[7])
    d7["operation"] = "swing"
    d7["overrides"] = ["operation"]
    doors = dict(base)
    doors[7] = d7
    case("Kanatlıya çevrilen kapının cebi kalmaz", [to_swing, q_step(doors)])
    moved = {"op": "execute", "command": "cad.openings.edit",
             "input": {"uids": ["$uidOf:5"], "operation": {"kind": "move", "by": -1.0}}, "result": "$any"}
    d5 = copy.deepcopy(base[5])
    d5["at"] = 4.5
    doors = dict(base)
    doors[5] = d5
    case("Taşınan cep aynı hacmi düşer", [moved, q_step(doors)])
    case("Geri alınan kapı: cep yeniden düşülür", [gone, {"op": "undo", "expect": {"revision": "changed"}}, q_step(base)])
    # A refused pocket on the thin arc wall: nothing written, nothing taken off.
    thin = {"op": "execute", "command": "cad.openings.create",
            "input": {"layerId": "kapi", "openings": [{"host": H(2), "at": 3.0, "type": "CG1"}]}, "result": "$any",
            "expect": {"ids": IDS, "revision": "same"}}
    case("İnce yaylı duvara cep reddedilir: metraj aynı", [thin, q_step(base)])
    # Only the doors' layer: no wall rows, the openings still.
    case("Yalnız kapı katmanı: duvar satırı yok", [q_step(base, {"layers": ["kapi"]}, walls=())])
    case("Kullanım süzgeci: yalnız mahaller", [{"op": "execute", "input": {"usages": ["Oturma"]}, "within": 1e-9,
                                               "result": {"status": "completed", "output": {
                                                   "spaces": [], "walls": [], "openings": [],
                                                   "totals": {"spaceArea": 0.0, "wallLength": 0.0, "wallNetArea": 0.0,
                                                              "openings": 0, "openingArea": 0.0}}, "warnings": []},
                                               "expect": {"revision": "same"}}])
    # Two pockets added on two walls.
    two = dict(base)
    a = sd.from_type(CG1, 1, 3.0, hand="end")
    b = sd.from_type(CG1, 3, 4.0)
    two[NEW], two[NEW + 1] = a, b
    case("İki cep iki duvara: her duvar kendi cebini düşer",
         [create_step({"host": H(1), "at": 3.0, "type": "CG1", "hand": "end"}, a, NEW),
          create_step({"host": H(3), "at": 4.0, "type": "CG1"}, b, NEW + 1), q_step(two)])
    return out


def build():
    return {
        "geometry": geometry(),
        "create": doc("cad.openings.create", "Yaylı duvarda cebe giren kapı", NOTE, create_cases()),
        "quantities": doc("cad.spaces.quantities", "Cebin metrajdan düşülmesi", NOTE, quantity_cases()),
    }


def files(out):
    return {
        GEOMETRY: out["geometry"],
        COMMANDS / "cad.openings.create.arcpocket.json": out["create"],
        COMMANDS / "cad.spaces.quantities.pockets.json": out["quantities"],
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for path, value in files(build()).items():
        text = json.dumps(value, ensure_ascii=False, indent=1) + "\n"
        if args.check:
            if not path.exists() or path.read_text("utf-8") != text:
                print(f"farklı: {path.relative_to(ROOT)}")
                bad += 1
        else:
            path.write_text(text, "utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(value['cases'])} durum)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
