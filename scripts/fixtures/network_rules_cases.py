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


def plain_with_layers(count):
    """A utility network with `count` junction layers (the 17-devices case needs them all)."""
    return changed(water(), set_("junctions", [{"layer": f"d{i}", "role": "junction"} for i in range(count)]))


def device(**over):
    return {"layer": "su-vana", "terminals": [{"id": "giris", "name": "Giriş"}, {"id": "cikis", "name": "Çıkış"}],
            "paths": [["giris", "cikis"]], **over}


def config(value="üç yollu", **over):
    return {"value": value, "terminals": [{"id": "a", "name": "A"}, {"id": "b", "name": "B"}, {"id": "c", "name": "C"}],
            "paths": [["a", "b", "c"]], **over}


def configured(*configs, **over):
    """A device chosen by the attribute `tip` among `configs` (docs/adr/0562 §2)."""
    return terminals(device(by="tip", configs=list(configs) or [config()], **over))


def terminals(*devices, **over):
    return {"start": "uc_bas", "end": "uc_son", "devices": list(devices) or [device()], **over}


def subnets(**over):
    return {"name": "Ad", "attribute": "Alt ağ", "reach": "downstream", **over}


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
    # Turns, levels and time profiles (docs/adr/0521).
    ("dönüşler", changed(plain(), set_("turns", {"uTurns": "deadEnds", "right": 5.0, "left": 15.0, "back": 30.0,
                                                 "layers": [{"layer": "donus", "field": "gecikme"}]}))),
    ("geri dönüş yok, gecikmesiz", changed(plain(), set_("turns", {"uTurns": "never"}))),
    ("dönüş gecikmesi 3600 sn", changed(plain(), set_("turns", {"uTurns": "allowed", "left": 3600.0}))),
    ("dönüş gecikmesi 3600 sn'den uzun", changed(plain(), set_("turns", {"uTurns": "allowed", "left": 3600.5}))),
    ("dönüş gecikmesi eksi", changed(plain(), set_("turns", {"uTurns": "allowed", "right": -1.0}))),
    ("5 dönüş katmanı", changed(plain(), set_("turns", {"uTurns": "allowed", "layers": [{"layer": f"d{i}"} for i in range(5)]}))),
    ("dönüş katmanının kimliği boş", changed(plain(), set_("turns", {"uTurns": "allowed", "layers": [{"layer": ""}]}))),
    ("dönüş katmanının alanı kırpılmamış", changed(plain(), set_("turns", {"uTurns": "allowed", "layers": [{"layer": "d", "field": " g"}]}))),
    ("aynı dönüş katmanı aynı süzgeçle", changed(plain(), set_("turns", {"uTurns": "allowed", "layers": [{"layer": "d"}, {"layer": "d"}]}))),
    ("seviyeler", changed(plain(), set_("levels", {"start": "z_bas", "end": "z_son", "inside": "z_orta"}))),
    ("seviyenin bitiş alanı boş", changed(plain(), set_("levels", {"start": "z_bas", "end": ""}))),
    ("zaman profili", changed(plain(), set_("costs", [speed(periods=[{"from": 420, "to": 570, "factor": 0.5},
                                                                      {"from": 1020, "to": 1140, "field": "hiz_aksam"}])]))),
    ("zaman profili 24:00'a kadar", changed(plain(), set_("costs", [speed(periods=[{"from": 1380, "to": 1440, "factor": 0.0}])]))),
    ("dönemler örtüşüyor", changed(plain(), set_("costs", [speed(periods=[{"from": 420, "to": 570, "factor": 0.5},
                                                                           {"from": 500, "to": 600, "factor": 0.7}])]))),
    ("dönem bitişinden önce başlamıyor", changed(plain(), set_("costs", [speed(periods=[{"from": 600, "to": 600, "factor": 0.5}])]))),
    ("dönem günün dışında", changed(plain(), set_("costs", [speed(periods=[{"from": 1400, "to": 1441, "factor": 0.5}])]))),
    ("dönemin çarpanı da alanı da var", changed(plain(), set_("costs", [speed(periods=[{"from": 420, "to": 570, "factor": 0.5, "field": "h"}])]))),
    ("dönemin çarpanı da alanı da yok", changed(plain(), set_("costs", [speed(periods=[{"from": 420, "to": 570}])]))),
    ("dönemin çarpanı 10'dan büyük", changed(plain(), set_("costs", [speed(periods=[{"from": 420, "to": 570, "factor": 10.5}])]))),
    ("bütün gün kapalı", changed(plain(), set_("costs", [speed(periods=[{"from": 0, "to": 1440, "factor": 0.0}])]))),
    ("25 dönem", changed(plain(), set_("costs", [speed(periods=[{"from": i * 50, "to": i * 50 + 10, "factor": 0.5} for i in range(25)])]))),
    ("alan maliyetinde zaman profili", changed(plain(), set_("costs", [field_cost(periods=[{"from": 420, "to": 570, "factor": 0.5}])]))),
    # Terminals and subnetworks (docs/adr/0557).
    ("terminaller", changed(water(), set_("terminals", terminals()))),
    ("terminaller, kimlik ve ad aynı", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "yg", "name": "YG"}, {"id": "ag", "name": "ag"}], paths=[["yg", "ag"]]))))),
    ("terminaller, 3 terminalli iki yollu cihaz", changed(water(), set_("terminals", terminals(device(
        terminals=[{"id": "a", "name": "A"}, {"id": "b", "name": "B"}, {"id": "c", "name": "C"}], paths=[["a", "b"], ["b", "c"]]))))),
    ("terminal başlangıç ve bitiş alanı aynı", changed(water(), set_("terminals", terminals(end="uc_bas")))),
    ("terminal başlangıç alanı kırpılmamış", changed(water(), set_("terminals", terminals(start=" uc_bas")))),
    ("terminal bitiş alanı boş", changed(water(), set_("terminals", terminals(end="")))),
    ("terminal alanı 65 karakter", changed(water(), set_("terminals", terminals(start="u" * 65)))),
    ("cihaz yok", changed(water(), set_("terminals", {"start": "uc_bas", "end": "uc_son", "devices": []}))),
    ("17 cihaz", changed(plain_with_layers(16), set_("terminals", {"start": "uc_bas", "end": "uc_son", "devices": [device(layer=f"d{i}") for i in range(16)] + [device(layer="d0")]}))),
    ("cihazın katmanı düğüm katmanı değil", changed(water(), set_("terminals", terminals(device(layer="yok"))))),
    ("aynı katman iki cihaz", changed(water(), set_("terminals", terminals(device(), device())))),
    ("tek terminal", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "giris", "name": "Giriş"}], paths=[["giris", "giris"]]))))),
    ("9 terminal", changed(water(), set_("terminals", terminals(device(terminals=[{"id": f"t{i}", "name": f"T{i}"} for i in range(9)], paths=[["t0", "t1"]]))))),
    ("terminal kimliği büyük harf", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "Giris", "name": "Giriş"}, {"id": "cikis", "name": "Çıkış"}]))))),
    ("terminal adı boş", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "giris", "name": ""}, {"id": "cikis", "name": "Çıkış"}]))))),
    ("terminal adı kırpılmamış", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "giris", "name": "Giriş "}, {"id": "cikis", "name": "Çıkış"}]))))),
    ("terminal adı 41 karakter", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "giris", "name": "g" * 41}, {"id": "cikis", "name": "Çıkış"}]))))),
    ("terminal adı öbürünün kimliği", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "giris", "name": "cikis"}, {"id": "cikis", "name": "Çıkış"}]))))),
    ("terminal adları Türkçe harfle aynı", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "a", "name": "IŞIK"}, {"id": "b", "name": "ışık"}], paths=[["a", "b"]]))))),
    ("terminal adları I ile ı başka", changed(water(), set_("terminals", terminals(device(terminals=[{"id": "a", "name": "ILIK"}, {"id": "b", "name": "ilik"}], paths=[["a", "b"]]))))),
    ("yol yok", changed(water(), set_("terminals", terminals(device(paths=[]))))),
    ("17 yol", changed(water(), set_("terminals", terminals(device(paths=[["giris", "cikis"]] * 17))))),
    ("yolda tek terminal", changed(water(), set_("terminals", terminals(device(paths=[["giris"]]))))),
    ("yolda cihazın olmayan terminali", changed(water(), set_("terminals", terminals(device(paths=[["giris", "yok"]]))))),
    ("yolda aynı terminal iki kez", changed(water(), set_("terminals", terminals(device(paths=[["giris", "giris"]]))))),
    ("alt ağlar", changed(water(), set_("subnetworks", subnets()))),
    ("alt ağlar, adı alanı olmadan", changed(water(), set_("subnetworks", {"attribute": "AltAg", "reach": "connected"}))),
    ("alt ağlar, kaynak katmanı yok", changed(water(), lambda n: (set_("junctions", [{"layer": "su-vana", "role": "valve"}])(n), set_("subnetworks", subnets())(n)))),
    ("alt ağ özniteliği boş", changed(water(), set_("subnetworks", subnets(attribute="")))),
    ("alt ağ özniteliği kırpılmamış", changed(water(), set_("subnetworks", subnets(attribute=" Alt ağ")))),
    ("alt ağ özniteliği 65 karakter", changed(water(), set_("subnetworks", subnets(attribute="a" * 65)))),
    ("alt ağ adı alanı boş", changed(water(), set_("subnetworks", subnets(name="")))),
    ("alt ağ özniteliği adı alanıyla aynı", changed(water(), set_("subnetworks", subnets(attribute="Ad")))),
    ("alt ağ özniteliği terminal alanıyla aynı", changed(water(), lambda n: (set_("terminals", terminals())(n), set_("subnetworks", subnets(attribute="uc_son"))(n)))),
    ("alt ağ özniteliği yön alanıyla aynı", changed(water(), lambda n: (set_("direction", field_direction(forward=["FT"]))(n), set_("subnetworks", subnets(attribute="yon"))(n)))),
    ("terminaller ve alt ağlar birlikte", changed(water(), lambda n: (set_("terminals", terminals())(n), set_("subnetworks", subnets())(n)))),
    # Configurations by an attribute and keeping subnetworks current (docs/adr/0562).
    ("yapılandırma: üç yollu", changed(water(), set_("terminals", configured()))),
    ("yapılandırma: iki yapılandırma, biri trafo", changed(water(), set_("terminals", configured(
        config(), config("trafo", terminals=[{"id": "yg", "name": "Yüksek gerilim"}, {"id": "ag", "name": "Alçak gerilim"}], paths=[["yg", "ag"]]))))),
    ("yapılandırma: kimlikler cihazın kendisiyle aynı olabilir", changed(water(), set_("terminals", configured(
        config("iki", terminals=[{"id": "giris", "name": "Giriş"}, {"id": "cikis", "name": "Çıkış"}], paths=[["giris", "cikis"]]))))),
    ("yapılandırma: değerler I ile ı başka", changed(water(), set_("terminals", configured(config("ILIK"), config("ilik"))))),
    ("yapılandırma var, seçen alan yok", changed(water(), set_("terminals", terminals(device(configs=[config()]))))),
    ("seçen alan var, yapılandırma yok", changed(water(), set_("terminals", terminals(device(by="tip"))))),
    ("seçen alan boş", changed(water(), set_("terminals", terminals(device(by="", configs=[config()]))))),
    ("seçen alan kırpılmamış", changed(water(), set_("terminals", terminals(device(by=" tip", configs=[config()]))))),
    ("seçen alan 65 karakter", changed(water(), set_("terminals", terminals(device(by="t" * 65, configs=[config()]))))),
    ("17 yapılandırma", changed(water(), set_("terminals", configured(*[config(f"v{i}") for i in range(17)])))),
    ("yapılandırma değeri boş", changed(water(), set_("terminals", configured(config(""))))),
    ("yapılandırma değeri kırpılmamış", changed(water(), set_("terminals", configured(config("üç yollu "))))),
    ("yapılandırma değeri 41 karakter", changed(water(), set_("terminals", configured(config("v" * 41))))),
    ("yapılandırma değeri iki kez", changed(water(), set_("terminals", configured(config(), config())))),
    ("yapılandırma değeri Türkçe harfle iki kez", changed(water(), set_("terminals", configured(config("IŞIK"), config("ışık"))))),
    ("yapılandırmada tek terminal", changed(water(), set_("terminals", configured(config(terminals=[{"id": "a", "name": "A"}], paths=[["a", "a"]]))))),
    ("yapılandırmada 9 terminal", changed(water(), set_("terminals", configured(config(terminals=[{"id": f"t{i}", "name": f"T{i}"} for i in range(9)], paths=[["t0", "t1"]]))))),
    ("yapılandırmada terminal kimliği büyük harf", changed(water(), set_("terminals", configured(config(terminals=[{"id": "A", "name": "A"}, {"id": "b", "name": "B"}], paths=[["A", "b"]]))))),
    ("yapılandırmada terminal adı öbürünün kimliği", changed(water(), set_("terminals", configured(config(terminals=[{"id": "a", "name": "b"}, {"id": "b", "name": "B"}], paths=[["a", "b"]]))))),
    ("yapılandırmada yol yok", changed(water(), set_("terminals", configured(config(paths=[]))))),
    ("yapılandırmada yolda tek terminal", changed(water(), set_("terminals", configured(config(paths=[["a"]]))))),
    ("yapılandırmada yolda olmayan terminal", changed(water(), set_("terminals", configured(config(paths=[["a", "z"]]))))),
    ("yapılandırmada yolda aynı terminal iki kez", changed(water(), set_("terminals", configured(config(paths=[["a", "a"]]))))),
    ("alt ağlar: güncel tut", changed(water(), set_("subnetworks", subnets(keepCurrent=True)))),
    ("alt ağlar: güncel tut, terminaller ve yapılandırmalarla", changed(water(), lambda n: (set_("terminals", configured())(n), set_("subnetworks", subnets(keepCurrent=True))(n)))),
    ("alt ağ özniteliği yapılandırmayı seçen alanla aynı", changed(water(), lambda n: (set_("terminals", configured())(n), set_("subnetworks", subnets(attribute="tip"))(n)))),
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
