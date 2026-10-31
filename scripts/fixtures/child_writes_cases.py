"""Writes fixtures/forms/v1/child-writes.json: what a Veri girişi may do with a record's children through its
form, and what it may write at all (docs/adr/0764), built here without KentOS code from the ADR's rule.

- `offers`: each action a related item offers and, for the record's user, whether and why not. A record not saved
  is asked first; then view and show are always allowed; then the record locked, the child layer locked, the role
  writing nothing; a Veri girişi neither creates nor deletes; a Veri girişi changes (update, link, unlink) only the
  children of a layer a live form writes (published, not withdrawn), and links or unlinks only when that form shows
  the relation's field (a field item; a related item shows nothing).
- `writes`: a Veri girişi's change of a record's attributes. A field the layer's live forms show is readable: its
  absence is its removal. A field they do not show is hidden: the writer never read it, so its absence keeps it; a
  value it gives that differs from the stored one is a write of what it did not read. Nothing changed: allowed. A
  change on a layer no live form writes: `noForm`. Else the first hidden field written, in name order: `hidden`.

    python3 scripts/fixtures/child_writes_cases.py           # writes the file
    python3 scripts/fixtures/child_writes_cases.py --check   # compares it with the one on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (its digest rule, written without KentOS code)

OUT = ROOT / "fixtures/forms/v1/child-writes.json"

ALL = ["create", "update", "delete", "link", "unlink", "view", "show"]
DEFAULT = ["create", "update", "link", "unlink", "view", "show"]

# The layers' fields: a key is the field's id when it has one, else its name; an attribute is stored by name.
FIELDS = {
    "bina": [{"name": "Ad", "kind": "text"}],
    "sayac": [{"name": "No", "kind": "text"}, {"name": "Durum", "kind": "text"},
              {"id": "bina-bag", "name": "bina", "kind": "text"}],
    "abone": [{"name": "Ad", "kind": "text"}, {"name": "bina", "kind": "text"}, {"name": "Not", "kind": "text"}],
    "malik": [{"name": "Ad", "kind": "text"}, {"name": "bina", "kind": "text"}],
    "eski": [{"name": "Ad", "kind": "text"}, {"name": "bina", "kind": "text"}],
    "kayit": [{"name": "X", "kind": "text"}, {"name": "Y", "kind": "text"}, {"name": "Z", "kind": "text"}],
}


def field(fid, key):
    return {"type": "field", "id": fid, "field": key}


def body(*items, revision=1):
    return {"revision": revision, "items": list(items)}


def release(b, v=1):
    return {"version": v, "digest": fc.digest(b), "body": b}


def form(fid, layer, released, draft=None, withdrawn=False):
    out = {"id": fid, "name": fid, "layer": layer, "draft": draft or released or body()}
    if released is not None:
        out["releases"] = [release(released)]
    if withdrawn:
        out["withdrawn"] = True
    return out


FORMS = [
    form("bina-formu", "bina", body(field("ad", "Ad"), {"type": "related", "id": "sayaclar", "layer": "sayac",
                                                          "field": "bina-bag"})),
    # The meter's form shows its number and state, not its link to the building.
    form("sayac-formu", "sayac", body(field("no", "No"), field("durum", "Durum"))),
    # The subscriber's shows its link; its draft would show Not too, but a draft is no one's reach.
    form("abone-formu", "abone", body(field("ad", "Ad"), field("bag", "bina")),
         draft=body(field("ad", "Ad"), field("bag", "bina"), field("not", "Not"), revision=2)),
    # The owner's layer has a form never published.
    form("malik-formu", "malik", None, draft=body(field("ad", "Ad"), field("bag", "bina"))),
    # The old layer's only form was withdrawn.
    form("eski-formu", "eski", body(field("ad", "Ad"), field("bag", "bina")), withdrawn=True),
    # Two live forms on one layer read together.
    form("kayit-1", "kayit", body(field("x", "X"))),
    form("kayit-2", "kayit", body(field("y", "Y"))),
]


def live(f):
    return bool(f.get("releases")) and not f.get("withdrawn", False)


def name_of(layer, key):
    for fl in FIELDS[layer]:
        if fl.get("id", fl["name"]) == key:
            return fl["name"]
    return None


def bound(layer):
    """None: no form published on the layer; 'closed': published, none live; else the set of names shown."""
    published = [f for f in FORMS if f["layer"] == layer and f.get("releases")]
    if not published:
        return None
    shown, any_live = set(), False
    for f in published:
        if not live(f):
            continue
        any_live = True
        for it in walk(f["releases"][-1]["body"]["items"]):
            if it["type"] == "field":
                n = name_of(layer, it["field"])
                if n is not None:
                    shown.add(n)
    return shown if any_live else "closed"


def walk(items):
    for it in items:
        yield it
        if it["type"] in ("tab", "group"):
            yield from walk(it["items"])


def hidden(layer, name):
    b = bound(layer)
    if b is None:
        return False
    if b == "closed":
        return True
    return name not in b


def child_entry(layer, key):
    b = bound(layer)
    form_live = isinstance(b, set)
    name = name_of(layer, key) or key
    return {"layer": layer, "field": name, "form": form_live, "showsField": form_live and name in b}


# ---- offers ----

def refusal(action, p):
    if not p["saved"]:
        return "unsaved"
    if action in ("view", "show"):
        return None
    if p.get("recordLocked"):
        return "record"
    if p.get("childLocked"):
        return "childLocked"
    if p.get("noWrites"):
        return "noWrites"
    if p.get("attributesOnly"):
        if action in ("create", "delete"):
            return "attributes"
        c = p.get("child")
        if c is not None:
            if action in ("update", "link", "unlink") and not c["form"]:
                return "noForm"
            if action in ("link", "unlink") and not c["showsField"]:
                return "fieldHidden"
    return None


def offer_case(name, permits, actions=None):
    offered = DEFAULT if actions is None else actions
    expect = [{"action": a, "refused": refusal(a, permits)} for a in ALL if a in offered]
    return {"name": name, "actions": actions, "permits": permits, "expect": expect}


def entry(child=None, **kw):
    p = {"saved": True, "attributesOnly": True}
    p.update(kw)
    if child is not None:
        p["child"] = child
    return p


OFFERS = [
    offer_case("bir düzenleyici her şeyi yapar", {"saved": True}),
    offer_case("düzenleyicinin bütün eylemleri", {"saved": True}, ALL),
    offer_case("kaydedilmemiş kayıtta görmek de yok", {"saved": False}, ALL),
    offer_case("kilitli kayıtta yalnız görmek ve göstermek",
               {"saved": True, "recordLocked": "Kayıt kilitli."}, ALL),
    offer_case("alt katman kilitli", {"saved": True, "childLocked": "Sayaçlar"}, ALL),
    offer_case("rol nesne yazmaz", {"saved": True, "noWrites": True}, ALL),
    offer_case("Veri girişi: alt katmanı bilinmiyor (yerel kural)", entry(), ALL),
    offer_case("Veri girişi: sayacın formu bağ alanını göstermiyor",
               entry(child_entry("sayac", "bina-bag")), ALL),
    offer_case("Veri girişi: abonenin formu bağ alanını gösteriyor",
               entry(child_entry("abone", "bina")), ALL),
    offer_case("Veri girişi: malikin yayımlanmış formu yok",
               entry(child_entry("malik", "bina")), ALL),
    offer_case("Veri girişi: eski katmanın formu yayından kalktı",
               entry(child_entry("eski", "bina")), ALL),
    offer_case("Veri girişi: yalnız bağlayan öğe, alan gizli",
               entry(child_entry("sayac", "bina-bag")), ["link", "view"]),
    offer_case("Veri girişi: yalnız değiştiren öğe, alan gizli ama form var",
               entry(child_entry("sayac", "bina-bag")), ["update"]),
    offer_case("Veri girişi: boş eylem listesi", entry(child_entry("malik", "bina")), []),
    offer_case("Veri girişi: kilitli alt katman kendi sebebini söyler",
               entry(child_entry("malik", "bina"), childLocked="Malikler"), ALL),
    offer_case("Veri girişi: kaydedilmemiş kayıt önce", entry(child_entry("abone", "bina"), saved=False), ALL),
    offer_case("Veri girişi: iki formlu katmanda gösterilen alan",
               entry(child_entry("kayit", "Y")), ["update", "link", "unlink"]),
    offer_case("Veri girişi: iki formlu katmanda gösterilmeyen alan",
               entry(child_entry("kayit", "Z")), ["update", "link", "unlink"]),
    offer_case("düzenleyiciye alt katmanın formu sorulmaz",
               {"saved": True, "child": child_entry("malik", "bina")}, ALL),
    offer_case("Veri girişi ve rolü yazmaz: rol önce",
               entry(child_entry("malik", "bina"), noWrites=True), ALL),
]


# ---- writes ----

def changed_keys(layer, before, after):
    out = []
    for k in sorted(set(before) | set(after)):
        if hidden(layer, k):
            if k in after and after[k] != before.get(k):
                out.append(k)
        elif before.get(k) != after.get(k):
            out.append(k)
    return out


def write_rule(layer, before, after):
    keys = changed_keys(layer, before, after)
    if not keys:
        return None
    if not isinstance(bound(layer), set):
        return {"problem": "noForm"}
    for k in keys:
        if hidden(layer, k):
            return {"problem": "hidden", "field": k}
    return None


def write_case(name, layer, before, after):
    return {"name": name, "layer": layer, "before": before, "after": after,
            "expect": write_rule(layer, before, after)}


WRITES = [
    write_case("sayacın durumu formdan yazılır", "sayac", {"No": "1", "Durum": "açık"},
               {"No": "1", "Durum": "kapalı"}),
    write_case("sayacın gizli bağı yazılamaz", "sayac", {"No": "1"}, {"No": "1", "bina": "B-2"}),
    write_case("gizli bağın değişmesi yazılamaz", "sayac", {"No": "1", "bina": "B-1"},
               {"No": "1", "bina": "B-2"}),
    write_case("gizli alanın yokluğu onu korur", "sayac", {"No": "1", "bina": "B-1"}, {"No": "1"}),
    write_case("gizli alana aynı değer yazmak değişiklik değil", "sayac", {"No": "1", "bina": "B-1"},
               {"No": "2", "bina": "B-1"}),
    write_case("hiçbir şey değişmedi", "sayac", {"No": "1"}, {"No": "1"}),
    write_case("okunan alanın yokluğu silinmesidir", "sayac", {"No": "1", "Durum": "açık"}, {"No": "1"}),
    write_case("abonenin bağı formdan yazılır", "abone", {"Ad": "Ayşe"}, {"Ad": "Ayşe", "bina": "B-1"}),
    write_case("abonenin taslakta görünen notu yazılamaz", "abone", {"Ad": "Ayşe"},
               {"Ad": "Ayşe", "Not": "x"}),
    write_case("malikin yayımlanmış formu yok", "malik", {"Ad": "Ali"}, {"Ad": "Veli"}),
    write_case("formsuz katmanda değişiklik yoksa sorun yok", "malik", {"Ad": "Ali"}, {"Ad": "Ali"}),
    write_case("yayından kalkmış formun katmanı yazılamaz", "eski", {"Ad": "a"}, {"Ad": "a", "bina": "B"}),
    write_case("yayından kalkmış formun katmanında yokluk değişiklik değil", "eski", {"Ad": "a"}, {}),
    write_case("iki formun alanları birlikte okunur", "kayit", {"X": "1", "Y": "1"}, {"X": "2", "Y": "2"}),
    write_case("iki formun göstermediği alan yazılamaz", "kayit", {"X": "1"}, {"X": "1", "Z": "9"}),
    write_case("ilk gizli alan ad sırasıyla söylenir", "kayit", {}, {"Z": "1", "W": "2", "X": "3"}),
    write_case("bilinmeyen gizli alan da yazılamaz", "sayac", {"No": "1"}, {"No": "1", "Başka": "v"}),
    write_case("okunan ve gizli alan birlikte", "sayac", {"No": "1", "Durum": "a"},
               {"No": "2", "Durum": "b", "bina": "B"}),
    write_case("üst katmanın formu kendi alanını yazar", "bina", {"Ad": "A"}, {"Ad": "B"}),
    write_case("boş değer de değerdir", "sayac", {"No": "1"}, {"No": "1", "bina": ""}),
]


def document():
    return {"format": "kentos.child-writes", "version": 1,
            "note": "ADR 0764: Veri girişinin alt kayıtla ve kaydın alanlarıyla yapabildikleri, KentOS kodu olmadan "
                    "yazılmış başvurudan (scripts/fixtures/child_writes_cases.py). `offers`: ilişkili kayıtlar öğesinin "
                    "her eylemi ve reddin türü (unsaved, record, childLocked, noWrites, attributes, noForm, "
                    "fieldHidden; cümleleri çekirdeğindir). `writes`: Veri girişinin bir kaydın özniteliklerini "
                    "değiştirmesi; formların göstermediği alanın yokluğu onu korur, farklı değeri yazmadır.",
            "fields": FIELDS, "forms": FORMS, "offers": OFFERS, "writes": WRITES}


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: {len(OFFERS)} sunma, {len(WRITES)} yazma durumu güncel")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(OFFERS)} sunma, {len(WRITES)} yazma durumu")


if __name__ == "__main__":
    main()
