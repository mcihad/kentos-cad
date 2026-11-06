#!/usr/bin/env python3
"""The shared cases of `road.pay.edit` and `road.pay.query` (plan/road.md Y11.4, docs/adr/0909 §4; KentOS Yol's pay
item quantities).

The sources' measures are their own references': road_guardrail_cases.py's guardrails, road_culvert_cases.py's
culverts, road_wall_cases.py's walls and road_volume_command_cases.py's earthwork volumes, one drawing holding them
all; the book's rules (each rule's measure of every source of its kind, of its material when given, times its factor;
an item the sum; an amount its unit price times its quantity) are ADR 0909 §4's. No KentOS code. The items, codes and
prices are the cases' own, not an agency's.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_pay_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_culvert_cases as cv  # noqa: E402
import road_grades  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402
import road_volume_command_cases as vc  # noqa: E402
import road_wall_cases as wv  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = ccmd.UNCHANGED, ccmd.STEPPED, ccmd.CONFLICT, ccmd.BAD_REVISION
W = 1e-6
NOT_ROAD = failed("not_a_road_project",
                  "Poz metrajı yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
NO_BOOK = failed("no_pay_book", "Projede poz defteri yok; önce poz defterini yazın.")

ITEMS = [{"code": "P-1", "name": "Kutu menfez betonu", "unit": "m³", "price": 100.0},
         {"code": "P-2", "name": "Menfez borusu", "unit": "m", "price": 50.0},
         {"code": "P-3", "name": "Otokorkuluk", "unit": "m"},
         {"code": "P-4", "name": "İstinat duvarı betonu", "unit": "m³", "price": 120.0},
         {"code": "P-5", "name": "Kazı (kabarmış)", "unit": "m³", "price": 10.0},
         {"code": "P-6", "name": "Dolgu", "unit": "m³"}]
RULES = [{"measure": "culvertConcrete", "item": "P-1", "factor": 1.0, "material": "Betonarme"},
         {"measure": "culvertPipes", "item": "P-2", "factor": 1.0},
         {"measure": "guardrailLength", "item": "P-3", "factor": 1.0},
         {"measure": "wallConcrete", "item": "P-4", "factor": 1.05},
         {"measure": "volumeFactoredCut", "item": "P-5", "factor": 1.0},
         {"measure": "volumeFill", "item": "P-6", "factor": 1.0}]
BOOK = {"source": "Durumların poz listesi", "version": "2026-1", "revision": 1, "currency": "TRY", "items": ITEMS,
        "rules": RULES}
SOURCES = {"guardrails": grc.START, "culverts": cv.START, "walls": wv.START, "volumes": vc.START}


def road_of(book=BOOK, sources=None):
    s = {**SOURCES, **(sources or {})}
    out = ccmd.road_of(grc.CORRIDORS, volumes=s["volumes"])
    for k in ("guardrails", "culverts", "walls"):
        if s[k]:
            out[k] = copy.deepcopy(list(s[k]))
    if "walls" in out:
        out["walls"] = [{**w, "shape": wv.compact(w["shape"])} for w in out["walls"]]
    if book is not None:
        out["pay"] = copy.deepcopy(book)
    return out


def setup(book=BOOK, sources=None, work="road"):
    base = grc.setup([], work)
    if work == "road":
        base["settings"]["road"] = road_of(book, sources)
    return base


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, words):
    return Refused("invalid_pay_book", words, path or None)


KIND = {"guardrailLength": "guardrail", "culvertLength": "culvert", "culvertPipes": "culvert",
        "culvertConcrete": "culvert", "wallLength": "wall", "wallFace": "wall", "wallConcrete": "wall",
        "volumeCut": "volume", "volumeFill": "volume", "volumeFactoredCut": "volume", "volumeFactoredFill": "volume"}
WORD = {"guardrail": "otokorkuluğu", "culvert": "menfezi", "wall": "istinat duvarı", "volume": "kübajı"}


def read_kind(kind, s):
    out = []
    if kind == "guardrail":
        for g in s["guardrails"]:
            try:
                r = grc.read(g, s["guardrails"])[0]
                vals = {"guardrailLength": r["left"] + r["right"]}
                stale = r["stale"]
            except grc.Refused:
                vals, stale = None, False
            out.append({"kind": kind, "id": g["id"], "name": g["name"], "material": None, "stale": stale,
                        "values": vals})
    elif kind == "culvert":
        for c in s["culverts"]:
            try:
                r = cv.read(c)[0]
                vals = {"culvertLength": r["length"], "culvertPipes": r["pipes"], "culvertConcrete": r["concrete"]}
                stale = r["stale"]
            except cv.Refused:
                vals, stale = None, False
            out.append({"kind": kind, "id": c["id"], "name": c["name"], "material": c.get("material"),
                        "stale": stale, "values": vals})
    elif kind == "wall":
        for w in s["walls"]:
            try:
                r = wv.read(w)[0]
                vals = {"wallLength": r["length"], "wallFace": r["face"], "wallConcrete": r["concrete"]}
                stale = r["stale"]
            except wv.Refused:
                vals, stale = None, False
            out.append({"kind": kind, "id": w["id"], "name": w["name"], "material": w.get("material"),
                        "stale": stale, "values": vals})
    else:
        for v in s["volumes"]:
            r = vc.report(v, grc.CORRIDORS)
            read = "problem" not in r
            vals = {"volumeCut": r["cut"], "volumeFill": r["fill"], "volumeFactoredCut": r["factoredCut"],
                    "volumeFactoredFill": r["factoredFill"]} if read else None
            out.append({"kind": kind, "id": v["id"], "name": v["name"], "material": None,
                        "stale": read and r["stale"], "values": vals})
    return out


def answer(book, sources=None, item=None):
    s = {**SOURCES, **(sources or {})}
    if book is None:
        raise Refused("no_pay_book", "Projede poz defteri yok; önce poz defterini yazın.")
    items = book["items"]
    if item is not None:
        items = [i for i in items if i["code"] == item]
        if not items:
            raise bad("item", f"“{item}” numaralı poz defterde yok.")
    kinds, seen, warnings, rows = {}, set(), [], []
    for it in items:
        lines = []
        for rule in [r for r in book.get("rules", []) if r["item"] == it["code"]]:
            kind = KIND[rule["measure"]]
            if kind not in kinds:
                kinds[kind] = read_kind(kind, s)
            for src in kinds[kind]:
                if rule.get("material") is not None and src["material"] != rule["material"]:
                    continue
                key = (kind, src["id"])
                if key not in seen:
                    seen.add(key)
                    if src["values"] is None:
                        warnings.append({"code": "pay_source_unread", "path": "rules",
                                         "message": f"“{src['name']}” {WORD[kind]} okunamıyor; metrajı yok."})
                    elif src["stale"]:
                        warnings.append({"code": "pay_source_stale", "path": "rules",
                                         "message": f"“{src['name']}” {WORD[kind]} koridoru değişti; metrajı "
                                                    "eskimiş olabilir."})
                if src["values"] is None:
                    continue
                value = src["values"][rule["measure"]]
                lines.append({"kind": kind, "id": src["id"], "name": src["name"], "measure": rule["measure"],
                              "value": value, "factor": rule["factor"], "quantity": value * rule["factor"],
                              "stale": src["stale"]})
        quantity = sum(line["quantity"] for line in lines)
        row = {"code": it["code"], "name": it["name"], "unit": it["unit"], "quantity": quantity, "lines": lines}
        if it.get("price") is not None:
            row["price"] = it["price"]
            row["amount"] = it["price"] * quantity
        rows.append(row)
    out = {"source": book["source"], "version": book["version"], "revision": book.get("revision", 0),
           "rows": rows, "total": sum(r.get("amount", 0.0) for r in rows),
           "unpriced": sum(1 for r in rows if "price" not in r and r["quantity"] > 0)}
    if book.get("currency") is not None:
        out["currency"] = book["currency"]
    return out, warnings


def rule_problem(b):
    """The book's rules (contracts' pay_problem); the field after the item's or rule's index, else none."""
    def no(field, words):
        raise bad(field, f"Poz defteri kurallarına uymuyor: {words}.")

    def text(t, most):
        return bool(t.strip()) and t.strip() == t and len(t) <= most
    if not text(b["source"], 200):
        no("", "poz defterinin kaynağı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 200 karakter")
    if not text(b["version"], 64):
        no("", "poz defterinin sürümü boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 64 karakter")
    if b.get("currency") is not None and not text(b["currency"], 12):
        no("", "para birimi boş olmamalı, en çok 12 karakter")
    if not 1 <= len(b["items"]) <= 2000:
        no("", "poz defterinde 1 ile 2000 arası poz olur")
    codes = set()
    for it in b["items"]:
        if not text(it["code"], 64):
            no("code", "pozun numarası boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 64 karakter")
        if it["code"] in codes:
            no("code", "bu numarada başka bir poz var")
        codes.add(it["code"])
        if not text(it["name"], 300):
            no("name", "pozun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 300 karakter")
        if not text(it["unit"], 16):
            no("unit", "pozun birimi boş olmamalı, en çok 16 karakter")
        if it.get("price") is not None and not 0 <= it["price"] <= 1e12:
            no("price", "pozun birim fiyatı 0 ya da büyük, sonlu olmalı")
    for r in b.get("rules", []):
        if r["item"] not in codes:
            no("item", "kuralın pozu defterde yok")
        if not 0 < r["factor"] <= 1e6:
            no("factor", "kuralın çarpanı 0'dan büyük, en çok 1 000 000 olur")
        if r.get("material") is not None:
            if KIND[r["measure"]] not in ("culvert", "wall"):
                no("material", "malzeme yalnız menfez ve istinat duvarı ölçülerinde olur")
            if not text(r["material"], 120):
                no("material", "kuralın malzemesi boş olmamalı, en çok 120 karakter")


def edit(input_, start=BOOK):
    op = input_["operation"]
    if op == "remove":
        if start is None:
            raise Refused("no_pay_book", "Projede poz defteri yok; önce poz defterini yazın.")
        return None
    if op == "rebind":
        raise bad("operation", "Poz defteri bir koridora bağlı değil; yeniden bağlanmaz.")
    fields = ("source", "version", "currency", "items", "rules")
    if start is not None:
        if not any(k in input_ for k in fields):
            raise bad("", "Değiştirilecek bir şey yok: defterin bir alanını verin.")
        d = copy.deepcopy(start)
        for k in fields:
            if k in input_:
                if input_[k] is None:
                    d.pop(k, None)
                else:
                    d[k] = copy.deepcopy(input_[k])
        if d != start:
            d["revision"] = start.get("revision", 0) + 1
    else:
        missing = next((f for f in ("source", "version", "items") if input_.get(f) is None), None)
        if missing:
            raise bad(missing, "Yeni poz defteri için source, version ve items verilmeli.")
        d = {"source": input_["source"], "version": input_["version"], "revision": 1, "items": input_["items"]}
        if input_.get("currency") is not None:
            d["currency"] = input_["currency"]
        if input_.get("rules"):
            d["rules"] = input_["rules"]
    if not d.get("rules"):
        d.pop("rules", None)
    rule_problem(d)
    return d


def edit_cases():
    cases = []

    def ok(name, input_, op="execute", undo=False, start=BOOK):
        book = edit(input_, start)
        out = {"revision": "$current"}
        if book is not None:
            out["book"] = book
        if op == "validate":
            result, expect = completed(None), {**UNCHANGED, "road": road_of(start)}
        elif op == "plan":
            result, expect = completed(out), {**UNCHANGED, "road": road_of(start)}
        elif book == start:
            result, expect = completed(out), {**UNCHANGED, "road": road_of(start)}
        else:
            result, expect = completed(out), {**STEPPED, "road": road_of(book)}
        s = step(op, input_, result, expect)
        s["within"] = W
        steps = [s]
        if undo:
            steps.append({"op": "undo", "returns": "Poz defteri", "expect": {"road": road_of(start), "canRedo": True}})
        cases.append(case(name, steps, None if start is BOOK else setup(start)))

    def no(name, input_, result=None, setup_=None, start=BOOK):
        if result is None:
            try:
                edit(input_, start)
            except Refused as r:
                result = r.result
            else:
                raise AssertionError(f"{name}: refuses")
        road = None if setup_ is not None and start is BOOK else road_of(start)
        if setup_ is None and start is not BOOK:
            setup_ = setup(start)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    new = {"operation": "set", "source": "İdarenin birim fiyat listesi", "version": "2026", "currency": "TRY",
           "items": [{"code": "A-1", "name": "Menfez betonu", "unit": "m³", "price": 90.0}],
           "rules": [{"measure": "culvertConcrete", "item": "A-1", "factor": 1.0}]}
    ok("Yeni defter", new, start=None)
    ok("Yeni defter, kuralsız ve parasız", {k: v for k, v in new.items() if k not in ("rules", "currency")},
       start=None)
    ok("Kaynağı ve sürümü değiştir", {"operation": "set", "source": "Yeni liste", "version": "2026-2"})
    ok("Para birimini kaldır", {"operation": "set", "currency": None})
    ok("Pozları değiştir", {"operation": "set", "items": ITEMS[:4] + [{**ITEMS[4], "price": 12.5}, ITEMS[5]]})
    ok("Kuralları değiştir", {"operation": "set", "rules": RULES[:3]})
    ok("Kuralları kaldır", {"operation": "set", "rules": []})
    ok("Aynısını yaz: sürüm artmaz", {"operation": "set", "version": "2026-1"})
    ok("Kaldır", {"operation": "remove"})
    ok("Planı", {"operation": "set", "version": "2026-3"}, op="plan")
    ok("Doğrulama", {"operation": "set", "version": "2026-3"}, op="validate")
    ok("Tek geri alma adımı", {"operation": "set", "version": "2026-3"}, undo=True)
    no("Yol projesi değil", new, NOT_ROAD, setup(work="map"))
    no("Olmayan defteri kaldır", {"operation": "remove"}, start=None)
    no("Yeniden bağlanmaz", {"operation": "rebind"})
    no("Pozsuz yeni", {k: v for k, v in new.items() if k != "items"}, start=None)
    no("Değiştirilecek bir şey yok", {"operation": "set"})
    no("Boş pozlar", {"operation": "set", "items": []})
    no("Kaynaksız", {"operation": "set", "source": " "})
    no("Aynı numara iki kez", {"operation": "set", "items": ITEMS + [{**ITEMS[0], "name": "Başka"}]})
    no("Eksi fiyat", {"operation": "set", "items": [{**ITEMS[0], "price": -1.0}] + ITEMS[1:]})
    no("Kuralın pozu yok", {"operation": "set", "rules": [{**RULES[0], "item": "Z-9"}]})
    no("Sıfır çarpan", {"operation": "set", "rules": [{**RULES[1], "factor": 0.0}]})
    no("Kübajda malzeme", {"operation": "set", "rules": [{**RULES[4], "material": "Betonarme"}]})
    no("Geçersiz beklenen sürüm", {"operation": "set", "version": "x", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"operation": "set", "version": "x", "expectedRevision": "41"}, CONFLICT)
    return cases


def query_cases():
    cases = []

    def ok(name, input_, book=BOOK, sources=None):
        out, warns = answer(book, sources, input_.get("item"))
        s = step("execute", input_, completed(out, warns), {**UNCHANGED, "road": road_of(book, sources)})
        s["within"] = W
        cases.append(case(name, [s], None if book is BOOK and sources is None else setup(book, sources)))

    def no(name, input_, result, setup_=None):
        cases.append(case(name, [step("execute", input_, result, UNCHANGED)], setup_))

    ok("Bütün pozlar", {})
    for it in ITEMS:
        ok(f"Tek poz: {it['name']}", {"item": it["code"]})
    ok("Bir poza iki kural", {}, {**BOOK, "rules": RULES + [{"measure": "culvertConcrete", "item": "P-4", "factor": 1.0}]})
    ok("Çarpanlı", {"item": "P-3"}, {**BOOK, "rules": [{**RULES[2], "factor": 2.5}]})
    ok("Malzemesi tutmayan kural", {"item": "P-1"}, {**BOOK, "rules": [{**RULES[0], "material": "Çelik"}]})
    ok("Malzemesiz bütün menfezler", {"item": "P-1"}, {**BOOK, "rules": [{k: v for k, v in RULES[0].items()
                                                                          if k != "material"}]})
    ok("Duvar boyu ve yüzü", {}, {**BOOK, "rules": [{"measure": "wallLength", "item": "P-3", "factor": 1.0},
                                                       {"measure": "wallFace", "item": "P-4", "factor": 1.0}]})
    ok("Menfez boyu ve kabarmasız kazı", {}, {**BOOK, "rules": [
        {"measure": "culvertLength", "item": "P-2", "factor": 1.0}, {"measure": "volumeCut", "item": "P-5", "factor": 1.0},
        {"measure": "volumeFactoredFill", "item": "P-6", "factor": 1.0}]})
    ok("Kuralsız defter", {}, {k: v for k, v in BOOK.items() if k != "rules"})
    ok("Okunamayan menfez", {"item": "P-2"}, BOOK,
       {"culverts": cv.START + [{k: v for k, v in {**cv.M1, "id": "m-3", "name": "Yarma", "station": 60.0}.items() if k != "material"}]})
    ok("Yapısız proje", {}, BOOK, {"guardrails": [], "culverts": [], "walls": [], "volumes": []})
    ok("Parasız defter", {}, {**{k: v for k, v in BOOK.items() if k != "currency"}, "items": [{k: v for k, v in i.items() if k != "price"}
                                                                  for i in ITEMS]})
    no("Yol projesi değil", {}, NOT_ROAD, setup(work="map"))
    no("Defter yok", {}, NO_BOOK, setup(None))
    no("Olmayan poz", {"item": "Z-9"}, failed("invalid_pay_book", "“Z-9” numaralı poz defterde yok.", "item"))
    return cases


def file(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_pay_cases.py yazar; ölçüler yapıların ve kübajın kendi başvurularından "
                    "(road_guardrail_cases.py, road_culvert_cases.py, road_wall_cases.py, "
                    "road_volume_command_cases.py), kurallar ADR 0909 §4'ten; KentOS kodu yok. Pozlar, numaralar ve "
                    "fiyatlar durumlarındır, bir idarenin değil.",
            "setup": setup(), "cases": cases}


def main():
    files = {"road.pay.edit": file("road.pay.edit", "Poz defterini düzenle", edit_cases()),
             "road.pay.query": file("road.pay.query", "Poz metrajını sorgula", query_cases())}
    check = "--check" in sys.argv
    for name, data in files.items():
        path = OUT / f"{name}.json"
        data = road_grades.fixture(data)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if path.read_text(encoding="utf-8") != text:
                raise SystemExit(f"{path} güncel değil: python3 {Path(__file__).name}")
            print(f"{path.name}: {len(data['cases'])} durum güncel")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
