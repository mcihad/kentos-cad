"""The shared cases of Plan yolu çizimi (docs/adr/0198): a road's areas from
its axis (the road, the carriageway inside its kerbs, the median closed with
half circles), the inner corners of an area rounded, and two lines closed
into a median.

    python3 scripts/fixtures/plan_road_cases.py           # writes the file
    python3 scripts/fixtures/plan_road_cases.py --check   # writes nothing; compares

Writes fixtures/plan-road/v1/cases.json. The rules are written here from the
ADR on their own, not from an implementation's output; the geometry core
(crates/shared/geometry-core, `ops::road`, natively and through WASM:
`roadParts`, `roundInnerCorners`, `medianRing`) is held to them within 1e-9,
a ring compared from any of its vertices.

- Offset of an open axis by d (left of travel positive): each edge moved d
  along its left normal; inner vertices where the moved edges meet; the ends
  moved square to their edges. The cases keep every corner mitred.
- A corridor of widths l (left) and r (right): the left side, then the right
  side backwards; counter-clockwise (reversed when its signed area is
  negative). A road of width G is the corridor G/2, G/2; with kerbs K the
  carriageway is the corridor G/2 − K on both sides.
- A median of width R: the sides at ±R/2 closed into a ring; the two edges
  between the sides' ends are half circles (bulge 1 once the ring is
  counter-clockwise: outwards).
- Two lines into a median: the second taken as it is when its start is
  nearer the first's end than its end is, else reversed (points backwards,
  bulges backwards and negated); the ring is the first's points then the
  second's; counter-clockwise by its vertices' signed area; the two joining
  edges half circles (bulge 1) or straight.
- Inner corners: on a ring with the area's inside on its left (an outer ring
  whose vertices' signed area is positive, a hole whose is negative), a right
  turn; on the other, a left turn. A corner between two straight edges whose
  turn is at least 1e-6 rad and at most π − 1e-6 is rounded with radius R:
  d = R·tan(turn/2) from the vertex along each edge; it is left when d
  exceeds either edge by more than 1e-9·max(1, length), and two corners are
  both left when together they need more than the edge between them. A
  rounded corner becomes its two tangent points, the arc between them of
  bulge ±tan(turn/4) (positive for a left turn). Counts: corners rounded,
  inner corners left.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/plan-road/v1/cases.json"
STRAIGHT = 1e-6


def P(x, y):
    return {"x": x, "y": y}


def xy(p):
    return (p["x"], p["y"])


def offset(pts, d):
    segs = []
    for (ax, ay), (bx, by) in zip(pts, pts[1:]):
        l = math.hypot(bx - ax, by - ay)
        nx, ny = -(by - ay) / l * d, (bx - ax) / l * d
        segs.append(((ax + nx, ay + ny), (bx + nx, by + ny)))
    out = [segs[0][0]]
    for (a1, b1), (a2, b2) in zip(segs, segs[1:]):
        d1 = (b1[0] - a1[0], b1[1] - a1[1])
        d2 = (b2[0] - a2[0], b2[1] - a2[1])
        den = d1[0] * d2[1] - d1[1] * d2[0]
        t = ((a2[0] - a1[0]) * d2[1] - (a2[1] - a1[1]) * d2[0]) / den
        out.append((a1[0] + t * d1[0], a1[1] + t * d1[1]))
    out.append(segs[-1][1])
    return out


def signed(pts):
    s = 0.0
    for i, (x1, y1) in enumerate(pts):
        x2, y2 = pts[(i + 1) % len(pts)]
        s += x1 * y2 - x2 * y1
    return s / 2


def reverse_ring(pts, bulges):
    m = len(pts)
    return pts[::-1], [-bulges[(m - 2 - k) % m] + 0.0 for k in range(m)]


def corridor(axis, left, right):
    ring = offset(axis, left) + offset(axis, -right)[::-1]
    if signed(ring) < 0:
        ring = ring[::-1]
    return ring


def close_median(first, fb, second, sb, round_caps):
    n1, n2 = len(first), len(second)
    fb = fb or [0.0] * (n1 - 1)
    sb = sb or [0.0] * (n2 - 1)
    e = first[-1]
    if not math.hypot(second[0][0] - e[0], second[0][1] - e[1]) < math.hypot(second[-1][0] - e[0], second[-1][1] - e[1]):
        second = second[::-1]
        sb = [-b + 0.0 for b in sb[::-1]]
    pts = first + second
    bulges = fb + [0.0] + sb + [0.0]
    caps = {n1 - 1, n1 + n2 - 1}
    marks = [i in caps for i in range(len(pts))]
    if signed(pts) < 0:
        m = len(pts)
        pts, bulges = reverse_ring(pts, bulges)
        marks = [marks[(m - 2 - k) % m] for k in range(m)]
    if round_caps:
        bulges = [1.0 if mk else b for b, mk in zip(bulges, marks)]
    return pts, bulges


def round_inner(pts, area_left, r):
    n = len(pts)
    def edge(i):
        a, b = pts[i], pts[(i + 1) % n]
        return math.hypot(b[0] - a[0], b[1] - a[1])
    corners = []
    for i in range(n):
        p, v, q = pts[(i - 1) % n], pts[i], pts[(i + 1) % n]
        lin, lout = math.hypot(v[0] - p[0], v[1] - p[1]), math.hypot(q[0] - v[0], q[1] - v[1])
        din = ((v[0] - p[0]) / lin, (v[1] - p[1]) / lin)
        dout = ((q[0] - v[0]) / lout, (q[1] - v[1]) / lout)
        turn = math.acos(max(-1.0, min(1.0, din[0] * dout[0] + din[1] * dout[1])))
        cross = din[0] * dout[1] - din[1] * dout[0]
        if turn < STRAIGHT or math.pi - turn < STRAIGHT:
            corners.append(None)
            continue
        inner = cross < 0 if area_left else cross > 0
        corners.append((din, dout, turn, cross, r * math.tan(turn / 2)) if inner else None)
    eps = lambda l: 1e-9 * max(1.0, l)
    take = [c is not None and c[4] <= edge((i - 1) % n) + eps(edge((i - 1) % n)) and c[4] <= edge(i) + eps(edge(i)) for i, c in enumerate(corners)]
    clash = [False] * n
    for i in range(n):
        j = (i + 1) % n
        if take[i] and take[j] and corners[i][4] + corners[j][4] > edge(i) + eps(edge(i)):
            clash[i] = clash[j] = True
    take = [t and not c for t, c in zip(take, clash)]
    out, bulges = [], []
    for i, v in enumerate(pts):
        if take[i]:
            din, dout, turn, cross, d = corners[i]
            out.append((v[0] - d * din[0], v[1] - d * din[1]))
            bulges.append((1 if cross > 0 else -1) * math.tan(turn / 4))
            out.append((v[0] + d * dout[0], v[1] + d * dout[1]))
            bulges.append(0.0)
        else:
            out.append(v)
            bulges.append(0.0)
    inner = sum(c is not None for c in corners)
    done = sum(take)
    return out, bulges, done, inner - done


def ring_json(pts, bulges=None):
    j = {"pts": [P(*p) for p in pts]}
    if bulges and any(b != 0 for b in bulges):
        j["bulges"] = bulges
    return j


def road(axis, width, kerb, median):
    a = [xy(p) for p in axis]
    out = {"road": {"outer": ring_json(corridor(a, width / 2, width / 2)), "holes": []}}
    if kerb > 0:
        out["carriageway"] = {"outer": ring_json(corridor(a, width / 2 - kerb, width / 2 - kerb)), "holes": []}
    if median > 0:
        pts, bulges = close_median(offset(a, median / 2), None, offset(a, -median / 2), None, True)
        out["median"] = {"outer": ring_json(pts, bulges), "holes": []}
    return out


ROADS = [
    ("düz-yol", [P(0, 0), P(60, 0)], 10, 0, 0),
    ("kırık-yol-kaldırımlı", [P(487000, 4420000), P(487040, 4420010), P(487070, 4419990)], 15, 3, 0),
    ("refüjlü-bulvar", [P(0, 0), P(50, 0), P(90, 30)], 30, 4, 2),
    ("yaya-yolu-ters", [P(30, 20), P(0, 20)], 5, 0, 0),
]

PLUS = [(25, 0), (35, 0), (35, 25), (60, 25), (60, 35), (35, 35), (35, 60), (25, 60), (25, 35), (0, 35), (0, 25), (25, 25)]
TEE = [(16, 0), (24, 0), (24, 10), (40, 10), (40, 20), (0, 20), (0, 10), (16, 10)]
SLOT = [(0, 0), (30, 0), (30, 20), (18, 20), (18, 8), (12, 8), (12, 20), (0, 20)]
RING_OUT = [(0, 0), (50, 0), (50, 50), (0, 50)]
RING_HOLE = [(10, 10), (10, 40), (40, 40), (40, 10)]
SLANT = [(0, 0), (40, 0), (40, 10), (60, 30), (52, 38), (35, 21), (35, 10), (0, 10)]

CORNERS = [
    ("artı-kavşak", PLUS, [], 6),
    ("artı-kavşak-uzak", [(487000 + x, 4420000 + y) for x, y in PLUS], [], 4.5),
    ("te-kavşak", TEE, [], 3),
    ("yarık-çakışan", SLOT, [], 4),
    ("artı-sığmayan", PLUS, [], 30),
    ("çevre-yolu-adası", RING_OUT, [RING_HOLE], 5),
    ("ters-yönlü-halka", PLUS[::-1], [], 6),
    ("eğik-kavşak", SLANT, [], 3),
]

MEDIANS = [
    ("aynı-yönlü", [P(0, 0), P(30, 0)], None, [P(0, 4), P(30, 4)], None, True),
    ("ters-yönlü", [P(0, 0), P(30, 0)], None, [P(30, 4), P(0, 4)], None, True),
    ("üstten-başlayan", [P(0, 4), P(30, 4)], None, [P(0, 0), P(30, 0)], None, True),
    ("kırık-kenarlar", [P(0, 0), P(20, 0), P(30, 5)], None, [P(0, 3), P(19.2, 3), P(28.6, 7.7)], None, True),
    ("yaylı-kenar", [P(0, 0), P(20, 0), P(40, 0)], [0, 0.1], [P(40, 3), P(20, 3), P(0, 3)], [-0.1, 0], True),
    ("düz-uç", [P(487000, 4420000), P(487025, 4420000)], None, [P(487000, 4420002.5), P(487025, 4420002.5)], None, False),
]


def build():
    roads = [{"name": n, "axis": axis, "width": w, "kerb": k, "median": m, "want": road(axis, w, k, m)} for n, axis, w, k, m in ROADS]
    corners = []
    for n, outer, holes, r in CORNERS:
        o, ob, done, left = round_inner(outer, signed(outer) > 0, r)
        hs = []
        for h in holes:
            hp, hb, d2, l2 = round_inner(h, signed(h) < 0, r)
            done += d2
            left += l2
            hs.append(ring_json(hp, hb))
        corners.append({
            "name": n,
            "area": {"outer": ring_json(outer), "holes": [ring_json(h) for h in holes]},
            "radius": r,
            "want": {"area": {"outer": ring_json(o, ob), "holes": hs}, "done": done, "skipped": left},
        })
    medians = []
    for n, a, ab, b, bb, rnd in MEDIANS:
        pts, bulges = close_median([xy(p) for p in a], ab, [xy(p) for p in b], bb, rnd)
        medians.append({"name": n, "first": {"pts": a, **({"bulges": ab} if ab else {})}, "second": {"pts": b, **({"bulges": bb} if bb else {})},
                        "round": rnd, "want": ring_json(pts, bulges)})
    return {
        "format": "kentos.plan-road-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/plan_road_cases.py",
        "title": "Plan yolu çizimi: yolun alanları, iç köşelerin yuvarlanması, refüj kapama (ADR 0198)",
        "note": "Halkalar herhangi bir köşesinden başlayarak karşılaştırılır. Yol: G/2 iki yana; kaldırımla taşıt yolu G/2 − K; refüj ±R/2, uçları yarım daire. İç köşe: alanın içi solda iken sağa dönüş; d = R·tan(dönüş/2).",
        "roads": roads,
        "corners": corners,
        "medians": medians,
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=2) + "\n"


def main():
    want = text_of(build())
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != want:
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("plan road cases match")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(want, encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
