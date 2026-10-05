#!/usr/bin/env python3
"""Writes fixtures/template-members/v1/cases.json: what a group template's
members make from the drawn shape (docs/adr/0176 §5), worked out here from the
rule with exact fractions, without KentOS code.

The rule:

- An offset member makes the shape's parallels at its distance: one on its
  side, two for “both” (left or inside first). On an open shape (a line, a
  polyline) the side is left or right of the drawing's direction; on a closed
  one (an area) inside or outside, whichever way it was drawn. A parallel is
  Ötele's: every segment moved by the distance along its left normal (or
  right), neighbours joined where their lines meet (a closed ring's first
  point is where its last and first segments meet); a meeting farther than
  four distances from the corner is cut, the two segments' ends both kept.
  An arc of a polyline is the same circle's arc at the radius moved by the
  distance (here: a half circle, its bulge kept; a bulge per vertex, the
  open path's last one 0).
- An unknown side, a distance not above zero, an area of several parts, a
  shape that is not a line, a polyline or an area, and a side that does not
  fit the shape are refused, in this order, each with its message.
- An Ağırlık merkezine member's point is the label's place: an area's
  centroid, a line's middle, a polyline's middle vertex (the one at half its
  count, rounded down), none for a polyline without vertices.

The core (`kentos_geometry_core::ops::template_members`, called by name as
`templateMemberOffsets` and `templateMemberCentroid`) natively and through WASM
gives these within 1e-9 m.

    python3 scripts/fixtures/template_member_cases.py          # writes the cases
    python3 scripts/fixtures/template_member_cases.py --check  # fails when they differ
"""

import json
import sys
from fractions import Fraction as F

PATH = "fixtures/template-members/v1/cases.json"
MITRE_LIMIT = 4


def P(x, y):
    return {"x": x, "y": y}


def pt(p):
    return (F(p["x"]), F(p["y"]))


def out(p):
    return {"x": float(p[0]), "y": float(p[1])}


def meet(a1, b1, a2, b2):
    """Where the lines a1b1 and a2b2 meet; None for parallel lines."""
    d1 = (b1[0] - a1[0], b1[1] - a1[1])
    d2 = (b2[0] - a2[0], b2[1] - a2[1])
    den = d1[0] * d2[1] - d1[1] * d2[0]
    if den == 0:
        return None
    t = ((a2[0] - a1[0]) * d2[1] - (a2[1] - a1[1]) * d2[0]) / den
    return (a1[0] + t * d1[0], a1[1] + t * d1[1])


def unit_normal(a, b):
    """The left normal of a→b; only for the cases' segments, whose lengths are whole (3-4-5, 5-12-13 or axis)."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    length2 = dx * dx + dy * dy
    for n in range(1, 1000):
        if F(n * n) == length2:
            return (-dy / n, dx / n)
    raise ValueError(f"a segment of a length that is not whole: {a} → {b}")


def offset_path(pts, d, closed):
    n = len(pts)
    count = n if closed else n - 1
    segs = []
    for i in range(count):
        a, b = pts[i], pts[(i + 1) % n]
        if a == b:
            continue
        nx, ny = unit_normal(a, b)
        segs.append(((a[0] + nx * d, a[1] + ny * d), (b[0] + nx * d, b[1] + ny * d)))
    res = []

    def join(s1, s2, corner):
        h = meet(s1[0], s1[1], s2[0], s2[1])
        if h is None:
            res.append(s2[0])
        elif (h[0] - corner[0]) ** 2 + (h[1] - corner[1]) ** 2 > (MITRE_LIMIT * abs(d)) ** 2:
            res.append(s1[1])
            res.append(s2[0])
        else:
            res.append(h)

    m = len(segs)
    if closed:
        for i in range(m):
            join(segs[(i + m - 1) % m], segs[i], pts[i])
    else:
        res.append(segs[0][0])
        for i in range(1, m):
            join(segs[i - 1], segs[i], pts[i])
        res.append(segs[m - 1][1])
    return res


def signed_area(pts):
    return sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts))) / 2


SIDES = {"left", "right", "inside", "outside", "both"}


def offsets(entity, distance, side):
    # In this order: the side's name, the distance, the parts, the kind, the side for the shape.
    if side not in SIDES:
        return {"error": f"Bilinmeyen öteleme yanı “{side}”."}
    if not distance > 0:
        return {"error": "Öteleme mesafesi sıfırdan büyük olmalı."}
    kind = entity["kind"]
    if entity.get("parts"):
        return {"error": "Çok parçalı nesne grup şablonunun üyesiyle ötelenmez."}
    if kind not in ("line", "polyline", "polygon"):
        return {"error": "Grup şablonunun ötelemesi yalnız çizgi, çoklu çizgi ve kapalı alanda olur."}
    closed = kind == "polygon"
    if closed and side not in ("inside", "outside", "both"):
        return {"error": "Kapalı şekil içe, dışa ya da iki yana ötelenir."}
    if not closed and side not in ("left", "right", "both"):
        return {"error": "Açık şekil sola, sağa ya da iki yana ötelenir."}
    signs = {"left": [1], "right": [-1], "inside": [1], "outside": [-1], "both": [1, -1]}[side]
    pts = [pt(p) for p in entity["pts"]] if kind != "line" else [pt(entity["a"]), pt(entity["b"])]
    # Left of the drawing's direction is inside a counter-clockwise ring.
    factor = (1 if signed_area(pts) > 0 else -1) if closed else 1
    geometries = []
    for s in signs:
        d = F(s * factor) * F(distance)
        bulges = entity.get("bulges")
        if bulges and any(b != 0 for b in bulges):
            geometries.append(half_circle(entity, d))
        elif kind == "line":
            a, b = offset_path(pts, d, False)
            geometries.append({"kind": "line", "a": out(a), "b": out(b)})
        else:
            geometries.append({"kind": kind, "pts": [out(p) for p in offset_path(pts, d, closed)]})
    return {"geometries": geometries}


def half_circle(entity, d):
    """A polyline of one half circle (bulge 1 or −1): the same circle's half at the radius moved by `d` (left: toward
    the centre of a counter-clockwise arc)."""
    (a, b), (bulge, _) = [pt(p) for p in entity["pts"]], entity["bulges"]
    assert abs(bulge) == 1
    c = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    # Counter-clockwise (bulge 1): left is toward the centre.
    inward = d if bulge > 0 else -d
    r = (abs(b[0] - a[0]) + abs(b[1] - a[1])) / 2  # the cases' arcs have an axis-parallel chord
    k = (r - inward) / r
    moved = [(c[0] + (p[0] - c[0]) * k, c[1] + (p[1] - c[1]) * k) for p in (a, b)]
    # A bulge per vertex: the last one's, after the open path's end, none (0).
    return {"kind": "polyline", "pts": [out(p) for p in moved], "bulges": [float(bulge), 0.0]}


def centroid(entity):
    kind = entity["kind"]
    if kind == "polygon":
        pts = [pt(p) for p in entity["pts"]]
        a = signed_area(pts)
        cx = cy = F(0)
        for i in range(len(pts)):
            (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % len(pts)]
            cross = x0 * y1 - x1 * y0
            cx += (x0 + x1) * cross
            cy += (y0 + y1) * cross
        return out((cx / (6 * a), cy / (6 * a)))
    if kind == "line":
        a, b = pt(entity["a"]), pt(entity["b"])
        return out(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2))
    if kind == "polyline":
        pts = entity["pts"]
        return out(pt(pts[len(pts) // 2])) if pts else None
    raise ValueError(kind)


SQUARE_CCW = [P(0, 0), P(10, 0), P(10, 10), P(0, 10)]
SQUARE_CW = [P(0, 0), P(0, 10), P(10, 10), P(10, 0)]
L_SHAPE = [P(0, 0), P(10, 0), P(10, 10)]
OFFSET_CASES = [
    ("çizgi sola", {"kind": "line", "a": P(0, 0), "b": P(10, 0)}, 2, "left"),
    ("çizgi sağa", {"kind": "line", "a": P(0, 0), "b": P(10, 0)}, 2, "right"),
    ("eğik çizgi iki yana: önce sol", {"kind": "line", "a": P(0, 0), "b": P(3, 4)}, 5, "both"),
    ("çoklu çizgi sola: köşe içte", {"kind": "polyline", "pts": L_SHAPE}, 1, "left"),
    ("çoklu çizgi sağa: köşe dışta", {"kind": "polyline", "pts": L_SHAPE}, 1, "right"),
    ("çoklu çizgi iki yana", {"kind": "polyline", "pts": L_SHAPE}, 1, "both"),
    ("keskin dönüşün dışı: köşe kırpılır (dört uzaklığın ötesi)", {"kind": "polyline", "pts": [P(0, 0), P(10, 0), P(-2, 5)]}, 1, "right"),
    ("kapalı alan saat yönünün tersine: içe", {"kind": "polygon", "pts": SQUARE_CCW}, 1, "inside"),
    ("kapalı alan saat yönünde: içe yine içe", {"kind": "polygon", "pts": SQUARE_CW}, 1, "inside"),
    ("kapalı alan dışa", {"kind": "polygon", "pts": SQUARE_CCW}, 1, "outside"),
    ("saat yönündeki alan dışa", {"kind": "polygon", "pts": SQUARE_CW}, 2, "outside"),
    ("kapalı alan iki yana: önce iç", {"kind": "polygon", "pts": SQUARE_CCW}, 1, "both"),
    ("yarım çember sola: merkeze doğru", {"kind": "polyline", "pts": [P(0, 0), P(10, 0)], "bulges": [1, 0]}, 1, "left"),
    ("yarım çember sağa", {"kind": "polyline", "pts": [P(0, 0), P(10, 0)], "bulges": [1, 0]}, 1, "right"),
    ("saat yönündeki yarım çember sola: merkezden dışa", {"kind": "polyline", "pts": [P(0, 0), P(10, 0)], "bulges": [-1, 0]}, 1, "left"),
    ("sıfır uzaklık reddedilir", {"kind": "line", "a": P(0, 0), "b": P(10, 0)}, 0, "left"),
    ("açık şekle iç yan reddedilir", {"kind": "line", "a": P(0, 0), "b": P(10, 0)}, 1, "inside"),
    ("kapalı şekle sol yan reddedilir", {"kind": "polygon", "pts": SQUARE_CCW}, 1, "left"),
    ("bilinmeyen yan reddedilir", {"kind": "polyline", "pts": L_SHAPE}, 1, "up"),
    ("daire ötelenmez", {"kind": "circle", "c": P(0, 0), "r": 5}, 1, "both"),
    ("çok parçalı alan ötelenmez", {"kind": "polygon", "pts": SQUARE_CCW, "parts": [{"pts": [P(20, 0), P(30, 0), P(30, 10)], "holes": []}]}, 1, "inside"),
]
CENTROID_CASES = [
    ("kare", {"kind": "polygon", "pts": SQUARE_CCW}),
    ("saat yönündeki kare", {"kind": "polygon", "pts": SQUARE_CW}),
    ("L biçimli alan: ağırlık merkezi", {"kind": "polygon", "pts": [P(0, 0), P(4, 0), P(4, 1), P(1, 1), P(1, 4), P(0, 4)]}),
    ("üçgen", {"kind": "polygon", "pts": [P(0, 0), P(6, 0), P(0, 3)]}),
    ("çizginin ortası", {"kind": "line", "a": P(0, 0), "b": P(10, 4)}),
    ("çoklu çizginin ortadaki köşesi", {"kind": "polyline", "pts": [P(0, 0), P(4, 0), P(4, 4), P(8, 4)]}),
    ("üç köşeli çoklu çizgi: ortadaki", {"kind": "polyline", "pts": [P(0, 0), P(4, 0), P(4, 4)]}),
]


def build():
    return {
        "format": "kentos.template-member-cases",
        "version": 1,
        "note": (
            "ADR 0176 §5: grup şablonunun üyelerinin geometrisi. Öteleme üyesi şeklin paralellerini uzaklığında yazar: yanında "
            "bir, iki yanda iki (önce sol ya da iç). Açık şekilde (çizgi, çoklu çizgi) yan çizim yönünün solu ya da sağıdır; kapalı "
            "şekilde (alan) çizim yönü ne olursa olsun iç ya da dış. Paralel Ötele'nindir: her parça sol (ya da sağ) normali boyunca "
            "uzaklık kadar kayar, komşular doğrularının kesiştiği yerde birleşir (kapalı halkanın ilk noktası son ve ilk parçanın "
            "buluştuğu yer); köşeden dört uzaklıktan öte buluşma kırpılır, iki parçanın uçları kalır. Çoklu çizginin yayı aynı "
            "çemberin yarıçapı uzaklık kadar değişmiş yayıdır. Sıfırdan büyük olmayan uzaklık, şekle uymayan ya da bilinmeyen yan, "
            "çizgi, çoklu çizgi ve alan dışındaki şekil ve çok parçalı alan iletisiyle reddedilir. Ağırlık merkezi üyesinin noktası "
            "etiketin yeridir: alanın ağırlık merkezi, çizginin ortası, çoklu çizginin ortadaki köşesi (köşe sayısının yarısı, "
            "aşağı yuvarlanmış). Sayılar 1e-9 m içinde. Üretici: scripts/fixtures/template_member_cases.py (KentOS kodu olmadan, "
            "kesin kesirlerle)."
        ),
        "offsets": [
            {"name": name, "entity": entity, "distance": distance, "side": side, "expected": offsets(entity, distance, side)}
            for name, entity, distance, side in OFFSET_CASES
        ],
        "centroids": [{"name": name, "entity": entity, "expected": centroid(entity)} for name, entity in CENTROID_CASES],
    }


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(", ", ": "))


def text_of(doc):
    lines = ["{"]
    for key in ("format", "version", "note"):
        lines.append(f"  {json.dumps(key)}: {compact(doc[key])},")
    lines.append('  "offsets": [')
    lines.append(",\n".join(f"    {compact(c)}" for c in doc["offsets"]))
    lines.append("  ],")
    lines.append('  "centroids": [')
    lines.append(",\n".join(f"    {compact(c)}" for c in doc["centroids"]))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main():
    text = text_of(build())
    if "--check" in sys.argv[1:]:
        with open(PATH, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{PATH} is not what this script writes: run it without --check and read the difference.")
        print(f"{len(OFFSET_CASES)} + {len(CENTROID_CASES)} cases match")
        return
    with open(PATH, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"{len(OFFSET_CASES)} + {len(CENTROID_CASES)} cases written")


if __name__ == "__main__":
    main()
