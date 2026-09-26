"""The shared cases of the product command cad.entities.array (docs/adr/0047).

    python3 scripts/fixtures/array_command_cases.py           # writes the file
    python3 scripts/fixtures/array_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.array.json. The checks, their
order, codes, paths and messages are written here by hand from the ADR. The
copies are computed independently, from the contract's definitions: a
grid's copy j columns and i rows away moved by (j·dx, i·dy), row after row;
a polar array's step fill/count for a full turn and fill/(count − 1)
otherwise, the k-th copy turned k steps about the centre, or, when copies
do not turn, moved as the middle of the copied objects' box goes round
(the box of lines, circles and points taken from their ends, centres and
radii here). What each map does to an object is affine_reference.py's, in
IEEE double arithmetic in the same order of operations, with Python's own
math.cos/sin/atan2. Nothing is copied from an implementation's output; the
web's and the desktop's handlers must meet these values bit for bit.

The polar cases turn by multiples of a quarter or an eighth of a turn: there
the core's sine and cosine (fdlibm's, as V8's) and this Python's libm
agree. Elsewhere they can differ in the last bit (sin(-240°): the core's is
one unit in the last place above the correctly rounded value), which would
move a TM coordinate by some 1e-10 m: the reference would then test the
libraries, not the array.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero, moved, rotation, translation  # noqa: E402  (the maps, beside this file)

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
    {"kind": "polygon", "id": 9, "layerId": "sinir", "attrs": {"Ada": "101"}, "symbol": "parsel", "pts": [P(487000, 4420000), P(487040, 4420000), P(487040, 4420030)], "bulges": [0.25, 0.5, -0.25]},
    {"kind": "line", "id": 10, "layerId": "yapi", "attrs": {"Tür": "Duvar"}, "color": "#E5484D", "a": P(487010, 4420010), "b": P(487020, 4420010)},
    {"kind": "arc", "id": 11, "layerId": "yapi", "attrs": {}, "c": P(487020, 4420020), "r": 4, "a0": 0, "a1": HALF_PI},
    {"kind": "line", "id": 12, "layerId": "kilitli", "attrs": {}, "a": P(487005, 4420005), "b": P(487015, 4420005)},
    {"kind": "circle", "id": 13, "layerId": "yapi", "attrs": {}, "c": P(487030, 4420010), "r": 2.5},
    {"kind": "text", "id": 14, "layerId": "yapi", "attrs": {}, "p": P(487005, 4420025), "text": "Ada 101", "height": 2, "rotation": 0},
    {"kind": "line", "id": 15, "layerId": "eski", "attrs": {}, "a": P(487005, 4420010), "b": P(487015, 4420010)},
    {"kind": "hatch", "id": 16, "layerId": "yapi", "attrs": {}, "ring": [P(487050, 4420000), P(487060, 4420000), P(487060, 4420010), P(487050, 4420010)], "holes": [[P(487053, 4420003), P(487057, 4420003), P(487057, 4420007)]], "pattern": {"type": "lines", "angle": 0, "spacing": 1.5}},
    {"kind": "dimension", "id": 17, "layerId": "yapi", "attrs": {}, "a": P(487000, 4420040), "b": P(487010, 4420040), "offset": 2, "height": 0.5},
    {"kind": "line", "id": 20, "layerId": "gizli", "attrs": {}, "a": P(487005, 4420015), "b": P(487015, 4420015)},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = 21

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
    ],
    "activeLayer": "yapi",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# ── The layouts, from the contract's definitions ───────────────────────


def grid(rows, cols, dx, dy):
    """The copies' maps, row after row, each row column after column; the originals' place left out."""
    return [translation(j * dx, i * dy) for i in range(rows) for j in range(cols) if i or j]


def polar(c, count, fill, rotate, middle=None):
    """The k-th copy k steps round the centre: turned about it, or moved as `middle` goes round."""
    step = fill / count if abs(abs(fill) - 360.0) < 1e-9 else fill / (count - 1.0)
    out = []
    for k in range(1, count):
        a = (step * k * math.pi) / 180.0
        if rotate:
            out.append(rotation(a, c))
        else:
            dx, dy = middle[0] - c[0], middle[1] - c[1]
            cs, sn = math.cos(a), math.sin(a)
            out.append(translation(cs * dx - sn * dy - dx, sn * dx + cs * dy - dy))
    return out


def middle(*ids):
    """The middle of the box around lines, circles and points, from their ends, centres and radii."""
    xs, ys = [], []
    for i in ids:
        e = BY_ID[i]
        if e["kind"] == "line":
            xs += [e["a"]["x"], e["b"]["x"]]
            ys += [e["a"]["y"], e["b"]["y"]]
        elif e["kind"] == "circle":
            xs += [e["c"]["x"] - e["r"], e["c"]["x"] + e["r"]]
            ys += [e["c"]["y"] - e["r"], e["c"]["y"] + e["r"]]
        elif e["kind"] == "point":
            xs.append(e["p"]["x"])
            ys.append(e["p"]["y"])
        else:
            raise ValueError(e["kind"])
    return ((min(xs) + max(xs)) / 2.0, (min(ys) + max(ys)) / 2.0)


def copies(ids, maps, first=NEXT):
    """The copies as written: map after map, each the objects in order; slots from `first`."""
    out, slot = [], first
    for m in maps:
        for i in ids:
            out.append((slot, moved(BY_ID[i], m, slot)))
            slot += 1
    return out


# ── Answers ──────────────────────────────────────────────────────────────

AXIS = {"x": "doğu (Y)", "y": "kuzey (X)"}


def NFC(whose, axis):
    return f"{whose} {AXIS[axis]} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def NFV(what, fix):
    return f"{what} sonlu bir sayı değil (NaN ya da sonsuz). {fix}"


CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
NOTHING = "Diziye alınacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin."
COUNT = "Satır × sütun 2 ile 10 000 arasında olmalı; satır ve sütun en az 1'dir. Başka bir satır ve sütun sayısı verin."
POLAR_COUNT = "Adet 2 ile 1000 arasında bir tam sayı olmalı. Başka bir adet verin."
FILL = "Doldurma açısı 0 ile ±360 derece arasında olmalı; 0 olamaz. Başka bir açı verin."
NO_DX = "Sütunlar arasındaki aralık (dY) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin."
NO_DY = "Satırlar arasındaki aralık (dX) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin."
SPACING_FIX = "Aralığı sonlu bir sayıyla verin."
OVERFLOW = "Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"


def BADREV(r):
    return f"Beklenen sürüm “{r}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."


def BADUID(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."


def NOTFOUND(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def LOCKED(n):
    return f"{n} nesne kilitli katmanda olduğu için atlandı. Kopyalamak için katmanın kilidini Katmanlar panelinden açın."


def U(i):
    return f"$uidOf:{i}"


def done(made, locked=(), warnings=()):
    return {"status": "completed", "output": {"created": [U(slot) for slot, _ in made], "locked": list(locked), "revision": "$current"}, "warnings": list(warnings)}


def valid(warnings=()):
    return {"status": "completed", "output": None, "warnings": list(warnings)}


def planned(sources, made, locked=(), warnings=()):
    entities = [{**e, "id": 0} for _, e in made]
    return {"status": "completed", "output": {"sources": list(sources), "entities": entities, "locked": list(locked), "revision": "$current"}, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def conflict():
    return {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}


def locked_warning(n):
    return {"code": "layer_locked", "message": LOCKED(n), "path": "uids"}


def G(rows, cols, dx, dy):
    return {"kind": "grid", "rows": rows, "cols": cols, "dx": dx, "dy": dy}


def R(cx, cy, count, fill, rotate):
    return {"kind": "polar", "center": P(cx, cy), "count": count, "fill": fill, "rotate": rotate}


def written(made, extra=None):
    """What the document holds after a write: the originals and the copies."""
    out = {"ids": IDS + [slot for slot, _ in made], "entities": {str(slot): e for slot, e in made}, "revision": "changed"}
    if extra:
        out.update(extra)
    return out


untouched = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
ORIG = lambda i: BY_ID[i]

cases = []

# ── Rectangular arrays ───────────────────────────────────────────────────

made = copies([10], grid(2, 3, 12.5, -4))
cases.append({
    "name": "Dizi: 2 × 3 yerde beş kopya, satır satır, her satırda sütun sütun; kopya bütün alanları ve yeni bir kalıcı kimlik alır; tek adımda geri alınır, yinelenir",
    "steps": [
        {"op": "captureUid", "id": 10, "as": "cizgi"},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(2, 3, 12.5, -4)}, "result": done(made),
         "expect": written(made, {"uids": {"10": "cizgi", **{str(s): "new" for s, _ in made}}, "canUndo": True, "canRedo": False, "dirty": True})},
        {"op": "undo", "returns": "Dizi", "note": "Dizi aracının adımı: bütün kopyalar tek adımda.", "expect": {"ids": IDS, "entities": {"10": ORIG(10)}, "uids": {"10": "cizgi"}, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Dizi", "expect": {"ids": IDS + [s for s, _ in made], "canUndo": True, "canRedo": False}},
    ],
})

made = copies([13, 4], grid(1, 3, 10, 0))
cases.append({
    "name": "birden çok nesne: her yerde nesneler girdinin sırasıyla; tekrarlanan kimlik bir kez; tek satırda satır aralığı gerekmez",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13), U(4), U(13)], "layout": G(1, 3, 10, 0)}, "result": done(made), "expect": written(made)},
    ],
})

made = copies([14, 9], grid(3, 1, 0, 7.5))
cases.append({
    "name": "tek sütunda sütun aralığı gerekmez: yazı ve kapalı alan (yay değerleri, simgesi) üç satıra",
    "steps": [
        {"op": "execute", "input": {"uids": [U(14), U(9)], "layout": G(3, 1, 0, 7.5)}, "result": done(made), "expect": written(made)},
    ],
})

made = copies([11, 16, 17], grid(2, 2, 25, 30))
cases.append({
    "name": "her tür aynı kuralla kopyalanır: yay, delikli tarama (desenin açısı ve aralığı), ölçü",
    "steps": [
        {"op": "execute", "input": {"uids": [U(11), U(16), U(17)], "layout": G(2, 2, 25, 30)}, "result": done(made), "expect": written(made)},
        {"op": "undo", "returns": "Dizi", "expect": {"ids": IDS}},
    ],
})

made = copies([20], grid(1, 2, 5, 0))
cases.append({
    "name": "gizli katmandaki nesne de kopyalanır; uyarı yok",
    "steps": [
        {"op": "execute", "input": {"uids": [U(20)], "layout": G(1, 2, 5, 0)}, "result": done(made), "expect": written(made)},
    ],
})

# ── Polar arrays ─────────────────────────────────────────────────────────

C = (487010, 4420010)
made = copies([10], polar(C, 4, 360, True))
cases.append({
    "name": "Kutupsal dizi: tam turda 4 öğe, kopyalar merkez etrafında 90°, 180°, 270° döner; adım Kutupsal dizi",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*C, 4, 360, True)}, "result": done(made), "expect": written(made)},
        {"op": "undo", "returns": "Kutupsal dizi", "note": "Kutupsal dizi aracının adımı.", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
    ],
})

made = copies([13, 11], polar(C, 3, 180, True))
cases.append({
    "name": "kısmi dolgu: son kopya bitiş açısındadır (180° içinde 3 öğe, 90° arayla); yay saat yönünün tersine kalır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13), U(11)], "layout": R(*C, 3, 180, True)}, "result": done(made), "expect": written(made)},
    ],
})

made = copies([10], polar(C, 4, -360, True))
cases.append({
    "name": "eksi açı saat yönündedir: -360° tam turda 4 öğe, -90° arayla",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*C, 4, -360, True)}, "result": done(made), "expect": written(made)},
    ],
})

O = (487000, 4420000)
made = copies([10, 13], polar(O, 4, 360, False, middle(10, 13)))
cases.append({
    "name": "dönmeden: kopyalar yönünü korur, kopyalanan nesnelerin kutusunun ortası merkez etrafında döner",
    "note": "Çizginin ve dairenin kutusu doğuda 487010–487032,5, kuzeyde 4420007,5–4420012,5: ortası 487021,25; 4420010.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10), U(13)], "layout": R(*O, 4, 360, False)}, "result": done(made), "expect": written(made)},
    ],
})

made = copies([13], polar(O, 3, 90, False, middle(13)))
cases.append({
    "name": "dönmeden, kilitli nesne kopyalanmaz ve ortayı değiştirmez: orta yalnız kopyalanan nesnelerin kutusudur",
    "steps": [
        {"op": "execute", "input": {"uids": [U(12), U(13)], "layout": R(*O, 3, 90, False)}, "result": done(made, locked=[U(12)], warnings=[locked_warning(1)]),
         "expect": written(made, {"entities": {**{str(s): e for s, e in made}, "12": ORIG(12)}})},
    ],
})

# ── Locked layers ────────────────────────────────────────────────────────

made = copies([10], grid(1, 2, 20, 0))
cases.append({
    "name": "kilitli katmandaki nesnenin kopyası yapılmaz; öbürleri uyarıyla kopyalanır",
    "note": "Web'in dizi araçları eskiden kilitli nesneyi de kilitli katmanına kopyalıyordu; ADR 0047 bu farkı kaydeder.",
    "steps": [
        {"op": "execute", "input": {"uids": [U(12), U(10)], "layout": G(1, 2, 20, 0)}, "result": done(made, locked=[U(12)], warnings=[locked_warning(1)]), "expect": written(made)},
    ],
})

cases.append({
    "name": "kilitli grubun katmanı da kilitlidir; hepsi kilitliyse hiçbir şey yazılmaz: layer_locked",
    "steps": [
        {"op": "execute", "input": {"uids": [U(12), U(15)], "layout": G(2, 2, 5, 5)}, "result": failed("layer_locked", LOCKED(2), "uids"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(15)], "layout": R(*O, 4, 360, True)}, "result": failed("layer_locked", LOCKED(1), "uids"), "expect": untouched},
    ],
})

# ── Refusals, in the contract's order ──────────────────────────────────

cases.append({
    "name": "nesne verilmedi: no_entities",
    "steps": [
        {"op": "execute", "input": {"uids": [], "layout": G(2, 2, 5, 5)}, "result": failed("no_entities", NOTHING, "uids"), "expect": untouched},
        {"op": "validate", "input": {"uids": [], "layout": R(*O, 4, 360, True)}, "result": failed("no_entities", NOTHING, "uids"), "expect": untouched},
    ],
})

cases.append({
    "name": "kimlik küçük harfli, tireli bir UUID yazısıdır; ilk bozuk kimlik söylenir",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10), "ABC"], "layout": G(2, 2, 5, 5)}, "result": failed("invalid_uid", BADUID("ABC"), "uids[1]"), "expect": untouched},
    ],
})

cases.append({
    "name": "çizimde olmayan kimlik: entity_not_found, hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10), MISSING], "layout": G(2, 2, 5, 5)}, "result": failed("entity_not_found", NOTFOUND(MISSING), "uids[1]"), "expect": untouched},
    ],
})

cases.append({
    "name": "dizinin sayıları sonlu olmalı: önce doğu aralığı, sonra kuzey; kutupsalda önce merkez, sonra açı",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(2, 2, 5, 5)}, "nonFinite": {"layout.dx": "NaN", "layout.dy": "Infinity"},
         "result": failed("not_finite", NFV("Doğu (Y) yönündeki aralık", SPACING_FIX), "layout.dx"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(2, 2, 5, 5)}, "nonFinite": {"layout.dy": "-Infinity"},
         "result": failed("not_finite", NFV("Kuzey (X) yönündeki aralık", SPACING_FIX), "layout.dy"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*O, 4, 360, True)}, "nonFinite": {"layout.center.y": "NaN", "layout.fill": "NaN"},
         "result": failed("not_finite", NFC("Merkezin", "y"), "layout.center.y"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*O, 4, 360, True)}, "nonFinite": {"layout.fill": "Infinity"},
         "result": failed("not_finite", NFV("Doldurma açısı", "Açıyı sonlu bir sayıyla verin."), "layout.fill"), "expect": untouched},
    ],
})

cases.append({
    "name": "satır ve sütun en az 1, yerleri 2 ile 10 000 arası: invalid_count; 100 × 100 olur",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(0, 3, 5, 5)}, "result": failed("invalid_count", COUNT, "layout.rows"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(3, 0, 5, 5)}, "result": failed("invalid_count", COUNT, "layout.cols"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(1, 1, 5, 5)}, "result": failed("invalid_count", COUNT, "layout"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(101, 100, 5, 5)}, "result": failed("invalid_count", COUNT, "layout"), "expect": untouched},
        {"op": "validate", "input": {"uids": [U(10)], "layout": G(100, 100, 5, 5)}, "result": valid(), "expect": untouched},
    ],
})

cases.append({
    "name": "kutupsal dizinin adedi 2 ile 1000 arası: invalid_count; 1000 olur",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*O, 1, 360, True)}, "result": failed("invalid_count", POLAR_COUNT, "layout.count"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*O, 1001, 360, True)}, "result": failed("invalid_count", POLAR_COUNT, "layout.count"), "expect": untouched},
        {"op": "validate", "input": {"uids": [U(10)], "layout": R(*O, 1000, 360, True)}, "result": valid(), "expect": untouched},
    ],
})

cases.append({
    "name": "birden çok yeri olan yönün aralığı sıfır olamaz: invalid_spacing; bir nanometreden küçük aralık sıfırdır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(1, 3, 0, 5)}, "result": failed("invalid_spacing", NO_DX, "layout.dx"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(3, 1, 4, 0)}, "result": failed("invalid_spacing", NO_DY, "layout.dy"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(2, 2, 1e-10, 0)}, "result": failed("invalid_spacing", NO_DX, "layout.dx"), "expect": untouched},
        {"op": "validate", "input": {"uids": [U(10)], "layout": G(2, 2, 1e-9, -1e-9)}, "result": valid(), "expect": untouched},
    ],
})

cases.append({
    "name": "doldurma açısı 0 olamaz, ±360'ı geçemez: invalid_fill; -360 tam turdur",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*O, 4, 0, True)}, "result": failed("invalid_fill", FILL, "layout.fill"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*O, 4, 360.5, True)}, "result": failed("invalid_fill", FILL, "layout.fill"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": R(*O, 4, -400, False)}, "result": failed("invalid_fill", FILL, "layout.fill"), "expect": untouched},
        {"op": "validate", "input": {"uids": [U(10)], "layout": R(*O, 4, -360, False)}, "result": valid(), "expect": untouched},
    ],
})

cases.append({
    "name": "beklenen sürüm ondalık bir tamsayı yazısıdır; çizimin sürümüyse yazılır",
    "steps": [
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(1, 2, 5, 0), "expectedRevision": "01"}, "result": failed("invalid_revision", BADREV("01"), "expectedRevision"), "expect": untouched},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(1, 2, 5, 0), "expectedRevision": "$current"}, "result": done(copies([10], grid(1, 2, 5, 0))), "expect": written(copies([10], grid(1, 2, 5, 0)))},
    ],
})

cases.append({
    "name": "çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; girdinin hatası çakışmadan, çakışma nesneden ve kilitten önce",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": {"uids": [U(13)], "layout": G(1, 2, 5, 0)}, "result": done(copies([13], grid(1, 2, 5, 0)))},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(1, 2, 5, 0), "expectedRevision": "$r0"}, "result": conflict(), "expect": {"ids": IDS + [NEXT], "revision": "same"}},
        {"op": "execute", "input": {"uids": [U(10)], "layout": G(1, 1, 5, 0), "expectedRevision": "$r0"}, "result": failed("invalid_count", COUNT, "layout"), "expect": {"revision": "same"}},
        {"op": "execute", "input": {"uids": [MISSING], "layout": G(1, 2, 5, 0), "expectedRevision": "$r0"}, "result": conflict(), "expect": {"revision": "same"}},
        {"op": "execute", "input": {"uids": [U(12)], "layout": G(1, 2, 5, 0), "expectedRevision": "$r0"}, "result": conflict(), "expect": {"revision": "same"}},
    ],
})

made = copies([10], grid(1, 2, 5, 0))
cases.append({
    "name": "doğrulama hiçbir şey yazmaz; plan kopyaları yuvaları 0 olarak gösterir; yazma planın sürümüyle planı yazar",
    "steps": [
        {"op": "validate", "input": {"uids": [U(10), U(12)], "layout": G(1, 2, 5, 0)}, "result": valid([locked_warning(1)]), "expect": untouched},
        {"op": "plan", "input": {"uids": [U(10), U(12)], "layout": G(1, 2, 5, 0)}, "result": planned([U(10)], made, locked=[U(12)], warnings=[locked_warning(1)]), "expect": untouched},
        {"op": "captureRevision", "as": "plan"},
        {"op": "execute", "input": {"uids": [U(10), U(12)], "layout": G(1, 2, 5, 0), "expectedRevision": "$plan"}, "result": done(made, locked=[U(12)], warnings=[locked_warning(1)]), "expect": written(made)},
    ],
})

cases.append({
    "name": "kutupsal dizinin planı da yazmaz; plan ve doğrulama geçmişe dokunmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "layout": G(1, 2, 5, 0)}, "result": done(copies([13], grid(1, 2, 5, 0)))},
        {"op": "undo", "returns": "Dizi"},
        {"op": "plan", "input": {"uids": [U(4)], "layout": R(*O, 2, 180, False)}, "result": planned([U(4)], copies([4], polar(O, 2, 180, False, middle(4)))), "expect": {"canRedo": True, "revision": "same"}},
        {"op": "redo", "returns": "Dizi"},
    ],
})

made = copies([13], grid(1, 2, 1e308, 0))
cases.append({
    "name": "sayı taşması: bir kopya sonlu olmayan bir değere düşerse hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [U(13)], "layout": G(1, 3, 1e308, 0)}, "result": failed("not_finite", OVERFLOW, "layout"), "expect": untouched},
        {"op": "plan", "input": {"uids": [U(13)], "layout": G(1, 2, 1e308, 0)}, "result": planned([U(13)], made), "note": "Doğuya 1e308 metre sonludur: taşma yok.", "expect": untouched},
    ],
})

assert len(cases) >= 20, len(cases)
for c in cases:
    assert_no_negative_zero(c, c["name"])

# The overflow case must overflow and its neighbour must not, by the definitions.
assert not math.isfinite(moved(ORIG(13), translation(2e308, 0))["c"]["x"])
assert math.isfinite(made[0][1]["c"]["x"])


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
    "cad.entities.array",
    "Nesneleri diziye kopyala: doğrulama, plan, yazma, geri alma",
    "ADR 0047. Denetim sırası: en az bir nesne; her kimliğin yazımı; yerleşimin sayılarının sonlu olması (sırasıyla), satır ve sütunun ya da adedin aralığı, birden çok yeri olan yönün aralığı ya da doldurma açısı; beklenen sürümün yazımı, sonra çizimin sürümü; her kimliğin çizimde olması; hepsinin kilitli katmanda olmaması; kopyaların sonlu kalması. Kopya aslının bütün alanlarını ve yeni bir kalıcı kimlik alır; kopyalar yer yer, her yerde girdinin sırasıyla yazılır. Kilitli katmandaki nesnenin kopyası yapılmaz. Adım aracın adıdır: Dizi, Kutupsal dizi. Kurulumdaki en büyük kimlik 20; kopyalar 21'den başlar. $uidOf:N, N yuvasındaki nesnenin kalıcı kimliğidir.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
