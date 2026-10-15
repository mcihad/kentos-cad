"""Patlat on curves: shared cases (docs/adr/0308).

    python3 scripts/fixtures/explode_curve_cases.py           # writes the file
    python3 scripts/fixtures/explode_curve_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.explode.curves.json. The ADR's rules, computed here without KentOS code
(mpmath, 40 digits; the ellipse's arc lengths by quadrature, its places by root finding):

- a circle, an arc, an ellipse and an elliptical arc explode into one polyline whose vertices lie on the curve, its
  pieces equal in length along it: a closed curve (a circle, a full ellipse) a closed polyline of n vertices starting
  where the curve starts, an open one n + 1 vertices from its start to its end;
- n: `count` as given (a closed curve at least 3); `step` the least whole number of pieces none longer than the step;
  `deviation` (absent: 0.01 m) the least whole number of pieces of an arc of the curve's least radius of curvature
  (the circle's radius, an ellipse's b²/a) whose chord lies within the tolerance: θ = 2·acos(1 − t/ρ), n = ⌈L / (ρθ)⌉;
- more than 100 000 vertices: not exploded, said; a count out of 1…100 000, a step or a tolerance not above zero:
  invalid_density;
- the polyline keeps the curve's layer, look and data; a spline becomes a polyline through its curve (its vertices
  the core's fine chords', not checked here vertex by vertex).
"""
import json
import sys
from pathlib import Path

import mpmath as mp

sys.dont_write_bytecode = True
mp.mp.dps = 40

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.explode.curves.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
LABEL = "Patlat"
PI = mp.pi
NEXT = 9  # the first new object's slot: the setup has 8.


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


DATA = {"attrs": {"Ad": "Ağaç 4"}, "color": "#30A46C"}
CIRCLE = {"kind": "circle", "id": 1, "layerId": "cizim", **DATA, "c": P(0, 0), "r": 10}
ARC = {"kind": "arc", "id": 2, "layerId": "cizim", "attrs": {}, "c": P(30, 0), "r": 5, "a0": 0, "a1": float(PI / 2)}
ELLIPSE = {"kind": "ellipse", "id": 3, "layerId": "cizim", "attrs": {}, "c": P(60, 0), "major": P(8, 0), "ratio": 0.5,
           "t0": 0, "t1": float(2 * PI)}
HALF = {"kind": "ellipse", "id": 4, "layerId": "cizim", "attrs": {}, "c": P(90, 0), "major": P(6, 0), "ratio": 0.5,
        "t0": 0, "t1": float(PI)}
SPLINE = {"kind": "spline", "id": 5, "layerId": "cizim", "attrs": {}, "pts": [P(0, 60), P(5, 64), P(10, 60), P(15, 64)],
          "closed": False}
LOCKED = {"kind": "circle", "id": 6, "layerId": "kilitli", "attrs": {}, "c": P(0, 30), "r": 2}
LINE = {"kind": "line", "id": 7, "layerId": "cizim", "attrs": {}, "a": P(0, -20), "b": P(10, -20)}
SMALL = {"kind": "circle", "id": 8, "layerId": "cizim", "attrs": {}, "c": P(30, 30), "r": 1}
ENTITIES = [CIRCLE, ARC, ELLIPSE, HALF, SPLINE, LOCKED, LINE, SMALL]


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Patlat: eğriler",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100, "workspace": "cad"},
        "origin": P(0, 0),
        "layers": [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True)],
        "activeLayer": "cizim",
        "entities": ENTITIES,
        "styles": {"items": [], "categories": []},
    }


# ── The curves, exactly ────────────────────────────────────────────────────────────────────────────────────────────

class Circular:
    """A circle or an arc: its points at arc length s from its start."""

    def __init__(self, e):
        self.c = (mp.mpf(e["c"]["x"]), mp.mpf(e["c"]["y"]))
        self.r = mp.mpf(e["r"])
        self.a0 = mp.mpf(e.get("a0", 0))
        sweep = 2 * PI if e["kind"] == "circle" else (mp.mpf(e["a1"]) - self.a0) % (2 * PI)
        self.sweep = sweep
        self.closed = e["kind"] == "circle"
        self.length = self.r * sweep
        self.rho = self.r

    def at(self, s):
        a = self.a0 + s / self.r
        return (self.c[0] + self.r * mp.cos(a), self.c[1] + self.r * mp.sin(a))


class Elliptic:
    """An ellipse or an elliptical arc: its points at arc length s from its start, along the true curve."""

    def __init__(self, e):
        self.c = (mp.mpf(e["c"]["x"]), mp.mpf(e["c"]["y"]))
        mx, my = mp.mpf(e["major"]["x"]), mp.mpf(e["major"]["y"])
        self.a = mp.sqrt(mx * mx + my * my)
        self.b = self.a * mp.mpf(e["ratio"])
        self.turn = mp.atan2(my, mx)
        self.t0 = mp.mpf(e["t0"])
        t1 = mp.mpf(e["t1"])
        self.closed = abs((t1 - self.t0) - 2 * PI) < mp.mpf("1e-12")
        self.t1 = self.t0 + 2 * PI if self.closed else t1
        self.length = self.run(self.t1)
        self.rho = self.b * self.b / self.a

    def speed(self, t):
        return mp.sqrt((self.a * mp.sin(t)) ** 2 + (self.b * mp.cos(t)) ** 2)

    def run(self, t):
        return mp.quad(self.speed, [self.t0, t])

    def at(self, s):
        if s <= 0:
            t = self.t0
        elif s >= self.length:
            t = self.t1
        else:
            guess = self.t0 + (self.t1 - self.t0) * s / self.length
            t = mp.findroot(lambda u: self.run(u) - s, guess)
        x, y = self.a * mp.cos(t), self.b * mp.sin(t)
        ct, st = mp.cos(self.turn), mp.sin(self.turn)
        return (self.c[0] + x * ct - y * st, self.c[1] + x * st + y * ct)


def curve(e):
    return Circular(e) if e["kind"] in ("circle", "arc") else Elliptic(e)


MAX = 100_000
DEVIATION = mp.mpf("0.01")


def pieces(k, density):
    """How many equal pieces; or the core's words when too many."""
    least = 3 if k.closed else 1
    if density is None:
        density = {"kind": "deviation", "tolerance": DEVIATION}
    if density["kind"] == "count":
        n = density["segments"]
    elif density["kind"] == "step":
        n = int(mp.ceil(k.length / mp.mpf(density["length"])))
    else:
        theta = 2 * mp.acos(1 - mp.mpf(density["tolerance"]) / k.rho)
        x = k.length / (k.rho * theta)
        assert abs(x - mp.nint(x)) > mp.mpf("1e-6"), "a count too near a whole number"
        n = int(mp.ceil(x))
    n = max(n, least)
    if n > MAX:
        return None
    return n


def polyline(e, density, slot):
    k = curve(e)
    n = pieces(k, density)
    count = n if k.closed else n + 1
    pts = [k.at(k.length * i / n) for i in range(count)]
    out = {"kind": "polyline", "id": slot, "layerId": e["layerId"], "attrs": e["attrs"]}
    if "color" in e:
        out["color"] = e["color"]
    out["pts"] = [P(float(x), float(y)) for x, y in pts]
    if k.closed:
        out["closed"] = True
    return out


TOO_DENSE = ("Bu sıklıkta eğri 100000 köşeden çok olur; daha az bölüm, daha uzun aralık ya da daha büyük sapma "
             "verin.")
NOT_BASIC = "Bu nesne zaten temel bir nesne; patlatılacak bir şey yok."
LOCKED_MSG = "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."


# ── Steps ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def ids_after(removed, made):
    return sorted([e["id"] for e in ENTITIES if e["id"] not in removed] + made)


def exploded(name, targets, density=None, undo=False, warnings=(), plan=False):
    """The curves `targets` (setup entities) exploded, in order; each one new polyline."""
    inp = {"uids": [U(e["id"]) for e in targets]}
    if density is not None:
        inp["density"] = density
    slots = list(range(NEXT, NEXT + len(targets)))
    made = {str(s): polyline(e, density, s) for e, s in zip(targets, slots)}
    captured = [{"op": "captureUid", "id": e["id"], "as": f"o{e['id']}"} for e in targets]
    if plan:
        return {"name": name, "steps": captured + [{
            "op": "plan", "input": inp, "within": 1e-9,
            "result": {"status": "completed", "warnings": list(warnings), "output": {
                "changed": [], "created": [{**made[str(s)], "id": 0} for s in slots],
                "removed": [f"$uid:o{e['id']}" for e in targets], "revision": "$current"}},
            "expect": {"revision": "same"}}]}
    # The vertices are checked on the plan's objects (±1e-9 m: sines and cosines); the written ones by the rest.
    shown = {"op": "plan", "input": inp, "within": 1e-9,
             "result": {"status": "completed", "warnings": list(warnings), "output": {
                 "changed": [], "created": [{**made[str(s)], "id": 0} for s in slots],
                 "removed": [f"$uid:o{e['id']}" for e in targets], "revision": "$current"}},
             "expect": {"revision": "same"}}
    written = {str(s): {**m, "pts": "$any"} for s, m in made.items()}
    step = {"op": "execute", "input": inp,
            "result": {"status": "completed", "warnings": list(warnings), "output": {
                "changed": [], "created": [U(s) for s in slots],
                "removed": [f"$uid:o{e['id']}" for e in targets], "revision": "$current"}},
            "expect": {"ids": ids_after([e["id"] for e in targets], slots), "entities": written, "revision": "changed",
                       "canUndo": True}}
    steps = captured + [shown, step]
    if undo:
        steps.append({"op": "undo", "returns": LABEL,
                      "expect": {"ids": [e["id"] for e in ENTITIES],
                                 "entities": {str(e["id"]): e for e in targets}}})
    return {"name": name, "steps": steps}


def refused(name, inp, code, message, path):
    return {"name": name, "steps": [{"op": "execute", "input": inp,
                                     "result": {"status": "failed",
                                                "error": {"code": code, "message": message, "path": path}},
                                     "expect": {"revision": "same"}}]}


def cases():
    count = lambda n: {"kind": "count", "segments": n}  # noqa: E731
    step = lambda l: {"kind": "step", "length": l}  # noqa: E731
    dev = lambda t: {"kind": "deviation", "tolerance": t}  # noqa: E731
    out = [
        exploded("Daire, sıklık verilmeden: kirişleri daireye 0,01 m'den yakın kapalı çoklu çizgi; verisi ve rengi kalır; "
                 "adım “Patlat”, geri alınır", [CIRCLE], undo=True),
        exploded("Daire 24 eşit bölüme: 24 köşe, ilki dairenin başında", [CIRCLE], count(24)),
        exploded("Daire 1 bölüm istense de kapalı çizgi en az 3 köşe", [CIRCLE], count(1)),
        exploded("Daire 5 m aralıkla: hiçbir bölüm 5 m'den uzun değil", [CIRCLE], step(5)),
        exploded("Daire 0,5 m sapmayla: az köşe", [CIRCLE], dev(0.5)),
        exploded("Daire 1 mm sapmayla: çok köşe", [SMALL], dev(0.001)),
        exploded("Yay, sıklık verilmeden: açık çoklu çizgi, uçları yayın uçları", [ARC]),
        exploded("Yay 4 bölüme: 5 köşe", [ARC], count(4)),
        exploded("Yay 1 m aralıkla", [ARC], step(1)),
        exploded("Yay tek bölüm: kirişi", [ARC], count(1)),
        exploded("Elips 8 eşit uzunlukta bölüme: köşeler gerçek elipsin üstünde", [ELLIPSE], count(8)),
        exploded("Elips 0,1 m sapmayla: en küçük eğrilik yarıçapı b²/a", [ELLIPSE], dev(0.1)),
        exploded("Eliptik yay 4 bölüme: açık çoklu çizgi", [HALF], count(4)),
        exploded("Eliptik yay 2 m aralıkla", [HALF], step(2)),
        exploded("Daire ve yay birlikte, verildikleri sırayla, aynı sıklıkla", [CIRCLE, ARC], count(6)),
        exploded("Çizgi ile daire: çizgi uyarıyla atlanır, daire patlar", [CIRCLE], count(6),
                 warnings=[{"code": "not_explodable", "path": "uids",
                            "message": f"1 nesne patlatılamadığı için atlandı: {NOT_BASIC}"}]),
        exploded("Plan yazmadan gösterir", [CIRCLE], count(6), plan=True),
        refused("Bölüm sayısı sıfır: invalid_density", {"uids": [U(1)], "density": count(0)}, "invalid_density",
                "Bölüm sayısı 1 ile 100 000 arasında olmalı.", "density.segments"),
        refused("Bölüm sayısı 100 000'den çok: invalid_density", {"uids": [U(1)], "density": count(100_001)},
                "invalid_density", "Bölüm sayısı 1 ile 100 000 arasında olmalı.", "density.segments"),
        refused("Aralık sıfır: invalid_density", {"uids": [U(1)], "density": step(0)}, "invalid_density",
                "Aralık sıfırdan büyük bir sayı olmalı.", "density.length"),
        refused("Sapma eksi: invalid_density", {"uids": [U(1)], "density": dev(-1)}, "invalid_density",
                "Sapma sıfırdan büyük bir sayı olmalı.", "density.tolerance"),
        refused("Çok sık: 100 000 köşeden çok, not_explodable", {"uids": [U(1)], "density": step(0.0001)},
                "not_explodable", TOO_DENSE, "uids[0]"),
        refused("Kilitli katmandaki daire: layer_locked", {"uids": [U(6)]}, "layer_locked", LOCKED_MSG, "uids[0]"),
    ]
    # The line before the circle: the warning's case lists both.
    for st in out[15]["steps"][-2:]:
        st["input"]["uids"] = [U(7), U(1)]
    out[15]["steps"][-1]["expect"]["ids"] = ids_after([1], [NEXT])
    # A spline with a count: one open polyline through its curve, its data kept (vertices the core's).
    out.append({"name": "Eğri 10 bölüme: köşeleri eğrinin üstünde açık çoklu çizgi",
                "steps": [{"op": "captureUid", "id": 5, "as": "o5"},
                          {"op": "execute", "input": {"uids": [U(5)], "density": count(10)},
                           "result": {"status": "completed", "warnings": [], "output": {
                               "changed": [], "created": [U(NEXT)], "removed": ["$uid:o5"], "revision": "$current"}},
                           "expect": {"ids": ids_after([5], [NEXT]), "revision": "changed",
                                      "entities": {str(NEXT): {"kind": "polyline", "id": NEXT, "layerId": "cizim",
                                                               "attrs": {}, "pts": "$any"}}}}]})
    # The non-finite step a hand-made input may carry.
    out.append({"name": "Aralık sayı değil: invalid_density",
                "steps": [{"op": "execute", "input": {"uids": [U(1)], "density": step(1)},
                           "nonFinite": {"density.length": "NaN"},
                           "result": {"status": "failed", "error": {
                               "code": "invalid_density", "message": "Aralık sıfırdan büyük bir sayı olmalı.",
                               "path": "density.length"}},
                           "expect": {"revision": "same"}}]})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.explode", "commandVersion": 1,
            "title": "Patlat: eğriler ve nokta sıklığı",
            "note": "ADR 0308. Bağımsız başvuru: scripts/fixtures/explode_curve_cases.py (mpmath, 40 basamak).",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
