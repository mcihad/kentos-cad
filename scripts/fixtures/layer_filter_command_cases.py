"""Writes fixtures/commands/v1/cad.layers.filter.json: the shared cases of `cad.layers.filter` v1 (docs/adr/0211 §5),
their expectations built here from the contract's rules (crates/shared/contracts/src/cad_layers.rs, layer_filter.rs: the
checks and their order, the codes, the paths and the words; the step's name) and from the ADR's meaning of a filter
(§3: a condition passes when it is true; a list passes the ids it names; both, both), without KentOS code: each case's
condition is written below as Python over the objects' attributes and their exact areas. The web
(apps/web/src/product/fixtures.test.ts), the desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_filter_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_filter_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import operator
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.filter.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
BLUE = {"color": "#0090FF", "lineType": "continuous", "lineWeight": 0.35}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
LABEL = "Katman süzgeci"
EXPRESSION_MAX = 10_000


def node(id_, name, type_="layer", children=None, style=STYLE, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True, "style": dict(style), "children": children or [], **extra}


SETUP_LAYERS = [
    node("parsel", "Parsel", style=BLUE),
    node("bina", "Bina"),
    node("koruma", "Koruma alanı", "group", [node("kilitli", "Sit alanı")], locked=True),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
    node("0", "0"),
]


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


X, Y = 500000, 4400000
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Nitelik": "Arsa"}, "pts": rect(X, Y, 40, 30)},
    {"kind": "polygon", "id": 2, "layerId": "parsel", "attrs": {"Nitelik": "Arsa"}, "pts": rect(X + 40, Y, 20, 20)},
    {"kind": "polygon", "id": 3, "layerId": "parsel", "attrs": {"Nitelik": "Tarla"}, "pts": rect(X + 60, Y, 30, 30)},
    {"kind": "polygon", "id": 4, "layerId": "parsel", "attrs": {}, "pts": rect(X + 90, Y, 25, 20)},
    {"kind": "polygon", "id": 5, "layerId": "bina", "attrs": {"Kat": "4"}, "pts": rect(X + 5, Y + 5, 10, 10)},
    {"kind": "polygon", "id": 6, "layerId": "bina", "attrs": {"Kat": "2"}, "pts": rect(X + 65, Y + 5, 10, 10)},
    {"kind": "point", "id": 7, "layerId": "kilitli", "attrs": {}, "p": {"x": X + 10, "y": Y + 50}},
    {"kind": "line", "id": 8, "layerId": "0", "attrs": {}, "a": {"x": X - 10, "y": Y + 40}, "b": {"x": X + 120, "y": Y + 40}},
]
IDS = [e["id"] for e in ENTITIES]


def area(e):
    """The exact area of a polygon by the shoelace formula, in fractions; none for an object without one (`$alan` is
    empty there, and a comparison with empty is false)."""
    if e["kind"] != "polygon":
        return None
    pts = [(Fraction(p["x"]), Fraction(p["y"])) for p in e["pts"]]
    s = sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(pts, pts[1:] + pts[:1]))
    return abs(s) / 2


def as_number(text):
    """A text as the language compares it with a number (JavaScript's relational rule): its number, else none."""
    try:
        return Fraction(text)
    except (TypeError, ValueError):
        return None


# The cases' conditions as the ADR means them (§3), written over an object's attributes and exact area. None starts
# with “$”: an input's text that does is a captured revision's name (fixtures/commands/README.md).
def cmp(a, op, b):
    """A comparison as the language makes it: with an empty side it is false."""
    return a is not None and op(a, b)


MEANING = {
    "Nitelik = 'Arsa'": lambda e: e["attrs"].get("Nitelik") == "Arsa",
    "Nitelik = 'Arsa' ve $alan > 500": lambda e: e["attrs"].get("Nitelik") == "Arsa" and cmp(area(e), operator.gt, 500),
    "500 <= $alan": lambda e: cmp(area(e), operator.ge, 500),
    "Nitelik = 'Tarla' veya $alan < 450": lambda e: e["attrs"].get("Nitelik") == "Tarla" or cmp(area(e), operator.lt, 450),
    "Kat >= 3": lambda e: cmp(as_number(e["attrs"].get("Kat")), operator.ge, 3),
}


def passed(layer, expression=None, objects=None):
    """How many of the layer's objects pass, and how many it has."""
    on = [e for e in ENTITIES if e["layerId"] == layer]
    ok = [e for e in on if (expression is None or MEANING[expression](e)) and (objects is None or e["id"] in objects)]
    return len(ok), len(on)


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Katman süzgeçleri",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(SETUP_LAYERS),
        "activeLayer": "parsel",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


def tree():
    return copy.deepcopy(SETUP_LAYERS)


def get(nodes, id_):
    for n in nodes:
        if n["id"] == id_:
            return n
        hit = get(n["children"], id_)
        if hit:
            return hit
    return None


def changed(layers, id_, drop=(), **fields):
    out = copy.deepcopy(layers)
    n = get(out, id_)
    n.update(copy.deepcopy(fields))
    for k in drop:
        n.pop(k, None)
    return out


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_=None, result=None, expect=None, **extra):
    s = {"op": op}
    if input_ is not None:
        s["input"] = input_
    if result is not None:
        s["result"] = result
    s.update(extra)
    if expect is not None:
        s["expect"] = expect
    return s


UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def filtered(layer, changed_=True, expression=None, objects=None):
    p, t = passed(layer, expression, objects)
    return completed({"layer": layer, "changed": changed_, "passed": p, "total": t, "revision": "$current"})


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree()})]}


def problem_words(f):
    """`LayerFilter::problem`'s words for the refusals below, in the contract's order."""
    if "expression" not in f and not f.get("objects"):
        return "süzgeçte ne ifade ne nesne listesi var"
    e = f.get("expression")
    if e is not None:
        if e.strip() != e or not e:
            return "süzgecin ifadesi boş ya da başında veya sonunda boşluk var"
        if len(e) > EXPRESSION_MAX:
            return f"süzgecin ifadesi {EXPRESSION_MAX} karakterden uzun"
    seen = set()
    for x in f.get("objects", []):
        if x in seen:
            return f"süzgecin listesinde {x} iki kez var"
        seen.add(x)
    return None


def cases():
    out = []
    arsa = {"expression": "Nitelik = 'Arsa'"}
    with_arsa = changed(tree(), "parsel", filter=arsa)
    out.append({
        "name": "İfade: yalnız arsalar geçer; adımı “Katman süzgeci”, geri alınır ve yinelenir; nesneler değişmez",
        "steps": [
            step("execute", {"layer": "parsel", "filter": arsa}, filtered("parsel", expression=arsa["expression"]),
                 {"ids": IDS, "entities": {"1": ENTITIES[0], "3": ENTITIES[2]}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": with_arsa}),
            step("undo", returns=LABEL, expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=LABEL, expect={"layers": with_arsa, "canUndo": True, "canRedo": False}),
        ],
    })
    for expression, layer, why in (
        ("Nitelik = 'Arsa' ve $alan > 500", "parsel", "öznitelik ve alan birlikte"),
        ("500 <= $alan", "parsel", "alan sınırda: 500 m² geçer"),
        ("Nitelik = 'Tarla' veya $alan < 450", "parsel", "veya; Niteliği olmayan nesne alanıyla sınanır"),
        ("Kat >= 3", "bina", "sayı gibi okunan metin sayıyla karşılaştırılır"),
    ):
        f = {"expression": expression}
        out.append({
            "name": f"İfade: {why}",
            "steps": [step("execute", {"layer": layer, "filter": f}, filtered(layer, expression=expression), {"layers": changed(tree(), layer, filter=f), "canUndo": True})],
        })
    listed = {"objects": ["$uidOf:1", "$uidOf:3"]}
    out.append({
        "name": "Liste: yalnız adı geçen nesneler (Seçimden süzgeç)",
        "steps": [step("execute", {"layer": "parsel", "filter": listed}, filtered("parsel", objects={1, 3}), {"layers": changed(tree(), "parsel", filter=listed), "canUndo": True})],
    })
    both = {"expression": "Nitelik = 'Arsa'", "objects": ["$uidOf:1", "$uidOf:3"]}
    out.append({
        "name": "İfade ve liste: ikisinden de geçmeli",
        "steps": [step("execute", {"layer": "parsel", "filter": both}, filtered("parsel", expression=both["expression"], objects={1, 3}), {"layers": changed(tree(), "parsel", filter=both)})],
    })
    out.append({
        "name": "Çizimde olmayan kimlik hata değildir: hiçbir nesneye uymaz",
        "steps": [step("execute", {"layer": "parsel", "filter": {"objects": ["0192f5a1-7777-7000-8000-00000000ffff"]}}, filtered("parsel", objects=set()),
                       {"layers": changed(tree(), "parsel", filter={"objects": ["0192f5a1-7777-7000-8000-00000000ffff"]})})],
    })
    out.append({
        "name": "Değiştir: aynı katmana başka bir süzgeç, tek adım",
        "steps": [
            step("execute", {"layer": "parsel", "filter": arsa}, filtered("parsel", expression=arsa["expression"]), {"layers": with_arsa}),
            step("execute", {"layer": "parsel", "filter": {"expression": "500 <= $alan"}}, filtered("parsel", expression="500 <= $alan"),
                 {"layers": changed(tree(), "parsel", filter={"expression": "500 <= $alan"})}),
            step("undo", returns=LABEL, expect={"layers": with_arsa}),
        ],
    })
    out.append({
        "name": "Kaldır: filter null; geri almada süzgeç döner; bütün nesneler sayılır",
        "steps": [
            step("execute", {"layer": "parsel", "filter": arsa}, filtered("parsel", expression=arsa["expression"]), {"layers": with_arsa}),
            step("execute", {"layer": "parsel", "filter": None}, filtered("parsel"), {"layers": tree(), "revision": "changed"}),
            step("undo", returns=LABEL, expect={"layers": with_arsa}),
        ],
    })
    out.append({
        "name": "Kaldır: filter verilmezse de; süzgeci olmayan katmanda changed false",
        "steps": [step("execute", {"layer": "parsel"}, filtered("parsel", False), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Aynı süzgeç: bir şey yazılmaz, changed false",
        "steps": [
            step("execute", {"layer": "parsel", "filter": arsa}, filtered("parsel", expression=arsa["expression"]), {"layers": with_arsa}),
            step("execute", {"layer": "parsel", "filter": arsa}, filtered("parsel", False, expression=arsa["expression"]), {"layers": with_arsa, "revision": "same", "canUndo": True}),
        ],
    })
    out.append({
        "name": "Kilitli katman: süzgeç katmanın ayarıdır, nesnelere dokunmaz; yazılır",
        "steps": [step("execute", {"layer": "kilitli", "filter": {"expression": "500 <= $alan"}}, filtered("kilitli", expression="500 <= $alan"),
                       {"ids": IDS, "layers": changed(tree(), "kilitli", filter={"expression": "500 <= $alan"})})],
    })
    out.append({
        "name": "Servis katmanından süzgeci kaldırmak reddedilmez: süzgeci yok",
        "steps": [step("execute", {"layer": "altlik"}, filtered("altlik", False), UNCHANGED)],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"layer": "parsel", "filter": arsa}, {"status": "completed", "output": None, "warnings": []}, {**UNCHANGED, "layers": tree()})],
    })
    p, t = passed("parsel", arsa["expression"])
    out.append({
        "name": "Plan: katmanın yazılacak hâli ve geçenler; yazmaz",
        "steps": [step("plan", {"layer": "parsel", "filter": arsa}, completed({"node": get(with_arsa, "parsel"), "changed": True, "passed": p, "total": t, "revision": "$current"}),
                       {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: kaldırmada süzgeçsiz katman, bütün nesneler",
        "steps": [step("plan", {"layer": "parsel", "filter": None}, completed({"node": get(tree(), "parsel"), "changed": False, "passed": 4, "total": 4, "revision": "$current"}), UNCHANGED)],
    })
    # The refusals, in the contract's order: the filter's rules, its condition, then the revision and the layer.
    for name, f in (
        ("ne ifade ne liste", {}),
        ("ifadenin başında boşluk", {"expression": " Nitelik = 'Arsa'"}),
        ("boş ifade", {"expression": ""}),
        ("10 001 karakterlik ifade", {"expression": "a" * 10_001}),
        ("aynı kimlik iki kez", {"objects": ["0192f5a1-7777-7000-8000-000000000001", "0192f5a1-7777-7000-8000-000000000001"]}),
    ):
        out.append(refused(f"bozuk süzgeç: {name}", {"layer": "parsel", "filter": f}, "invalid_filter", f"Katmanın süzgeci: {problem_words(f)}.", "filter"))
    out.append(refused("$sıra süzgeçte kullanılamaz", {"layer": "parsel", "filter": {"expression": "2 >= $sıra"}}, "invalid_expression",
                       "Süzgecin ifadesi: Süzgeçte $sıra kullanılamaz: süzgeç çalıştırmaya göre değişmemeli.", "filter/expression"))
    out.append(refused("$ölçek süzgeçte kullanılamaz", {"layer": "parsel", "filter": {"expression": "500 < $ölçek"}}, "invalid_expression",
                       "Süzgecin ifadesi: Süzgeçte $ölçek kullanılamaz: süzgeç çizimin ölçeğine göre değişmemeli.", "filter/expression"))
    out.append(refused("bozuk süzgeç sürümden ve katmandan önce", {"layer": "yok", "filter": {}, "expectedRevision": "on iki"}, "invalid_filter",
                       f"Katmanın süzgeci: {problem_words({})}.", "filter"))
    out.append(refused("geçersiz beklenen sürüm", {"layer": "parsel", "filter": arsa, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"layer": "bina", "filter": {"expression": "Kat >= 3"}}, filtered("bina", expression="Kat >= 3"), {"revision": "changed"}),
            step("execute", {"layer": "parsel", "filter": arsa, "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "layers": changed(tree(), "bina", filter={"expression": "Kat >= 3"})}),
        ],
    })
    out.append(refused("çizimde olmayan katman", {"layer": "yok", "filter": arsa}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"layer": "koruma", "filter": arsa}, "not_a_layer", "“Koruma alanı” bir katman grubu; süzgeç yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("servis katmanı", {"layer": "altlik", "filter": arsa}, "service_layer", "“OpenStreetMap” servisten çizilir ve nesne tutmaz; süzgeç nesneleri olan katmanın olur.", "layer"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.filter",
        "commandVersion": 1,
        "title": "Katman süzgeci",
        "note": "Written by scripts/fixtures/layer_filter_command_cases.py from the contract's rules and docs/adr/0211 §3, §5; do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.layers.filter durumları güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
