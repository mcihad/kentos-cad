"""Writes fixtures/commands/v1/cad.layers.table.json: the shared cases of `cad.layers.table` v1 (Koşullu biçim,
docs/adr/0413), their expectations built here from the ADR's rules (the rules' rule and its words, the checks' order,
the codes, the paths, the step's name), without KentOS code. The desktop (crates/native/application/tests/all/
fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

Also writes fixtures/table-styles/v1/looks.json: what the rules give a cell (§1: a field's rules before the row's, the
first that holds gives each of background, colour and bold), which the contracts' test reads.

    python3 scripts/fixtures/table_styles_command_cases.py           # writes the files
    python3 scripts/fixtures/table_styles_command_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.table.json"
LOOKS = ROOT / "fixtures/table-styles/v1/looks.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
LABEL = "Koşullu biçim"
FIELDS = [{"name": "Durum", "kind": "text"}, {"name": "Kat", "kind": "integer"}, {"name": "Açıklama", "kind": "text"}]


def node(id_, name, type_="layer", children=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True, "style": dict(STYLE),
            "children": children or [], **extra}


SETUP_LAYERS = [
    node("bina", "Bina", fields=FIELDS),
    node("yol", "Yol"),
    node("koruma", "Koruma alanı", "group", [node("kilitli", "Sit alanı", fields=FIELDS)], locked=True),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
]
X, Y = 500000, 4400000
ENTITIES = [{"kind": "polygon", "id": 1, "layerId": "bina", "attrs": {"Durum": "kotu", "Kat": "6"},
             "pts": [{"x": X, "y": Y}, {"x": X + 10, "y": Y}, {"x": X + 10, "y": Y + 8}]}]
IDS = [1]


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Koşullu biçim",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
            "origin": {"x": X, "y": Y}, "layers": copy.deepcopy(SETUP_LAYERS), "activeLayer": "bina",
            "entities": copy.deepcopy(ENTITIES), "styles": {"items": [], "categories": []}}


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


def with_styles(layers, id_, styles):
    out = copy.deepcopy(layers)
    n = get(out, id_)
    if styles:
        n["tableStyles"] = copy.deepcopy(styles)
    else:
        n.pop("tableStyles", None)
    return out


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


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
SIRA = "Koşullu biçimde $sıra kullanılamaz: biçim tablonun sırasına göre değişmemeli."
OLCEK = "Koşullu biçimde $ölçek kullanılamaz: biçim çizimin ölçeğine göre değişmemeli."

HASAR = {"condition": "Durum = 'kotu'", "background": "#FFE4E1", "color": "#B91C1C"}
YUKSEK = {"field": "Kat", "condition": "Kat > 5", "bold": True}
NOTLU = {"field": "Açıklama", "condition": "Açıklama <> ''", "color": "#1D4ED8"}


def problem(styles):
    """The rules' rule (docs/adr/0413 §1): (place, words) of the first break, or None."""
    if len(styles) > 50:
        return 50, f"Bir katmanın tablosunda en çok 50 biçim kuralı olur; {len(styles)} verildi."
    hexed = lambda c: re.fullmatch(r"#[0-9A-Fa-f]{6}", c) is not None  # noqa: E731
    for i, s in enumerate(styles):
        n = i + 1
        if not s["condition"].strip():
            return i, f"{n}. kuralın koşulu boş."
        if len(s["condition"]) > 2000:
            return i, f"{n}. kuralın koşulu en çok 2000 karakter olabilir."
        if "field" in s and not s["field"].strip():
            return i, f"{n}. kuralın alanı boş; satır için alanı vermeyin."
        for key, word in (("background", "zemin"), ("color", "yazı")):
            if key in s and not hexed(s[key]):
                return i, f"{n}. kuralın {word} rengi “{s[key]}” #RRGGBB biçiminde değil."
        if "background" not in s and "color" not in s and not s.get("bold"):
            return i, f"{n}. kural hiçbir şey vermiyor: zemin rengi, yazı rengi ya da kalınlık verin."
    return None


def written(layer, changed_=True):
    return completed({"layer": layer, "changed": changed_, "revision": "$current"})


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree()})]}


def bad_rules(name, styles, **extra):
    at, words = problem(styles)
    return refused(name, {"layer": "bina", "styles": styles, **extra}, "invalid_table_styles", words, f"styles/{at}")


def cases():
    out = []
    two = [HASAR, YUKSEK]
    out.append({"name": "iki kural sırasıyla yazılır; adımı “Koşullu biçim”, geri alınır ve yinelenir; nesneler değişmez", "steps": [
        step("execute", {"layer": "bina", "styles": two}, written("bina"),
             {"ids": IDS, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": with_styles(tree(), "bina", two)}),
        step("undo", returns=LABEL, expect={"canUndo": False, "canRedo": True, "layers": tree()}),
        step("redo", returns=LABEL, expect={"canUndo": True, "canRedo": False, "layers": with_styles(tree(), "bina", two)}),
    ]})
    out.append({"name": "aynı kurallar yeniden: değişmez, adım yazılmaz", "steps": [
        step("execute", {"layer": "bina", "styles": two}, written("bina"), {"revision": "changed"}),
        step("execute", {"layer": "bina", "styles": two}, written("bina", False), {"revision": "same", "canUndo": True}),
    ]})
    out.append({"name": "boş liste kuralları kaldırır", "steps": [
        step("execute", {"layer": "bina", "styles": two}, written("bina"), {"revision": "changed"}),
        step("execute", {"layer": "bina", "styles": []}, written("bina"), {"revision": "changed", "layers": tree()}),
    ]})
    out.append({"name": "kuralsız katmana boş liste: değişmez", "steps": [
        step("execute", {"layer": "yol"}, written("yol", False), {**UNCHANGED, "layers": tree()}),
    ]})
    out.append({"name": "plan katmanı yazılacak hâliyle gösterir, hiçbir şey yazmaz; doğrulama da", "steps": [
        step("plan", {"layer": "bina", "styles": [NOTLU]},
             completed({"node": get(with_styles(tree(), "bina", [NOTLU]), "bina"), "changed": True, "revision": "$current"}), {**UNCHANGED, "layers": tree()}),
        step("validate", {"layer": "bina", "styles": [NOTLU]}, completed(None), {**UNCHANGED, "layers": tree()}),
    ]})
    out.append({"name": "kilitli grubun katmanı da alır: biçim çizimi değiştirmez", "steps": [
        step("execute", {"layer": "kilitli", "styles": [HASAR]}, written("kilitli"), {"revision": "changed", "layers": with_styles(tree(), "kilitli", [HASAR])}),
    ]})
    out.append({"name": "yalnız kalınlık veren alan kuralı", "steps": [
        step("execute", {"layer": "bina", "styles": [YUKSEK]}, written("bina"), {"revision": "changed", "layers": with_styles(tree(), "bina", [YUKSEK])}),
    ]})
    out.append(bad_rules("boş koşul", [{"condition": "  ", "bold": True}]))
    out.append(bad_rules("zemin rengi #RRGGBB değil", [{"condition": "Kat > 1", "background": "red"}]))
    out.append(bad_rules("yazı rengi beş haneli", [{"condition": "Kat > 1", "color": "#12345"}]))
    out.append(bad_rules("hiçbir şey vermeyen kural", [{"condition": "Kat > 1"}]))
    out.append(bad_rules("boş alan adı", [{"field": " ", "condition": "Kat > 1", "bold": True}]))
    out.append(bad_rules("51 kural", [YUKSEK] * 51))
    out.append(bad_rules("2001 karakterlik koşul", [{"condition": "a" * 2001, "bold": True}]))
    out.append(bad_rules("ikinci kuralın sorunu yeriyle", [HASAR, {"condition": "Kat > 1", "background": "#GGGGGG"}]))
    out.append(bad_rules("bozuk kural sürümden ve katmandan önce", [{"condition": ""}], expectedRevision="on iki"))
    out.append(refused("geçersiz beklenen sürüm", {"layer": "bina", "styles": [HASAR], "expectedRevision": "on iki"}, "invalid_revision",
                       INVALID_REVISION, "expectedRevision"))
    out.append({"name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz", "steps": [
        step("captureRevision", **{"as": "once"}),
        step("execute", {"layer": "yol", "styles": [HASAR]}, written("yol"), {"revision": "changed"}),
        step("execute", {"layer": "bina", "styles": [HASAR], "expectedRevision": "$once"},
             {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
             {"revision": "same", "layers": with_styles(tree(), "yol", [HASAR])}),
    ]})
    out.append(refused("çizimde olmayan katman", {"layer": "yok", "styles": [HASAR]}, "layer_not_found",
                       "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"layer": "koruma", "styles": [HASAR]}, "not_a_layer",
                       "“Koruma alanı” bir katman grubu; koşullu biçim yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("servis katmanı", {"layer": "altlik", "styles": [HASAR]}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; tablosu yoktur.", "layer"))
    out.append(refused("katmanda olmayan alan", {"layer": "bina", "styles": [HASAR, {"field": "Yükseklik", "condition": "Kat > 1", "bold": True}]},
                       "field_not_found", "“Bina” katmanında “Yükseklik” alanı yok.", "styles/1/field"))
    out.append(refused("koşulda $sıra", {"layer": "bina", "styles": [{"condition": "10 > $sıra", "bold": True}]}, "invalid_expression",
                       f"1. kuralın koşulu: {SIRA}", "styles/0/condition"))
    out.append(refused("ikinci kuralın koşulunda $ölçek", {"layer": "bina", "styles": [HASAR, {"condition": "500 < $ölçek", "bold": True}]},
                       "invalid_expression", f"2. kuralın koşulu: {OLCEK}", "styles/1/condition"))
    out.append(refused("aynı kuralda alan koşuldan önce", {"layer": "bina", "styles": [{"field": "Yok", "condition": "1 < $sıra", "bold": True}]},
                       "field_not_found", "“Bina” katmanında “Yok” alanı yok.", "styles/0/field"))
    return out


def look(styles, holds, field):
    """What the rules give `field`'s cell (none: the row's own), `holds[i]` whether rule i holds (§1)."""
    out = {"background": None, "color": None, "bold": False}
    order = [i for i, s in enumerate(styles) if field is not None and s.get("field") == field] + \
            [i for i, s in enumerate(styles) if "field" not in s]
    bold_given = False
    for i in order:
        if not holds[i]:
            continue
        s = styles[i]
        if out["background"] is None and "background" in s:
            out["background"] = s["background"]
        if out["color"] is None and "color" in s:
            out["color"] = s["color"]
        if not bold_given and s.get("bold"):
            out["bold"] = True
            bold_given = True
    return out


def looks():
    rules = [HASAR, YUKSEK, {"field": "Kat", "condition": "Kat > 9", "background": "#00FF00", "color": "#0000FF"}, NOTLU,
             {"condition": "Kat > 0", "background": "#EEEEEE", "bold": True}]
    out = []
    for holds in ([True] * 5, [False, True, True, False, True], [True, False, False, False, False], [False] * 5,
                  [False, False, True, True, False]):
        for field in (None, "Kat", "Açıklama", "Durum"):
            out.append({"holds": holds, "field": field, "want": look(rules, holds, field)})
    return {"format": "kentos.table-style-looks", "version": 1,
            "note": "Written by scripts/fixtures/table_styles_command_cases.py from docs/adr/0413 §1; do not edit by hand.",
            "styles": rules, "cases": out}


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.layers.table", "commandVersion": 1,
            "title": "Koşullu biçim",
            "note": "Written by scripts/fixtures/table_styles_command_cases.py from docs/adr/0413; do not edit by hand.",
            "setup": setup(), "cases": cases()}


def main() -> int:
    outputs = {OUT: build(), LOOKS: looks()}
    bad = 0
    for path, data in outputs.items():
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad = 1
            else:
                print(f"{path.relative_to(ROOT)} güncel")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"{path.relative_to(ROOT)} yazıldı")
    return bad


if __name__ == "__main__":
    sys.exit(main())
