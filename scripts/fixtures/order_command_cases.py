"""The shared cases of the product command cad.entities.order: Öne getir,
Arkaya gönder, Nesnenin üstüne and Nesnenin altına, an object's draw order
within its layer (docs/adr/0260 §5, 0310 §3).

    python3 scripts/fixtures/order_command_cases.py           # writes the file
    python3 scripts/fixtures/order_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.order.json. The rule is written here
from the ADR, without KentOS code: a layer draws its objects by the key
(order, place in the document); Öne getir gives the objects moved, in the
order they are drawn, the orders after the largest of the layer's other
objects', Arkaya gönder the orders before the smallest; when they are
already the first or the last that the layer draws, or the layer has no
other object, nothing changes. Above (under) gives the objects the orders
right after (before) the reference's; the layer's other objects drawn after
(before) the reference move together by the smallest step that leaves them
room, so every other pair keeps its order; objects already right above
(under) it do not change. The checks, their order, codes, paths and
messages are written by hand from the contract
(crates/shared/commands/src/cad_order.rs).

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import sys

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
I32_MAX = 2**31 - 1
I32_MIN = -(2**31)


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def group(i, name, children, visible=True, locked=False):
    return {"id": i, "name": name, "type": "group", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": children}


def P(x, y):
    return {"x": x, "y": y}


def line(i, layer_id, y, **more):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, "a": P(487000, 4420000 + y), "b": P(487020, 4420000 + y), **more}


ENTITIES = [
    line(1, "yapi", 0),
    {"kind": "polygon", "id": 2, "layerId": "yapi", "attrs": {"Parsel": "7"}, "pts": [P(487030, 4420000), P(487050, 4420000), P(487050, 4420020), P(487030, 4420020)]},
    {"kind": "point", "id": 3, "layerId": "yapi", "attrs": {}, "p": P(487060, 4420000)},
    line(4, "yol", 30),
    line(5, "kilitli", 35),
    line(6, "eski", 40),
    {"kind": "text", "id": 7, "layerId": "yapi", "attrs": {}, "p": P(487000, 4420010), "text": "Not", "height": 2.5, "rotation": 0},
    {"kind": "circle", "id": 8, "layerId": "yol", "attrs": {}, "c": P(487070, 4420030), "r": 3, "order": 2},
    line(9, "yol", 32, order=-1),
    line(10, "gizli", 45),
]
LAYERS = [
    layer("yapi", "Yapı"),
    layer("yol", "Yol"),
    layer("kilitli", "Kilitli katman", locked=True),
    layer("gizli", "Gizli katman", visible=False),
    group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
]


def setup(entities):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Ürün komutu",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "cad", "drawingFont": "barlow"},
        "origin": {"x": 487000, "y": 4420000},
        "layers": LAYERS,
        "activeLayer": "yapi",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


SETUP = setup(ENTITIES)
IDS = [e["id"] for e in ENTITIES]


# ── The rule (docs/adr/0260 §5) ───────────────────────────────────────


class Drawing:
    """The objects' orders, their layers and their places (their place in the
    list: the document's order), as the command reads them."""

    def __init__(self, entities):
        self.entities = [json.loads(json.dumps(e)) for e in entities]

    def by_id(self, i):
        return next(e for e in self.entities if e["id"] == i)

    def key(self, e):
        return (e.get("order", 0), self.entities.index(e))

    def move(self, ids, to):
        """The new orders of `ids` (each once), by the ADR's rule; {} when nothing changes."""
        out = {}
        moved = [self.by_id(i) for i in dict.fromkeys(ids)]
        for layer_id in dict.fromkeys(e["layerId"] for e in moved):
            mine = sorted((e for e in moved if e["layerId"] == layer_id), key=self.key)
            others = [e for e in self.entities if e["layerId"] == layer_id and e not in mine]
            if not others:
                continue
            drawn = sorted(self.entities, key=self.key)
            drawn = [e for e in drawn if e["layerId"] == layer_id]
            if to == "front" and drawn[-len(mine):] == mine or to == "back" and drawn[:len(mine)] == mine:
                continue
            if to == "front":
                start = max(e.get("order", 0) for e in others) + 1
            else:
                start = min(e.get("order", 0) for e in others) - len(mine)
            # An order is a 32-bit integer: past its bounds the layer has no room left.
            if start < I32_MIN or start + len(mine) - 1 > I32_MAX:
                raise OverflowError(layer_id)
            for k, e in enumerate(mine):
                out[e["id"]] = start + k
        return out

    def place(self, ids, to, ref):
        """The new orders of `ids` right above or under `ref`, and of the others moved to make room; {} when nothing changes."""
        moved = [self.by_id(i) for i in dict.fromkeys(ids)]
        r = self.by_id(ref)
        drawn = [e for e in sorted(self.entities, key=self.key) if e["layerId"] == r["layerId"]]
        mine = [e for e in drawn if e in moved]
        n, ri, ro = len(mine), drawn.index(r), r.get("order", 0)
        at = [drawn.index(e) for e in mine]
        if to == "above":
            if at == list(range(ri + 1, ri + 1 + n)):
                return {}
            far = [e for e in drawn[ri + 1:] if e not in moved]
            start, need = ro + 1, ro + n + 1
            step = max(0, need - min((e.get("order", 0) for e in far), default=need))
        else:
            if at == list(range(ri - n, ri)):
                return {}
            far = [e for e in drawn[:ri] if e not in moved]
            start, need = ro - n, ro - n - 1
            step = min(0, need - max((e.get("order", 0) for e in far), default=need))
        news = {e["id"]: start + k for k, e in enumerate(mine)}
        if step:
            news.update({e["id"]: e.get("order", 0) + step for e in far})
        if any(not I32_MIN <= o <= I32_MAX for o in news.values()):
            raise OverflowError(r["layerId"])
        # In the input's order the objects moved, then the others in their drawing order; only what changes.
        first = [i for i in dict.fromkeys(ids) if news[i] != self.by_id(i).get("order", 0)]
        rest = [e["id"] for e in far if e["id"] in news and news[e["id"]] != e.get("order", 0)]
        return {i: news[i] for i in first + rest}

    def apply(self, orders):
        for i, order in orders.items():
            e = self.by_id(i)
            if order == 0:
                e.pop("order", None)
            else:
                e["order"] = order


def E(i, entities=ENTITIES):
    return json.loads(json.dumps(next(e for e in entities if e["id"] == i)))


def with_order(i, order, entities=ENTITIES):
    e = E(i, entities)
    if order == 0:
        e.pop("order", None)
    else:
        e["order"] = order
    return e


def uid(i):
    return f"$uidOf:{i}"


def uids(*slots):
    return [uid(i) for i in slots]


def done(slots, warnings=()):
    return {"status": "completed", "output": {"changed": uids(*slots), "ids": list(slots), "revision": "$current"}, "warnings": list(warnings)}


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def LOCKED_SKIPPED(n):
    """The warning of the objects left out for their locked layer (docs/adr/0278)."""
    return {"code": "layer_locked", "message": f"{n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}


def locked_object(name, i):
    return failed("layer_locked", f"“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{i}]")


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
NO_ENTITIES = "Sırası değişecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin."
UID_MESSAGE = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
MISSING_MESSAGE = f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
STEP = {"front": "Öne getir", "back": "Arkaya gönder", "above": "Nesnenin üstüne", "under": "Nesnenin altına"}
NO_REFERENCE = "Nesnenin üstüne ve altına bir başvuru nesnesi ister; üstüne ya da altına konacak nesnenin kimliğini verin."
NEEDLESS_REFERENCE = "Öne getir ve Arkaya gönder başvuru nesnesi almaz; başvuruyu kaldırın ya da Nesnenin üstüne ya da altına'yı seçin."
REFERENCE_MOVED = "Başvuru nesnesi taşınan nesnelerden biri olamaz; başka bir nesnenin üstüne ya da altına koyun."


def limit(name, to):
    way = "daha öne alınamaz. Öndeki nesneleri arkaya göndererek" if to in ("front", "above") else "daha arkaya gönderilemez. Arkadaki nesneleri öne getirerek"
    return failed("order_limit", f"“{name}” katmanında çizim sırası sınıra geldi; nesne {way} yer açın.", "to")


def conflict():
    return {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}


def moved(ids, to, entities=ENTITIES):
    """The written objects of a move from `entities`, by the rule."""
    d = Drawing(entities)
    orders = d.move(ids, to)
    return {str(i): with_order(i, o, entities) for i, o in orders.items()}, list(orders)


def placed(ids, to, ref, entities=ENTITIES):
    """The written objects of an above or under from `entities`, by the rule, and their ids in the output's order."""
    d = Drawing(entities)
    orders = d.place(ids, to, ref)
    return {str(i): with_order(i, o, entities) for i, o in orders.items()}, list(orders)


def other_layer(a, b, i):
    return failed("not_same_layer", f"Nesne “{a}” katmanında, başvuru “{b}” katmanında; çizim sırası katmanın içindedir. Nesneyi başvurunun katmanına taşıyın ya da katmanların sırasını Katmanlar panelinde değiştirin.", f"uids[{i}]")


cases = []

# ── Writing ────────────────────────────────────────────────────────────

written, changed = moved([2], "front")
assert changed == [2] and written["2"]["order"] == 1
cases.append({
    "name": "Öne getir: nesne katmanının öbür nesnelerinin en büyük sırasından sonra çizilir (1); yuvası ve kimliği kalır; tek adımda geri alınır, yinelenir",
    "steps": [
        {"op": "captureUid", "id": 2, "as": "parsel"},
        {"op": "execute", "input": {"uids": uids(2), "to": "front"}, "result": done([2]),
         "expect": {"ids": IDS, "entities": written, "uids": {"2": "parsel"}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Öne getir", "note": "Değiştir › Sırala ▾ › Öne getir'in adımı.", "expect": {"entities": {"2": E(2)}, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Öne getir", "expect": {"entities": written, "uids": {"2": "parsel"}}},
    ],
})

written, changed = moved([3], "back")
assert written["3"]["order"] == -1
cases.append({
    "name": "Arkaya gönder: nesne katmanının öbür nesnelerinin en küçük sırasından önce çizilir (−1); adım “Arkaya gönder”",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "to": "back"}, "result": done([3]), "expect": {"entities": written, "revision": "changed"}},
        {"op": "undo", "returns": "Arkaya gönder", "expect": {"entities": {"3": E(3)}, "canUndo": False}},
    ],
})

assert moved([7], "front")[1] == [] and moved([1], "back")[1] == []
cases.append({
    "name": "zaten katmanının en önünde ya da en arkasında olan değişmez: çıktıda yoktur, hiçbir şey yazılmaz",
    "note": "7 Yapı'nın son nesnesidir (sırası 0, belgede en sonda); 1 ilkidir.",
    "steps": [
        {"op": "execute", "input": {"uids": uids(7), "to": "front"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "to": "back"}, "result": done([]), "expect": NOTHING},
    ],
})

written, changed = moved([1, 2], "front")
assert [written["1"]["order"], written["2"]["order"]] == [1, 2]
cases.append({
    "name": "birden çok nesne: kendi aralarındaki çizim sırası korunur (önce çizilen küçük sırayı alır)",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1, 2), "to": "front"}, "result": done([1, 2]), "expect": {"entities": written, "revision": "changed"}},
    ],
})

written2, changed2 = moved([2, 1], "front")
assert written2 == written
cases.append({
    "name": "girdinin sırası çizim sırasını değiştirmez; çıktı girdinin sırasıyladır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 1), "to": "front"}, "result": done([2, 1]), "expect": {"entities": written2, "revision": "changed"}},
    ],
})

written, changed = moved([4, 2], "front")
assert written["4"]["order"] == 3 and written["2"]["order"] == 1
cases.append({
    "name": "ayrı katmanlardaki nesneler her biri kendi katmanında öne gelir: Yol'da sırası 2 olan dairenin önüne (3), Yapı'da 1",
    "steps": [
        {"op": "execute", "input": {"uids": uids(4, 2), "to": "front"}, "result": done([4, 2]), "expect": {"entities": written, "revision": "changed"}},
    ],
})

written, changed = moved([8], "back")
assert written["8"]["order"] == -2
cases.append({
    "name": "sıraları verilmiş katmanda Arkaya gönder en küçük sıranın (−1) arkasına koyar (−2)",
    "steps": [
        {"op": "execute", "input": {"uids": uids(8), "to": "back"}, "result": done([8]), "expect": {"entities": written, "revision": "changed"}},
    ],
})

assert moved([8], "front")[1] == [] and moved([9], "back")[1] == []
cases.append({
    "name": "kendi sırasıyla zaten en önde ya da en arkada olan değişmez (Yol'un dairesi 2, çizgisi −1)",
    "steps": [
        {"op": "execute", "input": {"uids": uids(8), "to": "front"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(9), "to": "back"}, "result": done([]), "expect": NOTHING},
    ],
})

assert moved([4, 8, 9], "front")[1] == [] and moved([9, 4, 8], "back")[1] == []
cases.append({
    "name": "katmanın bütün nesneleri taşınırsa öbür nesne olmadığından hiçbir şey değişmez",
    "steps": [
        {"op": "execute", "input": {"uids": uids(4, 8, 9), "to": "front"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(9, 4, 8), "to": "back"}, "result": done([]), "expect": NOTHING},
    ],
})

d = Drawing(ENTITIES)
first = d.move([3], "front")
d.apply(first)
second = d.move([3], "back")
assert first == {3: 1} and second == {3: -1}
cases.append({
    "name": "önce öne, sonra arkaya: her yazma kendi adımıdır; ikincisi öbürlerinin en küçüğüne (0) göre −1",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "to": "front"}, "result": done([3]), "expect": {"entities": {"3": with_order(3, 1)}, "revision": "changed"}},
        {"op": "execute", "input": {"uids": uids(3), "to": "back"}, "result": done([3]), "expect": {"entities": {"3": with_order(3, -1)}, "revision": "changed"}},
        {"op": "undo", "returns": "Arkaya gönder", "expect": {"entities": {"3": with_order(3, 1)}}},
        {"op": "undo", "returns": "Öne getir", "expect": {"entities": {"3": E(3)}, "canUndo": False}},
    ],
})

written, changed = moved([1, 7], "front")
assert written["1"]["order"] == 1 and written["7"]["order"] == 2
cases.append({
    "name": "öndeki nesne arkadakiyle birlikte öne getirilince ikisi de yeni sıra alır; aralarındaki sıra korunur",
    "note": "7 zaten en öndeydi ama 1 değildi: küme en önde değildir.",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1, 7), "to": "front"}, "result": done([1, 7]), "expect": {"entities": written, "revision": "changed"}},
    ],
})

written, changed = moved([2], "front")
cases.append({
    "name": "yinelenen kimlik bir kez sayılır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 2), "to": "front"}, "result": done([2]), "expect": {"entities": written, "revision": "changed"}},
    ],
})

assert moved([10], "front")[1] == []
cases.append({
    "name": "gizli katmanın tek nesnesi: öbür nesne yok, değişmez; uyarı yoktur",
    "steps": [
        {"op": "execute", "input": {"uids": uids(10), "to": "back"}, "result": done([]), "expect": NOTHING},
    ],
})

T_ENTITIES = ENTITIES + [line(11, "gizli", 46)]
written, changed = moved([10], "front", T_ENTITIES)
assert written["10"]["order"] == 1
cases.append({
    "name": "gizli katmandaki nesnenin de sırası değişir, uyarısız",
    "setup": setup(T_ENTITIES),
    "steps": [
        {"op": "execute", "input": {"uids": uids(10), "to": "front"}, "result": done([10]), "expect": {"entities": written, "revision": "changed"}},
    ],
})

L_ENTITIES = ENTITIES + [line(11, "yol", 34, order=I32_MAX), line(12, "yapi", 36, order=I32_MIN)]
for ids, to, where in (([4], "front", "yol"), ([1, 2], "back", "yapi")):
    try:
        moved(ids, to, L_ENTITIES)
        raise AssertionError("the bound is not reached")
    except OverflowError as e:
        assert str(e) == where
cases.append({
    "name": "katmanın sırası sınırdaysa (2³¹ − 1 ya da −2³¹) daha öne ya da arkaya gidilmez: order_limit; hiçbir şey yazılmaz",
    "setup": setup(L_ENTITIES),
    "steps": [
        {"op": "execute", "input": {"uids": uids(4), "to": "front"}, "result": limit("Yol", "front"), "expect": {**NOTHING, "ids": [e["id"] for e in L_ENTITIES]}},
        {"op": "execute", "input": {"uids": uids(1, 2), "to": "back"}, "result": limit("Yapı", "back"), "expect": {**NOTHING, "ids": [e["id"] for e in L_ENTITIES]}},
    ],
})

# ── Above and under (docs/adr/0310 §3) ────────────────────────────────

written, changed = placed([1], "above", 2)
assert changed == [1, 3, 7] and written["1"]["order"] == 1 and written["3"]["order"] == 2 and written["7"]["order"] == 2
cases.append({
    "name": "Nesnenin üstüne: nesne başvurunun hemen üstünde çizilir (1); başvurudan sonra çizilenlere yer açılır, hepsi aynı adımla (2) kayar, aralarındaki sıra korunur; tek adım, geri alınır",
    "note": "Yapı 1, 2, 3, 7 diye çizilir (hepsinin sırası 0). 1, 2'nin üstüne: 2, 1, 3, 7.",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "to": "above", "reference": uid(2)}, "result": done(changed),
         "expect": {"entities": {**written, "2": E(2)}, "canUndo": True, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Nesnenin üstüne", "expect": {"entities": {"1": E(1), "3": E(3), "7": E(7)}, "canUndo": False}},
    ],
})

written, changed = placed([7], "under", 2)
assert changed == [7, 1] and written["7"]["order"] == -1 and written["1"]["order"] == -2
cases.append({
    "name": "Nesnenin altına: nesne başvurunun hemen altında çizilir (−1); başvurudan önce çizilenler yer açmak için arkaya kayar (−2)",
    "note": "7, 2'nin altına: 1, 7, 2, 3.",
    "steps": [
        {"op": "execute", "input": {"uids": uids(7), "to": "under", "reference": uid(2)}, "result": done(changed),
         "expect": {"entities": {**written, "2": E(2), "3": E(3)}, "revision": "changed"}},
        {"op": "undo", "returns": "Nesnenin altına", "expect": {"entities": {"1": E(1), "7": E(7)}}},
    ],
})

written, changed = placed([9], "above", 4)
assert changed == [9] and written["9"]["order"] == 1
cases.append({
    "name": "yer varken öbürleri kaymaz: Yol'da 9 (−1), 4'ün (0) üstüne 1 alır; dairenin sırası (2) zaten büyük",
    "steps": [
        {"op": "execute", "input": {"uids": uids(9), "to": "above", "reference": uid(4)}, "result": done(changed),
         "expect": {"entities": {**written, "4": E(4), "8": E(8)}, "revision": "changed"}},
    ],
})

assert placed([2], "above", 1)[1] == [] and placed([1], "under", 2)[1] == []
cases.append({
    "name": "zaten başvurunun hemen üstünde ya da altında olan değişmez; hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2), "to": "above", "reference": uid(1)}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "to": "under", "reference": uid(2)}, "result": done([]), "expect": NOTHING},
    ],
})

written, changed = placed([3, 1], "above", 7)
assert changed == [3, 1] and written["1"]["order"] == 1 and written["3"]["order"] == 2
cases.append({
    "name": "birden çok nesne başvurunun üstüne kendi aralarındaki çizim sırasıyla gelir; en öndeki başvurunun üstü öne getirmekle aynıdır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3, 1), "to": "above", "reference": uid(7)}, "result": done(changed),
         "expect": {"entities": written, "revision": "changed"}},
    ],
})

written, changed = placed([2], "above", 3)
cases.append({
    "name": "kilitli katmandaki nesne atlanır ve sayılır; öbürü başvurunun üstüne gider",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 5), "to": "above", "reference": uid(3)}, "result": done(changed, [LOCKED_SKIPPED(1)]),
         "expect": {"entities": {**written, "5": E(5)}, "revision": "changed"}},
    ],
})

written, changed = placed([2], "under", 1)
assert changed == [2] and written["2"]["order"] == -1, "nothing is before the reference: no room to make"
cases.append({
    "name": "plan yazılacakları (taşınanlar, sonra yer açmak için kayanlar) yuvalarıyla gösterir; doğrulama yazmaz",
    "steps": [
        {"op": "validate", "input": {"uids": uids(2), "to": "under", "reference": uid(1)}, "result": {"status": "completed", "output": None, "warnings": []}, "expect": NOTHING},
        {"op": "plan", "input": {"uids": uids(2), "to": "under", "reference": uid(1)},
         "result": {"status": "completed", "output": {"changed": [written[str(i)] for i in changed], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(2), "to": "under", "reference": uid(1)}, "result": done(changed), "expect": {"entities": written, "revision": "changed"}},
    ],
})

M_ENTITIES = ENTITIES + [line(11, "yol", 34, order=I32_MAX)]
try:
    placed([9], "above", 11, M_ENTITIES)
    raise AssertionError("the bound is not reached")
except OverflowError as e:
    assert str(e) == "yol"
cases.append({
    "name": "başvurunun sırası sınırdaysa üstüne yer yoktur: order_limit; hiçbir şey yazılmaz",
    "setup": setup(M_ENTITIES),
    "steps": [
        {"op": "execute", "input": {"uids": uids(9), "to": "above", "reference": uid(11)}, "result": limit("Yol", "above"), "expect": {**NOTHING, "ids": [e["id"] for e in M_ENTITIES]}},
    ],
})

cases.append({
    "name": "başvurunun kuralı: üstüne ve altına başvuru ister, öne ve arkaya istemez, başvuru taşınanlardan biri olamaz: invalid_reference; biçimi bozuk başvuru invalid_uid",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "to": "above"}, "result": failed("invalid_reference", NO_REFERENCE, "reference"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "to": "front", "reference": uid(2)}, "result": failed("invalid_reference", NEEDLESS_REFERENCE, "reference"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1, 2), "to": "under", "reference": uid(2)}, "result": failed("invalid_reference", REFERENCE_MOVED, "reference"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "to": "under", "reference": "duvar"},
         "result": failed("invalid_uid", "Başvuru “duvar” kalıcı kimlik değil: küçük harfli, tireli UUID metni verin.", "reference"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "çizimde olmayan başvuru: entity_not_found (yeri reference); başka katmandaki nesne: not_same_layer; hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "to": "above", "reference": MISSING},
         "result": failed("entity_not_found", f"Başvuru “{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "reference"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1, 4), "to": "above", "reference": uid(2)}, "result": other_layer("Yol", "Yapı", 1), "expect": NOTHING},
    ],
})

# ── Refusals ───────────────────────────────────────────────────────────

cases.append({
    "name": "nesne verilmezse: no_entities",
    "steps": [
        {"op": "execute", "input": {"uids": [], "to": "front"}, "result": failed("no_entities", NO_ENTITIES, "uids"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "geçersiz kimlik: invalid_uid, yeri sırasıyla",
    "steps": [
        {"op": "execute", "input": {"uids": [uid(1), "duvar"], "to": "front"}, "result": failed("invalid_uid", UID_MESSAGE.format("duvar"), "uids[1]"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "çizimde olmayan kimlik: entity_not_found; hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [uid(2), MISSING], "to": "front"}, "result": failed("entity_not_found", MISSING_MESSAGE, "uids[1]"),
         "expect": {**NOTHING, "entities": {"2": E(2)}}},
    ],
})

written, changed = moved([2], "front")
cases.append({
    "name": "kilitli katmandaki nesnenin sırası değişmez: öbürleri yazılır, kilitliler sayıyla uyarılır (ADR 0278); hepsi kilitliyse layer_locked; kilitli grubun katmanı da kilitlidir",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 5), "to": "front"}, "result": done(changed, [LOCKED_SKIPPED(1)]),
         "expect": {"entities": {**written, "5": E(5)}, "canUndo": True, "dirty": True, "revision": "changed"}},
        {"op": "execute", "input": {"uids": uids(6), "to": "back"}, "result": locked_object("Eski", 0),
         "expect": {"entities": {"6": E(6)}, "revision": "same"}},
    ],
})

cases.append({
    "name": "beklenen sürüm ondalık bir tamsayı yazısıdır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2), "to": "front", "expectedRevision": "07"},
         "result": failed("invalid_revision", "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; çakışma nesneden ve kilitten önce",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": {"uids": uids(3), "to": "front"}, "result": done([3])},
        {"op": "execute", "input": {"uids": uids(2), "to": "front", "expectedRevision": "$r0"}, "result": conflict(), "expect": {"entities": {"2": E(2)}, "revision": "same"}},
        {"op": "execute", "input": {"uids": [uid(5), MISSING], "to": "back", "expectedRevision": "$r0"}, "result": conflict(), "expect": {"revision": "same"}},
    ],
})

cases.append({
    "name": "girdinin hatası çizimin durumundan önce gelir: kimlikler sürümden önce denetlenir",
    "steps": [
        {"op": "execute", "input": {"uids": [], "to": "back", "expectedRevision": "999"}, "result": failed("no_entities", NO_ENTITIES, "uids"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": ["X"], "to": "back", "expectedRevision": "999"}, "result": failed("invalid_uid", UID_MESSAGE.format("X"), "uids[0]"), "expect": NOTHING},
    ],
})

written, changed = moved([1, 2], "front")
cases.append({
    "name": "doğrulama hiçbir şey yazmaz; plan yazılacak nesneleri yuvalarıyla gösterir; yazma planın sürümüyle planı yazar",
    "steps": [
        {"op": "validate", "input": {"uids": uids(1, 2), "to": "front"}, "result": {"status": "completed", "output": None, "warnings": []}, "expect": NOTHING},
        {"op": "plan", "input": {"uids": uids(2, 1), "to": "front"},
         "result": {"status": "completed", "output": {"changed": [written["2"], written["1"]], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
        {"op": "captureRevision", "as": "plan"},
        {"op": "execute", "input": {"uids": uids(2, 1), "to": "front", "expectedRevision": "$plan"}, "result": done([2, 1]), "expect": {"entities": written, "revision": "changed"}},
    ],
})

cases.append({
    "name": "değişmeyecek nesnenin planı boştur; doğrulama ve plan geçmişe dokunmaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "to": "front"}, "result": done([3])},
        {"op": "undo", "returns": "Öne getir", "expect": {"canUndo": False, "canRedo": True}},
        {"op": "validate", "input": {"uids": uids(7), "to": "front"}, "result": {"status": "completed", "output": None, "warnings": []}},
        {"op": "plan", "input": {"uids": uids(7), "to": "front"}, "result": {"status": "completed", "output": {"changed": [], "revision": "$current"}, "warnings": []},
         "expect": {"canUndo": False, "canRedo": True, "revision": "same"}},
        {"op": "redo", "returns": "Öne getir", "expect": {"entities": {"3": with_order(3, 1)}}},
    ],
})


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def write(command, title, note, cases):
    out = ["{"]
    head = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note}
    for key, value in head.items():
        out.append(f'  "{key}": {compact(value)},')
    s = SETUP
    out.append('  "setup": {')
    for key in ["format", "version", "name", "settings", "origin"]:
        out.append(f'    "{key}": {compact(s[key])},')
    out.append('    "layers": [')
    out.append(",\n".join(f"      {compact(item)}" for item in s["layers"]))
    out.append("    ],")
    out.append(f'    "activeLayer": {compact(s["activeLayer"])},')
    out.append('    "entities": [')
    out.append(",\n".join(f"      {compact(e)}" for e in s["entities"]))
    out.append("    ],")
    out.append(f'    "styles": {compact(s["styles"])}')
    out.append("  },")
    out.append('  "cases": [')
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        if "setup" in c:
            lines.append(f'      "setup": {compact(c["setup"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st)}" for st in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    text = "\n".join(out) + "\n"
    path = f"fixtures/commands/v1/{command}.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


write(
    "cad.entities.order",
    "Öne getir, Arkaya gönder, Nesnenin üstüne ve altına",
    "ADR 0260 §5, 0310 §3: bir katmanın nesneleri (order, belgedeki yer) anahtarıyla çizilir. Öne getir taşınanlara, çizildikleri sırayla, "
    "katmanın öbür nesnelerinin en büyük sırasından sonraki sıraları, Arkaya gönder en küçüğünden önceki sıraları verir; zaten en önde ya "
    "da en arkadaysalar, ya da katmanın öbür nesnesi yoksa bir şey değişmez. Nesnenin üstüne (altına) başvurunun hemen sonraki (önceki) "
    "sıralarını verir; başvurunun öbür yanındakiler yer kalmayınca aynı adımla kayar. Kural ve beklenenler "
    "scripts/fixtures/order_command_cases.py'de KentOS kodu olmadan yazılmıştır.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
