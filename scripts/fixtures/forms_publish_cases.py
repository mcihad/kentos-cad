"""Writes fixtures/commands/v1/cad.forms.edit.publish.json: the shared cases of `cad.forms.edit`'s publish (docs/adr/0442
§1, §2), their expectations built here without KentOS code: a publish runs the draft's scenarios (the scenario twin of
scripts/fixtures/forms_test_cases.py) and refuses at the first failing one, its first difference said; over an earlier
release it counts the layer's records the earlier release held and the new one does not (the form twin of forms_cases.py:
completed as kept, then validated; unique values aside), by the fields the new release finds wrong.

    python3 scripts/fixtures/forms_publish_cases.py           # writes the file
    python3 scripts/fixtures/forms_publish_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the form twin, written from the ADRs without KentOS code)
import forms_test_cases as ft  # noqa: E402  (the scenario twin)

OUT = ROOT / "fixtures/commands/v1/cad.forms.edit.publish.json"
FIELDS = ft.FIELDS
ITEMS = ft.ITEMS

ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "bina", "attrs": {"Durum": "iyi", "Kat": "3"}, "pts": fc.square(0, 0)},
    {"kind": "polygon", "id": 2, "layerId": "bina",
     "attrs": {"Durum": "inceleme", "Kat": "2", "Açıklama": "Çatı katı ayrıca ölçülmeli."}, "pts": fc.square(20, 0)},
    {"kind": "polygon", "id": 3, "layerId": "bina", "attrs": {"Durum": "iyi"}, "pts": fc.square(40, 0)},
    {"kind": "polygon", "id": 4, "layerId": "bina", "attrs": {"Kod": "B-4"}, "pts": fc.square(60, 0)},
]
OBJECTS = {e["id"]: fc.SQUARE for e in ENTITIES}


def body(items, tests=None, revision=1):
    out = {"revision": revision, "items": copy.deepcopy(items)}
    if tests:
        out["tests"] = copy.deepcopy(tests)
    return out


def items_with(change):
    xs = copy.deepcopy(ITEMS)
    change(xs)
    return xs


def item(xs, i):
    return next(x for x in xs if x["id"] == i)


KAT_REQUIRED = items_with(lambda xs: item(xs, "kat").update(required=True))
LONGER_NOTE = items_with(lambda xs: (item(xs, "kat").update(required=True), item(xs, "aciklama")["checks"][0].update(
    rule="uzunluk(Açıklama) >= 30", message="Açıklama en az 30 karakter olmalı.")))
WITH_NOTE = ITEMS + [{"type": "note", "id": "bilgi", "text": "Kat sayısı zemin dahil."}]
RELEASED = body(ITEMS)


def form(draft, releases=()):
    out = {"id": "bina", "name": "Bina denetimi", "layer": "bina", "draft": copy.deepcopy(draft)}
    if releases:
        out["releases"] = [fc.release_of(b, k + 1) for k, b in enumerate(releases)]
    return out


def project(forms):
    return fc.Project(forms, fields_of={"bina": FIELDS}, entities=ENTITIES, objects=OBJECTS)


def setup(forms):
    return fc.setup(forms, FIELDS, ENTITIES)


def publish(p, **more):
    return p.step({"kind": "publish", "id": "bina", **more})


def planned(p, **more):
    """The publish as plan says it: the same answer, nothing written."""
    before = copy.deepcopy(p.forms)
    s = publish(p, **more)
    p.forms = before
    s["op"] = "plan"
    s["expect"] = {"ids": fc.IDS, "settings": {"forms": [fc.form_json(f) for f in before]}, "canUndo": False,
                   "revision": "same"}
    return s


def failing(p, test):
    first = ft.differences(ITEMS, test)[0]
    return p.refused({"kind": "publish", "id": "bina"}, fc.failed(
        "form_tests_failed", f"“Bina denetimi” formu yayınlanamıyor: “{test['name']}” senaryosu tutmuyor ({first}). "
                             "Senaryoyu ya da formu düzeltin.", "edit.id"))


def cases():
    out = []

    def case(name, forms, steps_of):
        p = project(forms)
        out.append({"name": name, "setup": setup(forms), "steps": steps_of(p)})

    passing = body(ITEMS, ft.PASSING, 2)
    case("senaryoları geçen taslak yayınlanır", [form(passing)], lambda p: [publish(p)])
    broken_tests = body(ITEMS, ft.PASSING + ft.FAILING[:1], 2)
    case("düşen senaryo yayını durdurur, ilk farkıyla", [form(broken_tests)], lambda p: [failing(p, ft.FAILING[0])])
    later = body(ITEMS, ft.PASSING[:1] + ft.FAILING[1:3], 2)
    case("senaryolar sırayla: ilk düşen söylenir", [form(later)], lambda p: [failing(p, ft.FAILING[1])])
    released_failing = body(ITEMS, ft.FAILING[:1], 2)
    case("son yayınla aynı taslak yeniden çalıştırılmaz, yayınlanmaz", [form(released_failing, [released_failing])],
         lambda p: [publish(p)])
    case("senaryosuz taslak yalnız ön denetimle yayınlanır", [form(body(ITEMS, revision=2))], lambda p: [publish(p)])
    case("önceki yayına uyan kayıt yeni yayına uymuyor: uyarı ve sayılar", [form(body(KAT_REQUIRED, revision=2),
                                                                                [RELEASED])], lambda p: [publish(p)])
    case("iki alan kırılır: alanlar baytlarının sırasıyla", [form(body(LONGER_NOTE, revision=2), [RELEASED])],
         lambda p: [publish(p)])
    case("plan kırılan kayıtları yazmadan söyler", [form(body(KAT_REQUIRED, revision=2), [RELEASED])],
         lambda p: [planned(p), publish(p)])
    case("kırmayan değişiklik uyarmaz", [form(body(WITH_NOTE, revision=2), [RELEASED])], lambda p: [publish(p)])
    case("ilk yayın kırılma saymaz: önceki yok", [form(body(KAT_REQUIRED, revision=2))], lambda p: [publish(p)])
    case("senaryo kırılmadan önce denetlenir", [form(body(KAT_REQUIRED, ft.FAILING[:1], 2), [RELEASED])],
         lambda p: [failing(p, ft.FAILING[0])])
    case("ikinci yayın kırılanları birinciye göre değil son yayına göre sayar",
         [form(body(LONGER_NOTE, revision=3), [RELEASED, body(KAT_REQUIRED, revision=2)])], lambda p: [publish(p)])
    case("yayın notu senaryolar geçince yazılır", [form(passing)], lambda p: [publish(p, note="Senaryolar geçti")])
    case("sorun farkıyla düşen senaryo", [form(body(ITEMS, ft.FAILING[3:4], 2))], lambda p: [failing(p, ft.FAILING[3])])
    case("zorunluluk farkıyla düşen senaryo", [form(body(ITEMS, ft.FAILING[2:3], 2))],
         lambda p: [failing(p, ft.FAILING[2])])
    case("yeni kayıt senaryosu varsayılanlarıyla geçer", [form(body(ITEMS, ft.PASSING[1:2], 2))], lambda p: [publish(p)])
    case("taslak güncellenip yayınlanınca kırılanlar güncel taslağa göre",
         [form(body(ITEMS, revision=1), [RELEASED])],
         lambda p: [p.step({"kind": "update", "id": "bina", "body": body(KAT_REQUIRED, revision=1)}), publish(p)])
    case("kırılan yayından sonra kurallar uygulanabilir: profil son yayındır",
         [form(body(KAT_REQUIRED, revision=2), [RELEASED])],
         lambda p: [publish(p), p.step({"kind": "enforce", "id": "bina", "on": True})])
    fifty = form(body(KAT_REQUIRED, revision=2), [RELEASED] * 50)
    case("kırılma uyarısı en eski yayının düşmesinden önce", [fifty], lambda p: [publish(p)])
    case("düşen senaryonun planı da reddedilir", [form(broken_tests)], lambda p: [
        {**failing(p, ft.FAILING[0]), "op": "plan"}])
    case("olmayan formun yayını", [form(passing)], lambda p: [p.refused({"kind": "publish", "id": "yok"},
                                                                       fc.unknown_form("yok", "edit.id"))])
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.edit", "commandVersion": 1,
            "title": "Formlar: yayın",
            "note": "ADR 0442 §1, §2: yayının senaryoları ve kırdığı kayıtlar. Written by "
                    "scripts/fixtures/forms_publish_cases.py with the twins of forms_cases.py and forms_test_cases.py, "
                    "without KentOS code; do not edit by hand.", "setup": setup([form(body(ITEMS, ft.PASSING, 2))]),
            "cases": cases()}


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
