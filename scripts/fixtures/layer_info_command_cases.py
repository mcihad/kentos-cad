"""Writes fixtures/commands/v1/cad.layers.info.json: the shared cases of `cad.layers.info` v1 (docs/adr/0225 §4–§6),
their expectations built here from the contract's rules (crates/shared/contracts/src/cad_layers.rs, layer_info.rs: the
checks and their order, the codes, the paths and the words; the step's name) and from the ADR's templates (§6: `{…}` an
expression, `{{` and `}}` plain braces, positions 1-based), without KentOS code. The expression language's own words
for the few broken expressions below are its lexer's and its world rule's (docs/adr/0100, 0214). The web
(apps/web/src/product/fixtures.test.ts), the desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_info_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_info_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.info.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
LABEL = "Bilgi kartı"
UNCLOSED = "“{” kapanmamış: ifadeyi “}” ile kapatın (düz ayraç için {{ yazın)."
LONE = "“}” tek başına; düz ayraç için }} yazın."
EMPTY = "“{}” boş: araya bir ifade yazın."
WORLD_REFUSED = "Bu işlev başka katmanlara bakar; burada kullanılamaz."
NO_FIELD = "Köşeli parantez içinde alan adı yok."


def node(id_, name, type_="layer", children=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True, "style": dict(STYLE), "children": children or [], **extra}


SETUP_LAYERS = [
    node("parsel", "Parsel"),
    node("bina", "Bina"),
    node("koruma", "Koruma alanı", "group", [node("kilitli", "Sit alanı")], locked=True),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
    node("0", "0"),
]

X, Y = 500000, 4400000


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Ada": "101", "Parsel": "5", "Malik": "Ayşe Yılmaz"}, "pts": rect(X, Y, 40, 30)},
    {"kind": "polygon", "id": 2, "layerId": "bina", "attrs": {"Kat": "4"}, "pts": rect(X + 5, Y + 5, 10, 10)},
    {"kind": "point", "id": 3, "layerId": "kilitli", "attrs": {}, "p": {"x": X + 10, "y": Y + 50}},
]
IDS = [e["id"] for e in ENTITIES]

SORGU = {"name": "Parsel sorgu", "target": "https://parselsorgu.example/{Ada}/{Parsel}"}
HARITA = {"name": "Haritada aç", "target": "https://www.google.com/maps?q={$enlem},{$boylam}"}
KLASOR = {"name": "Tapu klasörü", "target": "belgeler/{Ada}-{Parsel}.pdf"}
CARD = {"title": "Parsel {Ada}/{Parsel}", "fields": ["Malik", "Nitelik"], "text": "Alan: {yuvarla($alan, 2)} m²\nTapu: {[Tapu alanı]} m² {{not}}"}


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Bilgi kartı",
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


def changed(layers, id_, **fields):
    out = copy.deepcopy(layers)
    get(out, id_).update(copy.deepcopy(fields))
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


def informed(layer, changed_=True):
    return completed({"layer": layer, "changed": changed_, "revision": "$current"})


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree()})]}


def cases():
    out = []
    full = changed(tree(), "parsel", actions=[SORGU, HARITA, KLASOR], card=CARD)
    out.append({
        "name": "Eylemler ve kart: sırasıyla yazılır; adımı “Bilgi kartı”, geri alınır ve yinelenir; nesneler değişmez",
        "steps": [
            step("execute", {"layer": "parsel", "actions": [SORGU, HARITA, KLASOR], "card": CARD}, informed("parsel"),
                 {"ids": IDS, "entities": {"1": ENTITIES[0]}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": full}),
            step("undo", returns=LABEL, expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=LABEL, expect={"layers": full, "canUndo": True, "canRedo": False}),
        ],
    })
    hidden = {"attachments": False}
    out.append({
        "name": "Yalnız kart: ekleri gizleyen düğme yalnız kapalıyken yazılır",
        "steps": [step("execute", {"layer": "bina", "card": hidden}, informed("bina"), {"layers": changed(tree(), "bina", card=hidden), "canUndo": True})],
    })
    out.append({
        "name": "İçi boş kart kart değildir: düğmeleri açık kart da; kartı olmayan katmanda bir şey yazılmaz",
        "steps": [
            step("execute", {"layer": "bina", "card": {}}, informed("bina", False), {**UNCHANGED, "layers": tree()}),
            step("execute", {"layer": "bina", "card": {"attachments": True, "actions": True}}, informed("bina", False), {**UNCHANGED, "layers": tree()}),
        ],
    })
    out.append({
        "name": "Kaldır: eylemsiz ve kartsız girdi; geri almada döner",
        "steps": [
            step("execute", {"layer": "parsel", "actions": [SORGU], "card": CARD}, informed("parsel"), {"layers": changed(tree(), "parsel", actions=[SORGU], card=CARD)}),
            step("execute", {"layer": "parsel"}, informed("parsel"), {"layers": tree(), "revision": "changed"}),
            step("undo", returns=LABEL, expect={"layers": changed(tree(), "parsel", actions=[SORGU], card=CARD)}),
        ],
    })
    out.append({
        "name": "Aynısı: bir şey yazılmaz, changed false",
        "steps": [
            step("execute", {"layer": "parsel", "actions": [SORGU]}, informed("parsel"), {"layers": changed(tree(), "parsel", actions=[SORGU])}),
            step("execute", {"layer": "parsel", "actions": [SORGU]}, informed("parsel", False), {"layers": changed(tree(), "parsel", actions=[SORGU]), "revision": "same", "canUndo": True}),
        ],
    })
    out.append({
        "name": "Kilitli katman: eylemler ve kart katmanın ayarıdır, nesnelere dokunmaz; yazılır",
        "steps": [step("execute", {"layer": "kilitli", "actions": [KLASOR]}, informed("kilitli"), {"ids": IDS, "layers": changed(tree(), "kilitli", actions=[KLASOR])})],
    })
    out.append({
        "name": "Servis katmanından eylem ve kart kaldırmak reddedilmez: yokturlar",
        "steps": [step("execute", {"layer": "altlik"}, informed("altlik", False), UNCHANGED)],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"layer": "parsel", "actions": [SORGU], "card": CARD}, {"status": "completed", "output": None, "warnings": []}, {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: katmanın yazılacak hâli; yazmaz",
        "steps": [step("plan", {"layer": "parsel", "actions": [SORGU, HARITA, KLASOR], "card": CARD},
                       completed({"node": get(full, "parsel"), "changed": True, "revision": "$current"}), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: içi boş kartla katman kartsızdır, değişmez",
        "steps": [step("plan", {"layer": "bina", "card": {}}, completed({"node": get(tree(), "bina"), "changed": False, "revision": "$current"}), UNCHANGED)],
    })
    # The refusals, in the contract's order: the actions' rules, the card's, the templates, then the revision and the layer.
    for why, actions, words in (
        ("boş ad", [{"name": "", "target": "https://a.org"}], "eylemin adı boş ya da başında veya sonunda boşluk var"),
        ("başında boşluklu ad", [{"name": " Sorgu", "target": "https://a.org"}], "eylemin adı boş ya da başında veya sonunda boşluk var"),
        ("büyük küçük harften bağımsız aynı ad", [SORGU, {"name": "PARSEL SORGU", "target": "https://b.org"}], "“PARSEL SORGU” adlı iki eylem var"),
        ("boş hedef", [{"name": "Sorgu", "target": "  "}], "“Sorgu” eyleminin hedefi boş"),
        ("33 eylem", [{"name": f"Eylem {k}", "target": f"https://a.org/{k}"} for k in range(33)], "katmanın 33 eylemi var; en çok 32"),
    ):
        out.append(refused(f"bozuk eylemler: {why}", {"layer": "parsel", "actions": actions}, "invalid_actions", f"Katmanın eylemleri: {words}.", "actions"))
    for why, card, words in (
        ("boş başlık", {"title": " "}, "kartın başlığı boş, 1000 karakterden uzun ya da satır sonu içeriyor"),
        ("satır sonlu başlık", {"title": "Parsel\n{Ada}"}, "kartın başlığı boş, 1000 karakterden uzun ya da satır sonu içeriyor"),
        ("sekmeli metin", {"text": "a\tb"}, "kartın metni boş, 10000 karakterden uzun ya da denetim karakteri içeriyor"),
        ("iki kez geçen alan", {"fields": ["Malik", "Malik"]}, "kart “Malik” alanını iki kez gösteriyor"),
        ("boş alan adı", {"fields": ["Malik", " "]}, "kartın alanlarında boş ad var"),
    ):
        out.append(refused(f"bozuk kart: {why}", {"layer": "parsel", "card": card}, "invalid_card", f"Katmanın bilgi kartı: {words}.", "card"))
    out.append(refused("eylemler karttan önce", {"layer": "parsel", "actions": [{"name": "", "target": "x"}], "card": {"title": " "}}, "invalid_actions",
                       "Katmanın eylemleri: eylemin adı boş ya da başında veya sonunda boşluk var.", "actions"))
    # Templates: the position is the template's own (1-based UTF-16); an expression's error at its place in it.
    unclosed = "https://a.org/{Ada"
    out.append(refused("şablon: kapanmamış ayraç", {"layer": "parsel", "actions": [{"name": "Sorgu", "target": unclosed}]}, "invalid_template",
                       f"“Sorgu” eyleminin hedefi: {unclosed.index('{') + 1}. karakterde: {UNCLOSED}", "actions/0/target"))
    out.append(refused("şablon: ikinci eylemin tek başına kapanan ayracı", {"layer": "parsel", "actions": [SORGU, {"name": "Klasör", "target": "a } b"}]}, "invalid_template",
                       f"“Klasör” eyleminin hedefi: 3. karakterde: {LONE}", "actions/1/target"))
    out.append(refused("şablon: boş ayraç kartın başlığında", {"layer": "parsel", "card": {"title": "Parsel {}"}}, "invalid_template", f"Kartın başlığı: 8. karakterde: {EMPTY}", "card/title"))
    out.append(refused("şablon: $sıra kartın metninde", {"layer": "parsel", "card": {"text": "Sıra: { $sıra }"}}, "invalid_template",
                       "Kartın metni: 8. karakterde: Şablonda $sıra kullanılamaz.", "card/text"))
    out.append(refused("şablon: $ölçek kartın metninde", {"layer": "parsel", "card": {"text": "{$ölçek}"}}, "invalid_template",
                       "Kartın metni: 2. karakterde: Şablonda $ölçek kullanılamaz.", "card/text"))
    out.append(refused("şablon: ifadenin hatası şablondaki yeriyle", {"layer": "parsel", "card": {"title": "Ada {[ ]}"}}, "invalid_template",
                       f"Kartın başlığı: 6. karakterde: {NO_FIELD}", "card/title"))
    world = "Mahalle: {katmandan('Mahalle', Ad, 'Kod', Mahalle)}"
    out.append(refused("şablon: başka katmana bakan işlev", {"layer": "parsel", "card": {"text": world}}, "invalid_template",
                       f"Kartın metni: {world.index('katmandan') + 1}. karakterde: {WORLD_REFUSED}", "card/text"))
    out.append(refused("şablon: eylemler kartın başlığından önce", {"layer": "parsel", "actions": [{"name": "Sorgu", "target": unclosed}], "card": {"title": "{}"}}, "invalid_template",
                       f"“Sorgu” eyleminin hedefi: {unclosed.index('{') + 1}. karakterde: {UNCLOSED}", "actions/0/target"))
    out.append(refused("bozuk şablon sürümden ve katmandan önce", {"layer": "yok", "card": {"title": "{"}, "expectedRevision": "on iki"}, "invalid_template",
                       f"Kartın başlığı: 1. karakterde: {UNCLOSED}", "card/title"))
    out.append(refused("geçersiz beklenen sürüm", {"layer": "parsel", "actions": [SORGU], "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"layer": "bina", "card": hidden}, informed("bina"), {"revision": "changed"}),
            step("execute", {"layer": "parsel", "actions": [SORGU], "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "layers": changed(tree(), "bina", card=hidden)}),
        ],
    })
    out.append(refused("çizimde olmayan katman", {"layer": "yok", "actions": [SORGU]}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"layer": "koruma", "actions": [SORGU]}, "not_a_layer",
                       "“Koruma alanı” bir katman grubu; eylemler ve bilgi kartı yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("servis katmanı", {"layer": "altlik", "card": CARD}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; eylemler ve bilgi kartı nesneleri olan katmanın olur.", "layer"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.info",
        "commandVersion": 1,
        "title": "Bilgi kartı ve eylemler",
        "note": "Written by scripts/fixtures/layer_info_command_cases.py from the contract's rules and docs/adr/0225 §4–§6; do not edit by hand.",
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
