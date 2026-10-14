"""The shared cases of the product command cad.points.construct (docs/adr/0270).

    python3 scripts/fixtures/point_construct_cases.py           # writes the file
    python3 scripts/fixtures/point_construct_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.points.construct.json. The checks, their order, codes, paths and messages are
written here by hand from the ADR; the points are computed from the constructions' definitions, without KentOS
code:

- İki nokta ortası: ((a.x + b.x) / 2, (a.y + b.y) / 2);
- Uzantı: past the open end along the one edge ending there: a straight edge's direction from its other end, an
  arc's circle the same way round, the angle turned by distance / radius;
- Paralel: from the point along the object's straight edge nearest the near point, the edge's way from its first
  point to its second, the distance signed; an ellipse's, a spline's and a NURBS curve's chords are no edges;
- İzleme: the UCS x of the first point and the UCS y of the second (the drawing has no UCS: the world's);
- Kutupsal: from the point, distance along (cos angle, sin angle);
- Hat üzerinde: a + (b − a) · distance / |b − a|.

The drawing is chosen so that every value is a double exactly (whole and half metres; the arc's turns are right
angles, whose cosines and sines round away beside the drawing's large coordinates): the script asserts it.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.points.construct.json"

E0, N0 = 487000, 4420000
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def P(x, y):
    return {"x": x, "y": y}


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": []}


ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "attrs": {}, "a": P(E0, N0), "b": P(E0 + 10, N0)},
    {"kind": "polyline", "id": 2, "layerId": "yapi", "attrs": {},
     "pts": [P(E0, N0 + 20), P(E0 + 10, N0 + 20), P(E0 + 10, N0 + 30)]},
    {"kind": "polygon", "id": 3, "layerId": "yapi", "attrs": {},
     "pts": [P(E0 + 20, N0), P(E0 + 30, N0), P(E0 + 30, N0 + 10), P(E0 + 20, N0 + 10)]},
    {"kind": "arc", "id": 4, "layerId": "yapi", "attrs": {}, "c": P(E0 + 50, N0), "r": 5, "a0": 0,
     "a1": math.pi / 2},
    {"kind": "circle", "id": 5, "layerId": "yapi", "attrs": {}, "c": P(E0 + 70, N0), "r": 3},
    {"kind": "point", "id": 6, "layerId": "yapi", "attrs": {}, "p": P(E0 + 80, N0 + 10)},
    {"kind": "ellipse", "id": 7, "layerId": "yapi", "attrs": {}, "c": P(E0 + 100, N0), "major": P(8, 0), "ratio": 0.5,
     "t0": 0, "t1": 2 * math.pi},
]
IDS = [e["id"] for e in ENTITIES]

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Nokta yapımı",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": P(E0, N0), "layers": [layer("yapi", "Yapı")], "activeLayer": "yapi", "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

SAME = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"


def at(x, y):
    return {"kind": "at", "at": P(x, y)}


def place(slot, p, path=None):
    out = {"kind": "place", "uid": f"$uidOf:{slot}", "place": p}
    if path is not None:
        out["path"] = path
    return out


def end(slot, p, path=None):
    out = {"uid": f"$uidOf:{slot}", "place": p}
    if path is not None:
        out["path"] = path
    return out


START, END, CENTER = {"kind": "start"}, {"kind": "end"}, {"kind": "center"}


def vertex(i, ring=0, part=0):
    return {"kind": "vertex", "part": part, "ring": ring, "index": i}


NODE = {"kind": "node", "part": 0}


def exact(x):
    """A value the script computed, as a double that must be the exact value."""
    f = float(x)
    assert F(f) == F(x) or isinstance(x, float), x
    return f


def done(points):
    return {"status": "completed", "output": {"points": [P(exact(x), exact(y)) for x, y in points]},
            "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(constructions, result, op="execute", **more):
    s = {"op": op, "input": {"constructions": constructions}, "result": result, "expect": SAME}
    s.update(more)
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def arc_point(c, r, t):
    """A point of the circle at angle t: the cosine and sine of a right angle round away beside the coordinates."""
    x, y = c[0] + r * math.cos(t), c[1] + r * math.sin(t)
    assert x == round(x) and y == round(y), (x, y)
    return (x, y)


NOT_AN_END = ("Yer nesnenin açık bir ucu değil: uzantı çizginin, açık çoklu çizginin ya da yayın ucundan başlar "
              "(kapalı yolun ya da yolun içindeki köşe olmaz).")
NO_PIECE = ("Blok yolunda böyle bir parça yok: yolun bir sırası tanımın dışında, yol bir yerleştirmede bitiyor ya da "
            "yerleştirme olmayan bir nesneden geçiyor.")


def cases():
    out = []
    # İki nokta ortası.
    out.append(case("iki koordinatın ortası", step([{"kind": "mid", "a": at(E0, N0), "b": at(E0 + 7, N0 + 3)}],
                                                   done([(F(2 * E0 + 7, 2), F(2 * N0 + 3, 2))]))))
    out.append(case("çizginin başlangıcı ile alanın köşesinin ortası, yerleriyle",
                    step([{"kind": "mid", "a": place(1, START), "b": place(3, vertex(2))}],
                         done([(E0 + 15, N0 + 5)]))))
    out.append(case("noktanın düğümü ile çoklu çizginin köşesinin ortası",
                    step([{"kind": "mid", "a": place(6, NODE), "b": place(2, vertex(1))}],
                         done([(E0 + 45, N0 + 15)]))))
    # Uzantı.
    out.append(case("çizginin ucundan 2,5 m uzantı", step([{"kind": "extension", "end": end(1, END), "distance": 2.5}],
                                                         done([(F(2 * E0 + 25, 2), N0)]))))
    out.append(case("çizginin başlangıcından 4 m geriye uzantı",
                    step([{"kind": "extension", "end": end(1, START), "distance": 4}], done([(E0 - 4, N0)]))))
    out.append(case("açık çoklu çizginin son köşesinden 1,25 m kuzeye",
                    step([{"kind": "extension", "end": end(2, vertex(2)), "distance": 1.25}],
                         done([(E0 + 10, F(4 * (N0 + 30) + 5, 4))]))))
    out.append(case("açık çoklu çizginin ilk köşesinden sıfır uzantı köşenin kendisidir",
                    step([{"kind": "extension", "end": end(2, vertex(0)), "distance": 0}], done([(E0, N0 + 20)]))))
    c4, r4 = (E0 + 50, N0), 5
    d = r4 * (math.pi / 2)
    assert d / r4 == math.pi / 2
    out.append(case("yayın ucundan çember boyunca çeyrek tur: batıdaki nokta",
                    step([{"kind": "extension", "end": end(4, END), "distance": d}],
                         done([arc_point(c4, r4, math.pi / 2 + d / r4)]))))
    out.append(case("yayın başlangıcından çember boyunca geriye çeyrek tur: güneydeki nokta",
                    step([{"kind": "extension", "end": end(4, START), "distance": d}],
                         done([arc_point(c4, r4, 0 - d / r4)]))))
    out.append(case("kapalı alanın köşesi uç değildir: not_an_end",
                    step([{"kind": "extension", "end": end(3, vertex(1)), "distance": 1}],
                         failed("not_an_end", NOT_AN_END, "constructions[0].end.place"))))
    out.append(case("çoklu çizginin içteki köşesi uç değildir",
                    step([{"kind": "extension", "end": end(2, vertex(1)), "distance": 1}],
                         failed("not_an_end", NOT_AN_END, "constructions[0].end.place"))))
    out.append(case("dairenin merkezi uç değildir",
                    step([{"kind": "extension", "end": end(5, CENTER), "distance": 1}],
                         failed("not_an_end", NOT_AN_END, "constructions[0].end.place"))))
    out.append(case("uzantının uzaklığı eksi olamaz: invalid_distance",
                    step([{"kind": "extension", "end": end(1, END), "distance": -1}],
                         failed("invalid_distance", "Uzantının uzaklığı sıfır ya da daha büyük olmalı: uç geride kalmaz.",
                                "constructions[0].distance"))))
    # Paralel.
    out.append(case("noktadan çizgiye paralel 3 m: çizginin doğrultusunda doğuya",
                    step([{"kind": "parallel", "through": at(E0 + 5, N0 + 50),
                           "edge": {"uid": "$uidOf:1", "near": P(E0 + 4, N0 + 1)}, "distance": 3}],
                         done([(E0 + 8, N0 + 50)]))))
    out.append(case("çoklu çizginin ikinci kenarına paralel eksi 2 m: kenarın tersine, güneye",
                    step([{"kind": "parallel", "through": place(6, NODE),
                           "edge": {"uid": "$uidOf:2", "near": P(E0 + 11, N0 + 26)}, "distance": -2}],
                         done([(E0 + 80, N0 + 8)]))))
    out.append(case("dairenin düz kenarı yok: no_straight_edge",
                    step([{"kind": "parallel", "through": at(E0, N0),
                           "edge": {"uid": "$uidOf:5", "near": P(E0 + 73, N0)}, "distance": 1}],
                         failed("no_straight_edge",
                                "Nesnenin düz kenarı yok: paralelin doğrultusu düz bir kenardan alınır; çizgi, çoklu çizgi ya da alan verin.",
                                "constructions[0].edge"))))
    out.append(case("elipsin kirişleri düz kenar sayılmaz: no_straight_edge",
                    step([{"kind": "parallel", "through": at(E0, N0),
                           "edge": {"uid": "$uidOf:7", "near": P(E0 + 108, N0)}, "distance": 1}],
                         failed("no_straight_edge",
                                "Nesnenin düz kenarı yok: paralelin doğrultusu düz bir kenardan alınır; çizgi, çoklu çizgi ya da alan verin.",
                                "constructions[0].edge"))))
    # İzleme, kutupsal, hat üzerinde.
    out.append(case("izleme: çizginin ucunun X'i, noktanın Y'si",
                    step([{"kind": "track", "x": place(1, END), "y": place(6, NODE)}], done([(E0 + 10, N0 + 10)]))))
    out.append(case("kutupsal: doğuya 10 m ve kuzeye 6 m",
                    step([{"kind": "polar", "from": at(E0, N0), "angle": 0, "distance": 10},
                          {"kind": "polar", "from": place(3, vertex(0)), "angle": math.pi / 2, "distance": 6}],
                         done([(E0 + 10, N0), arc_point((E0 + 20, N0), 6, math.pi / 2)]))))
    out.append(case("hat üzerinde: 6–8–10 üçgeninin hipotenüsünde 5 m",
                    step([{"kind": "along", "a": at(E0, N0), "b": at(E0 + 6, N0 + 8), "distance": 5}],
                         done([(E0 + 3, N0 + 4)]))))
    out.append(case("hat üzerinde eksi uzaklık geriye gider",
                    step([{"kind": "along", "a": place(1, START), "b": place(1, END), "distance": -2.5}],
                         done([(F(2 * E0 - 5, 2), N0)]))))
    out.append(case("hattın iki noktası aynı: same_points",
                    step([{"kind": "along", "a": at(E0, N0), "b": place(1, START), "distance": 1}],
                         failed("same_points",
                                "Hattın iki noktası aynı yerde; aralarında bir doğrultu yok. Ayrı iki nokta verin.",
                                "constructions[0].b"))))
    # Several, the plan, the checks.
    out.append(case("birkaç yapım sırasıyla; plan aynı cevabı verir, validate yalnız denetler",
                    step([{"kind": "mid", "a": at(E0, N0), "b": at(E0 + 2, N0)},
                          {"kind": "extension", "end": end(1, END), "distance": 1}],
                         done([(E0 + 1, N0), (E0 + 11, N0)]), op="plan"),
                    step([{"kind": "mid", "a": at(E0, N0), "b": at(E0 + 2, N0)}],
                         {"status": "completed", "output": None, "warnings": []}, op="validate")))
    out.append(case("ikinci yapım reddedilince hiçbir nokta verilmez; yer ikinci yapımındır",
                    step([{"kind": "mid", "a": at(E0, N0), "b": at(E0 + 2, N0)},
                          {"kind": "extension", "end": end(3, vertex(0)), "distance": 1}],
                         failed("not_an_end", NOT_AN_END, "constructions[1].end.place"))))
    out.append(case("hiç yapım yok: no_constructions",
                    step([], failed("no_constructions", "0 yapım verildi: bir ile 10 000 arasında yapım verin.",
                                    "constructions"))))
    out.append(case("kimliğin yazımı: invalid_uid",
                    step([{"kind": "track", "x": {"kind": "place", "uid": "ABC", "place": END}, "y": at(E0, N0)}],
                         failed("invalid_uid",
                                "“ABC” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).",
                                "constructions[0].x.uid"))))
    out.append(case("çizimde olmayan nesne: entity_not_found",
                    step([{"kind": "mid", "a": {"kind": "place", "uid": MISSING, "place": END}, "b": at(E0, N0)}],
                         failed("entity_not_found",
                                f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                                "constructions[0].a.uid"))))
    out.append(case("nesnede olmayan yer: no_such_place",
                    step([{"kind": "mid", "a": place(1, CENTER), "b": at(E0, N0)}],
                         failed("no_such_place",
                                "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor.",
                                "constructions[0].a.place"))))
    out.append(case("yerleştirme olmayan nesnede blok yolu: piece_gone",
                    step([{"kind": "extension", "end": end(1, END, [0]), "distance": 1}],
                         failed("piece_gone", NO_PIECE, "constructions[0].end.path"))))
    out.append(case("sayılar sonlu olmalı: önce noktanın doğusu, sonra kutupsalın açısı",
                    step([{"kind": "mid", "a": at(E0, N0), "b": at(E0 + 1, N0)}],
                         failed("not_finite",
                                "Noktanın doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                                "constructions[0].a.at.x"), nonFinite={"constructions[0].a.at.x": "Infinity"}),
                    step([{"kind": "polar", "from": at(E0, N0), "angle": 0, "distance": 1}],
                         failed("not_finite", "Açı sonlu bir sayı değil (NaN ya da sonsuz). Radyan olarak bir sayı verin.",
                                "constructions[0].angle"), nonFinite={"constructions[0].angle": "NaN"})))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.points.construct", "commandVersion": 1,
        "title": "Nokta yapımı: iki nokta ortası, uzantı, paralel, izleme, kutupsal, hat üzerinde",
        "note": "ADR 0270. Yapım yapım sırayla: sayılar sonlu, uzantının uzaklığı sıfır ya da daha büyük, noktaların "
                "ve kenarın nesneleri, uzantının yeri açık uç, paralelin nesnesinde düz kenar, hattın noktaları ayrı. "
                "Çizim değişmez. Bağımsız başvuru: scripts/fixtures/point_construct_cases.py.",
        "setup": SETUP, "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"nokta yapımı durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
