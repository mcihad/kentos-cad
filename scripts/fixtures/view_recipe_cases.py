"""The shared cases of cad.views.edit's recipe (docs/adr/0774 §1, §2): a named view's architectural recipe — the level
(and the one under it faint), phase, layer state, the categories left out and the scale a sheet's map linked to the view
draws with.

    python3 scripts/fixtures/view_recipe_cases.py           # writes the file
    python3 scripts/fixtures/view_recipe_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.views.edit.recipe.json. The rule is written here from the ADR, without KentOS code:

- `recipe` sets the view's recipe (its version the old one's plus one, 1 for a first; the input's version is ignored) or,
  absent, takes it away; the same recipe again changes nothing (its version stays);
- a recipe holds: not empty (a level, phase, layer state, category, scale, wall detail (docs/adr/0781) or crop; `crop`
  false is no crop), a level id and a state name not blank, a
  category once, a scale in 1–100 000 (`invalid_recipe`, path `edit.recipe`, before the name is checked); then the view
  must exist (`unknown_view`), its level be one of the project's (`level_not_found`, `edit.recipe.level.id`) and its
  state one of the project's layer states by name (`state_not_found`, `edit.recipe.layerState`);
- a view whose recipe changes keeps a stable id: an empty one becomes the one its name gives (`v-` and the name's
  FNV-1a), so the maps linked to it keep their link; a view saved over one keeps its recipe; renaming keeps it;
- a project setting: no undo step, no object changes.
"""
import json
import sys

E0, N0 = 487000, 4420000
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def P(x, y):
    return {"x": x, "y": y}


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": style, "children": []}


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "duvar", "attrs": {}, "pts": [P(E0, N0), P(E0 + 8, N0), P(E0 + 8, N0 + 6), P(E0, N0 + 6)]},
    {"kind": "line", "id": 2, "layerId": "olcu", "attrs": {}, "a": P(E0, N0 - 1), "b": P(E0 + 8, N0 - 1)},
]
IDS = [e["id"] for e in ENTITIES]
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]
STATES = [{"id": "s1", "name": "Sunum", "nodes": [{"node": "olcu", "visible": False, "style": style}]}]
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
            "workspace": "cad", "drawingFont": "barlow", "arch": {"levels": LEVELS}, "layerStates": STATES}
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Görünüş tarifi", "settings": SETTINGS, "origin": P(E0, N0),
    "layers": [layer("duvar", "Duvar"), layer("olcu", "Ölçü")], "activeLayer": "duvar", "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}
CATEGORIES = ["dimensions", "tags", "texts", "hatches", "blocks", "spaces", "grids", "viewmarks"]
LABELS = {"dimensions": "Ölçüler", "tags": "Etiketler", "texts": "Yazılar", "hatches": "Taramalar",
          "blocks": "Bloklar ve donatı", "spaces": "Mahaller", "grids": "Akslar", "viewmarks": "Görünüm işaretleri"}
HERE = P(E0 + 4, N0 + 3)


def copy(v):
    return json.loads(json.dumps(v))


def fnv(name):
    h = 0x811C9DC5
    for b in name.encode():
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return f"v-{h:08x}"


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def problem(r):
    """The recipe's own rule (no project): the words of the first thing wrong, or none."""
    if not any(k in r for k in ("level", "phase", "layerState", "scale", "detail")) and not r.get("hidden") and not r.get("crop"):
        return "Görünüş tarifi boş: kat, evre, katman durumu, gizli tür, ölçek, duvar ayrıntısı ya da kırpmadan birini verin."
    lvl = r.get("level")
    if lvl is not None and not lvl["id"].strip():
        return "Görünüş tarifinin katı bir kat kimliği olmalı."
    st = r.get("layerState")
    if st is not None and not st.strip():
        return "Görünüş tarifinin katman durumu bir durumun adı olmalı."
    seen = []
    for c in r.get("hidden", []):
        if c in seen:
            return f"“{LABELS[c]}” gizlenenlerde iki kez var."
        seen.append(c)
    s = r.get("scale")
    if s is not None and not 1 <= s <= 100_000:
        return "Görünüş tarifinin ölçeği 1:1 ile 1:100 000 arasında olur."
    return None


def stored(r, version):
    """The recipe as the view keeps it: its version first, no field at its default."""
    out = {"version": version}
    for k in ("level", "phase", "layerState", "hidden", "scale", "detail", "crop"):
        if k == "hidden" and not r.get("hidden"):
            continue
        if k == "crop" and not r.get("crop"):
            continue
        if k == "level" and "level" in r:
            out["level"] = {"id": r["level"]["id"], **({"below": True} if r["level"].get("below") else {})}
            continue
        if k in r:
            out[k] = r[k]
    return out


class Project:
    def __init__(self):
        self.views = []

    def run(self, edit):
        """The rule's answer to `edit` and its expectation."""
        old = copy(self.views)
        step = {"op": "execute", "input": {"edit": edit}}
        kind = edit["kind"]
        warnings = []
        kept = None
        if kind == "save":
            name = edit["name"]
            v = {"name": name, "center": edit["center"], "width": edit["width"], "height": edit["height"], "twist": 0.0}
            at = next((i for i, x in enumerate(self.views) if x["name"] == name), None)
            if at is None:
                self.views.append(v)
                kept = v
            else:
                was = self.views[at]
                v = {**({"id": was["id"]} if "id" in was else {}), **v, **({"recipe": was["recipe"]} if "recipe" in was else {})}
                self.views[at] = v
                kept = v
                warnings.append({"code": "view_replaced", "message": f"“{name}” adlı görünümün yerine yenisi kaydedildi.",
                                 "path": "edit.name"})
        elif kind == "rename":
            at = next(i for i, x in enumerate(self.views) if x["name"] == edit["name"])
            if "id" not in self.views[at]:
                self.views[at] = {"id": fnv(edit["name"]), **self.views[at]}
            self.views[at] = {**self.views[at], "name": edit["to"]}
            kept = self.views[at]
        elif kind == "delete":
            self.views = [x for x in self.views if x["name"] != edit["name"]]
        elif kind == "recipe":
            r = edit.get("recipe")
            if r is not None:
                why = problem(r)
                if why:
                    return self.refused(step, failed("invalid_recipe", f"{why} Tarifi düzeltip yeniden verin.", "edit.recipe"))
            at = next((i for i, x in enumerate(self.views) if x["name"] == edit["name"]), None)
            if at is None:
                return self.refused(step, failed(
                    "unknown_view",
                    f"“{edit['name']}” adlı bir görünüm yok. Adlı görünümlerden birini seçin ya da geçerli görünümü bu adla kaydedin.",
                    "edit.name"))
            if r is not None:
                lvl = r.get("level")
                if lvl and not any(l["id"] == lvl["id"] for l in LEVELS):
                    return self.refused(step, failed(
                        "level_not_found",
                        f"Projede “{lvl['id']}” kimlikte kat yok. Proje ayarları › Mimari'deki katlardan birinin kimliğini verin.",
                        "edit.recipe.level.id"))
                st = r.get("layerState")
                if st is not None and not any(s["name"].strip() == st.strip() for s in STATES):
                    return self.refused(step, failed(
                        "state_not_found",
                        f"Projede “{st}” adlı katman durumu yok. Katmanlar › Katman durumları'ndan birinin adını verin.",
                        "edit.recipe.layerState"))
            v = self.views[at]
            was = v.get("recipe")
            new = None if r is None else stored(r, (was or {}).get("version", 0) + 1)
            same = (was is None and new is None) or (was is not None and new is not None and
                                                     {k: x for k, x in was.items() if k != "version"} ==
                                                     {k: x for k, x in new.items() if k != "version"})
            if not same:
                if "id" not in v:
                    v = {"id": fnv(v["name"]), **v}
                v = {k: x for k, x in v.items() if k != "recipe"}
                if new is not None:
                    v["recipe"] = new
                self.views[at] = v
            kept = self.views[at]
        changed = self.views != old
        output = {"views": copy(self.views), "changed": changed, "revision": "$current"}
        if kept is not None:
            output["view"] = copy(kept)
        step["result"] = {"status": "completed", "output": output, "warnings": warnings}
        expect = {"ids": IDS, "settings": {"namedViews": copy(self.views) or None}, "canUndo": False, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        step["expect"] = expect
        return step

    def refused(self, step, result):
        step["result"] = result
        step["expect"] = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same"}
        return step


def save(name, w=10.0, h=8.0):
    return {"kind": "save", "name": name, "center": HERE, "width": w, "height": h}


def recipe(name, r):
    out = {"kind": "recipe", "name": name}
    if r is not None:
        out["recipe"] = r
    return out


cases = []


def case(name, edits):
    p = Project()
    cases.append({"name": name, "steps": [p.run(e) for e in edits]})


APP = {"level": {"id": "zemin"}, "scale": 50}
FURN = {"level": {"id": "zemin"}, "scale": 100, "hidden": ["dimensions", "tags"]}
case("Tarif: kat ve ölçek; sürüm 1, görünüm adından kalıcı kimliğini alır", [save("Uygulama"), recipe("Uygulama", APP)])
case("Tarif değişir: sürüm 2", [save("Mobilya"), recipe("Mobilya", APP), recipe("Mobilya", FURN)])
case("Aynı tarif yeniden: değişiklik yok, sürüm kalır", [save("A"), recipe("A", APP), recipe("A", APP)])
case("Tarif kaldırılır; kimlik kalır", [save("A"), recipe("A", APP), recipe("A", None)])
case("Tarifsiz görünümden tarifi kaldırmak bir şey değiştirmez", [save("A"), recipe("A", None)])
case("Üzerine kaydedilen görünüm tarifini korur", [save("A"), recipe("A", FURN), save("A", 12.0, 9.0)])
case("Yeniden adlandırılan görünüm tarifini ve kimliğini korur", [save("A"), recipe("A", APP), {"kind": "rename", "name": "A", "to": "B"}])
case("Tarifli görünüm silinir", [save("A"), recipe("A", APP), {"kind": "delete", "name": "A"}])
case("Girdinin sürümü yok sayılır", [save("A"), recipe("A", {**APP, "version": 7})])
case("Yalnız evre: yıkım planı", [save("Yıkım"), recipe("Yıkım", {"phase": "demolition"})])
case("Alt kat soluk", [save("Kat 1"), recipe("Kat 1", {"level": {"id": "kat-1", "below": True}, "scale": 100})])
case("Katman durumu adıyla", [save("Sunum"), recipe("Sunum", {"layerState": "Sunum", "scale": 200})])
case("Sekiz tür gizli, verildiği sırayla", [save("Boş"), recipe("Boş", {"hidden": list(reversed(CATEGORIES))})])
case("İki görünüm: biri tarifli, öbürü dokunulmaz", [save("A"), save("B"), recipe("B", FURN)])
case("Boş tarif reddedilir", [save("A"), recipe("A", {})])
case("Aynı tür iki kez reddedilir", [save("A"), recipe("A", {"hidden": ["tags", "dimensions", "tags"]})])
case("Ölçek 0 reddedilir", [save("A"), recipe("A", {"scale": 0})])
case("Ölçek 1:100 000'den büyük reddedilir", [save("A"), recipe("A", {"scale": 100_001})])
case("Ölçek sınırları: 1:1 ve 1:100 000 kabul", [save("A"), recipe("A", {"scale": 1}), recipe("A", {"scale": 100_000})])
case("Boş kat kimliği reddedilir", [save("A"), recipe("A", {"level": {"id": "  "}})])
case("Boş katman durumu reddedilir", [save("A"), recipe("A", {"layerState": " "})])
case("Projede olmayan kat reddedilir", [save("A"), recipe("A", {"level": {"id": "çatı"}})])
case("Projede olmayan katman durumu reddedilir", [save("A"), recipe("A", {"layerState": "Yok"})])
case("Olmayan görünüme tarif reddedilir", [recipe("Yok", APP)])
case("Bozuk tarif, olmayan görünümden önce söylenir", [recipe("Yok", {})])
# docs/adr/0781: the walls' detail and the crop.
case("Yalnız duvar ayrıntısı: ayrıntılı", [save("Detay"), recipe("Detay", {"detail": "detailed"})])
case("Yalnız kırpma", [save("Kırpık"), recipe("Kırpık", {"crop": True})])
case("Kırpma yok (false) tek başına boş tariftir", [save("A"), recipe("A", {"crop": False})])
case("Ölçek, şematik duvar ve kırpma birlikte", [save("A"), recipe("A", {**APP, "detail": "schematic", "crop": True})])
case("Ayrıntı değişir: sürüm 2", [save("A"), recipe("A", {**APP, "detail": "standard"}), recipe("A", {**APP, "detail": "detailed"})])
case("Kırpma kalkar, ayrıntı kalır: sürüm 2", [save("A"), recipe("A", {"detail": "detailed", "crop": True}),
                                                 recipe("A", {"detail": "detailed"})])
case("Kırpma false yazılmaz: aynı tarif, sürüm kalır", [save("A"), recipe("A", APP), recipe("A", {**APP, "crop": False})])
case("Ayrıntılı ve kırpık tarif yeniden adlandırmada kalır",
     [save("A"), recipe("A", {"detail": "detailed", "crop": True}), {"kind": "rename", "name": "A", "to": "B"}])


def write():
    doc = {
        "format": "kentos.command-cases", "version": 1, "command": "cad.views.edit", "commandVersion": 1,
        "title": "Görünüş tarifi: adlı görünümün kat, evre, katman durumu, gizlenen türler, ölçeği, duvar ayrıntısı ve kırpması (ADR 0774, 0781)",
        "note": "Kural ve beklenenler scripts/fixtures/view_recipe_cases.py'de ADR 0774 ve 0781'dan, KentOS kodu olmadan yazılmıştır.",
        "setup": SETUP, "cases": cases,
    }
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = "fixtures/commands/v1/cad.views.edit.recipe.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} bağımsız başvurudan farklı.")
        print(f"cad.views.edit (tarif): {len(cases)} durum başvuruyla aynı.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"{path}: {len(cases)} durum yazıldı.")


write()
