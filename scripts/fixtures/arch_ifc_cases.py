#!/usr/bin/env python3
"""The shared cases of `cad.arch.ifc` (plan/architecture-advanced.md M47, docs/adr/8047), without KentOS
code.

The drawing is section_building.py's: two storeys, 16 walls, 14 openings (5 doors, 9 windows), 10 rooms, 3 slabs,
16 columns, 8 beams and a stair. A case says what the file holds (bytes and digest aside): the schema EK-5 §5.4 asks
(IFC 4.3, `IFC4X3_ADD2`) and how many of each class, counted from the definition. A ramp is written (docs/adr/8070
§1) and nothing is said of it; grids all in one direction are no IfcGrid and are said (§4); a drawing without a
building writes nothing. The file's content is arch_ifc_check.py's: IfcOpenShell against the same definition.

    python3 scripts/fixtures/arch_ifc_cases.py [--check]
"""

import copy
import math
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


def curtain_frames(w):
    """A curtain wall's mullions and transoms (ADR 8070 §7), worked out here: each straight stretch in as many equal bays
    as leave none wider than its spacing, its height in as many equal rows likewise; a mullion on every vertical line of
    a stretch (its two ends among them), a transom in each bay on every horizontal line (its foot and top among them)."""
    pts = w["pts"] + ([w["pts"][0]] if w.get("closed") else [])
    g, h = w["curtain"], w["height"]
    rows = max(1, math.ceil(h / g["rows"] - 1e-9))
    n = 0
    for a, b in zip(pts, pts[1:]):
        bays = max(1, math.ceil(math.hypot(b["x"] - a["x"], b["y"] - a["y"]) / g["spacing"] - 1e-9))
        n += (bays + 1) + bays * (rows + 1)
    return n


def grid_axes(ents):
    """The grid axes written (ADR 8070 §4): a storey's (none: the building's) axes make a grid when they run two ways,
    those parallel to its first straight one and the rest; one way alone, none."""
    by = {}
    for e in ents:
        if e["kind"] == "grid":
            by.setdefault(e.get("arch", {}).get("level"), []).append(e)
    n = 0
    for gs in by.values():
        first = next((g for g in gs if not g.get("bulges")), None)
        if first is None:
            continue
        (a, b) = first["pts"]
        ux, uy = b["x"] - a["x"], b["y"] - a["y"]

        def parallel(g):
            (p, q) = g["pts"]
            return not g.get("bulges") and abs(ux * (q["y"] - p["y"]) - uy * (q["x"] - p["x"])) <= 1e-9
        if any(not parallel(g) for g in gs):
            n += len(gs)
    return n


def counts(d):
    """How many of each class the file holds: the definition's objects by kind; a curtain wall its own class and its
    frames members, a post or a rail a member (ADR 8070 §8, §9)."""
    ents = d["entities"]
    kind = lambda k: [e for e in ents if e["kind"] == k]  # noqa: E731
    openings = kind("opening")
    curtains = [w for w in kind("wall") if w.get("curtain")]
    # No case has a chimney or a library block: their rules (the flue and the stack over it one; the card's class) are
    # the application tests'.
    assert not any(o["form"] == "flue" for o in openings) and not any(r.get("parts") for r in kind("roof"))
    assert not kind("insert")
    return {"storeys": max(len(d["settings"].get("arch", {}).get("levels", [])), 1),
            "walls": len(kind("wall")) - len(curtains),
            "doors": sum(o["form"] == "door" for o in openings),
            "windows": sum(o["form"] == "window" for o in openings),
            "openings": len(openings),
            "spaces": len(kind("space")),
            "slabs": sum(1 for e in kind("slab") if not e.get("covering")),
            "columns": sum(1 for e in kind("column") if not e.get("member")),
            "beams": sum(1 for e in kind("beam") if not e.get("member")),
            "roofs": len(kind("roof")),
            "railings": len(kind("railing")),
            "stairs": len(kind("stair")),
            "ramps": len(kind("ramp")),
            "chimneys": 0,
            "curtainWalls": len(curtains),
            "members": sum(1 for e in kind("column") + kind("beam") if e.get("member"))
            + sum(curtain_frames(w) for w in curtains),
            "fixtures": 0,
            "gridAxes": grid_axes(ents),
            "coverings": sum(1 for e in kind("slab") if e.get("covering"))}


def output(path, d=SETUP, view="designTransfer"):
    """What the export says: its bodies whole (designTransfer) or already cut (reference, docs/adr/8071 §4); the same
    objects either way."""
    return {"path": path, "schema": "IFC4X3_ADD2", "view": view, **counts(d), "bytes": "$any", "sha256": "$any"}


def with_ramp():
    """The building with a ramp at the entrance: the file is written with it, nothing said."""
    d = copy.deepcopy(SETUP)
    n = max(e["id"] for e in d["entities"]) + 1
    d["entities"].append({"kind": "ramp", "id": n, "layerId": "merdiven", "attrs": {},
                          "pts": [{"x": 5.3, "y": 12.0}, {"x": 5.3, "y": 9.25}], "width": 1.2, "rise": 0.15,
                          "slope": 5.0, "arch": {"level": "zemin"}})
    d["uids"].append(sb.uid(n))
    return d


def with_members():
    """The building with a curtain wall south of it, a post and a rail (ADR 8070 §7, §8): the curtain wall one, its 5
    mullions and 16 transoms members (6 m at 1.50: 4 bays; 3 m at 1.20: 3 rows), the post and the rail two more."""
    d = copy.deepcopy(SETUP)
    n = max(e["id"] for e in d["entities"]) + 1
    P = lambda x, y: {"x": x, "y": y}  # noqa: E731
    for k, e in enumerate([
        {"kind": "wall", "layerId": "cizim", "pts": [P(20.0, -3.0), P(26.0, -3.0)], "width": 0.15, "height": 3.0,
         "arch": {"level": "zemin"}, "curtain": {"spacing": 1.5, "rows": 1.2, "panel": "Cam", "frames": "Alüminyum"}},
        {"kind": "column", "layerId": "kolon", "at": P(28.0, 2.0), "width": 0.1, "height": 2.4, "material": "Çelik",
         "member": {"predefined": "POST"}, "arch": {"level": "zemin"}},
        {"kind": "beam", "layerId": "kiris", "pts": [P(27.0, 0.0), P(29.0, 0.0)], "width": 0.05, "height": 0.1,
         "baseOffset": 1.0, "material": "Çelik", "member": {"predefined": "USERDEFINED", "name": "Kayıt"},
         "arch": {"level": "zemin"}},
    ]):
        d["entities"].append({"id": n + k, "attrs": {}, **e})
        d["uids"].append(sb.uid(n + k))
    return d


def one_way():
    """The building with its grids A–D alone (1–3 taken away): all in one direction, they make no IfcGrid."""
    d = copy.deepcopy(SETUP)
    keep = [i for i, e in enumerate(d["entities"]) if not (e["kind"] == "grid" and e["name"].isdigit())]
    d["entities"] = [d["entities"][i] for i in keep]
    d["uids"] = [d["uids"][i] for i in keep]
    return numbered(d)


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
    members = with_members()
    lone = one_way()
    ground = ground_only()
    nothing = without_building()
    return [
        case("Bölüm binası: iki kat, duvarlar, kapılar, pencereler, mahaller, döşemeler, kolonlar, kirişler, merdiven",
             [step("execute", {"path": "$tmp:bina-1.ifc", "stamp": STAMP}, completed(output("$tmp:bina-1.ifc")))]),
        case("Reference View: gövdeler açıklıkları kesilmiş, sayılar aynı",
             [step("execute", {"path": "$tmp:bina-22.ifc", "stamp": STAMP, "view": "reference"},
                   completed(output("$tmp:bina-22.ifc", view="reference")))]),
        case("Planı: Reference View",
             [step("plan", {"path": "$tmp:bina-23.ifc", "view": "reference"},
                   completed(output("$tmp:bina-23.ifc", view="reference")))]),
        case("Planı: dosya yazılmadan",
             [step("plan", {"path": "$tmp:bina-2.ifc", "stamp": STAMP}, completed(output("$tmp:bina-2.ifc")))]),
        case("Yalnız doğrulama", [step("validate", {"path": "$tmp:bina-3.ifc"}, completed(None))]),
        case("Var olan dosya: overwrite olmadan reddedilir, overwrite ile yazılır",
             [step("execute", {"path": "$tmp:bina-4.ifc", "stamp": STAMP}, completed(output("$tmp:bina-4.ifc"))),
              step("execute", {"path": "$tmp:bina-4.ifc", "stamp": STAMP},
                   failed("file_exists", "$tmp:bina-4.ifc var; üzerine yazmak için overwrite verin.", "path")),
              step("execute", {"path": "$tmp:bina-4.ifc", "stamp": STAMP, "overwrite": True},
                   completed(output("$tmp:bina-4.ifc")))]),
        case("Rampa yazılır, bir şey söylenmez",
             [step("execute", {"path": "$tmp:bina-5.ifc", "stamp": STAMP},
                   completed(output("$tmp:bina-5.ifc", ramp)))],
             setup=ramp),
        case("Giydirme cephe, dikme ve kayıt: kendi sınıflarıyla sayılır",
             [step("execute", {"path": "$tmp:bina-21.ifc", "stamp": STAMP},
                   completed(output("$tmp:bina-21.ifc", members)))],
             setup=members),
        case("Tek yönde akslar ızgara olmaz ve söylenir",
             [step("execute", {"path": "$tmp:bina-20.ifc", "stamp": STAMP},
                   completed(output("$tmp:bina-20.ifc", lone),
                             [{"code": "export_losses",
                               "message": f"{sum(1 for e in lone['entities'] if e['kind'] == 'grid')} aks IFC'ye "
                                          "yazılmadı: katında hep bir yönde; IFC'nin aks ızgarası iki yönde aks "
                                          "ister."}]))],
             setup=lone),
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
                          "korkuluk, merdiven, rampa ya da mahal yok."))],
             setup=nothing),
        case("Planı: rampalı bina, uyarısız",
             [step("plan", {"path": "$tmp:bina-8.ifc", "stamp": STAMP},
                   completed(output("$tmp:bina-8.ifc", ramp)))],
             setup=ramp),
        case("Doğrulama: rampalı bina, uyarısız",
             [step("validate", {"path": "$tmp:bina-9.ifc"}, completed(None))],
             setup=ramp),
        case("Doğrulama: bina yok",
             [step("validate", {"path": "$tmp:bina-10.ifc"},
                   failed("invalid_ifc", "Yazılacak bina yok: çizimde gövdesi olan duvar, döşeme, kolon, kiriş, çatı, "
                          "korkuluk, merdiven, rampa ya da mahal yok."))],
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
