"""The shared cases of the product command cad.entities.transform (docs/adr/0037, 0047).

    python3 scripts/fixtures/transform_command_cases.py           # writes the file
    python3 scripts/fixtures/transform_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.transform.json. The checks, their
order, codes, paths and messages are written here by hand from the ADR. The
expected geometry is computed independently (affine_reference.py, beside
this file), from the definitions of the transforms (a displacement; a
rotation, a scale and a reflection as 2D affine maps x' = a*x + c*y + e,
y' = b*x + d*y + f; an alignment as the maps composed: scale about the
first source point, turn about it, carry it onto the first target), in IEEE
double arithmetic in the same order of operations, with Python's own
math.cos/sin/atan2/hypot. Nothing is copied from an implementation's
output; the web's and the desktop's handlers must meet these values bit
for bit.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import alignment, assert_no_negative_zero, mirror, moved, rotation, scaling, translation  # noqa: E402  (the maps, beside this file)
from numeric_display import shown  # noqa: E402  (the display rule, docs/adr/0149)
import sheet_f64  # noqa: E402  (Kauçuk levha's sheet in the core's arithmetic, docs/adr/0158)

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def group(i, name, children, visible=True, locked=False):
    return {"id": i, "name": name, "type": "group", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": children}


def P(x, y):
    return {"x": x, "y": y}


HALF_PI = math.pi / 2

ENTITIES = [
    {"kind": "point", "id": 4, "layerId": "yapi", "attrs": {"Ad": "N1"}, "label": "N1", "p": P(487001, 4420001), "z": 12.5},
    {"kind": "polygon", "id": 9, "layerId": "sinir", "attrs": {"Ada": "101"}, "pts": [P(487000, 4420000), P(487040, 4420000), P(487040, 4420030)], "bulges": [0.25, 0.5, -0.25]},
    {"kind": "line", "id": 10, "layerId": "yapi", "attrs": {}, "color": "#E5484D", "a": P(487010, 4420010), "b": P(487020, 4420010)},
    {"kind": "arc", "id": 11, "layerId": "yapi", "attrs": {}, "c": P(487020, 4420020), "r": 4, "a0": 0, "a1": HALF_PI},
    {"kind": "line", "id": 12, "layerId": "kilitli", "attrs": {}, "a": P(487005, 4420005), "b": P(487015, 4420005)},
    {"kind": "circle", "id": 13, "layerId": "yapi", "attrs": {}, "c": P(487030, 4420010), "r": 2.5},
    {"kind": "text", "id": 14, "layerId": "yapi", "attrs": {}, "p": P(487005, 4420025), "text": "Ada 101", "height": 2, "rotation": 0},
    {"kind": "line", "id": 15, "layerId": "eski", "attrs": {}, "a": P(487005, 4420010), "b": P(487015, 4420010)},
    {"kind": "ellipse", "id": 16, "layerId": "yapi", "attrs": {}, "c": P(487050, 4420050), "major": P(8, 0), "ratio": 0.5, "t0": 0.5, "t1": 1.5},
    {"kind": "dimension", "id": 17, "layerId": "yapi", "attrs": {}, "a": P(487000, 4420040), "b": P(487010, 4420040), "offset": 2, "height": 0.5},
    {"kind": "spline", "id": 18, "layerId": "yapi", "attrs": {}, "pts": [P(487060, 4420000), P(487064, 4420004), P(487068, 4420000)], "closed": False},
    {"kind": "leader", "id": 19, "layerId": "yapi", "attrs": {}, "pts": [P(487070, 4420010), P(487074, 4420014), P(487080, 4420014)], "text": "Ø150 PVC",
     "height": 2, "rotation": 15, "arrow": "open", "mask": True},
    {"kind": "line", "id": 20, "layerId": "gizli", "attrs": {}, "a": P(487005, 4420015), "b": P(487015, 4420015)},
    {"kind": "point", "id": 21, "layerId": "notlar", "attrs": {"Ad": "N2"}, "p": P(487002, 4420002)},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Ürün komutu",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "hybrid", "drawingFont": "barlow"},
    "origin": {"x": 487000, "y": 4420000},
    "layers": [
        layer("yapi", "Yapı"),
        layer("kilitli", "Kilitli katman", locked=True),
        layer("gizli", "Gizli katman", visible=False),
        group("pafta", "Pafta", [layer("sinir", "Sınır")]),
        group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
        group("taslak", "Taslak", [layer("notlar", "Notlar")], visible=False),
    ],
    "activeLayer": "yapi",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# ── Answers ──────────────────────────────────────────────────────────────

AXIS = {"x": "doğu (Y)", "y": "kuzey (X)"}


def NFC(whose, axis):
    return f"{whose} {AXIS[axis]} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def NFV(what, fix):
    return f"{what} sonlu bir sayı değil (NaN ya da sonsuz). {fix}"


CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
NOTHING = "Dönüştürülecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin."
FACTOR = "Ölçek faktörü sıfırdan büyük olmalı. Pozitif bir faktör verin."
AXIS_SAME = "Simetri ekseninin iki noktası aynı; eksenin yönü yok. Birbirinden ayrı iki nokta verin."
OVERFLOW = "Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin."
MOVE_FIX = "Kaydırmayı sonlu bir sayıyla verin."


def BADREV(r):
    return f"Beklenen sürüm “{r}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."


def BADUID(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."


def NOTFOUND(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def LOCKED(n):
    return f"{n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."


def U(i):
    return f"$uidOf:{i}"


def done(changed=(), created=(), locked=(), warnings=()):
    return {"status": "completed", "output": {"changed": list(changed), "created": list(created), "locked": list(locked), "revision": "$current"}, "warnings": list(warnings)}


def valid(warnings=()):
    return {"status": "completed", "output": None, "warnings": list(warnings)}


def planned(sources, entities, locked=(), warnings=()):
    return {"status": "completed", "output": {"sources": list(sources), "entities": list(entities), "locked": list(locked), "revision": "$current"}, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def conflict():
    return {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}


def locked_warning(n):
    return {"code": "layer_locked", "message": LOCKED(n), "path": "uids"}


untouched = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def move(dx, dy):
    return {"kind": "move", "dx": dx, "dy": dy}


def rotate(cx, cy, angle):
    return {"kind": "rotate", "center": P(cx, cy), "angle": angle}


def scale(cx, cy, factor):
    return {"kind": "scale", "center": P(cx, cy), "factor": factor}


def mirror_t(ax, ay, bx, by):
    return {"kind": "mirror", "a": P(ax, ay), "b": P(bx, by)}


def align_t(s, t, s2=None, t2=None, scale=None):
    out = {"kind": "align", "source": P(*s), "target": P(*t)}
    if s2 is not None:
        out["source2"] = P(*s2)
    if t2 is not None:
        out["target2"] = P(*t2)
    if scale is not None:
        out["scale"] = scale
    return out


def entities(*pairs):
    return {str(i): e for i, e in pairs}


ORIG = lambda i: BY_ID[i]

M_MOVE = translation(12.5, -7.25)
M_UP = translation(0, 20)
O10 = (487010, 4420010)
M_ROT = rotation(HALF_PI, O10)
M_HALF = rotation(math.pi, O10)
M_SCALE2 = scaling(2, (487000, 4420000))
M_SCALE_HALF = scaling(0.5, (487030, 4420010))
AX = (487025, 4420000, 487025, 4420010)
M_MIRROR = mirror((AX[0], AX[1]), (AX[2], AX[3]))
# Hizala: the line's start onto a point 30 m east, 20 m north; its direction (east) onto north.
S1, D1 = (487010, 4420010), (487040, 4420030)
S2, D2 = (487020, 4420010), (487040, 4420050)
M_ALIGN_MOVE = alignment(S1, D1)
M_ALIGN = alignment(S1, D1, S2, D2)
M_ALIGN_SCALED = alignment(S1, D1, S2, D2, scale=True)
# Within a nanometre of D1 (the nearest float64 to 487040 + 5e-10 lies about 5.2e-10 m away).
D1_NEAR = (487040 + 5e-10, 4420030)
assert 0 < D1_NEAR[0] - D1[0] < 1e-9
NEXT = 22

cases = []

cases.append({
    "name": "yerinde taşınır: yuvası, kalıcı kimliği ve öbür alanları kalır; tek adımda geri alınır, yinelenir",
    "steps": [
        {"op": "captureUid", "id": 10, "as": "cizgi"},
        {"op": "execute", "input": {"uids": [U(10)], "transform": move(12.5, -7.25)}, "result": done(changed=[U(10)]),
         "expect": {"ids": IDS, "entities": entities((10, moved(ORIG(10), M_MOVE))), "uids": {"10": "cizgi"}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Taşı", "note": "Taşı aracının hep yazdığı adım adı.", "expect": {"entities": entities((10, ORIG(10))), "uids": {"10": "cizgi"}, "canUndo": False, "canRedo": True, "revision": "changed"}},
        {"op": "redo", "returns": "Taşı", "expect": {"entities": entities((10, moved(ORIG(10), M_MOVE))), "canUndo": True, "canRedo": False, "revision": "changed"}},
    ],
})

cases.append({
    "name": "kopya olarak taşınır (Kopyala): kopya bütün alanları ve yeni bir kalıcı kimlik alır, asıl yerinde kalır; adım Kopyala",
    "steps": [
        {"op": "captureUid", "id": 10, "as": "cizgi"},
        {"op": "execute", "input": {"uids": [U(10)], "transform": move(0, 20), "copy": True}, "result": done(created=[U(NEXT)]),
         "expect": {"ids": IDS + [NEXT], "entities": entities((10, ORIG(10)), (NEXT, moved(ORIG(10), M_UP, NEXT))), "uids": {"10": "cizgi", str(NEXT): "new"}, "canUndo": True, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Kopyala", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}},
    ],
})

cases.append({
    "name": "birden çok nesne ve tekrarlanan kimlik: her nesne bir kez, girdinin sırasıyla; yayın açıları ve noktanın kotu, yazısı, öznitelikleri kalır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(11), U(4), U(11)], "transform": move(12.5, -7.25)}, "result": done(changed=[U(11), U(4)]),
         "expect": {"ids": IDS, "entities": entities((11, moved(ORIG(11), M_MOVE)), (4, moved(ORIG(4), M_MOVE))), "revision": "changed"}},
        {"op": "undo", "returns": "Taşı", "note": "İki nesne tek adımda geri gelir.", "expect": {"entities": entities((11, ORIG(11)), (4, ORIG(4))), "canUndo": False}},
    ],
})

cases.append({
    "name": "döndürme: çizgi başlangıcı etrafında saat yönünün tersine 90°; adım Döndür",
    "note": "Beklenen koordinatlar döndürme tanımından (cos, sin, aynı işlem sırası) çift duyarlıkla hesaplandı.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": rotate(487010, 4420010, HALF_PI)}, "result": done(changed=[U(10)]),
         "expect": {"entities": entities((10, moved(ORIG(10), M_ROT))), "revision": "changed"}},
        {"op": "undo", "returns": "Döndür", "expect": {"entities": entities((10, ORIG(10)))}},
    ],
})

cases.append({
    "name": "döndürülmüş kopya (Kopya): asıl yerinde kalır, kopya 180° döner; adım yine Döndür",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": rotate(487010, 4420010, math.pi), "copy": True}, "result": done(created=[U(NEXT)]),
         "expect": {"ids": IDS + [NEXT], "entities": entities((10, ORIG(10)), (NEXT, moved(ORIG(10), M_HALF, NEXT))), "uids": {str(NEXT): "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Döndür", "expect": {"ids": IDS}},
    ],
})

cases.append({
    "name": "ölçekleme: daire, yazı ve eğri iki katına; yarıçap ve yazı yüksekliği de; adım Ölçekle",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13), U(14), U(18)], "transform": scale(487000, 4420000, 2)}, "result": done(changed=[U(13), U(14), U(18)]),
         "expect": {"entities": entities((13, moved(ORIG(13), M_SCALE2)), (14, moved(ORIG(14), M_SCALE2)), (18, moved(ORIG(18), M_SCALE2))), "revision": "changed"}},
        {"op": "undo", "returns": "Ölçekle", "expect": {"entities": entities((13, ORIG(13)), (14, ORIG(14)), (18, ORIG(18)))}},
    ],
})

cases.append({
    "name": "ölçeklenmiş kopya: merkezinde yarı yarıçaplı bir daire eklenir",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "transform": scale(487030, 4420010, 0.5), "copy": True}, "result": done(created=[U(NEXT)]),
         "expect": {"ids": IDS + [NEXT], "entities": entities((13, ORIG(13)), (NEXT, moved(ORIG(13), M_SCALE_HALF, NEXT))), "revision": "changed"}},
        {"op": "undo", "returns": "Ölçekle", "expect": {"ids": IDS}},
    ],
})

cases.append({
    "name": "aynalama: yay saat yönünün tersine kalır, kapalı alanın yay değerleri döner; adım Aynala",
    "note": "Düşey eksen (doğu 487025): yayın çeyreği batıya döner, a0 π/2, a1 π olur.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(11), U(9)], "transform": mirror_t(*AX)}, "result": done(changed=[U(11), U(9)]),
         "expect": {"entities": entities((11, moved(ORIG(11), M_MIRROR)), (9, moved(ORIG(9), M_MIRROR))), "revision": "changed"}},
        {"op": "undo", "returns": "Aynala", "expect": {"entities": entities((11, ORIG(11)), (9, ORIG(9)))}},
    ],
})

cases.append({
    "name": "simetrik kopya (kaynağı korur): yazı okunur kalır; adım Aynala",
    "steps": [
        {"op": "execute", "input": {"uids": [U(14)], "transform": mirror_t(*AX), "copy": True}, "result": done(created=[U(NEXT)]),
         "expect": {"ids": IDS + [NEXT], "entities": entities((14, ORIG(14)), (NEXT, moved(ORIG(14), M_MIRROR, NEXT))), "revision": "changed"}},
        {"op": "undo", "returns": "Aynala", "expect": {"ids": IDS}},
    ],
})

cases.append({
    "name": "her tür aynı kuralla: elips yayı, ölçü ve eğrinin simetriği",
    "note": "Elipsin parametreleri t → −t döner; hizalı ölçünün uzaklığı işaret değiştirir.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(16), U(17), U(18)], "transform": mirror_t(*AX)}, "result": done(changed=[U(16), U(17), U(18)]),
         "expect": {"entities": entities((16, moved(ORIG(16), M_MIRROR)), (17, moved(ORIG(17), M_MIRROR)), (18, moved(ORIG(18), M_MIRROR))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "hizalama (Hizala), tek çiftle: birinci kaynak noktası birinci hedefe taşınır; adım Hizala",
    "steps": [
        {"op": "captureUid", "id": 10, "as": "cizgi"},
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1)}, "result": done(changed=[U(10)]),
         "expect": {"entities": entities((10, moved(ORIG(10), M_ALIGN_MOVE))), "uids": {"10": "cizgi"}, "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Hizala", "note": "Hizala aracının adımı.", "expect": {"entities": entities((10, ORIG(10))), "canUndo": False, "canRedo": True}},
    ],
})

cases.append({
    "name": "iki çiftle hizalama: nesneler birinci kaynak noktası etrafında döner, kaynak doğrultusu hedef doğrultusuna oturur; boy değişmez",
    "note": "Kaynak doğrultusu doğu, hedef doğrultusu kuzey: çeyrek tur. Yay, yazı ve kapalı alanın yay değerleri aynı kuralla döner.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10), U(11), U(14), U(9)], "transform": align_t(S1, D1, S2, D2)}, "result": done(changed=[U(10), U(11), U(14), U(9)]),
         "expect": {"entities": entities((10, moved(ORIG(10), M_ALIGN)), (11, moved(ORIG(11), M_ALIGN)), (14, moved(ORIG(14), M_ALIGN)), (9, moved(ORIG(9), M_ALIGN))), "revision": "changed"}},
        {"op": "undo", "returns": "Hizala", "expect": {"entities": entities((10, ORIG(10)), (11, ORIG(11)), (14, ORIG(14)), (9, ORIG(9))), "canUndo": False}},
    ],
})

cases.append({
    "name": "scale ile boy da eşitlenir: 10 m'lik kaynak doğrultusu 20 m'lik hedefe; yarıçap ve yazı yüksekliği iki katına",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13), U(14), U(10)], "transform": align_t(S1, D1, S2, D2, scale=True)}, "result": done(changed=[U(13), U(14), U(10)]),
         "expect": {"entities": entities((13, moved(ORIG(13), M_ALIGN_SCALED)), (14, moved(ORIG(14), M_ALIGN_SCALED)), (10, moved(ORIG(10), M_ALIGN_SCALED))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "scale ikinci çift olmadan bir şey değiştirmez; hizalanmış kopya da yapılabilir, adım yine Hizala",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "transform": align_t(S1, D1, scale=True), "copy": True}, "result": done(created=[U(NEXT)]),
         "expect": {"ids": IDS + [NEXT], "entities": entities((13, ORIG(13)), (NEXT, moved(ORIG(13), M_ALIGN_MOVE, NEXT))), "uids": {str(NEXT): "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Hizala", "expect": {"ids": IDS}},
    ],
})

cases.append({
    "name": "kilitli katmandaki nesne hizalanmaz; öbürleri uyarıyla hizalanır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(12), U(10)], "transform": align_t(S1, D1, S2, D2)}, "result": done(changed=[U(10)], locked=[U(12)], warnings=[locked_warning(1)]),
         "expect": {"entities": entities((10, moved(ORIG(10), M_ALIGN)), (12, ORIG(12))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "ikinci çift yarım verilemez: invalid_align; eksik nokta söylenir",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1, s2=S2)},
         "result": failed("invalid_align", "Hizalamanın ikinci hedef noktası verilmedi; ikinci çift iki noktayla verilir. İkinci hedef noktasını verin ya da ikinci kaynak noktasını çıkarın.", "transform.target2"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1, t2=D2, scale=True)},
         "result": failed("invalid_align", "Hizalamanın ikinci kaynak noktası verilmedi; ikinci çift iki noktayla verilir. İkinci kaynak noktasını verin ya da ikinci hedef noktasını çıkarın.", "transform.source2"), "expect": untouched},
    ],
})

cases.append({
    "name": "ikinci çiftin noktası birincinin bir nanometre yakınındaysa doğrultu yoktur: invalid_align",
    "note": "Hedef noktası 487040'tan yaklaşık 5,2e-10 m uzakta: çekirdeğin ölçüsüyle aynı nokta.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1, S1, D2)},
         "result": failed("invalid_align", "Kaynak noktaları çakışıyor; kaynak doğrultusunun yönü yok. Birbirinden ayrı iki kaynak noktası verin.", "transform.source2"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1, S2, D1_NEAR, scale=True)},
         "result": failed("invalid_align", "Hedef noktaları çakışıyor; hedef doğrultusunun yönü yok. Birbirinden ayrı iki hedef noktası verin.", "transform.target2"), "expect": untouched},
    ],
})

cases.append({
    "name": "hizalamanın noktaları sırayla denetlenir: önce sonlu olmayan değer, sonra ikinci çift",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1, S2, D2)}, "nonFinite": {"transform.target.y": "NaN", "transform.source2.x": "Infinity"},
         "result": failed("not_finite", NFC("Birinci hedef noktasının", "y"), "transform.target.y"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1, s2=S2)}, "nonFinite": {"transform.source2.y": "-Infinity"},
         "result": failed("not_finite", NFC("İkinci kaynak noktasının", "y"), "transform.source2.y"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": align_t(S1, D1, S2, D2)}, "nonFinite": {"transform.target2.x": "NaN"},
         "result": failed("not_finite", NFC("İkinci hedef noktasının", "x"), "transform.target2.x"), "expect": untouched},
    ],
})

cases.append({
    "name": "hizalamanın planı yazılacak nesneleri gösterir; hiçbir şey yazmaz",
    "steps": [
        {"op": "plan", "input": {"uids": [U(11)], "transform": align_t(S1, D1, S2, D2, scale=True)}, "result": planned([U(11)], [moved(ORIG(11), M_ALIGN_SCALED)]), "expect": untouched},
    ],
})

cases.append({
    "name": "kilitli katmandaki nesne yerinde kalır; öbürleri uyarıyla taşınır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10), U(12)], "transform": move(12.5, -7.25)}, "result": done(changed=[U(10)], locked=[U(12)], warnings=[locked_warning(1)]),
         "expect": {"entities": entities((10, moved(ORIG(10), M_MOVE)), (12, ORIG(12))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "kilitli grubun katmanı da kilitlidir; kilitli nesnenin kopyası da yapılmaz",
    "note": "Web'in araçları eskiden kilitli nesnenin kopyasını kilitli katmana ekliyordu; ADR 0037 bu farkı kaydeder.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(15), U(10)], "transform": move(0, 20), "copy": True}, "result": done(created=[U(NEXT)], locked=[U(15)], warnings=[locked_warning(1)]),
         "expect": {"ids": IDS + [NEXT], "entities": entities((NEXT, moved(ORIG(10), M_UP, NEXT)), (15, ORIG(15))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "hepsi kilitliyse hiçbir şey yazılmaz: layer_locked; kopya da yapılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [U(12), U(15)], "transform": move(12.5, -7.25)}, "result": failed("layer_locked", LOCKED(2), "uids"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(12)], "transform": mirror_t(*AX), "copy": True}, "result": failed("layer_locked", LOCKED(1), "uids"), "expect": untouched},
    ],
})

cases.append({
    "name": "gizli katmandaki ve gizli grubun katmanındaki nesne de dönüştürülür; uyarı yok",
    "steps": [
        {"op": "execute", "input": {"uids": [U(20), U(21)], "transform": move(12.5, -7.25)}, "result": done(changed=[U(20), U(21)]),
         "expect": {"entities": entities((20, moved(ORIG(20), M_MOVE)), (21, moved(ORIG(21), M_MOVE))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "nesne verilmedi: no_entities",
    "steps": [
        {"op": "execute", "input": {"uids": [], "transform": move(1, 1)}, "result": failed("no_entities", NOTHING, "uids"), "expect": untouched},
        {"op": "validate", "input": {"uids": [], "transform": scale(0, 0, 2)}, "result": failed("no_entities", NOTHING, "uids"), "expect": untouched},
    ],
})

cases.append({
    "name": "kimlik küçük harfli, tireli bir UUID yazısıdır; ilk bozuk kimlik söylenir",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10), "ABC", "01925F3E-7C1A-7D2B-9E4F-0A1B2C3D4E5F"], "transform": move(1, 1)}, "result": failed("invalid_uid", BADUID("ABC"), "uids[1]"), "expect": untouched},
        {"op": "execute", "input": {"uids": ["01925F3E-7C1A-7D2B-9E4F-0A1B2C3D4E5F"], "transform": move(1, 1)}, "result": failed("invalid_uid", BADUID("01925F3E-7C1A-7D2B-9E4F-0A1B2C3D4E5F"), "uids[0]"), "expect": untouched},
    ],
})

MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
cases.append({
    "name": "çizimde olmayan kimlik: entity_not_found, hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10), MISSING], "transform": move(1, 1)}, "result": failed("entity_not_found", NOTFOUND(MISSING), "uids[1]"), "expect": untouched},
    ],
})

cases.append({
    "name": "NaN ya da sonsuz kaydırma yazılmaz; yol ve ileti hangi değer olduğunu söyler",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": move(1, 1)}, "nonFinite": {"transform.dy": "NaN"},
         "result": failed("not_finite", NFV("Kuzey (X) yönündeki kaydırma", MOVE_FIX), "transform.dy"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": move(1, 1)}, "nonFinite": {"transform.dx": "Infinity", "transform.dy": "NaN"},
         "result": failed("not_finite", NFV("Doğu (Y) yönündeki kaydırma", MOVE_FIX), "transform.dx"), "expect": untouched},
    ],
})

cases.append({
    "name": "ilk bozuk değer söylenir: önce merkez (x, sonra y), sonra açı; kimlik hatası dönüşümden önce",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": rotate(0, 0, 1)}, "nonFinite": {"transform.center.y": "-Infinity", "transform.angle": "NaN"},
         "result": failed("not_finite", NFC("Merkezin", "y"), "transform.center.y"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": rotate(0, 0, 1)}, "nonFinite": {"transform.angle": "NaN"},
         "result": failed("not_finite", NFV("Dönme açısı", "Açıyı sonlu bir sayıyla verin."), "transform.angle"), "expect": untouched},
        {"op": "execute", "input": {"uids": ["ABC"], "transform": rotate(0, 0, 1)}, "nonFinite": {"transform.angle": "NaN"},
         "result": failed("invalid_uid", BADUID("ABC"), "uids[0]"), "expect": untouched},
    ],
})

cases.append({
    "name": "ölçek faktörü sıfırdan büyük olmalı: sıfır ve eksi reddedilir; sonsuz faktör sonlu değildir",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "transform": scale(487030, 4420010, 0)}, "result": failed("invalid_factor", FACTOR, "transform.factor"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(13)], "transform": scale(487030, 4420010, -2)}, "result": failed("invalid_factor", FACTOR, "transform.factor"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(13)], "transform": scale(487030, 4420010, 2)}, "nonFinite": {"transform.factor": "Infinity"},
         "result": failed("not_finite", NFV("Ölçek faktörü", "Faktörü sonlu bir sayıyla verin."), "transform.factor"), "expect": untouched},
    ],
})

cases.append({
    "name": "simetri ekseninin iki noktası ayrı olmalı; eksenin noktaları sonlu olmalı",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": mirror_t(487025, 4420000, 487025, 4420000)}, "result": failed("invalid_axis", AXIS_SAME, "transform.b"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": mirror_t(487025, 4420000, 487025, 4420000)}, "nonFinite": {"transform.a.x": "NaN"},
         "result": failed("not_finite", NFC("Eksenin ilk noktasının", "x"), "transform.a.x"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": mirror_t(487025, 4420000, 487025, 4420010)}, "nonFinite": {"transform.b.y": "Infinity"},
         "result": failed("not_finite", NFC("Eksenin ikinci noktasının", "y"), "transform.b.y"), "expect": untouched},
    ],
})

cases.append({
    "name": "beklenen sürüm çizimin sürümüyse yazılır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "transform": move(1, 2), "expectedRevision": "$current"}, "result": done(changed=[U(13)]),
         "expect": {"entities": entities((13, moved(ORIG(13), translation(1, 2)))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; kopya da yapılmaz",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": {"uids": [U(13)], "transform": move(1, 2)}, "result": done(changed=[U(13)])},
        {"op": "execute", "input": {"uids": [U(10)], "transform": move(12.5, -7.25), "copy": True, "expectedRevision": "$r0"}, "result": conflict(),
         "expect": {"ids": IDS, "entities": entities((10, ORIG(10))), "revision": "same"}},
    ],
})

cases.append({
    "name": "beklenen sürüm ondalık bir tamsayı yazısıdır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "transform": move(1, 1), "expectedRevision": "01"}, "result": failed("invalid_revision", BADREV("01"), "expectedRevision"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "transform": move(1, 1), "expectedRevision": "-1"}, "result": failed("invalid_revision", BADREV("-1"), "expectedRevision"), "expect": untouched},
    ],
})

cases.append({
    "name": "girdinin hatası çizimin durumundan önce gelir; çakışma nesneden ve kilitten önce",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": {"uids": [U(13)], "transform": move(1, 2)}, "result": done(changed=[U(13)])},
        {"op": "execute", "input": {"uids": [U(10)], "transform": scale(0, 0, 0), "expectedRevision": "$r0"}, "result": failed("invalid_factor", FACTOR, "transform.factor"), "expect": {"revision": "same"}},
        {"op": "execute", "input": {"uids": [MISSING], "transform": move(1, 1), "expectedRevision": "$r0"}, "result": conflict(), "expect": {"revision": "same"}},
        {"op": "execute", "input": {"uids": [U(12)], "transform": move(1, 1), "expectedRevision": "$r0"}, "result": conflict(), "expect": {"revision": "same"}},
    ],
})

cases.append({
    "name": "doğrulama hiçbir şey yazmaz; plan yazılacak nesneleri gösterir; yazma planın sürümüyle planı yazar",
    "steps": [
        {"op": "validate", "input": {"uids": [U(10), U(12)], "transform": move(12.5, -7.25)}, "result": valid([locked_warning(1)]), "expect": untouched},
        {"op": "plan", "input": {"uids": [U(10), U(12)], "transform": move(12.5, -7.25)}, "result": planned([U(10)], [moved(ORIG(10), M_MOVE)], locked=[U(12)], warnings=[locked_warning(1)]), "expect": untouched},
        {"op": "captureRevision", "as": "plan"},
        {"op": "execute", "input": {"uids": [U(10), U(12)], "transform": move(12.5, -7.25), "expectedRevision": "$plan"}, "result": done(changed=[U(10)], locked=[U(12)], warnings=[locked_warning(1)]),
         "expect": {"entities": entities((10, moved(ORIG(10), M_MOVE))), "revision": "changed"}},
    ],
})

cases.append({
    "name": "kopyanın planında nesnenin yuvası 0'dır: yuva yazılınca verilir",
    "steps": [
        {"op": "plan", "input": {"uids": [U(11)], "transform": move(12.5, -7.25), "copy": True}, "result": planned([U(11)], [moved(ORIG(11), M_MOVE, 0)]), "expect": untouched},
    ],
})

cases.append({
    "name": "plan ve doğrulama geçmişe dokunmaz: yinelenecek adım kalır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "transform": move(1, 2)}, "result": done(changed=[U(13)])},
        {"op": "undo", "returns": "Taşı"},
        {"op": "validate", "input": {"uids": [U(13)], "transform": move(1, 2)}, "result": valid(), "expect": {"canRedo": True, "revision": "same"}},
        {"op": "plan", "input": {"uids": [U(13)], "transform": move(1, 2)}, "result": planned([U(13)], [moved(ORIG(13), translation(1, 2))]), "expect": {"canRedo": True, "revision": "same"}},
        {"op": "redo", "returns": "Taşı", "expect": {"entities": entities((13, moved(ORIG(13), translation(1, 2))))}},
    ],
})

cases.append({
    "name": "sayı taşması: dönüşüm sonlu olmayan bir değer verirse hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "transform": scale(0, 0, 1e308)}, "result": failed("not_finite", OVERFLOW, "transform"), "expect": untouched},
        {"op": "plan", "input": {"uids": [U(13)], "transform": move(1e308, 0)}, "result": planned([U(13)], [moved(ORIG(13), translation(1e308, 0))]), "note": "Doğuya 1e308 metre taşımak sonludur: taşma yok.", "expect": untouched},
    ],
})

assert len(cases) >= 20, len(cases)
for c in cases:
    assert_no_negative_zero(c, c["name"])

# The overflow case must overflow and its neighbour must not, by the definitions.
big = moved(ORIG(13), scaling(1e308, (0, 0)))
assert not (math.isfinite(big["c"]["x"]) and math.isfinite(big["r"])), big
far = moved(ORIG(13), translation(1e308, 0))
assert math.isfinite(far["c"]["x"]), far


cases.append({
    "name": "kılavuz: köşeleri döner ve ölçeklenir, notu yazı gibi döner, yüksekliği ölçekle çarpılır; aynalamada not okunur kalır; oku, notu ve zemini kalır (ADR 0146)",
    "note": "Beklenen değerler dönüşümlerin tanımından ve yazının dönüş kuralından çift duyarlıkla hesaplandı.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(19)], "transform": rotate(487010, 4420010, HALF_PI)}, "result": done(changed=[U(19)]),
         "expect": {"entities": entities((19, moved(ORIG(19), M_ROT))), "revision": "changed"}},
        {"op": "undo", "returns": "Döndür", "expect": {"entities": entities((19, ORIG(19)))}},
        {"op": "execute", "input": {"uids": [U(19)], "transform": scale(487000, 4420000, 2)}, "result": done(changed=[U(19)]),
         "expect": {"entities": entities((19, moved(ORIG(19), M_SCALE2))), "revision": "changed"}},
        {"op": "undo", "returns": "Ölçekle", "expect": {"entities": entities((19, ORIG(19)))}},
        {"op": "execute", "input": {"uids": [U(19)], "transform": mirror_t(*AX)}, "result": done(changed=[U(19)]),
         "expect": {"entities": entities((19, moved(ORIG(19), M_MIRROR))), "revision": "changed"}},
        {"op": "undo", "returns": "Aynala", "expect": {"entities": entities((19, ORIG(19))), "canUndo": False}},
    ],
})

# ── Yeni ölçü türleri (docs/adr/0147 §4): their own drawing ──
NEW_DIMENSIONS = [
    {"kind": "dimension", "id": 1, "layerId": "yapi", "attrs": {}, "a": P(487000, 4420030), "b": P(487006, 4420050), "offset": 0, "height": 2.5,
     "style": "ordinate", "angle": 0},
    {"kind": "dimension", "id": 2, "layerId": "yapi", "attrs": {}, "a": P(487060, 4420030), "b": P(487050, 4420040), "c": P(487050, 4420030), "offset": 2,
     "height": 2, "style": "arcLength"},
    {"kind": "dimension", "id": 3, "layerId": "yapi", "attrs": {}, "a": P(487100, 4419730), "b": P(487100, 4420030), "c": P(487104, 4420010), "offset": 5,
     "height": 2, "style": "jogged"},
    {"kind": "dimension", "id": 4, "layerId": "yapi", "attrs": {}, "a": P(487040, 4420060), "b": P(487080, 4420060), "offset": 1.5, "height": 2,
     "style": "slope", "za": 105.25, "zb": 104.75, "mask": True},
]
NEW_SETUP = {**SETUP, "entities": NEW_DIMENSIONS}
NEW_BY_ID = {e["id"]: e for e in NEW_DIMENSIONS}
NEW_UIDS = [U(i) for i in NEW_BY_ID]
M_NEW_MIRROR = mirror((487025, 4420000), (487025, 4420010))
M_NEW_TURN = rotation(HALF_PI, (487000, 4420000))
M_NEW_DOUBLE = scaling(2, (487000, 4420000))
cases.append({
    "name": "yeni ölçü türleri (ADR 0147 §4): aynada yay uzunluğu aynı yayı ölçer (uçları yer değiştirir), kırıklı yarıçapın ve koordinatın ötelenmesi işaret değiştirmez, eğiminki değiştirir; döndürmede koordinatın ekseni dünyanındır; ölçek uzaklığı ve yüksekliği çarpar, kotlar kalır",
    "setup": NEW_SETUP,
    "steps": [
        {"op": "execute", "input": {"uids": NEW_UIDS, "transform": mirror_t(487025, 4420000, 487025, 4420010)}, "result": done(changed=NEW_UIDS),
         "expect": {"entities": entities(*((i, moved(NEW_BY_ID[i], M_NEW_MIRROR)) for i in NEW_BY_ID)), "revision": "changed"}},
        {"op": "undo", "returns": "Aynala", "expect": {"entities": entities(*((i, NEW_BY_ID[i]) for i in NEW_BY_ID))}},
        {"op": "execute", "input": {"uids": NEW_UIDS, "transform": rotate(487000, 4420000, HALF_PI)}, "result": done(changed=NEW_UIDS),
         "expect": {"entities": entities(*((i, moved(NEW_BY_ID[i], M_NEW_TURN)) for i in NEW_BY_ID)), "revision": "changed"}},
        {"op": "undo", "returns": "Döndür"},
        {"op": "execute", "input": {"uids": NEW_UIDS, "transform": scale(487000, 4420000, 2)}, "result": done(changed=NEW_UIDS),
         "expect": {"entities": entities(*((i, moved(NEW_BY_ID[i], M_NEW_DOUBLE)) for i in NEW_BY_ID)), "revision": "changed"}},
    ],
})


# ── Oturt (docs/adr/0156 §6): the similarity, affine and projective transforms in centred form ──
# Their geometry is the core's warp (ops::warp; its rules have their own reference, warp_cases.py): a
# similarity moves every kind as the modify tools do, in three passes (to the source centre, the linear
# part, to the target centre); otherwise a point goes to `to + g(p − from)`. The cases compare only what
# that arithmetic gives bit for bit (points, lines, straight areas with their elevations, and the
# similarity's every kind); texts and curves are in the input for the warnings, their geometry checked
# by fixtures/fit/v1/warp.json.
OTURT_FROM, OTURT_TO = (487000, 4420000), (487100.25, 4420050.5)


def similarity_t(a, b):
    return {"kind": "similarity", "from": P(*OTURT_FROM), "to": P(*OTURT_TO), "a": a, "b": b}


def affine_t(m):
    return {"kind": "affine", "from": P(*OTURT_FROM), "to": P(*OTURT_TO), "m": m}


def projective_t(h):
    return {"kind": "projective", "from": P(*OTURT_FROM), "to": P(*OTURT_TO), "h": h}


def similar(e, a, b, new_id=None):
    """The core's similarity: to the source centre, the linear part, to the target centre."""
    step = moved(e, translation(-OTURT_FROM[0], -OTURT_FROM[1]))
    step = moved(step, [a, b, -b, a, 0.0, 0.0])
    return moved(step, translation(OTURT_TO[0], OTURT_TO[1]), new_id)


def warp_point(p, t):
    x, y = p["x"] - OTURT_FROM[0], p["y"] - OTURT_FROM[1]
    if t["kind"] == "affine":
        a, b, c, d = t["m"]
        gx, gy = a * x + c * y, b * x + d * y
    else:
        a1, a2, a3, b1, b2, b3, c1, c2 = t["h"]
        w = c1 * x + c2 * y + 1.0
        gx, gy = (a1 * x + a2 * y + a3) / w, (b1 * x + b2 * y + b3) / w
    return P(OTURT_TO[0] + gx, OTURT_TO[1] + gy)


def warped(e, t, new_id=None):
    """A point, a line or a straight path under a non-similar warp: its vertices by f, its elevations kept."""
    out = json.loads(json.dumps(e))
    if new_id is not None:
        out["id"] = new_id
    if e["kind"] == "point":
        out["p"] = warp_point(e["p"], t)
    elif e["kind"] == "line":
        out["a"], out["b"] = warp_point(e["a"], t), warp_point(e["b"], t)
    else:
        out["pts"] = [warp_point(q, t) for q in e["pts"]]
        for h in out.get("holes") or []:
            h["pts"] = [warp_point(q, t) for q in h["pts"]]
    return out


OTURT = [
    {"kind": "point", "id": 1, "layerId": "yapi", "attrs": {"Ad": "101"}, "label": "101", "p": P(487010.5, 4420020.25), "z": 101.5},
    {"kind": "line", "id": 2, "layerId": "yapi", "attrs": {}, "a": P(487000, 4420000), "b": P(487030, 4420010), "za": 100.0, "zb": 102.0},
    {"kind": "polygon", "id": 3, "layerId": "sinir", "attrs": {"Ada": "101"}, "pts": [P(487000, 4420000), P(487040, 4420000), P(487040, 4420030), P(487000, 4420030)],
     "zs": [100.0, 100.5, None, 101.0], "holes": [{"pts": [P(487010, 4420010), P(487015, 4420010), P(487015, 4420015)], "zs": [99.0, 99.5, 99.25]}]},
    {"kind": "text", "id": 4, "layerId": "yapi", "attrs": {}, "p": P(487005, 4420025), "text": "Ada 101", "height": 2, "rotation": 0},
    {"kind": "circle", "id": 5, "layerId": "yapi", "attrs": {}, "c": P(487030, 4420010), "r": 2.5},
    {"kind": "polyline", "id": 6, "layerId": "yapi", "attrs": {}, "pts": [P(487000, 4420040), P(487010, 4420040), P(487020, 4420050)], "bulges": [0.0, 0.4, 0.0],
     "zs": [100.0, 101.0, 102.0]},
    {"kind": "line", "id": 7, "layerId": "kilitli", "attrs": {}, "a": P(487005, 4420005), "b": P(487015, 4420005)},
    {"kind": "arc", "id": 8, "layerId": "yapi", "attrs": {}, "c": P(487020, 4420020), "r": 4, "a0": 0, "a1": HALF_PI},
]
OTURT_SETUP = {**SETUP, "entities": OTURT}
OTURT_BY_ID = {e["id"]: e for e in OTURT}
OTURT_IDS = [e["id"] for e in OTURT]
O = lambda i: OTURT_BY_ID[i]
SIM_A, SIM_B = 0.9998, 0.0175
AFFINE = [1.002, -0.0004, 0.0007, 0.999]
PROJECTIVE = [1.0, 0.002, 0.25, -0.001, 0.998, -0.5, 1e-6, -2e-6]


def CURVES(n):
    return {"code": "warp_curves", "message": f"{n} nesnenin eğrileri 0,1 mm'lik köşelere açıldı; dönüşüm benzerlik değil, eğri olarak kalamazlar.", "path": "transform"}


def SHAPES(n):
    return {"code": "warp_shapes", "message": f"{n} yazı, not, blok, ölçü ya da tarama deseni yerinde döndürülüp ölçeklendi; dönüşüm benzerlik değil, biçimleri eğilmez.", "path": "transform"}


SINGULAR = "Dönüşüm tekil: doğrusal kısmı nesneleri bir doğruya ya da noktaya ezer. Dönüşümün sayılarını denetleyin ya da başka bir dönüşüm türü seçin."
HORIZON = "Projektif dönüşümün ufku nesnelerin arasından geçiyor: bir nesnenin noktası ufkun ötesinde kalıyor, dönüştürülemez. O nesneleri dışarıda bırakın ya da kontrol noktalarını denetleyin."
NUMBER_FIX = "Dönüşümün sayılarını sonlu verin."

cases.append({
    "name": "Oturt, benzerlik (Helmert): her tür taşı ve döndür gibi; daire daire kalır, kotlar kalır; adım Oturt",
    "note": "Merkezli biçim: kaynak merkezine taşı, doğrusal kısım, hedef merkezine taşı; beklenen değerler bu üç adımın tanımından.",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "captureUid", "id": 3, "as": "ada"},
        {"op": "execute", "input": {"uids": [U(i) for i in (1, 2, 3, 4, 5, 6, 8)], "transform": similarity_t(SIM_A, SIM_B)},
         "result": done(changed=[U(i) for i in (1, 2, 3, 4, 5, 6, 8)]),
         "expect": {"ids": OTURT_IDS, "entities": entities(*((i, similar(O(i), SIM_A, SIM_B)) for i in (1, 2, 3, 4, 5, 6, 8))), "uids": {"3": "ada"},
                    "canUndo": True, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Oturt", "note": "Vektör oturtma penceresinin adımı.", "expect": {"entities": entities(*((i, O(i)) for i in (1, 2, 3, 4, 5, 6, 8))), "canUndo": False}},
    ],
})

cases.append({
    "name": "Oturt, afin: nokta, çizgi ve düz alan (deliği ve kotlarıyla) tam; yazı biçimini korur, yaylar köşelere açılır, ikisi de sayısıyla söylenir; kilitli katmandaki çizgi kalır",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "execute", "input": {"uids": [U(i) for i in (1, 2, 3, 4, 5, 6, 7)], "transform": affine_t(AFFINE)},
         "result": done(changed=[U(i) for i in (1, 2, 3, 4, 5, 6)], locked=[U(7)], warnings=[locked_warning(1), CURVES(1), SHAPES(1)]),
         "expect": {"ids": OTURT_IDS, "entities": entities(*((i, warped(O(i), affine_t(AFFINE))) for i in (1, 2, 3)), (7, O(7))), "revision": "changed"}},
        {"op": "undo", "returns": "Oturt", "expect": {"entities": entities(*((i, O(i)) for i in (1, 2, 3, 4, 5, 6))), "canUndo": False}},
    ],
})

cases.append({
    "name": "Oturt, afin kopyası: kopyalar yeni kalıcı kimlik alır, asıllar yerinde kalır; adım yine Oturt",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "execute", "input": {"uids": [U(2), U(3)], "transform": affine_t(AFFINE), "copy": True}, "result": done(created=[U(9), U(10)]),
         "expect": {"ids": OTURT_IDS + [9, 10], "entities": entities((2, O(2)), (3, O(3)), (9, warped(O(2), affine_t(AFFINE), 9)), (10, warped(O(3), affine_t(AFFINE), 10))),
                    "uids": {"9": "new", "10": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Oturt", "expect": {"ids": OTURT_IDS}},
    ],
})

cases.append({
    "name": "Oturt, projektif: nokta, çizgi ve düz alan tam (doğru doğruya gider); daire köşelere açılır",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "execute", "input": {"uids": [U(i) for i in (1, 2, 3, 5)], "transform": projective_t(PROJECTIVE)},
         "result": done(changed=[U(i) for i in (1, 2, 3, 5)], warnings=[CURVES(1)]),
         "expect": {"ids": OTURT_IDS, "entities": entities(*((i, warped(O(i), projective_t(PROJECTIVE))) for i in (1, 2, 3))), "revision": "changed"}},
        {"op": "undo", "returns": "Oturt", "expect": {"entities": entities(*((i, O(i)) for i in (1, 2, 3, 5)))}},
    ],
})

cases.append({
    "name": "Oturt'un retleri: sonlu olmayan sayı (yolu ve sırası), tekil afin, ufkun ötesinde kalan nesne; hiçbiri yazılmaz",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "execute", "input": {"uids": [U(2)], "transform": similarity_t(SIM_A, SIM_B)}, "nonFinite": {"transform.from.y": "NaN", "transform.a": "Infinity"},
         "result": failed("not_finite", NFC("Kaynak merkezinin", "y"), "transform.from.y"), "expect": {"ids": OTURT_IDS, "canUndo": False, "revision": "same"}},
        {"op": "execute", "input": {"uids": [U(2)], "transform": similarity_t(SIM_A, SIM_B)}, "nonFinite": {"transform.b": "NaN"},
         "result": failed("not_finite", NFV("Dönüşümün b sayısı", NUMBER_FIX), "transform.b"), "expect": {"ids": OTURT_IDS, "canUndo": False, "revision": "same"}},
        {"op": "execute", "input": {"uids": [U(2)], "transform": affine_t(AFFINE)}, "nonFinite": {"transform.m[2]": "-Infinity"},
         "result": failed("not_finite", NFV("Dönüşümün 3. sayısı", NUMBER_FIX), "transform.m[2]"), "expect": {"ids": OTURT_IDS, "canUndo": False, "revision": "same"}},
        {"op": "execute", "input": {"uids": [U(2)], "transform": projective_t(PROJECTIVE)}, "nonFinite": {"transform.to.x": "NaN", "transform.h[6]": "NaN"},
         "result": failed("not_finite", NFC("Hedef merkezinin", "x"), "transform.to.x"), "expect": {"ids": OTURT_IDS, "canUndo": False, "revision": "same"}},
        {"op": "execute", "input": {"uids": [U(2)], "transform": affine_t([1.0, 2.0, 0.5, 1.0])},
         "result": failed("invalid_transform", SINGULAR, "transform"), "expect": {"ids": OTURT_IDS, "canUndo": False, "revision": "same"}},
        {"op": "execute", "input": {"uids": [U(2), U(1)], "transform": projective_t([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -0.05, 0.0])},
         "result": failed("beyond_horizon", HORIZON, "transform"), "expect": {"ids": OTURT_IDS, "canUndo": False, "revision": "same"}},
    ],
})


# ── Kauçuk levha (docs/adr/0158 §3–§4): only vertices move by the sheet ──
# A sheet's bits are its solution's arithmetic: sheet_f64.py (beside this file) solves it in the core's
# order of operations and is held to the 50-digit reference (rubber_cases.py). Points, ends and vertices
# go to their image, bulges and elevations stay; a circle and an arc move by the nearest similarity at
# their centre, as the modify tools move them (moved()). The text is in the input for `warp_shapes`, its
# geometry checked by fixtures/fit/v1/rubber-warp.json. `rubber_bends` counts the shapes bent over 0.1 mm
# and shows the largest in mm with one decimal (the display rule).


def link(fx, fy, tx, ty):
    return {"from": P(fx, fy), "to": P(tx, ty)}


# Two fixed corners, two corners and the middle moved by centimetres: what is left after Oturt.
LINKS = [
    link(486990.0, 4419990.0, 486990.0, 4419990.0),
    link(487050.0, 4419990.0, 487050.02, 4419990.01),
    link(487050.0, 4420060.0, 487050.0, 4420060.0),
    link(486990.0, 4420060.0, 486989.985, 4420060.02),
    link(487020.0, 4420025.0, 487020.035, 4420024.98),
]
SHEET = sheet_f64.Sheet(LINKS)
assert SHEET.error is None
sheet_f64.assert_near_reference(LINKS, [l["from"] for l in LINKS] + [P(487010.5, 4420020.25), P(487040, 4420030), P(487100, 4420100)])


def rubber_t(links=LINKS):
    return {"kind": "rubbersheet", "links": links}


def on_sheet(e, new_id=None):
    """An object on the sheet: its vertices by the map; a circle or an arc by the nearest similarity at its centre."""
    if e["kind"] in ("circle", "arc"):
        c = e["c"]
        to = SHEET.map(c)
        step = moved(e, translation(-c["x"], -c["y"]))
        step = moved(step, sheet_f64.nearest_similarity(SHEET.jacobian(c)))
        return moved(step, translation(to["x"], to["y"]), new_id)
    out = json.loads(json.dumps(e))
    if new_id is not None:
        out["id"] = new_id
    if e["kind"] == "point":
        out["p"] = SHEET.map(e["p"])
    elif e["kind"] == "line":
        out["a"], out["b"] = SHEET.map(e["a"]), SHEET.map(e["b"])
    else:
        out["pts"] = [SHEET.map(q) for q in e["pts"]]
        for h in out.get("holes") or []:
            h["pts"] = [SHEET.map(q) for q in h["pts"]]
    return out


def BENDS(ids):
    """The bend warning of these objects on SHEET: how many over 0.1 mm, the largest in mm (one decimal, a decimal comma)."""
    bends = [sheet_f64.bend(O(i), SHEET) for i in ids]
    sheet_f64.margins(bends)
    bent = sum(1 for b in bends if b > sheet_f64.CHORD)
    mm = shown(max(bends) * 1000, 1).replace(".", ",")
    message = f"{bent} nesne gerçek görüntüsünden 0,1 mm'den çok sapıyor (en çok {mm} mm): kauçuk levha yalnız köşeleri taşır, kenarlar doğru, yaylar şişkinliğiyle kalır."
    return {"code": "rubber_bends", "message": message, "path": "transform"}


def LINKS_REFUSED(why):
    return failed("invalid_links", why, "transform.links")


TOO_FEW = "Kauçuk levha için en az 3 bağ gerekir. Bağ ekleyin."
DUPLICATE = "İki bağın kaynağı aynı nokta. Birini çıkarın ya da Kullan'dan bırakın."
COLLINEAR = "Bağların kaynakları bir doğru üstünde; levha kurulamaz. Doğrunun dışında bir bağ ekleyin."
SINGULAR_LINKS = "Bağların denklem takımının tek çözümü yok. Birbirine çok yakın kaynakları birleştirin."
# Two sources a last bit apart at national coordinates: not one point, but no single solution.
NEXT = math.nextafter(487000.0, math.inf)
NEAR = [link(487000.0, 4420000.0, 487000.0, 4420000.0), link(NEXT, 4420000.0, NEXT, 4420000.01), link(487100.0, 4420000.0, 487100.0, 4420000.0), link(487000.0, 4420100.0, 487000.0, 4420100.0)]
assert sheet_f64.Sheet(NEAR).error == "singular"
SHEET_IDS = (1, 2, 3, 4, 5, 6, 8)
RUBBER_UNTOUCHED = {"ids": OTURT_IDS, "canUndo": False, "revision": "same"}

cases.append({
    "name": "Kauçuk levha: nokta, çizgi, delikli alan ve yaylı çoklu çizgi yalnız köşeleriyle taşınır, şişkinlik ve kotlar kalır; daire ve yay türünü korur; yazı biçimini korur; kilitli katmandaki çizgi kalır; 0,1 mm'den çok sapanlar sayılır; adım Kauçuk levha",
    "note": "Bağlar: iki köşe sabit, iki köşe ve orta santimetrelerle kayar. Beklenen değerler levhanın çekirdekle aynı işlem sırasıyla çözümünden (sheet_f64.py, 50 basamaklı başvuruya bağlı).",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "captureUid", "id": 3, "as": "ada"},
        {"op": "validate", "input": {"uids": [U(i) for i in OTURT_IDS], "transform": rubber_t()},
         "result": valid([locked_warning(1), SHAPES(1), BENDS(SHEET_IDS)]), "expect": RUBBER_UNTOUCHED},
        {"op": "execute", "input": {"uids": [U(i) for i in OTURT_IDS], "transform": rubber_t()},
         "result": done(changed=[U(i) for i in SHEET_IDS], locked=[U(7)], warnings=[locked_warning(1), SHAPES(1), BENDS(SHEET_IDS)]),
         "expect": {"ids": OTURT_IDS, "entities": entities(*((i, on_sheet(O(i))) for i in (1, 2, 3, 5, 6, 8)), (7, O(7))), "uids": {"3": "ada"},
                    "canUndo": True, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Kauçuk levha", "note": "Vektör oturtma penceresinin Kauçuk levha adımı.",
         "expect": {"entities": entities(*((i, O(i)) for i in OTURT_IDS)), "canUndo": False}},
    ],
})

cases.append({
    "name": "Kauçuk levha kopyası: kopyalar yeni kalıcı kimlik alır, asıllar yerinde kalır; adım yine Kauçuk levha",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "execute", "input": {"uids": [U(2), U(3)], "transform": rubber_t(), "copy": True}, "result": done(created=[U(9), U(10)], warnings=[BENDS((2, 3))]),
         "expect": {"ids": OTURT_IDS + [9, 10], "entities": entities((2, O(2)), (3, O(3)), (9, on_sheet(O(2), 9)), (10, on_sheet(O(3), 10))),
                    "uids": {"9": "new", "10": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Kauçuk levha", "expect": {"ids": OTURT_IDS}},
    ],
})

cases.append({
    "name": "Kauçuk levha'nın retleri: bağın sonlu olmayan sayısı (yolu, sayılar önce), 3'ten az bağ, aynı kaynak, bir doğru üstündeki kaynaklar, tek çözümü olmayan takım; hiçbiri yazılmaz",
    "setup": OTURT_SETUP,
    "steps": [
        {"op": "execute", "input": {"uids": [U(2)], "transform": rubber_t()}, "nonFinite": {"transform.links[1].to.y": "NaN", "transform.links[3].from.x": "Infinity"},
         "result": failed("not_finite", NFC("2. bağın hedefinin", "y"), "transform.links[1].to.y"), "expect": RUBBER_UNTOUCHED},
        {"op": "execute", "input": {"uids": [U(2)], "transform": rubber_t(LINKS[:2])}, "nonFinite": {"transform.links[0].from.x": "-Infinity"},
         "result": failed("not_finite", NFC("1. bağın kaynağının", "x"), "transform.links[0].from.x"), "expect": RUBBER_UNTOUCHED},
        {"op": "execute", "input": {"uids": [U(2)], "transform": rubber_t([])}, "result": LINKS_REFUSED(TOO_FEW), "expect": RUBBER_UNTOUCHED},
        {"op": "execute", "input": {"uids": [U(2)], "transform": rubber_t(LINKS[:2])}, "result": LINKS_REFUSED(TOO_FEW), "expect": RUBBER_UNTOUCHED},
        {"op": "execute", "input": {"uids": [U(2)], "transform": rubber_t(LINKS[:3] + [link(487050.0, 4419990.0, 487050.5, 4419990.5)])},
         "result": LINKS_REFUSED(DUPLICATE), "expect": RUBBER_UNTOUCHED},
        {"op": "execute", "input": {"uids": [U(2)], "transform": rubber_t([link(487000.0, 4420000.0, 487000.0, 4420000.0), link(487010.0, 4420010.0, 487010.02, 4420010.0), link(487030.0, 4420030.0, 487030.0, 4420030.0)])},
         "result": LINKS_REFUSED(COLLINEAR), "expect": RUBBER_UNTOUCHED},
        {"op": "execute", "input": {"uids": [U(2)], "transform": rubber_t(NEAR)}, "result": LINKS_REFUSED(SINGULAR_LINKS), "expect": RUBBER_UNTOUCHED},
    ],
})


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def write(command, title, note, cases):
    fixture = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note, "setup": SETUP, "cases": cases}
    out = ["{"]
    for key in ["format", "version", "command", "commandVersion", "title", "note"]:
        out.append(f'  "{key}": {compact(fixture[key])},')
    s = fixture["setup"]
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
    "cad.entities.transform",
    "Nesneleri dönüştür: doğrulama, plan, yazma, geri alma",
    "ADR 0037. Denetim sırası: en az bir kimlik ve her kimliğin yazımı; dönüşümün sayılarının sonlu olması (alanların sırasıyla), ölçek faktörünün sıfırdan büyük olması, simetri ekseninin yönü olması; beklenen sürümün yazımı, sonra çizimin sürümü; her kimliğin çizimde olması; hepsinin kilitli katmanda olmaması; sonucun sonlu olması. Kilitli katmandaki nesne ne değişir ne kopyalanır. Adım aracın adıdır: Taşı, Kopyala, Döndür, Ölçekle, Aynala. Beklenen geometri dönüşümlerin tanımından, aynı işlem sırasıyla çift duyarlıkla hesaplandı. Kurulumdaki en büyük kimlik 21; kopyalar 22'den başlar. $uidOf:N, N yuvasındaki nesnenin kalıcı kimliğidir.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
