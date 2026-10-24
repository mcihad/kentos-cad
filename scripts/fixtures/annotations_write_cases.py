"""The shared cases of cad.annotations.write (docs/adr/0622 §9): Koordinat yaz, Köşelere koordinat yaz and Km yaz.

    python3 scripts/fixtures/annotations_write_cases.py           # writes the file
    python3 scripts/fixtures/annotations_write_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.annotations.write.json, without KentOS code. The labels' places, lines and layout
are coordinate_label_cases.py's (docs/adr/0185), the stations' and what stands at them stationing_cases.py's
(docs/adr/0189); here they become the objects the command writes, in its order: a label's leader then its lines;
the ticks, the km texts, the cross-sections, then the points (each section and point with its `Km`). A text's
height in the drawing is its paper height times the scale (1:1000 here: millimetres are metres). The numbers are
compared within 10⁻⁹ m: the references compute exactly, the core in doubles.
"""
import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
sys.path.insert(0, str(Path(__file__).resolve().parent))
import coordinate_label_cases as coords  # noqa: E402
import point_calc_cases as pc  # noqa: E402
import stationing_cases as km  # noqa: E402
import text_along_command_cases as along_ref  # noqa: E402
import labels_to_text_cases as labels_ref  # noqa: E402
import quick_dimension_cases as quick_ref  # noqa: E402

POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000
FILE = OUT / "cad.annotations.write.json"
WITHIN = 1e-9


def W(p):
    return {"x": X + p[0], "y": Y + p[1]}


SQUARE = [(0, 0), (10, 0), (10, 10), (0, 10)]
ROUTE = [(0, 100), (45, 100)]
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "yapi", "attrs": {}, "pts": [W(p) for p in SQUARE]},
    {"kind": "point", "id": 2, "layerId": "yapi", "attrs": {}, "p": W((30, 0)), "z": 12.5, "label": "P1"},
    {"kind": "line", "id": 3, "layerId": "yapi", "attrs": {}, "a": W(ROUTE[0]), "b": W(ROUTE[1])},
    {"kind": "text", "id": 4, "layerId": "yapi", "attrs": {}, "p": W((60, 0)), "text": "Ada", "height": 2, "rotation": 0},
    {"kind": "arc", "id": 5, "layerId": "yapi", "attrs": {}, "c": W((100, 0)), "r": 20, "a0": math.radians(20),
     "a1": math.radians(160)},
    {"kind": "circle", "id": 6, "layerId": "yapi", "attrs": {}, "c": W((150, 0)), "r": 10},
]
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k != "entities"}, "entities": ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}
GIS = {"axes": "gis", "unit": "m"}
FONT = SETUP["settings"].get("drawingFont", "barlow")


def U(i):
    return f"$uidOf:{i}"


def num(v):
    v = F(v)
    return int(v) if v.denominator == 1 else float(v)


def P(p):
    return {"x": num(p[0]), "y": num(p[1])}


def write(work, height=None, **extra):
    inp = {"layerId": "yapi", "work": work}
    if height is not None:
        inp["height"] = height
    inp.update(extra)
    return inp


def done(name, inp, objects, count, undo, skipped=0):
    n = len(objects)
    fields = {str(NEXT + i): o for i, o in enumerate(objects)}
    output = {"ids": [NEXT + i for i in range(n)], "uids": [f"$uidOf:{NEXT + i}" for i in range(n)], "count": count,
              "skipped": skipped, "revision": "$current"}
    step = {"op": "execute", "input": inp, "within": WITHIN,
            "result": {"status": "completed", "output": output, "warnings": []},
            "expect": {"ids": IDS + [NEXT + i for i in range(n)], "fields": fields, "canUndo": True, "dirty": True}}
    back = {"op": "undo", "returns": undo, "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def refused(name, inp, code, message, path, non_finite=None):
    step = {"op": "execute", "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


# ── Koordinat yaz ────────────────────────────────────────────────────────────────────────────────────────────────


def world_place(p, z=None, name=None, centre=None):
    return {"p": (F(X) + p[0], F(Y) + p[1]), "z": z, "name": name, "centre": centre}


def label_objects(r, height):
    out = []
    for lb in r["labels"]:
        if lb["leader"] is not None:
            out.append({"kind": "polyline", "pts": [P(q) for q in lb["leader"]]})
        for x, y, t, a in lb["texts"]:
            o = {"kind": "text", "p": P((x, y)), "text": t, "height": height}
            if a:
                o["align"] = a
            out.append(o)
    return out


def options(template="{Y}|{X}", decimals=3, height=2.5, leader=True, lean="ne"):
    return {"template": template, "decimals": decimals, "height": height, "leader": leader, "direction": lean,
            "font": FONT}


LEAN = {"ne": "northEast", "nw": "northWest", "sw": "southWest", "se": "southEast", "auto": "auto"}


def coordinate_cases():
    out = []
    o = options()
    r = coords.labels([world_place((5, 5))], o, GIS)
    out.append(done("Koordinat yaz: kollu, iki satır, sağ üste; adım “Koordinat yaz”",
                    write({"kind": "coordinates", "at": [W((5, 5))], "leader": True, "lean": "northEast"}, 2.5),
                    label_objects(r, 2.5), 1, "Koordinat yaz"))
    o = options(template="{ad}|{Y}|{X}|{Z}", leader=False, lean="sw")
    r = coords.labels([world_place((30, 0), z=12.5, name="P1")], o, GIS)
    out.append(done("Koordinat yaz: noktanın adı ve kotu, kolsuz, sol alta",
                    write({"kind": "coordinates", "at": [W((30, 0))], "template": "{ad}|{Y}|{X}|{Z}",
                           "lean": "southWest"}, 2.5),
                    label_objects(r, 2.5), 1, "Koordinat yaz"))
    o = options(decimals=1, height=2)
    r = coords.labels([world_place((5, 5)), world_place((15, 5))], o, GIS)
    out.append(done("Koordinat yaz: iki yer, bir basamak",
                    write({"kind": "coordinates", "at": [W((5, 5)), W((15, 5))], "decimals": 1, "leader": True,
                           "lean": "northEast"}, 2),
                    label_objects(r, 2), 2, "Koordinat yaz"))
    # Köşelere koordinat yaz: the square's corners, unnamed, named 1 … 4, leaning away from its middle.
    centre = (F(X) + 5, F(Y) + 5)
    places = [world_place(p, name=str(i + 1), centre=centre) for i, p in enumerate(SQUARE)]
    o = options(template="{ad}", leader=False, lean="auto")
    r = coords.labels(places, o, GIS)
    out.append(done("Köşelere koordinat yaz: alanın köşeleri adlarıyla, kutunun ortasından dışa",
                    write({"kind": "coordinates", "ids": [U(1)], "template": "{ad}"}, 2.5),
                    label_objects(r, 2.5), 4, "Koordinat yaz"))
    out += [
        refused("Koordinat yaz: şablon satır bırakmıyor",
                write({"kind": "coordinates", "at": [W((5, 5))], "template": "{Z}"}, 2.5), "nothing_derived",
                "Koordinat yaz: şablon bu yerlere satır bırakmadı (adı ya da kotu yok).", "work.template"),
        refused("Koordinat yaz: hem yer hem nesne", write({"kind": "coordinates", "at": [W((5, 5))], "ids": [U(1)]}),
                "invalid_input",
                "Koordinatın yerlerini (at) ya da köşeleri yazılacak nesneleri (ids) verin; ikisi birden değil.",
                "work.at"),
        refused("Koordinat yaz: ne yer ne nesne", write({"kind": "coordinates"}), "invalid_input",
                "Koordinatın yerlerini (at) ya da köşeleri yazılacak nesneleri (ids) verin; ikisi birden değil.",
                "work.at"),
        refused("Koordinat yaz: yer yok", write({"kind": "coordinates", "at": []}), "too_few_points",
                "Koordinatı yazılacak en az bir yer verin.", "work.at"),
        refused("Köşelere koordinat yaz: yazının köşesi yok", write({"kind": "coordinates", "ids": [U(4)]}),
                "not_a_path", "Köşelere koordinat yaz: seçimde nokta, çizgi, çoklu çizgi ya da alan yok.", "work.ids"),
        refused("Koordinat yaz: yükseklik sıfır", write({"kind": "coordinates", "at": [W((5, 5))]}, 0),
                "invalid_height", "Yazı yüksekliği sıfırdan büyük olmalı.", "height"),
        refused("Koordinat yaz: genişlik çarpanı sıfır",
                write({"kind": "coordinates", "at": [W((5, 5))]}, widthFactor=0), "invalid_width_factor",
                "Genişlik çarpanı sıfırdan büyük olmalı.", "widthFactor"),
        refused("Koordinat yaz: yer sonlu değil", write({"kind": "coordinates", "at": [W((5, 5))]}), "not_finite",
                "Yerin doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                "work.at[0].x", non_finite={"work.at[0].x": "NaN"}),
    ]
    return out


# ── Km yaz ───────────────────────────────────────────────────────────────────────────────────────────────────────


def station_objects(route, rules, look):
    w = km.written(route, rules, look)
    assert "problem" not in w, w
    out = []
    for t in w["ticks"]:
        out.append({"kind": "line", "a": P(t["a"]), "b": P(t["b"])})
    for t in w["texts"]:
        o = {"kind": "text", "p": P(t["p"]), "text": t["text"], "height": look["height"], "rotation": t["rotation"]}
        if t["align"]:
            o["align"] = t["align"]
        out.append(o)
    for t in w["sections"]:
        out.append({"kind": "line", "a": P(t["a"]), "b": P(t["b"]), "attrs": {"Km": t["km"]}})
    for t in w["points"]:
        out.append({"kind": "point", "p": P(t["p"]), "attrs": {"Km": t["km"]}})
    return out, len(w["stations"])


ROUTE_SHAPE = pc.line((X + ROUTE[0][0], Y + ROUTE[0][1]), (X + ROUTE[1][0], Y + ROUTE[1][1]))


def station(work_extra, rules, look, height_mm):
    work = {"kind": "stations", "id": U(3), **work_extra}
    objects, n = station_objects(ROUTE_SHAPE, rules, look)
    return work, objects, n


def station_cases():
    out = []
    for name, extra, rules, look in (
        ("Km yaz: 20 m'de bir, uçlar, işaret, sol yazı; adım “Km yaz”",
         {"interval": 20, "ends": True, "text": "left", "tick": True}, km.rules(), km.look(height=2.0, tick=2.0)),
        ("Km yaz: başlangıç 15, uçsuz, sağ yazı, enkesit ve nokta",
         {"interval": 20, "start": 15, "text": "right", "section": 5, "point": 3},
         km.rules(start=15.0, ends=False), km.look(text="right", height=2.0, tick=0.0, section=5.0, point=3.0)),
        ("Km yaz: sondan, yazısız, yalnız işaret",
         {"interval": 10, "reverse": True, "ends": True, "text": "none", "tick": True},
         km.rules(interval=10.0, reverse=True), km.look(text=None, height=2.0, tick=2.0)),
    ):
        work, objects, n = station(extra, rules, look, 2.0)
        out.append(done(name, write(work, 2.0), objects, n, "Km yaz"))
    too_many = km.written(ROUTE_SHAPE, km.rules(interval=0.001), km.look())["problem"]
    out += [
        refused("Km yaz: aralık sıfır", write({"kind": "stations", "id": U(3), "interval": 0, "text": "left"}, 2.0),
                "invalid_distance", km.BAD_INTERVAL, "work.interval"),
        refused("Km yaz: çok istasyon",
                write({"kind": "stations", "id": U(3), "interval": 0.001, "text": "left"}, 2.0), "too_many_points",
                too_many, "work.interval"),
        refused("Km yaz: noktanın yolu yok", write({"kind": "stations", "id": U(2), "interval": 20, "text": "left"}),
                "no_route", km.NO_ROUTE, "work.id"),
        refused("Km yaz: yazılacak bir şey yok",
                write({"kind": "stations", "id": U(3), "interval": 20, "text": "none"}), "nothing_derived",
                "Km yaz: yazılacak bir şey yok; Yazı, İşaret, Enkesit ya da Nokta’yı açın.", "work.text"),
        refused("Km yaz: aralık sonlu değil", write({"kind": "stations", "id": U(3), "interval": 20, "text": "left"}),
                "not_finite", "Aralık sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.",
                "work.interval", non_finite={"work.interval": "Infinity"}),
    ]
    return out


def hidden_case():
    first = copy.deepcopy(coordinate_cases()[0])
    step = first["steps"][0]
    step["input"]["layerId"] = "gizli"
    for o in step["expect"]["fields"].values():
        o["layerId"] = "gizli"
    step["result"]["warnings"] = [
        {"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}]
    first["name"] = "Gizli katmana uyarıyla"
    return first


# ── Eğri boyunca yazı ────────────────────────────────────────────────────────────────────────────────────────────


def along_text(curve_id, click, words, align=None, height=2.5):
    curve = next(e for e in ENTITIES if e["id"] == curve_id)
    c = (X + click[0], Y + click[1])
    return along_ref.placed(curve, c, words, height, align or "bottomCenter", FONT)


def along_work(curve_id, click, words, align=None):
    w = {"kind": "along", "id": U(curve_id), "at": W(click), "text": words}
    if align:
        w["align"] = align
    return w


def along_cases():
    rows = [
        ("Eğri boyunca yazı: çizginin üstünde, tıklanan yer yazının ortası", 3, (20, 101), "Ada yolu", None),
        ("Eğri boyunca yazı: başı tıklanan yerde, uca sığmazsa geri çekilir", 3, (44, 100), "Yeşil sokak", "bottomLeft"),
        ("Eğri boyunca yazı: çizginin altında", 3, (10, 99), "Yol", "topCenter"),
        ("Eğri boyunca yazı: yayın üstünde", 5, (103, 22), "Çevre yolu", None),
        ("Eğri boyunca yazı: yayın ortasında", 5, (90, 19), "Kanal", "middleCenter"),
        ("Eğri boyunca yazı: dairenin altı okunur yöne çevrilir", 6, (149, -10.5), "Meydan", None),
        ("Eğri boyunca yazı: alanın halkası", 1, (5, -0.5), "Ada", None),
        ("Eğri boyunca yazı: çizgiden uzun yazı çizginin tamamında", 3, (20, 100), "Atatürk Bulvarı ve İnönü Caddesi kavşağı",
         None),
    ]
    out = [done(name, write(along_work(cid, click, words, align), 2.5), [along_text(cid, click, words, align)], 1,
                "Eğri boyunca yazı") for name, cid, click, words, align in rows]
    out += [
        refused("Eğri boyunca yazı: yazı eğri değil", write(along_work(4, (60, 0), "Ada")), "not_a_path",
                "Yazının izleyeceği nesne çizgi, yay, daire, alan ya da eğri olmalı.", "work.id"),
        refused("Eğri boyunca yazı: yazı boş", write(along_work(3, (20, 101), "  ")), "empty_text",
                "Eğri boyunca yazılacak yazıyı verin; yazı boş olamaz.", "work.text"),
        refused("Eğri boyunca yazı: yer sonlu değil", write(along_work(3, (20, 101), "Yol")), "not_finite",
                "Tıklanan yerin doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla "
                "verin.", "work.at.x", non_finite={"work.at.x": "NaN"}),
        refused("Eğri boyunca yazı: kilitli katmana yazılmaz", {**write(along_work(3, (20, 101), "Yol")), "layerId": "kilitli"},
                "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "layerId"),
    ]
    return out


# ── Hızlı ölçü ───────────────────────────────────────────────────────────────────────────────────────────────────


def quick_objects(ids, at, typed=None, height=2.5, mask=False):
    objs = [next(e for e in ENTITIES if e["id"] == i) for i in ids]
    r = quick_ref.quick(objs, (X + at[0], Y + at[1]), typed)
    out = []
    for d in r["dimensions"]:
        o = {"kind": "dimension", "a": d["a"], "b": d["b"], "offset": d["offset"], "height": height}
        if "c" in d:
            o["c"] = d["c"]
            o["style"] = d["style"]
        if mask:
            o["mask"] = True
        out.append(o)
    return out, r["skipped"]


def quick_work(ids, at, **extra):
    return {"kind": "dimensions", "ids": [U(i) for i in ids], "at": W(at), **extra}


def quick_cases():
    out = []
    rows = [
        ("Hızlı ölçü: alanın kenarları dışarıda, imlecin uzaklığında", [1], (5, -3), None, False),
        ("Hızlı ölçü: çizgi imlecin yanında", [3], (20, 104), None, False),
        ("Hızlı ölçü: yazılan uzaklık, yanı imleçten", [3], (20, 98), 2, False),
        ("Hızlı ölçü: alan ve çizgi birlikte", [1, 3], (5, -3), None, False),
        ("Hızlı ölçü: ölçülmeyen nesne sayılır", [1, 2, 4], (5, -3), None, False),
        ("Hızlı ölçü: Zemin", [1], (5, -3), 1.5, True),
        ("Hızlı ölçü: eksi yazılan uzaklığın büyüklüğü", [1], (5, -3), -2, False),
    ]
    for name, ids, at, typed, mask in rows:
        objects, skipped = quick_objects(ids, at, typed, mask=mask)
        extra = {}
        if typed is not None:
            extra["distance"] = typed
        if mask:
            extra["mask"] = True
        out.append(done(name, write(quick_work(ids, at, **extra), 2.5), objects, len(objects), "Hızlı ölçü",
                        skipped=skipped))
    out += [
        refused("Hızlı ölçü: ölçülecek nesne yok", write(quick_work([2, 4], (5, -3))), "nothing_derived",
                "Seçimde ölçülecek çizgi, çoklu çizgi ya da alan yok.", "work.ids"),
        refused("Hızlı ölçü: olmayan ölçü stili", write(quick_work([1], (5, -3), style="Yok stil")), "unknown_style",
                "“Yok stil” adlı ölçü stili yok. Projenin ölçü stillerinden birinin adını verin ya da alanı kaldırın "
                "(Standart).", "work.style"),
        refused("Hızlı ölçü: yer sonlu değil", write(quick_work([1], (5, -3))), "not_finite",
                "Ölçülerin yerinin doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir "
                "sayıyla verin.", "work.at.x", non_finite={"work.at.x": "NaN"}),
    ]
    return out


# ── Etiketleri yazıya çevir ──────────────────────────────────────────────────────────────────────────────────────


def labels_cases():
    def done_in(name, inp, objects, count, undo, skipped=0, setup=None):
        ids = [e["id"] for e in setup["entities"]]
        nxt = max(ids) + 1
        n = len(objects)
        output = {"ids": [nxt + i for i in range(n)], "uids": [f"$uidOf:{nxt + i}" for i in range(n)],
                  "count": count, "skipped": skipped, "revision": "$current"}
        step = {"op": "execute", "input": inp, "within": WITHIN,
                "result": {"status": "completed", "output": output, "warnings": []},
                "expect": {"ids": ids + [nxt + i for i in range(n)],
                           "fields": {str(nxt + i): o for i, o in enumerate(objects)}, "canUndo": True,
                           "dirty": True}}
        back = {"op": "undo", "returns": undo, "expect": {"ids": ids, "canUndo": False, "canRedo": True}}
        return {"name": name, "setup": setup, "steps": [step, back]}

    def refused_in(name, inp, code, message, path, setup=None):
        ids = [e["id"] for e in setup["entities"]]
        return {"name": name, "setup": setup, "steps": [{
            "op": "execute", "input": inp,
            "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
            "expect": {"ids": ids, "dirty": False, "revision": "same"}}]}

    return labels_ref.cases(done_in, refused_in)


def general_cases():
    at = {"kind": "coordinates", "at": [W((5, 5))]}
    return [hidden_case(),
        refused("Olmayan katman", {**write(at), "layerId": "yok"}, "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Katman değil grup", {**write(at), "layerId": "pafta"}, "not_a_layer",
                "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        {"name": "Sürüm değiştiyse hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "base"},
            {"op": "execute", "input": write(at), "result": {"status": "completed", "output": "$any", "warnings": []}},
            {"op": "execute", "input": write({"kind": "coordinates", "at": [W((6, 6))]}, expectedRevision="$base"),
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin "
                            "şimdiki hâline göre yeniden hazırlayın.",
                 "path": "expectedRevision", "revision": "$current"}},
             "expect": {"revision": "same"}}]},
        refused("Kilitli katmana yazılmaz", {**write({"kind": "coordinates", "at": [W((5, 5))]}), "layerId": "kilitli"},
                "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "layerId"),
        refused("Olmayan kimlik", write({"kind": "stations", "id": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                                         "interval": 20, "text": "left"}), "entity_not_found",
                "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
                "olabilir. Var olan bir nesnenin kimliğini verin.", "work.id"),
        refused("Kimlik değil", write({"kind": "coordinates", "ids": ["kose"]}), "invalid_uid",
                "“kose” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "work.ids[0]"),
        refused("Sürüm yazısı geçersiz", write({"kind": "coordinates", "at": [W((5, 5))]}, expectedRevision="012"),
                "invalid_revision",
                "Beklenen sürüm “012” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                "ya da komutun planından alın.", "expectedRevision"),
        {"name": "Plan: yazılmaz", "steps": [
            {"op": "plan", "input": write({"kind": "coordinates", "at": [W((5, 5))]}),
             "result": {"status": "completed", "output": "$any", "warnings": []},
             "expect": UNCHANGED}]},
    ]


def build():
    cases = coordinate_cases() + station_cases() + along_cases() + quick_cases() + labels_cases() + general_cases()
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.annotations.write", "commandVersion": 1,
            "title": "Açıklama yaz",
            "note": "Written by scripts/fixtures/annotations_write_cases.py from coordinate_label_cases.py and "
                    "stationing_cases.py, Eğri boyunca yazı from text_along_command_cases.py, Etiketleri yazıya çevir from "
                    "labels_to_text_cases.py, Hızlı ölçü from quick_dimension_cases.py (docs/adr/0622 §9, 0185, 0189, 0196, "
                    "0175, 0147 §7); do not edit by hand.",
            "setup": SETUP, "cases": cases}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not FILE.exists() or FILE.read_text(encoding="utf-8") != text:
            print("cad.annotations.write durumları güncel değil; betiği --check'siz çalıştırın.")
            sys.exit(1)
        print(f"cad.annotations.write durumları güncel: {len(build()['cases'])} durum.")
        return
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
