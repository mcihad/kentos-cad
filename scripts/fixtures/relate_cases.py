#!/usr/bin/env python3
"""The shared cases of the product command cad.entities.relate (docs/adr/0501, plan/gis.md CBS-22).

    python3 scripts/fixtures/relate_cases.py           # writes the file
    python3 scripts/fixtures/relate_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.relate.json. Every straight-edged pair's DE-9IM matrix is GEOS's, through
QGIS 4.2's Python (`QgsGeometryEngine.relate`, /usr/share/qgis/python); the predicates are read from the matrix by OGC
SFA 1.2.1 §6.1.15.3 as PostGIS reads them, written here. GEOS takes a circle as chords, so the pairs with the circle
are written by hand from the definition (each beside its case). Neither uses KentOS code.
"""
import json
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
sys.path.insert(0, "/usr/share/qgis/python")
from qgis.core import QgsGeometry  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.relate.json"
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": STYLE, "children": []}


def base(kind, i, lay="parsel"):
    return {"kind": kind, "id": i, "layerId": lay, "attrs": {}}


def sq(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def ring_wkt(pts):
    return "(" + ",".join(f"{x} {y}" for x, y in pts + pts[:1]) + ")"


# Each object: its entity and its WKT for GEOS (none: the circle and the construction line).
OBJECTS = {}


def poly(i, outer, holes=(), lay="parsel"):
    e = {**base("polygon", i, lay), "pts": [P(*p) for p in outer]}
    if holes:
        e["holes"] = [{"pts": [P(*p) for p in h]} for h in holes]
    OBJECTS[i] = (e, "POLYGON(" + ",".join(ring_wkt(r) for r in [outer, *holes]) + ")")


def line(i, a, b, lay="yol"):
    OBJECTS[i] = ({**base("line", i, lay), "a": P(*a), "b": P(*b)}, f"LINESTRING({a[0]} {a[1]},{b[0]} {b[1]})")


def path(i, pts, lay="yol"):
    OBJECTS[i] = ({**base("polyline", i, lay), "pts": [P(*p) for p in pts]},
                  "LINESTRING(" + ",".join(f"{x} {y}" for x, y in pts) + ")")


def point(i, p, more=(), lay="nokta"):
    e = {**base("point", i, lay), "p": P(*p)}
    if more:
        e["parts"] = [{"p": P(*q)} for q in more]
        wkt = "MULTIPOINT(" + ",".join(f"({x} {y})" for x, y in [p, *more]) + ")"
    else:
        wkt = f"POINT({p[0]} {p[1]})"
    OBJECTS[i] = (e, wkt)


poly(1, sq(0, 0, 10, 10))
poly(2, sq(10, 0, 20, 10))
poly(3, sq(2, 2, 4, 4))
poly(4, sq(5, 5, 15, 15))
poly(5, sq(30, 0, 40, 10), [list(reversed(sq(33, 3, 37, 7)))])
poly(6, sq(34, 4, 36, 6))
point(7, (10, 5))
point(8, (5, 5))
point(9, (50, 50))
line(10, (-5, 5), (15, 5))
line(11, (0, 0), (10, 0))
path(12, [(0, 0), (10, 0), (10, 10), (0, 10)])
line(13, (10, 0), (10, 10))
line(14, (2, -5), (2, 0))
OBJECTS[15] = ({**base("circle", 15), "c": P(60, 0), "r": 5.0}, None)
point(16, (65, 0))
point(17, (60, 0))
poly(18, sq(55, -5, 65, 5))
point(19, (5, 5), [(25, 5)])
line(20, (0, 0), (10, 0))
line(21, (12, 5), (18, 5))
OBJECTS[22] = ({**base("xline", 22, "yol"), "p": P(0, 0), "dir": P(1, 0)}, None)
path(23, [(0, 0), (10, 0), (10, 10), (0, 10), (0, 0)])

IDS = sorted(OBJECTS)
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Standart ilişkiler",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": P(0, 0),
    "layers": [layer("parsel", "Parsel"), layer("yol", "Yol"), layer("nokta", "Nokta", visible=False, locked=True)],
    "activeLayer": "parsel", "entities": [OBJECTS[i][0] for i in IDS], "styles": {"items": [], "categories": []},
}
SAME = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}

# The circle's pairs, from the definition: a closed disc of radius 5 about (60, 0); the square 18 holds it, touching it
# at (65, 0), (60, 5), (55, 0), (60, −5).
BY_HAND = {
    (15, 16): "FF20F1FF2",  # a point on the circle: in its boundary only
    (15, 17): "0F2FF1FF2",  # the centre: in its interior, the rest of the disc and its circle outside the point
    (15, 18): "2FF10F212",  # the disc in the square: boundaries meet at four points, the disc's boundary elsewhere inside
    (18, 15): "212F01FF2",
    (15, 1): "FF2FF1212",  # far apart
}


def matrix(a, b):
    if (a, b) in BY_HAND:
        return BY_HAND[(a, b)]
    wa, wb = OBJECTS[a][1], OBJECTS[b][1]
    assert wa and wb, (a, b)
    g, h = QgsGeometry.fromWkt(wa), QgsGeometry.fromWkt(wb)
    e = QgsGeometry.createGeometryEngine(g.constGet())
    e.prepareGeometry()
    return e.relate(h.constGet())


def dim(i):
    w = OBJECTS[i][1]
    if w is None:
        return 2  # the circle
    return {"POLYGON": 2, "LINESTRING": 1, "POINT": 0, "MULTIPOINT": 0}[w.split("(")[0]]


def matches(m, pattern):
    for d, c in zip(m, pattern.upper()):
        if c == "*":
            continue
        if c == "T" and d == "F":
            return False
        if c == "F" and d != "F":
            return False
        if c in "012" and d != c:
            return False
    return True


def holds(pred, m, da, db):
    t = lambda p: matches(m, p)  # noqa: E731
    return {
        "intersects": not t("FF*FF****"),
        "disjoint": t("FF*FF****"),
        "contains": t("T*****FF*"),
        "within": t("T*F**F***"),
        "covers": t("T*****FF*") or t("*T****FF*") or t("***T**FF*") or t("****T*FF*"),
        "coveredBy": t("T*F**F***") or t("*TF**F***") or t("**FT*F***") or t("**F*TF***"),
        "touches": t("FT*******") or t("F**T*****") or t("F***T****"),
        "crosses": (t("0********") if (da, db) == (1, 1) else t("T*T******") if da < db
                    else t("T*****T**") if da > db else False),
        "overlaps": (t("1*T***T**") if (da, db) == (1, 1) else t("T*T***T**") if da == db else False),
        "equals": t("T*F**FFF*"),
    }[pred]


def uid(i):
    return f"$uidOf:{i}"


def ask(a, b, predicate=None, pattern=None):
    inp = {"a": [uid(i) for i in a], "b": [uid(i) for i in b]}
    if predicate:
        inp["predicate"] = predicate
    if pattern:
        inp["pattern"] = pattern
    pairs, unread = [], []
    for i in a + b:
        if OBJECTS[i][0]["kind"] == "xline" and uid(i) not in unread:
            unread.append(uid(i))
    for i in a:
        if OBJECTS[i][0]["kind"] == "xline":
            continue
        for j in b:
            if OBJECTS[j][0]["kind"] == "xline":
                continue
            m = matrix(i, j)
            keep = (holds(predicate, m, dim(i), dim(j)) if predicate else matches(m, pattern) if pattern else True)
            if keep:
                pairs.append({"a": uid(i), "b": uid(j), "matrix": m})
    out = {"pairs": pairs, "revision": "$current"}
    if unread:
        out["unread"] = unread
    return inp, {"status": "completed", "output": out, "warnings": []}


def step(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result, "expect": SAME}


def case(name, a, b, predicate=None, pattern=None, op="execute"):
    inp, result = ask(a, b, predicate, pattern)
    return {"name": name, "steps": [step(inp, result, op)]}


def failed(name, inp, code, message, path):
    return {"name": name, "steps": [step(inp, {"status": "failed",
                                                "error": {"code": code, "message": message, "path": path}})]}


def cases():
    return [
        case("sınırdaki noktanın matrisi", [1], [7]),
        case("sınırdaki noktayı kapsar", [1], [7], "covers"),
        case("sınırdaki noktayı içermez", [1], [7], "contains"),
        case("içteki noktayı içerir", [1], [7, 8, 9, 19], "contains"),
        case("ortak kenarlı parseller değer", [1], [2, 3, 4], "touches"),
        case("örtüşen parseller", [1, 2], [4], "overlaps"),
        case("içteki parsel içinde kalır", [3, 4], [1], "within"),
        case("parseli kesen yol", [10, 11, 21], [1, 2], "crosses"),
        case("sınır boyunca çizgi kapsanır ama içinde kalmaz", [11, 12], [1], "coveredBy"),
        case("sınır boyunca çizgi içinde kalmaz", [11, 12], [1], "within"),
        case("ortak sınır çizgisi iki parsele de değer", [13], [1, 2], "touches"),
        case("ucu sınırda biten çizgi değer", [14], [1], "touches"),
        case("delikteki parsel ayrık", [6], [5], "disjoint"),
        case("delikli parsel deliktekiyle kesişmez", [5], [6], "intersects"),
        case("aynı çizgi eşit", [11], [20, 13], "equals"),
        case("kapalı yolun sınırı yok: çizgi kapsanır", [23], [1], "coveredBy"),
        case("kapalı yolla açık yol: matrisler", [23], [12]),
        case("çok noktalının biri dışarıda: içinde kalmaz, kesişir", [19], [1], "within"),
        case("çok noktalı kesişir", [19], [1], "intersects"),
        case("içerme deseni", [1], [3, 7, 8], pattern="T*****FF*"),
        case("küçük harfli desen", [1], [3, 7, 8], pattern="t*****ff*"),
        case("dairenin sınırındaki nokta", [15], [16, 17]),
        case("daire kareye teğet içinde", [15], [18], "within"),
        case("kare daireyi kapsar", [18], [15], "covers"),
        case("uzaktaki daire ayrık", [15], [1], "disjoint"),
        case("yardımcı çizgi okunmaz", [22, 1], [8]),
        case("gizli ve kilitli katmandaki nokta da okunur", [8], [1], "within"),
        case("plan aynı cevabı verir", [1], [2, 3, 4], "touches", op="plan"),
        {"name": "validate yalnız denetler", "steps": [step({"a": [uid(1)], "b": [uid(2)], "predicate": "touches"},
                                                             {"status": "completed", "output": None,
                                                              "warnings": []}, op="validate")]},
        failed("boş a", {"a": [], "b": [uid(1)]}, "no_objects",
               "a listesi boş: ilişkisi sorulacak nesnelerin kalıcı kimliklerini verin.", "a"),
        failed("hem yüklem hem desen", {"a": [uid(1)], "b": [uid(2)], "predicate": "touches", "pattern": "F********"},
               "invalid_input",
               "Hem yüklem hem desen verildi: birini verin (predicate ya da pattern) ya da hiçbirini (bütün çiftlerin "
               "matrisleri).", "pattern"),
        failed("kısa desen", {"a": [uid(1)], "b": [uid(2)], "pattern": "T*F"}, "invalid_pattern",
               "“T*F” bir DE-9IM deseni değil: dokuz harf, her biri T, F, *, 0, 1 ya da 2 olmalı (içerme T*****FF* "
               "gibi).", "pattern"),
        failed("bilinmeyen nesne", {"a": [uid(1), "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"], "b": [uid(2)]},
               "entity_not_found",
               "“0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
               "olabilir. Var olan bir nesnenin kimliğini verin.", "a[1]"),
    ]


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.entities.relate", "commandVersion": 1,
        "title": "Standart ilişkiler: DE-9IM matrisi ve yüklemler",
        "note": "ADR 0501. Düz kenarlı çiftlerin matrisleri GEOS'tan (QGIS 4.2'nin Python'u), yüklemler OGC SFA "
                "1.2.1'in desenlerinden; dairenin çiftleri tanımdan elle. Konuma göre seç'in 1 mm toleransı yok: "
                "sınırdaki nokta covers'ta tutar, contains'te tutmaz. Çizim değişmez. Bağımsız başvuru: "
                "scripts/fixtures/relate_cases.py.",
        "setup": SETUP, "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"standart ilişki durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
