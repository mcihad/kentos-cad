"""Taramaları düzenle: shared cases of cad.hatches.edit (docs/adr/0329 §4).

    python3 scripts/fixtures/hatch_edit_cases.py           # writes the file
    python3 scripts/fixtures/hatch_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.hatches.edit.json. The checks, their order, codes, paths and messages are written here
by hand from the ADR; what the edits do is worked out on a drawing of axis-parallel areas, without KentOS code:

- A tied hatch's islands added or taken away: its region is its closed object less every closed island named, as the
  geometry core cuts an area: the outer ring counter-clockwise from its lower left corner, each hole clockwise from its
  lower right corner, the holes in the order the cut gives them (left to right here).
- An untied hatch's islands added: its region less the islands' areas, the largest part left; taken away: the holes
  named by their places dropped, the rings kept as they were.
- Sınırı yeniden oluştur: the boundary's loops as they are (a straight hatch's ring and holes): an area (the ring its
  points, the holes its holes) or one closed polyline a loop, a whole ellipse an ellipse; on the hatch's layer unless
  one is named; tied, the hatch follows the area (outer) or the polylines (the first its bounds, the others islands).
- Sınırı yeniden seç: the region as cad.entities.hatch's point region finds it (fixtures/commands/v1/cad.entities.hatch.json
  says how); the pattern, layer and look kept.
- İlişkiyi kopar: the hatch's tie dropped; a hatch without one is left as it is.
- One undo step named after the edit.
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import region_command_cases as R  # noqa: E402  (messages and the hatch command's own rules)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.hatches.edit.json"
P = R.P
UID = R.UID
TAU = 2 * math.pi

PARCEL = (0, 0, 20, 16)
BUILDING = (4, 4, 8, 8)
POOL = (12, 4, 16, 8)
ROOM = (30, 0, 40, 8)
ROOM_HOLE = (32, 2, 34, 4)
COL = (36, 4, 38, 6)
LOCKED = (70, 0, 74, 4)
LINES_ROOM = (80, 0, 90, 8)
BIG = (28, -2, 42, 10)
ELL_C = (25, 30)


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def pts(ring):
    return [P(*p) for p in ring]


def outer_ring(r):
    """The core's outer ring: counter-clockwise from the lower left corner."""
    x0, y0, x1, y1 = r
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def hole_ring(r):
    """The core's hole: clockwise from the lower right corner."""
    x0, y0, x1, y1 = r
    return [(x1, y0), (x0, y0), (x0, y1), (x1, y1)]


def polygon(i, r, layer_id="yapi"):
    return {"kind": "polygon", "id": i, "layerId": layer_id, "attrs": {}, "pts": pts(rect(*r))}


def line(i, a, b, layer_id="yapi"):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, "a": P(*a), "b": P(*b)}


SOLID = {"type": "solid", "angle": 0, "spacing": 1}


def hatch(i, ring, holes=(), assoc=None, layer_id="tarama", boundary=None):
    e = {"kind": "hatch", "id": i, "layerId": layer_id, "attrs": {}, "ring": pts(ring)}
    if holes:
        e["holes"] = [pts(h) for h in holes]
    e["pattern"] = SOLID
    if assoc:
        e["assoc"] = assoc
    if boundary:
        e["boundary"] = boundary
    return e


ENTITIES = [
    polygon(1, PARCEL),
    polygon(2, BUILDING),
    polygon(3, POOL),
    hatch(4, rect(*ROOM), [hole_ring(ROOM_HOLE)]),
    polygon(5, COL),
    line(6, (50, 0), (60, 0)),
    hatch(7, rect(*LOCKED), layer_id="kilitli"),
    line(8, (80, 0), (90, 0)),
    line(9, (90, 0), (90, 8)),
    line(10, (90, 8), (80, 8)),
    # The left wall stops 0.125 m short of the floor: the room closes across the gap only (docs/adr/0329 §3).
    line(11, (80, 8), (80, R.F(1, 8))),
    polygon(12, BIG),
    {"kind": "ellipse", "id": 13, "layerId": "yapi", "attrs": {}, "c": P(*ELL_C), "major": {"x": 4, "y": 0}, "ratio": 0.5,
     "t0": 0, "t1": TAU},
]
# docs/adr/0331: patterned hatches to change and line up; their patterns as cad.entities.hatch writes them at 1:500.
ANSI_A = (100, 0, 110, 8)
ANSI_B = (112, 0, 122, 8)
LINED = (124, 0, 134, 8)
ENTITIES += [
    {**hatch(14, rect(*ANSI_A)), "pattern": R.pattern("ANSI31")},
    {**hatch(15, rect(*ANSI_B)), "pattern": {**R.pattern("ANSI31", 2.0, 30.0), "origin": {"x": 101, "y": 1}}},
    {**hatch(16, rect(*LINED)), "pattern": R.pattern("Çizgili"), "annotative": {"at": 500}},
]
IDS = [e["id"] for e in ENTITIES]
BY_ID = {e["id"]: e for e in ENTITIES}
NEXT = 17
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Taramaları düzenle",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500},
    "origin": {"x": R.X, "y": R.Y},
    "layers": [
        R.layer("yapi", "Yapı"), R.layer("alan", "Alan"), R.layer("tarama", "Tarama"),
        R.layer("kilitli", "Kilitli", locked=True), R.layer("gizli", "Gizli", visible=False),
        R.layer("grup", "Grup", kind="group", children=[R.layer("alt", "Alt")]),
    ],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def seed(x, y):
    return {"x": x, "y": y}


def tied_hatch_step(at=(2, 2), islands=True):
    """Tarama at `at`, kapalı nesne, İlişkili: the hatch every tied case starts from (id NEXT); in the parcel the
    building and the pool its islands, unless Adalar is off."""
    region = {"kind": "point", "at": P(*at)}
    if not islands:
        region["islands"] = False
    return {"op": "execute", "command": "cad.entities.hatch",
            "input": {"region": region, "pattern": {"name": "Dolu"}, "layerId": "tarama"},
            "result": R.completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"})}


def edit(e, **more):
    return {"edit": e, **more}


def done(changed, created=(), warnings=()):
    out = {"changed": changed, "revision": "$current"}
    if created:
        out["created"] = list(created)
    return R.completed(out, warnings)


def case(name, steps):
    return {"name": name, "steps": steps}


def written(inp, result, entities, ids, undo=None, plan=False):
    if plan:
        return [{"op": "plan", "input": inp, "result": result}]
    step = {"op": "execute", "input": inp, "result": result,
            "expect": {"ids": ids, "entities": entities, "revision": "changed", "canUndo": True, "dirty": True}}
    out = [step]
    if undo:
        out.append({"op": "undo", "returns": undo})
    return out


def refused(name, inp, code, message, path, steps_before=(), non_finite=None):
    step = {"op": "execute", "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = R.failed(code, message, path)
    if not steps_before:
        step["expect"] = UNCHANGED
    return case(name, [*steps_before, step])


TIED = {"outer": UID(1), "islands": [UID(2), UID(3)], "seed": seed(2, 2)}
# The tied hatch as cad.entities.hatch writes it: its rings as that command's (holes clockwise from their lowest).
MADE_HOLES = [R.canonical(rect(*BUILDING), False), R.canonical(rect(*POOL), False)]
NOT_A_HATCH = "Verilen nesne bir tarama değil; bu komut yalnız taramaları değiştirir."
NOT_ENCLOSED = ("Taranacak bölge kalmadı: adalar taramanın bütün bölgesini kaplıyor ya da nesneleri bir bölge kapatmıyor.")


def cases():
    out = []
    t = NEXT
    all_ids = IDS + [t]
    # ── İlişkiyi kopar ──
    out.append(case("İlişkiyi kopar: tarama bir şeyi izlemez, yerinde kalır; adım “İlişkiyi kopar”", [
        tied_hatch_step(),
        *written(edit({"kind": "untie", "uids": [UID(t)]}), done([UID(t)]),
                 {str(t): hatch(t, outer_ring(PARCEL), MADE_HOLES)}, all_ids, undo="İlişkiyi kopar")]))
    out.append(case("İlişkiyi kopar: ilişkisiz tarama değişmez", [
        {"op": "execute", "input": edit({"kind": "untie", "uids": [UID(4)]}), "result": done([]),
         "expect": {"ids": IDS, "entities": {"4": BY_ID[4]}}}]))
    # ── Ada ekle ──
    out.append(case("Ada ekle, ilişkili: adasız taranmış parsele havuz ada olur, tarama nesnelerinden yeniden kesilir; adım “Ada ekle”", [
        tied_hatch_step(islands=False),
        *written(edit({"kind": "addIslands", "uid": UID(t), "uids": [UID(3)]}), done([UID(t)]),
                 {str(t): hatch(t, outer_ring(PARCEL), [hole_ring(POOL)],
                                {"outer": UID(1), "islands": [UID(3)], "seed": seed(2, 2)})}, all_ids, undo="Ada ekle")]))
    out.append(case("Ada ekle, ilişkili: zaten ada olan bir kez", [
        tied_hatch_step(),
        {"op": "execute", "input": edit({"kind": "addIslands", "uid": UID(t), "uids": [UID(2)]}), "result": done([])}]))
    out.append(case("Ada ekle, ilişkisiz: kolonun alanı bölgeden çıkar, yeni delik", [
        *written(edit({"kind": "addIslands", "uid": UID(4), "uids": [UID(5)]}), done([UID(4)]),
                 {"4": hatch(4, outer_ring(ROOM), [hole_ring(ROOM_HOLE), hole_ring(COL)])}, IDS)]))
    out.append(case("Ada ekle, ilişkisiz: bütün bölgeyi kaplayan ada not_enclosed", [
        {"op": "execute", "input": edit({"kind": "addIslands", "uid": UID(4), "uids": [UID(12)]}),
         "result": R.failed("not_enclosed", NOT_ENCLOSED, "edit.uids"), "expect": UNCHANGED}]))
    # ── Ada çıkar ──
    out.append(case("Ada çıkar, ilişkili: bina adalardan çıkar, yeniden taranır; adım “Ada çıkar”", [
        tied_hatch_step(),
        *written(edit({"kind": "removeIslands", "uid": UID(t), "uids": [UID(2)]}), done([UID(t)]),
                 {str(t): hatch(t, outer_ring(PARCEL), [hole_ring(POOL)], {"outer": UID(1), "islands": [UID(3)], "seed": seed(2, 2)})},
                 all_ids, undo="Ada çıkar")]))
    out.append(case("Ada çıkar, ilişkisiz: deliğin sırasıyla", [
        *written(edit({"kind": "removeIslands", "uid": UID(4), "holes": [0]}), done([UID(4)]),
                 {"4": hatch(4, rect(*ROOM))}, IDS)]))
    # ── Sınırı yeniden seç ──
    gap_ring = R.canonical([(80, 0), (90, 0), (90, 8), (80, 8), (80, R.F(1, 8))], True)
    gap_said = R.warning("gaps_closed", "1 boşluk kapatıldı (toplam 0.125 m, en uzunu 0.125 m); taranan alan 80.00 m².", "edit.region.gap")
    out.append(case("Sınırı yeniden seç: çizgilerle, boşluğu kapatarak; deseni ve katmanı kalır, yüzün çizgilerine ve boşluğa bağlı; adım “Sınırı yeniden seç”",
                    written(edit({"kind": "reselect", "uid": UID(4), "region": {"kind": "point", "at": P(85, 4), "way": "lines", "gap": 0.25}}),
                            done([UID(4)], warnings=[gap_said]),
                            {"4": hatch(4, gap_ring, [],
                                        {"bounds": [UID(8), UID(9), UID(10), UID(11)], "gap": 0.25, "seed": seed(85, 4)})}, IDS,
                            undo="Sınırı yeniden seç")))
    out.append(case("Sınırı yeniden seç: çizgilerde İç içe Dış gibi, islands_not_nested uyarısıyla", [
        {"op": "execute", "input": edit({"kind": "reselect", "uid": UID(4),
                                         "region": {"kind": "point", "at": P(85, 4), "way": "lines", "gap": 0.25, "islandStyle": "normal"}}),
         "result": done([UID(4)], warnings=[R.warning("islands_not_nested", "Çizgiler yolunda iç içe adalar taranmaz; adalar boş bırakıldı (Dış).",
                                                      "edit.region.islandStyle"), gap_said])}]))
    out.append(case("Sınırı yeniden seç: kapalı nesneyle, ilişkisiz", written(
        edit({"kind": "reselect", "uid": UID(4), "region": {"kind": "point", "at": P(2, 2), "associative": False}}), done([UID(4)]),
        {"4": hatch(4, R.canonical(rect(*PARCEL), True), MADE_HOLES)}, IDS)))
    # ── Sınırı yeniden oluştur ──
    area_obj = {"kind": "polygon", "id": t + 1, "layerId": "tarama", "attrs": {}, "pts": pts(outer_ring(PARCEL)),
                "holes": [{"pts": pts(h)} for h in MADE_HOLES]}
    out.append(case("Sınırı yeniden oluştur: alan, deliği taramanınki; tarama sonra alanı izler; adım “Sınırı yeniden oluştur”", [
        tied_hatch_step(),
        *written(edit({"kind": "recreate", "uid": UID(t)}), done([UID(t)], [UID(t + 1)]),
                 {str(t + 1): area_obj,
                  str(t): hatch(t, outer_ring(PARCEL), MADE_HOLES, {"outer": UID(t + 1), "seed": seed(2, 2)})},
                 all_ids + [t + 1], undo="Sınırı yeniden oluştur")]))
    out.append(case("Sınırı yeniden oluştur: başka katmana, ilişkisiz; tarama değişmez", [
        tied_hatch_step(),
        *written(edit({"kind": "recreate", "uid": UID(t), "layerId": "alan", "associative": False}), done([], [UID(t + 1)]),
                 {str(t + 1): {**area_obj, "layerId": "alan"},
                  str(t): hatch(t, outer_ring(PARCEL), MADE_HOLES, TIED)}, all_ids + [t + 1])]))
    out.append(case("Sınırı yeniden oluştur: gizli katmana, layer_hidden uyarısıyla", [
        tied_hatch_step(),
        {"op": "execute", "input": edit({"kind": "recreate", "uid": UID(t), "layerId": "gizli", "associative": False}),
         "result": done([], [UID(t + 1)], [R.warning("layer_hidden", "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "edit.layerId")])}]))
    poly = lambda i, ring: {"kind": "polyline", "id": i, "layerId": "tarama", "attrs": {}, "pts": pts(ring), "closed": True}  # noqa: E731
    out.append(case("Sınırı yeniden oluştur: çoklu çizgiler, her halka bir kapalı çizgi; ilişkisiz", [
        tied_hatch_step(),
        *written(edit({"kind": "recreate", "uid": UID(t), "form": "polyline", "associative": False}),
                 done([], [UID(t + 1), UID(t + 2), UID(t + 3)]),
                 {str(t + 1): poly(t + 1, outer_ring(PARCEL)), str(t + 2): poly(t + 2, MADE_HOLES[0]),
                  str(t + 3): poly(t + 3, MADE_HOLES[1])}, all_ids + [t + 1, t + 2, t + 3])]))
    ell = {"kind": "ellipse", "id": t + 1, "layerId": "tarama", "attrs": {}, "c": P(*ELL_C), "major": {"x": 4, "y": 0}, "ratio": 0.5,
           "t0": 0, "t1": TAU}
    out.append(case("Sınırı yeniden oluştur: elipsin taraması, çoklu çizgi biçiminde tam elips kesin", [
        tied_hatch_step(ELL_C),
        *written(edit({"kind": "recreate", "uid": UID(t), "form": "polyline", "associative": False}), done([], [UID(t + 1)]),
                 {str(t + 1): ell}, all_ids + [t + 1])]))
    out.append(case("Sınırı yeniden oluştur: elipsin taraması alan biçiminde kirişleriyle, curves_approximated uyarısıyla", [
        tied_hatch_step(ELL_C),
        {"op": "execute", "input": edit({"kind": "recreate", "uid": UID(t), "associative": False}),
         "result": done([], [UID(t + 1)], [R.warning("curves_approximated",
                                                     "Sınırın elips ya da eğri kenarları 0,1 mm'lik kirişleriyle yazıldı; alan yalnız doğru ve yay tutar.",
                                                     "edit.form")])}]))
    out.append(case("Plan: ada eklenmiş tarama, id'siyle; hiçbir şey yazılmaz", [
        {"op": "plan", "input": edit({"kind": "addIslands", "uid": UID(4), "uids": [UID(5)]}),
         "result": R.completed({"hatches": [hatch(4, outer_ring(ROOM), [hole_ring(ROOM_HOLE), hole_ring(COL)])], "revision": "$current"}),
         "expect": UNCHANGED}]))
    # ── Desen ve Hizala (docs/adr/0331) ──
    out += pattern_cases()
    # ── Retler ──
    out += [
        refused("Kimlik geçersiz", edit({"kind": "untie", "uids": [R.BAD]}), "invalid_uid", R.bad_uid(R.BAD), "edit.uids[0]"),
        refused("Nesne çizimde yok", edit({"kind": "addIslands", "uid": R.MISSING, "uids": [UID(5)]}), "entity_not_found",
                R.not_found(R.MISSING), "edit.uid"),
        refused("Tarama değil", edit({"kind": "recreate", "uid": UID(1)}), "not_a_hatch", NOT_A_HATCH, "edit.uid"),
        refused("Kilitli katmandaki tarama değişmez", edit({"kind": "untie", "uids": [UID(7)]}), "layer_locked",
                "“Kilitli” katmanı kilitli; üzerindeki tarama değişmez. Kilidi Katmanlar panelinden açın.", "edit.uids[0]"),
        refused("Ada yok", edit({"kind": "addIslands", "uid": UID(4), "uids": []}), "no_entities",
                "Ada yok. En az bir kapalı nesnenin kalıcı kimliğini verin.", "edit.uids"),
        refused("Açık çizgi ada olmaz", edit({"kind": "addIslands", "uid": UID(4), "uids": [UID(6)]}), "not_closed",
                "Nesne kapalı değil: ada alan, daire, tam elips ya da kapalı eğri olur.", "edit.uids[0]"),
        refused("İlişkili taramanın adası değil", edit({"kind": "removeIslands", "uid": UID(t), "uids": [UID(5)]}), "island_not_found",
                "Verilen nesne taramanın adalarından biri değil; ilişkideki adaları Öznitelikler gösterir.", "edit.uids[0]",
                steps_before=[tied_hatch_step()]),
        refused("İlişkili taramanın adası deliğin sırasıyla verilmez", edit({"kind": "removeIslands", "uid": UID(t), "holes": [0]}), "island_not_found",
                "İlişkili taramanın adası nesnesiyle (uids) verilir; deliğin sırası onu izlemez.", "edit.holes", steps_before=[tied_hatch_step()]),
        refused("İlişkisiz taramanın adası nesneyle verilmez", edit({"kind": "removeIslands", "uid": UID(4), "uids": [UID(5)]}), "island_not_found",
                "İlişkisiz taramanın adası deliğinin sırasıyla (holes) verilir; nesneleri yoktur.", "edit.uids"),
        refused("Delik sırası taramanın deliklerinden büyük", edit({"kind": "removeIslands", "uid": UID(4), "holes": [5]}), "island_not_found",
                "Taramanın 1 deliği var; 5. sırada delik yok (0 ilki).", "edit.holes[0]"),
        refused("Çıkarılacak ada yok", edit({"kind": "removeIslands", "uid": UID(4)}), "no_entities",
                "Çıkarılacak ada yok. Adaların nesnelerini (uids) ya da deliklerin sırasını (holes) verin.", "edit.uids"),
        refused("Sınırı yeniden seç: Çoklu tara'nın nesneleri alınmaz", edit({"kind": "reselect", "uid": UID(4),
                                                                               "region": {"kind": "objects", "uids": [UID(1)]}}),
                "invalid_region", "Bölge tek bir noktayla verilmeli (kind: point); Çoklu tara'nın nesneleri burada alınmaz.", "edit.region"),
        refused("Sınırı yeniden seç: ek nokta verilmez", edit({"kind": "reselect", "uid": UID(4),
                                                                 "region": {"kind": "point", "at": P(2, 2), "alsoAt": [P(3, 3)]}}),
                "invalid_region", "Bir taramanın bölgesi tek noktayla bulunur; alsoAt verilmez.", "edit.region.alsoAt"),
        refused("Sınırı yeniden seç: noktayı çevreleyen bölge yok", edit({"kind": "reselect", "uid": UID(4),
                                                                            "region": {"kind": "point", "at": P(200, 200)}}),
                "not_enclosed", R.NOT_ENCLOSED_OBJECT, "edit.region.at"),
        refused("Sınırı yeniden seç: nokta sonlu değil", edit({"kind": "reselect", "uid": UID(4), "region": {"kind": "point", "at": P(0, 0)}}),
                "not_finite", R.not_finite_point("Tıklanan noktanın", "x"), "edit.region.at.x", non_finite={"edit.region.at.x": "NaN"}),
        refused("Sınırı yeniden seç: boşluksuz oda kapanmaz", edit({"kind": "reselect", "uid": UID(4),
                                                                     "region": {"kind": "point", "at": P(85, 4), "way": "lines"}}),
                "not_enclosed", R.NOT_ENCLOSED_LINES, "edit.region.at"),
        refused("Sınırı yeniden seç: boşluk toleransı 1000 m'yi aşamaz", edit({"kind": "reselect", "uid": UID(4),
                                                                                "region": {"kind": "point", "at": P(85, 4), "way": "lines", "gap": 2000}}),
                "invalid_gap", "Boşluk toleransı 2000; 0 ile 1000 m arasında sonlu bir sayı olmalı.", "edit.region.gap"),
        refused("Sınırı yeniden oluştur: katman yok", edit({"kind": "recreate", "uid": UID(4), "layerId": "yok"}), "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "edit.layerId"),
        refused("Sınırı yeniden oluştur: grup katman değildir", edit({"kind": "recreate", "uid": UID(4), "layerId": "grup"}), "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "edit.layerId"),
        refused("İlişkiyi kopar: tarama yok", edit({"kind": "untie", "uids": []}), "no_entities",
                "Tarama yok. En az bir taramanın kalıcı kimliğini verin.", "edit.uids"),
        refused("Beklenen sürüm sayı değil", edit({"kind": "untie", "uids": [UID(4)]}, expectedRevision="on iki"), "invalid_revision",
                R.INVALID_REVISION, "expectedRevision"),
    ]
    out.append(case("Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", [
        {"op": "captureRevision", "as": "old"},
        tied_hatch_step(),
        {"op": "execute", "input": edit({"kind": "untie", "uids": [UID(t)]}, expectedRevision="$old"),
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": R.CONFLICT, "path": "expectedRevision",
                                                     "revision": "$current"}}}]))
    return out


def patterned(i, pat, **more):
    """Hatch `i` with `pat` (and its scales `more`), the rest of it as it is."""
    e = {k: v for k, v in BY_ID[i].items() if k not in ("pattern", "annotative")}
    return {**e, "pattern": pat, **more}


def origin(pat, x, y):
    return {**pat, "origin": {"x": x, "y": y}}


def pattern_cases():
    ansi = R.pattern("ANSI31")
    pat = lambda uids, spec: edit({"kind": "pattern", "uids": [UID(u) for u in uids], "pattern": spec})  # noqa: E731
    align = lambda uids, ref: edit({"kind": "align", "uids": [UID(u) for u in uids], "reference": UID(ref)})  # noqa: E731
    return [
        case("Desen: dolu tarama ANSI31 olur, bölgesi kalır; adım “Deseni değiştir”",
             written(pat([4], {"name": "ANSI31"}), done([UID(4)]), {"4": patterned(4, ansi)}, IDS, undo="Deseni değiştir")),
        case("Desen: ölçek ve açı Tarama'nınki gibi",
             written(pat([14], {"name": "ANSI31", "scale": 2, "angle": 15}), done([UID(14)]),
                     {"14": patterned(14, R.pattern("ANSI31", 2.0, 15.0))}, IDS)),
        case("Desen: başlangıç verilmezse her tarama kendininkini korur",
             written(pat([15], {"name": "ANSI31"}), done([UID(15)]), {"15": patterned(15, origin(ansi, 101, 1))}, IDS)),
        case("Desen: başlangıç sol alt, her taramanın kendi kutusunun",
             written(pat([14, 15], {"name": "ANSI31", "origin": {"kind": "corner", "corner": "bottomLeft"}}), done([UID(14), UID(15)]),
                     {"14": patterned(14, origin(ansi, 100, 0)), "15": patterned(15, origin(ansi, 112, 0))}, IDS)),
        case("Desen: başlangıç sol alt, nesneler boyunca hizalı: ikisinin kutusunun",
             written(pat([14, 15], {"name": "ANSI31", "origin": {"kind": "corner", "corner": "bottomLeft", "across": True}}),
                     done([UID(14), UID(15)]), {"14": patterned(14, origin(ansi, 100, 0)), "15": patterned(15, origin(ansi, 100, 0))}, IDS)),
        case("Desen: başlangıç dünya kendi başlangıcını kaldırır",
             written(pat([15], {"name": "ANSI31", "origin": {"kind": "world"}}), done([UID(15)]), {"15": patterned(15, ansi)}, IDS)),
        case("Desen: ölçekli yapılır, çizim ölçeğinde",
             written(pat([14], {"name": "ANSI31", "annotative": True}), done([UID(14)]),
                     {"14": patterned(14, ansi, annotative={"at": 500})}, IDS)),
        case("Desen: ölçekli tarama çizgili kalırken ölçeklerini korur",
             written(pat([16], {"name": "Çizgili", "angle": 10}), done([UID(16)]),
                     {"16": patterned(16, R.pattern("Çizgili", 1.0, 10.0), annotative={"at": 500})}, IDS)),
        case("Desen: dolu olunca ölçekleri ve başlangıcı düşer",
             written(pat([15, 16], {"name": "Dolu"}), done([UID(15), UID(16)]),
                     {"15": patterned(15, SOLID), "16": patterned(16, SOLID)}, IDS)),
        case("Desen: ölçekli kapatılır",
             written(pat([16], {"name": "Çizgili", "annotative": False}), done([UID(16)]),
                     {"16": patterned(16, R.pattern("Çizgili"))}, IDS)),
        case("Desen: aynı desen değişiklik değil, yazılmaz",
             [{"op": "execute", "input": pat([14], {"name": "ANSI31"}), "result": done([]), "expect": {"ids": IDS, "entities": {"14": BY_ID[14]}}}]),
        case("Hizala: aynı desen başlangıcı, açıyı ve ölçeği alır; adım “Taramaları hizala”",
             written(align([14], 15), done([UID(14)]), {"14": patterned(14, origin(R.pattern("ANSI31", 2.0, 30.0), 101, 1))}, IDS,
                     undo="Taramaları hizala")),
        case("Hizala: başka desen yalnız başlangıcı alır",
             written(align([16], 15), done([UID(16)]), {"16": patterned(16, origin(R.pattern("Çizgili"), 101, 1), annotative={"at": 500})}, IDS)),
        case("Hizala: dolu tarama değişmez",
             [{"op": "execute", "input": align([4], 15), "result": done([]), "expect": {"ids": IDS, "entities": {"4": BY_ID[4]}}}]),
        case("Hizala: dünya başlangıçlı başvuru başlangıcı kaldırır",
             written(align([15], 14), done([UID(15)]), {"15": patterned(15, ansi)}, IDS)),
        case("Plan: desen değişmiş tarama; hiçbir şey yazılmaz",
             [{"op": "plan", "input": pat([4], {"name": "ANSI31"}),
               "result": R.completed({"hatches": [patterned(4, ansi)], "revision": "$current"}), "expect": UNCHANGED}]),
        refused("Desen: bilinmeyen ad", pat([14], {"name": "Yok"}), "unknown_pattern",
                "“Yok” adında desen yok. Desen kitaplığındaki ya da projenin desenlerindeki bir adı verin (Dolu, Çizgili, ANSI31, ISO02W100 …).",
                "edit.pattern.name"),
        refused("Desen: tarama yok", pat([], {"name": "ANSI31"}), "no_entities", "Tarama yok. En az bir taramanın kalıcı kimliğini verin.", "edit.uids"),
        refused("Desen: ölçekli ve çizimde birlikte olmaz", pat([14], {"name": "ANSI31", "basis": "world", "annotative": True}), "not_annotative",
                "Ölçekli taramanın deseni kâğıttaki ölçüsüyle çizilir; çizimdeki ölçekle (basis: world) ölçekli olmaz.", "edit.pattern.annotative"),
        refused("Desen: ölçek sıfır olamaz", pat([14], {"name": "ANSI31", "scale": 0}), "invalid_scale",
                "Desenin ölçeği sıfırdan büyük, en çok 10 000 olan sonlu bir sayı olmalı.", "edit.pattern.scale"),
        refused("Desen: ikinci renk renk değil", pat([14], {"name": "Degrade küre", "secondColor": "mor-ötesi"}), "invalid_color",
                "“mor-ötesi” bir renk değil: #RRGGBB ya da bir renk adı (Beyaz, Siyah, Kırmızı …) verin.", "edit.pattern.secondColor"),
        refused("Desen: çok sık", pat([14], {"name": "Çizgili", "scale": 0.0001}), "pattern_too_dense",
                "Desen bir tarama için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin.", "edit.pattern"),
        refused("Hizala: başvuru dolu tarama", align([14], 4), "not_a_hatch",
                "Başvuru taramasının çizgisi yok (dolu ya da degrade); çizgili ya da desenli bir tarama verin.", "edit.reference"),
        refused("Hizala: başvuru tarama değil", align([14], 1), "not_a_hatch", NOT_A_HATCH, "edit.reference"),
        refused("Hizala: kilitli katmandaki tarama değişmez", align([7], 15), "layer_locked",
                "“Kilitli” katmanı kilitli; üzerindeki tarama değişmez. Kilidi Katmanlar panelinden açın.", "edit.uids[0]"),
    ]


def build():
    data = {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.hatches.edit",
        "commandVersion": 1,
        "title": "Taramaları düzenle: sınırı yeniden oluştur, yeniden seç, ada ekle ve çıkar, ilişkiyi kopar (docs/adr/0329 §4)",
        "note": "scripts/fixtures/hatch_edit_cases.py yazar; kurallar ADR'den, eksene paralel çizimde KentOS kodu olmadan. "
                "Çekirdeğin kestiği alanda dış halka saat yönünün tersine sol alttan, delik saat yönünde sağ alttan.",
        "setup": SETUP,
        "cases": cases(),
    }
    R.assert_no_negative_zero(data, "cad.hatches.edit")
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


def main():
    text = build()
    n = len(json.loads(text)["cases"])
    if "--check" in sys.argv:
        if OUT.read_text("utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} eski: python3 scripts/fixtures/hatch_edit_cases.py ile yeniden yazın.")
        print(f"cad.hatches.edit durumları güncel: {n} durum.")
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({n} durum)")


if __name__ == "__main__":
    main()
