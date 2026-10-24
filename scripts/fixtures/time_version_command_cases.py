#!/usr/bin/env python3
"""The shared cases of `cad.time.version` (Yeni sürüm oluştur and Sona erdir; docs/adr/0210 §7, 0622 §9).

    python3 scripts/fixtures/time_version_command_cases.py           # writes the file
    python3 scripts/fixtures/time_version_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.time.version.json with Python's standard library, no KentOS code, from ADR 0210 §7 and
the command's contract. A moment's text is read, written and shown by `scripts/fixtures/temporal_cases.py` (ADR 0210
§3's grammar), a typed field's value checked by `scripts/fixtures/layer_field_cases.py` (ADR 0199 §2):

- every object on a ranged temporal layer (a start and an end field), not locked, its dates readable; Yeni sürüm
  oluştur also copies it, which a record or an architectural object cannot be;
- the date read as the attributes are; on a layer whose start or end field is of the date kind it is that day's
  midnight; every object's start strictly before it and its end strictly after (a missing start or end: none);
- each object's end becomes the date, written as the layer writes it (`YYYY-MM-DD` at midnight, else with the time;
  a typed field's canonical text); Yeni sürüm oluştur writes beside each a copy with its geometry, colour, weight,
  attributes, label and symbol, starting at the date and ending where the object ended; the copies take the next slots
  layer by layer, in the order the layers are first met, and `created` lists them in the objects' order;
- one undo step named after the tool; a hidden layer's new versions are said;
- the refusals in the contract's order.
"""
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.time.version.json"
sys.path.insert(0, str(Path(__file__).resolve().parent))
os.environ["KENTOS_NO_MAIN"] = "1"
import layer_field_cases as fields  # noqa: E402  (ADR 0199 §2's values)
import temporal_cases as temporal  # noqa: E402  (ADR 0210 §3's moments)

SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 1000, "workspace": "gis"}
RANGED = {"start": "bas", "end": "bit"}


def layer(i, name, time=None, typed=None, locked=False, visible=True):
    node = {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    if time:
        node["time"] = time
    if typed:
        node["fields"] = typed
    return node


LAYERS = [
    layer("parsel", "Parseller", {"start": "bas", "end": "bit", "key": "no"}),
    layer("bina", "Binalar", RANGED, [{"name": "no", "kind": "text"}, {"name": "bas", "kind": "date"},
                                      {"name": "bit", "kind": "date"}]),
    layer("saatli", "Saatli", RANGED, [{"name": "bas", "kind": "datetime"}, {"name": "bit", "kind": "datetime"}]),
    layer("anlik", "Anlık", {"start": "tarih"}),
    layer("duz", "Düz"),
    layer("kilitli", "Kilitli", RANGED, locked=True),
    layer("gizli", "Gizli", RANGED, visible=False),
    layer("sayili", "Sayılı", RANGED, [{"name": "bit", "kind": "integer"}]),
    layer("yilli", "Yıllı", RANGED, [{"name": "bas", "kind": "integer"}]),
    layer("kayit", "Kayıtlar", RANGED),
]
NODES = {n["id"]: n for n in LAYERS}


def pt(x, y):
    return {"x": float(x), "y": float(y)}


def square(i, layer_id, attrs, x, **more):
    e = {"kind": "polygon", "id": i, "layerId": layer_id, "attrs": attrs,
         "pts": [pt(x, 4420000), pt(x + 10, 4420000), pt(x + 10, 4420010), pt(x, 4420010)]}
    e.update(more)
    return e


def point(i, layer_id, attrs, x):
    return {"kind": "point", "id": i, "layerId": layer_id, "attrs": attrs, "p": pt(x, 4420100)}


ENTITIES = [
    square(1, "parsel", {"no": "101", "bas": "2010-01-01", "bit": "2030-01-01"}, 487000, color="#c0392b",
           lineWeight=0.5, label="101"),
    square(2, "parsel", {"no": "102", "bas": "2012-06-01T08:00:00"}, 487020),
    point(3, "bina", {"no": "B1", "bas": "2005-03-01", "bit": "2040-12-31"}, 487000),
    point(4, "saatli", {"bas": "2015-01-01T00:00:00", "bit": "2025-01-01T00:00:00"}, 487010),
    point(5, "anlik", {"tarih": "2020-01-01"}, 487020),
    point(6, "duz", {}, 487030),
    point(7, "kilitli", {"bas": "2010-01-01", "bit": "2030-01-01"}, 487040),
    point(8, "gizli", {"bas": "2010-01-01", "bit": "2030-01-01"}, 487050),
    point(9, "sayili", {"bas": "2010-01-01"}, 487060),
    square(10, "parsel", {"no": "103", "bas": "eski", "bit": "2030-01-01"}, 487040),
    point(11, "yilli", {"bit": "2030-01-01"}, 487070),
    square(12, "parsel", {"no": "104", "bas": "2021-01-01", "bit": "2030-01-01"}, 487060),
    {"kind": "record", "id": 13, "layerId": "kayit", "attrs": {"bas": "2010-01-01", "bit": "2030-01-01",
                                                               "malik": "Ayşe"}},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
SETUP = {"format": "kentos.document", "version": 1, "name": "Sürümler", "settings": SETTINGS,
         "origin": {"x": 487000.0, "y": 4420000.0}, "layers": LAYERS, "activeLayer": "parsel",
         "styles": {"items": [], "categories": []}, "entities": ENTITIES}
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}
# The kinds a copy cannot be made of (a record, an architectural object): their copies are their own commands'.
UNCOPIED = {"record"}
# What a copy carries of its object besides the attributes (the geometry by its kind).
CARRIED = ("kind", "layerId", "color", "lineWeight", "label", "symbol")


def refused(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def typed(layer_id, name):
    return next((f for f in NODES[layer_id].get("fields", []) if f["name"] == name), None)


def as_field(layer_id, name, text):
    """The text a typed field keeps of `text` (its canonical form), or `text` on a field without a kind."""
    f = typed(layer_id, name)
    if f is None:
        return text
    got = fields.check(f, text)
    assert "value" in got, got
    return got["value"]


def plan(uids, at, kind):
    """What the command does, or (code, message, path) when it refuses, the checks in the contract's order."""
    moment = temporal.read(at)
    if not isinstance(moment, int):
        return ("invalid_moment", f"“{at}” bir tarih olarak okunamadı; 05.03.2024, 2024-03-05 ya da 2024-03-05T10:30 "
                "gibi yazın.", "at")
    seen, groups, order = set(), [], []
    outside = []
    for i, slot in enumerate(uids):
        if slot in seen:
            continue
        seen.add(slot)
        e = BY_ID[slot]
        node = NODES[e["layerId"]]
        name = node["name"]
        time = node.get("time") or {}
        if "end" not in time:
            return ("not_ranged", f"“{name}” katmanı başlangıç ve bitiş alanı olan bir zamansal katman değil; sürüm "
                    "yalnız aralıklı katmanda yapılır. Katmanın zaman ayarlarına bakın. Hiçbir nesne yazılmadı.",
                    f"uids[{i}]")
        if node["locked"]:
            return ("layer_locked", f"“{name}” katmanı kilitli; hiçbir nesne yazılmadı. Kilidi Katmanlar panelinden "
                    "açın.", f"uids[{i}]")
        start, end = (temporal.read(e["attrs"].get(k, "")) for k in (time["start"], time["end"]))
        if "unreadable" in (start, end):
            return ("time_unreadable", f"“{name}” katmanındaki bir nesnenin başlangıcı ya da bitişi tarih olarak "
                    "okunamıyor; önce Öznitelikler’den düzeltin. Hiçbir nesne yazılmadı.", f"uids[{i}]")
        if kind == "version" and e["kind"] in UNCOPIED:
            return ("not_versionable", f"“{name}” katmanındaki nesnenin yeni sürümü yazılamaz: kayıtların ve mimari "
                    "nesnelerin kopyası kendi komutlarıyladır. Nesneyi yalnız sona erdirebilirsiniz.", f"uids[{i}]")
        days = any(typed(e["layerId"], k) and typed(e["layerId"], k)["kind"] == "date"
                   for k in (time["start"], time["end"]))
        on = temporal.floor_to(moment, "day") if days else moment
        low = start if isinstance(start, int) else -temporal.INF
        high = end if isinstance(end, int) else temporal.INF
        if not low < on < high:
            outside.append((i, temporal.show(on, "day")))
        order.append(slot)
        group = next((g for g in groups if g["layer"] == e["layerId"]), None)
        if group is None:
            groups.append({"layer": e["layerId"], "start": time["start"], "end": time["end"], "at": on,
                           "objects": [slot], "first": i})
        else:
            group["objects"].append(slot)
    if outside:
        i, when = outside[0]
        return ("not_within", f"{len(outside)} nesnenin zamanı {when} anını içermiyor: başlangıcı bu tarihten önce, "
                "bitişi sonra olmalı. Hiçbir nesne yazılmadı.", f"uids[{i}]")
    for g in groups:
        text = temporal.write(g["at"], False)
        for name in [g["end"]] + ([g["start"]] if kind == "version" else []):
            f = typed(g["layer"], name)
            if f is not None:
                got = fields.check(f, text)
                if "error" in got:
                    return ("invalid_attribute", got["message"], "at")
    return {"groups": groups, "order": order}


def outcome(name, uids, at, kind=None, note=None, warnings=()):
    """A write: the output, the objects' ends, the copies, one undo step; the objects as they were after undo."""
    k = kind or "version"
    done = plan(uids, at, k)
    assert isinstance(done, dict), (name, done)
    slot = max(IDS)
    fields_after, copies, layers = {}, {}, []
    for g in done["groups"]:
        text = temporal.write(g["at"], False)
        end_text = as_field(g["layer"], g["end"], text)
        layers.append({"layer": g["layer"], "at": end_text})
        for s in g["objects"]:
            e = BY_ID[s]
            attrs = dict(e["attrs"])
            attrs[g["end"]] = end_text
            fields_after[str(s)] = {"attrs": attrs}
            if k != "version":
                continue
            slot += 1
            copy = {key: as_field(g["layer"], key, v) for key, v in e["attrs"].items()}
            copy[g["start"]] = as_field(g["layer"], g["start"], text)
            copies[s] = slot
            want = {key: e[key] for key in CARRIED if key in e}
            want["attrs"] = copy
            want["pts" if e["kind"] == "polygon" else "p"] = e["pts" if e["kind"] == "polygon" else "p"]
            for key in CARRIED:
                want.setdefault(key, None)
            fields_after[str(slot)] = want
    created = [copies[s] for s in done["order"] if s in copies]
    output = {"kind": k, "ended": [f"$uidOf:{s}" for s in done["order"]],
              "created": [f"$uidOf:{s}" for s in created], "ids": created, "layers": layers,
              "revision": "$current"}
    step_input = {"uids": [f"$uidOf:{s}" for s in uids], "at": at}
    if kind:
        step_input["kind"] = kind
    ids = IDS + sorted(copies.values())
    step = {"op": "execute", "input": step_input,
            "result": {"status": "completed", "output": output, "warnings": list(warnings)},
            "expect": {"ids": ids, "canUndo": True, "dirty": True, "revision": "changed", "fields": fields_after}}
    before = {str(s): {"attrs": BY_ID[s]["attrs"]} for s in done["order"]}
    undo = {"op": "undo", "returns": "Yeni sürüm oluştur" if k == "version" else "Sona erdir",
            "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "fields": before}}
    case = {"name": name, "steps": [step, undo]}
    if note:
        case["note"] = note
    return case


def refusal(name, ids, at, kind=None, note=None):
    """A refusal the checks reach, of objects by their slots."""
    got = plan(ids, at, kind or "version")
    assert isinstance(got, tuple), (name, got)
    code, message, path = got
    step_input = {"uids": [f"$uidOf:{i}" for i in ids], "at": at}
    if kind:
        step_input["kind"] = kind
    return stated(name, step_input, refused(code, message, path), note)


def stated(name, step_input, result, note=None):
    case = {"name": name, "steps": [{"op": "execute", "input": step_input, "result": result, "expect": UNCHANGED}]}
    if note:
        case["note"] = note
    return case


UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4099"


def build_cases():
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; yeni sürümler görünmeyecek.",
              "path": "uids[0]"}
    cases = [
        outcome("Yeni sürüm: parsel tarihte biter, kopyası tarihte başlar", [1], "2020-06-15",
                note="Kopya nesnenin geometrisini, rengini, kalınlığını, özniteliklerini ve etiketini taşır; bitişi "
                     "nesnenin eski bitişidir."),
        outcome("Bitişi olmayan nesne: kopyası da bitişsiz", [2], "2020-06-15T14:30",
                note="Saatli tarih, saatiyle yazılır: 2020-06-15T14:30:00."),
        outcome("İki parsel GG.AA.YYYY tarihle", [1, 2], "15.06.2020"),
        outcome("İki katman: katmanlar ilk görüldükleri sırayla, kopyalar nesnelerin sırasıyla", [3, 1],
                "2020-06-15T14:30",
                note="Binalar'ın tarihleri tarih türünde: orada yalnız gün yazılır; Parseller'de saatiyle."),
        outcome("Sona erdir: yalnız bitiş yazılır", [1], "2020-06-15", kind="end"),
        outcome("Sona erdir: bitişi olmayan nesne", [2], "01.01.2021", kind="end"),
        outcome("Tarih ve saat alanı: alanın kendi yazımıyla", [4], "2020-06-15",
                note="Saatli katmanın alanları tarih ve saat türünde: tarih 2020-06-15T00:00:00 yazılır."),
        outcome("Saat dilimli tarih UTC'ye çevrilir", [1], "2020-06-15T02:00:00+03:00"),
        outcome("Aynı nesne iki kez: bir kez sayılır", [1, 1], "2020-06-15"),
        outcome("Gizli katmanda yeni sürüm: söylenir", [8], "2020-06-15", warnings=[hidden]),
        outcome("Gizli katmanda sona erdir: uyarı yok", [8], "2020-06-15", kind="end"),
        outcome("Gün içinde bir an: tür alanı yoksa saatiyle", [1], "2010-01-01T12:00",
                note="Parseller'in alanları metin: an, başlangıç gününün öğlesidir ve aralığın içindedir."),
        outcome("Başlangıç alanı tam sayı: sona erdir yazar", [11], "2020-06-15", kind="end",
                note="Yalnız bitiş yazılır; başlangıç alanının türü sorulmaz."),
        outcome("Kaydı sona erdir", [13], "2020-06-15", kind="end"),
        {"name": "Plan: yazılmaz",
         "steps": [{"op": "plan", "input": {"uids": ["$uidOf:1"], "at": "2020-06-15"},
                    "result": {"status": "completed",
                               "output": {"changed": "$any", "created": "$any", "removed": [],
                                          "revision": "$current"},
                               "warnings": []},
                    "expect": UNCHANGED}]},
        {"name": "Doğrulama: yazılmaz",
         "steps": [{"op": "validate", "input": {"uids": ["$uidOf:1"], "at": "2020-06-15"},
                    "result": {"status": "completed", "output": None, "warnings": []}, "expect": UNCHANGED}]},
        stated("Nesne verilmedi", {"uids": [], "at": "2020-06-15"},
               refused("no_entities", "Sürümü yazılacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.",
                       "uids")),
        stated("Kimlik UUID değil", {"uids": ["nesne-1"], "at": "2020-06-15"},
               refused("invalid_uid", "“nesne-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir "
                       "UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun "
                       "çıktısından ya da çizimden alın.", "uids[0]")),
        refusal("Tarih okunamıyor", [1], "15/06/2020"),
        refusal("Tarih boş", [1], ""),
        stated("Beklenen sürüm geçersiz", {"uids": ["$uidOf:1"], "at": "2020-06-15", "expectedRevision": "dün"},
               refused("invalid_revision", "Beklenen sürüm “dün” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                       "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")),
        stated("Kimlik çizimde yok", {"uids": ["$uidOf:1", UID], "at": "2020-06-15"},
               refused("entity_not_found", f"“{UID}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
                       "olabilir. Var olan bir nesnenin kimliğini verin.", "uids[1]")),
        refusal("Anlık katman: aralıklı değil", [5], "2020-06-15"),
        refusal("Zaman ayarı olmayan katman", [6], "2020-06-15"),
        refusal("Kilitli katman", [7], "2020-06-15"),
        refusal("Başlangıç tarih olarak okunamıyor", [10], "2020-06-15"),
        refusal("Kaydın yeni sürümü yazılamaz", [13], "2020-06-15"),
        refusal("Bir nesnenin zamanı tarihi içermiyor", [1, 12], "2011-01-01"),
        refusal("İki nesnenin zamanı tarihi içermiyor", [1, 12], "2031-01-01"),
        refusal("Tarih başlangıca eşit: içermez", [1], "2010-01-01"),
        refusal("Tarih alanında gün yazılır: başlangıç günündeki an içermez", [3], "2005-03-01T12:00",
                note="Binalar'da tarih o günün gece yarısıdır, başlangıca eşittir."),
        refusal("Önce her nesnenin katmanı: aralıklı olmayan, içermeyen nesneden önce söylenir", [12, 5],
                "2011-01-01"),
        refusal("Bitiş alanı tam sayı: tarihi almaz", [9], "2020-06-15"),
        refusal("Yeni sürümde başlangıç alanı tam sayı: tarihi almaz", [11], "2020-06-15"),
    ]
    cases.insert(16, {"name": "Beklenen sürüm eski: çakışma",
                      "steps": [{"op": "execute", "input": {"uids": ["$uidOf:1"], "at": "2020-06-15",
                                                            "expectedRevision": "999"},
                                 "result": {"status": "conflict",
                                            "error": {"code": "revision_conflict",
                                                      "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir "
                                                                 "şey yazılmadı. Komutu çizimin şimdiki hâline göre "
                                                                 "yeniden hazırlayın.",
                                                      "path": "expectedRevision", "revision": "$current"}},
                                 "expect": UNCHANGED}]})
    return cases


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build(cases):
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.time.version",',
           '  "commandVersion": 1,', f'  "title": {compact("Yeni sürüm oluştur")},',
           f'  "note": {compact("Written by scripts/fixtures/time_version_command_cases.py from docs/adr/0210 §7 and 0622 §9; do not edit by hand.")},',
           f'  "setup": {compact(SETUP)},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(s)}" for s in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    return "\n".join(out) + "\n"


def main():
    cases = build_cases()
    text = build(cases)
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


if __name__ == "__main__":
    main()
