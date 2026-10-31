#!/usr/bin/env python3
"""The office material library (docs/adr/0791): `cad.materials.edit`'s import and export, from the ADR's rules, without
KentOS code.

Writes fixtures/commands/v1/cad.materials.edit.library.json; `--check` compares and writes nothing. The drawing is the
one `materials_cases.py` sets up (its model of the bindings is used for what follows a replaced material). The rules,
as the ADR gives them:

- a library is a `kentos.materials` 1 file (its name not blank, its materials holding a project's rules: ids and names
  each once) or KentOS's own by its name (`kentos-temel`, `catalog/v1/materials/kentos-temel.json`); exactly one of the
  two is given (`invalid_library` otherwise, and for a broken file or an unknown name);
- `materials` names those taken (each by its id, else its name, case aside; one not in the library is
  `material_not_found`); none: all; they are taken in the library's order;
- a library material the project has the same of (same id, same fields) is passed over; one of an id the project has is
  refused (`material_exists`, onConflict refuse), kept (keep: `report.kept`, warning `material_kept`) or put in its place
  (replace: `report.replaced`, everything bound to it following, those on locked layers reported); one of a name
  another project material has (case aside) is kept with keep and refused otherwise; a new one is added
  (`report.taken`);
- nothing taken and nothing replaced is `nothing_to_set`; the import is one undo step “Malzeme”;
- export gives the project's materials (those named, in the project's order; none: all) as a library's content,
  writing nothing; a blank library name is `invalid_library`, a project without materials `nothing_to_set`.
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import materials_cases as M  # noqa: E402  (the reference model of ADR 0785, itself without KentOS code)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.materials.edit.library.json"
KENTOS = ROOT / "catalog/v1/materials/kentos-temel.json"
FORMAT, VERSION = "kentos.materials", 1
REPORT = {"styles": [], "walls": [], "slabs": [], "locked": [], "left": []}


def builtin(name):
    return json.loads(KENTOS.read_text()) if name == "kentos-temel" else None


def hex_ok(c):
    return c is None or (len(c) == 7 and c[0] == "#" and all(x in "0123456789abcdefABCDEF" for x in c[1:]))


def material_problem(m):
    if not M.id_ok(m["id"]):
        return "id", f"“{m['id']}” malzeme kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf."
    if M.name_problem(m["name"]):
        return "name", M.name_problem(m["name"])
    h = m.get("hatch")
    if h is not None:
        if not h["pattern"].strip():
            return "hatch.pattern", f"“{m['name']}” malzemesinin tarama deseni boş."
        if not h["scale"] > 0:
            return "hatch.scale", f"“{m['name']}” malzemesinin tarama ölçeği sıfırdan büyük olmalı."
        if not hex_ok(h.get("color")):
            return "hatch.color", f"“{m['name']}” malzemesinin tarama rengi #RRGGBB olmalı."
    e = m.get("cut")
    if e is not None:
        if not hex_ok(e.get("color")):
            return "cut.color", f"“{m['name']}” malzemesinin kesit kaleminin rengi #RRGGBB olmalı."
        w = e.get("lineWeight")
        if w is not None and not 0 <= w <= 100:
            return "cut.lineWeight", f"“{m['name']}” malzemesinin kesit kalemi 0 ile 100 mm arasında olmalı."
    return None


def table_problem(ms):
    if len(ms) > 256:
        return "materials", f"Bir projede en çok 256 malzeme olur; {len(ms)} verildi."
    for i, m in enumerate(ms):
        p = material_problem(m)
        if p:
            return f"materials[{i}].{p[0]}", p[1]
        for j, o in enumerate(ms[:i]):
            if o["id"] == m["id"]:
                return f"materials[{i}].id", f"“{m['id']}” kimliği iki malzemede ({j + 1}. ve {i + 1}.) var."
        if any(o["name"].lower() == m["name"].lower() for o in ms[:i]):
            return f"materials[{i}].name", f"Projede “{m['name']}” adında bir malzeme zaten var; her malzemenin adı ayrı olmalı."
    return None


def library_of(op):
    has_file, has_name = "library" in op, "name" in op
    if has_file == has_name:
        raise M.Refused("invalid_library",
                        "Bir kitaplık dosyası (library) ya da KentOS kitaplığının adı (name) verilir; ikisi birden ya da hiçbiri değil.",
                        "operation")
    if has_name:
        lib = builtin(op["name"])
        if lib is None:
            raise M.Refused("invalid_library",
                            f"KentOS'un “{op['name']}” adlı malzeme kitaplığı yok; kentos-temel verin ya da bir kitaplık dosyası seçin.",
                            "operation.name")
    else:
        lib = op["library"]
    if lib["format"] != FORMAT or lib["version"] != VERSION:
        raise M.Refused("invalid_library",
                        f"Malzeme kitaplığı {FORMAT} {VERSION} biçiminde olmalı; “{lib['format']}” {lib['version']} verildi.",
                        "operation.library.format")
    if not lib["library"].strip():
        raise M.Refused("invalid_library", "Malzeme kitaplığının adı boş olmamalı.", "operation.library.library")
    p = table_problem(lib["materials"])
    if p:
        raise M.Refused("invalid_library", p[1], f"operation.library.{p[0]}")
    return lib


def chosen(source, keys, where):
    if not keys:
        return copy.deepcopy(source)
    at = []
    for i, k in enumerate(keys):
        j = next((n for n, m in enumerate(source) if m["id"] == k), None)
        if j is None:
            j = next((n for n, m in enumerate(source) if m["name"].lower() == k.lower()), None)
        if j is None:
            raise M.Refused("material_not_found",
                            f"{where} “{k}” kimlikli ya da adlı malzeme yok. Listedeki bir malzemeyi verin.",
                            f"operation.materials[{i}]")
        if j not in at:
            at.append(j)
    return [copy.deepcopy(source[j]) for j in sorted(at)]


def revision(inp):
    if "expectedRevision" not in inp:
        return
    if inp["expectedRevision"].isdigit():
        r = M.Refused("revision_conflict",
                      "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                      "expectedRevision")
        r.result["status"] = "conflict"
        r.result["error"]["revision"] = "$current"
        raise r
    raise M.Refused("invalid_revision",
                    f"Beklenen sürüm “{inp['expectedRevision']}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                    "expectedRevision")


def run(doc, inp):
    """The command on `doc`: (result, the document after, whether it wrote) — refused as `M.Refused`."""
    table = copy.deepcopy(doc["settings"]["arch"].get("materials", []))
    styles = copy.deepcopy(doc["settings"].get("wallStyles", []))
    op = inp["operation"]
    if op["kind"] == "export":
        if not op["library"].strip():
            raise M.Refused("invalid_library", "Malzeme kitaplığının adı boş olmamalı.", "operation.library")
        if not table:
            raise M.Refused("nothing_to_set",
                            "Projede kitaplığa yazılacak malzeme yok. Önce Yeni malzeme ya da Kitaplıktan al ile malzeme açın.",
                            "operation")
        picked = chosen(table, op.get("materials", []), "Projede")
        revision(inp)
        out = {"materials": table, "report": copy.deepcopy(REPORT),
               "library": {"format": FORMAT, "version": VERSION, "library": op["library"].strip(), "materials": picked},
               "revision": "$current"}
        return {"status": "completed", "output": out, "warnings": []}, doc, False
    lib = library_of(op)
    picked = chosen(lib["materials"], op.get("materials", []), "Kitaplıkta")
    on = op.get("onConflict", "refuse")
    taken, replaced, kept = [], [], []
    for m in picked:
        same_id = next((j for j, p in enumerate(table) if p["id"] == m["id"]), None)
        same_name = next((j for j, p in enumerate(table) if p["name"].lower() == m["name"].lower() and p["id"] != m["id"]), None)
        if same_id is not None and M.norm(table[same_id]) == M.norm(m):
            continue
        if on == "keep" and (same_id is not None or same_name is not None):
            kept.append(m["id"])
        elif same_id is not None and same_name is None and on == "replace":
            table[same_id] = m
            replaced.append(m["id"])
        elif same_id is not None and on == "refuse":
            raise M.Refused("material_exists",
                            f"Projede “{m['id']}” kimlikli bir malzeme (“{table[same_id]['name']}”) zaten var; projeninkini korumak için Koru, kitaplığınkini almak için Değiştir seçin.",
                            "operation.onConflict")
        elif same_name is not None:
            p = table[same_name]
            raise M.Refused("material_exists",
                            f"Projede “{p['name']}” adında başka kimlikli (“{p['id']}”) bir malzeme var; kitaplığın “{m['id']}” malzemesi alınamaz. Koru seçin ya da projedekinin adını değiştirin.",
                            "operation.onConflict")
        else:
            table.append(m)
            taken.append(m["id"])
    if not taken and not replaced:
        raise M.Refused("nothing_to_set",
                        "Kitaplıktan alınacak ya da değişecek malzeme yok: seçilenlerin hepsi projede aynısıyla var ya da korunuyor.",
                        "operation")
    p = table_problem(table)
    if p:
        raise M.Refused("invalid_material", p[1], f"operation.{p[0]}")
    revision(inp)
    after, updates, report = M.follow(doc, styles, table, {})
    for k, v in (("taken", taken), ("replaced", replaced), ("kept", kept)):
        if v:
            report[k] = v
    warnings = []
    if report["locked"]:
        warnings.append({"code": "material_locked",
                         "message": f"Malzemeye bağlı {len(report['locked'])} nesne kilitli katmanda; eski değerleriyle kaldı. Kilidi açıp yeniden kaydedin.",
                         "path": "report.locked"})
    if kept:
        warnings.append({"code": "material_kept",
                         "message": f"Kitaplığın {', '.join('“' + t + '”' for t in kept)} malzemesi projede zaten vardı; projeninki korundu, kitaplığınki alınmadı.",
                         "path": "report.kept"})
    out = {"materials": table, "report": report, "revision": "$current"}
    doc2 = copy.deepcopy(doc)
    doc2["settings"]["arch"]["materials"] = table
    doc2["settings"]["wallStyles"] = after
    for e in doc2["entities"]:
        if e["id"] in updates:
            nxt = updates[e["id"]]
            e.clear()
            e.update(nxt)
    return {"status": "completed", "output": out, "warnings": warnings}, doc2, True


def steps_for(inp, undo=False, setup=None):
    doc = setup or M.setup()
    out = []
    try:
        result, after, written = run(doc, inp)
    except M.Refused as r:
        for op in ("validate", "plan", "execute"):
            out.append({"op": op, "input": inp, "result": copy.deepcopy(r.result), "expect": {"revision": "same"}})
        return out
    out.append({"op": "plan", "input": inp, "result": result, "expect": {"revision": "same"}})
    out.append({"op": "execute", "input": inp, "result": result,
                "expect": M.expect_of(after, written) if written else {"revision": "same"}})
    if undo:
        out.append({"op": "undo", "returns": M.STEP, "expect": M.expect_of(doc, True)})
        out.append({"op": "redo", "returns": M.STEP, "expect": M.expect_of(after, True)})
    return out


def office(materials, name="Ofis malzemeleri", fmt=FORMAT, version=VERSION):
    return {"format": fmt, "version": version, "library": name, "materials": materials}


TASYUNU = {"id": "tasyunu", "name": "Taşyünü", "category": "insulation", "hatch": {"pattern": "ANSI37", "scale": 0.5, "angle": 0.0},
           "cut": {"lineWeight": 0.18}}
SIVA = {"id": "ic-siva", "name": "İç sıva", "category": "finish", "quantity": "area"}
TUGLA_NEW = {"id": "tugla", "name": "Tuğla duvar", "category": "structure",
             "hatch": {"pattern": "BRICK", "scale": 0.8, "angle": 0.0, "color": "#A0522D"}, "cut": {"lineWeight": 0.35}}
BETON_OTHER = {"id": "beton-c30", "name": "BETONARME", "category": "structure"}


def cases():
    c = []

    def case(name, inp, undo=False, setup=None):
        entry = {"name": name}
        if setup is not None:
            entry["setup"] = setup
        entry["steps"] = steps_for(inp, undo, setup)
        c.append(entry)

    def imp(**kw):
        return {"operation": {"kind": "import", **kw}}

    def exp(**kw):
        return {"operation": {"kind": "export", **kw}}

    case("KentOS temel: hepsi, projede olanlar korunur", imp(name="kentos-temel", onConflict="keep"), undo=True)
    case("KentOS temel: seçilen ikisi, çakışma yok", imp(name="kentos-temel", materials=["tasyunu", "eps"]))
    case("KentOS temel: adıyla seçilen, büyük küçük harf ayrımsız", imp(name="kentos-temel", materials=["TAŞYÜNÜ", "İç sıva"]))
    case("KentOS temel: hepsi, aynı kimlik reddedilir", imp(name="kentos-temel"))
    case("KentOS temel: hepsi, aynı kimlikliler değişir ve bağlılar izler", imp(name="kentos-temel", onConflict="replace"),
         undo=True)
    case("KentOS temel: bilinmeyen kitaplık adı", imp(name="kentos-ofis"))
    case("Dosya ve ad birlikte verilmez", imp(name="kentos-temel", library=office([TASYUNU])))
    case("Dosya da ad da verilmedi", imp(materials=["tasyunu"]))
    case("Ofis kitaplığı: yeni iki malzeme", imp(library=office([TASYUNU, SIVA])), undo=True)
    case("Ofis kitaplığı: biçimi başka", imp(library=office([TASYUNU], fmt="kentos.blocks")))
    case("Ofis kitaplığı: sürümü 2", imp(library=office([TASYUNU], version=2)))
    case("Ofis kitaplığı: adı boş", imp(library=office([TASYUNU], name="  ")))
    case("Ofis kitaplığı: aynı kimlik iki kez", imp(library=office([TASYUNU, dict(SIVA, id="tasyunu")])))
    case("Ofis kitaplığı: büyük harfli kimlik", imp(library=office([dict(TASYUNU, id="Tasyunu")])))
    case("Ofis kitaplığı: tarama ölçeği sıfır",
         imp(library=office([dict(TASYUNU, hatch={"pattern": "ANSI37", "scale": 0.0, "angle": 0.0})])))
    case("Ofis kitaplığı: kalem rengi geçersiz", imp(library=office([dict(TASYUNU, cut={"color": "kırmızı"})])))
    case("Ofis kitaplığı: başka kimlikli aynı ad reddedilir", imp(library=office([TASYUNU, BETON_OTHER])))
    case("Ofis kitaplığı: başka kimlikli aynı ad korunur, öbürü alınır",
         imp(library=office([TASYUNU, BETON_OTHER]), onConflict="keep"))
    case("Ofis kitaplığı: başka kimlikli aynı ad değiştirmede de reddedilir",
         imp(library=office([TASYUNU, BETON_OTHER]), onConflict="replace"))
    case("Ofis kitaplığı: projedekiyle aynı malzeme, alınacak bir şey yok", imp(library=office([M.MATERIALS[0]])))
    case("Ofis kitaplığı: hepsi korunuyor, alınacak bir şey yok",
         imp(library=office([TUGLA_NEW]), onConflict="keep"))
    case("Ofis kitaplığı: seçilen kitaplıkta yok", imp(library=office([TASYUNU]), materials=["camyunu"]))
    case("Ofis kitaplığı: tuğla değişir, stil, duvarlar ve kendi duvarı izler, kilitli kalır",
         imp(library=office([TUGLA_NEW]), onConflict="replace"), undo=True)
    case("Kitaplıktan al: beklenen sürüm eski", dict(imp(name="kentos-temel", materials=["eps"]), expectedRevision="999"))
    case("Kitaplığa yaz: projenin bütün malzemeleri", exp(library="Ofis malzemeleri"))
    case("Kitaplığa yaz: adıyla seçilen, adın kenar boşlukları atılır", exp(library=" Beton ", materials=["BETONARME"]))
    case("Kitaplığa yaz: seçilen projede yok", exp(library="Ofis", materials=["kerpic"]))
    case("Kitaplığa yaz: kitaplığın adı boş", exp(library=" "))
    empty = M.setup()
    empty["settings"]["arch"].pop("materials")
    case("Kitaplığa yaz: projede malzeme yok", exp(library="Ofis"), setup=empty)
    case("Kitaplığa yaz: beklenen sürüm sayı değil", dict(exp(library="Ofis"), expectedRevision="x"))
    return c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    data = {
        "format": "kentos.command-cases", "version": 1, "command": "cad.materials.edit", "commandVersion": 1,
        "title": "Malzeme kitaplığı",
        "note": "ADR 0791. Beklenenler scripts/fixtures/material_library_cases.py'nin ADR'den yazılmış kurallarından, KentOS "
                "kodu olmadan (bağların izlenmesi materials_cases.py'nin modeliyle); KentOS temel kitaplığı "
                "catalog/v1/materials/kentos-temel.json'dan okunur. Kurulum materials_cases.py'ninki.",
        "setup": M.setup(),
        "cases": cases(),
    }
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
