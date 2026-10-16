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


def done(style, changed=True, followed=0, locked=0, **lists):
    out = {"style": style, "changed": changed, "followed": followed, "locked": locked}
    for key in ("added", "replaced", "skipped", "library"):
        if lists.get(key):
            out[key] = lists[key]
    out["revision"] = "$current"
    return {"status": "completed", "output": out, "warnings": []}


FORMAT = "kentos.annotation-styles"


def library(kind, styles, fmt=FORMAT, version=1):
    """A styles file's text as export writes it (docs/adr/0322 §4): canonical JSON, the keys sorted, no white space."""
    return json.dumps({"format": fmt, "version": version, "kind": kind, "styles": styles}, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"))


def bad_library(why):
    return f"Stil dosyası okunamadı: {why}. KentOS'un stilleri dışa aktarmasının yazdığı bir {FORMAT} dosyası verin."


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
    out.extend(transfer_cases())
    out.extend(override_cases())
    return out


TEXT_OVERRIDES = ("font", "bold", "italic", "oblique", "widthFactor", "height")
DIMENSION_OVERRIDES = ("arrow", "arrowSize", "extOffset", "extBeyond", "textGap", "textPlace", "decimals", "unit", "prefix", "suffix",
                       "font", "dimLineColor", "dimLineWeight", "dimLineType", "extColor", "extWeight", "extLineType", "textColor",
                       "options", "height")


def text_fields(t):
    return {"font": t.get("font"), "bold": bool(t.get("bold")), "italic": bool(t.get("italic")), "oblique": t.get("oblique"),
            "widthFactor": t.get("widthFactor", 1), "height": t["height"]}


def styled_text_fields(style, t):
    face = A.face_of(style)
    return {"font": face.get("font"), "bold": bool(face.get("bold")), "italic": bool(face.get("italic")), "oblique": face.get("oblique"),
            "widthFactor": style.get("widthFactor", 1),
            "height": A.height_at(style["height"], SCALE) if style.get("height") is not None else t["height"]}


def dimension_fields(d):
    return {**{k: d.get(k) for k in DIMENSION_OVERRIDES if k != "height"}, "height": d["height"]}


def styled_dimension_fields(style):
    look = A.look_of(style)
    return {**{k: look.get(k) for k in DIMENSION_OVERRIDES if k != "height"}, "height": A.height_at(style["height"], SCALE)}


def overrides_after(names, before, after, style, own):
    """docs/adr/0322 §5: a field the step changed is the object's own when it differs from its style's value; one it
    did not change is as the overrides the step left it with say (`own`)."""
    return [n for n in names if (after[n] != style[n] if before[n] != after[n] else n in own)]


def with_overrides(entity, own):
    out = {k: v for k, v in entity.items() if k != "overrides"}
    if own:
        out["overrides"] = own
    return out


def hand(entity, style, **changes):
    """The object after a hand edit of `changes`, with its overrides as the document writes them."""
    after = {**entity, **changes}
    after = {k: v for k, v in after.items() if v is not None}
    # The edit writes the object as it was but for `changes`: its overrides too.
    if entity["kind"] == "text":
        own = overrides_after(TEXT_OVERRIDES, text_fields(entity), text_fields(after), styled_text_fields(style, after), after.get("overrides", []))
    else:
        own = overrides_after(DIMENSION_OVERRIDES, dimension_fields(entity), dimension_fields(after), styled_dimension_fields(style),
                              after.get("overrides", []))
    if style.get("height") is None and entity["kind"] == "text":
        own = [n for n in own if n != "height"]
    return with_overrides(after, own)


def edit_input(after):
    """cad.entities.edit's `properties` update writing `after`'s geometry (its face or look with it)."""
    geometry = {k: v for k, v in after.items() if k not in ("id", "layerId", "attrs", "color")}
    return {"operation": "properties", "changes": [{"kind": "update", "uid": f"$uidOf:{after['id']}", "geometry": geometry}]}


def edited(entity_after):
    return step("execute", edit_input(entity_after), command="cad.entities.edit", result="$any",
                expect={"entities": {str(entity_after["id"]): entity_after}})


def update_style(kind, style, expect):
    return step("execute", {"kind": kind, "operation": "update", kind: style}, "$any", {"entities": expect})


def override_cases():
    """Overrides (docs/adr/0322 §5): what a hand edit sets apart from the style stays when the style changes, though it
    may once hold the style's value; a field set back to the style's value follows again; an object no edit touched
    follows by the older rule. Each with the commands Öznitelikler and the Yazı and Ölçü stilleri windows use."""
    out = []
    t1, d5 = E["1"], E["5"]
    t1_font = hand(t1, ADA, font="overpass")
    out.append({"name": "Geçersiz kılma: yazı tipi elle değişince geçersiz kılınanlara yazılır", "steps": [edited(t1_font)]})
    via, then = dict(ADA, font="overpass"), dict(ADA, font="barlow")
    out.append({"name": "Geçersiz kılma: stil önce o yazı tipine sonra başkasına geçse de elle verilen kalır",
                "steps": [edited(t1_font), update_style("text", via, {"1": t1_font}),
                          update_style("text", then, {"1": A.follow_text(via, then, t1_font, SCALE)})]})
    out.append({"name": "Geçersiz kılma yokken (eski dosyalar) aynı adımlarda yazı stile uyar",
                "steps": [update_style("text", via, {"1": A.follow_text(ADA, via, t1, SCALE)}),
                          update_style("text", then, {"1": A.follow_text(via, then, A.follow_text(ADA, via, t1, SCALE), SCALE)})]})
    back = hand(t1_font, ADA, font="arimo")
    out.append({"name": "Geçersiz kılma: stilin değerine elle döndürülen alan yeniden stile uyar",
                "steps": [edited(t1_font), edited(back)]})
    t1_high = hand(t1, ADA, height=5.0)
    taller = dict(ADA, height=5.0)
    tallest = dict(ADA, height=6.0)
    out.append({"name": "Geçersiz kılma: elle verilen yükseklik stilin yüksekliği değişince kalır",
                "steps": [edited(t1_high), update_style("text", taller, {"1": A.follow_text(ADA, taller, t1_high, SCALE)}),
                          update_style("text", tallest, {"1": A.follow_text(taller, tallest, A.follow_text(ADA, taller, t1_high, SCALE), SCALE)})]})
    d5_own = hand(d5, OLCU, decimals=4)
    olcu2 = dict(OLCU, decimals=4)
    olcu3 = dict(OLCU, decimals=2)
    out.append({"name": "Geçersiz kılma: ölçünün basamağı elle; stil o basamağa sonra başkasına geçse de kalır",
                "steps": [edited(d5_own), update_style("dimension", olcu2, {"5": A.follow_dimension(OLCU, olcu2, d5_own, SCALE)}),
                          update_style("dimension", olcu3, {"5": A.follow_dimension(olcu2, olcu3, A.follow_dimension(OLCU, olcu2, d5_own, SCALE), SCALE)})]})
    removed = {k: v for k, v in t1_font.items() if k not in ("textStyle", "overrides")}
    out.append({"name": "Geçersiz kılma: stil silinince stilin bağıyla geçersiz kılınanlar da gider",
                "steps": [edited(t1_font), step("execute", {"kind": "text", "operation": "remove", "style": ADA["id"]}, "$any",
                                                {"entities": {"1": removed}})]})
    out.append({"name": "Geçersiz kılma: geri alma geçersiz kılmayı da geri alır",
                "steps": [edited(t1_font), step("undo", returns="Değiştir", expect={"entities": {"1": t1}})]})
    yol_text = A.apply_text(YOL, {k: v for k, v in t1_font.items() if k != "overrides"}, SCALE)
    out.append({"name": "Geçersiz kılma: başka stile geçen yazı baştan başlar",
                "steps": [edited(t1_font), edited(dict(yol_text, textStyle=YOL["id"]))]})
    wrong = {**t1, "overrides": ["renk"]}
    geometry = {k: v for k, v in wrong.items() if k not in ("id", "layerId", "attrs", "color")}
    out.append({"name": "Geçersiz kılma: stilin alanı olmayan ad reddedilir",
                "steps": [step("execute", {"operation": "properties", "changes": [{"kind": "update", "uid": "$uidOf:1", "geometry": geometry}]},
                               failed("invalid_style", "“renk” stilin bir alanı değil; geçersiz kılınan alanlar: font, bold, italic, oblique, widthFactor, height.",
                                      "changes[0].geometry.overrides"),
                               {"entities": {"1": t1}}, command="cad.entities.edit")]})
    return out


def transfer_cases():
    """Export and import (docs/adr/0322 §4): the kind's styles given out as a file's text, writing nothing; a file's
    styles taken in by name, trimmed and case folded: a new one added (its id kept when free), one the table has
    skipped, or with `replace` updated in its place, its objects following as an update makes them."""
    out = []
    out.append({"name": "Dışa aktar: bütün yazı stilleri tablonun sırasıyla; yazmaz",
                "steps": [step("execute", {"kind": "text", "operation": "export"},
                               done(ADA["id"], False, library=library("text", [ADA, YOL])), {**UNCHANGED, "settings": TABLES})]})
    out.append({"name": "Dışa aktar: seçilen ölçü stili",
                "steps": [step("execute", {"kind": "dimension", "operation": "export", "styles": [OLCU["id"]]},
                               done(OLCU["id"], False, library=library("dimension", [OLCU])), {**UNCHANGED, "settings": TABLES})]})
    out.append({"name": "Dışa aktar: seçimin sırası değil, tablonunki",
                "steps": [step("execute", {"kind": "text", "operation": "export", "styles": [YOL["id"], ADA["id"]]},
                               done(ADA["id"], False, library=library("text", [ADA, YOL])), {**UNCHANGED, "settings": TABLES})]})
    out.append(refused("Dışa aktar: projede olmayan stil", {"kind": "text", "operation": "export", "styles": [ADA["id"], NOT["id"]]},
                       "style_not_found", f"“{NOT['id']}” kimlikli yazı stili projede yok.", "styles[1]"))
    file = library("text", [NOT, dict(ADA, name="ADA NO", font="overpass")])
    flags = {"ids": IDS, "dirty": True, "revision": "changed", "canUndo": False}
    out.append({"name": "Al: yeni ad eklenir (kimliği korunur), tablodaki ad (büyük küçük harf ayırmadan) olduğu gibi kalır; adım yok",
                "steps": [step("execute", {"kind": "text", "operation": "import", "library": file},
                               done(NOT["id"], True, added=[NOT["id"]], skipped=["ADA NO"]),
                               {**flags, "settings": {"textStyles": [ADA, YOL, NOT]}})]})
    new_ada = {**ADA, "font": "overpass", "height": 5.0}
    texts, locked = follow_texts(ADA, new_ada)
    out.append({"name": "Al, değiştirerek: aynı adlı stil yerinde, kimliği ve adıyla güncellenir, yazıları izler; tek adım “Yazı stili”",
                "steps": [step("execute", {"kind": "text", "operation": "import", "replace": True,
                                           "library": library("text", [dict(new_ada, id=NOT["id"], name=" ada no ")])},
                               done(ADA["id"], True, len(texts), locked, replaced=[ADA["id"]]),
                               {"ids": IDS, "entities": texts, "settings": {"textStyles": [new_ada, YOL]}, "canUndo": True,
                                "dirty": True})]})
    out.append({"name": "Al: plan yazmaz",
                "steps": [step("plan", {"kind": "text", "operation": "import", "library": file},
                               done(NOT["id"], True, added=[NOT["id"]], skipped=["ADA NO"]), {**UNCHANGED, "settings": TABLES})]})
    out.append(refused("Al: dosyasız", {"kind": "text", "operation": "import"}, "no_library",
                       "Alınacak stil dosyasının metnini (library) verin.", "library"))
    out.append(refused("Al: başka biçim", {"kind": "text", "operation": "import", "library": library("text", [], fmt="kentos.blocks")},
                       "invalid_library", bad_library(f"biçimi {FORMAT} değil"), "library"))
    out.append(refused("Al: başka sürüm", {"kind": "text", "operation": "import", "library": library("text", [], version=2)},
                       "invalid_library", bad_library("sürümü 2; bu KentOS 1. sürümü okur"), "library"))
    out.append(refused("Al: ölçü stilleri yazı stili olarak alınmaz", {"kind": "text", "operation": "import", "library": library("dimension", [CENTIK])},
                       "invalid_library", bad_library("dosyanın stilleri “dimension”; istenen “text”"), "library"))
    standart = dict(NOT, name="Standart")
    out.append(refused("Al: Standart adı ayrılmıştır", {"kind": "text", "operation": "import", "library": library("text", [standart])},
                       "invalid_style", text_problem([ADA, YOL, standart]), "library"))
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
