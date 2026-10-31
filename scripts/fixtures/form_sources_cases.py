"""Writes fixtures/commands/v1/cad.forms.sources.json: the shared cases of `cad.forms.sources` v1 (Formun
kaynakları, docs/adr/0763), their expectations built here without KentOS code from the ADR's rule: the layers a
form lists from (its fields' lists from other layers and its related items, tabs and groups walked in order, a layer
named twice once with both items, a layer the project has not left out), each one's state: the drawing's own layer
`local`; a GeoPackage's file not there or its table not in it `missing`, a file that is not a GeoPackage `failed`,
one whose SHA-256 (hashlib) is not the link's revision `changed`, else `ready`; each with its code, its words and its
way out. A database's states need a server and are tested in kentos-postgis (a closed port, a silent server) and in
scripts/fixtures/postgis_cases.py; a shared case names no database (a host with a database provider would answer
otherwise than one without).

    python3 scripts/fixtures/form_sources_cases.py           # writes the file
    python3 scripts/fixtures/form_sources_cases.py --check   # compares it with the one on disk
"""
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (its digest rule, written without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.forms.sources.json"
GPKG = ROOT / "fixtures/geopackage/v1"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


READY_SHA = sha(GPKG / "iliskili.gpkg")
DIGEST = "f" * 64

# The layers: the form's own, a local list, and bound ones whose sources stand each a way.
SOURCES = {
    # layer id: (name, file, table, revision)
    "malik": ("Malikler", "iliskili.gpkg", "malik", READY_SHA),
    "parsel": ("Parseller", "iliskili.gpkg", "parsel", "0" * 64),
    "yapi": ("Yapılar", "yok-boyle.gpkg", "bina", "1" * 64),
    "eski": ("Eski kayıtlar", "iliskili.gpkg", "kaldirildi", READY_SHA),
    "bozuk": ("Bozuk kaynak", "bozuk.gpkg", "bina", "2" * 64),
}
LOCAL = {"ilce": "İlçeler", "sayac": "Sayaçlar", "hane": "Haneler"}


def layer(lid, name, fields):
    return {"id": lid, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": [],
            "fields": [{"name": f, "kind": "text"} for f in fields]}


def lookup(item, field, lid):
    return {"type": "field", "id": item, "field": field, "lookup": {"layer": lid, "key": "kod"}}


def related(item, lid):
    return {"type": "related", "id": item, "layer": lid, "field": "bina"}


ALL = {"revision": 1, "items": [
    lookup("ilce", "Ilce", "ilce"),
    lookup("malik", "Malik", "malik"),
    lookup("parsel", "Parsel", "parsel"),
    lookup("yapi", "Yapi", "yapi"),
    lookup("eski", "Eski", "eski"),
    lookup("bozuk", "Bozuk", "bozuk"),
    related("sayaclar", "sayac"),
]}
NESTED = {"revision": 1, "items": [
    {"type": "tab", "id": "genel", "label": "Genel", "items": [
        {"type": "group", "id": "kim", "label": "Kim", "items": [lookup("malik-adi", "Malik", "malik")]},
        lookup("hayalet", "Eski", "hayalet"),
    ]},
    {"type": "tab", "id": "baglar", "label": "Bağlar", "items": [
        related("malikler", "malik"), lookup("ilce-2", "Ilce", "ilce"), lookup("malik-2", "Malik", "malik")]},
]}
PLAIN = {"revision": 1, "items": [{"type": "field", "id": "ad", "field": "Ad"}]}
RELEASED = {"revision": 1, "items": [lookup("ilce", "Ilce", "ilce")]}
DRAFTED = {"revision": 2, "items": [lookup("ilce", "Ilce", "ilce"), lookup("malik", "Malik", "malik"),
                                    related("haneler", "hane")]}


def release(body, v):
    return {"version": v, "digest": fc.digest(body), "body": body}


FORMS = [
    {"id": "hepsi", "name": "Bütün kaynaklar", "layer": "bina", "draft": ALL, "releases": [release(ALL, 1)]},
    {"id": "ic-ice", "name": "İç içe", "layer": "bina", "draft": NESTED, "releases": [release(NESTED, 1)]},
    {"id": "duz", "name": "Listesiz", "layer": "bina", "draft": PLAIN, "releases": [release(PLAIN, 1)]},
    {"id": "taslak", "name": "Taslağı farklı", "layer": "bina", "draft": DRAFTED, "releases": [release(RELEASED, 1)]},
    {"id": "yayinsiz", "name": "Yayınsız", "layer": "bina", "draft": DRAFTED},
    {"id": "hazir", "name": "Yalnız hazır", "layer": "bina", "draft": RELEASED,
     "releases": [release({"revision": 1, "items": [lookup("malik", "Malik", "malik")]}, 1)]},
]


def setup():
    layers = [layer("bina", "Binalar", ["Ad", "Ilce", "Malik", "Parsel", "Yapi", "Eski", "Bozuk"])]
    for lid, name in LOCAL.items():
        layers.append(layer(lid, name, ["kod", "bina"]))
    for lid, (name, *_rest) in SOURCES.items():
        layers.append(layer(lid, name, ["kod", "bina"]))
    sources = [{"id": f"kaynak-{i + 1}", "kind": "geopackage", "path": f"$fixture:geopackage/v1/{f}", "table": t,
                "layer": lid, "revision": rev, "digest": DIGEST, "srid": 5256, "writable": True}
               for i, (lid, (_n, f, t, rev)) in enumerate(SOURCES.items())]
    return {"format": "kentos.document", "version": 1, "name": "Formun kaynakları",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis", "forms": FORMS, "sources": sources},
            "origin": {"x": 0.0, "y": 0.0}, "layers": layers, "activeLayer": "bina", "entities": [], "styles": {"items": [], "categories": []}}


# ---- The rule, written from the ADR ----

def listed(items, out):
    for it in items:
        if it["type"] == "field" and "lookup" in it:
            out.append((it["lookup"]["layer"], it["id"]))
        elif it["type"] == "related":
            out.append((it["layer"], it["id"]))
        elif it["type"] in ("tab", "group"):
            listed(it["items"], out)
    return out


def name_of(lid):
    return LOCAL.get(lid) or (SOURCES[lid][0] if lid in SOURCES else None)


def state(lid, items):
    name = name_of(lid)
    out = {"layer": lid, "name": name, "items": items}
    if lid not in SOURCES:
        out["state"] = "local"
        return out
    _n, file, table, rev = SOURCES[lid]
    out["kind"] = "geopackage"
    out["table"] = table
    path = GPKG / file

    def said(st, code, words, retry):
        out.update(state=st, code=code, message=f"{words} Liste son okunan kopyadan sunuluyor.", retry=retry)
        return out

    if not path.is_file():
        return said("missing", "form_source_missing",
                    f"“{name}” katmanının dosyası ({file}) yerinde yok; dosyayı yeniden bağlayın.", "relink")
    head = path.read_bytes()[:16]
    if head != b"SQLite format 3\x00":
        return said("failed", "form_source_failed",
                    f"“{name}” katmanının dosyası ({file}) GeoPackage olarak okunamadı.", "retry")
    import sqlite3
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    tables = {r[0] for r in con.execute("SELECT table_name FROM gpkg_contents")}
    con.close()
    if table not in tables:
        return said("missing", "form_source_missing",
                    f"“{name}” katmanının tablosu (“{table}”) {file} dosyasında yok; katmanı yeniden bağlayın.", "relink")
    if sha(path) != rev:
        return said("changed", "form_source_changed",
                    f"“{name}” katmanının dosyası ({file}) okunduktan sonra değişti; kaynaktan yeniden okuyun.", "read")
    out["state"] = "ready"
    return out


def answer(form_id, draft=False):
    form = next(f for f in FORMS if f["id"] == form_id)
    body = form["draft"] if draft else form["releases"][-1]["body"]
    order = []
    for lid, item in listed(body["items"], []):
        if name_of(lid) is None:
            continue
        hit = next((o for o in order if o[0] == lid), None)
        if hit is None:
            order.append((lid, [item]))
        elif item not in hit[1]:
            hit[1].append(item)
    return {"sources": [state(lid, items) for lid, items in order]}


QUIET = {"ids": [], "canUndo": False, "dirty": False, "revision": "same"}


def done(name, inp, op="execute", **kw):
    out = answer(inp["form"], inp.get("draft", False))
    out["revision"] = "$current"
    return {"name": name, "steps": [{"op": op, "input": inp,
                                     "result": {"status": "completed", "output": None if op == "validate" else out,
                                                "warnings": []}, "expect": QUIET}]}


def refused(name, inp, code, path, message, op="execute"):
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": QUIET}]}


UNPUB = "“Yayınsız” formu hiç yayınlanmadı. Önce yayınlayın ya da taslağıyla çalışın."


def cases():
    return [
        done("her durum ayrı: yerel, hazır, değişmiş, dosyası yok, tablosu yok, okunamayan", {"form": "hepsi"}),
        done("listesi olmayan form kaynak söylemez", {"form": "duz"}),
        done("yalnız hazır kaynak: kodu ve yolu yok", {"form": "hazir"}),
        done("sekme ve gruptaki liste sırasıyla; aynı katman bir kez, öğeleri birlikte", {"form": "ic-ice"}),
        done("projede olmayan katman ön denetime kalır", {"form": "ic-ice", "draft": True}),
        done("son yayın: taslağın eklediği liste yok", {"form": "taslak"}),
        done("taslak: eklenen liste ve ilişkili kayıtlar", {"form": "taslak", "draft": True}),
        done("yayınsız formun taslağı", {"form": "yayinsiz", "draft": True}),
        done("bekleme en az 100 ms", {"form": "hepsi", "waitMs": 100}),
        done("bekleme en çok 30 000 ms", {"form": "hepsi", "waitMs": 30000}),
        done("beklemeyi dosya kaynakları kullanmaz", {"form": "hazir", "waitMs": 250}),
        done("plan aynı cevabı verir, hiçbir şey yazmaz", {"form": "hepsi"}, op="plan"),
        done("plan taslakla", {"form": "taslak", "draft": True}, op="plan"),
        done("doğrulama kaynakları sormaz", {"form": "hepsi"}, op="validate"),
        done("doğrulama taslakla", {"form": "yayinsiz", "draft": True}, op="validate"),
        refused("bekleme 100 ms'den az", {"form": "hepsi", "waitMs": 99}, "invalid_input", "waitMs",
                "waitMs 99: bekleme 100 ile 30000 milisaniye arasında verilir."),
        refused("bekleme 30 000 ms'den çok", {"form": "hepsi", "waitMs": 30001}, "invalid_input", "waitMs",
                "waitMs 30001: bekleme 100 ile 30000 milisaniye arasında verilir."),
        refused("bekleme formdan önce denetlenir", {"form": "yok", "waitMs": 0}, "invalid_input", "waitMs",
                "waitMs 0: bekleme 100 ile 30000 milisaniye arasında verilir."),
        refused("olmayan form", {"form": "yok"}, "unknown_form", "form",
                "“yok” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin."),
        refused("yayınsız formun son yayını yok", {"form": "yayinsiz"}, "form_unpublished", "form", UNPUB),
        refused("doğrulama da yayını ister", {"form": "yayinsiz"}, "form_unpublished", "form", UNPUB,
                op="validate"),
        refused("plan da formu ister", {"form": "yok"}, "unknown_form", "form",
                "“yok” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.", op="plan"),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.sources", "commandVersion": 1,
            "title": "Formun kaynakları: listelerin kaynağının durumu, ayrı kodu ve çıkış yolu",
            "note": "docs/adr/0763. Beklenenler scripts/fixtures/form_sources_cases.py'den, KentOS kodu olmadan: "
                    "GeoPackage dosyaları fixtures/geopackage/v1'den, SHA-256 hashlib'le, tablolar sqlite3'le. "
                    "Veritabanının durumları kentos-postgis'in testlerinde ve postgis_cases.py'de.",
            "setup": setup(), "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print(f"{OUT} eski; betiği --check olmadan çalıştırın.")
            sys.exit(1)
        print(f"{OUT}: {len(build()['cases'])} durum güncel.")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(build()['cases'])} durum yazıldı.")


if __name__ == "__main__":
    main()
