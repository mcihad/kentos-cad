"""Writes fixtures/commands/v1/cad.layers.compose.json: the shared cases of `cad.layers.compose` v1 (docs/adr/0226 §3,
§4, §6), their expectations built here from the contract's rules (crates/shared/commands/src/cad_layers.rs,
layer_compose.rs: the checks and their order, the codes, the paths and the words; what says nothing is not kept; the
step's name), without KentOS code. The web (apps/web/src/product/fixtures.test.ts), the desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_compose_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_compose_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.compose.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
LABEL = "Katman çizimi"


def node(id_, name, type_="layer", children=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True, "style": dict(STYLE), "children": children or [], **extra}


SETUP_LAYERS = [
    node("kabartma", "Gölgeli kabartma"),
    node("parsel", "Parsel"),
    node("yol", "Yol"),
    node("koruma", "Koruma alanı", "group", [node("kilitli", "Sit alanı")], locked=True),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
    node("0", "0"),
]

X, Y = 500000, 4400000


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Ada": "101", "Parsel": "5"}, "label": "101/5", "pts": rect(X, Y, 40, 30)},
    {"kind": "line", "id": 2, "layerId": "yol", "attrs": {"Ad": "Atatürk Caddesi"}, "a": {"x": X - 10, "y": Y + 35}, "b": {"x": X + 60, "y": Y + 35}},
    {"kind": "point", "id": 3, "layerId": "kilitli", "attrs": {}, "p": {"x": X + 10, "y": Y + 50}},
]
IDS = [e["id"] for e in ENTITIES]

MASK = {"labels": True, "size": 1.0, "targets": ["parsel", "kabartma"]}


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Katman çizimi",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(SETUP_LAYERS),
        "activeLayer": "parsel",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


def tree():
    return copy.deepcopy(SETUP_LAYERS)


def get(nodes, id_):
    for n in nodes:
        if n["id"] == id_:
            return n
        hit = get(n["children"], id_)
        if hit:
            return hit
    return None


def kept(compose):
    """The composition as the layer keeps it: an opacity of 1, the normal mode and a mask's switches off are left out;
    None when nothing is left (`LayerCompose::normalized`)."""
    out = {}
    if compose.get("opacity") is not None and compose["opacity"] != 1:
        out["opacity"] = compose["opacity"]
    if compose.get("blend") not in (None, "normal"):
        out["blend"] = compose["blend"]
    if compose.get("mask") is not None:
        m = compose["mask"]
        out["mask"] = {**({"labels": True} if m.get("labels") else {}), **({"markers": True} if m.get("markers") else {}), "size": m["size"], "targets": list(m["targets"])}
    return out or None


def composed(layers, id_, compose):
    out = copy.deepcopy(layers)
    n = get(out, id_)
    n.pop("compose", None)
    c = kept(compose)
    if c is not None:
        n["compose"] = c
    return out


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_=None, result=None, expect=None, **extra):
    s = {"op": op}
    if input_ is not None:
        s["input"] = input_
    if result is not None:
        s["result"] = result
    s.update(extra)
    if expect is not None:
        s["expect"] = expect
    return s


UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def answer(layer, changed_=True):
    return completed({"layer": layer, "changed": changed_, "revision": "$current"})


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree()})]}


def cases():
    out = []
    shade = {"layer": "kabartma", "opacity": 0.6, "blend": "multiply"}
    after = composed(tree(), "kabartma", shade)
    out.append({
        "name": "Saydamlık ve karışım kipi: yazılır; adımı “Katman çizimi”, geri alınır ve yinelenir; nesneler değişmez",
        "steps": [
            step("execute", shade, answer("kabartma"),
                 {"ids": IDS, "entities": {"1": ENTITIES[0]}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": after}),
            step("undo", returns=LABEL, expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=LABEL, expect={"layers": after, "canUndo": True, "canRedo": False}),
        ],
    })
    masked = composed(tree(), "yol", {"mask": MASK})
    out.append({
        "name": "Maske: yol adları parseli ve kabartmayı boşaltır; maskenin kapalı düğmesi yazılmaz",
        "steps": [step("execute", {"layer": "yol", "mask": {**MASK, "markers": False}}, answer("yol"), {"layers": masked, "canUndo": True})],
    })
    out.append({
        "name": "Hiçbir şey söylemeyen çizim çizim değildir: 1 saydamlık ve normal kip; çizimi olmayan katmanda bir şey yazılmaz",
        "steps": [
            step("execute", {"layer": "parsel", "opacity": 1, "blend": "normal"}, answer("parsel", False), {**UNCHANGED, "layers": tree()}),
            step("execute", {"layer": "parsel"}, answer("parsel", False), {**UNCHANGED, "layers": tree()}),
        ],
    })
    out.append({
        "name": "Verilen olur: verilmeyen alan varsayılandır, eski kip kalkar",
        "steps": [
            step("execute", shade, answer("kabartma"), {"layers": after}),
            step("execute", {"layer": "kabartma", "opacity": 0.4}, answer("kabartma"), {"layers": composed(tree(), "kabartma", {"opacity": 0.4}), "revision": "changed"}),
        ],
    })
    out.append({
        "name": "Kaldır: boş girdi; geri almada döner",
        "steps": [
            step("execute", shade, answer("kabartma"), {"layers": after}),
            step("execute", {"layer": "kabartma"}, answer("kabartma"), {"layers": tree(), "revision": "changed"}),
            step("undo", returns=LABEL, expect={"layers": after}),
        ],
    })
    out.append({
        "name": "Aynısı: bir şey yazılmaz, changed false",
        "steps": [
            step("execute", shade, answer("kabartma"), {"layers": after}),
            step("execute", shade, answer("kabartma", False), {"layers": after, "revision": "same", "canUndo": True}),
        ],
    })
    out.append({
        "name": "Kilitli katman ve servis katmanı: katman çizimi katmanın ayarıdır, nesnelere dokunmaz; yazılır",
        "steps": [
            step("execute", {"layer": "kilitli", "opacity": 0.5}, answer("kilitli"), {"ids": IDS, "layers": composed(tree(), "kilitli", {"opacity": 0.5})}),
            step("execute", {"layer": "altlik", "opacity": 0.7, "blend": "screen"}, answer("altlik"),
                 {"layers": composed(composed(tree(), "kilitli", {"opacity": 0.5}), "altlik", {"opacity": 0.7, "blend": "screen"})}),
        ],
    })
    out.append({
        "name": "Saydamlık sınırları: 0 ve 1 geçerlidir",
        "steps": [step("execute", {"layer": "parsel", "opacity": 0}, answer("parsel"), {"layers": composed(tree(), "parsel", {"opacity": 0})})],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", shade, {"status": "completed", "output": None, "warnings": []}, {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: katmanın yazılacak hâli; yazmaz",
        "steps": [step("plan", {"layer": "yol", "mask": MASK}, completed({"node": get(masked, "yol"), "changed": True, "revision": "$current"}), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: hiçbir şey söylemeyen çizimle katman değişmez",
        "steps": [step("plan", {"layer": "parsel", "opacity": 1}, completed({"node": get(tree(), "parsel"), "changed": False, "revision": "$current"}), UNCHANGED)],
    })
    # The refusals, in the contract's order: the opacity, the mask's rule, then the revision, the layer and the targets.
    for why, opacity in (("1'den büyük", 1.5), ("eksi", -0.25)):
        out.append(refused(f"saydamlık {why}", {"layer": "parsel", "opacity": opacity}, "invalid_compose", "Katman çizimi: saydamlık 0 ile 1 arasında olmalı.", "opacity"))
    for why, mask, words in (
        ("kaynaksız", {"size": 1.0, "targets": ["parsel"]}, "maskenin kaynağı yok: etiketler ya da işaretler seçilmeli"),
        ("boyu 10 mm'den büyük", {"markers": True, "size": 10.5, "targets": ["parsel"]}, "maskenin boyu 0 ile 10 mm arasında olmalı"),
        ("eksi boy", {"labels": True, "size": -1.0, "targets": ["parsel"]}, "maskenin boyu 0 ile 10 mm arasında olmalı"),
        ("maskelenen yok", {"labels": True, "size": 1.0, "targets": []}, "maskelenecek katman yok"),
        ("65 katman", {"labels": True, "size": 1.0, "targets": [f"k{i}" for i in range(65)]}, "maske en çok 64 katmanı maskeler"),
        ("boş kimlik", {"labels": True, "size": 1.0, "targets": ["parsel", " "]}, "maskelenen katmanlar arasında boş kimlik var"),
        ("iki kez", {"labels": True, "size": 1.0, "targets": ["parsel", "parsel"]}, "“parsel” katmanı iki kez maskeleniyor"),
    ):
        out.append(refused(f"bozuk maske: {why}", {"layer": "yol", "mask": mask}, "invalid_compose", f"Katman çizimi: {words}.", "mask"))
    out.append(refused("saydamlık maskeden önce", {"layer": "yol", "opacity": 2, "mask": {"size": 1.0, "targets": []}}, "invalid_compose",
                       "Katman çizimi: saydamlık 0 ile 1 arasında olmalı.", "opacity"))
    out.append(refused("bozuk maske sürümden ve katmandan önce", {"layer": "yok", "mask": {"size": 1.0, "targets": ["parsel"]}, "expectedRevision": "on iki"}, "invalid_compose",
                       "Katman çizimi: maskenin kaynağı yok: etiketler ya da işaretler seçilmeli.", "mask"))
    out.append(refused("geçersiz beklenen sürüm", {"layer": "kabartma", "opacity": 0.5, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"layer": "parsel", "opacity": 0.5}, answer("parsel"), {"revision": "changed"}),
            step("execute", {"layer": "kabartma", "opacity": 0.6, "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "layers": composed(tree(), "parsel", {"opacity": 0.5})}),
        ],
    })
    out.append(refused("çizimde olmayan katman", {"layer": "yok", "opacity": 0.5}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"layer": "koruma", "opacity": 0.5}, "not_a_layer",
                       "“Koruma alanı” bir katman grubu; katman çizimi yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("katman maskelenenlerden önce", {"layer": "koruma", "mask": {"labels": True, "size": 1.0, "targets": ["yok"]}}, "not_a_layer",
                       "“Koruma alanı” bir katman grubu; katman çizimi yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("kendini maskeleme", {"layer": "yol", "mask": {"labels": True, "size": 1.0, "targets": ["parsel", "yol"]}}, "mask_target_self",
                       "“Yol” kendini maskeleyemez; maskelenecek başka bir katman verin.", "mask/targets/1"))
    out.append(refused("çizimde olmayan maskelenen", {"layer": "yol", "mask": {"labels": True, "size": 1.0, "targets": ["dere"]}}, "mask_target_not_found",
                       "Maskelenecek “dere” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "mask/targets/0"))
    out.append(refused("grup maskelenmez", {"layer": "yol", "mask": {"markers": True, "size": 0.5, "targets": ["parsel", "koruma"]}}, "mask_target_not_a_layer",
                       "“Koruma alanı” bir katman grubu; maske katmana uygulanır. Grubun katmanlarını verin.", "mask/targets/1"))
    out.append(refused("maskelenenler sırasıyla: önce kendisi", {"layer": "yol", "mask": {"labels": True, "size": 1.0, "targets": ["yol", "dere"]}}, "mask_target_self",
                       "“Yol” kendini maskeleyemez; maskelenecek başka bir katman verin.", "mask/targets/0"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.compose",
        "commandVersion": 1,
        "title": "Katman çizimi",
        "note": "Written by scripts/fixtures/layer_compose_command_cases.py from the contract's rules and docs/adr/0226 §3, §4, §6; do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({len(build()['cases'])} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
