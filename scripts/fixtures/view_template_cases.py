"""The shared cases of view templates and pen sets (docs/adr/0783): cad.views.edit's `template` and `templateRename`, a
recipe's `template`, `pens` and `mono`, and cad.sheets.set drawing a linked map by its view's template.

    python3 scripts/fixtures/view_template_cases.py           # writes the files
    python3 scripts/fixtures/view_template_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.views.edit.template.json and fixtures/commands/v1/cad.sheets.set.template.json. The rule is
written here from the ADR, without KentOS code:

- a pen (`pens`) names a category once (walls, openings, columns, stairs, slabs, spaces, dimensions, tags, texts, hatches,
  blocks, grids, viewmarks, others) and gives a paper weight (0–5 mm) or a colour (`#rrggbb`, lower case kept as given)
  or both; `mono` (true only) draws every line black and hatches grey; pens and mono count as a recipe's content;
- a template (`template` edit) is a recipe kept under a name in the project's `viewTemplates`: its id `t-` and the name's
  FNV-1a, the next free `-2`, `-3` … when taken; its recipe holds no level, crop or template (`invalid_recipe`, path
  `edit.recipe`); its version is the old one's plus one, the same recipe again changes nothing; at most 50
  (`too_many_templates`); no recipe removes it, and the views bound to it keep its fields as their own recipe (their
  version one more, the template gone) with the warning `template_detached`;
- `templateRename` renames it keeping its id (`unknown_template`, `template_exists`);
- a view's recipe bound to a template (`template`: its id or its name, kept as the id) holds only its level and crop
  besides (`invalid_recipe`), the template must exist (`unknown_template`, `edit.recipe.template`);
- a linked map draws the view's recipe with its template's fields: the template's phase, layer state, hidden categories,
  scale, detail, pens and mono, the view's level and crop; a template changed changes every map bound to it.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sheet_set_cases as ssc  # noqa: E402
import view_recipe_sheet_cases as vrs  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
E0, N0 = 487000, 4420000
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
PENS = ["walls", "openings", "columns", "stairs", "slabs", "spaces", "dimensions", "tags", "texts", "hatches", "blocks",
        "grids", "viewmarks", "others"]
PEN_LABELS = {"walls": "Duvarlar", "openings": "Kapı ve pencereler", "columns": "Kolonlar", "stairs": "Merdiven ve rampalar",
              "slabs": "Döşemeler", "spaces": "Mahaller", "dimensions": "Ölçüler", "tags": "Etiketler", "texts": "Yazılar",
              "hatches": "Taramalar", "blocks": "Bloklar ve donatı", "grids": "Akslar", "viewmarks": "Görünüm işaretleri",
              "others": "Öbür çizgiler"}
LABELS = {"dimensions": "Ölçüler", "tags": "Etiketler", "texts": "Yazılar", "hatches": "Taramalar",
          "blocks": "Bloklar ve donatı", "spaces": "Mahaller", "grids": "Akslar", "viewmarks": "Görünüm işaretleri"}


def P(x, y):
    return {"x": x, "y": y}


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": style, "children": []}


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "duvar", "attrs": {}, "pts": [P(E0, N0), P(E0 + 8, N0), P(E0 + 8, N0 + 6), P(E0, N0 + 6)]},
]
IDS = [1]
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]
STATES = [{"id": "s1", "name": "Sunum", "nodes": [{"node": "duvar", "visible": True, "style": style}]}]
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
            "workspace": "cad", "drawingFont": "barlow", "arch": {"levels": LEVELS}, "layerStates": STATES}
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Görünüş şablonu", "settings": SETTINGS, "origin": P(E0, N0),
    "layers": [layer("duvar", "Duvar")], "activeLayer": "duvar", "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}
HERE = P(E0 + 4, N0 + 3)
MOST_TEMPLATES = 50


def copy(v):
    return json.loads(json.dumps(v))


def fnv(prefix, name):
    h = 0x811C9DC5
    for b in name.encode():
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return f"{prefix}-{h:08x}"


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


CONTENT = ("level", "phase", "layerState", "scale", "detail", "template")


def pen_problem(pens):
    seen = []
    for p in pens:
        if p["of"] in seen:
            return f"“{PEN_LABELS[p['of']]}” kalemlerde iki kez var."
        seen.append(p["of"])
        if "weight" not in p and "color" not in p:
            return f"“{PEN_LABELS[p['of']]}” kaleminin kalınlığı ya da rengi olmalı."
        w = p.get("weight")
        if w is not None and not 0 <= w <= 5:
            return "Kalemin kalınlığı 0 ile 5 mm arasında olur."
        c = p.get("color")
        if c is not None and not (len(c) == 7 and c[0] == "#" and all(ch in "0123456789abcdefABCDEF" for ch in c[1:])):
            return "Kalemin rengi #rrggbb biçiminde olur."
    return None


def problem(r):
    """The recipe's own rule (docs/adr/0774, 0781, 0783): the words of the first thing wrong, or none."""
    if not any(k in r for k in CONTENT) and not r.get("hidden") and not r.get("crop") and not r.get("pens") and not r.get("mono"):
        return "Görünüş tarifi boş: kat, evre, katman durumu, gizli tür, ölçek, duvar ayrıntısı, kırpma, kalem, şablon ya da siyah beyazdan birini verin."
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
    why = pen_problem(r.get("pens", []))
    if why:
        return why
    t = r.get("template")
    if t is not None:
        if not t.strip():
            return "Görünüş tarifinin şablonu bir şablonun kimliği ya da adı olmalı."
        if any(k in r for k in ("phase", "layerState", "scale", "detail")) or r.get("hidden") or r.get("pens") or r.get("mono"):
            return "Şablona bağlı tarifte yalnız kat ve kırpma görünümündür; öbürleri şablondan gelir."
    return None


def template_problem(r):
    why = problem(r)
    if why:
        return why
    if "level" in r or r.get("crop") or "template" in r:
        return "Görünüş şablonu kat, kırpma ya da başka şablon taşımaz: bunlar görünümündür."
    return None


ORDER = ("level", "phase", "layerState", "hidden", "scale", "detail", "crop", "pens", "mono", "template")


def stored(r, version):
    """The recipe as kept: its version first, no field at its default, in the contract's order."""
    out = {"version": version}
    for k in ORDER:
        if k in ("hidden", "pens") and not r.get(k):
            continue
        if k in ("crop", "mono") and not r.get(k):
            continue
        if k == "level" and "level" in r:
            out["level"] = {"id": r["level"]["id"], **({"below": True} if r["level"].get("below") else {})}
            continue
        if k == "pens" and r.get("pens"):
            out["pens"] = [{"of": p["of"], **({"weight": p["weight"]} if "weight" in p else {}),
                            **({"color": p["color"]} if "color" in p else {})} for p in r["pens"]]
            continue
        if k in r:
            out[k] = r[k]
    return out


def without_version(r):
    return {k: v for k, v in r.items() if k != "version"}


class Project:
    def __init__(self):
        self.views = []
        self.templates = []

    def expect_settings(self):
        return {"namedViews": copy(self.views) or None, "viewTemplates": copy(self.templates) or None}

    def run(self, edit):
        old = (copy(self.views), copy(self.templates))
        step = {"op": "execute", "input": {"edit": edit}}
        kind = edit["kind"]
        warnings = []
        kept_view = None
        kept_template = None
        if kind == "save":
            v = {"name": edit["name"], "center": edit["center"], "width": edit["width"], "height": edit["height"], "twist": 0.0}
            self.views.append(v)
            kept_view = v
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
            if r is not None and "template" in r:
                key = r["template"].strip()
                t = next((t for t in self.templates if t["id"] == key), None) or next(
                    (t for t in self.templates if t["name"] == key), None)
                if t is None:
                    return self.refused(step, failed(
                        "unknown_template",
                        f"“{key}” adlı ya da kimlikli bir görünüş şablonu yok. Görünümler › Şablonlar'dan birini seçin ya da önce şablonu kaydedin.",
                        "edit.recipe.template"))
                r = {**r, "template": t["id"]}
            v = self.views[at]
            was = v.get("recipe")
            new = None if r is None else stored(r, (was or {}).get("version", 0) + 1)
            same = (was is None and new is None) or (was is not None and new is not None
                                                     and without_version(was) == without_version(new))
            if not same:
                if "id" not in v:
                    v = {"id": fnv("v", v["name"]), **v}
                v = {k: x for k, x in v.items() if k != "recipe"}
                if new is not None:
                    v["recipe"] = new
                self.views[at] = v
            kept_view = self.views[at]
        elif kind == "template":
            name = edit["name"].strip()
            r = edit.get("recipe")
            if r is not None:
                why = template_problem(r)
                if why:
                    return self.refused(step, failed("invalid_recipe", f"{why} Şablonu düzeltip yeniden verin.", "edit.recipe"))
            at = next((i for i, t in enumerate(self.templates) if t["name"] == name), None)
            if r is None:
                if at is None:
                    return self.refused(step, self.unknown_template(name, "edit.name"))
                gone = self.templates.pop(at)
                detached = []
                for i, v in enumerate(self.views):
                    rv = v.get("recipe")
                    if rv and rv.get("template") == gone["id"]:
                        own = {**without_version(gone["recipe"]),
                               **({"level": rv["level"]} if "level" in rv else {}), **({"crop": True} if rv.get("crop") else {})}
                        self.views[i] = {**v, "recipe": stored(own, rv["version"] + 1)}
                        detached.append(v["name"])
                if detached:
                    names = ", ".join(f"“{n}”" for n in detached)
                    warnings.append({"code": "template_detached",
                                     "message": f"Şablon kaldırıldı; {names} görünümü tarifini kendi tarifi olarak tutar.",
                                     "path": "edit.name"})
            else:
                if at is None:
                    if len(self.templates) >= MOST_TEMPLATES:
                        return self.refused(step, failed(
                            "too_many_templates",
                            "Projede en çok 50 görünüş şablonu olur. Kullanmadığınız birini kaldırın ya da var olanın üzerine kaydedin.",
                            "edit.name"))
                    own = fnv("t", name)
                    taken = {t["id"] for t in self.templates}
                    tid = own
                    k = 2
                    while tid in taken:
                        tid = f"{own}-{k}"
                        k += 1
                    t = {"id": tid, "name": name, "recipe": stored(r, 1)}
                    self.templates.append(t)
                    kept_template = t
                else:
                    t = self.templates[at]
                    new = stored(r, t["recipe"]["version"] + 1)
                    if without_version(new) != without_version(t["recipe"]):
                        self.templates[at] = {**t, "recipe": new}
                    kept_template = self.templates[at]
        elif kind == "templateRename":
            name, to = edit["name"].strip(), edit["to"].strip()
            at = next((i for i, t in enumerate(self.templates) if t["name"] == name), None)
            if at is None:
                return self.refused(step, self.unknown_template(name, "edit.name"))
            if to != name and any(t["name"] == to for t in self.templates):
                return self.refused(step, failed(
                    "template_exists",
                    f"“{to}” adlı başka bir görünüş şablonu var. Başka bir ad yazın ya da önce onu kaldırın.",
                    "edit.to"))
            self.templates[at] = {**self.templates[at], "name": to}
            kept_template = self.templates[at]
        changed = (self.views, self.templates) != old
        output = {"views": copy(self.views)}
        if self.templates:
            output["templates"] = copy(self.templates)
        if kept_view is not None:
            output["view"] = copy(kept_view)
        if kept_template is not None:
            output["template"] = copy(kept_template)
        output.update({"changed": changed, "revision": "$current"})
        step["result"] = {"status": "completed", "output": output, "warnings": warnings}
        expect = {"ids": IDS, "settings": self.expect_settings(), "canUndo": False, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        step["expect"] = expect
        return step

    def unknown_template(self, name, path):
        return failed("unknown_template",
                      f"“{name}” adlı ya da kimlikli bir görünüş şablonu yok. Görünümler › Şablonlar'dan birini seçin ya da önce şablonu kaydedin.",
                      path)

    def refused(self, step, result):
        step["result"] = result
        step["expect"] = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same"}
        return step


def save(name):
    return {"kind": "save", "name": name, "center": HERE, "width": 10.0, "height": 8.0}


def recipe(name, r):
    out = {"kind": "recipe", "name": name}
    if r is not None:
        out["recipe"] = r
    return out


def template(name, r):
    out = {"kind": "template", "name": name}
    if r is not None:
        out["recipe"] = r
    return out


cases = []


def case(name, edits, p=None):
    p = p or Project()
    cases.append({"name": name, "steps": [p.run(e) for e in edits]})


UYG = {"scale": 50, "detail": "detailed", "pens": [{"of": "walls", "weight": 0.5}, {"of": "dimensions", "weight": 0.13}]}
MOB = {"scale": 100, "hidden": ["dimensions", "tags"], "mono": True}
case("Şablon kaydedilir: kimliği adından, sürümü 1", [template("Uygulama 1:50", UYG)])
case("Şablon değişir: sürüm 2, kimlik kalır", [template("Uygulama 1:50", UYG), template("Uygulama 1:50", {**UYG, "scale": 20})])
case("Aynı şablon yeniden: değişiklik yok, sürüm kalır", [template("A", UYG), template("A", UYG)])
case("Görünüm şablona bağlanır: kat görünümün, öbürü şablonun",
     [save("Zemin"), template("Uygulama", UYG), recipe("Zemin", {"level": {"id": "zemin"}, "template": fnv("t", "Uygulama")})])
case("Şablon adıyla verilir, kimliğiyle tutulur",
     [save("Zemin"), template("Mobilya", MOB), recipe("Zemin", {"level": {"id": "zemin"}, "template": "Mobilya"})])
case("Şablona bağlı tarifte kırpma da görünümündür",
     [save("Banyo"), template("Detay", {"scale": 20, "detail": "detailed"}),
      recipe("Banyo", {"level": {"id": "zemin"}, "crop": True, "template": "Detay"})])
case("Yalnız şablon: katsız bağlı tarif",
     [save("A"), template("Sunum", {"layerState": "Sunum", "mono": True}), recipe("A", {"template": "Sunum"})])
case("Şablonun değişmesi görünümün tarifini değiştirmez: sürümü kalır",
     [save("A"), template("T", UYG), recipe("A", {"template": "T"}), template("T", {**UYG, "scale": 100})])
case("Şablon kaldırılınca bağlı görünümler tarifini kendi tarifi olarak tutar",
     [save("A"), save("B"), save("C"), template("T", UYG),
      recipe("A", {"level": {"id": "zemin"}, "template": "T"}), recipe("B", {"level": {"id": "kat-1", "below": True}, "crop": True, "template": "T"}),
      recipe("C", {"scale": 200}), template("T", None)])
case("Bağlısı olmayan şablon kaldırılır: uyarısız", [template("T", UYG), template("T", None)])
case("Şablon yeniden adlandırılır, kimliği ve bağlar kalır",
     [save("A"), template("T", UYG), recipe("A", {"template": "T"}), {"kind": "templateRename", "name": "T", "to": "Uygulama"}])
case("Kaldırılan adın kimliği alınmışken yeni şablon sonraki kimliği alır",
     [template("T", UYG), {"kind": "templateRename", "name": "T", "to": "U"}, template("T", MOB)])
case("Kalemler ve siyah beyaz görünümün kendi tarifinde",
     [save("A"), recipe("A", {"scale": 100, "pens": [{"of": "walls", "weight": 0.7, "color": "#202020"}, {"of": "others", "color": "#808080"}], "mono": True})])
case("Kalem kalınlık sınırları: 0 ve 5 mm kabul",
     [save("A"), recipe("A", {"pens": [{"of": "walls", "weight": 0}, {"of": "openings", "weight": 5}]})])
case("Yalnız siyah beyaz tarif", [save("A"), recipe("A", {"mono": True})])
case("Siyah beyaz false yazılmaz: boş tarif", [save("A"), recipe("A", {"mono": False})])
case("Aynı kalem iki kez reddedilir", [save("A"), recipe("A", {"pens": [{"of": "walls", "weight": 0.5}, {"of": "walls", "weight": 0.3}]})])
case("Kalınlığı ve rengi olmayan kalem reddedilir", [save("A"), recipe("A", {"pens": [{"of": "texts"}]})])
case("Kalem kalınlığı 5 mm'den büyük reddedilir", [save("A"), recipe("A", {"pens": [{"of": "walls", "weight": 5.5}]})])
case("Kalemin rengi #rrggbb değilse reddedilir", [save("A"), recipe("A", {"pens": [{"of": "walls", "color": "kırmızı"}]})])
case("Şablonda kat reddedilir", [template("T", {"level": {"id": "zemin"}, "scale": 50})])
case("Şablonda kırpma reddedilir", [template("T", {"scale": 50, "crop": True})])
case("Boş şablon reddedilir", [template("T", {})])
case("Şablona bağlı tarifte ölçek reddedilir", [save("A"), template("T", UYG), recipe("A", {"template": "T", "scale": 100})])
case("Olmayan şablona bağlanmak reddedilir", [save("A"), recipe("A", {"template": "Yok"})])
case("Olmayan şablonu kaldırmak reddedilir", [template("Yok", None)])
case("Olmayan şablonu yeniden adlandırmak reddedilir", [{"kind": "templateRename", "name": "Yok", "to": "Var"}])
case("Başka şablonun adına yeniden adlandırmak reddedilir",
     [template("A", UYG), template("B", MOB), {"kind": "templateRename", "name": "A", "to": "B"}])
full = Project()
for i in range(MOST_TEMPLATES):
    full.run(template(f"Ş{i:02d}", {"scale": 100 + i}))
cases.append({"name": "Elli birinci şablon reddedilir", "steps": [
    {"op": "execute", "input": {"edit": template(f"Ş{i:02d}", {"scale": 100 + i})}, "result": "$any", "expect": {"revision": "changed"}}
    for i in range(MOST_TEMPLATES)] + [full.run(template("Fazla", {"scale": 50}))]})


# cad.sheets.set: the maps linked to views bound to a template draw by it (ADR 0774's book, docs/adr/0783 §3).
T_ZEMIN = {"version": 1, "hidden": ["hatches"], "scale": 100}


def views_with(tpl):
    out = copy(vrs.VIEWS)
    for v in out:
        if v["name"] == "Zemin":
            v["recipe"] = {"version": 2, "level": {"id": "zemin"}, "template": "t-ozel"}
    return out


def resolved(views, tpl):
    """The views as a map draws them: a bound recipe takes its template's fields, keeps its level and crop."""
    out = copy(views)
    for v in out:
        r = v.get("recipe")
        if r and r.get("template") == "t-ozel":
            v["recipe"] = {**{k: x for k, x in tpl.items() if k != "version"}, "version": r["version"],
                           **({"level": r["level"]} if "level" in r else {})}
    return out


def sheet_setup(tpl, views):
    s = copy(vrs.SETUP)
    s["name"] = "Şablonlu pafta seti"
    s["settings"]["namedViews"] = views
    s["settings"]["viewTemplates"] = [{"id": "t-ozel", "name": "Özel", "recipe": tpl}]
    return s


SHEET_TPL = T_ZEMIN
SHEET_VIEWS = views_with(SHEET_TPL)
sheet_cases = []


def sheet_case(name, steps):
    sheet_cases.append({"name": name, "steps": copy(vrs.PRELUDE) + steps})


def tpl_edit(r):
    return vrs.views_edit({"kind": "template", "name": "Özel", "recipe": r})


draw = resolved(SHEET_VIEWS, SHEET_TPL)
sheet_case("Şablona bağlı görünümün haritası şablonun ölçeğiyle", [vrs.step(vrs.Book(draw))])
sheet_case("Şablonun gizlediği tarama haritada yok", [vrs.step(vrs.Book(draw), [4])])
sheet_case("Görünümün katı: zemin kolonu gösterilir, 1. kat kolonu gösterilmez", [vrs.step(vrs.Book(draw), [1, 2])])
for i, (label, tpl) in enumerate([
    ("Şablon taramayı göstermeye dönünce harita taramayı gösterir", {"version": 2, "scale": 100}),
    ("Şablonun ölçeği 1:20 olunca bağlı harita küçülür, uzaktaki çizgi dışarıda kalabilir", {"version": 2, "hidden": ["hatches"], "scale": 20}),
    ("Şablon Yeni durum evresiyle: yıkılacak kolon gösterilmez", {"version": 2, "hidden": ["hatches"], "scale": 100, "phase": "proposed"}),
    ("Şablonun kalemleri ve siyah beyazı neyin gösterildiğini değiştirmez", {"version": 2, "hidden": ["hatches"], "scale": 100,
                                                                             "pens": [{"of": "walls", "weight": 0.7}], "mono": True}),
]):
    edit = {k: x for k, x in tpl.items() if k != "version"}
    sheet_case(label, [tpl_edit(edit), vrs.step(vrs.Book(resolved(SHEET_VIEWS, tpl)), [1, 3, 4, 7, 8])])
sheet_case("Bütün nesneler şablonla", [vrs.step(vrs.Book(draw), vrs.IDS)])
sheet_case("Şablon kaldırılınca görünüm aynı tarifle çizer (sürümü bir fazla)",
           [vrs.views_edit({"kind": "template", "name": "Özel"}),
            vrs.step(vrs.Book([{**v, "recipe": {**v["recipe"], "version": 3}} if v["name"] == "Zemin" else v for v in draw]), [4, 1])])


def write(path, doc, label, n):
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = ROOT / path
    if "--check" in sys.argv[1:]:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            sys.exit(f"{path.relative_to(ROOT)} bağımsız başvurudan farklı.")
        print(f"{label}: {n} durum başvuruyla aynı.")
        return
    path.write_text(text, encoding="utf-8")
    print(f"{path.relative_to(ROOT)}: {n} durum yazıldı.")


write("fixtures/commands/v1/cad.views.edit.template.json", {
    "format": "kentos.command-cases", "version": 1, "command": "cad.views.edit", "commandVersion": 1,
    "title": "Görünüş şablonu ve kalem seti (ADR 0783)",
    "note": "Kural ve beklenenler scripts/fixtures/view_template_cases.py'de ADR 0783'ten, KentOS kodu olmadan yazılmıştır.",
    "setup": SETUP, "cases": cases}, "cad.views.edit (şablon)", len(cases))
write("fixtures/commands/v1/cad.sheets.set.template.json", {
    "format": "kentos.command-cases", "version": 1, "command": "cad.sheets.set", "commandVersion": 1,
    "title": "Pafta seti ve görünüş şablonu (ADR 0783)",
    "note": "scripts/fixtures/view_template_cases.py yazar; ADR 0347, 0774 ve 0783'ün kurallarından, KentOS kodu olmadan.",
    "setup": sheet_setup(SHEET_TPL, SHEET_VIEWS), "cases": sheet_cases}, "cad.sheets.set (şablon)", len(sheet_cases))
