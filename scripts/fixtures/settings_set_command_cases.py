"""Writes fixtures/commands/v1/cad.settings.set.json: the shared cases of `cad.settings.set` v1 (docs/adr/0248 §5).

The expectations are built here from the contract's rules (crates/shared/commands/src/cad_settings.rs: the keys, the
checks and their order, the codes, paths and words) and from the ADRs' meaning of each key, without KentOS code:

- a coordinate system is assigned, never a transformation: the objects stay where they are (CLAUDE.md §5); it takes
  the place of a defined system (docs/adr/0168 §1) and makes the unit metres (docs/adr/0165 §2); the systems are the
  registry's (fixtures/crs/v1/registry.json, data), 0 a local project;
- a second system is another registered system than the project's own, for a project with one (docs/adr/0167 §1);
- the plot scale's change gives the texts at the old general height the new one in one undo step “Yazı yüksekliklerini
  uydur” (docs/adr/0205 §3): a text without its own height on paper is at the project's height for texts (2.5 mm by
  default), on paper, times the scale over a thousand;
- every other key is the project's setting: an edit, no undo step.

    python3 scripts/fixtures/settings_set_command_cases.py           # writes the file
    python3 scripts/fixtures/settings_set_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.settings.set.json"
sys.path.insert(0, str(Path(__file__).resolve().parent))
import levels_assign_cases as la  # noqa: E402  (ADR 0717's drawing: every kind that has a level of its own)
REGISTRY = {s["srid"] for s in json.loads((ROOT / "fixtures/crs/v1/registry.json").read_text(encoding="utf-8"))["systems"]}

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
X, Y = 500000, 4400000
SCALE = 1000
TEXT_MM = 2.5  # the general height of a text on paper (docs/adr/0205 §1)
FOLLOW = "Yazı yüksekliklerini uydur"

INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def node(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": dict(STYLE), "children": []}


def height_at(mm, scale):
    return mm / 1000 * scale


def text(id_, layer, height, x=0):
    return {"kind": "text", "id": id_, "layerId": layer, "attrs": {}, "p": {"x": X + x, "y": Y}, "text": f"T{id_}", "height": height, "rotation": 0}


ENTITIES = [
    text(1, "yazi", height_at(TEXT_MM, SCALE)),
    text(2, "yazi", 7.0, x=10),
    text(3, "kilitli", height_at(TEXT_MM, SCALE), x=20),
    {"kind": "line", "id": 4, "layerId": "yazi", "attrs": {}, "a": {"x": X, "y": Y + 10}, "b": {"x": X + 50, "y": Y + 10}},
]
IDS = [e["id"] for e in ENTITIES]
E = {str(e["id"]): e for e in ENTITIES}

SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": SCALE,
            "workspace": "gis", "drawingFont": "barlow", "secondSrid": 4326}


def setup(**over):
    s = {**SETTINGS, **over}
    for k in [k for k, v in s.items() if v is None]:
        s.pop(k)
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Ayarlar",
        "settings": s,
        "origin": {"x": X, "y": Y},
        "layers": [node("yazi", "Yazı"), node("kilitli", "Kilitli", locked=True)],
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


def done(changed, followed=0, locked=0):
    return {"status": "completed", "output": {"changed": list(changed), "followed": followed, "locked": locked, "revision": "$current"}, "warnings": []}


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
FLAGS = {"ids": IDS, "dirty": True, "revision": "changed", "canUndo": False}


def refused(name, input_, code, message, path=None, setup_doc=None):
    c = {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "settings": {k: SETTINGS[k] for k in ("srid", "secondSrid", "plotScale")}})]}
    if setup_doc is not None:
        c["setup"] = setup_doc
        c["steps"][0]["expect"].pop("settings")
    return c


def followers(scale):
    """The texts at the old general height that take the new one, those a locked layer keeps."""
    old, new = height_at(TEXT_MM, SCALE), height_at(TEXT_MM, scale)
    out, locked = {}, 0
    for e in ENTITIES:
        if e["kind"] == "text" and e["height"] == old and old != new:
            if e["layerId"] == "kilitli":
                locked += 1
            else:
                out[str(e["id"])] = dict(e, height=new)
    return out, locked


def cases():
    out = []
    out.append({"name": "Ad ve ilk tür: proje ayarıdır, geri alma adımı değildir",
                "setup": setup(workspace=None),
                "steps": [step("execute", {"name": "  Mimari proje ", "projectType": "cad"}, done(["name", "projectType"]),
                               {**FLAGS, "name": "Mimari proje", "settings": {"workspace": "cad"}})]})
    # The type is immutable at the product command boundary too (UI, Python and MCP).
    for before, after in (("gis", "cad"), ("cad", "gis")):
        for op in ("validate", "plan", "execute"):
            out.append({"name": f"Tür kilidi: {before} → {after}, {op}; ad ve ölçek de değişmez",
                        "setup": setup(workspace=before),
                        "steps": [step(op, {"projectType": after, "name": "Değişmesin", "plotScale": 500},
                                       failed("project_type_locked", "Proje türü projeyle başlar ve değişmez; CAD ile CBS arasında geçiş ileride projeyi dönüştürerek olacak.", "projectType"),
                                       {**UNCHANGED, "name": "Ayarlar", "settings": {"workspace": before, "plotScale": SCALE}})]})
        out.append({"name": f"Tür kilidi: aynı tür {before} tekrar verilebilir",
                    "setup": setup(workspace=before),
                    "steps": [step("execute", {"projectType": before}, done([]), UNCHANGED)]})
    for before in (None, "hybrid"):
        for after in ("gis", "cad"):
            out.append({"name": f"Eski projenin ilk türü: {before} → {after}",
                        "setup": setup(workspace=before),
                        "steps": [step("execute", {"projectType": after}, done(["projectType"]),
                                       {**FLAGS, "settings": {"workspace": after}})]})
    out.extend(work_cases())
    # Otomatik seçim (docs/adr/0622 §5): the project's choice, written even when it is the type's own; no object changes.
    out.append({"name": "Otomatik seçim: kapatılır ve açılır; proje ayarıdır",
                "steps": [step("execute", {"autoSelect": False}, done(["autoSelect"]), {**FLAGS, "settings": {"autoSelect": False}}),
                          step("execute", {"autoSelect": False}, done([]), {"settings": {"autoSelect": False}}),
                          step("execute", {"autoSelect": True}, done(["autoSelect"]), {"settings": {"autoSelect": True}})]})
    out.append({"name": "Basamaklar ve birimler",
                "steps": [step("execute", {"lengthDecimals": 2, "areaDecimals": 4, "areaUnit": "donum", "angleUnit": "deg", "drawingFont": "arimo"},
                               done(["lengthDecimals", "areaDecimals", "areaUnit", "angleUnit", "drawingFont"]),
                               {**FLAGS, "settings": {"lengthDecimals": 2, "areaDecimals": 4, "areaUnit": "donum", "angleUnit": "deg", "drawingFont": "arimo"}})]})
    out.append({"name": "Aynı değerler bir şey değiştirmez",
                "steps": [step("execute", {"lengthDecimals": 3, "areaUnit": "m2", "srid": 5256}, done([]), {**UNCHANGED, "settings": {"srid": 5256}})]})
    out.append({"name": "Koordinat sistemi atanır, dönüştürülmez: nesneler yerinde; ikinci sistem kalır",
                "steps": [step("execute", {"srid": 5255}, done(["srid"]),
                               {**FLAGS, "entities": {"4": E["4"]}, "settings": {"srid": 5255, "secondSrid": 4326}})]})
    out.append({"name": "Yerel proje (0): ikinci sistem düşer; birim verilebilir",
                "steps": [step("execute", {"srid": 0, "drawingUnit": "mm"}, done(["srid", "secondSrid", "drawingUnit"]),
                               {**FLAGS, "settings": {"srid": 0, "secondSrid": None, "drawingUnit": "mm"}})]})
    out.append({"name": "Yerel projeye sistem atanınca birim metre olur",
                "setup": setup(srid=0, secondSrid=None, drawingUnit="cm"),
                "steps": [step("execute", {"srid": 5256}, done(["srid", "drawingUnit"]), {**FLAGS, "settings": {"srid": 5256, "drawingUnit": None}})]})
    out.append({"name": "Birim metre: yazılmaz",
                "setup": setup(srid=0, secondSrid=None, drawingUnit="cm"),
                "steps": [step("execute", {"drawingUnit": "m"}, done(["drawingUnit"]), {**FLAGS, "settings": {"drawingUnit": None}})]})
    # Feet and inches (docs/adr/0262): a local project's in and ft, and the length styles they allow.
    out.append({"name": "İnç birimi ve mimari biçim (ADR 0262)",
                "setup": setup(srid=0, secondSrid=None),
                "steps": [step("execute", {"drawingUnit": "in", "lengthStyle": "architectural"}, done(["drawingUnit", "lengthStyle"]),
                               {**FLAGS, "settings": {"drawingUnit": "in", "lengthStyle": "architectural"}})]})
    out.append({"name": "Ayak birimi, mühendislik biçimi; ondalık biçim yazılmaz",
                "setup": setup(srid=0, secondSrid=None),
                "steps": [step("execute", {"drawingUnit": "ft", "lengthStyle": "engineering"}, done(["drawingUnit", "lengthStyle"]),
                               {**FLAGS, "settings": {"drawingUnit": "ft", "lengthStyle": "engineering"}}),
                          step("execute", {"lengthStyle": "decimal"}, done(["lengthStyle"]), {"settings": {"drawingUnit": "ft", "lengthStyle": None}, "revision": "changed"})]})
    out.append({"name": "Metrik birime dönünce ayak ve inç biçimi ondalık olur",
                "setup": setup(srid=0, secondSrid=None, drawingUnit="in", lengthStyle="fractional"),
                "steps": [step("execute", {"drawingUnit": "mm"}, done(["drawingUnit", "lengthStyle"]),
                               {**FLAGS, "settings": {"drawingUnit": "mm", "lengthStyle": None}})]})
    out.append({"name": "İnç projesine sistem atanınca birim metre, biçim ondalık olur",
                "setup": setup(srid=0, secondSrid=None, drawingUnit="in", lengthStyle="architectural"),
                "steps": [step("execute", {"srid": 5256}, done(["srid", "drawingUnit", "lengthStyle"]),
                               {**FLAGS, "settings": {"srid": 5256, "drawingUnit": None, "lengthStyle": None}})]})
    out.append({"name": "İkinci sistem: başka bir sistem; 0 kaldırır",
                "steps": [step("execute", {"secondSrid": 2321}, done(["secondSrid"]), {**FLAGS, "settings": {"secondSrid": 2321}}),
                          step("execute", {"secondSrid": 0}, done(["secondSrid"]), {"settings": {"secondSrid": None}, "revision": "changed"})]})
    texts, locked = followers(500)
    out.append({"name": "Çizim ölçeği: eski genel yükseklikteki yazılar yenisini tek adımda alır; elle verilen ve kilitli katmandaki kalır",
                "steps": [step("execute", {"plotScale": 500}, done(["plotScale"], len(texts), locked),
                               {"ids": IDS, "entities": {**texts, "2": E["2"], "3": E["3"]}, "settings": {"plotScale": 500}, "canUndo": True, "dirty": True}),
                          step("undo", returns=FOLLOW, expect={"entities": {k: E[k] for k in texts}, "settings": {"plotScale": 500}})]})
    out.append({"name": "Plan: değişecek anahtarlar ve yazılar; yazmaz",
                "steps": [step("plan", {"plotScale": 500, "projectType": "gis"}, done(["plotScale"], len(texts), locked), {**UNCHANGED, "settings": {"plotScale": SCALE}})]})
    out.append({"name": "Doğrula: yazmaz", "steps": [step("validate", {"srid": 5255}, {"status": "completed", "output": None, "warnings": []}, UNCHANGED)]})
    out.append(refused("anahtarsız", {}, "nothing_to_set", "Değişecek bir ayar yok: en az bir anahtar verin."))
    out.append(refused("boş ad", {"name": " "}, "empty_name", "Çizimin adı boş olamaz; bir ad yazın.", "name"))
    out.append(refused("seçilemeyen tür", {"projectType": "plan3d"}, "invalid_project_type", "Proje türü cad ya da gis olabilir; öbürleri henüz seçilemez.", "projectType"))
    out.append(refused("uzunluk basamağı 5", {"lengthDecimals": 5}, "invalid_decimals", "Gösterilen basamak 0 ile 4 arasında bir tam sayı olmalı.", "lengthDecimals"))
    out.append(refused("alan basamağı 9", {"lengthDecimals": 4, "areaDecimals": 9}, "invalid_decimals", "Gösterilen basamak 0 ile 4 arasında bir tam sayı olmalı.", "areaDecimals"))
    scale_words = "Çizim ölçeği 1 ile 1 000 000 arasında bir tam sayı payda olmalı (1:1000 için 1000)."
    for name, value in (("küsuratlı ölçek", 1000.5), ("ölçek 0", 0), ("ölçek 1 000 001", 1000001)):
        out.append(refused(name, {"plotScale": value}, "invalid_scale", scale_words, "plotScale"))
    nan = refused("ölçek NaN", {"plotScale": 1000}, "invalid_scale", scale_words, "plotScale")
    nan["steps"][0]["nonFinite"] = {"plotScale": "NaN"}
    out.append(nan)
    assert 9999 not in REGISTRY and 2321 in REGISTRY and 5255 in REGISTRY
    out.append(refused("kayıtta olmayan sistem", {"srid": 9999}, "unknown_srid",
                       "EPSG:9999 koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin ya da Proje ayarları'nda sistemi tanımlayın.", "srid"))
    out.append(refused("kayıtta olmayan ikinci sistem", {"secondSrid": 31700}, "unknown_srid",
                       "EPSG:31700 koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin ya da Proje ayarları'nda sistemi tanımlayın.", "secondSrid"))
    out.append(refused("yerel projeye ikinci sistem", {"srid": 0, "secondSrid": 4326}, "second_needs_system",
                       "Koordinat sistemi olmayan yerel projenin ikinci sistemi olmaz; önce projeye bir sistem atayın.", "secondSrid"))
    out.append(refused("ikinci sistem projenin kendisi", {"secondSrid": 5256}, "second_is_own",
                       "EPSG:5256 projenin kendi sistemi; ikinci sistem başka bir sistem olmalı.", "secondSrid"))
    out.append(refused("sistemi olan projede mm", {"drawingUnit": "mm"}, "unit_needs_local",
                       "Koordinat sistemi olan projenin birimi metredir; mm, cm, inç ya da ayak yalnız sistemsiz yerel projede seçilir.", "drawingUnit"))
    out.append(refused("sistemi olan projede inç", {"drawingUnit": "in"}, "unit_needs_local",
                       "Koordinat sistemi olan projenin birimi metredir; mm, cm, inç ya da ayak yalnız sistemsiz yerel projede seçilir.", "drawingUnit"))
    out.append(refused("metre projesinde mimari biçim", {"lengthStyle": "architectural"}, "style_needs_imperial",
                       "Ayak ve inç biçimleri (mimari, mühendislik, kesirli) yalnız inç ya da ayak biriminde seçilir; önce birimi inç ya da ayak yapın.", "lengthStyle"))
    out.append(refused("mm'ye geçerken kesirli biçim", {"srid": 0, "drawingUnit": "mm", "lengthStyle": "fractional"}, "style_needs_imperial",
                       "Ayak ve inç biçimleri (mimari, mühendislik, kesirli) yalnız inç ya da ayak biriminde seçilir; önce birimi inç ya da ayak yapın.", "lengthStyle"))
    # The project's levels and the walls' detail (docs/adr/0339 §3): given whole; an empty object takes them away; a
    # level a wall names stays.
    levels = [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 3}, {"id": "k1", "name": "1. kat", "elevation": 3}]
    out.append({"name": "Mimari: katlar ve görünüş ayrıntısı (ADR 0339)",
                "steps": [step("execute", {"arch": {"levels": levels, "detail": "standard"}}, done(["arch"]),
                               {**FLAGS, "settings": {"arch": {"levels": levels, "detail": "standard"}}}),
                          step("execute", {"arch": {}}, done(["arch"]), {"settings": {"arch": None}, "revision": "changed"})]})
    wall = {"kind": "wall", "id": 5, "layerId": "yazi", "attrs": {}, "pts": [{"x": X, "y": Y + 20}, {"x": X + 5, "y": Y + 20}],
            "width": 0.2, "justify": "center", "arch": {"level": "zemin"}}
    walled = setup(arch={"levels": levels})
    walled["entities"].append(wall)
    in_use = refused("duvarın andığı kat kaldırılmaz", {"arch": {"levels": levels[1:]}}, "level_in_use",
                     "“Zemin kat” katını 1 duvar anıyor; kaldırılmadı. " + FIX_OBJECTS, "arch.levels", walled)
    in_use["steps"][0]["result"]["error"]["uids"] = ["$uidOf:5"]
    in_use["steps"][0]["expect"]["ids"] = IDS + [5]
    in_use["steps"][0]["expect"]["settings"] = {"arch": {"levels": levels}}
    out.append(in_use)
    out.extend(level_state_cases(levels, walled))
    out.append({"name": "duvarın andığı kat kaldıkça öbürleri değişir", "setup": walled,
                "steps": [step("execute", {"arch": {"levels": levels[:1], "detail": "detailed"}}, done(["arch"]),
                               {**FLAGS, "ids": IDS + [5], "settings": {"arch": {"levels": levels[:1], "detail": "detailed"}}})]})
    out.append(refused("iki kat aynı kimlikte", {"arch": {"levels": [levels[0], dict(levels[1], id="zemin")]}}, "invalid_arch",
                       "Katlar geçersiz: “zemin” kimlikli kat iki kez var. Her katın kimliği ve adı ayrı olmalı.", "arch.levels"))
    out.append(refused("katın yüksekliği sıfır", {"arch": {"levels": [dict(levels[0], height=0)]}}, "invalid_arch",
                       "“Zemin kat” katının yüksekliği sıfırdan büyük olmalı.", "arch.levels[0].height"))
    # What the elevations are and the coordinates' epoch (docs/adr/0519 §1): assigned, no elevation moves; unknown and 0
    # take them away; ellipsoidal heights and an epoch need a system, and go with it.
    ortho = {"kind": "orthometric", "verticalSrid": 5775, "geoid": {"name": "TG-20"}}
    out.append({"name": "Kotlar ve epok (ADR 0519): atanır, kotlar değişmez; bilinmiyor ve 0 kaldırır",
                "steps": [step("execute", {"heights": ortho, "coordinateEpoch": 2023.5}, done(["heights", "coordinateEpoch"]),
                               {**FLAGS, "entities": {"4": E["4"]}, "settings": {"heights": ortho, "coordinateEpoch": 2023.5}}),
                          step("execute", {"heights": {"kind": "unknown"}, "coordinateEpoch": 0}, done(["heights", "coordinateEpoch"]),
                               {"settings": {"heights": None, "coordinateEpoch": None}, "revision": "changed"})]})
    out.append({"name": "Yerel projede ortometrik kot olur",
                "setup": setup(srid=0, secondSrid=None),
                "steps": [step("execute", {"heights": {"kind": "orthometric", "verticalSrid": 5714}}, done(["heights"]),
                               {**FLAGS, "settings": {"heights": {"kind": "orthometric", "verticalSrid": 5714}}})]})
    out.append({"name": "Sistem kalkınca elipsoit yüksekliği ve epok da kalkar",
                "setup": setup(heights={"kind": "ellipsoidal"}, coordinateEpoch=2020.0),
                "steps": [step("execute", {"srid": 0}, done(["srid", "secondSrid", "heights", "coordinateEpoch"]),
                               {**FLAGS, "settings": {"srid": 0, "secondSrid": None, "heights": None, "coordinateEpoch": None}})]})
    heights_words = "Yükseklik tanımı: "
    out.append(refused("elipsoit yüksekliği düşey sistem anmaz", {"heights": {"kind": "ellipsoidal", "verticalSrid": 3855}},
                       "invalid_heights", heights_words + "düşey sistem ve geoit modeli yalnız ortometrik yüksekliğin olur; "
                       "elipsoit yüksekliğinin elipsoidi yatay sistemindir.", "heights.kind"))
    out.append(refused("bilinmeyen düşey sistem", {"heights": {"kind": "orthometric", "verticalSrid": 5703}}, "invalid_heights",
                       heights_words + "EPSG:5703 KentOS'un bildiği düşey sistemlerden değil (Antalya, EGM2008, EGM96, MSL, "
                       "Karadeniz).", "heights.verticalSrid"))
    out.append(refused("geoit dosyasının özeti SHA-256 değil",
                       {"heights": {"kind": "orthometric", "geoid": {"name": "TG-20", "sha256": "ABC"}}}, "invalid_heights",
                       heights_words + "geoit dosyasının özeti küçük harfli 64 onaltılık rakamla SHA-256 olmalı.",
                       "heights.geoid.sha256"))
    out.append(refused("epok 1850", {"coordinateEpoch": 1850}, "invalid_epoch",
                       "Epok: koordinat epoku 1900 ile 2100 arasında ondalık bir yıl olmalı (2023.5 gibi); 0 epoku kaldırır.",
                       "coordinateEpoch"))
    need = ("Koordinat sistemi olmayan yerel projede elipsoit yüksekliği ve koordinat epoku olmaz: ikisi de bir datumun "
            "elipsoidine ve zamanına bağlıdır. Önce projeye bir sistem atayın ya da ortometrik yüksekliği seçin.")
    out.append(refused("yerel projede elipsoit yüksekliği", {"srid": 0, "heights": {"kind": "ellipsoidal"}},
                       "heights_need_system", need, "heights.kind"))
    out.append(refused("yerel projede epok", {"srid": 0, "coordinateEpoch": 2023.5}, "heights_need_system", need,
                       "coordinateEpoch"))
    out.append(refused("girdinin kuralları sürümden önce", {"lengthDecimals": 7, "expectedRevision": "on iki"}, "invalid_decimals",
                       "Gösterilen basamak 0 ile 4 arasında bir tam sayı olmalı.", "lengthDecimals"))
    out.append(refused("geçersiz beklenen sürüm", {"srid": 5255, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append({"name": "sürüm çakışması: hiçbir şey yazılmaz",
                "steps": [step("captureRevision", **{"as": "once"}),
                          step("execute", {"areaDecimals": 3}, done(["areaDecimals"]), {"revision": "changed"}),
                          step("execute", {"srid": 5255, "expectedRevision": "$once"},
                               {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                               {"revision": "same", "settings": {"srid": 5256, "areaDecimals": 3}})]})
    return out


# What a CAD project is drawn for (docs/adr/0801): `architecture`, `map` or `detail`; a CAD project's only (a project
# without a type takes it with `projectType: cad`), chosen once like the type; the same work again changes nothing. The
# refusals come after the type's and before the decimals': `cad_work_needs_cad`, then `cad_work_locked`.
NEEDS_CAD = ("İş (cadWork) yalnız CAD projesinin ayarıdır; bu proje CAD projesi değil. Türü seçilmemiş eski projede "
             "projectType: cad ile birlikte verin.")
WORK_LOCKED = ("Projenin işi projeyle başlar ve değişmez; Mimari ile Harita arasında geçiş ileride projeyi dönüştürerek "
               "olacak.")
TYPE_LOCKED = "Proje türü projeyle başlar ve değişmez; CAD ile CBS arasında geçiş ileride projeyi dönüştürerek olacak."


def work_cases():
    out = []
    for work in ("architecture", "map", "detail"):
        out.append({"name": f"İş: işi sorulmamış CAD projesine bir kez verilir ({work}); aynısı bir şey değiştirmez",
                    "setup": setup(workspace="cad"),
                    "steps": [step("execute", {"cadWork": work}, done(["cadWork"]),
                                   {**FLAGS, "settings": {"workspace": "cad", "cadWork": work}}),
                              step("execute", {"cadWork": work}, done([]),
                                   {"settings": {"cadWork": work}, "revision": "same", "canUndo": False})]})
    for before in (None, "hybrid"):
        out.append({"name": f"Tür ve iş birlikte: türü sorulmamış proje ({before}) CAD ve detay olur",
                    "setup": setup(workspace=before),
                    "steps": [step("execute", {"projectType": "cad", "cadWork": "detail"}, done(["projectType", "cadWork"]),
                                   {**FLAGS, "settings": {"workspace": "cad", "cadWork": "detail"}})]})
    out.append({"name": "İş: plan yazmaz",
                "setup": setup(workspace="cad"),
                "steps": [step("plan", {"cadWork": "map"}, done(["cadWork"]),
                               {**UNCHANGED, "settings": {"workspace": "cad", "cadWork": None}})]})
    for op in ("validate", "plan", "execute"):
        out.append({"name": f"İş kilidi: harita → mimari, {op}; ad da değişmez",
                    "setup": setup(workspace="cad", cadWork="map"),
                    "steps": [step(op, {"cadWork": "architecture", "name": "Değişmesin"},
                                   failed("cad_work_locked", WORK_LOCKED, "cadWork"),
                                   {**UNCHANGED, "name": "Ayarlar", "settings": {"workspace": "cad", "cadWork": "map"}})]})
    out.append({"name": "İş kilidi: detay → mimari de değişmez (aynı programın işleri)",
                "setup": setup(workspace="cad", cadWork="detail"),
                "steps": [step("execute", {"cadWork": "architecture"}, failed("cad_work_locked", WORK_LOCKED, "cadWork"),
                               {**UNCHANGED, "settings": {"cadWork": "detail"}})]})
    for name, over, input_ in (
            ("CBS projesine iş verilmez", {"workspace": "gis"}, {"cadWork": "architecture"}),
            ("türü sorulmamış projeye iş tek başına verilmez", {"workspace": None}, {"cadWork": "map"}),
            ("eski Hibrit projesine iş tek başına verilmez", {"workspace": "hybrid"}, {"cadWork": "map"}),
            ("türsüz projeye CBS türüyle iş verilmez", {"workspace": None}, {"projectType": "gis", "cadWork": "map"})):
        # The former Hibrit mode is held as a type not asked yet (docs/adr/0165 §1).
        held = None if over["workspace"] == "hybrid" else over["workspace"]
        out.append({"name": f"İş: {name}",
                    "setup": setup(**over),
                    "steps": [step("execute", input_, failed("cad_work_needs_cad", NEEDS_CAD, "cadWork"),
                                   {**UNCHANGED, "settings": {"workspace": held, "cadWork": None}})]})
    out.append({"name": "İş: tür kilidi önce söylenir",
                "setup": setup(workspace="cad"),
                "steps": [step("execute", {"projectType": "gis", "cadWork": "map"},
                               failed("project_type_locked", TYPE_LOCKED, "projectType"),
                               {**UNCHANGED, "settings": {"workspace": "cad", "cadWork": None}})]})
    out.append({"name": "İş: girdinin kuralları önce (basamak), sonra iş",
                "setup": setup(workspace="gis"),
                "steps": [step("execute", {"cadWork": "map", "lengthDecimals": 9},
                               failed("invalid_decimals", "Gösterilen basamak 0 ile 4 arasında bir tam sayı olmalı.", "lengthDecimals"),
                               {**UNCHANGED, "settings": {"workspace": "gis", "cadWork": None}})]})
    return out


# A level an object names by its own level or a sheet's map shows stays (docs/adr/0339 §3, 0720 §3): the refusal says
# the level by its name, how many of each kind name it (in this order) and the maps that show it; the objects' ids go
# with it. Only the levels the input takes away are looked at. A Kot bound to a level names it for its value
# (docs/adr/0721 §2): it counts too, last.
KINDS = ["duvar", "mahal", "mahal ayırıcı", "kolon", "aks", "merdiven", "rampa", "kot işareti"]
FIX_OBJECTS = "Önce bu nesnelere başka kat verin (seçip Öznitelikler › Kat) ya da katı bırakın."
FIX_MAPS = "Önce haritanın katını değiştirin (paftada Harita › Kat) ya da katı bırakın."
FIX_BOTH = "Önce nesnelere ve haritalara başka kat verin ya da katı bırakın."


def kind_of(e):
    if e["kind"] == "dimension":
        return KINDS.index("kot işareti") if "level" in e else None
    if e["kind"] == "polyline":
        return KINDS.index("mahal ayırıcı") if "spaceSeparator" in e else None
    names = {"wall": "duvar", "space": "mahal", "column": "kolon", "grid": "aks", "stair": "merdiven", "ramp": "rampa"}
    return KINDS.index(names[e["kind"]]) if e["kind"] in names else None


def listed(items):
    return items[0] if len(items) == 1 else ", ".join(items[:-1]) + " ve " + items[-1]


def in_use_words(doc, level):
    """What the refusal of taking `level` away says, and the ids of the objects that name it (None: nothing does)."""
    counts, ids = [0] * len(KINDS), []
    for e in doc["entities"]:
        k = kind_of(e)
        named = (e.get("level") or {}).get("id") if e["kind"] == "dimension" else la.level_of(e)
        if k is not None and named == level["id"]:
            counts[k] += 1
            ids.append(e["id"])
    book = (doc.get("sheets") or {}).get("book") or {}
    maps = sum(1 for sheet in book.get("sheets", []) + book.get("masters", []) for it in sheet.get("items", [])
               if it.get("kind", {}).get("type") == "map" and (it["kind"].get("level") or {}).get("id") == level["id"])
    objects = [f"{n} {name}" for n, name in zip(counts, KINDS) if n]
    if not objects and not maps:
        return None, []
    if objects and not maps:
        what, fix = f"{listed(objects)} anıyor", FIX_OBJECTS
    elif maps and not objects:
        what, fix = f"{maps} pafta haritası gösteriyor", FIX_MAPS
    else:
        what, fix = f"{listed(objects)} anıyor, {maps} pafta haritası gösteriyor", FIX_BOTH
    return f"“{level['name']}” katını {what}; kaldırılmadı. {fix}", ids


def book_with_map(level_id):
    """A sheet book of one sheet whose map shows `level_id` (docs/adr/0716), the preflight cases' book's form."""
    book = copy.deepcopy(json.loads((ROOT / "fixtures/commands/v1/file.preflight.sheets.json").read_text(encoding="utf-8"))["book"])
    book["sheets"][0]["items"][0]["kind"]["level"] = {"id": level_id}
    return book


def as_v2(doc, uid_base=0x5e7):
    """The v1 setup as the file's v2 form: its objects' persistent ids given, so a sheet book can go with it."""
    d = copy.deepcopy(doc)
    d["version"] = 2
    d["uids"] = [f"0192f5a1-7171-7000-8000-{uid_base * 0x1000 + e['id']:012x}" for e in d["entities"]]
    # A file's elevations and heights are floats (KCAD's arch settings, docs/specs/kcad-v2.md).
    for level in (d["settings"].get("arch") or {}).get("levels", []):
        for key in ("elevation", "height"):
            if key in level:
                level[key] = float(level[key])
    return d


def level_state_cases(levels, walled):
    """Taking levels away (ADR 0720 §3) and the level and view a drawing opens with (ADR 0720 §1)."""
    out = []
    zemin, k1 = levels
    # Every kind that names a level by its own, ADR 0717's drawing with them all on Zemin kat (a locked wall too).
    every = la.drawing()
    for e in every["entities"]:
        if e["id"] in (la.WALL, la.ROOM, la.COLUMN, la.STAIR, la.RAMP, la.GRID, la.SEPARATOR, la.LOCKED):
            e.update(la.given(e, "zemin"))
    every_levels = every["settings"]["arch"]["levels"]
    words, ids = in_use_words(every, every_levels[0])
    assert words.startswith("“Zemin kat” katını 2 duvar, 1 mahal, 1 mahal ayırıcı, 1 kolon, 1 aks, 1 merdiven ve 1 rampa anıyor")
    c = refused("her türün andığı kat kaldırılmaz: sayılar ve kimlikler", {"arch": {"levels": every_levels[1:]}},
                "level_in_use", words, "arch.levels", every)
    c["steps"][0]["result"]["error"]["uids"] = [f"$uidOf:{i}" for i in ids]
    c["steps"][0]["expect"].pop("ids")
    c["steps"][0]["expect"]["settings"] = {"arch": {"levels": every_levels}}
    out.append(c)
    # Both levels taken away: the first in the project's order is said.
    words, ids = in_use_words(every, every_levels[0])
    c = refused("bütün katlar kaldırılınca ilk kullanılan kat söylenir", {"arch": {}}, "level_in_use", words, "arch.levels", every)
    c["steps"][0]["result"]["error"]["uids"] = [f"$uidOf:{i}" for i in ids]
    c["steps"][0]["expect"].pop("ids")
    out.append(c)
    # 1. kat names only its own wall.
    words, ids = in_use_words(every, every_levels[1])
    assert words.startswith("“1. kat” katını 1 duvar anıyor")
    c = refused("üst katın tek duvarı", {"arch": {"levels": every_levels[:1]}}, "level_in_use", words, "arch.levels", every)
    c["steps"][0]["result"]["error"]["uids"] = [f"$uidOf:{i}" for i in ids]
    c["steps"][0]["expect"].pop("ids")
    out.append(c)
    # A sheet's map shows 1. kat; nothing else names it.
    mapped = as_v2(setup(arch={"levels": levels}))
    mapped["sheets"] = {"book": book_with_map("k1")}
    words, _ = in_use_words(mapped, k1)
    assert words == "“1. kat” katını 1 pafta haritası gösteriyor; kaldırılmadı. " + FIX_MAPS
    c = refused("pafta haritasının gösterdiği kat kaldırılmaz", {"arch": {"levels": levels[:1]}}, "level_in_use", words,
                "arch.levels", mapped)
    c["steps"][0]["expect"]["settings"] = {"arch": {"levels": levels}}
    out.append(c)
    # A wall and a map on the same level.
    both = copy.deepcopy(mapped)
    wall = {"kind": "wall", "id": 5, "layerId": "yazi", "attrs": {}, "pts": [{"x": X, "y": Y + 20}, {"x": X + 5, "y": Y + 20}],
            "width": 0.2, "justify": "center", "arch": {"level": "k1"}}
    both["entities"].append(wall)
    both["uids"].append("0192f5a1-7171-7000-8000-0000005e7005")
    words, ids = in_use_words(both, k1)
    assert words == "“1. kat” katını 1 duvar anıyor, 1 pafta haritası gösteriyor; kaldırılmadı. " + FIX_BOTH
    c = refused("hem duvar hem pafta haritası", {"arch": {"levels": levels[:1]}}, "level_in_use", words, "arch.levels", both)
    c["steps"][0]["result"]["error"]["uids"] = ["$uidOf:5"]
    c["steps"][0]["expect"]["ids"] = IDS + [5]
    c["steps"][0]["expect"]["settings"] = {"arch": {"levels": levels}}
    out.append(c)
    # A Kot bound to 1. kat (docs/adr/0721 §2) keeps it; a wall on Zemin kat says nothing of it.
    kotted = setup(arch={"levels": levels})
    kotted["entities"].append(dict(walled["entities"][-1], arch={"level": "zemin"}))
    kotted["entities"].append({"kind": "dimension", "id": 6, "layerId": "yazi", "attrs": {}, "a": {"x": X, "y": Y + 30},
                               "b": {"x": X + 2, "y": Y + 32}, "offset": 0, "height": 1, "style": "elevation", "za": 2.98,
                               "level": {"id": "k1", "offset": -0.02}})
    words, ids = in_use_words(kotted, k1)
    assert words == "“1. kat” katını 1 kot işareti anıyor; kaldırılmadı. " + FIX_OBJECTS
    c = refused("kata bağlı kot işaretinin katı kaldırılmaz", {"arch": {"levels": levels[:1]}}, "level_in_use", words,
                "arch.levels", kotted)
    c["steps"][0]["result"]["error"]["uids"] = [f"$uidOf:{i}" for i in ids]
    c["steps"][0]["expect"]["ids"] = IDS + [5, 6]
    c["steps"][0]["expect"]["settings"] = {"arch": {"levels": levels}}
    out.append(c)
    # The map's level kept, another taken away: written.
    out.append({"name": "haritanın katı kaldıkça öbür kat kaldırılır", "setup": mapped,
                "steps": [step("execute", {"arch": {"levels": levels[1:]}}, done(["arch"]),
                               {**FLAGS, "settings": {"arch": {"levels": levels[1:]}}})]})
    # A level the project lacked before is not looked at: a wall on “eski” does not keep other changes out.
    stale = setup(arch={"levels": levels[:1]})
    stale["entities"].append(dict(walled["entities"][-1], arch={"level": "eski"}))
    out.append({"name": "projede zaten olmayan katı anan duvar öbür değişikliği engellemez", "setup": stale,
                "steps": [step("execute", {"arch": {"levels": levels}}, done(["arch"]),
                               {**FLAGS, "ids": IDS + [5], "settings": {"arch": {"levels": levels}}})]})
    # The level and view a drawing opens with (ADR 0720 §1): a level's id or null (Katsız); Bütün katlar is no
    # setting; one of the project's levels, and none without levels.
    on = {"levels": levels, "activeLevel": "k1", "levelView": "below"}
    out.append({"name": "Etkin kat ve kat görünümü: açılışın katı ve görünümü yazılır",
                "steps": [step("execute", {"arch": on}, done(["arch"]), {**FLAGS, "settings": {"arch": on}}),
                          step("execute", {"arch": {"levels": levels, "activeLevel": None, "levelView": "active"}}, done(["arch"]),
                               {"settings": {"arch": {"levels": levels, "activeLevel": None, "levelView": "active"}},
                                "revision": "changed"}),
                          step("execute", {"arch": {"levels": levels}}, done(["arch"]),
                               {"settings": {"arch": {"levels": levels}}, "revision": "changed"})]})
    out.append({"name": "Bütün katlar yazılmaz: kat görünümü yokmuş gibi",
                "steps": [step("execute", {"arch": {"levels": levels, "activeLevel": "zemin", "levelView": "all"}}, done(["arch"]),
                               {**FLAGS, "settings": {"arch": {"levels": levels, "activeLevel": "zemin"}}})]})
    out.append(refused("etkin kat projede yok", {"arch": {"levels": levels, "activeLevel": "yok"}}, "invalid_arch",
                       "Etkin kat olarak verilen “yok” kimlikli kat projede yok; var olan bir katın kimliğini verin ya da null (Katsız) yazın.",
                       "arch.activeLevel"))
    out.append(refused("kaldırılan kat etkin kat olarak kalamaz", {"arch": {"levels": levels[1:], "activeLevel": "zemin"}}, "invalid_arch",
                       "Etkin kat olarak verilen “zemin” kimlikli kat projede yok; var olan bir katın kimliğini verin ya da null (Katsız) yazın.",
                       "arch.activeLevel"))
    out.append(refused("katsız projenin kat görünümü olmaz", {"arch": {"levelView": "active"}}, "invalid_arch",
                       "Katı olmayan projenin etkin katı ve kat görünümü olmaz; önce kat ekleyin.", "arch.levelView"))
    out.append(refused("katsız projede Katsız da yazılmaz", {"arch": {"activeLevel": None, "detail": "standard"}}, "invalid_arch",
                       "Katı olmayan projenin etkin katı ve kat görünümü olmaz; önce kat ekleyin.", "arch.activeLevel"))
    return out


def room_choice_cases():
    """Optional project suggestions: settings history stays unchanged; no source is renamed."""
    levels=[dict(id='zemin',name='Zemin kat',elevation=0.0),dict(id='kat-1',name='1. kat',elevation=3.0)]
    base=setup(workspace='cad',srid=0,secondSrid=None)
    profiles=[dict(spaceNames=['Mutfak','mutfak','Çalışma'],spaceUsages=['Konut','Ofis'],levels=levels,
                   spaceNumberPrefixes={'zemin':'Z-','kat-1':'01-'}),dict(spaceNames=['Mutfak','mutfak']),
              dict(spaceUsages=['Konut']),dict(levels=levels,spaceNumberPrefixes={'zemin':'ç'*253}),
              dict(spaceNames=['ç'*256]),dict(spaceUsages=[f'K{i}'for i in range(256)])]
    out=[]
    for i,arch in enumerate(profiles):
        unchanged={**UNCHANGED,'entities':E,'settings':{'arch':None}}
        out.append(dict(name=f'Mahal listeleri: sıra/tam metin/sınır, profil {i}',setup=copy.deepcopy(base),steps=[
            step('validate',{'arch':arch},{'status':'completed','output':None,'warnings':[]},unchanged),
            step('plan',{'arch':arch},done(['arch']),unchanged),
            step('execute',{'arch':arch},done(['arch']),{**FLAGS,'entities':E,'settings':{'arch':arch}}),
            step('execute',{'arch':arch},done([]),{'revision':'same','canUndo':False,'entities':E}),
            step('execute',{'arch':{}},done(['arch']),{'revision':'changed','canUndo':False,'entities':E,'settings':{'arch':None}})]))
    out.append(dict(name='Mahal listeleri: boş tercihler eski kanonik kaydı değiştirmez',setup=copy.deepcopy(base),steps=[
        step('execute',{'arch':{'spaceNames':[],'spaceUsages':[],'spaceNumberPrefixes':{}}},done([]),
             {**UNCHANGED,'entities':E,'settings':{'arch':None}})]))
    def refuse(name,arch,path,message):
        c=refused('Mahal listeleri: '+name,{'arch':arch},'invalid_arch',message,'arch.'+path,copy.deepcopy(base))
        original=c['steps'][0];original['expect']['entities']=E
        c['steps']=[dict(copy.deepcopy(original),op=op)for op in ['validate','plan','execute']];out.append(c)
    for key,label in [('spaceNames','Mahal adı'),('spaceUsages','Mahal kullanımı')]:
        for name,values in [('boş',['']),('boşluk',[' ']),('baş',[' A']),('son',['A ']),('kontrol',['A\nB']),('uzun',['ç'*257])]:
            refuse(key+' '+name,{key:values},key+'[0]',f'{label} boş, başında/sonunda boşluklu veya kontrol karakterli olamaz; en çok 256 karakterdir.')
        refuse(key+' yinelenen',{key:['A','A']},key+'[1]',f'“A” {label.lower()} listesinde iki kez var.')
        refuse(key+' fazla',{key:[f'N{i}'for i in range(257)]},key,f'{label} listesi en çok 256 değer içerir.')
    refuse('bilinmeyen kat',dict(levels=levels,spaceNumberPrefixes={'yok':'Z-'}),'spaceNumberPrefixes.yok','Mahal numara önekinin “yok” katı projede yok.')
    for name,prefix in [('boş',''),('baş',' Z-'),('son','Z- '),('kontrol','Z\t'),('uzun','ç'*254)]:
        refuse('önek '+name,dict(levels=levels,spaceNumberPrefixes={'zemin':prefix}),'spaceNumberPrefixes.zemin',
               'Mahal numara öneki boş, başında/sonunda boşluklu veya kontrol karakterli olamaz; en çok 253 karakterdir.')
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.settings.set",
        "commandVersion": 1,
        "title": "Proje ayarları",
        "note": "Written by scripts/fixtures/settings_set_command_cases.py from the contract's rules and docs/adr/0165, 0167, 0168, 0205, 0248 §5, 0339 §3, 0720, 0801; do not edit by hand.",
        "setup": setup(),
        "cases": cases()+room_choice_cases(),
    }


def main() -> int:
    doc = build()
    text_ = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text_:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.settings.set durumları güncel: {len(doc['cases'])} durum.")
        return 0
    OUT.write_text(text_, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
