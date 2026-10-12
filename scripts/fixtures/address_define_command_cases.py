"""Writes fixtures/commands/v1/cad.addresses.define.json: the shared cases of `cad.addresses.define` v1 (docs/adr/0229
§14), their expectations built here from the contract's rules (crates/shared/contracts/src/cad_addresses.rs: the checks
and their order, the codes, the paths and the words), without KentOS code. A locator that breaks a rule takes its words
from fixtures/geocode/v1/rules.json (the contract's, held by Rust). The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/address_define_command_cases.py           # writes the file
    python3 scripts/fixtures/address_define_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.addresses.define.json"
RULES = json.loads((ROOT / "fixtures/geocode/v1/rules.json").read_text(encoding="utf-8"))

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def node(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": dict(STYLE), "children": []}


DOORS = {
    "id": "adres-1",
    "name": "Numarataj",
    "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO", "neighbourhood": "MAHALLE"},
    "minScore": 85,
    "reach": 100.0,
}
STREETS = {
    "id": "adres-2",
    "name": "Yol orta hatları",
    "streets": {"layer": "yol", "name": "YOL_ADI"},
    "areas": [{"layer": "mahalle", "level": "neighbourhood", "name": "AD", "district": "ILCE"}],
    "minScore": 70,
    "reach": 250.0,
}

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Adresler",
    "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "addressLocators": [copy.deepcopy(DOORS)]},
    "origin": {"x": 500000, "y": 4400000},
    "layers": [node("kapi", "Kapı"), node("yol", "Yol orta hattı"), node("mahalle", "Mahalle"), node("0", "0")],
    "activeLayer": "0",
    "entities": [{"kind": "point", "id": 1, "layerId": "kapi", "attrs": {"YOL_ADI": "Atatürk Bulvarı", "KAPI_NO": "12", "MAHALLE": "Kızılay"}, "p": {"x": 500010, "y": 4400005}}],
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
# A locator is a project setting: the drawing is dirty, its revision changes, and no undo step is written.
WRITTEN = {"ids": [1], "canUndo": False, "canRedo": False, "dirty": True, "revision": "changed"}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "addressLocators": [DOORS]})]}


def rule_words(title):
    case = next(c for c in RULES["locators"] if c["name"] == title)
    return case["value"], case["problem"]


def unknown_layer(layer):
    return {"code": "unknown_layer", "message": f"“{layer}” kimlikli katman çizimde yok; adres dizini kurulurken bu katman atlanır.", "path": "locator"}


def cases():
    out = []
    out.append({
        "name": "Ekle: yeni tanım sona; proje ayarıdır, geri alma adımı yazılmaz",
        "steps": [step("execute", {"operation": "set", "locator": STREETS}, completed({"locators": [DOORS, STREETS], "revision": "$current"}), {**WRITTEN, "addressLocators": [DOORS, STREETS]})],
    })
    doors2 = {**DOORS, "minScore": 90, "doors": {**DOORS["doors"], "code": "UAVT"}}
    out.append({
        "name": "Değiştir: aynı kimlikli tanım yerinde değişir",
        "steps": [
            step("execute", {"operation": "set", "locator": STREETS}, completed({"locators": [DOORS, STREETS], "revision": "$current"})),
            step("execute", {"operation": "set", "locator": doors2}, completed({"locators": [doors2, STREETS], "revision": "$current"}), {**WRITTEN, "addressLocators": [doors2, STREETS]}),
        ],
    })
    out.append({
        "name": "Sil",
        "steps": [step("execute", {"operation": "remove", "id": "adres-1"}, completed({"locators": [], "revision": "$current"}), {**WRITTEN, "addressLocators": []})],
    })
    lost = {**STREETS, "streets": {"layer": "yok", "name": "AD"}, "areas": [{"layer": "mahalle", "level": "neighbourhood", "name": "AD"}, {"layer": "eski", "level": "district", "name": "AD"}]}
    out.append({
        "name": "Çizimde olmayan katmanlar: tanım yazılır, her katman bir uyarıyla",
        "steps": [step("execute", {"operation": "set", "locator": lost}, completed({"locators": [DOORS, lost], "revision": "$current"}, [unknown_layer("yok"), unknown_layer("eski")]), {**WRITTEN, "addressLocators": [DOORS, lost]})],
    })
    out.append({
        "name": "Doğrula: hiçbir şey yazılmaz, uyarılar söylenir",
        "steps": [step("validate", {"operation": "set", "locator": lost}, {"status": "completed", "output": None, "warnings": [unknown_layer("yok"), unknown_layer("eski")]}, {**UNCHANGED, "addressLocators": [DOORS]})],
    })
    out.append({
        "name": "Plan: yazılacak liste ve şimdiki sürüm; hiçbir şey yazılmaz",
        "steps": [step("plan", {"operation": "set", "locator": STREETS}, completed({"locators": [DOORS, STREETS], "revision": "$current"}), {**UNCHANGED, "addressLocators": [DOORS]})],
    })
    full = {
        "id": "adres-3",
        "name": "Bütün alanlarıyla",
        "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO", "neighbourhood": "MAHALLE", "district": "ILCE", "province": "IL", "postcode": "POSTA_KODU", "building": "BINA_ADI", "code": "UAVT"},
        "streets": {"layer": "yol", "name": "YOL_ADI", "neighbourhood": "MAHALLE", "district": "ILCE"},
        "areas": [{"layer": "mahalle", "level": "neighbourhood", "name": "AD", "district": "ILCE"}, {"layer": "mahalle", "level": "district", "name": "ILCE_ADI"}, {"layer": "mahalle", "level": "province", "name": "IL_ADI"}],
        "minScore": 50,
        "reach": 10000.0,
    }
    out.append({
        "name": "Ekle: bütün alanlarıyla (kapı, yol, üç düzeyde bölge; en düşük puan 50, en büyük uzaklık)",
        "steps": [step("execute", {"operation": "set", "locator": full}, completed({"locators": [DOORS, full], "revision": "$current"}), {**WRITTEN, "addressLocators": [DOORS, full]})],
    })
    out.append({
        "name": "Plan: silmenin listesi; hiçbir şey yazılmaz",
        "steps": [step("plan", {"operation": "remove", "id": "adres-1"}, completed({"locators": [], "revision": "$current"}), {**UNCHANGED, "addressLocators": [DOORS]})],
    })
    out.append(refused("tanımsız yazma", {"operation": "set"}, "no_locator", "Yazılacak adres verisi (locator) verilmedi; tanımını verin.", "locator"))
    out.append(refused("kimliksiz silme", {"operation": "remove"}, "no_id", "Silinecek adres verisinin kimliğini (id) verin.", "id"))
    out.append(refused("boş kimlikle silme", {"operation": "remove", "id": ""}, "no_id", "Silinecek adres verisinin kimliğini (id) verin.", "id"))
    for title in ("kimlik büyük harf", "kaynağı yok", "kapıların kapı alanı kırpılmamış", "aynı düzey iki kez", "ilçe alanlarında ilçe", "en düşük puan 49", "uzaklık sıfır"):
        value, words = rule_words(title)
        out.append(refused(f"kuralına uymayan tanım: {title}", {"operation": "set", "locator": value}, "invalid_locator", words, "locator"))
    twin = {**STREETS, "name": "NUMARATAJ"}
    out.append(refused("aynı adlı ikinci tanım (büyük küçük harf ayrımı yok)", {"operation": "set", "locator": twin}, "invalid_locator", "“NUMARATAJ” adlı adres verisi iki kez var", "locator"))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "set", "locator": STREETS, "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"operation": "set", "locator": STREETS}, completed({"locators": [DOORS, STREETS], "revision": "$current"}), {"revision": "changed"}),
            step("execute", {"operation": "remove", "id": "adres-1", "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "addressLocators": [DOORS, STREETS]}),
        ],
    })
    out.append(refused("projede olmayan tanımı silme", {"operation": "remove", "id": "adres-9"}, "unknown_locator", "“adres-9” kimlikli adres verisi projede yok. Var olan bir tanımın kimliğini verin.", "id"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.addresses.define",
        "commandVersion": 1,
        "title": "Adres verisi: projenin adres verisini yaz ya da sil (ADR 0229 §14)",
        "note": "scripts/fixtures/address_define_command_cases.py yazar; beklentiler sözleşmenin kurallarından (crates/shared/contracts/src/cad_addresses.rs) KentOS kodu olmadan kurulur, kuralı bozan tanımların sözleri fixtures/geocode/v1/rules.json'dan. `expect.addressLocators` projenin adres verileridir. Adres verisi proje ayarıdır: yazmak çizimi kirli yapar, geri alma adımı yazmaz.",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"cad.addresses.define durumları güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
