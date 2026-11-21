"""Writes fixtures/commands/v1/cad.layers.classes.json: the shared cases of `cad.layers.classes` v1 (docs/adr/9105 §2).

Every expectation is worked out here from the contract's rules (crates/shared/commands/src/gis/cad_layers_classes.rs)
and from the style's meaning (docs/STYLE.md §4): a category takes the objects whose value is its value (the first
category of a value takes them; an object without a value has the empty value), Diğer değerler what no category
takes, a graduated class the numbers from its minimum up to its maximum (the last one's maximum included), a rule its
parent's objects that meet its condition, a “değilse” rule its parent's objects no enabled sibling took. Texts and
dimensions are drawn elsewhere and count nowhere; a layer's filter leaves its objects out. A class switched off still
says what it takes. The class ids, labels and conditions in the language are written again here from the same rules.
No KentOS code is used.

The data layer reads fixtures/data/v1/satirlar.gpkg, which scripts/fixtures/layer_rows_command_cases.py writes; its
rows are read from that script's own definition, not from KentOS. The desktop (crates/native/application/tests/all/
fixtures) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_classes_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_classes_command_cases.py --check   # compares the cases with the ones on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.classes.json"
ADDRESS = "$fixture:data/v1/satirlar.gpkg"


def _rows_of_the_table():
    spec = importlib.util.spec_from_file_location("rows_cases", ROOT / "scripts/fixtures/layer_rows_command_cases.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.ROWS


TABLE = _rows_of_the_table()

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
SYM = {"fill": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#d9c27a"}]}}
MARK = {"marker": {"type": "marker", "layers": [{"id": "m", "type": "shape", "shape": "circle", "fill": "#2b7a3d"}]}}

# The drawing layer's objects: (use, area) with a value left out when empty; a text that is not drawn.
POINTS = [
    ("Konut", "120"), ("Konut", "480"), ("Ticaret", "900"), ("Park", "1500"), ("", "60"),
    ("Sanayi", ""), ("Konut", "2000"), ("Park", "700"),
]


def entities():
    out = []
    for i, (use, area) in enumerate(POINTS):
        attrs = {}
        if use:
            attrs["kullanim"] = use
        if area:
            attrs["alan"] = area
        attrs["kat"] = str(i % 3)
        out.append({"kind": "point", "id": i + 1, "layerId": "noktalar", "attrs": attrs,
                    "p": {"x": 600000.0 + 10 * i, "y": 4410000.0}})
    out.append({"kind": "text", "id": 50, "layerId": "noktalar", "attrs": {"kullanim": "Konut", "alan": "100"},
                "p": {"x": 600000.0, "y": 4410050.0}, "text": "Yazı", "height": 2.5, "rotation": 0})
    return out


def drawn_points(keep=lambda attrs: True):
    """The drawing layer's drawn objects' attributes (the text left out)."""
    return [e["attrs"] for e in entities() if e["kind"] != "text" and keep(e["attrs"])]


def table_rows():
    """The data layer's rows as the classes read them: the fields as text, the area of the square."""
    out = []
    for r in TABLE:
        attrs = {"ada": str(r["ada"]), "parsel": str(r["parsel"]), "mahalle": r["mahalle"]}
        if r["kullanim"] is not None:
            attrs["kullanim"] = r["kullanim"]
        a = r["alan"]
        attrs["alan"] = str(int(a)) if float(a).is_integer() else repr(a)
        attrs["$alan"] = r["side"] * r["side"]
        out.append(attrs)
    return out


# ── The classes' meaning, written again ───────────────────────────────────────────────────────────────────

def number(text):
    try:
        return float(text)
    except (TypeError, ValueError):
        return None


def categorized_counts(rows, field, categories, other):
    first = {}
    for i, c in enumerate(categories):
        first.setdefault(c["value"], i)
    counts = [0] * (len(categories) + (1 if other else 0))
    for r in rows:
        v = r.get(field, "")
        if v in first:
            counts[first[v]] += 1
        elif other:
            counts[-1] += 1
    return counts


def graduated_counts(rows, field, classes):
    counts = [0] * len(classes)
    last = len(classes) - 1
    for r in rows:
        n = r.get(field)
        n = n if isinstance(n, float) else number(n)
        if n is None:
            continue
        for i, k in enumerate(classes):
            if n >= k["min"] and (n < k["max"] or (i == last and n <= k["max"])):
                counts[i] += 1
                break
    return counts


def rule_counts(rows, rules, conditions):
    """`conditions[id]` is a rule's condition as a Python function of a row's attributes (none: every row)."""
    out = []

    def walk(level, parent, prefix):
        own = {}
        for i, r in enumerate(level):
            if not r.get("isElse"):
                cond = conditions.get(f"{prefix}{i}")
                own[i] = [p and (cond is None or cond(row)) for p, row in zip(parent, rows)]
        taken = [any(own[i][k] for i, r in enumerate(level) if not r.get("isElse") and r.get("enabled", True))
                 for k in range(len(rows))]
        for i, r in enumerate(level):
            mask = own[i] if not r.get("isElse") else [p and not t for p, t in zip(parent, taken)]
            out.append(sum(mask))
            walk(r.get("children", []), mask, f"{prefix}{i}.")
    walk(rules, [True] * len(rows), "r")
    return out


def text(v):
    return "'" + v.replace("'", "''") + "'"


def category_expression(field, value):
    return f"({field} boş veya {field} = '')" if value == "" else f"{field} = {text(value)}"


def other_expression(field, categories):
    values = []
    for c in categories:
        if c["value"] not in values:
            values.append(c["value"])
    listed = ", ".join(text(v) for v in values)
    if "" in values:
        return f"{field} değil içinde ({listed})"
    return f"({field} değil içinde ({listed}) veya {field} boş)"


def num(x):
    return str(int(x)) if float(x).is_integer() else repr(x)


def graduated_expression(field, k, last):
    upper = "<=" if last else "<"
    return f"{field} >= {num(k['min'])} ve {field} {upper} {num(k['max'])}"


def klass(cid, label, enabled=True, depth=0, expression=None, count=None, error=None):
    out = {"id": cid, "label": label, "enabled": enabled, "depth": depth}
    if expression is not None:
        out["expression"] = expression
    if count is not None:
        out["count"] = count
    if error is not None:
        out["countError"] = error
    return out


def categorized_classes(r, counts=None):
    out = []
    for i, c in enumerate(r["categories"]):
        label = c["label"].strip() or c["value"] or "(boş)"
        out.append(klass(f"c{i}", label, c.get("enabled", True), 0, category_expression(r["expr"], c["value"]),
                         None if counts is None else counts[i]))
    if r.get("other") is not None:
        out.append(klass("other", "Diğer değerler", r.get("otherEnabled", True), 0,
                         other_expression(r["expr"], r["categories"]) if r["categories"] else None,
                         None if counts is None else counts[-1]))
    return out


def graduated_classes(r, counts=None):
    last = len(r["classes"]) - 1
    return [klass(f"g{i}", k["label"], k.get("enabled", True), 0, graduated_expression(r["expr"], k, i == last),
                  None if counts is None else counts[i]) for i, k in enumerate(r["classes"])]


def rule_classes(rules, counts=None):
    out = []

    def walk(level, parent, prefix, depth):
        taken = []
        everything = False
        for r in level:
            if r.get("enabled", True) and not r.get("isElse"):
                f = (r.get("filter") or "").strip()
                if f:
                    taken.append(f)
                else:
                    everything = True
        for i, r in enumerate(level):
            if r.get("isElse"):
                if everything:
                    own = "yanlış"
                elif taken:
                    own = "değil (" + " veya ".join(f"({f})" for f in taken) + ")"
                else:
                    own = None
            else:
                own = (r.get("filter") or "").strip() or None
            if parent is None:
                expr = own
            elif own is None:
                expr = parent
            else:
                expr = f"({parent}) ve ({own})"
            label = r.get("label", "").strip() or "(adsız kural)"
            out.append(klass(f"{prefix}{i}", label, r.get("enabled", True), depth, expr,
                             None if counts is None else counts[len(out)]))
            walk(r.get("children", []), expr, f"{prefix}{i}.", depth + 1)
    walk(rules, None, "r", 0)
    return out


# ── The document ─────────────────────────────────────────────────────────────────────────────────────────

CATEGORIZED = {
    "type": "categorized", "expr": "kullanim",
    "categories": [
        {"value": "Konut", "label": "Konut", "symbols": MARK},
        {"value": "Ticaret", "label": "", "symbols": MARK, "enabled": False},
        {"value": "", "label": "", "symbols": MARK},
        {"value": "Park", "label": "Yeşil alan", "symbols": MARK},
    ],
    "other": MARK,
}
GRADUATED = {
    "type": "graduated", "expr": "alan",
    "classes": [
        {"min": 0, "max": 500, "label": "0 – 500", "symbols": MARK},
        {"min": 500, "max": 1000, "label": "500 – 1 000", "symbols": MARK, "enabled": False},
        {"min": 1000, "max": 1500, "label": "1 000 – 1 500", "symbols": MARK},
    ],
}
RULES = {
    "type": "rules",
    "rules": [
        {"id": "1", "label": "Büyük", "filter": "alan >= 500", "symbols": MARK,
         "children": [{"id": "1a", "label": "Park", "filter": "kullanim = 'Park'", "symbols": MARK}]},
        {"id": "2", "label": "Konut", "filter": "kullanim = 'Konut'", "symbols": MARK, "enabled": False},
        {"id": "3", "label": "", "isElse": True, "symbols": MARK},
    ],
}
RULE_CONDITIONS = {
    "r0": lambda a: (number(a.get("alan")) or float("-inf")) >= 500,
    "r0.0": lambda a: a.get("kullanim") == "Park",
    "r1": lambda a: a.get("kullanim") == "Konut",
}
TABLE_CATEGORIZED = {
    "type": "categorized", "expr": "kullanim",
    "categories": [
        {"value": "Konut", "label": "Konut", "symbols": SYM},
        {"value": "Park", "label": "Park", "symbols": SYM},
        {"value": "Ticaret", "label": "Ticaret", "symbols": SYM},
    ],
    "other": SYM,
}
TABLE_GRADUATED = {
    "type": "graduated", "expr": "alan",
    "classes": [
        {"min": 400, "max": 600, "label": "400 – 600", "symbols": SYM},
        {"min": 600, "max": 900, "label": "600 – 900", "symbols": SYM},
        {"min": 900, "max": 1200, "label": "900 – 1 200", "symbols": SYM},
    ],
}
TABLE_RULES = {
    "type": "rules",
    "rules": [
        {"id": "b", "label": "Büyük parsel", "filter": "ada > 0 ve $alan > 900", "symbols": SYM},
        {"id": "d", "label": "Diğerleri", "isElse": True, "symbols": SYM},
    ],
}


def layer(lid, name, renderer=None, **more):
    style = dict(STYLE)
    if renderer is not None:
        style["renderer"] = renderer
    node = {"id": lid, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": style, "children": []}
    node.update(more)
    return node


TABLE_FIELDS = [{"name": "ada", "kind": "integer"}, {"name": "parsel", "kind": "integer"},
                {"name": "mahalle", "kind": "text"}, {"name": "alan", "kind": "decimal"},
                {"name": "kullanim", "kind": "text"}]
DATA = {"driver": "gpkg", "address": ADDRESS, "table": "parseller", "key": "fid",
        "geometry": {"column": "geom", "typeName": "POLYGON", "srid": 5256}}
POINT_FIELDS = [{"name": "kullanim", "kind": "text"}, {"name": "alan", "kind": "text"}, {"name": "kat", "kind": "text"}]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Katmanın sınıfları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 600000, "y": 4410000},
    "layers": [
        layer("noktalar", "Noktalar", CATEGORIZED, fields=POINT_FIELDS),
        layer("aralik", "Aralıklı", GRADUATED),
        layer("kural", "Kurallı", RULES),
        layer("tek", "Tek sembol", {"type": "single", "symbols": MARK}),
        layer("duz", "Düz"),
        layer("surekli", "Sürekli renk", {"type": "unclassed", "expr": "alan", "min": 0, "max": 2000,
                                          "ramp": "viridis", "symbols": MARK}),
        {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
        layer("parseller", "Parseller", TABLE_CATEGORIZED, geometry="area", data=DATA, fields=TABLE_FIELDS),
        layer("alanlar", "Alanlar", TABLE_GRADUATED, geometry="area", data=DATA, fields=TABLE_FIELDS),
        layer("buyukler", "Büyükler", TABLE_RULES, geometry="area", data=DATA, fields=TABLE_FIELDS),
        layer("altlik", "OpenStreetMap",
              service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}),
    ],
    "activeLayer": "noktalar",
    "entities": [],
    "styles": {"items": [], "categories": []},
}


def setup_with_points(layers=None):
    s = copy.deepcopy(SETUP)
    s["entities"] = entities()
    if layers is not None:
        for lid, change in layers.items():
            for n in s["layers"]:
                if n["id"] == lid:
                    change(n)
    return s


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def refused(code, message, path):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def answer(lid, kind, classes, total=None):
    out = {"layer": lid}
    if kind is not None:
        out["kind"] = kind
    out["classes"] = classes
    if total is not None:
        out["total"] = total
    return completed(out)


def step(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result}


def case(name, steps, setup=None):
    out = {"name": name, "steps": steps}
    if setup is not None:
        out["setup"] = setup
    return out


def cases():
    points = drawn_points()
    cat_counts = categorized_counts(points, "kullanim", CATEGORIZED["categories"], True)
    grad_counts = graduated_counts(points, "alan", GRADUATED["classes"])
    rule_c = rule_counts(points, RULES["rules"], RULE_CONDITIONS)
    rows = table_rows()
    t_cat = categorized_counts(rows, "kullanim", TABLE_CATEGORIZED["categories"], True)
    t_grad = graduated_counts(rows, "alan", TABLE_GRADUATED["classes"])
    t_rule = rule_counts(rows, TABLE_RULES["rules"], {"r0": lambda a: int(a["ada"]) > 0 and a["$alan"] > 900})
    with_points = setup_with_points()
    # The table not there: counting fails; listing does not need it.
    gone = copy.deepcopy(SETUP)
    for n in gone["layers"]:
        if n.get("data"):
            n["data"] = dict(n["data"], address="$fixture:data/v1/yok.gpkg")
    out = [
        case("Kategorili: her kategori ve Diğer değerler, koşullarıyla; sayı istenmeden",
             [step({"layer": "noktalar"}, answer("noktalar", "categorized", categorized_classes(CATEGORIZED)))],
             with_points),
        case("Kategorili sayılar: kapalı kategori de aldığını söyler, yazı sayılmaz",
             [step({"layer": "noktalar", "count": True},
                   answer("noktalar", "categorized", categorized_classes(CATEGORIZED, cat_counts), len(points)))],
             with_points),
    ]
    # A field in brackets stays as it is written in the conditions.
    bracket = copy.deepcopy(CATEGORIZED)
    bracket["expr"] = "[kullanim]"
    out.append(case("Köşeli parantezli alan adı koşullarda olduğu gibi kalır",
                    [step({"layer": "noktalar", "count": True},
                          answer("noktalar", "categorized", categorized_classes(bracket, cat_counts), len(points)))],
                    setup_with_points({"noktalar": lambda n: n["style"].__setitem__("renderer", bracket)})))
    # Without the empty category, Diğer değerler takes the objects without a value too.
    no_empty = copy.deepcopy(CATEGORIZED)
    del no_empty["categories"][2]
    ne_counts = categorized_counts(points, "kullanim", no_empty["categories"], True)
    out.append(case("Boş değerli kategori yokken Diğer değerler değeri olmayanları da alır",
                    [step({"layer": "noktalar", "count": True},
                          answer("noktalar", "categorized", categorized_classes(no_empty, ne_counts), len(points)))],
                    setup_with_points({"noktalar": lambda n: n["style"].__setitem__("renderer", no_empty)})))
    # Without Diğer değerler, the others are in no class.
    no_other = copy.deepcopy(CATEGORIZED)
    del no_other["other"]
    no_counts = categorized_counts(points, "kullanim", no_other["categories"], False)
    out.append(case("Diğer değerler yokken hiçbir kategorinin almadığı hiçbir sınıfta değildir",
                    [step({"layer": "noktalar", "count": True},
                          answer("noktalar", "categorized", categorized_classes(no_other, no_counts), len(points)))],
                    setup_with_points({"noktalar": lambda n: n["style"].__setitem__("renderer", no_other)})))
    # The same value twice: the first category takes the objects.
    twice = copy.deepcopy(CATEGORIZED)
    twice["categories"].append({"value": "Konut", "label": "Konut 2", "symbols": MARK})
    tw_counts = categorized_counts(points, "kullanim", twice["categories"], True)
    out.append(case("Aynı değerde iki kategori: nesneleri ilki alır",
                    [step({"layer": "noktalar", "count": True},
                          answer("noktalar", "categorized", categorized_classes(twice, tw_counts), len(points)))],
                    setup_with_points({"noktalar": lambda n: n["style"].__setitem__("renderer", twice)})))
    # A layer's filter leaves its objects out.
    filtered = drawn_points(lambda a: a.get("kat") != "0")
    f_counts = categorized_counts(filtered, "kullanim", CATEGORIZED["categories"], True)
    out.append(case("Katman süzgeci: süzgecin bıraktıkları sayılmaz",
                    [step({"layer": "noktalar", "count": True},
                          answer("noktalar", "categorized", categorized_classes(CATEGORIZED, f_counts), len(filtered)))],
                    setup_with_points({"noktalar": lambda n: n.__setitem__("filter", {"expression": "kat <> '0'"})})))
    out += [
        case("Aralıklı: sınırlar ve son aralığın üst sınırı dahil; sayı olmayan sayılmaz",
             [step({"layer": "aralik", "count": True},
                   answer("aralik", "graduated", graduated_classes(GRADUATED, grad_counts), len(points)))],
             setup_with_points() | {"entities": [dict(e, layerId="aralik") for e in entities()]}),
        case("Kurallı: alt kural üstünden sonra, derinliğiyle; kapalı kardeş değilse kuralından almaz",
             [step({"layer": "kural", "count": True},
                   answer("kural", "rules", rule_classes(RULES["rules"], rule_c), len(points)))],
             setup_with_points() | {"entities": [dict(e, layerId="kural") for e in entities()]}),
        case("Kurallı, sayısız: koşullar üst kuralın koşuluyla birleşir",
             [step({"layer": "kural"}, answer("kural", "rules", rule_classes(RULES["rules"])))]),
        case("Tek sembol: sınıfı yok, sayı katmanın nesneleri",
             [step({"layer": "tek", "count": True}, answer("tek", None, [], len(points)))],
             setup_with_points() | {"entities": [dict(e, layerId="tek") for e in entities()]}),
        case("Kendi görünüşüyle çizilen katman: sınıfı yok", [step({"layer": "duz"}, answer("duz", None, []))]),
        case("Sürekli renk: tematik işleyicinin sınıfı yok",
             [step({"layer": "surekli"}, answer("surekli", None, []))]),
        case("Veri katmanı, kategorili: satırlar kaynaktan sayılır",
             [step({"layer": "parseller", "count": True},
                   answer("parseller", "categorized", categorized_classes(TABLE_CATEGORIZED, t_cat), len(rows)))]),
        case("Veri katmanı, aralıklı: alan alanının ondalıkları",
             [step({"layer": "alanlar", "count": True},
                   answer("alanlar", "graduated", graduated_classes(TABLE_GRADUATED, t_grad), len(rows)))]),
        case("Veri katmanı, kurallı: koşul geometriyi ölçer ($alan)",
             [step({"layer": "buyukler", "count": True},
                   answer("buyukler", "rules", rule_classes(TABLE_RULES["rules"], t_rule), len(rows)))]),
        case("Veri katmanı, sayısız: kaynağa gidilmez (dosyası olmasa da)",
             [step({"layer": "parseller"}, answer("parseller", "categorized", categorized_classes(TABLE_CATEGORIZED)))],
             gone),
        case("Plan da aynı cevabı verir, hiçbir şey yazmaz",
             [step({"layer": "noktalar", "count": True},
                   answer("noktalar", "categorized", categorized_classes(CATEGORIZED, cat_counts), len(points)),
                   op="plan")],
             with_points),
    ]
    # A condition that does not compile: its count says why.
    broken = {"type": "rules", "rules": [
        {"id": "1", "label": "Bozuk", "filter": "kullanim = (", "symbols": MARK},
        {"id": "2", "label": "Hepsi", "symbols": MARK},
    ]}
    out.append(case("Derlenmeyen koşul: sayısı yerine nedeni",
                    [step({"layer": "kural", "count": True},
                          answer("kural", "rules", [
                              klass("r0", "Bozuk", True, 0, "kullanim = (", None, "$any"),
                              klass("r1", "Hepsi", True, 0, None, len(points)),
                          ], len(points)))],
                    setup_with_points({"kural": lambda n: n["style"].__setitem__("renderer", broken)})
                    | {"entities": [dict(e, layerId="kural") for e in entities()]}))
    # A class switched off with cad.layers.set: the classes say so.
    off = copy.deepcopy(CATEGORIZED)
    off["categories"][0]["enabled"] = False
    out.append(case("cad.layers.set ile kapatılan sınıf kapalı görünür",
                    [{"op": "execute", "command": "cad.layers.set",
                      "input": {"operation": "classVisible", "layer": "noktalar", "classes": ["c0"], "value": False},
                      "result": "$any"},
                     step({"layer": "noktalar"}, answer("noktalar", "categorized", categorized_classes(off)))],
                    with_points))
    out += [
        case("Ret: kaynak okunamıyor (sayılırken)",
             [step({"layer": "parseller", "count": True}, refused("source_unreachable", "$any", None))], gone),
        case("Ret: katman verilmedi", [step({"layer": "  "},
                                            refused("no_layer", "Sınıfları okunacak katmanı (layer) verin.", "layer"))]),
        case("Ret: katman yok", [step({"layer": "yok"},
                                      refused("layer_not_found", "“yok” kimlikli katman çizimde yok.", "layer"))]),
        case("Ret: grup", [step({"layer": "grup"}, refused(
            "not_a_layer", "“Grup” bir katman grubu; sınıflar katmanın stilinindir. Grubun bir katmanını verin.",
            "layer"))]),
        case("Ret: harita servisi katmanı", [step({"layer": "altlik"}, refused(
            "service_layer", "“OpenStreetMap” bir harita servisinden çizilir; sınıfları yoktur.", "layer"))]),
    ]
    return out


def document():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.classes",
        "commandVersion": 1,
        "title": "Katmanın sınıfları: kategoriler, aralıklar, kurallar, koşulları ve nesne sayıları (ADR 9105 §2)",
        "note": "ADR 9105. Beklentiler sınıfların anlamıyla burada hesaplanır: değerin ilk kategorisi, Diğer değerler, "
                "aralığın sınırları (son aralığın üst sınırı dahil), üst kuralın nesneleri, değilse kuralı; yazılar "
                "sayılmaz, katman süzgeci bıraktıklarını dışarıda tutar. Veri katmanı satirlar.gpkg'ı okur "
                "(scripts/fixtures/layer_rows_command_cases.py yazar).",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        same = OUT.exists() and OUT.read_text() == text
        print("aynı" if same else f"{OUT} farklı")
        sys.exit(0 if same else 1)
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(cases())} durum)")


if __name__ == "__main__":
    main()
