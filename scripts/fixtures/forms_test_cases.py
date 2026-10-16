"""Writes fixtures/commands/v1/cad.forms.test.json: the shared cases of `cad.forms.test` v1 (Form senaryolarını çalıştır,
docs/adr/0420), their expectations built here without KentOS code: a scenario's outcome is the form's (the independent
form twin of scripts/fixtures/forms_cases.py: `Form.assess` onto an empty record, `Form.described`), compared with what the
scenario expects, its differences in the ADR's words and order. The desktop (crates/native/application/tests/all/
fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/forms_test_cases.py           # writes the file
    python3 scripts/fixtures/forms_test_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the form twin, written from the ADRs without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.forms.test.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

FIELDS = [
    {"name": "Durum", "kind": "text", "values": [{"code": "iyi", "label": "İyi"}, {"code": "inceleme", "label": "Ek inceleme gerekli"}],
     "required": True},
    {"id": "kat-sayisi", "name": "Kat", "kind": "integer", "min": "1", "max": "40"},
    {"name": "Açıklama", "kind": "text", "length": 500},
    {"name": "Yükseklik", "kind": "decimal", "scale": 1},
    {"name": "Kod", "kind": "text"},
]
ITEMS = [
    {"type": "field", "id": "durum", "field": "Durum", "required": True},
    {"type": "field", "id": "kat", "field": "kat-sayisi"},
    {"type": "field", "id": "kod", "field": "Kod", "defaultExpr": "'B-' || Kat"},
    {"type": "field", "id": "yukseklik", "field": "Yükseklik", "calculate": "Kat * 3"},
    {"type": "field", "id": "aciklama", "field": "Açıklama", "visibleWhen": "Durum = 'inceleme'",
     "requiredWhen": "Durum = 'inceleme'",
     "checks": [{"rule": "uzunluk(Açıklama) >= 10", "message": "Açıklama en az 10 karakter olmalı."}]},
]


def outcome(items, values, new):
    """What the form makes of `values` on an empty record (docs/adr/0420 §1)."""
    form = fc.Form({"revision": 1, "items": items}, FIELDS)
    after, issues = form.assess({}, [[k, v] for k, v in values.items()], None, new)
    states = form.described(after, None)
    expect = {}
    if after:
        expect["attrs"] = dict(sorted(after.items()))
    for key, word in (("shown", "shown"), ("required", "required"), ("readOnly", "readOnly")):
        ids = [s["id"] for s in states if s.get(word)]
        if ids:
            expect[key] = ids
    said = [f"{i['code']}@{i['item']}" if "item" in i else i["code"] for i in issues]
    if said:
        expect["issues"] = said
    return expect


def test(name, values, new=False, items=ITEMS, change=None):
    t = {"name": name}
    if values:
        t["values"] = dict(sorted(values.items()))
    if new:
        t["new"] = True
    expect = outcome(items, values, new)
    if change:
        change(expect)
    t["expect"] = expect
    return t


def differences(items, t):
    got = outcome(items, t.get("values", {}), t.get("new", False))
    want = t["expect"]
    out = []
    wa, ga = want.get("attrs", {}), got.get("attrs", {})
    for name in sorted(set(wa) | set(ga)):
        if wa.get(name) != ga.get(name):
            say = lambda v: "boş" if v is None else f"“{v}”"  # noqa: E731
            out.append(f"“{name}” değeri: beklenen {say(wa.get(name))}, bulunan {say(ga.get(name))}.")
    lst = lambda xs: ", ".join(xs) if xs else "hiçbiri"  # noqa: E731
    for words, key in (("Gösterilen öğeler", "shown"), ("Zorunlu öğeler", "required"), ("Salt okunur öğeler", "readOnly"),
                       ("Sorunlar", "issues")):
        if want.get(key, []) != got.get(key, []):
            out.append(f"{words}: beklenen {lst(want.get(key, []))}, bulunan {lst(got.get(key, []))}.")
    return out


PASSING = [
    test("iyi bina", {"Durum": "iyi", "kat-sayisi": "3"}),
    test("yeni kayıt varsayılanlarıyla", {"Durum": "iyi", "kat-sayisi": "4"}, new=True),
    test("incelemede açıklama zorunlu", {"Durum": "inceleme", "kat-sayisi": "2"}),
    test("kısa açıklama denetime takılır", {"Durum": "inceleme", "kat-sayisi": "2", "Açıklama": "Kısa"}),
    test("boş kayıt", {}),
]
FAILING = [
    test("eski hesap", {"Durum": "iyi", "kat-sayisi": "3"}, change=lambda e: e["attrs"].update({"Yükseklik": "6"})),
    test("açıklama görünmüyor sanılmış", {"Durum": "inceleme", "kat-sayisi": "2"},
         change=lambda e: e.update(shown=[x for x in e["shown"] if x != "aciklama"])),
    test("zorunluluk eksik sanılmış", {"Durum": "inceleme", "kat-sayisi": "2"}, change=lambda e: e.update(required=["durum"])),
    test("sorun beklenmemiş", {"Durum": "inceleme", "kat-sayisi": "2", "Açıklama": "Kısa"}, change=lambda e: e.pop("issues")),
]
DRAFT = {"revision": 2, "items": ITEMS, "tests": PASSING + FAILING}
RELEASE = {"revision": 1, "items": ITEMS, "tests": PASSING[:2]}


def digest(body):
    return fc.digest(body)


FORMS = [
    {"id": "bina", "name": "Bina denetimi", "layer": "bina", "draft": DRAFT,
     "releases": [{"version": 1, "digest": digest(RELEASE), "body": RELEASE}]},
    {"id": "taslak", "name": "Taslak form", "layer": "bina", "draft": {"revision": 0, "items": ITEMS[:2], "tests": [
        test("durum ve kat", {"Durum": "iyi", "kat-sayisi": "1"}, items=ITEMS[:2])]}},
    {"id": "bozuk", "name": "Bozuk form", "layer": "bina", "draft": {"revision": 0, "items": [
        {"type": "field", "id": "yok", "field": "Yok"}]}},
]


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Form senaryoları",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "forms": copy.deepcopy(FORMS)},
            "origin": {"x": 500000, "y": 4400000},
            "layers": [{"id": "bina", "name": "Bina", "type": "layer", "visible": True, "locked": False, "expanded": True,
                        "style": copy.deepcopy(STYLE), "children": [], "fields": copy.deepcopy(FIELDS)}],
            "activeLayer": "bina", "entities": [], "styles": {"items": [], "categories": []}}


UNCHANGED = {"ids": [], "canUndo": False, "dirty": False, "revision": "same"}


def run(items, tests):
    results = []
    for t in tests:
        d = differences(items, t)
        r = {"name": t["name"], "passed": not d}
        if d:
            r["differences"] = d
        results.append(r)
    return {"results": results, "passed": sum(r["passed"] for r in results), "total": len(results)}


def ok(name, input_, items, tests, op="execute"):
    return {"name": name, "steps": [{"op": op, "input": input_,
                                     "result": {"status": "completed", "output": run(items, tests), "warnings": []},
                                     "expect": UNCHANGED}]}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [{"op": "execute", "input": input_,
                                     "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                                     "expect": UNCHANGED}]}


def cases():
    by = {t["name"]: t for t in DRAFT["tests"]}
    out = [
        ok("taslağın bütün senaryoları: geçenler ve farklarıyla düşenler", {"form": "bina", "draft": True}, ITEMS, DRAFT["tests"]),
        ok("son yayının senaryoları yayınla", {"form": "bina"}, ITEMS, RELEASE["tests"]),
        ok("plan aynı cevabı verir", {"form": "bina", "draft": True}, ITEMS, DRAFT["tests"], op="plan"),
        ok("yalnız adı verilenler, verilen sırayla", {"form": "bina", "draft": True, "names": ["eski hesap", "iyi bina"]},
           ITEMS, [by["eski hesap"], by["iyi bina"]]),
        ok("hesap farkı söylenir", {"form": "bina", "draft": True, "names": ["eski hesap"]}, ITEMS, [by["eski hesap"]]),
        ok("gösterilen öğe farkı", {"form": "bina", "draft": True, "names": ["açıklama görünmüyor sanılmış"]}, ITEMS,
           [by["açıklama görünmüyor sanılmış"]]),
        ok("zorunlu öğe farkı", {"form": "bina", "draft": True, "names": ["zorunluluk eksik sanılmış"]}, ITEMS,
           [by["zorunluluk eksik sanılmış"]]),
        ok("sorun farkı", {"form": "bina", "draft": True, "names": ["sorun beklenmemiş"]}, ITEMS, [by["sorun beklenmemiş"]]),
        ok("varsayılanlarla yeni kayıt geçer", {"form": "bina", "draft": True, "names": ["yeni kayıt varsayılanlarıyla"]}, ITEMS,
           [by["yeni kayıt varsayılanlarıyla"]]),
        ok("boş kayıt geçer", {"form": "bina", "draft": True, "names": ["boş kayıt"]}, ITEMS, [by["boş kayıt"]]),
        ok("yayınlanmamış formun taslağı", {"form": "taslak", "draft": True}, ITEMS[:2], FORMS[1]["draft"]["tests"]),
        ok("aynı ad iki kez: iki kez çalışır", {"form": "bina", "draft": True, "names": ["iyi bina", "iyi bina"]}, ITEMS,
           [by["iyi bina"], by["iyi bina"]]),
        ok("senaryosu olmayan yayın: boş sonuç", {"form": "bina", "names": []}, ITEMS, RELEASE["tests"]),
    ]
    out.append({"name": "doğrulama: geçerli girdi hiçbir şey yazmaz", "steps": [
        {"op": "validate", "input": {"form": "bina", "draft": True},
         "result": {"status": "completed", "output": None, "warnings": []}, "expect": UNCHANGED}]})
    out.append(refused("olmayan form", {"form": "yok"}, "unknown_form",
                       "“yok” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.", "form"))
    out.append(refused("yayınlanmamış formun yayını", {"form": "taslak"}, "form_unpublished",
                       "“Taslak form” formu hiç yayınlanmadı. Önce yayınlayın ya da taslağıyla çalışın.", "draft"))
    out.append(refused("ön denetimde hatası olan form", {"form": "bozuk", "draft": True}, "form_preflight",
                       "“Bozuk form” formu kullanılamıyor: Katmanda “Yok” alanı yok. Formu tasarımcıda düzeltin.", "form"))
    out.append(refused("olmayan senaryo", {"form": "bina", "draft": True, "names": ["iyi bina", "yok"]}, "unknown_test",
                       "“Bina denetimi” formunda “yok” senaryosu yok.", "names/1"))
    out.append(refused("yayının senaryosu olmayan ad", {"form": "bina", "names": ["eski hesap"]}, "unknown_test",
                       "“Bina denetimi” formunda “eski hesap” senaryosu yok.", "names/0"))
    out.append(refused("olmayan form, ad da yanlış: önce form", {"form": "yok", "names": ["x"]}, "unknown_form",
                       "“yok” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.", "form"))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.test", "commandVersion": 1,
            "title": "Form senaryolarını çalıştır",
            "note": "Written by scripts/fixtures/forms_test_cases.py from docs/adr/0420; do not edit by hand.",
            "setup": setup(), "cases": cases()}


def main() -> int:
    assert all(not differences(ITEMS, t) for t in PASSING) and all(differences(ITEMS, t) for t in FAILING)
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({len(build()['cases'])} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
