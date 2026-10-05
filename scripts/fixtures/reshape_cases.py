#!/usr/bin/env python3
"""Independent reference of Biçim değiştir and the hole operations (docs/adr/0173).

Writes fixtures/reshape/v1/cases.json from the ADR's rules, no KentOS code. The core works on the overlay engine (an area
split by the sketch, the pockets the sketch closes, unions and differences); this reference splices rings and paths at
their meetings with exact fractions instead:

- an area reshaped by a sketch meeting its outer ring twice: the ring cut at the sketch's first and last meeting, the
  sketch's stretch between them closing each side into a candidate ring; the larger candidate is the result (the larger
  piece of a cut, the area with its pocket when the sketch runs outside);
- a path reshaped: its stretch between the two meetings replaced by the sketch's (turned to the path's direction), or,
  with one meeting, its shorter side replaced by the sketch's longer side;
- a hole added as drawn (its ring inside the outer ring, meeting no hole), removed, or its ring given back; a ring
  meeting a hole merges with it, the merged hole worked out by hand for the case.

The arc case (a path whose arc the sketch cuts) is worked out with mpmath at 50 digits: the cut point on the circle and
the two arcs' bulges tan(θ·t/4). Refusals are the ADR's, stated per case.

Sürdür (§4): a path continued from its last end takes the drawn points after its own; from its first end the drawn
points go before them in turned order, each drawn arc on its other side (a bulge's sign is its arc's side seen along
the travel). The directions out of a path's ends: straight edges along their chords, an arc's end along the chord
turned by half the arc's sweep (2·atan b, counter-clockwise for b > 0), in mpmath.

The cases compare rings as shapes: the start vertex, the direction and vertices on straight lines between straight
edges do not matter; coordinates within 1e-9 m, bulges within 1e-12.
"""

import argparse
import json
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "reshape" / "v1" / "cases.json"
mp.mp.dps = 50


def P(x, y):
    return {"x": float(x), "y": float(y)}


def pts(*xy):
    return [P(x, y) for x, y in xy]


def rect(x0, y0, x1, y1):
    """A counter-clockwise rectangle."""
    return pts((x0, y0), (x1, y0), (x1, y1), (x0, y1))


def polygon(outer, holes=None, parts=None, bulges=None):
    e = {"kind": "polygon", "pts": outer}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": h} for h in holes]
    if parts:
        e["parts"] = [{"pts": q} if not isinstance(q, dict) else q for q in parts]
    return e


def polyline(p, bulges=None):
    e = {"kind": "polyline", "pts": p}
    if bulges:
        e["bulges"] = bulges
    return e


# ── Exact geometry ─────────────────────────────────────────────────────────

def fr(p):
    return (F(p["x"]), F(p["y"]))


def cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def seg_hit(a, b, c, d):
    """The meeting of segments ab and cd: (t along ab, u along cd, point), or None; collinear overlaps are not cases here."""
    r = (b[0] - a[0], b[1] - a[1])
    s = (d[0] - c[0], d[1] - c[1])
    den = r[0] * s[1] - r[1] * s[0]
    if den == 0:
        return None
    q = (c[0] - a[0], c[1] - a[1])
    t = (q[0] * s[1] - q[1] * s[0]) / den
    u = (q[0] * r[1] - q[1] * r[0]) / den
    if 0 <= t <= 1 and 0 <= u <= 1:
        return t, u, (a[0] + r[0] * t, a[1] + r[1] * t)
    return None


def area2(ring):
    n = len(ring)
    return sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))


def splice_ring(ring, sketch):
    """The outer ring (exact points, straight) reshaped by the sketch: the larger candidate, or None."""
    n = len(ring)
    hits = []
    for j in range(len(sketch) - 1):
        for i in range(n):
            h = seg_hit(ring[i], ring[(i + 1) % n], sketch[j], sketch[j + 1])
            if h:
                t, u, p = h
                hits.append((j + u, i + t, p))
    hits.sort()
    uniq = []
    for h in hits:
        if not uniq or uniq[-1][2] != h[2]:
            uniq.append(h)
    if len(uniq) != 2:
        return None
    (s1, l1, p1), (s2, l2, p2) = uniq
    mid = [sketch[k] for k in range(len(sketch)) if s1 < k < s2]

    def walk(from_l, to_l):
        """The ring's vertices strictly between positions from_l and to_l, going forward (round past the last)."""
        span = (to_l - from_l) % n
        ahead = [((k - from_l) % n, k) for k in range(n)]
        return [ring[k] for d, k in sorted(ahead) if 0 < d < span]

    a = [p1] + mid + [p2] + walk(l2, l1)
    b = [p2] + mid[::-1] + [p1] + walk(l1, l2)
    return max([a, b], key=lambda r: abs(area2(r)))


def splice_path(path, sketch):
    """The path (exact points, straight) reshaped by the sketch, or None when it meets nowhere."""
    hits = []
    for j in range(len(sketch) - 1):
        for i in range(len(path) - 1):
            h = seg_hit(path[i], path[i + 1], sketch[j], sketch[j + 1])
            if h:
                t, u, p = h
                hits.append((j + u, i + t, p))
    hits.sort()
    uniq = []
    for h in hits:
        if not uniq or uniq[-1][2] != h[2]:
            uniq.append(h)
    if not uniq:
        return None

    def head(l, p):
        return [path[k] for k in range(len(path)) if k < l] + [p]

    def tail(l, p):
        return [p] + [path[k] for k in range(len(path)) if k > l]

    def length(ps):
        def m(v):
            return mp.mpf(v.numerator) / v.denominator
        return sum(mp.sqrt((m(b[0]) - m(a[0])) ** 2 + (m(b[1]) - m(a[1])) ** 2) for a, b in zip(ps, ps[1:]))

    if len(uniq) >= 2:
        (s1, l1, p1), (s2, l2, p2) = uniq[0], uniq[-1]
        mid = [sketch[k] for k in range(len(sketch)) if s1 < k < s2]
        if l1 <= l2:
            return head(l1, p1) + mid + tail(l2, p2)
        return head(l2, p2) + mid[::-1] + tail(l1, p1)
    s, l, p = uniq[0]
    hd, tl = head(l, p), tail(l, p)
    before = [p] + [sketch[k] for k in range(len(sketch) - 1, -1, -1) if k < s]
    after = [p] + [sketch[k] for k in range(len(sketch)) if k > s]
    outward = before if length(before) > length(after) else after
    if length(tl) > length(hd):
        return outward[::-1] + tl[1:]
    return hd + outward[1:]


def out_pts(ps):
    return [P(x, y) for x, y in ps]


# ── The cases ──────────────────────────────────────────────────────────────

SQUARE = rect(0, 0, 100, 100)
HOLED = polygon(SQUARE, holes=[rect(40, 60, 60, 80)])


def area_case(name, shape, sketch, expect_holes=None, refusal=None, part=None):
    """`expect_holes`: the kept area's holes, by hand; `part`: which part changes (others as they are)."""
    case = {"name": name, "op": "reshape", "shape": shape, "sketch": sketch}
    if refusal:
        case["refusal"] = refusal
        return case
    k = part or 0
    outer = shape["pts"] if k == 0 else shape["parts"][k - 1]["pts"]
    ring = splice_ring([fr(p) for p in outer], [fr(p) for p in sketch])
    assert ring is not None, name
    parts = [{"pts": out_pts(ring), "holes": [{"pts": h} for h in expect_holes or []]}]
    if shape.get("parts"):
        others = [{"pts": shape["pts"], "holes": shape.get("holes", [])}] + [
            {"pts": q["pts"], "holes": q.get("holes", [])} for q in shape["parts"]]
        others[k] = parts[0]
        parts = others
    case["expect"] = {"kind": "polygon", "parts": parts}
    return case


def path_case(name, shape, sketch, refusal=None):
    case = {"name": name, "op": "reshape", "shape": shape, "sketch": sketch}
    if refusal:
        case["refusal"] = refusal
        return case
    src = [fr(p) for p in (shape["pts"] if shape["kind"] == "polyline" else [shape["a"], shape["b"]])]
    out = splice_path(src, [fr(p) for p in sketch])
    assert out is not None, name
    case["expect"] = {"kind": "polyline", "pts": out_pts(out)}
    return case


def arc_path_case():
    """A path (0,0)→(100,0) on an arc of bulge 0.5, then (100,0)→(100,50); a vertical sketch at x = 50 from below:
    the arc cut at its crossing, the head kept (longer), the sketch's longer side going on (one meeting)."""
    b = mp.mpf("0.5")
    a, z = (mp.mpf(0), mp.mpf(0)), (mp.mpf(100), mp.mpf(0))
    chord = mp.mpf(100)
    sweep = 4 * mp.atan(b)
    r = chord * (1 + b * b) / (4 * b)
    k = (1 - b * b) / (4 * b)
    c = ((a[0] + z[0]) / 2 - (z[1] - a[1]) * k, (a[1] + z[1]) / 2 + (z[0] - a[0]) * k)
    # The arc runs counter-clockwise from a (bulge > 0): below the chord. x = 50 meets it at its lowest point.
    y = c[1] - mp.sqrt(r * r - (50 - c[0]) ** 2)
    a0 = mp.atan2(a[1] - c[1], a[0] - c[0])
    ang = mp.atan2(y - c[1], 50 - c[0])
    t = ((ang - a0) % (2 * mp.pi)) / sweep
    shape = polyline(pts((0, 0), (100, 0), (100, 50)), [0.5, 0, 0])
    sketch = pts((50, -40), (50, 30))
    # The sketch meets the arc once (the vertical line crosses the arc once, below the chord) and the chord's other
    # edge never: one meeting. The head (0,0)→meeting is the arc's first half; the tail is the second half and the
    # vertical edge: longer, so the tail is kept and the sketch's longer side (up, to (50, 30)) runs into it.
    tail_bulge = mp.tan(sweep * (1 - t) / 4)
    meet = P(float(mp.mpf(50)), float(y))
    return {"name": "yayı kesen tek buluşma: yay çemberinde kalır, uzun taraf tutulur", "op": "reshape", "shape": shape,
            "sketch": sketch,
            "expect": {"kind": "polyline", "pts": [P(50, 30), meet, P(100, 0), P(100, 50)],
                       "bulges": [0.0, float(tail_bulge), 0.0, 0.0]}}


def cases():
    out = []
    # Areas: cuts keep the larger piece.
    out.append(area_case("yatay kesim: büyük parça kalır", polygon(SQUARE), pts((-10, 30), (110, 30))))
    out.append(area_case("kırık çizgiyle kesim", polygon(SQUARE), pts((-10, 20), (50, 40), (110, 20))))
    out.append(area_case("dikey kesim: sağdaki büyük", polygon(SQUARE), pts((35, 120), (35, -20))))
    out.append(area_case("kesim, delik kalan parçada", HOLED, pts((-10, 30), (110, 30)), expect_holes=[rect(40, 60, 60, 80)]))
    # Areas: a sketch running outside adds its pocket.
    out.append(area_case("sınırdan sınıra cep", polygon(SQUARE), pts((100, 30), (130, 30), (130, 70), (100, 70))))
    out.append(area_case("içten içe dolanan cep", polygon(SQUARE), pts((90, 30), (130, 30), (130, 70), (90, 70))))
    out.append(area_case("üstte üçgen cep", HOLED, pts((20, 100), (50, 130), (80, 100)), expect_holes=[rect(40, 60, 60, 80)]))
    # Multi-part: the part the sketch meets changes, the other stays.
    two = polygon(rect(0, 0, 40, 40), parts=[rect(100, 0, 140, 40)])
    out.append(area_case("çok parçalı alanın ikinci parçası kesilir", two, pts((90, 10), (150, 10)), part=1))
    # Refusals.
    out.append(area_case("hem kesen hem büyüten", polygon(SQUARE), pts((-10, 50), (120, 50), (120, 120), (50, 120), (50, 100)),
                         refusal="bothWays"))
    out.append(area_case("dışarıda kalan çizgi", polygon(SQUARE), pts((120, 0), (130, 50)), refusal="noCrossing"))
    out.append(area_case("içeride kalan çizgi", polygon(SQUARE), pts((20, 20), (80, 30)), refusal="noCrossing"))
    out.append(area_case("deliğe değen çizgi", HOLED, pts((-10, 70), (110, 70)), refusal="touchesHole"))
    out.append(area_case("iki parçaya değen çizgi", two, pts((-10, 20), (150, 20)), refusal="manyParts"))
    out.append(area_case("tek noktalı çizgi", polygon(SQUARE), pts((10, 10)), refusal="tooShort"))
    # Paths.
    ell = polyline(pts((0, 0), (100, 0), (100, 100)))
    out.append(path_case("iki buluşma: aradaki bölüm değişir", ell, pts((30, -10), (30, 20), (120, 70))))
    out.append(path_case("iki buluşma, ters yönde çizilmiş", ell, pts((120, 70), (30, 20), (30, -10))))
    out.append(path_case("üç buluşma: ilk ve son arası", polyline(pts((0, 0), (100, 0))), pts((10, -10), (20, 10), (30, -10), (40, 10))))
    out.append(path_case("tek buluşma: kısa taraf yeniden çizilir", polyline(pts((0, 0), (100, 0))), pts((80, -10), (80, 10), (120, 40))))
    out.append(path_case("uçtan başlayan çizgi: sürdürür", polyline(pts((0, 0), (100, 0))), pts((100, 0), (120, 20))))
    out.append(path_case("çizgi (line) çoklu çizgi olur", {"kind": "line", "a": P(0, 0), "b": P(100, 0)}, pts((60, -10), (60, 10), (90, 10), (90, -10))))
    out.append(path_case("buluşmayan çizgi", polyline(pts((0, 0), (100, 0))), pts((0, 10), (100, 10)), refusal="noCrossing"))
    out.append(arc_path_case())
    # Holes.
    out.append({"name": "delik ekle: içte, deliğe değmeyen halka olduğu gibi", "op": "holeAdd", "shape": polygon(SQUARE),
                "ring": {"pts": rect(10, 10, 30, 30)},
                "expect": {"kind": "polygon", "parts": [{"pts": SQUARE, "holes": [{"pts": rect(10, 10, 30, 30)}]}]}})
    out.append({"name": "delik ekle: var olan deliğin yanına", "op": "holeAdd", "shape": HOLED,
                "ring": {"pts": rect(10, 10, 30, 30)},
                "expect": {"kind": "polygon", "parts": [{"pts": SQUARE, "holes": [{"pts": rect(40, 60, 60, 80)}, {"pts": rect(10, 10, 30, 30)}]}]}})
    out.append({"name": "delik ekle: var olan delikle birleşir", "op": "holeAdd", "shape": HOLED,
                "ring": {"pts": rect(50, 50, 70, 70)},
                "expect": {"kind": "polygon", "parts": [{"pts": SQUARE, "holes": [
                    {"pts": pts((40, 60), (50, 60), (50, 50), (70, 50), (70, 70), (60, 70), (60, 80), (40, 80))}]}]}})
    out.append({"name": "delik ekle: ikinci parçaya", "op": "holeAdd", "shape": two, "ring": {"pts": rect(110, 10, 120, 20)},
                "expect": {"kind": "polygon", "parts": [{"pts": rect(0, 0, 40, 40), "holes": []},
                                                        {"pts": rect(100, 0, 140, 40), "holes": [{"pts": rect(110, 10, 120, 20)}]}]}})
    out.append({"name": "delik ekle: yaylı halka yayıyla", "op": "holeAdd", "shape": polygon(SQUARE),
                "ring": {"pts": pts((20, 20), (40, 20), (40, 40), (20, 40)), "bulges": [0, 0, 0.25, 0]},
                "expect": {"kind": "polygon", "parts": [{"pts": SQUARE, "holes": [
                    {"pts": pts((20, 20), (40, 20), (40, 40), (20, 40)), "bulges": [0, 0, 0.25, 0]}]}]}})
    out.append({"name": "delik ekle: sınırı aşan halka", "op": "holeAdd", "shape": polygon(SQUARE),
                "ring": {"pts": rect(90, 10, 110, 30)}, "refusal": "outside"})
    out.append({"name": "delik ekle: sınıra değen halka", "op": "holeAdd", "shape": polygon(SQUARE),
                "ring": {"pts": rect(80, 10, 100, 30)}, "refusal": "outside"})
    out.append({"name": "delik ekle: deliğin içindeki halka", "op": "holeAdd", "shape": HOLED,
                "ring": {"pts": rect(45, 65, 55, 75)}, "refusal": "outside"})
    out.append({"name": "delik ekle: alanın dışındaki halka", "op": "holeAdd", "shape": polygon(SQUARE),
                "ring": {"pts": rect(200, 10, 220, 30)}, "refusal": "outside"})
    out.append({"name": "delik ekle: iki köşeli halka", "op": "holeAdd", "shape": polygon(SQUARE),
                "ring": {"pts": pts((10, 10), (20, 20))}, "refusal": "degenerate"})
    out.append({"name": "deliği sil", "op": "holeRemove", "shape": HOLED, "at": P(50, 70),
                "expect": {"kind": "polygon", "parts": [{"pts": SQUARE, "holes": []}]}})
    out.append({"name": "deliği sil: delik dışına tıklama", "op": "holeRemove", "shape": HOLED, "at": P(20, 20),
                "refusal": "notInHole"})
    out.append({"name": "deliği doldur: deliğin halkası", "op": "holeRing", "shape": HOLED, "at": P(50, 70),
                "expect": {"ring": {"pts": rect(40, 60, 60, 80)}}})
    out.append({"name": "deliği doldur: delik dışına tıklama", "op": "holeRing", "shape": polygon(SQUARE), "at": P(50, 70),
                "refusal": "notInHole"})
    # Sürdür.
    flat = polyline(pts((0, 0), (100, 0)))
    bent = polyline(pts((0, 0), (50, 0), (100, 50)), [0, 0.3, 0])
    out.append(continue_case("sürdür: sondan iki nokta", flat, False, pts((100, 0), (120, 20), (140, 20))))
    out.append(continue_case("sürdür: baştan, nesnenin yönü korunur", polyline(pts((0, 0), (100, 0), (100, 50))), True,
                             pts((0, 0), (-20, 0), (-20, -30))))
    out.append(continue_case("sürdür: baştan yaylı çizim, yay öbür yana döner", flat, True,
                             pts((0, 0), (-20, 20), (-40, 20)), [0.4, 0]))
    out.append(continue_case("sürdür: yaylı nesnenin sonundan yayla", bent, False, pts((100, 50), (100, 80), (70, 110)), [0, -0.25]))
    out.append(continue_case("sürdür: çizgi (line) çoklu çizgi olur", {"kind": "line", "a": P(0, 0), "b": P(50, 0)}, True,
                             pts((0, 0), (0, 30))))
    out.append(continue_case("sürdür: ucun kendisi, eklenecek nokta yok", flat, False, pts((100, 0), (100, 0)), none=True))
    out.append(continue_case("sürdür: alan sürdürülmez", polygon(SQUARE), False, pts((0, 0), (-10, 0)), none=True))
    out.append(ends_case("uçlar: yaylı çoklu çizgi", bent))
    out.append(ends_case("uçlar: çizgi (line)", {"kind": "line", "a": P(10, 10), "b": P(40, 50)}))
    return out


def path_points(shape):
    return shape["pts"] if shape["kind"] == "polyline" else [shape["a"], shape["b"]]


def continued(shape, from_first, drawn, bulges):
    """The ADR's rule, vertex by vertex: the object's own per-vertex bulges (none: 0), the drawn segments' after them
    or, turned, before them."""
    src = path_points(shape)
    sb = list(shape.get("bulges") or [0] * len(src))
    k = len(drawn) - 1
    if from_first:
        out = [drawn[j] for j in range(k, 0, -1)] + src
        ob = [-bulges[j] if bulges[j] else 0 for j in range(k - 1, -1, -1)] + sb[:-1] + [0]
    else:
        out = src + drawn[1:]
        ob = sb[:-1] + list(bulges[:k]) + [0]
    e = {"kind": "polyline", "pts": out}
    if any(ob):
        e["bulges"] = [float(b) for b in ob]
    return e


def continue_case(name, shape, from_first, drawn, bulges=None, none=False):
    bulges = bulges or [0] * (len(drawn) - 1)
    case = {"name": name, "op": "continue", "shape": shape, "fromFirst": from_first, "drawn": drawn,
            "bulges": [float(b) for b in bulges]}
    case["expect"] = None if none else continued(shape, from_first, drawn, bulges)
    return case


def direction(a, b, bulge=0, at_end=True):
    """The travel direction at a segment's end (`at_end`) or start: the chord turned by ±2·atan(bulge)."""
    dx, dy = mp.mpf(b["x"]) - mp.mpf(a["x"]), mp.mpf(b["y"]) - mp.mpf(a["y"])
    l = mp.sqrt(dx * dx + dy * dy)
    half = 2 * mp.atan(mp.mpf(bulge))
    t = half if at_end else -half
    return (dx / l * mp.cos(t) - dy / l * mp.sin(t), dx / l * mp.sin(t) + dy / l * mp.cos(t))


def ends_case(name, shape):
    src = path_points(shape)
    b = shape.get("bulges") or [0] * len(src)
    into = direction(src[0], src[1], b[0], at_end=False)
    onwards = direction(src[-2], src[-1], b[-2], at_end=True)
    return {"name": name, "op": "pathEnds", "shape": shape,
            "expect": {"ends": {"first": src[0], "last": src[-1], "outFirst": P(float(-into[0]), float(-into[1])),
                                "outLast": P(float(onwards[0]), float(onwards[1]))}}}


def build():
    return {"format": "kentos.reshape", "version": 1, "cases": cases()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what the rules give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(json.loads(text)['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
