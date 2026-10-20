#!/usr/bin/env python3
"""The shared cases of file.import.shp, file.import.geojson and file.import.kml (docs/adr/0563, plan/gis.md CBS-05),
without KentOS code.

    uv run --no-project --with pyproj python3 scripts/fixtures/gis_import_cases.py --files   # GDAL writes fixtures/gis-import/v1
    uv run --no-project --with pyproj python3 scripts/fixtures/gis_import_cases.py           # writes fixtures/commands/v1/file.import.{shp,geojson,kml}.json
    uv run --no-project --with pyproj python3 scripts/fixtures/gis_import_cases.py --check   # compares; GDAL's OGR reads every file again

pyproj'lu Python (PROJ'un çevirmeleri) bu betiği koşar; GDAL'ı QGIS 4.2'nin Python'unda koşan
scripts/fixtures/gis_import_ogr.py yazar ve okur (dosyaları yazar, her dosyanın köşelerini OGR'dan alır).

What is expected comes from three references that share nothing with KentOS:

- GDAL's OGR (QGIS 4.2.3, scripts/fixtures/gis_import_ogr.py) writes the files that are not hand-made (a Shapefile, a GeoJSON file and a KML/KMZ of the
  QGIS GeoPackage's tables, in the project's system and in WGS 84, which OGR moves with PROJ) and, with --check, reads
  every file here once more: the vertices OGR finds are the vertices of the objects the references give.
- tools/formats/gis.py (ADR 0046) reads the Shapefile and GeoJSON files by the reading rules, with the standard
  library only: the objects, their attributes as text, the layers' fields, the declared system.
- scripts/fixtures/kml_cases.py (ADR 0222) reads KML and KMZ with expat and moves coordinates with PROJ along the
  ADR's EPSG paths (scripts/fixtures/crs_transform_cases.py, within 1e-6 m).

What the commands add to a reading is written here from the ADR 0563 rules: the system the coordinates are taken to be
in (the input's `srid`, else the file's, else the project's) and what that asks (crs_unknown, crs_mismatch, the move
and how sure it is), where each file's layer goes (`layers`, `layer`, the drawing's layer of the same name, else a new
one in the file's group), the ids the layers and the group get, the objects' numbers, one undo step named after the
format and the file.
"""
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/formats"))
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import gis  # noqa: E402  (tools/formats/gis.py: the independent Shapefile and GeoJSON reader)
import kml_cases as K  # noqa: E402  (the independent KML reader and PROJ's moves)

DIR = ROOT / "fixtures/gis-import/v1"
GIS = "formats/v1/gis"
KML = "formats/v1/kml"
QGIS_GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"
OUTS = {
    "shp": ROOT / "fixtures/commands/v1/file.import.shp.json",
    "geojson": ROOT / "fixtures/commands/v1/file.import.geojson.json",
    "kml": ROOT / "fixtures/commands/v1/file.import.kml.json",
}
WORD = {"shp": "Shapefile", "geojson": "GeoJSON", "kml": "KML"}
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
WITHIN = 1e-6


# ── The files ───────────────────────────────────────────────────────────────────


def ogr_run(*args):
    """scripts/fixtures/gis_import_ogr.py, which opens itself in QGIS's Python. It runs in the python3 of the user's own
    PATH, without a virtual environment's marks: QGIS's Python does not find PROJ's database from under /usr/bin/python3
    or a virtual environment's."""
    venv = os.environ.get("VIRTUAL_ENV", "")
    path = os.pathsep.join(p for p in os.environ.get("PATH", "").split(os.pathsep) if "/uv/" not in p and not (venv and p.startswith(venv)))
    env = {k: v for k, v in os.environ.items() if k != "VIRTUAL_ENV"}
    env["PATH"] = path
    python = shutil.which("python3", path=path) or "python3"
    return subprocess.run([python, str(ROOT / "scripts/fixtures/gis_import_ogr.py"), *args], check=True,
                          capture_output=True, text=True, env=env).stdout


def write_files():
    """GDAL writes the files: scripts/fixtures/gis_import_ogr.py, in QGIS's Python."""
    print(ogr_run("files").strip().splitlines()[-1])


# ── What the references read ────────────────────────────────────────────────────


def fixture_path(rel):
    return ROOT / "fixtures" / rel


def stem(name):
    return name.rsplit(".", 1)[0] if "." in name else name


def ext_of(name):
    return name.rsplit(".", 1)[1].lower() if "." in name else ""


def sha(data):
    return hashlib.sha256(data).hexdigest()


def shp_sidecars(path):
    files = {".shp": path.read_bytes()}
    for e in (".shx", ".dbf", ".prj", ".cpg"):
        for c in (path.with_suffix(e), path.with_suffix(e.upper())):
            if c.is_file():
                files[e] = c.read_bytes()
                break
    return files


def prj_name(prj):
    """A .prj's system name: the first quoted word of its PROJCS or GEOGCS."""
    m = re.match(r'\s*(?:PROJCS|GEOGCS|GEOGCRS|PROJCRS)\["([^"]*)"', prj.decode("utf-8", errors="replace"))
    return m.group(1) if m else ""


def zip_layers(data):
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        return [n[:-4] for n in z.namelist() if n.lower().endswith(".shp") and not n.endswith("/")]


def zip_files(data, layer):
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        out = {}
        for n in z.namelist():
            if "." not in n:
                continue
            s, e = n.rsplit(".", 1)
            if s.lower() == layer.lower() and e.lower() in ("shp", "shx", "dbf", "prj", "cpg"):
                out["." + e.lower()] = z.read(n)
        return out


def merge_fields(into, more):
    for layer, f in more.items():
        into[layer] = f


def read_ref(rel, entry=None):
    """What the file is and what the references read: name, bytes, sha, declared, layers in order, objects, fields."""
    path = fixture_path(rel)
    data = path.read_bytes()
    name = path.name
    ext = ext_of(name)
    if ext == "shp":
        files = shp_sidecars(path)
        r = gis.read_shapefile_files(files, stem(name))
        declared = ({"srid": r["declaredSrid"], "source": "prj", "text": prj_name(files[".prj"])}
                    if ".prj" in files else None)
        objects, fields = r["objects"], r.get("fields", {})
    elif ext == "zip":
        layers = zip_layers(data)
        chosen = layers
        if entry is not None:
            wanted = entry[:-4] if entry.lower().endswith(".shp") else entry
            exact = [l for l in layers if l.lower() == wanted.lower()]
            named = [l for l in layers if l.rsplit("/", 1)[-1].lower() == wanted.lower()]
            chosen = exact or (named if len(named) == 1 else None)
            if chosen is None:
                return {"entry_error": (entry, layers, len(named))}
            chosen = [chosen[0]] if isinstance(chosen, list) else chosen
        objects, fields, declared = [], {}, None
        for l in chosen:
            files = zip_files(data, l)
            r = gis.read_shapefile_files(files, l.rsplit("/", 1)[-1])
            objects += r["objects"]
            merge_fields(fields, r.get("fields", {}))
            if declared is None and ".prj" in files:
                declared = {"srid": r["declaredSrid"], "source": "prj", "text": prj_name(files[".prj"])}
    elif ext in ("geojson", "json"):
        r = gis.read_geojson(data, stem(name))
        root = json.loads(data)
        declared = {"srid": r["declaredSrid"], "source": "geoJsonCrs" if "crs" in root else "rfc7946"}
        if "crs" in root:
            crs = root["crs"]
            props = crs.get("properties") or {}
            declared["text"] = f"EPSG:{props.get('code')}" if crs.get("type") == "EPSG" else props.get("name", "")
        else:
            declared["text"] = "RFC 7946: WGS 84 boylam, enlem (dosyada crs üyesi yok)"
        objects, fields = r["objects"], r.get("fields", {})
    else:
        r = K.read(data)
        if "error" in r:
            return {"error": r["error"], "file": name}
        e = r["expect"] if "expect" in r else r
        objects, fields = e["objects"], {}
        declared = {"srid": e["declared"]["srid"], "text": e["declared"]["text"], "source": "kml"}
        return {"file": name, "bytes": len(data), "sha": sha(data), "declared": declared,
                "layers": [l["name"] for l in e["layers"] if l["count"]], "objects": objects, "fields": fields,
                "kml": e}
    order = []
    for o in objects:
        if o["layer"] not in order:
            order.append(o["layer"])
    return {"file": name, "bytes": len(data) if ext != "shp" else len(files[".shp"]),
            "sha": sha(data) if ext != "shp" else sha(files[".shp"]), "declared": declared, "layers": order,
            "objects": objects, "fields": fields}


# ── The rules ───────────────────────────────────────────────────────────────────

FOLD = {"Ç": "C", "Ğ": "G", "İ": "I", "Ö": "O", "Ş": "S", "Ü": "U"}


def fold(text):
    return "".join(FOLD.get(c, c) for c in text.strip().upper())


def fresh_id(name, taken):
    slug = ""
    for c in fold(name).lower():
        if ("a" <= c <= "z") or ("0" <= c <= "9"):
            slug += c
        elif not slug.endswith("-"):
            slug += "-"
    slug = slug.strip("-") or "katman"
    base = out = f"import-{slug}"
    k = 2
    while out in taken:
        out = f"{base}-{k}"
        k += 1
    taken.add(out)
    return out


def layer_node(id_, name, locked=False, fields=None):
    n = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
         "style": STYLE, "children": []}
    if fields:
        n["fields"] = fields
    return n


def group_node(id_, name, children=()):
    return {"id": id_, "name": name, "type": "group", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": list(children)}


def setup(srid=5256, layers=None, name="Shapefile, GeoJSON ve KML alma", workspace="gis"):
    settings = {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000, "workspace": workspace}
    if srid == 0:
        settings["drawingUnit"] = "m"
    return {
        "format": "kentos.document", "version": 1, "name": name, "settings": settings,
        "origin": {"x": 0.0, "y": 0.0},
        "layers": layers if layers is not None else [
            layer_node("cizim", "Çizim"), layer_node("kilitli", "Kilitli", True), group_node("grup", "Grup")],
        "activeLayer": "cizim",
        "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": {"x": 487500.0, "y": 4420500.0}}],
        "styles": {"items": [], "categories": []},
    }


SETUP = setup()


def tree_ids(doc):
    out, stack = set(), list(doc["layers"])
    while stack:
        n = stack.pop()
        out.add(n["id"])
        stack.extend(n.get("children", []))
    return out


def flat_layers(doc):
    out, stack = [], list(doc["layers"])
    while stack:
        n = stack.pop(0)
        out.append(n)
        stack = list(n.get("children", [])) + stack
    return out


def rust_number(a):
    """A float as Rust's Display writes it: 1.0 → 1, 2.1 → 2.1."""
    text = repr(float(a))
    return text[:-2] if text.endswith(".0") else text


def accuracy_text(accuracy, via, unofficial):
    if not via:
        return "kesin, yalnız projeksiyon"
    text = f"±{rust_number(accuracy)} m, {via}" if accuracy is not None else f"doğruluğu bilinmiyor, {via}"
    return text + "; resmî dönüşüm değil" if unofficial else text


# ── Entities ────────────────────────────────────────────────────────────────────


def P(p):
    return {"x": p[0], "y": p[1]}


def ring(pts, zs=None):
    out = {"pts": [P(p) for p in pts]}
    if zs is not None:
        out["zs"] = zs
    return out


def area_fields(o):
    out = ring(o["pts"], o.get("zs"))
    if o.get("holes"):
        zs = o.get("holeZs")
        out["holes"] = [ring(h, zs[i] if zs and zs[i] is not None else None) for i, h in enumerate(o["holes"])]
    return out


def entity_fields(o):
    """The fields of the object a reading gives, as the entity's JSON holds them (those a case names)."""
    kind = o["kind"]
    out = {"kind": kind, "attrs": o.get("attrs", {})}
    if "label" in o:
        out["label"] = o["label"]
    if "color" in o:
        out["color"] = o["color"]
    if "lineWeight" in o:
        out["lineWeight"] = o["lineWeight"]
    if kind == "point":
        out["p"] = P(o["p"])
        if "z" in o:
            out["z"] = o["z"]
        if o.get("parts"):
            out["parts"] = [{"p": P(q["p"]), **({"z": q["z"]} if "z" in q else {})} for q in o["parts"]]
    elif kind == "line":
        out["a"], out["b"] = P(o["a"]), P(o["b"])
        for k in ("za", "zb", "ma", "mb"):
            if k in o:
                out[k] = o[k]
    elif kind == "polyline":
        out["pts"] = [P(p) for p in o["pts"]]
        for k in ("zs", "ms"):
            if k in o:
                out[k] = o[k]
        if o.get("parts"):
            out["parts"] = [{"pts": [P(p) for p in q["pts"]],
                             **{k: q[k] for k in ("zs", "ms") if k in q}} for q in o["parts"]]
    elif kind == "polygon":
        out.update(area_fields(o))
        if o.get("parts"):
            out["parts"] = [area_fields(q) for q in o["parts"]]
    else:
        raise ValueError(kind)
    return out


def declared_json(d):
    if d is None:
        return None
    out = {"text": d.get("text", "$any"), "source": d["source"]}
    if d.get("srid") is not None:
        out["srid"] = d["srid"]
    return out


# ── The step ────────────────────────────────────────────────────────────────────


def warning(code, message, path=None):
    w = {"code": code, "message": message}
    if path:
        w["path"] = path
    return w


def failed(inp, code, message, path, extra_expect=None):
    expect = {"ids": [1], "revision": "same", "canUndo": False}
    expect.update(extra_expect or {})
    return {"op": "execute", "input": inp,
            "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
            "expect": expect}


def project_srid(doc):
    return doc["settings"]["srid"]


def run(fmt, rel, doc=SETUP, *, srid=None, reproject=False, layers=None, layer=None, skip=None, group=None,
        entry=None, op="execute", losses=False, ref=None, extra_inputs=None):
    """The step importing `rel` into `doc` and what it gives, or the failing step."""
    ref = ref or read_ref(rel, entry)
    inp = {"path": f"$fixture:{rel}"}
    if entry is not None:
        inp["entry"] = entry
    if srid is not None:
        inp["srid"] = srid
    if reproject:
        inp["reproject"] = True
    if skip:
        inp["skipLayers"] = list(skip)
    if layers:
        inp["layers"] = dict(layers)
    if layer is not None:
        inp["layer"] = layer
    if group is not None:
        inp["group"] = group
    inp.update(extra_inputs or {})
    name = ref["file"]
    warnings = []
    declared = ref["declared"]
    project = project_srid(doc)
    local = project == 0
    # §2: the system the coordinates are in.
    taken = None
    if srid is not None:
        taken = srid
        if declared and declared.get("srid") is not None and declared["srid"] != srid:
            warnings.append(warning(
                "crs_overridden",
                f"{name} EPSG:{declared['srid']} diyor; srid ile EPSG:{srid} verildi: koordinatlar EPSG:{srid} sayıldı, "
                f"dönüştürülmeden (reproject verilirse yalnız EPSG:{srid}'den projenin sistemine çevrilir).", "srid"))
    elif declared is not None:
        if declared.get("srid") is None:
            return failed(inp, "crs_unknown", f"{name}'nin koordinat sistemi okunamadı (“{declared.get('text', '$any')}”). "
                                              "Koordinatların hangi sistemde olduğunu srid ile verin.", "srid")
        if declared["srid"] not in K.registry():
            return failed(inp, "crs_unknown", f"{name} koordinatların EPSG:{declared['srid']} sisteminde olduğunu "
                                              "söylüyor; bu sistem KentOS'ta tanımlı değil. Koordinatlar projenin "
                                              "sistemindeyse srid olarak onu verin, değilse dosyayı projenin "
                                              "sistemine çevirip verin.", "srid")
        taken = declared["srid"]
    else:
        warnings.append(warning(
            "crs_undeclared", f"{name} koordinat sistemi bildirmiyor; koordinatlar projenin sisteminde sayılıp "
                              "olduğu gibi alındı.", "path"))
    # Layers: where each goes (refusals before the move is tried).
    tree = flat_layers(doc)
    routes = {fold(k): v for k, v in (layers or {}).items()}
    skipped = {fold(s) for s in (skip or [])}
    plan = []  # (source, target id | None for new)
    for source in ref["layers"]:
        if fold(source) in skipped:
            continue
        target = None
        if fold(source) in routes:
            target = routes[fold(source)]
            at = f"layers.{next(k for k in (layers or {}) if fold(k) == fold(source))}"
        elif layer is not None:
            target, at = layer, "layer"
        else:
            same = [n for n in tree if n["type"] == "layer" and fold(n["name"]) == fold(source)]
            if same:
                if same[0]["locked"]:
                    return failed(inp, "layer_locked", f"Dosyanın “{source}” katmanı çizimdeki aynı adlı "
                                  f"“{same[0]['name']}” katmanına gider ama o katman kilitli. Kilidi açın, katmanı "
                                  "skipLayers ile dışarıda bırakın ya da layers ile başka bir katmana yönlendirin.",
                                  "layers")
                target = same[0]["id"]
                at = None
        if target is not None and at is not None:
            node = next((n for n in tree if n["id"] == target), None)
            if node is None:
                return failed(inp, "layer_not_found", f"“{target}” kimlikli katman çizimde yok; “{source}” "
                              "katmanının nesnelerini var olan bir katmanın kimliğine yönlendirin.", at)
            if node["type"] != "layer":
                return failed(inp, "not_a_layer", f"“{node['name']}” bir grup; nesneler yalnız katmana konur. "
                              "Grubun altındaki bir katmanı verin.", at)
            if node["locked"]:
                return failed(inp, "layer_locked", f"“{node['name']}” katmanı kilitli; kilidi Katmanlar panelinden "
                              "açın ya da başka bir katman verin.", at)
        plan.append((source, target))
    # §2: the move.
    objects = ref["objects"]
    moved = None
    mover = None
    if taken is not None and not (taken == project and not local):
        if local:
            if reproject:
                return failed(inp, "crs_mismatch", f"{name} EPSG:{taken} sisteminde; projenin koordinat sistemi yok "
                              "(yerel proje), çevrilecek bir sistem yok. Projeyi bir koordinat sisteminde açın ya da "
                              "reproject vermeyin: koordinatlar olduğu gibi alınır.", "reproject")
        elif not reproject:
            return failed(inp, "crs_mismatch", f"{name} EPSG:{taken} sisteminde, proje EPSG:{project}. Koordinatların "
                          f"projenin sistemine çevrilmesi için reproject verin ya da projeyi EPSG:{taken}'de açın.",
                          "reproject")
        else:
            reg = K.registry()
            t, accuracy, via = K.transformer(reg[taken], reg[project])
            unofficial = "ED50" in (reg[taken]["datum"], reg[project]["datum"])
            moved_objects = [K.moved_obj(o, t) for o in objects]
            mover = {"from": taken, "to": project, "via": via, "accuracy": accuracy, "unofficial": unofficial,
                     "text": accuracy_text(accuracy, via, unofficial), "moved": len(moved_objects), "left": 0}
            moved = mover
            objects = moved_objects
            if unofficial:
                warnings.append(warning(
                    "crs_unofficial", f"{name} koordinatları resmî olmayan bir dönüşümle projenin sistemine çevrildi "
                                      f"({mover['text']}); resmî dönüşümü olan bir dosya gerekiyorsa dosyayı o "
                                      "dönüşümle çevirip verin.", "reproject"))
    chosen = {s for s, _ in plan}
    counted = [o for o in objects if o["layer"] in chosen]
    if not counted:
        every = not ref["layers"]
        if every:
            return failed(inp, "nothing_to_import", f"{name} dosyasında alınacak nesne yok.", "path")
        return failed(inp, "nothing_to_import", f"{name} dosyasının seçilen katmanlarında nesne yok; skipLayers'ı "
                      "denetleyin.", "skipLayers" if not plan and skip else "path")
    if losses:
        warnings.append(warning("import_losses", "$any"))
    # ids: the new layers in order, then the group.
    taken_ids = tree_ids(doc)
    made, targets, ids = [], {}, {}
    for source, target in plan:
        if target is None:
            target = fresh_id(source, taken_ids)
            made.append({"id": target, "name": source})
        targets[source] = target
        ids[source] = target
    group_id = None
    if made:
        g = name if group is None else group.strip()
        if g:
            found = [n for n in tree if n["type"] == "group" and fold(n["name"]) == fold(g)]
            group_id = found[0]["id"] if found else fresh_id(g, taken_ids)
    output = {"file": name, "bytes": ref["bytes"], "sha256": ref["sha"], "objects": len(counted),
              "sourceLayers": ref["layers"], "targets": targets, "layers": made, "report": "$any",
              "revision": "$current"}
    if declared:
        output["declared"] = declared_json(declared)
    if taken is not None:
        output["srid"] = taken
    if moved:
        output["moved"] = {k: v for k, v in moved.items() if not (k == "accuracy" and v is None)}
    if group_id:
        output["group"] = group_id
    step = {"op": op, "input": inp, "result": {"status": "completed", "output": output, "warnings": warnings}}
    if op != "execute":
        step["expect"] = {"ids": [1], "revision": "same", "canUndo": False}
        if moved:
            step["within"] = WITHIN
        return step
    n = len(counted)
    fields = {str(2 + i): {"layerId": ids[o["layer"]], **entity_fields(o)} for i, o in enumerate(counted)}
    expect = {"ids": [1, *range(2, 2 + n)], "fields": fields, "revision": "changed", "canUndo": True}
    nodes = {}
    for m in made:
        node = {"name": m["name"], "type": "layer", "parent": group_id}
        f = ref["fields"].get(m["name"])
        if f:
            node["fields"] = f
        nodes[m["id"]] = node
    if group_id and group_id not in tree_ids(doc):
        nodes[group_id] = {"name": name if group is None else group.strip(), "type": "group", "parent": None}
    if nodes:
        expect["nodes"] = nodes
    step["expect"] = expect
    if moved:
        step["within"] = WITHIN
    return step


def case(title, *steps, doc=None):
    c = {"name": title, "steps": list(steps)}
    if doc is not None:
        c["setup"] = doc
    return c


def undo(returns, ids):
    return {"op": "undo", "returns": returns, "expect": {"ids": ids, "canUndo": False, "canRedo": True}}


def refusal(fmt, rel, code, message, path, **kw):
    """A refusal the files themselves cause: the message names the file and its path."""
    inp = {"path": f"$fixture:{rel}"}
    inp.update(kw)
    return failed(inp, code, message, path)


# ── Cases ───────────────────────────────────────────────────────────────────────

NAMED = setup(layers=[layer_node("cizim", "Çizim"), layer_node("kilitli", "Kilitli", True),
                      group_node("grup", "Grup"), layer_node("parcel-var", "PARSEL"),
                      layer_node("nokta-var", "Nokta"), group_node("parsel-grubu", "Parsel.shp")],
              name="Aynı adlı katmanlar")
LOCKED_NAMED = setup(layers=[layer_node("cizim", "Çizim"), layer_node("parcel-kilitli", "parsel", True)],
                     name="Aynı adlı kilitli katman")
LOCAL = setup(srid=0, workspace="cad", name="Yerel proje")
ED50 = setup(srid=2320, name="ED50 / TM30")


def shp_cases():
    g = lambda n: f"{GIS}/{n}"
    d = lambda n: f"gis-import/v1/{n}"
    out = [
        case("GDAL'ın parselleri: çok parçalı alan, geometrisiz satır, alanlar türleriyle (TM36)",
             run("shp", d("parsel.shp"))),
        case("plan hiçbir şey yazmaz, execute bir adımda yazar ve geri alınır",
             run("shp", d("parsel.shp"), op="plan"),
             run("shp", d("parsel.shp")),
             undo("Shapefile: parsel.shp", [1])),
        case("validate yalnız denetler",
             {"op": "validate", "input": {"path": f"$fixture:{d('parsel.shp')}"},
              "result": {"status": "completed", "output": None, "warnings": []},
              "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("GDAL'ın Z'li ve ölçülü yolları (PolyLineZ): kotlar ve ölçüler", run("shp", d("yol.shp"))),
        case("GDAL'ın Z'li noktaları", run("shp", d("nokta.shp"))),
        case("noktalar.shp: Windows-1254 .cpg'siz, N, F, L ve D alanları, Z",
             run("shp", g("noktalar.shp"))),
        case("kuyular.shp: .prj yok, çok noktalı nesne; crs_undeclared", run("shp", g("kuyular.shp"))),
        case("sistemsiz.shp: GDAL'ın .prj'siz dosyası olduğu gibi alınır", run("shp", d("sistemsiz.shp"))),
        case("Var olan katmana: layer", run("shp", d("nokta.shp"), layer="cizim")),
        case("Var olan katmana: layers dosyanın katman adıyla (büyük küçük harf ayırmadan)",
             run("shp", d("nokta.shp"), layers={"NOKTA": "cizim"})),
        case("Çizimde aynı adlı katman varsa nesneler ona gider, yeni katman açılmaz",
             run("shp", d("nokta.shp"), NAMED), doc=NAMED),
        case("Grup adı: var olan grup adıyla bulunur", run("shp", d("yol.shp"), NAMED, group="PARSEL.SHP"),
             doc=NAMED),
        case("Grup boş: yeni katman ağacın üstünde", run("shp", d("nokta.shp"), group="")),
        case("Grup verilmişse o adla açılır", run("shp", d("nokta.shp"), group="Saha ölçüsü")),
        case("skipLayers dosyanın tek katmanını bırakırsa alınacak nesne kalmaz",
             run("shp", d("nokta.shp"), skip=["nokta"])),
        case("TM36 dosya TM36 projeye çevirme istenmeden de alınır; reproject boşuna istenmişse yok sayılır",
             run("shp", d("nokta.shp"), reproject=True)),
        case("WGS 84'teki .prj: çevirme istenmeden alınmaz",
             run("shp", d("parsel-wgs84.shp"))),
        case("WGS 84'teki parseller reproject ile projenin sistemine çevrilir (PROJ, EPSG:5261)",
             run("shp", d("parsel-wgs84.shp"), reproject=True)),
        case("WGS 84'teki noktalar çevrilirken kotları olduğu gibi kalır",
             run("shp", d("nokta-wgs84.shp"), reproject=True)),
        case("WGS 84 / UTM 36N'deki alanlar (delikli, Z'li) çevrilir",
             run("shp", g("alanlarz.shp"), reproject=True)),
        case("WGS 84 / UTM 36N dosyası çevirme istenmeden alınmaz", run("shp", g("alanlarz.shp"))),
        case("ED50 / TM30'daki yollar resmî olmayan dönüşümle çevrilir: crs_unofficial, doğruluk ±2.1 m",
             run("shp", g("yollar.shp"), reproject=True, losses=True)),
        case("ED50 projesine TUREF dosyası: datum dönüşümü ve doğruluğu",
             run("shp", d("nokta.shp"), ED50, reproject=True), doc=ED50),
        case("srid dosyanın .prj'sini ezer: crs_overridden, koordinatlar verilen sistemde sayılır",
             run("shp", d("parsel-wgs84.shp"), srid=5256)),
        case("Okunamayan .prj için srid verilirse alınır",
             run("shp", g("karisik.shp"), srid=5256)),
        case("Okunamayan .prj: crs_unknown", run("shp", g("karisik.shp"))),
        case("Yerel proje her dosyanın koordinatlarını olduğu gibi alır (WGS 84 dosya dahil)",
             run("shp", d("parsel-wgs84.shp"), LOCAL), doc=LOCAL),
        case("Yerel projede reproject çevirecek sistem olmadığından reddedilir",
             run("shp", d("parsel-wgs84.shp"), LOCAL, reproject=True), doc=LOCAL),
        case("Zip: tek katmanlı arşiv", run("shp", d("parsel.zip"))),
        case("Zip: üç katmanlı arşivin her katmanı bir katman olur",
             run("shp", d("katmanlar.zip"))),
        case("Zip: entry arşivdeki yolla bir katmanı seçer",
             run("shp", d("katmanlar.zip"), entry="veri/yol.shp")),
        case("Zip: entry katmanın adıyla da seçer",
             run("shp", d("katmanlar.zip"), entry="NOKTA")),
        case("Zip: skipLayers arşivin bir katmanını bırakır",
             run("shp", d("katmanlar.zip"), skip=["parsel"])),
        case("Zip: olmayan entry", refusal(
            "shp", d("katmanlar.zip"), "entry_not_found",
            "“yok” katmanı arşivde yok; arşivdeki katmanlar: veri/parsel, veri/yol, veri/nokta.", "entry",
            entry="yok")),
        case("Zip: aynı adlı iki katman entry ister", refusal(
            "shp", d("ayni-ad.zip"), "entry_not_found",
            "Arşivde aynı adlı birden çok katman var (parsel, parsel); entry ile birinin arşivdeki yolunu verin.",
            "entry")),
        case("Zip: aynı adlı katmanlardan biri yoluyla alınır", run("shp", d("ayni-ad.zip"), entry="b/parsel")),
        case("Kilitli hedef katman", refusal("shp", d("nokta.shp"), "layer_locked",
                                              "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka "
                                              "bir katman verin.", "layer", layer="kilitli")),
        case("Aynı adlı katman kilitliyse", run("shp", d("parsel.shp"), LOCKED_NAMED), doc=LOCKED_NAMED),
        case("Hedef bir grup", refusal("shp", d("nokta.shp"), "not_a_layer",
                                       "“Grup” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı "
                                       "verin.", "layer", layer="grup")),
        case("Olmayan hedef katman", refusal("shp", d("nokta.shp"), "layer_not_found",
                                             "“yok” kimlikli katman çizimde yok; “nokta” katmanının nesnelerini var "
                                             "olan bir katmanın kimliğine yönlendirin.", "layers.nokta",
                                             layers={"nokta": "yok"})),
        case("Boş Shapefile", run("shp", d("bos.shp"))),
        case("Shapefile olmayan .shp", refusal(
            "shp", d("bozuk.shp"), "import_unreadable",
            "bozuk.shp Shapefile olarak okunamadı: “.shp” dosyası 100 baytlık başlıktan kısa (24 bayt): Shapefile değil ya da "
            "yarım kalmış.", "path")),
        case("Zip olmayan .zip", refusal(
            "shp", d("bozuk.zip"), "import_unreadable", "bozuk.zip Shapefile olarak okunamadı: Zip arşivi bozuk: sonundaki dizin kaydı bulunamadı. "
            "Arşivi yeniden oluşturup deneyin.", "path")),
        case("Shapefile'sız zip", refusal(
            "shp", d("bos.zip"), "import_unreadable",
            "bos.zip Shapefile olarak okunamadı: Zip arşivinde Shapefile (.shp) yok. Arşivde .shp, .shx ve .dbf "
            "dosyaları olmalı.", "path")),
        case("Beklenen sürüm bir tamsayı değilse", {
            "op": "execute", "input": {"path": f"$fixture:{d('parsel.shp')}", "expectedRevision": "abc"},
            "result": {"status": "failed", "error": {
                "code": "invalid_revision", "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” "
                "gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                "path": "expectedRevision"}},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("Beklenen sürüm değişmişse çatışma", {
            "op": "execute", "input": {"path": f"$fixture:{d('parsel.shp')}", "expectedRevision": "999"},
            "result": {"status": "conflict", "error": {
                "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                "revision": "$current"}},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("Olmayan dosya", refusal("shp", d("yok.shp"), "file_not_found",
                                      f"$fixture:{d('yok.shp')} yok; dosyanın yolunu denetleyin.", "path")),
        case("Göreli yol", {"op": "execute", "input": {"path": "veri/parsel.shp"},
                            "result": {"status": "failed", "error": {
                                "code": "invalid_file_path", "message": "“veri/parsel.shp” mutlak bir dosya yolu "
                                "değil; okunacak dosyanın tam yolunu verin (/klasör/ad.shp).", "path": "path"}},
                            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("Klasör", refusal("shp", "gis-import/v1", "invalid_file_path",
                               "$fixture:gis-import/v1 bir klasör; okunacak Shapefile dosyasının adını da verin.",
                               "path")),
        case("Uzantısı .shp ya da .zip olmayan dosya", refusal(
            "shp", d("parsel.dbf"), "invalid_file_path",
            f"“$fixture:{d('parsel.dbf')}” bir Shapefile'ın .shp'si ya da .zip arşivi değil; katmanın .shp "
            "dosyasının ya da arşivin yolunu verin.", "path")),
    ]
    return out


def geojson_cases():
    g = lambda n: f"{GIS}/{n}"
    d = lambda n: f"gis-import/v1/{n}"
    return [
        case("GDAL'ın RFC 7946 parselleri WGS 84'te: çevirme istenmeden alınmaz",
             run("geojson", d("parsel-rfc7946.geojson"), losses=True)),
        case("RFC 7946 parselleri reproject ile TM36'ya çevrilir (GDAL'ın yazdığı WGS 84, PROJ'un çevirdiği TM36)",
             run("geojson", d("parsel-rfc7946.geojson"), reproject=True, losses=True)),
        case("plan hiçbir şey yazmaz; execute bir adımda yazar ve geri alınır",
             run("geojson", d("parsel-rfc7946.geojson"), losses=True, reproject=True, op="plan"),
             run("geojson", d("parsel-rfc7946.geojson"), losses=True, reproject=True),
             undo("GeoJSON: parsel-rfc7946.geojson", [1])),
        case("validate yalnız denetler",
             {"op": "validate", "input": {"path": f"$fixture:{d('parsel-tm36.geojson')}"},
              "result": {"status": "completed", "output": None, "warnings": []},
              "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("crs üyesi TM36 olan dosya projenin sisteminde olduğundan çevirme istemez",
             run("geojson", d("parsel-tm36.geojson"), losses=True)),
        case("crs üyeli çizgiler: Z, M ve alanlar türleriyle", run("geojson", d("yol-tm36.geojson"))),
        case("RFC 7946 noktaları çevrilir", run("geojson", d("nokta-rfc7946.geojson"), reproject=True)),
        case("Dosyanın katman adı yoksa dosya adı: bare-polygon", run("geojson", g("bare-polygon.geojson"),
                                                                      reproject=True, losses=True)),
        case("KentOS'un kentos üyesi katmanı ve etiketi geri verir (iki katman, grup tek)",
             run("geojson", d("katmanli.geojson"))),
        case("tm.geojson: crs adı EPSG:5256, katmanlar ve etiketler, iki noktalı nesne",
             run("geojson", g("tm.geojson"))),
        case("kotlu.geojson: kotlar köşe başına", run("geojson", g("kotlu.geojson"))),
        case("features.geojson: her geometri türü, WGS 84; nesne çevrilmeden önce çevirme ister",
             run("geojson", g("features.geojson"))),
        case("features.geojson: eğrili olmayanların hepsi çevrilir", run("geojson", g("features.geojson"),
                                                                       reproject=True, losses=True)),
        case("EPSG:2320 crs üyesi (ED50 / TM30): resmî olmayan dönüşüm",
             run("geojson", g("feature-epsg.geojson"), reproject=True)),
        case("EPSG:2320 çevirme istenmeden alınmaz", run("geojson", g("feature-epsg.geojson"))),
        case("srid dosyanın crs'ini ezer", run("geojson", g("feature-epsg.geojson"), srid=5256)),
        case("Okunamayan crs adı: crs_unknown", run("geojson", d("belirsiz-crs.geojson"))),
        case("Okunamayan crs adına srid verilirse alınır", run("geojson", d("belirsiz-crs.geojson"), srid=5256)),
        case("KentOS'ta tanımlı olmayan EPSG kodu: crs_unknown", run("geojson", d("bilinmeyen-epsg.geojson"))),
        case("Yerel proje WGS 84 dosyanın koordinatlarını olduğu gibi alır",
             run("geojson", d("nokta-rfc7946.geojson"), LOCAL), doc=LOCAL),
        case("Yerel projede reproject reddedilir", run("geojson", d("nokta-rfc7946.geojson"), LOCAL,
                                                       reproject=True), doc=LOCAL),
        case("layer var olan katmana", run("geojson", d("parsel-tm36.geojson"), losses=True, layer="cizim")),
        case("layers kentos üyesinin katman adıyla yönlendirir",
             run("geojson", d("katmanli.geojson"), layers={"Bina": "cizim"})),
        case("skipLayers bir katmanı bırakır", run("geojson", d("katmanli.geojson"), skip=["Bina"])),
        case("Hedef kilitli", refusal("geojson", d("katmanli.geojson"), "layer_locked",
                                      "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir "
                                      "katman verin.", "layers.Parsel", layers={"Parsel": "kilitli"})),
        case("Nesnesiz FeatureCollection", run("geojson", d("bos.geojson"), reproject=True)),
        case("Hedef katman yok", refusal("geojson", d("parsel-tm36.geojson"), "layer_not_found",
                                         "“yok” kimlikli katman çizimde yok; “parsel” katmanının nesnelerini var olan "
                                         "bir katmanın kimliğine yönlendirin.", "layer", layer="yok")),
        case("Hedef bir grup", refusal("geojson", d("parsel-tm36.geojson"), "not_a_layer",
                                       "“Grup” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı "
                                       "verin.", "layer", layer="grup")),
        case("Beklenen sürüm bir tamsayı değilse", {
            "op": "execute", "input": {"path": f"$fixture:{d('parsel-tm36.geojson')}", "expectedRevision": "abc"},
            "result": {"status": "failed", "error": {
                "code": "invalid_revision", "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” "
                "gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                "path": "expectedRevision"}},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("Bozuk JSON", refusal("geojson", d("bozuk.geojson"), "import_unreadable",
                                   "bozuk.geojson GeoJSON olarak okunamadı: GeoJSON okunamadı: satır 2: dosya bir değerin yerinde bitiyor. Dosya "
                                   "bozuk ya da yarım kalmış olabilir; hiçbir şey alınmadı.", "path")),
        case("Olmayan dosya", refusal("geojson", d("yok.geojson"), "file_not_found",
                                      f"$fixture:{d('yok.geojson')} yok; dosyanın yolunu denetleyin.", "path")),
        case("Göreli yol", {"op": "execute", "input": {"path": "veri/parsel.geojson"},
                            "result": {"status": "failed", "error": {
                                "code": "invalid_file_path", "message": "“veri/parsel.geojson” mutlak bir dosya "
                                "yolu değil; okunacak dosyanın tam yolunu verin (/klasör/ad.geojson).",
                                "path": "path"}},
                            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("Beklenen sürüm değişmişse çatışma", {
            "op": "execute", "input": {"path": f"$fixture:{d('parsel-tm36.geojson')}", "expectedRevision": "999"},
            "result": {"status": "conflict", "error": {
                "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                "revision": "$current"}},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
    ]


def kml_cases():
    k = lambda n: f"{KML}/{n}"
    d = lambda n: f"gis-import/v1/{n}"
    return [
        case("parsel.kml: klasörler katman, biçemin rengi ve kalınlığı, ad, veriler; WGS 84 çevirme ister",
             run("kml", k("parsel.kml"))),
        case("parsel.kml reproject ile TM36'ya çevrilir (PROJ, EPSG:5261); kotlar kalır",
             run("kml", k("parsel.kml"), reproject=True)),
        case("plan hiçbir şey yazmaz; execute bir adımda yazar ve geri alınır",
             run("kml", k("parsel.kml"), reproject=True, op="plan"),
             run("kml", k("parsel.kml"), reproject=True),
             undo("KML: parsel.kml", [1])),
        case("validate yalnız denetler",
             {"op": "validate", "input": {"path": f"$fixture:{k('parsel.kml')}", "reproject": True},
              "result": {"status": "completed", "output": None, "warnings": []},
              "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("alanlar.kmz: KMZ'nin doc.kml'i, adsız belge, göreli yükseklik notu", run("kml", k("alanlar.kmz"),
                                                                                    reproject=True)),
        case("gezi.kml: klasör yolları, izler, karışık MultiGeometry, zaman", run("kml", k("gezi.kml"),
                                                                              reproject=True, losses=True)),
        case("GDAL'ın LIBKML parselleri (TM36'dan WGS 84'e GDAL çevirdi, TM36'ya KentOS çevirir)",
             run("kml", d("parsel.kml"), reproject=True)),
        case("GDAL'ın LIBKML noktaları, kotlarıyla", run("kml", d("nokta.kml"), reproject=True)),
        case("GDAL'ın LIBKML yolları", run("kml", d("yol.kml"), reproject=True)),
        case("GDAL'ın KMZ'si: üç klasör", run("kml", d("hepsi.kmz"), reproject=True)),
        case("layer var olan katmana", run("kml", d("nokta.kml"), reproject=True, layer="cizim")),
        case("layers klasörün adıyla yönlendirir", run("kml", k("parsel.kml"), reproject=True,
                                                        layers={"Kadastro": "cizim"})),
        case("Çizimde aynı adlı katman varsa nesneler ona gider", run("kml", d("nokta.kml"), NAMED,
                                                                    reproject=True), doc=NAMED),
        case("skipLayers bir klasörü bırakır", run("kml", k("parsel.kml"), reproject=True, skip=["Saha"])),
        case("skipLayers hepsini bırakırsa alınacak nesne kalmaz", run("kml", k("alanlar.kmz"), reproject=True,
                                                                       skip=["KML", "Bina"])),
        case("group adı", run("kml", k("parsel.kml"), reproject=True, group="Google Earth")),
        case("srid KML'in WGS 84'ünü ezer: crs_overridden", run("kml", k("parsel.kml"), srid=5256)),
        case("Yerel proje koordinatları olduğu gibi alır", run("kml", k("parsel.kml"), LOCAL), doc=LOCAL),
        case("Yerel projede reproject reddedilir", run("kml", k("parsel.kml"), LOCAL, reproject=True), doc=LOCAL),
        case("ED50 projesine: datum dönüşümü ve resmî olmayan uyarısı", run("kml", k("alanlar.kmz"), ED50,
                                                                           reproject=True), doc=ED50),
        case("Hedef kilitli", refusal("kml", k("parsel.kml"), "layer_locked",
                                      "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir "
                                      "katman verin.", "layers.Kadastro", reproject=True,
                                      layers={"Kadastro": "kilitli"})),
        case("Hedef bir grup", refusal("kml", k("parsel.kml"), "not_a_layer",
                                       "“Grup” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı "
                                       "verin.", "layer", reproject=True, layer="grup")),
        case("Nesnesiz belge", run("kml", d("bos.kml"))),
        case("XML olmayan dosya", refusal("kml", k("bozuk.kml"), "import_unreadable",
                                          "bozuk.kml KML olarak okunamadı: KML okunamadı: dosya XML değil; hiçbir şey "
                                          "alınmadı.", "path")),
        case("kml olmayan kök", refusal("kml", k("kok.kml"), "import_unreadable",
                                        "kok.kml KML olarak okunamadı: KML değil: kök öğe “gpx” (kml bekleniyordu); "
                                        "hiçbir şey alınmadı.", "path")),
        case("İçinde KML olmayan KMZ", refusal("kml", k("kml-yok.kmz"), "import_unreadable",
                                               "kml-yok.kmz KML olarak okunamadı: KMZ arşivinde .kml dosyası yok; "
                                               "hiçbir şey alınmadı.", "path")),
        case("Hedef katman yok", refusal("kml", k("parsel.kml"), "layer_not_found",
                                         "“yok” kimlikli katman çizimde yok; “Kadastro” katmanının nesnelerini var "
                                         "olan bir katmanın kimliğine yönlendirin.", "layers.Kadastro",
                                         reproject=True, layers={"Kadastro": "yok"})),
        case("Beklenen sürüm bir tamsayı değilse", {
            "op": "execute", "input": {"path": f"$fixture:{k('parsel.kml')}", "expectedRevision": "abc"},
            "result": {"status": "failed", "error": {
                "code": "invalid_revision", "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” "
                "gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                "path": "expectedRevision"}},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("Beklenen sürüm değişmişse çatışma", {
            "op": "execute", "input": {"path": f"$fixture:{k('parsel.kml')}", "expectedRevision": "999"},
            "result": {"status": "conflict", "error": {
                "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                "revision": "$current"}},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("Olmayan dosya", refusal("kml", k("yok.kml"), "file_not_found",
                                      f"$fixture:{k('yok.kml')} yok; dosyanın yolunu denetleyin.", "path")),
        case("Göreli yol", {"op": "execute", "input": {"path": "veri/yerler.kml"},
                            "result": {"status": "failed", "error": {
                                "code": "invalid_file_path", "message": "“veri/yerler.kml” mutlak bir dosya yolu "
                                "değil; okunacak dosyanın tam yolunu verin (/klasör/ad.kml).", "path": "path"}},
                            "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
    ]


CASES = {"shp": shp_cases, "geojson": geojson_cases, "kml": kml_cases}
TITLES = {
    "shp": "Shapefile içe aktar: dosyalar ve zip, alanlar türleriyle, sistem sorusu, çevirme",
    "geojson": "GeoJSON içe aktar: RFC 7946 ve crs üyesi, kentos katmanları, sistem sorusu, çevirme",
    "kml": "KML/KMZ içe aktar: klasörler katman, biçemler, sistem sorusu, çevirme",
}


def build(fmt):
    return {
        "format": "kentos.command-cases", "version": 1, "command": f"file.import.{fmt}", "commandVersion": 1,
        "title": TITLES[fmt],
        "note": "ADR 0563. Dosyaları GDAL (QGIS 4.2.3) yazar ya da elle yazılmıştır (fixtures/formats/v1); beklenenler "
                "bağımsız başvurulardan kurulur: Shapefile ve GeoJSON tools/formats/gis.py'den, KML "
                "scripts/fixtures/kml_cases.py'den, çevirme PROJ'dan (±1e-6 m); komutun sistem sorusu, katman "
                "yönlendirmesi ve kimlikleri ADR'nin kuralından. Bağımsız başvuru: scripts/fixtures/gis_import_cases.py; "
                "elle düzenlemeyin.",
        "setup": SETUP,
        "cases": CASES[fmt](),
    }


# ── OGR reads every file again ──────────────────────────────────────────────────


def vertices_of(o, acc):
    for key in ("p", "a", "b"):
        if key in o:
            acc.add((round(o[key][0], 9), round(o[key][1], 9)))
    for p in o.get("pts", []):
        acc.add((round(p[0], 9), round(p[1], 9)))
    for h in o.get("holes", []):
        for p in h:
            acc.add((round(p[0], 9), round(p[1], 9)))
    for part in o.get("parts", []):
        vertices_of(part, acc)


def ogr_json(*args):
    return json.loads(ogr_run(*args).strip().splitlines()[-1])


def check_files():
    """The vertices OGR finds in every file are the vertices of the objects the references read."""
    rels = [f"gis-import/v1/{p.name}" for p in sorted(DIR.iterdir())
            if ext_of(p.name) in ("shp", "zip", "geojson", "kml", "kmz")
            and p.name not in ("bozuk.shp", "bozuk.zip", "bos.zip", "bozuk.geojson", "bos.shp", "bos.geojson",
                               "bos.kml")]
    rels += [f"{GIS}/{n}" for n in ("noktalar.shp", "parseller.shp", "yollar.shp", "alanlarz.shp", "yollarz.shp")]
    rels += [f"{KML}/{n}" for n in ("parsel.kml", "gezi.kml", "alanlar.kmz")]
    found = ogr_json("vertices", *rels)
    problems = []
    for rel in rels:
        got = found[rel]
        if isinstance(got, str):
            problems.append(f"{rel}: OGR okuyamadı: {got}")
            continue
        ref = read_ref(rel)
        want = set()
        for o in ref["objects"]:
            vertices_of(o, want)
        got = {tuple(v) for v in got}
        if want - got:
            problems.append(f"{rel}: OGR'ın bulmadığı köşe(ler): {sorted(want - got)[:3]}")
        # GDAL's own files hold nothing the references leave out (the hand-made files do, by the ADR's rules).
        if got - want and rel.startswith("gis-import/") and ext_of(rel) not in ("kml", "kmz"):
            problems.append(f"{rel}: başvurunun okumadığı köşe(ler): {sorted(got - want)[:3]}")
    # OGR moved the WGS 84 parcels from TM36 itself and wrote them to 7 decimals of a degree (1 cm): moving them back
    # lands on the original within 2 cm.
    reg = K.registry()
    t, _, _ = K.transformer(reg[4326], reg[5256])
    original = [tuple(v) for v in ogr_json("original")]
    back = gis.read_geojson((DIR / "parsel-rfc7946.geojson").read_bytes(), "parsel-rfc7946")
    moved = set()
    for o in back["objects"]:
        vertices_of(K.moved_obj(o, t), moved)
    for x, y in moved:
        if not any(abs(x - ox) < 2e-2 and abs(y - oy) < 2e-2 for ox, oy in original):
            problems.append(f"parsel-rfc7946.geojson: ({x}, {y}) GeoPackage'ın köşelerinde yok (2 cm)")
            break
    return problems


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    texts = {fmt: json.dumps(build(fmt), ensure_ascii=False, indent=1) + "\n" for fmt in CASES}
    if "--check" in sys.argv:
        problems = []
        for fmt, text in texts.items():
            out = OUTS[fmt]
            if not out.exists() or out.read_text("utf-8") != text:
                problems.append(f"{out.relative_to(ROOT)} eski: betiği --check'siz çalıştırın")
        problems += check_files()
        if problems:
            print("\n".join(problems), file=sys.stderr)
            return 1
        counts = ", ".join(f"{fmt} {len(json.loads(t)['cases'])}" for fmt, t in texts.items())
        print(f"Shapefile, GeoJSON ve KML içe aktarma durumları tutarlı ({counts}); OGR her dosyayı okudu.")
        return 0
    for fmt, text in texts.items():
        OUTS[fmt].write_text(text, "utf-8")
        print(f"yazıldı: {OUTS[fmt].relative_to(ROOT)} ({len(json.loads(text)['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
