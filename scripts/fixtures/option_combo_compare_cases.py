#!/usr/bin/env python3
"""Two combinations of design options compared (docs/adr/0809, ADV-05): the shared cases of `cad.options.query`'s
`compareCombos`, written from the ADR's rules without KentOS code.

The drawing and the sets are option_combo_cases.py's: the kitchen (Mutfak: A, B) and a note (Not: A, B). Its
reference reads the sets, an option comparison and the combinations; this one adds the comparison of two combinations.

The rules (docs/adr/0809 §1):

- `compareCombos` names two combinations (`a`, `b`) by id or name (case aside); one the project lacks is refused
  (`option_combo_not_found`, its path `compareCombos.a` or `.b`); the same twice is refused (`invalid_input`). The
  search distance and the tolerance are an option comparison's (absent 0.5 m and 1 mm; below zero refused).
- Every set, in the project's order, is in `alike` when both show the same option of it (its own choice, else its
  primary), else compared as two options are (`a`'s option against `b`'s).
- `objectsA` and `objectsB` count the plan each shows (every object but the hidden options' of each set);
  `spaceArea` and `wallLength` are the plans' totals' differences, b − a.
- The read's other answers (the sets, an option comparison, the combinations) are as before.

    python3 scripts/fixtures/option_combo_compare_cases.py          # writes the cases
    python3 scripts/fixtures/option_combo_compare_cases.py --check  # compares with the files
"""

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import option_cases as oc  # noqa: E402
import option_combo_cases as occ  # noqa: E402

ROOT = oc.ROOT
OUT = ROOT / "fixtures/commands/v1/cad.options.query.compare-combos.json"
UID, Refused, ch = oc.UID, oc.Refused, occ.ch


def find(st, key, path):
    for c in st.combos:
        if c["id"] == key:
            return c
    for c in st.combos:
        if c["name"].lower() == key.strip().lower():
            return c
    raise Refused("option_combo_not_found", f"Projede “{key}” adlı ya da kimlikli seçenek bileşimi yok.", path)


def shown(st, combo):
    """What a combination shows of every set, in the project's order."""
    out = []
    for s in st.doc.sets:
        own = next((x["option"] for x in combo.get("choices", [])
                    if x["set"] == s["id"] and any(o["id"] == x["option"] for o in s["options"])), s["primary"])
        out.append((s, own))
    return out


def plan(st, choices):
    doc = st.doc
    hidden = {m for s, own in choices for o in s["options"] if o["id"] != own for m in o.get("members", [])}
    objects, area, length = 0, 0.0, 0.0
    for slot in sorted(doc.ents):
        if doc.uid[slot] in hidden:
            continue
        e = doc.ents[slot]
        objects += 1
        if e["kind"] == "space":
            for part in e["parts"]:
                area += oc.ring_area(part["outer"]["pts"]) - sum(oc.ring_area(h["pts"]) for h in part.get("holes", []))
        if e["kind"] == "wall":
            pts = e["pts"]
            length += sum(math.hypot(pts[i + 1]["x"] - pts[i]["x"], pts[i + 1]["y"] - pts[i]["y"]) for i in range(len(pts) - 1))
    return objects, area, length


def compare_combos(st, q):
    a, b = find(st, q["a"], "compareCombos.a"), find(st, q["b"], "compareCombos.b")
    if a["id"] == b["id"]:
        raise Refused("invalid_input", "Bir bileşim kendisiyle karşılaştırılmaz; iki ayrı bileşim verin.", "compareCombos.b")
    for key in ("search", "tolerance"):
        if key in q and not q[key] >= 0:
            raise Refused("invalid_input", "Arama uzaklığı ve tolerans sonlu, sıfır ya da daha büyük olmalı.", f"compareCombos.{key}")
    ca, cb = shown(st, a), shown(st, b)
    out = {"a": a["id"], "b": b["id"]}
    sets, alike = [], []
    for (s, x), (_, y) in zip(ca, cb):
        if x == y:
            alike.append(s["id"])
            continue
        oa = next(o for o in s["options"] if o["id"] == x)
        ob = next(o for o in s["options"] if o["id"] == y)
        sets.append(oc.compare(st.doc, s, oa, ob, q.get("search", 0.5), q.get("tolerance", 0.001)))
    if sets:
        out["sets"] = sets
    if alike:
        out["alike"] = alike
    na, aa, la = plan(st, ca)
    nb, ab, lb = plan(st, cb)
    out.update({"objectsA": na, "objectsB": nb, "spaceArea": ab - aa, "wallLength": lb - la})
    return out


class Case(occ.Case):
    def query(self, q=None):
        q = q or {}
        rest = {k: v for k, v in q.items() if k != "compareCombos"}
        try:
            out, warnings = oc.query(self.st.doc, rest)
            out = {k: v for k, v in out.items() if k != "revision"}
            if "compareCombos" in q:
                out["combosCompared"] = compare_combos(self.st, q["compareCombos"])
            rows = occ.combo_rows(self.st)
            if rows:
                out["combos"] = rows
            out["revision"] = "$current"
            result = {"status": "completed", "output": oc.mapped(out), "warnings": warnings}
        except Refused as r:
            result = {"status": "failed", "error": r.error}
        self.steps.append({"op": "execute", "command": "cad.options.query", "input": q, "result": result, "within": 1e-9})
        return self


def two(name):
    return occ.two_sets(Case(name, other=True))


def offers(c):
    """Teklif 1 (the primaries now), Teklif B (both B), Teklif M (only the kitchen's B)."""
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b"), ch("not", "b")]}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif M", "choices": [ch("mutfak", "b")]}, quiet=True)
    return c


def cases():
    out = []
    c = offers(two("İki küme ayrı: ikisi de seçenek seçenek karşılaştırılır"))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-b"}})
    out.append(c.done())

    c = offers(two("Yalnız mutfak ayrı: not kümesi aynı"))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m"}})
    out.append(c.done())

    c = offers(two("Ters yön: farkların işareti döner"))
    c.query({"compareCombos": {"a": "teklif-b", "b": "teklif-1"}})
    out.append(c.done())

    c = offers(two("Bileşimler adlarıyla, büyük küçük harften bağımsız"))
    c.query({"compareCombos": {"a": "teklif 1", "b": "TEKLIF M"}})
    out.append(c.done())

    c = two("Aynı şeyi gösteren iki bileşim: bütün kümeler aynı")
    c.edit({"kind": "saveCombo", "name": "Açık", "choices": [ch("mutfak", "a"), ch("not", "a")]}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Boş", "choices": []}, quiet=True)
    c.query({"compareCombos": {"a": "acik", "b": "bos"}})
    out.append(c.done())

    c = offers(two("Yalnız notu seçen bileşimle mutfağı seçen"))
    c.edit({"kind": "saveCombo", "name": "Teklif N", "choices": [ch("not", "b")]}, quiet=True)
    c.query({"compareCombos": {"a": "teklif-m", "b": "teklif-n"}})
    out.append(c.done())

    c = offers(two("B'nin bölme duvarı 30 cm kaydı: duvar ve kapısı değişti"))
    c.run("cad.entities.transform", {"uids": ["$uidOf:16"], "transform": {"kind": "move", "dx": 0.3, "dy": 0.0}}, lambda st: oc.moved(16, 0.3)(st.doc))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m"}})
    out.append(c.done())

    c = offers(two("B'nin donatısı 80 cm kaydı: 50 cm aramada eşleşmez"))
    c.run("cad.entities.transform", {"uids": ["$uidOf:20"], "transform": {"kind": "move", "dx": 0.8, "dy": 0.0}}, lambda st: oc.moved(20, 0.8)(st.doc))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m"}})
    out.append(c.done())

    c = offers(two("Aynı kayma 1 m aramada değişen çift"))
    c.run("cad.entities.transform", {"uids": ["$uidOf:20"], "transform": {"kind": "move", "dx": 0.8, "dy": 0.0}}, lambda st: oc.moved(20, 0.8)(st.doc))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m", "search": 1.0}})
    out.append(c.done())

    c = offers(two("Kayma 30 cm, tolerans 50 cm: duvar aynı sayılır"))
    c.run("cad.entities.transform", {"uids": ["$uidOf:16"], "transform": {"kind": "move", "dx": 0.3, "dy": 0.0}}, lambda st: oc.moved(16, 0.3)(st.doc))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m", "search": 1.0, "tolerance": 0.5}})
    out.append(c.done())

    c = offers(two("B'nin mahali silindi: yalnız A'da, planın alanı düşer"))

    def gone(st):
        del st.doc.ents[18]
    c.run("cad.entities.delete", {"uids": ["$uidOf:18"]}, gone)
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m"}})
    out.append(c.done())

    c = offers(two("B'ye çizgi eklendi: yalnız B'de"))
    c.edit({"kind": "addMembers", "set": "mutfak", "option": "b", "uids": [UID(15)]}, quiet=True)
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m"}})
    out.append(c.done())

    c = offers(two("Üç seçenekli kümede C ile B"))
    c.edit({"kind": "copy", "set": "mutfak", "option": "a", "name": "Geniş"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif C", "choices": [ch("mutfak", "Geniş")]}, quiet=True)
    c.query({"compareCombos": {"a": "teklif-m", "b": "teklif-c"}})
    out.append(c.done())

    c = offers(two("Uygulanan bileşim karşılaştırmayı değiştirmez"))
    c.edit({"kind": "applyCombo", "combo": "teklif-b"}, quiet=True)
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-b"}})
    out.append(c.done())

    c = offers(two("Birincil değişince seçimsiz küme yeni birincili gösterir"))
    c.edit({"kind": "primary", "set": "not", "option": "b"}, quiet=True)
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m"}})
    out.append(c.done())

    c = offers(two("Seçeneği silinen bileşim birincili gösterir; kümeler aynı"))
    c.edit({"kind": "delete", "set": "mutfak", "option": "b"}, quiet=True)
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-m"}})
    out.append(c.done())

    c = offers(two("Seçenek karşılaştırmasıyla birlikte"))
    c.query({"compare": {"set": "not", "a": "a", "b": "b"}, "compareCombos": {"a": "teklif-1", "b": "teklif-b"}})
    out.append(c.done())

    c = offers(two("Kümeyle süzülen okuma bütün kümeleri karşılaştırır"))
    c.query({"set": "not", "compareCombos": {"a": "teklif-1", "b": "teklif-b"}})
    out.append(c.done())

    c = Case("Tek kümeli projede", other=True)
    c.setup(occ.KITCHEN, occ.KITCHEN_B)
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-b"}})
    out.append(c.done())

    c = offers(two("Ret: olmayan ilk bileşim"))
    c.query({"compareCombos": {"a": "teklif-x", "b": "teklif-b"}})
    out.append(c.done())

    c = offers(two("Ret: olmayan ikinci bileşim"))
    c.query({"compareCombos": {"a": "teklif-1", "b": "Teklif X"}})
    out.append(c.done())

    c = offers(two("Ret: bileşim kendisiyle karşılaştırılmaz (kimlik ve ad)"))
    c.query({"compareCombos": {"a": "teklif-b", "b": "Teklif B"}})
    out.append(c.done())

    c = offers(two("Ret: eksi arama uzaklığı"))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-b", "search": -0.1}})
    out.append(c.done())

    c = offers(two("Ret: eksi tolerans"))
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-b", "tolerance": -1.0}})
    out.append(c.done())

    c = two("Ret: bileşimsiz projede")
    c.query({"compareCombos": {"a": "teklif-1", "b": "teklif-b"}})
    out.append(c.done())

    c = offers(two("Ret: önce olmayan küme süzgeci söylenir"))
    c.query({"set": "banyo", "compareCombos": {"a": "teklif-x", "b": "teklif-b"}})
    out.append(c.done())
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "cad.options.query", "commandVersion": 1,
            "title": "İki bileşimi karşılaştır (docs/adr/0809)",
            "note": "scripts/fixtures/option_combo_compare_cases.py yazar: ADR 0809'un kurallarından, KentOS kodu "
                    "olmadan. Değiştirmeyin; betiği düzeltin.",
            "setup": oc.setup(), "cases": cases()}
    assert len(data["cases"]) >= 20, len(data["cases"])
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)}: başvurudan farklı", file=sys.stderr)
            sys.exit(1)
        print(f"cad.options.query ({OUT.name}): {len(data['cases'])} durum başvuruyla aynı.")
    else:
        OUT.write_text(text)
        print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum yazıldı.")


if __name__ == "__main__":
    main()
