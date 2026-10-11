#!/usr/bin/env python3
"""Doğrusal referans'ın resimlerinin çizimi ve olay tabloları (docs/adr/0224): bir devlet yolu (D-100: 600 m düz, 900 m
yarıçaplı 450 m'lik sola viraj, 650 m düz; km'si 12+000'dan başlar) ve kavşağından kuzeye ayrılan yol (D-200, 0'dan), ölçülü
rotalar olarak; aynı yolların parçaları (Rota oluştur'un girdisi, Yol alanıyla), D-100'ün km taşları (Km; eski ölçümün birkaç
metre farkıyla, Rotayı kalibre et'in noktaları), yolların yanındaki trafik kazaları (No, Tür, Yıl; Rota boyunca konumla'nın
nesneleri) ve dört olay tablosu: kaplama ve hız sınırı kesimleri (Olayları çakıştır'ın iki tablosu), korkuluklar (çizgi
olayları, ötelemeleriyle yolun iki yanında) ve trafik levhaları (nokta olayları, yolun iki yanında).

    python3 scripts/fixtures/lrs_scene.py          # fixtures/interaction/v1/lrs.kcad'i ve lrs/*.csv'yi yazar
    python3 scripts/fixtures/lrs_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz: köşeler milimetreye yuvarlanır; D-100'ün ölçüleri tasarımın uzunluklarıdır, D-200'ünkiler
yuvarlanmış köşelerden uzunlukla milimetreye; km taşları ve kazalar yolun o uzunluktaki yerinden.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "interaction" / "v1" / "lrs.kcad"
TABLES = ROOT / "fixtures" / "interaction" / "v1" / "lrs"
E, N = 487000, 4420000
START = 12000.0
RADIUS = 900.0
TURN = 0.5  # radians, to the left


def mm(v):
    return round(v, 3)


def xy(p):
    return {"x": mm(E + p[0]), "y": mm(N + p[1])}


# D-100: P0 → P1 straight east, a left curve to P2 tangent at both ends, P2 → P3 straight.
P0 = (-900.0, -140.0)
P1 = (-300.0, -140.0)
CENTRE = (P1[0], P1[1] + RADIUS)
P2 = (mm(CENTRE[0] + RADIUS * math.sin(TURN)), mm(CENTRE[1] - RADIUS * math.cos(TURN)))
P3 = (mm(P2[0] + 650 * math.cos(TURN)), mm(P2[1] + 650 * math.sin(TURN)))
BULGE = math.tan(TURN / 4)
# D-200 from the junction at P1, north and a little west.
Q = [P1, (-330.0, 160.0), (-420.0, 470.0)]


def arc_of(a, b, bulge):
    """The arc from a to b of this bulge: its centre, radius, start angle and sweep (counter-clockwise positive)."""
    theta = 4 * math.atan(bulge)
    c = math.hypot(b[0] - a[0], b[1] - a[1])
    r = c / (2 * math.sin(abs(theta) / 2))
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    # The centre lies off the chord's middle, to the left of a → b for a counter-clockwise arc.
    d = math.sqrt(max(r * r - (c / 2) ** 2, 0.0))
    ux, uy = (b[0] - a[0]) / c, (b[1] - a[1]) / c
    side = 1 if theta > 0 else -1
    cx, cy = mx - uy * d * side, my + ux * d * side
    if abs(theta) > math.pi:
        cx, cy = mx + uy * d * side, my - ux * d * side
    a0 = math.atan2(a[1] - cy, a[0] - cx)
    return (cx, cy), r, a0, theta


def edges(pts, bulges):
    """Each edge: its length and a function from a length along it to the point there and the unit direction."""
    out = []
    for i in range(len(pts) - 1):
        a, b = pts[i], pts[i + 1]
        bulge = bulges[i] if bulges else 0.0
        if bulge == 0.0:
            ln = math.hypot(b[0] - a[0], b[1] - a[1])
            u = ((b[0] - a[0]) / ln, (b[1] - a[1]) / ln)
            out.append((ln, lambda s, a=a, u=u: ((a[0] + u[0] * s, a[1] + u[1] * s), u)))
        else:
            (cx, cy), r, a0, theta = arc_of(a, b, bulge)
            ln = r * abs(theta)
            sign = 1 if theta > 0 else -1

            def at(s, cx=cx, cy=cy, r=r, a0=a0, sign=sign):
                ang = a0 + sign * s / r
                return (cx + r * math.cos(ang), cy + r * math.sin(ang)), (-math.sin(ang) * sign, math.cos(ang) * sign)

            out.append((ln, at))
    return out


def walk(pts, bulges=None):
    """The lengths walked to each vertex and a function from a length to the point and the direction there."""
    es = edges(pts, bulges)
    along = [0.0]
    for ln, _ in es:
        along.append(along[-1] + ln)

    def at(s):
        for k, ((ln, f), s0) in enumerate(zip(es, along)):
            if s <= s0 + ln or k == len(es) - 1:
                return f(s - s0)
        raise ValueError(s)

    return along, at


D100 = [P0, P1, P2, P3]
D100_BULGES = [0.0, BULGE, 0.0]
D100_ALONG, D100_AT = walk(D100, D100_BULGES)
D200_ALONG, D200_AT = walk(Q)
# The route's measures are its design's lengths: 12+000, 12+600, 13+050, 13+700.
D100_MS = [START, START + 600, START + 600 + RADIUS * TURN, START + 600 + RADIUS * TURN + 650]
D200_MS = [mm(s) for s in D200_ALONG]


def beside(at, s, offset):
    """The place `offset` m right (+) of a road at length `s` along it."""
    (x, y), (ux, uy) = at(s)
    return (x + uy * offset, y - ux * offset)


def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": st, "children": []}


ACCIDENTS = [
    ("D-100", 120, 6, "Çarpışma", 2023),
    ("D-100", 410, -9, "Yaya", 2024),
    ("D-100", 640, 4, "Kayma", 2025),
    ("D-100", 720, -12, "Çarpışma", 2024),
    ("D-100", 980, 7, "Devrilme", 2023),
    ("D-100", 1210, -5, "Çarpışma", 2025),
    ("D-100", 1460, 10, "Yaya", 2024),
    ("D-100", 1690, -3, "Kayma", 2023),
    ("D-200", 80, 5, "Çarpışma", 2025),
    ("D-200", 300, -8, "Yaya", 2023),
    ("D-200", 450, 12, "Devrilme", 2024),
    ("D-200", 600, -4, "Çarpışma", 2025),
]
# The km stones: where they stand along D-100 (m) and what an older survey wrote on them.
STONES = [(500, 12510), (1000, 13015), (1500, 13520)]


def drawing():
    entities = []

    def add(e, lid, attrs):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs})

    # The measured routes.
    add({"kind": "polyline", "pts": [xy(p) for p in D100], "bulges": D100_BULGES, "ms": D100_MS}, "rota", {"Rota": "D-100", "Ad": "Devlet yolu"})
    add({"kind": "polyline", "pts": [xy(p) for p in Q], "ms": D200_MS}, "rota", {"Rota": "D-200", "Ad": "Kuzey bağlantısı"})
    # The same roads in pieces, as a survey gives them: Rota oluştur's input.
    add({"kind": "line", "a": xy(P0), "b": xy(P1)}, "yol", {"Yol": "D-100"})
    add({"kind": "polyline", "pts": [xy(P1), xy(P2)], "bulges": [BULGE]}, "yol", {"Yol": "D-100"})
    add({"kind": "line", "a": xy(P2), "b": xy(P3)}, "yol", {"Yol": "D-100"})
    add({"kind": "line", "a": xy(Q[0]), "b": xy(Q[1])}, "yol", {"Yol": "D-200"})
    add({"kind": "line", "a": xy(Q[1]), "b": xy(Q[2])}, "yol", {"Yol": "D-200"})
    for s, km in STONES:
        add({"kind": "point", "p": xy(D100_AT(s)[0])}, "km", {"Rota": "D-100", "Km": str(km)})
    for no, (route, s, offset, kind, year) in enumerate(ACCIDENTS, start=1):
        at = D100_AT if route == "D-100" else D200_AT
        add({"kind": "point", "p": xy(beside(at, s, offset))}, "kaza", {"No": f"K{no:02d}", "Tür": kind, "Yıl": str(year)})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Doğrusal referans",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 5000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": E, "y": N},
        "layers": [layer("kaza", "Kazalar", style("#D6409F", point={"symbol": "ring", "size": 5})),
                   layer("km", "Km taşları", style("#F76B15", point={"symbol": "triangle", "size": 6})),
                   layer("rota", "Ölçülü rotalar", style("#0D74CE", 0.7)),
                   layer("yol", "Yol parçaları", style("#8C9AAA", 0.35))],
        "activeLayer": "kaza",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def end(ms):
    """A route's last measure as a table writes it: millimetres, no trailing zeros."""
    return f"{ms[-1]:.3f}".rstrip("0").rstrip(".")


def tables():
    """The event tables, semicolons between the cells (the dialog reads both)."""
    paving = [
        ("D-100", "12000", "12450", "asfalt"),
        ("D-100", "12450", "12900", "beton"),
        ("D-100", "12900", "13350", "asfalt"),
        ("D-100", "13350", end(D100_MS), "parke"),
        ("D-200", "0", "350", "asfalt"),
        ("D-200", "350", end(D200_MS), "stabilize"),
    ]
    # Guardrails beside the roads: both sides round the curve, the right of the last straight, the left of D-200.
    rails = [
        ("D-100", "12550", "13100", "7", "çelik"),
        ("D-100", "12550", "13100", "-7", "çelik"),
        ("D-100", "13400", "13650", "7", "beton"),
        ("D-200", "200", "420", "-6", "çelik"),
    ]
    speed = [
        ("D-100", "12000", "12700", "90"),
        ("D-100", "12700", "13300", "50"),
        ("D-100", "13300", end(D100_MS), "70"),
        ("D-200", "0", end(D200_MS), "50"),
    ]
    signs = [
        ("D-100", "12080", "-9", "Hız 90"),
        ("D-100", "12560", "9", "Dikkat viraj"),
        ("D-100", "12700", "-9", "Hız 50"),
        ("D-100", "13240", "9", "Okul geçidi"),
        ("D-100", "13300", "-9", "Hız 70"),
        ("D-200", "150", "7", "Dur"),
        ("D-200", "500", "-7", "Köprü"),
    ]

    def csv(header, rows):
        return "\n".join(";".join(r) for r in [header, *rows]) + "\n"

    return {
        "kaplama.csv": csv(("Rota", "Bas", "Son", "Kaplama"), paving),
        "korkuluk.csv": csv(("Rota", "Bas", "Son", "Ofset", "Tür"), rails),
        "hiz.csv": csv(("Rota", "Bas", "Son", "Hiz"), speed),
        "levhalar.csv": csv(("Rota", "Km", "Ofset", "Levha"), signs),
    }


def main():
    files = {OUT: json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"}
    files.update({TABLES / name: text for name, text in tables().items()})
    if "--check" in sys.argv[1:]:
        stale = [p for p, text in files.items() if not p.exists() or p.read_text("utf-8") != text]
        for p in stale:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        if stale:
            return 1
        print("fixtures/interaction/v1/lrs.kcad ve lrs/: güncel.")
        return 0
    TABLES.mkdir(parents=True, exist_ok=True)
    for p, text in files.items():
        p.write_text(text, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
