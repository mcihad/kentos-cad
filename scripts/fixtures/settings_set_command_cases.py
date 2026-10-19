"""Writes fixtures/commands/v1/cad.settings.set.json: the shared cases of `cad.settings.set` v1 (docs/adr/0248 §5).

The expectations are built here from the contract's rules (crates/shared/contracts/src/cad_settings.rs: the keys, the
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
    out.append({"name": "Ad ve tür: proje ayarıdır, geri alma adımı değildir",
                "steps": [step("execute", {"name": "  Mimari proje ", "projectType": "cad"}, done(["name", "projectType"]),
                               {**FLAGS, "name": "Mimari proje", "settings": {"workspace": "cad"}})]})
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
                     "“zemin” katını duvarlar anıyor; kaldırılmadı. Önce duvarların katını değiştirin ya da katı bırakın.", "arch.levels", walled)
    in_use["steps"][0]["expect"]["ids"] = IDS + [5]
    in_use["steps"][0]["expect"]["settings"] = {"arch": {"levels": levels}}
    out.append(in_use)
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


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.settings.set",
        "commandVersion": 1,
        "title": "Proje ayarları",
        "note": "Written by scripts/fixtures/settings_set_command_cases.py from the contract's rules and docs/adr/0165, 0167, 0168, 0205, 0248 §5, 0339 §3; do not edit by hand.",
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
        print(f"cad.settings.set durumları güncel: {len(doc['cases'])} durum.")
        return 0
    OUT.write_text(text_, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
