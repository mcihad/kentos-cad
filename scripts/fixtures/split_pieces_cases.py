"""Parçala's shared cases in cad.shapes.edit (docs/adr/0622 §9, 0140): the `split` work, without KentOS code.

Imported by shape_edits_cases.py, which adds these objects to its drawing (ids from 101, 100 m north of its own) and
writes the cases into fixtures/commands/v1/cad.shapes.edit.json. The rules, from docs/adr/0140:

- a line, an open polyline, an arc and a circle are cut (not an area, a multi-part polyline or anything without a
  length); a piece keeps the kind, except that a straight piece of two points is a line, and a circle's pieces are
  arcs, counter-clockwise from its east point, their angles in [0, 2π);
- Kesişimlerden: each given object that can be cut is cut where the other given objects cross it (any object with
  edges cuts, an area too); an end is not a cut;
- Eşit parçalara: `parts` pieces of equal length, 2 to 10 000;
- Uzunluktan: pieces `length` long from the start (`fromEnd`: from the end), the last one what remains; a circle from
  its east point; a length not shorter than the object leaves it whole, more than 10 000 pieces is refused;
- the first piece keeps the object's place, id and data, the others are new objects after it with the same data, in
  the order the objects were given; one undo step, “Parçala”.

The pieces are found here by walking each path's edges by arc length: straight edges with fractions, arcs with their
angles.
"""
import math

X, Y = 487000, 4420100
FIRST = 100
TAU = 2 * math.pi
R2 = math.sqrt(2)


def W(p):
    return {"x": X + p[0], "y": Y + p[1]}


def U(i):
    return f"$uidOf:{FIRST + i}"


ENTITIES = [
    {"kind": "line", "id": FIRST + 1, "layerId": "yapi", "attrs": {}, "a": W((0, 0)), "b": W((10, 0))},
    {"kind": "line", "id": FIRST + 2, "layerId": "yapi", "attrs": {}, "a": W((5, -5)), "b": W((5, 5))},
    {"kind": "polyline", "id": FIRST + 3, "layerId": "yapi", "attrs": {}, "pts": [W(p) for p in [(20, 0), (30, 0), (30, 10)]]},
    {"kind": "arc", "id": FIRST + 4, "layerId": "yapi", "attrs": {}, "c": W((50, 0)), "r": 10, "a0": 0, "a1": math.pi / 2},
    {"kind": "circle", "id": FIRST + 5, "layerId": "yapi", "attrs": {}, "c": W((70, 0)), "r": 4},
    {"kind": "polygon", "id": FIRST + 6, "layerId": "yapi", "attrs": {},
     "pts": [W(p) for p in [(80, 0), (90, 0), (90, 10), (80, 10)]]},
    {"kind": "line", "id": FIRST + 7, "layerId": "kilitli", "attrs": {}, "a": W((0, 20)), "b": W((10, 20))},
    {"kind": "line", "id": FIRST + 8, "layerId": "yapi", "attrs": {}, "a": W((85, -5)), "b": W((85, 15))},
    {"kind": "polyline", "id": FIRST + 9, "layerId": "yapi", "attrs": {}, "pts": [W(p) for p in [(100, 0), (110, 0)]],
     "parts": [{"pts": [W(p) for p in [(100, 5), (110, 5)]]}]},
    {"kind": "line", "id": FIRST + 10, "layerId": "yapi", "attrs": {"Tür": "Sınır"}, "a": W((0, 30)), "b": W((10, 30))},
    {"kind": "line", "id": FIRST + 11, "layerId": "kilitli", "attrs": {}, "a": W((5, 15)), "b": W((5, 25))},
    {"kind": "line", "id": FIRST + 12, "layerId": "yapi", "attrs": {}, "a": W((25, -5)), "b": W((25, 5))},
    {"kind": "line", "id": FIRST + 13, "layerId": "yapi", "attrs": {}, "a": W((27, 5)), "b": W((35, 5))},
    {"kind": "line", "id": FIRST + 14, "layerId": "yapi", "attrs": {}, "a": W((50, 0)), "b": W((60, 10))},
    {"kind": "line", "id": FIRST + 15, "layerId": "yapi", "attrs": {}, "a": W((60, 0)), "b": W((80, 0))},
]
IDS = []  # the whole drawing's, set by cases()
NEXT = 0
WITHIN = 1e-9


# ── Paths by arc length ──────────────────────────────────────────────────────────────────────────────────────────


def local(p):
    return (p["x"] - X, p["y"] - Y)


def line_piece(a, b):
    return {"kind": "line", "a": W(a), "b": W(b)}


def at(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def polyline_pieces(pts, cuts):
    """An open straight polyline cut at arc lengths `cuts` (sorted, inside): each piece's points."""
    lengths = [math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:])]
    total = sum(lengths)
    bounds = [0.0] + list(cuts) + [total]
    out = []
    for s0, s1 in zip(bounds, bounds[1:]):
        piece = []
        walked = 0.0
        for (a, b), n in zip(zip(pts, pts[1:]), lengths):
            e0, e1 = walked, walked + n
            walked = e1
            if e1 <= s0 or e0 >= s1:
                continue
            start = at(a, b, (s0 - e0) / n) if s0 > e0 else a
            end = at(a, b, (s1 - e0) / n) if s1 < e1 else b
            if not piece:
                piece.append(start)
            piece.append(end)
        out.append(piece)
    return out


def arc_piece(c, r, a0, a1):
    return {"kind": "arc", "c": W(c), "r": r, "a0": a0 % TAU, "a1": a1 % TAU}


def path_piece(pts):
    """A straight polyline piece: a line when it has two points."""
    if len(pts) == 2:
        return line_piece(pts[0], pts[1])
    return {"kind": "polyline", "pts": [W(p) for p in pts]}


# ── Cases ────────────────────────────────────────────────────────────────────────────────────────────────────────


def work(method, ids, **extra):
    w = {"kind": "split", "ids": [U(i) for i in ids], "method": method}
    w.update(extra)
    return w


def done(name, w, cut):
    """`cut`: (object id, [piece fields]) in the given order; the first piece stays the object."""
    fields = {}
    changed, created = [], []
    nxt = NEXT
    for oid, pieces in cut:
        changed.append(U(oid))
        fields[str(FIRST + oid)] = pieces[0]
        for p in pieces[1:]:
            fields[str(nxt)] = p
            created.append(f"$uidOf:{nxt}")
            nxt += 1
    count = sum(len(p) for _, p in cut)
    skipped = len(set(w["ids"])) - len(cut)
    output = {"changed": changed, "before": 0, "after": 0, "objects": len(cut), "skipped": skipped,
              "count": count, "distance": 0, "created": created, "revision": "$current"}
    step = {"op": "execute", "input": {"work": w}, "within": WITHIN,
            "result": {"status": "completed", "output": output, "warnings": []},
            "expect": {"ids": IDS + list(range(NEXT, nxt)), "fields": fields, "canUndo": True, "dirty": True}}
    back = {"op": "undo", "returns": "Parçala", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def refused(name, w, code, message, path, non_finite=None):
    step = {"op": "execute", "input": {"work": w}}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = {"ids": IDS, "dirty": False, "revision": "same"}
    return {"name": name, "steps": [step]}


LOCKED = "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
NOT_CUT = "Bölünecek çizgi, açık çoklu çizgi, yay ya da daire yok; alanları önce Patlat ile çizgilere ayırın."
PARTS = "Parça sayısı 2 ile 10 000 arasında olmalı."
SHORT = "Parça uzunluğu nesnenin boyundan kısa olmalı; hiçbir nesne bölünmedi."


def crossing_cases():
    p45 = (50 + 5 * R2, 5 * R2)
    return [
        done("Parçala: iki çizgi kesişim yerinden", work("crossings", [1, 2]),
             [(1, [line_piece((0, 0), (5, 0)), line_piece((5, 0), (10, 0))]),
              (2, [line_piece((5, -5), (5, 0)), line_piece((5, 0), (5, 5))])]),
        done("Parçala: alan çizgiyi keser, kendisi bölünmez", work("crossings", [6, 8]),
             [(8, [line_piece((85, -5), (85, 0)), line_piece((85, 0), (85, 10)), line_piece((85, 10), (85, 15))])]),
        done("Parçala: çoklu çizgi iki yerinden, çizgiler birer yerinden", work("crossings", [3, 12, 13]),
             [(3, [path_piece(q)
                   for q in polyline_pieces([(20, 0), (30, 0), (30, 10)], [5, 15])]),
              (12, [line_piece((25, -5), (25, 0)), line_piece((25, 0), (25, 5))]),
              (13, [line_piece((27, 5), (30, 5)), line_piece((30, 5), (35, 5))])]),
        done("Parçala: yay ve onu kesen çizgi", work("crossings", [4, 14]),
             [(4, [arc_piece((50, 0), 10, 0, math.pi / 4), arc_piece((50, 0), 10, math.pi / 4, math.pi / 2)]),
              (14, [line_piece((50, 0), p45), line_piece(p45, (60, 10))])]),
        done("Parçala: daire iki yerinden iki yaya", work("crossings", [5, 15]),
             [(5, [arc_piece((70, 0), 4, 0, math.pi), arc_piece((70, 0), 4, math.pi, TAU)]),
              (15, [line_piece((60, 0), (66, 0)), line_piece((66, 0), (74, 0)), line_piece((74, 0), (80, 0))])]),
        refused("Parçala: kesişmeyen nesneler", work("crossings", [1, 8]), "no_intersection",
                "Verilen nesneler birbirini kesmiyor; bölünecek yer yok.", "work.ids"),
        refused("Parçala: tek nesne kesişimden bölünmez", work("crossings", [1]), "too_few_objects",
                "Kesişimlerden bölmek için en az iki nesne verin.", "work.ids"),
        refused("Parçala: kilitli katmandaki nesne bölünmez", work("crossings", [7, 11]), "layer_locked", LOCKED,
                "work"),
    ]


def equal_cases():
    return [
        done("Parçala: çizgi dört eşit parçaya", work("equal", [1], parts=4),
             [(1, [line_piece((x, 0), (x + 2.5, 0)) for x in (0, 2.5, 5, 7.5)])]),
        done("Parçala: çoklu çizgi köşesinden de geçerek dört parçaya", work("equal", [3], parts=4),
             [(3, [path_piece(q)
                   for q in polyline_pieces([(20, 0), (30, 0), (30, 10)], [5, 10, 15])])]),
        done("Parçala: yay üç eşit parçaya", work("equal", [4], parts=3),
             [(4, [arc_piece((50, 0), 10, k * math.pi / 6, (k + 1) * math.pi / 6) for k in range(3)])]),
        done("Parçala: daire dört yaya", work("equal", [5], parts=4),
             [(5, [arc_piece((70, 0), 4, k * math.pi / 2, (k + 1) * math.pi / 2) for k in range(4)])]),
        done("Parçala: parçalar nesnenin özniteliklerini taşır", work("equal", [1, 10], parts=2),
             [(1, [line_piece((0, 0), (5, 0)), line_piece((5, 0), (10, 0))]),
              (10, [{**line_piece((0, 30), (5, 30)), "attrs": {"Tür": "Sınır"}},
                    {**line_piece((5, 30), (10, 30)), "attrs": {"Tür": "Sınır"}, "layerId": "yapi"}])]),
        done("Parçala: bölünmeyen nesneler sayılır", work("equal", [1, 6, 9], parts=2),
             [(1, [line_piece((0, 0), (5, 0)), line_piece((5, 0), (10, 0))])]),
        refused("Parçala: tek parça", work("equal", [1], parts=1), "invalid_count", PARTS, "work.parts"),
        refused("Parçala: çok parça", work("equal", [1], parts=10001), "invalid_count", PARTS, "work.parts"),
        refused("Parçala: parça sayısı yok", work("equal", [1]), "invalid_count", PARTS, "work.parts"),
        refused("Parçala: alan bölünmez", work("equal", [6], parts=2), "not_a_path", NOT_CUT, "work.ids"),
        refused("Parçala: çok parçalı çizgi bölünmez", work("equal", [9], parts=2), "not_a_path", NOT_CUT,
                "work.ids"),
        refused("Parçala: kilitli katmanda eşit parçalar", work("equal", [7], parts=2), "layer_locked", LOCKED,
                "work"),
    ]


def length_cases():
    return [
        done("Parçala: çizgi 3 m'lik parçalara, kalan sonda", work("length", [1], length=3),
             [(1, [line_piece((0, 0), (3, 0)), line_piece((3, 0), (6, 0)), line_piece((6, 0), (9, 0)),
                   line_piece((9, 0), (10, 0))])]),
        done("Parçala: sondan ölçülür, kalan başta", work("length", [1], length=3, fromEnd=True),
             [(1, [line_piece((0, 0), (1, 0)), line_piece((1, 0), (4, 0)), line_piece((4, 0), (7, 0)),
                   line_piece((7, 0), (10, 0))])]),
        done("Parçala: çoklu çizgi 6 m'lik parçalara", work("length", [3], length=6),
             [(3, [path_piece(q)
                   for q in polyline_pieces([(20, 0), (30, 0), (30, 10)], [6, 12, 18])])]),
        done("Parçala: daire doğusundan çeyrek boylarda", work("length", [5], length=2 * math.pi),
             [(5, [arc_piece((70, 0), 4, k * math.pi / 2, (k + 1) * math.pi / 2) for k in range(4)])]),
        refused("Parçala: uzunluk sıfır", work("length", [1], length=0), "invalid_distance",
                "Parça uzunluğu sıfırdan büyük olmalı.", "work.length"),
        refused("Parçala: uzunluk yok", work("length", [1]), "invalid_distance",
                "Parça uzunluğu sıfırdan büyük olmalı.", "work.length"),
        refused("Parçala: uzunluk nesne boyunda", work("length", [1], length=10), "invalid_distance", SHORT,
                "work.length"),
        refused("Parçala: 10 000'den çok parça", work("length", [1], length=0.0001), "too_many_points",
                "Bu uzunlukla 10 000'den çok parça olur; uzunluğu artırın.", "work.length"),
        refused("Parçala: uzunluk sonlu değil", work("length", [1], length=1), "not_finite",
                "Uzunluk sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "work.length",
                non_finite={"work.length": "Infinity"}),
    ]


def cases(ids):
    """The cases in a drawing whose objects are `ids` (these among them)."""
    global IDS, NEXT
    IDS, NEXT = list(ids), max(ids) + 1
    return crossing_cases() + equal_cases() + length_cases()
