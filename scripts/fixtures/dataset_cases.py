#!/usr/bin/env python3
"""The shared cases of `cad.datasets.edit` and `cad.datasets.describe` (docs/adr/0633, plan/gis.md CBS-10).

    python3 scripts/fixtures/dataset_cases.py           # writes fixtures/commands/v1/cad.datasets.{edit,describe}.json
    python3 scripts/fixtures/dataset_cases.py --check   # compares

No KentOS code. The rules, from the ADR:

- A layer's description as a dataset: title and license a line (1-200 characters, no space round it, no control
  character), description, constraints, completeness and statement a text (1-10 000 characters, not blank), a
  responsible party with a name, an organisation or an e-mail (a line with one @, a dot in the host, no space), at most
  50 keywords none twice, the day it was brought up to date a real `YYYY-AA-GG` day, the scale's denominator above
  zero, the resolution and the accuracy metres above zero. An empty description is none. The edit is one undo step
  “Veri kümesi bilgisi”; the same one again writes nothing; a group has none.
- Describing works it out: the objects by kind, their box in the project's system and in WGS 84 (PROJ's
  `transform_bounds` with 16 points per side, the EPSG operations crs_transform_cases.py names), the system's title,
  the layer's place in the tree, the data dictionary (each field: name, alias, kind, required, unit, the values its
  list allows, the objects with a value), and for a layer a processing tool made, its record and each input's state:
  the layer gone, its content as when read or changed; stale when an input is not current or a layer it came from is
  stale; the layers it came from and the layers made from it, nearest first.
- With `records`, each layer as an OGC API Records record: a GeoJSON Feature with the record core's properties.

The layers' content digests are KentOS's (`sources::layer_digest`); the cases do not compute them: a derivation the
drawing gives names a digest no layer has (`changed`), and the ones a run writes are `$any`, their state then read.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_transform_cases as ct  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
X0, Y0 = 487000.0, 4420000.0
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
LOCAL_CRS = "Yerel (koordinat sistemi yok)"


# ── The drawing ─────────────────────────────────────────────────────────


def layer(id_, name, fields=None, kind="layer", children=None, **extra):
    n = {"id": id_, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True,
         "style": dict(STYLE), "children": children or []}
    if fields:
        n["fields"] = fields
    n.update(extra)
    return n


def square(id_, layer_id, x, y, side, attrs):
    return {"kind": "polygon", "id": id_, "layerId": layer_id, "attrs": attrs,
            "pts": [{"x": X0 + x, "y": Y0 + y}, {"x": X0 + x + side, "y": Y0 + y},
                    {"x": X0 + x + side, "y": Y0 + y + side}, {"x": X0 + x, "y": Y0 + y + side}]}


def point(id_, layer_id, x, y, attrs):
    return {"kind": "point", "id": id_, "layerId": layer_id, "attrs": attrs, "p": {"x": X0 + x, "y": Y0 + y}}


DOMAINS = [{"id": "malzeme", "name": "Yapı malzemesi", "kind": "text",
            "values": [{"code": "ah", "label": "Ahşap"}, {"code": "bt", "label": "Betonarme"},
                       {"code": "ym", "label": "Yığma"}]}]
PARSEL_FIELDS = [
    {"name": "no", "kind": "text", "alias": "Parsel no", "required": True},
    {"name": "nitelik", "kind": "text", "values": [{"code": "tarla", "label": "Tarla"}, {"code": "bahce", "label": "Bahçe"}]},
    {"name": "alan", "kind": "decimal", "scale": 2, "unit": "m²"},
]
BINA_FIELDS = [
    {"name": "malzeme", "kind": "text", "values": DOMAINS[0]["values"], "domain": "malzeme"},
    {"name": "kat", "kind": "integer"},
]
ENTITIES = [
    square(1, "parsel", 0, 0, 20, {"no": "101/1", "nitelik": "tarla", "alan": "400"}),
    square(2, "parsel", 20, 0, 20, {"no": "101/2", "alan": "400"}),
    square(3, "parsel", 0, 30, 10, {"no": "102/1", "nitelik": " "}),
    point(4, "bina", 5, 5, {"malzeme": "bt", "kat": "3"}),
    point(5, "bina", 25, 8, {"kat": "1"}),
    point(6, "ic", -10, -4, {"no": "1"}),
]
FAR = "0" * 63 + "f"


def derivation(inputs, tool="geometry.buffer", label="Tampon"):
    return {"tool": tool, "label": label, "toolVersion": 1, "parameters": {"distance": 2.0},
            "inputs": inputs, "documentRevision": "3", "engine": "KentOS 0.1.0", "at": "2026-10-25T09:30:00Z"}


def document(srid=5256, extra_layers=(), described=None):
    settings = {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "domains": DOMAINS}
    parsel = layer("parsel", "Parsel", PARSEL_FIELDS)
    if described:
        parsel["dataset"] = described
    return {
        "format": "kentos.document", "version": 1, "name": "Veri kümeleri", "settings": settings,
        "origin": {"x": X0, "y": Y0},
        "layers": [
            parsel,
            layer("bina", "Bina", BINA_FIELDS),
            layer("grup", "Kadastro", kind="group", children=[layer("ic", "İç", [{"name": "no", "kind": "text"}])]),
            *extra_layers,
        ],
        "activeLayer": "parsel", "entities": copy.deepcopy(ENTITIES), "styles": {"items": [], "categories": []},
    }


# Layers whose records the drawing gives: a buffer of the parcels recorded against a digest the parcels do not have
# (changed), one of a layer gone, and one made from the first (stale through it).
DERIVED = [
    layer("tampon", "Tampon", derivation=derivation([{"layer": "parsel", "name": "Parsel", "digest": FAR}])),
    layer("eski-tampon", "Eski tampon", derivation=derivation([{"layer": "silinen", "name": "Silinen", "digest": FAR}])),
    layer("ikinci", "İkinci tampon", derivation=derivation([{"layer": "tampon", "name": "Tampon", "digest": FAR}])),
]


# ── The reading ─────────────────────────────────────────────────────────

REGISTRY = json.loads(ct.REGISTRY.read_text(encoding="utf-8"))
SYSTEMS = {e["srid"]: e for e in REGISTRY["systems"] if e["kind"] != "local"}


def crs_title(srid):
    if srid == 0:
        return LOCAL_CRS
    return f"{SYSTEMS[srid]['name']} (EPSG:{srid})"


def extent_of(entities, layer_id):
    xs, ys = [], []
    for e in entities:
        if e["layerId"] != layer_id:
            continue
        pts = e["pts"] if e["kind"] == "polygon" else [e["p"]]
        xs += [p["x"] for p in pts]
        ys += [p["y"] for p in pts]
    if not xs:
        return None
    return [min(xs), min(ys), max(xs), max(ys)]


def wgs84(box, srid):
    if box is None or srid == 0:
        return None
    t, _, _ = ct.transformer(SYSTEMS[srid], SYSTEMS[4326])
    return list(t.transform_bounds(*box, densify_pts=16))


def dictionary(fields, entities, layer_id):
    out = []
    for f in fields or []:
        e = {"name": f["name"]}
        if "alias" in f:
            e["alias"] = f["alias"]
        e["kind"] = f["kind"]
        if f.get("required"):
            e["required"] = True
        if "unit" in f:
            e["unit"] = f["unit"]
        if "values" in f:
            e["choices"] = len(f["values"])
        e["filled"] = sum(1 for x in entities if x["layerId"] == layer_id and x["attrs"].get(f["name"], "").strip())
        out.append(e)
    return out


def leaves(nodes, path=()):
    for n in nodes:
        here = path + (n["name"],)
        if n["type"] == "layer":
            yield n, " / ".join(here)
        else:
            yield from leaves(n["children"], here)


def record_of(d, info):
    props = {"type": "dataset", "title": d["title"]}
    if "description" in info:
        props["description"] = info["description"]
    if info.get("keywords"):
        props["keywords"] = info["keywords"]
    props["language"] = {"code": "tr"}
    if "responsible" in info:
        p = info["responsible"]
        c = {}
        if "name" in p:
            c["name"] = p["name"]
        if "organisation" in p:
            c["organization"] = p["organisation"]
        if "email" in p:
            c["emails"] = [{"value": p["email"]}]
        c["roles"] = ["pointOfContact"]
        props["contacts"] = [c]
    for key, name in (("license", "license"), ("constraints", "rights"), ("updated", "updated")):
        if key in info:
            props[name] = info[key]
    if d.get("srid"):
        props["kentos:crs"] = f"http://www.opengis.net/def/crs/EPSG/0/{d['srid']}"
    for key in ("scale", "resolution", "accuracy"):
        if key in info:
            props[f"kentos:{key}"] = float(info[key])
    props["kentos:objects"] = d["objects"]
    if "derivation" in d:
        props["kentos:derivedFrom"] = {"tool": d["derivation"]["tool"],
                                       "inputs": [i["layer"] for i in d["derivation"]["inputs"]],
                                       "stale": d.get("stale", False)}
    geo = d.get("geographic")
    geometry = None if geo is None else {
        "type": "Polygon",
        "coordinates": [[[geo[0], geo[1]], [geo[2], geo[1]], [geo[2], geo[3]], [geo[0], geo[3]], [geo[0], geo[1]]]]}
    return {"id": d["layer"], "type": "Feature",
            "conformsTo": ["http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/record-core"],
            "geometry": geometry, "time": {"date": info["updated"]} if "updated" in info else None,
            "properties": props, "links": []}


def describe(doc, ids=None, records=False, digests=None):
    """The answer of `cad.datasets.describe`; `digests` the layers' digests now (absent: none matches)."""
    digests = digests or {}
    tree = list(leaves(doc["layers"]))
    by_id = {n["id"]: (n, p) for n, p in tree}
    srid = doc["settings"]["srid"]
    made_from = {}
    for n, _ in tree:
        for i in n.get("derivation", {}).get("inputs", []):
            made_from.setdefault(i["layer"], []).append(n["id"])

    def now(n):
        out = []
        for i in n.get("derivation", {}).get("inputs", []):
            if i["layer"] not in by_id:
                state = "gone"
            else:
                state = "current" if digests.get(i["layer"]) == i["digest"] else "changed"
            out.append({"layer": i["layer"], "name": i["name"], "state": state})
        return out

    def stale(id_, seen):
        if id_ in seen:
            return seen[id_]
        seen[id_] = False
        if id_ not in by_id:
            return False
        ins = now(by_id[id_][0])
        out = any(i["state"] != "current" for i in ins) or any(stale(i["layer"], seen) for i in ins)
        seen[id_] = out
        return out

    def walk(start, step):
        out, queue, seen = [], [start], {start}
        while queue:
            at = queue.pop(0)
            for nxt in step(at):
                if nxt not in seen:
                    seen.add(nxt)
                    out.append(nxt)
                    queue.append(nxt)
        return out

    def ups(at):
        n = by_id.get(at, (None,))[0]
        return [i["layer"] for i in (n or {}).get("derivation", {}).get("inputs", [])]

    chosen = ids if ids else [n["id"] for n, _ in tree if "service" not in n]
    out = []
    for id_ in chosen:
        n, path = by_id[id_]
        ents = [e for e in doc["entities"] if e["layerId"] == id_]
        kinds = {}
        for e in ents:
            kinds[e["kind"]] = kinds.get(e["kind"], 0) + 1
        box = extent_of(doc["entities"], id_)
        d = {"layer": id_, "name": n["name"], "path": path,
             "title": n.get("dataset", {}).get("title", n["name"])}
        if "dataset" in n:
            d["dataset"] = n["dataset"]
        d["objects"] = len(ents)
        if kinds:
            d["kinds"] = dict(sorted(kinds.items()))
        if box:
            d["extent"] = box
        geo = wgs84(box, srid)
        if geo:
            d["geographic"] = geo
        if srid:
            d["srid"] = srid
        d["crs"] = crs_title(srid)
        dic = dictionary(n.get("fields"), doc["entities"], id_)
        if dic:
            d["dictionary"] = dic
        if "derivation" in n:
            d["derivation"] = n["derivation"]
            d["inputs"] = now(n)
            if stale(id_, {}):
                d["stale"] = True
        up = walk(id_, ups)
        if up:
            d["upstream"] = up
        down = walk(id_, lambda at: made_from.get(at, []))
        if down:
            d["downstream"] = down
        if records:
            d["record"] = record_of(d, n.get("dataset", {}))
        out.append(d)
    return {"datasets": out, "revision": "$current"}


# ── The cases ────────────────────────────────────────────────────────────

FULL = {"title": "Kadastro parselleri", "description": "İlçenin tescilli parselleri, tapu ve kadastro verisiyle.",
        "responsible": {"name": "Harita Müdürlüğü", "organisation": "Örnek Belediyesi", "email": "harita@ornek.bel.tr"},
        "license": "CC-BY-4.0", "constraints": "Kurum içi kullanım.", "keywords": ["kadastro", "parsel"],
        "updated": "2026-10-20", "scale": 1000, "resolution": 0.5, "accuracy": 0.05,
        "completeness": "Bütün ilçe.", "statement": "TKGM'nin 2026 aktarımı."}
SHORT = {"title": "Binalar", "updated": "2025-01-31"}
IDS = [e["id"] for e in ENTITIES]
READ = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same", "dirty": False}


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, path, status="failed"):
    err = {"code": code, "message": "$any", "path": path}
    if code == "revision_conflict":
        err["revision"] = "$current"
    return {"status": status, "error": err}


def tree_with(doc, layer_id, dataset):
    """The layer tree after writing `dataset` (none: taken away)."""
    layers = copy.deepcopy(doc["layers"])

    def put(nodes):
        for n in nodes:
            if n["id"] == layer_id:
                if dataset:
                    n["dataset"] = dataset
                else:
                    n.pop("dataset", None)
            put(n["children"])
    put(layers)
    return layers


def edit_step(input_, result, expect):
    return {"op": "execute", "command": "cad.datasets.edit", "input": input_, "result": result, "expect": expect}


def edit_case(name, input_, setup=None, before=()):
    doc = setup or document()
    layer_id = input_["layer"]
    node = next(n for n, _ in leaves(doc["layers"]) if n["id"] == layer_id)
    given = input_.get("dataset") or None
    changed = node.get("dataset") != given
    expect = {"ids": IDS, "canUndo": changed, "canRedo": False, "dirty": changed,
              "revision": "changed" if changed else "same", "layers": tree_with(doc, layer_id, given)}
    out = {"layer": layer_id, "changed": changed, "revision": "$current"}
    return {"name": name, "setup": doc, "steps": list(before) + [edit_step(input_, completed(out), expect)]}


def refused_case(name, input_, code, path, setup=None, status="failed"):
    return {"name": name, "setup": setup or document(),
            "steps": [edit_step(input_, failed(code, path, status), READ)]}


def edit_cases():
    described = document(described=FULL)
    bad = lambda **k: {"layer": "parsel", "dataset": {**SHORT, **k}}  # noqa: E731
    cases = [
        edit_case("bütün parçalarıyla veri kümesi bilgisi yazılır; adım “Veri kümesi bilgisi”", {"layer": "parsel", "dataset": FULL}),
        edit_case("yalnız başlık ve gün", {"layer": "bina", "dataset": SHORT}),
        edit_case("grubun içindeki katmana", {"layer": "ic", "dataset": {"title": "İç parseller", "scale": 500}}),
        edit_case("aynısı yeniden: yazılmaz", {"layer": "parsel", "dataset": FULL}, setup=described),
        edit_case("başkasıyla değiştirilir", {"layer": "parsel", "dataset": SHORT}, setup=described),
        edit_case("verilmeyen bilgi kaldırılır", {"layer": "parsel"}, setup=described),
        edit_case("boş bilgi kaldırır", {"layer": "parsel", "dataset": {}}, setup=described),
        edit_case("bilgisi olmayan katmandan kaldırmak: yazılmaz", {"layer": "bina"}),
        edit_case("yalnız e-postalı sorumlu", {"layer": "bina", "dataset": {"responsible": {"email": "yapi@ornek.bel.tr"}}}),
        refused_case("başlık boşluk", bad(title=" "), "invalid_dataset", "dataset"),
        refused_case("başlığın başında boşluk", bad(title=" Binalar"), "invalid_dataset", "dataset"),
        refused_case("başlık 201 karakter", bad(title="a" * 201), "invalid_dataset", "dataset"),
        refused_case("lisansta denetim karakteri", bad(license="CC\u0007BY"), "invalid_dataset", "dataset"),
        refused_case("açıklama yalnız boşluk", bad(description="   "), "invalid_dataset", "dataset"),
        refused_case("sorumlu boş", bad(responsible={}), "invalid_dataset", "dataset"),
        refused_case("e-posta değil", bad(responsible={"email": "harita"}), "invalid_dataset", "dataset"),
        refused_case("e-postanın sunucusu noktasız", bad(responsible={"email": "harita@belediye"}), "invalid_dataset", "dataset"),
        refused_case("anahtar sözcük iki kez", bad(keywords=["bina", "bina"]), "invalid_dataset", "dataset"),
        refused_case("51 anahtar sözcük", bad(keywords=[f"k{i}" for i in range(51)]), "invalid_dataset", "dataset"),
        refused_case("olmayan gün", bad(updated="2026-02-30"), "invalid_dataset", "dataset"),
        refused_case("gün başka biçimde", bad(updated="25.10.2026"), "invalid_dataset", "dataset"),
        refused_case("ölçek sıfır", bad(scale=0), "invalid_dataset", "dataset"),
        refused_case("çözünürlük sıfır", bad(resolution=0.0), "invalid_dataset", "dataset"),
        refused_case("doğruluk eksi", bad(accuracy=-0.1), "invalid_dataset", "dataset"),
        refused_case("beklenen sürüm geçersiz", {"layer": "parsel", "dataset": SHORT, "expectedRevision": "07"},
                     "invalid_revision", "expectedRevision"),
        refused_case("beklenen sürüm çakışan", {"layer": "parsel", "dataset": SHORT, "expectedRevision": "999"},
                     "revision_conflict", "expectedRevision", status="conflict"),
        refused_case("katman yok", {"layer": "yok", "dataset": SHORT}, "layer_not_found", "layer"),
        refused_case("grup", {"layer": "grup", "dataset": SHORT}, "not_a_layer", "layer"),
    ]
    # Undone: the description goes, the step's name returned; redone: it comes back.
    doc = document()
    after = tree_with(doc, "parsel", FULL)
    cases.append({"name": "geri alınır ve yinelenir", "setup": doc, "steps": [
        edit_step({"layer": "parsel", "dataset": FULL}, completed({"layer": "parsel", "changed": True, "revision": "$current"}),
                  {"ids": IDS, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": after}),
        {"op": "undo", "returns": "Veri kümesi bilgisi",
         "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed", "layers": doc["layers"]}},
        {"op": "redo", "returns": "Veri kümesi bilgisi",
         "expect": {"ids": IDS, "canUndo": True, "canRedo": False, "revision": "changed", "layers": after}},
    ]})
    # Plan: the node as it would be, nothing written.
    node = copy.deepcopy(next(n for n in doc["layers"] if n["id"] == "parsel"))
    node["dataset"] = FULL
    cases.append({"name": "plan yazmaz; katmanın olacağı hâlini verir", "setup": doc, "steps": [
        {"op": "plan", "command": "cad.datasets.edit", "input": {"layer": "parsel", "dataset": FULL},
         "result": completed({"node": node, "changed": True, "revision": "$current"}), "expect": READ}]})
    return cases


def describe_step(input_, output, expect=READ, within=None):
    s = {"op": "execute", "command": "cad.datasets.describe", "input": input_, "result": completed(output),
         "expect": expect}
    if within is not None:
        s["within"] = within
    return s


def d_case(name, input_, doc, digests=None):
    out = describe(doc, input_.get("layers"), input_.get("records", False), digests)
    return {"name": name, "setup": doc, "steps": [describe_step(input_, out, within=1e-9)]}


def run_buffer(layer_id):
    return {"op": "execute", "command": "cad.processing.run",
            "input": {"tool": "geometry.buffer", "parameters": {"input": {"scope": "layer", "layerId": layer_id},
                                                                 "distance": 2}},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}}


# The parcels' box grown by the buffer's 2 m on every side: a square's buffer reaches its sides' distance exactly.
BUFFERED = [X0 - 2, Y0 - 2, X0 + 42, Y0 + 42]


def describe_cases():
    doc = document()
    described = document(described=FULL)
    derived = document(extra_layers=DERIVED)
    local = document(srid=0)
    cases = [
        d_case("bütün katmanlar ağacın sırasıyla: nesneler, türleri, kapsam ve WGS 84 kutusu, sistem, sözlük", {}, doc),
        d_case("yalnız istenen katmanlar, verilen sırayla", {"layers": ["bina", "parsel"]}, doc),
        d_case("grubun içindeki katman: yolu grupla", {"layers": ["ic"]}, doc),
        d_case("veri kümesi bilgisi: başlık bilgiden", {"layers": ["parsel"]}, described),
        d_case("OGC API Records kaydı: iletişim, lisans, gün, kapsamın dörtgeni", {"layers": ["parsel"], "records": True}, described),
        d_case("bilgisiz katmanın kaydı: başlık katmanın adı, zaman yok", {"layers": ["bina"], "records": True}, doc),
        d_case("yerel proje: WGS 84 kutusu ve EPSG kodu yok", {"layers": ["parsel", "bina"]}, local),
        d_case("yerel projede kayıt: geometrisi yok", {"layers": ["parsel"], "records": True}, local),
        d_case("nesnesi olmayan katman: kapsamı yok", {"layers": ["tampon"]}, derived),
        d_case("türetim kaydı: okunan katman sonradan değişmiş, eski", {"layers": ["tampon"]}, derived),
        d_case("girdisi silinmiş katman: gone, eski", {"layers": ["eski-tampon"]}, derived),
        d_case("eskimiş katmandan üretilen de eski; geldiği katmanlar yakından uzağa", {"layers": ["ikinci"]}, derived),
        d_case("girdinin üretilenleri yakından uzağa", {"layers": ["parsel"]}, derived),
        d_case("türetilmiş katmanın kaydı neyden üretildiğini söyler", {"layers": ["ikinci"], "records": True}, derived),
        d_case("bütün katmanlar türetilenlerle", {}, derived),
        {"name": "katman yok: ret", "setup": doc, "steps": [
            {"op": "execute", "command": "cad.datasets.describe", "input": {"layers": ["parsel", "yok"]},
             "result": failed("layer_not_found", "layers.1"), "expect": READ}]},
        {"name": "grup: ret", "setup": doc, "steps": [
            {"op": "execute", "command": "cad.datasets.describe", "input": {"layers": ["grup"]},
             "result": failed("not_a_layer", "layers.0"), "expect": READ}]},
    ]
    # A run's derivation: the buffer of the parcels says what it read; its input is current until the parcels change.
    after_run = {"ids": "$any", "canUndo": True, "canRedo": False, "revision": "changed", "dirty": True}
    same = {"ids": "$any", "canUndo": True, "canRedo": False, "revision": "same", "dirty": True}

    def made(stale_now, state):
        return {"datasets": [{
            "layer": "islem-tampon", "name": "Tampon", "path": "Tampon", "title": "Tampon", "objects": 3,
            "kinds": {"polygon": 3}, "extent": BUFFERED, "geographic": wgs84(BUFFERED, 5256), "srid": 5256,
            "crs": crs_title(5256),
            # The values the run took, the tool's defaults filled in (docs/adr/0201 §2), but its features input.
            "derivation": {"tool": "geometry.buffer", "label": "Tampon", "toolVersion": 1,
                           "parameters": {"dissolve": False, "distance": 2, "distanceField": "",
                                          "layer": {"newName": "Tampon"}, "rings": 1, "side": "both"},
                           "inputs": [{"layer": "parsel", "name": "Parsel", "digest": "$any"}],
                           "documentRevision": "0", "engine": "$any", "at": "$any"},
            "inputs": [{"layer": "parsel", "name": "Parsel", "state": state}],
            **({"stale": True} if stale_now else {}),
            "upstream": ["parsel"]}], "revision": "$current"}

    cases.append({"name": "işlem aracının açtığı katman türetim kaydını taşır; girdisi aynıyken güncel", "setup": doc,
                  "steps": [dict(run_buffer("parsel"), expect=after_run),
                            describe_step({"layers": ["islem-tampon"]}, made(False, "current"), same, within=1e-9)]})
    edit = {"op": "execute", "command": "cad.entities.set",
            "input": {"uids": ["$uidOf:1"], "operation": "attributes", "attrs": {"no": "101/9", "nitelik": "tarla", "alan": "400"}},
            "result": {"status": "completed", "output": "$any", "warnings": []},
            "expect": {"ids": "$any", "canUndo": True, "canRedo": False, "revision": "changed", "dirty": True}}
    cases.append({"name": "girdisi değişince eski: changed", "setup": doc,
                  "steps": [dict(run_buffer("parsel"), expect=after_run), edit,
                            describe_step({"layers": ["islem-tampon"]}, made(True, "changed"), same, within=1e-9)]})
    cases.append({"name": "değişiklik geri alınınca yine güncel", "setup": doc,
                  "steps": [dict(run_buffer("parsel"), expect=after_run), edit,
                            {"op": "undo", "expect": {"ids": "$any", "canUndo": True, "canRedo": True, "revision": "changed"}},
                            describe_step({"layers": ["islem-tampon"]}, made(False, "current"),
                                          {"ids": "$any", "canUndo": True, "canRedo": True, "revision": "same"},
                                          within=1e-9)]})
    cases.append({"name": "kaydın kendisi geri alınınca katmanla gider", "setup": doc,
                  "steps": [dict(run_buffer("parsel"), expect=after_run),
                            {"op": "undo", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}},
                            describe_step({}, describe(doc), {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "same"},
                                          within=1e-9)]})
    return cases


def files():
    edit = {"format": "kentos.command-cases", "version": 1, "command": "cad.datasets.edit", "commandVersion": 1,
            "title": "Veri kümesi bilgisi",
            "note": "ADR 0633 §1: katmanın veri kümesi bilgisi tek geri alma adımında; kurallar ve beklenenler "
                    "scripts/fixtures/dataset_cases.py'de KentOS kodu olmadan yazılmıştır; retlerin cümleleri "
                    "çekirdeğindir ($any).",
            "setup": document(), "cases": edit_cases()}
    desc = {"format": "kentos.command-cases", "version": 1, "command": "cad.datasets.describe", "commandVersion": 1,
            "title": "Veri kümesini tanımla",
            "note": "ADR 0633 §2–§4: kapsam (WGS 84'e PROJ'un transform_bounds'uyla, kenar başına 16 nokta), sistem, "
                    "sözlük, türetim kaydı, girdilerin durumu, eskime, bağımlılık grafı, OGC API Records kaydı; "
                    "scripts/fixtures/dataset_cases.py'de KentOS kodu olmadan. İçerik özetleri KentOS'undur ($any).",
            "setup": document(), "cases": describe_cases()}
    return [(OUT / "cad.datasets.edit.json", edit), (OUT / "cad.datasets.describe.json", desc)]


def written():
    """The GeoPackage the headless test wrote from fixtures/kcad/v2/datasets.kcad (KENTOS_WRITE_DATASETS=1): each
    table's metadata documents read with sqlite3, its description the layer's, the derivation record with the owners'
    table; GDAL opens the file and its validator finds nothing."""
    import sqlite3
    path = ROOT / "fixtures/datasets/v1/out/veri-kumeleri.gpkg"
    source = json.loads((ROOT / "fixtures/kcad/v2/datasets.json").read_text(encoding="utf-8"))
    want = {n["name"]: n for n, _ in leaves(source["layers"]) if "dataset" in n or "derivation" in n}
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    rows = c.execute(
        "SELECT r.table_name, m.md_standard_uri, m.mime_type, m.md_scope, m.metadata FROM gpkg_metadata m "
        "JOIN gpkg_metadata_reference r ON r.md_file_id = m.id ORDER BY r.table_name, m.md_standard_uri").fetchall()
    ext = {r[0] for r in c.execute("SELECT extension_name FROM gpkg_extensions")}
    c.close()
    assert "gpkg_metadata" in ext, ext
    got = {}
    for table, standard, mime, scope, text in rows:
        assert mime == "application/json" and scope == "dataset", (table, mime, scope)
        got.setdefault(table, {})[standard] = json.loads(text)
    for name, node in want.items():
        docs = got.get(name, {})
        if "dataset" in node:
            assert docs.get("https://kentos.app/dataset/1") == node["dataset"], (name, docs)
        if "derivation" in node:
            assert docs.get("https://kentos.app/derivation/1") == node["derivation"], (name, docs)
    from osgeo import ogr
    from osgeo_utils.samples import validate_gpkg
    ogr.UseExceptions()
    ds = ogr.Open(str(path))
    names = {ds.GetLayer(i).GetName() for i in range(ds.GetLayerCount())}
    assert set(want) <= names, names
    findings = validate_gpkg.check(str(path), abort_at_first_error=False) or []
    assert not findings, findings
    print(f"{path.relative_to(ROOT)}: {len(rows)} belge, {len(want)} tablo, doğrulayıcı temiz")
    return 0


def main():
    if "--written" in sys.argv[1:]:
        return written()
    outs = [(p, (json.dumps(d, ensure_ascii=False, indent=1) + "\n").encode("utf-8")) for p, d in files()]
    if "--check" in sys.argv[1:]:
        bad = [p for p, data in outs if not p.exists() or p.read_bytes() != data]
        for p in bad:
            print(f"{p.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print(f"Veri kümelerinin durumları tutarlı: {sum(len(json.loads(d)['cases']) for _, d in outs)} durum")
        return 0
    for p, data in outs:
        p.write_bytes(data)
        print(f"{p.relative_to(ROOT)}: {len(json.loads(data)['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
