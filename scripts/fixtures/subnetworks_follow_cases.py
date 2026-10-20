#!/usr/bin/env python3
"""Writes fixtures/commands/v1/cad.subnetworks.follow.json: the shared cases of a network that keeps its subnetwork
names current (docs/adr/0562 §3), their expectations built here from the ADR's rules, without KentOS code.

A write by any command that touches objects of one of the network's layers (added, removed or changed one) is
followed, in its own step, by the rewriting of the subnetwork attribute where it is not what the subnetworks say, the
way `cad.subnetworks.update` writes it; where the attribute cannot be brought up to date (the network has no source in
the drawing, a layer is locked or lacks the attribute's field, a value breaks the field's rules) the answer carries
`subnetwork_dirty`. The reference reuses the document, the subnetworks and the update's expectations of
subnetworks_command_cases.py (which reads the very document a case sets up), applies the user's write to the document
itself (an attribute, a new pipe, a deletion, a move, a layer) and takes the update's answer on the result. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/subnetworks_follow_cases.py           # writes the file
    python3 scripts/fixtures/subnetworks_follow_cases.py --check   # compares it with the one on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
_spec = importlib.util.spec_from_file_location("subnetworks_command_cases", ROOT / "scripts/fixtures/subnetworks_command_cases.py")
base = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(base)

ATTRIBUTE = base.ATTRIBUTE
NETWORK_LAYERS = ("boru", "hat2", "kilitli", "kaynak", "vana", "trafo")
NO_CONTROLLERS = "“Elektrik” ağının çizimde Kaynak düğümü yok; alt ağ denetleyicisi bulunamadı."


def dirty(why):
    return {"code": "subnetwork_dirty",
            "message": f"“Elektrik” ağı alt ağ adlarını güncel tutuyor ama bu yazma onları güncelleyemedi: {why} Alt ağları güncelle ile yenileyin."}


def has_controllers(setup):
    net = setup["settings"]["networks"][0]
    sources = {layer["layer"] for layer in net["junctions"] if layer["role"] == "source"}
    return any(e["layerId"] in sources and e["kind"] == "point" for e in setup["entities"])


def synced(setup):
    """The drawing with the update's writes applied: the subnetwork attribute right wherever it can be written."""
    s = copy.deepcopy(setup)
    _, _, writes, _ = base.update_answer(s)
    by_id = {e["id"]: e for e in s["entities"]}
    for i, want in writes:
        if want is None:
            by_id[i]["attrs"].pop(ATTRIBUTE, None)
        else:
            by_id[i]["attrs"][ATTRIBUTE] = want
    return s


def apply_change(setup, change):
    """The drawing after the user's write, the command and its input, the layers it touched and the objects whose
    attributes it set (id → attrs after) or that it added."""
    s = copy.deepcopy(setup)
    by_id = {e["id"]: e for e in s["entities"]}
    kind = change[0]
    if kind == "attrs":
        _, i, attrs = change
        for k, v in attrs.items():
            if v is None:
                by_id[i]["attrs"].pop(k, None)
            else:
                by_id[i]["attrs"][k] = v
        return s, "cad.entities.set", {"uids": [f"$uidOf:{i}"], "attrs": attrs, "operation": "attributes"}, [by_id[i]["layerId"]], {i}
    if kind == "line":
        _, layer, a, b = change
        new = max(by_id) + 1
        s["entities"].append(base.line(new, layer, a, b))
        input_ = {"layerId": layer, "a": {"x": base.OX + a[0], "y": base.OY + a[1]}, "b": {"x": base.OX + b[0], "y": base.OY + b[1]}}
        return s, "cad.line.create", input_, [layer], {new}
    if kind == "delete":
        _, i = change
        layer = by_id[i]["layerId"]
        s["entities"] = [e for e in s["entities"] if e["id"] != i]
        return s, "cad.entities.delete", {"uids": [f"$uidOf:{i}"]}, [layer], set()
    if kind == "move":
        _, i, dx, dy = change
        e = by_id[i]
        for key in ("a", "b", "p"):
            if key in e:
                e[key] = {"x": e[key]["x"] + dx, "y": e[key]["y"] + dy}
        return s, "cad.entities.transform", {"uids": [f"$uidOf:{i}"], "transform": {"kind": "move", "dx": dx, "dy": dy}}, [e["layerId"]], set()
    if kind == "layer":
        _, i, layer = change
        old = by_id[i]["layerId"]
        by_id[i]["layerId"] = layer
        return s, "cad.entities.set", {"uids": [f"$uidOf:{i}"], "layerId": layer, "operation": "layer"}, [old, layer], set()
    raise ValueError(kind)


def after_follow(after, runs):
    """The update's writes on the drawing after the user's write, its warnings and the drawing with them applied."""
    if not runs:
        return [], [], after
    if not has_controllers(after):
        return [dirty(NO_CONTROLLERS)], [], after
    _, warnings, writes, _ = base.update_answer(after)
    skipped = base.update_answer(after)[0]["skipped"]
    out = copy.deepcopy(after)
    by_id = {e["id"]: e for e in out["entities"]}
    for i, want in writes:
        if want is None:
            by_id[i]["attrs"].pop(ATTRIBUTE, None)
        else:
            by_id[i]["attrs"][ATTRIBUTE] = want
    said = list(warnings)
    if skipped:
        said.append(dirty(f"{skipped} nesnenin değeri yazılmadı."))
    return said, writes, out


def case(name, setup, change, undo=False, redo=False):
    after, command, input_, layers, touched = apply_change(setup, change)
    net = after["settings"]["networks"][0]
    keep = bool((net.get("subnetworks") or {}).get("keepCurrent"))
    runs = keep and any(layer in NETWORK_LAYERS for layer in layers)
    warnings, writes, done = after_follow(after, runs)
    now = {e["id"]: e for e in done["entities"]}
    fields = {str(i): {"attrs": now[i]["attrs"]} for i in sorted({i for i, _ in writes} | {t for t in touched if t in now})}
    expect = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "fields": fields}
    steps = [{"op": "execute", "command": command, "input": input_,
              "result": {"status": "completed", "output": "$any", "warnings": warnings}, "expect": expect}]
    if undo or redo:
        before = {e["id"]: e for e in setup["entities"]}
        undone = {"canUndo": False, "canRedo": True,
                  "fields": {str(i): {"attrs": before[i]["attrs"]} for i in sorted(set(fields_ids(fields)) & set(before))}}
        if change[0] == "line":
            undone["ids"] = sorted(before)
        steps.append({"op": "undo", "expect": undone})
        if redo:
            steps.append({"op": "redo", "expect": {"canUndo": True, "canRedo": False, "fields": fields}})
    return {"name": name, "setup": setup, "steps": steps}


def fields_ids(fields):
    return [int(i) for i in fields]


def cases():
    out = []
    # Make the locked layer a clean one for the cases that are not about it.
    clean = base.scene(keep=True, hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}])
    clean["layers"][2]["locked"] = False
    clean = synced(clean)
    out.append(case("vananın kapanması: iki alt ağın erişimi değişir, adlar aynı adımda yazılır", clean, ("attrs", 10, {"durum": "kapalı"}), undo=True, redo=True))
    out.append(case("kapalı vananın açılması", synced(base.scene(keep=True, valve="kapalı", hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}], preset=None)), ("attrs", 10, {"durum": "açık"})))
    out.append(case("yeni boru ortak vanaya bağlanır: iki adı alır", clean, ("line", "boru", (300, 0), (350, 0)), undo=True))
    out.append(case("yeni boru kaynaksız yerde: öznitelik yazılmaz", clean, ("line", "boru", (900, 100), (950, 100))))
    out.append(case("yeni boru A'nın ucuna bağlanır (Merkez A)", clean, ("line", "boru", (200, 0), (200, -50))))
    out.append(case("ağın katmanı olmayan katmana yazma ağı izletmez", clean, ("line", "0", (0, 0), (10, 10))))
    stale = base.scene(keep=True, hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}])
    stale["layers"][2]["locked"] = False
    out.append(case("ağın katmanı olmayan katmana yazma eski öznitelikleri düzeltmez", stale, ("line", "0", (0, 0), (10, 10))))
    out.append(case("ağın bir katmanına yazma bütün ağı düzeltir (eski öznitelikler de)", stale, ("line", "boru", (900, 100), (950, 100))))
    out.append(case("boru silinir: ötesi alt ağdan çıkar, adı kalkar", clean, ("delete", 5), undo=True))
    out.append(case("kaynak silinir: onun ağının nesneleri adsız kalır", clean, ("delete", 2)))
    out.append(case("kaynaksız boru taşınıp ağa bağlanır", clean, ("move", 8, -300.0, 0.0)))
    out.append(case("kaynak ağdan uzaklaştırılır", clean, ("move", 1, 900.0, 900.0), undo=True))
    out.append(case("boru ağın katmanından çıkarılır: ağdan çıkar, değeri olduğu gibi kalır", clean, ("layer", 8, "0")))
    outside = copy.deepcopy(clean)
    next(e for e in outside["entities"] if e["id"] == 8)["layerId"] = "0"
    out.append(case("katman değişimi ağa girer: öbür katmandan boru ağa alınır", synced(outside), ("layer", 8, "boru"), undo=True))
    out.append(case("alt ağ özniteliğini elle değiştirme aynı adımda geri düzeltilir", clean, ("attrs", 5, {ATTRIBUTE: "Elle"})))
    out.append(case("alt ağ özniteliğini elle silme geri yazılır", clean, ("attrs", 3, {ATTRIBUTE: None})))
    # The device's terminals by an attribute change what a write of the attribute does.
    cfg = base.with_tip("ters", keep=True, hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}])
    cfg["layers"][2]["locked"] = False
    cfg = synced(cfg)
    out.append(case("cihazın yapılandırma değeri değişir: erişim cihazın yeni terminallerine göre", cfg, ("attrs", 9, {"tip": "üçlü"}), undo=True, redo=True))
    out.append(case("yapılandırma değeri silinir: cihazın kendi terminalleri", cfg, ("attrs", 9, {"tip": None})))
    out.append(case("yapılandırmasız kenar yeniden bağlanır", cfg, ("attrs", 3, {"uc_son": "x"})))
    # What cannot be brought up to date is said.
    locked = synced(base.scene(keep=True, hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}]))
    out.append(case("kilitli katman: adı yazılamaz, her yazma bunu söyler (subnetwork_dirty)", locked, ("attrs", 10, {"durum": "kapalı"})))
    nofield = synced(base.scene(keep=True))
    nofield["layers"][2]["locked"] = False
    out.append(case("özniteliğin alanı olmayan katman atlanır ve söylenir", nofield, ("line", "boru", (300, 0), (350, 0))))
    short = base.scene(keep=True, hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text", "length": 5}])
    short["layers"][2]["locked"] = False
    out.append(case("alanın uzunluğunu aşan ad yazılmaz ve söylenir", synced(short), ("line", "boru", (300, 0), (350, 0))))
    nosource = base.scene(keep=True, sources=False, hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}])
    nosource["layers"][2]["locked"] = False
    out.append(case("çizimde Kaynak düğümü yok: ağ kurulamaz, yazma yine de olur ve söylenir", nosource, ("line", "boru", (300, 0), (350, 0))))
    # A network that does not keep its names current is left as it is.
    manual = base.scene(hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}])
    manual["layers"][2]["locked"] = False
    out.append(case("güncel tutmayan ağ: yazma yalnız kendi işini yapar", synced(manual), ("attrs", 10, {"durum": "kapalı"})))
    out.append(case("güncel tutmayan ağ: yeni boru adsız kalır", synced(manual), ("line", "boru", (300, 0), (350, 0))))
    # Two writes in a row are two steps, each followed.
    again = copy.deepcopy(clean)
    first = case("ilk yazma", clean, ("line", "boru", (300, 0), (350, 0)))
    after_first, _, _, _, _ = apply_change(clean, ("line", "boru", (300, 0), (350, 0)))
    _, _, done_first = after_follow(after_first, True)
    second_case = case("ikinci yazma", done_first, ("attrs", 10, {"durum": "kapalı"}))
    steps = first["steps"] + [dict(second_case["steps"][0], expect={**second_case["steps"][0]["expect"]})]
    out.append({"name": "art arda iki yazma: her biri kendi adımında izlenir", "setup": again, "steps": steps})
    return out


def other_cases():
    """Cases of what is not followed: the update itself, a refused write, a command that touches no object."""
    out = []
    keep = base.scene(keep=True, hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text"}])
    keep["layers"][2]["locked"] = False
    answer, warnings, writes, entity = base.update_answer(keep)
    expect = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "fields": base.entity_attrs(entity, writes, ATTRIBUTE)}
    out.append({"name": "güncelle komutunun kendisi izlenmez: kendi uyarılarından başkası yok", "setup": keep,
                "steps": [{"op": "execute", "input": {"network": "ag-1"}, "result": base.completed(answer, warnings), "expect": expect}]})
    out.append({"name": "reddedilen yazma izlenmez: hiçbir şey yazılmaz", "setup": keep, "steps": [
        {"op": "execute", "command": "cad.entities.set",
         "input": {"uids": [], "attrs": {"durum": "kapalı"}, "operation": "attributes"},
         "result": {"status": "failed", "error": "$any"}, "expect": base.UNCHANGED}]})
    return out


def build():
    cs = cases() + other_cases()
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.subnetworks.update",
        "commandVersion": 1,
        "title": "Alt ağ adlarını güncel tutmak (ADR 0562 §3)",
        "note": "scripts/fixtures/subnetworks_follow_cases.py yazar; beklentiler ADR 0562'nin kurallarından, KentOS kodu olmadan (alt ağlar ve güncelleme subnetworks_command_cases.py'nin başvurusuyla). Her durum kendi çizimini kurar.",
        "setup": cs[0]["setup"],
        "cases": cs,
    }


def main():
    path = ROOT / "fixtures/commands/v1/cad.subnetworks.follow.json"
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"Alt ağ güncel tutma durumları güncel: {len(json.loads(text)['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"{path.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
