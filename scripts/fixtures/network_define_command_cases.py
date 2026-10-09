"""Writes fixtures/commands/v1/cad.network.define.json: the shared cases of `cad.network.define` v1 (docs/adr/0209 §11),
their expectations built here from the contract's rules (crates/shared/contracts/src/cad_networks.rs: the checks and
their order, the codes, the paths and the words), without KentOS code. A network that breaks a rule takes its words
from fixtures/network/v1/rules.json (the contract's, held by Rust and TypeScript). The web
(apps/web/src/product/fixtures.test.ts), the desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/network_define_command_cases.py           # writes the file
    python3 scripts/fixtures/network_define_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.network.define.json"
RULES = json.loads((ROOT / "fixtures/network/v1/rules.json").read_text(encoding="utf-8"))

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def node(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": dict(STYLE), "children": []}


ROADS = {
    "id": "ag-1",
    "name": "Yollar",
    "kind": "road",
    "edges": [{"layer": "yol"}],
    "connect": "ends",
    "tolerance": 0.01,
    "direction": {"kind": "field", "field": "yon", "forward": ["FT"], "backward": ["TF"], "closed": ["N"]},
    "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0}],
}
WATER = {
    "id": "ag-2",
    "name": "İçme suyu",
    "kind": "utility",
    "edges": [{"layer": "su"}],
    "junctions": [{"layer": "vana", "role": "valve", "closed": "durum = 'kapalı'"}],
    "connect": "ends",
    "tolerance": 0.005,
    "direction": {"kind": "digitized"},
}

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Ağlar",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "networks": [copy.deepcopy(ROADS)]},
    "origin": {"x": 500000, "y": 4400000},
    "layers": [node("yol", "Yol ekseni"), node("su", "Su hattı"), node("vana", "Vana"), node("0", "0")],
    "activeLayer": "0",
    "entities": [{"kind": "line", "id": 1, "layerId": "yol", "attrs": {"yon": "FT"}, "a": {"x": 500000, "y": 4400000}, "b": {"x": 500100, "y": 4400000}}],
    "styles": {"items": [], "categories": []},
}


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


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


UNCHANGED = {"ids": [1], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
# A network is a project setting: the drawing is dirty, its revision changes, and no undo step is written.
WRITTEN = {"ids": [1], "canUndo": False, "canRedo": False, "dirty": True, "revision": "changed"}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "networks": [ROADS]})]}


def rule_words(title):
    case = next(c for c in RULES["networks"] if c["name"] == title)
    return case["value"], case["problem"]


def unknown_layer(layer):
    return {"code": "unknown_layer", "message": f"“{layer}” kimlikli katman çizimde yok; ağ kurulurken bu katman atlanır.", "path": "network"}


def cases():
    out = []
    out.append({
        "name": "Ekle: yeni ağ sona; proje ayarıdır, geri alma adımı yazılmaz",
        "steps": [step("execute", {"operation": "set", "network": WATER}, completed({"networks": [ROADS, WATER], "revision": "$current"}), {**WRITTEN, "networks": [ROADS, WATER]})],
    })
    roads2 = {**ROADS, "tolerance": 0.05, "closed": "durum = 'kapalı'"}
    out.append({
        "name": "Değiştir: aynı kimlikli ağ yerinde değişir",
        "steps": [
            step("execute", {"operation": "set", "network": WATER}, completed({"networks": [ROADS, WATER], "revision": "$current"})),
            step("execute", {"operation": "set", "network": roads2}, completed({"networks": [roads2, WATER], "revision": "$current"}), {**WRITTEN, "networks": [roads2, WATER]}),
        ],
    })
    out.append({
        "name": "Sil",
        "steps": [step("execute", {"operation": "remove", "id": "ag-1"}, completed({"networks": [], "revision": "$current"}), {**WRITTEN, "networks": []})],
    })
    lost = {**WATER, "edges": [{"layer": "yok"}], "junctions": [{"layer": "vana", "role": "valve"}, {"layer": "eski", "role": "source"}]}
    out.append({
        "name": "Çizimde olmayan katmanlar: ağ yazılır, her katman bir uyarıyla",
        "steps": [step("execute", {"operation": "set", "network": lost}, completed({"networks": [ROADS, lost], "revision": "$current"}, [unknown_layer("yok"), unknown_layer("eski")]), {**WRITTEN, "networks": [ROADS, lost]})],
    })
    out.append({
        "name": "Doğrula: hiçbir şey yazılmaz, uyarılar söylenir",
        "steps": [step("validate", {"operation": "set", "network": lost}, {"status": "completed", "output": None, "warnings": [unknown_layer("yok"), unknown_layer("eski")]}, {**UNCHANGED, "networks": [ROADS]})],
    })
    out.append({
        "name": "Plan: yazılacak liste ve şimdiki sürüm; hiçbir şey yazılmaz",
        "steps": [step("plan", {"operation": "set", "network": WATER}, completed({"networks": [ROADS, WATER], "revision": "$current"}), {**UNCHANGED, "networks": [ROADS]})],
    })
    full = {
        "id": "ag-3",
        "name": "Bisiklet ve servis yolları",
        "kind": "road",
        "edges": [{"layer": "yol", "filter": "bisiklet = 'evet'"}, {"layer": "su", "filter": "servis_yolu = 'evet'"}],
        "junctions": [{"layer": "vana", "role": "junction", "filter": "tur = 'kavsak'"}],
        "connect": "vertices",
        "tolerance": 0.5,
        "direction": {"kind": "field", "field": "bisiklet_yon", "forward": ["ileri"], "backward": [], "closed": []},
        "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 15.0}, {"name": "Eğim", "kind": "field", "field": "egim_cezasi", "unit": "puan"}],
        "closed": "durum = 'kapalı'",
    }
    out.append({
        "name": "Ekle: bütün alanlarıyla (süzgeçler, düğüm katmanı, köşelerde bağlanma, boş yön listeleri, iki maliyet, kapalı kenarlar)",
        "steps": [step("execute", {"operation": "set", "network": full}, completed({"networks": [ROADS, full], "revision": "$current"}), {**WRITTEN, "networks": [ROADS, full]})],
    })
    out.append({
        "name": "Plan: silmenin listesi; hiçbir şey yazılmaz",
        "steps": [step("plan", {"operation": "remove", "id": "ag-1"}, completed({"networks": [], "revision": "$current"}), {**UNCHANGED, "networks": [ROADS]})],
    })
    out.append(refused("ağsız yazma", {"operation": "set"}, "no_network", "Yazılacak ağ (network) verilmedi; ağın tanımını verin.", "network"))
    out.append(refused("kimliksiz silme", {"operation": "remove"}, "no_id", "Silinecek ağın kimliğini (id) verin.", "id"))
    out.append(refused("boş kimlikle silme", {"operation": "remove", "id": ""}, "no_id", "Silinecek ağın kimliğini (id) verin.", "id"))
    for title in ("kimlik büyük harf", "tolerans 10 m'den büyük", "yön değeri yok", "yön değeri iki listede", "maliyetin adı UZUNLUK", "süre maliyetinin birimi"):
        value, words = rule_words(title)
        out.append(refused(f"kuralına uymayan ağ: {title}", {"operation": "set", "network": value}, "invalid_network", words, "network"))
    twin = {**WATER, "name": "YOLLAR"}
    out.append(refused("aynı adlı ikinci ağ (büyük küçük harf ayrımı yok)", {"operation": "set", "network": twin}, "invalid_network", "“YOLLAR” adlı ağ iki kez var", "network"))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "set", "network": WATER, "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"operation": "set", "network": WATER}, completed({"networks": [ROADS, WATER], "revision": "$current"}), {"revision": "changed"}),
            step("execute", {"operation": "remove", "id": "ag-1", "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "networks": [ROADS, WATER]}),
        ],
    })
    out.append(refused("projede olmayan ağı silme", {"operation": "remove", "id": "ag-9"}, "unknown_network", "“ag-9” kimlikli ağ projede yok. Var olan bir ağın kimliğini verin.", "id"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.network.define",
        "commandVersion": 1,
        "title": "Ağ tanımı: projenin ağını yaz ya da sil (ADR 0209 §11)",
        "note": "scripts/fixtures/network_define_command_cases.py yazar; beklentiler sözleşmenin kurallarından (crates/shared/contracts/src/cad_networks.rs) KentOS kodu olmadan kurulur, kuralı bozan ağların sözleri fixtures/network/v1/rules.json'dan. `expect.networks` projenin ağlarıdır. Ağ proje ayarıdır: yazmak çizimi kirli yapar, geri alma adımı yazmaz.",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"cad.network.define durumları güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
