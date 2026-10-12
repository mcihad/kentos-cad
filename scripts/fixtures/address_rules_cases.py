#!/usr/bin/env python3
"""The rules of a project's address data (docs/adr/0229 §2) as shared cases: fixtures/geocode/v1/rules.json.

Without --check: writes the cases' values (a locator, or a project's list of them) from the ADR's rules, each with the
verdict of tools/kcad/kcad.py's own copy of the rules; a case's words (`problem`) are the contract's and Rust writes
them (`KENTOS_WRITE_ADDRESS_RULES=1 cargo test -p kentos-contracts --test all address_rules`), words already in the file
are kept. With --check: the reader written from the specification refuses exactly the cases the contract gives words
for.

    python3 scripts/fixtures/address_rules_cases.py [--check]
"""
import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/geocode/v1/rules.json"
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402


def numarataj():
    return {
        "id": "adres-1",
        "name": "Numarataj",
        "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO", "neighbourhood": "MAHALLE", "district": "ILCE",
                  "province": "IL", "postcode": "POSTA_KODU", "building": "BINA_ADI", "code": "UAVT"},
        "streets": {"layer": "yol", "name": "YOL_ADI", "neighbourhood": "MAHALLE"},
        "areas": [{"layer": "mahalle", "level": "neighbourhood", "name": "AD", "district": "ILCE"},
                  {"layer": "ilce", "level": "district", "name": "AD"}],
        "minScore": 85,
        "reach": 100.0,
    }


def plain():
    return {"id": "a", "name": "A", "streets": {"layer": "yol", "name": "AD"}, "minScore": 50, "reach": 0.001}


def changed(base, f):
    v = copy.deepcopy(base)
    f(v)
    return v


def set_(key, value):
    def f(v):
        if value is None:
            v.pop(key, None)
        else:
            v[key] = value

    return f


def door(**fields):
    def f(v):
        v["doors"] = {**v["doors"], **fields}
        for k, x in fields.items():
            if x is None:
                v["doors"].pop(k)

    return f


def area(level, **extra):
    return {"layer": level, "level": level, "name": "AD", **extra}


LOCATORS = [
    ("numarataj", numarataj()),
    ("en kısa tanım", plain()),
    ("yalnız bölgeler", {"id": "b", "name": "B", "areas": [area("province")], "minScore": 100, "reach": 10000.0}),
    ("kimlik büyük harf", changed(plain(), set_("id", "Adres-1"))),
    ("kimlik Türkçe harf", changed(plain(), set_("id", "adres-ı"))),
    ("kimlik 40 karakter", changed(plain(), set_("id", "a" * 40))),
    ("kimlik 41 karakter", changed(plain(), set_("id", "a" * 41))),
    ("ad boş", changed(plain(), set_("name", ""))),
    ("ad kırpılmamış", changed(plain(), set_("name", "Numarataj "))),
    ("ad 80 karakter", changed(plain(), set_("name", "n" * 80))),
    ("ad 81 karakter", changed(plain(), set_("name", "n" * 81))),
    ("kaynağı yok", {"id": "a", "name": "A", "minScore": 85, "reach": 100.0}),
    ("kapı katmanının kimliği boş", changed(numarataj(), door(layer=""))),
    ("kapıların yol alanı boşluk", changed(numarataj(), door(street="  "))),
    ("kapıların kapı alanı kırpılmamış", changed(numarataj(), door(door="KAPI_NO "))),
    ("kapıların mahalle alanı 65 karakter", changed(numarataj(), door(neighbourhood="m" * 65))),
    ("kapıların posta kodu alanında denetim karakteri", changed(numarataj(), door(postcode="POSTA\tKODU"))),
    ("kapıların adres kodu alanı 64 karakter", changed(numarataj(), door(code="u" * 64))),
    ("yol katmanının kimliği boş", changed(plain(), set_("streets", {"layer": "", "name": "AD"}))),
    ("yolların ilçe alanı boş", changed(plain(), set_("streets", {"layer": "yol", "name": "AD", "district": ""}))),
    ("dört bölge katmanı", changed(plain(), set_("areas", [area("neighbourhood"), area("district"), area("province"), area("province")]))),
    ("aynı düzey iki kez", changed(plain(), set_("areas", [area("district"), area("district")]))),
    ("bölge katmanının kimliği boş", changed(plain(), set_("areas", [{"layer": "", "level": "province", "name": "AD"}]))),
    ("bölgenin ad alanı boş", changed(plain(), set_("areas", [{"layer": "il", "level": "province", "name": " "}]))),
    ("ilçe alanlarında ilçe", changed(plain(), set_("areas", [area("district", district="ILCE")]))),
    ("mahalle alanlarında boş ilçe", changed(plain(), set_("areas", [area("neighbourhood", district="")]))),
    ("en düşük puan 49", changed(plain(), set_("minScore", 49))),
    ("en düşük puan 101", changed(plain(), set_("minScore", 101))),
    ("uzaklık sıfır", changed(plain(), set_("reach", 0.0))),
    ("uzaklık eksi", changed(plain(), set_("reach", -5.0))),
    ("uzaklık 10 000 m'den büyük", changed(plain(), set_("reach", 10000.5))),
]


def second(name, lid):
    return changed(plain(), lambda v: v.update({"id": lid, "name": name}))


LISTS = [
    ("hiç tanım", []),
    ("iki tanım", [numarataj(), plain()]),
    ("aynı kimlik", [plain(), second("B", "a")]),
    ("aynı ad, büyük küçük harf", [plain(), second("a", "b")]),
    ("Türkçe ad: IŞIK ile ışık aynı", [second("IŞIK", "a"), second("ışık", "b")]),
    ("16 tanım", [second(f"A{i}", f"a{i}") for i in range(16)]),
    ("17 tanım", [second(f"A{i}", f"a{i}") for i in range(17)]),
    ("ikincisi kurala uymuyor", [plain(), changed(second("B", "b"), set_("reach", 0.0))]),
]


def build(old):
    words = {}
    if old:
        for group in ("locators", "lists"):
            for c in old.get(group, []):
                words[(group, c["name"])] = c["problem"]
    out = {
        "format": "kentos.address-rules",
        "version": 1,
        "note": "The rules of a project's address data (docs/adr/0229 §2), each case with the contract's words; null where the value is one. "
        "scripts/fixtures/address_rules_cases.py writes the values from the ADR and checks the verdicts with tools/kcad/kcad.py's own copy "
        "of the rules; Rust (crates/shared/contracts/tests/all/address_rules.rs, KENTOS_WRITE_ADDRESS_RULES=1 writes the words) gives the words.",
        "locators": [],
        "lists": [],
    }
    for name, value in LOCATORS:
        refused = kcad.address_locator_problem(value) is not None
        out["locators"].append({"name": name, "value": value, "problem": words.get(("locators", name), "?" if refused else None)})
    for name, value in LISTS:
        refused = kcad.address_locators_problem(value) is not None
        out["lists"].append({"name": name, "value": value, "problem": words.get(("lists", name), "?" if refused else None)})
    return out


def verdicts(cases):
    wrong = []
    for group, problem in (("locators", kcad.address_locator_problem), ("lists", kcad.address_locators_problem)):
        for c in cases[group]:
            refused = problem(c["value"]) is not None
            if refused != (c["problem"] is not None):
                wrong.append(f"{group}/{c['name']}: okuyucu {'reddediyor' if refused else 'kabul ediyor'}")
    return wrong


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="check the verdicts instead of writing the values")
    args = ap.parse_args()
    old = json.loads(OUT.read_text(encoding="utf-8")) if OUT.exists() else None
    if args.check:
        if old is None:
            print(f"{OUT.relative_to(ROOT)} yok: betiği --check olmadan çalıştırın")
            return 1
        fresh = build(old)
        values = lambda d: [(c["name"], c["value"]) for g in ("locators", "lists") for c in d[g]]
        wrong = verdicts(old)
        if values(fresh) != values(old):
            wrong.append("değerler betiğin yazdığından farklı: betiği --check olmadan çalıştırın ve farkı okuyun")
        if any(c["problem"] == "?" for g in ("locators", "lists") for c in old[g]):
            wrong.append("sözleri yazılmamış durumlar var: KENTOS_WRITE_ADDRESS_RULES=1 cargo test -p kentos-contracts --test all address_rules")
        if wrong:
            print("\n".join(wrong))
            return 1
        n = len(old["locators"]) + len(old["lists"])
        print(f"{OUT.relative_to(ROOT)}: {n} durumun hükmü okuyucunun kurallarıyla aynı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(build(old), ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
