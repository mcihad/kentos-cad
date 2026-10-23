"""The shared cases of cad.areas.edit (docs/adr/0622 §9): Alan birleştir, kesiştir, çıkar, böl; Alana and Çizgiye
çevir; Parçaları birleştir and Parçalara ayır; Delik ekle, Deliği sil, Deliği doldur.

    python3 scripts/fixtures/areas_edit_cases.py           # writes the file
    python3 scripts/fixtures/areas_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.areas.edit.json. Every area here has straight edges along east and north on whole
metres, so the reference works on the unit cells of the plane, without KentOS code and without a single rounding
(the machinery of area_rules_cases.py):

- an area is the set of cells whose centres are inside it; a union is the union of the cells, an overlap their
  intersection, a cut their difference, a hole a ring's cells taken out;
- a result's corners are the cell corners where one or three of the four cells round it are in, or two across, and
  the corners of the input rings lying on its boundary (the overlay keeps input corners where they fall);
- a result's pieces are the 4-connected groups of its cells; a piece is written as an area of its own, or with Tek
  nesne (`single`) all of them as one multi-part area;
- the objects keep their persistent ids as the ADR says: Birleştir and Kesiştir (`erase`) take the first object's
  data into a new object, Çıkar writes the pieces left as new objects with their source's data, Böl keeps the area
  as its first piece, Alana çevir and Çizgiye çevir keep the object in place, Parçaları birleştir the first, Parçalara
  ayır the object as its first part.

The runner compares an area's shape whatever corner its rings start from (`shapes`). The messages, codes and paths
are the ADR's.
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
sys.path.insert(0, str(Path(__file__).resolve().parent))
import area_rules_cases as cellwork  # noqa: E402  (cells, corners, groups, holes: no KentOS code)

POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000
FILE = OUT / "cad.areas.edit.json"


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def W(p):
    return {"x": X + p[0], "y": Y + p[1]}


def polygon(id_, layer_id, ring, attrs=None, holes=(), parts=(), label=None):
    e = {"kind": "polygon", "id": id_, "layerId": layer_id, "attrs": attrs or {}, "pts": [W(p) for p in ring]}
    if holes:
        e["holes"] = [{"pts": [W(p) for p in h]} for h in holes]
    if parts:
        e["parts"] = [{"pts": [W(p) for p in q]} for q in parts]
    if label:
        e["label"] = label
    return e


def line(id_, a, b, layer_id="yapi"):
    return {"kind": "line", "id": id_, "layerId": layer_id, "attrs": {}, "a": W(a), "b": W(b)}


def point(id_, p, layer_id="yapi", parts=()):
    e = {"kind": "point", "id": id_, "layerId": layer_id, "attrs": {}, "p": W(p)}
    if parts:
        e["parts"] = [{"p": W(q)} for q in parts]
    return e


def polyline(id_, pts, layer_id="yapi", parts=()):
    e = {"kind": "polyline", "id": id_, "layerId": layer_id, "attrs": {}, "pts": [W(p) for p in pts]}
    if parts:
        e["parts"] = [{"pts": [W(p) for p in q]} for q in parts]
    return e


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────

A, B, C, D = rect(0, 0, 10, 10), rect(5, 0, 15, 10), rect(30, 0, 40, 10), rect(2, 2, 4, 4)
K = rect(0, 20, 10, 30)
E, E_HOLE = rect(50, 0, 60, 10), rect(53, 3, 57, 7)
M1, M2 = rect(70, 0, 75, 5), rect(80, 0, 85, 5)
SQUARE = [(100, 0), (110, 0), (110, 10), (100, 10)]
ENTITIES = [
    polygon(1, "yapi", A, {"Ada": "101", "Parsel": "5"}, label="5"),
    polygon(2, "yapi", B, {"Ada": "101", "Parsel": "6"}),
    polygon(3, "yapi", C, {"Ada": "102", "Parsel": "1"}),
    polygon(4, "yapi", D),
    polygon(5, "kilitli", K),
    line(6, (35, -5), (35, 15)),
    polygon(7, "yapi", E, {"Ada": "103", "Parsel": "2"}, holes=[E_HOLE]),
    polygon(8, "yapi", M1, {"Ada": "104"}, parts=[M2]),
    line(9, (90, 0), (95, 0)),
    line(10, (90, 5), (95, 5)),
] + [line(11 + i, SQUARE[i], SQUARE[(i + 1) % 4]) for i in range(4)] + [
    point(15, (120, 0)),
    point(16, (125, 0)),
    polyline(17, [(130, 0), (135, 0)], parts=[[(130, 5), (135, 5)]]),
    point(18, (140, 0), parts=[(145, 0), (150, 0)]),
]
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k != "entities"}, "entities": ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1
RINGS = {e["id"]: [(p["x"] - X, p["y"] - Y) for p in e["pts"]] for e in ENTITIES if e["kind"] == "polygon"}
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}
AREA_KINDS = "kapalı alan, daire, elips ya da kapalı eğri"


def U(i):
    return f"$uidOf:{i}"


def cells(ring):
    return cellwork.cells(ring)


def corners_on(cs, rings):
    """The cell corners and the given rings' corners lying on the cells' boundary."""
    own = cellwork.corners(cs)
    return own | {c for r in rings for c in r if cellwork.on_boundary(cs, c, own)}


def shape(cs, rings=(), parts=None):
    cn = sorted(corners_on(cs, rings))
    n = len(cellwork.groups(cs)) if parts is None else parts
    return {"corners": [[X + x, Y + y] for x, y in cn], "area": len(cs), "parts": n, "holes": len(cellwork.holes(cs))}


def edit(work):
    return {"work": work}


def done(name, work, ids, shapes, output, fields=None, undo=None):
    step = {"op": "execute", "input": edit(work),
            "result": {"status": "completed", "output": {"revision": "$current", **output}, "warnings": []},
            "expect": {"ids": ids, "shapes": shapes, "canUndo": True, "dirty": True}}
    if fields:
        step["expect"]["fields"] = fields
    back = {"op": "undo", "returns": undo, "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def refused(name, work, code, message, path):
    return {"name": name, "steps": [{"op": "execute", "input": edit(work),
                                     "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                                     "expect": UNCHANGED}]}


def gone(*ids):
    return [i for i in IDS if i not in ids]


def out(changed=(), created=(), removed="$any", result=(), objects=0, pieces=0, size=0, more=0, sizes=()):
    return {"changed": [U(i) for i in changed], "created": [U(i) for i in created],
            "removed": removed if removed == "$any" else [U(i) for i in removed],
            "result": [U(i) for i in result], "objects": objects, "pieces": pieces, "size": size, "more": more,
            "sizes": list(sizes)}


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────


def union_cases():
    ab = cells(A) | cells(B)
    ac = cells(A) | cells(C)
    return [
        done("Birleştir: iki örtüşen alan tek alan, ilkinin verisiyle; adım “Alan birleştir”",
             {"kind": "union", "ids": [U(1), U(2)]}, gone(1, 2) + [NEXT],
             {str(NEXT): shape(ab, [A, B])},
             out(created=[NEXT], result=[NEXT], objects=2, pieces=1, size=len(ab)),
             fields={str(NEXT): {"attrs": {"Ada": "101", "Parsel": "5"}, "label": "5"}}, undo="Alan birleştir"),
        done("Birleştir, Tek nesne: değmeyen iki alan iki parçalı tek alan",
             {"kind": "union", "ids": [U(1), U(3)], "single": True}, gone(1, 3) + [NEXT],
             {str(NEXT): shape(ac, [A, C])},
             out(created=[NEXT], result=[NEXT], objects=2, pieces=2, size=len(ac)), undo="Alan birleştir"),
        done("Birleştir: aynı kimlik iki kez bir sayılır",
             {"kind": "union", "ids": [U(1), U(2), U(1)]}, gone(1, 2) + [NEXT],
             {str(NEXT): shape(ab, [A, B])},
             out(created=[NEXT], result=[NEXT], objects=2, pieces=1, size=len(ab)), undo="Alan birleştir"),
        refused("Birleştir: tek alan yetmez", {"kind": "union", "ids": [U(1)]}, "too_few_objects",
                f"Birleştirmek için en az iki alan seçin ({AREA_KINDS}).", "work.ids"),
        refused("Birleştir: çizgi alan sayılmaz", {"kind": "union", "ids": [U(1), U(6)]}, "too_few_objects",
                f"Birleştirmek için en az iki alan seçin ({AREA_KINDS}).", "work.ids"),
        refused("Birleştir: kilitli katmandaki alan", {"kind": "union", "ids": [U(1), U(5)]}, "layer_locked",
                "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.",
                "work"),
        refused("Olmayan kimlik", {"kind": "union", "ids": [U(1), "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]},
                "entity_not_found",
                "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
                "olabilir. Var olan bir nesnenin kimliğini verin.", "work.ids[1]"),
        refused("Kimlik değil", {"kind": "union", "ids": ["parsel-5"]}, "invalid_uid",
                "“parsel-5” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "work.ids[0]"),
    ]


def intersect_cases():
    both = cells(A) & cells(B)
    return [
        done("Kesiştir: ortak parça yeni, boş alan; kaynaklar kalır",
             {"kind": "intersect", "ids": [U(1), U(2)]}, IDS + [NEXT],
             {str(NEXT): shape(both, [A, B])},
             out(created=[NEXT], removed=[], result=[NEXT], objects=2, pieces=1, size=len(both)),
             fields={str(NEXT): {"attrs": {}}}, undo="Alan kesiştir"),
        done("Kesiştir, Kaynakları sil: ortak parça ilkinin verisini alır",
             {"kind": "intersect", "ids": [U(1), U(2)], "erase": True}, gone(1, 2) + [NEXT],
             {str(NEXT): shape(both, [A, B])},
             out(created=[NEXT], result=[NEXT], objects=2, pieces=1, size=len(both)),
             fields={str(NEXT): {"attrs": {"Ada": "101", "Parsel": "5"}}}, undo="Alan kesiştir"),
        refused("Kesiştir: ortak parça yok", {"kind": "intersect", "ids": [U(1), U(3)]}, "no_overlap",
                "Seçili alanların ortak bir parçası yok.", "work.ids"),
        refused("Kesiştir: tek alan yetmez", {"kind": "intersect", "ids": [U(2)]}, "too_few_objects",
                f"Kesiştirmek için en az iki alan seçin ({AREA_KINDS}).", "work.ids"),
    ]


def subtract_cases():
    left = cells(A) - cells(B)
    holed = cells(A) - cells(D)
    return [
        done("Çıkar: kalan parça kaynağın verisiyle yeni alan",
             {"kind": "subtract", "from": [U(1)], "cut": [U(2)]}, gone(1) + [NEXT],
             {str(NEXT): shape(left, [A, B])},
             out(created=[NEXT], result=[NEXT], objects=1, pieces=1, size=len(left)),
             fields={str(NEXT): {"attrs": {"Ada": "101", "Parsel": "5"}}}, undo="Alan çıkar"),
        done("Çıkar: içteki alan delik bırakır",
             {"kind": "subtract", "from": [U(1)], "cut": [U(4)]}, gone(1) + [NEXT],
             {str(NEXT): shape(holed, [A, D])},
             out(created=[NEXT], result=[NEXT], objects=1, pieces=1, size=len(holed)), undo="Alan çıkar"),
        done("Çıkar, Çıkarılanları sil: kesen alan da gider",
             {"kind": "subtract", "from": [U(1)], "cut": [U(2)], "erase": True}, gone(1, 2) + [NEXT],
             {str(NEXT): shape(left, [A, B])},
             out(created=[NEXT], result=[NEXT], objects=1, pieces=1, size=len(left)), undo="Alan çıkar"),
        done("Çıkar: iki kesen birlikte",
             {"kind": "subtract", "from": [U(2)], "cut": [U(1), U(3)]}, gone(2) + [NEXT],
             {str(NEXT): shape(cells(B) - cells(A), [A, B])},
             out(created=[NEXT], result=[NEXT], objects=1, pieces=1, size=len(cells(B) - cells(A))),
             undo="Alan çıkar"),
        done("Çıkar, Çıkarılanları sil: kilitli katmandaki kesen kalır",
             {"kind": "subtract", "from": [U(2)], "cut": [U(1), U(5)], "erase": True}, gone(1, 2) + [NEXT],
             {str(NEXT): shape(cells(B) - cells(A), [A, B])},
             out(created=[NEXT], result=[NEXT], objects=1, pieces=1, size=len(cells(B) - cells(A))),
             undo="Alan çıkar"),
        refused("Çıkar: örtüşmeyen alan hiçbir şeyi değiştirmez", {"kind": "subtract", "from": [U(1)], "cut": [U(3)]},
                "no_overlap", "Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.", "work.cut"),
        refused("Çıkar: kesilecek alan yok", {"kind": "subtract", "from": [U(6)], "cut": [U(2)]}, "too_few_objects",
                f"Önce kesilecek alanları seçin ({AREA_KINDS}).", "work.from"),
        refused("Çıkar: çıkarılacak alan kendisi", {"kind": "subtract", "from": [U(1)], "cut": [U(1)]},
                "too_few_objects", f"Çıkarılacak en az bir alan seçin ({AREA_KINDS}).", "work.cut"),
    ]


def hole_cases():
    filled = cells(E) - cells(E_HOLE)
    return [
        done("Delik ekle: halka alanın içinde",
             {"kind": "holeAdd", "id": U(1), "ring": [W(p) for p in rect(3, 3, 7, 7)]}, IDS,
             {"1": shape(cells(A) - cells(rect(3, 3, 7, 7)), [A, rect(3, 3, 7, 7)])},
             out(changed=[1], removed=[], result=[1], objects=1, pieces=1, size=100 - 16), undo="Delik ekle"),
        done("Delik ekle: deliğe değen halka onunla birleşir",
             {"kind": "holeAdd", "id": U(7), "ring": [W(p) for p in rect(55, 3, 58, 7)]}, IDS,
             {"7": shape(filled - cells(rect(55, 3, 58, 7)), [E, E_HOLE, rect(55, 3, 58, 7)])},
             out(changed=[7], removed=[], result=[7], objects=1, pieces=1,
                 size=len(filled - cells(rect(55, 3, 58, 7))), more=1), undo="Delik ekle"),
        refused("Delik ekle: sınırı aşan halka", {"kind": "holeAdd", "id": U(1), "ring": [W(p) for p in rect(5, 5, 12, 8)]},
                "invalid_hole", "Delik alanın içinde kalmalı; sınırı aşan bölümü çıkarmak için Alan çıkar'ı kullanın.",
                "work.ring"),
        refused("Delik ekle: çizgiye delik eklenmez", {"kind": "holeAdd", "id": U(6), "ring": [W(p) for p in rect(1, 1, 2, 2)]},
                "not_an_area", "Delik yalnız kapalı alana eklenir.", "work.id"),
        done("Deliği sil: alan doluya döner",
             {"kind": "holeRemove", "id": U(7), "at": W((55, 5))}, IDS, {"7": shape(cells(E), [E])},
             out(changed=[7], removed=[], result=[7], objects=1, pieces=1, size=100), undo="Deliği sil"),
        done("Deliği doldur: deliğin halkası yeni alan, delik kalır",
             {"kind": "holeFill", "id": U(7), "at": W((55, 5))}, IDS + [NEXT],
             {str(NEXT): shape(cells(E_HOLE), [E_HOLE]), "7": shape(filled, [E, E_HOLE])},
             out(created=[NEXT], removed=[], result=[NEXT], objects=1, pieces=1, size=16), undo="Deliği doldur"),
        refused("Deliği sil: nokta delikte değil", {"kind": "holeRemove", "id": U(7), "at": W((51, 1))}, "no_hole",
                "Bir deliğin içine tıklayın.", "work.at"),
    ]


def parts_cases():
    m = cells(M1) | cells(M2)
    am = cells(A) | cells(C)
    return [
        done("Parçaları birleştir: değmeyen iki alan iki parçalı tek alan, ilkinin yerinde",
             {"kind": "partsJoin", "ids": [U(1), U(3)]}, gone(3), {"1": shape(am, [A, C])},
             out(changed=[1], result=[1], objects=2, pieces=2, size=len(am)), undo="Parçaları birleştir"),
        done("Parçaları birleştir: örtüşenler tek parça",
             {"kind": "partsJoin", "ids": [U(1), U(2)]}, gone(2), {"1": shape(cells(A) | cells(B), [A, B])},
             out(changed=[1], result=[1], objects=2, pieces=1, size=len(cells(A) | cells(B)), more=1),
             undo="Parçaları birleştir"),
        refused("Parçaları birleştir: türler karışık", {"kind": "partsJoin", "ids": [U(1), U(6)]}, "mixed_kinds",
                "Parçaları birleştir aynı türden nesneleri birleştirir: alanları, çizgileri ya da noktaları.", "work.ids"),
        refused("Parçaları birleştir: tek nokta", {"kind": "partsJoin", "ids": [U(15)]}, "too_few_objects",
                "Parçaları birleştirmek için en az iki nokta seçin.", "work.ids"),
        done("Parçaları birleştir: iki nokta tek nesne",
             {"kind": "partsJoin", "ids": [U(15), U(16)]}, gone(16), {},
             out(changed=[15], result=[15], objects=2, pieces=2),
             fields={"15": {"p": W((120, 0)), "parts": [{"p": W((125, 0))}]}}, undo="Parçaları birleştir"),
        done("Parçaları birleştir: iki çizgi çok parçalı çoklu çizgi",
             {"kind": "partsJoin", "ids": [U(9), U(10)]}, gone(10), {},
             out(changed=[9], result=[9], objects=2, pieces=2, size=10),
             fields={"9": {"kind": "polyline", "pts": [W((90, 0)), W((95, 0))], "parts": [{"pts": [W((90, 5)), W((95, 5))]}]}},
             undo="Parçaları birleştir"),
        done("Parçalara ayır: çok parçalı alan, ilk parça yerinde",
             {"kind": "partsSplit", "ids": [U(8)]}, IDS + [NEXT],
             {"8": shape(cells(M1), [M1]), str(NEXT): shape(cells(M2), [M2])},
             out(changed=[8], created=[NEXT], removed=[], result=[8, NEXT], objects=1, pieces=2, more=1),
             fields={str(NEXT): {"attrs": {"Ada": "104"}}}, undo="Parçalara ayır"),
        done("Parçalara ayır: alan önce, sonra çizgi ve nokta",
             {"kind": "partsSplit", "ids": [U(18), U(17), U(8)]}, IDS + [NEXT, NEXT + 1, NEXT + 2, NEXT + 3],
             {"8": shape(cells(M1), [M1]), str(NEXT): shape(cells(M2), [M2])},
             out(changed=[8, 18, 17], created=[NEXT, NEXT + 1, NEXT + 2, NEXT + 3], removed=[],
                 result=[8, NEXT, 18, NEXT + 1, NEXT + 2, 17, NEXT + 3], objects=3, pieces=7),
             fields={"18": {"p": W((140, 0))}, str(NEXT + 1): {"p": W((145, 0))}, str(NEXT + 2): {"p": W((150, 0))},
                     str(NEXT + 3): {"pts": [W((130, 5)), W((135, 5))]}}, undo="Parçalara ayır"),
        refused("Parçalara ayır: tek parçalı nesne", {"kind": "partsSplit", "ids": [U(1), U(6)]}, "not_multi_part",
                "Parçalarına ayrılacak çok parçalı bir nesne seçin: alan, çoklu çizgi ya da çok noktalı nesne.",
                "work.ids"),
    ]


def convert_cases():
    sq = cells(SQUARE)
    return [
        done("Alana çevir: kapalı bölge oluşturan dört çizgi yeni alan, çizgiler kalır",
             {"kind": "toArea", "ids": [U(11), U(12), U(13), U(14)]}, IDS + [NEXT],
             {str(NEXT): shape(sq, [SQUARE])},
             out(created=[NEXT], removed=[], result=[NEXT], pieces=1, size=len(sq), more=1, sizes=[len(sq)]),
             fields={str(NEXT): {"attrs": {}}}, undo="Alana çevir"),
        refused("Alana çevir: zaten alan", {"kind": "toArea", "ids": [U(1)]}, "not_an_area",
                "Seçili nesneler zaten alan.", "work.ids"),
        refused("Alana çevir: kapanmayan çizgiler", {"kind": "toArea", "ids": [U(9), U(10)]}, "not_an_area",
                "Alana çevrilecek kapalı nesne ya da kapalı bölge oluşturan çizgi bulunamadı. Çizgilerin uçları "
                "birleşmeli ya da kesişmeli.", "work.ids"),
        done("Çizgiye çevir: delikli alanın dışı yerinde, deliği yeni çoklu çizgi",
             {"kind": "toPolyline", "ids": [U(7)]}, IDS + [NEXT], {},
             out(changed=[7], created=[NEXT], removed=[], result=[7, NEXT], objects=1, pieces=2),
             fields={"7": {"kind": "polyline", "attrs": {"Ada": "103", "Parsel": "2"}},
                     str(NEXT): {"kind": "polyline", "attrs": {}}}, undo="Çizgiye çevir"),
        refused("Çizgiye çevir: alan değil", {"kind": "toPolyline", "ids": [U(6)]}, "not_an_area",
                "Çizgiye çevrilecek bir kapalı alan seçin.", "work.ids"),
    ]


def split_cases():
    return [
        refused("Böl: çizgi alanı geçmiyor", {"kind": "split", "ids": [U(1)], "line": [W((20, -5)), W((20, 15))]},
                "no_overlap", "Kesme çizgisi alanı baştan başa geçmiyor; çizgi alanın sınırını iki yerden kesmeli.",
                "work.line"),
        refused("Böl: tek nokta çizgi değil", {"kind": "split", "ids": [U(1)], "line": [W((5, -5)), W((5, -5))]},
                "degenerate_shape", "Kesme çizgisi için en az iki ayrı nokta gösterin.", "work.line"),
        refused("Böl: çizgi de kesici de verilmedi", {"kind": "split", "ids": [U(1)]}, "invalid_input",
                "Kesme çizgisini noktalarıyla (line) ya da kesici nesneyle (cutter) verin; ikisi birden değil.",
                "work.line"),
        {"name": "Böl: kesme çizgisinin noktası sonlu değil", "steps": [{
            "op": "execute", "input": edit({"kind": "split", "ids": [U(1)], "line": [W((4, -5)), W((4, 15))]}),
            "nonFinite": {"work.line[0].x": "NaN"},
            "result": {"status": "failed", "error": {
                "code": "not_finite",
                "message": "Kesme çizgisinin noktasının doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). "
                           "Koordinatı sonlu bir sayıyla verin.",
                "path": "work.line[0].x"}},
            "expect": UNCHANGED}]},
        refused("Böl: alan değil", {"kind": "split", "ids": [U(6)], "line": [W((5, -5)), W((5, 15))]}, "not_an_area",
                f"Bölünecek bir alan seçin ({AREA_KINDS}).", "work.ids"),
    ]


def split_written():
    """Alan böl's pieces: the area itself keeps the piece the overlay gives first (by the ADR: the one of the
    smallest x of its lowest-left corner), the others are new with its data."""
    out_ = []
    for name, ids, work, ring, cut_x in (
        ("Böl: noktalarla kesilen alan iki parça, ilki yerinde",
         [1], {"kind": "split", "ids": [U(1)], "line": [W((4, -5)), W((4, 15))]}, A, 4),
        ("Böl, Çizgiyle kes: kesici çizgi alanı ikiye böler",
         [3], {"kind": "split", "ids": [U(3), U(6)], "cutter": U(6)}, C, 35),
    ):
        whole = cells(ring)
        west = {c for c in whole if c[0] < cut_x}
        east = whole - west
        cut = [(cut_x, -5), (cut_x, 15)]
        first, second = west, east
        sid = ids[0]
        out_.append(done(name, work, IDS + [NEXT],
                         {str(sid): shape(first, [ring, cut]), str(NEXT): shape(second, [ring, cut])},
                         out(changed=[sid], created=[NEXT], removed=[], result=[sid, NEXT], objects=1, pieces=2,
                             size=len(whole), sizes=[len(first), len(second)]),
                         fields={str(NEXT): {"attrs": dict(ENTITIES[sid - 1]["attrs"])}}, undo="Alan böl"))
    return out_


def plan_case():
    return {"name": "Plan: birleşim yazılmaz", "steps": [
        {"op": "plan", "input": edit({"kind": "union", "ids": [U(1), U(2)]}),
         "result": {"status": "completed", "output": {"changed": [], "created": "$any", "removed": "$any",
                                                       "revision": "$current"}, "warnings": []},
         "expect": UNCHANGED}]}


def revision_case():
    return {"name": "Sürüm değiştiyse hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "base"},
        {"op": "execute", "input": edit({"kind": "partsJoin", "ids": [U(15), U(16)]}),
         "result": {"status": "completed", "output": "$any", "warnings": []}},
        {"op": "execute", "input": {**edit({"kind": "union", "ids": [U(1), U(2)]}), "expectedRevision": "$base"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki "
                        "hâline göre yeniden hazırlayın.",
             "path": "expectedRevision", "revision": "$current"}},
         "expect": {"revision": "same"}},
    ]}


def bad_revision_case():
    return {"name": "Sürüm yazısı geçersiz", "steps": [
        {"op": "execute", "input": {**edit({"kind": "union", "ids": [U(1), U(2)]}), "expectedRevision": "012"},
         "result": {"status": "failed", "error": {
             "code": "invalid_revision",
             "message": "Beklenen sürüm “012” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü "
                        "belgeden ya da komutun planından alın.",
             "path": "expectedRevision"}},
         "expect": UNCHANGED}]}


def build():
    cases = union_cases() + intersect_cases() + subtract_cases() + split_cases() + split_written() + convert_cases() \
        + hole_cases() + parts_cases() + [plan_case(), revision_case(), bad_revision_case()]
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.areas.edit", "commandVersion": 1,
            "title": "Alan işlemleri",
            "note": "Written by scripts/fixtures/areas_edit_cases.py from the area operations' definitions on unit cells "
                    "(docs/adr/0622 §9, 0065, 0143, 0173); do not edit by hand.",
            "setup": SETUP, "cases": cases}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not FILE.exists() or FILE.read_text(encoding="utf-8") != text:
            print("cad.areas.edit durumları güncel değil; betiği --check'siz çalıştırın.")
            sys.exit(1)
        print(f"cad.areas.edit durumları güncel: {len(build()['cases'])} durum.")
        return
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
