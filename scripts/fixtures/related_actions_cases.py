"""Writes fixtures/commands/v1/cad.forms.submit.actions.json: the shared cases of a related item's actions in
`cad.forms.submit` (docs/adr/0762 §3), their expectations built here without KentOS code from the ADR's rule:

- a child not in the drawing asks `create`; one not linked to the record asks `link`; a linked one whose link is
  taken off asks `unlink`; one given a value that differs from what it holds asks `update` (with the link too);
  unlinking one not linked and giving a value it holds ask nothing;
- the related items showing the relation's child layer and field offer together their `actions`, else the default
  six (every action but `delete`), whatever their visibility rule; no such item: the form says nothing and the
  child goes in;
- the first child, in the input's order, asking an action not offered refuses the whole submission
  (`form_related_action`, path `children[i]`, the child's id for one in the drawing), its first missing action in
  the order create, link, unlink, update named by its word, the item by its label or the child layer's name.

    python3 scripts/fixtures/related_actions_cases.py           # writes the file
    python3 scripts/fixtures/related_actions_cases.py --check   # compares it with the one on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the relations' drawing, written without KentOS code)
import related_count_cases as rc  # noqa: E402  (its setup, links and children)

OUT = ROOT / "fixtures/commands/v1/cad.forms.submit.actions.json"
DEFAULT = ["create", "update", "link", "unlink", "view", "show"]
WORDS = {"create": "Yeni", "update": "Düzenle", "delete": "Sil", "link": "Bağla", "unlink": "Bağı kopar",
         "view": "Aç", "show": "Git"}
OBJECTS = {i: (lay, a) for i, lay, a in fc.REL_OBJECTS}


def related(i, layer, field, actions=None, label=None, visible=None):
    out = rc.related(i, layer, field, label=label, visible=visible)
    if actions is not None:
        out["actions"] = actions
    return out


def items_of(items):
    for x in items:
        yield x
        yield from items_of(x.get("items", []))


def offer(b, rel):
    """What the items showing the relation offer together; None when none shows it."""
    out = None
    for x in items_of(b["items"]):
        if x["type"] == "related" and (x["layer"], x["field"]) == (rel["child"], rel["field"]):
            out = (out or set()) | set(x.get("actions", DEFAULT))
    return out


def label_of(b, rel):
    for x in items_of(b["items"]):
        if x["type"] == "related" and (x["layer"], x["field"]) == (rel["child"], rel["field"]):
            return x.get("label", rc.LAYER_NAME[rel["child"]])
    return rc.LAYER_NAME[rel["child"]]


def asked(c, links, parent):
    """The child's actions (docs/adr/0762 §3's table)."""
    if not c["uid"].startswith("$uidOf:"):
        return ["create"]
    slot = int(c["uid"].split(":")[1])
    rel = rc.REL_OF[c["relation"]]
    linked = any(s == slot and f == rel["field"] and parent in ps for s, f, ps in links)
    out = []
    if c.get("unlink") and linked:
        out.append("unlink")
    if not c.get("unlink") and not linked:
        out.append("link")
    own = OBJECTS[slot][1]
    if any(own.get(k, "") != v for k, v in c.get("values", {}).items()):
        out.append("update")
    return out


def verdict(b, children, links, parent):
    for i, c in enumerate(children):
        rel = rc.REL_OF[c["relation"]]
        offered = offer(b, rel)
        if offered is None:
            continue
        missing = next((a for a in asked(c, links, parent) if a not in offered), None)
        if missing:
            error = {"code": "form_related_action",
                     "message": f"“{label_of(b, rel)}” alt kayıtlarında “{WORDS[missing]}” sunmuyor; formun "
                                f"tasarımında öğenin eylemlerine ekleyin.",
                     "path": f"children[{i}]"}
            if c["uid"].startswith("$uidOf:"):
                error["uids"] = [f"$uid:o{c['uid'].split(':')[1]}"]
            return {"status": "failed", "error": error}
    return None


def case(name, b, children, links=(), parent=1, op="execute"):
    inp = {"form": "bina", "uid": f"$uidOf:{parent}", "children": children}
    refused = verdict(b, children, links, parent)
    new = any(not c["uid"].startswith("$uidOf:") for c in children)
    if refused:
        result, expect = refused, {"ids": fc.REL_IDS, "revision": "same"}
    else:
        result = {"status": "completed", "output": fc.ANY, "warnings": []}
        written = op == "execute" and rc.changes(links, children, parent, {})
        expect = {"ids": fc.REL_IDS + ([10] if new and written else []), "revision": "changed" if written else "same"}
    steps = rc.CAPTURE + [rc.link_step(c, f, ps) for c, f, ps in links] + [{"op": op, "input": inp, "result": result,
                                                                            "expect": expect}]
    return {"name": name, "setup": rc.setup(b), "steps": steps}


def body(*items):
    return rc.body(*items)


S = lambda actions=None, **k: related("sayaclar", "sayac", "bina", actions, **k)  # noqa: E731
child = rc.child
NEW = rc.CHILD_NEW


def cases():
    one = [(3, "bina", (1,))]
    return [
        case("yalnız bağla: var olan sayaç bağlanır", body(S(["link"])), [child("bina-sayac", 4)]),
        case("yalnız bağla: yeni sayaç reddedilir", body(S(["link"])),
             [child("bina-sayac", uid=NEW, values={"no": "S-99"})]),
        case("yalnız bağla: bağı koparmak reddedilir", body(S(["link"])),
             [child("bina-sayac", 3, unlink=True)], links=one),
        case("yalnız bağla: bağlarken değer değiştirmek reddedilir", body(S(["link"])),
             [child("bina-sayac", 4, values={"no": "S-20"})]),
        case("yalnız bağla: bağlarken aynı değeri vermek eylem değil", body(S(["link"])),
             [child("bina-sayac", 4, values={"no": "S-2"})]),
        case("salt okunur liste: hiçbir alt kayıt yazılmaz", body(S([])), [child("bina-sayac", 4)]),
        case("salt okunur liste: bağlıyı yeniden göndermek eylem değil", body(S([])), [child("bina-sayac", 3)],
             links=one),
        case("salt okunur liste: bağlı olmayanın bağını koparmak eylem değil", body(S([])),
             [child("bina-sayac", 4, unlink=True)]),
        case("öntanımlı eylemler: yeni sayaç yazılır", body(S()),
             [child("bina-sayac", uid=NEW, values={"no": "S-99"})]),
        case("öntanımlı eylemler: bağı kopar sunulur", body(S()), [child("bina-sayac", 3, unlink=True)], links=one),
        case("öntanımlı eylemler: bağlı sayacın değeri değişir", body(S()),
             [child("bina-sayac", 3, values={"no": "S-10"})], links=one),
        case("yalnız düzenle: bağlı sayacın değeri değişir", body(S(["update"])),
             [child("bina-sayac", 3, values={"no": "S-10"})], links=one),
        case("yalnız düzenle: bağlamak reddedilir", body(S(["update"])), [child("bina-sayac", 4)]),
        case("yeni ve bağla sunulur, düzenle sunulmaz", body(S(["create", "link"])),
             [child("bina-sayac", uid=NEW, values={"no": "S-99"}), child("bina-sayac", 3, values={"no": "S-10"})],
             links=one),
        case("ilk sunulmayan alt kayıt söylenir", body(S(["unlink"])),
             [child("bina-sayac", 3, unlink=True), child("bina-sayac", 4), child("bina-sayac", 5)], links=one),
        case("adı verilen öğe iletide adıyla", body(S(["link"], label="Sayaçlar")),
             [child("bina-sayac", uid=NEW, values={"no": "S-99"})]),
        case("iki öğe sunduklarını birleştirir", body(S(["link"]), related("sayac-yeni", "sayac", "bina", ["create"])),
             [child("bina-sayac", uid=NEW, values={"no": "S-99"}), child("bina-sayac", 4)]),
        case("gizli öğe de formun tasarımıdır", body(S(["link"], visible="Ad = 'X'")),
             [child("bina-sayac", uid=NEW, values={"no": "S-99"})]),
        case("ilişkiyi göstermeyen form bir şey söylemez", body(S(["link"])),
             [child("bina-abone", uid=NEW, values={"ad": "Veli"})]),
        case("başka ilişkinin öğesi sunar: abone yazılmaz", body(related("aboneler", "abone", "bina", ["link"])),
             [child("bina-abone", uid=NEW, values={"ad": "Veli"})]),
        case("çoktan çoğa: yalnız bağla, malik bağlanır", body(related("malikler", "malik", "binalar", ["link"])),
             [child("bina-malik", 9)]),
        case("çoktan çoğa: bağ kopar sunulmaz", body(related("malikler", "malik", "binalar", ["link"])),
             [child("bina-malik", 9, unlink=True)], links=[(9, "binalar", (1, 2))]),
        case("sil yalnız açıkça yazılır, bağlamayı sunmaz", body(S(["delete"])), [child("bina-sayac", 4)]),
        case("plan da reddeder", body(S(["link"])), [child("bina-sayac", uid=NEW, values={"no": "S-99"})], op="plan"),
        case("plan geçen gönderimi yazmaz", body(S(["link"])), [child("bina-sayac", 4)], op="plan"),
        case("ikinci binada: kendi sayacının bağını koparmak", body(S(["unlink"])),
             [child("bina-sayac", 5, unlink=True)], links=[(5, "bina", (2,))], parent=2),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.submit", "commandVersion": 1,
            "title": "Formla kaydet: ilişkili kayıtların eylemleri",
            "note": "ADR 0762 §3: ilişkili kayıtlar öğesinin sunduğu eylemler, gönderimin alt kayıtlarıyla. Written by "
                    "scripts/fixtures/related_actions_cases.py with the relations' drawing of forms_cases.py, without "
                    "KentOS code; do not edit by hand.",
            "setup": rc.setup(body(S(["link"]))), "cases": cases()}


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    n = text.count('"name":') - 1
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)",
                  file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({len(build()['cases'])} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(build()['cases'])} durum)")
    return 0 if n else 1


if __name__ == "__main__":
    sys.exit(main())
