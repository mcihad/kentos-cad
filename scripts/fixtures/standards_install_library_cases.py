#!/usr/bin/env python3
"""The shared cases of cad.standards.install's block library references and sheet template revisions
(docs/adr/1100 §2–§3), written from the ADR without KentOS code.

    python3 scripts/fixtures/standards_install_library_cases.py           # writes fixtures/commands/v1/cad.standards.install.library.json
    python3 scripts/fixtures/standards_install_library_cases.py --check   # writes nothing; compares

The libraries are read as data: KentOS's block library files (`catalog/v1/blocks/*.kblok`, in the order
`kentos-temel`, `kentos-ofis`) and the system sheet templates (`crates/shared/sheet/templates/*.json`, library
`sistem`). A block reference resolves to the first library block whose publication has its family, type and release
(and, when the reference names a unit, that unit); otherwise it is `unavailable`, with `release` when a library has
another release of the type, `units` when only the unit differs. A template reference resolves when a library lists
its id (case aside) at its revision; otherwise `unavailable`, with `revision` when another revision is listed.

An input's `libraries` (docs/adr/1019) are absolute `.kblok` paths read in the order given, before KentOS's; a row's
`library` is the file's stem. The project libraries here are written by this script to
fixtures/commands/v1/standards-libraries/: `proje` holds BILINMEYEN's publication and a KAPI release 2 in metres
that differs from kentos-ofis's (one line more), `ikinci` another BILINMEYEN. A path not absolute or not `.kblok` is
`invalid_file_path`, a missing file `file_not_found`, a file that is not `kentos.blocks` `block_library_unreadable`;
the path of each is `libraries[i]`, and nothing is read past the first refused.

A resolved block is missing, same or differs against the drawing's block of the reference's name (case aside); the
comparison leaves out the id, the name, the description and the card but the card's publication; an insert inside is
compared by its block's name. A resolved template is missing, same or differs (`revision`) against the drawing's
record of that id. Choices: missing add|keepLocal, same keepLocal, differs keepLocal|replace|renameIncoming (a
template: no renameIncoming), unavailable keepLocal. Replace keeps the drawing's id and name, so inserts follow; an
added block takes the library's id unless the drawing has it. `uses`: a block's inserts in the drawing and in its
definitions; a template's sheets made from it (never changed).
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.standards.install.library.json"
STEP = "Standart kaynaklarını kur"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

LIBRARY_DIR = ROOT / "fixtures/commands/v1/standards-libraries"
LIBRARIES = [(name, json.loads((ROOT / f"catalog/v1/blocks/{name}.kblok").read_text("utf-8"))["blocks"])
             for name in ("kentos-temel", "kentos-ofis")]
TEMPLATES = [json.loads(p.read_text("utf-8"))["meta"] for p in sorted((ROOT / "crates/shared/sheet/templates").glob("*.json"))]

DOOR = {"familyId": "0192f5b1-0000-7000-8000-0000000f0001", "typeId": "0192f5b1-0000-7000-8000-0000000e0001",
        "typeName": "Tek kanatlı kapı 90"}
NORTH = {"familyId": "0192f5b1-0000-7000-8000-0000000f0002", "typeId": "0192f5b1-0000-7000-8000-0000000e0002",
         "typeName": "Kuzey oku"}
LEVEL = {"familyId": "0192f5b1-0000-7000-8000-0000000f0002", "typeId": "0192f5b1-0000-7000-8000-0000000e0003",
         "typeName": "Kot işareti"}
UNKNOWN = {"familyId": "0192f5b1-0000-7000-8000-0000000f0009", "typeId": "0192f5b1-0000-7000-8000-0000000e0009",
           "typeName": "Bilinmeyen tip"}

def project_libraries():
    """The libraries this script writes, by stem: (stem, file text)."""
    door = copy.deepcopy(next(b for b in json.loads((ROOT / "catalog/v1/blocks/kentos-ofis.kblok").read_text("utf-8"))["blocks"]
                              if (b.get("card") or {}).get("publication", {}).get("release") == 2
                              and b["card"]["publication"]["typeId"] == DOOR["typeId"]))
    door["id"] = "0192f5b1-0000-7000-8000-0000000000a2"
    door["name"] = "Proje kapı 90 · yayın 2"
    door["entities"].append({"kind": "line", "id": 99, "layerId": "", "attrs": {}, "a": {"x": 0.0, "y": 0.9},
                             "b": {"x": 0.9, "y": 0.9}})
    unknown = lambda id_, name, shape: {"id": id_, "name": name, "base": {"x": 0.0, "y": 0.0}, "entities": [shape],
                                        "card": {"publication": dict(UNKNOWN, release=1), "version": 1}}
    proje = [unknown("0192f5b1-0000-7000-8000-0000000000a1", "Proje bilinmeyen",
                     {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 1.0, "y": 0.0}}),
             door]
    ikinci = [unknown("0192f5b1-0000-7000-8000-0000000000b1", "İkinci bilinmeyen",
                      {"kind": "circle", "id": 1, "layerId": "", "attrs": {}, "c": {"x": 0.0, "y": 0.0}, "r": 0.5})]
    text = lambda blocks: json.dumps({"format": "kentos.blocks", "version": 2, "blocks": blocks}, ensure_ascii=False, indent=1) + "\n"
    return {"proje": text(proje), "ikinci": text(ikinci),
            "bozuk": json.dumps({"format": "kentos.document", "version": 2}) + "\n"}


def given(stems):
    """The libraries `stems` names, as the library search reads them: (name, blocks)."""
    made = project_libraries()
    return [(stem, json.loads(made[stem])["blocks"]) for stem in stems]


def path_of(stem):
    return f"$fixture:commands/v1/standards-libraries/{stem}.kblok"


D_KAPI = "0192f5a0-0000-7000-8000-00000000d001"
D_KUZEY = "0192f5a0-0000-7000-8000-00000000d002"
D_PLAN = "0192f5a0-0000-7000-8000-00000000d003"
A = {"id": "ofis-a", "revision": "2026.1"}
B = {"id": "ofis-b", "revision": "2026.3"}


def ref(name, pub, release, units=None):
    r = {"name": name, "publication": dict(pub, release=release)}
    if units:
        r["units"] = units
    return r


A_RES = {
    "blocks": [ref("KAPI", DOOR, 2, "m"), ref("KUZEY", NORTH, 1), ref("ESKI-KAPI", DOOR, 9),
               ref("BILINMEYEN", UNKNOWN, 1), ref("KUZEY-MM", NORTH, 1, "mm")],
    "sheetTemplates": [{"templateId": "sys:cad-mimari", "revision": 1}, {"templateId": "sys:cad-teknik", "revision": 1},
                       {"templateId": "sys:genel-a4-dikey", "revision": 7}, {"templateId": "ofis:a3-yatay", "revision": 1}],
}
B_RES = {
    "blocks": [ref("kapı", DOOR, 1), ref("KOT", LEVEL, 1)],
    "sheetTemplates": [{"templateId": "SYS:CAD-MIMARI", "revision": 3}],
}


def fold(name):
    return name.replace("I", "ı").replace("İ", "i").lower()


def library_block(r, extra=()):
    """(library, block, its library's blocks) or (None, fields); `extra`: the given libraries' stems, first."""
    p, nearest = r["publication"], None
    for name, blocks in given(extra) + LIBRARIES:
        for b in blocks:
            q = (b.get("card") or {}).get("publication")
            if not q or q["familyId"] != p["familyId"] or q["typeId"] != p["typeId"]:
                continue
            if q["release"] != p["release"]:
                nearest = nearest or ["release"]
                continue
            if "units" in r and b.get("units") != r["units"]:
                nearest = ["units"]
                continue
            return name, b, blocks
    return None, nearest or []


def library_template(r):
    other = False
    for meta in TEMPLATES:
        if meta["id"].lower() == r["templateId"].lower():
            if meta["revision"] == r["revision"]:
                return "sistem", None
            other = True
    return None, ["revision"] if other else []


def library_copy(name):
    for _, blocks in LIBRARIES:
        for b in blocks:
            if b["name"] == name:
                return copy.deepcopy(b)
    raise KeyError(name)


def local_blocks():
    kapi = library_copy("Ofis kapı 90 · yayın 1")
    kapi.update(id=D_KAPI, name="KAPI")
    kuzey = library_copy("Ofis kuzey oku")
    kuzey.update(id=D_KUZEY, name="kuzey")
    plan = {"id": D_PLAN, "name": "PLAN", "base": {"x": 0.0, "y": 0.0}, "entities": [
        {"kind": "insert", "id": 1, "layerId": "0", "attrs": {}, "block": D_KAPI, "p": {"x": 5.0, "y": 0.0},
         "scale": 1.0, "rotation": 0.0}]}
    return [kapi, kuzey, plan]


def insert(n, block, x):
    return {"kind": "insert", "id": n, "layerId": "0", "attrs": {}, "block": block, "p": {"x": x, "y": 0.0},
            "scale": 1.0, "rotation": 0.0}


PAGE = {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
        "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}}
OLD_APPROVED = [{"templateId": "sys:cad-mimari", "revision": 3, "standard": "eski-ofis", "standardRevision": "2025.1"}]


def package(id_, rev, res):
    return {"format": "kentos.drawing-standard", "version": 2, "id": id_, "revision": rev,
            "standard": {"name": id_, "layers": []}, "resources": res}


def setup(blocks=None):
    def sheet(id_, template, revision):
        return {"id": id_, "name": id_.upper(), "page": PAGE, "items": [],
                "origin": {"templateId": template, "revision": revision}}
    return {"format": "kentos.document", "version": 2, "name": "Ofis kitaplığı",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 100.0, "workspace": "cad",
                         "standardDocs": [{"text": json.dumps(package("ofis-a", "2026.1", A_RES), ensure_ascii=False)},
                                          {"text": json.dumps(package("ofis-b", "2026.3", B_RES), ensure_ascii=False)}],
                         "approvedSheetTemplates": copy.deepcopy(OLD_APPROVED)},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                        "style": STYLE, "children": []}],
            "activeLayer": "0",
            "entities": [insert(1, D_KAPI, 0.0), insert(2, D_KAPI, 2.0), insert(3, D_KUZEY, 4.0),
                         insert(4, D_PLAN, 10.0)],
            "uids": [f"0192f5a1-2222-7000-8000-00000000000{n}" for n in range(1, 5)],
            "styles": {"items": [], "categories": []},
            "blocks": blocks if blocks is not None else local_blocks(),
            "sheets": {"book": {"schema": "kentos.sheet/1",
                                "sheets": [sheet("a-101", "sys:cad-mimari", 3), sheet("a-102", "sys:cad-mimari", 3),
                                           sheet("t-101", "sys:cad-teknik", 1)],
                                "masters": [], "assets": []}}}


def first_resources():
    """cad.standards.resolve's answer: each name (case aside) from the first standard that has it."""
    blocks, seen = [], set()
    for origin, res in ((A, A_RES), (B, B_RES)):
        for r in res["blocks"]:
            if fold(r["name"]) not in seen:
                seen.add(fold(r["name"]))
                blocks.append((r, origin))
    templates, seen = [], set()
    for origin, res in ((A, A_RES), (B, B_RES)):
        for r in res["sheetTemplates"]:
            if r["templateId"].lower() not in seen:
                seen.add(r["templateId"].lower())
                templates.append((r, origin))
    return blocks, templates


def look(block, names):
    v = {k: copy.deepcopy(x) for k, x in block.items() if k not in ("id", "name", "description", "card")}
    v["publication"] = (block.get("card") or {}).get("publication")
    for e in v.get("entities", []):
        if e["kind"] == "insert":
            e["block"] = names.get(e["block"], "")
    return v


def differing(a, b):
    def equal(x, y):
        if isinstance(x, (int, float)) and isinstance(y, (int, float)) and not isinstance(x, bool):
            return abs(x - y) <= 1e-9
        if isinstance(x, list) and isinstance(y, list):
            return len(x) == len(y) and all(equal(p, q) for p, q in zip(x, y))
        if isinstance(x, dict) and isinstance(y, dict):
            return x.keys() == y.keys() and all(equal(x[k], y[k]) for k in x)
        return x == y
    return sorted(k for k in set(a) | set(b) if not (k in a and k in b and equal(a[k], b[k])))


def uses_of(blocks, entities, id_):
    every = entities + [e for b in blocks for e in b["entities"]]
    return sum(1 for e in every if e["kind"] == "insert" and e["block"] == id_)


def rows(s=None, extra=()):
    s = s or setup()
    blocks, templates = first_resources()
    names = {b["id"]: b["name"] for b in s["blocks"]}
    out = []
    for r, origin in blocks:
        local = next((b for b in s["blocks"] if fold(b["name"]) == fold(r["name"])), None)
        row = {"kind": "block", "name": r["name"], "status": "unavailable", "action": "keepLocal", "source": origin}
        found = library_block(r, extra)
        if found[0] is None:
            if found[1]:
                row["fields"] = found[1]
        else:
            library, block, all_ = found
            row["library"] = library
            if local is None:
                row["status"], row["action"] = "missing", "add"
            else:
                fields = differing(look(local, names), look(block, {b["id"]: b["name"] for b in all_}))
                row["status"] = "differs" if fields else "same"
                if fields:
                    row["fields"] = fields
        if local is not None and uses_of(s["blocks"], s["entities"], local["id"]):
            row["uses"] = uses_of(s["blocks"], s["entities"], local["id"])
        out.append(row)
    made = {}
    for sh in s["sheets"]["book"]["sheets"]:
        made[sh["origin"]["templateId"].lower()] = made.get(sh["origin"]["templateId"].lower(), 0) + 1
    for r, origin in templates:
        local = next((t for t in s["settings"]["approvedSheetTemplates"] if t["templateId"].lower() == r["templateId"].lower()), None)
        row = {"kind": "sheetTemplate", "name": r["templateId"], "status": "unavailable", "action": "keepLocal",
               "source": origin}
        library, fields = library_template(r)
        if library is None:
            if fields:
                row["fields"] = fields
        else:
            row["library"] = library
            if local is None:
                row["status"], row["action"] = "missing", "add"
            elif local["revision"] == r["revision"]:
                row["status"] = "same"
            else:
                row["status"], row["fields"] = "differs", ["revision"]
        if made.get(r["templateId"].lower()):
            row["uses"] = made[r["templateId"].lower()]
        out.append(row)
    return out


def chosen(choices, s=None, extra=()):
    out = rows(s, extra)
    for c in choices:
        for r in out:
            if r["kind"] == c["kind"] and r["name"].lower() == c["name"].lower():
                r["action"] = c["action"]
                if "newName" in c:
                    r["newName"] = c["newName"]
    return out


def answer(rs):
    n = lambda a: sum(1 for r in rs if r["action"] == a and r["status"] != "same")
    return {"rows": rs, "added": n("add"), "replaced": n("replace"), "renamed": n("renameIncoming"),
            "kept": sum(1 for r in rs if r["action"] == "keepLocal"),
            "unavailable": sum(1 for r in rs if r["status"] == "unavailable"), "revision": "$current"}


def installed(rs, s=None, extra=()):
    """The drawing's blocks (without ids, inserts by `$blockOf:`), their ids and approved templates after `rs`."""
    s = s or setup()
    blocks = copy.deepcopy(s["blocks"])
    taken = {b["id"] for b in blocks}
    ids = {}
    blocks_of = {r["name"]: r for r, _ in first_resources()[0]}
    for r in rs:
        if r["kind"] != "block" or r["status"] in ("same", "unavailable") or r["action"] == "keepLocal":
            continue
        _, lib, _ = library_block(blocks_of[r["name"]], extra)
        new = copy.deepcopy(lib)
        if r["action"] == "replace":
            i = next(i for i, b in enumerate(blocks) if fold(b["name"]) == fold(r["name"]))
            new["id"], new["name"] = blocks[i]["id"], blocks[i]["name"]
            blocks[i] = new
            ids[new["name"]] = new["id"]
        else:
            new["name"] = r.get("newName", r["name"])
            if new["id"] in taken:
                ids[new["name"]] = "new"
            else:
                ids[new["name"]] = new["id"]
            taken.add(new["id"])
            blocks.append(new)
    names = {b["id"]: b["name"] for b in blocks}
    shown = []
    for b in blocks:
        b = copy.deepcopy(b)
        for e in b["entities"]:
            if e["kind"] == "insert" and e["block"] in names:
                e["block"] = "$blockOf:" + names[e["block"]]
        b.pop("id")
        shown.append(b)
    approved = copy.deepcopy(s["settings"]["approvedSheetTemplates"])
    templates = {t["templateId"].lower(): (t, o) for t, o in first_resources()[1]}
    for r in rs:
        if r["kind"] != "sheetTemplate":
            continue
        t, o = templates[r["name"].lower()]
        record = {"templateId": t["templateId"], "revision": t["revision"], "standard": o["id"],
                  "standardRevision": o["revision"]}
        if r["status"] == "missing" and r["action"] == "add":
            approved.append(record)
        elif r["status"] == "differs" and r["action"] == "replace":
            local = next(x for x in approved if x["templateId"].lower() == r["name"].lower())
            local.update(revision=record["revision"], standard=record["standard"],
                         standardRevision=record["standardRevision"])
    return shown, ids, approved


def done(output):
    return {"status": "completed", "output": output, "warnings": []}


def choice(kind, name, action, new_name=None):
    c = {"kind": kind, "name": name, "action": action}
    if new_name is not None:
        c["newName"] = new_name
    return c


def writes(rs):
    return any(r["status"] not in ("same", "unavailable") and r["action"] != "keepLocal" for r in rs)


def run(choices, op="execute", s=None, case_setup=None, libraries=()):
    rs = chosen(choices, case_setup, libraries)
    step = {"op": op, "input": {"choices": choices} if choices else {}}
    if libraries:
        step["input"]["libraries"] = [path_of(stem) for stem in libraries]
    step["result"] = done(None) if op == "validate" else done(answer(rs))
    if op == "execute" and writes(rs):
        blocks, ids, approved = installed(rs, case_setup, libraries)
        step["expect"] = {"revision": "changed", "canUndo": True, "blocks": blocks, "blockIds": ids,
                          "settings": {"approvedSheetTemplates": approved}}
    else:
        blocks, _, approved = installed([], case_setup)
        step["expect"] = {"revision": "same", "canUndo": False, "blocks": blocks,
                          "settings": {"approvedSheetTemplates": approved}}
    return step


def refused(choices, code, message, path, libraries=None):
    blocks, _, approved = installed([])
    step_input = {"choices": choices} if choices else {}
    if libraries is not None:
        step_input["libraries"] = libraries
    return {"op": "execute", "input": step_input,
            "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
            "expect": {"revision": "same", "canUndo": False, "blocks": blocks,
                       "settings": {"approvedSheetTemplates": approved}}}


SKIP_DEFAULTS = [choice("block", "KOT", "keepLocal"), choice("sheetTemplate", "sys:cad-teknik", "keepLocal")]


def cases():
    out = []

    def case(name, steps, case_setup=None):
        c = {"name": name, "steps": steps}
        if case_setup is not None:
            c["setup"] = case_setup
        out.append(c)

    case("Seçimsiz: eksik KOT bloğu ve onaylı teknik şablon eklenir, farklılar korunur", [run([])])
    case("Plan yazmaz; kitaplıkta olmayanlar ayrı bulgu satırıdır", [run([], op="plan")])
    case("Doğrulama yazmaz", [run([], op="validate")])
    case("Farklı KAPI yerinde değiştirilir: çizimdeki ad ve kimlik kalır, yerleştirmeler izler", [
        run([choice("block", "KAPI", "replace")] + SKIP_DEFAULTS)])
    case("Değiştirilen KAPI'yı PLAN bloğunun içindeki yerleştirme de izler", [
        run([choice("block", "kapi", "replace")])])
    case("Farklı KAPI yeni adla eklenir; çizimdeki kalır", [
        run([choice("block", "KAPI", "renameIncoming", "KAPI-R2")] + SKIP_DEFAULTS)])
    plan_on_library_id = local_blocks()
    plan_on_library_id[2]["id"] = "0192f5b1-0000-7000-8000-000000000002"
    taken_setup = setup(plan_on_library_id)
    taken_setup["entities"][3]["block"] = "0192f5b1-0000-7000-8000-000000000002"
    case("Yeni adla eklenen bloğun kitaplık kimliği çizimde varsa yeni kimlik alır", [
        run([choice("block", "KAPI", "renameIncoming", "KAPI-R2")] + SKIP_DEFAULTS, case_setup=taken_setup)],
        case_setup=taken_setup)
    case("Eksik KOT kitaplık kimliğiyle eklenir", [run([choice("sheetTemplate", "sys:cad-teknik", "keepLocal")])])
    case("Eksikler atlanınca yazım yok", [run(SKIP_DEFAULTS)])
    case("Onaylı mimari şablonun revizyonu değiştirilir; paftalar değişmez", [
        run([choice("sheetTemplate", "sys:cad-mimari", "replace")] + SKIP_DEFAULTS)])
    case("Kitaplıkta olmayan blok korunur ve bulgu olarak kalır", [
        run([choice("block", "ESKI-KAPI", "keepLocal"), choice("block", "BILINMEYEN", "keepLocal")], op="plan")])
    case("İlk standart önce gelir: KAPI ofis-a'nın 2. yayını, mimari şablon ofis-a'nın 1. revizyonu", [run([], op="plan")])
    case("Kurulumu geri al: bloklar ve onaylı şablonlar önceki hâline döner", [
        run([choice("block", "KAPI", "replace"), choice("sheetTemplate", "sys:cad-mimari", "replace")]),
        {"op": "undo", "returns": STEP, "expect": {"blocks": installed([])[0],
                                                   "settings": {"approvedSheetTemplates": installed([])[2]}}},
        {"op": "redo", "returns": STEP, "expect": {
            "blocks": installed(chosen([choice("block", "KAPI", "replace"), choice("sheetTemplate", "sys:cad-mimari", "replace")]))[0],
            "settings": {"approvedSheetTemplates": installed(chosen([choice("block", "KAPI", "replace"), choice("sheetTemplate", "sys:cad-mimari", "replace")]))[2]}}}])
    case("Aynı olan KUZEY değiştirilemez: ret", [refused([choice("block", "KUZEY", "replace")], "invalid_install_choice",
        "“KUZEY” blok çizimdekiyle aynı: değişecek bir şey yok.", "choices[0].action")])
    case("Kitaplıkta olmayan blok eklenemez: ret", [refused([choice("block", "ESKI-KAPI", "add")], "invalid_install_choice",
        "“ESKI-KAPI” blok okunabilen hiçbir kitaplıkta yok: kurulamaz, yalnız çizimdeki korunabilir.", "choices[0].action")])
    case("Birimi onaylanandan farklı yayın kurulamaz: ret", [refused([choice("block", "KUZEY-MM", "replace")], "invalid_install_choice",
        "“KUZEY-MM” blok okunabilen hiçbir kitaplıkta yok: kurulamaz, yalnız çizimdeki korunabilir.", "choices[0].action")])
    case("Kitaplıkta olmayan şablon kaydedilemez: ret", [refused([choice("sheetTemplate", "ofis:a3-yatay", "add")], "invalid_install_choice",
        "“ofis:a3-yatay” pafta şablonu okunabilen hiçbir kitaplıkta yok: kurulamaz, yalnız çizimdeki korunabilir.", "choices[0].action")])
    case("Şablon yeni adla eklenemez: ret", [refused([choice("sheetTemplate", "sys:cad-mimari", "renameIncoming", "X")], "invalid_install_choice",
        "“sys:cad-mimari” pafta şablonu kimliğiyle kaydedilir, yeni adla eklenemez: değiştirilebilir ya da çizimdeki korunabilir.", "choices[0].action")])
    case("Eksik KOT değiştirilemez: ret", [refused([choice("block", "KOT", "replace")], "invalid_install_choice",
        "“KOT” blok çizimde yok: eklenebilir ya da atlanabilir.", "choices[0].action")])
    case("Yeni ad çizimdeki bir blokta var: ret", [refused([choice("block", "KAPI", "renameIncoming", "PLAN")], "duplicate_name",
        "“PLAN” adlı blok çizimde ya da standartta var; başka bir ad verin.", "choices[0].newName")])
    case("Yeni ad standarttaki bir blokta var: ret", [refused([choice("block", "KAPI", "renameIncoming", "kot")], "duplicate_name",
        "“kot” adlı blok çizimde ya da standartta var; başka bir ad verin.", "choices[0].newName")])
    case("Standartta olmayan blok: ret", [refused([choice("block", "YOK", "add")], "standard_resource_not_found",
        "Bağlı standartlarda “YOK” adlı blok yok. Kaynağı standartların listesinden seçin.", "choices[0].name")])
    case("Standartta olmayan şablon: ret", [refused([choice("sheetTemplate", "sys:yok", "add")], "standard_resource_not_found",
        "Bağlı standartlarda “sys:yok” adlı pafta şablonu yok. Kaynağı standartların listesinden seçin.", "choices[0].name")])
    case("Aynı bloğa iki seçim: ret", [refused([choice("block", "KAPI", "replace"), choice("block", "kapi", "keepLocal")],
        "invalid_install_choice", "“kapi” blok için iki seçim verildi; her kaynağa bir seçim verin.", "choices[1]")])
    # docs/adr/1019: the project's own libraries, given in the input.
    case("Proje kitaplığı verilince kitaplıkta olmayan BILINMEYEN eksik olur ve eklenir; satırın kitaplığı “proje”", [
        run([], libraries=["proje"])])
    case("Proje kitaplığı KentOS'unkinden önce aranır: KAPI'nın 2. yayını projeninkidir", [
        run([], op="plan", libraries=["proje"])])
    case("Verilen kitaplıklar verilen sırayla aranır: önce ikinci", [
        run([choice("block", "KAPI", "keepLocal")] + SKIP_DEFAULTS, libraries=["ikinci", "proje"])])
    case("Verilen kitaplıklar verilen sırayla aranır: önce proje", [
        run([choice("block", "KAPI", "keepLocal")] + SKIP_DEFAULTS, libraries=["proje", "ikinci"])])
    case("Proje kitaplığındaki KAPI yerinde değiştirilir", [
        run([choice("block", "KAPI", "replace")] + SKIP_DEFAULTS, libraries=["proje"])])
    case("Kitaplıkla doğrulama yazmaz", [run([], op="validate", libraries=["proje"])])
    case("Kitaplık yolu göreli: ret", [refused([], "invalid_file_path",
        "“proje.kblok” bir blok kitaplığının (.kblok) mutlak yolu değil; tam yolu verin.", "libraries[0]", ["proje.kblok"])])
    wrong = "$fixture:commands/v1/standards-libraries/proje.json"
    case("Kitaplık .kblok değil: ret", [refused([], "invalid_file_path",
        f"“{wrong}” bir blok kitaplığının (.kblok) mutlak yolu değil; tam yolu verin.", "libraries[1]",
        [path_of("proje"), wrong])])
    missing = path_of("yok")
    case("Kitaplık yok: ret", [refused([], "file_not_found", f"“{missing}” yok; kitaplığın yolunu denetleyin.",
                                       "libraries[0]", [missing])])
    case("Kitaplık kentos.blocks değil: ret", [refused([], "block_library_unreadable",
        f"“{path_of('bozuk')}” okunamadı: biçimi kentos.blocks değil.", "libraries[0]", [path_of("bozuk")])])
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.standards.install", "commandVersion": 1,
            "title": "Standart kaynaklarını kur: blok kitaplığı ve pafta şablonu",
            "note": "ADR 1100. Bağımsız başvuru scripts/fixtures/standards_install_library_cases.py: kitaplıklar veri dosyalarından okunur, kurallar ADR'den elle yazılır; KentOS kodu yok.",
            "setup": setup(), "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    made = project_libraries()
    if "--check" in sys.argv[1:]:
        stale = [stem for stem, t in made.items()
                 if not (LIBRARY_DIR / f"{stem}.kblok").exists() or (LIBRARY_DIR / f"{stem}.kblok").read_text("utf-8") != t]
        if not OUT.exists() or OUT.read_text("utf-8") != text or stale:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    LIBRARY_DIR.mkdir(parents=True, exist_ok=True)
    for stem, t in made.items():
        (LIBRARY_DIR / f"{stem}.kblok").write_text(t, "utf-8")
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
