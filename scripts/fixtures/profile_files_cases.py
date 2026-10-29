"""Writes the shared cases of an institution's own data profile (docs/adr/0558; plan/gis.md CBS-34's remainder):

- fixtures/commands/v1/cad.profiles.import.json (`cad.profiles.import` v1: a profile or a mapping the user brings, kept
  in the project's `profiller` folder beside the drawing or inside the drawing's own file, docs/adr/0735)
- fixtures/commands/v1/cad.profiles.list.json (`cad.profiles.list` v1: KentOS's profiles, the drawing's and the folder's)
- fixtures/commands/v1/cad.profiles.remove.json (`cad.profiles.remove` v1: a profile or a mapping taken out of the drawing,
  docs/adr/0735)
- fixtures/commands/v1/cad.profiles.migrate.json (`cad.profiles.migrate` v1: the data moved from one release of a profile
  to a later one by the profile's own steps)
- fixtures/profiles/v1/files/ (the profiles, mappings and broken files the cases bring or find in the folder) and
  fixtures/profiles/v1/park-r1.json (the park project at release 1, for the desktop's pictures)

The rules are written here again from the ADR and the contract (crates/shared/commands/src/cad_profile_files.rs), the
tree, the objects and the settings as plain JSON, without KentOS code: the five steps run one after another on the
drawing as it is then (a layer found by Turkish-folded names, a field renamed with its values, a value through the new
field's rule by layer_field_cases.py's `check`, values recoded, a code retired and the linked fields' copies written
anew), the values that did not fit grouped by value, the notes and the warnings in their words. The files the commands
write are read back by later steps of the cases (`cad.profiles.list`, `cad.profiles.check`). The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/profile_files_cases.py           # writes the files
    python3 scripts/fixtures/profile_files_cases.py --check   # compares them with the ones on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
REF = ROOT / "fixtures/profiles/v1"
FILES = REF / "files"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PC = _load("profile_cases")
FIELDS = PC.FIELDS
node, step, completed, failed, VALID = PC.node, PC.step, PC.completed, PC.failed, PC.VALID
INVALID_REVISION, CONFLICT = PC.INVALID_REVISION, PC.CONFLICT
IMPORT, LIST, MIGRATE = "cad.profiles.import", "cad.profiles.list", "cad.profiles.migrate"
REMOVE = "cad.profiles.remove"
FIX = "$fixture:profiles/v1/files/"
STYLE = PC.NEW_STYLE
UNIT_KIND = PC.KIND_LABEL
STEP_NAME = "Profil göçü"
_FOLDER = [0]


def folder():
    _FOLDER[0] += 1
    return f"pf{_FOLDER[0]:02d}"


# ── The institution's profile: three releases ────────────────────────────────

AGAC = ["Peyzaj", "Ağaç"]
YESIL = ["Peyzaj", "Yeşil alan"]
TUR = {"id": "agac-tur", "name": "Ağaç türü", "kind": "text", "values": [
    {"code": "cinar", "label": "Çınar"}, {"code": "mese", "label": "Meşe"}, {"code": "palmiye", "label": "Palmiye"},
    {"code": "diger", "label": "Diğer"}]}
SULAMA = {"id": "sulama", "name": "Sulama", "kind": "text", "values": [
    {"code": "damla", "label": "Damla"}, {"code": "yagmur", "label": "Yağmurlama"}, {"code": "yok", "label": "Yok"}]}
YAS = {"name": "Yas", "alias": "Yaş", "kind": "integer", "min": "0", "max": "500"}
TUR_FIELD = {"name": "Tur", "alias": "Tür", "kind": "text", "domain": "agac-tur"}
SULAMA_FIELD = {"name": "Sulama", "kind": "text", "domain": "sulama"}

M12 = {"from": 1, "to": 2, "title": "Ağaç katmanı yeni adlarıyla", "steps": [
    {"kind": "renameLayer", "layer": ["Peyzaj", "Ağaçlar"], "to": "Ağaç"},
    {"kind": "renameField", "layer": AGAC, "from": "No", "to": "AgacNo"},
    {"kind": "renameField", "layer": AGAC, "from": "Alan", "to": "YesilAlan"},
    {"kind": "changeFieldType", "layer": AGAC, "field": YAS, "onFail": "keep"},
]}
M23 = {"from": 2, "to": 3, "title": "Türler ve sulama listeleri; yeşil alan türleri", "steps": [
    {"kind": "changeFieldType", "layer": AGAC, "field": TUR_FIELD, "onFail": "keep"},
    {"kind": "retireCode", "domain": "agac-tur", "code": "palmiye", "replaceWith": "diger"},
    {"kind": "changeFieldType", "layer": AGAC, "field": SULAMA_FIELD, "onFail": "clear"},
    {"kind": "mapValues", "layer": YESIL, "field": "Tip", "map": [
        {"from": "Bitki", "to": "orman-bitkisi", "whenField": "Bolge", "whenEquals": "orman"},
        {"from": "Bitki", "to": "bahce-bitkisi", "whenField": "Bolge", "whenEquals": "bahce"},
        {"from": "Bitki", "to": "bitki"}]},
]}

KURUM = {
    "format": "kentos.data-profile", "version": 1, "name": "park-bahce", "title": "Park ve bahçeler profili",
    "description": "Ağaçlar ve yeşil alanlar; üç sürüm.", "basis": "Kurumun kendi çalışma profili; resmî bir şema değildir.",
    "release": 3, "migrations": [M12, M23], "domains": [TUR, SULAMA],
    "classes": [
        {"id": "agac", "title": "Ağaç", "theme": "Peyzaj", "layer": AGAC, "color": "#30A46C", "geometry": "point", "key": "AgacNo",
         "fields": [{"name": "AgacNo", "kind": "text", "required": True}, TUR_FIELD, YAS, SULAMA_FIELD,
                    {"name": "YesilAlan", "kind": "text"}]},
        {"id": "yesil", "title": "Yeşil alan", "theme": "Peyzaj", "layer": YESIL, "color": "#46A758", "geometry": "area",
         "fields": [{"name": "Ad", "kind": "text"}, {"name": "Bolge", "alias": "Bölge", "kind": "text"}, {"name": "Tip", "kind": "text"}]},
    ],
    "relations": [{"id": "agac-yesil", "name": "Ağacın alanı", "parent": "yesil", "child": "agac", "field": "YesilAlan"}],
}
KURUM_R2 = {**KURUM, "release": 2, "migrations": [M12]}
KURUM_R1 = {**KURUM, "release": 1, "migrations": []}
KURUM_MAP = PC.mapping("park-bahce-kurum", "Park ve bahçeler eşlemesi", "park-bahce", [
    {"class": "agac", "fields": PC.renames([("AgacNo", "NO")])}])
# docs/adr/0647 §4: the institution's own layer, field and code names.
INST_MAP = PC.mapping("park-bahce-kurum2", "Kurumun ağaçları", "park-bahce", [
    {"class": "agac", "layer": ["Peyzaj", "Kurum ağaçları"], "fields": PC.renames([("AgacNo", "NO"), ("Tur", "TUR_KOD")]),
     "codes": [PC.code_map("Tur", [("cinar", "CN"), ("mese", "MS"), ("palmiye", "PL"), ("diger", "DG")])]}])
BROKEN_MAP = PC.mapping("park-bahce-bozuk", "Bozuk eşleme", "park-bahce", [{"class": "orman", "layer": ["Orman"]}])
GHOST_MAP = PC.mapping("hayalet-esleme", "Profili olmayan eşleme", "hayalet", [{"class": "x", "layer": ["X"]}])

BASE_FILES = {
    "park-bahce.profil.json": KURUM,
    "park-bahce-r2.profil.json": KURUM_R2,
    "park-bahce-r1.profil.json": KURUM_R1,
    "park-bahce-kurum.eslem.json": KURUM_MAP,
    "park-bahce-kurum2.eslem.json": INST_MAP,
    "park-bahce-bozuk.eslem.json": BROKEN_MAP,
    "hayalet-esleme.eslem.json": GHOST_MAP,
    "profil-surum2.json": {"format": "kentos.data-profile", "version": 2, "name": "peyzaj"},
    "profil-surumsuz.json": {"format": "kentos.data-profile", "name": "peyzaj"},
    "esleme-surum2.json": {"format": "kentos.profile-mapping", "version": 2, "name": "x"},
    "taninmayan.json": {"format": "kentos.baska-sey", "version": 1},
    "profil-sinifsiz.json": {**PC.SMALL, "name": "sinifsiz", "classes": []},
    "profil-adi-yanlis.json": {**PC.SMALL, "name": "Peyzaj Profili"},
    "profil-yerlesik-adli.json": {**PC.SMALL, "name": "tucbs-temel"},
    "profil-basliksiz.json": {k: v for k, v in PC.SMALL.items() if k != "title"},
    "esleme-basliksiz.json": {**PC.SIVAS, "title": " "},
    "esleme-sinifsiz.json": {**PC.SIVAS, "classes": []},
    "esleme-gecersiz.json": {**PC.SIVAS, "name": "gecersiz", "classes": [{"class": "orman", "layer": ["Orman"]}]},
}
RAW_FILES = {"bozuk.json": "{ bu bir JSON degil\n"}


# ── Words ────────────────────────────────────────────────────────────────────

def words(path):
    return " › ".join(path)


def kind_word(f):
    return UNIT_KIND[f["kind"]].lower()


def trim(s):
    return FIELDS.trim(s)


def tmp(f, name=None):
    return f"$tmp:{f}/cizim.kcad" if name is None else f"$tmp:{f}/profiller/{name}"


def put(f, **files):
    """Steps that put fixture files into the folder of case `f` (name in the folder → file under fixtures/profiles/v1/files)."""
    return [step("copyFile", **{"from": FIX + src, "to": tmp(f, name)}) for name, src in files.items()]


def place_drawing(f):
    """A step that puts the case's drawing file in place (its folder exists then); the commands read only the folder."""
    return step("copyFile", **{"from": FIX + "park-bahce.profil.json", "to": f"$tmp:{f}/cizim.kcad"})


# ── The files a case finds in the folder, read as the list reads them ────────

def read_project(f_files):
    """name → (suffix kind, parsed dict | raw text) of the files copied into the folder: (kind, name) → content."""
    return f_files


def parse_content(content):
    """(profile | mapping | problem text) as `parse` in cad_profile_files.rs reads a file's JSON value."""
    if isinstance(content, str):
        try:
            json.loads(content)
        except ValueError:
            return ("error", "$any")
    value = content if isinstance(content, dict) else json.loads(content)
    fmt = value.get("format")
    version = value.get("version")
    if fmt == "kentos.data-profile":
        if version != 1:
            return ("error", f"Profilin biçim sürümü {version if version is not None else 'yok'}; bu KentOS 1. sürümü okur.")
        if "title" not in value:
            return ("error", "$any")
        why = profile_problem(value)
        if why:
            return ("error", f"Veri profili geçersiz: {why}")
        return ("profile", value)
    if fmt == "kentos.profile-mapping":
        if version != 1:
            return ("error", f"Eşlemenin biçim sürümü {version if version is not None else 'yok'}; bu KentOS 1. sürümü okur.")
        why = mapping_shape(value)
        if why:
            return ("error", f"Eşleme geçersiz: {why}")
        return ("mapping", value)
    return ("error", "Dosyanın biçimi tanınmıyor: veri profili için “kentos.data-profile”, eşleme için “kentos.profile-mapping” yazılı olmalı.")


def mapping_shape(m):
    if not PC.plain_name(m["name"]):
        return f"“{m['name']}” eşleme adı olamaz: küçük harf, rakam ve tire, en çok 64."
    if not m["title"].strip():
        return "Eşlemenin başlığı boş olamaz."
    if not m["classes"] or len(m["classes"]) > 256:
        return "Eşlemede 1 ile 256 arası sınıf olur."
    return None


def profile_problem(p):
    """The few problems of a profile these cases bring (the contract's `profile_problem`, crates/shared/commands/src/cad_profiles.rs)."""
    if not p["name"].strip() or not p["title"].strip():
        return "Profilin adı ve başlığı boş olamaz."
    if not p.get("classes") or len(p["classes"]) > 64:
        return "Profilde 1 ile 64 arası sınıf olur."
    return None


def release_of(p):
    return p.get("release", 1)


# ── List ─────────────────────────────────────────────────────────────────────

def builtin_item():
    return {"kind": "profile", "source": "builtin", "name": "tucbs-temel", "title": PC.PROFILE["title"], "release": 1,
            "classes": len(PC.PROFILE["classes"]), "migrations": 0}


def list_item(f, name, kind, content, others):
    """What `cad.profiles.list` says of one file of the folder: `others` finds the profiles by name (the folder's and the built-in)."""
    suffix = ".profil.json" if kind == "profile" else ".eslem.json"
    item = {"kind": kind, "source": "project", "name": name, "title": "", "release": 0, "classes": 0, "migrations": 0,
            "path": tmp(f, name + suffix)}
    got, value = parse_content(content)
    if got == "error":
        item["problem"] = value
    elif kind == "profile" and got == "mapping":
        item["problem"] = f"{item['path']} bir veri profili değil, eşleme."
    elif kind == "mapping" and got == "profile":
        item["problem"] = f"{item['path']} bir eşleme değil, veri profili."
    elif got == "profile" and value["name"] != name:
        item["problem"] = f"{item['path']} dosyasının içindeki profilin adı “{value['name']}”; dosya adıyla aynı olmalı."
    elif got == "mapping" and value["name"] != name:
        item["problem"] = f"{item['path']} dosyasının içindeki eşlemenin adı “{value['name']}”; dosya adıyla aynı olmalı."
    elif got == "profile":
        item.update({"title": value["title"], "release": release_of(value), "classes": len(value["classes"]),
                     "migrations": len(value.get("migrations", []))})
    else:
        item.update({"title": value["title"], "classes": len(value["classes"]), "profile": value["profile"]})
        p = others(value["profile"])
        why = PC.mapping_problem(value, p) if p is not None else None
        if why:
            item["problem"] = why
    if not item["title"]:
        del item["title"]
    return item


def listing(f, files, carried=(), drawing=True):
    """The listing's output for the folder holding `files` (file name → content), name order, a profile before a mapping;
    the drawing's own (`carried`: (kind, name, content)) before them, and a folder file whose name and kind the drawing
    carries too `shadowed` (docs/adr/0735). Without the drawing's file (`drawing` false) no folder is read."""
    own = {(k, n): c for k, n, c in carried}

    def find_profile(name):
        if name == "tucbs-temel":
            return PC.PROFILE
        if ("profile", name) in own:
            got, value = carried_parse("profile", name, own[("profile", name)])
            return value if got == "profile" else None
        if not drawing:
            return None
        content = files.get(name + ".profil.json")
        if content is None:
            return None
        got, value = parse_content(content)
        return value if got == "profile" and value["name"] == name else None
    items = [builtin_item()]
    for kind, name, content in sorted(carried, key=lambda t: (t[1], t[0] == "mapping")):
        items.append(drawing_item(kind, name, content, find_profile))
    if not drawing:
        return {"items": items}
    found = []
    for file, content in files.items():
        for suffix, kind in ((".profil.json", "profile"), (".eslem.json", "mapping")):
            if file.endswith(suffix) and file[: -len(suffix)]:
                found.append((file[: -len(suffix)], kind == "mapping", kind, content))
    for name, _, kind, content in sorted(found, key=lambda t: (t[0], t[1])):
        item = list_item(f, name, kind, content, find_profile)
        if (kind, name) in own:
            item["shadowed"] = True
        items.append(item)
    return {"items": items, "folder": f"$tmp:{f}/profiller"}


# ── The drawing's own (docs/adr/0735) ────────────────────────────────────────

def carried_text(value):
    """The text a case keeps a profile or a mapping by inside the drawing: indented JSON ending in a newline, the folder
    files' form (the commands read any JSON text)."""
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def compact(value):
    """A copy's text written without indentation: the same document as another text."""
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def carry(doc, *entries):
    """`doc` carrying `entries` ((kind, name, value | raw text)) in the order the file format keeps them: by name, a
    profile before a mapping."""
    d = copy.deepcopy(doc)
    files = [{"kind": k, "name": n, "text": v if isinstance(v, str) else carried_text(v)} for k, n, v in entries]
    files.sort(key=lambda x: (x["name"], x["kind"] == "mapping"))
    d["settings"]["profileFiles"] = files
    return d


def kept(*entries):
    """The settings' `profileFiles` after a step: the kinds and names the drawing carries, in order, any text; none when
    it carries none (the field is not written)."""
    files = sorted(({"kind": k, "name": n, "text": "$any"} for k, n in entries), key=lambda x: (x["name"], x["kind"] == "mapping"))
    return {"profileFiles": files or None}


def carried_parse(kind, name, content):
    """(profile | mapping | error, value) as the commands read the drawing's copy: its text as JSON, then as a folder
    file, then its kind and the name inside against the ones it is kept by."""
    got, value = parse_content(content)
    if got == "error":
        return got, value
    if got == "profile" and kind == "profile" and value["name"] != name:
        return "error", f"Çizimin içindeki “{name}” profilinin kendi adı “{value['name']}”; aynı olmalı."
    if got == "mapping" and kind == "mapping" and value["name"] != name:
        return "error", f"Çizimin içindeki “{name}” eşlemesinin kendi adı “{value['name']}”; aynı olmalı."
    if got == "mapping" and kind == "profile":
        return "error", f"Çizimin içindeki “{name}” bir veri profili değil, eşleme."
    if got == "profile" and kind == "mapping":
        return "error", f"Çizimin içindeki “{name}” bir eşleme değil, veri profili."
    return got, value


def drawing_item(kind, name, content, find_profile):
    """What `cad.profiles.list` says of one the drawing carries: no path; a mapping checked with its profile, found as
    the commands find it."""
    item = {"kind": kind, "source": "drawing", "name": name, "title": "", "release": 0, "classes": 0, "migrations": 0}
    got, value = carried_parse(kind, name, content)
    if got == "error":
        item["problem"] = value
    elif got == "profile":
        item.update({"title": value["title"], "release": release_of(value), "classes": len(value["classes"]),
                     "migrations": len(value.get("migrations", []))})
    else:
        item.update({"title": value["title"], "classes": len(value["classes"]), "profile": value["profile"]})
        p = find_profile(value["profile"])
        why = PC.mapping_problem(value, p) if p is not None else None
        if why:
            item["problem"] = why
    if not item["title"]:
        del item["title"]
    return item


def content_of(src):
    if src in RAW_FILES:
        return RAW_FILES[src]
    return BASE_FILES.get(src) or {"peyzaj.profil.json": PC.SMALL, "sivas-bel.eslem.json": PC.SIVAS,
                                   "peyzaj-kurum.eslem.json": PC.MAP_SMALL}[src]


def list_case(name, files, drawing=True, plan=False):
    """A case that puts `files` (file name in the folder → fixture file) into the folder and lists it."""
    f = folder()
    steps = put(f, **files)
    contents = {n: content_of(s) for n, s in files.items()}
    out = listing(f, contents)
    inp = {"drawing": tmp(f)} if drawing else {}
    if not drawing:
        out = {"items": [builtin_item()]}
    if plan:
        steps.append(step("plan", inp, completed(out), {"revision": "same"}))
    steps.append(step("execute", inp, completed(out), {"revision": "same"}))
    return {"name": name, "setup": PC.empty_doc(), "steps": steps}


def list_cases():
    cases = []
    cases.append(list_case("Çizimin dosyası verilmeyince yalnız KentOS'un profili listelenir", {}, drawing=False))
    # The folder that is not there.
    f = folder()
    cases.append({"name": "Klasör yoksa yalnız yerleşik profil gelir; klasör yolu söylenir", "setup": PC.empty_doc(), "steps": [
        step("execute", {"drawing": tmp(f)}, completed({"items": [builtin_item()], "folder": f"$tmp:{f}/profiller"}))]})
    cases.append(list_case("Projenin profili ve eşlemesi: önce profil, sonra eşleme; sürüm, sınıf ve göç sayıları",
                           {"park-bahce.profil.json": "park-bahce.profil.json", "park-bahce-kurum.eslem.json": "park-bahce-kurum.eslem.json"},
                           plan=True))
    cases.append(list_case("Adlar sırasıyla: peyzaj, park-bahce, sivas-bel",
                           {"sivas-bel.eslem.json": "sivas-bel.eslem.json", "peyzaj.profil.json": "peyzaj.profil.json",
                            "park-bahce.profil.json": "park-bahce.profil.json"}))
    cases.append(list_case("Aynı adlı profil ve eşleme: profil önce",
                           {"peyzaj.eslem.json": "peyzaj-kurum.eslem.json", "peyzaj.profil.json": "peyzaj.profil.json"}))
    cases.append(list_case("Sürümü 2 ve 1 olan profiller göç sayılarıyla",
                           {"a-r2.profil.json": "park-bahce-r2.profil.json", "b-r1.profil.json": "park-bahce-r1.profil.json"}))
    cases.append(list_case("Okunmayan dosya düşmez, nedeniyle gelir (bozuk JSON)", {"bozuk.profil.json": "bozuk.json"}))
    cases.append(list_case("Biçimi tanınmayan dosya", {"x.profil.json": "taninmayan.json"}))
    cases.append(list_case("Biçim sürümü 2 olan profil", {"x.profil.json": "profil-surum2.json"}))
    cases.append(list_case("Biçim sürümü olmayan profil", {"x.profil.json": "profil-surumsuz.json"}))
    cases.append(list_case("Biçim sürümü 2 olan eşleme", {"x.eslem.json": "esleme-surum2.json"}))
    cases.append(list_case("Sınıfsız profil: geçersiz", {"sinifsiz.profil.json": "profil-sinifsiz.json"}))
    cases.append(list_case("Başlığı olmayan profil: okunmaz", {"x.profil.json": "profil-basliksiz.json"}))
    cases.append(list_case("Başlığı boş eşleme", {"x.eslem.json": "esleme-basliksiz.json"}))
    cases.append(list_case("Sınıfsız eşleme", {"x.eslem.json": "esleme-sinifsiz.json"}))
    cases.append(list_case("İçindeki ad dosyanın adından farklı profil", {"baska.profil.json": "peyzaj.profil.json"}))
    cases.append(list_case("İçindeki ad dosyanın adından farklı eşleme", {"baska.eslem.json": "sivas-bel.eslem.json"}))
    cases.append(list_case("Eşleme dosyası profil olarak adlandırılmış", {"x.profil.json": "sivas-bel.eslem.json"}))
    cases.append(list_case("Profil dosyası eşleme olarak adlandırılmış", {"x.eslem.json": "peyzaj.profil.json"}))
    cases.append(list_case("Eşleme KentOS'un profili için: profille denetlenir, tutuyorsa sorun yok",
                           {"sivas-bel.eslem.json": "sivas-bel.eslem.json"}))
    cases.append(list_case("Eşleme profiline uymuyor: sorun nedeniyle söylenir", {"park-bahce-bozuk.eslem.json": "park-bahce-bozuk.eslem.json",
                                                                                "park-bahce.profil.json": "park-bahce.profil.json"}))
    cases.append(list_case("Eşlemenin profili bulunamıyor: denetlenmeden listelenir", {"hayalet-esleme.eslem.json": "hayalet-esleme.eslem.json"}))
    cases.append(list_case("Eşlemenin profili projede: profille denetlenir",
                           {"park-bahce-kurum.eslem.json": "park-bahce-kurum.eslem.json", "park-bahce.profil.json": "park-bahce.profil.json"}))
    # Other files and folders in the folder are not listed.
    cases.append(list_case("Başka dosyalar listelenmez (yalnız .profil.json ve .eslem.json)",
                           {"notlar.txt": "bozuk.json", "peyzaj.profil.json.bak": "peyzaj.profil.json",
                            "peyzaj.profil.json": "peyzaj.profil.json"}))
    # A relative drawing path has no folder; so does an empty one.
    cases.append({"name": "Göreli ya da boş çizim yolunun klasörü yok: yalnız yerleşik profil", "setup": PC.empty_doc(), "steps": [
        step("execute", {"drawing": "cizim.kcad"}, completed({"items": [builtin_item()]})),
        step("execute", {"drawing": "  "}, completed({"items": [builtin_item()]}))]})
    cases.append({"name": "Doğrulama", "setup": PC.empty_doc(), "steps": [step("validate", {}, VALID)]})
    return cases + carried_list_cases()


def carried_list_case(name, carried, files=None, plan=False):
    """A case whose drawing carries `carried` ((kind, name, value | raw text)); with `files` the folder holds them too."""
    setup = carry(PC.empty_doc(), *carried)
    steps = []
    if files is None:
        inp, out = {}, listing(None, {}, carried, drawing=False)
    else:
        f = folder()
        steps = put(f, **files)
        inp, out = {"drawing": tmp(f)}, listing(f, {n: content_of(src) for n, src in files.items()}, carried)
    if plan:
        steps.append(step("plan", inp, completed(out), {"revision": "same"}))
    steps.append(step("execute", inp, completed(out), {"revision": "same", "dirty": False}))
    return {"name": name, "setup": setup, "steps": steps}


def carried_list_cases():
    """What the drawing carries inside its own file (docs/adr/0735): after KentOS's, before the folder's."""
    small = PC.SMALL
    return [
        carried_list_case("Çizimin taşıdığı profil ve eşleme: KentOS'unkinden sonra, adlarıyla; çizimin dosyası gerekmez",
                          [("mapping", "peyzaj-kurum", PC.MAP_SMALL), ("profile", "peyzaj", small)], plan=True),
        carried_list_case("Çizimdekiler önce, klasördekiler sonra; aynı ad ve türdeki klasör dosyası gölgede (shadowed)",
                          [("profile", "peyzaj", small)],
                          {"peyzaj.profil.json": "peyzaj.profil.json", "park-bahce.profil.json": "park-bahce.profil.json"}),
        carried_list_case("Aynı adlı eşleme klasörde: yalnız aynı türdeki gölgede kalır",
                          [("profile", "peyzaj", small)], {"peyzaj.eslem.json": "peyzaj-kurum.eslem.json"}),
        carried_list_case("Çizimin taşıdığı okunmayan metin düşmez, nedeniyle gelir", [("profile", "bozuk", RAW_FILES["bozuk.json"])]),
        carried_list_case("Çizimdeki profilin kendi adı farklı", [("profile", "baska", small)]),
        carried_list_case("Profil diye taşınan eşleme", [("profile", "sivas-bel", PC.SIVAS)]),
        carried_list_case("Eşleme diye taşınan profil", [("mapping", "peyzaj", small)]),
        carried_list_case("Biçim sürümü 2 olan taşınan profil", [("profile", "peyzaj", content_of("profil-surum2.json"))]),
        carried_list_case("Çizimdeki eşleme KentOS'un profiliyle denetlenir", [("mapping", "sivas-bel", PC.SIVAS)]),
        carried_list_case("Çizimdeki eşleme klasördeki profile uymuyor: nedeniyle", [("mapping", "park-bahce-bozuk", BROKEN_MAP)],
                          {"park-bahce.profil.json": "park-bahce.profil.json"}),
        carried_list_case("Klasördeki eşleme çizimdeki profille denetlenir (çizimdeki önce okunur)", [("profile", "park-bahce", KURUM)],
                          {"park-bahce-bozuk.eslem.json": "park-bahce-bozuk.eslem.json"}),
        carried_list_case("Çizimdeki eşlemenin profili yok: denetlenmeden listelenir", [("mapping", "hayalet-esleme", GHOST_MAP)]),
    ]


# ── Import ───────────────────────────────────────────────────────────────────

SOURCE_PROFILE = "Profil verilmedi"
NO_SOURCE = "İçe aktarılacak tek bir kaynak verin: bir JSON dosyası (source), profil (profile) ya da eşleme (mapping)."
NO_DRAWING = ("Profil çizimin dosyasının yanındaki “profiller” klasörüne yazılır; çizimin dosyası yok. Çizimi önce kaydedin ya da "
              "dosyasının mutlak yolunu (drawing) verin.")


def imported(kind, p, f, suffix, **extra):
    out = {"kind": kind, "name": p["name"], "title": p["title"],
           "release": release_of(p) if kind == "profile" else 0,
           "classes": len(p["classes"]),
           "migrations": len(p.get("migrations", [])) if kind == "profile" else 0,
           "store": "folder", "path": tmp(f, p["name"] + suffix), "replaced": False, "bytes": "$any"}
    out.update(extra)
    return out


def carried_out(kind, p, **extra):
    """What the import says of one it keeps inside the drawing (docs/adr/0735): no path."""
    out = {"kind": kind, "name": p["name"], "title": p["title"],
           "release": release_of(p) if kind == "profile" else 0,
           "classes": len(p["classes"]),
           "migrations": len(p.get("migrations", [])) if kind == "profile" else 0,
           "store": "drawing", "replaced": False, "bytes": "$any"}
    out.update(extra)
    return out


def carried_import(name, setup, inp, out, after_kept, before_kept=(), warnings=(), before=(), after=()):
    """A case that keeps a profile or a mapping inside the drawing: the plan writes nothing; execute changes the
    drawing's settings, an edit and no undo step (docs/adr/0735 §3)."""
    steps = list(before)
    steps.append(step("plan", inp, completed(out, list(warnings)), {"revision": "same", "dirty": False, "settings": kept(*before_kept)}))
    steps.append(step("execute", inp, completed(out, list(warnings)),
                      {"revision": "changed", "dirty": True, "canUndo": False, "settings": kept(*after_kept)}))
    steps.extend(after)
    return {"name": name, "setup": setup, "steps": steps}


def carried_refusal(name, setup, inp, code, message, path, still=(), before=()):
    return {"name": name, "setup": setup, "steps": list(before) + [
        step("execute", inp, failed(code, message, path), {"revision": "same", "dirty": False, "settings": kept(*still)})]}


def carried_import_cases():
    """`store: drawing` (docs/adr/0735): the profile or the mapping kept inside the drawing's own file."""
    cases = []
    small, empty = PC.SMALL, PC.empty_doc()
    cases.append(carried_import("Çizimin içine (store: drawing): dosyası olmayan çizime de; ayardır, geri alma adımı değildir; listede ve adıyla denetlenir",
                                empty, {"profile": small, "store": "drawing"}, carried_out("profile", small), [("profile", "peyzaj")],
                                after=[step("execute", {}, completed(listing(None, {}, [("profile", "peyzaj", small)], drawing=False)), command=LIST),
                                       step("execute", {"name": "peyzaj"}, completed(PC.report(PC.linked(PC.empty_doc(), []), small)),
                                            command=PC.CHECK)]))
    cases.append(carried_import("Dosyadan çizimin içine: okunan profil yazılır, sürümü ve göçleriyle", empty,
                                {"source": FIX + "park-bahce.profil.json", "store": "drawing"}, carried_out("profile", KURUM),
                                [("profile", "park-bahce")],
                                after=[step("execute", {}, completed(listing(None, {}, [("profile", "park-bahce", KURUM)], drawing=False)),
                                            command=LIST)]))
    cases.append(carried_import("Eşleme çizimin içine (KentOS'un profili için): adıyla okunur", empty,
                                {"mapping": PC.SIVAS, "store": "drawing"}, carried_out("mapping", PC.SIVAS), [("mapping", "sivas-bel")],
                                after=[step("execute", {"name": "tucbs-temel", "mappingName": "sivas-bel"},
                                            completed(PC.report(PC.linked(PC.empty_doc(), []), PC.through(PC.PROFILE, PC.SIVAS), "sivas-bel")),
                                            command=PC.CHECK)]))
    own = carry(empty, ("profile", "park-bahce", KURUM))
    cases.append(carried_import("Eşleme çizimdeki profil için: o profille denetlenir", own, {"mapping": KURUM_MAP, "store": "drawing"},
                                carried_out("mapping", KURUM_MAP), [("profile", "park-bahce"), ("mapping", "park-bahce-kurum")],
                                before_kept=[("profile", "park-bahce")]))
    cases.append(carried_refusal("Eşleme çizimdeki profile uymuyor: invalid_profile_mapping", own, {"mapping": BROKEN_MAP, "store": "drawing"},
                                 "invalid_profile_mapping", f"Eşleme geçersiz: {PC.mapping_problem(BROKEN_MAP, KURUM)}", "mapping",
                                 still=[("profile", "park-bahce")]))
    cases.append(carried_import("Eşlemenin profili bulunamıyor: yazılır, denetlenmediği söylenir (mapping_unchecked)", empty,
                                {"mapping": GHOST_MAP, "store": "drawing"}, carried_out("mapping", GHOST_MAP), [("mapping", "hayalet-esleme")],
                                warnings=[{"code": "mapping_unchecked", "path": "mapping",
                                           "message": "“hayalet” profili bulunamadı; eşlemenin adları profille denetlenmedi."}]))
    r2 = carry(empty, ("profile", "park-bahce", KURUM_R2))
    cases.append(carried_import("Çizimdeki eski sürümün üstüne yenisi: overwrite olmadan değiştirilir, söylenir (profile_replaced)", r2,
                                {"source": FIX + "park-bahce.profil.json", "store": "drawing"},
                                carried_out("profile", KURUM, replaced=True, previousRelease=2), [("profile", "park-bahce")],
                                before_kept=[("profile", "park-bahce")],
                                warnings=[{"code": "profile_replaced", "path": "profile",
                                           "message": "Çizimin içindeki “park-bahce” profili 2. sürümden 3. sürüme değiştirildi."}],
                                after=[step("execute", {}, completed(listing(None, {}, [("profile", "park-bahce", KURUM)], drawing=False)),
                                            command=LIST)]))
    cases.append(carried_refusal("Çizimde aynı sürüm, overwrite yok: profile_exists", own,
                                 {"source": FIX + "park-bahce.profil.json", "store": "drawing"}, "profile_exists",
                                 "Çizimin içinde “park-bahce” profili var; aynı sürümün üzerine yazmak için overwrite verin.", "profile",
                                 still=[("profile", "park-bahce")]))
    # The copies the next two replace are written compact: the import's own text differs, the drawing changes.
    cases.append(carried_import("Çizimde aynı sürüm overwrite ile değiştirilir", carry(empty, ("profile", "park-bahce", compact(KURUM))),
                                {"source": FIX + "park-bahce.profil.json", "store": "drawing", "overwrite": True},
                                carried_out("profile", KURUM, replaced=True, previousRelease=3), [("profile", "park-bahce")],
                                before_kept=[("profile", "park-bahce")],
                                warnings=[{"code": "profile_replaced", "path": "profile",
                                           "message": "Çizimin içindeki “park-bahce” profili 3. sürümden 3. sürüme değiştirildi."}]))
    cases.append(carried_refusal("Çizimdekinden eski sürüm: profile_downgrade", own,
                                 {"source": FIX + "park-bahce-r2.profil.json", "store": "drawing"}, "profile_downgrade",
                                 "Çizimin içindeki “park-bahce” profili 3. sürümde; içe aktarılan 2. sürüm daha eski. Üzerine yazmak için overwrite verin.",
                                 "profile", still=[("profile", "park-bahce")]))
    sivas = carry(empty, ("mapping", "sivas-bel", PC.SIVAS))
    cases.append(carried_refusal("Çizimde aynı adlı eşleme, overwrite yok: profile_exists", sivas, {"mapping": PC.SIVAS, "store": "drawing"},
                                 "profile_exists", "Çizimin içinde “sivas-bel” eşlemesi var; üzerine yazmak için overwrite verin.", "mapping",
                                 still=[("mapping", "sivas-bel")]))
    cases.append(carried_import("Çizimdeki eşleme overwrite ile değiştirilir", carry(empty, ("mapping", "sivas-bel", compact(PC.SIVAS))),
                                {"mapping": PC.SIVAS, "store": "drawing", "overwrite": True},
                                carried_out("mapping", PC.SIVAS, replaced=True), [("mapping", "sivas-bel")], before_kept=[("mapping", "sivas-bel")],
                                warnings=[{"code": "profile_replaced", "path": "mapping",
                                           "message": "Çizimin içindeki “sivas-bel” eşlemesi değiştirildi."}]))
    broken = carry(empty, ("profile", "peyzaj", RAW_FILES["bozuk.json"]))
    cases.append(carried_refusal("Okunmayan kopyanın üstüne overwrite olmadan yazılmaz", broken, {"profile": small, "store": "drawing"},
                                 "profile_exists", "Çizimin içinde “peyzaj” profili var; aynı sürümün üzerine yazmak için overwrite verin.",
                                 "profile", still=[("profile", "peyzaj")]))
    cases.append(carried_import("Okunmayan kopya overwrite ile yenisiyle değiştirilir", broken, {"profile": small, "store": "drawing", "overwrite": True},
                                carried_out("profile", small, replaced=True), [("profile", "peyzaj")], before_kept=[("profile", "peyzaj")],
                                warnings=[{"code": "profile_replaced", "path": "profile",
                                           "message": "Çizimin içindeki “peyzaj” profilinin okunamayan kopyası yenisiyle değiştirildi."}]))
    # Both places: the drawing's copy is read; each import says so.
    f = folder()
    cases.append(carried_import("Klasörde aynı adlı dosya varken çizimin içine: söylenir (profile_shadowed); listede klasördeki gölgede", empty,
                                {"profile": small, "store": "drawing", "drawing": tmp(f)}, carried_out("profile", small), [("profile", "peyzaj")],
                                warnings=[{"code": "profile_shadowed", "path": "profile",
                                           "message": f"Çizimin “profiller” klasöründe de {tmp(f, 'peyzaj.profil.json')} var; komutlar çizimin içindekini okur."}],
                                before=put(f, **{"peyzaj.profil.json": "peyzaj.profil.json"}),
                                after=[listed_after_with(f, {"peyzaj.profil.json": small}, [("profile", "peyzaj", small)])]))
    f = folder()
    mine = carry(empty, ("profile", "peyzaj", small))
    cases.append({"name": "Çizimde aynı adlı profil varken klasöre: söylenir (profile_shadowed); çizim değişmez", "setup": mine, "steps": [
        place_drawing(f),
        step("execute", {"profile": small, "drawing": tmp(f)},
             completed(imported("profile", small, f, ".profil.json"), [
                 {"code": "profile_shadowed", "path": "profile",
                  "message": "Çizimin içinde de “peyzaj” profili var; komutlar onu okur, klasördeki bu dosyayı değil."}]),
             {"revision": "same", "dirty": False, "settings": kept(("profile", "peyzaj"))}),
        listed_after_with(f, {"peyzaj.profil.json": small}, [("profile", "peyzaj", small)])]})
    cases.append({"name": "Doğrulama (çizimin içine)", "setup": empty, "steps": [step("validate", {"profile": small, "store": "drawing"}, VALID)]})
    # The expected revision.
    cases.append({"name": "Beklenen sürüm tutmuyor: hiçbir şey yazılmaz", "setup": empty, "steps": [
        step("captureRevision", **{"as": "once"}),
        step("execute", {"mapping": PC.SIVAS, "store": "drawing"}, completed(carried_out("mapping", PC.SIVAS)), {"revision": "changed"}),
        step("execute", {"profile": small, "store": "drawing", "expectedRevision": "$once"},
             {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
             {"revision": "same", "settings": kept(("mapping", "sivas-bel"))})]})
    cases.append(carried_import("Beklenen sürüm tutunca yazılır", empty, {"profile": small, "store": "drawing", "expectedRevision": "$current"},
                                carried_out("profile", small), [("profile", "peyzaj")]))
    cases.append(carried_refusal("geçersiz beklenen sürüm", empty, {"profile": small, "store": "drawing", "expectedRevision": "on iki"},
                                 "invalid_revision", INVALID_REVISION, "expectedRevision"))
    # The drawing carries at most 256.
    full = carry(empty, *[("mapping", f"m{i:03d}", "{}") for i in range(256)])
    cases.append(carried_refusal("Çizim 256 profil ve eşleme taşıyor: yenisi alınmaz", full, {"profile": small, "store": "drawing"},
                                 "invalid_profile", "Çizim en çok 256 profil ve eşleme taşır; birini kaldırın ya da profiller klasörüne alın.",
                                 "profile", still=[("mapping", f"m{i:03d}") for i in range(256)]))
    # The source's own rules stay; the folder needs the drawing's file.
    cases.append(carried_refusal("çizimin içine, göreli kaynak yolu ve çizimin dosyası yok", empty, {"source": "gelen/peyzaj.json", "store": "drawing"},
                                 "invalid_file_path",
                                 "“gelen/peyzaj.json” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin.",
                                 "source"))
    cases.append(carried_refusal("klasöre (store: folder) ama çizimin dosyası yok", empty, {"profile": small, "store": "folder"},
                                 "invalid_file_path", NO_DRAWING, "drawing"))
    cases.append(carried_refusal("KentOS'un profilinin adı çizimin içine de alınmaz", empty,
                                 {"source": FIX + "profil-yerlesik-adli.json", "store": "drawing"}, "profile_name_taken",
                                 "“tucbs-temel” KentOS'un kendi profilinin adı; projenin profiline başka bir ad verin.", "profile"))
    return cases


def listed_after_with(f, files, carried):
    return step("execute", {"drawing": tmp(f)}, completed(listing(f, files, carried)), command=LIST)


def listed_after(f, files):
    return step("execute", {"drawing": tmp(f)}, completed(listing(f, files)), command=LIST)


def import_case(name, f, inp, out, warnings=(), before=(), after=(), plan=True, undo_check=True):
    steps = list(before)
    if plan:
        steps.append(step("plan", inp, completed(out, list(warnings)), {"revision": "same"}))
    steps.append(step("execute", inp, completed(out, list(warnings)), {"revision": "same", "dirty": False}))
    steps.extend(after)
    return {"name": name, "setup": PC.empty_doc(), "steps": steps}


def import_refusal(name, inp, code, message, path, before=None):
    if before is None:
        d = inp.get("drawing", "")
        before = []
        if d.startswith("$tmp:pf") and "/" in d:
            before = [place_drawing(d[len("$tmp:"):].split("/")[0])]
    return {"name": name, "setup": PC.empty_doc(), "steps": list(before) + [
        step("execute", inp, failed(code, message, path), {"revision": "same", "dirty": False})]}


def import_cases():
    cases = []
    small = PC.SMALL
    # A profile given itself.
    f = folder()
    cases.append(import_case("Profil kendisiyle verilir: klasöre yazılır, listede görünür, adıyla uygulanır ve denetlenir", f,
                             {"profile": small, "drawing": tmp(f)}, imported("profile", small, f, ".profil.json"),
                             before=[place_drawing(f)],
                             after=[listed_after(f, {"peyzaj.profil.json": small}),
                                    step("execute", {"name": "peyzaj", "drawing": tmp(f)},
                                         completed(PC.report(PC.linked(PC.empty_doc(), []), small)), command=PC.CHECK)]))
    # A profile from a file by absolute path.
    f = folder()
    cases.append(import_case("Dosyadan (mutlak yol): sürümü, sınıf ve göç sayısı söylenir", f,
                             {"source": FIX + "park-bahce.profil.json", "drawing": tmp(f)}, imported("profile", KURUM, f, ".profil.json"),
                             before=[place_drawing(f)], after=[listed_after(f, {"park-bahce.profil.json": KURUM})]))
    # A profile from a file by a path from the drawing's folder.
    f = folder()
    before = [step("copyFile", **{"from": FIX + "peyzaj.profil.json", "to": f"$tmp:{f}/gelen/peyzaj.json"})]
    cases.append(import_case("Dosyadan (çizimin klasöründen göreli yol)", f,
                             {"source": "gelen/peyzaj.json", "drawing": tmp(f)}, imported("profile", small, f, ".profil.json"),
                             before=before, after=[listed_after(f, {"peyzaj.profil.json": small})]))
    # A mapping given itself, for KentOS's profile.
    f = folder()
    cases.append(import_case("Eşleme kendisiyle verilir (KentOS'un profili için): klasöre yazılır, listede ve apply'da kullanılır", f,
                             {"mapping": PC.SIVAS, "drawing": tmp(f)}, imported("mapping", PC.SIVAS, f, ".eslem.json"),
                             before=[place_drawing(f)], after=[listed_after(f, {"sivas-bel.eslem.json": PC.SIVAS}),
                                                               step("execute", {"name": "tucbs-temel", "mappingName": "sivas-bel", "drawing": tmp(f)},
                                                                    completed(PC.report(PC.linked(PC.empty_doc(), []), PC.through(PC.PROFILE, PC.SIVAS), "sivas-bel")),
                                                                    command=PC.CHECK)]))
    # A mapping from a file.
    f = folder()
    cases.append(import_case("Eşleme dosyadan", f, {"source": FIX + "sivas-bel.eslem.json", "drawing": tmp(f)},
                             imported("mapping", PC.SIVAS, f, ".eslem.json"), before=[place_drawing(f)]))
    # A mapping for a profile of the project.
    f = folder()
    cases.append(import_case("Eşleme projenin profili için: profille denetlenir", f, {"mapping": KURUM_MAP, "drawing": tmp(f)},
                             imported("mapping", KURUM_MAP, f, ".eslem.json"),
                             before=put(f, **{"park-bahce.profil.json": "park-bahce.profil.json"})))
    # A mapping whose profile is not found: written, said.
    f = folder()
    cases.append(import_case("Eşlemenin profili bulunamıyor: yazılır, denetlenmediği söylenir (mapping_unchecked)", f,
                             {"mapping": GHOST_MAP, "drawing": tmp(f)}, imported("mapping", GHOST_MAP, f, ".eslem.json"),
                             warnings=[{"code": "mapping_unchecked", "path": "mapping",
                                        "message": "“hayalet” profili bulunamadı; eşlemenin adları profille denetlenmedi."}],
                             before=[place_drawing(f)]))
    # Replacing: an older release is replaced by a newer one without overwrite.
    f = folder()
    cases.append(import_case("Eski sürümün üstüne yeni sürüm: overwrite olmadan değiştirilir, söylenir (profile_replaced)", f,
                             {"source": FIX + "park-bahce.profil.json", "drawing": tmp(f)},
                             imported("profile", KURUM, f, ".profil.json", replaced=True, previousRelease=2),
                             warnings=[{"code": "profile_replaced", "path": "profile",
                                        "message": "Projedeki “park-bahce” profili 2. sürümden 3. sürüme değiştirildi."}],
                             before=put(f, **{"park-bahce.profil.json": "park-bahce-r2.profil.json"}) + [place_drawing(f)],
                             after=[listed_after(f, {"park-bahce.profil.json": KURUM})]))
    # The same release needs overwrite.
    f = folder()
    cases.append(import_refusal("Aynı sürüm overwrite olmadan: file_exists", {"source": FIX + "park-bahce.profil.json", "drawing": tmp(f)},
                                "file_exists", f"{tmp(f, 'park-bahce.profil.json')} var; aynı sürümün üzerine yazmak için overwrite verin.",
                                "profile", before=put(f, **{"park-bahce.profil.json": "park-bahce.profil.json"})))
    f = folder()
    cases.append(import_case("Aynı sürüm overwrite ile: değiştirilir", f,
                             {"source": FIX + "park-bahce.profil.json", "drawing": tmp(f), "overwrite": True},
                             imported("profile", KURUM, f, ".profil.json", replaced=True, previousRelease=3),
                             warnings=[{"code": "profile_replaced", "path": "profile",
                                        "message": "Projedeki “park-bahce” profili 3. sürümden 3. sürüme değiştirildi."}],
                             before=put(f, **{"park-bahce.profil.json": "park-bahce.profil.json"})))
    # Downgrade.
    f = folder()
    cases.append(import_refusal("Daha eski sürüm: profile_downgrade", {"source": FIX + "park-bahce-r2.profil.json", "drawing": tmp(f)},
                                "profile_downgrade",
                                "Projedeki “park-bahce” profili 3. sürümde; içe aktarılan 2. sürüm daha eski. Üzerine yazmak için overwrite verin.",
                                "profile", before=put(f, **{"park-bahce.profil.json": "park-bahce.profil.json"})))
    f = folder()
    cases.append(import_case("Daha eski sürüm overwrite ile yazılır", f,
                             {"source": FIX + "park-bahce-r2.profil.json", "drawing": tmp(f), "overwrite": True},
                             imported("profile", KURUM_R2, f, ".profil.json", replaced=True, previousRelease=3),
                             warnings=[{"code": "profile_replaced", "path": "profile",
                                        "message": "Projedeki “park-bahce” profili 3. sürümden 2. sürüme değiştirildi."}],
                             before=put(f, **{"park-bahce.profil.json": "park-bahce.profil.json"})))
    # A mapping needs overwrite too.
    f = folder()
    cases.append(import_refusal("Eşleme var, overwrite yok: file_exists", {"mapping": PC.SIVAS, "drawing": tmp(f)}, "file_exists",
                                f"{tmp(f, 'sivas-bel.eslem.json')} var; üzerine yazmak için overwrite verin.", "mapping",
                                before=put(f, **{"sivas-bel.eslem.json": "sivas-bel.eslem.json"})))
    f = folder()
    cases.append(import_case("Eşleme overwrite ile değiştirilir (profile_replaced)", f,
                             {"mapping": PC.SIVAS, "drawing": tmp(f), "overwrite": True},
                             imported("mapping", PC.SIVAS, f, ".eslem.json", replaced=True),
                             warnings=[{"code": "profile_replaced", "path": "mapping",
                                        "message": "Projedeki “sivas-bel” eşlemesi değiştirildi."}],
                             before=put(f, **{"sivas-bel.eslem.json": "sivas-bel.eslem.json"})))
    # The plan writes nothing.
    f = folder()
    cases.append({"name": "Plan yazmaz: aynı cevabı verir, klasör boş kalır", "setup": PC.empty_doc(), "steps": [
        place_drawing(f),
        step("plan", {"profile": small, "drawing": tmp(f)}, completed(imported("profile", small, f, ".profil.json")), {"revision": "same"}),
        listed_after(f, {})]})
    f = folder()
    cases.append({"name": "Doğrulama", "setup": PC.empty_doc(), "steps": [
        place_drawing(f), step("validate", {"profile": small, "drawing": tmp(f)}, VALID)]})
    # The first profile written to a folder made for it.
    f = folder()
    cases.append(import_case("Klasör yoksa açılır", f, {"profile": small, "drawing": tmp(f)}, imported("profile", small, f, ".profil.json"),
                             before=[place_drawing(f)], plan=False))
    # Refusals.
    f = folder()
    d = tmp(f)
    cases.append(import_refusal("kaynak verilmedi", {"drawing": d}, "profile_source_missing", NO_SOURCE, "source"))
    cases.append(import_refusal("iki kaynak verildi", {"profile": small, "mapping": PC.SIVAS, "drawing": d}, "profile_source_missing",
                                NO_SOURCE, "source"))
    cases.append(import_refusal("kaynak yolu boş", {"source": "  ", "drawing": d}, "invalid_file_path",
                                "Dosya yolu boş; bir JSON dosyasının yolunu verin.", "source"))
    cases.append(import_refusal("göreli kaynak yolu, çizimin dosyası yok", {"source": "gelen/peyzaj.json"}, "invalid_file_path",
                                "“gelen/peyzaj.json” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin.",
                                "source"))
    cases.append(import_refusal("çizimin dosyası yok", {"profile": small}, "invalid_file_path", NO_DRAWING, "drawing"))
    cases.append(import_refusal("göreli çizim yolu", {"profile": small, "drawing": "cizim.kcad"}, "invalid_file_path", NO_DRAWING, "drawing"))
    cases.append(import_refusal("çizimin klasörü yok", {"profile": small, "drawing": "$tmp:olmayan-klasor/cizim.kcad"}, "invalid_file_path",
                                "Çizimin klasörü yok; profil yazılamaz.", "drawing"))
    cases.append(import_refusal("kaynak dosya yok", {"source": FIX + "yok.json", "drawing": d, "overwrite": False}, "file_not_found",
                                f"{FIX}yok.json dosyası yok.", "source", before=[place_drawing(f)]))
    cases.append(import_refusal("bozuk JSON", {"source": FIX + "bozuk.json", "drawing": d}, "invalid_profile", "$any", "source",
                                before=[place_drawing(f)]))
    cases.append(import_refusal("biçimi tanınmayan dosya", {"source": FIX + "taninmayan.json", "drawing": d}, "unknown_profile_format",
                                "Dosyanın biçimi tanınmıyor: veri profili için “kentos.data-profile”, eşleme için “kentos.profile-mapping” yazılı olmalı.",
                                "source", before=[place_drawing(f)]))
    cases.append(import_refusal("profil biçim sürümü 2", {"source": FIX + "profil-surum2.json", "drawing": d}, "unsupported_profile_version",
                                "Profilin biçim sürümü 2; bu KentOS 1. sürümü okur.", "source", before=[place_drawing(f)]))
    cases.append(import_refusal("profilin biçim sürümü yok", {"source": FIX + "profil-surumsuz.json", "drawing": d},
                                "unsupported_profile_version", "Profilin biçim sürümü yok; bu KentOS 1. sürümü okur.", "source",
                                before=[place_drawing(f)]))
    cases.append(import_refusal("eşleme biçim sürümü 2", {"source": FIX + "esleme-surum2.json", "drawing": d}, "unsupported_profile_version",
                                "Eşlemenin biçim sürümü 2; bu KentOS 1. sürümü okur.", "source", before=[place_drawing(f)]))
    cases.append(import_refusal("profil kendisiyle verilir, biçimi yanlış", {"profile": {**small, "format": "kentos.profile"}, "drawing": d},
                                "unknown_profile_format", "Profilin biçimi “kentos.profile”; “kentos.data-profile” olmalı.", "profile"))
    cases.append(import_refusal("profil kendisiyle verilir, sürümü 2", {"profile": {**small, "version": 2}, "drawing": d},
                                "unsupported_profile_version", "Profilin biçim sürümü 2; bu KentOS 1. sürümü okur.", "profile"))
    cases.append(import_refusal("profil okunamıyor (başlık yok)", {"source": FIX + "profil-basliksiz.json", "drawing": d}, "invalid_profile",
                                "$any", "source", before=[place_drawing(f)]))
    cases.append(import_refusal("profil geçersiz (sınıf yok)", {"source": FIX + "profil-sinifsiz.json", "drawing": d}, "invalid_profile",
                                "Veri profili geçersiz: Profilde 1 ile 64 arası sınıf olur.", "source", before=[place_drawing(f)]))
    cases.append(import_refusal("profilin adı dosya adı olamaz", {"source": FIX + "profil-adi-yanlis.json", "drawing": d}, "invalid_profile",
                                "“Peyzaj Profili” projenin profilinin adı olamaz: küçük harf, rakam ve tire, en çok 64 (dosyanın adı olur).",
                                "profile", before=[place_drawing(f)]))
    cases.append(import_refusal("KentOS'un profilinin adı alınamaz", {"source": FIX + "profil-yerlesik-adli.json", "drawing": d},
                                "profile_name_taken",
                                "“tucbs-temel” KentOS'un kendi profilinin adı; projenin profiline başka bir ad verin.", "profile",
                                before=[place_drawing(f)]))
    cases.append(import_refusal("eşleme başlığı boş", {"source": FIX + "esleme-basliksiz.json", "drawing": d}, "invalid_profile_mapping",
                                "Eşleme geçersiz: Eşlemenin başlığı boş olamaz.", "source", before=[place_drawing(f)]))
    cases.append(import_refusal("eşleme sınıfsız", {"mapping": {**PC.SIVAS, "classes": []}, "drawing": d}, "invalid_profile_mapping",
                                "Eşleme geçersiz: Eşlemede 1 ile 256 arası sınıf olur.", "mapping"))
    cases.append(import_refusal("eşleme profiline uymuyor (sınıf profilde yok)", {"source": FIX + "esleme-gecersiz.json", "drawing": d},
                                "invalid_profile_mapping",
                                f"Eşleme geçersiz: {PC.mapping_problem(content_of('esleme-gecersiz.json'), PC.PROFILE)}", "mapping",
                                before=[place_drawing(f)]))
    cases.append(import_refusal("eşleme projenin profiline uymuyor", {"mapping": BROKEN_MAP, "drawing": d}, "invalid_profile_mapping",
                                f"Eşleme geçersiz: {PC.mapping_problem(BROKEN_MAP, KURUM)}", "mapping",
                                before=put(f, **{"park-bahce.profil.json": "park-bahce.profil.json"})))
    return cases + carried_import_cases()


# ── Migrate ──────────────────────────────────────────────────────────────────

def agac_fields(**override):
    fields = [{"name": "No", "kind": "text"}, {"name": "Tur", "kind": "text"}, {"name": "Yas", "kind": "text"},
              {"name": "Sulama", "kind": "text"}, {"name": "Alan", "kind": "text"}]
    return [{**fl, **override.get(fl["name"], {})} for fl in fields]


def yesil_fields():
    return [{"name": "Ad", "kind": "text"}, {"name": "Bolge", "kind": "text"}, {"name": "Tip", "kind": "text"}]


def peyzaj_layers(agac_name="Ağaçlar", locked=False, group_locked=False, fields=None):
    return [node("peyzaj", "Peyzaj", "group", [
        node("agac", agac_name, style={**STYLE, "color": "#30A46C"}, fields=agac_fields() if fields is None else fields, locked=locked),
        node("yesil", "Yeşil alan", style={**STYLE, "color": "#46A758"}, fields=yesil_fields())], locked=group_locked)]


def trees(a, **kw):
    return {"No": a[0], "Tur": a[1], "Yas": a[2], "Sulama": a[3], **kw}


def peyzaj_entities():
    P = PC.point
    G = PC.polygon
    return [
        P(1, "agac", 1, 1, trees(("A-1", "Çınar", "12", "Damla"))),
        P(2, "agac", 2, 1, trees(("A-2", "çınar", "on beş", "Yağmurlama"))),
        P(3, "agac", 3, 1, trees(("A-3", "Palmiye", "600", "Bilinmiyor"))),
        P(4, "agac", 4, 1, trees(("A-4", "Akasya", "7", "damla"))),
        P(5, "agac", 5, 1, trees(("A-5", " ", "", ""))),
        P(6, "agac", 6, 1, trees(("A-6", "Meşe", "3,5", "Yok"))),
        P(7, "agac", 7, 1, trees(("A-7", "Palmiye", "020", "yagmur"))),
        P(8, "agac", 8, 1, trees(("A-8", "Çınar; Palmiye", "on beş", "Bilinmiyor"))),
        G(9, "yesil", PC.rect(0, 10, 10, 10), {"Ad": "Park", "Bolge": "orman", "Tip": "Bitki"}),
        G(10, "yesil", PC.rect(20, 10, 10, 10), {"Ad": "Bahçe", "Bolge": "bahce", "Tip": "Bitki"}),
        G(11, "yesil", PC.rect(40, 10, 10, 10), {"Ad": "Meydan", "Bolge": "meydan", "Tip": "Bitki"}),
        G(12, "yesil", PC.rect(60, 10, 10, 10), {"Ad": "Çim", "Bolge": "orman", "Tip": "Çim"}),
    ]


CODES = {1: "cinar", 3: "palmiye", 4: "mese", 7: "palmiye", 8: "palmiye; cinar"}
RELATIONS = [{"id": "agac-yesil", "name": "Ağacın alanı", "parent": "yesil", "child": "agac", "field": "Alan"}]


def peyzaj_doc(layers=None, entities=None, domains=None, relations=RELATIONS):
    return PC.reference(layers=peyzaj_layers() if layers is None else layers,
                        entities=peyzaj_entities() if entities is None else entities, domains=domains, relations=relations)


# The engine: the steps run one after another on the drawing as it is then (cad_profile_files.rs `migrate`).

class Refused(Exception):
    def __init__(self, message, code="layer_refused"):
        super().__init__(message)
        self.code = code


def locked_above(nodes, id_):
    def walk(ns, inherited):
        for n in ns:
            here = inherited or n["locked"]
            if n["id"] == id_:
                return here
            hit = walk(n["children"], here)
            if hit is not None:
                return hit
        return None
    return bool(walk(nodes, False))


def outcome(layer=None, applied=False, objects=0, note="", warning=None, losses=()):
    return {"layer": layer, "applied": applied, "objects": objects, "note": note, "warning": warning, "losses": list(losses)}


def skipped(layer, note):
    return outcome(layer, note=note)


def missing(layer, note):
    return outcome(layer, note=note, warning=("migration_step_skipped", note))


def lock_outcome(layer, name):
    note = f"“{name}” katmanı kilitli; adım atlandı. Kilidi Katmanlar panelinden açın."
    return outcome(layer, note=note, warning=("profile_layer_skipped", note))


def layer_of(doc, path):
    place = PC.find(doc["layers"], path)
    if place[0] != "layer":
        return None, missing(None, f"“{words(path)}” katmanı çizimde yok; adım atlandı.")
    n = PC.get(doc["layers"], place[1])
    if locked_above(doc["layers"], place[1]):
        return None, lock_outcome(place[1], n["name"])
    return (place[1], n), None


def on_layer(doc, id_):
    return [e for e in doc["entities"] if e["layerId"] == id_]


def recode(field, value, change):
    if field is not None and field.get("multiple") and field["kind"] == "text":
        parts = [trim(p) for p in value.split(";") if trim(p) != ""]
        changed, out = False, []
        for p in parts:
            n = change(p)
            if n is not None:
                changed = True
            else:
                n = p
            if n != "" and n not in out:
                out.append(n)
        return "; ".join(out) if changed else None
    return change(value)


def step_rename_layer(doc, st):
    path, to = st["layer"], st["to"].strip()
    place = PC.find(doc["layers"], path)
    renamed = path[:-1] + [to]
    if place[0] != "layer":
        p2 = PC.find(doc["layers"], renamed)
        if p2[0] == "layer":
            return skipped(p2[1], f"“{words(path)}” katmanı zaten “{to}” adıyla duruyor.")
        return missing(None, f"“{words(path)}” katmanı çizimde yok; adım atlandı.")
    id_ = place[1]
    n = PC.get(doc["layers"], id_)
    current = n["name"]
    if current.strip() == to:
        return skipped(id_, f"“{current}” katmanı zaten bu adda.")
    p2 = PC.find(doc["layers"], renamed)
    if p2[0] in ("layer", "blocked") and p2[1] != id_:
        raise Refused(f"“{current}” katmanı “{to}” adını alamaz: aynı yerde bu adda bir katman ya da grup var.")
    n["name"] = to
    return outcome(id_, True, 0, f"“{current}” katmanı “{to}” oldu.")


def step_rename_field(doc, st, touched):
    got, out = layer_of(doc, st["layer"])
    if out:
        return out
    id_, n = got
    name, frm, to = n["name"], st["from"], st["to"]
    fields = n.get("fields", [])
    at = next((i for i, f in enumerate(fields) if f["name"] == frm), None)
    if at is None:
        if any(f["name"] == to for f in fields):
            return skipped(id_, f"“{name}” katmanının “{frm}” alanı zaten “{to}” adıyla duruyor.")
        return missing(id_, f"“{name}” katmanında “{frm}” alanı yok; adım atlandı.")
    if any(f["name"] == to for f in fields):
        raise Refused(f"“{name}” katmanında “{frm}” alanı “{to}” adını alamaz: bu adda bir alan zaten var.")
    holders = [e for e in on_layer(doc, id_) if frm in e["attrs"]]
    field = fields[at]
    keyed = any(r["child"] == id_ and r["field"] == (field.get("id") or frm) for r in doc["settings"].get("relations", []))
    if "id" not in field and keyed:
        field["id"] = frm
    field["name"] = to
    for e in holders:
        e["attrs"] = {(to if k == frm else k): v for k, v in e["attrs"].items()}
        touched.add(e["id"])
    return outcome(id_, True, len(holders), f"“{name}” katmanında “{frm}” alanı “{to}” oldu; {len(holders)} nesnenin değeri taşındı.")


def follow_all(doc):
    """The linked fields' copies written anew after the value lists changed (`set_domains`)."""
    domains = doc["settings"].get("domains", [])

    def walk(nodes):
        for n in nodes:
            if n["type"] == "layer":
                if "fields" in n:
                    n["fields"][:] = PC.follow(n["fields"], domains)
            else:
                walk(n["children"])
    walk(doc["layers"])


def step_change_type(doc, profile, st, touched):
    got, out = layer_of(doc, st["layer"])
    if out:
        return out
    id_, n = got
    name, field = n["name"], copy.deepcopy(st["field"])
    fields = n.get("fields", [])
    at = next((i for i, f in enumerate(fields) if f["name"] == field["name"]), None)
    if at is None:
        return missing(id_, f"“{name}” katmanında “{field['name']}” alanı yok; adım atlandı.")
    added = None
    domains = doc["settings"].get("domains", [])
    if field.get("domain") and not any(d["id"] == field["domain"] for d in domains):
        own = next((d for d in profile.get("domains", []) if d["id"] == field["domain"]), None)
        if own is not None:
            domains = doc["settings"].setdefault("domains", [])
            domains.append(copy.deepcopy(own))
            added = own["name"]
            follow_all(doc)
    if "id" not in field and "id" in fields[at]:
        field["id"] = fields[at]["id"]
    shown = PC.applied(field, domains)
    if fields[at] == shown:
        return skipped(id_, f"“{name}” katmanının “{field['name']}” alanı zaten bu tanımda.")
    loose = {k: v for k, v in shown.items() if k != "required"}
    writes, losses = [], {}
    for e in on_layer(doc, id_):
        v = e["attrs"].get(field["name"])
        if v is None or trim(v) == "":
            continue
        r = FIELDS.check(loose, v)
        if "value" in r:
            if r["value"] != v:
                writes.append((e, r["value"]))
        else:
            clear = st.get("onFail", "keep") == "clear"
            entry = losses.setdefault((v, clear), [])
            entry.append(f"$uidOf:{e['id']}")
            if clear:
                writes.append((e, ""))
    fields[at] = shown
    for e, value in writes:
        e["attrs"][field["name"]] = value
        touched.add(e["id"])
    unfit = sum(len(u) for u in losses.values())
    found = [{"layer": id_, "layerName": name, "field": field["name"], "kind": "cleared" if clear else "kept", "value": v,
              "uids": uids, "count": len(uids)} for (v, clear), uids in sorted(losses.items())]
    note = f"“{name}” katmanının “{field['name']}” alanı {kind_word(shown)} oldu; {len(writes)} nesnenin değeri yazıldı"
    if unfit:
        note += f", {unfit} nesnenin değeri yeni kurala uymadı"
    if added:
        note += f"; projeye “{added}” değer listesi eklendi"
    return outcome(id_, True, len(writes), note + ".", losses=found)


def step_map_values(doc, st, touched):
    got, out = layer_of(doc, st["layer"])
    if out:
        return out
    id_, n = got
    name, field = n["name"], st["field"]
    held = next((f for f in n.get("fields", []) if f["name"] == field), None)
    if held is None:
        return missing(id_, f"“{name}” katmanında “{field}” alanı yok; adım atlandı.")
    writes = []
    for e in on_layer(doc, id_):
        v = e["attrs"].get(field)
        if v is None:
            continue

        def change(code, e=e):
            def hit(conditional):
                return next((m for m in st["map"] if m["from"] == code and ("whenField" in m) == conditional and
                             (not conditional or e["attrs"].get(m["whenField"]) == m["whenEquals"])), None)
            m = hit(True) or hit(False)
            return m["to"] if m else None
        nv = recode(held, v, change)
        if nv is not None and nv != v:
            writes.append((e, nv))
    if not writes:
        return skipped(id_, f"“{name}” katmanının “{field}” alanında değişecek değer yok.")
    for e, nv in writes:
        e["attrs"][field] = nv
        touched.add(e["id"])
    return outcome(id_, True, len(writes), f"“{name}” katmanının “{field}” alanında {len(writes)} nesnenin değeri yeniden kodlandı.")


def step_rename_domain(doc, st):
    """docs/adr/0647 §3: the project's list takes the new id; every field linked to it follows."""
    old, to = st["domain"].strip(), st["to"].strip()
    domains = doc["settings"].get("domains", [])
    d = next((x for x in domains if x["id"] == old), None)
    if d is None:
        if any(x["id"] == to for x in domains):
            return skipped(None, f"“{old}” değer listesi zaten “{to}” kimliğiyle duruyor.")
        return missing(None, f"Projede “{old}” değer listesi yok; adım atlandı.")
    if any(x["id"] == to for x in domains):
        raise Refused(f"“{d['name']}” listesinin kimliği “{to}” olamaz: projede bu kimlikte bir liste var.", "domain_exists")
    d["id"] = to
    linked = 0

    def walk(nodes):
        nonlocal linked
        for n in nodes:
            if n["type"] == "layer":
                for f in n.get("fields", []):
                    if f.get("domain") == old:
                        f["domain"] = to
                        linked += 1
            else:
                walk(n["children"])
    walk(doc["layers"])
    return outcome(None, True, 0, f"“{d['name']}” listesinin kimliği “{to}” oldu; {linked} alan yeni kimliğe bağlandı.")


# ── A migration read through an institution's mapping (docs/adr/0647 §4) ─────────────────────────────────────────────

def chain_of(profile, frm, to):
    out = []
    for release in range(frm, to):
        m = next(m for m in profile["migrations"] if m["from"] == release)
        for index, st in enumerate(m["steps"]):
            out.append((release, index, st))
    return out


def final_path(chain, k, path):
    """`path` taken through the chain's later layer renames: its name at the target release."""
    cur = list(path)
    for _, _, st in chain[k + 1:]:
        if st["kind"] == "renameLayer" and st["layer"] == cur:
            cur = cur[:-1] + [st["to"].strip()]
    return cur


def class_at(profile, chain, k, path):
    final = final_path(chain, k, path)
    return next((c for c in profile["classes"] if c["layer"] == final), None)


def final_field(profile, chain, k, cls, name):
    cur = name
    for j in range(k + 1, len(chain)):
        st = chain[j][2]
        if st["kind"] == "renameField" and st["from"] == cur:
            c = class_at(profile, chain, j, st["layer"])
            if c is not None and c["id"] == cls["id"]:
                cur = st["to"]
    return cur


def mapped_class(mapping, cls):
    return next((c for c in mapping["classes"] if c["class"] == cls["id"]), None) if cls else None


def mapped_field(mc, name):
    return next((f for f in (mc or {}).get("fields", []) if f["profile"] == name), None)


def mapped_codes(mc, name):
    return next((x for x in (mc or {}).get("codes", []) if x["field"] == name), None)


def translate(doc, profile, mapping, chain, k):
    """The step as it runs on the institution's names, or the outcome of a step the mapping makes moot."""
    st = chain[k][2]
    kind = st["kind"]
    if kind in ("retireCode", "renameDomain"):
        return st, None
    if kind == "renameLayer":
        cls = class_at(profile, chain, k, st["layer"][:-1] + [st["to"].strip()])
        mc = mapped_class(mapping, cls)
        if mc and mc.get("layer"):
            place = PC.find(doc["layers"], mc["layer"])
            layer = place[1] if place[0] == "layer" else None
            final = final_path(chain, k, st["layer"][:-1] + [st["to"].strip()])
            return None, skipped(layer, f"Eşleme “{words(final)}” katmanına “{words(mc['layer'])}” adını veriyor; ad değişmez.")
        return st, None
    cls = class_at(profile, chain, k, st["layer"])
    mc = mapped_class(mapping, cls)
    if mc is None:
        return st, None
    layer = list(mc["layer"]) if mc.get("layer") else st["layer"]
    out = copy.deepcopy(st)
    out["layer"] = layer
    if kind == "renameField":
        final = final_field(profile, chain, k, cls, st["to"])
        mf = mapped_field(mc, final)
        if mf:
            place = PC.find(doc["layers"], layer)
            lid = place[1] if place[0] == "layer" else None
            return None, skipped(lid, f"Eşleme “{final}” alanına “{mf['name']}” adını veriyor; ad değişmez.")
        return out, None
    if kind == "changeFieldType":
        f = st["field"]
        final = final_field(profile, chain, k, cls, f["name"])
        codes = mapped_codes(mc, final)
        nf = copy.deepcopy(f)
        if codes is not None:
            nf = PC.follow([nf], profile.get("domains", []))[0]
            nf = copy.deepcopy(nf)
            nf.pop("domain", None)
            for v in nf.get("values", []):
                hit = next((x for x in codes["codes"] if x["profile"] == v["code"]), None)
                if hit:
                    v["code"] = hit["name"]
            if nf.get("default") is not None:
                hit = next((x for x in codes["codes"] if x["profile"] == nf["default"]), None)
                if hit:
                    nf["default"] = hit["name"]
        mf = mapped_field(mc, final)
        if mf:
            nf["id"] = final
            nf["name"] = mf["name"]
        out["field"] = nf
        return out, None
    # mapValues
    final = final_field(profile, chain, k, cls, st["field"])
    mf = mapped_field(mc, final)
    if mf:
        out["field"] = mf["name"]
    codes = mapped_codes(mc, final)

    def code(c):
        hit = next((x for x in (codes or {}).get("codes", []) if x["profile"] == c), None)
        return hit["name"] if hit else c
    for m in out["map"]:
        m["from"], m["to"] = code(m["from"]), code(m["to"])
        if "whenField" in m:
            wf = mapped_field(mc, final_field(profile, chain, k, cls, m["whenField"]))
            if wf:
                m["whenField"] = wf["name"]
    return out, None


def step_retire(doc, st, touched):
    domain, code, repl = st["domain"], st["code"], st.get("replaceWith")
    domains = doc["settings"].get("domains", [])
    key = PC.fold_name(domain.strip())
    d = next((x for x in domains if x["id"] == domain), None) or next((x for x in domains if PC.fold_name(x["name"].strip()) == key), None)
    if d is None:
        return missing(None, f"Projede “{domain}” değer listesi yok; adım atlandı.")
    values = d.get("values")
    if not values:
        return missing(None, f"“{d['name']}” değer listesinin kodları yok; adım atlandı.")
    old = next((v for v in values if v["code"] == code), None)
    if old is None:
        return missing(None, f"“{d['name']}” değer listesinde “{code}” kodu yok; adım atlandı.")
    if repl is not None and not any(v["code"] == repl for v in values):
        return missing(None, f"“{d['name']}” değer listesinde yerine konacak “{repl}” kodu yok; adım atlandı.")
    writes, locked = [], []
    if repl is not None:
        def leaves(nodes):
            for n in nodes:
                if n["type"] == "layer":
                    yield n
                else:
                    yield from leaves(n["children"])
        for n in leaves(doc["layers"]):
            for f in n.get("fields", []):
                if f.get("domain") != d["id"]:
                    continue
                layer_writes = []
                for e in on_layer(doc, n["id"]):
                    v = e["attrs"].get(f["name"])
                    if v is None:
                        continue
                    nv = recode(f, v, lambda c: repl if c == code else None)
                    if nv is not None and nv != v:
                        layer_writes.append((e, f["name"], nv))
                if not layer_writes:
                    continue
                if locked_above(doc["layers"], n["id"]):
                    locked.append(n["name"])
                else:
                    writes.extend(layer_writes)
    already = old.get("retired", False)
    old["retired"] = True
    follow_all(doc)
    for e, fname, nv in writes:
        e["attrs"][fname] = nv
        touched.add(e["id"])
    objects = len(writes)
    name = d["name"]
    if already and objects == 0:
        note = f"“{name}” listesinin “{code}” kodu zaten kullanımdan kalkmış."
    elif already:
        note = f"“{name}” listesinin “{code}” kodu zaten kullanımdan kalkmıştı."
    else:
        note = f"“{name}” listesinin “{code}” kodu kullanımdan kaldırıldı."
    if repl is not None and objects > 0:
        note += f" {objects} nesnenin değeri “{repl}” oldu."
    warning = ("profile_layer_skipped", f"{', '.join(locked)} katmanı kilitli; değerleri “{code}” koduyla kaldı.") if locked else None
    return outcome(None, objects > 0 or not already, objects, note, warning)


def migrate(doc, profile, frm, to, mapping=None):
    """The chain's steps run on `doc` (changed in place); (output, warnings). Raises Refused. With a mapping each step is
    read through it first (docs/adr/0647 §4)."""
    steps, losses, warnings, touched = [], [], [], set()
    unfit, samples = 0, []
    chain = chain_of(profile, frm, to)
    for k, (release, index, st) in enumerate(chain):
        kind = st["kind"]
        moot = None
        if mapping is not None:
            st, moot = translate(doc, profile, mapping, chain, k)
        if True:
            if moot is not None:
                out = moot
            elif kind == "renameLayer":
                out = step_rename_layer(doc, st)
            elif kind == "renameField":
                out = step_rename_field(doc, st, touched)
            elif kind == "changeFieldType":
                out = step_change_type(doc, profile, st, touched)
            elif kind == "mapValues":
                out = step_map_values(doc, st, touched)
            elif kind == "renameDomain":
                out = step_rename_domain(doc, st)
            else:
                out = step_retire(doc, st, touched)
            if out["warning"]:
                warnings.append({"code": out["warning"][0], "message": out["warning"][1], "path": f"migrations/{release}/steps/{index}"})
            for lo in out["losses"]:
                unfit += lo["count"]
                if len(samples) < 5:
                    samples.append(f"“{lo['value']}”")
                verb = "olduğu gibi bırakıldı" if lo["kind"] == "kept" else "boşaltıldı"
                losses.append({"migration": release, "step": index, "layer": lo["layer"], "field": lo["field"], "kind": lo["kind"],
                               "message": f"“{lo['layerName']}” katmanının “{lo['field']}” alanındaki “{lo['value']}” değeri yeni kurala uymuyor ({lo['count']} nesne); {verb}.",
                               "value": lo["value"], "uids": lo["uids"], "count": lo["count"]})
            s = {"migration": release, "index": index, "kind": kind, "applied": out["applied"], "objects": out["objects"], "note": out["note"]}
            if out["layer"] is not None:
                s["layer"] = out["layer"]
            steps.append(s)
    if unfit:
        warnings.append({"code": "values_unfit", "path": "losses",
                         "message": f"{unfit} nesnenin değeri yeni kurala uymadı ({', '.join(samples)}); göçün raporunda değer değer yazılı. Formla düzenle ya da Öznitelik tablosuyla düzeltin."})
    out = {"changed": any(s["applied"] for s in steps), "from": frm, "to": to, "steps": steps, "losses": losses,
           "objects": len(touched), "revision": "$current"}
    if mapping is not None:
        out["mapping"] = mapping["name"]
    return out, warnings


def state(doc):
    s = {"layers": doc["layers"], "entities": {str(e["id"]): e for e in doc["entities"]},
         "settings": {k: doc["settings"].get(k) for k in ("domains", "relations")}}
    return s


def migrate_case(name, doc, inp, profile, frm, to, before=(), undo=True, plan=True, mapping=None):
    start = copy.deepcopy(doc)
    after = copy.deepcopy(doc)
    out, warnings = migrate(after, profile, frm, to, mapping)
    steps = list(before)
    if plan:
        steps.append(step("plan", inp, completed(out, warnings), {"revision": "same", "layers": start["layers"]}))
    steps.append(step("execute", inp, completed(out, warnings), state(after)))
    # A layer's name is the drawing's flag, never an undo step (docs/adr/0020): undo gives back the fields, the values and
    # the value lists, and the layers keep the names the migration gave; a migration that only renamed layers has nothing to undo.
    undoable = any(s["applied"] and s["kind"] != "renameLayer" for s in out["steps"])
    if undo and undoable:
        steps.append(step("undo", returns=STEP_NAME, expect=state(with_names_of(start, after))))
    return {"name": name, "setup": doc, "steps": steps}, after, out


def with_names_of(start, after):
    """`start` with the layers named as in `after` (the same ids)."""
    out = copy.deepcopy(start)

    def names(nodes):
        for n in nodes:
            hit = PC.get(after["layers"], n["id"])
            if hit is not None:
                n["name"] = hit["name"]
            names(n["children"])
    names(out["layers"])
    return out


def migrate_refusal(name, doc, inp, code, message, path, before=()):
    return {"name": name, "setup": doc, "steps": list(before) + [
        step("execute", inp, failed(code, message, path),
             {"canUndo": False, "layers": doc["layers"], "entities": {str(e["id"]): e for e in doc["entities"]}})]}


def migrate_cases():
    cases = []
    NAME = {"profile": KURUM, "from": 1}
    doc = peyzaj_doc()
    c, after_all, out_all = migrate_case("Birinci sürümden üçüncüye: iki göçün bütün adımları sırasıyla; plan aynı cevabı verir; tek adımda geri alınır",
                                         doc, NAME, KURUM, 1, 3)
    cases.append(c)
    # Release 1 → 2 only.
    c, mid, _ = migrate_case("Birinci sürümden ikinciye (to): katman, iki alan adı ve Yas'ın türü; uymayan değerler olduğu gibi kalır",
                             doc, {"profile": KURUM, "from": 1, "to": 2}, KURUM, 1, 2)
    cases.append(c)
    # Release 2 → 3 on what release 1 → 2 left.
    c, _, _ = migrate_case("İkinci sürümden üçüncüye: liste bağları, kullanımdan kalkan kod ve yeşil alan türleri", mid, {"profile": KURUM, "from": 2}, KURUM, 2, 3)
    cases.append(c)
    # By name, from the project's folder.
    f = folder()
    c, _, _ = migrate_case("Profil adıyla (projenin klasöründen): park-bahce", doc, {"name": "park-bahce", "from": 1, "drawing": tmp(f)}, KURUM, 1, 3,
                           before=put(f, **{"park-bahce.profil.json": "park-bahce.profil.json"}))
    cases.append(c)
    # By name, from the drawing's own copy (docs/adr/0735): no drawing file is needed.
    c, _, _ = migrate_case("Profil adıyla (çizimin içinden): park-bahce", carry(doc, ("profile", "park-bahce", KURUM)),
                           {"name": "park-bahce", "from": 1}, KURUM, 1, 3)
    cases.append(c)
    # The drawing's copy is read before the folder's file: release 3 there, release 2 in the folder.
    f = folder()
    c, _, _ = migrate_case("Çizimdeki kopya klasördekinden önce okunur (çizimde 3., klasörde 2. sürüm)", carry(doc, ("profile", "park-bahce", KURUM)),
                           {"name": "park-bahce", "from": 1, "drawing": tmp(f)}, KURUM, 1, 3,
                           before=put(f, **{"park-bahce.profil.json": "park-bahce-r2.profil.json"}))
    cases.append(c)
    # Run again: nothing is left.
    again = copy.deepcopy(after_all)
    out2, w2 = migrate(again, KURUM, 1, 3)
    c = {"name": "İkinci kez çalışınca kalan bir şey yok: adımlar zaten uygulanmış (sessiz atlanır, nesne yazılmaz)", "setup": doc, "steps": [
        step("execute", NAME, completed(*migrate(copy.deepcopy(doc), KURUM, 1, 3)), state(after_all)),
        step("execute", NAME, completed(out2, w2), state(again))]}
    cases.append(c)
    # The relation's field keeps its key when renamed.
    # (covered by the first case: Alan → YesilAlan keeps id "Alan"; the relation is unchanged)
    # onFail clear: the values that do not fit become empty.
    clear = copy.deepcopy(KURUM)
    clear["migrations"][1]["steps"][0] = {**M23["steps"][0], "onFail": "clear"}
    c, _, _ = migrate_case("onFail clear: liste dışı değerler boşaltılır ve boşaltıldı diye raporlanır", mid,
                           {"profile": clear, "from": 2}, clear, 2, 3)
    cases.append(c)
    # A step whose layer is gone.
    gone = peyzaj_doc(layers=peyzaj_layers(agac_name="Başka ad"))
    c, _, _ = migrate_case("Katmanı olmayan adım atlanır ve söylenir (migration_step_skipped); öbür adımlar çalışır", gone, NAME, KURUM, 1, 3)
    cases.append(c)
    # A step whose field is gone.
    nofield = peyzaj_doc(layers=peyzaj_layers(fields=[f for f in agac_fields() if f["name"] != "Yas"]))
    c, _, _ = migrate_case("Alanı olmayan adım atlanır (Yas yok)", nofield, NAME, KURUM, 1, 3)
    cases.append(c)
    # A locked layer.
    lock = peyzaj_doc(layers=peyzaj_layers(locked=True))
    c, _, _ = migrate_case("Kilitli katmanın alan adımları atlanır (profile_layer_skipped); adlandırma kilitli katmanda yapılır", lock, NAME, KURUM, 1, 3)
    cases.append(c)
    gl = peyzaj_doc(layers=peyzaj_layers(group_locked=True))
    c, _, _ = migrate_case("Kilitli grubun altındaki katman da kilitlidir", gl, NAME, KURUM, 1, 3)
    cases.append(c)
    # Rename onto a name that is taken.
    taken = peyzaj_layers()
    taken[0]["children"].append(node("eski", "Ağaç", style=STYLE))
    cases.append(migrate_refusal("Yeni ad alınmış: layer_refused, hiçbir şey yazılmaz", peyzaj_doc(layers=taken), NAME, "layer_refused",
                                 "“Ağaçlar” katmanı “Ağaç” adını alamaz: aynı yerde bu adda bir katman ya da grup var.", "profile"))
    # Rename a field onto a name that is taken, after a layer was renamed (atomic).
    twice = peyzaj_layers(fields=agac_fields() + [{"name": "AgacNo", "kind": "text"}])
    cases.append(migrate_refusal("Alan adı alınmış, önceki adım yazılmıştı: layer_refused, hiçbiri yazılmaz (tek adım)", peyzaj_doc(layers=twice), NAME,
                                 "layer_refused", "“Ağaç” katmanında “No” alanı “AgacNo” adını alamaz: bu adda bir alan zaten var.", "profile"))
    # Already at the new names: skipped silently.
    done = peyzaj_layers(agac_name="Ağaç", fields=[{"name": "AgacNo", "kind": "text"}, {"name": "Tur", "kind": "text"},
                                                   {"name": "Yas", "kind": "text"}, {"name": "Sulama", "kind": "text"},
                                                   {"name": "YesilAlan", "kind": "text"}])
    c, _, _ = migrate_case("Zaten yeni adlarıyla duran katman ve alan: adımlar sessiz atlanır", peyzaj_doc(layers=done), {"profile": KURUM, "from": 1, "to": 2},
                           KURUM, 1, 2)
    cases.append(c)
    # A retired code that is already retired; and no replacement leaves the values.
    retire_only = copy.deepcopy(KURUM)
    retire_only["migrations"] = [{"from": 1, "to": 2, "title": "Palmiye", "steps": [{"kind": "retireCode", "domain": "agac-tur", "code": "palmiye"}]}]
    retire_only["release"] = 2
    # The project's field Tur is linked to its list (the objects hold its codes): the copies follow the retirement.
    coded = peyzaj_entities()
    for e in coded:
        if e["id"] in CODES:
            e["attrs"]["Tur"] = CODES[e["id"]]
    linked_layers = peyzaj_layers(fields=PC.follow(agac_fields(Tur={"domain": "agac-tur"}), [TUR]))
    linked_doc = peyzaj_doc(layers=linked_layers, entities=coded, domains=[TUR])
    c, ret_after, _ = migrate_case("Kod kullanımdan kaldırılır (replaceWith yok): listede retired, bağlı alanların kopyaları izler, değerler kalır",
                                   linked_doc, {"profile": retire_only, "from": 1}, retire_only, 1, 2)
    cases.append(c)
    again = copy.deepcopy(ret_after)
    out2, w2 = migrate(again, retire_only, 1, 2)
    cases.append({"name": "Zaten kullanımdan kalkmış kod: adım sessiz atlanır", "setup": linked_doc, "steps": [
        step("execute", {"profile": retire_only, "from": 1}, completed(*migrate(copy.deepcopy(linked_doc), retire_only, 1, 2)), state(ret_after)),
        step("execute", {"profile": retire_only, "from": 1}, completed(out2, w2), state(again))]})
    # A retired code replaced in a locked layer.
    rep = copy.deepcopy(retire_only)
    rep["migrations"][0]["steps"][0]["replaceWith"] = "diger"
    lk = peyzaj_doc(layers=peyzaj_layers(locked=True, fields=PC.follow(agac_fields(Tur={"domain": "agac-tur"}), [TUR])),
                    entities=coded, domains=[TUR])
    c, _, _ = migrate_case("Kilitli katmandaki değerler yerinde kalır (profile_layer_skipped); kod yine kullanımdan kalkar", lk, {"profile": rep, "from": 1}, rep, 1, 2)
    cases.append(c)
    c, _, _ = migrate_case("Kullanımdan kalkan kodun yerine başkası konur: bağlı alanın değerleri değişir (replaceWith)", linked_doc,
                           {"profile": rep, "from": 1}, rep, 1, 2)
    cases.append(c)
    # Unknown domain, code or replacement.
    for title, step_ in (("Projede olmayan değer listesi", {"kind": "retireCode", "domain": "yok", "code": "palmiye"}),
                         ("Listede olmayan kod", {"kind": "retireCode", "domain": "agac-tur", "code": "yok"}),
                         ("Listede olmayan yerine konacak kod", {"kind": "retireCode", "domain": "agac-tur", "code": "palmiye", "replaceWith": "yok"})):
        p = copy.deepcopy(retire_only)
        p["migrations"][0]["steps"] = [step_]
        c, _, _ = migrate_case(f"{title}: adım atlanır (migration_step_skipped)", linked_doc, {"profile": p, "from": 1}, p, 1, 2)
        cases.append(c)
    # A multiple field: each code recoded.
    multi_fields = agac_fields(Tur={"multiple": True})
    multi = peyzaj_doc(layers=peyzaj_layers(fields=multi_fields))
    mv = copy.deepcopy(KURUM)
    mv["migrations"] = [{"from": 1, "to": 2, "title": "Çınar", "steps": [{"kind": "mapValues", "layer": ["Peyzaj", "Ağaçlar"], "field": "Tur",
                                                                         "map": [{"from": "Çınar", "to": "cinar"}, {"from": "Palmiye", "to": "cinar"}]}]}]
    mv["release"] = 2
    c, _, _ = migrate_case("Çok değerli alanda her kod yeniden kodlanır, yinelenenler tek kalır", multi, {"profile": mv, "from": 1}, mv, 1, 2)
    cases.append(c)
    # mapValues finds nothing.
    nm = copy.deepcopy(mv)
    nm["migrations"][0]["steps"][0]["map"] = [{"from": "Yok", "to": "x"}]
    c, _, _ = migrate_case("Yeniden kodlanacak değer yoksa adım uygulanmaz (sessiz)", doc, {"profile": nm, "from": 1}, nm, 1, 2)
    cases.append(c)
    # docs/adr/0647 §3: a list's id changed.
    rd = copy.deepcopy(retire_only)
    rd["migrations"][0]["steps"] = [{"kind": "renameDomain", "domain": "agac-tur", "to": "agac-turu"}]
    rd["domains"] = [{**TUR, "id": "agac-turu"}, SULAMA]
    rd["classes"][0]["fields"] = [({**f, "domain": "agac-turu"} if f.get("domain") == "agac-tur" else f)
                                  for f in rd["classes"][0]["fields"]]
    c, rd_after, _ = migrate_case("Listenin kimliği değişir (renameDomain): bağlı alanlar yeni kimliğe bağlanır, değerler kalır",
                                  linked_doc, {"profile": rd, "from": 1}, rd, 1, 2)
    cases.append(c)
    again = copy.deepcopy(rd_after)
    out2, w2 = migrate(again, rd, 1, 2)
    cases.append({"name": "Kimliği zaten değişmiş liste: adım sessiz atlanır", "setup": linked_doc, "steps": [
        step("execute", {"profile": rd, "from": 1}, completed(*migrate(copy.deepcopy(linked_doc), rd, 1, 2)), state(rd_after)),
        step("execute", {"profile": rd, "from": 1}, completed(out2, w2), state(again))]})
    c, _, _ = migrate_case("Projede olmayan listenin kimliği: adım atlanır (migration_step_skipped)", doc,
                           {"profile": rd, "from": 1}, rd, 1, 2)
    cases.append(c)
    both = peyzaj_doc(layers=linked_layers, entities=coded, domains=[TUR, {**TUR, "id": "agac-turu", "name": "Ağaç türü (yeni)"}])
    cases.append(migrate_refusal("Yeni kimlikte başka liste var: domain_exists, hiçbir şey yazılmaz", both, {"profile": rd, "from": 1},
                                 "domain_exists", "“Ağaç türü” listesinin kimliği “agac-turu” olamaz: projede bu kimlikte bir liste var.",
                                 "profile"))
    bad_rd = copy.deepcopy(rd)
    bad_rd["migrations"][0]["steps"] = [{"kind": "renameDomain", "domain": "agac-tur", "to": "Ağaç Türü"}]
    cases.append(migrate_refusal("Listenin yeni kimliği geçersiz: invalid_profile", doc, {"profile": bad_rd, "from": 1}, "invalid_profile",
                                 "Veri profili geçersiz: 1. göcün 1. adımı: listenin eski kimliği dolu, yeni kimliği küçük harf, rakam ve "
                                 "“-” olmalı ve eskisinden farklı.", "profile"))
    # docs/adr/0647 §4: a migration read through the institution's mapping.
    imap = INST_MAP
    inst_fields = [{"name": "NO", "kind": "text"}, {"name": "TUR_KOD", "kind": "text"}, {"name": "Yas", "kind": "text"},
                   {"name": "Sulama", "kind": "text"}, {"name": "Alan", "kind": "text"}]
    inst_layers = [node("peyzaj", "Peyzaj", "group", [
        node("agac", "Kurum ağaçları", style={**STYLE, "color": "#30A46C"}, fields=inst_fields),
        node("yesil", "Yeşil alan", style={**STYLE, "color": "#46A758"}, fields=yesil_fields())])]
    inst_ents = []
    for e in peyzaj_entities():
        e = copy.deepcopy(e)
        if e["layerId"] == "agac":
            a = e["attrs"]
            e["attrs"] = {"NO": a["No"], "TUR_KOD": {"Çınar": "CN", "Palmiye": "PL"}.get(a["Tur"], a["Tur"]), "Yas": a["Yas"],
                          "Sulama": a["Sulama"]}
        inst_ents.append(e)
    inst = peyzaj_doc(layers=inst_layers, entities=inst_ents)
    c, _, _ = migrate_case("Eşlemeyle: kurumun adlı katmanı ve alanları; eşlenen ad değişmez, kodlar kurumunkiyle",
                           inst, {"profile": KURUM, "from": 1, "mapping": imap}, KURUM, 1, 3, mapping=imap)
    cases.append(c)
    f = folder()
    c, _, _ = migrate_case("Eşlemenin adıyla (projenin klasöründen)", inst,
                           {"profile": KURUM, "from": 1, "mappingName": "park-bahce-kurum2", "drawing": tmp(f)}, KURUM, 1, 3, mapping=imap,
                           before=put(f, **{"park-bahce-kurum2.eslem.json": "park-bahce-kurum2.eslem.json"}))
    cases.append(c)
    mvc = copy.deepcopy(KURUM)
    mvc["migrations"] = [{"from": 1, "to": 2, "title": "Palmiye çınar olur", "steps": [
        {"kind": "changeFieldType", "layer": AGAC, "field": TUR_FIELD, "onFail": "keep"},
        {"kind": "mapValues", "layer": AGAC, "field": "Tur", "map": [{"from": "palmiye", "to": "cinar"}]}]}]
    mvc["release"] = 2
    c, _, _ = migrate_case("Eşlemeyle değer eşlemesi: profilin kodları kurumun kodlarına çevrilir (PL → CN)", inst,
                           {"profile": mvc, "from": 1, "mapping": imap}, mvc, 1, 2, mapping=imap)
    cases.append(c)
    # docs/adr/0735: the mapping and the profile the drawing carries inside its own file.
    c, _, _ = migrate_case("Eşlemenin adıyla (çizimin içinden): çizimin dosyası gerekmez", carry(inst, ("mapping", imap["name"], imap)),
                           {"profile": KURUM, "from": 1, "mappingName": imap["name"]}, KURUM, 1, 3, mapping=imap)
    cases.append(c)
    cases.append(migrate_refusal("Çizimde olmayan eşleme: çizimin taşıdıkları söylenir", carry(inst, ("mapping", imap["name"], imap)),
                                 {"profile": KURUM, "from": 1, "mappingName": "yok"}, "unknown_mapping",
                                 "“yok” adlı eşleme yok; çizimin içindeki eşlemeler: park-bahce-kurum2.", "mappingName"))
    cases.append(migrate_refusal("Başka profilin eşlemesi: invalid_profile_mapping", inst, {"profile": KURUM, "from": 1, "mapping": GHOST_MAP},
                                 "invalid_profile_mapping", f"Eşleme geçersiz: {PC.mapping_problem(GHOST_MAP, KURUM)}", "mapping"))
    f = folder()
    cases.append(migrate_refusal("Klasörde olmayan eşleme: unknown_mapping", inst,
                                 {"profile": KURUM, "from": 1, "mappingName": "yok", "drawing": tmp(f)}, "unknown_mapping",
                                 PC.no_mapping_in_folder(), "mappingName", before=[place_drawing(f)]))
    # The profile's list is added by a step that needs it.
    # (covered by the first case: the project has no lists; the step adds agac-tur and sulama)
    # Validate and plan only.
    cases.append({"name": "Doğrulama", "setup": doc, "steps": [step("validate", NAME, VALID)]})
    # Expected revision.
    c, _, _ = migrate_case("Beklenen sürüm tutunca yazılır", doc, {**NAME, "expectedRevision": "$once"}, KURUM, 1, 3, undo=False, plan=False)
    c["steps"].insert(0, step("captureRevision", **{"as": "once"}))
    cases.append(c)
    # Refusals.
    cases.append(migrate_refusal("bilinmeyen profil", doc, {"name": "yok", "from": 1}, "unknown_profile",
                                 "“yok” adlı veri profili yok; KentOS'un profili: tucbs-temel.", "name"))
    cases.append(migrate_refusal("bilinmeyen profil: çizimin taşıdıkları da söylenir", carry(doc, ("profile", "park-bahce", KURUM)),
                                 {"name": "yok", "from": 1}, "unknown_profile",
                                 "“yok” adlı veri profili yok; KentOS'un profili: tucbs-temel; çizimin içindeki profiller: park-bahce.", "name"))
    f = folder()
    cases.append(migrate_refusal("bilinmeyen profil: çizimdekiler ve klasör", carry(doc, ("profile", "park-bahce", KURUM)),
                                 {"name": "yok", "from": 1, "drawing": tmp(f)}, "unknown_profile",
                                 "“yok” adlı veri profili yok; KentOS'un profili: tucbs-temel; çizimin içindeki profiller: park-bahce; "
                                 "çizimin “profiller” klasöründe de “yok.profil.json” yok.", "name", before=[place_drawing(f)]))
    f = folder()
    cases.append(migrate_refusal("projenin klasöründe de yok", doc, {"name": "yok", "from": 1, "drawing": tmp(f)}, "unknown_profile",
                                 PC.no_profile_in_folder(), "name", before=[place_drawing(f)]))
    cases.append(migrate_refusal("profil verilmedi", doc, {"from": 1}, "unknown_profile",
                                 "Profili adıyla (name: tucbs-temel) ya da kendisini (profile) verin.", "name"))
    bad_profile = {**KURUM, "format": "kentos.profile"}
    cases.append(migrate_refusal("geçersiz profil", doc, {"profile": bad_profile, "from": 1}, "invalid_profile",
                                 "Veri profili geçersiz: Profilin biçimi “kentos.data-profile”, sürümü 1 olmalı.", "profile"))
    cases.append(migrate_refusal("profilin göçleri geçersiz (bir göç iki sürüm atlıyor)", doc,
                                 {"profile": {**KURUM, "migrations": [{**M12, "to": 3}]}, "from": 1}, "invalid_profile",
                                 "Veri profili geçersiz: 1. göç 1 sürümünden 3 sürümüne gidiyor; bir göç bir sürümden sonrakine gider.", "profile"))
    cases.append(migrate_refusal("profilin göçü sürümünü aşıyor", doc, {"profile": {**KURUM, "release": 2}, "from": 1}, "invalid_profile",
                                 "Veri profili geçersiz: 2. göç 3 sürümüne gidiyor ama profilin sürümü 2.", "profile"))
    cases.append(migrate_refusal("aynı sürümden iki göç", doc, {"profile": {**KURUM, "migrations": [M12, M12]}, "from": 1}, "invalid_profile",
                                 "Veri profili geçersiz: 1 sürümünden iki göç var.", "profile"))
    cases.append(migrate_refusal("göç adımsız", doc, {"profile": {**KURUM, "migrations": [{**M12, "steps": []}]}, "from": 1}, "invalid_profile",
                                 "Veri profili geçersiz: 1. göçte 1 ile 64 arası adım olur.", "profile"))
    cases.append(migrate_refusal("adım geçersiz (alanın eski ve yeni adı aynı)", doc,
                                 {"profile": {**KURUM, "migrations": [{**M12, "steps": [{"kind": "renameField", "layer": AGAC, "from": "A", "to": "A"}]}]}, "from": 1},
                                 "invalid_profile", "Veri profili geçersiz: 1. göcün 1. adımı: alanın eski ve yeni adı dolu ve farklı olmalı.", "profile"))
    cases.append(migrate_refusal("hedef kaynaktan büyük değil (to = from)", doc, {"profile": KURUM, "from": 2, "to": 2}, "invalid_migration",
                                 "2. sürümden 2. sürüme göç olmaz: hedef kaynaktan büyük ve en çok profilin sürümü (3) olmalı.", "to"))
    cases.append(migrate_refusal("hedef profilin sürümünü aşıyor", doc, {"profile": KURUM, "from": 1, "to": 4}, "invalid_migration",
                                 "1. sürümden 4. sürüme göç olmaz: hedef kaynaktan büyük ve en çok profilin sürümü (3) olmalı.", "to"))
    cases.append(migrate_refusal("kaynak sürümü 0", doc, {"profile": KURUM, "from": 0}, "invalid_migration",
                                 "0. sürümden 3. sürüme göç olmaz: hedef kaynaktan büyük ve en çok profilin sürümü (3) olmalı.", "from"))
    cases.append(migrate_refusal("kaynak sürüm hedeften büyük (to verilmedi)", doc, {"profile": KURUM, "from": 3}, "invalid_migration",
                                 "3. sürümden 3. sürüme göç olmaz: hedef kaynaktan büyük ve en çok profilin sürümü (3) olmalı.", "from"))
    gap = {**KURUM, "migrations": [M23]}
    cases.append(migrate_refusal("göçler arasında boşluk: migration_gap", doc, {"profile": gap, "from": 1}, "migration_gap",
                                 "“park-bahce” profilinde 1. sürümden 2. sürüme göç yok; 1. ile 3. sürüm arasında kesintisiz yol kurulamaz.", "from"))
    cases.append(migrate_refusal("geçersiz beklenen sürüm", doc, {**NAME, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION,
                                 "expectedRevision"))
    first = step("execute", {"uids": ["$uidOf:1"], "operation": "attributes", "attrs": {"Tur": "Meşe"}},
                 completed({"changed": ["$uidOf:1"], "ids": [1], "revision": "$current"}), command="cad.entities.set")
    cases.append({"name": "sürüm çakışması: hiçbir şey yazılmaz", "setup": doc, "steps": [
        step("captureRevision", **{"as": "once"}), first,
        step("execute", {**NAME, "expectedRevision": "$once"},
             {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
             {"revision": "same", "layers": doc["layers"]})]})
    return cases


# ── Remove (docs/adr/0735) ───────────────────────────────────────────────────

def removed(kind, name, value, folder_path=None):
    """What the removal says: the copy's kind, name and length in bytes; the folder's file read from now on, if any."""
    text = value if isinstance(value, str) else carried_text(value)
    out = {"kind": kind, "name": name, "bytes": len(text.encode("utf-8"))}
    if folder_path:
        out["folderPath"] = folder_path
    return out


def still_named(mapping, profile):
    return {"code": "profile_still_named", "path": "name",
            "message": f"“{mapping}” eşlemesi “{profile}” profilini adlandırıyor; profilin başka kopyası kalmadı, eşleme profille denetlenemez."}


def remove_case(name, carried, inp, out, left, warnings=(), before=(), after=(), plan=True):
    """A drawing carrying `carried` loses one: the plan writes nothing; execute changes the settings, an edit and no
    undo step; `left` is what it carries then."""
    setup = carry(PC.empty_doc(), *carried)
    had = [(k, n) for k, n, _ in carried]
    steps = list(before)
    if plan:
        steps.append(step("plan", inp, completed(out, list(warnings)), {"revision": "same", "dirty": False, "settings": kept(*had)}))
    steps.append(step("execute", inp, completed(out, list(warnings)),
                      {"revision": "changed", "dirty": True, "canUndo": False, "settings": kept(*left)}))
    steps.extend(after)
    return {"name": name, "setup": setup, "steps": steps}


def remove_refusal(name, carried, inp, code, message, before=(), after=(), op="execute", path="name"):
    setup = carry(PC.empty_doc(), *carried) if carried else PC.empty_doc()
    had = [(k, n) for k, n, _ in carried]
    return {"name": name, "setup": setup, "steps": list(before) + [
        step(op, inp, failed(code, message, path), {"revision": "same", "dirty": False, "settings": kept(*had)})] + list(after)}


def remove_cases():
    cases = []
    small, sivas, own_map = PC.SMALL, PC.SIVAS, {**PC.MAP_SMALL, "name": "peyzaj", "title": "Peyzaj eşlemesi (belediyenin)"}
    p, s = ("profile", "peyzaj", small), ("mapping", "sivas-bel", sivas)
    cases.append(remove_case("Çizimin taşıdığı profil çıkarılır: ayardır, geri alma adımı değildir; plan aynı cevabı verir, çıkarmaz",
                             [p, s], {"kind": "profile", "name": "peyzaj"}, removed("profile", "peyzaj", small), [("mapping", "sivas-bel")],
                             after=[step("execute", {}, completed(listing(None, {}, [s], drawing=False)), command=LIST)]))
    cases.append(remove_case("Son kopya çıkınca alan hiç kalmaz; adıyla artık bulunmaz", [p], {"kind": "profile", "name": "peyzaj"},
                             removed("profile", "peyzaj", small), [],
                             after=[step("execute", {"name": "peyzaj"},
                                         failed("unknown_profile", "“peyzaj” adlı veri profili yok; KentOS'un profili: tucbs-temel.", "name"),
                                         command=PC.CHECK)]))
    cases.append(remove_case("Eşleme çıkarılır, aynı adlı profil kalır", [p, ("mapping", "peyzaj", own_map)],
                             {"kind": "mapping", "name": "peyzaj"}, removed("mapping", "peyzaj", own_map), [("profile", "peyzaj")]))
    cases.append(remove_case("Profil çıkarılır, aynı adlı eşleme kalır ve söylenir (profile_still_named)", [p, ("mapping", "peyzaj", own_map)],
                             {"kind": "profile", "name": "peyzaj"}, removed("profile", "peyzaj", small), [("mapping", "peyzaj")],
                             warnings=[still_named("peyzaj", "peyzaj")]))
    cases.append(remove_case("Profili adlandıran çizimdeki eşleme söylenir", [p, ("mapping", "peyzaj-kurum", PC.MAP_SMALL)],
                             {"kind": "profile", "name": "peyzaj"}, removed("profile", "peyzaj", small), [("mapping", "peyzaj-kurum")],
                             warnings=[still_named("peyzaj-kurum", "peyzaj")]))
    f = folder()
    cases.append(remove_case("Profili adlandıran klasördeki eşleme de söylenir", [p], {"kind": "profile", "name": "peyzaj", "drawing": tmp(f)},
                             removed("profile", "peyzaj", small), [], warnings=[still_named("peyzaj-kurum", "peyzaj")],
                             before=put(f, **{"peyzaj-kurum.eslem.json": "peyzaj-kurum.eslem.json"})))
    f = folder()
    cases.append(remove_case("Birden çok eşleme: önce çizimdekiler, sonra klasördekiler adlarıyla", [p, ("mapping", "peyzaj", own_map)],
                             {"kind": "profile", "name": "peyzaj", "drawing": tmp(f)}, removed("profile", "peyzaj", small), [("mapping", "peyzaj")],
                             warnings=[still_named("peyzaj", "peyzaj"), still_named("peyzaj-kurum", "peyzaj")],
                             before=put(f, **{"peyzaj-kurum.eslem.json": "peyzaj-kurum.eslem.json",
                                              "sivas-bel.eslem.json": "sivas-bel.eslem.json"})))
    f = folder()
    cases.append(remove_case("Klasörde aynı adlı dosya varken: bundan sonra o okunur (folderPath); eşlemeler için uyarı yok",
                             [p, ("mapping", "peyzaj-kurum", PC.MAP_SMALL)], {"kind": "profile", "name": "peyzaj", "drawing": tmp(f)},
                             removed("profile", "peyzaj", small, tmp(f, "peyzaj.profil.json")), [("mapping", "peyzaj-kurum")],
                             before=put(f, **{"peyzaj.profil.json": "peyzaj.profil.json"}),
                             after=[listed_after_with(f, {"peyzaj.profil.json": small}, [("mapping", "peyzaj-kurum", PC.MAP_SMALL)]),
                                    step("execute", {"name": "peyzaj", "drawing": tmp(f)},
                                         completed(PC.report(PC.linked(PC.empty_doc(), []), small)), command=PC.CHECK)]))
    cases.append(remove_case("Okunmayan kopya da çıkarılır", [("profile", "bozuk", RAW_FILES["bozuk.json"])], {"kind": "profile", "name": "bozuk"},
                             removed("profile", "bozuk", RAW_FILES["bozuk.json"]), []))
    cases.append(remove_case("Okunmayan eşleme uyarı vermez", [p, ("mapping", "bozuk", RAW_FILES["bozuk.json"])],
                             {"kind": "profile", "name": "peyzaj"}, removed("profile", "peyzaj", small), [("mapping", "bozuk")]))
    cases.append(remove_case("Göreli çizim yolunun klasörü yok: yalnız çizimdeki çıkar", [p], {"kind": "profile", "name": "peyzaj", "drawing": "cizim.kcad"},
                             removed("profile", "peyzaj", small), []))
    cases.append(remove_case("Beklenen sürüm tutunca çıkarılır", [p], {"kind": "profile", "name": "peyzaj", "expectedRevision": "$current"},
                             removed("profile", "peyzaj", small), [], plan=False))
    cases.append({"name": "İki kez çıkarılamaz", "setup": carry(PC.empty_doc(), p), "steps": [
        step("execute", {"kind": "profile", "name": "peyzaj"}, completed(removed("profile", "peyzaj", small)), {"settings": kept()}),
        step("execute", {"kind": "profile", "name": "peyzaj"}, failed("unknown_profile", "Çizimin içinde “peyzaj” profili yok.", "name"),
             {"revision": "same", "settings": kept()})]})
    cases.append({"name": "Beklenen sürüm tutmuyor: hiçbir şey çıkarılmaz", "setup": carry(PC.empty_doc(), p, s), "steps": [
        step("captureRevision", **{"as": "once"}),
        step("execute", {"kind": "mapping", "name": "sivas-bel"}, completed(removed("mapping", "sivas-bel", sivas)), {"revision": "changed"}),
        step("execute", {"kind": "profile", "name": "peyzaj", "expectedRevision": "$once"},
             {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
             {"revision": "same", "settings": kept(("profile", "peyzaj"))})]})
    cases.append({"name": "Doğrulama", "setup": carry(PC.empty_doc(), p), "steps": [
        step("validate", {"kind": "profile", "name": "peyzaj"}, VALID, {"revision": "same", "settings": kept(("profile", "peyzaj"))})]})
    # Refusals.
    cases.append(remove_refusal("çizimde olmayan profil", [], {"kind": "profile", "name": "peyzaj"}, "unknown_profile",
                                "Çizimin içinde “peyzaj” profili yok."))
    cases.append(remove_refusal("doğrulama da reddeder", [], {"kind": "profile", "name": "peyzaj"}, "unknown_profile",
                                "Çizimin içinde “peyzaj” profili yok.", op="validate"))
    cases.append(remove_refusal("çizimde olmayan eşleme", [p], {"kind": "mapping", "name": "sivas-bel"}, "unknown_mapping",
                                "Çizimin içinde “sivas-bel” eşlemesi yok."))
    cases.append(remove_refusal("türü farklı: eşlemenin adıyla profil", [s], {"kind": "profile", "name": "sivas-bel"}, "unknown_profile",
                                "Çizimin içinde “sivas-bel” profili yok."))
    cases.append(remove_refusal("KentOS'un profili çizimde değildir", [p], {"kind": "profile", "name": "tucbs-temel"}, "unknown_profile",
                                "Çizimin içinde “tucbs-temel” profili yok."))
    cases.append(remove_refusal("büyük harfli ad başka addır", [p], {"kind": "profile", "name": "Peyzaj"}, "unknown_profile",
                                "Çizimin içinde “Peyzaj” profili yok."))
    f = folder()
    cases.append(remove_refusal("yalnız klasörde olan profil bu komutla silinmez", [], {"kind": "profile", "name": "peyzaj", "drawing": tmp(f)},
                                "unknown_profile",
                                f"Çizimin içinde “peyzaj” profili yok. Klasördeki {tmp(f, 'peyzaj.profil.json')} bu komutla silinmez; dosyayı klasörden kaldırın.",
                                before=put(f, **{"peyzaj.profil.json": "peyzaj.profil.json"}),
                                after=[listed_after(f, {"peyzaj.profil.json": small})]))
    f = folder()
    cases.append(remove_refusal("yalnız klasörde olan eşleme de", [], {"kind": "mapping", "name": "sivas-bel", "drawing": tmp(f)},
                                "unknown_mapping",
                                f"Çizimin içinde “sivas-bel” eşlemesi yok. Klasördeki {tmp(f, 'sivas-bel.eslem.json')} bu komutla silinmez; dosyayı klasörden kaldırın.",
                                before=put(f, **{"sivas-bel.eslem.json": "sivas-bel.eslem.json"})))
    cases.append(remove_refusal("geçersiz beklenen sürüm", [p], {"kind": "profile", "name": "peyzaj", "expectedRevision": "on iki"},
                                "invalid_revision", INVALID_REVISION, path="expectedRevision"))
    return cases


# ── Files ────────────────────────────────────────────────────────────────────

def files():
    def wrap(command, title, cases):
        return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
                "note": "Written by scripts/fixtures/profile_files_cases.py from the contract's rules, docs/adr/0558 and docs/adr/0735; do not edit by hand.",
                "setup": PC.empty_doc(), "cases": cases}
    out = {
        OUT / f"{IMPORT}.json": wrap(IMPORT, "Veri profili içe aktar", import_cases()),
        OUT / f"{LIST}.json": wrap(LIST, "Veri profillerini listele", list_cases()),
        OUT / f"{MIGRATE}.json": wrap(MIGRATE, "Veri profili göçü", migrate_cases()),
        OUT / f"{REMOVE}.json": wrap(REMOVE, "Veri profilini çizimden kaldır", remove_cases()),
    }
    for name, value in BASE_FILES.items():
        out[FILES / name] = value
    out[REF / "park-r1.json"] = peyzaj_doc()
    return out


def main() -> int:
    bad_files = 0
    for path, doc in files().items():
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad_files += 1
            else:
                print(f"{path.name} güncel" + (f": {len(doc['cases'])} durum." if "cases" in doc else "."))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)}" + (f" ({len(doc['cases'])} durum)" if "cases" in doc else ""))
    for name, text in RAW_FILES.items():
        path = FILES / name
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil", file=sys.stderr)
                bad_files += 1
        else:
            path.write_text(text, encoding="utf-8")
    return 1 if bad_files else 0


if __name__ == "__main__":
    sys.exit(main())
