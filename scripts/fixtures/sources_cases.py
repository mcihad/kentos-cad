"""The shared cases of value lists' packages and of a layer's values turned into their canonical texts (docs/adr/0439
§7, §8), their expectations built here without KentOS code:

- fixtures/commands/v1/cad.domains.export.json: `cad.domains.export` gives the lists by their ids in the order given
  (each once), every list in the project's order without ids; an id the project has not is refused;
- fixtures/commands/v1/cad.domains.edit.import.json: `cad.domains.edit`'s import (the rule is
  scripts/fixtures/domains_cases.py's): the package's lists added, one of an id the project has kept when equal and
  refused unless `replace`, which replaces it as an update does;
- fixtures/commands/v1/cad.fields.migrate.json: `cad.fields.migrate` writes each chosen field's value in its canonical
  text (scripts/fixtures/layer_field_cases.py's single forms), keeps a value that does not fit as written and counts
  it, leaves empty values and attributes no field names as they are.

    python3 scripts/fixtures/sources_cases.py           # writes the files
    python3 scripts/fixtures/sources_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import domains_cases as dc  # noqa: E402  (the value lists' rule, written without KentOS code)
import forms_cases as fc  # noqa: E402  (the revision words)
import layer_field_cases as lf  # noqa: E402  (a field's single forms)

OUT = ROOT / "fixtures/commands/v1"
UNCHANGED = dc.UNCHANGED


def header(command, title, note):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": f"{note} Written by scripts/fixtures/sources_cases.py without KentOS code; do not edit by hand."}


# ── cad.domains.export ─────────────────────────────────────────────────

MORE = [
    {"id": "egim", "name": "Eğim", "kind": "decimal", "min": "0", "max": "90", "maxExclusive": True, "unit": "°"},
    {"id": "durum", "name": "Durum", "description": "Yapının kullanım durumu.", "kind": "text",
     "values": [dc.ch("k", "Kullanılıyor"), {"code": "b", "label": "Boş", "description": "İçinde kimse yok."},
                dc.ch("y", "Yıkık", True), {"code": "r", "label": "Restorasyon", "from": "2026-01-01"}]},
]


def export_setup(domains=None):
    s = dc.setup()
    s["settings"]["domains"] = copy.deepcopy(dc.DOMAINS + MORE if domains is None else domains)
    if not s["settings"]["domains"]:
        del s["settings"]["domains"]
    return s


def exported(domains, ids):
    """The package of `ids` (none: every list), or the refusal of the first unknown id."""
    if not ids:
        chosen = domains
    else:
        chosen = []
        for i, d in enumerate(ids):
            found = next((x for x in domains if x["id"] == d), None)
            if found is None:
                return fc.failed("unknown_domain", f"“{d}” kimlikli bir değer listesi yok. Değer listeleri penceresinden "
                                 "var olan bir listeyi seçin.", f"ids/{i}")
            if found not in chosen:
                chosen.append(found)
    return {"status": "completed", "output": {"format": "kentos.domains", "version": 1,
                                              "domains": copy.deepcopy(chosen)}, "warnings": []}


def export_cases():
    all_lists = dc.DOMAINS + MORE
    out = []

    def case(name, ids=None, op="execute", domains=None):
        lists = all_lists if domains is None else domains
        inp = {"ids": ids} if ids else {}
        result = exported(lists, ids or [])
        if op == "validate" and result["status"] == "completed":
            result = {"status": "completed", "output": None, "warnings": []}
        c = {"name": name, "steps": [{"op": op, "input": inp, "result": result, "expect": UNCHANGED}]}
        if domains is not None:
            c["setup"] = export_setup(domains)
        out.append(c)

    case("bütün listeler projenin sırasıyla")
    case("tek liste", ["kat"])
    case("verilen sırayla", ["durum", "malzeme"])
    case("aynı kimlik bir kez", ["kat", "kat", "malzeme", "kat"])
    case("hariç uçlu ve birimli aralık", ["egim"])
    case("açıklamalı, kullanımdan kalkmış ve günlü değerleriyle", ["durum"])
    case("ondalık kodlu liste", ["genislik"])
    case("olmayan liste", ["yok"])
    case("ikinci kimlik olmayan liste", ["kat", "yok"])
    case("üçüncü kimlik olmayan liste, öncekiler geçerli", ["kat", "malzeme", "eksik"])
    case("büyük küçük harf ayrıdır", ["Kat"])
    case("plan aynı paketi verir", ["malzeme"], op="plan")
    case("planda olmayan liste", ["yok"], op="plan")
    case("doğrulama", ["egim", "durum"], op="validate")
    case("doğrulamada olmayan liste", ["egim", "yok"], op="validate")
    case("bütün listelerin planı", op="plan")
    case("listesiz proje boş paket verir", domains=[])
    case("listesiz projede kimlik verilirse ret", ["kat"], domains=[])
    case("tek listeli proje", domains=[dc.DOMAINS[1]])
    case("bütün kimlikler ters sırayla", [d["id"] for d in reversed(all_lists)])
    case("başta boşluklu kimlik başka kimliktir", [" kat"])
    return out


# ── cad.domains.edit's import ──────────────────────────────────────────

def import_cases():
    out = []
    one = dc.one
    malzeme = copy.deepcopy(dc.DOMAINS[0])
    kat = copy.deepcopy(dc.DOMAINS[1])
    yeni = {"id": "cati", "name": "Çatı türü", "kind": "text", "values": [dc.ch("dz", "Düz"), dc.ch("bh", "Beşik")]}
    yas = {"id": "yas", "name": "Bina yaşı", "kind": "integer", "min": "0", "max": "300"}
    out.append(one("yeni listeler eklenir", {"kind": "import", "domains": [yeni, yas]}))
    out.append(one("projedekiyle aynı liste olduğu gibi kalır: değişiklik yok", {"kind": "import", "domains": [malzeme]}))
    out.append(one("aynı ve yeni liste: yalnız yeni eklenir", {"kind": "import", "domains": [malzeme, yeni]}))
    changed = {**copy.deepcopy(malzeme), "values": [dc.ch("ah", "Ahşap yapı"), dc.ch("bt", "Betonarme"),
                                                    dc.ch("ym", "Yığma"), dc.ch("ce", "Çelik")]}
    out.append(one("aynı kimlikli farklı liste reddedilir", {"kind": "import", "domains": [changed]}))
    out.append(one("replace: liste değişir, bağlı alanlar izler (kilitli katmanınki de)",
                   {"kind": "import", "domains": [changed], "replace": True}))
    dropped = {**copy.deepcopy(malzeme), "values": [dc.ch("ah", "Ahşap"), dc.ch("bt", "Betonarme")]}
    out.append(one("replace: kayıtların tuttuğu kod listeden çıkamaz",
                   {"kind": "import", "domains": [dropped], "replace": True}))
    retired = {**copy.deepcopy(malzeme), "values": [dc.ch("ah", "Ahşap"), dc.ch("bt", "Betonarme"),
                                                    dc.ch("ym", "Yığma", True)]}
    out.append(one("replace: kullanımdan kalkan kod kalır", {"kind": "import", "domains": [retired], "replace": True}))
    out.append(one("replace: türü değişemez",
                   {"kind": "import", "domains": [{"id": "kat", "name": "Kat sayısı", "kind": "decimal", "min": "1"}],
                    "replace": True}))
    out.append(one("replace: aralık dışında kalan kayıt uyarıyla",
                   {"kind": "import", "domains": [{**kat, "max": "30"}], "replace": True}))
    out.append(one("boş paket", {"kind": "import", "domains": []}))
    out.append(one("pakette aynı kimlik iki kez", {"kind": "import", "domains": [yeni, {**yeni, "name": "Başka çatı"}]}))
    out.append(one("geçersiz liste: ne kod ne aralık",
                   {"kind": "import", "domains": [{"id": "bos", "name": "Boş", "kind": "text"}]}))
    out.append(one("adı projedeki başka listeninkiyle aynı",
                   {"kind": "import", "domains": [{"id": "kat2", "name": "kat sayısı", "kind": "integer", "min": "0"}]}))
    out.append(one("replace: ondalık kod bağlı alanın basamağına sığmaz",
                   {"kind": "import", "domains": [{**copy.deepcopy(dc.DOMAINS[2]),
                                                    "values": [dc.ch("2.5", "Dar"), dc.ch("3.75", "Çok geniş"),
                                                               dc.ch("3.5", "Geniş")]}], "replace": True}))
    out.append(one("replace olmadan aynı liste yine değişiklik değildir",
                   {"kind": "import", "domains": [malzeme, kat], "replace": True}))
    out.append(one("plan aynı cevabı verir, hiçbir şey yazmaz", {"kind": "import", "domains": [yeni]}, op="plan"))
    out.append(one("doğrulama", {"kind": "import", "domains": [yeni, yas]}, op="validate"))
    out.append(one("doğrulamada aynı kimlikli farklı liste", {"kind": "import", "domains": [changed]}, op="validate"))
    out.append(one("hariç uçlu, birimli aralık ve günlü değerli liste", {"kind": "import", "domains": copy.deepcopy(MORE)}))
    out.append(one("ilk liste geçerli, ikincisi reddedilince hiçbiri eklenmez",
                   {"kind": "import", "domains": [yeni, {**copy.deepcopy(malzeme), "name": "Malzeme 2"}]}))
    # Undo takes the lists back.
    s0 = dc.first_state()
    a, _ = dc.step(s0, {"kind": "import", "domains": [changed], "replace": True})
    undo = {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "settings": {"domains": s0["domains"]},
                                     "layers": s0["layers"]}}
    out.append({"name": "geri alma listeleri ve kopyaları geri verir", "steps": [a, undo]})
    stale = dc.one("eski sürüme göre hazırlanmış paket", {"kind": "import", "domains": [yeni]}, expectedRevision="07")
    stale["steps"][0].update(result=fc.failed("invalid_revision", fc.REVISION, "expectedRevision"), expect=UNCHANGED)
    out.append(stale)
    return out


# ── cad.fields.migrate ─────────────────────────────────────────────────

M_FIELDS = [
    {"name": "Ada", "kind": "integer"},
    {"id": "alan", "name": "Alan", "kind": "decimal", "scale": 2},
    {"name": "Tarih", "kind": "date"},
    {"name": "Ruhsat", "kind": "boolean"},
    {"name": "Tür", "kind": "text", "values": [dc.ch("k", "Konut"), dc.ch("t", "Ticari")]},
    {"name": "Not", "kind": "text"},
    {"name": "Kat", "kind": "integer", "min": "1", "max": "40"},
    {"name": "Tespit", "kind": "datetime"},
    {"name": "Saat", "kind": "time"},
    {"name": "Etiket", "kind": "text", "multiple": True, "values": [dc.ch("a", "A"), dc.ch("b", "B")]},
]
M_ENTITIES = [
    (1, "parsel", {"Ada": "101", "Alan": "450,5", "Tarih": "17.10.2026", "Ruhsat": "evet", "Tür": "Konut",
                   "Not": "köşe parsel", "ESKI_KOD": "A1", "Tespit": "17.10.2026 14:30", "Saat": "8:05",
                   "Etiket": "A; B"}),
    (2, "parsel", {"Ada": " 102 ", "Alan": "1.234,5", "Tarih": "2026-13-40", "Ruhsat": "", "Tür": "t", "Kat": "45",
                   "Saat": "08:05:00"}),
    (3, "parsel", {"Alan": "abc", "MAHALLE": "Kızılay", "Kat": "3"}),
    (4, "parsel", {"Ada": "103", "Alan": "12.50", "Tarih": "2026-10-17", "Ruhsat": "true", "Tür": "k",
                   "Etiket": "b"}),
    (5, "eski", {"Ada": "7,0", "Ruhsat": "hayır"}),
    (6, "temiz", {"Ada": "5", "Ruhsat": "false"}),
]


def m_setup():
    return {"format": "kentos.document", "version": 1, "name": "Değerlerin göçü", "settings": dict(dc.SETTINGS),
            "origin": {"x": 487000, "y": 4420000},
            "layers": [dc.layer("parsel", "Parsel", copy.deepcopy(M_FIELDS)),
                       dc.layer("grup", "Eski", kind="group",
                                children=[dc.layer("eski", "Eski parsel", copy.deepcopy(M_FIELDS[:4]), locked=True)]),
                       dc.layer("temiz", "Temiz", copy.deepcopy(M_FIELDS[:4]))],
            "activeLayer": "parsel",
            "entities": [{"kind": "polygon", "id": i, "layerId": lay, "pts": copy.deepcopy(dc.SQUARE), "attrs": dict(a)}
                         for i, lay, a in M_ENTITIES],
            "styles": {"items": [], "categories": []}}


def typed(f):
    return f["kind"] != "text" or bool(f.get("values"))


def migrated(state, inp):
    """(output, warnings, writes {id: attrs}) of `inp`, or a refusal."""
    layers = {n["id"]: n for n in [state["layers"][0], state["layers"][1], state["layers"][1]["children"][0],
                                   state["layers"][2]]}
    lay = inp["layer"]
    n = layers.get(lay)
    if n is None:
        return fc.failed("layer_not_found", f"“{lay}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                         "layer")
    if n["type"] != "layer":
        return fc.failed("not_a_layer", f"“{n['name']}” bir katman grubu; alanlar yalnız katmanın olur. Grubun bir katmanını "
                         "verin.", "layer")
    fields = n["fields"]
    if inp.get("fields"):
        chosen = []
        for i, k in enumerate(inp["fields"]):
            f = next((f for f in fields if dc.key(f) == k), None)
            if f is None:
                return fc.failed("field_not_found", f"“{n['name']}” katmanında “{k}” alanı yok.", f"fields/{i}")
            chosen.append(dc.key(f))
        chosen = [f for f in fields if dc.key(f) in chosen]
    else:
        chosen = [f for f in fields if typed(f)]
    names = {f["name"] for f in fields}
    rows = [{"field": dc.key(f), "changed": 0, "kept": 0, "unfit": 0, "blank": 0, "absent": 0, "samples": []}
            for f in chosen]
    unknown, writes, unfit = set(), {}, []
    for i, l2, a in state["entities"]:
        if l2 != lay:
            continue
        unknown |= {k for k in a if k not in names}
        new = None
        for f, m in zip(chosen, rows):
            v = a.get(f["name"])
            if v is None:
                m["absent"] += 1
                continue
            if not lf.trim(v):
                m["blank"] += 1
                continue
            r = lf.check({**f, "required": False}, v)
            if "error" in r:
                m["unfit"] += 1
                if len(m["samples"]) < 5:
                    m["samples"].append(v)
                if len(unfit) < 5:
                    unfit.append(v)
            elif r["value"] == v:
                m["kept"] += 1
            else:
                m["changed"] += 1
                new = new or dict(a)
                new[f["name"]] = r["value"]
        if new is not None:
            writes[i] = new
    if writes and n.get("locked"):
        return fc.failed("layer_locked", f"“{n['name']}” katmanı kilitli; değerleri biçime çevrilemez. Kilidi Katmanlar "
                         "panelinden açın.", "layer")
    for m in rows:
        if not m["samples"]:
            del m["samples"]
    total = sum(m["unfit"] for m in rows)
    warnings = [{"code": "values_unfit", "message": f"{total} değer alanına uymuyor; yazıldığı gibi bırakıldı "
                 f"({', '.join(f'“{v}”' for v in unfit)}). Formla düzenle ya da Öznitelik tablosuyla düzeltin.",
                 "path": "fields"}] if total else []
    out = {"layer": lay, "fields": rows, "objects": len(writes), **({"unknown": sorted(unknown)} if unknown else {}),
           "changed": bool(writes), "revision": "$current"}
    return out, warnings, writes


def m_state():
    return {"layers": m_setup()["layers"], "entities": [(i, lay, dict(a)) for i, lay, a in M_ENTITIES]}


def m_step(state, inp, op="execute"):
    r = migrated(state, inp)
    if isinstance(r, dict):
        return {"op": op, "input": inp, "result": r, "expect": UNCHANGED}, state
    out, warnings, writes = r
    if op == "validate":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": None, "warnings": warnings},
                "expect": UNCHANGED}, state
    result = {"status": "completed", "output": out, "warnings": warnings}
    if op == "plan" or not writes:
        return {"op": op, "input": inp, "result": result, "expect": UNCHANGED}, state
    after = copy.deepcopy(state)
    after["entities"] = [(i, lay, writes.get(i, a)) for i, lay, a in after["entities"]]
    expect = {"canUndo": True, "dirty": True, "revision": "changed",
              "fields": {str(i): {"attrs": a} for i, a in sorted(writes.items())}}
    return {"op": op, "input": inp, "result": result, "expect": expect}, after


def migrate_cases():
    out = []

    def case(name, inp, op="execute"):
        s, _ = m_step(m_state(), inp, op)
        out.append({"name": name, "steps": [s]})

    case("bütün tipli alanlar: virgüllü ondalık, tarih, evet, etiket, tarih ve saat, saat, çok değerli",
         {"layer": "parsel"})
    case("tek alan", {"layer": "parsel", "fields": ["alan"]})
    case("alanlar katmanın sırasıyla, her biri bir kez", {"layer": "parsel", "fields": ["Saat", "Ada", "Saat"]})
    case("listesiz metin alanı istenirse değerleri olduğu gibi sayılır", {"layer": "parsel", "fields": ["Not"]})
    case("listedeki etiket kodu olur", {"layer": "parsel", "fields": ["Tür"]})
    case("aralık dışındaki değer uymaz, yazıldığı gibi kalır", {"layer": "parsel", "fields": ["Kat"]})
    case("boşluklu tam sayı biçimine çevrilir", {"layer": "parsel", "fields": ["Ada"]})
    case("geçersiz tarih uymaz", {"layer": "parsel", "fields": ["Tarih"]})
    case("boş değer boş kalır", {"layer": "parsel", "fields": ["Ruhsat"]})
    case("tarih ve saat", {"layer": "parsel", "fields": ["Tespit"]})
    case("çok değerli alanın etiketleri kodları olur", {"layer": "parsel", "fields": ["Etiket"]})
    case("olmayan alan", {"layer": "parsel", "fields": ["Ada", "Renk"]})
    case("olmayan katman", {"layer": "yok"})
    case("grup katman değil", {"layer": "grup"})
    case("kilitli katmanın değeri çevrilemez", {"layer": "eski"})
    case("kilitli katmanda değişecek bir şey yoksa ret yok", {"layer": "eski", "fields": ["Tarih"]})
    case("biçimindeki değerler: değişiklik yok, geri alma adımı yok", {"layer": "temiz"})
    case("plan aynı cevabı verir, hiçbir şey yazmaz", {"layer": "parsel"}, op="plan")
    case("doğrulama uyarıyı söyler", {"layer": "parsel"}, op="validate")
    case("planda olmayan alan", {"layer": "parsel", "fields": ["Renk"]}, op="plan")
    s0 = m_state()
    a, _ = m_step(s0, {"layer": "parsel", "fields": ["alan"]})
    undo = {"op": "undo", "expect": {"canUndo": False, "canRedo": True,
                                     "fields": {"1": {"attrs": dict(M_ENTITIES[0][2])}}}}
    out.append({"name": "geri alma değerleri yazıldıkları gibi geri verir", "steps": [a, undo]})
    stale = {"name": "geçersiz sürüm", "steps": [{"op": "execute", "input": {"layer": "parsel", "expectedRevision": "07"},
                                                  "result": fc.failed("invalid_revision", fc.REVISION, "expectedRevision"),
                                                  "expect": UNCHANGED}]}
    out.append(stale)
    conflict = {"name": "çizim değiştikten sonra hazırlanmış göç", "steps": [
        {"op": "execute", "input": {"layer": "parsel", "expectedRevision": "999"},
         "result": fc.failed("revision_conflict", fc.CONFLICT, "expectedRevision", "conflict"), "expect": UNCHANGED}]}
    out.append(conflict)
    return out


def files():
    return {
        "cad.domains.export.json": {**header("cad.domains.export", "Değer listelerini dışa aktar",
                                             "ADR 0439 §7: projenin değer listelerinin paketi."),
                                    "setup": export_setup(), "cases": export_cases()},
        "cad.domains.edit.import.json": {**header("cad.domains.edit", "Değer listeleri",
                                                  "ADR 0439 §7: değer listesi paketinin içe aktarılması."),
                                         "setup": dc.setup(), "cases": import_cases()},
        "cad.fields.migrate.json": {**header("cad.fields.migrate", "Değerleri biçime çevir",
                                             "ADR 0439 §8: katmanın değerlerinin alanların biçimine çevrilmesi."),
                                    "setup": m_setup(), "cases": migrate_cases()},
    }


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = 0
    for name, data in files().items():
        path = OUT / name
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        n = len(data["cases"])
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)",
                      file=sys.stderr)
                bad += 1
            else:
                print(f"{path.relative_to(ROOT)} güncel ({n} durum)")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.relative_to(ROOT)} yazıldı ({n} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
