#!/usr/bin/env python3
"""The shared cases of `cad.arch.ifc` (plan/architecture-advanced.md M47, docs/adr/8047), without KentOS
code.

The drawing is section_building.py's: two storeys, 16 walls, 14 openings (5 doors, 9 windows), 10 rooms, 3 slabs,
16 columns, 8 beams and a stair. A case says what the file holds (bytes and digest aside): the schema EK-5 §5.4 asks
(IFC 4.3, `IFC4X3_ADD2`) and how many of each class, counted from the definition. A ramp is not written yet and is
said; a drawing without a building writes nothing. The file's content is arch_ifc_check.py's: IfcOpenShell against the
same definition.

    python3 scripts/fixtures/arch_ifc_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import section_building as sb  # noqa: E402

OUT = ROOT / "fixtures/commands/v1/cad.arch.ifc.json"
STAMP = "2026-11-11T12:00:00"
# Writing a file changes nothing in the drawing.
SAME = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
SETUP = copy.deepcopy(sb.d)


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def step(op, input_, result):
    return {"op": op, "input": input_, "result": result, "expect": dict(SAME)}


def case(name, steps, setup=None):
    c = {"name": name, "steps": steps}
    if setup is not None:
        c["setup"] = setup
    return c


def counts(d):
    """How many of each class the file holds: the definition's objects by kind."""
    ents = d["entities"]
    kind = lambda k: [e for e in ents if e["kind"] == k]  # noqa: E731
    openings = kind("opening")
    return {"storeys": max(len(d["settings"].get("arch", {}).get("levels", [])), 1),
            "walls": len(kind("wall")),
            "doors": sum(o["form"] == "door" for o in openings),
            "windows": sum(o["form"] == "window" for o in openings),
            "openings": len(openings),
            "spaces": len(kind("space")),
            "slabs": len(kind("slab")),
            "columns": len(kind("column")),
            "beams": len(kind("beam")),
            "roofs": len(kind("roof")),
            "railings": len(kind("railing")),
            "stairs": len(kind("stair"))}


def output(path, d=SETUP):
    return {"path": path, "schema": "IFC4X3_ADD2", **counts(d), "bytes": "$any", "sha256": "$any"}


def with_ramp():
    """The building with a ramp at the entrance: the file is written, the ramp is not, and that is said."""
    d = copy.deepcopy(SETUP)
    n = max(e["id"] for e in d["entities"]) + 1
    d["entities"].append({"kind": "ramp", "id": n, "layerId": "merdiven", "attrs": {},
                          "pts": [{"x": 5.3, "y": 12.0}, {"x": 5.3, "y": 9.25}], "width": 1.2, "rise": 0.15,
                          "slope": 5.0, "arch": {"level": "zemin"}})
    d["uids"].append(sb.uid(n))
    return d


def numbered(d):
    """The objects' ids in file order again (1, 2 …), as a drawing's are; their persistent ids (and the openings' hosts,
    which name those) stay."""
    for k, e in enumerate(d["entities"]):
        e["id"] = k + 1
    return d


def ground_only():
    """The 1. kat emptied: its storey is written all the same, the ground's objects alone in it."""
    d = copy.deepcopy(SETUP)
    keep = [i for i, e in enumerate(d["entities"])
            if e.get("arch", {}).get("level") != "k1"
            and not (e["kind"] == "opening" and any(w.get("arch", {}).get("level") == "k1"
                                                     for w, u in zip(d["entities"], d["uids"]) if u == e["host"]))]
    d["entities"] = [d["entities"][i] for i in keep]
    d["uids"] = [d["uids"][i] for i in keep]
    return numbered(d)


def without_building():
    """The drawing's grids and section marks alone: no building to write."""
    d = copy.deepcopy(SETUP)
    keep = [i for i, e in enumerate(d["entities"]) if e["kind"] in ("grid", "viewmark")]
    d["entities"] = [d["entities"][i] for i in keep]
    d["uids"] = [d["uids"][i] for i in keep]
    return numbered(d)


def cases():
    path_words = "IFC'nin yolu boş ya da göreli; mutlak bir .ifc yolu verin."
    there = "$fixture:arch/v1/ifc/var.ifc"
    ramp = with_ramp()
    ground = ground_only()
    nothing = without_building()
    return [
        case("Bölüm binası: iki kat, duvarlar, kapılar, pencereler, mahaller, döşemeler, kolonlar, kirişler, merdiven",
             [step("execute", {"path": "$tmp:bina-1.ifc", "stamp": STAMP}, completed(output("$tmp:bina-1.ifc")))]),
        case("Planı: dosya yazılmadan",
             [step("plan", {"path": "$tmp:bina-2.ifc", "stamp": STAMP}, completed(output("$tmp:bina-2.ifc")))]),
        case("Yalnız doğrulama", [step("validate", {"path": "$tmp:bina-3.ifc"}, completed(None))]),
        case("Var olan dosya: overwrite olmadan reddedilir, overwrite ile yazılır",
             [step("execute", {"path": "$tmp:bina-4.ifc", "stamp": STAMP}, completed(output("$tmp:bina-4.ifc"))),
              step("execute", {"path": "$tmp:bina-4.ifc", "stamp": STAMP},
                   failed("file_exists", "$tmp:bina-4.ifc var; üzerine yazmak için overwrite verin.", "path")),
              step("execute", {"path": "$tmp:bina-4.ifc", "stamp": STAMP, "overwrite": True},
                   completed(output("$tmp:bina-4.ifc")))]),
        case("Rampa henüz yazılmaz ve söylenir",
             [step("execute", {"path": "$tmp:bina-5.ifc", "stamp": STAMP},
                   completed(output("$tmp:bina-5.ifc", ramp),
                             [{"code": "export_losses",
                               "message": "1 rampa IFC'ye yazılmadı: henüz IFC'ye aktarılmıyor."}]))],
             setup=ramp),
        case("Boş yol", [step("execute", {"path": ""}, failed("invalid_file_path", path_words, "path"))]),
        case("Göreli yol", [step("execute", {"path": "bina.ifc"}, failed("invalid_file_path", path_words, "path"))]),
        case("IFC değil", [step("execute", {"path": "$tmp:bina.dxf"},
                                failed("invalid_file_path", "“$tmp:bina.dxf” bir .ifc dosyası değil.", "path"))]),
        case("Klasörü yok", [step("execute", {"path": "$tmp:yok/bina.ifc"},
                                  failed("invalid_file_path",
                                         "“$tmp:yok” klasörü yok; dosya var olan bir klasöre yazılır.", "path"))]),
        case("Dosya var", [step("execute", {"path": there},
                                failed("file_exists", f"{there} var; üzerine yazmak için overwrite verin.", "path"))]),
        case("Okunmayan saat", [step("execute", {"path": "$tmp:bina-6.ifc", "stamp": "2026-11-11 12:00"},
                                     failed("invalid_ifc", "“2026-11-11 12:00” bir tarih ve saat değil; "
                                            "YYYY-MM-DDThh:mm:ss yazın.", "stamp"))]),
        case("Bina yok: yalnız akslar ve kesit işaretleri",
             [step("execute", {"path": "$tmp:bina-7.ifc", "stamp": STAMP},
                   failed("invalid_ifc", "Yazılacak bina yok: çizimde gövdesi olan duvar, döşeme, kolon, kiriş, çatı, "
                          "korkuluk, merdiven ya da mahal yok."))],
             setup=nothing),
        case("Planı: rampa uyarısıyla",
             [step("plan", {"path": "$tmp:bina-8.ifc", "stamp": STAMP},
                   completed(output("$tmp:bina-8.ifc", ramp),
                             [{"code": "export_losses",
                               "message": "1 rampa IFC'ye yazılmadı: henüz IFC'ye aktarılmıyor."}]))],
             setup=ramp),
        case("Doğrulama: rampa uyarısıyla",
             [step("validate", {"path": "$tmp:bina-9.ifc"},
                   completed(None, [{"code": "export_losses",
                                     "message": "1 rampa IFC'ye yazılmadı: henüz IFC'ye aktarılmıyor."}]))],
             setup=ramp),
        case("Doğrulama: bina yok",
             [step("validate", {"path": "$tmp:bina-10.ifc"},
                   failed("invalid_ifc", "Yazılacak bina yok: çizimde gövdesi olan duvar, döşeme, kolon, kiriş, çatı, "
                          "korkuluk, merdiven ya da mahal yok."))],
             setup=nothing),
        case("Planı: göreli yol", [step("plan", {"path": "bina.ifc"}, failed("invalid_file_path", path_words, "path"))]),
        case("Doğrulama: dosya var",
             [step("validate", {"path": there},
                   failed("file_exists", f"{there} var; üzerine yazmak için overwrite verin.", "path"))]),
        case("Planı: var olan dosya overwrite ile (plan dosyaya dokunmaz)",
             [step("plan", {"path": there, "stamp": STAMP, "overwrite": True}, completed(output(there)))]),
        case("Büyük harfli uzantı",
             [step("execute", {"path": "$tmp:BINA-11.IFC", "stamp": STAMP}, completed(output("$tmp:BINA-11.IFC")))]),
        case("Yolun başındaki ve sonundaki boşluk kırpılır",
             [step("execute", {"path": "  $tmp:bina-12.ifc  ", "stamp": STAMP},
                   completed(output("$tmp:bina-12.ifc")))]),
        case("Saat verilmezse şimdiki zaman yazılır",
             [step("execute", {"path": "$tmp:bina-13.ifc"}, completed(output("$tmp:bina-13.ifc")))]),
        case("Yalnız zemin katı dolu: boş 1. kat da yazılır",
             [step("execute", {"path": "$tmp:bina-14.ifc", "stamp": STAMP},
                   completed(output("$tmp:bina-14.ifc", ground)))],
             setup=ground),
    ]


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "cad.arch.ifc",
            "commandVersion": 1, "title": "Binayı IFC'ye aktar",
            "note": "scripts/fixtures/arch_ifc_cases.py yazar; çizim section_building.py'nin tanımıdır, sayılar ondan; "
                    "KentOS kodu yok. Dosyanın içeriği arch_ifc_check.py ile IfcOpenShell'le (şema, EXPRESS kuralları, "
                    "EK-5 adları, yapı, özellikler, gövdeler) aynı tanıma karşı denetlenir.",
            "setup": SETUP, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
