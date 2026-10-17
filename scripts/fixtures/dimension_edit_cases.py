"""The shared cases of cad.dimensions.edit and cad.dimensions.query (docs/adr/0321 §3, §4; plan/cad.md CAD-ANNO-08, 09,
10).

    python3 scripts/fixtures/dimension_edit_cases.py           # writes the files
    python3 scripts/fixtures/dimension_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.dimensions.edit.json and cad.dimensions.query.json. Worked out here from the ADR,
without KentOS code:

- A straight dimension (aligned: its direction a→b; linear: its angle from east) has its line at `a + n·offset`, n the
  direction's left normal. Ölçü aralığı measures each dimension's line along the base's normal n0 (`pos = line·n0`),
  orders them by their distance from the base's, each side apart, and puts the k-th of a side at `pos0 ± k·spacing`:
  the new offset is `(q − a·n0) / (n·n0)`. A spacing absent is three times the base's value height.
- Ölçüyü kır: where an object's edge crosses one of a straight dimension's three lines (the dimension line d1→d2,
  the extension lines a→d1 and b→d2) at the fraction t of the line; an extension line's crossings within its gap from
  the measured point (0.5 × the value height) are none. t from the crossing point, in the line's order.
- The other edits set a field; a dimension already so is not counted.
- What a dimension shows is scripts/fixtures/dimension_value_cases.py's rule; a written value without `<>` is an
  override; what its points follow: free, partial, bound (every point it measures, or its rule's), broken.

The numbers are whole or halves; `within` where a direction or an angle comes in.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)
from dimension_value_cases import value as written  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
X, Y = 487000, 4420000
PLOT_SCALE = 500
H = 2.5 / 1000.0 * PLOT_SCALE
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
UID = lambda i: f"$uidOf:{i}"  # noqa: E731
BAD = "Ölçü-01"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
PROJECT = {"unit": "m", "lengthDecimals": 3, "angleUnit": "grad"}


def plain(v):
    return int(v) if float(v).is_integer() else v


def P(x, y):
    return {"x": plain(x), "y": plain(y)}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": style, "children": []}


def dim(i, a, b, offset, layer_id="olcu", **extra):
    return {"kind": "dimension", "id": i, "layerId": layer_id, "attrs": {}, "a": P(*a), "b": P(*b), "offset": plain(offset),
            "height": plain(H), **extra}


ENTITIES = [
    {"kind": "line", "id": 10, "layerId": "yapi", "attrs": {}, "a": P(X, Y), "b": P(X + 10, Y)},
    dim(11, (X, Y), (X + 10, Y), 2),
    dim(12, (X, Y), (X + 10, Y), 3.5),
    dim(13, (X + 10, Y), (X, Y), -6),
    dim(14, (X, Y - 1), (X + 8, Y - 1), -2.5, style="linear", angle=0),
    dim(15, (X + 33, Y), (X + 30, Y + 3), 2, style="angular", c=P(X + 30, Y)),
    dim(16, (X + 20, Y), (X + 20, Y + 5), 1),
    {"kind": "line", "id": 17, "layerId": "yapi", "attrs": {}, "a": P(X + 5, Y - 5), "b": P(X + 5, Y + 10)},
    dim(18, (X, Y + 20), (X + 6, Y + 20), 1.5, text="<> tip", textShift={"along": 1, "across": 0.5},
        breaks=[{"line": "first", "t": 0.5}], options={"zeros": "trailing"}),
    dim(19, (X, Y + 30), (X + 4, Y + 30), 1, layer_id="kilitli"),
    dim(20, (X, Y + 40), (X + 5, Y + 40), 1, text="5.00"),
    dim(21, (X, Y + 50), (X + 2.5, Y + 50), 1, unit="mm", decimals=0, options={"tolerance": {"kind": "symmetric", "upper": 0.001}}),
    {"kind": "text", "id": 22, "layerId": "yapi", "attrs": {}, "p": P(X + 5, Y + 2), "text": "Not", "height": 1, "rotation": 0},
]
IDS = [e["id"] for e in ENTITIES]
BY_ID = {e["id"]: e for e in ENTITIES}
NEXT = 23
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Ölçü düzenleme",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": PLOT_SCALE},
    "origin": {"x": X, "y": Y},
    "layers": [layer("yapi", "Yapı"), layer("olcu", "Ölçü"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def bad_uid(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."


def not_found(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def completed(output, warnings=()):
    return {"status": "completed", "warnings": list(warnings), "output": output}


# ── The geometry (see the module) ──────────────────────────────────────────────────────────────────────────────────

def frame(d):
    if d.get("style") == "linear":
        t = d.get("angle", 0) * math.pi / 180.0
        u = (math.cos(t), math.sin(t))
    else:
        dx, dy = d["b"]["x"] - d["a"]["x"], d["b"]["y"] - d["a"]["y"]
        length = math.hypot(dx, dy)
        u = (dx / length, dy / length)
    n = (-u[1], u[0])
    a = (d["a"]["x"], d["a"]["y"])
    return u, n, (a[0] + n[0] * d["offset"], a[1] + n[1] * d["offset"])


def dot(p, q):
    return p[0] * q[0] + p[1] * q[1]


def spaced(base_id, ids, spacing=None):
    """Each dimension of `ids` spaced from the base: the offsets that change, in the input's order."""
    base = BY_ID[base_id]
    _, n0, line0 = frame(base)
    pos0 = dot(line0, n0)
    spacing = 3.0 * base["height"] if spacing is None else spacing
    placed = []
    for i, k in enumerate(ids):
        if k == base_id:
            continue
        d = BY_ID[k]
        _, n, line = frame(d)
        placed.append((dot(line, n0) - pos0, k, n, i))
    placed.sort(key=lambda p: abs(p[0]))
    count = {1: 0, -1: 0}
    out = []
    for frm, k, n, i in placed:
        side = -1 if frm < 0 else 1
        count[side] += 1
        q = pos0 + side * count[side] * spacing
        a = (BY_ID[k]["a"]["x"], BY_ID[k]["a"]["y"])
        off = (q - dot(a, n0)) / dot(n, n0)
        if off != BY_ID[k]["offset"]:
            out.append((i, k, off))
    out.sort()
    return [(k, off) for _, k, off in out]


def with_fields(k, **fields):
    e = copy.deepcopy(BY_ID[k])
    for f, v in fields.items():
        if v is None:
            e.pop(f, None)
        else:
            e[f] = v
    return e


# ── cad.dimensions.edit ────────────────────────────────────────────────────────────────────────────────────────────

def edit(kind, **fields):
    return {"edit": {"kind": kind, **fields}}


def changed(name, inp, made, step, warnings=(), plan=False, within=None):
    """`made`: {id: the dimension after}."""
    ids = list(made)
    if plan:
        s = {"op": "plan", "input": inp, "result": completed({"dimensions": [made[k] for k in ids], "revision": "$current"}, warnings),
             "expect": UNCHANGED}
        if within:
            s["within"] = within
        return {"name": name, "steps": [s]}
    s = {"op": "execute", "input": inp, "result": completed({"changed": [UID(k) for k in ids], "revision": "$current" if ids else "$current"}, warnings),
         "expect": {"ids": IDS, "entities": {str(k): made[k] for k in ids}, "canUndo": bool(ids), "dirty": bool(ids),
                    "revision": "changed" if ids else "same"}}
    if within:
        s["within"] = within
    steps = [s]
    if ids:
        steps.append({"op": "undo", "returns": step, "expect": {"entities": {str(k): BY_ID[k] for k in ids}}})
    return {"name": name, "steps": steps}


def refused(name, inp, code, message, path, non_finite=None):
    s = {"op": "execute", "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    s["result"] = failed(code, message, path)
    s["expect"] = UNCHANGED
    return {"name": name, "steps": [s]}


def edit_cases():
    out = []
    space = lambda ids, **f: edit("space", base=UID(11), uids=[UID(k) for k in ids], **f)  # noqa: E731
    for name, ids, spacing in (("Aralık 1 m: üstteki iki ölçü, ters çizilmişi de, temelden 1 ve 2 m", [12, 13], 1),
                               ("Aralık verilmemiş: temel değer yüksekliğinin üç katı", [12, 13], None),
                               ("Aralık 0: ölçüler temelin çizgisine dizilir", [12, 13], 0),
                               ("Aralık iki yana: alttaki doğrusal ölçü alta, üstteki üste", [14, 12], 1),
                               ("Temel listede de: kendisi oynamaz", [11, 12], 1)):
        f = {} if spacing is None else {"spacing": spacing}
        made = {k: with_fields(k, offset=plain(off)) for k, off in spaced(11, ids, spacing)}
        out.append(changed(name, space(ids, **f), made, "Ölçü aralığı", within=1e-9))
    made = {k: with_fields(k, offset=plain(off)) for k, off in spaced(11, [12, 13], 1)}
    out.append(changed("Aralık: plan, hiçbir şey yazılmaz", space([12, 13], spacing=1), made, "Ölçü aralığı", plan=True, within=1e-9))
    out.append(changed("Aralık zaten öyle: hiçbir ölçü değişmez, hiçbir şey yazılmaz", space([12], spacing=1.5), {}, "Ölçü aralığı"))
    out.append(refused("Aralık: ölçü temele paralel değil", space([12, 16], spacing=1), "not_parallel",
                       "Ölçü temel ölçüye paralel değil; aralık yalnız paralel ölçülerde verilir.", "edit.uids[1]"))
    out.append(refused("Aralık: açı ölçüsü düz değil", space([15], spacing=1), "not_straight",
                       "Ölçü hizalı ya da doğrusal değil: aralık ve kırma yalnız hizalı ve doğrusal ölçülerde yapılır.", "edit.uids[0]"))
    out.append(refused("Aralık eksi", space([12], spacing=-1), "invalid_offset",
                       "Aralık -1; 0 ya da büyük olmalı (0 ölçüleri temelin çizgisine dizer).", "edit.spacing"))
    out.append(refused("Aralık sonlu değil", space([12], spacing=1), "not_finite",
                       "Aralık sonlu bir sayı değil (NaN ya da sonsuz). Aralığı metre olarak sonlu bir sayıyla verin.", "edit.spacing",
                       non_finite={"edit.spacing": "NaN"}))
    # Breaks: the dimension line of 11 runs at y = Y + 2 from x = X to X + 10; the line 17 at x = X + 5 crosses it in
    # its middle. The wall 10 meets the extension lines at the measured points: within their gap, none.
    br = [{"line": "dimension", "t": 0.5}]
    out.append(changed("Kır: verilen nesnenin kestiği yerde, ölçü çizgisinin ortasında", edit("break", uids=[UID(11)], objects=[UID(17)]),
                       {11: with_fields(11, breaks=br)}, "Ölçüyü kır"))
    out.append(changed("Kır: nesne verilmemiş, görünen bütün nesneler; ölçülen noktaya değen duvar kırmaz", edit("break", uids=[UID(11)]),
                       {11: with_fields(11, breaks=br)}, "Ölçüyü kır"))
    out.append(changed("Kır: kesen yok, uyarıyla; eski kırması kalkar", edit("break", uids=[UID(18)], objects=[UID(17)]),
                       {18: with_fields(18, breaks=None)}, "Ölçüyü kır",
                       warnings=[{"code": "no_crossing", "message": "Ölçünün çizgilerini kesen nesne yok; kırma yapılmadı.", "path": "edit.uids[0]"}]))
    out.append(refused("Kır: açı ölçüsü düz değil", edit("break", uids=[UID(15)]), "not_straight",
                       "Ölçü hizalı ya da doğrusal değil: aralık ve kırma yalnız hizalı ve doğrusal ölçülerde yapılır.", "edit.uids[0]"))
    out.append(changed("Kırmayı kaldır", edit("unbreak", uids=[UID(18), UID(11)]), {18: with_fields(18, breaks=None)}, "Kırmayı kaldır"))
    out.append(changed("Yazıyı taşı: kendi doğrultusunda 1, dik 0,5", edit("moveText", uid=UID(11), along=1, across=0.5),
                       {11: with_fields(11, textShift={"along": 1, "across": 0.5})}, "Ölçü yazısını taşı"))
    out.append(changed("Yazıyı yerine döndür: 0 ve 0", edit("moveText", uid=UID(18), along=0, across=0),
                       {18: with_fields(18, textShift=None)}, "Ölçü yazısını taşı"))
    out.append(refused("Yazının kayması sonlu değil", edit("moveText", uid=UID(11), along=1, across=0), "not_finite",
                       "Kayma sonlu bir sayı değil (NaN ya da sonsuz). Kaymayı metre olarak sonlu bir sayıyla verin.", "edit.along",
                       non_finite={"edit.along": "Infinity"}))
    out.append(changed("Değeri yaz: <> ölçülen değerdir, iki ölçüye", edit("text", uids=[UID(11), UID(12)], text="<> (tipik)"),
                       {11: with_fields(11, text="<> (tipik)"), 12: with_fields(12, text="<> (tipik)")}, "Ölçü değerini yaz"))
    out.append(changed("Ölçülen değere dön: null", edit("text", uids=[UID(18)], text=None), {18: with_fields(18, text=None)}, "Ölçü değerini yaz"))
    out.append(refused("Yazılan değer boş", edit("text", uids=[UID(11)], text="  "), "empty_text",
                       "Yazılan değer boş; ölçülen değere dönmek için null verin.", "edit.text"))
    opts = {"tolerance": {"kind": "deviation", "upper": 0.02, "lower": 0.01}, "alt": {"unit": "mm", "decimals": 0}}
    out.append(changed("Seçenekler: sapma toleransı ve mm alternatifi", edit("options", uids=[UID(11)], options=opts),
                       {11: with_fields(11, options=opts)}, "Ölçü seçenekleri"))
    out.append(changed("Seçenekleri kaldır: null", edit("options", uids=[UID(18), UID(12)], options=None),
                       {18: with_fields(18, options=None)}, "Ölçü seçenekleri"))
    out.append(refused("Seçenekler kurala uymuyor: artış sıfır", edit("options", uids=[UID(11)], options={"round": 0}), "invalid_style",
                       "Ölçünün yuvarlama artışı sıfırdan büyük, en çok 1000000 m olmalı; 0 verildi.", "edit.options"))
    out.append(refused("Beklenen sürüm sayı değil", {**edit("unbreak", uids=[UID(18)]), "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                       "expectedRevision"))
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": edit("unbreak", uids=[UID(18)]), "result": "$any"},
        {"op": "execute", "input": {**edit("moveText", uid=UID(11), along=1, across=0), "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict",
                    "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                    "path": "expectedRevision", "revision": "$current"}}}]})
    out.append(refused("Kimlik geçersiz", edit("unbreak", uids=[BAD]), "invalid_uid", bad_uid(BAD), "edit.uids[0]"))
    out.append(refused("Ölçü yok", edit("unbreak", uids=[]), "no_entities", "Düzenlenecek ölçü yok. En az bir ölçünün kimliğini verin.", "edit.uids"))
    out.append(refused("Ölçü çizimde yok", edit("unbreak", uids=[MISSING]), "entity_not_found", not_found(MISSING), "edit.uids[0]"))
    out.append(refused("Ölçü değil: çizgi", edit("unbreak", uids=[UID(10)]), "not_a_dimension",
                       "Verilen nesne bir ölçü değil; bu düzenleme yalnız ölçüleri değiştirir.", "edit.uids[0]"))
    out.append(refused("Kilitli katmandaki ölçü", edit("unbreak", uids=[UID(19)]), "layer_locked",
                       "“Kilitli” katmanı kilitli; üzerindeki ölçü değişmez. Kilidi Katmanlar panelinden açın.", "edit.uids[0]"))
    return out


# ── cad.dimensions.query ───────────────────────────────────────────────────────────────────────────────────────────

def measured(d):
    """What the core lays out: a straight dimension's length along its direction, an angle's sweep."""
    if d.get("style") == "angular":
        c, a, b = d["c"], d["a"], d["b"]
        t0 = math.atan2(a["y"] - c["y"], a["x"] - c["x"])
        return math.fmod(math.atan2(b["y"] - c["y"], b["x"] - c["x"]) - t0 + 4 * math.pi, 2 * math.pi), "angle", ""
    u, _, _ = frame(d)
    return abs((d["b"]["x"] - d["a"]["x"]) * u[0] + (d["b"]["y"] - d["a"]["y"]) * u[1]), "length", ""


def read(k, bond="free", uid=None, entity=None):
    d = entity or BY_ID[k]
    v, unit, prefix = measured(d)
    look = {f: d[f] for f in ("unit", "decimals", "prefix", "suffix", "options") if f in d}
    text = d.get("text")
    return {"uid": uid or UID(k), "kind": d.get("style", "aligned"), "measured": v, "unit": unit,
            "shown": written({"value": v, "unit": unit, "prefix": prefix}, look, PROJECT, text),
            "overridden": text is not None and "<>" not in text, "bond": bond}


def query(name, inp, rows, within=None, before=()):
    s = {"op": "plan", "input": inp, "result": completed({"dimensions": rows, "revision": "$current"}), "expect": {"revision": "same"}}
    if within:
        s["within"] = within
    return {"name": name, "steps": [*before, s]}


def query_cases():
    dims = [e["id"] for e in ENTITIES if e["kind"] == "dimension"]
    out = [query("Bütün ölçüler, çizimin sırasıyla", {}, [read(k) for k in dims], within=1e-12)]
    for k, name in ((11, "Hizalı ölçü: ölçülen ve gösterilen aynı"), (13, "Ters çizilmiş hizalı ölçü"),
                    (14, "Doğrusal ölçü: doğrultusu boyunca"), (15, "Açı ölçüsü: radyan ölçülür, grad gösterilir"),
                    (18, "<> ile yazılan değer: ölçülen içinde, sıfırları bastırılmış; elle değil"),
                    (20, "<> olmadan yazılan değer: elle, ölçülen gösterilmez"),
                    (21, "mm birimi ve simetrik tolerans"),
                    (19, "Kilitli katmandaki ölçü de okunur")):
        out.append(query(name, {"uids": [UID(k)]}, [read(k)], within=1e-12))
    out.append(query("İki ölçü, verilen sırayla", {"uids": [UID(16), UID(12)]}, [read(16), read(12)]))
    # Bound by places: made with cad.dimensions.create, then read.
    make = lambda measure, at: {"op": "execute", "command": "cad.dimensions.create",  # noqa: E731
                                "input": {"measure": measure, "at": P(*at), "layerId": "olcu"}, "result": "$any"}
    start, end = ({"kind": "place", "uid": UID(10), "place": {"kind": k}} for k in ("start", "end"))
    bound = dim(NEXT, (X, Y), (X + 10, Y), 1)
    out.append(query("Bağlı ölçü: iki ucu çizginin uçlarında", {"uids": [UID(NEXT)]},
                     [read(None, "bound", UID(NEXT), bound)],
                     before=[make({"kind": "aligned", "a": start, "b": end}, (X + 5, Y + 1))]))
    out.append(query("Kısmen bağlı ölçü: bir ucu serbest", {"uids": [UID(NEXT)]},
                     [read(None, "partial", UID(NEXT), dim(NEXT, (X, Y), (X + 4, Y), 1))],
                     before=[make({"kind": "aligned", "a": start, "b": {"kind": "at", "at": P(X + 4, Y)}}, (X + 2, Y + 1))]))
    out.append(query("Kopuk bağlı ölçü: çizgi silindi", {"uids": [UID(NEXT)]}, [read(None, "broken", UID(NEXT), bound)],
                     before=[make({"kind": "aligned", "a": start, "b": end}, (X + 5, Y + 1)),
                             {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [UID(10)]}, "result": "$any"}]))
    out.append(query("Bağlı ölçü elle yazılınca: değeri elle, bağı yerinde", {"uids": [UID(NEXT)]},
                     [read(None, "bound", UID(NEXT), {**bound, "text": "10 m"})],
                     before=[make({"kind": "aligned", "a": start, "b": end}, (X + 5, Y + 1)),
                             {"op": "execute", "command": "cad.dimensions.edit",
                              "input": {"edit": {"kind": "text", "uids": [UID(NEXT)], "text": "10 m"}}, "result": "$any"}]))
    out.append(query("Seçenek yazıldıktan sonra gösterilen", {"uids": [UID(11)]},
                     [read(None, "free", UID(11), {**BY_ID[11], "options": {"round": 0.5, "zeros": "trailing"}})],
                     before=[{"op": "execute", "command": "cad.dimensions.edit",
                              "input": {"edit": {"kind": "options", "uids": [UID(11)], "options": {"round": 0.5, "zeros": "trailing"}}},
                              "result": "$any"}]))
    out.append(query("Hiç ölçü verilmemiş: boş liste", {"uids": []}, []))
    for name, inp, code, message in (
            ("Kimlik geçersiz", {"uids": [BAD]}, "invalid_uid", bad_uid(BAD)),
            ("Ölçü çizimde yok", {"uids": [MISSING]}, "entity_not_found", not_found(MISSING)),
            ("Ölçü değil: yazı", {"uids": [UID(22)]}, "not_a_dimension", "Verilen nesne bir ölçü değil; yalnız ölçüler okunur.")):
        out.append({"name": name, "steps": [{"op": "plan", "input": inp, "result": failed(code, message, "uids[0]"), "expect": {"revision": "same"}}]})
    out.append({"name": "Çalıştırınca da yazmaz", "steps": [{"op": "execute", "input": {"uids": [UID(14)]},
                                                             "result": completed({"dimensions": [read(14)], "revision": "$current"}),
                                                             "within": 1e-12, "expect": UNCHANGED}]})
    return out


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "Written by scripts/fixtures/dimension_edit_cases.py from docs/adr/0321; do not edit by hand.",
            "setup": SETUP, "cases": cases}


def main() -> int:
    files = {"cad.dimensions.edit.json": build("cad.dimensions.edit", "Ölçüyü düzenle", edit_cases()),
             "cad.dimensions.query.json": build("cad.dimensions.query", "Ölçüleri oku", query_cases())}
    bad = False
    for name, doc in files.items():
        assert_no_negative_zero(doc, name)
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        path = OUT / name
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil", file=sys.stderr)
                bad = True
            continue
        path.write_text(text, encoding="utf-8")
        print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    if "--check" in sys.argv[1:] and not bad:
        print("cad.dimensions.edit ve cad.dimensions.query durumları güncel.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
