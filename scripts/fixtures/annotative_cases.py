"""The shared cases of annotative annotations (docs/adr/0323): the product
command cad.annotations.scales, the plot scale's follow of an annotative
object (cad.settings.set), a move carrying a scale's own point
(cad.entities.transform), and the representations a sheet's map draws.

    python3 scripts/fixtures/annotative_cases.py           # writes the files
    python3 scripts/fixtures/annotative_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.annotations.scales.json and
fixtures/annotative/v1/cases.json. The rules are written here from the ADR,
without KentOS code: an annotative text, dimension or leader is its own
geometry at its scale `at`; at another of its scales it is `scale / at` times
as high (a text's box, a dimension's value shift with it), with what that
scale sets apart (a text's and a leader's point and direction, a dimension's
offset and value shift; a leader's points after the tip move with its landing
start); at a scale it does not take part in it is not drawn. When the plot
scale changes, its own geometry becomes the representation at the new scale
and every other scale keeps drawing as it did. The checks, their order,
codes, paths and messages are written by hand from the contract
(crates/shared/contracts/src/cad_annotations.rs, annotative.rs).
"""
import copy
import json
import sys

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": style, "children": []}


def P(x, y):
    return {"x": x, "y": y}


X, Y = 487000, 4420000
H = 0.25  # 2.5 mm at 1:100


def text(i, x, y, words, **more):
    return {"kind": "text", "id": i, "layerId": more.pop("layerId", "yazi"), "attrs": {}, "p": P(X + x, Y + y), "text": words, "height": H, "rotation": 0, **more}


ENTITIES = [
    text(1, 0, 0, "SALON"),
    {"kind": "dimension", "id": 2, "layerId": "yazi", "attrs": {}, "a": P(X, Y + 10), "b": P(X + 4, Y + 10), "offset": 0.5, "height": H},
    {"kind": "leader", "id": 3, "layerId": "yazi", "attrs": {}, "pts": [P(X + 2, Y + 2), P(X + 3, Y + 3)], "text": "Kapı", "height": H, "rotation": 0},
    text(4, 0, 20, "DERE", path={"pts": [P(5, 0)]}),
    {"kind": "line", "id": 5, "layerId": "yazi", "attrs": {}, "a": P(X, Y + 30), "b": P(X + 10, Y + 30)},
    text(6, 0, 40, "ARŞİV", layerId="kilitli"),
    text(7, 10, 0, "MUTFAK", annotative={"at": 100, "scales": [{"scale": 50, "point": P(X + 10.5, Y + 0.25)}]}),
    {"kind": "dimension", "id": 8, "layerId": "yazi", "attrs": {}, "a": P(X, Y + 50), "b": P(X + 4, Y + 50), "offset": 0.5, "height": H,
     "annotative": {"at": 100, "scales": [{"scale": 50, "offset": 0.3}]}},
]
RING = [P(X + 20, Y), P(X + 30, Y), P(X + 30, Y + 8), P(X + 20, Y + 8)]
ANSI31 = {"type": "pattern", "angle": 0, "spacing": 1, "name": "ANSI31", "scale": 0.1, "lines": [{"angle": 45, "origin": [0, 0], "offset": [0, 3.175]}]}
ENTITIES += [
    # docs/adr/0331 §3: a patterned and a solid hatch, a lined hatch annotative at 1:100 and 1:50.
    {"kind": "hatch", "id": 9, "layerId": "yazi", "attrs": {}, "ring": RING, "pattern": ANSI31},
    {"kind": "hatch", "id": 10, "layerId": "yazi", "attrs": {}, "ring": RING, "pattern": {"type": "solid", "angle": 0, "spacing": 1}},
    {"kind": "hatch", "id": 11, "layerId": "yazi", "attrs": {}, "ring": RING, "pattern": {"type": "lines", "angle": 45, "spacing": 0.2},
     "annotative": {"at": 100, "scales": [{"scale": 50}]}},
]
LAYERS = [layer("yazi", "Yazı"), layer("kilitli", "Kilitli katman", locked=True)]
SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Ölçekli açıklamalar",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100, "workspace": "cad", "drawingFont": "barlow"},
    "origin": P(X, Y),
    "layers": LAYERS,
    "activeLayer": "yazi",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}
IDS = [e["id"] for e in ENTITIES]
PLOT = 100


# ── The rules (docs/adr/0323) ─────────────────────────────────────────────


def same_scale(a, b):
    return abs(a - b) <= 1e-9 * max(abs(a), abs(b))


def why_not(e):
    if e["kind"] == "text" and "path" in e:
        return "Eğri boyunca yazı ölçekli olamaz: harfleri eğrisinin üstündedir."
    if e["kind"] == "text" and "labelOf" in e:
        return "Nesneye bağlı yazı ölçekli olamaz: kendi etiket ölçeğini izler; önce Bağı kopar."
    if e["kind"] == "hatch" and e["pattern"]["type"] in ("solid", "gradient"):
        return "Dolu ve degrade taramanın ölçeklenecek deseni yok; yalnız çizgili ve desenli tarama ölçekli olur."
    if e["kind"] in ("text", "dimension", "leader", "hatch"):
        return None
    return "Yalnız yazı, ölçü, kılavuz ve tarama ölçekli olur."


def represented(e, at, entry):
    """`e` (its own at `at`) as it is drawn at `entry`'s scale."""
    f = entry["scale"] / at
    out = copy.deepcopy(e)
    if e["kind"] == "hatch":
        # Its pattern as large on paper: the lines' spacing or the families' scale times the ratio (docs/adr/0331 §3).
        if e["pattern"]["type"] in ("lines", "cross"):
            out["pattern"]["spacing"] = e["pattern"]["spacing"] * f
        elif e["pattern"]["type"] == "pattern":
            out["pattern"]["scale"] = e["pattern"]["scale"] * f
        return out
    out["height"] = e["height"] * f
    if e["kind"] == "text":
        if "boxWidth" in e:
            out["boxWidth"] = e["boxWidth"] * f
        out["p"] = entry.get("point", e["p"])
        out["rotation"] = entry.get("rotation", e["rotation"])
    elif e["kind"] == "leader":
        if "point" in entry:
            last = e["pts"][-1]
            dx, dy = entry["point"]["x"] - last["x"], entry["point"]["y"] - last["y"]
            out["pts"] = [e["pts"][0]] + [P(q["x"] + dx, q["y"] + dy) for q in e["pts"][1:]]
        out["rotation"] = entry.get("rotation", e["rotation"])
    else:
        out["offset"] = entry.get("offset", e["offset"])
        if "textShift" in entry:
            t = entry["textShift"]
            if t["along"] == 0 and t["across"] == 0:
                out.pop("textShift", None)
            else:
                out["textShift"] = t
        elif "textShift" in e:
            out["textShift"] = {"along": e["textShift"]["along"] * f, "across": e["textShift"]["across"] * f}
    return out


def at_scale(e, scale):
    """None: not drawn; the object itself when the scale is its own (or it is not annotative)."""
    a = e.get("annotative")
    if a is None or same_scale(a["at"], scale):
        return e
    entry = next((s for s in a.get("scales", []) if same_scale(s["scale"], scale)), None)
    if entry is None:
        return None
    return represented(e, a["at"], entry)


def near(a, b):
    return abs(a - b) <= 1e-9 * max(abs(a), abs(b), 1.0)


def values(e):
    if e["kind"] == "hatch":
        return {}
    if e["kind"] == "text":
        return {"point": e["p"], "rotation": e["rotation"]}
    if e["kind"] == "leader":
        return {"point": e["pts"][-1], "rotation": e["rotation"]}
    return {"offset": e["offset"], "textShift": e.get("textShift", {"along": 0, "across": 0})}


def entry_for(base, at, scale, wanted):
    shown, want = values(represented(base, at, {"scale": scale})), values(wanted)
    out = {"scale": scale}
    if "point" in want and not (near(shown["point"]["x"], want["point"]["x"]) and near(shown["point"]["y"], want["point"]["y"])):
        out["point"] = want["point"]
    if "rotation" in want and not near(shown["rotation"], want["rotation"]):
        out["rotation"] = want["rotation"]
    if "offset" in want and not near(shown["offset"], want["offset"]):
        out["offset"] = want["offset"]
    if "textShift" in want and not (near(shown["textShift"]["along"], want["textShift"]["along"]) and near(shown["textShift"]["across"], want["textShift"]["across"])):
        out["textShift"] = want["textShift"]
    return out


def with_scales(e, at, scales):
    out = copy.deepcopy(e)
    out["annotative"] = {"at": at, **({"scales": sorted(scales, key=lambda s: s["scale"])} if scales else {})}
    return out


def rebase(e, scale):
    a = e["annotative"]
    entry = next((s for s in a.get("scales", []) if same_scale(s["scale"], scale)), {"scale": scale})
    base = represented(e, a["at"], entry)
    wanted = [(a["at"], e)] + [(s["scale"], represented(e, a["at"], s)) for s in a.get("scales", []) if not same_scale(s["scale"], scale)]
    return with_scales(base, scale, [entry_for(base, scale, s, w) for s, w in wanted])


def E(i):
    return copy.deepcopy(next(e for e in ENTITIES if e["id"] == i))


def ratio(s):
    return "1:" + ("%.6f" % s).rstrip("0").rstrip(".")


def rs(x):
    """A number as Rust's Display writes an f64."""
    return str(int(x)) if float(x).is_integer() and abs(x) < 1e16 else repr(float(x))


def uid(i):
    return f"$uidOf:{i}"


def done(slots, warnings=()):
    return {"status": "completed", "output": {"changed": [uid(i) for i in slots], "ids": list(slots), "revision": "$current"}, "warnings": list(warnings)}


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def unchanged():
    return {"status": "completed", "output": {"changed": [], "ids": [], "revision": "$current"}, "warnings": []}


LOCKED_SKIPPED = lambda n: {"code": "layer_locked", "message": f"{n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}  # noqa: E731
LEFT_OUT = lambda n: {"code": "not_annotative", "message": f"{n} nesne ölçekli olmadığı ya da olamadığı için atlandı (yalnız yazı, ölçü ve kılavuz; eğri boyunca ve bağlı yazı değil).", "path": "uids"}  # noqa: E731
NOT_YET = "Nesne ölçekli değil; önce Ölçekli yap'ı kullanın."
LABELS = {"make": "Ölçekli yap", "unmake": "Ölçekliliği kaldır", "add": "Ölçek ekle", "remove": "Ölçek kaldır", "set": "Ölçekteki yeri", "reset": "Ölçeği sıfırla"}


def run(op, ids, after, warnings=(), **more):
    """A completed run of `op` on `ids` writing `after` ({id: object}), then undo and redo."""
    inp = {"operation": op, "uids": [uid(i) for i in ids], **more}
    return [
        {"op": "execute", "input": inp, "result": done(sorted((int(k) for k in after), key=list(ids).index), warnings), "expect": {"ids": IDS, "entities": after, "canUndo": True, "dirty": True}},
        {"op": "undo", "returns": LABELS[op], "expect": {"entities": {k: E(int(k)) for k in after}}},
        {"op": "redo", "returns": LABELS[op], "expect": {"entities": after}},
    ]


def refused(op, ids, code, message, path, **more):
    return [{"op": "execute", "input": {"operation": op, "uids": [uid(i) for i in ids], **more}, "result": failed(code, message, path), "expect": {"ids": IDS, "canUndo": False}}]


cases = []


def case(name, steps, note=None):
    c = {"name": name, "steps": steps}
    if note:
        c["note"] = note
    cases.append(c)


# ── Ölçekli yap ────────────────────────────────────────────────────────
case("Ölçekli yap: yazı projenin çizim ölçeğinde (1:100) kendi geometrisiyle ölçekli olur; tek adımda geri alınır",
     run("make", [1], {"1": with_scales(E(1), PLOT, [])}))
case("Ölçekli yap ölçeklerle: kendi ölçeği (100) ve yinelenen 50 bir kez, ölçekler küçükten büyüğe",
     run("make", [1], {"1": with_scales(E(1), PLOT, [{"scale": 200}, {"scale": 50}])}, scales=[200, 50, 100, 50]))
case("Ölçekli yap ölçü ve kılavuzda", run("make", [2, 3], {"2": with_scales(E(2), PLOT, []), "3": with_scales(E(3), PLOT, [])}))
case("Ölçekli yap: çizgi ölçekli olamaz; başka nesne kalmayınca ret",
     refused("make", [5], "not_annotative", "Yalnız yazı, ölçü, kılavuz ve tarama ölçekli olur.", "uids[0]"))
case("Ölçekli yap: desenli tarama ölçekli olur (docs/adr/0331 §3)",
     run("make", [9], {"9": with_scales(E(9), PLOT, [{"scale": 50}])}, scales=[50]))
case("Ölçekli yap: dolu taramanın deseni yok; ret",
     refused("make", [10], "not_annotative", "Dolu ve degrade taramanın ölçeklenecek deseni yok; yalnız çizgili ve desenli tarama ölçekli olur.", "uids[0]"))
case("Ölçekli yap: eğri boyunca yazı ölçekli olamaz",
     refused("make", [4], "not_annotative", "Eğri boyunca yazı ölçekli olamaz: harfleri eğrisinin üstündedir.", "uids[0]"))
case("Ölçekli yap: yazı ve çizgi birlikte; çizgi uyarıyla atlanır",
     run("make", [1, 5], {"1": with_scales(E(1), PLOT, [])}, warnings=[LEFT_OUT(1)]))
case("Ölçekli yap: kilitli katmandaki tek nesne reddedilir",
     refused("make", [6], "layer_locked", "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"))
case("Ölçekli yap: kilitli katmandaki uyarıyla atlanır",
     run("make", [1, 6], {"1": with_scales(E(1), PLOT, [])}, warnings=[LOCKED_SKIPPED(1)]))
case("Ölçekli yap ölçekli nesnede yalnız ölçekleri ekler; ayrı değerler kalır",
     run("make", [7], {"7": with_scales(E(7), 100, E(7)["annotative"]["scales"] + [{"scale": 500}])}, scales=[500]))

# ── Ölçekliliği kaldır ──────────────────────────────────────────────────
nope = E(7)
del nope["annotative"]
case("Ölçekliliği kaldır: tek yükseklik, nesnenin kendisi; diğer ölçekler gider", run("unmake", [7], {"7": nope}))
case("Ölçekliliği kaldır: ölçekli olmayan yazı reddedilir", refused("unmake", [1], "not_annotative", NOT_YET, "uids[0]"))

# ── Ölçek ekle ve kaldır ────────────────────────────────────────────────
case("Ölçek ekle: 1:200 de; 1:50'nin yeri kalır",
     run("add", [7], {"7": with_scales(E(7), 100, E(7)["annotative"]["scales"] + [{"scale": 200}])}, scales=[200, 50]))
case("Ölçek ekle ölçeksiz: no_scales",
     refused("add", [7], "no_scales", "Ölçek ekle için ölçek verilmedi: scales'e 1:N ölçeklerinin N'lerini yazın (50, 100 gibi).", "scales"))
case("Ölçek ekle: sıfır ölçek", refused("add", [7], "invalid_scale", f"Ölçek {rs(0)} olamaz: sıfırdan büyük, en çok 1 000 000 olmalı (1:N'nin N'si).", "scales[0]", scales=[0]))
case("Ölçek ekle: 1:2 000 000 çok büyük",
     refused("add", [7], "invalid_scale", f"Ölçek {rs(2000000)} olamaz: sıfırdan büyük, en çok 1 000 000 olmalı (1:N'nin N'si).", "scales[1]", scales=[50, 2000000]))
case("Ölçek ekle: ölçekli olmayan yazı reddedilir", refused("add", [1], "not_annotative", NOT_YET, "uids[0]", scales=[50]))
plain7 = with_scales(E(7), 100, [])
case("Ölçek kaldır: 1:50 gider, yeri de", run("remove", [7], {"7": plain7}, scales=[50]))
case("Ölçek kaldır: kendi ölçeği kaldırılamaz",
     refused("remove", [7], "own_scale", f"{ratio(100)} nesnenin kendi ölçeği; kaldırılamaz. Nesnenin kendisini başka ölçekte istiyorsanız projenin çizim ölçeğini değiştirin ya da Ölçekliliği kaldır'ı kullanın.", "scales[1]", scales=[50, 100]))
case("Ölçek kaldır: katılmadığı ölçek bir şey değiştirmez",
     [{"op": "execute", "input": {"operation": "remove", "uids": [uid(7)], "scales": [500]}, "result": unchanged(), "expect": {"ids": IDS, "entities": {"7": E(7)}, "canUndo": False}}])

# ── Ölçekteki yeri ──────────────────────────────────────────────────────
moved = with_scales(E(7), 100, [{"scale": 50, "point": P(X + 12, Y + 1), "rotation": 30}])
case("Ölçekteki yeri: 1:50'de yazının yeri ve doğrultusu", run("set", [7], {"7": moved}, scale=50, point=P(X + 12, Y + 1), rotation=30))
dim = with_scales(E(8), 100, [{"scale": 50, "offset": 0.4, "textShift": {"along": 0.5, "across": 0}}])
case("Ölçekteki yeri: 1:50'de ölçünün çizgisi ve değeri", run("set", [8], {"8": dim}, scale=50, offset=0.4, textShift={"along": 0.5, "across": 0}))
case("Ölçekteki yeri: ölçünün yeri (point) olmaz",
     refused("set", [8], "invalid_annotative", "Ölçünün ölçeği yalnız çizgisinin uzaklığını (offset) ve değerinin yerini (textShift) değiştirir.", "uids[0]", scale=50, point=P(X, Y)))
case("Ölçekteki yeri: katılmadığı ölçek", refused("set", [7], "scale_not_taken", f"Nesne {ratio(200)} ölçeğinde değil: önce Ölçek ekle ile ekleyin.", "uids[0]", scale=200, rotation=10))
case("Ölçekteki yeri: kendi ölçeği nesnenin kendisidir",
     refused("set", [7], "own_scale", f"{ratio(100)} nesnenin kendi ölçeği: oradaki yeri nesnenin kendisidir. Nesneyi taşıyın ya da döndürün.", "scale", scale=100, rotation=10))
case("Ölçekteki yeri ölçeksiz: no_scale", refused("set", [7], "no_scale", "Değerleri yazılacak ölçek verilmedi: scale'e o ölçeğin N'sini yazın (1:50 için 50).", "scale", rotation=10))
case("Ölçekteki yeri değersiz: no_value",
     refused("set", [7], "no_value", "Ölçekteki yeri için değer verilmedi: yazının ve kılavuzun point ya da rotation'ı, ölçünün offset ya da textShift'i.", "point", scale=50))

# ── Ölçeği sıfırla ──────────────────────────────────────────────────────
case("Ölçeği sıfırla: bütün diğer ölçeklerde ayrı değer kalmaz", run("reset", [7, 8], {"7": with_scales(E(7), 100, [{"scale": 50}]), "8": with_scales(E(8), 100, [{"scale": 50}])}))
case("Ölçeği sıfırla yalnız 1:200: 1:50'nin yeri kalır",
     [{"op": "execute", "input": {"operation": "reset", "uids": [uid(7)], "scales": [200]}, "result": unchanged(), "expect": {"ids": IDS, "entities": {"7": E(7)}}}])
case("Kimliksiz: no_entities", [{"op": "execute", "input": {"operation": "make", "uids": []}, "result": failed("no_entities", "Ölçekleri değişecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"), "expect": {"ids": IDS}}])
case("Kimlik UUID değil: invalid_uid",
     [{"op": "execute", "input": {"operation": "make", "uids": ["yazi-1"]}, "result": failed("invalid_uid", "“yazi-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", "uids[0]"), "expect": {"ids": IDS}}])
case("Çizimde olmayan nesne: entity_not_found",
     [{"op": "execute", "input": {"operation": "make", "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]}, "result": failed("entity_not_found", "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]"), "expect": {"ids": IDS}}])
case("Beklenen sürüm sayı değil: invalid_revision",
     [{"op": "execute", "input": {"operation": "make", "uids": [uid(1)], "expectedRevision": "07"}, "result": failed("invalid_revision", "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": {"ids": IDS}}])
CONFLICT = {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}
case("Çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict",
     [{"op": "captureRevision", "as": "r0"},
      {"op": "execute", "input": {"operation": "make", "uids": [uid(2)]}, "result": done([2])},
      {"op": "execute", "input": {"operation": "make", "uids": [uid(1)], "expectedRevision": "$r0"}, "result": CONFLICT, "expect": {"entities": {"1": E(1)}, "revision": "same"}}])
case("Plan: yazacağını verir, yazmaz",
     [{"op": "plan", "input": {"operation": "make", "uids": [uid(1)], "scales": [50]}, "result": {"status": "completed", "output": {"changed": [with_scales(E(1), PLOT, [{"scale": 50}])], "revision": "$current"}, "warnings": []}, "expect": {"entities": {"1": E(1)}, "canUndo": False}}])

# ── Çizim ölçeği değişince (docs/adr/0323 §3) ───────────────────────────
follow = {}
for e in ENTITIES:
    if e["kind"] == "hatch" and "annotative" in e:
        # An annotative hatch takes the new scale as its own (docs/adr/0331 §3); a hatch has no height to follow.
        follow[str(e["id"])] = rebase(e, 50)
        continue
    if e["kind"] not in ("text", "dimension", "leader") or e["layerId"] == "kilitli":
        continue
    if "annotative" in e:
        r = rebase(e, 50)
        r["height"] = 0.125
        follow[str(e["id"])] = r
    else:
        f = copy.deepcopy(e)
        f["height"] = 0.125
        follow[str(e["id"])] = f
case("Çizim ölçeği 1:100'den 1:50'ye: ölçekli yazı ve ölçünün kendisi 1:50 temsili olur, 1:100 diğer ölçeklerine geçer; diğerleri yüksekliği izler",
     [{"op": "execute", "command": "cad.settings.set", "input": {"plotScale": 50},
       "result": {"status": "completed", "output": {"changed": ["plotScale"], "followed": len(follow), "locked": 1, "revision": "$current"}, "warnings": []},
       "expect": {"ids": IDS, "entities": follow, "settings": {"plotScale": 50}}}],
     note="Yazı 7'nin 1:50 yeri kendi yeri olur, eski yeri 1:100'ün yeridir; ölçü 8'in 1:50 uzaklığı (0,3) kendisinin, 1:100'ün uzaklığı 0,5.")

# ── Taşı (docs/adr/0323 §4) ─────────────────────────────────────────────
m7 = E(7)
m7["p"] = P(X + 20, Y - 5)
m7["annotative"]["scales"][0]["point"] = P(X + 20.5, Y - 4.75)
case("Taşı: ölçekli yazının 1:50 yeri de taşınır",
     [{"op": "execute", "command": "cad.entities.transform", "input": {"uids": [uid(7)], "transform": {"kind": "move", "dx": 10, "dy": -5}},
       "result": {"status": "completed", "output": {"changed": [uid(7)], "created": [], "locked": [], "revision": "$current"}, "warnings": []},
       "expect": {"entities": {"7": m7}}}])


def representation_cases():
    """What a map at a scale draws of each object (fixtures/annotative/v1/cases.json)."""
    out = []
    for e in ENTITIES:
        for scale in (50, 100, 200):
            drawn = at_scale(e, scale)
            out.append({"object": e, "scale": scale, "drawn": "own" if drawn is e else drawn})
    for e in (E(7), E(8), E(11)):
        out.append({"object": e, "rebase": 50, "result": rebase(e, 50)})
        out.append({"object": e, "rebase": 200, "result": rebase(e, 200)})
    lead = E(3)
    lead["annotative"] = {"at": 100, "scales": [{"scale": 50, "point": P(X + 4, Y + 3.5), "rotation": 15}]}
    out.append({"object": lead, "scale": 50, "drawn": at_scale(lead, 50)})
    out.append({"object": lead, "rebase": 50, "result": rebase(lead, 50)})
    return out


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def emit(path, text):
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def write_commands():
    out = ["{"]
    head = {"format": "kentos.command-cases", "version": 1, "command": "cad.annotations.scales", "commandVersion": 1,
            "title": "Ölçekli açıklamalar", "note": "ADR 0323: ölçekli yazı, ölçü ve kılavuz; kural ve beklenenler scripts/fixtures/annotative_cases.py'de KentOS kodu olmadan yazılmıştır."}
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
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st)}" for st in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    emit("fixtures/commands/v1/cad.annotations.scales.json", "\n".join(out) + "\n")


def write_representations():
    items = representation_cases()
    text = "{\n" + '  "format": "kentos.annotative-cases",\n  "version": 1,\n  "note": "ADR 0323 §2, §3: what a map at a scale draws, and an object rebased to another scale; scripts/fixtures/annotative_cases.py.",\n  "cases": [\n'
    text += ",\n".join(f"    {compact(c)}" for c in items) + "\n  ]\n}\n"
    emit("fixtures/annotative/v1/cases.json", text)
    return len(items)


write_commands()
n = write_representations()
print(f"{len(cases)} command cases, {n} representation cases" + (" match" if "--check" in sys.argv[1:] else " written"))
