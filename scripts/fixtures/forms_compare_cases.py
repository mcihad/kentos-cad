"""Writes fixtures/commands/v1/cad.forms.compare.json: the shared cases of `cad.forms.compare` v1 (Sürümleri karşılaştır,
docs/adr/0427), their expectations built here without KentOS code from the ADR's rule: each item added, removed, moved
(under another parent, or elsewhere among the siblings both versions share) or changed (its own properties, its items
aside, the names sorted), in the newer version's order, the removed ones after them in the older one's; then the form's
`clearHidden` and scenarios.

    python3 scripts/fixtures/forms_compare_cases.py           # writes the file
    python3 scripts/fixtures/forms_compare_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (its digest rule, written without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.forms.compare.json"
FIELDS = [
    {"name": "Durum", "kind": "text", "values": [{"code": "iyi", "label": "İyi"}, {"code": "inceleme", "label": "İnceleme"}]},
    {"name": "Kat", "kind": "integer"},
    {"name": "Not", "kind": "text"},
    {"name": "Yükseklik", "kind": "decimal", "scale": 1},
]


def field(i, f, **more):
    return {"type": "field", "id": i, "field": f, **more}


V1 = {"revision": 1, "items": [field("durum", "Durum"), field("kat", "Kat"), field("not", "Not")]}
V2 = {"revision": 2, "items": [
    field("durum", "Durum", required=True),
    {"type": "group", "id": "ayrinti", "label": "Ayrıntı", "items": [field("kat", "Kat"), field("not", "Not",
                                                                                                  visibleWhen="Kat > 2")]},
]}
V3 = {"revision": 3, "items": [
    {"type": "tab", "id": "genel", "label": "Genel", "items": [field("durum", "Durum", required=True, label="Durumu")]},
    {"type": "tab", "id": "olcu", "label": "Ölçü", "items": [
        {"type": "group", "id": "ayrinti", "label": "Ölçüler", "items": [
            field("yukseklik", "Yükseklik", calculate="Kat * 3"), field("kat", "Kat")]}]},
], "clearHidden": True}
DRAFT = {"revision": 4, "items": copy.deepcopy(V3["items"]), "tests": [
    {"name": "boş", "expect": {"shown": ["genel", "durum"]}}]}
DRAFT["items"][0]["items"].append({"type": "note", "id": "bilgi", "text": "Kat {Kat}."})


def release(body, v):
    return {"version": v, "digest": fc.digest(body), "body": body}


FORMS = [
    {"id": "bina", "name": "Bina denetimi", "layer": "bina", "draft": DRAFT,
     "releases": [release(V1, 1), release(V2, 2), release(V3, 3)]},
    {"id": "taslak", "name": "Taslak form", "layer": "bina", "draft": V1},
    {"id": "ayni", "name": "Aynı form", "layer": "bina", "draft": V2, "releases": [release(V2, 1)]},
]


# ---- The rule, written from the ADR ----

# The properties' Turkish names in the words (docs/adr/0427).
WORDS = {"label": "ad", "field": "alan", "help": "yardım", "placeholder": "yer tutucu", "widget": "giriş biçimi", "readOnly": "salt okunur", "required": "zorunlu", "visibleWhen": "görünme koşulu", "requiredWhen": "zorunluluk koşulu", "readOnlyWhen": "salt okunurluk koşulu", "default": "varsayılan", "defaultExpr": "varsayılan ifadesi", "calculate": "hesap", "choiceFilter": "seçenek süzgeci", "unique": "tekil", "reuseLast": "son değer", "overridable": "değiştirilebilir", "suggest": "öneriler", "lookup": "başka katmandan liste", "checks": "denetimler", "columns": "sütunlar", "collapsed": "kapalı açılır", "text": "metin", "layer": "katman", "view": "görünüm", "accept": "kabul edilen", "min": "en az", "max": "en çok"}

def flat(items, parent=None, out=None):
    out = [] if out is None else out
    for i in items:
        own = {k: v for k, v in i.items() if k != "items"}
        out.append((i["id"], parent, own))
        flat(i.get("items", []), i["id"], out)
    return out


def name_of(own, i):
    for k in ("label", "field"):
        if own.get(k):
            return own[k]
    return i


def siblings(lst, parent, both):
    return [i for i, p, _ in lst if p == parent and i in both]


def compare(old, new):
    a, b = flat(old["items"]), flat(new["items"])
    a_ids = [i for i, _, _ in a]
    both = [i for i, _, _ in b if i in a_ids]
    out = []
    for i, parent, own in b:
        name = name_of(own, i)
        was = next((x for x in a if x[0] == i), None)
        if was is None:
            out.append({"item": i, "kind": "added", "words": f"“{name}” eklendi."})
            continue
        _, was_parent, was_own = was
        moved = was_parent != parent or (siblings(a, was_parent, both).index(i) != siblings(b, parent, both).index(i))
        if moved:
            out.append({"item": i, "kind": "moved", "words": f"“{name}” taşındı."})
        if was_own != own:
            keys = sorted({k for k in [*was_own, *own] if was_own.get(k) != own.get(k)})
            said = ", ".join(WORDS.get(k, k) for k in keys)
            out.append({"item": i, "kind": "changed", "properties": keys, "words": f"“{name}” değişti: {said}."})
    for i, _, own in a:
        if i not in [x for x, _, _ in b]:
            out.append({"item": i, "kind": "removed", "words": f"“{name_of(own, i)}” kaldırıldı."})
    if bool(old.get("clearHidden")) != bool(new.get("clearHidden")):
        out.append({"kind": "changed", "properties": ["clearHidden"],
                    "words": "Gizlenenler silinir." if new.get("clearHidden") else "Gizlenenler artık silinmez."})
    if old.get("tests", []) != new.get("tests", []):
        out.append({"kind": "changed", "properties": ["tests"],
                    "words": f"Senaryolar değişti: {len(new.get('tests', []))} senaryo, önce {len(old.get('tests', []))}."})
    return out


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Sürümler", "settings": {
        "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
        "workspace": "gis", "drawingFont": "barlow", "forms": copy.deepcopy(FORMS)},
        "origin": {"x": 500000, "y": 4400000},
        "layers": [{"id": "bina", "name": "Bina", "type": "layer", "visible": True, "locked": False, "expanded": True,
                    "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": [],
                    "fields": copy.deepcopy(FIELDS)}],
        "activeLayer": "bina", "entities": [], "styles": {"items": [], "categories": []}}


UNCHANGED = {"ids": [], "canUndo": False, "dirty": False, "revision": "same"}
BODY = {1: V1, 2: V2, 3: V3}


def ok(name, inp, old, new, op="execute"):
    out = {"from": inp.get("from", 3 if inp["form"] == "bina" else 1)}
    if "to" in inp:
        out["to"] = inp["to"]
    out["changes"] = compare(old, new)
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "completed", "output": out,
                                                                        "warnings": []}, "expect": UNCHANGED}]}


def refused(name, inp, code, message, path):
    return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": UNCHANGED}]}


def cases():
    return [
        ok("taslak son yayından: not eklendi, senaryolar", {"form": "bina"}, V3, DRAFT),
        ok("ilk yayından ikinciye: grup eklendi, alanlar taşındı, kural ve zorunluluk", {"form": "bina", "from": 1, "to": 2},
           V1, V2),
        ok("ikinciden üçüncüye: sekmeler, grubun adı, yeni hesap, gizlenenler", {"form": "bina", "from": 2, "to": 3}, V2, V3),
        ok("üçüncüden birinciye: geriye doğru", {"form": "bina", "from": 3, "to": 1}, V3, V1),
        ok("ilk yayından taslağa", {"form": "bina", "from": 1}, V1, DRAFT),
        ok("aynı yayın: değişiklik yok", {"form": "bina", "from": 2, "to": 2}, V2, V2),
        ok("taslak son yayınla aynı: değişiklik yok", {"form": "ayni"}, V2, V2),
        ok("plan aynı cevabı verir", {"form": "bina", "from": 1, "to": 3}, V1, V3, op="plan"),
        ok("ikinciden taslağa", {"form": "bina", "from": 2}, V2, DRAFT),
        ok("birinciden üçüncüye", {"form": "bina", "from": 1, "to": 3}, V1, V3),
        ok("taslaktan değil, yayından yayına: to verilince", {"form": "bina", "to": 3}, V3, V3),
        ok("ikinciden birinciye: grup kaldırıldı, alanlar geri taşındı", {"form": "bina", "from": 2, "to": 1}, V2, V1),
        ok("üçüncüden ikinciye", {"form": "bina", "from": 3, "to": 2}, V3, V2),
        {"name": "doğrulama hiçbir şey yazmaz", "steps": [{"op": "validate", "input": {"form": "bina", "from": 1},
                                                            "result": {"status": "completed", "output": None,
                                                                       "warnings": []}, "expect": UNCHANGED}]},
        refused("olmayan form", {"form": "yok"}, "unknown_form",
                "“yok” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.", "form"),
        refused("yayınlanmamış form: karşılaştırılacak yayın yok", {"form": "taslak"}, "form_unpublished",
                "“Taslak form” formu hiç yayınlanmadı; karşılaştırılacak yayın yok.", "from"),
        refused("yayınlanmamış formun verilen yayını", {"form": "taslak", "from": 1}, "unknown_release",
                "“Taslak form” formunun 1. yayını yok.", "from"),
        refused("olmayan yayından", {"form": "bina", "from": 9}, "unknown_release",
                "“Bina denetimi” formunun 9. yayını yok.", "from"),
        refused("olmayan yayına", {"form": "bina", "from": 1, "to": 7}, "unknown_release",
                "“Bina denetimi” formunun 7. yayını yok.", "to"),
        refused("önce from denetlenir", {"form": "bina", "from": 8, "to": 7}, "unknown_release",
                "“Bina denetimi” formunun 8. yayını yok.", "from"),
        refused("olmayan form, yayın da yanlış: önce form", {"form": "yok", "from": 3}, "unknown_form",
                "“yok” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.", "form"),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.compare", "commandVersion": 1,
            "title": "Sürümleri karşılaştır",
            "note": "ADR 0427: formun iki sürümü arasındaki değişiklikler. Written by scripts/fixtures/forms_compare_cases.py "
                    "without KentOS code; do not edit by hand.", "setup": setup(), "cases": cases()}


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
