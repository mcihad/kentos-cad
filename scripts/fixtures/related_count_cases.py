"""Writes fixtures/commands/v1/cad.forms.submit.related.json: the shared cases of a related item's bounds in
`cad.forms.submit` (docs/adr/0444 §1), their expectations built here without KentOS code from the ADR's rule: a shown
related item with `min`/`max` counts the objects referring to the record, those the submission links (+1 for a child
not referring to it yet), unlinks (-1 for one that did) and leaves as they are; fewer than `min` is
`form_related_few`, more than `max` `form_related_many`, the first in the form's order refusing the whole submission
(`form_invalid`, path `values`). A hidden item is not judged. The parent is judged before its children.

    python3 scripts/fixtures/related_count_cases.py           # writes the file
    python3 scripts/fixtures/related_count_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the relations' drawing and the digest, written without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.forms.submit.related.json"
CHILD_NEW = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4a01"
CHILD_OTHER = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4a02"
RELS = [fc.relation("bina-sayac", "bina", "sayac", "bina", name="Binanın sayaçları"),
        fc.relation("bina-abone", "bina", "abone", "bina", name="Binanın aboneleri"),
        fc.relation("bina-malik", "bina", "malik", "binalar", name="Binanın malikleri")]
LAYER_OF = {i: lay for i, lay, _ in fc.REL_OBJECTS}
LAYER_NAME = {"sayac": "Sayac", "abone": "Abone", "malik": "Malik"}
REL_OF = {r["id"]: r for r in RELS}


def related(i, layer, field, lo=None, hi=None, label=None, visible=None):
    out = {"type": "related", "id": i, "layer": layer, "field": field}
    if label:
        out["label"] = label
    if visible:
        out["visibleWhen"] = visible
    if lo is not None:
        out["min"] = lo
    if hi is not None:
        out["max"] = hi
    return out


def body(*items):
    return {"revision": 1, "items": [{"type": "field", "id": "ad", "field": "Ad"}, *items]}


def setup(b):
    s = copy.deepcopy(fc.rel_setup(RELS))
    form = lambda i, name, layer, x: {"id": i, "name": name, "layer": layer, "draft": copy.deepcopy(x),  # noqa: E731
                                      "releases": [{"version": 1, "digest": fc.digest(x), "body": copy.deepcopy(x)}]}
    # The meters' own form: a new meter's number at least three letters (forms_cases.py's SAYAC_BODY).
    s["settings"]["forms"] = [form("bina", "Bina", "bina", b), form("sayac", "Sayaç", "sayac", fc.SAYAC_BODY)]
    return s


CAPTURE = [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in fc.REL_IDS]


def link_step(child, field, parents):
    value = "; ".join(f"$uid:o{p}" for p in parents)
    return {"op": "execute", "command": "cad.entities.set",
            "input": {"uids": [f"$uid:o{child}"], "attrs": {field: value}, "operation": "attributes"},
            "result": {"status": "completed", "output": {"changed": [f"$uid:o{child}"], "ids": [child],
                                                          "revision": "$current"}, "warnings": []},
            "expect": {"ids": fc.REL_IDS, "revision": "changed"}}


def field_of(r):
    return next(f for f in fc.REL_FIELDS[r["child"]] if fc.key_of(f) == r["field"])


def count(item, links, children, parent):
    """The objects referring to `parent` through the item's layer and field after the submission."""
    held = {c for c, f, ps in links if LAYER_OF[c] == item["layer"] and f == item["field"] and parent in ps}
    n = len(held)
    for c in children:
        r = REL_OF[c["relation"]]
        if (r["child"], r["field"]) != (item["layer"], item["field"]):
            continue
        slot = int(c["uid"].split(":")[1]) if c["uid"].startswith("$uidOf:") else None
        linked = slot in held
        if c.get("unlink"):
            n -= 1 if linked else 0
        else:
            n += 0 if linked else 1
    return max(n, 0)


def shown(item, attrs):
    """The cases' only visibility rule: `Ad = '<value>'`."""
    rule = item.get("visibleWhen")
    if not rule:
        return True
    want = rule.split("'")[1]
    return attrs.get("Ad", "") == want


def verdict(b, links, children, parent, values):
    attrs = dict(next(a for i, _, a in fc.REL_OBJECTS if i == parent))
    attrs.update(values)
    for item in b["items"]:
        if item["type"] != "related" or not shown(item, attrs):
            continue
        n = count(item, links, children, parent)
        label = item.get("label", LAYER_NAME[item["layer"]])
        if "min" in item and n < item["min"]:
            return f"“{label}” en az {item['min']} kayıt ister; kayda bağlı {n} var."
        if "max" in item and n > item["max"]:
            return f"“{label}” en çok {item['max']} kayıt alır; kayda bağlı {n} var."
    return None


def case(name, b, children, links=(), parent=1, values=None, op="execute", child_error=None):
    inp = {"form": "bina", "uid": f"$uidOf:{parent}", "children": children}
    if values:
        inp["values"] = values
    why = verdict(b, links, children, parent, values or {})
    new = any(not c["uid"].startswith("$uidOf:") and not c.get("unlink") for c in children)
    if why:
        result = fc.failed("form_invalid", f"{why} Değeri düzeltip yeniden kaydedin.", "values")
        expect = {"ids": fc.REL_IDS, "revision": "same"}
    elif child_error:
        result = fc.failed("form_invalid", fc.ANY, child_error)
        expect = {"ids": fc.REL_IDS, "revision": "same"}
    else:
        result = {"status": "completed", "output": fc.ANY, "warnings": []}
        written = op == "execute" and changes(links, children, parent, values or {})
        expect = {"ids": fc.REL_IDS + ([10] if new and written else []), "revision": "changed" if written else "same"}
    steps = CAPTURE + [link_step(c, f, ps) for c, f, ps in links] + [{"op": op, "input": inp, "result": result,
                                                                      "expect": expect}]
    return {"name": name, "setup": setup(b), "steps": steps}


def changes(links, children, parent, values):
    """Whether the submission writes anything: a value, a new child, a link made or taken off, a child's value."""
    attrs = next(a for i, _, a in fc.REL_OBJECTS if i == parent)
    if any(attrs.get(k) != v for k, v in values.items()):
        return True
    for c in children:
        if not c["uid"].startswith("$uidOf:"):
            return True
        slot = int(c["uid"].split(":")[1])
        r = REL_OF[c["relation"]]
        linked = any(s == slot and f == r["field"] and parent in ps for s, f, ps in links)
        own = next(a for i, _, a in fc.REL_OBJECTS if i == slot)
        if linked == bool(c.get("unlink")) or any(own.get(k) != v for k, v in c.get("values", {}).items()):
            return True
    return False


def child(rel, slot=None, uid=None, unlink=False, values=None):
    out = {"relation": rel, "uid": f"$uidOf:{slot}" if slot is not None else uid}
    if values:
        out["values"] = values
    if unlink:
        out["unlink"] = True
    return out


S = lambda lo=None, hi=None, **k: related("sayaclar", "sayac", "bina", lo, hi, **k)  # noqa: E731


def cases():
    one = [(3, "bina", (1,))]
    two = [(3, "bina", (1,)), (4, "bina", (1,))]
    three = two + [(5, "bina", (1,))]
    return [
        case("en az bir: var olan sayaç bağlanır, kayıt yazılır", body(S(1)), [child("bina-sayac", 3)]),
        case("en az bir: bağlı sayaç yok, değerle gönderim reddedilir", body(S(1)), [], values={"Ad": "A2"}),
        case("en az bir: bağlı bir sayaç var, değer yazılır", body(S(1)), [], links=one, values={"Ad": "A2"}),
        case("bire bir: bağlı biri varken ikincisi reddedilir", body(S(hi=1)), [child("bina-sayac", 4)], links=one),
        case("bire bir: biri koparılıp öbürü bağlanınca bir kalır", body(S(hi=1)),
             [child("bina-sayac", 3, unlink=True), child("bina-sayac", 4)], links=one),
        case("en çok iki: üçüncü reddedilir", body(S(hi=2)), [child("bina-sayac", 5)], links=two),
        case("en az iki: biri bağlıyken yeni kayıt ikinci olur", body(S(2)),
             [child("bina-sayac", uid=CHILD_NEW, values={"no": "S-99"})], links=one),
        case("bir ile bir: iki birden bağlanamaz", body(S(1, 1)), [child("bina-sayac", 3), child("bina-sayac", 4)]),
        case("en az bir: tek bağlı koparılırsa reddedilir", body(S(1)), [child("bina-sayac", 3, unlink=True)],
             links=one),
        case("bağlı sayaç yeniden gönderilir: sayı değişmez", body(S(1, 1)),
             [child("bina-sayac", 3, values={"no": "S-10"})], links=one),
        case("gizli öğe sayılmaz", body(S(1, visible="Ad = 'X'")), [], values={"Ad": "A2"}),
        case("görünen öğe sayılır: koşulu tutunca reddedilir", body(S(1, visible="Ad = 'X'")), [], values={"Ad": "X"}),
        case("adı verilen öğe iletide adıyla", body(S(1, label="Sayaçlar")), [], values={"Ad": "A2"}),
        case("plan da reddeder", body(S(1)), [], values={"Ad": "A2"}, op="plan"),
        case("plan geçen gönderimi yazmaz", body(S(1)), [child("bina-sayac", 3)], op="plan"),
        case("en çok bir, hiç yokken bir bağlanır", body(S(0, 1)), [child("bina-sayac", 3)]),
        case("en çok üç: üç bağlıyken değer yazılır", body(S(hi=3)), [], links=three, values={"Ad": "A2"}),
        case("en çok iki: zaten üç bağlıyken değer yazmak reddedilir", body(S(hi=2)), [], links=three,
             values={"Ad": "A2"}),
        case("başka binanın sayaçları sayılmaz", body(S(1)), [], links=[(5, "bina", (2,))], values={"Ad": "A2"}),
        case("ikinci binanın kendi sayacı sayılır", body(S(1)), [], links=[(5, "bina", (2,))], parent=2,
             values={"Ad": "B2"}),
        case("bağlı olmayanı koparmak sayıyı düşürmez", body(S(1)), [child("bina-sayac", 3, unlink=True)],
             links=[(4, "bina", (1,))]),
        case("yeni kayıt ve kopan eski: bire bir korunur", body(S(hi=1)),
             [child("bina-sayac", uid=CHILD_NEW, values={"no": "S-99"}), child("bina-sayac", 3, unlink=True)],
             links=one),
        case("iki öğe: ilk sorunlu öğe söylenir", body(S(hi=1), related("aboneler", "abone", "bina", 1)),
             [child("bina-sayac", 3)]),
        case("çoktan çoğa: malik bağlanınca en az bir tutar", body(related("malikler", "malik", "binalar", 1)),
             [child("bina-malik", 9)]),
        case("sayı tutar, alt kaydın kendi formu reddeder", body(S(1)),
             [child("bina-sayac", uid=CHILD_OTHER, values={"no": "ab"})], child_error="children[0].values.no"),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.submit", "commandVersion": 1,
            "title": "Formla kaydet: ilişkili kayıtların sayısı",
            "note": "ADR 0444 §1: ilişkili kayıtlar öğesinin en az ve en çok sayısı, gönderimin alt kayıtlarıyla. Written by "
                    "scripts/fixtures/related_count_cases.py with the relations' drawing of forms_cases.py, without KentOS "
                    "code; do not edit by hand.",
            "setup": setup(body(S(1))), "cases": cases()}


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
