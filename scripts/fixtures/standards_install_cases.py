#!/usr/bin/env python3
"""The shared cases of cad.standards.install (Standart kaynaklarını kur, docs/adr/1002), written from the ADR without
KentOS code.

    python3 scripts/fixtures/standards_install_cases.py           # writes fixtures/commands/v1/cad.standards.install.json
    python3 scripts/fixtures/standards_install_cases.py --check   # writes nothing; compares

Two office standards are bound to the drawing, the first before the second; a resource of a name both carry is the
first's (cad.standards.resolve's rule). Each resource is missing, same or differs against the drawing's of that name
(case aside; its name, id, description and source aside; numbers equal within 1e-9). A missing one is added by
default, a differing one kept by default; `replace` overwrites in place keeping the drawing's name and id,
`renameIncoming` adds the standard's under a new name, a style taking its own id or, when the drawing has that id,
`id-2`, `id-3` …. Nothing is removed. Rows are by kind (line types, hatch patterns, text, dimension, leader styles),
then by the standards' order. One undo step “Standart kaynaklarını kur”.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.standards.install.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
STEP = "Standart kaynaklarını kur"

LOCAL_LT = [
    {"name": "GAZ", "elements": [{"length": 1.0}, {"length": -1.0}], "unit": "paper"},
    {"name": "SINIR", "elements": [{"length": 3.0}, {"length": -1.0}], "unit": "paper"},
]
LOCAL_HP = [{"name": "TUGLA", "unit": "mm", "lines": [{"angle": 0.0, "origin": [0.0, 0.0], "offset": [0.0, 6.35]}]}]
LOCAL_TEXT = [{"id": "yerel-not", "name": "A-NOT", "font": "barlow", "height": 3.5}]
LOCAL_DIM = [{"id": "yerel-olcu", "name": "A-OLCU", "height": 2.5}]
LOCAL_LEADER = [{"id": "ofis-kilavuz", "name": "YEREL-KILAVUZ", "height": 2.5}]

A_RES = {
    "lineTypes": [
        {"name": "GAZ", "elements": [{"length": 2.0}, {"length": -1.0}], "unit": "paper"},
        {"name": "OFIS-KESIK", "elements": [{"length": 4.0}, {"length": -2.0}], "unit": "paper"},
        {"name": "sinir", "description": "Ofisin açıklaması", "elements": [{"length": 3.0000000001}, {"length": -1.0}], "unit": "paper"},
    ],
    "hatchPatterns": [
        {"name": "OFIS-TARAMA", "unit": "mm", "lines": [{"angle": 45.0, "origin": [0.0, 0.0], "offset": [0.0, 2.0]}]},
    ],
    "textStyleDefinitions": [{"id": "ofis-not", "name": "A-NOT", "font": "barlow", "height": 2.5}],
    "dimensionStyleDefinitions": [{"id": "ofis-olcu", "name": "A-OLCU", "height": 2.5}],
    "leaderStyles": [{"id": "ofis-kilavuz", "name": "OFIS-KILAVUZ", "height": 2.5}],
}
B_RES = {
    "lineTypes": [
        {"name": "GAZ", "elements": [{"length": 9.0}, {"length": -9.0}], "unit": "paper"},
        {"name": "B-NOKTA", "elements": [{"length": 0.0}, {"length": -1.0}], "unit": "paper"},
    ],
}


def package(id_, rev, res):
    return {"format": "kentos.drawing-standard", "version": 2, "id": id_, "revision": rev,
            "standard": {"name": id_, "layers": []}, "resources": res}


A = {"id": "ofis-a", "revision": "2026.1"}
B = {"id": "ofis-b", "revision": "2026.3"}


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Ofis kaynakları",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 100, "workspace": "cad",
                         "lineTypes": LOCAL_LT, "hatchPatterns": LOCAL_HP, "textStyles": LOCAL_TEXT,
                         "dimensionStyles": LOCAL_DIM, "leaderStyles": LOCAL_LEADER,
                         "standardDocs": [{"text": json.dumps(package("ofis-a", "2026.1", A_RES), ensure_ascii=False)},
                                          {"text": json.dumps(package("ofis-b", "2026.3", B_RES), ensure_ascii=False)}]},
            "origin": {"x": 0, "y": 0},
            "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                        "style": STYLE, "children": []}],
            "activeLayer": "0",
            "entities": [{"kind": "line", "id": 1, "layerId": "0", "attrs": {}, "linePattern": "GAZ", "lineType": "dashed",
                          "a": {"x": 0, "y": 0}, "b": {"x": 10, "y": 0}}],
            "styles": {"items": [], "categories": []}}


def row(kind, name, status, action, source, fields=(), new_name=None):
    r = {"kind": kind, "name": name, "status": status, "action": action, "source": source}
    if new_name is not None:
        r["newName"] = new_name
    if fields:
        r["fields"] = list(fields)
    return r


def default_rows(**over):
    """The rows when no choice is given; `over` replaces a row's action (and new name) by its name."""
    rows = [
        row("lineType", "GAZ", "differs", "keepLocal", A, ["elements"]),
        row("lineType", "OFIS-KESIK", "missing", "add", A),
        row("lineType", "sinir", "same", "keepLocal", A),
        row("lineType", "B-NOKTA", "missing", "add", B),
        row("hatchPattern", "OFIS-TARAMA", "missing", "add", A),
        row("textStyle", "A-NOT", "differs", "keepLocal", A, ["height"]),
        row("dimensionStyle", "A-OLCU", "same", "keepLocal", A),
        row("leaderStyle", "OFIS-KILAVUZ", "missing", "add", A),
    ]
    for r in rows:
        if r["name"] in over:
            action = over[r["name"]]
            if isinstance(action, tuple):
                r["action"], r["newName"] = action
            else:
                r["action"] = action
    return rows


def answer(rows):
    n = lambda a: sum(1 for r in rows if r["action"] == a and r["status"] != "same")
    return {"rows": rows, "added": n("add"), "replaced": n("replace"), "renamed": n("renameIncoming"),
            "kept": sum(1 for r in rows if r["action"] == "keepLocal"), "revision": "$current"}


def done(output):
    return {"status": "completed", "output": output, "warnings": []}


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def installed(rows):
    """The drawing's tables after `rows`."""
    lt = copy.deepcopy(LOCAL_LT)
    hp = copy.deepcopy(LOCAL_HP)
    text, dim, leader = copy.deepcopy(LOCAL_TEXT), copy.deepcopy(LOCAL_DIM), copy.deepcopy(LOCAL_LEADER)
    incoming = {
        "lineType": A_RES["lineTypes"] + [t for t in B_RES["lineTypes"] if t["name"].lower() not in {x["name"].lower() for x in A_RES["lineTypes"]}],
        "hatchPattern": A_RES["hatchPatterns"],
        "textStyle": A_RES["textStyleDefinitions"],
        "dimensionStyle": A_RES["dimensionStyleDefinitions"],
        "leaderStyle": A_RES["leaderStyles"],
    }
    tables = {"lineType": lt, "hatchPattern": hp, "textStyle": text, "dimensionStyle": dim, "leaderStyle": leader}
    for r in rows:
        if r["status"] == "same":
            continue
        new = copy.deepcopy(next(t for t in incoming[r["kind"]] if t["name"].lower() == r["name"].lower()))
        table = tables[r["kind"]]
        styled = r["kind"] in ("textStyle", "dimensionStyle", "leaderStyle")
        if r["action"] in ("add", "renameIncoming"):
            if r["action"] == "renameIncoming":
                new["name"] = r["newName"]
            if styled:
                ids = {t["id"] for t in table}
                base, k, fresh = new["id"], 2, new["id"]
                while fresh in ids:
                    fresh, k = f"{base}-{k}", k + 1
                new["id"] = fresh
            table.append(new)
        elif r["action"] == "replace":
            i = next(i for i, t in enumerate(table) if t["name"].lower() == r["name"].lower())
            new["name"] = table[i]["name"]
            if styled:
                new["id"] = table[i]["id"]
            table[i] = new
    return {"lineTypes": lt, "hatchPatterns": hp, "textStyles": text, "dimensionStyles": dim, "leaderStyles": leader}


def run(choices, rows, op="execute", **kw):
    step = {"op": op, "input": ({"choices": choices} if choices else {}) | kw, "result": done(answer(rows) if op != "validate" else None)}
    if op == "validate":
        step["result"] = done(None)
    if op == "execute" and any(r["status"] != "same" and r["action"] != "keepLocal" for r in rows):
        step["expect"] = {"revision": "changed", "canUndo": True, "settings": installed(rows)}
    else:
        step["expect"] = {"revision": "same", "canUndo": False, "settings": installed(default_rows(**{r["name"]: "keepLocal" for r in default_rows()}))}
    return step


def refused(input_, code, message, path):
    return {"op": "execute", "input": input_, "result": refusal(code, message, path),
            "expect": {"revision": "same", "canUndo": False}}


def choice(kind, name, action, new_name=None):
    c = {"kind": kind, "name": name, "action": action}
    if new_name is not None:
        c["newName"] = new_name
    return c


def cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    case("Seçimsiz: eksikler eklenir, farklılar korunur, aynılar dokunulmaz", [run([], default_rows())])
    case("Plan yazmaz, aynı satırları verir", [run([], default_rows(), op="plan")])
    case("Doğrulama yazmaz", [run([], default_rows(), op="validate")])
    case("Farklı GAZ yerinde değiştirilir: ad ve kullanan çizgi izler", [
        run([choice("lineType", "GAZ", "replace")], default_rows(GAZ="replace"))])
    case("Farklı GAZ yeni adla eklenir, çizimdeki kalır", [
        run([choice("lineType", "gaz", "renameIncoming", "GAZ-OFIS")], default_rows(GAZ=("renameIncoming", "GAZ-OFIS")))])
    case("Farklı A-NOT yerinde: yerel kimlik ve ad korunur", [
        run([choice("textStyle", "A-NOT", "replace")], default_rows(**{"A-NOT": "replace"}))])
    case("Farklı A-NOT yeni adla: kendi kimliğiyle eklenir", [
        run([choice("textStyle", "A-NOT", "renameIncoming", "A-NOT-OFIS")], default_rows(**{"A-NOT": ("renameIncoming", "A-NOT-OFIS")}))])
    case("Kılavuz stili eklenir; çizimde aynı kimlik var: kimlik -2 alır", [run([], default_rows())])
    case("Eksik olan atlanabilir", [
        run([choice("lineType", "OFIS-KESIK", "keepLocal"), choice("hatchPattern", "OFIS-TARAMA", "keepLocal")],
            default_rows(**{"OFIS-KESIK": "keepLocal", "OFIS-TARAMA": "keepLocal"}))])
    case("Hepsi atlanınca yazım yok", [
        run([choice("lineType", n, "keepLocal") for n in ("OFIS-KESIK", "B-NOKTA")]
            + [choice("hatchPattern", "OFIS-TARAMA", "keepLocal"), choice("leaderStyle", "OFIS-KILAVUZ", "keepLocal")],
            default_rows(**{n: "keepLocal" for n in ("OFIS-KESIK", "B-NOKTA", "OFIS-TARAMA", "OFIS-KILAVUZ")}))])
    case("Kurulumu geri al: tablolar önceki hâline döner", [
        run([choice("lineType", "GAZ", "replace")], default_rows(GAZ="replace")),
        {"op": "undo", "returns": STEP, "expect": {"settings": installed(default_rows(**{r["name"]: "keepLocal" for r in default_rows()}))}},
        {"op": "redo", "returns": STEP, "expect": {"settings": installed(default_rows(GAZ="replace"))}}])
    case("İlk standart önce gelir: GAZ ofis-a'nın", [run([], default_rows())])
    case("Standartta olmayan ad: ret", [refused({"choices": [choice("lineType", "YOK", "add")]}, "standard_resource_not_found",
        "Bağlı standartlarda “YOK” adlı çizgi tipi yok. Kaynağı standartların listesinden seçin.", "choices[0].name")])
    case("Türü yanlış: ret", [refused({"choices": [choice("hatchPattern", "GAZ", "replace")]}, "standard_resource_not_found",
        "Bağlı standartlarda “GAZ” adlı tarama deseni yok. Kaynağı standartların listesinden seçin.", "choices[0].name")])
    case("Aynı kaynağa iki seçim: ret", [refused({"choices": [choice("lineType", "GAZ", "replace"), choice("lineType", "gaz", "keepLocal")]},
        "invalid_install_choice", "“gaz” çizgi tipi için iki seçim verildi; her kaynağa bir seçim verin.", "choices[1]")])
    case("Eksik olanı değiştirmek: ret", [refused({"choices": [choice("lineType", "OFIS-KESIK", "replace")]}, "invalid_install_choice",
        "“OFIS-KESIK” çizgi tipi çizimde yok: eklenebilir ya da atlanabilir.", "choices[0].action")])
    case("Aynı olanı eklemek: ret", [refused({"choices": [choice("dimensionStyle", "A-OLCU", "add")]}, "invalid_install_choice",
        "“A-OLCU” ölçü stili çizimdekiyle aynı: değişecek bir şey yok.", "choices[0].action")])
    case("Farklı olanı eklemek: ret", [refused({"choices": [choice("lineType", "GAZ", "add")]}, "invalid_install_choice",
        "“GAZ” çizgi tipi çizimdekinden farklı: değiştirilebilir, yeni adla eklenebilir ya da çizimdeki korunabilir.", "choices[0].action")])
    case("Yeni adsız yeni adla ekleme: ret", [refused({"choices": [choice("lineType", "GAZ", "renameIncoming")]}, "invalid_install_choice",
        "Yeni adla eklemek için kenarı boşluksuz, boş olmayan bir ad verin (newName).", "choices[0].newName")])
    case("Kenarı boşluklu yeni ad: ret", [refused({"choices": [choice("lineType", "GAZ", "renameIncoming", " GAZ2")]}, "invalid_install_choice",
        "Yeni adla eklemek için kenarı boşluksuz, boş olmayan bir ad verin (newName).", "choices[0].newName")])
    case("Değiştirmede yeni ad: ret", [refused({"choices": [choice("lineType", "GAZ", "replace", "X")]}, "invalid_install_choice",
        "Yeni ad yalnız yeni adla eklemede (renameIncoming) verilir.", "choices[0].newName")])
    case("Yeni ad çizimde var: ret", [refused({"choices": [choice("lineType", "GAZ", "renameIncoming", "sinir")]}, "duplicate_name",
        "“sinir” adlı çizgi tipi çizimde ya da standartta var; başka bir ad verin.", "choices[0].newName")])
    case("Yeni ad standartta var: ret", [refused({"choices": [choice("lineType", "GAZ", "renameIncoming", "B-NOKTA")]}, "duplicate_name",
        "“B-NOKTA” adlı çizgi tipi çizimde ya da standartta var; başka bir ad verin.", "choices[0].newName")])
    case("Beklenen revizyon eski: çatışma", [{"op": "execute", "input": {"expectedRevision": "999"},
        "result": {"status": "conflict", "error": {"code": "revision_conflict",
            "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
            "path": "expectedRevision", "revision": "$current"}}, "expect": {"revision": "same"}}])
    case("Beklenen revizyon sayı değil: ret", [refused({"expectedRevision": "abc"}, "invalid_revision",
        "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
        "expectedRevision")])
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.standards.install", "commandVersion": 1,
            "title": "Standart kaynaklarını kur",
            "note": "ADR 1002. Bağımsız başvuru scripts/fixtures/standards_install_cases.py: kurallar ADR'den elle yazılır; KentOS kodu yok.",
            "setup": setup(), "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
