"""Writes fixtures/commands/v1/cad.linearref.define.json: the shared cases of `cad.linearref.define` v1 (docs/adr/0569
§2), their expectations built here from the contract's rules (crates/shared/commands/src/cad_linearref.rs and
crates/shared/contracts/src/linear_events.rs: the checks and their order, the codes, the paths and the words), without
KentOS code. The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py)
run them.

    python3 scripts/fixtures/linearref_define_command_cases.py           # writes the file
    python3 scripts/fixtures/linearref_define_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.linearref.define.json"

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def node(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": dict(STYLE), "children": []}


POINTS = {
    "id": "kazalar", "name": "Kazalar", "events": "kaza", "kind": "point", "routes": ["yol"],
    "routeField": "Yol", "eventRoute": "Yol", "measure": "Km",
}
LINES = {
    "id": "kesimler", "name": "Kaplama kesimleri", "events": "kesim", "kind": "line", "routes": ["yol", "yol-eski"],
    "routeField": "Yol", "eventRoute": "Yol", "from": "Bas", "to": "Son", "offset": "Sapma", "side": "left", "tolerance": 0.01,
}

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Olay bağları",
    "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "linearEvents": [copy.deepcopy(POINTS)]},
    "origin": {"x": 500000, "y": 4400000},
    "layers": [node("yol", "Yol"), node("yol-eski", "Eski yol"), node("kaza", "Kaza"), node("kesim", "Kesim"), node("0", "0")],
    "activeLayer": "0",
    "entities": [{"kind": "point", "id": 1, "layerId": "kaza", "attrs": {"Yol": "D-100", "Km": "4"}, "p": {"x": 500004, "y": 4400000}}],
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
# A link is a project setting: the drawing is dirty, its revision changes, and no undo step is written.
WRITTEN = {"ids": [1], "canUndo": False, "canRedo": False, "dirty": True, "revision": "changed"}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "linearEvents": [POINTS]})]}


def unknown_layer(layer):
    return {"code": "unknown_layer", "message": f"“{layer}” kimlikli katman çizimde yok; bağ bu katman gelene dek bir şey yapmaz.", "path": "link"}


def rule(title, value, words):
    return refused(f"kuralına uymayan bağ: {title}", {"operation": "set", "link": value}, "invalid_event_link", words, "link")


def cases():
    out = []
    out.append({
        "name": "Ekle: yeni bağ sona; proje ayarıdır, geri alma adımı yazılmaz",
        "steps": [step("execute", {"operation": "set", "link": LINES}, completed({"links": [POINTS, LINES], "revision": "$current"}), {**WRITTEN, "linearEvents": [POINTS, LINES]})],
    })
    points2 = {**POINTS, "offset": "Sapma", "tolerance": 0.5}
    out.append({
        "name": "Değiştir: aynı kimlikli bağ yerinde değişir",
        "steps": [
            step("execute", {"operation": "set", "link": LINES}, completed({"links": [POINTS, LINES], "revision": "$current"})),
            step("execute", {"operation": "set", "link": points2}, completed({"links": [points2, LINES], "revision": "$current"}), {**WRITTEN, "linearEvents": [points2, LINES]}),
        ],
    })
    out.append({
        "name": "Sil: olaylar olduğu yerde kalır",
        "steps": [step("execute", {"operation": "remove", "id": "kazalar"}, completed({"links": [], "revision": "$current"}), {**WRITTEN, "linearEvents": [], "ids": [1]})],
    })
    lost = {**LINES, "events": "yok", "routes": ["yol", "eski"]}
    out.append({
        "name": "Çizimde olmayan katmanlar: bağ yazılır, her katman bir uyarıyla (olay katmanı, sonra rotalar)",
        "steps": [step("execute", {"operation": "set", "link": lost}, completed({"links": [POINTS, lost], "revision": "$current"}, [unknown_layer("yok"), unknown_layer("eski")]), {**WRITTEN, "linearEvents": [POINTS, lost]})],
    })
    out.append({
        "name": "Doğrula: hiçbir şey yazılmaz, uyarılar söylenir",
        "steps": [step("validate", {"operation": "set", "link": lost}, {"status": "completed", "output": None, "warnings": [unknown_layer("yok"), unknown_layer("eski")]}, {**UNCHANGED, "linearEvents": [POINTS]})],
    })
    out.append({
        "name": "Plan: yazılacak liste ve şimdiki sürüm; hiçbir şey yazılmaz",
        "steps": [step("plan", {"operation": "set", "link": LINES}, completed({"links": [POINTS, LINES], "revision": "$current"}), {**UNCHANGED, "linearEvents": [POINTS]})],
    })
    out.append({
        "name": "Plan: silmenin listesi; hiçbir şey yazılmaz",
        "steps": [step("plan", {"operation": "remove", "id": "kazalar"}, completed({"links": [], "revision": "$current"}), {**UNCHANGED, "linearEvents": [POINTS]})],
    })
    out.append({
        "name": "Ekle: dört alanlı çizgi bağı, öteleme, sol yön ve sıfır tolerans",
        "steps": [step("execute", {"operation": "set", "link": {**LINES, "tolerance": 0.0}}, completed({"links": [POINTS, {**LINES, "tolerance": 0.0}], "revision": "$current"}), {**WRITTEN, "linearEvents": [POINTS, {**LINES, "tolerance": 0.0}]})],
    })
    many = {**LINES, "routes": [f"r{i}" for i in range(16)]}
    out.append({
        "name": "Ekle: en çok on altı rota katmanı (çizimde olmayanlar uyarıyla)",
        "steps": [step("execute", {"operation": "set", "link": many}, completed({"links": [POINTS, many], "revision": "$current"}, [unknown_layer(f"r{i}") for i in range(16)]), {**WRITTEN, "linearEvents": [POINTS, many]})],
    })
    out.append(refused("bağsız yazma", {"operation": "set"}, "no_event_link", "Yazılacak olay bağı (link) verilmedi; bağın tanımını verin.", "link"))
    out.append(refused("kimliksiz silme", {"operation": "remove"}, "no_id", "Silinecek olay bağının kimliğini (id) verin.", "id"))
    out.append(refused("boş kimlikle silme", {"operation": "remove", "id": ""}, "no_id", "Silinecek olay bağının kimliğini (id) verin.", "id"))
    out.append(rule("kimlikte boşluk", {**LINES, "id": "kaplama kesimi"}, "“kaplama kesimi” bir bağ kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı."))
    out.append(rule("adı boşlukla başlıyor", {**LINES, "name": " Kesimler"}, "“kesimler” bağının adı boş, baş ya da sonda boşluklu, 64 karakterden uzun ya da denetim karakterli."))
    out.append(rule("olay katmanı yazılmamış", {**LINES, "events": ""}, "“Kaplama kesimleri” bağının olay katmanı yazılmamış."))
    out.append(rule("rota katmanı yok", {**LINES, "routes": []}, "“Kaplama kesimleri” bağının 1–16 rota katmanı olmalı; 0 verildi."))
    out.append(rule("on yedi rota katmanı", {**LINES, "routes": [f"r{i}" for i in range(17)]}, "“Kaplama kesimleri” bağının 1–16 rota katmanı olmalı; 17 verildi."))
    out.append(rule("rota katmanlarından biri boş", {**LINES, "routes": ["yol", ""]}, "“Kaplama kesimleri” bağının rota katmanlarından biri boş."))
    out.append(rule("olay katmanı rota katmanı da", {**LINES, "routes": ["yol", "kesim"]}, "“Kaplama kesimleri” bağında olay katmanı rota katmanı da olamaz."))
    out.append(rule("rota katmanı iki kez", {**LINES, "routes": ["yol", "yol"]}, "“Kaplama kesimleri” bağında “yol” rota katmanı iki kez yazılmış."))
    out.append(rule("rota kimliği alanı boş", {**LINES, "routeField": ""}, "“Kaplama kesimleri” bağının rota kimliği alanı boş."))
    out.append(rule("olayın rota alanı boş", {**LINES, "eventRoute": ""}, "“Kaplama kesimleri” bağının olayın rota alanı boş."))
    out.append(rule("öteleme alanı boş", {**LINES, "offset": ""}, "“Kaplama kesimleri” bağının öteleme alanı boş."))
    out.append(rule("nokta bağında ölçü alanı yok", {k: v for k, v in POINTS.items() if k != "measure"}, "“Kazalar” nokta bağının ölçü alanı (measure) yazılmamış."))
    out.append(rule("nokta bağında ölçü alanı boş", {**POINTS, "measure": ""}, "“Kazalar” bağının ölçü alanı boş."))
    out.append(rule("nokta bağında başlangıç alanı", {**POINTS, "from": "Bas"}, "“Kazalar” nokta bağının başlangıç ve bitiş alanı olmaz; ölçü alanı verin."))
    out.append(rule("çizgi bağında bitiş alanı yok", {k: v for k, v in LINES.items() if k != "to"}, "“Kaplama kesimleri” çizgi bağının başlangıç (from) ve bitiş (to) ölçüsü alanları yazılmalı."))
    out.append(rule("çizgi bağında tek ölçü alanı", {**LINES, "measure": "Km"}, "“Kaplama kesimleri” çizgi bağının tek ölçü alanı olmaz; başlangıç ve bitiş alanlarını verin."))
    out.append(rule("eksi tolerans", {**LINES, "tolerance": -0.5}, "“Kaplama kesimleri” bağının toleransı sıfır ya da daha büyük bir sayı olmalı."))
    other = {**LINES, "id": "ikinci", "name": "İkinci", "events": "kaza"}
    out.append(rule("başka bağın olay katmanı", other, "“kaza” katmanının olayları iki bağda; bir katman bir bağın olay katmanıdır."))
    feeds = {**LINES, "id": "besleyen", "name": "Besleyen", "events": "kesim", "routes": ["kaza"]}
    out.append(rule("başka bağın olay katmanı bu bağın rotası", feeds, "“kaza” katmanı “Kazalar” bağının olay katmanı ve “Besleyen” bağının rota katmanı; ikisi birden olamaz."))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "set", "link": LINES, "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"operation": "set", "link": LINES}, completed({"links": [POINTS, LINES], "revision": "$current"}), {"revision": "changed"}),
            step("execute", {"operation": "remove", "id": "kazalar", "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "linearEvents": [POINTS, LINES]}),
        ],
    })
    out.append(refused("projede olmayan bağı silme", {"operation": "remove", "id": "yok"}, "unknown_event_link", "“yok” kimlikli olay bağı projede yok. Var olan bir bağın kimliğini verin.", "id"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.linearref.define",
        "commandVersion": 1,
        "title": "Olay bağı: bir olay katmanının rotalarına bağını yaz ya da sil (ADR 0569 §2)",
        "note": "scripts/fixtures/linearref_define_command_cases.py yazar; beklentiler sözleşmenin kurallarından (crates/shared/commands/src/cad_linearref.rs, crates/shared/contracts/src/linear_events.rs) KentOS kodu olmadan kurulur. `expect.linearEvents` projenin olay bağlarıdır. Olay bağı proje ayarıdır: yazmak çizimi kirli yapar, geri alma adımı yazmaz ve olayları kendiliğinden taşımaz; bağdan sonraki yazmalar taşır (cad.linearref.follow.json).",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"cad.linearref.define durumları güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
