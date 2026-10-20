"""Writes the shared cases of the drawing standard (docs/adr/0312 §2, §3):

- fixtures/commands/v1/cad.standards.check.json (`cad.standards.check` v1: what a drawing breaks of a standard)
- fixtures/commands/v1/cad.standards.apply.json (`cad.standards.apply` v1: the planned fix)

The rules are written here again from the ADR and the contract (crates/shared/commands/src/cad_standards.rs), the
tree, the objects and the settings as plain JSON, without KentOS code: the findings in their order (the standard's
layers, the drawing's other layers, the fonts and styles, the objects), each with its message; the apply as
Katmanları çevir's merging (layer_find_cases.py's `translate`), then the standard layers added or restyled, then
the fonts and the missing styles. The built-in standard is read from catalog/v1/standards/ as data. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/standards_cases.py           # writes the files
    python3 scripts/fixtures/standards_cases.py --check   # compares them with the ones on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


F = _load("layer_find_cases")
T = F.T
node, step, completed, failed, VALID, refused = T.node, T.step, T.completed, T.failed, T.VALID, T.refused
walk, get, inserted, fold, matches = T.walk, T.get, T.inserted, T.fold, F.matches
X, Y = T.X, T.Y
INVALID_REVISION, CONFLICT = T.INVALID_REVISION, T.CONFLICT

BUILT_IN = {"mimari-ofis": json.loads((ROOT / "catalog/v1/standards/mimari-ofis.json").read_text(encoding="utf-8"))}

LABEL = {"continuous": "Sürekli", "dashed": "Kesikli", "dashdot": "Noktalı kesik", "dotted": "Noktalı"}
TYPES = ["continuous", "dashed", "dashdot", "dotted"]
FONT = {"barlow": "Barlow", "arimo": "Arimo", "overpass": "Overpass", "quicksand": "Quicksand",
        "architects-daughter": "Architects Daughter", "courier-prime": "Courier Prime", "plex-mono": "IBM Plex Mono"}

# ── A small standard and a drawing that breaks it ────────────────────────────

STANDARD = {
    "name": "Deneme ofisi",
    "layers": [
        {"name": "A-DUVAR", "color": "ink", "lineType": "continuous", "lineWeight": 0.5, "from": "duvar*,wall*"},
        {"name": "A-KAPI", "color": "#C8A165", "lineType": "continuous", "lineWeight": 0.25, "from": "kapı*,kapi*"},
        {"name": "A-EKSEN", "color": "#E06C75", "lineType": "dashdot", "lineWeight": 0.18, "from": "aks*"},
        {"name": "A-YAZI", "color": "fg", "lineType": "continuous", "lineWeight": 0.18},
        {"name": "A-YARDIMCI", "color": "fg-dim", "lineType": "dashed", "lineWeight": 0.13, "plot": False},
    ],
    "naming": ["A-*"],
    "fonts": ["barlow", "arimo"],
    "textStyles": [{"name": "A-Yazı", "font": "barlow", "height": 2.5}, {"name": "A-Başlık", "font": "barlow", "height": 5}],
    "dimensionStyles": [{"name": "A-Ölçü", "height": 2.5}],
    "lineTypes": ["continuous", "dashed", "dashdot"],
    "byLayer": True,
}

DUVAR = {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.5}
EKSEN = {"color": "#e06c75", "lineType": "dashed", "lineWeight": 0.25}
YAZI = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}
NOKTA = {"color": "#8A94A6", "lineType": "dotted", "lineWeight": 0.13}


def layers():
    return [
        node("mimari", "Mimari", "group", [
            node("duvar", "A-DUVAR", style=DUVAR),
            node("eksen", "a-eksen", style=EKSEN),
            node("yazi", "A-YAZI", style=YAZI, plot=False),
        ]),
        node("wall", "Wall", style=DUVAR),
        node("kapi-eski", "kapı_eski"),
        node("anot", "A-NOT"),
        node("kat1", "Kat1"),
        node("cizgi", "Noktalı çizgiler", style=NOKTA),
        node("altlik", "OpenStreetMap", service=T.OSM),
    ]


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


def ent(id_, layer, k=0, **own):
    return {"kind": "polygon", "id": id_, "layerId": layer, "attrs": {}, "pts": rect(X + 10 * k, Y, 5, 5), **own}


ENTITIES = [ent(1, "duvar", 0), ent(2, "wall", 1), ent(3, "wall", 2, color="#FF0000"), ent(4, "kapi-eski", 3),
            ent(5, "kat1", 4, lineType="dotted"), ent(6, "kat1", 5, lineWeight=0.35), ent(7, "anot", 6),
            ent(8, "eksen", 7)]
IDS = [e["id"] for e in ENTITIES]

TEXT_STYLES = [
    {"id": "0192f5a0-0000-7000-8000-000000000001", "name": "A-Yazı", "font": "quicksand", "height": 2.5},
    {"id": "0192f5a0-0000-7000-8000-000000000002", "name": "Eski yazı", "font": "arimo"},
    {"id": "0192f5a0-0000-7000-8000-000000000003", "name": "El yazısı", "font": "architects-daughter"},
]
DIMENSION_STYLES = [{"id": "0192f5a0-0000-7000-8000-000000000011", "name": "Eski ölçü", "height": 3}]


def setup(tree=None, entities=None, active="kat1", **settings):
    s = {"textStyles": TEXT_STYLES, "dimensionStyles": DIMENSION_STYLES, **settings}
    doc = T.setup(layers() if tree is None else tree, ENTITIES if entities is None else entities, active, **s)
    doc["settings"]["drawingFont"] = settings.get("drawingFont", "quicksand")
    return doc


# ── The rules ────────────────────────────────────────────────────────────────


def weight_text(w):
    return str(int(w)) if w == int(w) else repr(w)


def plot_text(p):
    return "açık" if p else "kapalı"


def leaves(nodes):
    return [n for n, _ in walk(nodes) if n["type"] == "layer" and "service" not in n]


def standard_index(st, name):
    return next((i for i, l in enumerate(st["layers"]) if fold(l["name"].strip()) == fold(name.strip())), None)


def belongs(st, name):
    return next((i for i, l in enumerate(st["layers"]) if "from" in l and matches(l["from"], name)), None)


def chain_locked(nodes, id_):
    return T.locked(nodes, id_)


def issue(kind, subject, layer=None, expected=None, found=None, objects=0, fixable=False, message=""):
    out = {"kind": kind, "subject": subject}
    if layer is not None:
        out["layer"] = layer
    if expected is not None:
        out["expected"] = expected
    if found is not None:
        out["found"] = found
    if objects:
        out["objects"] = objects
    out["fixable"] = fixable
    out["message"] = message
    return out


def issues(st, tree, entities, settings):
    out = []
    all_ = leaves(tree)
    for l in st["layers"]:
        name = l["name"].strip()
        n = next((n for n in all_ if fold(n["name"]) == fold(name)), None)
        if n is None:
            out.append(issue("missingLayer", name, expected=name, fixable=True,
                             message=f"“{name}” katmanı çizimde yok; Standardı uygula onu ekler."))
            continue
        s, plot, want_plot = n["style"], n.get("plot") is not False, l.get("plot", True)
        for kind, what, same, ef, shown in [
            ("layerColor", "rengi", fold(l["color"]) == fold(s["color"]), (l["color"], s["color"]), (l["color"], s["color"])),
            ("layerLineType", "çizgi tipi", l["lineType"] == s["lineType"], (l["lineType"], s["lineType"]),
             (LABEL[l["lineType"]], LABEL[s["lineType"]])),
            ("layerWeight", "kalınlığı", abs(l["lineWeight"] - s["lineWeight"]) <= 1e-9,
             (weight_text(l["lineWeight"]), weight_text(s["lineWeight"])),
             (weight_text(l["lineWeight"]) + " mm", weight_text(s["lineWeight"]) + " mm")),
            ("layerPlot", "Yazdır'ı", want_plot == plot, (plot_text(want_plot), plot_text(plot)), (plot_text(want_plot), plot_text(plot))),
        ]:
            if not same:
                out.append(issue(kind, n["name"], n["id"], ef[0], ef[1], fixable=True,
                                 message=f"“{n['name']}” katmanının {what} {shown[1]}, standartta {shown[0]}."))
    for n in all_:
        if standard_index(st, n["name"]) is not None:
            continue
        i = belongs(st, n["name"])
        if i is not None:
            to = st["layers"][i]["name"].strip()
            locked = chain_locked(tree, n["id"])
            if locked:
                msg = f"“{n['name']}” katmanı standartta yok ve kilitli; kilidini açın, Standardı uygula nesnelerini “{to}” katmanına taşır."
            else:
                msg = f"“{n['name']}” katmanı standartta yok; Standardı uygula nesnelerini “{to}” katmanına taşır."
            out.append(issue("layerName", n["name"], n["id"], to, n["name"], fixable=not locked, message=msg))
        elif not any(matches(p, n["name"]) for p in st.get("naming", [])):
            tail = "katmanı yeniden adlandırın ya da Katmanları çevir'le bir standart katmana taşıyın."
            if st.get("naming"):
                msg = f"“{n['name']}” adı standartta yok ve {', '.join(st['naming'])} adlarına uymuyor; {tail}"
            else:
                msg = f"“{n['name']}” adı standartta yok; {tail}"
            out.append(issue("layerName", n["name"], n["id"], None, n["name"], message=msg))
        types = st.get("lineTypes", [])
        if types and n["style"]["lineType"] not in types:
            out.append(issue("layerLineType", n["name"], n["id"], ",".join(types), n["style"]["lineType"],
                             message=f"“{n['name']}” katmanının çizgi tipi {LABEL[n['style']['lineType']]}; standart yalnız "
                                     f"{', '.join(LABEL[t] for t in types)} çizgi tiplerini kullanır."))
    fonts = st.get("fonts", [])
    if fonts:
        font = settings.get("drawingFont", "barlow")
        if font not in fonts:
            out.append(issue("projectFont", FONT[font], expected=fonts[0], found=font, fixable=True,
                             message=f"Projenin yazı tipi {FONT[font]}; standart {', '.join(FONT[f] for f in fonts)} kullanır."))
    std_texts = st.get("textStyles", [])
    for s in settings.get("textStyles", []):
        wanted = next((t for t in std_texts if fold(t["name"].strip()) == fold(s["name"].strip())), None)
        if wanted is not None:
            expected = wanted["font"]
        else:
            expected = fonts[0] if fonts and s["font"] not in fonts else None
        if expected is not None and expected != s["font"]:
            out.append(issue("textFont", s["name"], expected=expected, found=s["font"], fixable=True,
                             message=f"“{s['name']}” yazı stilinin yazı tipi {FONT[s['font']]}, standartta {FONT[expected]}."))
        if wanted is None and std_texts:
            out.append(issue("textStyleName", s["name"], found=s["name"],
                             message=f"“{s['name']}” yazı stili standartta yok; yazılarını standardın bir stiline geçirin."))
    for t in std_texts:
        name = t["name"].strip()
        if not any(fold(s["name"].strip()) == fold(name) for s in settings.get("textStyles", [])):
            out.append(issue("missingTextStyle", name, expected=name, fixable=True,
                             message=f"“{name}” yazı stili projede yok; Standardı uygula onu ekler."))
    std_dims = st.get("dimensionStyles", [])
    for s in settings.get("dimensionStyles", []):
        if std_dims and not any(fold(d["name"].strip()) == fold(s["name"].strip()) for d in std_dims):
            out.append(issue("dimensionStyleName", s["name"], found=s["name"],
                             message=f"“{s['name']}” ölçü stili standartta yok; ölçülerini standardın bir stiline geçirin."))
    for d in std_dims:
        name = d["name"].strip()
        if not any(fold(s["name"].strip()) == fold(name) for s in settings.get("dimensionStyles", [])):
            out.append(issue("missingDimensionStyle", name, expected=name, fixable=True,
                             message=f"“{name}” ölçü stili projede yok; Standardı uygula onu ekler."))
    types = st.get("lineTypes", [])
    by_type = {t: sum(1 for e in entities if e.get("lineType") == t) for t in TYPES}
    if types:
        for t in TYPES:
            if by_type[t] and t not in types:
                out.append(issue("objectLineType", LABEL[t], found=t, objects=by_type[t],
                                 message=f"{by_type[t]} nesnenin kendi çizgi tipi {LABEL[t]}; standart yalnız "
                                         f"{', '.join(LABEL[x] for x in types)} çizgi tiplerini kullanır."))
    own = sum(1 for e in entities if "color" in e or "lineWeight" in e or "lineType" in e)
    if st.get("byLayer") and own:
        out.append(issue("ownLook", "Katmana göre", objects=own,
                         message=f"{own} nesnenin kendi rengi, çizgi tipi ya da kalınlığı var; standartta nesneler görünüşünü katmanından "
                                 "alır. Özellik kopyala ya da Öznitelikler'in “Katmana göre”siyle düzeltin."))
    return out


def report(st, tree=None, entities=None, settings=None):
    t = layers() if tree is None else tree
    e = ENTITIES if entities is None else entities
    s = setup()["settings"] if settings is None else settings
    found = issues(st, t, e, s)
    return {"standard": st["name"], "issues": found, "fixable": sum(1 for i in found if i["fixable"]), "revision": "$current"}


def apply(st, tree, entities, active, settings, layers_=True, restyle=True, rename=True, styles=True, parent=None):
    """docs/adr/0312 §3 as plain JSON: the tree, objects, active layer and settings after, and the answer."""
    answer = {"changed": False, "created": [], "restyled": [], "translations": [], "objects": 0, "textStyles": [],
              "dimensionStyles": [], "font": False, "revision": "$current"}
    t, ents, act, sets = copy.deepcopy(tree), copy.deepcopy(entities), active, copy.deepcopy(settings)
    if rename and any(standard_index(st, n["name"]) is None and belongs(st, n["name"]) is not None for n in leaves(t)):
        rules = [{"from": l["from"], "to": l["name"].strip(), "color": l["color"], "lineType": l["lineType"],
                  "lineWeight": l["lineWeight"]} for l in st["layers"] if "from" in l]
        t, ents, act, tr = F.translate(t, ents, act, rules, parent)
        answer["translations"], answer["objects"] = tr["translations"], tr["objects"]
        answer["created"] += tr["created"]
        answer["restyled"] += tr["restyled"]
    made = []
    for l in st["layers"]:
        name = l["name"].strip()
        n = next((n for n in leaves(t) if fold(n["name"]) == fold(name)), None)
        want = {"color": l["color"], "lineType": l["lineType"], "lineWeight": l["lineWeight"]}
        if n is not None:
            just = n["id"] in answer["created"]
            if not (restyle or just):
                continue
            changed = False
            if any(n["style"].get(k) != v for k, v in want.items()):
                n["style"].update(want)
                changed = True
            plot = n.get("plot") is not False
            if plot != l.get("plot", True):
                if l.get("plot", True):
                    n.pop("plot", None)
                else:
                    n["plot"] = False
                changed = True
            if changed and not just and n["id"] not in answer["restyled"]:
                answer["restyled"].append(n["id"])
        elif layers_:
            new_id = f"$layer:{len(answer['created']) + len(made) + 1}"
            extra = {} if l.get("plot", True) else {"plot": False}
            t = inserted(t, node(new_id, name, style=want, **extra), parent)
            made.append(new_id)
    answer["created"] += made
    if styles:
        fonts = st.get("fonts", [])
        if fonts and sets.get("drawingFont", "barlow") not in fonts:
            sets["drawingFont"] = fonts[0]
            answer["font"] = True
        texts = sets.get("textStyles", [])
        for s in texts:
            wanted = next((x for x in st.get("textStyles", []) if fold(x["name"].strip()) == fold(s["name"].strip())), None)
            f = wanted["font"] if wanted else (fonts[0] if fonts and s["font"] not in fonts else None)
            if f is not None and f != s["font"]:
                s["font"] = f
                answer["textStyles"].append(s["name"])
        for x in st.get("textStyles", []):
            name = x["name"].strip()
            if not any(fold(s["name"].strip()) == fold(name) for s in texts):
                added = {"id": "?", "name": name, "font": x["font"]}
                if "height" in x:
                    added["height"] = x["height"]
                texts.append(added)
                answer["textStyles"].append(name)
        dims = sets.get("dimensionStyles", [])
        for d in st.get("dimensionStyles", []):
            name = d["name"].strip()
            if not any(fold(s["name"].strip()) == fold(name) for s in dims):
                dims.append({"id": "?", "name": name, "height": d["height"]})
                answer["dimensionStyles"].append(name)
    answer["changed"] = bool(answer["created"] or answer["restyled"] or answer["translations"] or answer["textStyles"]
                             or answer["dimensionStyles"] or answer["font"])
    return t, ents, act, sets, answer


def style_rows(settings):
    """The styles as the runner reads them back: without their ids (new ones are UUIDv7s)."""
    text = [{k: v for k, v in s.items() if k != "id"} for s in settings.get("textStyles", [])]
    dims = [{k: v for k, v in s.items() if k != "id"} for s in settings.get("dimensionStyles", [])]
    return {"text": text, "dimension": dims}


# ── cad.standards.check ──────────────────────────────────────────────────────


def clean_doc():
    """A drawing that keeps the standard: its layers in their looks, a name the naming allows, objects by layer,
    its font and styles."""
    tree = [node(f"s{i}", l["name"], style={k: l[k] for k in ("color", "lineType", "lineWeight")},
                 **({} if l.get("plot", True) else {"plot": False})) for i, l in enumerate(STANDARD["layers"])]
    tree.append(node("plan", "A-PLAN"))
    return setup(tree=tree, entities=[ent(1, "s0", 0), ent(2, "plan", 1)], active="plan", drawingFont="barlow",
                 textStyles=[{"id": "0192f5a0-0000-7000-8000-00000000000%d" % (i + 4), **x} for i, x in enumerate(STANDARD["textStyles"])],
                 dimensionStyles=[{"id": "0192f5a0-0000-7000-8000-000000000012", **STANDARD["dimensionStyles"][0]}])


INPUT = {"standard": STANDARD}


def check_case(name, input_, st, doc=None, **kw):
    d = setup(**kw) if doc is None else doc
    answer = report(st, d["layers"], d["entities"], d["settings"])
    return {"name": name, "setup": d, "steps": [
        step("execute", input_, completed(answer), T.unchanged(d["layers"], [e["id"] for e in d["entities"]])),
        step("plan", input_, completed(answer), {"revision": "same"})]}


def check_cases():
    t = layers()
    out = [
        check_case("Bütün bulgular, sırasıyla: standart katmanlar, öbür katmanlar, yazı tipleri ve stiller, nesneler", INPUT, STANDARD),
        check_case("Yerleşik standart adıyla: Mimari ofis", {"name": "mimari-ofis"}, BUILT_IN["mimari-ofis"]),
        check_case("Renk büyük küçük harf ayırmadan karşılaştırılır; ad Türkçe katlanır (a-eksen A-EKSEN'dir)", INPUT, STANDARD),
        check_case("Ad örüntüsü yoksa standartta olmayan her ad bulgudur",
                   {"standard": {**STANDARD, "naming": []}}, {**STANDARD, "naming": []}),
        check_case("Yazı tipi listesi yoksa yazı tipi denetlenmez, standardın stilinin yazı tipi denetlenir",
                   {"standard": {**STANDARD, "fonts": []}}, {**STANDARD, "fonts": []}),
        check_case("Stil listesi yoksa projenin stilleri serbesttir",
                   {"standard": {**STANDARD, "textStyles": [], "dimensionStyles": []}},
                   {**STANDARD, "textStyles": [], "dimensionStyles": []}),
        check_case("Çizgi tipi listesi yoksa çizgi tipleri serbesttir", {"standard": {**STANDARD, "lineTypes": []}},
                   {**STANDARD, "lineTypes": []}),
        check_case("Katmana göre değilse nesnelerin kendi görünüşü bulgu değildir", {"standard": {**STANDARD, "byLayer": False}},
                   {**STANDARD, "byLayer": False}),
        check_case("Kilitli eski katman: bulgu ama düzeltilemez",
                   INPUT, STANDARD, doc=setup(tree=[*layers()[:2], node("kapi-eski", "kapı_eski", locked=True), *layers()[3:]])),
        check_case("Kilitli grubun altındaki eski katman da düzeltilemez",
                   INPUT, STANDARD, doc=setup(tree=[node("arsiv", "Arşiv", "group", [node("wall", "Wall", style=DUVAR)], locked=True),
                                                    node("kat1", "Kat1")],
                                              entities=[ent(1, "wall", 0), ent(2, "kat1", 1)])),
        check_case("Projenin yazı tipi standartta varsa bulgu yok", INPUT, STANDARD, doc=setup(drawingFont="arimo")),
        check_case("Servis katmanı denetlenmez", INPUT, STANDARD,
                   doc=setup(tree=[node("altlik", "OpenStreetMap", service=T.OSM), node("kat1", "A-YAZI", style=YAZI)],
                             entities=[ent(1, "kat1", 0)])),
        check_case("Boş çizim: bütün standart katmanlar ve stiller eksik",
                   INPUT, STANDARD, doc=setup(tree=[node("kat1", "A-YAZI", style=YAZI)], entities=[ent(1, "kat1", 0)],
                                              textStyles=[], dimensionStyles=[], drawingFont="barlow")),
        check_case("Standarda uyan çizim: bulgu yok", INPUT, STANDARD, doc=clean_doc()),
        {"name": "Doğrula: yazmaz", "setup": setup(),
         "steps": [step("validate", INPUT, VALID, T.unchanged(t, IDS))]},
    ]
    d = setup()
    out.append({**refused("standart verilmedi", {}, "no_standard",
                          "Standardı adıyla (name: mimari-ofis) ya da kendisini (standard) verin.", "name", t, IDS), "setup": d})
    out.append({**refused("yerleşik standart yok", {"name": "yok"}, "standard_not_found",
                          "“yok” adlı yerleşik standart yok; bunlardan birini verin: mimari-ofis.", "name", t, IDS), "setup": d})
    for name, st, why in [
        ("adı boş", {**STANDARD, "name": " "}, "Standardın adı boş"),
        ("katman iki kez", {**STANDARD, "layers": [STANDARD["layers"][0], {**STANDARD["layers"][0], "name": "a-duvar"}]},
         "“a-duvar” katmanı standartta iki kez var"),
        ("katman rengi geçersiz", {**STANDARD, "layers": [{**STANDARD["layers"][0], "color": "kırmızı"}]},
         "“A-DUVAR” katmanının rengi (“kırmızı”) bir katman rengi değil; #rrggbb ya da fg, fg-dim, ink, paper yazın"),
        ("katman kalınlığı aralık dışı", {**STANDARD, "layers": [{**STANDARD["layers"][0], "lineWeight": 120}]},
         "“A-DUVAR” katmanının kalınlığı 0 ile 100 mm arasında olmalı"),
        ("topladığı adlar okunmuyor", {**STANDARD, "layers": [{**STANDARD["layers"][0], "from": "duvar["}]},
         "“A-DUVAR” katmanının topladığı adlar (“duvar[”) bir örüntü değil: [ ile açılan küme ] ile kapanmıyor"),
        ("ad örüntüsü okunmuyor", {**STANDARD, "naming": ["A-["]}, "Katman adı örüntüsü “A-[” geçersiz: [ ile açılan küme ] ile kapanmıyor"),
        ("yazı stili iki kez", {**STANDARD, "textStyles": [STANDARD["textStyles"][0], STANDARD["textStyles"][0]]},
         "Yazı stili “A-Yazı” boş ya da iki kez var"),
        ("ölçü stilinin yüksekliği", {**STANDARD, "dimensionStyles": [{"name": "A-Ölçü", "height": 0}]},
         "“A-Ölçü” ölçü stilinin yüksekliği 0'dan büyük olmalı"),
    ]:
        out.append({**refused(f"standart geçersiz: {name}", {"standard": st}, "invalid_standard", f"Standart geçersiz: {why}.",
                              "standard", t, IDS), "setup": d})
    return out


# ── cad.standards.apply ──────────────────────────────────────────────────────


def applied(name, input_, st=STANDARD, doc=None, undo=True, warnings=(), **kw):
    d = setup() if doc is None else doc
    flags = {k: input_.get(k, True) for k in ("layers", "restyle", "rename", "styles")}
    tree, ents, act, sets, answer = apply(st, d["layers"], d["entities"], d["activeLayer"], d["settings"],
                                          layers_=flags["layers"], restyle=flags["restyle"], rename=flags["rename"],
                                          styles=flags["styles"], parent=input_.get("parent"))
    ids = [e["id"] for e in ents]
    if not answer["changed"]:
        steps = [step("execute", input_, completed(answer), T.unchanged(d["layers"], ids))]
    else:
        layer_step = bool(answer["created"] or answer["restyled"] or answer["translations"])
        expect = {"ids": ids, "layers": tree, "entities": {str(e["id"]): e for e in ents}, "activeLayer": act,
                  "settings": {"drawingFont": sets["drawingFont"]}, "styleNames": style_rows(sets), "dirty": True}
        if layer_step:
            expect["canUndo"] = True
        steps = [step("plan", input_, completed(answer, warnings), {"revision": "same", "layers": d["layers"]}),
                 step("execute", input_, completed(answer, warnings), expect)]
        if undo and layer_step and not flags["styles"]:
            # Yazdır is an edit, not an undo step, as a lock (docs/adr/0260 §4): undo leaves it as the apply set it.
            back = copy.deepcopy(d["layers"])
            for n, _ in walk(back):
                after = get(tree, n["id"])
                if after is not None and n["type"] == "layer":
                    n.pop("plot", None)
                    if "plot" in after:
                        n["plot"] = after["plot"]
            steps.append(step("undo", returns="Standardı uygula",
                              expect={"layers": back, "entities": {str(e["id"]): e for e in d["entities"]}}))
    return {"name": name, "setup": d, "steps": steps}


def apply_cases():
    t = layers()
    no_styles = {"standard": STANDARD, "styles": False}
    out = [
        applied("Hepsi: eski katmanlar birleşir, eksikler eklenir, görünüşler düzelir, yazı tipi ve stiller", {"standard": STANDARD}),
        applied("Yalnız katmanlar (styles kapalı): tek adım “Standardı uygula”, geri alınır", no_styles),
        applied("Eksik katmanlar eklenmez (layers kapalı)", {**no_styles, "layers": False}),
        applied("Görünüşler düzeltilmez (restyle kapalı); birleşmede açılan katman yine standardın görünüşünü alır",
                {**no_styles, "restyle": False}),
        applied("Eski katmanlar birleşmez (rename kapalı)", {**no_styles, "rename": False}),
        applied("Yalnız stiller: katmanlara dokunulmaz", {"standard": STANDARD, "layers": False, "restyle": False, "rename": False}),
        applied("Yeni katmanlar verilen grubun sonuna", {**no_styles, "parent": "mimari"}),
        applied("Yazdırılmayan standart katman Yazdır'ı kapalı eklenir",
                {"standard": {**STANDARD, "layers": [STANDARD["layers"][4]]}, "styles": False},
                st={**STANDARD, "layers": [STANDARD["layers"][4]]}),
        applied("Yerleşik standart adıyla: Mimari ofis", {"name": "mimari-ofis", "styles": False}, st=BUILT_IN["mimari-ofis"]),
        applied("Etkin katman birleşen katmansa standart katman etkin olur", no_styles, doc=setup(active="wall")),
        applied("Projenin yazı tipi standartta varsa değişmez", {"standard": STANDARD, "layers": False, "restyle": False,
                                                                 "rename": False}, doc=setup(drawingFont="arimo")),
        applied("Uyan çizimde değişen yok",
                {"standard": {**STANDARD, "layers": [STANDARD["layers"][3]], "fonts": [], "textStyles": [], "dimensionStyles": []}},
                st={**STANDARD, "layers": [STANDARD["layers"][3]], "fonts": [], "textStyles": [], "dimensionStyles": []},
                doc=setup(tree=[node("yazi", "A-YAZI", style=YAZI)], entities=[ent(1, "yazi", 0)], active="yazi")),
        applied("Gizli standart katmana taşınan nesneler söylenir", no_styles,
                doc=setup(tree=[node("mimari", "Mimari", "group", [node("duvar", "A-DUVAR", style=DUVAR, visible=False),
                                                                    node("eksen", "a-eksen", style=EKSEN),
                                                                    node("yazi", "A-YAZI", style=YAZI, plot=False)]),
                                *layers()[1:]]),
                warnings=[{"code": "layer_hidden", "message": "“A-DUVAR” katmanı gizli; taşınan nesneler görünmeyecek.", "path": "layerId"}]),
        {"name": "Doğrula: yazmaz", "setup": setup(), "steps": [step("validate", {"standard": STANDARD}, VALID, T.unchanged(t, IDS))]},
    ]
    d = setup()
    out.append({**refused("standart verilmedi", {}, "no_standard",
                          "Standardı adıyla (name: mimari-ofis) ya da kendisini (standard) verin.", "name", t, IDS), "setup": d})
    out.append({**refused("yerleşik standart yok", {"name": "yok"}, "standard_not_found",
                          "“yok” adlı yerleşik standart yok; bunlardan birini verin: mimari-ofis.", "name", t, IDS), "setup": d})
    out.append({**refused("standart geçersiz", {"standard": {**STANDARD, "name": ""}}, "invalid_standard",
                          "Standart geçersiz: Standardın adı boş.", "standard", t, IDS), "setup": d})
    out.append({**refused("grup çizimde yok", {"standard": STANDARD, "parent": "yok"}, "layer_not_found",
                          "“yok” kimlikli katman ya da grup çizimde yok. Var olan bir grubun kimliğini verin.", "parent", t, IDS), "setup": d})
    out.append({**refused("grup değil", {"standard": STANDARD, "parent": "kat1"}, "not_a_group",
                          "“Kat1” bir katman; yeni katmanlar yalnız bir grubun içine eklenir. Bir grubun kimliğini verin ya da parent'ı vermeyin (en üst).",
                          "parent", t, IDS), "setup": d})
    lt = [*layers()[:2], node("kapi-eski", "kapı_eski", locked=True), *layers()[3:]]
    out.append({**refused("birleşecek katman kilitli", {"standard": STANDARD}, "layer_locked",
                          "“kapı_eski” katmanı kilitli; çevrilemez. Kilidini Katmanlar panelinden açın ya da kuralı (“kapı*,kapi*”) daraltın.",
                          "standard", lt, IDS), "setup": setup(tree=lt)})
    servis = {**STANDARD, "layers": [{"name": "OpenStreetMap", "color": "fg", "lineType": "continuous", "lineWeight": 0.18,
                                       "from": "kat#"}]}
    out.append({**refused("standart katman servisten", {"standard": servis}, "service_layer",
                          "“OpenStreetMap” servisten çizilir ve nesne tutmaz; katmanlar ona çevrilemez. Kurala başka bir ad verin.",
                          "standard", t, IDS), "setup": d})
    out.append({**refused("geçersiz beklenen sürüm", {"standard": STANDARD, "expectedRevision": "on iki"}, "invalid_revision",
                          INVALID_REVISION, "expectedRevision", t, IDS), "setup": d})
    out.append({**T.conflict_case("sürüm çakışması: hiçbir şey yazılmaz",
                                  step("execute", {"standard": {**STANDARD, "layers": [STANDARD["layers"][3]], "fonts": [],
                                                                "textStyles": [], "dimensionStyles": []}, "rename": False},
                                       completed({"changed": True, "created": [], "restyled": ["yazi"], "translations": [],
                                                  "objects": 0, "textStyles": [], "dimensionStyles": [], "font": False,
                                                  "revision": "$current"}),
                                       {"revision": "changed"}),
                                  {"standard": STANDARD}, {}), "setup": d})
    return out


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "Written by scripts/fixtures/standards_cases.py from the contract's rules and docs/adr/0312; do not edit by hand.",
            "setup": setup(), "cases": cases}


def files():
    return {
        OUT / "cad.standards.check.json": build("cad.standards.check", "Standardı denetle", check_cases()),
        OUT / "cad.standards.apply.json": build("cad.standards.apply", "Standardı uygula", apply_cases()),
    }


def main() -> int:
    bad = 0
    for path, doc in files().items():
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad += 1
            else:
                print(f"{path.name} güncel: {len(doc['cases'])} durum.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
