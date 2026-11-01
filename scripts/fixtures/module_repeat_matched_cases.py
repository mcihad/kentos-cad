#!/usr/bin/env python3
"""A repeat's loose copies not made again where their like already stands (docs/adr/0810, ADV-05): the shared cases of
`cad.modules.edit`, written from the ADR's rules without KentOS code.

The drawing is module_storeys_cases.py's (four levels; a duplex's objects on the ground and first floors, a hall beside
it), here with no module defined: Katları tekrarla with `loose` copies the ground floor's objects (docs/adr/0796 §2).

The rule (the ADR's, in its words): a loose object of the source level is not copied to a target where an object of the
same kind already stands, in no module instance and no design option, equal to the copy it would make (on the target's
level, renumbered) but for its slot and what the document writes itself (an opening's jambs, a bound space's rings, a
tag's text and anchor, a broken link's reason); a copy's references go to its sources' likes where they have one, so a
like is sought again until none is found; each object there is the like of one copy at most, the first in the drawing's
order. Such an object is said in `report.matched` (the source, the one there, the level) with `module_repeat_matched`;
`again` copies them all the same. A repeat that would copy nothing and place no instance is refused
(`module_repeat_taken`).

    python3 scripts/fixtures/module_repeat_matched_cases.py          # writes the cases
    python3 scripts/fixtures/module_repeat_matched_cases.py --check  # compares with the file
"""

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import module_storeys_cases as ms  # noqa: E402

ROOT = ms.ROOT
OUT = ROOT / "fixtures/commands/v1/cad.modules.edit.matched.json"
UID, NEXT, ENTS, IDS = ms.UID, ms.NEXT, ms.ENTS, ms.IDS

# What the document writes itself, never compared (docs/adr/0779 §3).
WRITTEN = {"opening": ("place", "broken"), "tag": ("text", "anchor", "broken"), "dimension": ("broken",)}


def written(e):
    if e["kind"] == "space":
        return ("parts", "broken") if "sources" in e else ("broken",)
    return WRITTEN.get(e["kind"], ())


def compared(e):
    out = {k: v for k, v in e.items() if k != "id" and k not in written(e)}
    return json.dumps(out, sort_keys=True)


def source_level(i):
    return ms.level_of(ENTS[i])


def ref_ids(e):
    """The references of a drawing's object: (field path, uid)."""
    out = []
    if e["kind"] == "opening":
        out.append(("host", None, e["host"]))
    if e["kind"] == "space":
        out += [("sources", k, u) for k, u in enumerate(e.get("sources", []))]
    if e["kind"] == "tag":
        out.append(("of", None, e["of"]))
    if e["kind"] == "dimension":
        out += [("refs", n, r["uid"]) for n, r in e["refs"].items()]
    return out


def with_refs(e, to):
    """`e` with each reference uid `u` made `to(u)`."""
    e = copy.deepcopy(e)
    for field, key, u in ref_ids(e):
        if field in ("host", "of"):
            e[field] = to(u)
        elif field == "sources":
            e["sources"][key] = to(u)
        else:
            e["refs"][key]["uid"] = to(u)
    return e


def would_be(i, source, target, numbering, to):
    """The copy of source `i` on `target` (docs/adr/0796 §2), its references made `to(uid)`."""
    e = with_refs(ENTS[i], to)
    if e["kind"] in ("wall", "space", "insert"):
        e["arch"] = dict(e.get("arch") or {}, level=target)
    if "number" in e:
        if e["kind"] == "space":
            e["number"] = ms.renumbered(e["number"], ms.PREFIXES.get(source), ms.PREFIXES.get(target), numbering)
        else:
            e["number"] = ms.renumbered(e["number"], None, None, numbering)
    return e


class Drawing:
    """The drawing as the cases change it: each object's id (a base object's UID, a copy's `$uidOf:slot`), its value
    and level; the next slot."""

    def __init__(self):
        self.objects = [(UID(i), copy.deepcopy(ENTS[i]), ms.level_of(ENTS[i])) for i in IDS]
        self.next = NEXT

    def there(self, level):
        return [(u, e) for u, e, lv in self.objects if lv == level]

    def delete(self, uid):
        self.objects = [o for o in self.objects if o[0] != uid]

    def move(self, uid, dx, dy):
        for k, (u, e, lv) in enumerate(self.objects):
            if u == uid:
                e = copy.deepcopy(e)
                for key in ("p",):
                    if key in e:
                        e[key] = ms.P(e[key]["x"] + dx, e[key]["y"] + dy)
                if "pts" in e:
                    e["pts"] = [ms.P(p["x"] + dx, p["y"] + dy) for p in e["pts"]]
                self.objects[k] = (u, e, lv)

    def repeat(self, copies, targets, again=False, kept=(), source="zemin"):
        """A repeat of `copies` (source ids, the drawing's order) to `targets` ((level, numbering)): what it copies
        and what it matches, the drawing after; `kept` the objects no like may be (a design option's)."""
        copied, matched, made = [], [], []
        for level, numbering in targets:
            new = {i: f"$new:{i}" for i in copies}
            likes = {}
            if not again:
                there = [(u, e) for u, e in self.there(level) if u not in kept]
                used = set()
                while True:
                    found = False
                    for i in copies:
                        if i in likes:
                            continue
                        to = lambda u: likes.get(int(u[-12:], 16), new.get(int(u[-12:], 16), u))  # noqa: E731
                        mine = compared(would_be(i, source, level, numbering, to))
                        for k, (u, e) in enumerate(there):
                            if k not in used and compared(e) == mine:
                                used.add(k)
                                likes[i] = u
                                found = True
                                break
                    if not found:
                        break
            slots = {}
            for i in copies:
                if i not in likes:
                    slots[i] = f"$uidOf:{self.next}"
                    self.next += 1
            final = lambda u: likes.get(int(u[-12:], 16), slots.get(int(u[-12:], 16), u))  # noqa: E731
            for i in copies:
                if i in likes:
                    matched.append({"source": UID(i), "uid": likes[i], "level": level})
                else:
                    e = would_be(i, source, level, numbering, final)
                    self.objects.append((slots[i], e, level))
                    made.append((slots[i], e))
                    copied.append({"source": UID(i), "uid": slots[i], "level": level})
        return copied, matched, made


PLAIN, PLAIN_LEFT = ms.NO_MODULE
T2 = {"level": "kat2"}


def left_warning():
    return ms.left_warning(len(PLAIN_LEFT))


def matched_warning(n):
    return ms.warn("module_repeat_matched",
                   f"Kattaki {n} modülsüz nesnenin aynısı hedef katta zaten var; yeniden kopyalanmadı "
                   "(report.matched). Yine de kopyalamak için again verin.")


def taken(source="zemin"):
    return ms.refusal("module_repeat_taken",
                      f"“{source}” katının modülsüz nesnelerinin aynısı hedef katlarda zaten var; yine de kopyalamak "
                      "için again verin.", "operation.to")


def repeat_op(to, again=False, source="zemin"):
    op = ms.REPEAT(to, source=source)
    if again:
        op["again"] = True
    return op


def slot(u):
    return int(u[len("$uidOf:"):])


def fields(made):
    """The fields the cases look at of the objects a repeat made."""
    out = {}
    for u, e in made:
        f = {}
        for key in ("pts", "p", "host", "of", "sources", "number", "arch"):
            if key in e:
                f[key] = e[key]
        if "refs" in e:
            f["refs"] = e["refs"]
        out[str(slot(u))] = f
    return out


def answer(copied, matched, made, left=PLAIN_LEFT):
    out = {"created": [u for u, _ in made], "revision": "$current"}
    if copied:
        out["copied"] = copied
    report = {}
    if left:
        report["left"] = left
    if matched:
        report["matched"] = matched
    out["report"] = report
    warnings = []
    if left:
        warnings.append(ms.left_warning(len(left)))
    if matched:
        warnings.append(matched_warning(len(matched)))
    return ms.done(out, warnings)


def written_step(d, to, targets, again=False, source="zemin", copies=PLAIN, left=PLAIN_LEFT, kept=()):
    copied, matched, made = d.repeat(copies, targets, again, kept, source)
    ids = sorted({slot(u) for u, _, _ in d.objects if u.startswith("$uidOf:")}
                 | {int(u[-12:], 16) for u, _, _ in d.objects if not u.startswith("$")})
    return ms.run(repeat_op(to, again, source), answer(copied, matched, made, left),
                  {"ids": ids, "fields": fields(made)}, within=1e-9), (copied, matched, made)


def first(d):
    """The ground floor's objects copied to the 2nd floor once."""
    step, (copied, _, made) = written_step(d, [T2], [("kat2", None)])
    return step, {i: u for i, (u, _) in zip(PLAIN, made)}


def cases():
    out = []

    d = Drawing()
    s1, _ = first(d)
    out.append(ms.case("İkinci tekrar: her modülsüz nesnenin aynısı 2. katta var, ret", [
        s1, ms.run(repeat_op([T2]), taken(), {"revision": "same"})]))
    d = Drawing()
    s1, _ = first(d)
    out.append(ms.case("İkinci tekrarın planı da reddedilir, yazmaz", [
        s1, {"op": "plan", "input": {"operation": repeat_op([T2])}, "result": taken(),
             "expect": {"revision": "same"}}]))
    d = Drawing()
    s1, _ = first(d)
    s2, _ = written_step(d, [T2], [("kat2", None)], again=True)
    out.append(ms.case("again: aynısı olsa da yeniden kopyalanır", [s1, s2]))
    d = Drawing()
    s1, _ = first(d)
    s2, _ = written_step(d, [T2], [("kat2", None)], again=True)
    out.append(ms.case("again'den sonra üçüncü tekrar: hepsinin aynısı var, ret", [
        s1, s2, ms.run(repeat_op([T2]), taken(), {"revision": "same"})]))

    def after(name, prepare, targets=(("kat2", None),), to=None, again=False, source="zemin", copies=PLAIN,
              left=PLAIN_LEFT):
        d = Drawing()
        s1, c = first(d)
        steps = [s1] + prepare(d, c)
        step, _ = written_step(d, to or [T2], list(targets), again, source, copies, left)
        out.append(ms.case(name, steps + [step]))

    def delete(*ids):
        def go(d, c):
            for i in ids:
                d.delete(c[i])
            return [ms.other("cad.entities.delete", {"uids": [c[i] for i in ids]})]
        return go

    def move(i, dx, dy):
        def go(d, c):
            d.move(c[i], dx, dy)
            return [ms.other("cad.entities.transform",
                             {"uids": [c[i]], "transform": {"kind": "move", "dx": float(dx), "dy": float(dy)}})]
        return go

    after("Silinen donatının kopyası yeniden yapılır; öbürleri eşleşir", delete(18))
    after("Taşınan donatı kopyası artık aynısı değil: yeniden kopyalanır", move(18, 0.5, 0))
    after("Silinen holün duvarı yeniden yapılır; kapısı, mahali, etiketi ve ölçüleri onunla", delete(12))
    after("Taşınan duvar kopyası: duvar ve ona bağlananlar yeniden kopyalanır", move(13, 0, 0.25))
    after("Silinen kapı kopyası var olan duvar kopyasına yeniden konur", delete(14))
    after("Silinen etiket kopyası var olan mahal kopyasına yeniden bağlanır", delete(16))
    after("Silinen ölçü kopyası var olan iki duvar kopyasına bağlanır", delete(20))
    after("Silinen mahal kopyası yeniden yapılır, etiketi de", delete(15))
    after("Hepsi eşleşen 2. kat ve boş bodrum: yalnız bodruma kopyalanır", lambda d, c: [],
          targets=(("kat2", None), ("bodrum", None)), to=[T2, {"level": "bodrum"}])
    after("Numara öneki değişince numaralı mahal ve kapı yeniden kopyalanır, etiketi de", lambda d, c: [],
          targets=(("kat2", {"prefix": "A-"}),), to=[{"level": "kat2", "numbering": {"prefix": "A-"}}])
    after("again ile silinen donatı ve öbürleri yeniden kopyalanır", delete(18), again=True)
    k1, k1_left = ms.loose_plan("kat1", set(), set())
    after("1. kattan 2. kata: zeminden gelen duvar kopyaları 1. katın duvarlarının aynısı", lambda d, c: [],
          to=[T2], source="kat1", copies=k1, left=k1_left)

    # The 1st floor already holds walls drawn there by hand, the like of the ground floor's walls' copies.
    def kat1(name, prepare=lambda d: [], again=False, kept=()):
        d = Drawing()
        steps = prepare(d)
        step, _ = written_step(d, [{"level": "kat1"}], [("kat1", None)], again, kept=kept)
        out.append(ms.case(name, steps + [step]))

    kat1("1. katta elle çizilmiş aynı duvarlar eşleşir; kapının kopyası 1. katın duvarına konur")
    kat1("again: 1. kattaki aynı duvarlara rağmen duvarlar kopyalanır", again=True)

    def move_base(i, dx, dy):
        def go(d):
            d.move(UID(i), dx, dy)
            return [ms.other("cad.entities.transform",
                             {"uids": [UID(i)], "transform": {"kind": "move", "dx": float(dx), "dy": float(dy)}})]
        return go

    kat1("1. kattaki duvar taşınınca aynısı kalmaz; öbür duvar eşleşir", move_base(3, 0, 0.5))

    def option(d):
        return [ms.other("cad.options.edit", {"operation": {"kind": "define", "name": "Duvar seçeneği",
                                                            "uids": [UID(4)]}})]

    kat1("Tasarım seçeneğindeki duvar aynısı sayılmaz", option, kept={UID(4)})

    d = Drawing()
    s1, c = first(d)
    d.delete(c[18])
    step, _ = written_step(d, [T2], [("kat2", None)])
    out.append(ms.case("Eşleşmeli tekrar ve geri al: tek adım", [
        s1, ms.other("cad.entities.delete", {"uids": [c[18]]}), step,
        {"op": "undo", "returns": ms.STEP,
         "expect": {"ids": sorted(set(IDS) | set(range(NEXT, NEXT + len(PLAIN))) - {slot(c[18])})}}]))

    d = Drawing()
    s1, c = first(d)
    d.delete(c[18])
    copied, matched, _ = d.repeat(PLAIN, [("kat2", None)])
    ids = sorted(set(IDS) | set(range(NEXT, NEXT + len(PLAIN))) - {slot(c[18])})
    out.append(ms.case("Eşleşmeli tekrarın planı yazmaz; eşleşenleri söyler", [
        s1, ms.other("cad.entities.delete", {"uids": [c[18]]}),
        {"op": "plan", "input": {"operation": repeat_op([T2])},
         "result": ms.done({"created": "$any", "copied": "$any", "report": {"left": PLAIN_LEFT, "matched": matched},
                            "revision": "$current"}, [left_warning(), matched_warning(len(matched))]),
         "expect": {"revision": "same", "ids": ids}}]))

    d = Drawing()
    s1, _ = first(d)
    out.append(ms.case("İlk tekrar geri alınınca ikincisi her şeyi yeniden kopyalar", [
        s1, {"op": "undo", "returns": ms.STEP, "expect": {"ids": IDS}},
        ms.run(repeat_op([T2]), ms.done({"created": "$any", "copied": "$any", "report": {"left": PLAIN_LEFT},
                                         "revision": "$current"}, [left_warning()]))]))
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "cad.modules.edit", "commandVersion": 1,
            "title": "Katları tekrarla: hedefte aynısı olan modülsüz nesne yeniden kopyalanmaz (docs/adr/0810)",
            "note": "scripts/fixtures/module_repeat_matched_cases.py yazar: ADR 0810'un kuralından, KentOS kodu olmadan. "
                    "Değiştirmeyin; betiği düzeltin.",
            "setup": ms.setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)}: başvurudan farklı", file=sys.stderr)
            sys.exit(1)
        print(f"cad.modules.edit ({OUT.name}): {len(data['cases'])} durum başvuruyla aynı.")
    else:
        OUT.write_text(text)
        print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum yazıldı.")


if __name__ == "__main__":
    main()
