"""Writes the shared cases of the look's sources (docs/adr/0312 §4): fixtures/look/v1/cases.json.

An object draws each part of its look (colour, line type, its pattern's scale, line weight, transparency) from:

- itself, when it has its own (explicit);
- in a block, a part it lists under `byLayer`: its own layer (AutoCAD's BYLAYER in a block);
- in a block, any other part: the insert it is drawn through, as that insert draws it (BYBLOCK), nested inserts in
  turn;
- in the drawing: its layer. A layer has no transparency (0) and no pattern scale (1).

An object of a block on the block's layer (`layerId` "", the DXF's layer 0; docs/adr/0315 §2) has the layer of the
insert it is drawn through: its `byLayer` parts are that layer's, nested inserts in turn.

Each case is a chain, the outermost insert first and the drawn object last, each link with its layer and its own
parts; the expected look and where each part came from (`own`, `layer`, `block`). Written from the rule above, without
KentOS code. The contracts (crates/shared/contracts/src/look.rs, `resolve_look`) run them.

    python3 scripts/fixtures/look_cases.py           # writes the file
    python3 scripts/fixtures/look_cases.py --check   # compares it with the one on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/look/v1/cases.json"

LAYERS = {
    "duvar": {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.5},
    "eksen": {"color": "#E06C75", "lineType": "dashdot", "lineWeight": 0.18},
    "kapi": {"color": "#C8A165", "lineType": "continuous", "lineWeight": 0.25},
    "yardimci": {"color": "fg-dim", "lineType": "dashed", "lineWeight": 0.13},
    "sifir": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25},
}

PARTS = [("color", "color"), ("lineType", "lineType"), ("lineWeight", "lineWeight"), ("transparency", "transparency")]


def resolve(link, outer, layer_id):
    """One link's look: its own parts, else (in a block) its layer's for its byLayer parts and the insert's for the
    rest, else its layer's. `layer_id`: its layer, the insert's on the block's layer."""
    layer = LAYERS[layer_id]
    by_layer = set(link.get("byLayer", []))
    out, src = {}, {}
    for key, part in PARTS:
        if key in link:
            out[key], src[key] = link[key], "own"
        elif outer is not None and part not in by_layer:
            out[key], src[key] = outer[key], "block"
        else:
            out[key] = layer.get(key, 0.0) if key != "transparency" else 0.0
            src[key] = "layer"
    if "lineTypeScale" in link:
        out["lineTypeScale"], src["lineTypeScale"] = link["lineTypeScale"], "own"
    elif outer is not None:
        out["lineTypeScale"], src["lineTypeScale"] = outer["lineTypeScale"], "block"
    else:
        out["lineTypeScale"], src["lineTypeScale"] = 1.0, "layer"
    for key in ("lineWeight", "transparency", "lineTypeScale"):
        out[key] = float(out[key])
    out["from"] = src
    return out


def case(name, chain):
    r, layer_id = None, None
    for link in chain:
        layer_id = link["layerId"] or layer_id
        r = resolve(link, r, layer_id)
    return {"name": name, "chain": chain, "expect": r}


def cases():
    return [
        case("Çizimde kendi değeri olmayan nesne katmanından alır", [{"layerId": "duvar"}]),
        case("Çizimde kendi rengi olan nesne", [{"layerId": "duvar", "color": "#0090FF"}]),
        case("Çizimde kendi çizgi tipi, ölçeği ve saydamlığı", [{"layerId": "duvar", "lineType": "dashed", "lineTypeScale": 2,
                                                                "transparency": 0.5}]),
        case("Çizimde kendi kalınlığı (0 en ince çizgi, kendi değeridir)", [{"layerId": "eksen", "lineWeight": 0}]),
        case("Katmanın saydamlığı yok, çizgi ölçeği 1", [{"layerId": "yardimci"}]),
        case("Bloğun değersiz nesnesi yerleştirmenin katmanından alır (BYBLOCK)",
             [{"layerId": "kapi"}, {"layerId": "sifir"}]),
        case("Bloğun değersiz nesnesi yerleştirmenin kendi değerlerini alır",
             [{"layerId": "kapi", "color": "#00FF00", "lineType": "dashed", "lineWeight": 0.35, "transparency": 0.3},
              {"layerId": "sifir"}]),
        case("Bloğun nesnesinin kendi değerleri yerleştirmeninkini yener",
             [{"layerId": "kapi", "color": "#00FF00", "lineType": "dashed"}, {"layerId": "sifir", "color": "#FF0000",
                                                                             "lineType": "continuous"}]),
        case("Katmana göre renk: bloğun nesnesi kendi katmanının rengini alır",
             [{"layerId": "kapi", "color": "#00FF00"}, {"layerId": "duvar", "byLayer": ["color"]}]),
        case("Katmana göre çizgi tipi ve kalınlık, öbürleri yerleştirmeden",
             [{"layerId": "kapi", "color": "#00FF00", "lineType": "dashed", "lineWeight": 0.35},
              {"layerId": "eksen", "byLayer": ["lineType", "lineWeight"]}]),
        case("Katmana göre saydamlık: katmanın saydamlığı yok",
             [{"layerId": "kapi", "transparency": 0.4}, {"layerId": "duvar", "byLayer": ["transparency"]}]),
        case("Katmana göre her şey", [{"layerId": "kapi", "color": "#00FF00", "lineType": "dashed", "lineWeight": 0.35,
                                       "transparency": 0.2},
                                      {"layerId": "eksen", "byLayer": ["color", "lineType", "lineWeight", "transparency"]}]),
        case("Çizgi ölçeği yerleştirmeden gelir", [{"layerId": "kapi", "lineTypeScale": 3}, {"layerId": "sifir"}]),
        case("Kendi çizgi ölçeği yerleştirmeninkini yener", [{"layerId": "kapi", "lineTypeScale": 3},
                                                              {"layerId": "sifir", "lineTypeScale": 0.5}]),
        case("İç içe: içteki yerleştirmenin değeri dıştakini yener",
             [{"layerId": "kapi", "color": "#00FF00"}, {"layerId": "sifir", "color": "#0000FF"}, {"layerId": "sifir"}]),
        case("İç içe: içteki yerleştirme değersizse dıştakinden geçer",
             [{"layerId": "kapi", "color": "#00FF00", "lineType": "dashed"}, {"layerId": "sifir"}, {"layerId": "sifir"}]),
        case("İç içe: içteki yerleştirme katmana göre, nesnesi ondan alır",
             [{"layerId": "kapi", "color": "#00FF00"}, {"layerId": "duvar", "byLayer": ["color"]}, {"layerId": "sifir"}]),
        case("İç içe: dıştaki yerleştirme değersiz, kendi katmanından; nesne katmana göre kendi katmanından",
             [{"layerId": "eksen"}, {"layerId": "sifir"}, {"layerId": "yardimci", "byLayer": ["lineType"]}]),
        case("Üç kat iç içe, saydamlık en dıştan", [{"layerId": "kapi", "transparency": 0.6}, {"layerId": "sifir"},
                                                     {"layerId": "sifir"}, {"layerId": "sifir"}]),
        case("Tema rengi değer olarak taşınır (fg-dim)", [{"layerId": "yardimci"}, {"layerId": "sifir"}]),
        case("Kendi rengi tema rengi (ink) olan nesne", [{"layerId": "duvar", "color": "ink"}]),
        case("Yerleştirmenin katmanı kesikli, nesnenin kendi çizgi tipi sürekli",
             [{"layerId": "yardimci"}, {"layerId": "sifir", "lineType": "continuous"}]),
        case("0 katmanındaki nesne katmana göre: yerleştirmenin katmanının değerleri, kendi rengi değil",
             [{"layerId": "kapi", "color": "#00FF00"}, {"layerId": "", "byLayer": ["color", "lineType", "lineWeight",
                                                                                   "transparency"]}]),
        case("0 katmanındaki nesne bloğa göre: yerleştirmenin kendi rengi",
             [{"layerId": "kapi", "color": "#00FF00"}, {"layerId": ""}]),
        case("0 katmanındaki nesne kısmen katmana göre: renk katmandan, çizgi tipi yerleştirmeden",
             [{"layerId": "eksen", "lineType": "dashed"}, {"layerId": "", "byLayer": ["color"]}]),
        case("İç içe 0 katmanı: içteki yerleştirme de 0'da, nesne en dıştakinin katmanından",
             [{"layerId": "duvar"}, {"layerId": ""}, {"layerId": "", "byLayer": ["color", "lineWeight"]}]),
        case("İç içe 0 katmanı: içteki yerleştirme kendi katmanında, nesne onun katmanından",
             [{"layerId": "duvar"}, {"layerId": "eksen"}, {"layerId": "", "byLayer": ["color", "lineType"]}]),
        case("Kendi katmanındaki nesne katmana göre, 0'daki içteki yerleştirmeden geçerken",
             [{"layerId": "kapi", "color": "#00FF00"}, {"layerId": ""}, {"layerId": "yardimci",
                                                                         "byLayer": ["color", "lineType"]}]),
    ]


def main() -> int:
    doc = {"format": "kentos.look-cases", "version": 1,
           "note": "Written by scripts/fixtures/look_cases.py from docs/adr/0312 §4; do not edit by hand.",
           "layers": LAYERS, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.name} güncel: {len(doc['cases'])} durum.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
