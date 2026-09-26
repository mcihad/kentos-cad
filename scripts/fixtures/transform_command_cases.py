"""The shared cases of the product command cad.entities.transform (docs/adr/0037).

    python3 scripts/fixtures/transform_command_cases.py           # writes the file
    python3 scripts/fixtures/transform_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.transform.json. The checks, their
order, codes, paths and messages are written here by hand from the ADR. The
expected geometry is computed here independently, from the definitions of
the transforms (a displacement; a rotation, a scale and a reflection as 2D
affine maps x' = a*x + c*y + e, y' = b*x + d*y + f), in IEEE double
arithmetic in the same order of operations, with Python's own
math.cos/sin/atan2. Nothing is copied from an implementation's output; the
web's and the desktop's handlers must meet these values bit for bit.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys

TAU = math.pi * 2.0
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

# ── The transforms, as affine maps [a, b, c, d, e, f] ──────────────────


def translation(dx, dy):
    return [1.0, 0.0, 0.0, 1.0, dx, dy]


def rotation(angle, o):
    c = math.cos(angle)
    s = math.sin(angle)
    return [c, s, -s, c, o[0] - c * o[0] + s * o[1], o[1] - s * o[0] - c * o[1]]


def scaling(s, o):
    return [s, 0.0, 0.0, s, o[0] * (1.0 - s), o[1] * (1.0 - s)]


def mirror(p, q):
    dx = q[0] - p[0]
    dy = q[1] - p[1]
    l2 = dx * dx + dy * dy
    a = (dx * dx - dy * dy) / l2
    b = (2.0 * dx * dy) / l2
    return [a, b, b, -a, p[0] - a * p[0] - b * p[1], p[1] - b * p[0] + a * p[1]]


def apply(m, p):
    return (m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5])


def linear(m, v):
    return (m[0] * v[0] + m[2] * v[1], m[1] * v[0] + m[3] * v[1])


def det(m):
    return m[0] * m[3] - m[1] * m[2]


def scale_of(m):
    return math.sqrt(abs(det(m)))


def reflects(m):
    return det(m) < 0.0


def norm_angle(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0.0 else r


def xy(p):
    return (p["x"], p["y"])


def pt(t):
    return {"x": t[0], "y": t[1]}


def moved(e, m, new_id=None):
    """`e` under `m`: the geometry by the transforms' definitions, every other field kept."""
    e = json.loads(json.dumps(e))
    if new_id is not None:
        e["id"] = new_id
    k = e["kind"]
    s = scale_of(m)
    if k == "point":
        e["p"] = pt(apply(m, xy(e["p"])))
    elif k == "line":
        e["a"] = pt(apply(m, xy(e["a"])))
        e["b"] = pt(apply(m, xy(e["b"])))
    elif k in ("polyline", "polygon"):
        e["pts"] = [pt(apply(m, xy(p))) for p in e["pts"]]
        if "bulges" in e and reflects(m):
            e["bulges"] = [-b for b in e["bulges"]]
    elif k == "circle":
        e["c"] = pt(apply(m, xy(e["c"])))
        e["r"] = e["r"] * s
    elif k == "arc":
        c, r = xy(e["c"]), e["r"]
        start = (c[0] + math.cos(e["a0"]) * r, c[1] + math.sin(e["a0"]) * r)
        end = (c[0] + math.cos(e["a1"]) * r, c[1] + math.sin(e["a1"]) * r)
        c2 = apply(m, c)
        s2, e2 = apply(m, start), apply(m, end)
        ang = lambda q: norm_angle(math.atan2(q[1] - c2[1], q[0] - c2[0]))
        e["c"] = pt(c2)
        e["r"] = r * s
        if reflects(m):
            e["a0"], e["a1"] = ang(e2), ang(s2)
        else:
            e["a0"], e["a1"] = ang(s2), ang(e2)
    elif k == "ellipse":
        e["c"] = pt(apply(m, xy(e["c"])))
        e["major"] = pt(linear(m, xy(e["major"])))
        if reflects(m):
            e["t0"], e["t1"] = norm_angle(-e["t1"]), norm_angle(-e["t0"])
    elif k == "spline":
        e["pts"] = [pt(apply(m, xy(p))) for p in e["pts"]]
    elif k == "text":
        rad = (e["rotation"] * math.pi) / 180.0
        d = linear(m, (math.cos(rad), math.sin(rad)))
        rot = (math.atan2(d[1], d[0]) * 180.0) / math.pi
        if reflects(m):
            rot += 180.0
        rot = math.fmod(math.fmod(rot, 360.0) + 360.0, 360.0)
        e["p"] = pt(apply(m, xy(e["p"])))
        e["height"] = e["height"] * s
        e["rotation"] = rot
    elif k == "dimension":
        assert e.get("style", "aligned") == "aligned"
        e["a"] = pt(apply(m, xy(e["a"])))
        e["b"] = pt(apply(m, xy(e["b"])))
        e["offset"] = e["offset"] * s * (-1.0 if reflects(m) else 1.0)
        e["height"] = e["height"] * s
    else:
        raise ValueError(k)
    return e


def assert_no_negative_zero(v, where):
    """JSON cannot hold what JavaScript writes of −0: the cases keep clear of it."""
    if isinstance(v, float) and v == 0.0 and math.copysign(1.0, v) < 0:
        raise AssertionError(f"−0 at {where}")
    if isinstance(v, dict):
        for k, x in v.items():
            assert_no_negative_zero(x, f"{where}.{k}")
    if isinstance(v, list):
        for i, x in enumerate(v):
            assert_no_negative_zero(x, f"{where}[{i}]")


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
