"""Writes fixtures/commands/v1/cad.library.edit.json: the shared cases of `cad.library.edit` v1 (docs/adr/0314 §3).

The expectations are built here from the contract's rules (crates/shared/commands/src/cad_library.rs: the checks and
their order, the codes, paths and words) and the style library's rules as docs/adr/0092, 0176 and docs/STYLE.md §5 give
them (an item's id, kind, name and path; a copy “… (kopya)”; a blank name “Adsız”; a file's assets taken in before its
symbols, its templates last), without KentOS code:

- the library is a project part outside the undo steps: writing it makes the drawing dirty, no undo step;
- an item something uses (an object's own symbol, a layer's renderer, a symbol drawing an asset, a template) is not
  removed, the users counted;
- giving a symbol is one undo step through `cad.entities.set` (“Sembol ata”) or `cad.layers.renderer` (“Katman stili”);
- an export's text is the canonical JSON of a `.kstil` file without its time (checked by the Rust tests: `$any` here).

    python3 scripts/fixtures/library_command_cases.py           # writes the file
    python3 scripts/fixtures/library_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.library.edit.json"

X, Y = 500000, 4400000
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."

SVG = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 30"><circle cx="10" cy="10" r="8" fill="#2E7D32"/></svg>'
CIZIM = {"kind": "asset", "id": "p-cizim", "name": "Ağaç çizimi", "path": ["Çizimler"], "format": "svg", "data": SVG, "width": 20, "height": 30}
AGAC = {"kind": "symbol", "id": "p-agac", "name": "Ağaç", "path": ["Semboller"],
        "symbol": {"type": "marker", "layers": [{"id": "v", "type": "svg", "asset": "p-cizim", "size": 4}]}}
ALAN = {"kind": "symbol", "id": "p-alan", "name": "Yeşil alan", "path": ["Semboller"],
        "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#30A46C"}]}}
BOS = {"kind": "symbol", "id": "p-bos", "name": "Boş", "path": ["Semboller"],
       "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#EDC948"}]}}
ITEMS = [CIZIM, AGAC, ALAN, BOS]
CATEGORIES = [{"path": ["Semboller"], "order": 1}]
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def node(id_, name, style=STYLE, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": dict(style), "children": []}


LAYERS = [
    node("park", "Park"),
    node("agaclar", "Ağaçlar", {**STYLE, "renderer": {"type": "single", "symbols": {"marker": {"ref": "p-agac"}}}}),
    node("kilitli", "Kilitli", locked=True),
]


def square(id_, layer, x, **own):
    return {"kind": "polygon", "id": id_, "layerId": layer, "attrs": {}, "pts": [
        {"x": X + x, "y": Y}, {"x": X + x + 10, "y": Y}, {"x": X + x + 10, "y": Y + 10}, {"x": X + x, "y": Y + 10}], **own}


ENTITIES = [
    square(1, "park", 0, symbol="p-alan"),
    square(2, "park", 20),
    {"kind": "point", "id": 3, "layerId": "agaclar", "attrs": {}, "p": {"x": X + 5, "y": Y + 30}},
    square(4, "kilitli", 40),
]
IDS = [e["id"] for e in ENTITIES]


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Kitaplık",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(LAYERS),
        "activeLayer": "park",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": copy.deepcopy(ITEMS), "categories": copy.deepcopy(CATEGORIES)},
    }


# ── The rules ────────────────────────────────────────────────────────────────

def listed(items):
    return [{"id": i["id"], "kind": i["kind"], "name": i["name"], "path": i.get("path", [])} for i in items]


def done(changed=False, added=(), replaced=(), skipped=(), removed=(), kstil=None, objects=(), layer=None):
    out = {"changed": changed, "added": list(added), "replaced": list(replaced), "skipped": list(skipped), "removed": list(removed),
           "objects": list(objects)}
    if kstil is not None:
        out["kstil"] = kstil
    if layer is not None:
        out["layer"] = layer
    out["revision"] = "$current"
    return {"status": "completed", "output": out, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_, result, expect):
    return {"op": op, "input": input_, "result": result, "expect": expect}


def unchanged():
    return {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same", "library": listed(ITEMS)}


def wrote(name, input_, result, items):
    """An edit of the library: the plan writes nothing; a write that changes it makes the drawing dirty, no undo step."""
    return {"name": name, "steps": [
        step("plan", input_, result, {"revision": "same", "library": listed(ITEMS)}),
        step("execute", input_, result, {"ids": IDS, "canUndo": False, "dirty": result["output"]["changed"], "library": listed(items)}),
    ]}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), unchanged())]}


def with_(items, id_, **fields):
    out = copy.deepcopy(items)
    for i in out:
        if i["id"] == id_:
            for k, v in fields.items():
                if v is None:
                    i.pop(k, None)
                else:
                    i[k] = v
    return out


def kstil(items):
    return json.dumps({"format": "kentos-style", "version": 1, "items": items}, ensure_ascii=False)


def in_use(id_, objects, layers, items):
    return f"“{id_}” öğesini {objects} nesne, {layers} katman ve kitaplığın {items} öğesi kullanıyor; önce onlardan kaldırın."


def cases():
    yeni = {"kind": "symbol", "id": "p-yeni", "name": "Su", "path": ["Semboller"],
            "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#0090FF"}]}}
    gelen_bos = {**BOS, "name": "Gelen boş"}
    gelen = {"kind": "symbol", "id": "p-gelen", "name": "Gelen", "path": ["Gelenler"],
             "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#8A94A6"}]}}
    copy_of = {**copy.deepcopy(ALAN), "id": "p-alan-2", "name": "Yeşil alan 2"}
    out = [
        wrote("Oluştur: yeni sembol sona eklenir", {"operation": "create", "item": yeni}, done(True, added=["p-yeni"]), ITEMS + [yeni]),
        wrote("Güncelle: yamanın alanları yazılır, kimlik ve tür değişmez",
              {"operation": "update", "id": "p-alan", "patch": {"name": "Çimen", "tags": ["park"], "id": "baska", "kind": "asset"}},
              done(True, replaced=["p-alan"]), with_(ITEMS, "p-alan", name="Çimen", tags=["park"])),
        wrote("Kopyala: yeni kimlik ve adla, aynı yerde", {"operation": "duplicate", "id": "p-alan", "newId": "p-alan-2", "name": "Yeşil alan 2"},
              done(True, added=["p-alan-2"]), ITEMS + [copy_of]),
        wrote("Kopyala: ad verilmezse “(kopya)”, yol verilirse oraya",
              {"operation": "duplicate", "id": "p-bos", "newId": "p-bos-2", "path": ["Arşiv"]},
              done(True, added=["p-bos-2"]), ITEMS + [{**copy.deepcopy(BOS), "id": "p-bos-2", "name": "Boş (kopya)", "path": ["Arşiv"]}]),
        wrote("Ad ver: boş ad “Adsız”", {"operation": "rename", "id": "p-bos", "name": "  "}, done(True, replaced=["p-bos"]),
              with_(ITEMS, "p-bos", name="Adsız")),
        wrote("Taşı: başka kategoriye", {"operation": "move", "id": "p-bos", "path": ["Arşiv", "Eski"]}, done(True, replaced=["p-bos"]),
              with_(ITEMS, "p-bos", path=["Arşiv", "Eski"])),
        wrote("Sil: kullanılmayan sembol", {"operation": "remove", "id": "p-bos"}, done(True, removed=["p-bos"]), ITEMS[:3]),
        wrote("İçe al: yeni öğe eklenir, kitaplıkta olan kalır", {"operation": "import", "kstil": kstil([gelen_bos, gelen])},
              done(True, added=["p-gelen"], skipped=["p-bos"]), ITEMS + [gelen]),
        wrote("İçe al: replace ile kitaplıkta olan değişir, yerinde", {"operation": "import", "kstil": kstil([gelen_bos, gelen]), "conflict": "replace"},
              done(True, added=["p-gelen"], replaced=["p-bos"]), with_(ITEMS, "p-bos", name="Gelen boş") + [gelen]),
        wrote("İçe al: hepsi kitaplıkta, değişen yok", {"operation": "import", "kstil": kstil([BOS])}, done(skipped=["p-bos"]), ITEMS),
        wrote("Dışa aktar: sembol çizdiği çizimle; çizim değişmez", {"operation": "export", "ids": ["p-agac"]}, done(kstil="$any"), ITEMS),
        wrote("Dışa aktar: kimlik verilmezse bütün kitaplık", {"operation": "export"}, done(kstil="$any"), ITEMS),
        {"name": "Ver: sembol nesnelere tek adımda (Sembol ata), kilitli katmandaki atlanır", "steps": [
            step("execute", {"operation": "assign", "id": "p-alan", "uids": ["$uidOf:2"]},
                 done(True, objects=["$uidOf:2"]),
                 {"ids": IDS, "canUndo": True, "dirty": True, "library": listed(ITEMS),
                  "entities": {"2": {**copy.deepcopy(ENTITIES[1]), "symbol": "p-alan"}}}),
        ]},
        {"name": "Ver: sembol katmanın tek sembollü işleyicisine, türünün yuvasına (Katman stili)", "steps": [
            step("execute", {"operation": "assign", "id": "p-alan", "layer": "park"}, done(True, layer="park"),
                 {"ids": IDS, "canUndo": True, "dirty": True,
                  "layers": [node("park", "Park", {**STYLE, "renderer": {"type": "single", "symbols": {"fill": {"ref": "p-alan"}}}}),
                             copy.deepcopy(LAYERS[1]), copy.deepcopy(LAYERS[2])]}),
        ]},
        {"name": "Ver: tek sembollü işleyicinin öbür yuvaları kalır", "steps": [
            step("execute", {"operation": "assign", "id": "p-alan", "layer": "agaclar"}, done(True, layer="agaclar"),
                 {"ids": IDS, "canUndo": True,
                  "layers": [copy.deepcopy(LAYERS[0]),
                             node("agaclar", "Ağaçlar", {**STYLE, "renderer": {"type": "single", "symbols": {
                                 "marker": {"ref": "p-agac"}, "fill": {"ref": "p-alan"}}}}),
                             copy.deepcopy(LAYERS[2])]}),
        ]},
        {"name": "Doğrula: yazmaz", "steps": [step("validate", {"operation": "remove", "id": "p-bos"},
                                                   {"status": "completed", "output": None, "warnings": []}, unchanged())]},
        refused("Ret: oluşturulacak öğe yok", {"operation": "create"}, "no_item",
                "Eklenecek öğeyi verin (item): kimliği, türü, adı ve yolu ile.", "item"),
        refused("Ret: kimlik yok", {"operation": "rename", "name": "X"}, "no_id", "Öğenin kimliğini verin (id).", "id"),
        refused("Ret: içe alınacak metin yok", {"operation": "import", "kstil": "  "}, "no_kstil",
                "Alınacak .kstil dosyasının metnini verin (kstil).", "kstil"),
        refused("Ret: sembol hiçbir şeye verilmiyor", {"operation": "assign", "id": "p-alan", "uids": []}, "no_targets",
                "Sembolün verileceği nesneleri (uids) ya da katmanı (layer) verin.", "uids"),
        refused("Ret: öğenin adı yok", {"operation": "create", "item": {"kind": "symbol", "id": "p-x", "path": [], "symbol": ALAN["symbol"]}},
                "invalid_library_item", "Kitaplık öğesi kurallara uymuyor: öğe 1: ad yok.", "item"),
        refused("Ret: .kstil değil", {"operation": "import", "kstil": json.dumps({"format": "baska", "version": 1, "items": []})},
                "invalid_kstil", ".kstil dosyası okunamadı: Bu bir KentOS stil dosyası (.kstil) değil.", "kstil"),
        refused("Ret: geçersiz sürüm", {"operation": "remove", "id": "p-bos", "expectedRevision": "on iki"}, "invalid_revision",
                INVALID_REVISION, "expectedRevision"),
        refused("Ret: kimlik kitaplıkta var", {"operation": "create", "item": {**BOS, "name": "İkinci"}}, "library_item_exists",
                "Projenin kitaplığında “p-bos” kimlikli bir öğe var; başka bir kimlik verin.", "item.id"),
        refused("Ret: kopyanın kimliği kitaplıkta var", {"operation": "duplicate", "id": "p-alan", "newId": "p-bos"}, "library_item_exists",
                "Projenin kitaplığında “p-bos” kimlikli bir öğe var; başka bir kimlik verin.", "newId"),
        refused("Ret: öğe yok", {"operation": "update", "id": "p-yok", "patch": {"name": "x"}}, "library_item_not_found",
                "“p-yok” kimlikli öğe projenin kitaplığında yok.", "id"),
        refused("Ret: dışa aktarılacak öğe yok", {"operation": "export", "ids": ["p-agac", "p-yok"]}, "library_item_not_found",
                "“p-yok” kimlikli öğe projenin kitaplığında yok.", "ids[1]"),
        refused("Ret: nesnenin sembolü silinmez", {"operation": "remove", "id": "p-alan"}, "library_item_in_use", in_use("p-alan", 1, 0, 0), "id"),
        refused("Ret: katmanın sembolü silinmez", {"operation": "remove", "id": "p-agac"}, "library_item_in_use", in_use("p-agac", 0, 1, 0), "id"),
        refused("Ret: sembolün çizimi silinmez", {"operation": "remove", "id": "p-cizim"}, "library_item_in_use", in_use("p-cizim", 0, 0, 1), "id"),
        refused("Ret: sembolün verileceği katman yok (cad.layers.renderer'ın reddi)", {"operation": "assign", "id": "p-alan", "layer": "yok"},
                "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"),
        {"name": "Ret: hazırlandığı sürümden sonra çizim değişmiş", "steps": [
            {"op": "captureRevision", "as": "once"},
            step("execute", {"operation": "rename", "id": "p-bos", "name": "Bir"}, done(True, replaced=["p-bos"]), {"revision": "changed"}),
            step("execute", {"operation": "rename", "id": "p-bos", "name": "İki", "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "library": listed(with_(ITEMS, "p-bos", name="Bir"))}),
        ]},
        refused("Ret: çizim sembol değil", {"operation": "assign", "id": "p-cizim", "uids": ["$uidOf:1"]}, "not_a_symbol",
                "“Ağaç çizimi” bir sembol değil; nesnelere ve katmanlara yalnız sembol verilir.", "id"),
    ]
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.library.edit",
        "commandVersion": 1,
        "title": "Stil kitaplığı",
        "note": "Written by scripts/fixtures/library_command_cases.py from the contract's rules and docs/adr/0092, 0176, 0314 §3; "
                "do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum tutarlı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
