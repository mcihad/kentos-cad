"""Writes fixtures/commands/v1/cad.forms.edit.withdraw.json: the shared cases of withdrawing a form (docs/adr/0445 §1),
their expectations built here without KentOS code from the ADR's rule: `withdraw` (on) needs a release and a form whose
rules are not enforced; a withdrawn form writes no record (`cad.forms.submit`, `.fill`, `.transition`, `.recompute`
refuse `form_withdrawn` at `form`), validates still, is not enforced; put back (`on: false`) it writes again.

    python3 scripts/fixtures/forms_withdraw_cases.py           # writes the file
    python3 scripts/fixtures/forms_withdraw_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the form twin, written from the ADRs without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.forms.edit.withdraw.json"
NAME = "Bina denetimi"
WITHDRAWN = (f"“{NAME}” formu yayından kaldırıldı; kayıt bu formla yazılmaz. Formlar penceresinde Yeniden yayına al'ı "
             "kullanın.")
UNCHANGED = {"ids": fc.IDS, "revision": "same"}


def withdraw(on):
    return {"kind": "withdraw", "id": "denetim", "on": on}


def refused_call(command, inp):
    return {"op": "execute", "command": command, "input": inp,
            "result": fc.failed("form_withdrawn", WITHDRAWN, "form"), "expect": UNCHANGED}


def written(command, inp):
    return {"op": "execute", "command": command, "input": inp,
            "result": {"status": "completed", "output": fc.ANY, "warnings": fc.ANY},
            "expect": {"ids": fc.IDS, "revision": "changed"}}


SUBMIT = {"form": "denetim", "uid": "$uidOf:1", "values": {"kat-sayisi": "4"}}
FILL = {"form": "denetim", "uids": ["$uidOf:1", "$uidOf:2"], "values": {"kat-sayisi": "4"}}
TRANSITION = {"form": "denetim", "uid": "$uidOf:1", "step": "Onayla"}
RECOMPUTE = {"form": "denetim"}
VALIDATE = {"op": "execute", "command": "cad.forms.validate", "input": {"form": "denetim", "uid": "$uidOf:1"},
            "result": {"status": "completed", "output": fc.ANY, "warnings": []}, "expect": UNCHANGED}


def cases():
    out = []

    def case(name, forms, steps_of):
        p = fc.Project(copy.deepcopy(forms))
        out.append({"name": name, "setup": fc.setup(copy.deepcopy(forms)), "steps": steps_of(p)})

    case("yayından kaldırılan form: kaydedilir, form işaretlenir", [fc.PUBLISHED], lambda p: [p.step(withdraw(True))])
    case("yayından kaldırılan formla kayıt yazılmaz", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), refused_call("cad.forms.submit", SUBMIT)])
    case("toplu yazma da reddedilir", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), refused_call("cad.forms.fill", FILL)])
    case("iş akışı adımı da reddedilir", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), refused_call("cad.forms.transition", TRANSITION)])
    case("hesapları yenileme de reddedilir", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), refused_call("cad.forms.recompute", RECOMPUTE)])
    case("doğrulama sürer", [fc.PUBLISHED], lambda p: [p.step(withdraw(True)), VALIDATE])
    case("yeniden yayına alınan form yine yazar", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), p.step(withdraw(False)), written("cad.forms.submit", SUBMIT)])
    case("yayına almak zaten yayındakini değiştirmez", [fc.PUBLISHED], lambda p: [p.step(withdraw(False))])
    case("iki kez kaldırmak bir kez kaldırmaktır", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), p.step(withdraw(True))])
    case("yayınlanmamış form kaldırılamaz", [fc.FORM], lambda p: [p.refused(withdraw(True), fc.failed(
        "form_unpublished", f"“{NAME}” formu hiç yayınlanmadı; yayından kaldırılacak bir yayını yok.", "edit.on"))])
    case("kuralları uygulanan form kaldırılamaz", [fc.ENFORCED], lambda p: [p.refused(withdraw(True), fc.failed(
        "form_enforced", f"“{NAME}” formunun kuralları katmanın yazma kuralı; önce Kuralları uygula'yı kapatın.",
        "edit.on"))])
    case("kuralları kapatılınca kaldırılır", [fc.ENFORCED],
         lambda p: [p.step({"kind": "enforce", "id": "denetim", "on": False}), p.step(withdraw(True))])
    case("kaldırılan formun kuralları uygulanamaz", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), p.refused({"kind": "enforce", "id": "denetim", "on": True}, fc.failed(
             "form_withdrawn", f"“{NAME}” formu yayından kaldırıldı; kuralları uygulanmadan önce yeniden yayına alın.",
             "edit.on"), dirty=True)])
    case("kaldırılan formun kurallarını kapatmak serbest", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), p.step({"kind": "enforce", "id": "denetim", "on": False})])
    case("olmayan formu kaldırmak", [fc.PUBLISHED],
         lambda p: [p.refused({"kind": "withdraw", "id": "yok", "on": True}, fc.unknown_form("yok", "edit.id"))])
    case("taslağı düzenlemek ve yayınlamak sürer", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), p.step({"kind": "rename", "id": "denetim", "name": "Bina denetimi 2"})])
    case("yeni yayın kaldırılmışlığı değiştirmez", [{**fc.PUBLISHED, "draft": fc.body_with(
        lambda b: fc.item(b, "kat").update(help="Zemin dahil."))}],
         lambda p: [p.step(withdraw(True)), p.step({"kind": "publish", "id": "denetim"}),
                    refused_call("cad.forms.submit", SUBMIT)])
    case("taslakla yazmak da reddedilir", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), refused_call("cad.forms.submit", {**SUBMIT, "draft": True})])
    case("plan da reddeder", [fc.PUBLISHED],
         lambda p: [p.step(withdraw(True)), {**refused_call("cad.forms.submit", SUBMIT), "op": "plan"}])
    case("başka form etkilenmez", [fc.PUBLISHED, {**copy.deepcopy(fc.PUBLISHED), "id": "ikinci", "name": "İkinci"}],
         lambda p: [p.step(withdraw(True)), written("cad.forms.submit", {**SUBMIT, "form": "ikinci"})])
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.edit", "commandVersion": 1,
            "title": "Formlar: yayından kaldırma",
            "note": "ADR 0445 §1: yayından kaldırılan form kayıt yazmaz. Written by scripts/fixtures/forms_withdraw_cases.py "
                    "with the form twin of forms_cases.py, without KentOS code; do not edit by hand.",
            "setup": fc.setup([copy.deepcopy(fc.PUBLISHED)]), "cases": cases()}


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
