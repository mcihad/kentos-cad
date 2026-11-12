#!/usr/bin/env python3
"""The shared cases of `cad.profiles.carry`, Kataloğa taşı (plan/map-cad.md MAP-04, docs/adr/5340), written from the
ADR without KentOS code.

    python3 scripts/fixtures/profile_carry_cases.py           # writes the cases
    python3 scripts/fixtures/profile_carry_cases.py --check   # compares

The drawing and the two catalogs are the migration report's (profile_compare_cases.py, whose class tracking is the
report's reference); a receiving class's layer and fields are written as `cad.profiles.apply`'s reference writes them
(profile_cases.apply). The rules (ADR 5340), written here by hand:

- each old class with objects on the layer its path leads to, in the report's order: a same, changed, renamed or
  merged class sends every object to its one new class; a split class sends each object to the first of its new
  classes whose codes hold the object's code (the code field's leading capital letters; the field `Kod` unless given);
  a retired class's objects, and a split class's without such a code, stay and are counted `left`; an object already on
  its target's layer does not move;
- the new classes receiving objects, in the new catalog's order, are each written alone (the class, the lists its
  fields bind) as the profile writer writes them; the objects then go to their class's layer; values stay;
- refusals, in order: the revision, the catalogs (the report's own), a locked layer an object leaves, a locked layer
  one goes to, nothing to carry; one undo step “Kataloğa taşı”.
"""

import copy
import importlib.util
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.profiles.carry.json"
STEP = "Kataloğa taşı"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PC = _load("profile_compare_cases")
PR = _load("profile_cases")
OLD, NEW = PC.OLD, PC.NEW
SAME = {"revision": "same"}


def narrowed(p, cid):
    """The profile with only the class, the lists its fields bind, no relation or address."""
    out = copy.deepcopy(p)
    out["classes"] = [c for c in out["classes"] if c["id"] == cid]
    bound = {f["domain"] for c in out["classes"] for f in c["fields"] if f.get("domain")}
    out["domains"] = [d for d in out.get("domains", []) if d["id"] in bound]
    out["relations"] = []
    out.pop("address", None)
    return out


def code_of(e, field):
    v = (e["attrs"].get(field) or "").strip()
    letters = ""
    for ch in v:
        if "A" <= ch <= "Z":
            letters += ch
        else:
            break
    return letters or None


def locked(d, lid):
    def walk(nodes):
        for n in nodes:
            if n["id"] == lid:
                return n["locked"]
            r = walk(n.get("children", []))
            if r is not None:
                return r
        return None
    return bool(walk(d["layers"]))


def title_of(d, lid):
    def walk(nodes):
        for n in nodes:
            if n["id"] == lid:
                return n["name"]
            r = walk(n.get("children", []))
            if r is not None:
                return r
        return None
    return walk(d["layers"])


def carry(d, old, new, field="Kod"):
    """(plan output, execute output, warnings of execute, warnings of plan, the drawing after) or ("refused", code,
    message)."""
    report = PC.compare(d, old, new)
    classes, moves = [], []
    for row in report["classes"]:
        if not row.get("from"):
            continue
        c = next(k for k in old["classes"] if k["id"] == row["from"])
        lid = PC.find_layer(d, c["layer"])
        if lid is None:
            continue
        objs = [(i, e) for i, e in enumerate(d["entities"]) if e["layerId"] == lid]
        if not objs:
            continue
        each = []
        for i, e in objs:
            if row["kind"] in ("retired", "added"):
                t = None
            elif row["kind"] == "split":
                code = code_of(e, field)
                t = next((x for x in row["to"] if code and code in next(k for k in new["classes"]
                                                                          if k["id"] == x).get("codes", [])), None)
            else:
                t = row["to"][0]
            if t is not None:
                k = next(k for k in new["classes"] if k["id"] == t)
                if PC.find_layer(d, k["layer"]) == lid:
                    t = None
            each.append((i, t))
        moved = sum(t is not None for _, t in each)
        left = len(each) - moved if row["kind"] in ("retired", "split") else 0
        classes.append({"from": row["from"], "to": row["to"], "kind": row["kind"], "moved": moved, "left": left})
        moves.append(each)
        if moved and locked(d, lid):
            return ("refused", "layer_locked", f"“{c['title']}” katmanı kilitli; nesneleri yeni kataloğa taşınamaz. "
                                               "Kilidi Katmanlar panelinden açın.")
    receiving = [k for k in new["classes"] if any(t == k["id"] for each in moves for _, t in each)]
    for k in receiving:
        if PR.find(d["layers"], k["layer"])[0] == "blocked":
            return ("refused", "layer_refused", f"“{k['title']}” sınıfının yolu ({PR.path_words(k['layer'])}) çizimde "
                                                "bir gruba ya da katmanın içine çıkıyor; nesneler oraya taşınamaz.")
    for k in receiving:
        lid = PC.find_layer(d, k["layer"])
        if lid is not None and locked(d, lid):
            return ("refused", "layer_locked", f"“{k['title']}” sınıfının katmanı kilitli; nesneler ona taşınamaz. "
                                               "Kilidi Katmanlar panelinden açın.")
    if all(c["moved"] == 0 for c in classes):
        return ("refused", "nothing_to_set", "Taşınacak nesne yok: eski kataloğun sınıflarının nesneleri zaten yeni "
                                             "sınıflarının katmanında ya da yeni katalogda yerleri yok.")
    said = []
    for c in classes:
        if c["left"]:
            title = next(k["title"] for k in old["classes"] if k["id"] == c["from"])
            why = "yeni katalogda yok" if c["kind"] == "retired" else "kodu yeni sınıflarda yok"
            said.append(f"{title} ({why}) {c['left']}")
    left_w = [{"code": "catalog_objects_left", "message": f"Yerinde kalan nesneler: {', '.join(said)}."}] if said else []
    after = copy.deepcopy(d)
    layers, fields, warnings = [], [], []
    # The writer numbers the layers it adds from 1 on each call; one command adds them all, numbered on.
    def renamed(nodes, old, new_):
        for n in nodes:
            if n["id"] == old:
                n["id"] = new_
            renamed(n.get("children", []), old, new_)
    for k in receiving:
        for lid in layers:
            renamed(after["layers"], lid, "§" + lid)
        out, w = PR.apply(after, narrowed(new, k["id"]))
        fresh = []
        for j, lid in enumerate(out["layers"]):
            renamed(after["layers"], lid, f"§$layer:{len(layers) + j + 1}")
            fresh.append(f"$layer:{len(layers) + j + 1}")
        for lid in layers + fresh:
            renamed(after["layers"], "§" + lid, lid)
        layers += fresh
        fields += out["fields"]
        warnings += w
    for each in moves:
        for i, t in each:
            if t is None:
                continue
            k = next(k for k in new["classes"] if k["id"] == t)
            after["entities"][i]["layerId"] = PC.find_layer(after, k["layer"])
    total = {"moved": sum(c["moved"] for c in classes), "left": sum(c["left"] for c in classes)}
    plan = {"classes": classes, **total, "revision": "$current"}
    done = {"classes": classes, **total, "revision": "$current"}
    if layers:
        done["layers"] = layers
    if fields:
        done["fields"] = fields
    return plan, done, warnings + left_w, left_w, after


def completed(out, warnings=()):
    return {"status": "completed", "output": out, "warnings": list(warnings)}


def failed(code, message, path=None):
    err = {"code": code, "message": message}
    if path is not None:
        err["path"] = path
    return {"status": "failed", "error": err}


def written(d, inp, old, new, field="Kod", undo=True):
    plan, done, warnings, plan_w, after = carry(d, old, new, field)
    ids = list(range(1, len(d["entities"]) + 1))
    changed = {str(i + 1): {"layerId": e["layerId"]} for i, e in enumerate(after["entities"])
               if e["layerId"] != d["entities"][i]["layerId"]}
    before = {k: {"layerId": d["entities"][int(k) - 1]["layerId"]} for k in changed}
    domains = after["settings"].get("domains")
    steps = [{"op": "validate", "input": inp, "result": completed(None), "expect": SAME},
             {"op": "plan", "input": inp, "result": completed(plan, plan_w), "expect": SAME},
             {"op": "execute", "input": inp, "result": completed(done, warnings),
              "expect": {"ids": ids, "fields": changed, "layers": after["layers"], "settings": {"domains": domains},
                         "revision": "changed", "canUndo": True}}]
    if undo:
        # The value lists the writer adds stay after an undo, as cad.profiles.apply's do (profile_cases.apply_case).
        steps.append({"op": "undo", "returns": STEP, "expect": {"ids": ids, "fields": before, "layers": d["layers"],
                                                                 "settings": {"domains": domains}}})
    return steps, done


def refused_steps(d, inp, old, new, field="Kod"):
    r = carry(d, old, new, field)
    assert r[0] == "refused", r
    return [{"op": "execute", "input": inp, "result": failed(r[1], r[2]), "expect": SAME}]


def case(name, d, steps, note=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["setup"] = d
    c["steps"] = steps
    return c


def cases():
    d = PC.setup()
    to_new = {"from": "harita-varliklari", "toProfile": NEW}
    st, done = written(d, to_new, OLD, NEW)
    kinds = {c["from"]: (c["kind"], c["moved"], c["left"]) for c in done["classes"]}
    assert kinds["kontrol-noktasi"] == ("split", 2, 0) and kinds["agac"] == ("retired", 0, 1), kinds
    assert kinds["direk"] == ("renamed", 2, 0) and kinds["bordur"][1] == 1 and kinds["yol-kenari"][1] == 1, kinds
    out = [case("Harita varlıkları → belediye kataloğu: bölünen kontrol noktaları kodlarıyla, yeniden adlandırılan "
                "direkler, birleşen bordür ve yol kenarı, yeri değişen vanalar taşınır; ağaç yerinde kalır", d, st,
                "Yeni sınıfın katmanı yoksa alanlarıyla açılır; değerler nesneyle gider; tek adım geri alınır.")]
    out.append(case("Kataloğun kendisi verilir (fromProfile): aynı taşıma", d,
                    written(d, {"fromProfile": OLD, "toProfile": NEW}, OLD, NEW, undo=False)[0][1:3]))
    digits = PC.setup()
    digits["entities"][0]["attrs"]["Kod"] = "KN12"
    digits["entities"][1]["attrs"]["Kod"] = "PN7"
    out.append(case("Saha kodu numaralı (KN12, PN7): baştaki harflerle dağıtılır", digits,
                    written(digits, to_new, OLD, NEW, undo=False)[0][2:3]))
    nocode = PC.setup()
    del nocode["entities"][1]["attrs"]["Kod"]
    out.append(case("Bölünen sınıfın kodsuz nesnesi yerinde kalır, söylenir", nocode,
                    written(nocode, to_new, OLD, NEW, undo=False)[0][2:3]))
    other = PC.setup()
    other["entities"][0]["attrs"]["Kod"] = "XY"
    out.append(case("Bölünen sınıfın kodu yeni sınıflarda olmayan nesnesi yerinde kalır", other,
                    written(other, to_new, OLD, NEW, undo=False)[0][2:3]))
    by_name = PC.setup()
    for e in by_name["entities"]:
        if e["layerId"] == "hv-kn":
            e["attrs"]["SahaKodu"] = e["attrs"].pop("Kod")
    out.append(case("Kod alanı başka adla (field SahaKodu)", by_name,
                    written(by_name, dict(to_new, field="SahaKodu"), OLD, NEW, field="SahaKodu", undo=False)[0][2:3]))
    wrong_field = PC.setup()
    out.append(case("Kod alanı verilen adla bulunmazsa bölünen sınıfın nesneleri yerinde kalır", wrong_field,
                    written(wrong_field, dict(to_new, field="Yok"), OLD, NEW, field="Yok", undo=False)[0][2:3]))
    there = PC.setup()
    there["layers"][1]["children"].append(PC.layer("hv-ed", "Elektrik direkleri"))
    out.append(case("Yeni sınıfın katmanı çizimde var: nesneler oraya gider, eksik alanları eklenir", there,
                    written(there, to_new, OLD, NEW)[0][2:4]))
    upper = PC.setup()
    upper["layers"][1]["name"] = "HARİTA VARLIKLARI"
    out.append(case("Katman yolu Türkçe büyük harfle: katlanmış adla bulunur, yeni katmanlar onun altına", upper,
                    written(upper, to_new, OLD, NEW, undo=False)[0][2:3]))
    hidden = PC.setup()
    hidden["layers"][1]["children"][3]["visible"] = False
    out.append(case("Gizli katmanın nesneleri de taşınır", hidden,
                    written(hidden, to_new, OLD, NEW, undo=False)[0][2:3]))
    merged_only = copy.deepcopy(NEW)
    merged_only["classes"] = [c for c in merged_only["classes"] if c["id"] in ("yol-siniri",)]
    out.append(case("Yalnız birleşen sınıfı olan katalog: bordür ve yol kenarı taşınır, öbürleri yerinde kalır", d,
                    written(d, {"from": "harita-varliklari", "toProfile": merged_only}, OLD, merged_only,
                            undo=False)[0][1:3]))
    into_old = copy.deepcopy(NEW)
    for c in into_old["classes"]:
        if c["id"] == "yol-siniri":
            c.update(id="bordur", title="Bordür")
    out.append(case("Yol kenarı eski bordür sınıfına katılır: bordürün yolu değişti, ikisi de yeni katmana", d,
                    written(d, {"from": "harita-varliklari", "toProfile": into_old}, OLD, into_old, undo=False)[0][2:3]))
    one_code = copy.deepcopy(NEW)
    one_code["classes"] = [c for c in one_code["classes"] if c["id"] != "poligon-noktasi"]
    out.append(case("Bir kodu düşen kontrol noktası tek sınıfa gider: ikisi de GNSS noktalarına", d,
                    written(d, {"from": "harita-varliklari", "toProfile": one_code}, OLD, one_code, undo=False)[0][2:3]))
    # The writer's warnings: a field of another kind kept, the project's own list kept.
    differs = PC.setup()
    differs["layers"][1]["children"].append(dict(PC.layer("hv-ed", "Elektrik direkleri"),
                                                 fields=[{"name": "Tur", "kind": "integer"}]))
    out.append(case("Yeni sınıfın katmanındaki alan başka türde: katmanınki korunur, söylenir", differs,
                    written(differs, to_new, OLD, NEW, undo=False)[0][2:3]))
    own_list = PC.setup()
    own_list["settings"]["domains"] = [{"id": "kontrol-tur", "name": "Kontrol noktasının türü (kurum)", "kind": "text",
                                        "values": [{"code": "gnss", "label": "GNSS"}]}]
    out.append(case("Projenin aynı kimlikli değer listesi farklı: projeninki korunur, söylenir", own_list,
                    written(own_list, to_new, OLD, NEW, undo=False)[0][2:3]))
    # Refusals.
    blocked = PC.setup()
    blocked["layers"][1]["children"].append(PC.layer("hv-gr", "GNSS noktaları", "group"))
    out.append(case("Yeni sınıfın yolu bir gruba çıkıyor: hiçbir şey taşınmaz", blocked,
                    refused_steps(blocked, to_new, OLD, NEW)))
    lock_from = PC.setup()
    lock_from["layers"][1]["children"][3]["locked"] = True
    out.append(case("Nesnelerin bıraktığı katman kilitli: hiçbir şey taşınmaz", lock_from,
                    refused_steps(lock_from, to_new, OLD, NEW)))
    lock_to = PC.setup()
    lock_to["layers"][1]["children"].append(dict(PC.layer("hv-ed", "Elektrik direkleri"), locked=True))
    out.append(case("Nesnelerin gideceği katman kilitli: hiçbir şey taşınmaz", lock_to,
                    refused_steps(lock_to, to_new, OLD, NEW)))
    out.append(case("Aynı katalog: taşınacak nesne yok", d,
                    refused_steps(d, {"from": "harita-varliklari", "to": "harita-varliklari"}, OLD, OLD)))
    out.append(case("Ters yön: yeni kataloğun sınıflarının nesneleri zaten yerinde, taşınacak yok", d,
                    refused_steps(d, {"fromProfile": NEW, "to": "harita-varliklari"}, NEW, OLD)))
    bare = PC.setup(objects=PC.OBJECTS[15:], notes=False, layers=False)
    out.append(case("Sınıf katmanları olmayan çizim: taşınacak nesne yok", bare,
                    refused_steps(bare, to_new, OLD, NEW)))
    retire_all = copy.deepcopy(NEW)
    retire_all["classes"] = [retire_all["classes"][-1]]
    out.append(case("Bütün sınıflar kalkıyor: taşınacak nesne yok", d,
                    refused_steps(d, {"from": "harita-varliklari", "toProfile": retire_all}, OLD, retire_all)))
    out.append(case("Beklenen sürüm geçersiz", d, [{"op": "execute", "input": dict(to_new, expectedRevision="dün"),
                                                    "result": failed("invalid_revision", "Beklenen sürüm “dün” geçerli bir "
                                                                     "sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                                                                     "Sürümü belgeden ya da komutun planından alın.",
                                                                     "expectedRevision"), "expect": SAME}]))
    out.append(case("Çizim değişmiş: çakışma", d, [{"op": "execute", "input": dict(to_new, expectedRevision="999"),
                                                   "result": {"status": "conflict", "error": {
                                                       "code": "revision_conflict",
                                                       "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir "
                                                                  "şey yazılmadı. Komutu çizimin şimdiki hâline göre "
                                                                  "yeniden hazırlayın.",
                                                       "path": "expectedRevision", "revision": "$current"}},
                                                   "expect": SAME}]))
    out.append(case("Eski katalog verilmedi: unknown_profile", d, [
        {"op": "execute", "input": {"to": "harita-varliklari"},
         "result": failed("unknown_profile", "Eski kataloğu adıyla (from) ya da kendisiyle (fromProfile) verin.", "from"),
         "expect": SAME}]))
    out.append(case("Yeni katalog geçersiz: invalid_profile", d, [
        {"op": "execute", "input": {"from": "harita-varliklari", "toProfile": PC.bad_format(NEW)},
         "result": failed("invalid_profile", "Yeni katalog geçersiz: Profilin biçimi “kentos.data-profile”, sürümü 1 "
                          "olmalı.", "toProfile"), "expect": SAME}]))
    return out


def doc():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.profiles.carry",
        "commandVersion": 1,
        "title": "Kataloğa taşı: sınıfların nesneleri yeni kataloğun katmanlarına",
        "note": "ADR 5340 (MAP-04). Bağımsız başvuru scripts/fixtures/profile_carry_cases.py: kurallar ADR'den; KentOS "
                "kodu yok. Raporun sınıf izlemesi profile_compare_cases.py'den, katman yazıcısı profile_cases.py'den.",
        "setup": PC.setup(),
        "cases": cases(),
    }


def main():
    text = json.dumps(doc(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} eski; betiği --check olmadan çalıştırın.")
            sys.exit(1)
        print(f"Kataloğa taşı ortak durumları güncel ({len(doc()['cases'])}).")
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc()['cases'])} durum)")


if __name__ == "__main__":
    main()
