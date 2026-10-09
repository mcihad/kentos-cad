#!/usr/bin/env python3
"""The rules of a project's networks (docs/adr/0209 §2) as shared cases: fixtures/network/v1/rules.json.

Without --check: writes the cases' values (a network, or a project's list of them) from the ADR's rules, each with the
verdict of tools/kcad/kcad.py's own copy of the rules; a case's words (`problem`) are the contract's and Rust writes
them (`KENTOS_WRITE_NETWORK_RULES=1 cargo test -p kentos-contracts --test all network_rules`), words already in the
file are kept. With --check: the reader written from the specification refuses exactly the cases the contract gives
words for. Rust (crates/shared/contracts/tests/all/network_rules.rs) and TypeScript
(apps/web/src/model/networkRules.test.ts) give the same words.

    python3 scripts/fixtures/network_rules_cases.py [--check]
"""
import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/rules.json"
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402


def road():
    return {
        "id": "ag-1",
        "name": "Yollar",
        "kind": "road",
        "edges": [{"layer": "yol", "filter": "tur <> 'yaya'"}],
        "connect": "ends",
        "tolerance": 0.01,
        "direction": {"kind": "field", "field": "yon", "forward": ["FT", "1"], "backward": ["TF", "-1"], "closed": ["N"]},
        "costs": [
            {"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0},
            {"name": "Ücret", "kind": "field", "field": "ucret", "unit": "TL"},
        ],
        "closed": "durum = 'kapalı'",
    }


def water():
    return {
        "id": "ag-2",
        "name": "İçme suyu",
        "kind": "utility",
        "edges": [{"layer": "su-hatti"}],
        "junctions": [
            {"layer": "su-vana", "role": "valve", "closed": "durum = 'kapalı'"},
            {"layer": "su-depo", "role": "source"},
            {"layer": "su-vana", "role": "junction", "filter": "tur = 'baglanti'"},
        ],
        "connect": "vertices",
        "tolerance": 0.005,
        "direction": {"kind": "digitized"},
    }


def plain():
    return {"id": "a", "name": "A", "kind": "road", "edges": [{"layer": "0"}], "connect": "ends", "tolerance": 0.0001, "direction": {"kind": "both"}}


def changed(base, f):
    n = copy.deepcopy(base)
    f(n)
    return n


def field_direction(**lists):
    return {"kind": "field", "field": "yon", "forward": lists.get("forward", []), "backward": lists.get("backward", []), "closed": lists.get("closed", [])}


def speed(name="Süre", **extra):
    return {"name": name, "kind": "speed", "field": "hiz", "speed": 50.0, **extra}


def field_cost(name="Ücret", **extra):
    return {"name": name, "kind": "field", "field": "ucret", **extra}


def set_(key, value):
    def f(n):
        n[key] = value

    return f


NETWORKS = [
    ("yol ağı", road()),
    ("şebeke", water()),
    ("en kısa tanım", plain()),
    ("kimlik büyük harf", changed(plain(), set_("id", "Ag-1"))),
    ("kimlik boşluklu", changed(plain(), set_("id", "ag 1"))),
    ("kimlik Türkçe harf", changed(plain(), set_("id", "ağ-1"))),
    ("kimlik 40 karakter", changed(plain(), set_("id", "a" * 40))),
    ("kimlik 41 karakter", changed(plain(), set_("id", "a" * 41))),
    ("kimlik boş", changed(plain(), set_("id", ""))),
    ("ad boş", changed(plain(), set_("name", ""))),
    ("ad yalnız boşluk", changed(plain(), set_("name", "   "))),
    ("ad kırpılmamış", changed(plain(), set_("name", " Yollar"))),
    ("ad 80 karakter", changed(plain(), set_("name", "y" * 80))),
    ("ad 81 karakter", changed(plain(), set_("name", "y" * 81))),
    ("kenar katmanı yok", changed(plain(), set_("edges", []))),
    ("16 kenar katmanı", changed(plain(), set_("edges", [{"layer": f"k{i}"} for i in range(16)]))),
    ("17 kenar katmanı", changed(plain(), set_("edges", [{"layer": f"k{i}"} for i in range(17)]))),
    ("17 düğüm katmanı", changed(plain(), set_("junctions", [{"layer": f"d{i}", "role": "junction"} for i in range(17)]))),
    ("kenar katmanının kimliği boş", changed(plain(), set_("edges", [{"layer": ""}]))),
    ("kenar süzgeci boşluk", changed(plain(), set_("edges", [{"layer": "0", "filter": "  "}]))),
    ("kenar süzgeci 1000 karakter", changed(plain(), set_("edges", [{"layer": "0", "filter": "a" * 1000}]))),
    ("kenar süzgeci 1001 karakter", changed(plain(), set_("edges", [{"layer": "0", "filter": "a" * 1001}]))),
    ("aynı kenar katmanı aynı süzgeçle", changed(plain(), set_("edges", [{"layer": "0", "filter": "a = 1"}, {"layer": "0", "filter": "a = 1"}]))),
    ("aynı kenar katmanı süzgeçsiz iki kez", changed(plain(), set_("edges", [{"layer": "0"}, {"layer": "0"}]))),
    ("aynı katman başka süzgeçlerle", changed(plain(), set_("edges", [{"layer": "0", "filter": "a = 1"}, {"layer": "0", "filter": "a = 2"}]))),
    ("düğüm katmanının kimliği boş", changed(plain(), set_("junctions", [{"layer": "", "role": "valve"}]))),
    ("düğümlerin kapalı ifadesi boşluk", changed(plain(), set_("junctions", [{"layer": "v", "role": "valve", "closed": " "}]))),
    ("aynı düğüm katmanı aynı süzgeçle", changed(plain(), set_("junctions", [{"layer": "v", "role": "valve"}, {"layer": "v", "role": "source"}]))),
    ("tolerans en küçük", changed(plain(), set_("tolerance", 0.0001))),
    ("tolerans sıfır", changed(plain(), set_("tolerance", 0.0))),
    ("tolerans 10 m", changed(plain(), set_("tolerance", 10.0))),
    ("tolerans 10 m'den büyük", changed(plain(), set_("tolerance", 10.0001))),
    ("tolerans eksi", changed(plain(), set_("tolerance", -0.01))),
    ("yön alanı boş", changed(plain(), set_("direction", {**field_direction(forward=["FT"]), "field": ""}))),
    ("yön alanı 65 karakter", changed(plain(), set_("direction", {**field_direction(forward=["FT"]), "field": "f" * 65}))),
    ("yön değeri yok", changed(plain(), set_("direction", field_direction()))),
    ("yalnız kapalı değeri", changed(plain(), set_("direction", field_direction(closed=["N"])))),
    ("17 ileri değeri", changed(plain(), set_("direction", field_direction(forward=[str(i) for i in range(17)])))),
    ("yön değeri boşluk", changed(plain(), set_("direction", field_direction(forward=[" "])))),
    ("yön değeri 41 karakter", changed(plain(), set_("direction", field_direction(forward=["v" * 41])))),
    ("yön değeri iki listede", changed(plain(), set_("direction", field_direction(forward=["FT"], closed=["ft"])))),
    ("yön değeri bir listede iki kez", changed(plain(), set_("direction", field_direction(backward=["TF", " TF "])))),
    ("Türkçe I: IŞIK ile ışık aynı", changed(plain(), set_("direction", field_direction(forward=["IŞIK"], backward=["ışık"])))),
    ("Türkçe İ: İLERİ ile ileri aynı", changed(plain(), set_("direction", field_direction(forward=["İLERİ"], backward=["ileri"])))),
    ("Türkçe I: ILERI ile ileri başka", changed(plain(), set_("direction", field_direction(forward=["ILERI"], backward=["ileri"])))),
    ("8 maliyet", changed(plain(), set_("costs", [speed(f"S{i}") for i in range(8)]))),
    ("9 maliyet", changed(plain(), set_("costs", [speed(f"S{i}") for i in range(9)]))),
    ("maliyetin adı Uzunluk", changed(plain(), set_("costs", [field_cost("Uzunluk")]))),
    ("maliyetin adı UZUNLUK", changed(plain(), set_("costs", [field_cost("UZUNLUK")]))),
    ("maliyetin adı kırpılmamış", changed(plain(), set_("costs", [field_cost("Ücret ")]))),
    ("maliyetin adı 41 karakter", changed(plain(), set_("costs", [field_cost("m" * 41)]))),
    ("maliyet iki kez", changed(plain(), set_("costs", [speed("Süre"), speed("SÜRE")]))),
    ("maliyetin alanı boş", changed(plain(), set_("costs", [{**field_cost(), "field": " "}]))),
    ("süre maliyetinin birimi", changed(plain(), set_("costs", [speed(unit="dk")]))),
    ("süre maliyetinin hızı yok", changed(plain(), set_("costs", [{"name": "Süre", "kind": "speed", "field": "hiz"}]))),
    ("süre maliyetinin hızı sıfır", changed(plain(), set_("costs", [speed(speed=0.0)]))),
    ("süre maliyetinin hızı 1000", changed(plain(), set_("costs", [speed(speed=1000.0)]))),
    ("süre maliyetinin hızı 1000'den büyük", changed(plain(), set_("costs", [speed(speed=1000.5)]))),
    ("alan maliyetinin hızı", changed(plain(), set_("costs", [field_cost(speed=5.0)]))),
    ("alan maliyetinin birimi 12 karakter", changed(plain(), set_("costs", [field_cost(unit="b" * 12)]))),
    ("alan maliyetinin birimi 13 karakter", changed(plain(), set_("costs", [field_cost(unit="b" * 13)]))),
    ("alan maliyetinin birimi kırpılmamış", changed(plain(), set_("costs", [field_cost(unit=" TL")]))),
    ("kapalı kenarlar ifadesi boşluk", changed(plain(), set_("closed", "  "))),
    ("kapalı kenarlar ifadesi", changed(plain(), set_("closed", "durum = 'kapalı'"))),
]


def second(name, nid):
    return changed(plain(), lambda n: n.update({"id": nid, "name": name}))


LISTS = [
    ("hiç ağ", []),
    ("iki ağ", [road(), water()]),
    ("aynı kimlik", [plain(), second("B", "a")]),
    ("aynı ad, büyük küçük harf", [plain(), second("a", "b")]),
    ("Türkçe ad: IRMAK ile ırmak aynı", [second("IRMAK", "a"), second("ırmak", "b")]),
    ("32 ağ", [second(f"A{i}", f"a{i}") for i in range(32)]),
    ("33 ağ", [second(f"A{i}", f"a{i}") for i in range(33)]),
    ("ikincisi kurala uymuyor", [plain(), changed(second("B", "b"), set_("tolerance", 20.0))]),
]


def build(old):
    words = {}
    if old:
        for group in ("networks", "lists"):
            for c in old.get(group, []):
                words[(group, c["name"])] = c["problem"]
    out = {
        "format": "kentos.network-rules",
        "version": 1,
        "note": "The rules of a project's networks (docs/adr/0209 §2), each case with the contract's words; null where the value is one. "
        "scripts/fixtures/network_rules_cases.py writes the values from the ADR and checks the verdicts with tools/kcad/kcad.py's own copy "
        "of the rules; Rust (crates/shared/contracts/tests/all/network_rules.rs, KENTOS_WRITE_NETWORK_RULES=1 writes the words) and "
        "TypeScript (apps/web/src/model/networkRules.test.ts) give the same words.",
        "networks": [],
        "lists": [],
    }
    for name, value in NETWORKS:
        refused = kcad.network_problem(value) is not None
        out["networks"].append({"name": name, "value": value, "problem": words.get(("networks", name), "?" if refused else None)})
    for name, value in LISTS:
        refused = kcad.networks_problem(value) is not None
        out["lists"].append({"name": name, "value": value, "problem": words.get(("lists", name), "?" if refused else None)})
    return out


def verdicts(cases):
    wrong = []
    for group, problem in (("networks", kcad.network_problem), ("lists", kcad.networks_problem)):
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
        values = lambda d: [(c["name"], c["value"]) for g in ("networks", "lists") for c in d[g]]
        wrong = verdicts(old)
        if values(fresh) != values(old):
            wrong.append("değerler betiğin yazdığından farklı: betiği --check olmadan çalıştırın ve farkı okuyun")
        if wrong:
            print("\n".join(wrong))
            return 1
        n = len(old["networks"]) + len(old["lists"])
        print(f"{OUT.relative_to(ROOT)}: {n} durumun hükmü okuyucunun kurallarıyla aynı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(build(old), ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
