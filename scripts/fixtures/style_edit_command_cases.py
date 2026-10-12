"""Writes fixtures/commands/v1/cad.styles.edit.json: the shared cases of `cad.styles.edit` v1 (docs/adr/0248 §4).

The expectations are built here from the contract's rules (crates/shared/contracts/src/cad_styles.rs: the checks and
their order, the codes, paths and words) and from docs/adr/0183's meaning of a style table and of the objects that follow
it, through the independent reference of those rules (scripts/fixtures/annotation_style_cases.py: the tables' rule, a
style's face and look, an object following a changed style), without KentOS code:

- the table is the project's setting: changing it makes the drawing dirty and is no undo step;
- the texts (dimensions) of a changed style take its new values where they still held the old ones, in one undo step
  “Yazı stili” (“Ölçü stili”); those of a removed style keep their look and lose the link; those on a locked layer are
  left as they are and counted; an object that would not change is not written.

    python3 scripts/fixtures/style_edit_command_cases.py           # writes the file
    python3 scripts/fixtures/style_edit_command_cases.py --check   # compares it with the one on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.styles.edit.json"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


A = _load("annotation_style_cases")

SCALE = 1000
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
X, Y = 500000, 4400000
ADA, YOL, NOT, OLCU, CENTIK = A.ADA, A.YOL, A.NOT, A.OLCU, A.CENTIK
TEXT_LABEL, DIM_LABEL = "Yazı stili", "Ölçü stili"

INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def node(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": dict(STYLE), "children": []}


def text(id_, layer, words, style=None, x=0, **hand):
    """A text as a tool writes it with `style` (its face and its height on paper), then changed by hand."""
    base = {"kind": "text", "id": id_, "layerId": layer, "attrs": {}, "p": {"x": X + x, "y": Y}, "text": words, "height": 2.0, "rotation": 0}
    out = A.apply_text(style, base, SCALE) if style else base
    out.update(hand)
    return out


def dimension(id_, layer, style):
    base = {"kind": "dimension", "id": id_, "layerId": layer, "attrs": {}, "a": {"x": X, "y": Y + 50}, "b": {"x": X + 24.5, "y": Y + 50}, "offset": -2, "height": 2.0}
    return A.apply_dimension(style, base, SCALE)


ENTITIES = [
    text(1, "yazi", "101", ADA),
    text(2, "yazi", "102", ADA, x=20, font="barlow"),
    text(3, "kilitli", "103", ADA, x=40),
    text(4, "yazi", "Atatürk Caddesi", YOL, x=60),
    dimension(5, "olcu", OLCU),
    text(6, "yazi", "Not", None, x=80),
]
IDS = [e["id"] for e in ENTITIES]
E = {str(e["id"]): e for e in ENTITIES}


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Stiller",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": SCALE,
                     "workspace": "cad", "drawingFont": "barlow", "textStyles": [ADA, YOL], "dimensionStyles": [OLCU]},
        "origin": {"x": X, "y": Y},
        "layers": [node("yazi", "Yazı"), node("kilitli", "Kilitli", locked=True), node("olcu", "Ölçü")],
        "activeLayer": "yazi",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


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


def done(style, changed=True, followed=0, locked=0):
    return {"status": "completed", "output": {"style": style, "changed": changed, "followed": followed, "locked": locked, "revision": "$current"}, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
TABLES = {"textStyles": [ADA, YOL], "dimensionStyles": [OLCU]}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "settings": TABLES})]}


def follow_texts(old, new):
    """The texts of `old` after the style became `new` (None: removed): those that change, and how many a locked
    layer keeps."""
    out, locked = {}, 0
    for e in ENTITIES:
        if e["kind"] != "text" or e.get("textStyle") != old["id"]:
            continue
        if new is None:
            after = {k: v for k, v in e.items() if k != "textStyle"}
        else:
            after = A.follow_text(old, new, e, SCALE)
        if after == e:
            continue
        if e["layerId"] == "kilitli":
            locked += 1
            continue
        out[str(e["id"])] = after
    return out, locked


def follow_dimensions(old, new):
    out = {}
    for e in ENTITIES:
        if e["kind"] != "dimension" or e.get("dimStyle") != old["id"]:
            continue
        after = {k: v for k, v in e.items() if k != "dimStyle"} if new is None else A.follow_dimension(old, new, e, SCALE)
        if after != e:
            out[str(e["id"])] = after
    return out


def text_problem(styles):
    return f"Yazı stilleri: {A.text_styles_problem(styles)}."


def cases():
    out = []
    flags = {"ids": IDS, "dirty": True, "revision": "changed", "canUndo": False}
    out.append({"name": "Yazı stili oluştur: tablonun sonuna; tablo proje ayarıdır, geri alma adımı değildir",
                "steps": [step("execute", {"kind": "text", "operation": "create", "text": NOT}, done(NOT["id"]),
                               {**flags, "settings": {"textStyles": [ADA, YOL, NOT]}})]})
    out.append({"name": "Ölçü stili oluştur",
                "steps": [step("execute", {"kind": "dimension", "operation": "create", "dimension": CENTIK}, done(CENTIK["id"]),
                               {**flags, "settings": {"dimensionStyles": [OLCU, CENTIK]}})]})
    new_ada = {**ADA, "font": "overpass", "height": 5.0}
    texts, locked = follow_texts(ADA, new_ada)
    out.append({"name": "Yazı stilini güncelle: eski değeri taşıyan yazılar yenisini alır, elle değiştirilen kalır, kilitli katmandaki sayılır; tek adım “Yazı stili”",
                "steps": [step("execute", {"kind": "text", "operation": "update", "text": new_ada}, done(ADA["id"], True, len(texts), locked),
                               {"ids": IDS, "entities": texts, "settings": {"textStyles": [new_ada, YOL]}, "canUndo": True, "dirty": True}),
                          step("undo", returns=TEXT_LABEL, expect={"entities": {k: E[k] for k in texts}, "settings": {"textStyles": [new_ada, YOL]}})]})
    out.append({"name": "Yazı stilini güncelle: aynı tanım bir şey yazmaz",
                "steps": [step("execute", {"kind": "text", "operation": "update", "text": ADA}, done(ADA["id"], False), {**UNCHANGED, "settings": TABLES})]})
    new_yol = {k: v for k, v in YOL.items() if k != "italic"}
    texts, locked = follow_texts(YOL, new_yol)
    out.append({"name": "Yazı stilini güncelle: eğik kalkar",
                "steps": [step("execute", {"kind": "text", "operation": "update", "text": new_yol}, done(YOL["id"], True, len(texts), locked),
                               {"entities": texts, "settings": {"textStyles": [ADA, new_yol]}})]})
    out.append({"name": "Yeniden adlandır: nesneler stili kimliğiyle anar, değişmez; adım yok",
                "steps": [step("execute", {"kind": "text", "operation": "rename", "style": ADA["id"], "name": " Ada numarası "}, done(ADA["id"]),
                               {**flags, "entities": {"1": E["1"]}, "settings": {"textStyles": [dict(ADA, name="Ada numarası"), YOL]}})]})
    texts, locked = follow_texts(YOL, None)
    out.append({"name": "Sil: yazılar görünüşlerini korur, bağlarını yitirir; tek adım",
                "steps": [step("execute", {"kind": "text", "operation": "remove", "style": YOL["id"]}, done(YOL["id"], True, len(texts), locked),
                               {"entities": texts, "settings": {"textStyles": [ADA]}, "canUndo": True}),
                          step("undo", returns=TEXT_LABEL, expect={"entities": {"4": E["4"]}, "settings": {"textStyles": [ADA]}})]})
    texts, locked = follow_texts(ADA, None)
    out.append({"name": "Sil: kilitli katmandaki yazı bağını korur ve sayılır",
                "steps": [step("execute", {"kind": "text", "operation": "remove", "style": ADA["id"]}, done(ADA["id"], True, len(texts), locked),
                               {"entities": {**texts, "3": E["3"]}, "settings": {"textStyles": [YOL]}})]})
    new_olcu = {**OLCU, "decimals": 2, "height": 5.0}
    dims = follow_dimensions(OLCU, new_olcu)
    out.append({"name": "Ölçü stilini güncelle: ölçü yenisini alır; adım “Ölçü stili”",
                "steps": [step("execute", {"kind": "dimension", "operation": "update", "dimension": new_olcu}, done(OLCU["id"], True, len(dims)),
                               {"entities": dims, "settings": {"dimensionStyles": [new_olcu]}, "canUndo": True}),
                          step("undo", returns=DIM_LABEL, expect={"entities": {"5": E["5"]}})]})
    dims = follow_dimensions(OLCU, None)
    out.append({"name": "Ölçü stilini sil: ölçü bağını yitirir",
                "steps": [step("execute", {"kind": "dimension", "operation": "remove", "style": OLCU["id"]}, done(OLCU["id"], True, len(dims)),
                               {"entities": dims, "settings": {"dimensionStyles": None}})]})
    texts, locked = follow_texts(ADA, new_ada)
    out.append({"name": "Plan: izleyecek nesnelerin sayıları; yazmaz",
                "steps": [step("plan", {"kind": "text", "operation": "update", "text": new_ada}, done(ADA["id"], True, len(texts), locked), {**UNCHANGED, "settings": TABLES})]})
    out.append({"name": "Doğrula: yazmaz",
                "steps": [step("validate", {"kind": "text", "operation": "remove", "style": YOL["id"]}, {"status": "completed", "output": None, "warnings": []},
                               {**UNCHANGED, "settings": TABLES})]})
    out.append(refused("Oluştur: tanımsız", {"kind": "text", "operation": "create"}, "no_definition", "Stilin tanımını text ile verin.", "text"))
    out.append(refused("Oluştur: ölçü stili istenip yazı stili verilmiş", {"kind": "dimension", "operation": "create", "text": NOT}, "no_definition",
                       "Stilin tanımını dimension ile verin.", "dimension"))
    out.append(refused("Sil: stilsiz", {"kind": "text", "operation": "remove"}, "no_style", "Stili kimliğiyle verin (style).", "style"))
    out.append(refused("Yeniden adlandır: boş ad", {"kind": "text", "operation": "rename", "style": ADA["id"], "name": "  "}, "empty_name",
                       "Stilin adı boş olamaz; bir ad yazın.", "name"))
    out.append(refused("Oluştur: tabloda olan kimlik", {"kind": "text", "operation": "create", "text": dict(NOT, id=ADA["id"])}, "duplicate_style",
                       f"“{ADA['id']}” kimlikli yazı stili zaten var; yeni stile başka bir kimlik verin ya da kimliği boş bırakın.", "text.id"))
    out.append(refused("Güncelle: projede olmayan stil", {"kind": "text", "operation": "update", "text": NOT}, "style_not_found",
                       f"“{NOT['id']}” kimlikli yazı stili projede yok.", "text.id"))
    out.append(refused("Sil: projede olmayan ölçü stili", {"kind": "dimension", "operation": "remove", "style": CENTIK["id"]}, "style_not_found",
                       f"“{CENTIK['id']}” kimlikli ölçü stili projede yok.", "style"))
    clash = dict(NOT, name="ada NO")
    out.append(refused("Oluştur: aynı ad, büyük küçük harf ayırmadan", {"kind": "text", "operation": "create", "text": clash}, "invalid_style",
                       text_problem([ADA, YOL, clash]), "text"))
    standart = dict(NOT, name="Standart")
    out.append(refused("Oluştur: Standart ayrılmıştır", {"kind": "text", "operation": "create", "text": standart}, "invalid_style",
                       text_problem([ADA, YOL, standart]), "text"))
    flat = dict(NOT, height=0.0)
    out.append(refused("Oluştur: yükseklik 0", {"kind": "text", "operation": "create", "text": flat}, "invalid_style", text_problem([ADA, YOL, flat]), "text"))
    renamed = [ADA, dict(YOL, name="ADA NO")]
    out.append(refused("Yeniden adlandır: başka stilin adı", {"kind": "text", "operation": "rename", "style": YOL["id"], "name": "ADA NO"}, "invalid_style",
                       text_problem(renamed), "name"))
    out.append(refused("geçersiz beklenen sürüm", {"kind": "text", "operation": "remove", "style": YOL["id"], "expectedRevision": "on iki"}, "invalid_revision",
                       INVALID_REVISION, "expectedRevision"))
    out.append({"name": "sürüm çakışması: hiçbir şey yazılmaz",
                "steps": [step("captureRevision", **{"as": "once"}),
                          step("execute", {"kind": "text", "operation": "create", "text": NOT}, done(NOT["id"]), {"revision": "changed"}),
                          step("execute", {"kind": "text", "operation": "remove", "style": YOL["id"], "expectedRevision": "$once"},
                               {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                               {"revision": "same", "settings": {"textStyles": [ADA, YOL, NOT]}})]})
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.styles.edit",
        "commandVersion": 1,
        "title": "Yazı ve ölçü stilleri",
        "note": "Written by scripts/fixtures/style_edit_command_cases.py from the contract's rules and docs/adr/0183, 0248 §4; do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    doc = build()
    text_ = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text_:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.styles.edit durumları güncel: {len(doc['cases'])} durum.")
        return 0
    OUT.write_text(text_, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
