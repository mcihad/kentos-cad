#!/usr/bin/env python3
"""A new record's defaults (docs/adr/0752): a form's new record takes the layer fields' defaults where the form gives
none, as a drawing command fills them, every field of the layer, shown or not; the form's own default (a value or an
expression) comes before the subtype's and the field's everywhere, a field its expression left empty takes the
field's after all. Shared cases for `cad.forms.validate` with `defaults` and for `cad.polygon.create` on a layer whose
form enforces its rules. The rule and the answers are written here without KentOS code, with the form twin of
forms_cases.py and the subtypes' twin of subtypes_cases.py.

    python3 scripts/fixtures/form_defaults_cases.py          # writes the files
    python3 scripts/fixtures/form_defaults_cases.py --check  # compares them
"""

import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the form twin, written from the ADRs without KentOS code)
import subtypes_cases as st  # noqa: E402  (the subtypes' twin)

OUT = ROOT / "fixtures/commands/v1"


def choice(code, label):
    return {"code": code, "label": label}


# A district layer like the owner's (docs/adr/0750): literal defaults on most fields, none on the code and the note.
FIELDS = [
    {"name": "Kod", "kind": "text"},
    {"name": "Tür", "kind": "text", "default": "İlçe"},
    {"name": "Durum", "kind": "text", "values": [choice("taslak", "Taslak"), choice("etkin", "Etkin"),
                                                  choice("kapali", "Kapalı")], "default": "etkin"},
    {"name": "Merkez", "kind": "boolean", "default": "false"},
    {"name": "Aktif", "kind": "boolean", "default": "true"},
    {"name": "Kat", "kind": "integer", "default": "3"},
    {"name": "Oran", "kind": "decimal", "default": "0.5"},
    {"name": "Kuruluş", "kind": "date", "default": "2026-01-01"},
    {"name": "Not", "kind": "text"},
]
LAYER_DEFAULTS = {f["name"]: f["default"] for f in FIELDS if "default" in f}
REQUIRED = [dict(f, required=True) if f["name"] in ("Kod", "Durum") else f for f in FIELDS]
SUBTYPES = {"field": "Durum", "types": [
    {"code": "etkin", "fields": [{"field": "Kat", "default": "8"}]},
    {"code": "kapali", "fields": [{"field": "Kat", "default": "1"}]},
]}

E0, N0 = fc.E0, fc.N0
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "bina", "attrs": {"Kod": "A-1", "Tür": "Belde", "Durum": "taslak"},
     "pts": fc.square(0, 0)},
    {"kind": "polygon", "id": 2, "layerId": "bina", "attrs": {}, "pts": fc.square(20, 0)},
    {"kind": "line", "id": 3, "layerId": "yol", "attrs": {"Ad": "Atatürk Caddesi"}, "a": fc.P(E0 - 10, N0 - 5),
     "b": fc.P(E0 + 60, N0 - 5)},
]
IDS = [e["id"] for e in ENTITIES]
SQUARE = {"length": 40, "area": 100, "vertices": 4}
# The moment a command gives (`moment`): what `@tarih` reads.
GIVEN = [{"name": "tarih", "value": "2026-10-16"}, {"name": "simdi", "value": "2026-10-16T14:30:00"}]


def field(fid, name, **more):
    return {"type": "field", "id": fid, "field": name, **more}


def body(*changes):
    """The district form: the code, the kind, the state, the centre, the floors and the note; Aktif, Oran and
    Kuruluş not on it. `changes` are (item id, its new keys)."""
    items = [field("kod", "Kod"), field("tur", "Tür"), field("durum", "Durum"), field("merkez", "Merkez"),
             field("kat", "Kat"), field("not", "Not")]
    out = {"revision": 1, "items": items}
    for iid, more in changes:
        if iid == "body":
            out.update(more)
            continue
        hit = next((x for x in items if x["id"] == iid), None)
        if hit is None:
            items.append(more)
        else:
            hit.update(more)
    return out


def form(b, enforce=False, draft=None, fid="ilce"):
    out = {"id": fid, "name": "İlçe", "layer": "bina", "draft": copy.deepcopy(draft or b),
           "releases": [fc.release_of(b, 1)]}
    if enforce:
        out["enforce"] = True
    return out


def setup(forms=(), fields=FIELDS, subtypes=None):
    s = fc.setup(list(forms), copy.deepcopy(fields), copy.deepcopy(ENTITIES))
    s["name"] = "İlçeler"
    if subtypes:
        s["layers"][0]["subtypes"] = copy.deepcopy(subtypes)
    return s


def twin(b, fields, subtypes, variables=()):
    if subtypes:
        _, sub_form = st.form_twin()
        return sub_form(b, fields, subtypes, variables)
    return fc.Form(b, fields, variables)


# ---- cad.forms.validate: a new record (`defaults`) ----

def validate_cases():
    cases = []

    def run(name, b, values=(), slot=2, defaults=True, fields=FIELDS, subtypes=None, moment=False, draft=None):
        f = form(b, draft=draft)
        asked = draft if draft is not None else b
        variables = GIVEN if moment else ()
        issues = fc.preflight(asked, fields, variables)
        out = {"issues": [], "valid": True, "items": []}
        if draft is None:
            out["version"] = 1
        before = next(e["attrs"] for e in ENTITIES if e["id"] == slot)
        t = twin(asked, fields, subtypes, variables)
        after, found = t.assess(dict(before), fc.utf8_sorted(dict(values)), SQUARE, defaults)
        out["items"] = t.described(after, SQUARE)
        issues = issues + found + t.unique_issues(after, SQUARE, ())
        out["attrs"] = after
        out["issues"] = fc.shown_issues(issues)
        out["valid"] = not fc.errors(issues)
        inp = {"form": "ilce", "uid": f"$uidOf:{slot}"}
        if values:
            inp["values"] = dict(values)
        if defaults:
            inp["defaults"] = True
        if moment:
            inp["moment"] = "2026-10-16T14:30:00"
        if draft is not None:
            inp["draft"] = True
        cases.append({"name": name, "setup": setup([f], fields, subtypes), "steps": [{
            "op": "execute", "input": inp,
            "result": {"status": "completed", "output": fc.with_requires(out, asked), "warnings": []},
            "expect": {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}}]})

    plain = body()
    run("yeni kayıt alanların varsayılanlarını alır: formda olmayan Aktif, Oran ve Kuruluş da", plain)
    run("verilen değer alanın varsayılanının önünde", plain, {"Tür": "Belde", "Kat": "5"})
    run("boş verilen değer: alanın varsayılanı gelir", plain, {"Tür": "", "Kod": "B-7"})
    run("var olan kayıt (varsayılansız): alanların varsayılanları yazılmaz", plain, defaults=False)
    run("dolu kayıt varsayılanlarla: dolu alanlar kalır, boşlar dolar", plain, slot=1)
    run("formun varsayılanı alanınkinin önünde", body(("tur", {"default": "Belde"})))
    run("formun evet/hayır varsayılanı alanınkinin önünde", body(("merkez", {"default": "true"})))
    run("formun ifadesi alanınkinin önünde: Kat 4 + 3", body(("kat", {"defaultExpr": "4 + 3"})))
    run("ifade alanın varsayılanını okur: Tür || ' kaydı'", body(("not", {"defaultExpr": "Tür || ' kaydı'"})))
    run("ifade boş verirse alanın varsayılanı gelir", body(("durum", {"defaultExpr": "''"})))
    run("anı okuyan ifade, an verilmezse: alanın varsayılanı",
        body(("kurulus", field("kurulus", "Kuruluş", defaultExpr="@tarih"))))
    run("anı okuyan ifade, an verilince: anın günü",
        body(("kurulus", field("kurulus", "Kuruluş", defaultExpr="@tarih"))), moment=True)
    run("formda olmayan ondalık ve tarih alanları da varsayılanlarını alır", body(("not", {"label": "Açıklama"})))
    run("salt okunur alan varsayılanını alır", body(("tur", {"readOnly": True})))
    run("hesaplanan alan varsayılanın yerine hesabını alır", body(("kat", {"calculate": "2 * 2"})))
    run("zorunlu varsayılansız alan sorun, varsayılanlı zorunlu alan değil", plain, fields=REQUIRED)
    run("zorunlu alana değer verilince sorun kalkar", plain, {"Kod": "B-9"}, fields=REQUIRED)
    run("gizlenen alan gizlenenleri silmeyen formda varsayılanını tutar",
        body(("oran", field("oran", "Oran", visibleWhen="Durum = 'kapali'"))))
    run("gizlenenleri silen form: gizlenen alanın varsayılanı silinir",
        body(("body", {"clearHidden": True}), ("oran", field("oran", "Oran", visibleWhen="Durum = 'kapali'"))))
    run("gizlenenleri silen form: görünen alan varsayılanını tutar",
        body(("body", {"clearHidden": True}), ("oran", field("oran", "Oran", visibleWhen="Durum = 'kapali'"))),
        {"Durum": "kapali"})
    run("taslak sorulunca taslağın varsayılanı", plain, draft=body(("tur", {"default": "Köy"})))
    run("yayın sorulunca yayının: taslağın varsayılanı yok sayılır", plain)
    run("alt türün varsayılanı alanınkinin önünde: etkin kayıt Kat 8", plain, subtypes=SUBTYPES)
    run("verilen alt tür kendi varsayılanını getirir: kapalı kayıt Kat 1", plain, {"Durum": "kapali"},
        subtypes=SUBTYPES)
    run("formun ifadesi alt türünkinin önünde", body(("kat", {"defaultExpr": "4 + 3"})), subtypes=SUBTYPES)
    return cases


# ---- cad.polygon.create on a layer whose form enforces its rules ----

def rect(x, y, w, h):
    return [fc.P(E0 + x, N0 + y), fc.P(E0 + x + w, N0 + y), fc.P(E0 + x + w, N0 + y + h), fc.P(E0 + x, N0 + y + h)]


def walk(items):
    """A form's items, a container's after it."""
    for x in items:
        yield x
        yield from walk(x.get("items", []))


def form_plain(b):
    """A form's plain defaults by its fields' keys, in its order (the first one a field's)."""
    out = {}
    for x in walk(b["items"]):
        if x.get("type") == "field" and not x.get("mirror") and x.get("default"):
            out.setdefault(x["field"], x["default"])
    return out


def form_expressions(b):
    """The keys of the fields a form defaults by an expression."""
    return {x["field"] for x in walk(b["items"]) if x.get("type") == "field" and not x.get("mirror")
            and not x.get("default") and "defaultExpr" in x}


def drawn(given, b, enforce, fields, subtypes):
    """What a drawing command writes (docs/adr/0199 §2, 0402 §4, 0752): the values given; then, as the form fills a
    new record, the plain defaults of a form the layer enforces, then the subtype's and the fields' defaults but those
    the form gives by an expression; that form then completes the record in the same step, without the moment (an
    expression reading it is left out) and gives a field its expression left empty the field's default."""
    out = {k: v.strip(fc.JS_SPACE) for k, v in given.items() if v.strip(fc.JS_SPACE)}
    skip = set()
    if enforce:
        for key, d in form_plain(b).items():
            f = next(x for x in fields if fc.key_of(x) == key)
            if not out.get(f["name"], "").strip(fc.JS_SPACE):
                v, why = fc.check_value(dict(f, required=False), d)
                if not why and v:
                    out[f["name"]] = v
        skip = form_expressions(b)
    t = twin(body(), fields, subtypes)
    t.layer_defaults(out, skip)
    if enforce:
        out, _ = twin(b, fields, subtypes).complete(out, SQUARE, True)
    return out


def create_cases():
    cases = []

    def made(name, b=None, enforce=True, attrs=None, fields=FIELDS, subtypes=None, draft=None, layer_id="bina"):
        inp = {"layerId": layer_id, "pts": rect(100, 0, 10, 10)}
        if attrs is not None:
            inp["attrs"] = attrs
        forms = [] if b is None else [form(b, enforce, draft)]
        if layer_id == "bina":
            out = drawn(attrs or {}, b or body(), enforce and b is not None, fields, subtypes)
        else:
            out = {k: v for k, v in (attrs or {}).items() if v.strip()}
        entity = {"kind": "polygon", "id": 4, "layerId": layer_id, "attrs": out, "pts": rect(100, 0, 10, 10)}
        cases.append({"name": name, "setup": setup(forms, fields, subtypes), "steps": [{
            "op": "execute", "input": inp,
            "result": {"status": "completed", "output": {"uid": "$uid", "id": 4, "revision": "$current"}, "warnings": []},
            "expect": {"ids": IDS + [4], "entities": {"4": entity}, "canUndo": True, "canRedo": False, "dirty": True,
                       "revision": "changed"}}]})

    plain = body()
    made("formsuz katman: alanların varsayılanları", None)
    made("kuralları uygulanmayan formun varsayılanı çizimde kullanılmaz", body(("tur", {"default": "Belde"})),
         enforce=False)
    made("kuralları uygulanan, kendi varsayılanı olmayan form: alanların varsayılanları", plain)
    made("kuralları uygulanan formun varsayılanı alanınkinin önünde", body(("tur", {"default": "Belde"})))
    made("formun evet/hayır varsayılanı alanınkinin önünde", body(("merkez", {"default": "true"})))
    made("formun tarih varsayılanı alanınkinin önünde",
         body(("kurulus", field("kurulus", "Kuruluş", default="2026-05-19"))))
    made("formun ondalık varsayılanı alanınkinin önünde", body(("oran", field("oran", "Oran", default="0.75"))))
    made("formun ifadesi alanınkinin önünde: Kat 7", body(("kat", {"defaultExpr": "4 + 3"})))
    made("ifade alanın varsayılanını okur: İlçe kaydı", body(("not", {"defaultExpr": "Tür || ' kaydı'"})))
    made("ifade boş verirse alanın varsayılanı gelir", body(("durum", {"defaultExpr": "''"})))
    made("anı okuyan ifade çizimde bekler: alanın varsayılanı",
         body(("kurulus", field("kurulus", "Kuruluş", defaultExpr="@tarih"))))
    made("verilen değer her varsayılanın önünde", body(("tur", {"default": "Belde"})), attrs={"Tür": "Köy", "Kat": "9"})
    made("verilen boş değer: formun varsayılanı gelir", body(("tur", {"default": "Belde"})), attrs={"Tür": ""})
    made("yayında varsayılanı olmayan form: taslağınki çizimde yok", plain, draft=body(("tur", {"default": "Köy"})))
    made("salt okunur alan alanın varsayılanını alır", body(("tur", {"readOnly": True})))
    made("hesaplanan alan hesabını alır", body(("kat", {"calculate": "2 * 2"})))
    made("gizlenenleri silen form: gizlenen alanın varsayılanı çizimde silinir",
         body(("body", {"clearHidden": True}), ("oran", field("oran", "Oran", visibleWhen="Durum = 'kapali'"))))
    made("gizlenenleri silen form: verilen durumla görünen alan varsayılanını tutar",
         body(("body", {"clearHidden": True}), ("oran", field("oran", "Oran", visibleWhen="Durum = 'kapali'"))),
         attrs={"Durum": "kapali"})
    made("alt türün varsayılanı alanınkinin önünde: Kat 8", plain, subtypes=SUBTYPES)
    made("formun ifadesi alt türünkinin önünde: Kat 7", body(("kat", {"defaultExpr": "4 + 3"})), subtypes=SUBTYPES)
    made("formun varsayılan durumu alt türünü seçer: kapalı, Kat 1", body(("durum", {"default": "kapali"})),
         subtypes=SUBTYPES)
    made("başka katmana çizilen alan formdan ve alanlardan etkilenmez", body(("tur", {"default": "Belde"})),
         attrs={"Ad": "Cadde"}, layer_id="yol")
    return cases


def command_file(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note + " Kural ve beklenenler scripts/fixtures/form_defaults_cases.py'de KentOS kodu olmadan, "
                           "forms_cases.py'nin form ikiziyle yazılmıştır; sorunların cümleleri çekirdeğindir ($any).",
            "setup": setup(), "cases": cases}


def outputs():
    return {
        OUT / "cad.forms.validate.defaults.json": command_file(
            "cad.forms.validate", "Formu denetle: yeni kaydın varsayılanları",
            "ADR 0752: yeni kayıt, formun kendi varsayılanı olmayan her alanında katman alanının (alt türlü katmanda alt "
            "türünün) varsayılanını alır; formun varsayılanı ve ifadesi önce gelir, ifadenin boş bıraktığı alan "
            "alanınkini alır.", validate_cases()),
        OUT / "cad.polygon.create.formdefaults.json": command_file(
            "cad.polygon.create", "Alan çiz: kuralları uygulanan formun varsayılanları",
            "ADR 0752: çizilen yeni kayıt, kuralları uygulanan formun kendi varsayılanı olan alanında formunkini, "
            "öbürlerinde alt türün ve alanın varsayılanını alır; an çizimde verilmez.", create_cases()),
    }


def main() -> int:
    stale = False
    for path, data in outputs().items():
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        n = len(data["cases"])
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın ve farkı okuyun.",
                      file=sys.stderr)
                stale = True
            else:
                print(f"{path.relative_to(ROOT)}: {n} durum, güncel.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.relative_to(ROOT)}: {n} durum")
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main())
