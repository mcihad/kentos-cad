"""Writes the shared cases of a field's usage and its rename (docs/adr/0414), their expectations built here from the
ADR's rules without KentOS code:

- fixtures/commands/v1/cad.fields.usage.json: `cad.fields.usage` v1, where a field is named (§1: the places in order,
  their words, their paths, how they name it, the published releases kept);
- fixtures/commands/v1/cad.layers.edit.renames.json: `cad.layers.edit`'s fields with renames (§2: the field keeps its
  old name as its id when a key names it; names, expressions and templates follow, the published releases do not).

The language's words are read here by a lexer of its own (docs/adr/0100: quoted texts, bracketed fields, `$` and `@`
values, words, operators; a word is a field unless it is a keyword or is called). The cases name no word the grammar
reads as a phrase (`durum … son`, `içinde` …): those are the Rust tests'. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/field_usage_cases.py           # writes the files
    python3 scripts/fixtures/field_usage_cases.py --check   # compares them with the ones on disk
"""
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad as kc  # noqa: E402  (the canonical JSON of the release digest)

USAGE = ROOT / "fixtures/commands/v1/cad.fields.usage.json"
RENAMES = ROOT / "fixtures/commands/v1/cad.layers.edit.renames.json"

# ── The language's words ──────────────────────────────────────────────────

OPERATORS = ["<=", ">=", "!=", "<>", "==", "||", "=", "<", ">", "+", "-", "*", "/", "%", "(", ")", ",", "^"]
FOLD = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                      "ş": "S", "Ş": "S", "ü": "U", "Ü": "U"})
KEYWORDS = {"VE", "AND", "VEYA", "OR", "DEGIL", "NOT", "DOGRU", "TRUE", "YANLIS", "FALSE", "BOS", "NULL"}


def fold(w):
    return w.translate(FOLD).upper()


def is_word_start(c):
    return c.isalpha() or c == "_"


def is_word(c):
    return c.isalnum() or c == "_"


def lex(src):
    """The pieces: (kind, name, start, end); kind one of num, str, field, var, at, word, op, bad."""
    out, i = [], 0
    while i < len(src):
        c = src[i]
        if c.isspace():
            i += 1
            continue
        start = i
        if c.isdigit():
            while i < len(src) and (src[i].isdigit() or src[i] == "."):
                i += 1
            out.append(("num", None, start, i))
        elif c in "'\"":
            j = src.find(c, i + 1)
            i = len(src) if j < 0 else j + 1
            out.append(("str", None, start, i))
        elif c == "[":
            j = src.find("]", i + 1)
            i = len(src) if j < 0 else j + 1
            out.append(("field", src[start + 1:(j if j >= 0 else len(src))].strip(), start, i))
        elif c in "$@":
            i += 1
            while i < len(src) and is_word(src[i]):
                i += 1
            out.append(("var", src[start + 1:i], start, i))
        elif is_word_start(c):
            while i < len(src) and is_word(src[i]):
                i += 1
            out.append(("word", src[start:i], start, i))
        else:
            op = next((o for o in OPERATORS if src.startswith(o, i)), None)
            i += len(op) if op else 1
            out.append(("op" if op else "bad", op, start, i))
    return out


def field_of(pieces, k):
    kind, name, _, _ = pieces[k]
    if kind == "field" and name:
        return name
    if kind == "word" and fold(name) not in KEYWORDS:
        called = k + 1 < len(pieces) and pieces[k + 1][0] == "op" and pieces[k + 1][1] == "("
        return None if called else name
    return None


def fields_of(src):
    pieces, out = lex(src), []
    for k in range(len(pieces)):
        n = field_of(pieces, k)
        if n is not None and n not in out:
            out.append(n)
    return out


def names(src, name):
    return name in fields_of(src)


def rewrite(src, old, bare, bracketed):
    pieces, out, last, hit = lex(src), [], 0, False
    for k, (kind, _, s, e) in enumerate(pieces):
        if field_of(pieces, k) != old:
            continue
        out.append(src[last:s])
        out.append(bracketed if kind == "field" else bare)
        last, hit = e, True
    return "".join(out) + src[last:] if hit else None


def rename(src, old, new):
    bracketed = f"[{new}]"
    bare = rewrite(src, old, new, bracketed)
    if bare is None:
        return None
    want = []
    for n in fields_of(src):
        n = new if n == old else n
        if n not in want:
            want.append(n)
    return bare if fields_of(bare) == want else rewrite(src, old, bracketed, bracketed)


def template_exprs(text):
    """A template's expressions: (start, end) of each source; None when it does not read."""
    out, i = [], 0
    while i < len(text):
        if text.startswith("{{", i) or text.startswith("}}", i):
            i += 2
            continue
        if text[i] == "}":
            return None
        if text[i] == "{":
            j, quote = i + 1, None
            while j < len(text):
                c = text[j]
                if quote:
                    quote = None if c == quote else quote
                elif c in "'\"[":
                    quote = "]" if c == "[" else c
                elif c == "{":
                    return None
                elif c == "}":
                    break
                j += 1
            if j >= len(text):
                return None
            inner = text[i + 1:j].strip()
            if not inner:
                return None
            if inner not in ("$enlem", "$boylam"):
                out.append((i + 1, j))
            i = j + 1
            continue
        i += 1
    return out


def template_names(text, name):
    return any(names(text[s:e], name) for s, e in template_exprs(text) or [])


def template_rename(text, old, new):
    parts, last, hit = [], 0, False
    for s, e in template_exprs(text) or []:
        r = rename(text[s:e], old, new)
        if r is None:
            continue
        parts.append(text[last:s])
        parts.append(r)
        last, hit = e, True
    return "".join(parts) + text[last:] if hit else None


# ── The drawing ───────────────────────────────────────────────────────────

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
BINA_FIELDS = [
    {"name": "Durum", "kind": "text", "values": [{"code": "iyi", "label": "İyi"}, {"code": "kotu", "label": "Kötü"}]},
    {"name": "Kat", "kind": "integer"},
    {"name": "Not", "kind": "text"},
    {"name": "Ad", "kind": "text"},
    {"name": "Tarih", "kind": "date"},
    {"name": "Boş alan", "kind": "text"},
]
MALIK_FIELDS = [{"name": "Bina", "kind": "text"}, {"name": "Malik adı", "kind": "text"}]


def node(id_, name, type_="layer", children=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": copy.deepcopy(STYLE), "children": children or [], **extra}


def malik_node():
    """The owners' layer, its look categorized by an owner's name (ADR 0564)."""
    n = node("malik", "Malik", fields=MALIK_FIELDS)
    n["style"]["renderer"] = {"type": "categorized", "expr": "[Malik adı]", "categories": [
        {"value": "Ayşe", "label": "Ayşe", "symbols": {"marker": {"type": "marker", "layers": [
            {"id": "p", "type": "shape", "shape": "circle", "size": 3, "fill": "#e41a1c"}]}}}]}
    return n


BINA_STYLE = {**STYLE, "labels": {"mode": "rules", "classes": [
    {"name": "Yüksek", "when": "Kat > 5", "style": {"placement": "center", "size": 11, "text": "Ad"}},
]}}
BINA = node("bina", "Bina", fields=BINA_FIELDS,
            time={"start": "Tarih"},
            filter={"expression": "Kat >= 1 ve Durum <> 'yok'"},
            actions=[{"name": "Sorgu", "target": "https://ornek.example/bina/{Kat}/{Ad}"}],
            card={"title": "Bina {Ad}", "fields": ["Ad", "Kat"], "text": "Kat sayısı: {[Kat]} {{not}}"},
            tableStyles=[{"condition": "Kat > 5", "bold": True}, {"field": "Kat", "condition": "Kat >= 9", "color": "#B91C1C"}])
# The look (ADR 0564): rules with filters, a nested rule, an inline line whose width is data-defined, a library
# symbol (not walked).
BINA_STYLE["renderer"] = {"type": "rules", "rules": [
    {"id": "r1", "label": "Yüksek", "filter": "Kat > 5 ve Durum <> 'yok'", "symbols": {"fill": {"type": "fill", "layers": [
        {"id": "f", "type": "simpleFill", "color": "#1b9e77"},
        {"id": "o", "type": "simpleLine", "color": "#222222", "width": {"expr": "Kat / 10", "fallback": 0.3}}]}},
     "children": [{"id": "r2", "label": "Çok yüksek", "filter": "[Kat] >= 9"}]},
    {"id": "r3", "label": "Diğer", "isElse": True, "symbols": {"fill": {"ref": "kitaplik-dolgu"}}},
]}
BINA["style"] = copy.deepcopy(BINA_STYLE)
SETUP_LAYERS = [
    BINA,
    malik_node(),
    node("koruma", "Koruma alanı", "group", [node("sit", "Sit alanı")]),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
]


def digest(body):
    return hashlib.sha256(kc.canonical_json(body).encode("utf-8")).hexdigest()[:32]


DRAFT = {"revision": 2, "items": [
    {"type": "field", "id": "durum", "field": "Durum", "visibleWhen": "Kat > 0"},
    {"type": "field", "id": "kat", "field": "Kat", "label": "Kat sayısı",
     "checks": [{"rule": "Kat < 40", "message": "Kat 40'tan az olmalı."}]},
    {"type": "note", "id": "bilgi", "text": "Bu bina {Kat} katlı."},
    {"type": "group", "id": "ayrinti", "label": "Ayrıntı", "visibleWhen": "Kat >= 2", "items": [
        {"type": "field", "id": "not", "field": "Not", "requiredWhen": "Durum = 'kotu'"},
    ]},
    {"type": "related", "id": "malikler", "label": "Malikler", "layer": "malik", "field": "Bina"},
]}
RELEASE = {"revision": 1, "items": [
    {"type": "field", "id": "kat", "field": "Kat", "visibleWhen": "Durum <> ''"},
    {"type": "field", "id": "durum", "field": "Durum", "visibleWhen": "Kat > 0"},
]}
FORMS = [
    {"id": "bina-denetim", "name": "Bina denetimi", "layer": "bina", "draft": DRAFT,
     "releases": [{"version": 1, "digest": digest(RELEASE), "body": RELEASE}]},
    {"id": "malik-form", "name": "Malik", "layer": "malik", "draft": {"revision": 1, "items": [
        {"type": "field", "id": "bina", "field": "Bina", "lookup": {
            "layer": "bina", "key": "Ad", "label": "Kat", "matches": [{"field": "Malik adı", "record": "Ad"}], "filter": "Kat > 0"}},
    ]}},
]
RELATIONS = [{"id": "malikler", "name": "Binanın malikleri", "parent": "bina", "child": "malik", "field": "Bina"}]

X, Y = 500000, 4400000
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "bina", "attrs": {"Ad": "A Blok", "Kat": "6", "Durum": "iyi"},
     "pts": [{"x": X, "y": Y}, {"x": X + 10, "y": Y}, {"x": X + 10, "y": Y + 8}]},
    {"kind": "point", "id": 2, "layerId": "malik", "attrs": {"Bina": "A Blok", "Malik adı": "Ayşe"}, "p": {"x": X + 2, "y": Y + 2}},
]
IDS = [1, 2]


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Alanın kullanıldığı yerler",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow",
                         "forms": copy.deepcopy(FORMS), "relations": copy.deepcopy(RELATIONS)},
            "origin": {"x": X, "y": Y}, "layers": copy.deepcopy(SETUP_LAYERS), "activeLayer": "bina",
            "entities": copy.deepcopy(ENTITIES), "styles": {"items": [], "categories": []}}


# ── Where a field is named (§1) ───────────────────────────────────────────

SEP = " › "
ITEM_PROPS = [("visibleWhen", "Görünme koşulu"), ("requiredWhen", "Zorunluluk koşulu"), ("readOnlyWhen", "Salt okunurluk koşulu"),
              ("defaultExpr", "Varsayılan ifadesi"), ("calculate", "Hesap"), ("choiceFilter", "Seçenek süzgeci")]


def field_label(layers, layer, key):
    n = find(layers, layer)
    for f in (n or {}).get("fields", []):
        if f.get("id", f["name"]) == key:
            return f.get("alias", f["name"])
    return key


def find(nodes, id_):
    for n in nodes:
        if n["id"] == id_:
            return n
        hit = find(n["children"], id_)
        if hit:
            return hit
    return None


def sites(settings, layers):
    """Every place that may name a field: (layer of the names, how, path, place, value, kept)."""
    out = []
    for fi, form in enumerate(settings.get("forms", [])):
        bodies = [(f"settings/forms/{fi}/draft", f"“{form['name']}” formu", form["draft"], False)]
        for ri, r in enumerate(form.get("releases", [])):
            bodies.append((f"settings/forms/{fi}/releases/{ri}/body", f"“{form['name']}” formunun {r['version']}. sürümü", r["body"], True))
        for at, words, body, kept in bodies:
            items(out, layers, form["layer"], body["items"], f"{at}/items", words, kept)
    for ri, r in enumerate(settings.get("relations", [])):
        out.append((r["child"], "key", f"settings/relations/{ri}/field", f"“{r['name']}” ilişkisi{SEP}Alan", r["field"], False))
    layer_sites(out, layers)
    return out


def items(out, layers, layer, list_, at, words, kept):
    for k, it in enumerate(list_):
        p = f"{at}/{k}"
        t = it["type"]
        if t == "tab":
            items(out, layers, layer, it["items"], f"{p}/items", words, kept)
            continue
        if t == "field":
            name = it.get("label") or field_label(layers, layer, it["field"])
            w = f"{words}{SEP}“{name}” öğesi"
            out.append((layer, "key", f"{p}/field", f"{w}{SEP}Alan", it["field"], kept))
            for prop, said in ITEM_PROPS:
                if it.get(prop):
                    out.append((layer, "expression", f"{p}/{prop}", f"{w}{SEP}{said}", it[prop], kept))
            for c, check in enumerate(it.get("checks", [])):
                out.append((layer, "expression", f"{p}/checks/{c}/rule", f"{w}{SEP}{c + 1}. denetim", check["rule"], kept))
            lk = it.get("lookup")
            if lk:
                if lk["key"] != "$uid":
                    out.append((lk["layer"], "key", f"{p}/lookup/key", f"{w}{SEP}Listenin anahtarı", lk["key"], kept))
                if lk.get("label"):
                    out.append((lk["layer"], "key", f"{p}/lookup/label", f"{w}{SEP}Listenin etiketi", lk["label"], kept))
                for m, match in enumerate(lk.get("matches", [])):
                    out.append((layer, "key", f"{p}/lookup/matches/{m}/field", f"{w}{SEP}{m + 1}. eşleşmenin alanı", match["field"], kept))
                    out.append((lk["layer"], "key", f"{p}/lookup/matches/{m}/record", f"{w}{SEP}{m + 1}. eşleşmenin kayıt alanı", match["record"], kept))
                if lk.get("filter"):
                    out.append((lk["layer"], "expression", f"{p}/lookup/filter", f"{w}{SEP}Listenin süzgeci", lk["filter"], kept))
        elif t == "group":
            w = f"{words}{SEP}“{it.get('label') or it['id']}” grubu"
            if it.get("visibleWhen"):
                out.append((layer, "expression", f"{p}/visibleWhen", f"{w}{SEP}Görünme koşulu", it["visibleWhen"], kept))
            items(out, layers, layer, it["items"], f"{p}/items", words, kept)
        elif t == "note":
            out.append((layer, "template", f"{p}/text", f"{words}{SEP}“{it['id']}” notu{SEP}Metin", it["text"], kept))
        elif t == "related":
            w = f"{words}{SEP}“{it.get('label') or it['id']}” ilişkili kayıtları"
            out.append((it["layer"], "key", f"{p}/field", f"{w}{SEP}Alan", it["field"], kept))
            if it.get("visibleWhen"):
                out.append((layer, "expression", f"{p}/visibleWhen", f"{w}{SEP}Görünme koşulu", it["visibleWhen"], kept))
        elif t == "attachments":
            if it.get("visibleWhen"):
                w = f"{words}{SEP}“{it.get('label') or it['id']}” ekleri"
                out.append((layer, "expression", f"{p}/visibleWhen", f"{w}{SEP}Görünme koşulu", it["visibleWhen"], kept))


def renderer_sites(out, lid, v, at, place, rule):
    """The look's expressions (ADR 0564): `expr`, `exprX`, `exprY` and a rule's `filter`, the inline symbols'
    `{"expr": …}` too; keys in the order serde_json's map keeps them (sorted); a library symbol (`ref`) is not walked."""
    if isinstance(v, dict):
        if "ref" in v:
            return
        label = v.get("label")
        here = f"{place}{SEP}“{label}” kuralı" if rule and isinstance(label, str) and label else place
        for k in sorted(v):
            child, path = v[k], f"{at}/{k}"
            if (k in ("expr", "exprX", "exprY") or (rule and k == "filter")) and isinstance(child, str):
                if child:
                    out.append((lid, "expression", path, here, child, False))
                continue
            renderer_sites(out, lid, child, path, here, k in ("rules", "children"))
    elif isinstance(v, list):
        for i, item in enumerate(v):
            renderer_sites(out, lid, item, f"{at}/{i}", place, rule)


def layer_sites(out, nodes):
    for n in nodes:
        if n["type"] == "layer":
            lid, w, at = n["id"], f"“{n['name']}” katmanı", f"layers/{n['id']}"
            for i, s in enumerate(n.get("tableStyles", [])):
                rule = f"{w}{SEP}Koşullu biçim, {i + 1}. kural"
                if "field" in s:
                    out.append((lid, "name", f"{at}/tableStyles/{i}/field", f"{rule}{SEP}Alan", s["field"], False))
                out.append((lid, "expression", f"{at}/tableStyles/{i}/condition", f"{rule}{SEP}Koşul", s["condition"], False))
            if (n.get("filter") or {}).get("expression"):
                out.append((lid, "expression", f"{at}/filter/expression", f"{w}{SEP}Süzgeç", n["filter"]["expression"], False))
            if n["style"].get("renderer"):
                renderer_sites(out, lid, n["style"]["renderer"], f"{at}/style/renderer", f"{w}{SEP}Görünüş", False)
            label = (n["style"].get("label") or {})
            if label.get("text"):
                out.append((lid, "expression", f"{at}/style/label/text", f"{w}{SEP}Etiket{SEP}Metin", label["text"], False))
            for c, cls in enumerate((n["style"].get("labels") or {}).get("classes", [])):
                cw = f"{w}{SEP}“{cls['name']}” etiket sınıfı"
                if cls.get("when"):
                    out.append((lid, "expression", f"{at}/style/labels/classes/{c}/when", f"{cw}{SEP}Koşul", cls["when"], False))
                if cls["style"].get("text"):
                    out.append((lid, "expression", f"{at}/style/labels/classes/{c}/style/text", f"{cw}{SEP}Metin", cls["style"]["text"], False))
            time = n.get("time")
            if time:
                for key, said in (("start", "Başlangıç"), ("end", "Bitiş"), ("key", "Kimlik")):
                    if time.get(key):
                        out.append((lid, "name", f"{at}/time/{key}", f"{w}{SEP}Zaman{SEP}{said}", time[key], False))
            card = n.get("card")
            if card:
                if card.get("title"):
                    out.append((lid, "template", f"{at}/card/title", f"{w}{SEP}Bilgi kartı{SEP}Başlık", card["title"], False))
                for i, f in enumerate(card.get("fields", [])):
                    out.append((lid, "name", f"{at}/card/fields/{i}", f"{w}{SEP}Bilgi kartı{SEP}Alanlar", f, False))
                if card.get("text"):
                    out.append((lid, "template", f"{at}/card/text", f"{w}{SEP}Bilgi kartı{SEP}Metin", card["text"], False))
            for i, a in enumerate(n.get("actions", [])):
                out.append((lid, "template", f"{at}/actions/{i}/target", f"{w}{SEP}“{a['name']}” eylemi{SEP}Hedef", a["target"], False))
        layer_sites(out, n["children"])


def key_of(f):
    return f.get("id", f["name"])


def named(how, value, f):
    if how == "key":
        return value == key_of(f)
    if how == "name":
        return value == f["name"]
    if how == "expression":
        return names(value, f["name"])
    return template_names(value, f["name"])


def uses(settings, layers, layer, field):
    f = next(x for x in find(layers, layer)["fields"] if x["name"] == field)
    out = []
    for lid, how, path, place, value, kept in sites(settings, layers):
        if lid == layer and named(how, value, f):
            u = {"place": place, "path": path, "how": how}
            if kept:
                u["kept"] = True
            out.append(u)
    return out


# ── Library symbols (docs/adr/0635 §2) ───────────────────────────────────

# The project library: the symbol the building layer's look takes by reference, its opacity from Kat; another the look
# does not take.
LIBRARY = [
    {"id": "kitaplik-dolgu", "name": "Kitaplık dolgusu", "kind": "symbol",
     "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#CCCCCC",
                                            "opacity": {"expr": "Kat / 20", "fallback": 1}}]}},
    {"id": "baska", "name": "Kullanılmayan", "kind": "symbol",
     "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#333333",
                                            "opacity": {"expr": "Kat / 10", "fallback": 1}}]}},
]


def lib_setup():
    doc = setup()
    doc["styles"]["items"] = copy.deepcopy(LIBRARY)
    return doc


def referenced(layers, layer):
    out = []

    def walk(v):
        if isinstance(v, dict):
            if isinstance(v.get("ref"), str):
                if v["ref"] not in out:
                    out.append(v["ref"])
                return
            for c in v.values():
                walk(c)
        elif isinstance(v, list):
            for c in v:
                walk(c)
    r = find(layers, layer)["style"].get("renderer")
    if r:
        walk(r)
    return out


def library_uses(items, layers, layer, field):
    """The places in the library symbols the layer's look takes by reference that name `field` (kept: not rewritten)."""
    refs = referenced(layers, layer)
    out = []

    def walk(v, at, place):
        if isinstance(v, dict):
            if isinstance(v.get("expr"), str) and names(v["expr"], field):
                out.append({"place": place, "path": f"{at}/expr", "how": "expression", "kept": True})
            for k in sorted(v):
                if k != "expr":
                    walk(v[k], f"{at}/{k}", place)
        elif isinstance(v, list):
            for i, c in enumerate(v):
                walk(c, f"{at}/{i}", place)
    for i, item in enumerate(items):
        if item.get("id") in refs:
            walk(item, f"styles/items/{i}", f"Kitaplık{SEP}“{item.get('name', item['id'])}” simgesi{SEP}Veriye bağlı değer")
    return out


# ── Cases ─────────────────────────────────────────────────────────────────


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_, result, expect=None):
    s = {"op": op, "input": input_, "result": result}
    if expect is not None:
        s["expect"] = expect
    return s


UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def usage_case(name, layer, field):
    out = {"layer": layer, "field": field, "uses": uses(setup()["settings"], setup()["layers"], layer, field), "revision": "$current"}
    return {"name": name, "steps": [step("execute", {"layer": layer, "field": field}, completed(out), UNCHANGED),
                                    step("plan", {"layer": layer, "field": field}, completed(out), UNCHANGED)]}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), UNCHANGED)]}


def usage_cases():
    out = [
        usage_case("Kat: formun taslağı ve sürümü, liste, katmanın süzgeci, etiket sınıfı, koşullu biçim, kart ve eylem", "bina", "Kat"),
        usage_case("Durum: görünme ve zorunluluk koşulları, süzgeç; sürümdeki koşul değişmez", "bina", "Durum"),
        usage_case("Not: yalnız formun öğesi", "bina", "Not"),
        usage_case("Ad: liste anahtarı ve eşleşmenin kayıt alanı, etiket metni, kartın alanları ve başlığı, eylem", "bina", "Ad"),
        usage_case("Tarih: zamanın başlangıcı", "bina", "Tarih"),
        usage_case("Boş alan: hiçbir yerde kullanılmıyor", "bina", "Boş alan"),
        usage_case("Malik katmanının Bina alanı: ilişki, ilişkili kayıtlar ve formun öğesi", "malik", "Bina"),
        usage_case("Malik adı: listenin eşleşmesinin alanı", "malik", "Malik adı"),
    ]
    lib = lib_setup()
    for title, field in (("Kat: görünüşün kitaplık simgesi de sayılır, yeniden adlandırma onu değiştirmez", "Kat"),
                         ("Durum: kitaplık simgesi onu anmıyor", "Durum")):
        want = uses(lib["settings"], lib["layers"], "bina", field) + library_uses(lib["styles"]["items"], lib["layers"],
                                                                               "bina", field)
        o = {"layer": "bina", "field": field, "uses": want, "revision": "$current"}
        out.append({"name": title, "setup": lib, "steps": [
            step("execute", {"layer": "bina", "field": field}, completed(o), UNCHANGED)]})
    out.append(refused("çizimde olmayan katman", {"layer": "yok", "field": "Kat"}, "layer_not_found",
                       "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"layer": "koruma", "field": "Kat"}, "not_a_layer",
                       "“Koruma alanı” bir katman grubu; alanlar yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("servis katmanı", {"layer": "altlik", "field": "Kat"}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; alanı yoktur.", "layer"))
    out.append(refused("katmanda olmayan alan", {"layer": "bina", "field": "Yükseklik"}, "field_not_found",
                       "“Bina” katmanında “Yükseklik” alanı yok.", "field"))
    out.append(refused("alanı olmayan katman", {"layer": "sit", "field": "Kat"}, "field_not_found",
                       "“Sit alanı” katmanında “Kat” alanı yok.", "field"))
    out.append(refused("alan adı büyük küçük harfe duyarlı", {"layer": "bina", "field": "kat"}, "field_not_found",
                       "“Bina” katmanında “kat” alanı yok.", "field"))
    out.append(refused("başka katmanın alanı", {"layer": "malik", "field": "Kat"}, "field_not_found",
                       "“Malik” katmanında “Kat” alanı yok.", "field"))
    out.append({"name": "doğrulama: var olan alan geçer, hiçbir şey yazmaz", "steps": [
        step("validate", {"layer": "bina", "field": "Kat"}, completed(None), UNCHANGED)]})
    out.append({"name": "doğrulama: olmayan alanı söyler", "steps": [
        step("validate", {"layer": "bina", "field": "Kot"}, failed("field_not_found", "“Bina” katmanında “Kot” alanı yok.", "field"), UNCHANGED)]})
    out.append({"name": "plan: olmayan katmanı söyler", "steps": [
        step("plan", {"layer": "yok", "field": "Kat"},
             failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"), UNCHANGED)]})
    out.append(refused("boşluklu ad aynı ad değil", {"layer": "bina", "field": " Kat"}, "field_not_found",
                       "“Bina” katmanında “ Kat” alanı yok.", "field"))
    out.append(refused("katmanın adı alanın adı değil", {"layer": "malik", "field": "Ad"}, "field_not_found",
                       "“Malik” katmanında “Ad” alanı yok.", "field"))
    # The reference's own words, checked against the ADR's examples.
    assert rename("Kat > 3 ve [Kat] < 9 ve 'Kat' <> Ad", "Kat", "Kat sayısı") == "[Kat sayısı] > 3 ve [Kat sayısı] < 9 ve 'Kat' <> Ad"
    assert rename("Kat > 3", "Kat", "ve") == "[ve] > 3"
    assert template_rename("Kat {Kat} · {{Kat}} · {[Kat] * 3}", "Kat", "Kat sayısı") == "Kat {[Kat sayısı]} · {{Kat}} · {[Kat sayısı] * 3}"
    return out


# ── Renames (§2) ──────────────────────────────────────────────────────────


def set_at(doc, path, value):
    """Sets `value` at `path` of the drawing's JSON (`layers/<id>/…` by the layer's id)."""
    parts = path.split("/")
    if parts[0] == "layers":
        cur, parts = find(doc["layers"], parts[1]), parts[2:]
    else:
        cur = doc
    for p in parts[:-1]:
        cur = cur[int(p)] if isinstance(cur, list) else cur[p]
    last = parts[-1]
    if isinstance(cur, list):
        cur[int(last)] = value
    else:
        cur[last] = value


def renamed(layer, pairs):
    """The drawing after renaming `layer`'s fields by `pairs` ((old, new), in order; §2)."""
    doc = setup()
    before = setup()
    for old, new in pairs:
        if old == new:
            continue
        f = next(x for x in find(before["layers"], layer)["fields"] if x["name"] == old)
        keyed = any(lid == layer and how == "key" and value == key_of(f)
                    for lid, how, _, _, value, _ in sites(before["settings"], before["layers"]))
        for lid, how, path, _, value, kept in sites(doc["settings"], doc["layers"]):
            if lid != layer or kept:
                continue
            if how == "name" and value == old:
                set_at(doc, path, new)
            elif how == "expression":
                r = rename(value, old, new)
                if r is not None:
                    set_at(doc, path, r)
            elif how == "template":
                r = template_rename(value, old, new)
                if r is not None:
                    set_at(doc, path, r)
        fields = find(doc["layers"], layer)["fields"]
        for i, x in enumerate(fields):
            if x["name"] == old:
                nf = {}
                if "id" in x or keyed:
                    nf["id"] = x.get("id", old)
                nf.update({k: v for k, v in x.items() if k != "id"})
                nf["name"] = new
                fields[i] = nf
        for e in doc["entities"]:
            if e["layerId"] == layer and old in e["attrs"]:
                e["attrs"] = {(new if k == old else k): v for k, v in e["attrs"].items()}
    return doc


def fields_input(layer, pairs):
    fields = copy.deepcopy(find(setup()["layers"], layer)["fields"])
    for old, new in pairs:
        for x in fields:
            if x["name"] == old:
                x["name"] = new
    return {"operation": "fields", "layer": layer, "fields": fields, "renames": [{"from": o, "to": n} for o, n in pairs]}


def edited(layer, changed=True):
    return completed({"layer": layer, "changed": changed, "revision": "$current", "objects": 0, "removed": [], "kept": 0, "blocks": []})


def objects_of(doc, layer):
    return {str(e["id"]): e for e in doc["entities"] if e["layerId"] == layer}


def rename_case(name, layer, pairs, redo=False):
    after = renamed(layer, pairs)
    expect = {"ids": IDS, "canUndo": True, "revision": "changed", "layers": after["layers"],
              "settings": {"forms": after["settings"]["forms"], "relations": after["settings"]["relations"]},
              "entities": objects_of(after, layer)}
    steps = [
        step("execute", fields_input(layer, pairs), edited(layer), expect),
        {"op": "undo", "returns": "Alanlar", "expect": {"layers": setup()["layers"], "entities": objects_of(setup(), layer)}},
    ]
    if redo:
        steps.append({"op": "redo", "returns": "Alanlar", "expect": {"layers": after["layers"], "entities": objects_of(after, layer)}})
    return {"name": name, "steps": steps}


def rename_cases():
    out = [
        rename_case("Kat Kat sayısı olur: ifadeler köşeli parantezle, şablonlar, koşullu biçimin alanı ve kartın alanları yeni adla; "
                    "anahtarla anılan alan eski adını kimlik tutar; sürüm değişmez; geri alma katmanı ve nesneleri geri verir, yineleme yeniden", "bina",
                    [("Kat", "Kat sayısı")], redo=True),
        rename_case("Kat Katlar olur: tek sözcük kısa kalır", "bina", [("Kat", "Katlar")]),
        rename_case("Kat Kat_2 olur: alt çizgili tek sözcük kısa kalır", "bina", [("Kat", "Kat_2")]),
        rename_case("Kat 2. kat olur: rakamla başlayan ad köşeli paranteze alınır", "bina", [("Kat", "2. kat")]),
        rename_case("Durum ve olur: anahtar sözcük köşeli paranteze alınır", "bina", [("Durum", "ve")]),
        rename_case("Durum değil olur: Türkçe anahtar sözcük de", "bina", [("Durum", "değil")]),
        rename_case("Durum Hasar durumu olur: koşullar ve sürümün koşulu; sürümdeki değişmez", "bina", [("Durum", "Hasar durumu")]),
        rename_case("Tarih Başlangıç olur: zamanın başlangıcı izler", "bina", [("Tarih", "Başlangıç")]),
        rename_case("Boş alan Yedek olur: kullanılmayan alan kimlik almaz", "bina", [("Boş alan", "Yedek")]),
        rename_case("Ad Bina adı olur: listenin anahtarı ve kayıt alanı kimlikle kalır, etiket metni, kart ve eylem yeni adla", "bina",
                    [("Ad", "Bina adı")]),
        rename_case("Ad Ad Soyad olur: boşluklu ad köşeli parantezle", "bina", [("Ad", "Ad Soyad")]),
        rename_case("Not Açıklama olur: yalnız formun öğesi, kimlikle kalır", "bina", [("Not", "Açıklama")]),
        rename_case("iki alan birlikte: Kat K, Ad A olur", "bina", [("Kat", "K"), ("Ad", "A")]),
        rename_case("üç alan birlikte: Durum, Not ve Tarih", "bina", [("Durum", "D"), ("Not", "N"), ("Tarih", "T")]),
        rename_case("Malik katmanının Bina'sı Bina adı olur: ilişkinin ve ilişkili kayıtların anahtarı kimlikle kalır", "malik",
                    [("Bina", "Bina adı")]),
        rename_case("Malik adı Sahip olur: listenin eşleşmesinin alanı kimlikle kalır", "malik", [("Malik adı", "Sahip")]),
        rename_case("Malik katmanının iki alanı birlikte", "malik", [("Bina", "B"), ("Malik adı", "M")], redo=True),
    ]
    # docs/adr/0635 §2: the library symbol the look takes names Kat; the rename does not rewrite it and says so.
    lib = lib_setup()
    after = renamed("bina", [("Kat", "Katlar")])
    warned = {"code": "library_symbol_field", "message": (
        f"Kitaplık{SEP}“Kitaplık dolgusu” simgesi{SEP}Veriye bağlı değer “Kat” alanını anıyor (yeni adı “Katlar”). "
        "Kitaplığın simgesi katmanlar arasında paylaşılır, yeniden adlandırma onu değiştirmez: simgeyi Stil yöneticisinde "
        "düzenleyin."), "path": "renames"}
    out.append({"name": "Kat Katlar olur: görünüşün kitaplık simgesi eski adı anar, değişmez ve söylenir", "setup": lib,
                "steps": [step("execute", fields_input("bina", [("Kat", "Katlar")]),
                               {**edited("bina"), "warnings": [warned]},
                               {"ids": IDS, "canUndo": True, "revision": "changed", "layers": after["layers"]})]})
    out.append({"name": "aynı ad: hiçbir şey değişmez, adım yazılmaz", "steps": [
        step("execute", fields_input("bina", [("Tarih", "Tarih")]), edited("bina", False),
             {**UNCHANGED, "layers": setup()["layers"]})]})
    out.append({"name": "başka alanın adı: ret, hiçbir şey değişmez", "steps": [
        step("execute", fields_input("bina", [("Kat", "Durum")]),
             failed("invalid_fields", "“Durum” kimliği iki alanda var; bir alanın kimliği ya da adı başka alanın kimliği olamaz.", "fields"),
             {**UNCHANGED, "layers": setup()["layers"], "settings": {"forms": setup()["settings"]["forms"]}})]})
    out.append({"name": "plan katmanı yazılacak alanlarıyla gösterir, hiçbir şey yazmaz", "steps": [
        step("validate", fields_input("bina", [("Kat", "Katlar")]), completed(None), {**UNCHANGED, "layers": setup()["layers"]})]})
    return out


def build_usage():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.fields.usage", "commandVersion": 1,
            "title": "Alanın kullanıldığı yerler",
            "note": "Written by scripts/fixtures/field_usage_cases.py from docs/adr/0414; do not edit by hand.",
            "setup": setup(), "cases": usage_cases()}


def build_renames():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.layers.edit", "commandVersion": 1,
            "title": "Alanların adları ve kullanıldıkları yerler",
            "note": "Written by scripts/fixtures/field_usage_cases.py from docs/adr/0414 §2; do not edit by hand.",
            "setup": setup(), "cases": rename_cases()}


def main() -> int:
    bad = 0
    for path, data in ((USAGE, build_usage()), (RENAMES, build_renames())):
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad = 1
            else:
                print(f"{path.relative_to(ROOT)} güncel ({len(data['cases'])} durum)")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.relative_to(ROOT)} yazıldı ({len(data['cases'])} durum)")
    return bad


if __name__ == "__main__":
    sys.exit(main())
