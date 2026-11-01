#!/usr/bin/env python3
"""One storey of a room module of several placed alone (docs/adr/0811, ADV-05; Archicad's Single Story): the shared
cases of `cad.modules.edit` and `cad.modules.query`, written from the ADR's rules without KentOS code.

The drawing is module_storeys_cases.py's (four levels by elevation: basement, ground, 1st, 2nd; a duplex whose ground
floor holds two walls, a door and a living room, its first floor two walls, a door, a room, a bed, the room's tag and a
dimension on a wall), and its definition: the duplex of two storeys.

The rules (the ADR's, in its words):
- A member's storey: a wall's, a space's or a fixture's own (docs/adr/0796 §1); another object's that of the members it
  leans on (a door its wall's, a tag its space's, a dimension its walls'), the highest; else the base.
- `place`'s `storey` places only that storey's members, all on the level given (no storey above it is needed). A module
  of one storey, a storey past its last, or no level is refused (`invalid_input`).
- The instance keeps its `storey`. An update from it reads its objects as that storey's; an update of the definition
  reaches only the members it holds: a member gone to another storey leaves its object as it was
  (`ModuleConflictWhy::Storey`, warning `module_storey_left`), a member come onto its storey is placed (`addedKeys`).
- Modüle ekle from it takes objects on its own level only; another instance takes the new members it holds.
- Katları tekrarla repeats it as the same storey on the target, whatever stands above.

    python3 scripts/fixtures/module_single_storey_cases.py          # writes the cases and the sample
    python3 scripts/fixtures/module_single_storey_cases.py --check  # compares with the files
"""

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import module_storeys_cases as ms  # noqa: E402

ROOT = ms.ROOT
EDIT = ROOT / "fixtures/commands/v1/cad.modules.edit.single.json"
QUERY = ROOT / "fixtures/commands/v1/cad.modules.query.single.json"
SAMPLE = ROOT / "fixtures/kcad/v2/instance-storeys.json"
UID, P, NEXT, ENTS, IDS, DUPLEX = ms.UID, ms.P, ms.NEXT, ms.ENTS, ms.IDS, ms.DUPLEX
run, done, refusal, warn, case, other, ANY, STEP = (ms.run, ms.done, ms.refusal, ms.warn, ms.case, ms.other, ms.ANY,
                                                     ms.STEP)
DEFINE = ms.DEFINE


def leans_on(i):
    """The drawing's objects `i` names: a door's wall, a tag's object, a dimension's references."""
    e = ENTS[i]
    if e["kind"] == "opening":
        return [e["host"]]
    if e["kind"] == "tag":
        return [e["of"]]
    if e["kind"] == "dimension":
        return [r["uid"] for r in e["refs"].values()]
    return []


def floor(i, keys=DUPLEX):
    """A member's storey (the ADR's rule)."""
    if ENTS[i]["kind"] in ("wall", "space", "insert"):
        return ms.storey(i, keys)
    found = [floor(int(u[-12:], 16), keys) for u in leans_on(i) if int(u[-12:], 16) in keys]
    return max(found, default=0)


def held(storey):
    return [i for i in DUPLEX if floor(i) == storey]


class Single(ms.Placed):
    """An instance holding one storey: its members' objects all on its level."""

    def __init__(self, n, first, at, level, storey, numbering=None):
        super().__init__(n, first, at, level, numbering, keys=held(storey))
        self.storey = storey

    def record(self, version=1):
        r = super().record(version)
        r["storey"] = self.storey
        return r

    def level_of(self, i):
        return self.level


def PLACE(p, name="Dubleks"):
    op = ms.PLACE(p, name)
    if isinstance(p, Single):
        op["storey"] = p.storey
    return op


def move(uid, dx=0.5):
    return other("cad.entities.transform", {"uids": [uid], "transform": {"kind": "move", "dx": dx, "dy": 0.0}})


def UPDATE(src):
    return {"kind": "update", "module": "Dubleks", "from": src}


STOREY_LEFT = ("Tek katlı örneğin {n} nesnesinin tanımdaki karşılığı başka bir kata geçti; olduğu gibi kaldı ve "
               "artık izlemez.")


# ── cad.modules.edit ──────────────────────────────────────────────────────────────────────────────────────────────


def place_cases():
    out = []
    up = Single(2, NEXT, (20, 10), "kat2", 1)
    module = ms.defined()
    module["instances"].append(up.record())
    out.append(case("Tek kat: dubleksin üst katı tek başına 2. kata (kapı duvarıyla, etiket odasıyla, ölçü duvarıyla)", [
        DEFINE(),
        run(PLACE(up), done({"module": module, "instance": "dubleks-2", "created": [up.uid(m) for m in up.keys],
                             "report": {}, "revision": "$current"}),
            {"ids": IDS + sorted(up.slots.values()), "fields": up.fields(), "revision": "changed", "canUndo": True},
            within=1e-9)]))
    low = Single(2, NEXT, (20, 10), "kat1", 0)
    out.append(case("Tek kat: zemin katı 1. kata; üst katın nesneleri gelmez", [
        DEFINE(), run(PLACE(low), done("$any"), {"ids": IDS + sorted(low.slots.values()), "fields": low.fields()},
                      within=1e-9)]))
    top = Single(2, NEXT, (20, 10), "kat2", 0)
    out.append(case("Tek kat en üst kata yerleşir (bütün dubleks sığmazdı)", [
        DEFINE(), run(PLACE(top), done("$any"), {"fields": top.fields()}, within=1e-9)]))
    pre = Single(2, NEXT, (20, 10), "kat1", 1, {"prefix": "B-"})
    out.append(case("Tek kat: numaralar önekli", [
        DEFINE(), run(PLACE(pre), done("$any"), {"fields": pre.fields()}, within=1e-9)]))
    base = Single(2, NEXT, (20, 10), "bodrum", 1)
    out.append(case("Tek kat: üst kat bodruma da yerleşir (taban değil, kendi katı)", [
        DEFINE(), run(PLACE(base), done("$any"), {"fields": base.fields()}, within=1e-9)]))
    out.append(case("Ret: iki katlı modülün üçüncü katı yok", [
        DEFINE(), run(dict(PLACE(up), storey=2),
                      refusal("invalid_input", "“Dubleks” modülü 2 katlı; kat 0 ile 1 arasında olur.",
                              "operation.storey"), {"revision": "same", "ids": IDS})]))
    one = Single(2, NEXT, (20, 10), "kat1", 0)
    out.append(case("Ret: tek katlı modülde kat seçilmez", [
        DEFINE(ms.ONE, "Tek"),
        run(dict(PLACE(one, "Tek")),
            refusal("invalid_input", "“Tek” modülü tek katlı; kat seçmeden yerleştirin.", "operation.storey"))]))
    out.append(case("Ret: tek kat katsız yerleşmez", [
        DEFINE(), run({"kind": "place", "module": "Dubleks", "at": P(20, 10), "storey": 1},
                      refusal("invalid_input", "“Dubleks” modülünün tek katı bir katta yerleşir; bir kat seçin.",
                              "operation.level"))]))
    out.append(case("Tek kat yerleştir ve geri al: tek adım", [
        DEFINE(), run(PLACE(up), ANY), {"op": "undo", "returns": STEP, "expect": {"ids": IDS}}]))
    return out


def update_cases():
    out = []
    up = Single(2, NEXT, (20, 10), "kat2", 1)
    out.append(case("Güncelle: dubleksin üst kattaki duvarı taşınınca tek kat örneği izler", [
        DEFINE(), run(PLACE(up), ANY), move(UID(4)),
        run(UPDATE("dubleks-1"),
            done({"module": "$any", "instance": "dubleks-1", "changed": "$any",
                  "report": {"updated": ["dubleks-1", "dubleks-2"], "changedKeys": [UID(4)]}, "revision": "$current"},
                 "$any"),
            {"fields": {str(up.slots[4]): {"pts": [P(24.5, 10), P(24.5, 13)], "arch": {"level": "kat2"}}}},
            within=1e-9)]))
    low = Single(2, NEXT, (20, 10), "kat1", 0)
    out.append(case("Güncelle: zemin katı örneği üst kattaki değişikliği almaz", [
        DEFINE(), run(PLACE(low), ANY), move(UID(4)),
        run(UPDATE("dubleks-1"), done("$any", "$any"),
            {"ids": IDS + sorted(low.slots.values()), "fields": low.fields()}, within=1e-9)]))
    out.append(case("Güncelle tek kattan: taşınan duvarı dubleksin üst katına gider", [
        DEFINE(), run(PLACE(up), ANY), move(up.uid(4)),
        run(UPDATE("dubleks-2"),
            done({"module": "$any", "instance": "dubleks-2", "changed": "$any",
                  "report": {"updated": ["dubleks-2", "dubleks-1"], "changedKeys": [UID(4)]}, "revision": "$current"},
                 "$any"),
            {"fields": {"4": {"pts": [P(4.5, 0), P(4.5, 3)], "arch": {"level": "kat1"}}}}, within=1e-9)]))
    out.append(case("Ret: değişmemiş tek kattan güncellenecek bir şey yok", [
        DEFINE(), run(PLACE(up), ANY),
        run(UPDATE("dubleks-2"),
            refusal("nothing_to_set", "“dubleks-2” örneği kaynakla aynı; güncellenecek bir şey yok.", "operation.from"),
            {"revision": "same"})]))
    # The source's ground-floor wall goes to the first floor: its member is of storey 1 now. The ground floor's single
    # instance keeps its copy as it was; the first floor's gets one.
    low2 = Single(2, NEXT, (20, 10), "kat1", 0)
    up3 = Single(3, low2.last() + 1, (40, 10), "kat2", 1)
    new = up3.last() + 1
    out.append(case("Güncelle: zemindeki duvar üst kata geçince zemin örneğinde kalır, üst kat örneğine eklenir", [
        DEFINE(), run(PLACE(low2), ANY), run(PLACE(up3), ANY),
        other("cad.levels.assign", {"level": "kat1", "uids": [UID(2)]}),
        run(UPDATE("dubleks-1"),
            # No object is rewritten: the source's wall is the change itself, the ground floor's copy stays.
            done({"module": "$any", "instance": "dubleks-1", "created": [f"$uidOf:{new}"],
                  "report": {"updated": ["dubleks-1", "dubleks-2", "dubleks-3"],
                             "conflicts": [{"instance": "dubleks-2", "key": UID(2), "uid": low2.uid(2),
                                            "why": "storey"}],
                             "changedKeys": [UID(2)], "addedKeys": [UID(2)]},
                  "revision": "$current"},
                 [warn("module_storey_left", STOREY_LEFT.format(n=1))]),
            {"fields": {str(low2.slots[2]): {"pts": [P(24, 10), P(24, 13)], "arch": {"level": "kat1"}},
                        str(new): {"pts": [P(44, 10), P(44, 13)], "arch": {"level": "kat2"}}}},
            within=1e-9)]))
    out.append(case("Yerel istisna: tek kattaki taşınan duvar güncellemede kalır", [
        DEFINE(), run(PLACE(up), ANY), move(up.uid(4), 1.0),
        run({"kind": "except", "module": "Dubleks", "uids": [up.uid(4)]}, ANY), move(UID(4)),
        run(UPDATE("dubleks-1"), done("$any", "$any"),
            {"fields": {str(up.slots[4]): {"pts": [P(25, 10), P(25, 13)], "arch": {"level": "kat2"}}}},
            within=1e-9)]))
    out.append(case("Kaynağa döndür: tek kattaki taşınan duvar tanımdaki yerine döner", [
        DEFINE(), run(PLACE(up), ANY), move(up.uid(4)),
        run({"kind": "restore", "module": "Dubleks", "uids": [up.uid(4)]}, done("$any"),
            {"fields": {str(up.slots[4]): {"pts": [P(24, 10), P(24, 13)], "arch": {"level": "kat2"}}}},
            within=1e-9)]))
    out.append(case("Bağımsızlaştır: tek katın nesneleri kendi katında kalır", [
        DEFINE(), run(PLACE(up), ANY),
        run({"kind": "detach", "module": "Dubleks", "instance": "dubleks-2"}, done("$any"),
            {"ids": IDS + sorted(up.slots.values()), "fields": up.fields()}, within=1e-9)]))
    return out


def member_cases():
    out = []
    low = Single(2, NEXT, (20, 10), "kat1", 0)
    up = Single(3, low.last() + 1, (40, 10), "kat2", 1)
    new = up.last() + 1
    out.append(case("Modüle ekle: dubleksin 1. kattaki yeni duvarı üst kat örneğine gelir, zemin örneğine gelmez", [
        DEFINE(), run(PLACE(low), ANY), run(PLACE(up), ANY),
        run({"kind": "addMembers", "module": "Dubleks", "instance": "dubleks-1", "uids": [UID(21)]},
            done({"module": "$any", "instance": "dubleks-1", "created": [f"$uidOf:{new}"],
                  "report": {"updated": ["dubleks-1", "dubleks-2", "dubleks-3"], "addedKeys": [UID(21)]},
                  "revision": "$current"}),
            {"ids": IDS + sorted(low.slots.values()) + sorted(up.slots.values()) + [new],
             "fields": {str(new): {"pts": [P(60, 10), P(62, 10)], "arch": {"level": "kat2"}}}},
            within=1e-9)]))
    k1 = Single(2, NEXT, (20, 10), "kat1", 1)
    new = k1.last() + 1
    out.append(case("Modüle ekle tek kattan: kendi katındaki duvar üst kat üyesi olur, dubleksin 1. katına gelir", [
        DEFINE(), run(PLACE(k1), ANY),
        run({"kind": "addMembers", "module": "Dubleks", "instance": "dubleks-2", "uids": [UID(21)]},
            done({"module": "$any", "instance": "dubleks-2", "created": [f"$uidOf:{new}"],
                  "report": {"updated": ["dubleks-2", "dubleks-1"], "addedKeys": [UID(21)]},
                  "revision": "$current"}),
            {"fields": {str(new): {"pts": [P(0, -10), P(2, -10)], "arch": {"level": "kat1"}}}},
            within=1e-9)]))
    out.append(case("Ret: tek kata başka kattaki duvar eklenmez", [
        DEFINE(), run(PLACE(k1), ANY),
        run({"kind": "addMembers", "module": "Dubleks", "instance": "dubleks-2", "uids": [UID(12)]},
            refusal("module_levels", "Eklenen nesne tek katlı örneğin katında değil; örneğin katındaki nesneleri ekleyin.",
                    "operation.uids[0]"), {"revision": "same"})]))
    up2 = Single(2, NEXT, (20, 10), "kat2", 1)
    out.append(case("Modülden çıkar: tek kattaki duvar tanımdan çıkar, dubleksin karşılığı silinir", [
        DEFINE(), run(PLACE(up2), ANY),
        run({"kind": "removeMembers", "module": "Dubleks", "uids": [up2.uid(4)]}, done("$any"),
            {"ids": [i for i in IDS if i != 4] + sorted(up2.slots.values())})]))
    return out


def repeat_cases():
    out = []
    k1 = Single(2, NEXT, (20, 10), "kat1", 1)
    rep = Single(3, k1.last() + 1, (20, 10), "kat2", 1)
    out.append(case("Katları tekrarla: 1. kattaki tek kat 2. kata tek kat olarak (en üst kat da olur)", [
        DEFINE(), run(PLACE(k1), ANY),
        run({"kind": "repeat", "from": "kat1", "to": [{"level": "kat2"}]},
            done({"module": "$any", "created": [rep.uid(m) for m in rep.keys],
                  "placed": [{"module": "dubleks", "instance": "dubleks-3", "level": "kat2"}],
                  "report": {}, "revision": "$current"}),
            {"ids": IDS + sorted(k1.slots.values()) + sorted(rep.slots.values()), "fields": rep.fields()},
            within=1e-9)]))
    out.append(case("Ret: bütün dubleks 2. kata tekrarlanamaz", [
        DEFINE(),
        run({"kind": "repeat", "from": "zemin", "to": [{"level": "kat2"}]},
            refusal("module_storey_missing", ms.storey_missing("Dubleks", 2, "kat2", 0), "operation.to[0].level"),
            {"revision": "same"})]))
    return out


def edit_cases():
    return place_cases() + update_cases() + member_cases() + repeat_cases()


# ── cad.modules.query ─────────────────────────────────────────────────────────────────────────────────────────────


def query_cases():
    out = []
    E = "cad.modules.edit"

    def q(name, inp, result, *before):
        return case(name, list(before) + [{"op": "execute", "input": inp, "result": result}])

    def ed(op):
        return {"op": "execute", "command": E, "input": {"operation": op}, "result": ANY}

    de = dict(DEFINE(), command=E)

    def irow(id, level, storey=None, levels=(), version=1, objects=11, drifted=(), exceptions=(), missing=()):
        r = {"id": id, "version": version, "stale": False, "objects": objects}
        if exceptions:
            r["exceptions"] = list(exceptions)
        if missing:
            r["missing"] = list(missing)
        if drifted:
            r["drifted"] = list(drifted)
        r["level"] = level
        if storey is not None:
            r["levels"] = [level]
            r["storey"] = storey
        elif levels:
            r["levels"] = list(levels)
        r["mirror"] = False
        return r

    def row(instances, version=1, members=11):
        return {"id": "dubleks", "name": "Dubleks", "version": version, "members": members, "storeys": 2,
                "removed": False, "instances": instances}

    def ans(modules, instance=None, key=None, drifted=False):
        o = {"modules": modules}
        if instance:
            o["instance"] = instance
        if key:
            o["key"] = key
        if drifted:
            o["drifted"] = True
        o["revision"] = "$current"
        return done(o)

    first = irow("dubleks-1", "zemin", levels=["zemin", "kat1"])
    up = Single(2, NEXT, (20, 10), "kat2", 1)
    low = Single(2, NEXT, (20, 10), "kat1", 0)
    s_up = irow("dubleks-2", "kat2", 1, objects=7)
    s_low = irow("dubleks-2", "kat1", 0, objects=4)
    out.append(q("Üst kat tek başına: kat 1, katları yalnız 2. kat, yedi nesne", {}, ans([row([first, s_up])]),
                 de, ed(PLACE(up))))
    out.append(q("Zemin katı tek başına: kat 0, dört nesne", {}, ans([row([first, s_low])]), de, ed(PLACE(low))))
    up3 = Single(3, low.last() + 1, (40, 10), "kat2", 1)
    out.append(q("İki tek kat örneği yan yana", {},
                 ans([row([first, s_low, irow("dubleks-3", "kat2", 1, objects=7)])]), de, ed(PLACE(low)),
                 ed(PLACE(up3))))
    out.append(q("Tek kattaki duvarın örneği ve anahtarı", {"uid": up.uid(3)},
                 ans([row([first, s_up])], "dubleks-2", UID(3)), de, ed(PLACE(up))))
    out.append(q("Tek kattaki etiketin anahtarı (odasının katı)", {"uid": up.uid(10)},
                 ans([row([first, s_up])], "dubleks-2", UID(10)), de, ed(PLACE(up))))
    out.append(q("Tek kattaki ölçünün anahtarı (duvarının katı)", {"uid": up.uid(11)},
                 ans([row([first, s_up])], "dubleks-2", UID(11)), de, ed(PLACE(up))))
    out.append(q("Zemin katındaki kapının örneği", {"uid": low.uid(5)},
                 ans([row([first, s_low])], "dubleks-2", UID(5)), de, ed(PLACE(low))))
    out.append(q("Tek kattaki elle taşınan duvar değişmiş", {"uid": up.uid(4)},
                 ans([row([first, irow("dubleks-2", "kat2", 1, objects=7, drifted=[up.uid(4)])])],
                     "dubleks-2", UID(4), True), de, ed(PLACE(up)), move(up.uid(4))))
    out.append(q("Tek kattaki silinen duvar eksik", {},
                 ans([row([first, irow("dubleks-2", "kat2", 1, objects=6, missing=[UID(4)])])]),
                 de, ed(PLACE(up)), other("cad.entities.delete", {"uids": [up.uid(4)]})))
    out.append(q("Tek kattaki yerel istisna", {},
                 ans([row([first, irow("dubleks-2", "kat2", 1, objects=7, exceptions=[up.uid(4)])])]),
                 de, ed(PLACE(up)), move(up.uid(4)), ed({"kind": "except", "module": "Dubleks", "uids": [up.uid(4)]})))
    out.append(q("Kaynağa döndürülen artık değişmiş değil", {}, ans([row([first, s_up])]),
                 de, ed(PLACE(up)), move(up.uid(4)), ed({"kind": "restore", "module": "Dubleks", "uids": [up.uid(4)]})))
    out.append(q("Güncellenince tek kat örneği de yeni sürümde", {},
                 ans([row([irow("dubleks-1", "zemin", levels=["zemin", "kat1"], version=2),
                           irow("dubleks-2", "kat2", 1, objects=7, version=2)], version=2)]),
                 de, ed(PLACE(up)), move(UID(4)), ed(UPDATE("dubleks-1"))))
    out.append(q("Tek kattan güncellenince ikisi de yeni sürümde", {},
                 ans([row([irow("dubleks-1", "zemin", levels=["zemin", "kat1"], version=2),
                           irow("dubleks-2", "kat2", 1, objects=7, version=2)], version=2)]),
                 de, ed(PLACE(up)), move(up.uid(4)), ed(UPDATE("dubleks-2"))))
    out.append(q("Üst kata eklenen üye üst kat örneğinde sekiz nesne", {},
                 ans([row([irow("dubleks-1", "zemin", levels=["zemin", "kat1"], version=2, objects=12),
                           irow("dubleks-2", "kat2", 1, objects=8, version=2)], version=2, members=12)]),
                 de, ed(PLACE(up)),
                 ed({"kind": "addMembers", "module": "Dubleks", "instance": "dubleks-1", "uids": [UID(21)]})))
    out.append(q("Üst kata eklenen üye zemin örneğine gelmez ama sürümü ilerler", {},
                 ans([row([irow("dubleks-1", "zemin", levels=["zemin", "kat1"], version=2, objects=12),
                           irow("dubleks-2", "kat1", 0, objects=4, version=2)], version=2, members=12)]),
                 de, ed(PLACE(low)),
                 ed({"kind": "addMembers", "module": "Dubleks", "instance": "dubleks-1", "uids": [UID(21)]})))
    out.append(q("Tekrarlanan tek kat", {},
                 ans([row([first, irow("dubleks-2", "kat1", 1, objects=7), irow("dubleks-3", "kat2", 1, objects=7)])]),
                 de, ed(PLACE(Single(2, NEXT, (20, 10), "kat1", 1))),
                 ed({"kind": "repeat", "from": "kat1", "to": [{"level": "kat2"}]})))
    out.append(q("Bağımsızlaştırılan tek kat düşer", {}, ans([row([first])]),
                 de, ed(PLACE(up)), ed({"kind": "detach", "module": "Dubleks", "instance": "dubleks-2"})))
    out.append(q("Geri alınan tek kat sayılmaz", {}, ans([row([first])]),
                 de, ed(PLACE(up)), {"op": "undo", "returns": STEP}))
    out.append(q("Kimlikle modül", {"module": "dubleks"}, ans([row([first, s_up])]), de, ed(PLACE(up))))
    out.append(q("Numaralı tek kat", {}, ans([row([first, irow("dubleks-2", "kat1", 1, objects=7)])]),
                 de, ed(PLACE(Single(2, NEXT, (20, 10), "kat1", 1, {"prefix": "B-"})))))
    return out


# ── The sample (KCAD schema 155) ──────────────────────────────────────────────────────────────────────────────────


def sample():
    """The duplex (module_storeys_cases.py's sample) with its upper storey placed alone on the 2nd floor: the members'
    objects copied in place onto that level, their uids the definition's keys with 0x100 added."""
    d = ms.sample()
    d["name"] = "Tek katlı modül örneği"
    keys = held(1)
    uid = lambda i: UID(0x100 + i)  # noqa: E731
    remap = {UID(i): uid(i) for i in keys}
    slot = max(e["id"] for e in d["entities"])
    for i in keys:
        e = copy.deepcopy(ENTS[i])
        slot += 1
        e = json.loads(_remapped(json.dumps(e), remap))
        e["id"] = slot
        if "arch" in e:
            e["arch"]["level"] = "kat2"
        d["uids"].append(uid(i))
        d["entities"].append(e)
    d["settings"]["arch"]["modules"][0]["instances"].append(
        {"id": "dubleks-2", "at": P(0, 0), "level": "kat2", "storey": 1, "version": 1,
         "members": [{"key": UID(i), "uid": uid(i)} for i in keys]})
    return d


def _remapped(text, remap):
    for a, b in remap.items():
        text = text.replace(a, b)
    return text


def write(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/module_single_storey_cases.py yazar: ADR 0811'in kurallarından, KentOS kodu "
                    "olmadan. Değiştirmeyin; betiği düzeltin.",
            "setup": ms.setup(), "cases": cases}


def main():
    files = {
        EDIT: write("cad.modules.edit", "Oda modülünün tek katı (docs/adr/0811)", edit_cases()),
        QUERY: write("cad.modules.query", "Oda modülünün tek kat örnekleri (docs/adr/0811)", query_cases()),
        SAMPLE: sample(),
    }
    check = "--check" in sys.argv
    bad = False
    for path, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        n = len(data["cases"]) if "cases" in data else None
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)}: başvurudan farklı", file=sys.stderr)
                bad = True
            elif n is not None:
                print(f"{data['command']} ({path.name}): {n} durum başvuruyla aynı.")
        else:
            path.write_text(text)
            print(f"{path.relative_to(ROOT)}: " + (f"{n} durum yazıldı." if n is not None else "yazıldı."))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
