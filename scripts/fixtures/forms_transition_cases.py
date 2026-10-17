"""Writes fixtures/commands/v1/cad.forms.transition.json: the shared cases of `cad.forms.transition` v1 (İş akışı adımı,
docs/adr/0428), their expectations built here without KentOS code: the step's rule from the ADR, the form's own rules
by the independent form twin of scripts/fixtures/forms_cases.py (`Form.assess`, `.complete`, `.validate`).

    python3 scripts/fixtures/forms_transition_cases.py           # writes the file
    python3 scripts/fixtures/forms_transition_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the form twin, written from the ADRs without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.forms.transition.json"
ANY = fc.ANY
STATES = [("taslak", "Taslak"), ("gonderildi", "Gönderildi"), ("onaylandi", "Onaylandı"), ("iade", "İade edildi")]
FIELDS = [
    {"name": "Durum", "kind": "text", "values": [{"code": c, "label": w} for c, w in STATES]},
    {"name": "Kat", "kind": "integer", "min": "1", "max": "40"},
    {"name": "Yükseklik", "kind": "decimal", "scale": 1},
    {"name": "Açıklama", "kind": "text"},
    {"name": "Gerekçe", "kind": "text", "length": 200},
    {"name": "Geçmiş", "kind": "text", "length": 120},
    {"name": "Notlar", "kind": "text", "length": 160},
]
WORKFLOW = {"field": "Durum", "initial": "taslak", "locked": ["onaylandi"], "steps": [
    {"label": "Gönder", "from": ["taslak", "iade"], "to": "gonderildi", "valid": True},
    {"label": "Onayla", "from": ["gonderildi"], "to": "onaylandi", "valid": True},
    {"label": "Geri gönder", "from": ["gonderildi"], "to": "iade", "reason": "Gerekçe"},
    {"label": "Düzelt", "from": ["onaylandi"], "to": "iade", "reason": "Gerekçe"},
    {"label": "Taslağa al", "to": "taslak"},
]}
ITEMS = [
    {"type": "field", "id": "durum", "field": "Durum"},
    {"type": "field", "id": "kat", "field": "Kat", "required": True},
    {"type": "field", "id": "yukseklik", "field": "Yükseklik", "calculate": "Kat * 3"},
    {"type": "field", "id": "aciklama", "field": "Açıklama"},
    {"type": "field", "id": "gerekce", "field": "Gerekçe"},
]
BODY = {"revision": 1, "items": ITEMS, "workflow": WORKFLOW}
# docs/adr/0429: the same workflow logging its steps.
LOGGED_FLOW = {**WORKFLOW, "log": "Geçmiş"}
LOGGED = {"revision": 1, "items": ITEMS + [{"type": "field", "id": "gecmis", "field": "Geçmiş"}], "workflow": LOGGED_FLOW}
# docs/adr/0442 §4: the steps' notes on the record's fields, with the log.
NOTED_FLOW = {**LOGGED_FLOW, "notes": "Notlar"}
NOTED = {"revision": 1, "items": ITEMS + [{"type": "field", "id": "gecmis", "field": "Geçmiş"}], "workflow": NOTED_FLOW}
PLAIN = {"revision": 1, "items": ITEMS}
TWIN_BODY = {"revision": 1, "items": ITEMS}  # the twin is told of the workflow below, not by the body


def release(body):
    return [{"version": 1, "digest": fc.digest(body), "body": body}]


FORMS = [
    {"id": "denetim", "name": "Bina denetimi", "layer": "bina", "draft": copy.deepcopy(BODY), "releases": release(BODY)},
    {"id": "basit", "name": "Basit", "layer": "bina", "draft": copy.deepcopy(PLAIN), "releases": release(PLAIN)},
    {"id": "taslak", "name": "Taslak akış", "layer": "bina", "draft": copy.deepcopy(BODY)},
    {"id": "kilitli", "name": "Kilitli katman", "layer": "kilit", "draft": copy.deepcopy(BODY), "releases": release(BODY)},
    {"id": "kayitli", "name": "Kayıtlı akış", "layer": "bina", "draft": copy.deepcopy(LOGGED), "releases": release(LOGGED)},
    {"id": "notlu", "name": "Notlu akış", "layer": "bina", "draft": copy.deepcopy(NOTED), "releases": release(NOTED)},
    {"id": "bozuk", "name": "Bozuk akış", "layer": "bina",
     "draft": {**copy.deepcopy(BODY), "workflow": {**copy.deepcopy(WORKFLOW), "field": "Yok"}}},
]
RECORDS = {
    1: {"Durum": "taslak", "Kat": "3"},
    2: {"Durum": "taslak"},
    3: {"Durum": "gonderildi", "Kat": "2", "Yükseklik": "6"},
    4: {"Durum": "onaylandi", "Kat": "4", "Yükseklik": "12"},
    5: {},
    6: {"Kat": "1"},
}
LAYER_OF = {1: "bina", 2: "bina", 3: "bina", 4: "bina", 5: "bina", 6: "yol"}
P = lambda x, y: {"x": 500000 + x, "y": 4400000 + y}  # noqa: E731


def square(x):
    return [P(x, 0), P(x + 10, 0), P(x + 10, 10), P(x, 10)]


ENTITIES = [{"kind": "polygon", "id": k, "layerId": LAYER_OF[k], "attrs": a, "pts": square(20 * k)} for k, a in RECORDS.items()]
ENTITIES.append({"kind": "polygon", "id": 7, "layerId": "kilit", "attrs": {"Durum": "taslak", "Kat": "2"}, "pts": square(160)})
IDS = [e["id"] for e in ENTITIES]
ATTRS = {e["id"]: e["attrs"] for e in ENTITIES}


def layer(i, name, fields=None, locked=False):
    out = {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
           "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    if fields:
        out["fields"] = copy.deepcopy(fields)
    return out


def setup():
    return {"format": "kentos.document", "version": 1, "name": "İş akışı", "settings": {
        "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
        "workspace": "gis", "drawingFont": "barlow", "forms": copy.deepcopy(FORMS)}, "origin": P(0, 0),
        "layers": [layer("bina", "Bina", FIELDS), layer("yol", "Yol", FIELDS), layer("kilit", "Kilit", FIELDS, locked=True)],
        "activeLayer": "bina", "entities": ENTITIES, "styles": {"items": [], "categories": []}}


# ---- The step's rule (docs/adr/0428 §2) ----

def words(code):
    return next(w for c, w in STATES if c == code)


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.error = {"code": code, "message": message, "path": path}


BOUND = [x["field"] for x in ITEMS]


def take(slot, step_label, values, attrs=None, logged=False, moment=None, user=None, noted=False, notes=None, name=""):
    """The record afterwards, by the rule; or Refused."""
    notes = notes or {}
    before = dict(attrs if attrs is not None else ATTRS[slot])
    if logged and "Geçmiş" in values:
        raise Refused("form_invalid", "“Geçmiş” alanını iş akışının adımları yazar; değerlerden çıkarın.", "values.Geçmiş")
    # docs/adr/0442 §4: the notes, by the form's own fields.
    if noted and "Notlar" in values:
        raise Refused("form_invalid", "“Notlar” alanını iş akışının adımları yazar; notları `notes` ile verin.",
                      "values.Notlar")
    if notes and not noted:
        raise Refused("form_invalid", f"“{name}” formunun iş akışında not alanı yok; alanlara not yazılamaz.", "notes")
    for k, v in sorted(notes.items(), key=lambda kv: kv[0].encode()):
        if k not in BOUND and k not in (["Geçmiş"] if logged else []):
            raise Refused("form_invalid", f"“{k}” formun bir alanı değil; not yalnız formdaki alanlara yazılır.", f"notes.{k}")
        if not v.strip() or len(v) > 1000:
            raise Refused("form_invalid", "Not boş olamaz, en çok 1000 karakterdir ve denetim karakteri içermez.",
                          f"notes.{k}")
    step = next((s for s in WORKFLOW["steps"] if s["label"] == step_label), None)
    if step is None:
        raise Refused("unknown_step", f"“Bina denetimi” formunun iş akışında “{step_label}” adımı yok.", "step")
    state = (before.get("Durum") or "").strip() or WORKFLOW["initial"]
    if step.get("from") and state not in step["from"]:
        raise Refused("step_not_allowed", f"“{step_label}” adımı “{words(state)}” durumundaki kayıttan atılmaz.", "step")
    if "Durum" in values:
        raise Refused("form_invalid", "“Durum” alanı iş akışının adımlarıyla değişir; değerlerden çıkarın.", "values.Durum")
    form = fc.Form(TWIN_BODY, FIELDS)
    reason = step.get("reason")
    if state in WORKFLOW["locked"]:
        extra = next((k for k in sorted(values, key=lambda k: k.encode()) if k != reason), None)
        if extra is not None:
            raise Refused("record_locked", f"“{words(state)}” durumundaki kayıt değiştirilemez; bu adımla yalnız gerekçe yazılır.",
                          f"values.{extra}")
        after = dict(before)
        if reason and reason in values:
            v = values[reason].strip()
            if v:
                after[reason] = v
            else:
                after.pop(reason, None)
    else:
        after, issues = form.assess(before, fc.utf8_sorted(values), {}, False)
        wrong = [i for i in fc.errors(issues) if i.get("field") in values]
        if wrong:
            raise Refused("form_invalid", ANY, f"values.{wrong[0]['field']}")
    after["Durum"] = step["to"]
    after, worked = form.complete(after, {}, False)
    if step.get("valid"):
        issues = worked + form.validate(after, {})
        errs = fc.errors(issues)
        if errs:
            first = errs[0]
            raise Refused("form_invalid", ANY, f"values.{first['field']}" if first.get("field") in values else "step")
    if reason and not (after.get(reason) or "").strip():
        raise Refused("reason_required", f"“{step_label}” adımının gerekçesi (“{reason}”) yazılmalı.", f"values.{reason}")
    # docs/adr/0440 §4: the snapshot the step was taken on: the record, its state and log aside.
    snap = fc.snapshot(after, ("Durum", "Geçmiş", "Notlar") if noted else ("Durum", "Geçmiş") if logged else ("Durum",))
    if logged:
        # docs/adr/0429: the moment, the person, the step and its states, the reason; the oldest lines go.
        line = " · ".join([*([moment] if moment else []), *([user] if user else []),
                           f"{step_label} ({words(state)} → {words(step['to'])})"])
        if reason and (after.get(reason) or "").strip():
            line += f": {after[reason].strip()}"
        if notes:
            line += f" · {len(notes)} alan notu"
        line += f" · #{snap}"
        lines = (after.get("Geçmiş") or "").splitlines() if after.get("Geçmiş") else []
        lines.append(line)
        while len(lines) > 1 and len("\n".join(lines)) > 120:
            lines.pop(0)
        if len(lines[-1]) > 120:
            lines[-1] = lines[-1][:120]
        after["Geçmiş"] = "\n".join(lines)
    if noted:
        if notes:
            text = json.dumps({k: v.strip() for k, v in notes.items()}, ensure_ascii=False, separators=(",", ":"),
                              sort_keys=True)
            if len(text) > 160:
                raise Refused("form_invalid", f"Notlar “Notlar” alanına sığmıyor ({len(text)} karakter); notları kısaltın "
                                              "ya da alanın uzunluğunu artırın.", "notes")
            after["Notlar"] = text
        else:
            after.pop("Notlar", None)
    return state, step["to"], after, snap


def step(slot, label, values=None, now=None, op="execute", undo=False, form="denetim", draft=False, moment=None,
         user=None, notes=None):
    values = values or {}
    now = dict(now or ATTRS)
    inp = {"form": form, "uid": f"$uidOf:{slot}", "step": label}
    if draft:
        inp["draft"] = True
    if values:
        inp["values"] = values
    if moment:
        inp["moment"] = moment
    if notes:
        inp["notes"] = notes
    if user:
        inp["user"] = user
    names = {f["id"]: f["name"] for f in FORMS}
    try:
        frm, to, after, snap = take(slot, label, values, now[slot], logged=form in ("kayitli", "notlu"), moment=moment,
                                    user=user, noted=form == "notlu", notes=notes, name=names.get(form, ""))
    except Refused as r:
        expect = {"ids": IDS, "fields": {str(slot): {"attrs": now[slot]}}, "canUndo": undo, "canRedo": False,
                  "revision": "same"}
        if not undo:
            expect["dirty"] = False
        return {"op": "execute", "input": inp, "result": {"status": "failed", "error": r.error}, "expect": expect}, now
    out = {"uid": f"$uidOf:{slot}", "from": frm, "to": to, "attrs": after}
    if not draft:
        out["version"] = 1
    out["snapshot"] = snap
    out["revision"] = "$current"
    changed = after != now[slot]
    result = {"status": "completed", "output": out, "warnings": []}
    if op == "plan":
        return {"op": "plan", "input": inp, "result": result,
                "expect": {"ids": IDS, "fields": {str(slot): {"attrs": now[slot]}}, "canUndo": undo, "revision": "same"}}, now
    after_all = {**now, slot: after}
    expect = {"ids": IDS, "fields": {str(slot): {"attrs": after}}, "canUndo": changed or undo, "canRedo": False}
    expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
    return {"op": "execute", "input": inp, "result": result, "expect": expect}, after_all


def refused(name, inp, code, message, path):
    return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}},
        "expect": {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same", "dirty": False}}]}


def cases():
    out = []

    def one(name, *args, **kw):
        s, _ = step(*args, **kw)
        out.append({"name": name, "steps": [s]})

    one("geçerli taslak gönderilir; hesap tamamlanır", 1, "Gönder")
    one("zorunlu katı eksik taslak gönderilemez", 2, "Gönder")
    one("eksik kat değeriyle birlikte gönderilir", 2, "Gönder", {"Kat": "5"})
    one("taslak onaylanamaz: adım bu durumdan atılmaz", 1, "Onayla")
    one("gönderilen onaylanır ve kilitlenir", 3, "Onayla")
    one("geri göndermenin gerekçesi yazılmalı", 3, "Geri gönder")
    one("gerekçeyle geri gönderilir", 3, "Geri gönder", {"Gerekçe": "Kat sayısı yanlış."})
    one("onaylı kayıt gerekçeyle düzeltmeye açılır", 4, "Düzelt", {"Gerekçe": "Kat yeniden ölçülecek."})
    one("onaylı kayda başka değer yazılamaz", 4, "Düzelt", {"Gerekçe": "x", "Kat": "5"})
    one("her durumdan taslağa: kilitli kayıt değer almadan", 4, "Taslağa al")
    one("durumu boş kayıt ilk durumdadır", 5, "Gönder", {"Kat": "7"})
    one("durum alanı değerlerle verilemez", 1, "Gönder", {"Durum": "onaylandi"})
    one("olmayan adım", 1, "Yayınla")
    one("kurala uymayan değer", 1, "Gönder", {"Kat": "kırk"})
    one("geçerlilik istemeyen adım eksik kaydı da alır", 2, "Taslağa al")
    one("açıklama adımla birlikte yazılır", 1, "Gönder", {"Açıklama": "Ön cephe yenilendi."})
    s, _ = step(1, "Gönder", op="plan")
    out.append({"name": "plan yazmaz", "steps": [s]})
    a, now = step(3, "Geri gönder", {"Gerekçe": "Eksik ölçü."})
    b, _ = step(3, "Gönder", now=now, undo=True)
    out.append({"name": "geri gönderilen düzeltilmeden yeniden gönderilir", "steps": [a, b]})
    a, now = step(1, "Gönder")
    out.append({"name": "geri alma adımı geri getirir", "steps": [a, {"op": "undo", "expect": {
        "ids": IDS, "fields": {"1": {"attrs": ATTRS[1]}}, "canUndo": False, "canRedo": True}}]})
    s, _ = step(1, "Gönder", form="taslak", draft=True)
    out.append({"name": "yayınlanmamış formun taslağıyla", "steps": [s]})
    # docs/adr/0429: the log.
    one("geçmiş: an ve kişiyle bir satır", 1, "Gönder", form="kayitli", moment="2026-10-17T14:30:00", user="Ayşe Yılmaz")
    one("geçmiş: anı ve kişisi verilmeyen adım", 1, "Gönder", form="kayitli")
    one("geçmiş: gerekçe satırda", 3, "Geri gönder", {"Gerekçe": "Kat eksik."}, form="kayitli", user="Mehmet Kaya")
    one("geçmiş alanı değerlerle yazılamaz", 1, "Gönder", {"Geçmiş": "elle"}, form="kayitli")
    a, now = step(3, "Geri gönder", {"Gerekçe": "İlk ölçüm yanlış, yeniden ölçülmeli."}, form="kayitli",
                  moment="2026-10-17T09:00:00", user="Ayşe Yılmaz")
    b, now = step(3, "Gönder", now=now, undo=True, form="kayitli", moment="2026-10-17T11:00:00", user="Mehmet Kaya")
    c, _ = step(3, "Onayla", now=now, undo=True, form="kayitli", moment="2026-10-18T08:15:00", user="Zeynep Arslan")
    out.append({"name": "geçmiş: alan dolunca en eski satırlar gider", "steps": [a, b, c]})
    # docs/adr/0442 §4: the reviewer's notes on the record's fields.
    a, now = step(3, "Geri gönder", {"Gerekçe": "Ölçüler eksik."}, form="notlu", user="Ayşe Yılmaz",
                  notes={"Kat": "Çatı katı sayılmamış.", "Yükseklik": "  Yeniden ölçün.  "})
    b, _ = step(3, "Gönder", now=now, undo=True, form="notlu", user="Mehmet Kaya")
    out.append({"name": "notlar: geri gönderen alanlara not yazar, sonraki adım siler", "steps": [a, b]})
    one("notlar: formda olmayan alana not yazılmaz", 3, "Geri gönder", {"Gerekçe": "x"}, form="notlu",
        notes={"Yok": "x"})
    one("notlar: boş not", 3, "Geri gönder", {"Gerekçe": "x"}, form="notlu", notes={"Kat": "   "})
    one("notlar: not alanı olmayan iş akışı", 3, "Geri gönder", {"Gerekçe": "x"}, notes={"Kat": "x"})
    one("notlar: not alanı değerlerle yazılmaz", 3, "Geri gönder", {"Gerekçe": "x", "Notlar": "{}"}, form="notlu")
    one("notlar: alana sığmayan notlar", 3, "Geri gönder", {"Gerekçe": "x"}, form="notlu",
        notes={"Kat": "k" * 90, "Açıklama": "a" * 60})
    one("notlar: plan da notları gösterir", 3, "Geri gönder", {"Gerekçe": "Eksik."}, form="notlu",
        notes={"Açıklama": "Cephe yazılmamış."}, op="plan")
    out.append(refused("iş akışı olmayan form", {"form": "basit", "uid": "$uidOf:1", "step": "Gönder"}, "no_workflow",
                       "“Basit” formunun iş akışı yok; tasarımcının İş akışı penceresinden tanımlayın.", "step"))
    out.append(refused("olmayan form", {"form": "yok", "uid": "$uidOf:1", "step": "Gönder"}, "unknown_form",
                       "“yok” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.", "form"))
    out.append(refused("geçersiz kimlik", {"form": "denetim", "uid": "B-1", "step": "Gönder"}, "invalid_uid",
                       "“B-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                       "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "uid"))
    gone = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff"
    out.append(refused("çizimde olmayan kayıt", {"form": "denetim", "uid": gone, "step": "Gönder"}, "entity_not_found",
                       fc.NOT_FOUND.format(gone), "uid"))
    out.append(refused("formun katmanında olmayan kayıt", {"form": "denetim", "uid": "$uidOf:6", "step": "Gönder"},
                       "form_layer_mismatch", "Nesne “Bina denetimi” formunun katmanında değil. Formun katmanındaki bir "
                       "nesneyi seçin.", "uid"))
    out.append(refused("kilitli katman", {"form": "kilitli", "uid": "$uidOf:7", "step": "Gönder"}, "layer_locked",
                       "“Kilit” katmanı kilitli; kayıt değiştirilemez. Kilidi Katmanlar panelinden açın.", "uid"))
    out.append(refused("yayınlanmamış form taslaksız", {"form": "taslak", "uid": "$uidOf:1", "step": "Gönder"},
                       "form_unpublished", "“Taslak akış” formu hiç yayınlanmadı. Önce yayınlayın ya da taslağıyla çalışın.",
                       "draft"))
    out.append(refused("geçersiz beklenen sürüm", {"form": "denetim", "uid": "$uidOf:1", "step": "Gönder",
                                                    "expectedRevision": "07"}, "invalid_revision", fc.REVISION,
                       "expectedRevision"))
    out.append(refused("iş akışı katmana uymayan form", {"form": "bozuk", "draft": True, "uid": "$uidOf:1", "step": "Gönder"},
                       "form_preflight", "“Bozuk akış” formu kullanılamıyor: İş akışının durum alanı (“Yok”) katmanda yok. "
                       "Formu tasarımcıda düzeltin.", "form"))
    out.append(refused("geçersiz an", {"form": "denetim", "uid": "$uidOf:1", "step": "Gönder", "moment": "dün"},
                       "invalid_moment", "“dün” bir an değil. Bir tarih (2026-10-16) ya da tarih ve yerel saat "
                       "(2026-10-16T14:30:00) yazın.", "moment"))
    out.append(refused("geçersiz kullanıcı", {"form": "denetim", "uid": "$uidOf:1", "step": "Gönder", "user": " Ayşe"},
                       "invalid_user", "Kullanıcının adı boş olamaz, başında ve sonunda boşluk olmaz, en çok 64 karakterdir.",
                       "user"))
    conflict = refused("çakışan beklenen sürüm", {"form": "denetim", "uid": "$uidOf:1", "step": "Gönder",
                                                   "expectedRevision": "999"}, "revision_conflict", fc.CONFLICT,
                       "expectedRevision")
    conflict["steps"][0]["result"] = fc.failed("revision_conflict", fc.CONFLICT, "expectedRevision", "conflict")
    out.append(conflict)
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.transition", "commandVersion": 1,
            "title": "İş akışı adımı",
            "note": "ADR 0428: kaydın formun iş akışında bir adım ilerlemesi. Written by "
                    "scripts/fixtures/forms_transition_cases.py with the form twin of forms_cases.py, without KentOS code; "
                    "the form's sentences are the core's ($any).", "setup": setup(), "cases": cases()}


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    n = len(json.loads(text)["cases"])
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({n} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({n} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
