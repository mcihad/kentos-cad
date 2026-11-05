#!/usr/bin/env python3
"""The shared cases of cad.review.notes.edit, cad.review.notes.list and cad.drawing.take (docs/adr/1000), written from
the ADR without KentOS code, and the drawing files the take cases take from, written by the independent KCAD v2 writer
(kcad_v2_reference.py).

    python3 scripts/fixtures/review_cases.py           # writes fixtures/commands/v1/cad.review.notes.*.json,
                                                       # cad.drawing.take.json and fixtures/review/v1/*.kcad
    python3 scripts/fixtures/review_cases.py --check   # writes nothing; compares

The drawing is cad.drawing.compare's parcel map (drawing_compare_cases.py) and a locked layer with a point on it.

Notes (ADR 1000 §2–§4): a note's number is one more than the largest a note in the drawing has; a new note is open;
its anchors keep the object's stamp then (the stamp itself is SHA-256 of canonical JSON, ADR 0253 §2: the cases do
not compute it, they hold its equality: a moved, re-attributed or deleted object makes the anchor `changed` or
`removed`, an undo makes it `current` again, a rebind stamps it again). A note on a locked layer's object is allowed:
it changes no object. The checks in their order and their messages are the ADR's, written here by hand.

Taking (ADR 1000 §5): by persistent id, an object only the other state has comes back (a new slot, its id kept), one
only the open drawing has goes, one that differs is replaced (slot and id kept), one equal stays. All or nothing; one
undo step. A locked layer, a layer the drawing lacks, an id in neither refuse all; with a base, an object both sides
changed apart is a conflict unless `overwrite`.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import drawing_compare_cases as dc  # noqa: E402
import kcad_v2_reference as kr  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILES = ROOT / "fixtures/review/v1"
X0, Y0 = dc.X0, dc.Y0
uid = dc.uid
NOTE_A = "0192f5b0-2000-7000-8000-00000000000a"
NOTE_B = "0192f5b0-2000-7000-8000-00000000000b"
NOTE_C = "0192f5b0-2000-7000-8000-00000000000c"
GHOST = "0192f5a3-3333-7000-8000-000000000999"
Q = {"expect": {"revision": "same", "canUndo": False}}

REVISION_TEXT = ("Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
                 "komutun planından alın.")
TEXT = ("Notun metni boş olmamalı, kenarında boşluk ve satır sonu ile sekme dışında denetim karakteri bulunmamalı, "
        "en çok 4000 karakter olmalı.")
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict",
    "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
    "path": "expectedRevision", "revision": "$current"}}


def setup():
    d = dc.base()
    d["name"] = "İnceleme ve farkları alma"
    locked = dc.layer("kilit", "Kilitli")
    locked["locked"] = True
    d["layers"].append(locked)
    d["entities"].append({"kind": "point", "id": 8, "layerId": "kilit", "attrs": {}, "p": {"x": X0 + 90, "y": Y0 + 10}})
    d["uids"].append(uid(8))
    return d


def entity(d, u, slot):
    """The setup's object `u` as the drawing writes it in `slot`."""
    i = d["uids"].index(u)
    return dict(copy.deepcopy(d["entities"][i]), id=slot)


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


# ── Notes ────────────────────────────────────────────────────────────


def add(id_=None, text="Kapı genişliğini kontrol edin.", **kw):
    op = {"kind": "add", "text": text, **kw}
    if id_ is not None:
        op["id"] = id_
    return {"change": op}


def stored(id_, number, text="Kapı genişliğini kontrol edin.", anchors=(), status="open", **kw):
    n = {"id": id_, "number": number, "text": text, "status": status, **kw}
    if anchors:
        n["anchors"] = [{"uid": a, "stamp": "$any"} for a in anchors]
    return n


def edit(input_, result, expect=None, op="execute", command=None):
    step = {"op": op, "input": input_, "result": result}
    if command:
        step["command"] = command
    if expect is not None:
        step["expect"] = expect
    return step


def edited(id_, number, notes):
    return done({"id": id_, "number": number, "notes": notes, "revision": "$current"})


def note_step(input_, id_, number, notes, settings, **expect):
    return edit(input_, edited(id_, number, notes), {"revision": "changed", "settings": {"reviewNotes": settings}, **expect},
                command="cad.review.notes.edit")


def failed_edit(input_, code, message, path):
    return edit(input_, refusal(code, message, path), {"revision": "same", "settings": {"reviewNotes": None}},
                command="cad.review.notes.edit")


def step_move(u, dx, dy):
    return dc.step_move(u, dx, dy)


def step_delete(u):
    return dc.step_delete(u)


def edit_cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    a2 = stored(NOTE_A, 1, anchors=[uid(2)], author="Ayşe", revision="B")
    case("Not ekle: parsel 102'ye bağlı, açık, 1 numara; tek adım", [
        note_step(add(NOTE_A, anchors=[uid(2)], author="Ayşe", revision="B"), NOTE_A, 1, 1, [a2], canUndo=True, dirty=True)])
    case("Noktalı, AI kaynaklı, çok satırlı not; bağsız", [
        note_step(add(NOTE_A, text="Bir.\nİki\tüç", at={"x": X0 + 5.0, "y": Y0 + 5.0}, origin="ai"), NOTE_A, 1, 1,
                  [stored(NOTE_A, 1, text="Bir.\nİki\tüç", at={"x": X0 + 5.0, "y": Y0 + 5.0}, origin="ai")])])
    case("Kilitli katmandaki nesneye not bağlanır: not nesneyi değiştirmez", [
        note_step(add(NOTE_A, anchors=[uid(8)]), NOTE_A, 1, 1, [stored(NOTE_A, 1, anchors=[uid(8)])])])
    case("Notu geri al: liste boşalır; yinele: geri gelir", [
        note_step(add(NOTE_A), NOTE_A, 1, 1, [stored(NOTE_A, 1)]),
        {"op": "undo", "returns": "İnceleme notu", "expect": {"settings": {"reviewNotes": None}}},
        {"op": "redo", "returns": "İnceleme notu", "expect": {"settings": {"reviewNotes": [stored(NOTE_A, 1)]}}}])
    case("Kenarında boşluk olan metin: ret", [failed_edit(add(NOTE_A, text=" Kapı"), "invalid_review_note", TEXT, "change.text")])
    case("Boş metin: ret", [failed_edit(add(NOTE_A, text=""), "invalid_review_note", TEXT, "change.text")])
    case("4001 karakterlik metin: ret", [failed_edit(add(NOTE_A, text="a" * 4001), "invalid_review_note", TEXT, "change.text")])
    case("Zil karakterli metin: ret", [failed_edit(add(NOTE_A, text="a\u0007b"), "invalid_review_note", TEXT, "change.text")])
    case("Büyük harfli kimlik: ret", [failed_edit(add(NOTE_A.upper()), "invalid_uid",
        f"“{NOTE_A.upper()}” geçerli bir not kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği notlar listesinden alın.", "change.id")])
    case("Aynı kimlikle ikinci not: ret", [
        note_step(add(NOTE_A), NOTE_A, 1, 1, [stored(NOTE_A, 1)]),
        edit(add(NOTE_A), refusal("duplicate_review_note",
             f"“{NOTE_A}” kimlikli bir inceleme notu çizimde zaten var; başka bir kimlik verin ya da kimliği boş bırakın.", "change.id"),
             {"revision": "same", "settings": {"reviewNotes": [stored(NOTE_A, 1)]}}, command="cad.review.notes.edit")])
    case("Kenarı boşluklu yazar: ret", [failed_edit(add(NOTE_A, author="Ayşe "), "invalid_review_note",
        "Notun yazarı boş olmamalı, kenarında boşluk ve denetim karakteri bulunmamalı, en çok 120 karakter olmalı.", "change.author")])
    case("Noktanın doğusu NaN: ret", [dict(failed_edit(add(NOTE_A, at={"x": 0.0, "y": Y0}), "not_finite",
        "Notun noktası doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", "change.at.x"),
        nonFinite={"change.at.x": "NaN"})])
    case("Aynı nesne iki kez: ret", [failed_edit(add(NOTE_A, anchors=[uid(2), uid(2)]), "invalid_review_anchors",
        f"“{uid(2)}” nesnesi iki kez verildi; her nesneyi bir kez bağlayın.", "change.anchors[1]")])
    case("Bağ UUID değil: ret", [failed_edit(add(NOTE_A, anchors=["parsel-2"]), "invalid_uid",
        "“parsel-2” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "change.anchors[0]")])
    case("Çizimde olmayan nesne: ret", [failed_edit(add(NOTE_A, anchors=[uid(1), GHOST]), "entity_not_found",
        f"“{GHOST}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesneyi seçin.", "change.anchors[1]")])
    case("Olmayan pafta: ret", [failed_edit(add(NOTE_A, sheet="pafta-9"), "unknown_sheet",
        "“pafta-9” kimlikli pafta çizimde yok. Var olan bir paftayı seçin.", "change.sheet")])
    case("Olmayan adlı görünüş: ret", [failed_edit(add(NOTE_A, view="gorunus-9"), "unknown_view",
        "“gorunus-9” kimlikli adlı görünüş çizimde yok. Var olan bir görünüşü seçin.", "change.view")])
    case("Durum Yanıtlandı, revizyon C: aynı not, aynı numara", [
        note_step(add(NOTE_A), NOTE_A, 1, 1, [stored(NOTE_A, 1)]),
        note_step({"change": {"kind": "update", "id": NOTE_A, "status": "answered", "revision": "C"}}, NOTE_A, 1, 1,
                  [stored(NOTE_A, 1, status="answered", revision="C")])])
    case("Boş güncelleme: ret", [
        note_step(add(NOTE_A), NOTE_A, 1, 1, [stored(NOTE_A, 1)]),
        edit({"change": {"kind": "update", "id": NOTE_A}}, refusal("invalid_review_note",
             "Değişiklik boş: metin, durum ya da revizyondan en az birini verin.", "change"),
             {"revision": "same"}, command="cad.review.notes.edit")])
    case("Olmayan notu güncelleme: ret", [failed_edit({"change": {"kind": "update", "id": NOTE_B, "status": "resolved"}},
        "review_note_not_found", f"“{NOTE_B}” kimlikli inceleme notu çizimde yok: silinmiş olabilir. Notlar listesini yenileyin.", "change.id")])
    case("Yanıt ve Çözüldü: yanıt sona eklenir, durum yalnız verildiği için değişir", [
        note_step(add(NOTE_A), NOTE_A, 1, 1, [stored(NOTE_A, 1)]),
        note_step({"change": {"kind": "reply", "id": NOTE_A, "text": "Bakılacak.", "author": "Cihad"}}, NOTE_A, 1, 1,
                  [stored(NOTE_A, 1, replies=[{"text": "Bakılacak.", "author": "Cihad"}])]),
        note_step({"change": {"kind": "reply", "id": NOTE_A, "text": "Düzeltildi.", "status": "resolved"}}, NOTE_A, 1, 1,
                  [stored(NOTE_A, 1, status="resolved", replies=[{"text": "Bakılacak.", "author": "Cihad"}, {"text": "Düzeltildi."}])])])
    case("Numaralar: en büyüğün bir fazlası; aradaki silinen numara boş kalır", [
        note_step(add(NOTE_A), NOTE_A, 1, 1, [stored(NOTE_A, 1)]),
        note_step(add(NOTE_B), NOTE_B, 2, 2, [stored(NOTE_A, 1), stored(NOTE_B, 2)]),
        note_step({"change": {"kind": "remove", "id": NOTE_A}}, NOTE_A, 1, 1, [stored(NOTE_B, 2)]),
        note_step(add(NOTE_C), NOTE_C, 3, 2, [stored(NOTE_B, 2), stored(NOTE_C, 3)])])
    case("Son notu sil: liste yazılmaz", [
        note_step(add(NOTE_A), NOTE_A, 1, 1, [stored(NOTE_A, 1)]),
        note_step({"change": {"kind": "remove", "id": NOTE_A}}, NOTE_A, 1, 0, None)])
    case("Yeniden bağla: bağ listesi verilen liste olur", [
        note_step(add(NOTE_A, anchors=[uid(1), uid(2)]), NOTE_A, 1, 1, [stored(NOTE_A, 1, anchors=[uid(1), uid(2)])]),
        note_step({"change": {"kind": "rebind", "id": NOTE_A, "anchors": [uid(3)]}}, NOTE_A, 1, 1, [stored(NOTE_A, 1, anchors=[uid(3)])])])
    case("Silinen nesneye yeniden bağlanmaz: ret", [
        note_step(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1, [stored(NOTE_A, 1, anchors=[uid(2)])]),
        step_delete(uid(2)),
        edit({"change": {"kind": "rebind", "id": NOTE_A}}, refusal("entity_not_found",
             f"“{uid(2)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesneyi seçin.", "change.anchors[0]"),
             {"revision": "same"}, command="cad.review.notes.edit")])
    case("Doğrulama ve plan yazmaz; plan verilen kimliği ve numarayı söyler", [
        edit(add(NOTE_A), done(None), {"revision": "same", "canUndo": False, "settings": {"reviewNotes": None}}, op="validate",
             command="cad.review.notes.edit"),
        edit(add(NOTE_A), edited(NOTE_A, 1, 1), {"revision": "same", "canUndo": False, "settings": {"reviewNotes": None}}, op="plan",
             command="cad.review.notes.edit")])
    case("Beklenen revizyon eski: çatışma", [
        edit(dict(add(NOTE_A), expectedRevision="999"), CONFLICT, {"revision": "same", "settings": {"reviewNotes": None}},
             command="cad.review.notes.edit")])
    case("Beklenen revizyon sayı değil: ret", [failed_edit(dict(add(NOTE_A), expectedRevision="abc"), "invalid_revision",
                                                           REVISION_TEXT, "expectedRevision")])
    return out


def listed(notes, open_=0, answered=0, resolved=0, stale=0):
    return done({"notes": notes, "open": open_, "answered": answered, "resolved": resolved, "stale": stale, "revision": "$current"})


def state(note, anchors=(), stale=False):
    return {"note": note, "stale": stale, "anchorStates": [{"uid": u, "state": s} for u, s in anchors],
            "sheetMissing": False, "viewMissing": False}


def ask(input_, result):
    return {"op": "execute", "input": input_, "result": result, **copy.deepcopy(Q)}


def ask_after(input_, result):
    """A query after writes: it writes nothing more."""
    return {"op": "execute", "input": input_, "result": result, "expect": {"revision": "same"}}


def list_cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    def put(input_, id_, number, notes):
        return edit(input_, edited(id_, number, notes), {"revision": "changed"}, command="cad.review.notes.edit")

    n2 = stored(NOTE_A, 1, anchors=[uid(2)])
    case("Not yokken: boş liste, sayaçlar sıfır", [ask({}, listed([]))])
    case("Bağlı nesne değişmedi: güncel", [
        put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1),
        ask_after({}, listed([state(n2, [(uid(2), "current")])], open_=1))])
    case("Bağlı parsel taşındı: değişti, eskimiş", [
        put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1), step_move(uid(2), 5.0, 0.0),
        ask_after({}, listed([state(n2, [(uid(2), "changed")], True)], open_=1, stale=1))])
    case("Taşımayı geri al: yeniden güncel", [
        put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1), step_move(uid(2), 5.0, 0.0), {"op": "undo", "returns": "Taşı"},
        ask_after({}, listed([state(n2, [(uid(2), "current")])], open_=1))])
    case("Bağlı parsel silindi: yok, eskimiş", [
        put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1), step_delete(uid(2)),
        ask_after({}, listed([state(n2, [(uid(2), "removed")], True)], open_=1, stale=1))])
    case("Öznitelik değişti: değişti", [
        put(add(NOTE_A, anchors=[uid(1)]), NOTE_A, 1, 1), dc.step_attrs(uid(1), {"Sahip": "Z"}, 1),
        ask_after({}, listed([state(stored(NOTE_A, 1, anchors=[uid(1)]), [(uid(1), "changed")], True)], open_=1, stale=1))])
    case("Katman değişti: değişti", [
        put(add(NOTE_A, anchors=[uid(4)]), NOTE_A, 1, 1), dc.step_layer(uid(4), "parsel", 4),
        ask_after({}, listed([state(stored(NOTE_A, 1, anchors=[uid(4)]), [(uid(4), "changed")], True)], open_=1, stale=1))])
    case("İki bağdan biri değişti: sırasıyla güncel ve değişti", [
        put(add(NOTE_A, anchors=[uid(1), uid(2)]), NOTE_A, 1, 1), step_move(uid(2), 0.0, 1.0),
        ask_after({}, listed([state(stored(NOTE_A, 1, anchors=[uid(1), uid(2)]), [(uid(1), "current"), (uid(2), "changed")], True)],
                             open_=1, stale=1))])
    case("Taşındıktan sonra yeniden bağla: güncel", [
        put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1), step_move(uid(2), 5.0, 0.0),
        put({"change": {"kind": "rebind", "id": NOTE_A}}, NOTE_A, 1, 1),
        ask_after({}, listed([state(n2, [(uid(2), "current")])], open_=1))])
    case("Bağları boşaltarak yeniden bağla: bağsız, güncel", [
        put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1), step_move(uid(2), 5.0, 0.0),
        put({"change": {"kind": "rebind", "id": NOTE_A, "anchors": []}}, NOTE_A, 1, 1),
        ask_after({}, listed([state(stored(NOTE_A, 1))], open_=1))])
    two = [put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1), put(add(NOTE_B, anchors=[uid(3)]), NOTE_B, 2, 2),
           put({"change": {"kind": "update", "id": NOTE_B, "status": "resolved"}}, NOTE_B, 2, 2), step_move(uid(2), 1.0, 0.0)]
    nb = stored(NOTE_B, 2, anchors=[uid(3)], status="resolved")
    case("İki not: sayaçlar bütün notların", two + [
        ask_after({}, listed([state(n2, [(uid(2), "changed")], True), state(nb, [(uid(3), "current")])], open_=1, resolved=1, stale=1))])
    case("Durum süzgeci: yalnız çözülmüşler; sayaçlar değişmez", two + [
        ask_after({"status": ["resolved"]}, listed([state(nb, [(uid(3), "current")])], open_=1, resolved=1, stale=1))])
    case("Yalnız eskimişler", two + [
        ask_after({"staleOnly": True}, listed([state(n2, [(uid(2), "changed")], True)], open_=1, resolved=1, stale=1))])
    case("Süzgeçler birlikte: açık ve eskimiş olmayan yok", two + [
        ask_after({"status": ["resolved"], "staleOnly": True}, listed([], open_=1, resolved=1, stale=1))])
    case("Kilitli katmandaki nesneye bağlı not: güncel", [
        put(add(NOTE_A, anchors=[uid(8)]), NOTE_A, 1, 1),
        ask_after({}, listed([state(stored(NOTE_A, 1, anchors=[uid(8)]), [(uid(8), "current")])], open_=1))])
    case("Notlar numara sırasıyla", [
        put(add(NOTE_A), NOTE_A, 1, 1), put(add(NOTE_B), NOTE_B, 2, 2), put({"change": {"kind": "remove", "id": NOTE_A}}, NOTE_A, 1, 1),
        put(add(NOTE_C), NOTE_C, 3, 2), put({"change": {"kind": "reply", "id": NOTE_B, "text": "Tamam.", "status": "answered"}}, NOTE_B, 2, 2),
        ask_after({}, listed([state(stored(NOTE_B, 2, status="answered", replies=[{"text": "Tamam."}])), state(stored(NOTE_C, 3))],
                             open_=1, answered=1))])
    case("AI kaynaklı not kaynağıyla listelenir", [
        put(add(NOTE_A, origin="ai", revision="B"), NOTE_A, 1, 1),
        ask_after({}, listed([state(stored(NOTE_A, 1, origin="ai", revision="B"))], open_=1))])
    case("Notu geri al: liste boş", [
        put(add(NOTE_A), NOTE_A, 1, 1), {"op": "undo", "returns": "İnceleme notu"}, ask_after({}, listed([]))])
    case("Farkı geri alınca not yeniden güncel", [
        put(add(NOTE_A, anchors=[uid(2)]), NOTE_A, 1, 1), step_move(uid(2), 5.0, 0.0),
        edit({"against": {"kind": "history", "steps": 1}, "uids": [uid(2)]},
             done({"added": [], "replaced": [uid(2)], "removed": [], "unchanged": [], "revision": "$current"}),
             {"revision": "changed"}, command="cad.drawing.take"),
        ask_after({}, listed([state(n2, [(uid(2), "current")])], open_=1))])
    case("Doğrulama ve plan: aynı cevap, yazım yok", [
        edit({}, done(None), copy.deepcopy(Q["expect"]), op="validate"),
        edit({}, listed([]), copy.deepcopy(Q["expect"]), op="plan")])
    return out


# ── Taking ───────────────────────────────────────────────────────────


def revision_files(d):
    out = {}
    out["taban.kcad"] = copy.deepcopy(d)
    m = dc.Model(d)
    m.move(uid(2), -5.0, 0.0)
    m.attrs(uid(1), {"Sahip": "Z"})
    m.remove(uid(3))
    m.add(uid(9), {"kind": "line", "id": 9, "layerId": "yol", "attrs": {}, "a": {"x": X0, "y": Y0 + 70}, "b": {"x": X0 + 10, "y": Y0 + 70}})
    m.move(uid(8), 1.0, 0.0)
    out["surum.kcad"] = dc.from_model(d, m)
    e = copy.deepcopy(d)
    e["layers"].append(dc.layer("arsiv", "Arşiv"))
    e["entities"].append({"kind": "point", "id": 9, "layerId": "arsiv", "attrs": {}, "p": {"x": X0 + 95, "y": Y0 + 10}})
    e["uids"].append(uid(11))
    out["eksik-katman.kcad"] = e
    r = copy.deepcopy(d)
    r["layers"][1]["style"] = dict(r["layers"][1]["style"], color="#ff0000")
    i = r["uids"].index(uid(4))
    r["entities"][i]["b"] = {"x": X0 + 45, "y": Y0 + 30}
    out["renk.kcad"] = r
    return out


def path(name):
    return f"$fixture:review/v1/{name}"


def take(against, uids, **kw):
    return {"against": against, "uids": uids, **kw}


def taken(added=(), replaced=(), removed=(), unchanged=(), warnings=()):
    return done({"added": list(added), "replaced": list(replaced), "removed": list(removed), "unchanged": list(unchanged),
                 "revision": "$current"}, warnings)


def take_cases(d, files):
    out = []
    surum = files["surum.kcad"]

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    def ok(input_, result, **expect):
        return {"op": "execute", "input": input_, "result": result, "expect": {"revision": "changed", "canUndo": True, **expect}}

    def no(input_, code, message, at):
        return {"op": "execute", "input": input_, "result": refusal(code, message, at), "expect": {"revision": "same"}}

    hist = {"kind": "history", "steps": 1}
    sfile = {"kind": "file", "path": path("surum.kcad")}
    base = {"kind": "file", "path": path("taban.kcad")}
    case("Taşınan parseli bir adım öncesinden al: eski hâliyle, aynı yuvada", [
        step_move(uid(2), 5.0, 0.0),
        ok(take(hist, [uid(2)]), taken(replaced=[uid(2)]), entities={"2": entity(d, uid(2), 2)})])
    case("Aldıktan sonra geri al: taşınmış hâli döner", [
        step_move(uid(2), 5.0, 0.0), ok(take(hist, [uid(2)]), taken(replaced=[uid(2)])),
        {"op": "undo", "returns": "Farkları al", "expect": {"entities": {"2": dict(entity(d, uid(2), 2),
            pts=[{"x": p["x"] + 5.0, "y": p["y"]} for p in entity(d, uid(2), 2)["pts"]])}}}])
    case("Silinen çizgiyi al: aynı kimlikle yeni yuvada", [
        step_delete(uid(4)),
        ok(take(hist, [uid(4)]), taken(added=[uid(4)]), ids=[1, 2, 3, 5, 6, 7, 8, 9], entities={"9": entity(d, uid(4), 9)})])
    case("Sonradan çizilen çizgiyi al: silinir", [
        dc.step_line(9, (X0, Y0 + 80), (X0 + 5, Y0 + 80)), {"op": "captureUid", "id": 9, "as": "cizgi"},
        ok(take(hist, ["$uid:cizgi"]), taken(removed=["$uid:cizgi"]), ids=[1, 2, 3, 4, 5, 6, 7, 8])])
    case("Dosyadan 5 m gerideki parsel: değiştirilir", [
        ok(take(sfile, [uid(2)]), taken(replaced=[uid(2)]), entities={"2": dict(surum["entities"][1], id=2)})])
    case("Dosyada olmayan parsel 103: silinir", [ok(take(sfile, [uid(3)]), taken(removed=[uid(3)]), ids=[1, 2, 4, 5, 6, 7, 8])])
    case("Yalnız dosyada olan çizgi: eklenir", [
        ok(take(sfile, [uid(9)]), taken(added=[uid(9)]), ids=[1, 2, 3, 4, 5, 6, 7, 8, 9],
           entities={"9": dict(surum["entities"][surum["uids"].index(uid(9))], id=9)})])
    case("Yalnız özniteliği farklı parsel: değiştirilir", [
        ok(take(sfile, [uid(1)]), taken(replaced=[uid(1)]), entities={"1": dict(entity(d, uid(1), 1), attrs={"No": "101", "Sahip": "Z"})})])
    case("İkisinde aynı yol: dokunulmaz, adım yok", [
        {"op": "execute", "input": take(sfile, [uid(5)]), "result": taken(unchanged=[uid(5)]), "expect": {"revision": "same", "canUndo": False}}])
    case("Üç fark birlikte, tek geri alma", [
        ok(take(sfile, [uid(2), uid(3), uid(9), uid(5)]), taken(added=[uid(9)], replaced=[uid(2)], removed=[uid(3)], unchanged=[uid(5)]),
           ids=[1, 2, 4, 5, 6, 7, 8, 9]),
        {"op": "undo", "returns": "Farkları al", "expect": {"ids": [1, 2, 3, 4, 5, 6, 7, 8], "entities": {"2": entity(d, uid(2), 2)}}}])
    locked = "“Kilitli” katmanı kilitli; üzerindeki nesne değiştirilemez, hiçbir fark alınmadı. Kilidi Katmanlar panelinden açın."
    case("Kilitli katmandaki nokta: ret", [no(take(sfile, [uid(8)]), "layer_locked", locked, "uids[0]")])
    case("Biri kilitli: hiçbiri alınmaz", [no(take(sfile, [uid(2), uid(8)]), "layer_locked", locked, "uids[1]")])
    case("Katmanı açık çizimde olmayan nesne: ret", [
        no(take({"kind": "file", "path": path("eksik-katman.kcad")}, [uid(11)]), "layer_not_found",
           "“Arşiv” katmanı açık çizimde yok; nesne alınamaz. Önce katmanı ekleyin ya da sürümden alın.", "uids[0]")])
    case("İki tarafta da olmayan kimlik: ret", [no(take(sfile, [uid(1), GHOST]), "entity_not_found",
        f"“{GHOST}” kimlikli nesne ne açık çizimde ne de karşılaştırılan sürümde var; karşılaştırmayı yenileyin.", "uids[1]")])
    case("UUID olmayan kimlik: ret", [no(take(sfile, ["parsel-2"]), "invalid_uid",
        "“parsel-2” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği karşılaştırmanın çıktısından alın.", "uids[0]")])
    case("Boş liste: ret", [no(take(sfile, []), "invalid_input", "Alınacak nesne sayısı 1 ile 10000 arasında olmalı; 0 verildi.", "uids")])
    case("Aynı kimlik iki kez: ret", [no(take(sfile, [uid(2), uid(2)]), "invalid_input",
        f"“{uid(2)}” iki kez verildi; her nesneyi bir kez verin.", "uids[1]")])
    case("Geri alma geçmişi boşken: ret", [no(take(hist, [uid(2)]), "no_such_step",
        "Geri alma geçmişinde adım yok; önce çizimde bir değişiklik yapın.", "against.steps")])
    case("Olmayan dosya: ret", [no(take({"kind": "file", "path": path("yok.kcad")}, [uid(2)]), "file_not_found",
        f"{path('yok.kcad')} okunamadı: dosya yok ya da açılamıyor.", "against.path")])
    conflict = (f"“{uid(2)}” nesnesi ortak tabandan sonra iki tarafta da farklı değişti; hiçbir fark alınmadı. Üzerine yazmayı "
                "açıkça seçin ya da nesneyi karşılaştırmadan çıkarın.")
    case("Ortak tabandan iki tarafta değişen parsel: çatışma", [
        step_move(uid(2), 3.0, 0.0), no(take(sfile, [uid(2)], base=base), "take_conflict", conflict, "uids[0]")])
    case("Çatışmada açıkça üzerine yaz", [
        step_move(uid(2), 3.0, 0.0),
        ok(take(sfile, [uid(2)], base=base, overwrite=True), taken(replaced=[uid(2)]), entities={"2": dict(surum["entities"][1], id=2)})])
    case("Açık çizimde değişmeyen parsel tabanla çatışmaz", [
        step_move(uid(2), 3.0, 0.0), ok(take(sfile, [uid(1)], base=base), taken(replaced=[uid(1)]))])
    case("Plan yazmaz, alınacakları söyler", [
        {"op": "plan", "input": take(sfile, [uid(2), uid(3), uid(9)]), "result": taken(added=[uid(9)], replaced=[uid(2)], removed=[uid(3)]),
         "expect": {"revision": "same", "canUndo": False}},
        {"op": "validate", "input": take(sfile, [uid(2)]), "result": done(None), "expect": {"revision": "same", "canUndo": False}}])
    case("Beklenen revizyon eski: çatışma", [
        {"op": "execute", "input": take(sfile, [uid(2)], expectedRevision="999"), "result": CONFLICT, "expect": {"revision": "same"}}])
    case("Beklenen revizyon sayı değil: ret", [no(take(sfile, [uid(2)], expectedRevision="abc"), "invalid_revision", REVISION_TEXT,
                                                  "expectedRevision")])
    case("Göreli dosya yolu: ret", [no(take({"kind": "file", "path": "plan.kcad"}, [uid(2)]), "invalid_file_path",
        "“plan.kcad” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad).", "against.path")])
    case("Bozuk dosya: ret", [no(take({"kind": "file", "path": "$fixture:kcad/v2/broken/arrays-items-short.kcad"}, [uid(2)]),
                                 "import_unreadable", "$any", "against.path")])
    renk = files["renk.kcad"]
    case("Katman rengi farklı sürümden çizgi: alınır, katman görünüşü alınmaz, uyarı", [
        ok(take({"kind": "file", "path": path("renk.kcad")}, [uid(4)]),
           taken(replaced=[uid(4)], warnings=[{"code": "layer_style_differs",
               "message": "1 nesnenin katman görünüşü iki sürümde farklı; katman görünüşü alınmadı.", "path": "uids"}]),
           entities={"4": dict(renk["entities"][renk["uids"].index(uid(4))], id=4)}, layers=d["layers"])])
    return out


def doc(command, title, note, d, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": d, "cases": cases}


NOTE = ("ADR 1000. Bağımsız başvuru scripts/fixtures/review_cases.py: kurallar ADR'den elle yazılır, nesne damgasının "
        "değeri değil eşitliği denetlenir; sürüm dosyaları kcad_v2_reference.py'nin yazıcısıyla yazılır; KentOS kodu yok.")


def main():
    d = setup()
    files = revision_files(d)
    outputs = {FILES / name: kr.container(kr.document(f)) for name, f in files.items()}
    texts = {
        OUT / "cad.review.notes.edit.json": doc("cad.review.notes.edit", "İnceleme notunu düzenle", NOTE, d, edit_cases()),
        OUT / "cad.review.notes.list.json": doc("cad.review.notes.list", "İnceleme notlarını listele", NOTE, d, list_cases()),
        OUT / "cad.drawing.take.json": doc("cad.drawing.take", "Seçilen farkları al", NOTE, d, take_cases(d, files)),
    }
    texts = {p: json.dumps(v, ensure_ascii=False, indent=1) + "\n" for p, v in texts.items()}
    if "--check" in sys.argv[1:]:
        bad = [p for p, b in outputs.items() if not p.exists() or p.read_bytes() != b]
        bad += [p for p, t in texts.items() if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        if bad:
            return 1
        print("İnceleme notu ve farkları alma ortak durumları güncel.")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    for p, b in outputs.items():
        p.write_bytes(b)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    for p, t in texts.items():
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
