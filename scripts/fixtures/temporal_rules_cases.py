#!/usr/bin/env python3
"""The rules of temporal layers and scenarios in the layer tree (docs/adr/0210 §2) as shared cases:
fixtures/temporal/v1/rules.json.

Without --check: writes the cases' values (a layer's time setting, a scenario, a layer tree) from the ADR's rules, each
with the verdict of tools/kcad/kcad.py's own copy of the rules; a case's words (`problem`) are the contract's and Rust
writes them (`KENTOS_WRITE_TEMPORAL_RULES=1 cargo test -p kentos-contracts --test all temporal_rules`), words already in
the file are kept. With --check: the reader written from the specification refuses exactly the cases the contract gives
words for. Rust (crates/shared/contracts/tests/all/temporal_rules.rs) and TypeScript
(apps/web/src/model/temporalRules.test.ts) give the same words.

    python3 scripts/fixtures/temporal_rules_cases.py [--check]
"""
import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/temporal/v1/rules.json"
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402

GROUPS = ("times", "scenarios", "trees")


def time(**f):
    base = {"start": "gecerlilik_baslangic", "end": "gecerlilik_bitis"}
    base.update(f)
    return {k: v for k, v in base.items() if v is not None}


TIMES = [
    ("aralıklı", time()),
    ("anlık", time(end=None)),
    ("anahtarlı ve birikimli", time(key="parsel_no", cumulative=True)),
    ("Türkçe adlar", time(start="geçerlilik başlangıcı", end="geçerlilik bitişi")),
    ("64 karakter", time(start="b" * 64)),
    ("65 karakter", time(start="b" * 65)),
    ("kimlik 65 karakter", time(key="k" * 65)),
    ("boş başlangıç", time(start="")),
    ("boşluklu başlangıç", time(start=" baslangic")),
    ("boşluklu bitiş", time(end="bitis ")),
    ("boş bitiş", time(end="")),
    ("boş kimlik", time(key="")),
    ("aynı alan", time(end="gecerlilik_baslangic")),
    ("kimlik başlangıçla aynı", time(key="gecerlilik_baslangic")),
]

SCENARIOS = [
    ("notsuz", {}),
    ("notlu", {"note": "12 m'lik yol önerisi"}),
    ("500 karakter", {"note": "n" * 500}),
    ("501 karakter", {"note": "n" * 501}),
    ("boş not", {"note": ""}),
    ("boşluklu not", {"note": " öneri"}),
]


def node(nid, kind="layer", children=(), **extra):
    n = {
        "id": nid,
        "name": nid.replace("-", " ").title(),
        "type": kind,
        "visible": True,
        "locked": False,
        "expanded": True,
        "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
        "children": list(children),
    }
    n.update(extra)
    return n


def alt(children, nid="alt-a", scenario=None):
    return node(nid, "group", children, scenario=scenario or {})


OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png"}

TREES = [
    ("boş ağaç", []),
    ("zamanlı katman", [node("parsel", time=time())]),
    ("grupta zaman", [node("grup", "group", [node("parsel")], time=time())]),
    ("servis katmanında zaman", [node("altlik", service=OSM, time=time())]),
    ("bozuk zaman", [node("parsel", time=time(end="gecerlilik_baslangic"))]),
    ("senaryo ve ana katman", [alt([node("yol-a", replaces="yol"), node("park")]), node("yol")]),
    ("iki senaryo aynı katmanın yerine", [alt([node("yol-a", replaces="yol")]), alt([node("yol-b", replaces="yol")], nid="alt-b"), node("yol")]),
    ("katman senaryo", [node("yol", scenario={})]),
    ("iç içe senaryo", [alt([alt([], nid="alt-b")]), node("yol")]),
    ("bozuk not", [alt([], scenario={"note": ""}), node("yol")]),
    ("senaryo dışında yerine geçen", [node("yol-a", replaces="yol"), node("yol")]),
    ("grup yerine geçiyor", [alt([node("grup", "group", replaces="yol")]), node("yol")]),
    ("boş yerine geçen", [alt([node("yol-a", replaces="")]), node("yol")]),
    ("kendi yerine", [alt([node("yol-a", replaces="yol-a")]), node("yol")]),
    ("senaryo katmanının yerine", [alt([node("yol-a", replaces="yol-b"), node("yol-b")]), node("yol")]),
    ("başka senaryonun katmanının yerine", [alt([node("yol-a", replaces="yol-b")]), alt([node("yol-b")], nid="alt-b"), node("yol")]),
    ("grubun yerine", [alt([node("yol-a", replaces="grup")]), node("grup", "group", [node("yol")])]),
    ("aynı senaryoda iki kez", [alt([node("yol-a", replaces="yol"), node("yol-b", replaces="yol")]), node("yol")]),
    ("ağaçta olmayan katman", [alt([node("yol-a", replaces="silinmis")])]),
    ("grubun içindeki ana katman", [alt([node("yol-a", replaces="yol")]), node("grup", "group", [node("yol")])]),
    ("senaryonun alt grubunda", [alt([node("ic", "group", [node("yol-a", replaces="yol")])]), node("yol")]),
]


def build(old):
    words = {}
    if old:
        for group in GROUPS:
            for c in old.get(group, []):
                words[(group, c["name"])] = c["problem"]
    out = {
        "format": "kentos.temporal-rules",
        "version": 1,
        "note": "The rules of a layer's time setting, a scenario and the layer tree's scenarios (docs/adr/0210 §2), each case with "
        "the contract's words; null where the value is one. scripts/fixtures/temporal_rules_cases.py writes the values from the ADR "
        "and checks the verdicts with tools/kcad/kcad.py's own copy of the rules; Rust "
        "(crates/shared/contracts/tests/all/temporal_rules.rs, KENTOS_WRITE_TEMPORAL_RULES=1 writes the words) and TypeScript "
        "(apps/web/src/model/temporalRules.test.ts) give the same words.",
    }
    for group, cases, problem in (
        ("times", TIMES, kcad.layer_time_problem),
        ("scenarios", SCENARIOS, kcad.scenario_problem),
        ("trees", TREES, kcad.scenarios_problem),
    ):
        out[group] = []
        for name, value in cases:
            refused = problem(copy.deepcopy(value)) is not None
            out[group].append({"name": name, "value": value, "problem": words.get((group, name), "?" if refused else None)})
    return out


def verdicts(cases):
    wrong = []
    for group, problem in (("times", kcad.layer_time_problem), ("scenarios", kcad.scenario_problem), ("trees", kcad.scenarios_problem)):
        for c in cases[group]:
            refused = problem(copy.deepcopy(c["value"])) is not None
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
        values = lambda d: [(c["name"], c["value"]) for g in GROUPS for c in d[g]]
        wrong = verdicts(old)
        if values(fresh) != values(old):
            wrong.append("değerler betiğin yazdığından farklı: betiği --check olmadan çalıştırın ve farkı okuyun")
        if wrong:
            print("\n".join(wrong))
            return 1
        n = sum(len(old[g]) for g in GROUPS)
        print(f"{OUT.relative_to(ROOT)}: {n} durumun hükmü okuyucunun kurallarıyla aynı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(build(old), ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
